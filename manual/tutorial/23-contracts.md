# 23. Contracts

A function's types say what it takes and what it gives. A contract says
more: what must hold of the arguments, what holds of the result, and
what stays true of a struct between its methods. In this chapter you
state preconditions and postconditions, look at values as they were on
entry, keep a struct's invariant, and give a type a predicate that its
values satisfy. Then you choose how much a build checks, and let `sisu
test` turn each contract into a property test. Last, you see what waits
for a verifier that proves contracts before the program runs. You need
about an hour and a half.

## What a function requires

`requires` states what a caller must give. Its clause is a `bool`
expression over the parameters, written at the start of the body, and
it is checked each time the function is entered:

<!-- file: examples/23-contracts/requires.sisu -->
```sisu
# `requires` states what a caller must give, and is checked on entry.

## The average of `temps`, rounded down.
def average(temps: Slice[int]) -> int:
    requires len(temps) > 0
    var total = 0
    for t in temps:
        total += t
    return total // len(temps)

def main():
    print(average([80, 85, 120]))
    let none: list[int] = []
    print(average(none))
```

The first call keeps the contract and prints 95. The second gives an
empty list, and the program stops before the division by zero that the
list would cause:

<!-- output: examples/23-contracts/requires.console -->
```console
$ sisu run requires.sisu
95
panic: precondition of `average` failed: len(temps) > 0
    len(temps) > 0
    │   │
    0   []
  at average (requires.sisu:5:14)
  at main (requires.sisu:14:11)
```

- A failed precondition panics with a message that names the
  function, `average`, and the clause, and with the value of each part
  of the clause that only reads, drawn as a failed `assert` draws them
  in chapter 19.
- The first `at` line is the clause, and the second the call that broke
  it. A precondition puts the blame on the caller: `average` was given
  what it does not take.
- A contract's clauses are the first lines of a body, before any
  statement, where Python puts a docstring. A clause after a statement
  is E0800 (`contract.misplaced`).

A violated contract is a bug, so it panics, as chapter 8 says of bugs:
it is not an error that a caller handles.

> **From Python.** An `assert` at the top of a function, or a `raise
> ValueError`, does the same work. A `requires` is checked the same way,
> and it is also part of the function's documentation, a test of its
> own, and something a build can leave out, as the rest of this chapter
> shows.

## What a function promises

`ensures` states what a function gives back. In its clause the value
being returned is `result`, and the clause is checked at every exit of
the body: each `return`, each `?` and the end:

<!-- file: examples/23-contracts/ensures.sisu -->
```sisu
# `ensures` states what a function gives back, named `result`.

## The minutes from `now` until the sauna opens at `opens`; both are
## minutes after midnight.
def minutes_until(now: int, opens: int) -> int:
    requires 0 <= now < 1440 and 0 <= opens < 1440
    ensures 0 <= result < 1440
    return opens - now

def main():
    print(minutes_until(16 * 60, 17 * 60))
    print(minutes_until(20 * 60, 17 * 60))
```

At 16:00 the sauna opens in 60 minutes. At 20:00 it opened three hours
ago, and the function gives a negative number, which its postcondition
does not allow:

<!-- output: examples/23-contracts/ensures.console -->
```console
$ sisu run ensures.sisu
60
panic: postcondition of `minutes_until` failed: 0 <= result < 1440
    0 <= result < 1440
         │
         -180
  at minutes_until (ensures.sisu:7:13)
  at main (ensures.sisu:12:11)
```

A failed postcondition puts the blame on the function: it was given
what it takes, and it did not give what it promises. The values under
the clause show the result, `-180`.

**Try it.** Return `(opens - now) % 1440`, and run the program again.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/23-contracts/ensures_fixed.console -->
```console
$ sisu run ensures_fixed.sisu
60
1260
```

`%` takes the sign of the divisor, so the remainder is never negative:
from 20:00, the next opening at 17:00 is 1,260 minutes away.

</details>

## The value on entry

A method that changes `self`, or a function with a `mut` parameter,
often promises how the value changed. `old(e)` in an `ensures` is the
value of `e` when the function was entered. A result with labels, as
chapter 3 wrote them, is named by its labels instead of `result`:

<!-- file: examples/23-contracts/old.sisu -->
```sisu
# `old(e)` is the value of `e` on entry; a labelled result is its label.
struct Stove:
    celsius: int

    ## Heats the stove by `degrees`, to 110 at most.
    def heat(mut self, degrees: int):
        requires degrees >= 0
        ensures self.celsius == min(old(self.celsius) + degrees, 110)
        self.celsius = min(self.celsius + degrees, 110)

## Splits `minutes` into turns of `turn` minutes, and the minutes left.
def turns(minutes: int, turn: int) -> (count: int, left: int):
    requires turn > 0
    ensures count * turn + left == minutes and 0 <= left < turn
    return (minutes // turn, minutes % turn)

def main():
    var stove = Stove(celsius=60)
    stove.heat(30)
    stove.heat(30)
    print("{stove.celsius} degrees")
    let count, left = turns(95, 20)
    print("{count} turns and {left} minutes")
```

<!-- output: examples/23-contracts/old.out -->
```text
110 degrees
4 turns and 15 minutes
```

- `old(self.celsius)` is the temperature before `heat` changed it. The
  compiler copies it on entry, and the clause compares the new
  temperature with it on the way out.
- The second heating stops at 110, which is what the clause says:
  `min` of the old temperature plus the degrees, and 110.
- `turns` returns `(count: int, left: int)`, so its clause names
  `count` and `left`. It holds for any `minutes`, negative ones
  included, since `//` rounds down and `%` follows it.

**Try it.** Change the `requires` of `heat` to `requires
old(self.celsius) < 110`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/23-contracts/old_in_requires.console -->
```console
$ sisu run old_in_requires.sisu
error[E0803]: `old(...)` is a value as it was on entry, which only an `ensures` clause can ask for
  --> old_in_requires.sisu:7:18
  |
7 |         requires old(self.celsius) < 110
  |                  ^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0803 (`contract.old-outside`) says that only an `ensures` can ask for
`old`. On entry the value is as it is, so a `requires` names it
plainly: `self.celsius < 110`.

</details>

## Contracts only read

A contract describes the program; it must not change it. A clause can
call only `pure` functions, which chapter 16 introduced: they use no
capability, change nothing outside themselves, and give the same result
for the same arguments. A clause also changes nothing itself:

<!-- file: examples/23-contracts/pure_helper.sisu -->
```sisu
# A contract may call only `pure` functions, and changes nothing.

## Whether no hour of `hours` is there twice.
pure def all_different(hours: Slice[int]) -> bool:
    var seen: set[int] = set()
    for h in hours:
        if seen.contains(h):
            return False
        seen.add(h)
    return True

## Books `hour` in `hours`, where it must not be already.
def book(mut hours: list[int], hour: int):
    requires hour not in hours
    ensures all_different(hours)
    hours.push(hour)

def main():
    var hours = [17, 18]
    book(mut hours, 20)
    print(hours)
```

<!-- output: examples/23-contracts/pure_helper.out -->
```text
[17, 18, 20]
```

`all_different` builds a set of its own and changes it, which a `pure`
function may do: what it changes is its own. Because a clause only
reads, a build can leave every check out without changing what the
program does, and a clause that fails can be evaluated again, part by
part, to show the values under it.

**Try it.** Remove `pure` from `all_different`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/23-contracts/not_pure.console -->
```console
$ sisu run not_pure.sisu
error[E0801]: a contract only reads what it is given, and `all_different` is not `pure`
  --> not_pure.sisu:15:13
   |
15 |     ensures all_different(hours)
   |             ^^^^^^^^^^^^^^^^^^^^
  = fix: declare it `pure`, if it only reads what it is given: `pure `

error[D0006]: the program was not run: errors found (1)
```

E0801 (`contract.not-pure`) names the function that is not `pure`, and
its fix writes `pure` before its `def`, which the compiler then checks: a
function that prints, or takes a capability, cannot be `pure`, and a
contract cannot call it. A clause that changes what it reads, such as
`requires xs.pop() != None`, is E0802 (`contract.changes`).

</details>

## What stays true of a struct

An `invariant` among a struct's fields states what holds of every value
of the type. It is checked when a value is built and each time a `mut
self` method returns:

<!-- file: examples/23-contracts/schedule.sisu -->
```sisu
# An `invariant` holds after construction and after every `mut self` method.

## The hours a sauna is open, from `opens` to `closes`.
struct Schedule:
    opens: int
    closes: int
    invariant 0 <= self.opens < self.closes <= 24

    ## Opens `hours` earlier.
    def open_earlier(mut self, hours: int):
        self.opens -= hours

    ## How many hours it is open.
    def hours(self) -> int:
        return self.closes - self.opens

def main():
    var today = Schedule(opens=16, closes=21)
    today.open_earlier(2)
    print("open {today.opens} to {today.closes}, {today.hours()} hours")
    today.open_earlier(20)
    print("open {today.opens} to {today.closes}")
```

Opening two hours earlier keeps the schedule sensible. Opening twenty
hours earlier does not:

<!-- output: examples/23-contracts/schedule.console -->
```console
$ sisu run schedule.sisu
open 14 to 21, 7 hours
panic: invariant of `Schedule` failed: 0 <= self.opens < self.closes <= 24
    0 <= self.opens < self.closes <= 24
         │            │
         -6           21
  at the invariant of Schedule (schedule.sisu:7:15)
  at Schedule.open_earlier (schedule.sisu:11:28)
  at main (schedule.sisu:21:5)
```

- The panic names the type and the clause, and its trace runs from
  the clause through the method that broke it to the call.
- The check comes when `open_earlier` returns, by any exit. Inside a
  method the invariant may be broken for a while, as long as it holds
  again at the end.
- A struct can have several `invariant` lines; each is checked.

A method that only reads, such as `hours`, cannot break the invariant,
so it is not checked there.

## Only a type's own methods change its fields

An invariant is checked when a method returns. A field assigned from
outside the type would never be checked, so the compiler does not allow
it:

<!-- file: examples/23-contracts/outside.sisu -->
```sisu
# Only the methods of a type with an invariant may change its fields.

## The hours a sauna is open, from `opens` to `closes`.
struct Schedule:
    opens: int
    closes: int
    invariant 0 <= self.opens < self.closes <= 24

def main():
    var today = Schedule(opens=16, closes=21)
    today.closes = 25
    print("open {today.opens} to {today.closes}")
```

<!-- output: examples/23-contracts/outside.console -->
```console
$ sisu run outside.sisu
error[E0806]: `today.closes` is part of a value of `Schedule`, whose invariant only its own methods may change and check again; change it through one of them
  --> outside.sisu:11:5
   |
11 |     today.closes = 25
   |     ^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0806 (`contract.invariant-field`) says that `closes` belongs to a
`Schedule`, whose own methods must make the change, so that they check
the invariant again. Give the type a method that does what the
assignment meant, such as `close_later(mut self, hours: int)`, and call
it. Building a value by its fields is allowed everywhere, since
construction checks the invariant too.

> **From Python.** A Python class keeps its invariant by convention: an
> underscore on a field asks other code not to touch it. Here the
> compiler refuses the assignment, and the methods that may make it
> check the invariant each time.

## Types with a predicate

A refinement type is a type whose values satisfy a predicate:
`type Percent = int where 0 <= self <= 100`, with `self` the value of
the base type. A `Percent` is an `int` wherever an `int` is expected,
and an `int` becomes a `Percent` only where its predicate is checked:

<!-- file: examples/23-contracts/refinement.sisu -->
```sisu
# A refinement type is a type whose values satisfy a predicate.
type Percent = int where 0 <= self <= 100

## How the steam feels at `humidity`, which is an `int` too.
def steam(humidity: Percent) -> str:
    if humidity < 20:
        return "dry"
    if humidity < 50:
        return "soft"
    return "heavy"

def main():
    let after_ladle: Percent = 35
    print("{after_ladle}: {steam(after_ladle)}")
    for reading in ["12", "70", "140"]:
        let n = reading.parse_int() ?? 0
        let humidity = Percent.try_from(n) else as e:
            print("{n}: {e}")
            continue
        print("{n}: {steam(humidity)}")
```

<!-- output: examples/23-contracts/refinement.out -->
```text
35: soft
12: dry
70: heavy
140: Unsatisfied(ty="Percent", predicate="0 <= self <= 100")
```

- `let after_ladle: Percent = 35` gives a constant, which the compiler
  checks while it compiles.
- `Percent.try_from(n)` checks any other value when the program runs,
  and gives `Percent ! RefinementError`. Its error,
  `Unsatisfied(ty, predicate)`, names the type and the predicate that
  failed.
- `steam` compares its `Percent` with numbers as any `int` would be.
  What is computed from one, such as `humidity + 10`, is an `int` again,
  and becomes a `Percent` only through another check.

The predicate is checked once, where a value comes in, rather than in
every function that takes it. Each function that takes a `Percent` can
rely on it.

**Try it.** Make the first humidity `135`. Then put it back, and pass
`n` to `steam` in place of `humidity`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/23-contracts/refinement_constant.console -->
```console
$ sisu run refinement_constant.sisu
error[E0810]: `135` is not of type `Percent`: `0 <= self <= 100` is false for it
  --> refinement_constant.sisu:13:32
   |
13 |     let after_ladle: Percent = 135
   |                                ^^^

error[D0006]: the program was not run: errors found (1)
```

<!-- output: examples/23-contracts/refinement_unchecked.console -->
```console
$ sisu run refinement_unchecked.sisu
error[E0811]: expected `Percent`, found `int`; a value becomes a `Percent` where its predicate is checked, with `Percent.try_from(...)`
  --> refinement_unchecked.sisu:20:28
   |
20 |         print("{n}: {steam(n)}")
   |                            ^
  = note: check it where it runs: `Percent.try_from(...)`

error[D0006]: the program was not run: errors found (1)
```

E0810 (`refine.not-satisfied`) checks the constant at compile time, so
the program never runs with a humidity of 135. E0811
(`refine.needs-try-from`) refuses an `int` where a `Percent` is
expected, and its note names the check that makes one.

</details>

> **From Python.** A type hint such as `NewType("Percent", int)` checks
> nothing when the program runs, and a validating library checks a value
> where you call it. A refinement is part of the type: the compiler
> checks constants, `try_from` checks the rest, and no unchecked `int`
> becomes a `Percent`.

## A refinement of a refinement

A refinement can refine another one. A value of the outer type
satisfies every predicate down to the base, and `try_from` checks them
in that order, the innermost first:

<!-- file: examples/23-contracts/chain.sisu -->
```sisu
# A refinement of a refinement checks each predicate, the base's first.
type Percent = int where 0 <= self <= 100
type Dry = Percent where self < 20

def main():
    for n in [10, 30, 130]:
        print(Dry.try_from(n))
```

<!-- output: examples/23-contracts/chain.out -->
```text
Ok(10)
Err(Unsatisfied(ty="Dry", predicate="self < 20"))
Err(Unsatisfied(ty="Percent", predicate="0 <= self <= 100"))
```

30 is a `Percent` and not a `Dry`, so the error names `Dry`. 130 is
not even a `Percent`, so the error names that predicate, the first in
the chain that fails. A `Dry` is a `Percent`, and so an `int`, wherever
those are expected.

## How much is checked

Checks cost time when the program runs. `sisu run` checks every
contract unless it is told otherwise, and `--contracts` sets the level:
`all`, `preconditions` or `off`. Here is the bug of the section "What a
function promises", called once more with a time that does not exist:

<!-- file: examples/23-contracts/levels.sisu -->
```sisu
# How much is checked: every contract, the preconditions, or none.

## The minutes from `now` until `opens`, both minutes after midnight.
def minutes_until(now: int, opens: int) -> int:
    requires 0 <= now < 1440 and 0 <= opens < 1440
    ensures 0 <= result < 1440
    return opens - now

def main():
    print(minutes_until(20 * 60, 17 * 60))
    print(minutes_until(25 * 60, 17 * 60))
```

<!-- output: examples/23-contracts/levels.console -->
```console
$ sisu run levels.sisu
panic: postcondition of `minutes_until` failed: 0 <= result < 1440
    0 <= result < 1440
         │
         -180
  at minutes_until (levels.sisu:6:13)
  at main (levels.sisu:10:11)
$ sisu run --contracts=preconditions levels.sisu
-180
panic: precondition of `minutes_until` failed: 0 <= now < 1440
    0 <= now < 1440
         │
         1500
  at minutes_until (levels.sisu:5:14)
  at main (levels.sisu:11:11)
$ sisu run --contracts=off levels.sisu
-180
-480
$ sisu run --native --release levels.sisu
-180
panic: precondition of `minutes_until` failed: 0 <= now < 1440
    0 <= now < 1440
         │
         1500
  at minutes_until (levels.sisu:5:14)
  at main (levels.sisu:11:11)
$ sisu test --contracts=off levels.sisu
error[D0029]: `sisu test` checks every contract; `--contracts` sets what `sisu run` and `sisu sir` check
```

- At the level `all`, the first call fails its postcondition.
- At `preconditions`, the postcondition is not checked, so the first
  call prints `-180`, and the second fails its precondition: 25:00 is
  not a time of day.
- At `off`, nothing is checked, and the bug prints two wrong numbers.
- `sisu run --native --release` compiles the program with the release
  profile, which checks the preconditions by default: a broken
  precondition is a caller's bug that is cheap to find and costly to
  miss. In a package, `contracts = "all"` or `"off"` under
  `[profile.release]` in `sisu.toml` changes that.
- `sisu test` checks every contract, always, and D0029
  (`cli.test-checks-contracts`) refuses `--contracts`.

One check ignores the level: the invariant of a value in a `Mutex`, which
chapter 20 showed is checked when a panic drops the guard.

`debug_assert` is the assertion that follows the level. It is checked,
with the values of its parts, where every contract is, in `sisu run`
and `sisu test`, and left out, its condition not even computed, where
fewer are, as in a release build. Use it for a check that costs more
than the work it guards, such as whether a list is sorted before a
search that needs it:

<!-- file: examples/23-contracts/sorted_search.sisu -->
```sisu
# `debug_assert` is checked where every contract is, and left out elsewhere.

## Whether `x` is in `xs`, which must be sorted.
def contains(xs: list[int], x: int) -> bool:
    debug_assert xs == sorted(xs), "the numbers must be sorted"
    var lo = 0
    var hi = len(xs)
    while lo < hi:
        let mid = (lo + hi) // 2
        if xs[mid] < x:
            lo = mid + 1
        else:
            hi = mid
    return lo < len(xs) and xs[lo] == x

def main():
    print(contains([1, 3, 5, 8], 5))
    print(contains([8, 1, 5, 3], 8))
```

<!-- output: examples/23-contracts/sorted_search.console -->
```console
$ sisu run sorted_search.sisu
True
panic: assertion failed: xs == sorted(xs)
  message: the numbers must be sorted
    xs == sorted(xs)
    │     │      │
    [8, 1, 5, 3] [8, 1, 5, 3]
          [1, 3, 5, 8]
  at contains (sorted_search.sisu:5:5)
  at main (sorted_search.sisu:18:11)
$ sisu run --contracts=preconditions sorted_search.sisu
True
False
$ sisu run --native --release sorted_search.sisu
True
False
```

The release build answers `False` for a number that is there: the check
was the only thing that noticed the unsorted list. A broken
`debug_assert` is a bug as a broken `assert` is; it is left out for
speed, never because the bug is harmless.

## Contracts as tests and documentation

A contract says what a function does for every argument it accepts,
which is what a property test checks. For each function with a contract
whose parameters have generators, `sisu test` adds a test named
`contract of NAME`, which calls the function on `arbitrary` cases that
its `requires` accepts and lets the `ensures` and the invariants judge
each call:

<!-- file: examples/23-contracts/round_up.sisu -->
```sisu
# `sisu test` checks each function's contract on cases it makes.

## `minutes` rounded up to whole turns of `turn` minutes.
def round_up(minutes: int, turn: int) -> int:
    requires minutes >= 0 and turn > 0
    ensures result % turn == 0
    ensures minutes <= result < minutes + turn
    return (minutes // turn + 1) * turn

test "a turn and a half":
    assert round_up(30, 20) == 40

test "a little over two turns":
    assert round_up(41, 20) == 60
```

<!-- output: examples/23-contracts/round_up.console -->
```console
$ sisu test round_up.sisu
test "a turn and a half" ... ok
test "a little over two turns" ... ok
test "contract of round_up" ... FAILED

test "contract of round_up" ... FAILED
panic: the property failed for (0, 1) (case 2, shrunk 3 times)
  it panicked: postcondition of `round_up` failed: minutes <= result < minutes + turn
    minutes <= result < minutes + turn
    │          │        │       │ │
    0          1        0       1 1
  at test "contract of round_up" (<arbitrary>/main.sisu:13:5)
replay: sisu test --replay 1-bf1544b37d7f271b-0.0.1-t2-s0-r0-c0.0.0.1 round_up.sisu

2 passed, 1 failed, 0 blocked
```

The two tests written by hand pass. The third, which nobody wrote,
finds that a number of minutes that is already whole turns, here 0, is
rounded up by a whole turn: `round_up(0, 1)` gives 1, which is not
less than `0 + 1`. A case that the `requires` refuses is not tried, so
a precondition keeps the cases to what the function takes.

**Try it.** Return `(minutes + turn - 1) // turn * turn`, and run the
tests again.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/23-contracts/round_up_fixed.console -->
```console
$ sisu test round_up_fixed.sisu
test "a turn and a half" ... ok
test "a little over two turns" ... ok
test "contract of round_up" ... ok
3 passed, 0 failed, 0 blocked
```

</details>

Contracts are documentation too. `sisu doc` shows each function's
`requires` and `ensures` under its signature, a struct's invariants
under its fields, a refinement's predicate with the `try_from` that
checks it, and, under each `mut self` method of a type with an
invariant, that the method checks it again. Because they are part of a
function's promise, strengthening a precondition or weakening a
postcondition breaks the function's callers, and the API diff of `sisu
publish`, which chapter 28 is about, counts it as a breaking change.
A trait's method can have a contract too, which binds every impl of it
([07](../../proposal/07-contracts.md)): an impl's method checks the
trait's `requires` on entry and its `ensures` on exit, and a failure
names the trait's method. The contract binds an impl in any module or
package, as [R11](../reference/contracts.md#contracts-on-trait-methods)
shows with a trait in one module and its impl in another.

## What waits for the verifier

The contracts of this chapter are checked when the program runs. The
grammar of a clause is the grammar of expressions, restricted to `pure`
code, so that a static verifier can later prove a contract before the
program runs, and the optimiser can then leave out the checks it
proves. That verifier is planned after version 1, and the words it
needs are reserved:

<!-- file: examples/23-contracts/decreases.sisu -->
```sisu
# What a static verifier would read: `decreases` is not checked yet.

## The number of digits of `n`.
def digits(n: int) -> int:
    requires n >= 0
    decreases n
    if n < 10:
        return 1
    return 1 + digits(n // 10)

def main():
    print(digits(2026))
```

<!-- output: examples/23-contracts/decreases.console -->
```console
$ sisu run decreases.sisu
error[E0900]: a termination measure (`decreases`) is not implemented in stage 0 yet; it arrives with the contract verifier (after version 1)
  --> decreases.sisu:6:5
  |
6 |     decreases n
  |     ^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0900 (`stage0.unsupported`) says that `decreases`, a measure that
shows a recursion or a loop ends, waits for the verifier. An
`invariant` inside a loop's body gets the same E0900, and ghost code,
variables that exist only for a proof, is not in the language yet.
Until then, remove the line: the function runs, and its `requires` is
checked as before.

## Exercise 23.1: predict the refinements

Open
[`manual/exercises/23-contracts/predict_refinements.sisu`](../exercises/23-contracts/predict_refinements.sisu):

<!-- file: exercises/23-contracts/predict_refinements.sisu -->
```sisu
##! Exercise 23.1, predict the refinements. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of what `failed` or `steam` gives.
##! Write each as a literal, then run `sisu test predict_refinements.sisu`.
##! The hints are in chapter 23 of the tutorial, under "Exercise 23.1:
##! predict the refinements".

type Percent = int where 0 <= self <= 100
type Dry = Percent where self < 20
type Bone = Dry where self < 5

## The type that the error names when `n` is not a `Bone`, or "none".
def failed(n: int) -> String:
    match Bone.try_from(n):
        case Ok(_):
            return "none"
        case Err(Unsatisfied(ty, _)):
            return ty

## The humidity after `ladles` of water, each adding 15 percent.
def steam(start: Dry, ladles: int) -> int:
    return start + 15 * ladles

const ZERO: str = todo("failed(0)")
const SEVEN: str = todo("failed(7)")
const FIFTY: str = todo("failed(50)")
const MINUS_ONE: str = todo("failed(-1)")
const STEAM: int = todo("steam from 10 with 7 ladles")

test "nothing fails":
    assert failed(0) == ZERO

test "a little too wet":
    assert failed(7) == SEVEN

test "wet":
    assert failed(50) == FIFTY

test "below zero":
    assert failed(-1) == MINUS_ONE

test "steam from a dry start":
    let start = Dry.try_from(10).expect("10 is dry")
    assert steam(start, 7) == STEAM
```

<!-- output: solutions/23-contracts/predict_refinements.console -->
```console
$ sisu test predict_refinements.sisu
warning[W0001]: typed hole `failed(0)` of type `str`
  --> predict_refinements.sisu:24:19
   |
24 | const ZERO: str = todo("failed(0)")
   |                   ^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `failed(7)` of type `str`
  --> predict_refinements.sisu:25:20
   |
25 | const SEVEN: str = todo("failed(7)")
   |                    ^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `failed(50)` of type `str`
  --> predict_refinements.sisu:26:20
   |
26 | const FIFTY: str = todo("failed(50)")
   |                    ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `failed(-1)` of type `str`
  --> predict_refinements.sisu:27:24
   |
27 | const MINUS_ONE: str = todo("failed(-1)")
   |                        ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `steam from 10 with 7 ladles` of type `int`
  --> predict_refinements.sisu:28:20
   |
28 | const STEAM: int = todo("steam from 10 with 7 ladles")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "nothing fails" ... blocked by todo("failed(0)")
test "a little too wet" ... blocked by todo("failed(7)")
test "wet" ... blocked by todo("failed(50)")
test "below zero" ... blocked by todo("failed(-1)")
test "steam from a dry start" ... blocked by todo("steam from 10 with 7 ladles")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

`Bone` refines `Dry`, which refines `Percent`, so `Bone.try_from`
checks three predicates. Work out which of them each number fails.

</details>

<details>
<summary>Hint 2</summary>

The error names the first predicate that fails, in the order of the
section "A refinement of a refinement". What `steam` computes is not a
refinement at all.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/23-contracts/predict_refinements.sisu`](../solutions/23-contracts/predict_refinements.sisu).

## Exercise 23.2: a field changed from outside

Open
[`manual/exercises/23-contracts/stove.sisu`](../exercises/23-contracts/stove.sisu):

<!-- file: exercises/23-contracts/stove.sisu -->
```sisu
##! Exercise 23.2, a field changed from outside. Kind: fix. Level: 1.
##!
##! A sauna's stove keeps its temperature between 0 degrees and its limit,
##! and says so in its invariant. `throw_water` changes the temperature
##! from outside the type, and the file does not compile. Run
##! `sisu test stove.sisu`, read the diagnostic, and fix the program, not
##! the tests, so that every test passes. The hints are in chapter 23 of
##! the tutorial, under "Exercise 23.2: a field changed from outside".

## A stove at `celsius` degrees, which never goes past `limit`.
struct Stove:
    celsius: int
    limit: int
    invariant 0 <= self.celsius <= self.limit

    ## Heats the stove by `degrees`, up to its limit.
    def heat(mut self, degrees: int):
        requires degrees >= 0
        self.celsius = min(self.celsius + degrees, self.limit)

## Throws a ladle of water on the stones, which cools the stove by 15
## degrees, to 0 at the lowest.
def throw_water(mut stove: Stove):
    stove.celsius = max(stove.celsius - 15, 0)

test "heating stops at the limit":
    var stove = Stove(celsius=60, limit=110)
    stove.heat(30)
    assert stove.celsius == 90
    stove.heat(30)
    assert stove.celsius == 110

test "water cools the stove":
    var stove = Stove(celsius=80, limit=110)
    throw_water(mut stove)
    assert stove.celsius == 65
    throw_water(mut stove)
    assert stove.celsius == 50

test "water cannot cool it below zero":
    var stove = Stove(celsius=10, limit=110)
    throw_water(mut stove)
    assert stove.celsius == 0
```

<!-- output: solutions/23-contracts/stove.console -->
```console
$ sisu test stove.sisu
error[E0806]: `stove.celsius` is part of a value of `Stove`, whose invariant only its own methods may change and check again; change it through one of them
  --> stove.sisu:24:5
   |
24 |     stove.celsius = max(stove.celsius - 15, 0)
   |     ^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

E0806 says who may change `celsius`: the methods of `Stove`. The tests
call `throw_water` and need not know how it cools the stove.

</details>

<details>
<summary>Hint 2</summary>

`heat` shows the shape of a method that changes the temperature and
keeps the invariant. The same shape can cool.

</details>

When every test passes, compare your answer with
[`manual/solutions/23-contracts/stove.sisu`](../solutions/23-contracts/stove.sisu).

## Exercise 23.3: a search and its contract

Open
[`manual/exercises/23-contracts/search.sisu`](../exercises/23-contracts/search.sisu):

<!-- file: exercises/23-contracts/search.sisu -->
```sisu
##! Exercise 23.3, a search and its contract. Kind: fix. Level: 2.
##!
##! `find` looks for a number in a sorted list by halving the part of the
##! list it still searches. Its contract says what it gives, and its
##! tests pass, but the test that `sisu test` makes from the contract
##! fails. Run `sisu test search.sisu`, read the case that fails, and fix
##! the body of `find`, not its contract or the tests, so that every test
##! passes. The hints are in chapter 23 of the tutorial, under
##! "Exercise 23.3: a search and its contract".

## The place of `target` in `items`, which are sorted, or `None` when it
## is not there.
def find(items: Slice[int], target: int) -> int?:
    requires items.is_sorted()
    ensures match result:
        case Some(i): items[i] == target
        case None: not items.contains(target)
    var lo = 0
    var hi = len(items) - 1
    while lo < hi:
        let mid = (lo + hi) // 2
        if items[mid] == target:
            return Some(mid)
        if items[mid] < target:
            lo = mid + 1
        else:
            hi = mid
    return None

test "a number in the middle":
    assert find([1, 3, 5, 7], 5) == Some(2)
    assert find([10, 20, 30, 40, 50], 20) == Some(1)

test "a number that is not there":
    assert find([1, 3, 5, 7], 4) == None
    assert find([], 4) == None
```

<!-- output: solutions/23-contracts/search.console -->
```console
$ sisu test search.sisu
test "a number in the middle" ... ok
test "a number that is not there" ... ok
test "contract of find" ... FAILED

test "contract of find" ... FAILED
panic: the property failed for ([0], 0) (case 58, shrunk 3 times)
  it panicked: postcondition of `find` failed: match result:
        case Some(i): items[i] == target
        case None: not items.contains(target)
  at test "contract of find" (<arbitrary>/main.sisu:13:5)
replay: sisu test --replay 1-1b1679770fa54e9d-0.0.1-t2-s0-r0-c1.0.0.0.0.0 search.sisu

2 passed, 1 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

The failing case is a list of one number, and that number is the
target. Follow the body by hand for it: which values do `lo` and `hi`
start with, and does the loop run?

</details>

<details>
<summary>Hint 2</summary>

`lo` and `hi` mark the part of the list still to search. Decide
whether `hi` is the last place of that part or the place after it, and
make the start, the loop's condition and each change of `hi` agree.

</details>

When every test passes, compare your answer with
[`manual/solutions/23-contracts/search.sisu`](../solutions/23-contracts/search.sisu).

## Exercise 23.4: a time of day from text

Open
[`manual/exercises/23-contracts/clock_time.sisu`](../exercises/23-contracts/clock_time.sisu):

<!-- file: exercises/23-contracts/clock_time.sisu -->
```sisu
##! Exercise 23.4, a time of day from text. Kind: write. Level: 2.
##!
##! A sauna's booking form sends the time as text such as "18:30". Write
##! the bodies of `parse_time`, which checks the text and makes a `Time`,
##! and `later`, which moves a time on, so that every test passes. The
##! fields of `Time` are refinement types, so each number becomes an
##! `Hour` or a `Minute` only where its predicate is checked. Run the
##! tests with `sisu test clock_time.sisu`. The hints are in chapter 23 of
##! the tutorial, under "Exercise 23.4: a time of day from text".

type Hour = int where 0 <= self <= 23
type Minute = int where 0 <= self <= 59

## A time of day.
@derive(Eq, Debug)
struct Time:
    hour: Hour
    minute: Minute

error FormError:
    BadFormat(text: String)
    OutOfRange(text: String)

## The time that `text` writes as `HH:MM`: `BadFormat` when it is not two
## numbers with a colon between them, and `OutOfRange` when a number is
## not on the clock.
def parse_time(text: str) -> Time ! FormError:
    return todo("parse_time")

## The time `minutes` after `time`, past midnight into the next day if it
## must.
def later(time: Time, minutes: int) -> Time:
    requires minutes >= 0
    return todo("later")

test "a time of day":
    assert parse_time("18:30") == Ok(Time(hour=18, minute=30))
    assert parse_time("07:05") == Ok(Time(hour=7, minute=5))

test "numbers that are not on the clock":
    assert parse_time("24:00") == Err(OutOfRange("24:00"))
    assert parse_time("12:60") == Err(OutOfRange("12:60"))

test "text that is not a time":
    assert parse_time("noon") == Err(BadFormat("noon"))
    assert parse_time("12:3x") == Err(BadFormat("12:3x"))

test "later on the same day":
    let start = Time(hour=17, minute=45)
    assert later(start, 30) == Time(hour=18, minute=15)
    assert later(start, 0) == start

test "later, past midnight":
    let start = Time(hour=23, minute=50)
    assert later(start, 20) == Time(hour=0, minute=10)
    assert later(start, 24 * 60) == start
```

<!-- output: solutions/23-contracts/clock_time.console -->
```console
$ sisu test clock_time.sisu
warning[W0001]: typed hole `parse_time` of type `Time ! FormError`
  --> clock_time.sisu:28:12
   |
28 |     return todo("parse_time")
   |            ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `later` of type `Time`
  --> clock_time.sisu:34:12
   |
34 |     return todo("later")
   |            ^^^^^^^^^^^^^

test "a time of day" ... blocked by todo("parse_time")
test "numbers that are not on the clock" ... blocked by todo("parse_time")
test "text that is not a time" ... blocked by todo("parse_time")
test "later on the same day" ... blocked by todo("later")
test "later, past midnight" ... blocked by todo("later")
test "contract of later" ... blocked by todo("later")
0 passed, 0 failed, 6 blocked
```

<details>
<summary>Hint 1</summary>

`parse_time` has two kinds of failure: text that holds no two numbers,
and numbers that hold no time. `split_once` and `parse_int` find the
first kind, and `try_from` the second.

</details>

<details>
<summary>Hint 2</summary>

`later` is easier in minutes after midnight. Arithmetic on an `Hour`
gives an `int`, so the hour and the minute you compute become an
`Hour` and a `Minute` again through `try_from`, which cannot fail for
numbers you have kept in range.

</details>

When every test passes, compare your answer with
[`manual/solutions/23-contracts/clock_time.sisu`](../solutions/23-contracts/clock_time.sisu).

## Exercise 23.5: an account that keeps its promises

Open
[`manual/exercises/23-contracts/account.sisu`](../exercises/23-contracts/account.sisu):

<!-- file: exercises/23-contracts/account.sisu -->
```sisu
##! Exercise 23.5, an account that keeps its promises. Kind: write. Level: 3.
##!
##! An account's balance may go below zero, as far as its limit. Write
##! the bodies of `deposit`, `withdraw` and `transfer` so that every test
##! passes. A withdrawal that leaves the balance below zero costs a fee,
##! and one that would take the balance and the fee past the limit is
##! refused and changes nothing. `sisu test` also checks the contracts on
##! accounts of its own making, so each body must keep the invariant and
##! every `ensures`, not only pass the tests below. Run the tests with
##! `sisu test account.sisu`. The hints are in chapter 23 of the tutorial,
##! under "Exercise 23.5: an account that keeps its promises".

## What an overdrawn balance costs, in cents.
const FEE: int = 50

error AccountError:
    Overdrawn(short_by: int)

## An account's balance in cents, which goes below zero as far as `limit`.
struct Account:
    balance: int
    limit: int
    invariant self.limit >= 0
    invariant self.balance >= -self.limit

    ## Puts `cents` in.
    def deposit(mut self, cents: int):
        requires cents > 0
        ensures self.balance == old(self.balance) + cents
        todo("deposit")

    ## Takes `cents` out, and `FEE` too when the balance ends below zero,
    ## or gives `Overdrawn` with the cents missing and changes nothing.
    def withdraw(mut self, cents: int) -> ! AccountError:
        requires cents > 0
        ensures match result:
            case Ok(()): self.balance <= old(self.balance) - cents
            case Err(_): self.balance == old(self.balance)
        return todo("withdraw")

## Moves `cents` from `source` to `target` as a withdrawal and a deposit,
## or changes neither when `source` cannot pay.
def transfer(
    mut source: Account,
    mut target: Account,
    cents: int,
) -> ! AccountError:
    requires cents > 0
    ensures match result:
        case Ok(()): target.balance == old(target.balance) + cents
        case Err(_): source.balance == old(source.balance)
    return todo("transfer")

test "money in and out":
    var a = Account(balance=1000, limit=500)
    a.deposit(250)
    assert a.withdraw(1200) == Ok(())
    assert a.balance == 50

test "below zero costs the fee":
    var a = Account(balance=1000, limit=500)
    assert a.withdraw(1100) == Ok(())
    assert a.balance == -150

test "past the limit is refused":
    var a = Account(balance=1000, limit=500)
    assert a.withdraw(1600) == Err(Overdrawn(short_by=150))
    assert a.balance == 1000
    assert a.withdraw(1450) == Ok(())
    assert a.balance == -500

test "a transfer moves the money":
    var aino = Account(balance=2000, limit=0)
    var eero = Account(balance=100, limit=1000)
    assert transfer(mut aino, mut eero, 1500) == Ok(())
    assert aino.balance == 500 and eero.balance == 1600

test "a transfer that cannot be paid changes nothing":
    var aino = Account(balance=2000, limit=0)
    var eero = Account(balance=100, limit=1000)
    assert transfer(mut eero, mut aino, 1200) == Err(Overdrawn(short_by=150))
    assert aino.balance == 2000 and eero.balance == 100
```

<!-- output: solutions/23-contracts/account.console -->
```console
$ sisu test account.sisu
warning[W0001]: typed hole `deposit` of type `()`
  --> account.sisu:30:9
   |
30 |         todo("deposit")
   |         ^^^^^^^^^^^^^^^

warning[W0001]: typed hole `withdraw` of type `! AccountError`
  --> account.sisu:39:16
   |
39 |         return todo("withdraw")
   |                ^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `transfer` of type `! AccountError`
  --> account.sisu:52:12
   |
52 |     return todo("transfer")
   |            ^^^^^^^^^^^^^^^^

test "money in and out" ... blocked by todo("deposit")
test "below zero costs the fee" ... blocked by todo("withdraw")
test "past the limit is refused" ... blocked by todo("withdraw")
test "a transfer moves the money" ... blocked by todo("transfer")
test "a transfer that cannot be paid changes nothing" ... blocked by todo("transfer")
test "contract of Account.deposit" ... blocked by todo("deposit")
test "contract of Account.withdraw" ... blocked by todo("withdraw")
test "contract of transfer" ... blocked by todo("transfer")
0 passed, 0 failed, 8 blocked
```

<details>
<summary>Hint 1</summary>

The tests show the arithmetic: a withdrawal of 1,100 cents from 1,000
ends at -150, the fee included, and one of 1,600 is short by 150 of the
limit of 500.

</details>

<details>
<summary>Hint 2</summary>

Work out the balance that a withdrawal would leave, fee included,
before you change anything. Then either refuse, with nothing changed,
or make the change.

</details>

<details>
<summary>Hint 3</summary>

A transfer is a withdrawal followed by a deposit, and when the
withdrawal fails the deposit must not happen. The contract tests try
small accounts at their limits, where a check that forgets the fee
breaks the invariant.

</details>

When every test passes, compare your answer with
[`manual/solutions/23-contracts/account.sisu`](../solutions/23-contracts/account.sisu).

## What you learned

- `requires` is checked on entry and blames the caller; `ensures` is
  checked at every exit, with `result` or the result's labels, and
  blames the function. `old(e)` is `e` on entry.
- A failed contract panics with its clause and the values of its parts.
- Contracts call only `pure` functions and change nothing (E0801,
  E0802), so a build can leave them out.
- An `invariant` is checked after construction and after each `mut
  self` method; only the type's own methods change its fields (E0806).
- `type X = T where pred` is a refinement type: a constant is checked
  when it compiles (E0810), any other value with `X.try_from` (E0811
  otherwise), and a refinement of a refinement checks each predicate,
  the base's first.
- `--contracts=all|preconditions|off` sets what `sisu run` checks; a
  release build checks the preconditions, and `sisu test` checks every
  contract.
- `sisu test` adds a `contract of NAME` test for each function with a
  contract, and `sisu doc` shows contracts with the signatures.
- `decreases`, loop invariants and ghost code wait for the verifier
  (E0900).

Chapter 24 is about numbers in depth: integers of each size, arithmetic
that wraps, saturates or checks, conversions that can lose information,
floating point that gives the same bits everywhere, and the exact types
`BigInt`, `Ratio` and `Decimal`.
