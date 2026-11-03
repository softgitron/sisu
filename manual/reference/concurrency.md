# Concurrency

This page states the rules of tasks: how a scope starts them and waits
for them, what `join` gives, how failures, panics and cancellation move
through a scope, deadlines and races, what a task may borrow and which
values may cross to it, the types that share a value between tasks,
channels and `select`, the scheduler and its yield points, and `std.par`
for parallel computation over data.

**See also:** the tutorial's [chapter 20](../tutorial/20-concurrency.md);
the proposal's [06, Concurrency and
capabilities](../../proposal/06-concurrency-and-capabilities.md); the
compiler's [architecture notes](../../compiler/docs/architecture.md),
"Tasks and the scheduler" and "Send, Sync and what a task captures";
[R16](sisu-command.md#choosing-the-schedule) for the scheduler's
options; [R17, Diagnostics](diagnostics.md#codes-and-their-ranges) for
the codes. The types of `std.task` and `std.par` are in the standard
library's reference.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Structured concurrency without `async` is decided (D-014);
the library shapes of D-075 lean.

## Tasks and scopes

`task.scope do s:` opens a scope and runs its body with the scope `s`;
`s.spawn(closure)` or `s.spawn do:` starts a task on it and gives a
`Handle[T]`, where `T` is what the task returns.

- The scope waits for every task spawned on it before it ends, so no
  task outlives the frame that opened its scope, and there is no
  detached spawn.
- A scope and a handle are views of the frame: a scope's body cannot
  return either (E0430).
- The closures given to `task.scope`, `spawn` and the forms in the next
  sections are written at the call, as a closure or a `do` block, so
  that the checker sees what they capture (E0425).
- `task` is a module every module sees without an import
  ([R13](modules.md#names-every-module-sees)); a task needs no
  capability to start.

<!-- file: examples/reference-concurrency/scopes.sisu -->
```sisu
# A scope runs its tasks side by side and joins them before it ends.
def count_vowels(word: str) -> int:
    var n = 0
    for c in word.chars():
        if c in ['a', 'e', 'i', 'o', 'u', 'y', 'ä', 'ö']:
            n += 1
    return n

def main():
    let words = ["sauna", "löyly", "kiuas"]
    let total = task.scope do s:
        var handles: list[Handle[int]] = []
        for w in words:
            handles.push(s.spawn(() -> count_vowels(w)))
        var sum = 0
        for h in handles:
            let n = h.join() else: 0
            sum += n
        return sum
    print("{total} vowels")
```

<!-- output: examples/reference-concurrency/scopes.out -->
```text
9 vowels
```

**Stage 0 differs.** Tasks run on one interpreter loop, one at a time,
under the seeded scheduler of [The scheduler](#the-scheduler); 06's
M:N scheduler over operating-system threads is the `full` runtime
profile, which waits for the native runtime ([R22](../README.md#the-reference),
when it is written).

**Status.** In stage 0 (D-014).

## Joining a task

`h.join()` waits for the task and gives what it returned, as `T !
TaskError`; when the task returns `U ! E`, the sets flatten, so `join`
gives `U ! {E, TaskError}`. `TaskError` holds `Cancelled` and
`TaskPanicked(message, trace)`. A handle is joined once (R0077 for a
second `join`), and the type of a task's result must be known where it
is joined (E0432).

<!-- file: examples/reference-concurrency/results.sisu -->
```sisu
# `join` gives a task's result; its errors and `TaskError` flatten.
error ParseError:
    NotANumber(text: String)

def parse(text: str) -> int ! ParseError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def add(a: str, b: str) -> int ! {ParseError, TaskError}:
    return task.scope do s:
        let x = s.spawn(() -> parse(a))
        let y = s.spawn(() -> parse(b))
        return Ok(x.join()? + y.join()?)

def main():
    print(add("12", "30"))
    print(add("12", "kolme"))
```

<!-- output: examples/reference-concurrency/results.out -->
```text
Ok(42)
Err(NotANumber(text="kolme"))
```

**Status.** In stage 0.

## A scope's result and its failures

A task fails when it panics, or when it returns `Err` other than the
`Cancelled` of its own cancellation. Then:

- the first failure in time cancels the failing task's siblings;
- a failure that nobody joined becomes the scope's result, after every
  task has ended: the scope's type is the body's `T ! E` widened by
  `TaskError` and by the error sets of the tasks spawned on it;
- a failure that the result type cannot carry is a panic in the
  scope's task (R0079), so no failure is lost;
- a body that returns `Err` or panics cancels the tasks still running,
  and its error or panic is the scope's outcome once they end;
- the results nobody joined are dropped when the scope ends.

In a scope opened by `pure` code, a failure cancels only the tasks
spawned after it, and the failure reported is the one of the task
spawned first, so the result does not depend on the schedule.

<!-- file: examples/reference-concurrency/failure.sisu -->
```sisu
# The first task to fail cancels the others, and is the scope's result.
error ParseError:
    NotANumber(text: String)

def parse(text: str) -> int ! ParseError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def slow(clock: Clock) -> int ! TaskError:
    match clock.sleep(Duration.minutes(10)):
        case Ok(()):
            return Ok(10)
        case Err(e):
            print("the slow task stops: {e}")
            return Err(e)

def main(sys: System):
    let result = task.scope do s:
        s.spawn(() -> slow(sys.clock))
        s.spawn(() -> parse("kolme"))
        return Ok(())
    print(result)
```

<!-- output: examples/reference-concurrency/failure.out -->
```text
the slow task stops: Cancelled
Err(NotANumber(text="kolme"))
```

**Status.** In stage 0 (D-014).

## What a task may borrow

A task's closure captures as any closure does, and the checker then
decides how each capture reaches the task:

- a `Copy` value that the task does not change is copied into it;
- anything else is borrowed, which is allowed only for what the
  environment of the scope's body owns or borrows, since that outlives
  the scope: a local of the body itself is E0428, and a task spawned
  anywhere but in its scope's body, as on a scope passed to a helper,
  borrows nothing (E0429);
- an `own` task, `own () -> ...` or `do own:`, moves its captures in.

The loans a task holds last until its scope's body returns.

<!-- file: examples/reference-concurrency/borrow_local.sisu -->
```sisu
# A task cannot borrow a local of the scope's body.
def main():
    let names = ["Aino", "Eero"]
    task.scope do s:
        let greeting = "Hei".to_string()
        s.spawn do:
            print("{greeting}, {names[0]}")
```

<!-- output: examples/reference-concurrency/borrow_local.console -->
```console
$ sisu run borrow_local.sisu
error[E0428]: the task borrows `greeting`, which belongs to the scope's body: the body ends before the scope joins its tasks
  --> borrow_local.sisu:6:9
  |
6 |         s.spawn do:
  |         ^^^^^^^^^^^
  |                 --- borrowed here
  = note: move it into the task instead: write `own () -> ...` (or `do own:`), or copy what the task needs into a local before the `spawn`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

**Status.** In stage 0.

## Values that cross to a task

A value that moves or is copied into a task must be `Send`, and one the
task borrows must be `Sync`; one the task changes through a reference
must be `Send` (D-014; [R2](types.md#the-auto-traits)). The compiler
infers both from a type's parts:

| Type | `Send` | `Sync` |
|---|---|---|
| numbers, text, paths, times, capability handles | yes | yes |
| `Cell[T]` | when `T` is | never |
| `Mutex[T]`, `Atomic[T]`, a channel's half, a task's handle | when `T` is `Send` | when `T` is `Send` |
| `RwLock[T]` | when `T` is `Send` | when `T` is `Send` and `Sync` |
| `Shared[T]`, `Weak[T]`, `Slice[T]` | when `T` is `Sync` | when `T` is `Sync` |
| `ref T` | when `T` is `Sync` | when `T` is `Sync` |
| a mutex's guard | never | when `T` is `Sync` |
| a function or closure value | never | never |

A capture that is not `Send` is E0426, one shared by reference that is
not `Sync` E0427, one changed through a reference that is not `Send`
E0437, and a task's result that is not `Send` E0433. Each names the
field in the way and the wrapper that would help:

<!-- file: examples/reference-concurrency/not_sync.sisu -->
```sisu
# A `Cell` cannot be shared with another task by reference.
def main():
    let visits = Cell(0)
    task.scope do s:
        s.spawn do:
            visits.set(visits.get() + 1)
    print(visits.get())
```

<!-- output: examples/reference-concurrency/not_sync.console -->
```console
$ sisu run not_sync.sisu
error[E0427]: `visits` is shared with the task by reference, but `Cell[int]` is not `Sync`
  --> not_sync.sisu:5:9
  |
5 |         s.spawn do:
  |         ^^^^^^^^^^^
  = note: wrap it: `Mutex[int]` shares mutation between tasks; or move the value with an `own` task

error[D0006]: the program was not run: errors found (1)
```

A program that spawns no task never meets these rules.

**Status.** In stage 0.

## Cancellation

Each scope and each task has a cancellation flag. `s.cancel()` sets the
scope's, which cancels its tasks and the scopes nested in them; a task
is also cancelled when its scope is and when its deadline passes.

- Every blocking operation, `join`, `recv`, `send` on a full channel,
  `sleep`, a timer, `select`, I/O, checks the cancellation of the code
  that runs it first, and gives `Err(Cancelled)` when it is cancelled;
  `?` passes it on like any error. `Mutex.lock` and `RwLock`'s waits are
  not points of cancellation.
- `task.checkpoint()` is a blocking point that waits for nothing, for a
  long computation that wants to stop early, and `task.is_cancelled()`
  asks without stopping.
- `task.shield do:` runs its block where no outer cancellation is seen,
  for code that must finish, such as flushing a file.
- There is no cancellation token to pass: the scope is ambient to its
  tasks.

<!-- file: examples/reference-concurrency/cancel.sisu -->
```sisu
# Cancelling a scope stops its tasks at their next blocking point.
def heat(clock: Clock) -> ! TaskError:
    for _ in range(60):
        clock.sleep(Duration.minutes(1))?
    return Ok(())

def main(sys: System):
    task.scope do s:
        let heater = s.spawn(() -> heat(sys.clock))
        s.cancel()
        print("the heater: {heater.join()}")
        print("{task.is_cancelled()} {task.checkpoint()}")
        let shielded = task.shield do:
            return task.checkpoint()
        print("in a shield: {shielded}")
```

<!-- output: examples/reference-concurrency/cancel.out -->
```text
the heater: Err(Cancelled)
True Err(Cancelled)
in a shield: Ok(())
```

**Status.** In stage 0 (D-014, D-075).

## Deadlines and races

- `s.spawn_with(deadline=timer)` starts a task that is cancelled when
  the timer's moment passes on its clock.
- `task.race do r:` is a scope whose first task to finish wins, whatever
  its outcome: its result is the race's, flattened as `join` flattens
  it, and the others are cancelled and joined before the race returns.
  Every task of a race returns one type (E0431), and a race whose body
  spawns no task panics (R0078). Which task finishes first is the
  schedule, so `pure` code cannot race.

<!-- file: examples/reference-concurrency/deadlines.sisu -->
```sisu
# A deadline cancels a task that runs too long; a race keeps the first.
def answer(clock: Clock, seconds: int, name: str) -> String ! TaskError:
    clock.sleep(Duration.seconds(seconds))?
    return Ok(name.to_string())

test "a task past its deadline is cancelled"(clock: TestClock):
    let start = clock.now()
    let reply = task.scope do s:
        let deadline = clock.after(Duration.seconds(5))
        let slow = s.spawn_with(deadline=deadline) do:
            return answer(clock, 60, "slow")
        return slow.join()
    assert reply == Err(Cancelled)
    assert clock.now() - start == Duration.seconds(5)

test "the first to finish wins the race"(clock: TestClock):
    let reply = task.race do r:
        r.spawn(() -> answer(clock, 3, "replica"))
        r.spawn(() -> answer(clock, 1, "primary"))
    assert reply == Ok("primary")
```

<!-- output: examples/reference-concurrency/deadlines.test -->
```text
test "a task past its deadline is cancelled" ... ok
test "the first to finish wins the race" ... ok
2 passed, 0 failed, 0 blocked
```

**Status.** In stage 0 (D-075).

## Panics in tasks

A panic in a task unwinds the task, running its drops, to its scope,
which cancels the task's siblings and gives the joiner
`Err(TaskPanicked(message, trace))`. A panic in the scope's body cancels
and joins the tasks, then goes on. A supervisor is ordinary code: a
loop that spawns a worker, joins it, and decides from the error whether
to start it again.

<!-- file: examples/reference-concurrency/panics.sisu -->
```sisu
# A task's panic reaches the code that joins it as `TaskPanicked`.
def risky(n: int) -> int:
    if n > 2:
        panic("too many: {n}")
    return n

def main():
    task.scope do s:
        let fine = s.spawn(() -> risky(1))
        let broken = s.spawn(() -> risky(5))
        print(fine.join())
        match broken.join():
            case Err(TaskPanicked(message, ..)):
                print("the task panicked: {message}")
            case other:
                print("{other:?}")
```

<!-- output: examples/reference-concurrency/panics.out -->
```text
Ok(1)
the task panicked: too many: 5
```

**Status.** In stage 0 (D-014).

## Sharing a value that changes

A value that several tasks change lives in a type that owns it and
orders the changes:

| Type | Gives |
|---|---|
| `Mutex[T]` | `lock()`, a guard through which one task at a time reads and changes the value; `into_inner()` takes the value out |
| `RwLock[T]` (`std.task`) | `read()` for any number of readers, or `write()` for one writer |
| `Atomic[T]` | changes without a lock ([Atomics and orderings](#atomics-and-orderings)) |
| `task.Once` | `call_once(f)` runs `f` once among the tasks that share it |
| `task.Condvar` | `wait` lets go of a guard until another task calls `notify_one` or `notify_all` |

A guard unlocks when it is dropped. A task that asks again for a lock it
holds panics rather than wait for itself (R0073). When unwinding drops a
guard, the value's `invariant` is checked again: if it holds, or the type
has none, the lock unlocks as usual; if it fails, the lock is poisoned
and every later `lock` panics (R0074), as does `into_inner` (R0090)
(D-077).

<!-- file: examples/reference-concurrency/locks.sisu -->
```sisu
# A `Mutex` or an `RwLock` owns the value that tasks share.
from std.task import RwLock

def main():
    let words = ["sauna", "löyly", "kiuas"]
    let letters = Mutex(0)
    let menu = RwLock(["coffee".to_string()])
    task.scope do s:
        for word in words:
            s.spawn do:
                var guard = letters.lock()
                guard.value += len(word)
        s.spawn do:
            var guard = menu.write()
            guard.value.push("bun".to_string())
    print("{letters.into_inner()} letters, {menu.read().value}")
```

<!-- output: examples/reference-concurrency/locks.out -->
```text
15 letters, ["coffee", "bun"]
```

What a `Mutex`, an `RwLock` or an `Atomic` owns can be neither a view
(E0421) nor must-consume (E0422) ([R7](ownership.md#sharing-cells-and-graphs)).

**Status.** In stage 0 (D-077).

## Atomics and orderings

`Atomic[T]` holds a `Copy` value that tasks read and change without a
lock: `load`, `store`, `swap` and `compare_exchange`, and `fetch_add`
and `fetch_sub` for an `Atomic[int]`. Each takes a `MemoryOrder`, the
C11 model's `Relaxed`, `Acquire`, `Release`, `AcqRel` or `SeqCst`, which
is the default.

<!-- file: examples/reference-concurrency/atomics.sisu -->
```sisu
# An `Atomic` changes without a lock, with the ordering each call names.
def main():
    let bathers = Atomic(0)
    let lit = task.Once()
    task.scope do s:
        for _ in range(5):
            s.spawn do:
                lit.call_once(() -> print("the stove is lit, once"))
                _ = bathers.fetch_add(1, Relaxed)
    print("{bathers.load(Acquire)} bathers")
```

<!-- output: examples/reference-concurrency/atomics.out -->
```text
the stove is lit, once
5 bathers
```

**Stage 0 differs.** Stage 0 runs one task at a time, so no ordering
can be told apart from `SeqCst` there.

**Status.** In stage 0.

## Channels

`channel[T](capacity=n)` gives a `Sender[T]` and a `Receiver[T]` over a
queue of at most `n` values, `n` at least 1 (R0083):

- `tx.send(v)` waits while the queue is full, and gives `Err(Closed)`
  of `ChannelError` once the receiver is gone; `rx.recv()` waits while
  it is empty.
- `tx.close()`, or dropping every sender, closes the channel: the
  receiver gets what was sent, then `Closed`. `tx.copy()` makes another
  sender.
- `for x in rx` receives until the channel is closed and drained, or
  the task is cancelled.
- What a channel carries is neither a view nor must-consume.

<!-- file: examples/reference-concurrency/channels.sisu -->
```sisu
# A bounded channel carries values from the senders to the receiver.
def send_all(
    tx: Sender[String],
    names: list[str],
) -> ! {ChannelError, TaskError}:
    for name in names:
        tx.send(name.to_string())?
        print("sent {name}")
    tx.close()
    return Ok(())

def main():
    let tx, rx = channel[String](capacity=2)
    task.scope do s:
        s.spawn(() -> send_all(tx, ["Aino", "Eero", "Roni"]))
        for name in rx:
            print("welcome, {name}")
    print("everyone is in")
```

<!-- output: examples/reference-concurrency/channels.out -->
```text
sent Aino
sent Eero
welcome, Aino
welcome, Eero
sent Roni
welcome, Roni
everyone is in
```

**Stage 0 differs.** 06 has an unbounded channel as a separate
constructor; stage 0 has bounded ones only.

**Status.** In stage 0.

## Waiting for the first of several

`select:` is a statement whose arms are `case PATTERN = OPERATION:`,
and it runs the arm of the first operation that is ready: a `recv`, a
`send` or a `join` call, a `Timer` such as `clock.after(d)`, or a
`Cancellation` such as `s.cancelled()` (E0434 for anything else). Among
several ready arms the scheduler's seed chooses. When the code is
cancelled, a ready cancellation arm runs, or else the first arm whose
operation reports `Cancelled`.

<!-- file: examples/reference-concurrency/select.sisu -->
```sisu
# `select` waits for whichever of its arms is ready first.
def next_order(orders: Receiver[String], clock: Clock) -> String:
    select:
        case got = orders.recv():
            let order = got else:
                return "the counter is closed"
            return "order: {order}"
        case _ = clock.after(Duration.minutes(5)):
            return "no order in five minutes"

test "an order that is waiting"(clock: TestClock):
    let tx, rx = channel[String](capacity=1)
    tx.send("coffee".to_string()).expect("there is room")
    assert next_order(rx, clock) == "order: coffee"

test "no order comes"(clock: TestClock):
    let tx, rx = channel[String](capacity=1)
    assert next_order(rx, clock) == "no order in five minutes"
    _ = tx
```

<!-- output: examples/reference-concurrency/select.test -->
```text
test "an order that is waiting" ... ok
test "no order comes" ... ok
2 passed, 0 failed, 0 blocked
```

A test's clock moves only when it is told to or when every task waits,
so the test of the five minutes takes no time
([R15](testing.md#isolation-and-the-test-clock)).

**Status.** In stage 0.

## The scheduler

Tasks are scheduled cooperatively. A task runs until a yield point,
then the scheduler may run another:

- the yield points are the back edges of loops, calls of Sisu
  functions, spawns, `task.yield_now()` and every blocking operation;
- each task runs a budget of yield points that a generator seeded by
  `--schedule-seed N` draws, as it draws the next task, so a run is a
  function of the program, its input and the seed;
- when no task can run, the clocks of a test move to the earliest
  timer; with no timer either, the program is deadlocked, and it stops
  with a report of each task, what it waits for and where (R0080,
  R0081).

<!-- file: examples/reference-concurrency/schedules.sisu -->
```sisu
# Tasks that print without waiting for each other interleave by seed.
def call(name: str):
    for i in range(4):
        print("{name} {i}")
        task.yield_now()

def main():
    task.scope do s:
        s.spawn(() -> call("Aino"))
        s.spawn(() -> call("Eero"))
```

<!-- output: examples/reference-concurrency/schedules.console -->
```console
$ sisu run schedules.sisu
Eero 0
Eero 1
Eero 2
Eero 3
Aino 0
Aino 1
Aino 2
Aino 3
$ sisu run --schedule-seed 1 schedules.sisu
Aino 0
Aino 1
Aino 2
Eero 0
Aino 3
Eero 1
Eero 2
Eero 3
```

<!-- file: examples/reference-concurrency/deadlock.sisu -->
```sisu
# Every task waits and no timer can wake one: the run stops.
def main():
    let tx, rx = channel[int](capacity=1)
    task.scope do s:
        s.spawn do:
            print(rx.recv())
    print("never here {tx.send(1)}")
```

<!-- output: examples/reference-concurrency/deadlock.console -->
```console
$ sisu run deadlock.sisu
deadlock: every task is waiting, and no timer can wake any of them
  the root task waits for the end of a scope at deadlock.sisu:4:5
  task 1 waits for recv at deadlock.sisu:6:19
```

`--explore N` runs a program or its tests under many seeds and stops at
the first that fails; `--scheduler pct` and `exhaustive` search
schedules on purpose ([R16](sisu-command.md#choosing-the-schedule)).

Whether a task may also be preempted between these points is open
(Q-34).

**Status.** In stage 0.

## Logs from tasks

`log` writes a record to the standard error with its level and, for a
task, the task's path (`info [task 1.1]: ...` for the first task of the
first task), which stands in for the trace
context that D-075 gives a scope's tasks. `log` is ambient, as `task`
is.

**Status.** In stage 0; the trace context leans (D-075).

## Parallel computation over data

`std.par` runs `pure` code over data, apart from tasks (D-069). A body
cannot block and changes nothing shared, so removing the parallelism
gives the same result, bit for bit, on any number of workers.

- `par.map(xs, f)` gives `f` of each value, in order.
- `par.join(a, b)` runs two closures and gives both results;
  `par.join_all(fs)` runs a list of them.
- `par.reduce[R](xs, grain=n)` reduces with a `Reducer`, `par.Sum[T]`
  or `par.Collect[T]` or one of your own, over chunks fixed by index and
  merged pairwise up a fixed tree, so a sum is `sum`'s whatever the
  grain.
- `par.iter(xs)` iterates a list for a parallel loop.
- A failing parallel loop reports the lowest-index failure.

<!-- file: examples/reference-concurrency/parallel.sisu -->
```sisu
# `std.par` runs `pure` code over data; the result never depends on it.
import std.par

pure def square(n: ref int) -> int:
    return n * n

def main():
    let xs = [1, 2, 3, 4, 5]
    print(par.map(xs, square))
    let (low, high) = par.join(() -> sum([1, 2]), () -> sum([3, 4]))
    print("{low} {high}")
    print(par.reduce[par.Sum[f64]]([0.1, 0.2, 0.3], grain=1))
    var total = 0
    for x in par.iter(xs):
        total += x
    print(total)
```

<!-- output: examples/reference-concurrency/parallel.out -->
```text
[1, 4, 9, 16, 25]
3 7
0.6
15
```

**Stage 0 differs.** D-069 spells the parallel loop `xs.par_iter()` and
has `par_iter_mut()`; stage 0 has `par.iter(xs)` and `par.map` and no
`par_iter_mut`. It runs the chunks one after another on the task that
asks, with no worker pool (Q-6 is open), which a `pure` body cannot
tell apart.

**Status.** In stage 0, with the differences above (D-069).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| E0425, E0430 | `task.closure-literal`, `task.scope-escapes` | [Tasks and scopes](#tasks-and-scopes) |
| E0432, R0077 | `task.result-unknown`, `panic.joined-twice` | [Joining a task](#joining-a-task) |
| R0079, R0076 | `panic.unobserved-error`, `panic.scope-ended` | [A scope's result and its failures](#a-scopes-result-and-its-failures) |
| E0428, E0429 | `task.borrows-body-local`, `task.borrows-elsewhere` | [What a task may borrow](#what-a-task-may-borrow) |
| E0426, E0427, E0437, E0433 | `task.not-send`, `task.not-sync`, `task.not-send-mut`, `task.result-not-send` | [Values that cross to a task](#values-that-cross-to-a-task) |
| E0431, R0078 | `task.race-types`, `panic.empty-race` | [Deadlines and races](#deadlines-and-races) |
| R0073, R0074, R0090, E0421, E0422 | `panic.mutex-held`, `panic.mutex-poisoned`, `panic.into-inner-poisoned`, `borrow.view-in-owner`, `borrow.must-consume-in-owner` | [Sharing a value that changes](#sharing-a-value-that-changes) |
| R0083, R0091 | `panic.channel-capacity`, `panic.nested-recv` | [Channels](#channels) |
| E0434 | `task.select-arm` | [Waiting for the first of several](#waiting-for-the-first-of-several) |
| R0080, R0081 | `run.deadlock`, `run.deadlock-task` | [The scheduler](#the-scheduler) |
