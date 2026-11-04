# 2. Values and types

In this chapter you meet the values that programs are made of: whole
numbers, floating-point numbers, truth values, characters and text. You
do arithmetic, see what Sisu does when a number grows too large for its
type, compare values, combine conditions, give a name a value that
changes, and turn one kind of number into another. Most of the rules are
Python's, and the chapter points out each one that is not. You need
about an hour.

## Five basic types

Every value has a type, which says what the value is and what you can do
with it. Five types cover most of what a program holds. This worksheet
makes a value of each:

<!-- file: examples/02-values/basic_types.sisu -->
```sisu
#!/usr/bin/env sisu
# One value of each basic type.
let count = 42
let price = 2.5
let ready = True
let initial = 'A'
let name = "Aino"
```

`sisu ws basic_types.sisu` shows each value:

<!-- output: examples/02-values/basic_types.ws -->
```text
#!/usr/bin/env sisu
# One value of each basic type.
let count = 42  # => 42
let price = 2.5  # => 2.5
let ready = True  # => True
let initial = 'A'  # => 'A'
let name = "Aino"  # => "Aino"
```

The five names have these types:

| Name | Type | What it holds |
|---|---|---|
| `count` | `int` | a whole number of 64 bits, from -9223372036854775808 to 9223372036854775807 |
| `price` | `f64` | a floating-point number of 64 bits, like Python's `float` |
| `ready` | `bool` | `True` or `False` |
| `initial` | `char` | one Unicode character, written between single quotes |
| `name` | `str` | text, written between double quotes |

You wrote none of these types. Sisu works each one out from the value,
and it checks every use of each name against its type before the program
runs. `int` is also the type of every count, length and index: `len(name)`
is an `int`.

**Try it.** Put a name between single quotes in a program, as in `let
name = 'Aino'`, and run it.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/02-values/single_quotes.console -->
```console
$ sisu run single_quotes.sisu
error[E0016]: a character literal holds exactly one character
  --> single_quotes.sisu:3:16
  |
3 |     let name = 'Aino'
  |                ^^^^^^
  = fix: text is written in double quotes: `"`, `"`

error[D0006]: the program was not run: errors found (1)
```

Single quotes make a `char`, which is one character, so four characters
between them are an error. The fix line puts double quotes in their
place.

</details>

> **From Python.** Python lets text stand between single or double
> quotes. In Sisu, text is always between double quotes, and single
> quotes are for one character.

## Writing numbers

A number can be written in several ways. All of these are ordinary
numbers once the program runs:

<!-- file: examples/02-values/literals.sisu -->
```sisu
# Ways to write a number.
def main():
    let population = 5_600_000
    let red = 0xFF
    let flags = 0b1010
    let mode = 0o755
    let level = 200u8
    let wavelength = 6.5e-7
    let distance = 1.5e8
    print("{population} {red} {flags} {mode} {level}")
    print("{wavelength} {distance}")
```

`sisu run literals.sisu` prints:

<!-- output: examples/02-values/literals.out -->
```text
5600000 255 10 493 200
6.5e-7 150000000.0
```

- Underscores separate groups of digits, and Sisu ignores them:
  `5_600_000` is 5600000.
- `0x` starts a hexadecimal number, `0b` a binary one and `0o` an octal
  one. Each is an `int`, and `print` shows it in decimal.
- A suffix names a type: `200u8` is a `u8`, an unsigned integer of 8
  bits that holds 0 to 255. The sized integers, `i8` to `i128` and `u8`
  to `u128`, are for data with a fixed layout, such as the bytes of a
  file; chapter 24 covers them. Use `int` for everything else.
- A number with a decimal point or an exponent is an `f64`: `6.5e-7` is
  6.5 times 10 to the power -7. Sisu prints a float with the fewest
  digits that read back as the same number.

## Arithmetic

The arithmetic operators are Python's. This worksheet applies each of
them to 7 and 2:

<!-- file: examples/02-values/arithmetic.sisu -->
```sisu
#!/usr/bin/env sisu
# Arithmetic on integers and on floats.
7 + 2
7 - 2
7 * 2
7 ** 2
7 // 2
7 % 2
(-7) // 2
(-7) % 2
7.0 / 2.0
abs(-7)
```

<!-- output: examples/02-values/arithmetic.ws -->
```text
#!/usr/bin/env sisu
# Arithmetic on integers and on floats.
7 + 2  # => 9
7 - 2  # => 5
7 * 2  # => 14
7 ** 2  # => 49
7 // 2  # => 3
7 % 2  # => 1
(-7) // 2  # => -4
(-7) % 2  # => 1
7.0 / 2.0  # => 3.5
abs(-7)  # => 7
```

- `+`, `-` and `*` add, subtract and multiply. `**` raises to a power
  and groups from the right, as in mathematics.
- `//` divides two integers and rounds down, towards minus infinity:
  `(-7) // 2` is -4, not -3.
- `%` is the remainder, and it takes the sign of the divisor, so that
  `(a // b) * b + a % b` is always `a`.
- `/` divides two floats. A float is a binary fraction, so most decimal
  fractions, such as 0.1, are stored as the nearest binary one, and a
  result can differ from the exact one in its last digit.
- `abs` gives a number's absolute value. Like `len`, it is a prelude
  function: one that every file can call without importing it.

A worksheet puts `-7` in parentheses because a line that starts with an
operator, `-` included, continues the line before it: without them, it
would read `7 % 2 - 7 // 2`.

## Dividing integers

`/` divides floats only. Divide two integers with it, and the program
does not compile:

<!-- file: examples/02-values/average.sisu -->
```sisu
# Dividing two integers with `/` does not compile.
def main():
    let total = 17
    let count = 4
    print("average: {total / count}")
```

<!-- output: examples/02-values/average.console -->
```console
$ sisu run average.sisu
error[E0301]: `/` divides floats; for integers use `//`, or convert to `f64` first
  --> average.sisu:5:22
  |
5 |     print("average: {total / count}")
  |                      ^^^^^^^^^^^^^
  = fix: use `//` for integer division: `//`

error[D0006]: the program was not run: errors found (1)
```

E0301 (`type.integer-division`) names two ways out, and the fix line
offers the first. Which one is right depends on what the program means:

- `//` keeps the division in whole numbers and drops the fraction, and
  `%` gives what is left over. Choose it for things that come whole: 17
  apples shared by 4 people is 4 each, with 1 left over.
- To keep the fraction, turn both integers into floats with `.to_f64()`
  before dividing. Choose it for measurements, such as an average.

This program does both:

<!-- file: examples/02-values/average_fixed.sisu -->
```sisu
# Two ways to divide integers: in whole numbers, or as floats.
def main():
    let total = 17
    let count = 4
    print("whole: {total // count}, left over: {total % count}")
    print("exact: {total.to_f64() / count.to_f64()}")
```

<!-- output: examples/02-values/average_fixed.out -->
```text
whole: 4, left over: 1
exact: 4.25
```

`.to_f64()` is a method: a function that belongs to a type, called with
a dot after a value. The section "Converting numbers" has more of them.

> **From Python.** Python 3's `/` turns two integers into a float
> without asking. In Sisu `7 / 2` does not compile, so code that divides
> integers says which division it means. `//` and `%` mean the same in
> both languages, for negative numbers too.

## Numbers that do not fit

An `int` has 64 bits. When a result does not fit in them, Sisu stops the
program rather than give a wrong answer:

<!-- file: examples/02-values/overflow.sisu -->
```sisu
# An `int` holds 64 bits; a result that does not fit stops the program.
def main():
    let largest = 9_223_372_036_854_775_807
    print("largest: {largest}")
    print("one more: {largest + 1}")
```

<!-- output: examples/02-values/overflow.console -->
```console
$ sisu run overflow.sisu
largest: 9223372036854775807
panic: integer overflow: 9223372036854775807 + 1
  at main (overflow.sisu:5:23)
```

The first `print` ran. The addition in the second stopped the program
with a panic, which reports a bug: what went wrong and where, here line
5, column 23. Nothing after a panic runs, and the program exits with
status 101. Chapter 8 tells panics apart from the errors that a program
expects and handles. Every build checks for overflow, the
optimised one too.

Some algorithms want arithmetic that wraps around, such as a hash
function, and some want it to stop at the limit, such as a volume
control. Sisu has an operator for each, and you write which one you
mean:

<!-- file: examples/02-values/wrapping.sisu -->
```sisu
#!/usr/bin/env sisu
# Wrapping and saturating arithmetic, spelled out.
let largest = 9_223_372_036_854_775_807
largest +% 1
largest +| 1
let level = 250u8
level +% 10
level +| 10
level -| 251
```

<!-- output: examples/02-values/wrapping.ws -->
```text
#!/usr/bin/env sisu
# Wrapping and saturating arithmetic, spelled out.
let largest = 9_223_372_036_854_775_807  # => 9223372036854775807
largest +% 1  # => -9223372036854775808
largest +| 1  # => 9223372036854775807
let level = 250u8  # => 250
level +% 10  # => 4
level +| 10  # => 255
level -| 251  # => 0
```

- `+%` wraps: past the largest value, it carries on from the smallest,
  as the bits in the hardware do. `-%` and `*%` wrap too.
- `+|` saturates: it stops at the largest or the smallest value. `-|`
  and `*|` saturate too.
- A `u8` holds 0 to 255, so `250 +% 10` wraps around to 4, `250 +| 10`
  stops at 255, and `250 -| 251` stops at 0.

> **From Python.** A Python `int` grows as large as memory allows. A
> Sisu `int` has a fixed size, as in most compiled languages, and it
> never wraps around unless you ask. Chapter 24 introduces `BigInt`, for
> integers of any size.

## Comparing values

The comparison operators are `==`, `!=`, `<`, `<=`, `>` and `>=`, and
each gives a `bool`:

<!-- file: examples/02-values/comparisons.sisu -->
```sisu
#!/usr/bin/env sisu
# Comparisons give a `bool`.
3 == 3
3 != 4
2.5 < 3.0
"Aino" < "Eero"
'a' < 'b'
let age = 34
18 <= age < 65
0 < age < 18
```

<!-- output: examples/02-values/comparisons.ws -->
```text
#!/usr/bin/env sisu
# Comparisons give a `bool`.
3 == 3  # => True
3 != 4  # => True
2.5 < 3.0  # => True
"Aino" < "Eero"  # => True
'a' < 'b'  # => True
let age = 34  # => 34
18 <= age < 65  # => True
0 < age < 18  # => False
```

- `==` and `!=` compare values of one type. `1 == 1.0` does not
  compile, because an `int` and an `f64` are different types.
- Text compares character by character, by each character's number in
  Unicode. For English words in one case, that is the order of a
  dictionary.
- `18 <= age < 65` is a chain. It means `18 <= age and age < 65`, and it
  works out `age` once.

A chain must point one way. A chain that goes up and then down is almost
always a mistake, so it does not compile:

<!-- file: examples/02-values/wrong_chain.sisu -->
```sisu
# A chain of comparisons must point one way.
def main():
    let low = 1
    let peak = 5
    let high = 3
    print("{low < peak > high}")
```

<!-- output: examples/02-values/wrong_chain.console -->
```console
$ sisu run wrong_chain.sisu
error[E0113]: a chained comparison must point one way; `<` and `>` do not
  --> wrong_chain.sisu:6:24
  |
6 |     print("{low < peak > high}")
  |                        ^

error[D0006]: the program was not run: errors found (1)
```

E0113 (`parse.mixed-chain`) has no fix line, because only you know what
you meant. If you meant both comparisons, write them both and join them
with `and`: `low < peak and peak > high`.

> **From Python.** Python accepts `a < b > c` and gives it the meaning
> `a < b and b > c`. Sisu accepts a chain only when every operator
> points the same way, so a chain always means "in this order".

## Truth values and logic

`and`, `or` and `not` combine `bool` values. They are words, as in
Python:

<!-- file: examples/02-values/logic.sisu -->
```sisu
#!/usr/bin/env sisu
# `and`, `or` and `not` combine `bool` values.
let raining = True
let have_umbrella = False
raining and not have_umbrella
raining or have_umbrella
not raining
(raining and have_umbrella) or not raining
```

<!-- output: examples/02-values/logic.ws -->
```text
#!/usr/bin/env sisu
# `and`, `or` and `not` combine `bool` values.
let raining = True  # => True
let have_umbrella = False  # => False
raining and not have_umbrella  # => True
raining or have_umbrella  # => True
not raining  # => False
(raining and have_umbrella) or not raining  # => False
```

- `a and b` is `True` when both are. `a or b` is `True` when either is.
  Both stop as soon as the answer is known, so `b` runs only when it
  matters.
- `not` applies to everything after it up to an `and` or an `or`: `not
  a == b` means `not (a == b)`.
- Where `and` and `or` meet, parentheses say which goes first. Without
  them, the line does not compile.

**Try it.** Write `raining and have_umbrella or have_coat` without
parentheses, in a program, and run it.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/02-values/mixed_logic.console -->
```console
$ sisu run mixed_logic.sisu
error[E0108]: `and` and `or` cannot be mixed without parentheses
  --> mixed_logic.sisu:6:39
  |
6 |     print("{raining and have_umbrella or have_coat}")
  |                                       ^^

error[D0006]: the program was not run: errors found (1)
```

Python would read the line as `(raining and have_umbrella) or
have_coat`, which a reader can take for the other grouping. Sisu asks
you to write the parentheses.

</details>

Only a `bool` is a truth value. A number, a string or a list is never
`True` or `False` by itself, as this program shows. `if` runs the
indented lines under it when its condition is `True`; chapter 4 covers
it.

<!-- file: examples/02-values/truthy.sisu -->
```sisu
# A number is not a truth value.
def main():
    let unread = 3
    if unread:
        print("{unread} new messages")
```

<!-- output: examples/02-values/truthy.console -->
```console
$ sisu run truthy.sisu
error[E0302]: a condition must be a `bool`, not `int`; Sisu has no truthiness
  --> truthy.sisu:4:8
  |
4 |     if unread:
  |        ^^^^^^
  = note: write the test explicitly, as in `not xs.is_empty()` or `x != 0`

error[D0006]: the program was not run: errors found (1)
```

E0302 (`type.not-bool`) asks you to write the test you mean, here
`unread > 0` or `unread != 0`. The same rule holds for `and`, `or` and
`not`: each takes `bool` values and gives a `bool`.

> **From Python.** In Python, `0`, `""` and `[]` count as false in a
> condition, and `x or default` gives `x` itself when `x` counts as true.
> Sisu has no such counting: a condition is a `bool`, and `or` gives a
> `bool`. Chapter 8 shows Sisu's way to fall back on a default.

## Names whose value changes

A name made with `let` keeps its value. A name made with `var` can take
a new one:

<!-- file: examples/02-values/score.sisu -->
```sisu
# `var` makes a binding whose value can change.
def main():
    var score = 10
    score = score + 5
    score *= 2
    print("score: {score}")
```

`sisu run score.sisu` prints:

<!-- output: examples/02-values/score.out -->
```text
score: 30
```

- `score = score + 5` gives `score` a new value. An assignment is a
  statement on a line of its own: it has no value, so it cannot stand
  inside another expression.
- `score *= 2` is short for `score = score * 2`. The arithmetic
  operators have such forms: `+=`, `-=`, `*=`, `/=`, `//=`, `%=` and
  `**=`.

A name made with `let` or `var` is called a binding. Use `let` unless
the value has to change: a reader of a `let` knows that its value stays
the same.

**Try it.** Change `var` to `let` in `score.sisu` and run it again.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/02-values/let_score.console -->
```console
$ sisu run let_score.sisu
error[E0312]: `score` is immutable; declare it with `var` to assign to it
  --> let_score.sisu:4:5
  |
4 |     score = score + 5
  |     ^^^^^
  |
3 |     let score = 10
  |         ----- `score` is defined here
  = fix: declare `score` with `var`

error[E0312]: `score` is immutable; declare it with `var` to assign to it
  --> let_score.sisu:5:5
  |
5 |     score *= 2
  |     ^^^^^
  |
3 |     let score = 10
  |         ----- `score` is defined here
  = fix: declare `score` with `var`

error[D0006]: the program was not run: errors found (2)
```

E0312 (`type.assign-immutable`) points at each assignment and, under
it, at the `let` that made the binding. The fix changes that `let` to
`var`.

</details>

> **From Python.** Python lets you assign to any name at any time. In
> Sisu a binding changes only if it was made with `var`, and there is no
> `:=`: an assignment is never part of an expression.

## Constants

A constant is a name for a value that never changes and that the
compiler works out before the program runs:

<!-- file: examples/02-values/constants.sisu -->
```sisu
# `const` names a value that is fixed when the program is compiled.
const SECONDS_PER_MINUTE: int = 60
const MINUTES_PER_HOUR: int = 60
const SECONDS_PER_HOUR: int = SECONDS_PER_MINUTE * MINUTES_PER_HOUR

def main():
    let hours = 3
    print("{hours} hours is {hours * SECONDS_PER_HOUR} seconds")
```

<!-- output: examples/02-values/constants.out -->
```text
3 hours is 10800 seconds
```

- A `const` stands at the top level of a file, outside any function,
  and every function in the file can use it.
- Its type is always written, after a colon.
- Its value is computed when the program is compiled, and it can use
  other constants, as `SECONDS_PER_HOUR` does.
- Its name is in capitals, with underscores between the words.

A `let` inside a function is computed each time its line runs, from
values that can differ from run to run. A `const` is one value for the
whole program. Chapter 25 shows how much a constant's value can
compute.

## Naming rules

A name is made of letters, digits and underscores, and starts with a
letter or an underscore. The letters can be from any alphabet: `päivä`
(Finnish for "day") is a name. How a name is written follows Python's
style guide, and Sisu gives the case of its first letter a meaning:

- `snake_case` for bindings and functions: `unread_count`,
  `have_umbrella`.
- `CamelCase` for types and their variants, which chapter 7 introduces.
  The builtin types with a literal form are the exceptions, written in
  lowercase as in Python: `int`, `f64`, `bool`, `char`, `str`.
- `SCREAMING_CASE` for constants.

Keep binding names in lowercase: Sisu reads a capitalised name in a
`let` as a constant or a variant to compare with (chapter 7).

Sisu also reserves 47 keywords, which cannot be names. Many of them are
keywords in Python too, such as `def`, `if` and `for`. A few are words
that Python programs use as names, and those are the ones to watch
for:

<!-- file: examples/02-values/keyword_name.sisu -->
```sisu
# A keyword cannot be the name of a binding.
def main():
    let type = "csv"
    print("reading a {type} file")
```

<!-- output: examples/02-values/keyword_name.console -->
```console
$ sisu run keyword_name.sisu
error[E0105]: expected a pattern, found `type`
  --> keyword_name.sisu:3:9
  |
3 |     let type = "csv"
  |         ^^^^

error[E0103]: expected an expression, found `type`
  --> keyword_name.sisu:4:23
  |
4 |     print("reading a {type} file")
  |                       ^^^^

error[D0006]: the program was not run: errors found (2)
```

E0105 (`parse.expected-pattern`) and E0103 (`parse.expected-expression`)
say that the parser met the keyword `type` where it expected a name. The
diagnostics do not say that the word is a keyword; the fix is another
name, such as `kind` or `file_type`.

**Try it.** Rename `type` to `Kind` in `keyword_name.sisu`, in both
places, and run it.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/02-values/capital_name.console -->
```console
$ sisu run capital_name.sisu
error[E0327]: `str` has no variant `Kind`
  --> capital_name.sisu:3:9
  |
3 |     let Kind = "csv"
  |         ^^^^
  = fix: a binding's name starts with a lowercase letter: `kind`: `kind`

error[E0329]: this pattern might not match; add `else:` with a block that leaves the scope
  --> capital_name.sisu:3:9
  |
3 |     let Kind = "csv"
  |         ^^^^

error[E0200]: cannot find `Kind` in this scope
  --> capital_name.sisu:4:23
  |
4 |     print("reading a {Kind} file")
  |                       ^^^^

error[D0006]: the program was not run: errors found (3)
```

The three diagnostics have one cause. Sisu reads `let Kind = "csv"` as
a comparison of `"csv"` with a variant named `Kind`, which a `str` does
not have (E0327, `type.unknown-variant`) and which could fail to match
(E0329, `type.refutable-let`). No binding is made, so the next line
cannot find `Kind` (E0200). The fix of E0327 is the one to take: the
name in lowercase, `kind`.

</details>

> **From Python.** `type`, `match`, `error`, `test`, `view`, `ref` and
> `own` are ordinary names in Python and keywords in Sisu, so a port
> renames them.

## Writing a type down

Sisu works out the type of a `let` from its value: an integer literal
alone is an `int`, and a float literal alone is an `f64`. A type
annotation, a colon and a type after the name, states the type instead:

<!-- file: examples/02-values/annotations.sisu -->
```sisu
# A type annotation states the type; without one, Sisu infers it.
def main():
    let year = 2026
    let ratio: f64 = 0.75
    let level: u8 = 200
    let ready: bool = True
    print("{year} {ratio} {level} {ready}")
```

<!-- output: examples/02-values/annotations.out -->
```text
2026 0.75 200 True
```

- `year` has no annotation, so it is an `int`.
- `ratio: f64` states what the value already is, for the reader.
- `level: u8` changes what the literal `200` becomes: a `u8` rather than
  an `int`. A literal takes the type that its place expects.

Sisu checks that the value fits the annotation. A literal that is too
large for its type does not compile:

<!-- file: examples/02-values/too_big.sisu -->
```sisu
# A literal must fit the type it is given.
def main():
    let level: u8 = 300
    print("{level}")
```

<!-- output: examples/02-values/too_big.console -->
```console
$ sisu run too_big.sisu
error[E0374]: `300` does not fit in `u8`
  --> too_big.sisu:3:21
  |
3 |     let level: u8 = 300
  |                     ^^^
  = note: `u8` holds 0 to 255

error[D0006]: the program was not run: errors found (1)
```

E0374 (`type.literal-out-of-range`) says what the type holds, so the fix
is a smaller value or a wider type.

Inside a function, annotations are optional. The parameters and the
result of a function are always written, as chapter 3 shows, so its
first line tells you its types without reading its body.

## Converting numbers

Sisu converts a number to another type by itself only when nothing can
be lost. A `u8` fits in an `i32`, an `i32` fits in an `int`, and every
`f32` is also an `f64`, so these lines need no conversion:

<!-- file: examples/02-values/widening.sisu -->
```sisu
# A value widens to a larger type by itself when nothing is lost.
def main():
    let small: u8 = 200
    let medium: i32 = small
    let large: int = medium
    let single: f32 = 1.5
    let double: f64 = single
    print("{small} {medium} {large} {double}")
```

<!-- output: examples/02-values/widening.out -->
```text
200 200 200 1.5
```

The other way, a value could be lost: an `int` can hold 300, and a `u8`
cannot. Sisu does not convert by itself in that direction:

<!-- file: examples/02-values/narrowing.sisu -->
```sisu
# A smaller type could lose information, so Sisu does not convert.
def main():
    let large = 200
    let small: u8 = large
    print("{small}")
```

<!-- output: examples/02-values/narrowing.console -->
```console
$ sisu run narrowing.sisu
error[E0300]: expected `u8`, found `int`
  --> narrowing.sisu:4:21
  |
4 |     let small: u8 = large
  |                     ^^^^^
  |                -- expected because of this
  = note: an `int` narrows to `u8` through `.to_u8()`, which gives an option: `None` when the value does not fit

error[D0006]: the program was not run: errors found (1)
```

E0300 (`type.mismatch`) is the general diagnostic for a value of the
wrong type. It names the type expected and the type found, and its
label `--` marks what made Sisu expect that type, here the annotation.
Its note names the conversion, `.to_u8()`, which this section's last
example shows.

Integers and floats never mix in arithmetic, whatever their sizes:

<!-- file: examples/02-values/mixing.sisu -->
```sisu
# Arithmetic does not mix `int` and `f64`.
def main():
    let price = 2.5
    let count = 3
    print("total: {price * count}")
```

<!-- output: examples/02-values/mixing.console -->
```console
$ sisu run mixing.sisu
error[E0373]: `*` cannot mix `f64` and `int`; convert one of them explicitly
  --> mixing.sisu:5:20
  |
5 |     print("total: {price * count}")
  |                    ^^^^^^^^^^^^^
  = fix: make the `int` an `f64`: `.to_f64()`

error[D0006]: the program was not run: errors found (1)
```

The fix of E0373 (`type.mixed-numbers`) adds `.to_f64()` after `count`,
which turns the `int` into an `f64` and makes the line
`price * count.to_f64()`.

Every conversion that can lose information is a method that you call:

<!-- file: examples/02-values/conversions.sisu -->
```sisu
# Conversions that can lose information are method calls.
def main():
    let count = 300
    print("{count.to_f64() / 8.0}")
    print("{count.to_u16()} {count.to_u8()}")
    print("{count.truncate_to[u8]()}")
    let price = 2.75
    print("{price.round()} {price.round().truncate_to_int()}")
    print("{price.truncate_to_int()}")
```

<!-- output: examples/02-values/conversions.out -->
```text
37.5
Some(300) None
44
3.0 3
2
```

- `to_f64()` turns an `int` into the nearest `f64`, which is exact for
  every integer up to 2 to the power 53.
- `to_u16()` and `to_u8()` give an optional value: `Some` with the
  converted number when it fits, and `None` when it does not, as for 300
  in a `u8`. Chapter 8 shows how a program uses one.
- `truncate_to[u8]()` keeps what fits and drops the rest, the way `+%`
  wraps: 300 becomes 44, which is 300 minus 256. The type in square
  brackets is the type to convert to.
- `round()` rounds an `f64` to the nearest whole number, halves away
  from zero, and gives an `f64`. `truncate_to_int()` drops the fraction
  and gives an `int`. To get the nearest `int`, round first, then
  truncate. A method that does both, `round_to[int]()`, is designed in
  [the proposal's tour](../../proposal/01-tour.md#bindings-and-basic-types)
  and is not in the compiler yet.

> **From Python.** Python's `round(2.5)` is 2, because Python rounds a
> half to the even neighbour. Sisu's `2.5.round()` is 3.0: a half rounds
> away from zero.

## Reusing a name

Inside one block, a new `let` can reuse a name. The new binding replaces
the old one from its line on, and its type can differ:

<!-- file: examples/02-values/rebinding.sisu -->
```sisu
# A new `let` can reuse a name in the same block.
def main():
    let celsius = 21.6
    let celsius = celsius.round()
    let celsius = celsius.truncate_to_int()
    print("about {celsius} degrees")
```

<!-- output: examples/02-values/rebinding.out -->
```text
about 22 degrees
```

`celsius` is first an `f64`, then a rounded `f64`, then an `int`. Each
`let` makes a new binding rather than changing the old one, which is why
the old one does not need `var`. Reusing a name this way keeps a value
that goes through steps under one name, instead of `celsius_rounded` and
`celsius_whole`.

Reusing a name inside an inner block is different. The `if` block in
this program is an inner block, and its `let` gets a warning:

<!-- file: examples/02-values/shadowing.sisu -->
```sisu
# Reusing an outer name inside an inner block gets a warning.
def main():
    let total = 10
    let with_tip = True
    if with_tip:
        let total = total + 2
        print("inside the block: {total}")
    print("after the block: {total}")
```

<!-- output: examples/02-values/shadowing.console -->
```console
$ sisu run shadowing.sisu
warning[W0002]: `total` shadows a binding of an enclosing scope
  --> shadowing.sisu:6:13
  |
6 |         let total = total + 2
  |             ^^^^^

inside the block: 12
after the block: 10
```

W0002 (`resolve.shadow-outer`) is a warning, so the program runs. Inside
the block, `total` names the new binding, 12; after the block, the outer
binding is back, and it is still 10. The inner `let` hides the outer
binding without changing it, which is easy to misread as a change, so
Sisu warns. If you meant to change `total`, make it a `var` and assign to
it. If you meant a new value, give it a new name, such as
`total_with_tip`.

## Exercise 2.1: predict the values

Open
[`manual/exercises/02-values/predict_values.sisu`](../exercises/02-values/predict_values.sisu).
It is the first predict exercise: each hole is your prediction of a
value, and each test compares a prediction with the expression it
predicts.

<!-- file: exercises/02-values/predict_values.sisu -->
```sisu
##! Exercise 2.1, predict the values. Kind: predict. Level: 1.
##!
##! Each constant is a prediction of the value of the expression that
##! its test compares it with. Replace each hole with the value you
##! predict, written as a literal: a number, `True` or `False`. Decide
##! before you run anything, then run `sisu test predict_values.sisu`.
##! When a test fails, its report shows the value the expression really
##! has; work out why before you change your answer. The hints are in
##! chapter 2 of the tutorial, under "Exercise 2.1: predict the values".

const QUOTIENT: int = todo("(-17) // 5")
const REMAINDER: int = todo("(-17) % 5")
const POWER: int = todo("2 ** 3 ** 2")
const EXACT: bool = todo("0.1 + 0.2 == 0.3")
const CHAIN: bool = todo("1 < 5 <= 4")
const NEGATION: bool = todo("not 2 + 2 == 5")
const WRAPPED: u8 = todo("200u8 +% 100")

test "floor division of a negative number":
    assert (-17) // 5 == QUOTIENT

test "the remainder of a negative number":
    assert (-17) % 5 == REMAINDER

test "a power of a power":
    assert 2 ** 3 ** 2 == POWER

test "adding tenths":
    assert (0.1 + 0.2 == 0.3) == EXACT

test "a chain of comparisons":
    assert (1 < 5 <= 4) == CHAIN

test "what `not` applies to":
    assert (not 2 + 2 == 5) == NEGATION

test "wrapping a byte":
    assert 200u8 +% 100 == WRAPPED
```

Run the tests before you change anything. Each hole is reported with
its type, which is the kind of value to write, and every test is
blocked by its hole:

<!-- output: solutions/02-values/predict_values.console -->
```console
$ sisu test predict_values.sisu
warning[W0001]: typed hole `(-17) // 5` of type `int`
  --> predict_values.sisu:11:23
   |
11 | const QUOTIENT: int = todo("(-17) // 5")
   |                       ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `(-17) % 5` of type `int`
  --> predict_values.sisu:12:24
   |
12 | const REMAINDER: int = todo("(-17) % 5")
   |                        ^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `2 ** 3 ** 2` of type `int`
  --> predict_values.sisu:13:20
   |
13 | const POWER: int = todo("2 ** 3 ** 2")
   |                    ^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `0.1 + 0.2 == 0.3` of type `bool`
  --> predict_values.sisu:14:21
   |
14 | const EXACT: bool = todo("0.1 + 0.2 == 0.3")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `1 < 5 <= 4` of type `bool`
  --> predict_values.sisu:15:21
   |
15 | const CHAIN: bool = todo("1 < 5 <= 4")
   |                     ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `not 2 + 2 == 5` of type `bool`
  --> predict_values.sisu:16:24
   |
16 | const NEGATION: bool = todo("not 2 + 2 == 5")
   |                        ^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `200u8 +% 100` of type `u8`
  --> predict_values.sisu:17:21
   |
17 | const WRAPPED: u8 = todo("200u8 +% 100")
   |                     ^^^^^^^^^^^^^^^^^^^^

test "floor division of a negative number" ... blocked by todo("(-17) // 5")
test "the remainder of a negative number" ... blocked by todo("(-17) % 5")
test "a power of a power" ... blocked by todo("2 ** 3 ** 2")
test "adding tenths" ... blocked by todo("0.1 + 0.2 == 0.3")
test "a chain of comparisons" ... blocked by todo("1 < 5 <= 4")
test "what `not` applies to" ... blocked by todo("not 2 + 2 == 5")
test "wrapping a byte" ... blocked by todo("200u8 +% 100")
0 passed, 0 failed, 7 blocked
```

Replace each hole with a literal, such as `-5`, `0.5` or `True`. Write
all seven predictions before you run the tests again.

<details>
<summary>Hint 1</summary>

Work each expression out by hand in the order that Sisu does: which
operator applies first, and which values it applies to. The sections
"Arithmetic" and "Truth values and logic" say what each operator does.

</details>

<details>
<summary>Hint 2</summary>

Five rules decide the answers: `//` rounds towards minus infinity, `%`
takes the sign of the divisor, `**` groups from the right, a float can
only come close to most decimal fractions, and `+%` wraps at the width
of the type.

</details>

When every test passes, read the reasons in
[`manual/solutions/02-values/predict_values.sisu`](../solutions/02-values/predict_values.sisu).

## Exercise 2.2: split the bill

Open
[`manual/exercises/02-values/split_bill.sisu`](../exercises/02-values/split_bill.sisu).
It is a fix exercise: the program does not compile, and the diagnostics
guide you to a version that does and passes its tests.

<!-- file: exercises/02-values/split_bill.sisu -->
```sisu
##! Exercise 2.2, split the bill. Kind: fix. Level: 1.
##!
##! Aino, Eero and Roni share a restaurant bill, and they rate the meal.
##! The program does not compile. Run `sisu test split_bill.sisu`, read
##! each diagnostic and its fix, and change the two `return` lines until
##! every test passes. A fix that a diagnostic offers is not always the
##! one you want: check it against the tests. The hints are in chapter 2
##! of the tutorial, under "Exercise 2.2: split the bill".

## Each person's share of a bill of `total` cents, in whole cents. The
## cents left over go into the tip jar.
def share(total: int, people: int) -> int:
    return total / people

## The mean of ratings that add up to `total` over `count` ratings, with
## its fraction.
def mean_rating(total: int, count: int) -> f64:
    return total / count

test "a bill of 10 euros between three":
    assert share(1000, 3) == 333

test "a bill of 84 euros between four":
    assert share(8400, 4) == 2100

test "a mean with a fraction":
    assert mean_rating(9, 2) == 4.5

test "a whole mean":
    assert mean_rating(12, 3) == 4.0
```

<!-- output: solutions/02-values/split_bill.console -->
```console
$ sisu test split_bill.sisu
error[E0301]: `/` divides floats; for integers use `//`, or convert to `f64` first
  --> split_bill.sisu:13:12
   |
13 |     return total / people
   |            ^^^^^^^^^^^^^^
  = fix: use `//` for integer division: `//`

error[E0300]: expected `f64`, found `int`
  --> split_bill.sisu:18:12
   |
18 |     return total / count
   |            ^^^^^^^^^^^^^
  = fix: make the `int` an `f64`: `(`, `).to_f64()`

error[E0301]: `/` divides floats; for integers use `//`, or convert to `f64` first
  --> split_bill.sisu:18:12
   |
18 |     return total / count
   |            ^^^^^^^^^^^^^
  = fix: use `//` for integer division: `//`

error[D0006]: the program was not run: errors found (3)
```

`mean_rating` has two diagnostics on one line. Read both fixes before
you apply either: together they change the line to
`(total // count).to_f64()`.

<details>
<summary>Hint 1</summary>

The two functions divide for different reasons. Decide for each one
whether the answer is a whole number or a measurement with a fraction.

</details>

<details>
<summary>Hint 2</summary>

`share` wants the division in whole numbers, which is what E0301's fix
gives. In `mean_rating`, that fix throws the fraction away before
anything converts the result, so the test with 9 and 2 fails. Convert
before you divide.

</details>

When every test passes, compare your answer with
[`manual/solutions/02-values/split_bill.sisu`](../solutions/02-values/split_bill.sisu).

## Exercise 2.3: a weather report

Open
[`manual/exercises/02-values/weather_report.sisu`](../exercises/02-values/weather_report.sisu).
It is a complete exercise of level 2: the function is written except for
two holes, and filling them uses most of this chapter.

<!-- file: exercises/02-values/weather_report.sisu -->
```sisu
##! Exercise 2.3, a weather report. Kind: complete. Level: 2.
##!
##! A weather station in Oulu measures in degrees Celsius, and its
##! report also gives each temperature in degrees Fahrenheit, rounded to
##! a whole degree. To convert, multiply by 9, divide by 5 and add 32.
##!
##! Replace the two holes in `report` so that every test passes. Each
##! hole's type is written on its line. Run the tests with
##! `sisu test weather_report.sisu`. The hints are in chapter 2 of the
##! tutorial, under "Exercise 2.3: a weather report".

## A line of the report: `celsius`, and the same temperature in
## Fahrenheit, rounded to the nearest whole degree.
def report(celsius: f64) -> String:
    let fahrenheit: f64 = todo("celsius in Fahrenheit")
    let whole: int = todo("fahrenheit rounded, as an int")
    return "{celsius} °C is about {whole} °F"

test "a summer day":
    assert report(21.5) == "21.5 °C is about 71 °F"

test "a winter day":
    assert report(-12.0) == "-12.0 °C is about 10 °F"

test "water boils":
    assert report(100.0) == "100.0 °C is about 212 °F"

test "the two scales meet":
    assert report(-40.0) == "-40.0 °C is about -40 °F"
```

<!-- output: solutions/02-values/weather_report.console -->
```console
$ sisu test weather_report.sisu
warning[W0001]: typed hole `celsius in Fahrenheit` of type `f64`
  --> weather_report.sisu:15:27
   |
15 |     let fahrenheit: f64 = todo("celsius in Fahrenheit")
   |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `fahrenheit rounded, as an int` of type `int`
  --> weather_report.sisu:16:22
   |
16 |     let whole: int = todo("fahrenheit rounded, as an int")
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "a summer day" ... blocked by todo("celsius in Fahrenheit")
test "a winter day" ... blocked by todo("celsius in Fahrenheit")
test "water boils" ... blocked by todo("celsius in Fahrenheit")
test "the two scales meet" ... blocked by todo("celsius in Fahrenheit")
0 passed, 0 failed, 4 blocked
```

The holes are in `let` lines with type annotations, so each hole's type
is the annotation's.

<details>
<summary>Hint 1</summary>

The first hole is arithmetic on `celsius`, which is an `f64`. Every
number in that arithmetic must be an `f64` too.

</details>

<details>
<summary>Hint 2</summary>

The second hole takes two steps: round `fahrenheit` to the nearest whole
number, then turn the whole `f64` into an `int`. The section "Converting
numbers" has a method for each step, and the order of the steps matters.

</details>

When every test passes, compare your answer with
[`manual/solutions/02-values/weather_report.sisu`](../solutions/02-values/weather_report.sisu).

## What you learned

- The basic types are `int`, `f64`, `bool`, `char` and `str`. Sisu
  works out the type of a `let` from its value, and an annotation such
  as `let level: u8 = 200` states it.
- `//` and `%` divide integers, and `/` divides floats. An `int` that
  overflows stops the program with a panic, and `+%` and `+|` wrap and
  saturate when you ask for it.
- Comparisons give a `bool`, and a chain points one way. `and`, `or` and
  `not` take and give `bool` values, and nothing else is true or false.
- `var` makes a binding that can change, and `let` one that cannot.
  `const` names a value that the compiler computes.
- A value widens to a larger type by itself. Every conversion that can
  lose information is a method: `to_f64()`, `to_u8()`,
  `truncate_to[u8]()`, `round()` and `truncate_to_int()`.
- A new `let` in the same block reuses a name. Reusing an outer name in
  an inner block gets a warning.

Chapter 3 is about functions: parameters and results, default and
keyword arguments, `return`, and the tests and documentation that go
with a function.
