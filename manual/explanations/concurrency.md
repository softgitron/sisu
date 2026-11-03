# Concurrency without colours

Sisu has no `async` and no `await`. A function that waits for the
network is written like a function that adds numbers, and the runtime
parks the task that waits and runs another. Tasks start only inside a
`task.scope`, which waits for all of them before it returns, cancels
them together, and receives their failures. This page explains why
Sisu's concurrency has that shape, what the alternatives cost the
languages that chose them, what Sisu pays instead, and what the
measurements so far say.

**See also:** the tutorial's [chapter 20](../tutorial/20-concurrency.md)
(concurrency) and [chapter 22](../tutorial/22-web-service.md) (a service
with a task per connection); the reference's [R9,
Concurrency](../README.md#the-reference); the proposal's [06,
Concurrency and
capabilities](../../proposal/06-concurrency-and-capabilities.md) and
decisions D-014, D-040 and D-075 in its [decision
log](../../proposal/15-decision-log.md); the research in
[concurrency.md](../../research/topics/concurrency.md).

## The problem: what the usual answers cost

A program that serves many clients spends most of its time waiting: for
a socket, a disk, a timer. An operating-system thread per wait is
expensive, so languages have looked for a cheaper way to wait, and the
common answers each left a cost that their users still pay.

The first is colour. With `async` and `await`, in C#, JavaScript,
Python, Rust, Swift and Kotlin's `suspend`, a function that waits is
compiled differently from one that does not, and it can be called only
from another such function or through an executor. Bob Nystrom named
this the colour of a function, and the research finds it splitting
ecosystems ([concurrency.md](../../research/topics/concurrency.md),
finding 1). Python's synchronous libraries cannot be called from a
coroutine without blocking its loop, so the ecosystem wrote second
versions of them (`httpx`, `asyncpg`, `aiofiles`)
([python.md](../../research/languages/python.md)). Rust's async needed
`Pin` because its state machines refer to themselves, ties libraries to
executors, and is the part of Rust its own designers regret most; one of
them writes that Rust's compatibility with the C runtime "forced
stackless coroutines, which causes the function-coloring problem"
([rust.md](../../research/languages/rust.md)). Zig removed `async` and
`await` and redesigned its I/O around a value instead
([zig.md](../../research/languages/zig.md)).

The second is leaks. Go's goroutines have no colour, but nothing ties a
goroutine to the code that started it, so a goroutine can outlive its
purpose, blocked forever on a channel nobody reads. Leaks are common
enough that Go 1.27 added a built-in profile to find them
([go.md](../../research/languages/go.md)), and fire-and-forget tasks
were the main source of leaks and shutdown bugs in Go and in early Swift
concurrency.

The third is cancellation. A Go function that should stop when its
caller gives up takes a `context.Context` as its first parameter, and so
does every function it calls; C# threads a `CancellationToken` the same
way. Both work, and both are, in the research's word, universally
regretted.

The fourth is data races. Go finds them with a race detector, at run
time, on the paths a test happens to take; of the races that Uber fixed
after running the detector across its services, 46% were missing locks
around shared data and 39% came from slices sharing a backing array.
Swift 6 added Rust's static rules to a language whose libraries were
written without them, and the flood of errors made Swift 6.2 retreat to
a sequential default
([concurrency.md](../../research/topics/concurrency.md), findings 3 and
4).

## Tasks without colour

Sisu runs tasks, which are like threads but cheap, on a scheduler in the
runtime (D-014). A call that waits, such as a read from a socket or a
sleep on a clock, blocks the task that made it. The runtime parks that
task, runs others on the same operating-system thread, and resumes it
when the wait is over. Nothing in the source marks the difference, so
there is one kind of function and one set of libraries. This is the
model of Go and Erlang, and the one Java chose for its virtual threads
after a decade of reactive frameworks, citing goroutines and Erlang's
processes as the precedent.

These tests call the same function directly and from two tasks, on a
test clock that moves only when every task is waiting:

<!-- file: examples/explanation-concurrency/brew.sisu -->
```sisu
# One function with no `async`: called directly, it blocks its caller;
# spawned twice in a scope, the two calls wait at the same time.
def brew(clock: Clock, minutes: int) -> int ! TaskError:
    clock.sleep(Duration.minutes(minutes))?
    return Ok(minutes)

test "a direct call waits the whole time"(clock: TestClock):
    let start = clock.now()
    assert brew(clock, 10) == Ok(10)
    assert clock.now() - start == Duration.minutes(10)

test "two tasks wait side by side"(clock: TestClock):
    let start = clock.now()
    let total = task.scope do s:
        let first = s.spawn(() -> brew(clock, 10))
        let second = s.spawn(() -> brew(clock, 10))
        return Ok(first.join()? + second.join()?)
    assert total == Ok(20)
    assert clock.now() - start == Duration.minutes(10)
```

<!-- output: examples/explanation-concurrency/brew.test -->
```text
test "a direct call waits the whole time" ... ok
test "two tasks wait side by side" ... ok
2 passed, 0 failed, 0 blocked
```

`brew` is an ordinary function. Called directly, it holds up its caller
for ten minutes of the clock's time. Spawned twice, the two calls wait
together, so twenty minutes of brewing take ten. The function did not
change; only where it was called from did.

## Scopes as supervisors

A task can be started only inside a scope, and the scope does not end
until every task in it has ended. Trio's nurseries, Kotlin's coroutine
scopes, Swift's task groups, Java's structured task scope and Rust's
scoped threads reached this shape independently, which the research
counts as settled (finding 2). Sisu makes it the only form: there is no
`spawn_detached`. Work that lasts as long as the program lives in a
scope that lasts as long as the program, typically one opened in `main`,
so the program's tasks form a tree that follows the nesting of its code.

Three things follow from the rule. A task may borrow from the function
that started it, since the scope keeps that function's frame alive until
the task ends, so tasks share a list from that frame with no copy and no
reference count ([chapter
20](../tutorial/20-concurrency.md#what-a-task-may-borrow) shows it, and
[Ownership without lifetimes](ownership.md) explains why it needs no
lifetime). Nothing fails silently: when a task returns an error or
panics and nobody joins it, the scope cancels the other tasks, waits for
them, and returns the first failure, so no task's failure is lost and no
task outlives the scope. And a scope is a supervisor. A panic unwinds
its task to the scope, which sees `TaskPanicked` when it joins the task
(D-040), so restarting a worker is a loop around a `spawn` and a `match`
on the result. That is Erlang's supervision, which the research calls
the only way of handling errors with a decades-long record of
reliability (finding 6), written as ordinary code instead of as a
framework.

Data races are prevented by types, as in Rust. `Send` and `Sync` are
inferred from a type's fields, and checked only where a value crosses
into a task. Code that never spawns never meets them, which is Swift
6.2's lesson built in from the first line of the standard library rather
than retrofitted.

## Cancellation without tokens

Each scope has a cancellation flag. `s.cancel()` sets it for the scope
and everything nested in it, and every operation that blocks checks it
and returns `Err(Cancelled)`, which `?` passes on like any error. A
deadline is a task spawned with one, `s.spawn_with(deadline=...)`, and
`task.race` returns the first of several tasks to finish and cancels the
rest (D-075). Code that must finish, such as flushing a file, runs
inside `task.shield`.

No function takes a token. The current scope is ambient to the code
running in it, which makes it one of only two pieces of implicit context
in the language; the other is the scope's trace context, which `log`
attaches to each record. Neither reaches the world: nothing outside the
program can be touched through them. The design accepts implicit context
here, and refuses it for capabilities, because the alternative for
cancellation, Go's `ctx` threaded through every signature, is the
documented regret.

## The alternatives Sisu did not take

Stackless `async` has one real advantage, and the research names it: a
function compiled to a state machine needs no stack of its own and no
runtime, which suits embedded targets and WebAssembly without stack
switching (finding 1). Sisu answers those targets with runtime profiles
instead: `full` has the scheduler, `single` runs tasks on one thread for
WebAssembly and small tools, and `freestanding` has no scheduler, so
`task.scope` is a compile error there unless one is linked
([12](../../proposal/12-interop-and-targets.md)).

Zig's design passes an `Io` value and marks each concurrent call with
`io.async`. It is colourless, since the same function works under any
`Io`, and needs no scheduler in the language. Sisu takes half of it: the
value that reaches the world is a capability, as in Zig. It does not
take `io.async` at every concurrent call, because that moves the
colouring problem rather than removing it.

Actors as a language feature, as in Erlang, Pony and Swift, give each
actor its own state and a mailbox. Sisu writes an actor as a task that
owns its state and reads a channel, a library pattern, because Swift's
`actor` keyword and Pony's six reference capabilities showed that
building actors into the type system costs more than it gives for most
programs. Channels are bounded by default, since Erlang's unbounded
mailbox is its best documented failure in production (finding 7).

Parallel computation is a separate thing. Splitting a loop over eight
cores needs no waiting and no cancellation, so it is `std.par`, a
library that runs `pure` bodies on a pool of workers apart from the
tasks (D-069).

## What it costs

- **A runtime.** Colourless tasks need a scheduler. The profiles keep it
  out of programs that cannot have one, but a program on the full
  runtime carries it.
- **Stacks that cannot move.** Go grows a goroutine's stack by copying
  it, which needs a garbage collector that can fix every pointer into
  it, and Java's virtual threads copy frames into heap objects when they
  park. Sisu has no moving collector, so a task's stack stays where it
  was made. The runtime reserves stack memory and commits it only as it
  is used, and a task whose recursion runs past its stack panics with a
  message rather than corrupting memory. How large that reservation is,
  and what it costs per task, is the third of the proposal's bets.
- **Foreign calls that block.** A C function that blocks holds an
  operating-system thread, not a task. The runtime notices and starts
  another thread, and `@blocking` on the declaration says so in advance.
- **Cooperative scheduling.** Tasks switch at blocking points and at
  yield points that the compiler puts in loops (Q-34). A loop inside C
  code or `unsafe` code has no yield point and can keep its neighbours
  waiting.
- **No detached tasks.** Background work has to belong to a scope that
  lives long enough, which changes how a Go programmer structures a
  server.
- **Deadlock is still possible.** No mainstream type system prevents it.
  A `Mutex` that owns its data, locks that cannot outlive their frame
  and bounded channels make it rarer, and a lint for lock order is
  planned as tooling.

## What would change it

The third bet of the validation plan is that colourless tasks without a
moving collector can have acceptable stacks
([16](../../proposal/16-validation-plan.md), bet 3), with thresholds
fixed in advance
([20](../../proposal/20-pre-registration.md#24-bet-3-colourless-tasks-without-a-moving-gc-d-014-q-6)).
Its primary measure is memory per idle task, at 100,000 and at a million
tasks, against Go after its garbage collection: within four times Go's
holds the design. If every stackful strategy fails, task bodies are
compiled to state machines behind the same `scope` and `spawn`, so that
programs do not change, and the rule that a suspended task frame is
never leaked (D-058) must survive the move.

The harness of Phase 1, Rust code standing in for compiled Sisu, has run
that measurement ([its report](../../taskbench/docs/report-phase1.md)).
Pooled stacks with software probes, released while a task is parked,
stayed within 2.42 times Go's memory at a 1 MiB default reservation on
the two machines with 16 GiB, and within 1.76 times at 128 KiB. One
guard page per task stopped at about 32,730 tasks under Linux's default
limit on memory mappings, so guard pages per task are out. With yield
points every 10,000 loop iterations, the 99th percentile of a probe
task's latency under compute-bound neighbours stayed in microseconds,
where Go's was 39 milliseconds. And a million cancelled reads into stack
buffers, repeated 25 times, wrote no byte after the call that lent the
buffer had returned. These verdicts are provisional until the
pre-registration is frozen, and the guard against stacks overflowing in
ordinary code waits for the corpus, whose stack depths are measured in
the interpreter.

The questions that stay open follow the same measurements. Q-6 holds the
stack strategy and whether spawning should need a capability; the
harness's provisional answer keeps `task.scope` ambient, and the
research found no supply-chain attack that gating spawns would stop,
since attacks send data out rather than start tasks. Q-34 holds
preemption, to be revisited if starvation is measured. And the semantics
are tested by exploring schedules: any orphaned task, unobserved failure
or lock left held after a panic is a bug in the implementation, and a
class of them that needs a language change reopens D-014.
