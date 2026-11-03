# 20. Concurrency

A program that waits for the network, the disk or a timer can do other
work meanwhile. In Sisu that work runs in tasks, which a scope starts
and waits for. In this chapter you start tasks and collect their
results and errors, share values between them safely, pass values
along channels, wait for the first of several things, cancel work and
give it deadlines, and handle a task that panics. Last, you test all
of it under the seeded scheduler, which runs a test under many orders
of its tasks. You need about two and a half hours.

## Tasks in a scope

`task.scope` opens a scope, and `s.spawn` starts a task in it. A task
runs a function beside the code that started it, and the scope does
not end until every task in it has ended:

<!-- file: examples/20-concurrency/scope.sisu -->
```sisu
# A scope runs tasks side by side, and waits for every one of them.

## How many primes there are below `n`.
def primes_below(n: int) -> int:
    var count = 0
    for k in range(2, n):
        var d = 2
        while d * d <= k and k % d != 0:
            d += 1
        if d * d > k:
            count += 1
    return count

def main():
    task.scope do s:
        let small = s.spawn(() -> primes_below(1000))
        let large = s.spawn(() -> primes_below(5000))
        print("below 1000: {small.join()}")
        print("below 5000: {large.join()}")
    print("both tasks have ended")
```

<!-- output: examples/20-concurrency/scope.out -->
```text
below 1000: Ok(168)
below 5000: Ok(669)
both tasks have ended
```

- `task.scope do s:` runs the block with the scope `s`. `s.spawn(() ->
  primes_below(1000))` starts a task that runs the closure, and gives a
  handle to it.
- `small.join()` waits for the task to end and gives its result, as a
  result: `Ok(168)`, or an error when the task failed.
- When the block ends, the scope waits for any task still running.
  There is no way to start a task that outlives its scope, so no task is
  left running when the scope's line is done, and no failure goes
  unseen.

A task blocks only itself. While one waits for a file, a timer or
another task, the others run, and nothing in a function's signature
says whether it may wait: any function can be called from a task.

> **From Python.** There is no `async def` and no `await`. In Python's
> `asyncio`, a function that waits must be written `async`, and only
> other `async` functions can call it. In Sisu every function can wait,
> and tasks are where several waits overlap. A scope is closest to
> `asyncio.TaskGroup`, which also waits for every task it started.

## Results and errors from tasks

A task returns a value like any function, and `join` gives it back. The
error sets combine: joining a task whose function returns `int !
ParseError` gives `int ! {ParseError, TaskError}`, where `TaskError` is
what can go wrong with the task itself:

<!-- file: examples/20-concurrency/flatten.sisu -->
```sisu
# `join` gives a task's result, with its errors and `TaskError` together.
error ParseError:
    NotANumber(text: String)

def parse(text: str) -> int ! ParseError:
    let n = text.trim().parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def total(a: str, b: str) -> int ! {ParseError, TaskError}:
    return task.scope do s:
        let x = s.spawn(() -> parse(a))
        let y = s.spawn(() -> parse(b))
        return Ok(x.join()? + y.join()?)

def main():
    print(total("12", "30"))
    print(total("12", "kolme"))
```

<!-- output: examples/20-concurrency/flatten.out -->
```text
Ok(42)
Err(NotANumber(text="kolme"))
```

- `x.join()?` gives the number, or passes on the error, whether the
  parse failed or the task did.
- The scope's block returns a result, so the scope gives it, with the
  tasks' error sets added: `total` returns `int ! {ParseError,
  TaskError}`.
- `TaskError` has two variants: `Cancelled`, for a task stopped before
  it finished, and `TaskPanicked`, for a task that panicked. Later
  sections show both.

## When a task fails

A task fails when it returns an error or panics. The first failure
cancels the scope's other tasks, and when no code joins the failed
task, its failure becomes the scope's result:

<!-- file: examples/20-concurrency/failure.sisu -->
```sisu
# A task that fails cancels the others, and its failure is the scope's.
error ParseError:
    NotANumber(text: String)

def parse(text: str) -> int ! ParseError:
    let n = text.trim().parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def slow_count(clock: Clock) -> int ! TaskError:
    match clock.sleep(Duration.minutes(10)):
        case Ok(()):
            return Ok(10)
        case Err(e):
            print("the slow task stops: {e}")
            return Err(e)

def main(sys: System):
    let result = task.scope do s:
        s.spawn(() -> slow_count(sys.clock))
        s.spawn(() -> parse("kolme"))
        return Ok(())
    print(result)
```

<!-- output: examples/20-concurrency/failure.out -->
```text
the slow task stops: Cancelled
Err(NotANumber(text="kolme"))
```

The parse fails at once, and the scope cancels the slow task, which
was going to sleep for ten minutes: its `sleep` returns
`Err(Cancelled)` and it stops. The scope then gives the parse's error,
which nobody joined. A task that stops with `Cancelled` because its
scope cancelled it has not failed, so the scope's result is the
failure that started the cancelling.

## What a task may borrow

A task can borrow what the function around its scope owns, since the
scope ends, and its tasks with it, before that function returns:

<!-- file: examples/20-concurrency/borrow.sisu -->
```sisu
# A task may borrow what the frame around its scope owns.

## How many of `words` hold `letter`.
def count_with(words: list[str], letter: char) -> int:
    var n = 0
    for w in words:
        if w.contains("{letter}"):
            n += 1
    return n

def main():
    let words = ["sauna", "löyly", "vihta", "kiuas", "lauteet"]
    task.scope do s:
        let with_a = s.spawn(() -> count_with(words, 'a'))
        let with_y = s.spawn(() -> count_with(words, 'y'))
        print("{with_a.join()} with a, {with_y.join()} with y")
```

<!-- output: examples/20-concurrency/borrow.out -->
```text
Ok(4) with a, Ok(1) with y
```

`words` belongs to `main`, outside the scope's block, so both tasks
read it without a copy. *Löyly* is the steam of a sauna, *vihta* a
bundle of birch twigs, *kiuas* the stove and *lauteet* its benches.

**Try it.** Move `let words = ...` into the scope's block, before the
first `spawn`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/20-concurrency/body_local.console -->
```console
$ sisu run body_local.sisu
error[E0428]: the task borrows `words`, which belongs to the scope's body: the body ends before the scope joins its tasks
  --> body_local.sisu:14:22
   |
14 |         let with_a = s.spawn(() -> count_with(words, 'a'))
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |                              ---------------------------- borrowed here
  = note: move it into the task instead: write `own () -> ...` (or `do own:`), or copy what the task needs into a local before the `spawn`

error[E0428]: the task borrows `words`, which belongs to the scope's body: the body ends before the scope joins its tasks
  --> body_local.sisu:15:22
   |
15 |         let with_y = s.spawn(() -> count_with(words, 'y'))
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |                              ---------------------------- borrowed here
  = note: move it into the task instead: write `own () -> ...` (or `do own:`), or copy what the task needs into a local before the `spawn`

error[D0021]: the program was not run: the ownership checker found errors (2); `--defer-borrow` runs it under the monitor
```

E0428 (`task.borrows-body-local`) says that the block ends before the
scope waits for its tasks, so a task cannot borrow a local of the
block. Its note offers `own`, which moves `words` into the first task
and leaves none for the second. The fix here is the one the program
had: make the list outside the scope.

</details>

## Sharing a value that changes

Several tasks reading a value is safe. Several tasks changing one is
not, unless something makes them take turns. A `Mutex` holds a value
that one task at a time can change:

<!-- file: examples/20-concurrency/mutex.sisu -->
```sisu
# A `Mutex` lets tasks change one value, one task at a time.
def main():
    let words = ["sauna", "löyly", "vihta", "kiuas", "lauteet"]
    let letters = Mutex(0)
    task.scope do s:
        for w in words:
            s.spawn do:
                var guard = letters.lock()
                guard.value += len(w)
    print("{letters.into_inner()} letters")
```

<!-- output: examples/20-concurrency/mutex.out -->
```text
27 letters
```

- `letters.lock()` waits until no other task holds the mutex, and gives
  a guard; `guard.value` is the value, which the task can change while
  it holds the guard.
- The guard lets go of the mutex when it is dropped, at the end of the
  task's block here.
- `letters.into_inner()` takes the value out once the tasks are done.

**Try it.** Use a `Cell(0)`, the shared value of chapter 13, in place
of the mutex.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/20-concurrency/cell.console -->
```console
$ sisu run cell.sisu
error[E0427]: `letters` is shared with the task by reference, but `Cell[int]` is not `Sync`
  --> cell.sisu:7:13
  |
7 |             s.spawn do:
  |             ^^^^^^^^^^^
  = note: wrap it: `Mutex[int]` shares mutation between tasks; or move the value with an `own` task

error[D0006]: the program was not run: errors found (1)
```

E0427 (`task.not-sync`) says that the tasks share `letters` by
reference, and a `Cell[int]` is not `Sync`. Its note names the fix,
`Mutex[int]`.

</details>

`Sync` and `Send` are two traits that the compiler works out from a
type's parts. A `Sync` value may be borrowed by several tasks at once,
and a `Send` value may move to another task. Numbers, text, lists of
them and capabilities are both; a `Cell` is not `Sync`, since two
tasks could change it at the same time. You meet the two names only in
messages such as this one, which say which value is in the way and
what to wrap it in.

## Atomics, once, readers and writers

A mutex suits any value. Smaller jobs have smaller tools:

<!-- file: examples/20-concurrency/atomic.sisu -->
```sisu
# An `Atomic` changes without a lock; a `task.Once` runs a function once.
def main():
    let bathers = Atomic(0)
    let heating = task.Once()
    task.scope do s:
        for _ in range(5):
            s.spawn do:
                heating.call_once(() -> print("the stove is lit, once"))
                _ = bathers.fetch_add(1)
    print("{bathers.load()} bathers")
```

<!-- output: examples/20-concurrency/atomic.out -->
```text
the stove is lit, once
5 bathers
```

- `Atomic(0)` holds a `Copy` value, such as a number or a `bool`, that
  tasks change one step at a time without a lock: `fetch_add(1)` adds
  one and gives the old value, `load()` reads it, and `store`, `swap`
  and `compare_exchange` set it. Each takes a memory order of the C++
  model as its last argument, `SeqCst` unless you give another.
- `task.Once()` runs a function once, however many tasks call it:
  the first `call_once` runs it, and the others wait for it to finish
  and then run nothing.

Two more live in `task`. `task.RwLock(value)` lets many tasks read at
once with `read()`, or one task write with `write()`, for a value that
is read far more often than it changes. `task.Condvar()` lets a task
wait under a mutex until another task makes a condition true, with
`wait_while` and `notify_one` or `notify_all`.

## Channels

A channel carries values from tasks that send to tasks that receive.
It has room for a fixed number of values, so a sender that gets ahead
waits:

<!-- file: examples/20-concurrency/channel.sisu -->
```sisu
# A channel carries values from the tasks that send to those that receive.

## Sends each of `names`, then closes the channel.
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
        s.spawn(() -> send_all(tx, ["Aino", "Eero", "Roni", "Liisa"]))
        for name in rx:
            print("welcome, {name}")
    print("everyone is in")
```

**Predict.** The channel has room for two names. In what order do the
lines come out? Then compare:

<!-- output: examples/20-concurrency/channel.out -->
```text
sent Aino
sent Eero
welcome, Aino
welcome, Eero
sent Roni
sent Liisa
welcome, Roni
welcome, Liisa
everyone is in
```

- `channel[String](capacity=2)` gives a pair: a `Sender[String]` and a
  `Receiver[String]`.
- `tx.send(value)` waits while the channel is full, which is why two
  names are sent before the first is received. It fails with `Closed`
  when nobody can receive any more.
- `tx.close()` says that no more values come. `for name in rx:`
  receives until the channel is closed and empty.

What a channel carries must be owned, here a `String`: the receiver
may keep a value longer than anything it borrowed would live. When
every task waits and nothing can wake any of them, the program has
deadlocked, and Sisu stops it with a message that names what each task
waits for and where.

## Waiting for the first of several things

`select` waits for whichever of its arms is ready first: a value to
receive, room to send, a task to join, a timer, or a cancellation:

<!-- file: examples/20-concurrency/select.sisu -->
```sisu
# `select` waits for whichever of several things comes first.

## The next order from `orders`, or a note when none comes in time.
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
    let start = clock.now()
    assert next_order(rx, clock) == "no order in five minutes"
    assert clock.now() - start == Duration.minutes(5)
    _ = tx

test "the counter closes"(clock: TestClock):
    let tx, rx = channel[String](capacity=1)
    tx.close()
    assert next_order(rx, clock) == "the counter is closed"
```

<!-- output: examples/20-concurrency/select.test -->
```text
test "an order that is waiting" ... ok
test "no order comes" ... ok
test "the counter closes" ... ok
3 passed, 0 failed, 0 blocked
```

- `case got = orders.recv():` is ready when an order is waiting, or when
  the channel is closed, which `got` then says.
- `case _ = clock.after(Duration.minutes(5)):` is ready five minutes
  from now on that clock. Under the test clock of chapter 19, the
  second test waits five minutes in no time.
- When several arms are ready at once, the scheduler picks one.

A `case _ = s.cancelled():` arm is ready when the scope `s` is
cancelled, so that a task waiting in a `select` can stop cleanly.

## Cancellation

`s.cancel()` cancels a scope: every task in it, and the code of its
block, gets `Err(Cancelled)` from its next blocking operation, such as
`sleep`, `recv`, `send` or `join`:

<!-- file: examples/20-concurrency/cancel.sisu -->
```sisu
# Cancelling a scope stops its tasks at their next blocking point.

## Heats the stove for an hour, a minute at a time.
def heat(clock: Clock) -> ! TaskError:
    for _ in range(60):
        clock.sleep(Duration.minutes(1))?
    return Ok(())

def main(sys: System):
    task.scope do s:
        let heater = s.spawn(() -> heat(sys.clock))
        s.cancel()
        print("the heater: {heater.join()}")
        print("cancelled: {task.is_cancelled()}")
        print("a blocking point: {task.checkpoint()}")
        let shielded = task.shield do:
            return task.checkpoint()
        print("in a shield: {shielded}")
```

<!-- output: examples/20-concurrency/cancel.out -->
```text
the heater: Err(Cancelled)
cancelled: True
a blocking point: Err(Cancelled)
in a shield: Ok(())
```

- The heater was going to sleep sixty times, and the first `sleep`
  returns `Err(Cancelled)`, which `?` passes on.
- `task.is_cancelled()` asks, and `task.checkpoint()` is a blocking
  point that waits for nothing, for a long computation that wants to
  stop when it is cancelled.
- `task.shield do:` runs a block that cancellation does not reach, for
  work that must finish, such as writing a file to the end.

There is no cancellation token to pass around: a task is cancelled
through its scope, and the code it runs sees it at the next point
where it would wait.

## Deadlines and races

A task can have a deadline, and a race keeps only the first task to
finish:

<!-- file: examples/20-concurrency/race.sisu -->
```sisu
# A deadline cancels a task that runs too long; a race keeps the first.

## The answer `name` gives after `seconds` on `clock`.
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
    let start = clock.now()
    let reply = task.race do r:
        r.spawn(() -> answer(clock, 3, "replica"))
        r.spawn(() -> answer(clock, 1, "primary"))
    assert reply == Ok("primary")
    assert clock.now() - start == Duration.seconds(1)
```

<!-- output: examples/20-concurrency/race.test -->
```text
test "a task past its deadline is cancelled" ... ok
test "the first to finish wins the race" ... ok
2 passed, 0 failed, 0 blocked
```

- `s.spawn_with(deadline=timer)` starts a task that is cancelled when
  the timer comes, here five seconds from the start. The task, which
  would have answered after sixty, gives `Err(Cancelled)`.
- `task.race do r:` is a scope whose result is the first of its tasks
  to finish, whatever its outcome: the primary answers after one
  second, and the replica, still waiting, is cancelled. Every task of
  a race returns the same type.

A failure that finishes first wins a race too, so a race between a
request and a timer that returns `Err(TimedOut)` is a timeout, as
exercise 20.5 shows.

## Panics in tasks

A task that panics unwinds to its scope, and the task that joins it
gets `Err(TaskPanicked(message, trace))`. So a task can watch another
and start it again, which is all a supervisor is:

<!-- file: examples/20-concurrency/supervisor.sisu -->
```sisu
# A task that panics fails, and the task that joins it can start another.
def stove(round: int) -> int:
    if round < 2:
        panic("the stove went out in round {round}")
    return 80 + round

def main():
    var round = 0
    task.scope do s:
        while True:
            let r = round
            match s.spawn(() -> stove(r)).join():
                case Ok(degrees):
                    print("round {r}: {degrees} degrees")
                    break
                case Err(TaskPanicked(message, _)):
                    print("restarting after: {message}")
                    round += 1
                case Err(e):
                    print("giving up: {e}")
                    break
```

<!-- output: examples/20-concurrency/supervisor.out -->
```text
restarting after: the stove went out in round 0
restarting after: the stove went out in round 1
round 2: 82 degrees
```

The stove panics in rounds 0 and 1, and the loop starts a new one each
time, until round 2 gives its heat. A panic that no task joins becomes
the scope's result, as `Err(TaskPanicked(...))`, when the scope's type
has room for it, and otherwise goes on in the task that opened the
scope, so a bug is never lost in a task nobody watched.

## A lock that a panic leaves behind

A task that panics while it holds a mutex may leave the value half
changed. When the guard is dropped by the panic, Sisu checks the
value's `invariant`, a condition that every value of the type must
keep (chapter 23 covers invariants). If it holds, the mutex is
unlocked as usual; if not, the mutex is poisoned, and every later
`lock` panics (D-077):

<!-- file: examples/20-concurrency/poison.sisu -->
```sisu
# A guard that a panic drops checks the value's invariant (D-077).
struct Tank:
    litres: int
    invariant self.litres >= 0

    ## Pours out `litres`, and panics when a sensor fails halfway.
    def pour(mut self, litres: int, sensor_fails: bool):
        self.litres -= litres
        if sensor_fails:
            panic("the sensor failed")

def pour_in_a_task(tank: Mutex[Tank], litres: int):
    task.scope do s:
        let pouring = s.spawn do:
            var guard = tank.lock()
            guard.pour(litres, True)
        print("pouring {litres}: {pouring.join().is_err()}")
    print("poisoned: {tank.is_poisoned()}")

def main():
    let water = Mutex(Tank(litres=10))
    pour_in_a_task(water, 4)
    print("left: {water.lock().litres}")
    let more = Mutex(Tank(litres=10))
    pour_in_a_task(more, 15)
    print("left: {more.lock().litres}")
```

<!-- output: examples/20-concurrency/poison.console -->
```console
$ sisu run poison.sisu
pouring 4: True
poisoned: False
left: 6
pouring 15: True
poisoned: True
panic: the lock is poisoned: a task panicked holding it and left its value's invariant broken
  at main (poison.sisu:26:19)
```

- Pouring 4 of 10 litres leaves 6, which keeps the invariant, so the
  mutex is not poisoned, and `main` reads what is left.
- Pouring 15 leaves -5, which breaks it, so the mutex is poisoned, and
  the `lock` on the last line panics.

A supervisor can thus restart a worker that panicked and know that the
value it shares is still sound, or that it is not.

## Logs from tasks

`log` writes a record to standard error. A record written in a task
names the task by its path from the program's first task:

<!-- file: examples/20-concurrency/logs.sisu -->
```sisu
# A log record from a task names the task, and its children name it too.
def main():
    log.info("opening the sauna")
    task.scope do s:
        s.spawn do:
            log.info("heating the stove")
            task.scope do inner:
                inner.spawn(() -> log.warn("the stove is hot"))
        s.spawn(() -> log.info("carrying the water"))
    log.info("the sauna is ready")
```

<!-- output: examples/20-concurrency/logs.console -->
```console
$ sisu run logs.sisu
info: opening the sauna
info [task 2]: carrying the water
info [task 1]: heating the stove
warn [task 1.1]: the stove is hot
info: the sauna is ready
```

`[task 1]` is the first task spawned, `[task 2]` the second, and
`[task 1.1]` the first task spawned inside task 1. The records of one
piece of work keep its path however the tasks interleave, so you can
pick them out of a busy log.

## Testing under many schedules

The tasks of a program take turns in an order that the scheduler
chooses. In Sisu the scheduler is seeded: a run with the same seed
takes the same turns, so a test gives the same result every time. A
bug that shows up only in some orders needs a test run under many
seeds. This one loses an update when the two tasks interleave:

<!-- file: examples/20-concurrency/lost.sisu -->
```sisu
# A lost update: each task reads the count, gives way, and writes.

## Adds a visit to `visits`, the slow way: read, give way, write.
def add_visit(visits: Mutex[int]):
    let seen = visits.lock().value
    task.yield_now()
    visits.lock().value = seen + 1

test "two visits make two":
    let visits = Mutex(0)
    task.scope do s:
        s.spawn(() -> add_visit(visits))
        s.spawn(() -> add_visit(visits))
    let total = visits.into_inner()
    assert total == 2
```

`task.yield_now()` lets another task run, as a file read or any other
wait between the read and the write would:

<!-- output: examples/20-concurrency/lost.console -->
```console
$ sisu test lost.sisu
test "two visits make two" ... ok
1 passed, 0 failed, 0 blocked
$ sisu test --explore 20 lost.sisu
test "two visits make two" ... FAILED

test "two visits make two" ... FAILED
with schedule seed 2; `--schedule-seed 2` runs it again
panic: assertion failed: total == 2
    total == 2
    │
    1
  at test "two visits make two" (lost.sisu:15:5)
replay: sisu test --replay 1-9c7361fd0bdc4ec9-0.0.1-t0-s2-r0 lost.sisu

0 passed, 1 failed, 0 blocked
explored 20 schedules per test (seeds 0 to 19)
$ sisu test --scheduler pct --depth 2 --explore 10 lost.sisu
test "two visits make two" ... FAILED

test "two visits make two" ... FAILED
with schedule seed 0 of a PCT scheduler of depth 2; `--scheduler pct --depth 2 --schedule-seed 0` runs it again
panic: assertion failed: total == 2
    total == 2
    │
    1
  at test "two visits make two" (lost.sisu:15:5)
replay: sisu test --replay 1-9c7361fd0bdc4ec9-0.0.1-t0-s0-r0-p2 lost.sisu

0 passed, 1 failed, 0 blocked
explored 10 schedules of a PCT scheduler of depth 2 per test (seeds 0 to 9)
```

- Under the default seed, 0, the test passes: the tasks happen not to
  interleave between the read and the write.
- `--explore 20` runs the test under the seeds 0 to 19, and seed 2
  loses an update. `--schedule-seed 2` runs that order again, and so
  does the `replay:` command.
- `--scheduler pct --depth 2` uses a scheduler that is better at
  finding bugs that need a few switches at the right places, and finds
  this one under its first seed. `--scheduler exhaustive --depth N`
  tries every order with at most N switches.

**Try it.** Hold the lock from the read to the write: take the guard
with `var guard = visits.lock()`, give way, and add one to
`guard.value`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/20-concurrency/lost_fixed.console -->
```console
$ sisu test --explore 20 lost_fixed.sisu
test "two visits make two" ... ok
1 passed, 0 failed, 0 blocked
explored 20 schedules per test (seeds 0 to 19)
$ sisu test --scheduler exhaustive --depth 2 lost_fixed.sisu
test "two visits make two" ... ok
1 passed, 0 failed, 0 blocked
searched 32 schedules (preemption bound 2)
```

The second task now waits at `lock` until the first lets go, and no
order of the two loses an update: every one of the 32 orders with at
most two switches was tried.

</details>

## Exercise 20.1: predict the scopes

Open
[`manual/exercises/20-concurrency/predict_scopes.sisu`](../exercises/20-concurrency/predict_scopes.sisu):

<!-- file: exercises/20-concurrency/predict_scopes.sisu -->
```sisu
##! Exercise 20.1, predict the scopes. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of what a function gives, written as
##! `print` would write it. Write each as a literal, then run
##! `sisu test predict_scopes.sisu`. The hints are in chapter 20 of the
##! tutorial, under "Exercise 20.1: predict the scopes".

error OddError:
    Odd(n: int)

## Half of `n`, which must be even.
def half(n: int) -> int ! OddError:
    if n % 2 != 0:
        return Err(Odd(n))
    return Ok(n // 2)

## The halves of `a` and `b`, worked out by two tasks.
def halves(a: int, b: int) -> (int, int) ! {OddError, TaskError}:
    return task.scope do s:
        let x = s.spawn(() -> half(a))
        let y = s.spawn(() -> half(b))
        return Ok((x.join()?, y.join()?))

## How many tasks it started, one for each number, which nobody joins.
def started(numbers: list[int]) -> int ! {OddError, TaskError}:
    return task.scope do s:
        for n in numbers:
            s.spawn(() -> half(n))
        return Ok(len(numbers))

## What a task that waits gives when its scope is cancelled.
def cancelled(clock: Clock) -> String:
    return task.scope do s:
        let waiting = s.spawn(() -> clock.sleep(Duration.seconds(10)))
        s.cancel()
        return "{waiting.join()}"

const BOTH_EVEN: str = todo("halves(10, 4)")
const ONE_ODD: str = todo("halves(10, 3)")
const ALL_STARTED: str = todo("started([2, 4, 6])")
const ONE_FAILS: str = todo("started([2, 5, 6])")
const CANCELLED: str = todo("cancelled(clock)")

test "two even numbers":
    assert "{halves(10, 4)}" == BOTH_EVEN

test "an odd number":
    assert "{halves(10, 3)}" == ONE_ODD

test "tasks that nobody joins":
    assert "{started([2, 4, 6])}" == ALL_STARTED

test "a task that fails, which nobody joins":
    assert "{started([2, 5, 6])}" == ONE_FAILS

test "a cancelled task"(clock: TestClock):
    assert cancelled(clock) == CANCELLED
```

<!-- output: solutions/20-concurrency/predict_scopes.console -->
```console
$ sisu test predict_scopes.sisu
test "two even numbers" ... blocked by todo("halves(10, 4)")
test "an odd number" ... blocked by todo("halves(10, 3)")
test "tasks that nobody joins" ... blocked by todo("started([2, 4, 6])")
test "a task that fails, which nobody joins" ... blocked by todo("started([2, 5, 6])")
test "a cancelled task" ... blocked by todo("cancelled(clock)")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

`join()?` gives a task's value or passes on its error. A result prints
as `Ok(...)` or `Err(...)`, and a tuple in parentheses.

</details>

<details>
<summary>Hint 2</summary>

A task that fails, and that nobody joins, gives the scope its failure.
A task that is waiting when its scope is cancelled stops at that wait.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/20-concurrency/predict_scopes.sisu`](../solutions/20-concurrency/predict_scopes.sisu).

## Exercise 20.2: a count that tasks share

Open
[`manual/exercises/20-concurrency/long_words.sisu`](../exercises/20-concurrency/long_words.sisu):

<!-- file: exercises/20-concurrency/long_words.sisu -->
```sisu
##! Exercise 20.2, a count that tasks share. Kind: fix. Level: 1.
##!
##! `count_long` counts the long words of a list, with a task for each
##! word. The file does not compile. Run `sisu test long_words.sisu`,
##! read the diagnostic, and fix `count_long`, not the tests, so that
##! every test passes. The hints are in chapter 20 of the tutorial, under
##! "Exercise 20.2: a count that tasks share".

## How many of `words` have more than five letters, counted by a task
## for each word.
def count_long(words: list[str]) -> int:
    let count = Cell(0)
    task.scope do s:
        for word in words:
            s.spawn do:
                if len(word) > 5:
                    count.set(count.get() + 1)
    return count.get()

test "long words":
    assert count_long(["sauna", "lauteet", "kiulu", "saunavihta"]) == 2
    assert count_long(["löylyhuone"]) == 1

test "no words":
    assert count_long([]) == 0
```

<!-- output: solutions/20-concurrency/long_words.console -->
```console
$ sisu test long_words.sisu
error[E0427]: `count` is shared with the task by reference, but `Cell[int]` is not `Sync`
  --> long_words.sisu:15:13
   |
15 |             s.spawn do:
   |             ^^^^^^^^^^^
  = note: wrap it: `Mutex[int]` shares mutation between tasks; or move the value with an `own` task

```

<details>
<summary>Hint 1</summary>

The note of E0427 names the type to use in place of the `Cell`.

</details>

<details>
<summary>Hint 2</summary>

A mutex's value is changed through the guard that `lock()` gives, and
taken out at the end with `into_inner()`, as in the section "Sharing a
value that changes".

</details>

When every test passes, compare your answer with
[`manual/solutions/20-concurrency/long_words.sisu`](../solutions/20-concurrency/long_words.sisu).

## Exercise 20.3: a pool of workers

Open
[`manual/exercises/20-concurrency/pool.sisu`](../exercises/20-concurrency/pool.sisu):

<!-- file: exercises/20-concurrency/pool.sisu -->
```sisu
##! Exercise 20.3, a pool of workers. Kind: write. Level: 2.
##!
##! A sauna's bookings are checked one at a time, and each check takes a
##! second. `check_all` hands the bookings over a channel to `workers`
##! tasks, which check them side by side, and gives the bookings that
##! pass in alphabetical order. Write its body so that every test passes.
##! The tests run on the test clock, so they see how long the checks
##! took. Run them with `sisu test pool.sisu`. The hints are in chapter 20
##! of the tutorial, under "Exercise 20.3: a pool of workers".

## Whether `booking` names its hour; the check takes a second.
def check(clock: Clock, booking: str) -> bool ! TaskError:
    clock.sleep(Duration.seconds(1))?
    return Ok(not booking.contains("?"))

## The bookings that `check` passes, in alphabetical order, checked by
## `workers` tasks that take them from a channel.
def check_all(clock: Clock, bookings: list[str], workers: int) -> list[String]:
    return todo("check_all")

## A week of bookings, one of which has no hour.
def week() -> list[str]:
    return ["mon 18 Aino", "tue ? Eero", "wed 19 Roni", "thu 17 Liisa"]

test "the bookings that pass"(clock: TestClock):
    let passed = check_all(clock, week(), 2)
    assert passed == ["mon 18 Aino", "thu 17 Liisa", "wed 19 Roni"]

test "two workers take half the time of one"(clock: TestClock):
    let start = clock.now()
    _ = check_all(clock, week(), 2)
    assert clock.now() - start == Duration.seconds(2)

test "one worker checks one booking at a time"(clock: TestClock):
    let start = clock.now()
    _ = check_all(clock, week(), 1)
    assert clock.now() - start == Duration.seconds(4)

test "a worker for each booking"(clock: TestClock):
    let start = clock.now()
    assert len(check_all(clock, week(), 4)) == 3
    assert clock.now() - start == Duration.seconds(1)
```

<!-- output: solutions/20-concurrency/pool.console -->
```console
$ sisu test pool.sisu
in scope at the typed hole `todo("check_all")`:
  bookings = ["mon 18 Aino", "tue ? Eero", "wed 19 Roni", "thu 17 Liisa"]
  workers = 2
in scope at the typed hole `todo("check_all")`:
  bookings = ["mon 18 Aino", "tue ? Eero", "wed 19 Roni", "thu 17 Liisa"]
  workers = 2
in scope at the typed hole `todo("check_all")`:
  bookings = ["mon 18 Aino", "tue ? Eero", "wed 19 Roni", "thu 17 Liisa"]
  workers = 1
in scope at the typed hole `todo("check_all")`:
  bookings = ["mon 18 Aino", "tue ? Eero", "wed 19 Roni", "thu 17 Liisa"]
  workers = 4
test "the bookings that pass" ... blocked by todo("check_all")
test "two workers take half the time of one" ... blocked by todo("check_all")
test "one worker checks one booking at a time" ... blocked by todo("check_all")
test "a worker for each booking" ... blocked by todo("check_all")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Start `workers` tasks, each looping over the receiving end of one
channel and checking what it receives. The scope's block sends the
bookings, then closes the channel, so that the workers' loops end.

</details>

<details>
<summary>Hint 2</summary>

The workers add the bookings that pass to a list that they share,
which needs a `Mutex`. Sort it once the scope has ended. A check can
fail only when its task is cancelled, which nothing does here.

</details>

When every test passes, compare your answer with
[`manual/solutions/20-concurrency/pool.sisu`](../solutions/20-concurrency/pool.sisu).

## Exercise 20.4: a fan-out to a slow service

Open
[`manual/exercises/20-concurrency/fan_out.sisu`](../exercises/20-concurrency/fan_out.sisu):

<!-- file: exercises/20-concurrency/fan_out.sisu -->
```sisu
##! Exercise 20.4, a fan-out to a slow service. Kind: write. Level: 2.
##!
##! A weather service takes two seconds to answer. `forecasts` asks it
##! about several towns at once, with a task for each town, and gives the
##! answers in the order of the towns. Write its body so that every test
##! passes. The tests give it a fake service on the test clock, so they
##! see whether the questions were asked at once. Run them with
##! `sisu test fan_out.sisu`. The hints are in chapter 20 of the tutorial,
##! under "Exercise 20.4: a fan-out to a slow service".

error WeatherError:
    UnknownTown(name: String)

trait Weather:
    def forecast(self, town: str) -> String ! {WeatherError, TaskError}

## A fake service that answers after `delay` on `clock`, for the towns
## of `known`.
struct SlowWeather:
    clock: Clock
    delay: Duration
    known: dict[String, String]

impl Weather for SlowWeather:
    def forecast(self, town: str) -> String ! {WeatherError, TaskError}:
        self.clock.sleep(self.delay)?
        let answer = self.known.get(town.to_string()) else:
            return Err(UnknownTown(town.to_string()))
        return Ok(answer.copy())

## The forecasts for `towns`, in their order, asked of `service` at once.
def forecasts[W: Weather + Sync](
    service: W,
    towns: list[str],
) -> list[String] ! {WeatherError, TaskError}:
    return todo("forecasts")

## A service that knows three towns and takes two seconds to answer.
def service(clock: Clock) -> SlowWeather:
    let known: dict[String, String] = {
        "Oulu": "snow",
        "Turku": "rain",
        "Kemi": "sun",
    }
    return SlowWeather(clock=clock, delay=Duration.seconds(2), known=known)

test "the answers in the order of the towns"(clock: TestClock):
    let got = forecasts(service(clock), ["Turku", "Oulu", "Kemi"])
    assert got == Ok(["rain", "snow", "sun"])

test "every town at once"(clock: TestClock):
    let start = clock.now()
    _ = forecasts(service(clock), ["Turku", "Oulu", "Kemi"])
    assert clock.now() - start == Duration.seconds(2)

test "an unknown town"(clock: TestClock):
    let got = forecasts(service(clock), ["Atlantis", "Oulu"])
    assert got == Err(UnknownTown("Atlantis"))

test "no towns"(clock: TestClock):
    assert forecasts(service(clock), []) == Ok([])
```

<!-- output: solutions/20-concurrency/fan_out.console -->
```console
$ sisu test fan_out.sisu
in scope at the typed hole `todo("forecasts")`:
  towns = ["Turku", "Oulu", "Kemi"]
in scope at the typed hole `todo("forecasts")`:
  towns = ["Turku", "Oulu", "Kemi"]
in scope at the typed hole `todo("forecasts")`:
  towns = ["Atlantis", "Oulu"]
in scope at the typed hole `todo("forecasts")`:
  towns = []
test "the answers in the order of the towns" ... blocked by todo("forecasts")
test "every town at once" ... blocked by todo("forecasts")
test "an unknown town" ... blocked by todo("forecasts")
test "no towns" ... blocked by todo("forecasts")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Start every task before joining any, or each question waits for the
answer to the one before it, and the second test sees six seconds.

</details>

<details>
<summary>Hint 2</summary>

A list of handles, one for each town, keeps the order of the towns.
Joining them in that order with `?` gives the answers in order and
passes the first error on.

</details>

When every test passes, compare your answer with
[`manual/solutions/20-concurrency/fan_out.sisu`](../solutions/20-concurrency/fan_out.sisu).

## Exercise 20.5: a race against the clock

Open
[`manual/exercises/20-concurrency/first_price.sisu`](../exercises/20-concurrency/first_price.sisu):

<!-- file: exercises/20-concurrency/first_price.sisu -->
```sisu
##! Exercise 20.5, a race against the clock. Kind: write. Level: 3.
##!
##! A price service runs as two copies, a primary and a replica, and
##! either may be slow. `first_price` asks both at once and takes the
##! first answer, and gives up with `TimedOut` when neither answers within
##! `limit`. An error from a copy is an answer too. Write its body so that
##! every test passes. The tests give it fakes on the test clock. Run
##! them with `sisu test first_price.sisu`. The hints are in chapter 20 of
##! the tutorial, under "Exercise 20.5: a race against the clock".

error PriceError:
    TimedOut
    NoSuchItem(name: String)

trait Prices:
    def price(self, item: str) -> int ! {PriceError, TaskError}

## A fake copy of the service, which answers after `delay` on `clock`
## and knows the price of coffee only.
struct SlowPrices:
    clock: Clock
    delay: Duration

impl Prices for SlowPrices:
    def price(self, item: str) -> int ! {PriceError, TaskError}:
        self.clock.sleep(self.delay)?
        if item == "coffee":
            return Ok(350)
        return Err(NoSuchItem(item.to_string()))

## The price of `item` from whichever of `primary` and `replica` answers
## first, or `TimedOut` when neither answers within `limit` on `clock`.
def first_price[P: Prices + Sync](
    primary: P,
    replica: P,
    item: str,
    clock: Clock,
    limit: Duration,
) -> int ! {PriceError, TaskError}:
    return todo("first_price")

## A copy of the service that answers after `seconds`.
def after(clock: Clock, seconds: int) -> SlowPrices:
    return SlowPrices(clock=clock, delay=Duration.seconds(seconds))

test "the primary answers first"(clock: TestClock):
    let start = clock.now()
    let five = Duration.seconds(5)
    let p = first_price(after(clock, 1), after(clock, 3), "coffee", clock, five)
    assert p == Ok(350)
    assert clock.now() - start == Duration.seconds(1)

test "the replica answers first"(clock: TestClock):
    let start = clock.now()
    let five = Duration.seconds(5)
    let p = first_price(after(clock, 4), after(clock, 2), "coffee", clock, five)
    assert p == Ok(350)
    assert clock.now() - start == Duration.seconds(2)

test "neither answers in time"(clock: TestClock):
    let start = clock.now()
    let five = Duration.seconds(5)
    let p = first_price(after(clock, 8), after(clock, 9), "coffee", clock, five)
    assert p == Err(TimedOut)
    assert clock.now() - start == Duration.seconds(5)

test "an error is an answer too"(clock: TestClock):
    let five = Duration.seconds(5)
    let p = first_price(after(clock, 1), after(clock, 2), "tea", clock, five)
    assert p == Err(NoSuchItem("tea"))
```

<!-- output: solutions/20-concurrency/first_price.console -->
```console
$ sisu test first_price.sisu
in scope at the typed hole `todo("first_price")`:
  item = "coffee"
  limit = Duration(nanos=5000000000)
in scope at the typed hole `todo("first_price")`:
  item = "coffee"
  limit = Duration(nanos=5000000000)
in scope at the typed hole `todo("first_price")`:
  item = "coffee"
  limit = Duration(nanos=5000000000)
in scope at the typed hole `todo("first_price")`:
  item = "tea"
  limit = Duration(nanos=5000000000)
test "the primary answers first" ... blocked by todo("first_price")
test "the replica answers first" ... blocked by todo("first_price")
test "neither answers in time" ... blocked by todo("first_price")
test "an error is an answer too" ... blocked by todo("first_price")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Three things race: the primary, the replica and the clock. The section
"Deadlines and races" shows a race of two.

</details>

<details>
<summary>Hint 2</summary>

Every task of a race returns the same type. A small function that
sleeps for `limit` and then returns `Err(TimedOut)` has the type that
`price` has.

</details>

When every test passes, compare your answer with
[`manual/solutions/20-concurrency/first_price.sisu`](../solutions/20-concurrency/first_price.sisu).

## What you learned

- `task.scope do s:` opens a scope and `s.spawn` starts a task in it;
  the scope waits for every task, so none outlives it.
- `join` gives a task's result, its error sets joined with `TaskError`.
  The first failure cancels the other tasks, and a failure nobody
  joined is the scope's result.
- A task may borrow what the function around the scope owns (E0428
  otherwise). Values that tasks change go in a `Mutex`, an `Atomic`, or
  the `RwLock`, `Once` and `Condvar` of `task`; a `Cell` is not `Sync`
  (E0427).
- A bounded channel carries owned values; `close` ends the receiver's
  loop. `select` waits for the first ready arm, timers and cancellation
  included.
- `s.cancel()` makes blocking operations give `Err(Cancelled)`;
  `task.shield` holds it off; `spawn_with(deadline=...)` and
  `task.race` limit how long work runs.
- A panic in a task reaches its joiner as `TaskPanicked`; a mutex whose
  invariant a panic broke is poisoned (D-077).
- `log` names each task by its path. The scheduler is seeded, and
  `--explore`, `--scheduler pct` and `--scheduler exhaustive` run a test
  under many orders.

Chapter 21 tours the standard library: files and streams, time,
randomness, the environment and other programs, JSON, regular
expressions, collections, hashing, encodings, sorting and searching,
and logging.
