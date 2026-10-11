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
| Actors | A library pattern (a task owning a mailbox), not a keyword | Erlang's model is the goal; Swift's `actor` keyword and Pony's capabilities show that baking it into the type system costs more than it gives for most programs |
| Preemption | Cooperative at blocking points, plus compiler-inserted yield points in loops | Go's asynchronous preemption (1.14) exists because pure cooperation starves the GC; without a GC, loop-yield points suffice and are cheaper |

## Tasks and scopes

```sisu
def crawl(net: Net, roots: list[Url]) -> dict[Url, Page] ! CrawlError:
    var pages = Mutex(dict())
    let outcome = task.scope do s:
        for url in roots:
            s.spawn do:
                let page = net.get(url)?           # ? returns from the task; the scope collects it
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
  yields it. If a task returns an error or panics and nobody joins it, the
  scope's result carries the first such failure after cancelling the rest
  and joining everything. There are no orphaned tasks and no silent task
  failures: the leading cause of Go goroutine leaks and of unobserved
  `Task` failures in C# and Swift.
- Scopes nest. A scope is a value that cannot escape its frame (it is a
  view), so there is no way to spawn a task that outlives its parent frame.
  Long-lived background work lives in a scope that is itself long-lived
  (typically one opened in `main`): this is the Erlang supervision tree,
  expressed as lexical nesting.
- `s.spawn_detached` does not exist. The research on Go and on early Swift
  concurrency found fire-and-forget to be the dominant source of leaks and
  of shutdown bugs.

### Cancellation

Each scope has a cancellation flag. `s.cancel()` sets it for the subtree.
Every blocking operation (`net.get`, `channel.recv`, `clock.sleep`,
`handle.join`) checks the flag and returns `Err(Cancelled)` promptly;
`Cancelled` is a variant of the standard `TaskError` set and propagates via
`?` like any error. Code that must finish (flush a file) uses
`task.shield do:`. Timeouts are `s.spawn_with(deadline=clock.after(Duration.seconds(5)))`.
There is no cancellation token to pass and no `ctx` parameter: the scope is
ambient to the task, which is the one piece of implicit context the design
accepts, and only because the alternative (Go) is universally regretted.

### Panics

A panicking task unwinds to its scope, which cancels siblings and reports
`TaskPanicked` to the joiner. Supervisors are ordinary code:

```sisu
task.scope do s:
    while True:
        let worker = s.spawn(() -> serve(net, port))
        match worker.join():
            case Err(TaskPanicked(message, ..)):
                log.error("worker died: {message}, restarting")
            case Err(Cancelled):
                break
            case Ok(()):
                break
```

## Colourless I/O

A call such as `net.get(url)` blocks the *task*. The runtime parks the task,
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
  a clear message; this is open question Q-6.
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
`stdin`/`stdout`/`stderr`. Each is a value with methods; each can be
*attenuated* into a weaker capability (`fs.restrict_to(dir)`,
`fs.read_only()`, `net.only_hosts(...)`, `clock.frozen_at(t)` for tests) and
passed on.

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
  `print_err` and `debug.print` write to the process's standard streams
  without a capability (open question Q-5 records the dissent), and
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
  prototype measures the parameter burden. Recorded as open question Q-5.

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

Sisu therefore ships capabilities now and leaves effect typing as open
question Q-17. One thing from the effect world is kept: a function with no
capability parameters and no `unsafe` is provably pure modulo
non-termination and panics, and the compiler knows it (used for `comptime`
eligibility and for memoisation hints). A `pure` annotation to *require*
that property is cheap and is included.
