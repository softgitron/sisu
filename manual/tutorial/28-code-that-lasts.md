# 28. Code that lasts

A program that others use keeps changing after it works: it grows new
features, its names get better, its data takes new shapes. Each change
can break a program that depends on it. In this chapter you change a
library's functions and errors in ways that keep its users working, see
which changes break them and how the compiler shows it, keep an old
name alive beside a new one, and plan a change with typed holes. You
also see the habits from other languages that the compiler stops, and
what Sisu designs for editions, migrations and published packages. Last
comes the tutorial's final challenge. You need about an hour and a
half.

## Changing a function without breaking its callers

A new parameter with a default leaves every existing call as it was:
the call does not name it, so it gets the default, as chapter 3 showed:

<!-- file: examples/28-code-that-lasts/defaults.sisu -->
```sisu
# A parameter with a default keeps every call written before it working.

## The greeting for `name`, in `language`, which version 2 added.
pub def greeting(name: str, language: str = "fi") -> String:
    match language:
        case "en":
            return "Hello, {name}!"
        case "sv":
            return "Hej, {name}!"
        case _:
            return "Hei, {name}!"

def main():
    print(greeting("Aino"))
    print(greeting("Eero", language="sv"))
```

<!-- output: examples/28-code-that-lasts/defaults.out -->
```text
Hei, Aino!
Hej, Eero!
```

`greeting("Aino")`, written before version 2, still greets in Finnish,
and a new call asks for Swedish by naming the parameter. Adding a
function, a type, a variant to an enum that no caller matches on, or a
parameter with a default, keeps callers compiling. Removing or renaming
anything public, adding a parameter without a default, or changing a
type that callers see breaks them, and needs a new major version of the
package.

A change can also keep every caller compiling and still change what
they get. A default whose value changes, or a precondition that asks
more (chapter 23), is a change of behaviour: the old calls run, and do
something else. Exercise 28.1 is about one.

## Room to grow in a struct

A struct grows the same way. A field with a default, added after the
others, leaves every construction that was written before it as it was:

<!-- file: examples/28-code-that-lasts/fields.sisu -->
```sisu
# A field with a default keeps construction by keyword working.

## A turn in the sauna. Version 2 added `towel`.
@derive(Debug)
struct Turn:
    name: String
    hour: int
    towel: bool = False

def main():
    let aino = Turn(name="Aino", hour=18)
    let eero = Turn(name="Eero", hour=19, towel=True)
    print("{aino:?}")
    print("{eero:?}")
```

<!-- output: examples/28-code-that-lasts/fields.out -->
```text
Turn(name="Aino", hour=18, towel=False)
Turn(name="Eero", hour=19, towel=True)
```

`Turn(name="Aino", hour=18)` was written for version 1 and still
builds a turn, without a towel. A field without a default would break
it, since every construction must then give the field, and a field
added between two others would break each construction by position.
Construct the structs of a library by keyword, and add fields with
defaults at the end.

## Room to grow in a signature

A parameter after `*` can be passed only by its name, as chapter 3
showed. A library that makes its options keyword-only can reorder
them, and add new ones among them, without breaking a call:

<!-- file: examples/28-code-that-lasts/keywords.sisu -->
```sisu
# Keyword-only parameters, after `*`, leave room for the next version.

## The price in cents of a turn of `minutes`; version 2 added `towel`,
## and either version may add more after the `*`.
pub def price(minutes: int, *, per_hour: int = 300, towel: bool = False) -> int:
    var cents = minutes * per_hour // 60
    if towel:
        cents += 150
    return cents

def main():
    print(price(60))
    print(price(90, towel=True))
    print(price(30, per_hour=400))
```

<!-- output: examples/28-code-that-lasts/keywords.out -->
```text
300
600
200
```

Each call names the options it sets, so the call says what it means,
and the order of `per_hour` and `towel` in the signature is the
library's to change.

**Try it.** Pass the options by position: `price(90, 300, True)`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/28-code-that-lasts/keywords_positional.console -->
```console
$ sisu run keywords_positional.sisu
error[E0305]: wrong number of arguments to `price`: expected 1, found 3
  --> keywords_positional.sisu:13:11
   |
13 |     print(price(90, 300, True))
   |           ^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0305 (`type.argument-count`) counts one positional parameter, since
the others follow the `*`. The message does not say that they are
keyword-only; the signature does. A call that names them is the fix,
and it keeps working whatever the next version does to their order.

</details>

> **From Python.** This is Python's `*` in a signature, with the same
> purpose. A Python library warns about a deprecated call when the call
> runs, with `warnings.warn`; Sisu moves the warning to the compiler,
> and its design moves the fix there too.

## What a change can break

An error set is part of a function's signature, and a caller that
matches on it names every error it can get. Here a program shows what
each booking gave:

<!-- file: examples/28-code-that-lasts/errors.sisu -->
```sisu
# A caller's `match` names every error of a library's error set.
error BookingError:
    Taken(hour: int)
    Closed

## Books `hour`, which the sauna has free from 16 to 21, but not 18.
pub def book(hour: int) -> ! BookingError:
    if hour < 16 or hour > 21:
        return Err(Closed)
    if hour == 18:
        return Err(Taken(hour))
    return Ok(())

def main():
    for hour in [17, 18, 23]:
        match book(hour):
            case Ok(()):
                print("{hour}: booked")
            case Err(Taken(h)):
                print("{hour}: {h}:00 is taken")
            case Err(Closed):
                print("{hour}: the sauna is closed")
```

<!-- output: examples/28-code-that-lasts/errors.out -->
```text
17: booked
18: 18:00 is taken
23: the sauna is closed
```

**Try it.** Add a variant `Cleaning(until: int)` to `BookingError`, and
return `Err(Cleaning(until=17))` from `book` for the hour 16.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/28-code-that-lasts/errors_added.console -->
```console
$ sisu run errors_added.sisu
error[E0314]: this `match` does not cover `Err(Cleaning(_))`
  --> errors_added.sisu:19:9
   |
19 |         match book(hour):
   |         ^^^^^^^^^^^^^^^^
  = note: add an arm for `Err(Cleaning(_))`, or a `case _` arm

error[D0006]: the program was not run: errors found (1)
```

E0314 (`type.non-exhaustive`) says that the program's `match` does not
cover the new error. A new error breaks every caller that matches on the
set, which is the price of matches that the compiler checks: a caller
who handled every error before cannot miss the new one by accident. Its
note offers a `case _` arm, which would keep the program compiling and
treat the new error like every other; an arm of its own says what the
program does about it.

</details>

Keep error sets specific to what a function can really fail with, so
that an error is added only when the function has a new way to fail.

## A documented promise

A change of behaviour compiles, so only a test notices it. The examples
in a function's `##` comment, which `sisu test` runs as chapter 3
showed, are the tests a library's users read, and they hold the
library to what its documentation promises:

<!-- file: examples/28-code-that-lasts/promise.sisu -->
```sisu
# A documented example is a promise that `sisu test` keeps.

## The price in cents of a turn of `minutes`, towel included.
##
## ```
## assert price(60) == 450
## assert price(90) == 600
## ```
pub def price(minutes: int, per_hour: int = 300) -> int:
    return minutes * per_hour // 60 + 150
```

<!-- output: examples/28-code-that-lasts/promise.console -->
```console
$ sisu test promise.sisu
test "price (doc example)" ... ok
1 passed, 0 failed, 0 blocked
```

**Try it.** Make the default `per_hour` 350.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/28-code-that-lasts/promise_changed.console -->
```console
$ sisu test promise_changed.sisu
test "price (doc example)" ... FAILED

test "price (doc example)" ... FAILED
panic: assertion failed: price(60) == 450
  left:  500
  right: 450
  at test "price (doc example)" (promise_changed.sisu:6:4)
replay: sisu test --replay 1-f475ea55ed62d394-0.0.1-t0-s0-r0 promise_changed.sisu

0 passed, 1 failed, 0 blocked
```

The documented example fails at its own line of the comment. Either the
change was a mistake, or the library's behaviour changes for every
caller who relied on the default, and the next version must say so,
with a new major version when the change can break them. Updating the
example to make the test pass would hide that from the reader.

</details>

## Keeping an old name

A better name for a public function breaks its callers unless the old
name stays. The old function can call the new one, so that there is one
body to maintain:

<!-- file: examples/28-code-that-lasts/renamed.sisu -->
```sisu
# A renamed function keeps its old name, which calls the new one.

## Degrees Celsius of `fahrenheit` degrees.
pub def celsius_of(fahrenheit: f64) -> f64:
    return (fahrenheit - 32.0) * 5.0 / 9.0

## The old name of `celsius_of`, kept so that callers keep working.
@deprecated("use `celsius_of`, which this calls")
pub def to_celsius(fahrenheit: f64) -> f64:
    return celsius_of(fahrenheit)

def main():
    print(to_celsius(212.0))
    print(celsius_of(176.0))
```

<!-- output: examples/28-code-that-lasts/renamed.console -->
```console
$ sisu run renamed.sisu
warning[W0023]: `to_celsius` is deprecated: use `celsius_of`, which this calls
  --> renamed.sisu:13:11
   |
13 |     print(to_celsius(212.0))
   |           ^^^^^^^^^^^^^^^^^

100.0
80.0
```

`@deprecated("...")` marks the old name, with a message saying what to
use instead. Attributes are data, as chapter 25 showed, and the
compiler reads this one: each call of a deprecated function gets W0023
(`type.deprecated`), a warning with the attribute's message, and the
program still runs. A library will also mark such a wrapper
`@fix_inline`, so that `sisu fix --deps` rewrites each caller to call
the new function itself (D-067); until `sisu fix` exists, the warning
tells each caller what to change.

## Habits the compiler stops

Some spellings mean one thing in Python and another in Sisu, or nothing
at all. Sisu does not guess: such a habit does not compile, and the
diagnostic names the Sisu spelling (D-062). Here are three, in a program
written as a Python programmer would write it:

<!-- file: examples/28-code-that-lasts/habits.sisu -->
```sisu
# Habits from Python that mean something else in Sisu do not compile.

## The last of `temps`, and whether it is hot.
def last(temps: list[int]) -> (int, str):
    let t = temps[-1]
    let feel = "hot" if t > 80 else "warm"
    return (t, feel)

## The first reading above 90, or -1.
def first_hot(temps: list[int]) -> int:
    for t in temps:
        if t > 90:
            return t
    else:
        return -1

def main():
    print(last([70, 85]))
    print(first_hot([70, 95]))
```

<!-- output: examples/28-code-that-lasts/habits.console -->
```console
$ sisu run habits.sisu
error[E0344]: a constant negative index does not compile; count from the end with `^`: `temps[^1]`
  --> habits.sisu:5:19
  |
5 |     let t = temps[-1]
  |                   ^^
  = fix: count from the end with `^`: `^1`

error[E0115]: `a if c else b` is not Sisu; write `if c: a else: b`
  --> habits.sisu:6:16
  |
6 |     let feel = "hot" if t > 80 else "warm"
  |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: write it as `if c: a else: b`: `if t > 80: "hot" else: "warm"`

error[E0127]: a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:`
  --> habits.sisu:14:5
   |
14 |     else:
   |     ^^^^

error[D0006]: the program was not run: errors found (3)
```

- E0344 (`type.constant-negative-index`): `temps[-1]`, Python's last
  element, is `temps[^1]` in Sisu. Its fix writes it.
- E0115 (`parse.python-conditional`): Python's conditional expression
  becomes `if t > 80: "hot" else: "warm"`, and the fix writes that too.
- E0127 (`parse.loop-else`): a loop's `else` runs, in Python, when the
  loop ends without `break`, which readers often get wrong. Sisu has no
  such `else`: keep what the loop found in a variable, and test it after
  the loop.

The reference keeps the whole list of such spellings, from Python and
from Rust, with the diagnostic that catches each:
[false friends](../reference/false-friends.md). A program that passes
the compiler never means what the habit would have meant somewhere else.

## Planning a change with typed holes

A larger change starts with its signatures. Write each new function with
its doc comment and its type, give it a typed hole for a body, and write
its tests. The program compiles, and `sisu test` shows what is done and
what is still to do:

<!-- file: examples/28-code-that-lasts/plan.sisu -->
```sisu
# A change planned with typed holes: the signatures first, then bodies.

## A turn in the sauna: who, from which minute after midnight, how long.
@derive(Eq, Debug)
struct Slot:
    name: String
    start: int
    minutes: int

## Whether `a` and `b` share a minute.
pub def overlap(a: Slot, b: Slot) -> bool:
    return a.start < b.start + b.minutes and b.start < a.start + a.minutes

## The slot that `line` writes as "NAME HH:MM MINUTES".
pub def parse_slot(line: str) -> Slot?:
    return todo("parse a slot")

## The slots of `text`, a line each, that overlap none before them.
pub def free_slots(text: str) -> list[Slot]:
    return todo("keep the slots that fit")

test "slots that overlap":
    let aino = Slot(name="Aino", start=17 * 60, minutes=90)
    let eero = Slot(name="Eero", start=18 * 60, minutes=60)
    assert overlap(aino, eero)

test "a slot from text":
    let want = Slot(name="Aino", start=17 * 60 + 30, minutes=90)
    assert parse_slot("Aino 17:30 90") == Some(want)

test "the slots that fit":
    let slots = free_slots("Aino 17:00 90\nEero 18:00 60\nRoni 18:30 30")
    assert len(slots) == 2 and slots[1].name == "Roni"
```

<!-- output: examples/28-code-that-lasts/plan.console -->
```console
$ sisu check plan.sisu
warning[W0001]: typed hole `parse a slot` of type `Slot?`
  --> plan.sisu:16:12
   |
16 |     return todo("parse a slot")
   |            ^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `keep the slots that fit` of type `list[Slot]`
  --> plan.sisu:20:12
   |
20 |     return todo("keep the slots that fit")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

$ sisu test plan.sisu
in scope at the typed hole `todo("parse a slot")`:
  line = "Aino 17:30 90"
in scope at the typed hole `todo("keep the slots that fit")`:
  text = "Aino 17:00 90\nEero 18:00 60\nRoni 18:30 30"
test "slots that overlap" ... ok
test "a slot from text" ... blocked by todo("parse a slot")
test "the slots that fit" ... blocked by todo("keep the slots that fit")
1 passed, 0 failed, 2 blocked
```

- Each hole gets a warning, W0001 (`type.hole`), with its name and the
  type its body must give: the list of what remains.
- `overlap` is done, and its test passes. The tests of `parse_slot` and
  `free_slots` are blocked, not failed: they reach a hole, so they say
  nothing yet about the code that will fill it. Each shows what was in
  scope at its hole, the text the code will work on.
- You can fill the holes in any order, and each one you fill turns its
  tests from blocked to passed or failed.

This is how a library's next version can grow beside its old one: the
new functions start as holes with tests, while the old ones keep their
callers working. `sisu check --holes`, which will list every hole of a
package with the functions that reach it, is designed in
[10](../../proposal/10-tooling.md) and not in the compiler yet.

## What is still to come

Much of what keeps code working for years is designed and not in the
compiler. Editions, a new edition every three years with code of every
edition compiling together, and `sisu fix --edition`, which moves a
package to the next one, are the compatibility promise of D-036; the
manifest's `edition` key is read and checked for nothing yet.
`@preview` features, which a program asks for by name and which become
stable or are removed within four releases, are D-072's. `sisu publish`
will compare a package's public API with the version before it and
refuse a breaking change without a new major version, counting changes
of behaviour, such as a stronger precondition, too (D-031); `sisu
review` will report the specifications and the authority that a new
version of a dependency weakens or adds (D-066); and `sisu why` will say
why the compiler decided what it decided. None of these commands is in
the `sisu` command yet.

## Exercise 28.1: predict what the old calls give

Open
[`manual/exercises/28-code-that-lasts/predict_versions.sisu`](../exercises/28-code-that-lasts/predict_versions.sisu):

<!-- file: exercises/28-code-that-lasts/predict_versions.sisu -->
```sisu
##! Exercise 28.1, predict what the old calls give. Kind: predict. Level: 1.
##!
##! Version 1 of `price` took the minutes of a turn and charged 300 cents
##! an hour. Version 2 added two parameters with defaults. Each constant
##! is your prediction of what a call gives in version 2. Write each as a
##! literal, then run `sisu test predict_versions.sisu`. The hints are in
##! chapter 28 of the tutorial, under "Exercise 28.1: predict what the old
##! calls give".

## The price in cents of a turn of `minutes` at `per_hour` cents an
## hour, and of a towel, 150 cents, when `towel` is true.
pub def price(minutes: int, per_hour: int = 350, towel: bool = True) -> int:
    var cents = minutes * per_hour // 60
    if towel:
        cents += 150
    return cents

const AN_HOUR: int = todo("price(60), written for version 1")
const LONGER: int = todo("price(90), written for version 1")
const AS_BEFORE: int = todo("price(60, per_hour=300, towel=False)")
const SHORT: int = todo("price(30, towel=False)")

test "an hour, as version 1 users ask for it":
    assert price(60) == AN_HOUR

test "an hour and a half":
    assert price(90) == LONGER

test "version 1's price, asked for in version 2":
    assert price(60, per_hour=300, towel=False) == AS_BEFORE

test "half an hour without a towel":
    assert price(30, towel=False) == SHORT
```

<!-- output: solutions/28-code-that-lasts/predict_versions.console -->
```console
$ sisu test predict_versions.sisu
test "an hour, as version 1 users ask for it" ... blocked by todo("price(60), written for version 1")
test "an hour and a half" ... blocked by todo("price(90), written for version 1")
test "version 1's price, asked for in version 2" ... blocked by todo("price(60, per_hour=300, towel=False)")
test "half an hour without a towel" ... blocked by todo("price(30, towel=False)")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

A call of version 1 names only `minutes`, so both new parameters take
their defaults. Look at what those defaults are, and at the price version
1 charged.

</details>

<details>
<summary>Hint 2</summary>

The prices are whole cents, `minutes * per_hour // 60`, and the towel
adds 150 when `towel` is true. Every call still compiles; whether it
still gives what it gave is the question.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/28-code-that-lasts/predict_versions.sisu`](../solutions/28-code-that-lasts/predict_versions.sisu).

## Exercise 28.2: a new error in a library

Open
[`manual/exercises/28-code-that-lasts/new_error.sisu`](../exercises/28-code-that-lasts/new_error.sisu):

<!-- file: exercises/28-code-that-lasts/new_error.sisu -->
```sisu
##! Exercise 28.2, a new error in a library. Kind: fix. Level: 1.
##!
##! Version 2 of a sauna's booking library added `Cleaning` to its error
##! set, for the hour the stove is cleaned, and `describe`, written for
##! version 1, no longer compiles. Run `sisu test new_error.sisu`, read
##! the diagnostic, and fix `describe`, not the library or the tests, so
##! that every test passes. The hints are in chapter 28 of the tutorial,
##! under "Exercise 28.2: a new error in a library".

# The library, version 2.

error BookingError:
    Taken(hour: int)
    Closed
    Cleaning(until: int)

## Books `hour`. The sauna is open from 16 to 21; 18 is taken, and the
## stove is cleaned at 16.
pub def book(hour: int) -> ! BookingError:
    if hour < 16 or hour > 21:
        return Err(Closed)
    if hour == 18:
        return Err(Taken(hour))
    if hour == 16:
        return Err(Cleaning(until=17))
    return Ok(())

# The program that uses it, written for version 1.

## What booking `hour` gave, as a line for the screen.
def describe(hour: int) -> String:
    match book(hour):
        case Ok(()):
            return "{hour}:00 is yours"
        case Err(Taken(h)):
            return "{h}:00 is taken"
        case Err(Closed):
            return "the sauna is closed at {hour}:00"

test "a free hour":
    assert describe(19) == "19:00 is yours"

test "a taken hour and a closed one":
    assert describe(18) == "18:00 is taken"
    assert describe(23) == "the sauna is closed at 23:00"

test "the hour of cleaning":
    assert describe(16) == "the stove is being cleaned until 17:00"
```

<!-- output: solutions/28-code-that-lasts/new_error.console -->
```console
$ sisu test new_error.sisu
error[E0314]: this `match` does not cover `Err(Cleaning(_))`
  --> new_error.sisu:32:5
   |
32 |     match book(hour):
   |     ^^^^^^^^^^^^^^^^
  = note: add an arm for `Err(Cleaning(_))`, or a `case _` arm

```

<details>
<summary>Hint 1</summary>

E0314 names the error the `match` misses. The library's comment says
when it happens.

</details>

<details>
<summary>Hint 2</summary>

A `case _` arm would compile, and could not give the hour until which
the stove is cleaned. An arm for the new error can bind its field.

</details>

When every test passes, compare your answer with
[`manual/solutions/28-code-that-lasts/new_error.sisu`](../solutions/28-code-that-lasts/new_error.sisu).

## Challenge V: the next version

The tutorial's last challenge moves a small library to a new format of
its data, in three parts with a suite of tests each, without breaking
the programs that use its first version. Open
[`manual/exercises/28-code-that-lasts/next_version.sisu`](../exercises/28-code-that-lasts/next_version.sisu):

<!-- file: exercises/28-code-that-lasts/next_version.sisu -->
```sisu
##! Exercise 28.3, Challenge V: the next version. Kind: challenge. Level: 3.
##!
##! Programs that you cannot change use a sauna's booking library. Its
##! version 1 reads lines such as "Aino 18": a name and a whole hour.
##! Version 2 must read slots such as "Aino 18:30 90": a name, a start and
##! a length in minutes. Solve the parts in order, and do not break the
##! programs written for version 1: part 2's tests hold their code, which
##! must keep compiling and working. Run the tests with
##! `sisu test next_version.sisu`.
##!
##! Part 1. Write `parse_slot`, version 2's reader of "NAME HH:MM MINUTES".
##! A line of fewer than three parts gives `MissingField`, a time that is
##! not on the clock `BadTime`, and a length that is not a number above 0
##! `BadLength`.
##!
##! Part 2. Files of version 1 must still be read, and programs of version
##! 1 must read lines of version 2. `parse_slot` takes a line of version 1,
##! two parts with no `:` in the second, as a slot of 60 minutes from the
##! hour. `parse_booking` takes a line of version 2 whose slot is a whole
##! hour, from HH:00 for 60 minutes, and gives `BadHour` for any other
##! line of three parts. `BookingError` and `Booking` must not change.
##!
##! Part 3. `migrate` rewrites a file of lines of either version as lines
##! of version 2, in order, each written "NAME HH:MM MINUTES" with two
##! digits for the hour and two for the minute. The first line it cannot
##! read stops it with that line's error.
##!
##! The hints are in chapter 28 of the tutorial, under "Challenge V: the
##! next version".

# Version 1, which programs use.

error BookingError:
    NoHour(line: String)
    BadHour(line: String)

## A booking of version 1: a name and a whole hour.
@derive(Eq, Debug)
struct Booking:
    name: String
    hour: int

## The booking that `line` writes as "NAME HOUR".
pub def parse_booking(line: str) -> Booking ! BookingError:
    let name, hour = line.split_once(" ") else:
        return Err(NoHour(line.to_string()))
    let h = hour.parse_int() else:
        return Err(BadHour(line.to_string()))
    if h < 0 or h > 23:
        return Err(BadHour(line.to_string()))
    return Ok(Booking(name=name.to_string(), hour=h))

# Version 2.

error SlotError:
    MissingField(line: String)
    BadTime(line: String)
    BadLength(line: String)

## A slot of version 2: a name, a start in minutes after midnight and a
## length in minutes.
@derive(Eq, Debug)
struct Slot:
    name: String
    start: int
    minutes: int

## The slot that `line` writes as "NAME HH:MM MINUTES".
pub def parse_slot(line: str) -> Slot ! SlotError:
    return todo("parse_slot")

## The lines of `text`, of either version, as lines of version 2.
pub def migrate(text: str) -> String ! SlotError:
    return todo("migrate")

suite "part 1":
    test "a slot":
        let aino = Slot(name="Aino", start=18 * 60 + 30, minutes=90)
        assert parse_slot("Aino 18:30 90") == Ok(aino)
        let roni = Slot(name="Roni", start=7 * 60 + 5, minutes=30)
        assert parse_slot("Roni 07:05 30") == Ok(roni)

    test "a line with a part missing":
        assert parse_slot("Aino") == Err(MissingField("Aino"))
        assert parse_slot("Aino 18:30") == Err(MissingField("Aino 18:30"))

    test "a time that is not on the clock":
        assert parse_slot("Aino 25:00 60") == Err(BadTime("Aino 25:00 60"))
        assert parse_slot("Aino 18:75 60") == Err(BadTime("Aino 18:75 60"))
        assert parse_slot("Aino noon 60") == Err(BadTime("Aino noon 60"))

    test "a length that is not a length":
        assert parse_slot("Aino 18:00 0") == Err(BadLength("Aino 18:00 0"))
        assert parse_slot("Aino 18:00 x") == Err(BadLength("Aino 18:00 x"))

suite "part 2":
    test "a line of version 1 is a slot of an hour":
        let eero = Slot(name="Eero", start=19 * 60, minutes=60)
        assert parse_slot("Eero 19") == Ok(eero)
        assert parse_slot("Eero 24") == Err(BadTime("Eero 24"))

    test "a program of version 1 reads lines of version 2":
        let shown = match parse_booking("Eero 19:00 60"):
            case Ok(b): "{b.name} at {b.hour}"
            case Err(NoHour(line)): "no hour in {line}"
            case Err(BadHour(line)): "a bad hour in {line}"
        assert shown == "Eero at 19"
        assert parse_booking("Eero 19") == Ok(Booking(name="Eero", hour=19))

    test "a slot that is not a whole hour":
        assert parse_booking("Eero 19:00 60").is_ok()
        assert parse_booking("Eero 19:30 60") == Err(BadHour("Eero 19:30 60"))
        assert parse_booking("Eero 19:00 90") == Err(BadHour("Eero 19:00 90"))
        assert parse_booking("Eero") == Err(NoHour("Eero"))

suite "part 3":
    test "a file of both versions":
        let text = "Aino 17\nEero 18:30 45\nRoni 9"
        let want = "Aino 17:00 60\nEero 18:30 45\nRoni 09:00 60".to_string()
        assert migrate(text) == Ok(want)

    test "a line that cannot be read stops it":
        let text = "Aino 17\nEero 25:00 45\nRoni 9"
        assert migrate(text) == Err(BadTime("Eero 25:00 45"))

    test "an empty file":
        assert migrate("") == Ok("")
```

<!-- output: solutions/28-code-that-lasts/next_version.console -->
```console
$ sisu test next_version.sisu
in scope at the typed hole `todo("parse_slot")`:
  line = "Aino 18:30 90"
in scope at the typed hole `todo("parse_slot")`:
  line = "Aino"
in scope at the typed hole `todo("parse_slot")`:
  line = "Aino 25:00 60"
in scope at the typed hole `todo("parse_slot")`:
  line = "Aino 18:00 0"
in scope at the typed hole `todo("parse_slot")`:
  line = "Eero 19"
in scope at the typed hole `todo("migrate")`:
  text = "Aino 17\nEero 18:30 45\nRoni 9"
in scope at the typed hole `todo("migrate")`:
  text = "Aino 17\nEero 25:00 45\nRoni 9"
in scope at the typed hole `todo("migrate")`:
  text = ""
test "part 1/a slot" ... blocked by todo("parse_slot")
test "part 1/a line with a part missing" ... blocked by todo("parse_slot")
test "part 1/a time that is not on the clock" ... blocked by todo("parse_slot")
test "part 1/a length that is not a length" ... blocked by todo("parse_slot")
test "part 2/a line of version 1 is a slot of an hour" ... blocked by todo("parse_slot")
test "part 2/a program of version 1 reads lines of version 2" ... FAILED
test "part 2/a slot that is not a whole hour" ... FAILED
test "part 3/a file of both versions" ... blocked by todo("migrate")
test "part 3/a line that cannot be read stops it" ... blocked by todo("migrate")
test "part 3/an empty file" ... blocked by todo("migrate")

test "part 2/a program of version 1 reads lines of version 2" ... FAILED
panic: assertion failed: shown == "Eero at 19"
    shown == "Eero at 19"
    │
    "a bad hour in Eero 19:00 60"
  at test "part 2/a program of version 1 reads lines of version 2" (next_version.sisu:107:9)
replay: sisu test --replay 1-657429893577fb66-0.0.1-t5-s0-r0 next_version.sisu

test "part 2/a slot that is not a whole hour" ... FAILED
panic: assertion failed: parse_booking("Eero 19:00 60").is_ok()
  at test "part 2/a slot that is not a whole hour" (next_version.sisu:111:9)
replay: sisu test --replay 1-657429893577fb66-0.0.1-t6-s0-r0 next_version.sisu

0 passed, 2 failed, 8 blocked
```

The two tests of part 2 that fail call `parse_booking`, which version 1
already has, with lines of version 2.

<details>
<summary>Hint 1</summary>

Part 1: split the line into its parts first, and count them before you
read any. A small function that reads "HH:MM" into minutes after
midnight, or gives `None`, keeps `parse_slot` short, as the section
"Planning a change with typed holes" suggests: its signature first.

</details>

<details>
<summary>Hint 2</summary>

Part 2: let `parse_slot` read both versions, telling them apart by the
number of parts and the colon, so that there is one reader of slots.
Then `parse_booking` can hand a line of three parts to `parse_slot` and
turn its answer into a `Booking`, or into `BadHour`, never into a new
kind of error: a program of version 1 matches on exactly two.

</details>

<details>
<summary>Hint 3</summary>

Part 3: `migrate` reads each line with `parse_slot`, which already knows
both versions, and writes it again; `?` stops at the first line that it
cannot read. Two digits for the hour and the minute are a format spec,
as chapter 6 showed.

</details>

When every test passes, compare your answer with
[`manual/solutions/28-code-that-lasts/next_version.sisu`](../solutions/28-code-that-lasts/next_version.sisu).

## What you learned

- A parameter with a default, a new function or a new type keeps every
  caller compiling; removing, renaming or changing what callers use
  breaks them. A change of a default or a contract can keep them
  compiling and change what they get.
- A new error in an error set breaks each caller's `match` (E0314),
  which is how the compiler makes sure that no caller misses it.
- An old name can stay as a wrapper over the new one; `@deprecated`
  makes each call warn with its message (W0023), and `@fix_inline` will
  let `sisu fix` rewrite the callers.
- Habits from Python that would mean something else do not compile, and
  the diagnostics name the Sisu spelling (E0344, E0115, E0127).
- Typed holes plan a change: the program compiles, W0001 lists what is
  left, and the tests that reach a hole are blocked.

That is the end of the tutorial. You can write a program in Sisu from
its first line to its native build: values and functions, ownership,
traits and generics, capabilities, tests of every kind, tasks, the
standard library, contracts, numbers, compile-time code, data
parallelism, unsafe code and code that lasts. From here, the reference
describes each part of the language completely, the guides do one task
each, and the explanations say why Sisu is the way it is;
[SUMMARY.md](../SUMMARY.md) lists every page, and the manual's plan,
[README.md](../README.md), says what each one covers.
