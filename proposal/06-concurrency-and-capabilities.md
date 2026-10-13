# 06 - Concurrency and capabilities

These two topics are in one document because in Sisu they are the same
mechanism seen from two sides: the way a program reaches the outside world
(capabilities) is also the way it waits on the outside world (tasks).

## Concurrency: decisions

| Decision | Choice | Main evidence |
|---|---|---|
| Model | Tasks (green threads) multiplexed over OS threads by a runtime scheduler | Go, Erlang, Java virtual threads (JEP 444), Kotlin coroutines; colourless by construction |
| `async`/`await` | None | Nystrom's "What color is your function"; the Rust async survey complaints; C#'s and JS's colouring spreading everywhere; Zig removing `async` in 0.11 and redesigning around an `Io` value |
| Lifetime of tasks | Structured: spawn only inside a `scope`, which joins before returning | Trio nurseries, Kotlin `coroutineScope`, Swift task groups, Java `StructuredTaskScope`, Rust `thread::scope`; the retrofits (Go `context`, C# `CancellationToken`) show the cost of not having it |
| Data-race freedom | Static, via `Send`/`Sync` auto traits and scoped borrows | Rust (works, from day one); Swift 6 (retrofit, partially walked back in 6.2); Go (race detector only; Tu et al. 2019 find channel misuse as bug-prone as shared memory) |
| Cancellation | Structured: cancelling a scope cancels its subtree; cancellation is a `Cancelled` error at blocking points | Trio, Kotlin; Go's `ctx` threading as the counter-example |
| Primitives | Channels, mutexes, atomics, condition variables, `select` | Go for channels, Rust for the typed mutex (`Mutex[T]` owns its data) |
| Data parallelism | A library, `std.par`, running `pure` bodies on a pool apart from tasks; no `par:` block, no keyword yet | Rayon's `join` and adaptive splitting, Cilk's serial elision, Chapel's `forall`; OpenMP's trust-based annotations as the counter-example |
| Actors | A library pattern (a task owning a mailbox), not a keyword | Erlang's model is the goal; Swift's `actor` keyword and Pony's capabilities show that baking it into the type system costs more than it gives for most programs |
| Preemption | Cooperative at blocking points, plus compiler-inserted yield points in loops | Go's asynchronous preemption (1.14) exists because pure cooperation starves the GC; without a GC, loop-yield points suffice and are cheaper |

## Tasks and scopes

```sisu
def crawl(net: Net, roots: list[Url]) -> dict[Url, Page] ! CrawlError:
    var pages = Mutex(dict())
    let outcome = task.scope do s:
        for url in roots:
            s.spawn do:
                let page = http.get(net, url)?     # ? returns from the task; the scope collects it
                pages.lock().insert(url, page)
                return Ok(())
        return Ok(())
    outcome?                                       # the scope returns the first error, after joining all
    return Ok(pages.into_inner())
```

- `task.scope do s:` creates a scope bound to the current stack frame.
  `s.spawn(() -> ...)` or `s.spawn do:` starts a task. The closure may borrow anything the frame
  owns because the scope cannot end before the task does. The type system
  enforces that captured references are `Sync` (shared across tasks) and
  that moved values are `Send`.
- A task returns a value; `s.spawn` returns a `Handle[T]` whose `join()`
  yields it as `T ! TaskError`, flattened when `T` is itself a result
  (`Handle[U ! E].join()` is `U ! {E, TaskError}`). If a task returns an error or panics and nobody joins it, the
  scope's result carries the first such failure in time, after cancelling
  the rest and joining everything; inside `pure` code it carries the
  lowest-index failure instead (see Effects). There are no orphaned tasks
  and no silent task failures: the leading cause of Go goroutine leaks and
  of unobserved `Task` failures in C# and Swift.
- Scopes nest. A scope is a value that cannot escape its frame (it is a
  view), so there is no way to spawn a task that outlives its parent frame.
  Long-lived background work lives in a scope that is itself long-lived
  (typically one opened in `main`): this is the Erlang supervision tree,
  expressed as lexical nesting.
- `s.spawn_detached` does not exist. The research on Go and on early Swift
  concurrency found fire-and-forget to be the dominant source of leaks and
  of shutdown bugs.

`task.race` is the one other scope form (leaning, D-075). It is Céu's
`par/or`
([`synchronous-and-dataflow.md`](../research/unconventional/synchronous-and-dataflow.md),
idea 2): the first task to finish wins, whatever its outcome, so a
failure that finishes first is the result (JavaScript's `Promise.race`,
not `Promise.any`); the others get `Cancelled` at their next blocking
point and run their `Drop`. A first-success variant is a library addition
if the corpus asks for one.

```sisu
let reply = task.race do r:
    r.spawn(() -> fetch(net, primary, key))
    r.spawn(() -> fetch(net, replica, key))
```

It is a library function over `task.scope` and `s.cancel()`, so it joins
every task before it returns, like any scope. Under the interpreter's
seeded scheduler the winner is a function of the seed. Which task finishes
first is the schedule, so `pure` code cannot use it (see Effects).

### Cancellation

Each scope has a cancellation flag. `s.cancel()` sets it for the subtree.
Every blocking operation (`http.get`, `channel.recv`, `clock.sleep`,
`handle.join`) checks the flag and returns `Err(Cancelled)` promptly;
`Cancelled` is a variant of the standard `TaskError` set and propagates via
`?` like any error. An operation that lent a buffer to the kernel or a
device returns only once the device has let go of it, which is what lets
completion I/O borrow the caller's buffer
([04-memory-and-ownership.md](04-memory-and-ownership.md); leaning,
D-058). Code that must finish (flush a file) uses
`task.shield do:`. Timeouts are `s.spawn_with(deadline=clock.after(Duration.seconds(5)))`.
There is no cancellation token to pass and no `ctx` parameter: the scope is
ambient to the task, which is one of the two pieces of implicit context the
design accepts, and it is accepted only because the alternative (Go) is
universally regretted. The other is a scope's trace context, which carries
observation only (see Interceptors and trace context below).

### Panics

A panicking task unwinds to its scope, which cancels its siblings (in
`pure` code, only those spawned after it) and reports `TaskPanicked` to
the joiner. A `Mutex` guard dropped by the unwinding unlocks its mutex, or
poisons it if the protected value's invariant no longer holds (see Send,
Sync, and shared state below). Supervisors are ordinary code:

```sisu
task.scope do s:
    while True:
        let worker = s.spawn(() -> worker_loop(net, port))
        match worker.join():
            case Err(TaskPanicked(message, ..)):
                log.error("worker died: {message}, restarting")
            case Err(Cancelled):
                break
            case Ok(()):
                break
```

## Data parallelism

Tasks are for work that waits on the world. Parallel computation over
data goes through `std.par` (D-069). `par_iter` and `par_iter_mut` split a
collection into work-stealing chunks with the usual `Send`/`Sync` checks;
`par.join` runs two to eight closures for divide and conquer; reducers
(`par.Sum`, `par.Collect`, or a user type implementing `par.Reducer`) give
each worker its own view and merge the views in a tree fixed by the
input's length, not by which worker stole what.

```sisu
let lo, hi = xs.split_at_mut(mid)
par.join(() -> sort(mut lo), () -> sort(mut hi))    # disjoint `mut` borrows, proven today
for p in particles.par_iter_mut():                  # work-stealing chunks, `Send` checked
    p.step(dt)
```

The guarantee is serial elision made exact: removing the parallelism gives
the same result, so results are identical across runs and core counts,
and a failing loop reports the lowest-index error, as a `pure` scope
does. Reductions follow the fixed shape that strict floating point gives
`sum`, `product` and `dot` ([03-types-and-generics.md](03-types-and-generics.md)).
Splitting is adaptive, with a `grain=` override, and `sisu bench` warns
when a parallel loop is slower than its serial elision.

A `par` body or `par.join` closure cannot block, which is what
`@no_block` checks ([04-memory-and-ownership.md](04-memory-and-ownership.md);
leaning, D-059), and it is `pure` (see Effects): it may mutate its own
element and nothing shared. It cannot do I/O, take a lock, wait on a
channel, or use `task.scope`, `spawn` or `join`; nested fork-join goes
through `par.join`. It therefore runs to completion on a worker's stack, with no
task stack and no parking, on a pool apart from the task scheduler, which
keeps fine-grained parallelism out of the stack budget that bet 3
measures ([16-validation-plan.md](16-validation-plan.md)).
Whether the pool shares OS threads with the scheduler belongs to Q-6,
which is open. Function types carry `pure` and nothing else: `std.par`'s
signatures take generic closures, and the `@no_block` check and the
capture kinds are verified on the concrete closure after monomorphisation
(Q-52).

```sisu
for url in urls.par_iter():
    let page = http.get(net, url)?   # error: `http.get` uses the capability `net`; a `par` body
                                     #   cannot block. Use task.scope and s.spawn for concurrent I/O.
```

A `par:` block whose statements run concurrently is rejected: `task.scope`
already is occam's `PAR`, X10's `finish` and Chapel's `cobegin`, and a
statement block with task semantics has no clear meaning for `return` or
`?`. A `par` contextual keyword (`for p in par mut particles:`) is
deferred. The library ships first, under the owner's rule that the
language core stays small, and the keyword waits for the corpus count of
Q-47: more than one `par_iter` in a plain loop per 300 lines. The usage
rules of occam and the closure rules of Rust are what Sisu's exclusivity
and `Send`/`Sync` checks already give; OpenMP's and Fortran's
`do concurrent` trust-based annotations produce silent wrong answers; and
Rayon's index-ordered panic rule is the failure rule above
([`parallel-languages.md`](../research/unconventional/parallel-languages.md)).

## Colourless I/O

A call such as `http.get(net, url)` blocks the *task*. The runtime parks the task,
runs others on the same OS thread, and resumes it when the I/O completes.
From the programmer's side there is nothing to annotate and no split
between sync and async libraries. This is Go's and Erlang's model and the
one Java chose after a decade of reactive frameworks (Project Loom).

The costs, from the research:

- A runtime is required. Sisu's runtime profiles
  ([12-interop-and-targets.md](12-interop-and-targets.md)) address this:
  `full` (M:N scheduler), `single` (one thread, event loop, for WebAssembly
  and simple tools), `freestanding` (no scheduler: `task.scope` is a compile
  error unless a scheduler is linked). The scheduler is reached through the
  capability layer (below), which is what makes it replaceable.
- Stack size. Go uses growable segmented-then-copied stacks, which need a
  GC-aware runtime to relocate pointers. Without a GC, Sisu cannot move
  stacks. Two mitigations: tasks start with a small stack reserved in
  virtual memory and committed lazily (the Java virtual threads and
  Erlang approach on 64-bit targets), and the compiler computes stack
  bounds for non-recursive call trees so that most tasks get an exact
  allocation. Deep recursion in a task that exhausts its stack panics with
  a clear message; this is open question Q-6. `std.par` bodies draw
  nothing from this budget, since they run on worker stacks (above).
- FFI blocking. A C call that blocks blocks the OS thread. The runtime
  detects this (as Go does) and spins up another thread; `extern` functions
  may be annotated `@blocking` to make it explicit.
- Zig's 2025-2026 design, where `Io` is an explicit parameter and
  `io.async(f)`/`io.await(h)` make concurrency points visible, was the main
  alternative considered. It is colourless (the same function works under
  any `Io` implementation) and needs no scheduler in the language. Sisu
  takes the explicit-`Io`-value idea for *capabilities* and the structured
  scope for *tasks*, but does not require `io.async` at every concurrent
  call because that is the colouring problem in a different place. Whether
  tasks should additionally be reachable only through a capability (so that
  a library cannot spawn tasks unless handed a `Tasks` capability) is open
  question Q-6.

## Send, Sync, and shared state

`Send` (may be moved to another task) and `Sync` (may be referenced from
several tasks) are inferred structurally, exactly as in Rust. A `Mutex[T]`
is `Sync` for any `Send` `T`; a `Cell[T]` is never `Sync`; a `Shared[T]` is
`Send` and `Sync` when `T` is `Sync`. The programmer only meets these names
in an error message, which says which field of which type is the obstacle
and what to wrap it in.

The Swift 6 experience is the cautionary tale: the rules are the same as
Rust's, but added to a language whose libraries were written without them,
the migration produced so many errors that Swift 6.2 (SE-0466, SE-0461)
made the default context sequential and concurrency opt-in. Two lessons
are built into Sisu from the start. First, the checks exist from the first
line of the standard library, which the Rust experience shows is enough.
Second, the Swift 6.2 default is Sisu's default: code is sequential unless
it is inside a `scope`, and `Send`/`Sync` are checked only where a value
crosses a `spawn` boundary. A program with no `spawn` never meets them.

The empirical case for static checking over a race detector comes from
Uber's study of Go data races ([`go.md`](../research/languages/go.md)):
46% were missing mutual exclusion around shared heap data and 39% involved
implicitly aliased growable slices, both of which Sisu's ownership rules
reject at compile time; only 2.5% involved channels. Go's race detector
finds these at run time, on the paths a test happens to take.

What the type system does not prevent is deadlock. No mainstream approach
does ([`../research/evidence/studies.md`](../research/evidence/studies.md));
Sisu's position is that `Mutex[T]` owning its data, structured scopes
(no lock can outlive its frame) and bounded channels reduce the surface,
and that a lock-order lint is tooling work after version 1.

When unwinding drops a `Mutex` guard, the protected type's `invariant`
([07-contracts.md](07-contracts.md)) is re-checked. If the check fails,
the mutex is poisoned and every later `lock()` panics; if it passes, or
the type declares no invariant, the mutex unlocks normally (D-077).
The re-check runs in every build mode whatever the package's `contracts`
level ([07-contracts.md](07-contracts.md)), because the unwinding path is
exceptional and its cost irrelevant. `lock()` stays infallible in its
signature, and contracts get a run-time job they suit: deciding whether the state a panicking task left behind is
still usable by a sibling or a restarted worker. Rust always poisons,
which puts a `Result` on every `lock()`; its nightly non-poisoning locks
leave half-updated state visible; and its documentation calls `UnwindSafe`
"a speed bump"
([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
F4). The structured-concurrency test suite includes a panic-while-locked
case.

Atomics follow the C++11/C11 memory model verbatim (`Relaxed`, `Acquire`,
`Release`, `AcqRel`, `SeqCst`). Every language since has converged on it
([`cpp.md`](../research/languages/cpp.md), Java's JSR 133 in
[`java.md`](../research/languages/java.md)); inventing a new one would be
novelty for its own sake.

## Channels and select

```sisu
let tx, rx = channel[Event](capacity=128)       # bounded; unbounded is a separate constructor
s.spawn do:
    for e in rx:                                # rx is an iterator; ends when tx is closed and drained
        handle(e)
tx.send(Event.Start)?                           # ? because the receiver may be gone
select:
    case e = rx.recv():
        handle(e)
    case _ = clock.after(Duration.seconds(1)):
        timeout()
    case _ = s.cancelled():
        return
```

Channels are typed, bounded by default (Go's unbounded-by-accident and
Erlang's mailbox-overflow failure modes), and closing is explicit from the
sender. `select` is a statement over channel operations and timers, as in
Go, with the addition of a cancellation arm.

## Capabilities

### The rule

A function can affect the world outside the process only through a value it
was given. There are no global functions for opening files, connecting
sockets, reading the clock, generating random numbers, reading environment
variables, or spawning processes.

```sisu
def main(sys: System) -> ! AppError:
    let fs = sys.fs.restrict_to("./data")
    let net = sys.net.only_hosts(["api.example.com"])
    return serve(fs, net, sys.clock)

def serve(fs: Fs, net: Net, clock: Clock) -> ! AppError: ...
```

`System` is constructed by the runtime and passed to `main`. It cannot be
constructed in safe code. Its fields are the root capabilities: `fs: Fs`,
`net: Net`, `clock: Clock`, `rand: Rand`, `env: Env`, `proc: Proc`,
`stdin`/`stdout`/`stderr`. Each is a `Copy` handle (copying a capability grants nothing new; a fake's
state lives behind the handle, which is how a test's `clock.advance` is
seen by every copy); each can be *attenuated* into a weaker capability (`fs.restrict_to(dir)`,
`fs.read_only()`, `net.only_hosts(...)`, `Clock.frozen_at(t)` for tests) and
passed on.

Capabilities stay `Copy` (Q-51). A copy can be kept, so attenuation
narrows authority but never takes it back, and `std.cap` adds two forms
that bound it. A *lent* capability is a view, `Lent[Fs]`, so the callee
cannot store it and its authority ends with the call, at no run-time cost.
A *revocable* capability comes from a caretaker: once it is revoked,
every retained copy fails with `Err(Revoked)`, and `Revoked` joins the I/O
error sets. A capability's operations are a trait that its root object,
its attenuations, `Lent[T]`, the test fakes and the interceptors all
implement, so a parameter typed by the capability accepts every form, and
the lent form, being a view, cannot be copied out of the parameter.

```sisu
let plugin_fs, revoker = fs.revocable()    # a caretaker
run_plugin(plugin_fs)?
revoker.revoke()                           # retained copies now fail with Err(Revoked)
render(fs.lend(), report)                  # Lent[Fs] is a view: render cannot store it
```

E's caretakers and JavaScript's `Proxy.revocable` are the precedents for
revocation, and lending reuses the view rules of
[04-memory-and-ownership.md](04-memory-and-ownership.md)
([`capabilities-distribution-and-resources.md`](../research/unconventional/capabilities-distribution-and-resources.md),
idea 3).

### Why

Three independent lines of evidence converge:

1. **Supply-chain security.** The npm, PyPI and crates.io incidents in the
   research ([`javascript.md`](../research/languages/javascript.md),
   [`../research/evidence/studies.md`](../research/evidence/studies.md)) are
   all the same shape: a dependency that has no business touching the
   network or the file system does so. Every existing defence (lockfiles,
   provenance, audits, Deno's runtime permission flags) is a policy applied
   from outside. Capabilities make it a type error: a `left-pad` library
   that is not passed `Net` has no way to phone home. Austral's design
   document makes this argument in full; Sisu adopts it.
2. **Testability.** Code that takes `Clock` and `Fs` as parameters is tested
   by passing a `Clock.frozen_at(...)` and an in-memory `Fs`. No mocking
   framework, no global test hooks, no "dependency injection container".
   This is the pattern that Go's `io.Reader`, Zig's `Io`, and every
   well-regarded codebase's "pass the dependencies in" advice converge on;
   Sisu makes it the only way, so libraries are testable whether or not
   their authors cared.
3. **Portability.** The `Io`-as-value design is why the same Sisu code can
   run on an M:N runtime, a single-threaded event loop in WebAssembly, or a
   bare-metal firmware image: the capability's implementation is what
   differs. Roc's platform/application split and Zig's `Io` interface
   demonstrate this.

### The cost

Every function that does I/O takes one more parameter, and that parameter
is threaded through every caller up to `main`. The research on Zig's `Io`
redesign and on Austral records this as the primary objection. Sisu's
answers:

- Most functions do no I/O. The ones that do should say so; this is the
  same visibility argument as for errors.
- Capability parameters are ordinary values and compose: a struct that
  needs `Fs` and `Clock` stores them in fields at construction.
- A small amount of ambient authority is kept deliberately: `print`,
  `print_err`, `debug.print` and the `log` facade (which writes to the
  standard error stream unless given a sink) write to the process's standard streams
  without a capability (Q-5 kept them ambient and records the dissent), and
  `task.scope` is ambient (open question Q-6). Everything else is gated.
- Implicit context passing (Odin's `context`, Scala's `using`, Kotlin's
  context parameters) was considered as a way to pass capabilities
  invisibly. It was rejected for version 1 because it reintroduces ambient
  authority through the back door and makes the testability and
  supply-chain arguments weaker. The Zig research
  ([`zig.md`](../research/languages/zig.md)) is explicit that the explicit
  `Io` parameter is the main ergonomic objection to that design, and
  recommends a *tracked* capability: declared in the signature, passed by
  the compiler. A `with Fs, Clock` clause on function signatures is the
  reserved syntax for that, and it is the most likely addition after the
  prototype measures the parameter burden; Q-5 keeps it reserved until
  then. The trace context below is implicit too, and is accepted because
  it carries observation only, never authority.

### Interceptors and trace context

Every effect passes through a capability, so a capability is also the
place to observe effects. `std.cap` provides `Wrap[T]` over any
capability trait (D-075), built from the reflection behind `Fake[T]`
([17-testing.md](17-testing.md)). It intercepts every call to the outside
world, visibly in `main`, typed, and with no pointcuts:

```sisu
def main(sys: System) -> ! AppError:
    let tracer = Tracer.to(sys.stderr)
    let net = trace.wrap(sys.net, tracer)        # every connect, read and write is a span
    let fs = audit.wrap(sys.fs.restrict_to("./data"), tracer)
    return serve(fs, net, sys.clock)
```

Test fakes, tracing, auditing and the interpreter's recording for replay
(N-04 in [19-novel-features.md](19-novel-features.md)) share this one
interception point. It is the sanctioned replacement for aspect-oriented
programming, whose obliviousness and fragile pointcuts are its documented
failure: the interception is a call in `main`, not advice woven in
elsewhere
([`incremental-reversible-and-exotic-paradigms.md`](../research/unconventional/incremental-reversible-and-exotic-paradigms.md),
idea 2).

A `task.scope` also carries a trace context: the trace and span
identifiers of the work it belongs to. Its children inherit it, and `log`
attaches it to every record, as Kotlin's coroutine context and Erlang's
logger metadata do, so the records of one request stay together across
tasks with no parameter to thread. This is a second piece of implicit
context beside the scope itself (see Cancellation). It carries observation
only, never authority: nothing can be reached through it, so the
supply-chain and testability arguments above are unaffected.

### Enforcement

Capabilities are only a guarantee if there is no way around them. Three
things could bypass them: `unsafe` code (raw syscalls), foreign function
calls, and compile-time code execution. Therefore:

- `unsafe` and `extern` are permitted only in packages listed under
  `[trust]` in the *root* manifest. A transitive dependency cannot grant
  itself the permission. `sisu audit` lists every trusted package and the
  unsafe blocks in it.
- `comptime` code runs in an interpreter with no I/O at all.
- There are no build scripts. Code generation happens in `comptime` or in a
  separately invoked tool.

The `sim` profile that the experiment prototypes for `sisu test --sim`
([17-testing.md](17-testing.md), D-070) turns this into a compile-time
check. A `sim` build fails to compile if reachable code could escape the
simulation, which is possible only through `extern` or `unsafe` in a
`[trust]`ed package that supplies no simulation model. The ambient
standard streams are captured per simulated node, so output escapes the
simulation no more than any other effect does.

### Effects

Capabilities are *values*; they are not an effect system. The difference
matters: an effect system (Koka, Effekt, Flix, OCaml 5's handlers) tracks
what a function *does* in its type and lets callers *handle* those effects.
Capabilities track what a function *can do* by what it was *given*. The
research ([`koka-effekt.md`](../research/languages/koka-effekt.md),
[`flix.md`](../research/languages/flix.md),
[`scala.md`](../research/languages/scala.md) on capture checking) finds
typed effect systems still unsettled in ergonomics (effect-polymorphic
higher-order functions, noisy signatures) with no large production
deployment in 2026, while capabilities-as-values are in production in Zig's
and Roc's models and in every "pass your dependencies" codebase.

Sisu therefore ships capabilities and no effect system (Q-17). One
property from the effect world is kept and tracked in function types:
`pure`. A `pure` function takes no capability, contains no `unsafe` and
mutates nothing shared, and it is deterministic: it returns the same bits
for the same arguments on every run, process, thread count and target
(D-074). Three rules make that hold:

- `dict` and `set` iterate in insertion order, while the SipHash key stays
  per process, so flooding resistance is unchanged and only iteration
  stops depending on the key ([11-stdlib.md](11-stdlib.md)). Python 3.7
  made the same split.
- The operations that observe the schedule are errors in `pure` code:
  `select`, `Mutex`, `Atomic`, channels with more than one sender,
  `try_lock`, `try_recv`, `try_send`, `Weak.upgrade` across tasks, and
  values derived from addresses. `task.scope`, `spawn` and `join` stay
  legal, so fork-join parallelism remains available.
- A scope in `pure` code runs every sibling spawned before a failing task
  to completion, cancels those spawned after it, and reports the
  lowest-index failure, so its result does not depend on which task
  failed first. Effectful code keeps the first failure in time (see Tasks
  and scopes), because there prompt cancellation matters more than
  reproducibility.

```sisu
pure def apply(state: Ledger, batch: Slice[Transfer]) -> Ledger ! TaskError:
    let parts = task.scope do s:                  # fork-join of pure tasks stays legal
        let fees = s.spawn(() -> compute_fees(batch))
        let risk = s.spawn(() -> score_risk(state, batch))
        return Ok((fees.join()?, risk.join()?))   # join order, never completion order
    let fees, risk = parts?
    return Ok(state.posted(fees, risk))           # same bits on 1 or 64 workers, x86 or ARM
```

Floating point is strict by default
([03-types-and-generics.md](03-types-and-generics.md)), which carries the
guarantee across targets. `pure` is part of function types, so
higher-order `pure` code demands `pure` callbacks; the resource
attributes of [04-memory-and-ownership.md](04-memory-and-ownership.md) and
closure capture kinds stay out of function types and are checked on the
concrete call graph instead (Q-52). Each rule has shipped:
insertion order in Python, Ruby, JavaScript's `Map` and Starlark; Rayon's
`join` reports the first closure's panic by index; Haskell's purity, the
`Par` monad and LVish give deterministic parallelism without covering hash
order or float code generation. The combined guarantee is checked nowhere:
Temporal polices deterministic workflow code with a linter that "will not
catch all cases", and Durable Functions with a run-time detector
([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
candidate 1 and findings F1 to F3). It buys replicated state machines,
lockstep simulation, reproducible builds and pipelines, and the
deterministic core that simulation and replay assume. It costs an index array and tombstones
in `dict`, and failures in `pure` scopes surface later because earlier
siblings run to completion; one that never terminates delays the report
indefinitely.

A function that meets these rules is pure modulo non-termination and
panics, and the compiler knows it; a `pure` annotation *requires* the
property. Such a function is eligible for `comptime`, whose promise to run
"exactly what would run at run time"
([08-metaprogramming.md](08-metaprogramming.md)) holds for `dict` order and
floating point because of these rules, and its results can be memoised
soundly, across processes as well as within one. Memoisation is a library,
not an attribute: attributes cannot wrap functions, so there is no
`@memo`. One audited exception is proposed (N-63): `std.x.incr`, an
incremental-computation library shaped like Salsa, keeps mutable caches
behind `pure` queries, so its `Db.read` and `Db.get` need a declaration
that they are observationally pure. Only a `[trust]`ed package can make
that declaration, and it is audited like `unsafe`.
