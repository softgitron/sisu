# Test code that uses the clock

This guide makes code that reads or waits for the time testable: it
passes the clock in, freezes it at a known time in a test, moves it
forward, and tests code that sleeps or ticks without waiting. It
assumes capabilities and tests, from chapters 18 and 19 of the
tutorial.

**See also:** the tutorial's
[chapter 19](../tutorial/19-testing.md#a-clock-that-moves-when-every-task-waits)
(the test clock) and
[chapter 21](../tutorial/21-standard-library.md#time-that-does-not-drift) (`std.time`); the
source of `std.time`, [`compiler/std/time.sisu`](../../compiler/std/time.sisu);
the proposal's [17, Testing](../../proposal/17-testing.md#fixtures).

## 1. Take the clock as a parameter

A function that reads the time takes a `Clock` and asks it, rather
than reaching for the time some other way, and `main` passes it
`sys.clock`. This heater turns itself off two hours after it starts:

<!-- file: examples/guide-clock/heater.sisu -->
```sisu
# Code that reads the time takes a `Clock`, so a test can freeze it.

## A sauna's heater, which turns itself off two hours after it starts.
struct Heater:
    started: DateTime? = None

    ## Turns the heater on at the clock's time.
    def start(mut self, clock: Clock):
        self.started = Some(clock.now())

    ## Whether the heater is on at the clock's time.
    def is_on(self, clock: Clock) -> bool:
        if let Some(t) = self.started:
            return clock.now() - t < Duration.hours(2)
        return False

def main(sys: System):
    var heater = Heater()
    heater.start(sys.clock)
    print("on: {heater.is_on(sys.clock)}")

test "the heater is on for two hours":
    let start = DateTime.parse("2026-11-03T17:00:00Z").expect("a time")
    let clock = Clock.frozen_at(start)
    var heater = Heater()
    heater.start(clock)
    clock.advance(Duration.minutes(119))
    assert heater.is_on(clock)
    clock.advance(Duration.minutes(1))
    assert not heater.is_on(clock)

test "a heater that never started is off"(clock: TestClock):
    assert not Heater().is_on(clock)
```

The heater asks the clock whenever it needs the time, so the same code
reads real time in `main` and test time in the tests.

## 2. Freeze the clock at a known time

`Clock.frozen_at(t)` gives the test clock, a `TestClock`, set to `t`.
It stands still until the test moves it, so every assertion sees the
time the test chose. A `TestClock` becomes a `Clock` where one is
expected, as in `heater.start(clock)`. A test that only needs some
fixed time names `clock: TestClock` as a parameter, as the second test
does, and receives the test clock at 1970-01-01T00:00:00Z. There is
one test clock for each test: every `TestClock` handle reads it, and so
does `Fs.in_memory` when it stamps the time of a write.

## 3. Move the clock forward

`clock.advance(d)` moves the test clock on by `d`, and `clock.set(t)`
sets it to `t`. The first test advances to one minute before the
heater turns off and then to the minute itself, the two cases on each
side of the edge:

<!-- output: examples/guide-clock/heater.test -->
```text
test "the heater is on for two hours" ... ok
test "a heater that never started is off" ... ok
2 passed, 0 failed, 0 blocked
```

A test of a time limit is worth writing at the limit and next to it,
where the mistake of `<` for `<=` shows.

## 4. Test code that waits

The test clock also moves by itself when every task of the test is
waiting: it jumps to the earliest timer they wait for. Code that sleeps
for two hours therefore runs at once in a test, and wakes at the time
it would wake in production. This reminder waits until a quarter of an
hour before a turn:

<!-- file: examples/guide-clock/reminder.sisu -->
```sisu
# Code that sleeps runs at once under the test clock, which moves to the
# timer when every task waits.

## Waits until `minutes` before `turn`, and gives the time it woke.
def remind(clock: Clock, turn: DateTime, minutes: int) -> DateTime ! TaskError:
    let at = turn - Duration.minutes(minutes)
    if clock.now() < at:
        clock.sleep(at - clock.now())?
    return Ok(clock.now())

test "a reminder a quarter of an hour before"(clock: TestClock):
    clock.set(DateTime.parse("2026-11-03T16:00:00Z").expect("a time"))
    let turn = DateTime.parse("2026-11-03T18:00:00Z").expect("a time")
    let woke = remind(clock, turn, 15).expect("not cancelled")
    assert "{woke}" == "2026-11-03T17:45:00Z"

test "a reminder that is late goes at once"(clock: TestClock):
    clock.set(DateTime.parse("2026-11-03T17:50:00Z").expect("a time"))
    let turn = DateTime.parse("2026-11-03T18:00:00Z").expect("a time")
    let woke = remind(clock, turn, 15).expect("not cancelled")
    assert "{woke}" == "2026-11-03T17:50:00Z"
```

<!-- output: examples/guide-clock/reminder.test -->
```text
test "a reminder a quarter of an hour before" ... ok
test "a reminder that is late goes at once" ... ok
2 passed, 0 failed, 0 blocked
```

The first test takes no real time, though the reminder sleeps for an
hour and three quarters. A test of code that waits on several tasks
works the same way, as chapter 19's section on the test clock shows.

## 5. Test a loop that ticks

`Ticker.every(clock, period)` of `std.time` ticks once a period, and
`wait()` waits for the next tick. Its ticks do not drift: the `n`-th is
at `n` periods after the start whatever each turn took, and a tick that
passes while the loop is busy is skipped and counted in `missed`. Under
the test clock a ticking loop runs at once, tick by tick:

<!-- file: examples/guide-clock/ticker.sisu -->
```sisu
# A loop on a `Ticker` runs at once under the test clock, tick by tick.
from std.time import Ticker

## The minutes after the start at which a loop that ticks once a minute
## takes each of its first `count` turns.
def turns(clock: Clock, count: int) -> list[int] ! TaskError:
    let start = clock.now()
    var ticker = Ticker.every(clock, Duration.minutes(1))
    var at: list[int] = []
    for _ in range(count):
        ticker.wait()?
        at.push((clock.now() - start).as_seconds() // 60)
    return Ok(at)

test "one turn a minute"(clock: TestClock):
    assert turns(clock, 3) == Ok([1, 2, 3])

test "a tick missed while the loop was busy is skipped"(clock: TestClock):
    let start = clock.now()
    var ticker = Ticker.every(clock, Duration.minutes(1))
    clock.advance(Duration.seconds(150))
    ticker.wait().expect("not cancelled")
    assert ticker.missed == 2
    assert clock.now() - start == Duration.minutes(3)
```

<!-- output: examples/guide-clock/ticker.test -->
```text
test "one turn a minute" ... ok
test "a tick missed while the loop was busy is skipped" ... ok
2 passed, 0 failed, 0 blocked
```

The second test plays a turn that takes two and a half minutes:
`advance` stands for the slow work, and the ticker skips the two ticks
that passed and waits for the third.

Code that measures how long its work took reads an `Instant` from the
clock it was given, `Instant.now(clock)` of `std.time`, and asks its
`elapsed(clock)` at the end
([chapter 21](../tutorial/21-standard-library.md#measuring-how-long-work-took)). An `Instant`
reads the clock's monotonic time, which only goes forward, so on the
real clock a change of the computer's time does not move it. Under the
test clock the time it measures is what the test advanced, the same on
every run.
