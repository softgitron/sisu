# 8. Absent values and errors

In this chapter you handle the two things that go wrong in ordinary
code: a value that is not there, and an operation that fails. Sisu has
no `null` and no exceptions. A value that may be missing has a type that
says so, an operation that may fail returns its error as a value, and a
few operators make both short to handle. At the end you meet panics,
which are for bugs rather than for errors. You need about an hour and a
half.

## Values that may be absent

A function that looks for something may find nothing. Its result type
says so with a question mark: `int?` is an optional `int`, which is
either `Some(n)`, holding a number, or `None`:

<!-- file: examples/08-errors/find.sisu -->
```sisu
# A function whose answer may be missing returns an optional value.
def find_index(names: list[str], wanted: str) -> int?:
    for i, name in enumerate(names):
        if name == wanted:
            return Some(i)
    return None

def main():
    let names = ["Aino", "Eero", "Roni"]
    print(find_index(names, "Eero"))
    print(find_index(names, "Liisa"))
```

<!-- output: examples/08-errors/find.out -->
```text
Some(1)
None
```

`return Some(i)` gives back the position wrapped in `Some`, and `return
None` says that there is none. You met optional values already: `get`
on a dict in chapter 5, and `find` and `parse_int` on text in chapter
6.

An `int?` is not an `int`, and Sisu does not let you use one as the
other.

**Try it.** Change the first `print` to
`print(find_index(names, "Eero") + 1)` and run the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/08-errors/plus_one.console -->
```console
$ sisu run plus_one.sisu
error[E0310]: `+` cannot be applied to `int?` and `int`
  --> plus_one.sisu:10:11
   |
10 |     print(find_index(names, "Eero") + 1)
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: the value may be missing: give a default with `??`, or take it apart with `if let` or `match`

error[D0006]: the program was not run: errors found (1)
```

E0310 (`type.binary-operator`) refuses to add 1 to a value that may be
missing, and its note names the ways to take the value out, which the
next sections show.

</details>

> **From Python.** In Python any name can hold `None`, and code that
> forgets to check fails with an `AttributeError` or a `TypeError` when
> it runs, if a test happens to reach it. In Sisu only a `T?` can be
> absent, so the type tells you where to check, and the compiler makes
> you check there.

## Taking the value out

`if let` runs a block when an optional value holds something, and binds
what it holds to a name. `match` handles both cases:

<!-- file: examples/08-errors/if_let.sisu -->
```sisu
# `if let` and `match` take the value out of an optional value.
def main():
    let ages = {"Aino": 34, "Eero": 29}
    if let Some(age) = ages.get("Aino"):
        print("Aino is {age}")
    match ages.get("Roni"):
        case Some(age): print("Roni is {age}")
        case None: print("Roni's age is not known")
    if let Some(a) = ages.get("Aino"), let Some(e) = ages.get("Eero"), a > e:
        print("Aino is {a - e} years older than Eero")
```

<!-- output: examples/08-errors/if_let.out -->
```text
Aino is 34
Roni's age is not known
Aino is 5 years older than Eero
```

- `if let Some(age) = ages.get("Aino"):` matches the pattern
  `Some(age)` against the lookup. When it matches, `age` holds the age
  inside the block; when the lookup gives `None`, the block is skipped.
- `match` with the arms `Some(age)` and `None` covers both cases, and
  the exhaustiveness check of chapter 7 makes sure that it does.
- A condition can chain several parts with commas, here two lookups
  and a comparison (D-071). The parts run from left to right, each
  binding can be used by the parts after it, and the block runs only
  when every part succeeds.

## Defaults and optional fields

Often you want the value, or a default when there is none. `??` gives
that, and is read "or else": `a ?? b` is "a, or else b". `?.` reads a
field of a value that may be absent, and is read "if present": `x?.f`
is "f of x, if x is present":

<!-- file: examples/08-errors/defaults.sisu -->
```sisu
# `??` gives a default, and `?.` reads a field if the value is there.
struct Player:
    name: String
    level: int

def main():
    let players = [Player("Aino", 7), Player("Eero", 3)]
    let first = players.first()
    let sixth = players.get(5)
    print("{first?.level} {sixth?.level}")
    print("{first?.level ?? 0} {sixth?.level ?? 0}")
```

<!-- output: examples/08-errors/defaults.out -->
```text
Some(7) None
7 0
```

`players.first()` and `players.get(5)` both give an optional player:
the list has a first item and no sixth. `first?.level` is the level
inside `Some(7)`, and `sixth?.level` is `None`, since there is no
player to read it from. `?? 0` then replaces a `None` with 0.

`??` chains: `a ?? b ?? c` is a, or else b, or else c, which is how a
setting falls back from one place to the next.

## A fallback, or leaving early

`let` with `else` takes the value out of an optional value, and says
what happens when there is none. A value after `else:` is the fallback,
and an indented block after it must leave, with `return`, `break`,
`continue` or a panic:

<!-- file: examples/08-errors/fallback.sisu -->
```sisu
# `else` gives a fallback value, or a block that leaves.
def double(text: str) -> String:
    let n = text.trim().parse_int() else:
        return "{text:?} is not a number"
    return "{n} doubled is {n * 2}"

def main():
    print(double(" 21 "))
    print(double("kaksi"))
    let count = "many".parse_int() else: 1
    print("count is {count}")
```

<!-- output: examples/08-errors/fallback.out -->
```text
21 doubled is 42
"kaksi" is not a number
count is 1
```

*Kaksi* is Finnish for "two", and not a number that `parse_int` reads.

- In `double`, `let n = ... else:` with a block either binds `n` to the
  number or returns from the function. After the `let`, `n` is an
  `int`, so the rest of the function does not check again.
- `let count = "many".parse_int() else: 1` gives `count` the value 1
  when the text is not a number.

A block after `else:` that could fall through to the next line does
not compile, because the binding would then have no value.

## Passing absence on with ?

A function that returns an optional value can pass a `None` on to its
caller with `?` after an expression. `x?` is read "x, or return
early":

<!-- file: examples/08-errors/initials.sisu -->
```sisu
# `?` on an optional value returns `None` from the function at once.
def initials(full_name: str) -> String?:
    let first, last = full_name.split_once(" ")?
    let a = first.chars().next()?
    let b = last.chars().next()?
    return Some("{a}{b}")

def main():
    print(initials("Aino Virtanen"))
    print(initials("Eero"))
```

<!-- output: examples/08-errors/initials.out -->
```text
Some("AV")
None
```

Each `?` either takes the value out or returns `None` from `initials`
at once. "Eero" has no space, so `split_once(" ")` gives `None`, and so
does `initials`. Without `?` the same function would need three `let
... else` blocks that each return `None`.

## Repeating while there is a value

`while let` repeats a block for as long as a pattern matches:

<!-- file: examples/08-errors/plates.sisu -->
```sisu
# `while let` repeats for as long as there is a value.
def main():
    var plates = ["blue", "green", "red"]
    while let Some(plate) = plates.pop():
        print("washing the {plate} plate")
    print("{len(plates)} plates left")
```

<!-- output: examples/08-errors/plates.out -->
```text
washing the red plate
washing the green plate
washing the blue plate
0 plates left
```

`pop` takes the last item off a list and gives it as an optional value,
which is `None` once the list is empty. The loop binds each plate in
turn and stops at the first `None`.

## Errors are values

An optional value says that something is missing, not why. An
operation that can fail in several ways returns an error that says
which. `error` declares an error set, the ways a group of operations
can fail, and `T ! E`, read "T or error E", is the result type of a
function that gives a `T` or fails with an error from `E`:

<!-- file: examples/08-errors/withdraw.sisu -->
```sisu
# A function that can fail returns its value or an error.
error BankError:
    NotPositive(amount: int)
    TooLittle(balance: int, wanted: int)

def withdraw(balance: int, amount: int) -> int ! BankError:
    if amount <= 0:
        return Err(NotPositive(amount))
    if amount > balance:
        return Err(TooLittle(balance, amount))
    return Ok(balance - amount)

def main():
    for amount in [30, 0, 500]:
        match withdraw(100, amount):
            case Ok(left): print("took {amount}, {left} left")
            case Err(NotPositive(n)): print("cannot take {n}")
            case Err(TooLittle(balance, wanted)):
                print("wanted {wanted}, but only {balance} is there")
```

<!-- output: examples/08-errors/withdraw.out -->
```text
took 30, 70 left
cannot take 0
wanted 500, but only 100 is there
```

- `error BankError:` lists the variants of the error set, which can
  carry values as an enum's variants do.
- `int ! BankError` is either `Ok(n)`, the new balance, or `Err(e)`,
  an error from `BankError`. `return Ok(balance - amount)` succeeds and
  `return Err(NotPositive(amount))` fails.
- The caller takes the result apart with `match`, and the patterns can
  look inside the error, as `Err(TooLittle(balance, wanted))` does.

> **From Python.** Python raises an exception, which travels up through
> the callers until an `except` catches it, and a function's signature
> does not say which exceptions it can raise. In Sisu the error is part
> of the result type, every caller sees it, and it travels only where
> the code passes it on (D-011).

## Passing an error on with ?

`?` works on results too. On an `Ok`, it gives the value inside; on an
`Err`, it returns the error from the function at once:

<!-- file: examples/08-errors/clock.sisu -->
```sisu
# `?` passes an error on to the caller.
error ClockError:
    NoColon(text: String)
    NotANumber(text: String)
    OutOfRange(value: int)

def number(text: str, limit: int) -> int ! ClockError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    if not (0 <= n < limit):
        return Err(OutOfRange(n))
    return Ok(n)

def minutes_since_midnight(text: str) -> int ! ClockError:
    let hours, minutes = text.split_once(":") else:
        return Err(NoColon(text.to_string()))
    return Ok(number(hours, 24)? * 60 + number(minutes, 60)?)

def main():
    for text in ["7:45", "12.30", "25:00", "8:x"]:
        print("{text}: {minutes_since_midnight(text)}")
```

<!-- output: examples/08-errors/clock.out -->
```text
7:45: Ok(465)
12.30: Err(NoColon(text="12.30"))
25:00: Err(OutOfRange(value=25))
8:x: Err(NotANumber(text="x"))
```

`minutes_since_midnight` calls `number` twice. Each `?` either gives
the number or ends `minutes_since_midnight` with the error that
`number` returned, so the caller sees `OutOfRange(value=25)` for the
hour 25 and `NotANumber(text="x")` for the minutes "x". A result
prints as `Ok(...)` or `Err(...)`, with the error's variant and fields.

**Try it.** In `main`, take the minutes out with `?` before printing
them: `let minutes = minutes_since_midnight(text)?`, then
`print("{text}: {minutes}")`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/08-errors/question_in_main.console -->
```console
$ sisu run question_in_main.sisu
error[E0320]: `?` on `int ! ClockError` needs a function that returns `T ! E`, but this one returns `()`
  --> question_in_main.sisu:21:23
   |
21 |         let minutes = minutes_since_midnight(text)?
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: let `main` return its errors: ` -> ! ClockError`, `    return Ok(())`

error[D0006]: the program was not run: errors found (1)
```

E0320 (`type.try-context`) says that `?` needs a function that can
return the error, and `main` returns nothing. The section "When main
fails" shows a `main` that can; the other fix is to handle the error in
`main`, as the next section does.

</details>

## Handling an error on the spot

`else` works on a result as it does on an optional value, and `else as
e` binds the error so that the fallback can use it. `.or(default)` is
the same as `else` with a value:

<!-- file: examples/08-errors/recover.sisu -->
```sisu
# `else`, `else as e` and `.or` handle an error where it happens.
error ClockError:
    NoColon(text: String)
    NotANumber(text: String)

def hour(text: str) -> int ! ClockError:
    let before, _ = text.split_once(":") else:
        return Err(NoColon(text.to_string()))
    let h = before.parse_int() else:
        return Err(NotANumber(before.to_string()))
    return Ok(h)

def describe(text: str) -> String:
    let h = hour(text) else as e:
        return "no hour in {text:?}: {e}"
    return "hour {h}"

def main():
    let a = hour("7:45") else: 0
    let b = hour("noon") else: 12
    let c = hour("x:15").or(-1)
    print("{a} {b} {c}")
    print(describe("18:20"))
    print(describe("evening"))
```

<!-- output: examples/08-errors/recover.out -->
```text
7 12 -1
hour 18
no hour in "evening": NoColon(text="evening")
```

- `let a = hour("7:45") else: 0` gives `a` the hour, or 0 when there
  is an error.
- `let h = hour(text) else as e:` returns a message that includes the
  error `e`.
- `hour("x:15").or(-1)` gives -1, because "x" is not a number.

`.or_else`, which computes the fallback from the error only when it is
needed, is designed in [05](../../proposal/05-errors.md#handling) and
is not in the compiler yet.

## Error sets that combine

A function can fail in the ways of every function it calls. Its result
type names them all, as a union of error sets in braces:

<!-- file: examples/08-errors/union.sisu -->
```sisu
# A function can fail in the ways of every function it calls.
error ParseError:
    NotANumber(text: String)

error BankError:
    TooLittle(balance: int, wanted: int)

def amount(text: str) -> int ! ParseError:
    let n = text.trim().parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def withdraw(balance: int, wanted: int) -> int ! BankError:
    if wanted > balance:
        return Err(TooLittle(balance, wanted))
    return Ok(balance - wanted)

def withdraw_text(balance: int, text: str) -> int ! {ParseError, BankError}:
    return withdraw(balance, amount(text)?)

def withdraw_twice(balance: int, first: str, second: str) -> int !:
    let left = withdraw_text(balance, first)?
    return withdraw_text(left, second)

def main():
    print(withdraw_text(100, "30"))
    print(withdraw_text(100, "lots"))
    print(withdraw_twice(100, "30", "80"))
```

<!-- output: examples/08-errors/union.out -->
```text
Ok(70)
Err(NotANumber(text="lots"))
Err(TooLittle(balance=70, wanted=80))
```

- `withdraw_text` returns `int ! {ParseError, BankError}`: its errors
  come from either set. `amount(text)?` passes on a `ParseError`, and
  `withdraw` returns a `BankError`, and both fit, with no code to
  convert one into the other. A set fits into any union that includes
  it.
- `withdraw_twice` writes `int !`, with no set after the `!`. The
  compiler works the set out from the body: the union of every error
  that a `?` passes on or a `return` gives. Writing the set out is
  clearer for a function others call; an inferred set suits a helper
  whose set would be long to write.

## Handling some errors and passing on the rest

A caller often knows what to do about one error and not about the
others. A `match` can handle the variants it knows and return the
rest:

<!-- file: examples/08-errors/settings.sisu -->
```sisu
# Handle the errors you can, and pass on the rest.
error ConfigError:
    Missing(key: String)
    NotANumber(key: String, text: String)

def setting(config: dict[str, str], key: str) -> int ! ConfigError:
    let text = config.get(key) else:
        return Err(Missing(key.to_string()))
    let n = text.parse_int() else:
        return Err(NotANumber(key.to_string(), text.to_string()))
    return Ok(n)

def port(config: dict[str, str]) -> int ! ConfigError:
    let port: int
    match setting(config, "port"):
        case Ok(n):
            port = n
        case Err(Missing(_)):
            port = 8080
        case Err(e):
            return Err(e)
    return Ok(port)

def main():
    print(port({"port": "9000"}))
    print(port({}))
    print(port({"port": "eighty"}))
```

<!-- output: examples/08-errors/settings.out -->
```text
Ok(9000)
Ok(8080)
Err(NotANumber(key="port", text="eighty"))
```

- `let port: int` declares a binding without a value. The compiler
  checks that every path assigns it before it is used, so each arm of
  the `match` either assigns `port` or returns.
- `case Err(Missing(_))` handles a missing setting with the default
  port, 8080.
- `case Err(e): return Err(e)` passes on every other error as it is,
  here the text that is not a number.

## A result must be used

A result that nothing looks at would let an error pass in silence, so
it does not compile:

<!-- file: examples/08-errors/unused.sisu -->
```sisu
# A result that nothing looks at does not compile.
error BankError:
    TooLittle(balance: int, wanted: int)

def withdraw(balance: int, wanted: int) -> int ! BankError:
    if wanted > balance:
        return Err(TooLittle(balance, wanted))
    return Ok(balance - wanted)

def main():
    withdraw(100, 500)
    print("done")
```

<!-- output: examples/08-errors/unused.console -->
```console
$ sisu run unused.sisu
error[E0318]: this `int ! BankError` is not used; handle it, or discard it with `_ = ...`
  --> unused.sisu:11:5
   |
11 |     withdraw(100, 500)
   |     ^^^^^^^^^^^^^^^^^^
  = fix: discard it explicitly: `_ = `

error[D0006]: the program was not run: errors found (1)
```

E0318 (`type.unused-result`) says that the result is not used, and its
fix writes `_ = ` before the call. `_ = withdraw(100, 500)` says that
you looked and decided to ignore the result. That is rarely right for
an error; handle it, or pass it on with `?`. The same rule holds for an
optional value, such as the old value that `insert` on a dict returns.

## When main fails

`main` can return a result too. Declared `-> ! BankError`, it returns
nothing on success, which is written `Ok(())`, and can pass errors on
with `?`:

<!-- file: examples/08-errors/main_fails.sisu -->
```sisu
# `main` can return an error, which ends the program with status 1.
error BankError:
    TooLittle(balance: int, wanted: int)

def withdraw(balance: int, wanted: int) -> int ! BankError:
    if wanted > balance:
        return Err(TooLittle(balance, wanted))
    return Ok(balance - wanted)

def main() -> ! BankError:
    let left = withdraw(100, 30)?
    print("{left} left after the first")
    let last = withdraw(left, 80)?
    print("{last} left after the second")
    return Ok(())
```

<!-- output: examples/08-errors/main_fails.console -->
```console
$ sisu run main_fails.sisu
70 left after the first
error: TooLittle(balance=70, wanted=80)
  at main_fails.sisu:13:16
```

The second withdrawal fails, and `?` ends `main` with the error. Sisu
writes `error:` and the error to standard error, followed by the
error's trace: the place of each `?` that passed it on, here one. The
program then exits with status 1, which tells a shell or a script that
it failed. A `main` that reaches `return Ok(())` exits with status 0.

## Context and traces

An error such as `NotANumber(text="kolme")` says what went wrong, but
not what the program was doing at the time. `.context` on a result
wraps an error in a message that says so:

<!-- file: examples/08-errors/context.sisu -->
```sisu
# `.context` says what the program was doing when an error happened.
error ParseError:
    NotANumber(text: String)

def number(text: str) -> int ! ParseError:
    let n = text.trim().parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def total(lines: list[str]) -> int ! Contextual:
    var sum_so_far = 0
    for i, line in enumerate(lines):
        sum_so_far += number(line).context("reading line {i + 1}")?
    return Ok(sum_so_far)

def main():
    match total(["12", "kolme", "30"]):
        case Ok(n):
            print("the total is {n}")
        case Err(e):
            print("error: {e}")
            print(e.trace())
```

<!-- output: examples/08-errors/context.out -->
```text
error: reading line 2: NotANumber(text="kolme")
at context.sisu:13:23
```

`.context("reading line {i + 1}")` turns a `ParseError` into a
`Contextual` error that holds the message and the original error, and
prints as both. `e.trace()` lists the place of each `?` the error
passed through, which in a larger program shows the path from where it
started to where it was handled. *Kolme* is Finnish for "three".

## Panics are bugs

Some failures are not errors that a program should handle, but bugs:
an index past the end of a list, an `assert` that does not hold, a
value that the code made sure is there and is not. These panic. A
panic stops the program with a message and the place where it
happened. `expect` turns a missing value into a panic with your
message:

<!-- file: examples/08-errors/panic.sisu -->
```sisu
# `expect` turns a missing value into a panic, for a bug in the program.
def main():
    let size = "42".parse_int().expect("the size is written in the code")
    print("size {size}")
    let count = "4x".parse_int().expect("the count is a number")
    print("count {count}")
```

<!-- output: examples/08-errors/panic.console -->
```console
$ sisu run panic.sisu
size 42
panic: the count is a number
  at main (panic.sisu:5:17)
```

The first `expect` is fine: "42" is written in the code and is a
number. The second turns a value that should have been checked into a
panic, and the program exits with status 101, the status of a panic.
`panic("message")` panics directly.

The rule for choosing is about who is at fault:

- Text from a user, a file or the network can be wrong in ways the
  program must expect. That is an error: return it, and let the caller
  decide.
- A value that is wrong only if the program itself is wrong is a bug.
  `expect` and `assert` say what the code assumes, and panic when it
  does not hold.

A panic cannot be caught and handled like an error (D-040). Chapter 20
shows how a panic in one task reaches the task that started it.

## Exercise 8.1: predict the fallbacks

Open
[`manual/exercises/08-errors/predict_fallbacks.sisu`](../exercises/08-errors/predict_fallbacks.sisu):

<!-- file: exercises/08-errors/predict_fallbacks.sisu -->
```sisu
##! Exercise 8.1, predict the fallbacks. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_fallbacks.sisu`. The hints are in chapter 8 of
##! the tutorial, under "Exercise 8.1: predict the fallbacks".

error OddError:
    Odd(value: int)

## Half of `n`, which must be even.
def halve(n: int) -> int ! OddError:
    if n % 2 != 0:
        return Err(Odd(n))
    return Ok(n // 2)

## A quarter of `n`, which must be even, and so must its half.
def quarter(n: int) -> int ! OddError:
    let half = halve(n)?
    return halve(half)

const PARSED: int = todo("a number read from text, or 0")
const NOT_PARSED: int = todo("a word read as a number, or 0")
const PAST_THE_END: int = todo("the item at index 5, or -1")
const QUARTER: int = todo("a quarter of 12, or 0")
const NO_QUARTER: int = todo("a quarter of 6, or 0")
const SHOWN: str = todo("a result as text")

test "a number read from text":
    assert ("17".parse_int() ?? 0) == PARSED

test "a word read as a number":
    assert ("seventeen".parse_int() ?? 0) == NOT_PARSED

test "an index past the end":
    assert ([10, 20, 30].get(5) ?? -1) == PAST_THE_END

test "two halvings":
    assert quarter(12).or(0) == QUARTER

test "a halving that fails":
    assert quarter(6).or(0) == NO_QUARTER

test "a result as text":
    assert "{halve(5)}" == SHOWN
```

<!-- output: solutions/08-errors/predict_fallbacks.console -->
```console
$ sisu test predict_fallbacks.sisu
warning[W0001]: typed hole `a number read from text, or 0` of type `int`
  --> predict_fallbacks.sisu:22:21
   |
22 | const PARSED: int = todo("a number read from text, or 0")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a word read as a number, or 0` of type `int`
  --> predict_fallbacks.sisu:23:25
   |
23 | const NOT_PARSED: int = todo("a word read as a number, or 0")
   |                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the item at index 5, or -1` of type `int`
  --> predict_fallbacks.sisu:24:27
   |
24 | const PAST_THE_END: int = todo("the item at index 5, or -1")
   |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a quarter of 12, or 0` of type `int`
  --> predict_fallbacks.sisu:25:22
   |
25 | const QUARTER: int = todo("a quarter of 12, or 0")
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a quarter of 6, or 0` of type `int`
  --> predict_fallbacks.sisu:26:25
   |
26 | const NO_QUARTER: int = todo("a quarter of 6, or 0")
   |                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a result as text` of type `str`
  --> predict_fallbacks.sisu:27:20
   |
27 | const SHOWN: str = todo("a result as text")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^

test "a number read from text" ... blocked by todo("a number read from text, or 0")
test "a word read as a number" ... blocked by todo("a word read as a number, or 0")
test "an index past the end" ... blocked by todo("the item at index 5, or -1")
test "two halvings" ... blocked by todo("a quarter of 12, or 0")
test "a halving that fails" ... blocked by todo("a quarter of 6, or 0")
test "a result as text" ... blocked by todo("a result as text")
0 passed, 0 failed, 6 blocked
```

<details>
<summary>Hint 1</summary>

`??` gives the value inside `Some`, or the value after it when there
is `None`. `.or` does the same for a result and its error.

</details>

<details>
<summary>Hint 2</summary>

Follow `quarter(6)` one line at a time: what does the first `halve`
give, and what does `?` do with what the second gives? A result in
text looks the way `clock.sisu` printed its results.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/08-errors/predict_fallbacks.sisu`](../solutions/08-errors/predict_fallbacks.sisu).

## Exercise 8.2: a scoreboard

Open
[`manual/exercises/08-errors/scoreboard.sisu`](../exercises/08-errors/scoreboard.sisu):

<!-- file: exercises/08-errors/scoreboard.sisu -->
```sisu
##! Exercise 8.2, a scoreboard. Kind: fix. Level: 1.
##!
##! `total` adds up a scoreboard that has one score on each line. A line
##! that is not a number, such as a note that a player did not finish,
##! counts as 0. The file does not compile. Run
##! `sisu test scoreboard.sisu`, read the diagnostic, and fix `total`
##! so that every test passes. The hints are in chapter 8 of the
##! tutorial, under "Exercise 8.2: a scoreboard".

## The sum of the scores on the lines of `board`; a line that is not a
## number counts as 0.
def total(board: str) -> int:
    var sum_so_far = 0
    for line in board.lines():
        sum_so_far += line.trim().parse_int()
    return sum_so_far

test "numbers only":
    assert total("12\n30\n7") == 49
    assert total("5") == 5

test "a line that is not a number":
    assert total("12\ndid not finish\n30") == 42
    assert total("none") == 0
```

<!-- output: solutions/08-errors/scoreboard.console -->
```console
$ sisu test scoreboard.sisu
error[E0310]: `+` cannot be applied to `int` and `int?`
  --> scoreboard.sisu:15:9
   |
15 |         sum_so_far += line.trim().parse_int()
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: the value may be missing: give a default with `??`, or take it apart with `if let` or `match`

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

`parse_int` cannot know whether the line holds a number, so it gives
an optional value. The note in the diagnostic lists the ways to take
the number out.

</details>

<details>
<summary>Hint 2</summary>

The doc comment says what a line that is not a number counts as. One
of the ways in the note takes that value as its default.

</details>

When every test passes, compare your answer with
[`manual/solutions/08-errors/scoreboard.sisu`](../solutions/08-errors/scoreboard.sisu).

## Exercise 8.3: a setting with fallbacks

Open
[`manual/exercises/08-errors/language.sisu`](../exercises/08-errors/language.sisu):

<!-- file: exercises/08-errors/language.sisu -->
```sisu
##! Exercise 8.3, a setting with fallbacks. Kind: complete. Level: 2.
##!
##! A chat program writes its messages in the language a user chose,
##! or else in the one the user's team chose, or else in English, "en".
##! A choice must be a language the program has messages in. Replace the
##! two holes so that every test passes. Run the tests with
##! `sisu test language.sisu`. The hints are in chapter 8 of the
##! tutorial, under "Exercise 8.3: a setting with fallbacks".

error SettingError:
    UnknownLanguage(code: String)

type Settings = dict[str, str]

## Whether the program has messages in the language `code`.
def known(code: str) -> bool:
    return code in ["en", "fi", "sv"]

## The language for a user whose settings are `user` and whose team's
## settings are `team`; each may have the key "language".
def language(user: Settings, team: Settings) -> String ! SettingError:
    let code: str = todo("the code chosen, or a fallback")
    if not known(code):
        return todo("the error for an unknown code")
    return Ok(code.to_string())

test "the user's own choice":
    assert language({"language": "fi"}, {"language": "sv"}) == Ok("fi")
    assert language({"language": "sv"}, {}) == Ok("sv")

test "the team's choice":
    assert language({}, {"language": "sv"}) == Ok("sv")
    assert language({"theme": "dark"}, {"language": "fi"}) == Ok("fi")

test "no choice at all":
    assert language({}, {}) == Ok("en")

test "a language without messages":
    assert language({"language": "de"}, {}) == Err(UnknownLanguage("de"))
    assert language({}, {"language": "xx"}) == Err(UnknownLanguage("xx"))
```

<!-- output: solutions/08-errors/language.console -->
```console
$ sisu test language.sisu
warning[W0001]: typed hole `the code chosen, or a fallback` of type `str`
  --> language.sisu:22:21
   |
22 |     let code: str = todo("the code chosen, or a fallback")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the error for an unknown code` of type `String ! SettingError`
  --> language.sisu:24:16
   |
24 |         return todo("the error for an unknown code")
   |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("the code chosen, or a fallback")`:
  user = {"language": "fi"}
  team = {"language": "sv"}
in scope at the typed hole `todo("the code chosen, or a fallback")`:
  user = {}
  team = {"language": "sv"}
in scope at the typed hole `todo("the code chosen, or a fallback")`:
  user = {}
  team = {}
in scope at the typed hole `todo("the code chosen, or a fallback")`:
  user = {"language": "de"}
  team = {}
test "the user's own choice" ... blocked by todo("the code chosen, or a fallback")
test "the team's choice" ... blocked by todo("the code chosen, or a fallback")
test "no choice at all" ... blocked by todo("the code chosen, or a fallback")
test "a language without messages" ... blocked by todo("the code chosen, or a fallback")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

`get` on a dict gives an optional value. The first hole needs the
user's choice, or else the team's, or else a code written in the
function.

</details>

<details>
<summary>Hint 2</summary>

The second hole is the result of a function whose type is
`String ! SettingError`, for a code that `known` rejected. The section
"Errors are values" shows how a function returns an error.

</details>

When every test passes, compare your answer with
[`manual/solutions/08-errors/language.sisu`](../solutions/08-errors/language.sisu).

## Exercise 8.4: a version number

Open
[`manual/exercises/08-errors/version.sisu`](../exercises/08-errors/version.sisu):

<!-- file: exercises/08-errors/version.sisu -->
```sisu
##! Exercise 8.4, a version number. Kind: write. Level: 2.
##!
##! A version number such as "2.10.1" has three parts, the major, minor
##! and patch numbers, with a dot between each two. Write the body of
##! `parse_version` so that every test passes. Run the tests with
##! `sisu test version.sisu`. The hints are in chapter 8 of the
##! tutorial, under "Exercise 8.4: a version number".

error VersionError:
    Empty
    WrongCount(found: int)
    NotANumber(part: String)

@derive(Copy, Eq, Ord)
struct Version:
    major: int
    minor: int
    patch: int

## The version that `text` spells; spaces around it do not count.
def parse_version(text: str) -> Version ! VersionError:
    return todo("parse_version")

test "versions":
    assert parse_version("2.10.1") == Ok(Version(2, 10, 1))
    assert parse_version(" 0.9.0 ") == Ok(Version(0, 9, 0))

test "an empty text":
    assert parse_version("") == Err(Empty)
    assert parse_version("   ") == Err(Empty)

test "the wrong number of parts":
    assert parse_version("1.2") == Err(WrongCount(2))
    assert parse_version("1.2.3.4") == Err(WrongCount(4))

test "a part that is not a number":
    assert parse_version("1.x.3") == Err(NotANumber("x"))
    assert parse_version("1..3") == Err(NotANumber(""))

test "versions in order":
    let older = parse_version("1.9.0").or(Version(0, 0, 0))
    let newer = parse_version("1.10.0").or(Version(0, 0, 0))
    assert older < newer
```

<!-- output: solutions/08-errors/version.console -->
```console
$ sisu test version.sisu
warning[W0001]: typed hole `parse_version` of type `Version ! VersionError`
  --> version.sisu:22:12
   |
22 |     return todo("parse_version")
   |            ^^^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("parse_version")`:
  text = "2.10.1"
in scope at the typed hole `todo("parse_version")`:
  text = ""
in scope at the typed hole `todo("parse_version")`:
  text = "1.2"
in scope at the typed hole `todo("parse_version")`:
  text = "1.x.3"
in scope at the typed hole `todo("parse_version")`:
  text = "1.9.0"
test "versions" ... blocked by todo("parse_version")
test "an empty text" ... blocked by todo("parse_version")
test "the wrong number of parts" ... blocked by todo("parse_version")
test "a part that is not a number" ... blocked by todo("parse_version")
test "versions in order" ... blocked by todo("parse_version")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

Check the ways the text can be wrong in the order the tests need: the
empty text first, then the number of parts, then each part. A
comprehension over `split(".")` gives the parts as a list.

</details>

<details>
<summary>Hint 2</summary>

Each part goes through the same step, a number or `NotANumber`. A
small function for that step, returning `int ! VersionError`, lets `?`
pass the error on.

</details>

When every test passes, compare your answer with
[`manual/solutions/08-errors/version.sisu`](../solutions/08-errors/version.sisu).

## Exercise 8.5: three layers of errors

Open
[`manual/exercises/08-errors/orders.sisu`](../exercises/08-errors/orders.sisu):

<!-- file: exercises/08-errors/orders.sisu -->
```sisu
##! Exercise 8.5, three layers of errors. Kind: write. Level: 3.
##!
##! A café takes orders as text, one item on each line: a product, a
##! space and a quantity, such as "coffee 2". `parse_line` reads one
##! line, `parse_order` reads every line through it, and `order_total`
##! prices the order that `parse_order` read. Each passes on the errors
##! of the one it calls. Write the three bodies so that every test
##! passes. Run the tests with `sisu test orders.sisu`. The hints are in
##! chapter 8 of the tutorial, under "Exercise 8.5: three layers of
##! errors".

error LineError:
    NoQuantity(line: String)
    BadQuantity(text: String)

error PriceError:
    UnknownProduct(name: String)

type Prices = dict[str, int]

## The product and the quantity on `line`, such as ("coffee", 2) for
## "coffee 2".
def parse_line(line: str) -> (String, int) ! LineError:
    return todo("parse_line")

## The items of the order `text`, one for each line that is not blank.
def parse_order(text: str) -> list[(String, int)] ! LineError:
    return todo("parse_order")

## The price in cents of the order `text`, with `prices` in cents.
def order_total(text: str, prices: Prices) -> int ! {LineError, PriceError}:
    return todo("order_total")

## The café's prices, in cents.
def menu() -> Prices:
    return {"coffee": 350, "tea": 300, "bun": 280}

test "one line":
    assert parse_line("coffee 2") == Ok(("coffee", 2))
    assert parse_line("bun 12") == Ok(("bun", 12))

test "lines without a quantity":
    assert parse_line("coffee") == Err(NoQuantity("coffee"))
    assert parse_line("tea x") == Err(BadQuantity("x"))

test "an order of several lines":
    let items = parse_order("coffee 2\n\nbun 1\n")
    assert items == Ok([("coffee", 2), ("bun", 1)])

test "the total of an order":
    assert order_total("coffee 2\nbun 1", menu()) == Ok(980)
    assert order_total("", menu()) == Ok(0)

test "errors from every layer":
    assert order_total("tea 1\nbun", menu()) == Err(NoQuantity("bun"))
    assert order_total("tea two", menu()) == Err(BadQuantity("two"))
    assert order_total("cocoa 1", menu()) == Err(UnknownProduct("cocoa"))
```

<!-- output: solutions/08-errors/orders.console -->
```console
$ sisu test orders.sisu
warning[W0001]: typed hole `parse_line` of type `(String, int) ! LineError`
  --> orders.sisu:24:12
   |
24 |     return todo("parse_line")
   |            ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `parse_order` of type `list[(String, int)] ! LineError`
  --> orders.sisu:28:12
   |
28 |     return todo("parse_order")
   |            ^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `order_total` of type `int ! {LineError, PriceError}`
  --> orders.sisu:32:12
   |
32 |     return todo("order_total")
   |            ^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("parse_line")`:
  line = "coffee 2"
in scope at the typed hole `todo("parse_line")`:
  line = "coffee"
in scope at the typed hole `todo("parse_order")`:
  text = "coffee 2\n\nbun 1\n"
in scope at the typed hole `todo("order_total")`:
  text = "coffee 2\nbun 1"
  prices = {"coffee": 350, "tea": 300, "bun": 280}
in scope at the typed hole `todo("order_total")`:
  text = "tea 1\nbun"
  prices = {"coffee": 350, "tea": 300, "bun": 280}
test "one line" ... blocked by todo("parse_line")
test "lines without a quantity" ... blocked by todo("parse_line")
test "an order of several lines" ... blocked by todo("parse_order")
test "the total of an order" ... blocked by todo("order_total")
test "errors from every layer" ... blocked by todo("order_total")
0 passed, 0 failed, 5 blocked
```

`type Prices = dict[str, int]` is an alias, as in chapter 7.

<details>
<summary>Hint 1</summary>

Write `parse_line` first and run the tests: the first two test it
alone. `split_once(" ")` cuts a line in two at its first space, and
gives `None` when there is no space.

</details>

<details>
<summary>Hint 2</summary>

`parse_order` and `order_total` each call the layer below once per
line or item, and pass its errors on with `?`. Only `order_total`
knows about prices, so only it returns `UnknownProduct`.

</details>

<details>
<summary>Hint 3</summary>

The result types already allow every error to pass up unchanged; no
layer converts the errors of the layer below.

</details>

When every test passes, compare your answer with
[`manual/solutions/08-errors/orders.sisu`](../solutions/08-errors/orders.sisu).

## What you learned

- `T?` is a value that may be absent: `Some(x)` or `None`. `if let`,
  chained conditions, `match` and `while let` take the value out; `??`
  gives a default and `?.` reads a field if the value is present.
- `let ... else` gives a fallback value or leaves the function; `?`
  returns `None` or an error from the function at once.
- `error` declares an error set, and `T ! E` is a result: `Ok(value)`
  or `Err(error)`. `else`, `else as e` and `.or` handle an error where
  it happens, and `match` handles some variants and passes the others
  on.
- Error sets combine into unions with no conversion, and `T !` infers
  the set. A result must be used, or discarded with `_ =`.
- `main` can return `! E`; an error from it exits with status 1.
  `.context` adds what the program was doing, and `trace()` shows the
  path of the error.
- Panics are for bugs: `expect`, `assert` and `panic` stop the program
  with status 101.

Chapter 9 puts the chapters of Part I together in a complete program:
a word counter that reads files, takes options from its command line,
and is tested against files that exist only in memory.
