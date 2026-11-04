# Find a concurrency bug

This guide finds a bug that shows up only in some orders of a
program's tasks: it writes a test of what must hold in every order,
runs it under many orders, replays the one that fails, checks the fix
against every order up to a bound, and adds failures of the world with
the simulator. It assumes tasks, scopes and `Mutex`, from chapter 20 of
the tutorial.

**See also:** the tutorial's
[chapter 20](../tutorial/20-concurrency.md#testing-under-many-schedules)
(schedules) and
[chapter 22](../tutorial/22-web-service.md#faults-from-the-simulator)
(faults); [R16](../reference/sisu-command.md) for the options of `sisu
test`; the proposal's
[17, "Simulation and replay"](../../proposal/17-testing.md#simulation-and-replay).

## 1. Test what must hold in every order

The scheduler is seeded, so a test runs its tasks in the same order
every time, and an order that loses data can go unseen for ever. Write
the test so that it asserts what must be true whatever the order. Here
two guests book the same hour at once, and exactly one of them may get
it:

<!-- file: examples/guide-concurrency-bug/double.sisu -->
```sisu
# Two guests book the same hour at once.

## The sauna's bookings: who has each hour.
struct Sauna:
    booked: Mutex[dict[int, String]]

    ## Books `hour` for `name` if it is free, and says whether it did.
    def book(self, hour: int, name: str) -> bool:
        if hour in self.booked.lock().value:
            return False
        log.info("{name} finds {hour}:00 free")
        task.yield_now()  # as a write to a file would
        _ = self.booked.lock().value.insert(hour, name.to_string())
        return True

test "an hour is booked once":
    let sauna = Sauna(Mutex({}))
    let booked = task.scope do s:
        let aino = s.spawn(() -> sauna.book(18, "Aino"))
        let eero = s.spawn(() -> sauna.book(18, "Eero"))
        return Ok([aino.join()?, eero.join()?])
    assert booked == Ok([True, False]) or booked == Ok([False, True])
```

`task.yield_now()` lets another task run, as any blocking call would;
`log.info` is one too. Under the default order the test passes.

## 2. Run it under many orders

`--explore N` runs each test under the schedule seeds 0 to N-1, each
a different order of the tasks:

<!-- output: examples/guide-concurrency-bug/explore.console -->
```console
$ sisu test double.sisu
test "an hour is booked once" ... ok
1 passed, 0 failed, 0 blocked
$ sisu test --explore 20 double.sisu
test "an hour is booked once" ... FAILED

test "an hour is booked once" ... FAILED
with schedule seed 2; `--schedule-seed 2` runs it again
what the test printed:
  info [task 1]: Aino finds 18:00 free
  info [task 2]: Eero finds 18:00 free
panic: assertion failed: booked == Ok([True, False]) or booked == Ok([False, True])
    booked == Ok([True, False]) or booked == Ok([False, True])
    │         │
    │         Ok([True, False])
    Ok([True, True])
  at test "an hour is booked once" (double.sisu:22:5)
replay: sisu test --replay 1-3edcff83e8219252-0.0.1-t0-s2-r0 double.sisu

0 passed, 1 failed, 0 blocked
explored 20 schedules per test (seeds 0 to 19)
```

Under seed 2 both guests get the hour. The report names the seed, and
the `info` lines at the top are the log of the order that failed: both
guests found 18:00 free.

## 3. Run the failing order alone

`--schedule-seed 2` runs that order and no other, as the `replay:`
command does:

<!-- output: examples/guide-concurrency-bug/order.console -->
```console
$ sisu test --schedule-seed 2 double.sisu
test "an hour is booked once" ... FAILED

test "an hour is booked once" ... FAILED
what the test printed:
  info [task 1]: Aino finds 18:00 free
  info [task 2]: Eero finds 18:00 free
panic: assertion failed: booked == Ok([True, False]) or booked == Ok([False, True])
    booked == Ok([True, False]) or booked == Ok([False, True])
    │         │
    │         Ok([True, False])
    Ok([True, True])
  at test "an hour is booked once" (double.sisu:22:5)
replay: sisu test --replay 1-3edcff83e8219252-0.0.1-t0-s2-r0 double.sisu

0 passed, 1 failed, 0 blocked
```

Both tasks found 18:00 free before either booked it: the check and the
booking are two steps, and the other task ran between them. A failure
under a seed fails the same way on every run with that seed, so you
can add logs and run it again until the order is clear.

## 4. Fix it, and try the orders that matter

Make the check and the booking one step: take the lock once, with `var
guard = self.booked.lock()`, and hold it from the check to the
booking. Then search harder than random seeds do:

<!-- output: examples/guide-concurrency-bug/double_fixed.console -->
```console
$ sisu test --scheduler pct --depth 2 --explore 20 double_fixed.sisu
test "an hour is booked once" ... ok
1 passed, 0 failed, 0 blocked
explored 20 schedules of a PCT scheduler of depth 2 per test (seeds 0 to 19)
$ sisu test --scheduler exhaustive --depth 3 double_fixed.sisu
test "an hour is booked once" ... ok
1 passed, 0 failed, 0 blocked
searched 37 schedules (preemption bound 3)
$ sisu test --native --scheduler exhaustive --depth 3 double_fixed.sisu
test "an hour is booked once" ... ok
1 passed, 0 failed, 0 blocked
searched 37 schedules (preemption bound 3)
```

- `--scheduler pct --depth D` runs the PCT scheduler, which gives the
  tasks random priorities and changes them at `D - 1` random points. It
  finds the bugs that need a few switches at the right moments, which
  random seeds rarely hit when the tasks are long.
- `--scheduler exhaustive --depth N` tries every order with at most `N`
  switches. When it passes, no order of that kind fails; here 37
  orders with up to three switches.

A test that passes under random seeds is evidence; one that passes the
exhaustive search is a proof, up to its bound. The last command runs
the search on the compiled tests, which try the same 37 orders: the
schedulers run natively with the interpreter's choices, for `sisu run
--native` as for `sisu test --native`
([G20](native.md#3-run-the-tests-natively)).

## 5. Add the failures of the world

`sisu test --sim --faults P` runs the tests on a simulated system whose
capability calls fail now and then, at the rate `P`, each with an error
of its own error set other than `Revoked`, which only a revoked
caretaker gives and the simulator's own file system and network never
are. This sauna keeps each booking in memory and in a
file, and its test says that the two agree:

<!-- file: examples/guide-concurrency-bug/disk.sisu -->
```sisu
# Each booking is kept in memory and in a file, which must agree.

## The sauna's bookings: who has each hour, in memory and on disk.
struct Sauna:
    files: Fs
    booked: Mutex[dict[int, String]]

    ## Books `hour` for `name` if it is free, and says whether it did.
    def book(self, hour: int, name: str) -> bool ! IoError:
        var guard = self.booked.lock()
        if hour in guard.value:
            return Ok(False)
        _ = guard.value.insert(hour, name.to_string())
        self.files.write_text(Path.new("{hour}.txt"), name)?
        return Ok(True)

test "every booking is on the disk"(files: Fs):
    let sauna = Sauna(files, Mutex({}))
    _ = task.scope do s:
        let aino = s.spawn(() -> sauna.book(18, "Aino"))
        let eero = s.spawn(() -> sauna.book(19, "Eero"))
        return Ok([aino.join(), eero.join()])
    for hour in [18, 19]:
        let kept = hour in sauna.booked.lock().value
        assert kept == files.exists(Path.new("{hour}.txt"))
```

<!-- output: examples/guide-concurrency-bug/sim.console -->
```console
$ sisu test disk.sisu
test "every booking is on the disk" ... ok
1 passed, 0 failed, 0 blocked
$ sisu test --sim --faults 0.2 disk.sisu
test "every booking is on the disk" ... FAILED

test "every booking is on the disk" ... FAILED
panic: assertion failed: kept == files.exists(Path.new("{hour}.txt"))
  left:  True
  right: False
  at test "every booking is on the disk" (disk.sisu:25:9)
replay: sisu test --replay 1-0df33217e863cf48-0.0.1-t0-s0-r0-m200000 disk.sisu

0 passed, 1 failed, 0 blocked
```

The write failed after the booking was kept in memory, so the program
believes a booking that its disk has lost. Write first, and keep the
booking only once the write has succeeded. Then search orders and
faults together:

<!-- output: examples/guide-concurrency-bug/disk_fixed.console -->
```console
$ sisu test --sim --faults 0.2 --explore 20 disk_fixed.sisu
test "every booking is on the disk" ... ok
1 passed, 0 failed, 0 blocked
explored 20 schedules per test (seeds 0 to 19)
```

`--sim` combines with `--explore` and the schedulers, and its replay
token holds the fault rate, so a failure that the simulator finds runs
again the same way.

**Status.** `--explore`, `--schedule-seed` and the schedulers are in
stage 0 (D-014, D-075); `--sim` and its faults are a prototype of the
experiment (D-070), as [R15](../reference/testing.md#simulation) says.
