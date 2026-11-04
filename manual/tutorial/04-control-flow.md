# 4. Control flow

In this chapter you decide which lines of a program run and how often:
`if` chooses, `while` and `for` repeat, `break` and `continue` change a
loop's course, and `match` compares a value with a list of patterns.
Sisu's control flow is Python's with a few things left out, and the
chapter ends with what replaces each of them. You need about an hour.

## Choosing with if

`if` runs a block when its condition is `True`. `elif` adds another
condition, tried only when the ones before it were `False`, and `else`
runs when none of them was `True`:

<!-- file: examples/04-control-flow/wind.sisu -->
```sisu
# `if`, `elif` and `else` choose which lines run.
def describe_wind(speed: f64) -> str:
    if speed < 1.0:
        return "calm"
    elif speed < 8.0:
        return "a breeze"
    elif speed < 21.0:
        return "a strong wind"
    else:
        return "a storm"

def main():
    print("0.5 m/s is {describe_wind(0.5)}")
    print("12 m/s is {describe_wind(12.0)}")
    print("25 m/s is {describe_wind(25.0)}")
```

<!-- output: examples/04-control-flow/wind.out -->
```text
0.5 m/s is calm
12 m/s is a strong wind
25 m/s is a storm
```

- A condition is a `bool`, as chapter 2 showed: `speed < 1.0`, never
  `speed` alone.
- Each branch is a block of indented lines, and it can hold any
  statements, `return` included.
- Sisu tries the conditions from the top and runs the first branch whose
  condition holds. For 12 m/s, `speed < 8.0` is `False` and
  `speed < 21.0` is `True`, so the third branch runs.

This is the statement form of `if`. Chapter 3 showed the expression
form, `if c: a else: b`, for a choice between two values.

## Repeating with while

`while` runs its block again and again for as long as its condition
holds. Sisu checks the condition before each round:

<!-- file: examples/04-control-flow/double_money.sisu -->
```sisu
# `while` repeats its block for as long as its condition holds.
def years_to_double(rate: f64) -> int:
    var money = 1.0
    var years = 0
    while money < 2.0:
        money *= 1.0 + rate
        years += 1
    return years

def main():
    print("at 7%: {years_to_double(0.07)} years")
    print("at 2%: {years_to_double(0.02)} years")
```

**Predict.** The first line printed says 11 years. Decide whether the
second, at 2%, is about twice that, three times or more. Then run
`sisu run double_money.sisu`:

<!-- output: examples/04-control-flow/double_money.out -->
```text
at 7%: 11 years
at 2%: 36 years
```

The loop needs `var` bindings, because each round changes them: `money`
grows and `years` counts. When `money` reaches 2.0 the condition is
`False`, the loop ends, and the function returns. A `while` loop whose
condition never becomes `False` runs until something stops the program;
the section "Leaving a loop early" shows how to stop one from inside.

## Counting with for and range

`for` runs its block once for each value it is given. `range` gives a
run of numbers:

<!-- file: examples/04-control-flow/ranges.sisu -->
```sisu
# `for` runs its block once for each number of a `range`.
def main():
    for i in range(3):
        print("range(3) gives {i}")
    for i in range(10, 13):
        print("range(10, 13) gives {i}")
    for i in range(0, 10, 4):
        print("range(0, 10, 4) gives {i}")
    for i in range(3, 0, -1):
        print("range(3, 0, -1) gives {i}")
```

<!-- output: examples/04-control-flow/ranges.out -->
```text
range(3) gives 0
range(3) gives 1
range(3) gives 2
range(10, 13) gives 10
range(10, 13) gives 11
range(10, 13) gives 12
range(0, 10, 4) gives 0
range(0, 10, 4) gives 4
range(0, 10, 4) gives 8
range(3, 0, -1) gives 3
range(3, 0, -1) gives 2
range(3, 0, -1) gives 1
```

- `range(3)` counts from 0 up to 3, without 3: 0, 1 and 2.
- `range(10, 13)` starts at 10 and stops before 13.
- `range(0, 10, 4)` counts in steps of 4, and stops before reaching 10.
- `range(3, 0, -1)` counts down. A step can be negative, never zero.

The name after `for`, here `i`, is a new binding in each round, made
with the round's value. Like a `let`, it cannot be assigned to.

> **From Python.** `range` works as in Python, and so does `for` over
> it. There is no C-style `for (i = 0; i < n; i++)` in either language,
> and Sisu has no `++` either: write `i += 1`.

## Going through a list

`for` also goes through the items of a list, in order. A list is values
between square brackets, separated by commas; chapter 5 covers lists.
`enumerate` pairs each item with its position:

<!-- file: examples/04-control-flow/for_list.sisu -->
```sisu
# `for` goes through the items of a list; `enumerate` counts them.
def main():
    for name in ["Aino", "Eero", "Roni"]:
        print("Hei, {name}!")
    for place, name in enumerate(["Aino", "Eero", "Roni"]):
        print("{place + 1}. {name}")
```

<!-- output: examples/04-control-flow/for_list.out -->
```text
Hei, Aino!
Hei, Eero!
Hei, Roni!
1. Aino
2. Eero
3. Roni
```

`enumerate` gives pairs that count from 0, and `for place, name in ...`
unpacks each pair into two bindings, as `let` unpacked a tuple in
chapter 3. The program adds 1 to `place` to number the names from 1.

## Leaving a loop early

`break` leaves the loop at once. `continue` skips the rest of the
current round and starts the next one:

<!-- file: examples/04-control-flow/break_continue.sisu -->
```sisu
# `continue` skips to the next round, and `break` leaves the loop.
def main():
    var n = 0
    while True:
        n += 1
        if n % 3 == 0:
            continue
        if n > 10:
            break
        print("{n} is not divisible by 3")
    print("stopped at {n}")
```

<!-- output: examples/04-control-flow/break_continue.out -->
```text
1 is not divisible by 3
2 is not divisible by 3
4 is not divisible by 3
5 is not divisible by 3
7 is not divisible by 3
8 is not divisible by 3
10 is not divisible by 3
stopped at 11
```

`while True:` would loop forever, because its condition is always
`True`; the `break` is the only way out of it. Each round adds 1 to `n`.
For a multiple of 3, `continue` goes back to the top of the loop, so the
`print` never runs for it. When `n` passes 10, `break` ends the loop and
the line after it runs. `break` and `continue` work the same in a `for`
loop.

## Matching values

`match` compares one value with a list of patterns, from the top, and
runs the block of the first `case` that matches:

<!-- file: examples/04-control-flow/status_codes.sisu -->
```sisu
# `match` compares a value with patterns, from the top.
def explain(code: int) -> str:
    match code:
        case 200:
            return "OK"
        case 301 | 302:
            return "moved"
        case 404:
            return "not found"
        case _ if code >= 500:
            return "a server error"
        case _:
            return "something else"

def main():
    for code in [200, 302, 404, 503, 418]:
        print("{code}: {explain(code)}")
```

<!-- output: examples/04-control-flow/status_codes.out -->
```text
200: OK
302: moved
404: not found
503: a server error
418: something else
```

- A literal pattern, such as `200`, matches an equal value.
- `|` joins alternatives: `301 | 302` matches either.
- `_` matches any value.
- `if` after a pattern is a guard: the case matches only when the
  pattern matches and the guard is `True`. `case _ if code >= 500`
  matches any code of 500 or more.
- Only one case runs. Control never falls through into the next case.

A `match` must cover every value its subject can have. For an `int`,
only `_` covers the numbers that no literal names.

**Try it.** Delete the two cases that start with `case _` from
`explain` and run the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/04-control-flow/missing_case.console -->
```console
$ sisu run missing_case.sisu
error[E0314]: this `match` does not cover `_`
  --> missing_case.sisu:3:5
  |
3 |     match code:
  |     ^^^^^^^^^^
  = note: add an arm for `_`, or a `case _` arm

error[D0006]: the program was not run: errors found (1)
```

E0314 (`type.non-exhaustive`) names what is not covered, `_`: every
`int` except 200, 301, 302 and 404. For the types of chapter 7, which
have a fixed set of values, the same diagnostic lists the missing ones.

</details>

When each case is a single expression, `match` gives a value, as `if`
does:

<!-- file: examples/04-control-flow/match_value.sisu -->
```sisu
# `match` gives a value when each arm is one expression.
def main():
    for day in range(1, 8):
        let kind = match day:
            case 6 | 7: "weekend"
            case _: "weekday"
        print("day {day} is a {kind}")
```

<!-- output: examples/04-control-flow/match_value.out -->
```text
day 1 is a weekday
day 2 is a weekday
day 3 is a weekday
day 4 is a weekday
day 5 is a weekday
day 6 is a weekend
day 7 is a weekend
```

Chapter 7 uses `match` to take apart values of your own types, which is
where it does the most work.

> **From Python.** Python has `match` since version 3.10, with the same
> `case`, `|`, `_` and guards. Sisu's `match` also checks that the cases
> cover every value, and it is the only such statement: Sisu has no
> `switch`.

## Doing nothing

A block cannot be empty. `pass` is a block that does nothing, for a
branch that must exist but has no work:

<!-- file: examples/04-control-flow/log_levels.sisu -->
```sisu
# `pass` is a block that does nothing.
def show(level: str, message: str):
    match level:
        case "debug":
            pass
        case "warning" | "error":
            print("{level.to_uppercase()}: {message}")
        case _:
            print(message)

def main():
    show("debug", "the cache holds 12 entries")
    show("info", "starting")
    show("error", "the disk is full")
```

<!-- output: examples/04-control-flow/log_levels.out -->
```text
starting
ERROR: the disk is full
```

The `debug` case matches debug messages and does nothing with them, so
they are not shown. Without that case, they would reach `case _` and be
printed.

## What Sisu leaves out

Sisu has one way to write each kind of loop and choice, and leaves out
several that other languages have (D-021). This table gives the idiom
for each:

| Elsewhere | In Sisu |
|---|---|
| `switch` with `case` and `break` | `match`, whose cases never fall through |
| `do { ... } while (c)` | `while True:` with `if not c: break` as the block's last lines |
| `for (i = 0; i < n; i++)` | `for i in range(n):` |
| `i++` | `i += 1` |
| a loop label, to leave two loops at once | a function that returns from inside both loops |
| Python's `else` on a loop | a function that returns from inside the loop |

Python's loop `else` runs when a loop ends without a `break`, usually
after a search that found nothing. It does not compile in Sisu:

<!-- file: examples/04-control-flow/loop_else.sisu -->
```sisu
# A Python loop `else` does not compile in Sisu.
def first_frost(temperatures: list[f64]) -> String:
    var message = String()
    for hour, temperature in enumerate(temperatures):
        if temperature < 0.0:
            message = "frost at hour {hour}"
            break
    else:
        message = "no frost"
    return message

def main():
    print(first_frost([3.5, 1.0, -0.5, -2.0]))
```

<!-- output: examples/04-control-flow/loop_else.console -->
```console
$ sisu run loop_else.sisu
error[E0127]: a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:`
  --> loop_else.sisu:8:5
  |
8 |     else:
  |     ^^^^

error[D0006]: the program was not run: errors found (1)
```

E0127 (`parse.loop-else`) says that a loop has no `else`, and names one
way out: keep what the loop finds in a variable and test it after the
loop. The other is to put the search in a function and return as soon
as it finds what it looks for. The lines after the loop then run only
when the search found nothing, which is what the `else` meant:

<!-- file: examples/04-control-flow/first_frost.sisu -->
```sisu
# A search returns from inside the loop; after it, nothing matched.
def first_frost(temperatures: list[f64]) -> String:
    for hour, temperature in enumerate(temperatures):
        if temperature < 0.0:
            return "frost at hour {hour}"
    return "no frost"

def main():
    print(first_frost([3.5, 1.0, -0.5, -2.0]))
    print(first_frost([3.5, 1.0, 0.5]))
```

<!-- output: examples/04-control-flow/first_frost.out -->
```text
frost at hour 2
no frost
```

The function is shorter than the loop with `else`, and it needs no `var`.
Chapter 8 shows another way, which keeps the search's result as a
value.

## Exercise 4.1: predict the loops

Open
[`manual/exercises/04-control-flow/predict_loops.sisu`](../exercises/04-control-flow/predict_loops.sisu):

<!-- file: exercises/04-control-flow/predict_loops.sisu -->
```sisu
##! Exercise 4.1, predict the loops. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the call in its
##! hole's name. Trace each loop by hand, write the value as a literal,
##! and then run `sisu test predict_loops.sisu`. The hints are in
##! chapter 4 of the tutorial, under "Exercise 4.1: predict the loops".

## The sum of the numbers that `range(start, stop, step)` gives.
def range_sum(start: int, stop: int, step: int) -> int:
    var total = 0
    for i in range(start, stop, step):
        total += i
    return total

## How many odd numbers from 1 to `limit` the loop counts.
def odd_count(limit: int) -> int:
    var count = 0
    var n = 0
    while True:
        n += 1
        if n > limit:
            break
        if n % 2 == 0:
            continue
        count += 1
    return count

## A word for how many there are of something.
def amount(n: int) -> str:
    match n:
        case 0:
            return "none"
        case 1 | 2 | 3:
            return "a few"
        case _ if n < 0:
            return "impossible"
        case _:
            return "many"

const UP: int = todo("range_sum(1, 10, 3)")
const DOWN: int = todo("range_sum(10, 0, -4)")
const ODD: int = todo("odd_count(7)")
const NEGATIVE: str = todo("amount(-2)")
const PLENTY: str = todo("amount(4)")

test "counting up in threes":
    assert range_sum(1, 10, 3) == UP

test "counting down in fours":
    assert range_sum(10, 0, -4) == DOWN

test "skipping even numbers":
    assert odd_count(7) == ODD

test "a negative amount":
    assert amount(-2) == NEGATIVE

test "four of something":
    assert amount(4) == PLENTY
```

<!-- output: solutions/04-control-flow/predict_loops.console -->
```console
$ sisu test predict_loops.sisu
warning[W0001]: typed hole `range_sum(1, 10, 3)` of type `int`
  --> predict_loops.sisu:40:17
   |
40 | const UP: int = todo("range_sum(1, 10, 3)")
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `range_sum(10, 0, -4)` of type `int`
  --> predict_loops.sisu:41:19
   |
41 | const DOWN: int = todo("range_sum(10, 0, -4)")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `odd_count(7)` of type `int`
  --> predict_loops.sisu:42:18
   |
42 | const ODD: int = todo("odd_count(7)")
   |                  ^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `amount(-2)` of type `str`
  --> predict_loops.sisu:43:23
   |
43 | const NEGATIVE: str = todo("amount(-2)")
   |                       ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `amount(4)` of type `str`
  --> predict_loops.sisu:44:21
   |
44 | const PLENTY: str = todo("amount(4)")
   |                     ^^^^^^^^^^^^^^^^^

test "counting up in threes" ... blocked by todo("range_sum(1, 10, 3)")
test "counting down in fours" ... blocked by todo("range_sum(10, 0, -4)")
test "skipping even numbers" ... blocked by todo("odd_count(7)")
test "a negative amount" ... blocked by todo("amount(-2)")
test "four of something" ... blocked by todo("amount(4)")
0 passed, 0 failed, 5 blocked
```

The last two holes are `str` values, so the prediction is a string in
double quotes.

<details>
<summary>Hint 1</summary>

Write down the values that each `range` gives before adding anything
up, and remember where a range stops.

</details>

<details>
<summary>Hint 2</summary>

In `odd_count`, follow `n` round by round and note which line each
round reaches. In `amount`, a case runs only when both its pattern and
its guard hold, and the cases are tried from the top.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/04-control-flow/predict_loops.sisu`](../solutions/04-control-flow/predict_loops.sisu).

## Exercise 4.2: prime or not

Open
[`manual/exercises/04-control-flow/prime_or_not.sisu`](../exercises/04-control-flow/prime_or_not.sisu):

<!-- file: exercises/04-control-flow/prime_or_not.sisu -->
```sisu
##! Exercise 4.2, prime or not. Kind: fix. Level: 1.
##!
##! `describe` was ported from Python's tutorial, which uses an `else`
##! on its `for` loop: the `else` block runs when the loop ends without
##! a `break`. Sisu has no `else` on loops, so the file does not
##! compile. Rewrite `describe` without one, so that every test passes.
##! Run the tests with `sisu test prime_or_not.sisu`. The hints are in
##! chapter 4 of the tutorial, under "Exercise 4.2: prime or not".

## "N is prime", or "N = A * B" where A is the smallest factor of `n`
## above 1. `n` is 2 or more.
def describe(n: int) -> String:
    var text = String()
    for d in range(2, n):
        if n % d == 0:
            text = "{n} = {d} * {n // d}"
            break
    else:
        text = "{n} is prime"
    return text

test "primes":
    assert describe(2) == "2 is prime"
    assert describe(7) == "7 is prime"
    assert describe(97) == "97 is prime"

test "numbers with factors":
    assert describe(9) == "9 = 3 * 3"
    assert describe(12) == "12 = 2 * 6"
```

<!-- output: solutions/04-control-flow/prime_or_not.console -->
```console
$ sisu test prime_or_not.sisu
error[E0127]: a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:`
  --> prime_or_not.sisu:18:5
   |
18 |     else:
   |     ^^^^

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

The `else` block holds what to answer when no `d` divides `n`. Think
about where in the function that answer belongs once the `else` is
gone.

</details>

<details>
<summary>Hint 2</summary>

The idiom in the section "What Sisu leaves out" replaces the `break`
and the `else` both: a `return` inside the loop, and another after it.

</details>

When every test passes, compare your answer with
[`manual/solutions/04-control-flow/prime_or_not.sisu`](../solutions/04-control-flow/prime_or_not.sisu).

## Exercise 4.3: FizzBuzz

Open
[`manual/exercises/04-control-flow/fizzbuzz.sisu`](../exercises/04-control-flow/fizzbuzz.sisu):

<!-- file: exercises/04-control-flow/fizzbuzz.sisu -->
```sisu
##! Exercise 4.3, FizzBuzz. Kind: write. Level: 1.
##!
##! Write the body of `fizzbuzz` so that every test passes.
##! `fizzbuzz_line`, which is already written, calls it for each number
##! and builds a line of text: `String()` starts an empty text, and `+=`
##! adds to it. Run the tests with `sisu test fizzbuzz.sisu`. The hints
##! are in chapter 4 of the tutorial, under "Exercise 4.3: FizzBuzz".

## The FizzBuzz word for `n`: "FizzBuzz" when 3 and 5 both divide `n`,
## "Fizz" when 3 does, "Buzz" when 5 does, and the number otherwise.
def fizzbuzz(n: int) -> String:
    return todo("fizzbuzz")

## The FizzBuzz words for 1 to `last`, separated by spaces.
def fizzbuzz_line(last: int) -> String:
    var line = String()
    for n in range(1, last + 1):
        if n > 1:
            line += " "
        line += fizzbuzz(n)
    return line

test "plain numbers":
    assert fizzbuzz(1) == "1"
    assert fizzbuzz(98) == "98"

test "multiples of three or five":
    assert fizzbuzz(9) == "Fizz"
    assert fizzbuzz(25) == "Buzz"

test "multiples of fifteen":
    assert fizzbuzz(30) == "FizzBuzz"

test "a line of words":
    assert fizzbuzz_line(3) == "1 2 Fizz"
    assert fizzbuzz_line(10) == "1 2 Fizz 4 Buzz Fizz 7 8 Fizz Buzz"
```

<!-- output: solutions/04-control-flow/fizzbuzz.console -->
```console
$ sisu test fizzbuzz.sisu
warning[W0001]: typed hole `fizzbuzz` of type `String`
  --> fizzbuzz.sisu:12:12
   |
12 |     return todo("fizzbuzz")
   |            ^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("fizzbuzz")`:
  n = 1
in scope at the typed hole `todo("fizzbuzz")`:
  n = 9
in scope at the typed hole `todo("fizzbuzz")`:
  n = 30
in scope at the typed hole `todo("fizzbuzz")`:
  n = 1
test "plain numbers" ... blocked by todo("fizzbuzz")
test "multiples of three or five" ... blocked by todo("fizzbuzz")
test "multiples of fifteen" ... blocked by todo("fizzbuzz")
test "a line of words" ... blocked by todo("fizzbuzz")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

"3 divides `n`" means that the remainder of `n` divided by 3 is 0. A
number turns into text by interpolation.

</details>

<details>
<summary>Hint 2</summary>

Sisu runs the first branch whose condition holds, so the order of the
conditions matters. A multiple of 15 is also a multiple of 3 and of 5.

</details>

When every test passes, compare your answer with
[`manual/solutions/04-control-flow/fizzbuzz.sisu`](../solutions/04-control-flow/fizzbuzz.sisu).

## Exercise 4.4: the Collatz steps

Open
[`manual/exercises/04-control-flow/collatz.sisu`](../exercises/04-control-flow/collatz.sisu):

<!-- file: exercises/04-control-flow/collatz.sisu -->
```sisu
##! Exercise 4.4, the Collatz steps. Kind: write. Level: 2.
##!
##! Take a number of 1 or more. If it is even, halve it; if it is odd,
##! multiply it by 3 and add 1. Repeat, and every number tried so far
##! reaches 1. Write the body of `collatz_steps`, which counts the
##! steps, so that every test passes. Run the tests with
##! `sisu test collatz.sisu`. The hints are in chapter 4 of the
##! tutorial, under "Exercise 4.4: the Collatz steps".

## The number of steps that take `n` to 1, for `n` of 1 or more.
def collatz_steps(n: int) -> int:
    return todo("collatz_steps")

test "one is already there":
    assert collatz_steps(1) == 0

test "a power of two only halves":
    assert collatz_steps(16) == 4

test "small numbers":
    assert collatz_steps(6) == 8
    assert collatz_steps(7) == 16

test "twenty-seven takes a long way":
    assert collatz_steps(27) == 111
```

<!-- output: solutions/04-control-flow/collatz.console -->
```console
$ sisu test collatz.sisu
warning[W0001]: typed hole `collatz_steps` of type `int`
  --> collatz.sisu:12:12
   |
12 |     return todo("collatz_steps")
   |            ^^^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("collatz_steps")`:
  n = 1
in scope at the typed hole `todo("collatz_steps")`:
  n = 16
in scope at the typed hole `todo("collatz_steps")`:
  n = 6
in scope at the typed hole `todo("collatz_steps")`:
  n = 27
test "one is already there" ... blocked by todo("collatz_steps")
test "a power of two only halves" ... blocked by todo("collatz_steps")
test "small numbers" ... blocked by todo("collatz_steps")
test "twenty-seven takes a long way" ... blocked by todo("collatz_steps")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

You do not know in advance how many steps there are, so the loop is a
`while` loop that runs until the number is 1.

</details>

<details>
<summary>Hint 2</summary>

Two values change from round to round: the number and the count of
steps. A parameter cannot be assigned to, so each needs a `var` of its
own.

</details>

When every test passes, compare your answer with
[`manual/solutions/04-control-flow/collatz.sisu`](../solutions/04-control-flow/collatz.sisu).

## What you learned

- `if`, `elif` and `else` choose a block by conditions, tried from the
  top.
- `while` repeats a block while its condition holds. `for` runs a block
  once for each value of a `range` or each item of a list, and
  `enumerate` adds the positions.
- `break` leaves a loop, and `continue` starts its next round.
- `match` compares a value with patterns: literals, `|`, `_` and guards.
  It runs one case, and its cases must cover every value. `pass` is the
  empty block.
- Sisu has no `switch`, `do-while`, C-style `for`, `++`, loop labels or
  loop `else`; a function that returns from inside a loop replaces the
  last two.

Chapter 5 is about collections: lists, dicts, sets and tuples, the
functions that work on them, and the first rule of ownership.
