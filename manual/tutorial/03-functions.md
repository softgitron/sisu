# 3. Functions

In this chapter you write your own functions. You give them parameters
and a result, call them with arguments by position and by name, return
several values at once, choose a value with `if`, and let a function
call itself. Then you check what a function assumes with `assert`,
document it with an example that runs as a test, and leave a hole in a
function you have not finished. You need about an hour.

## Defining a function

A function has a name, parameters and, when it gives a value back, a
result type:

<!-- file: examples/03-functions/area.sisu -->
```sisu
# A function with two parameters and a result.
def area(width: f64, height: f64) -> f64:
    return width * height

def main():
    let floor = area(4.5, 3.0)
    print("the floor is {floor} square metres")
    print("the door is {area(0.8, 2.0)} square metres")
```

`sisu run area.sisu` prints:

<!-- output: examples/03-functions/area.out -->
```text
the floor is 13.5 square metres
the door is 1.6 square metres
```

- `def area(width: f64, height: f64) -> f64:` is the function's
  signature. Each parameter has a name and a type, and the type after
  `->` is the type of the result.
- `return width * height` gives the result back to the caller.
- A call passes one argument for each parameter, in order:
  `area(4.5, 3.0)`. A call is an expression, so it can stand anywhere a
  value can, inside braces in a string too.

The types in a signature are always written. Inside a body Sisu works
out the types of `let` bindings, as chapter 2 showed, but it never reads
a body to find out the types of a signature. A signature therefore says
everything a caller needs, and a mistake inside a body is reported
inside that body rather than at some distant call.

Functions can stand in any order in a file: `main` can call a function
that comes after it.

## Every value leaves through return

A function with a result gives it back only with `return`. The value of
the last line is not the result, as it is in some languages, and a
function whose end can be reached without a `return` does not compile:

<!-- file: examples/03-functions/no_return.sisu -->
```sisu
# A function with a result must return on every path.
def sign(n: int) -> int:
    if n > 0:
        return 1
    elif n < 0:
        return -1

def main():
    print("{sign(-5)}")
```

<!-- output: examples/03-functions/no_return.console -->
```console
$ sisu run no_return.sisu
error[E0315]: `sign` returns `int`, but control can reach the end of its body without a `return`
  --> no_return.sisu:2:5
  |
2 | def sign(n: int) -> int:
  |     ^^^^
  = note: return a value on that path too, or end the body with a `return`

error[D0006]: the program was not run: errors found (1)
```

`sign` returns nothing when `n` is 0: neither condition holds, and
control reaches the end of the body. E0315 (`type.missing-return`) says
so and names the function. The fix is a `return` on that path, here
`return 0` as the body's last line.

A function without `->` returns nothing. A `return` without a value
leaves it early:

<!-- file: examples/03-functions/report.sisu -->
```sisu
# A function without `->` returns nothing; `return` can leave it early.
def report(reading: f64):
    if reading < 0.0:
        print("no reading")
        return
    print("{reading} degrees")

def main():
    report(21.5)
    report(-1.0)
```

<!-- output: examples/03-functions/report.out -->
```text
21.5 degrees
no reading
```

> **From Python.** A Python function that reaches its end without
> `return` returns `None`, and the caller finds out when it uses the
> `None`. In Sisu such a function does not compile, unless it was
> declared to return nothing.

## Default values

A parameter can have a default value, written after `=`. A call can
leave such an argument out:

<!-- file: examples/03-functions/defaults.sisu -->
```sisu
# A parameter with a default value can be left out of a call.
def greet(name: str, mark: str = "."):
    print("Hei, {name}{mark}")

def main():
    greet("Aino")
    greet("Eero", "!")
```

<!-- output: examples/03-functions/defaults.out -->
```text
Hei, Aino.
Hei, Eero!
```

A default is an expression, and Sisu works it out at each call that
leaves the argument out, not once when the function is defined. This
program makes that visible with a default that prints:

<!-- file: examples/03-functions/default_each_call.sisu -->
```sisu
# A default value is computed at each call that leaves it out.
def guest_name() -> str:
    print("(working out the default)")
    return "guest"

def greet(name: str = guest_name()):
    print("Hei, {name}!")

def main():
    greet()
    greet("Aino")
    greet()
```

**Predict.** Decide how many times `(working out the default)` appears:
once, twice or three times. Then run `sisu run default_each_call.sisu`:

<!-- output: examples/03-functions/default_each_call.out -->
```text
(working out the default)
Hei, guest!
Hei, Aino!
(working out the default)
Hei, guest!
```

> **From Python.** Python works out a default once, when the `def` runs,
> so a default list is shared by every call that leaves it out, and what
> one call adds to it the next call sees. Sisu works out a default at
> each call, so each call gets a fresh value.

## Passing arguments by name

An argument can be passed by the name of its parameter, as `name=value`.
Named arguments come after the positional ones, in any order. Two
markers in a parameter list limit how a caller passes arguments:

<!-- file: examples/03-functions/keywords.sisu -->
```sisu
# Arguments can be passed by name; `*` and `/` limit how.
def url(host: str, *, port: int = 80, secure: bool = False) -> String:
    let scheme = if secure: "https" else: "http"
    return "{scheme}://{host}:{port}"

def clamp(value: int, /, low: int, high: int) -> int:
    return max(low, min(value, high))

def main():
    print(url("example.org"))
    print(url("example.org", secure=True, port=443))
    print("{clamp(15, 0, 10)} {clamp(-3, low=0, high=10)}")
```

<!-- output: examples/03-functions/keywords.out -->
```text
http://example.org:80
https://example.org:443
10 0
```

- A bare `*` makes the parameters after it keyword-only: a call must
  name them. In `url("example.org", secure=True, port=443)` the reader
  sees what `443` and `True` are for.
- A bare `/` makes the parameters before it positional-only: a call
  cannot name them, so `clamp(value=15, low=0, high=10)` does not
  compile. The function can then rename `value` without breaking any
  call.
- Without either marker, a parameter can be passed both ways, as
  `clamp(-3, low=0, high=10)` shows for `low` and `high`.

**Try it.** In `keywords.sisu`, pass the port by position:
`url("example.org", 443)`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/03-functions/positional_port.console -->
```console
$ sisu run positional_port.sisu
error[E0305]: wrong number of arguments to `url`: expected 1, found 2
  --> positional_port.sisu:7:11
  |
7 |     print(url("example.org", 443))
  |           ^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0305 (`type.argument-count`) counts only the parameters that take a
position, and `url` has one, `host`. It does not say that `port` is
keyword-only; the fix is to name it, `port=443`.

</details>

A parameter that collects any number of arguments, written `*prices`,
is designed in [02](../../proposal/02-syntax.md#declarations) and is
not in the compiler yet.

One name means one function. Sisu has no overloading by type or by the
number of parameters (D-050), so a second function with the same name is
an error:

<!-- file: examples/03-functions/two_areas.sisu -->
```sisu
# Two functions in one module cannot share a name.
def area(side: f64) -> f64:
    return side * side

def area(width: f64, height: f64) -> f64:
    return width * height

def main():
    print("{area(2.0)}")
```

<!-- output: examples/03-functions/two_areas.console -->
```console
$ sisu run two_areas.sisu
error[E0202]: `area` is defined more than once in this module
  --> two_areas.sisu:5:5
  |
5 | def area(width: f64, height: f64) -> f64:
  |     ^^^^
  |
2 | def area(side: f64) -> f64:
  |     ---- first defined here
  = note: rename one of them, or remove the one not wanted

error[D0006]: the program was not run: errors found (1)
```

E0202 (`resolve.duplicate-definition`) points at both definitions. Give
each function a name that says what it does, such as `square_area` and
`rectangle_area`, or give one function a parameter with a default value.

## Returning several values

A function can return several values as a tuple. Labels on the result
type name each value:

<!-- file: examples/03-functions/divide.sisu -->
```sisu
# A function can return several values as a labelled tuple.
def divide(total: int, people: int) -> (each: int, left_over: int):
    return (total // people, total % people)

def main():
    let split = divide(17, 5)
    print("{split.each} each, {split.left_over} left over")
    let each, left_over = divide(100, 7)
    print("{each=} {left_over=}")
```

<!-- output: examples/03-functions/divide.out -->
```text
3 each, 2 left over
each=14 left_over=2
```

- `-> (each: int, left_over: int)` is a labelled tuple: two values, each
  with a name. The labels are part of the signature, so they document
  what the function returns.
- `return (total // people, total % people)` builds the tuple, in the
  order of the labels.
- `split.each` reads one value by its label.
- `let each, left_over = divide(100, 7)` unpacks the tuple into two
  bindings, in order. Unpacking ignores the labels.

Chapter 5 covers tuples in general, with and without labels.

## Choosing a value with if

`if` can choose between values when each branch is a single expression:

<!-- file: examples/03-functions/if_expression.sisu -->
```sisu
# `if` gives a value when each branch is one expression.
def describe(n: int) -> str:
    return if n > 0: "positive" elif n < 0: "negative" else: "zero"

def main():
    let temperature = -4
    print("{temperature} is {describe(temperature)}")
    let clothes = if temperature < 0: "a warm coat" else: "a jacket"
    print("wear {clothes}")
```

<!-- output: examples/03-functions/if_expression.out -->
```text
-4 is negative
wear a warm coat
```

An `if` expression is written on one line, with `elif` branches if it
needs them, and it always ends with an `else` branch, because it must
have a value whichever condition holds. A branch that needs more than one
expression makes the `if` a statement, as in `sign` in the section
"Every value leaves through return", and each branch then returns or
assigns its value.

**Try it.** Delete `else: "zero"` from `describe` and run the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/03-functions/no_else.console -->
```console
$ sisu run no_else.sisu
error[E0116]: an `if` expression needs an `else` branch, because it must have a value
  --> no_else.sisu:3:55
  |
3 |     return if n > 0: "positive" elif n < 0: "negative"
  |                                                       ^

error[D0006]: the program was not run: errors found (1)
```

E0116 (`parse.if-expression-else`) points at the end of the expression,
where the `else` branch is missing.

</details>

> **From Python.** Python's conditional expression, `a if c else b`,
> puts the condition after the first value. Sisu uses its one `if` for
> both statements and expressions.

Write the Python form, and the diagnostic rewrites it for you:

<!-- output: examples/03-functions/python_if.console -->
```console
$ sisu run python_if.sisu
error[E0115]: `a if c else b` is not Sisu; write `if c: a else: b`
  --> python_if.sisu:4:19
  |
4 |     let clothes = "a warm coat" if temperature < 0 else "a jacket"
  |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: write it as `if c: a else: b`: `if temperature < 0: "a warm coat" else: "a jacket"`

error[D0006]: the program was not run: errors found (1)
```

## Functions that call themselves

A function can call itself. This one finds the greatest common divisor
of two numbers with Euclid's algorithm:

<!-- file: examples/03-functions/gcd.sisu -->
```sisu
# A function can call itself.
def gcd(a: int, b: int) -> int:
    if b == 0:
        return a
    return gcd(b, a % b)

def main():
    print("{gcd(48, 18)}")
    print("{gcd(17, 5)}")
```

<!-- output: examples/03-functions/gcd.out -->
```text
6
1
```

`gcd(48, 18)` calls `gcd(18, 12)`, which calls `gcd(12, 6)`, which
calls `gcd(6, 0)`, which returns 6 without calling anything. A function
that calls itself needs such a case, one that returns without calling
again, and each call must come closer to it.

## Checking with assert

`assert` states something that must be true at a point in a program. If
it is false, the program stops with a panic:

<!-- file: examples/03-functions/percent.sisu -->
```sisu
# `assert` stops the program when something that must hold does not.
def percent(part: int, whole: int) -> f64:
    assert whole > 0, "a percentage of nothing"
    return part.to_f64() * 100.0 / whole.to_f64()

def main():
    print("{percent(1, 4)}")
    print("{percent(1, 0)}")
```

<!-- output: examples/03-functions/percent.console -->
```console
$ sisu run percent.sisu
25.0
panic: assertion failed: whole > 0
  message: a percentage of nothing
    whole > 0
    │
    0
  at percent (percent.sisu:3:5)
  at main (percent.sisu:8:13)
```

The report shows the condition, the message after the comma, the value
of each name in the condition (here `whole` is 0), and the calls that
led there: `percent`, called from `main` on line 8. The message is
optional, and it can interpolate values like any string.

An `assert` that fails means the program has a bug, here in the call
that asked for a percentage of nothing. It is not a way to report bad
input from a user; chapter 8 shows that way. `assert` is the same
statement that judges tests, and it is always checked.

> **From Python.** `python -O` removes every `assert`. A Sisu `assert`
> is checked in every build of a program.

## Documentation that runs

A comment that starts with `##` documents the function after it. An
example inside it, between two lines of three backticks, is a test:

<!-- file: examples/03-functions/documented.sisu -->
```sisu
# A doc comment documents a function, and its example is a test.

## The price of `quantity` items at `unit_price` cents each, with a
## tenth off when `quantity` is 10 or more.
##
## ```
## assert price(3, 250) == 750
## assert price(10, 100) == 900
## ```
def price(quantity: int, unit_price: int) -> int:
    let full = quantity * unit_price
    if quantity >= 10:
        return full - full // 10
    return full

test "one item":
    assert price(1, 199) == 199
```

This file has no `main`, so it is not a program to run; `sisu test
documented.sisu` runs its test and its doc example:

<!-- output: examples/03-functions/documented.test -->
```text
test "one item" ... ok
test "price (doc example)" ... ok
2 passed, 0 failed, 0 blocked
```

The doc example runs as a test of its own, named after the function.
The example shows a reader how to call `price`, and because `sisu test`
runs it, it stays true when the function changes. `sisu doc` turns the
`##` comments into a page of documentation, which chapter 17 covers.

## Leaving a hole

Exercises put a typed hole, `todo("...")`, where your answer goes. Your
own code can use one too, to write a function in steps. The hole checks
as whatever type its place needs, so the rest of the program compiles
and runs until it reaches the hole:

<!-- file: examples/03-functions/shipping.sisu -->
```sisu
# A typed hole lets the program run before every part is written.
def shipping(weight_kg: f64) -> f64:
    if weight_kg <= 2.0:
        return 4.9
    return todo("the price of a heavy parcel")

def main():
    print("light: {shipping(1.5)}")
    print("heavy: {shipping(7.0)}")
```

<!-- output: examples/03-functions/shipping.console -->
```console
$ sisu run shipping.sisu
warning[W0001]: typed hole `the price of a heavy parcel` of type `f64`
  --> shipping.sisu:5:12
  |
5 |     return todo("the price of a heavy parcel")
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

light: 4.9
panic: reached the typed hole `todo("the price of a heavy parcel")`
  at shipping (shipping.sisu:5:12)
  at main (shipping.sisu:9:20)
```

The warning names the hole and its type, `f64`, which is what the
missing code must produce. The first call never reaches the hole and
prints its line; the second reaches it and stops with a panic that names
the hole. A test that reaches a hole is reported as blocked rather than
failed, as in the exercises. A report that lists each hole with the
bindings in scope and the tests that reach it, `sisu check --holes`, is
designed in [10](../../proposal/10-tooling.md) and is not in the
compiler yet.

## Exercise 3.1: predict the calls

Open
[`manual/exercises/03-functions/predict_calls.sisu`](../exercises/03-functions/predict_calls.sisu):

<!-- file: exercises/03-functions/predict_calls.sisu -->
```sisu
##! Exercise 3.1, predict the calls. Kind: predict. Level: 1.
##!
##! A museum prices its tickets with `ticket`, and `halvings` counts
##! how often a number can be halved before it reaches 1. Each constant
##! is your prediction of the value of the call in its hole's name.
##! Replace each hole with a literal number, then run
##! `sisu test predict_calls.sisu`. The hints are in chapter 3 of the
##! tutorial, under "Exercise 3.1: predict the calls".

## The price in euros of tickets for `group` people of age `age`.
def ticket(age: int, *, student: bool = False, group: int = 1) -> int:
    let each = if age < 7: 0 elif age < 18 or student: 5 else: 9
    return each * group

## How many times `n` can be halved, rounding down, before it is 1.
def halvings(n: int) -> int:
    return if n <= 1: 0 else: 1 + halvings(n // 2)

const ADULT: int = todo("ticket(30)")
const SCHOOL_CLASS: int = todo("ticket(12, group=3)")
const STUDENT: int = todo("ticket(25, student=True)")
const TODDLERS: int = todo("ticket(4, group=2, student=True)")
const HALVINGS: int = todo("halvings(100)")

test "one adult":
    assert ticket(30) == ADULT

test "three twelve-year-olds":
    assert ticket(12, group=3) == SCHOOL_CLASS

test "a student":
    assert ticket(25, student=True) == STUDENT

test "two small children":
    assert ticket(4, group=2, student=True) == TODDLERS

test "halving a hundred":
    assert halvings(100) == HALVINGS
```

<!-- output: solutions/03-functions/predict_calls.console -->
```console
$ sisu test predict_calls.sisu
warning[W0001]: typed hole `ticket(30)` of type `int`
  --> predict_calls.sisu:19:20
   |
19 | const ADULT: int = todo("ticket(30)")
   |                    ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `ticket(12, group=3)` of type `int`
  --> predict_calls.sisu:20:27
   |
20 | const SCHOOL_CLASS: int = todo("ticket(12, group=3)")
   |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `ticket(25, student=True)` of type `int`
  --> predict_calls.sisu:21:22
   |
21 | const STUDENT: int = todo("ticket(25, student=True)")
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `ticket(4, group=2, student=True)` of type `int`
  --> predict_calls.sisu:22:23
   |
22 | const TODDLERS: int = todo("ticket(4, group=2, student=True)")
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `halvings(100)` of type `int`
  --> predict_calls.sisu:23:23
   |
23 | const HALVINGS: int = todo("halvings(100)")
   |                       ^^^^^^^^^^^^^^^^^^^^^

test "one adult" ... blocked by todo("ticket(30)")
test "three twelve-year-olds" ... blocked by todo("ticket(12, group=3)")
test "a student" ... blocked by todo("ticket(25, student=True)")
test "two small children" ... blocked by todo("ticket(4, group=2, student=True)")
test "halving a hundred" ... blocked by todo("halvings(100)")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

For each call, write down the value of every parameter first: the
arguments given, by position or by name, and the defaults for the rest.

</details>

<details>
<summary>Hint 2</summary>

An `if` expression takes the first branch whose condition holds and
ignores the rest. `halvings` adds one for each call it makes before `n`
reaches 1.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/03-functions/predict_calls.sisu`](../solutions/03-functions/predict_calls.sisu).

## Exercise 3.2: exam grades

Open
[`manual/exercises/03-functions/grades.sisu`](../exercises/03-functions/grades.sisu):

<!-- file: exercises/03-functions/grades.sisu -->
```sisu
##! Exercise 3.2, exam grades. Kind: fix. Level: 1.
##!
##! An exam is marked out of 100 points. `grade` turns the points into
##! a grade from 0 to 5, and `verdict` says whether the exam is passed.
##! The file does not compile. Run `sisu test grades.sisu`, read the
##! diagnostics, and fix both functions so that every test passes. The
##! hints are in chapter 3 of the tutorial, under "Exercise 3.2: exam
##! grades".

## The grade for `points`: 5 for 90 or more, 4 for 75 or more, 3 for
## 60 or more, 2 for 50 or more, 1 for 40 or more, and 0 below 40.
def grade(points: int) -> int:
    if points >= 90:
        return 5
    elif points >= 75:
        return 4
    elif points >= 60:
        return 3
    elif points >= 50:
        return 2
    elif points >= 40:
        return 1

## "passed" for a grade of 1 or more, and "failed" for 0.
def verdict(points: int) -> str:
    return if grade(points) >= 1: "passed"

test "top grades":
    assert grade(95) == 5
    assert grade(75) == 4

test "the lowest passing grade":
    assert grade(40) == 1
    assert verdict(40) == "passed"

test "a failed exam":
    assert grade(39) == 0
    assert verdict(12) == "failed"
```

<!-- output: solutions/03-functions/grades.console -->
```console
$ sisu test grades.sisu
error[E0315]: `grade` returns `int`, but control can reach the end of its body without a `return`
  --> grades.sisu:12:5
   |
12 | def grade(points: int) -> int:
   |     ^^^^^
  = note: return a value on that path too, or end the body with a `return`

error[E0116]: an `if` expression needs an `else` branch, because it must have a value
  --> grades.sisu:26:43
   |
26 |     return if grade(points) >= 1: "passed"
   |                                           ^

error[D0006]: the program was not run: errors found (2)
```

<details>
<summary>Hint 1</summary>

Find, for each function, a number of points for which it has no value
to give back.

</details>

<details>
<summary>Hint 2</summary>

Both diagnostics are about a path with no value: a function body that
can end without `return`, and an `if` expression without a last branch.

</details>

When every test passes, compare your answer with
[`manual/solutions/03-functions/grades.sisu`](../solutions/03-functions/grades.sisu).

## Exercise 3.3: leap years

Open
[`manual/exercises/03-functions/leap_year.sisu`](../exercises/03-functions/leap_year.sisu):

<!-- file: exercises/03-functions/leap_year.sisu -->
```sisu
##! Exercise 3.3, leap years. Kind: write. Level: 1.
##!
##! Write the body of `is_leap_year` so that every test passes. Run the
##! tests with `sisu test leap_year.sisu`. The hints are in chapter 3 of
##! the tutorial, under "Exercise 3.3: leap years".

## Whether `year` is a leap year in the Gregorian calendar: a year that
## divides by 4, except a year that divides by 100 and not by 400.
def is_leap_year(year: int) -> bool:
    return todo("is_leap_year")

test "an ordinary year":
    assert not is_leap_year(2026)

test "every fourth year":
    assert is_leap_year(2024)
    assert is_leap_year(1996)

test "a century year that is not a leap year":
    assert not is_leap_year(1900)
    assert not is_leap_year(2100)

test "every fourth century year is a leap year":
    assert is_leap_year(2000)
```

<!-- output: solutions/03-functions/leap_year.console -->
```console
$ sisu test leap_year.sisu
warning[W0001]: typed hole `is_leap_year` of type `bool`
  --> leap_year.sisu:10:12
   |
10 |     return todo("is_leap_year")
   |            ^^^^^^^^^^^^^^^^^^^^

test "an ordinary year" ... blocked by todo("is_leap_year")
test "every fourth year" ... blocked by todo("is_leap_year")
test "a century year that is not a leap year" ... blocked by todo("is_leap_year")
test "every fourth century year is a leap year" ... blocked by todo("is_leap_year")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

"Divides by 4" is a test of a remainder: what is `year % 4` for a year
that divides by 4?

</details>

<details>
<summary>Hint 2</summary>

The rule is one condition in three parts. Decide which parts belong
together, and remember that `and` and `or` need parentheses where they
meet.

</details>

When every test passes, compare your answer with
[`manual/solutions/03-functions/leap_year.sisu`](../solutions/03-functions/leap_year.sisu).

## Exercise 3.4: the sum of the digits

Open
[`manual/exercises/03-functions/digit_sum.sisu`](../exercises/03-functions/digit_sum.sisu).
The doc example is one of the judges:

<!-- file: exercises/03-functions/digit_sum.sisu -->
```sisu
##! Exercise 3.4, the sum of the digits. Kind: write. Level: 2.
##!
##! Write the body of `digit_sum` so that its doc example and every test
##! pass. `sisu test digit_sum.sisu` runs the doc example as a test of
##! its own. The hints are in chapter 3 of the tutorial, under
##! "Exercise 3.4: the sum of the digits".

## The sum of the decimal digits of `n`, which is not negative.
##
## ```
## assert digit_sum(2026) == 10
## ```
def digit_sum(n: int) -> int:
    return todo("digit_sum")

test "one digit":
    assert digit_sum(7) == 7
    assert digit_sum(0) == 0

test "many digits":
    assert digit_sum(9_876_543_210) == 45
    assert digit_sum(1_000_001) == 2
```

<!-- output: solutions/03-functions/digit_sum.console -->
```console
$ sisu test digit_sum.sisu
warning[W0001]: typed hole `digit_sum` of type `int`
  --> digit_sum.sisu:14:12
   |
14 |     return todo("digit_sum")
   |            ^^^^^^^^^^^^^^^^^

test "one digit" ... blocked by todo("digit_sum")
test "many digits" ... blocked by todo("digit_sum")
test "digit_sum (doc example)" ... blocked by todo("digit_sum")
0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

`%` and `//` by 10 split a number into its last digit and the rest:
2026 is 202 and 6.

</details>

<details>
<summary>Hint 2</summary>

The sum of the digits of a number is its last digit plus the sum of the
digits of the rest. That is a recursion, and a number of one digit is
the case that ends it.

</details>

When every test passes, compare your answer with
[`manual/solutions/03-functions/digit_sum.sisu`](../solutions/03-functions/digit_sum.sisu).

## Exercise 3.5: hours, minutes and seconds

Open
[`manual/exercises/03-functions/split_time.sisu`](../exercises/03-functions/split_time.sisu).
This exercise combines the chapter's ideas:

<!-- file: exercises/03-functions/split_time.sisu -->
```sisu
##! Exercise 3.5, hours, minutes and seconds. Kind: write. Level: 2.
##!
##! A stopwatch counts seconds. Write the bodies of `split_time` and
##! `show_time` so that every test passes; `show_time` can call
##! `split_time`. Run the tests with `sisu test split_time.sisu`. The
##! hints are in chapter 3 of the tutorial, under "Exercise 3.5: hours,
##! minutes and seconds".

## `total` seconds as whole hours, minutes and seconds.
def split_time(total: int) -> (hours: int, minutes: int, seconds: int):
    return todo("split_time")

## `total` seconds as text, such as "1 h 2 min 5 s", or "1 h 2 min"
## when `seconds` is False.
def show_time(total: int, *, seconds: bool = True) -> String:
    return todo("show_time")

test "an hour, two minutes and five seconds":
    let time = split_time(3725)
    assert time.hours == 1
    assert time.minutes == 2
    assert time.seconds == 5

test "less than a minute":
    let time = split_time(59)
    assert time.hours == 0
    assert time.minutes == 0
    assert time.seconds == 59

test "shown with seconds":
    assert show_time(3725) == "1 h 2 min 5 s"
    assert show_time(86399) == "23 h 59 min 59 s"

test "shown without seconds":
    assert show_time(3725, seconds=False) == "1 h 2 min"
    assert show_time(7200, seconds=False) == "2 h 0 min"
```

<!-- output: solutions/03-functions/split_time.console -->
```console
$ sisu test split_time.sisu
warning[W0001]: typed hole `split_time` of type `(hours: int, minutes: int, seconds: int)`
  --> split_time.sisu:11:12
   |
11 |     return todo("split_time")
   |            ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `show_time` of type `String`
  --> split_time.sisu:16:12
   |
16 |     return todo("show_time")
   |            ^^^^^^^^^^^^^^^^^

test "an hour, two minutes and five seconds" ... blocked by todo("split_time")
test "less than a minute" ... blocked by todo("split_time")
test "shown with seconds" ... blocked by todo("show_time")
test "shown without seconds" ... blocked by todo("show_time")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

An hour is 3600 seconds and a minute is 60. `//` gives the whole hours,
and `%` what is left over after them.

</details>

<details>
<summary>Hint 2</summary>

`split_time` returns a labelled tuple, built in the order of its
labels. `show_time` calls it once, reads the three values by label, and
chooses between two strings with an `if` expression.

</details>

When every test passes, compare your answer with
[`manual/solutions/03-functions/split_time.sisu`](../solutions/03-functions/split_time.sisu).

## What you learned

- A function's signature names its parameters with their types and its
  result type, and Sisu never infers them.
- A value leaves a function only through `return`, and a function with a
  result returns on every path. A function without `->` returns nothing.
- Defaults are worked out at each call. Arguments can be passed by name;
  `*` makes the parameters after it keyword-only and `/` the ones before
  it positional-only. One name is one function.
- A labelled tuple returns several values, read by label or unpacked.
- `if c: a else: b` is an expression, and it needs its `else`.
- `assert` checks an assumption and panics when it fails. A `##` comment
  documents a function, and its example runs as a test.
- `todo("...")` leaves a typed hole that lets the rest of a program
  compile and run.

Chapter 4 is about control flow: `if` statements, loops, `break` and
`continue`, and `match`.
