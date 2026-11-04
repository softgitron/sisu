# Coming from Go

This guide carries a Go programmer's habits over to Sisu: errors that
are values and pass on with `?`, interfaces that a type implements by
saying so, goroutines that live in a scope, and cancellation without a
`context` to pass. It ends with the spellings that differ. It assumes
that you read Go well and have met Sisu's basics, in chapters 1 to 8
and 14 to 20 of the tutorial.

**See also:** the tutorial's [chapter 8](../tutorial/08-errors.md)
(errors), [chapter 14](../tutorial/14-traits.md) (traits) and
[chapter 20](../tutorial/20-concurrency.md) (tasks); the proposal's
[06, Concurrency and capabilities](../../proposal/06-concurrency-and-capabilities.md),
which compares tasks with goroutines.

## 1. Pass errors on with ?

Errors are values in Sisu as in Go, but a function that can fail
returns `T ! E`, a value or an error, never both, and `?` is `if err !=
nil { return err }` in one character. `.context(message)` wraps an
error with what the code was doing, as `fmt.Errorf("...: %w", err)`
does:

<!-- file: examples/guide-from-go/errors.sisu -->
```sisu
# Go's `if err != nil { return err }` is `?`, and `%w` is `.context`.

## Why a line is not a reading.
error ReadingError:
    NotANumber(text: String)

## The temperature that `line` holds.
def reading(line: str) -> int ! ReadingError:
    let n = line.trim().parse_int() else:
        return Err(NotANumber(line.to_string()))
    return Ok(n)

## The sum of the readings of `text`, one on each line.
def total(text: str) -> int ! Contextual:
    var sum = 0
    for i, line in enumerate(text.lines()):
        sum += reading(line).context("line {i + 1}")?
    return Ok(sum)

def main():
    print(total("82\n79\n85"))
    print(total("82\nhot\n85"))
```

<!-- output: examples/guide-from-go/errors.out -->
```text
Ok(246)
Err(Context(message="line 2", cause="NotANumber(text=\"hot\")"))
```

An error is an error set's variant with data, which a caller matches
case by case, where Go compares with `errors.Is` or `errors.As`. A
result must be used: dropping one silently, as Go lets an unchecked
`err` be dropped, does not compile, and `_ = f()` says that you mean it.

## 2. Implement interfaces by saying so

An interface is a trait, and a type implements it with `impl Trait for
Type`, written out, where Go matches the methods by name. A function
takes any implementation as a type parameter, `[H: Heater]`, which is
made once for each type it is called with, or as a `dyn Heater` behind
a `Box` or a reference, which is Go's interface value:

<!-- file: examples/guide-from-go/traits.sisu -->
```sisu
# Go's interfaces are traits, which a type implements by saying so.

## Anything that can say how warm it is.
trait Heater:
    def celsius(self) -> int

struct Stove:
    kilowatts: int

struct Fire:
    logs: int

impl Heater for Stove:
    def celsius(self) -> int:
        return 20 + 6 * self.kilowatts

impl Heater for Fire:
    def celsius(self) -> int:
        return 20 + 15 * self.logs

## Whether `h` makes a sauna hot enough.
def hot_enough[H: Heater](h: H) -> bool:
    return h.celsius() >= 70

def main():
    print(hot_enough(Stove(9)))
    print(hot_enough(Fire(3)))
    let heaters: list[Box[dyn Heater]] = [Box(Stove(6)), Box(Fire(4))]
    for h in heaters:
        print(h.celsius())
```

<!-- output: examples/guide-from-go/traits.out -->
```text
True
False
56
80
```

There is no embedding of one struct in another to borrow its methods;
`@delegate` on a field forwards a trait's methods to it, as chapter 14
shows.

## 3. Start goroutines in a scope

`go f()` is `s.spawn(() -> f())` inside `task.scope do s:`, and the
scope is a `sync.WaitGroup` that cannot be forgotten: it ends only when
every task in it has ended, so no task outlives the code that started
it. Channels are bounded, as a buffered Go channel is:

<!-- file: examples/guide-from-go/goroutines.sisu -->
```sisu
# Go's goroutines, channel and `WaitGroup`: tasks in a scope that waits.

## Heats `sauna` for `minutes` on `clock`, then says so on `done`.
def heat(
    clock: Clock,
    done: Sender[String],
    sauna: str,
    minutes: int,
) -> ! {ChannelError, TaskError}:
    clock.sleep(Duration.minutes(minutes))?
    return done.send("{sauna} after {minutes} minutes")

def main():
    let clock = Clock.fake()
    let tx, rx = channel[String](capacity=3)
    task.scope do s:
        s.spawn(() -> heat(clock, tx, "Kota", 40))
        s.spawn(() -> heat(clock, tx, "Ranta", 25))
        s.spawn(() -> heat(clock, tx, "Savu", 60))
    tx.close()
    for line in rx:
        print(line)
```

<!-- output: examples/guide-from-go/goroutines.out -->
```text
Ranta after 25 minutes
Kota after 40 minutes
Savu after 60 minutes
```

The example sleeps on the test clock, `Clock.fake()`, so that it runs
at once; `sys.clock` would take an hour. A task that fails cancels the
others, and its error is the scope's, as an `errgroup.Group` does.
`select` is Sisu's too, over receives, sends, joins, timers and
cancellation.

## 4. Cancel without a context

A Go function that can be cancelled takes a `context.Context`. A Sisu
task is cancelled through its scope: `s.cancel()` cancels every task in
the scope, and `s.spawn_with(deadline=timer)` cancels one task when the
timer comes. The code that runs sees `Err(Cancelled)` at its next
blocking point, such as a `sleep`, so it takes no extra parameter:

<!-- file: examples/guide-from-go/deadline.sisu -->
```sisu
# Go's `context.WithTimeout`: a deadline on a task, and no context.

## Heats for `minutes`, a minute at a time, and gives the minutes.
def heat(clock: Clock, minutes: int) -> int ! TaskError:
    for _ in range(minutes):
        clock.sleep(Duration.minutes(1))?
    return Ok(minutes)

def main():
    let clock = Clock.fake()
    let start = clock.now()
    let heated = task.scope do s:
        let half_an_hour = clock.after(Duration.minutes(30))
        let heater = s.spawn_with(deadline=half_an_hour) do:
            return heat(clock, 60)
        return heater.join()
    print(heated)
    let minutes = (clock.now() - start).as_seconds() // 60
    print("stopped after {minutes} minutes")
```

<!-- output: examples/guide-from-go/deadline.out -->
```text
Err(Cancelled)
stopped after 30 minutes
```

`heat` knows nothing of the deadline; its `sleep` returned
`Err(Cancelled)`, and `?` passed it on. `task.shield do:` keeps a block
of work, such as finishing a write, out of the reach of cancellation.

## 5. Check the spellings that differ

A few Go habits do not compile:

<!-- file: examples/guide-from-go/habits.sisu -->
```sisu
# Two Go habits, which `sisu check` refuses.
def main():
    count := 3
    go print("hello {count}")
```

<!-- file: examples/guide-from-go/nil.sisu -->
```sisu
# Go's `nil` for a value that is not there.
def main():
    let best: str? = nil
    print(best ?? "nobody")
```

<!-- output: examples/guide-from-go/habits.console -->
```console
$ sisu check habits.sisu
error[E0123]: Sisu has no `:=`; bind with `let` or use `if let`
  --> habits.sisu:3:11
  |
3 |     count := 3
  |           ^^

error[E0117]: unexpected `print` after the end of the statement
  --> habits.sisu:4:8
  |
4 |     go print("hello {count}")
  |        ^^^^^
  = note: a task starts in a scope, which joins it: `task.scope do s:` and `s.spawn(() -> f())`

$ sisu check nil.sisu
error[E0200]: cannot find `nil` in this scope
  --> nil.sisu:3:22
  |
3 |     let best: str? = nil
  |                      ^^^
  = fix: Sisu writes it so: `None`

```

E0123 names the fix, `let` (or `var`, for a value that changes), and
E0200's fix writes `None` for `nil`, which is for an optional type
`T?`; a type that is not optional has no `nil` at all. `go f()` gets a
note instead of a fix, since the task needs a scope around it: it is
`s.spawn(() -> f())` inside `task.scope do s:`. Some other differences:
a name is exported with `pub`, not with a capital letter; a variable
has no zero value, and one read before it is given a value is E0366;
`for i := 0; i < n; i++` is `for i in range(n)`, and a statement `i++`
gets the fix `i += 1`; `switch` gets the fix `match`; and `defer` is
Sisu's too, though it runs at the end of the block that holds it, a
loop's body included, rather than of the function.
