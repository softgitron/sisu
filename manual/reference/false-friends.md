# False friends

Where Sisu spells something as Python does, or as Rust does for syntax
the two share, it means the same or fails to compile (D-062). A false
friend is a spelling whose familiar meaning Sisu does not have; this page
lists them, each with the check that makes it loud, and the few where
Sisu's meaning differs without one. People and models read a familiar
spelling with its familiar meaning first, and Python 3's silent change
to `/` is the cautionary tale.

**See also:** the tutorial's "From Python" notes, in
[chapter 1](../tutorial/01-hello.md) onwards; the proposal's
[02, "Departures from Python, with reasons"](../../proposal/02-syntax.md),
[13](../../proposal/13-evolution-and-governance.md), which keeps the table
in the specification, and the decision D-062; [R17,
Diagnostics](diagnostics.md) for the codes.

## How to read this page

Each row shows a program written with the habit and what `sisu check`
says about it, or, for a spelling that compiles, what the program
prints. The fix is the Sisu spelling the row names. Every example ran
when the manual was last checked, with stage 0, the compiler in this
repository.

## The rows of D-062's table

These are the rows that the experiment's model arm measures
([20](../../proposal/20-pre-registration.md), section 6.9), each with
what Sisu does today.

### FF-1: `xs[-1]` for the last element

In Python, a negative index counts from the end. In Sisu, `xs[^1]` is
the last element, a constant negative index does not compile (E0344,
whose fix writes `^1`), and an index computed at run time that is
negative panics (R0015). The check is static wherever it can be.

<!-- file: examples/reference-false-friends/negative_index.sisu -->
```sisu
# Python's last element.
def main():
    let names = ["Aino", "Eero"]
    print(names[-1])
```

<!-- output: examples/reference-false-friends/negative_index.console -->
```console
$ sisu check negative_index.sisu
error[E0344]: a constant negative index does not compile; count from the end with `^`: `names[^1]`
  --> negative_index.sisu:4:17
  |
4 |     print(names[-1])
  |                 ^^
  = fix: count from the end with `^`: `^1`

```

### FF-2: the order of a `dict` or a `set`

A `dict` iterates in the order its keys were inserted, as Python's does
since 3.7, so code that relies on the order means the same (Q-42,
D-074). The row is expected to be empty, and the experiment measures it.

<!-- file: examples/reference-false-friends/dict_order.sisu -->
```sisu
# A `dict` iterates in the order its keys were inserted.
def main():
    var ages = {"Roni": 9}
    ages["Aino"] = 7
    ages["Eero"] = 8
    for name, age in ages.items():
        print("{name} {age}")
```

<!-- output: examples/reference-false-friends/dict_order.out -->
```text
Roni 9
Aino 7
Eero 8
```

### FF-3: `7 / 2` on integers

Python 3 gives 3.5 and Rust gives 3. Sisu gives neither: `/` divides
floats, and on integers it is E0301, whose fix writes `//`.

<!-- file: examples/reference-false-friends/int_division.sisu -->
```sisu
# `/` between two integers.
def main():
    print(7 / 2)
```

<!-- output: examples/reference-false-friends/int_division.console -->
```console
$ sisu check int_division.sisu
error[E0301]: `/` divides floats; for integers use `//`, or convert to `f64` first
  --> int_division.sisu:3:11
  |
3 |     print(7 / 2)
  |           ^^^^^
  = fix: use `//` for integer division: `//`

```

### FF-4: `if xs:` for a test of emptiness

Sisu has no truthiness (D-025): a condition is a `bool`, so a list, a
text or a number as a condition is E0302, whose note names the explicit
test.

<!-- file: examples/reference-false-friends/truthiness.sisu -->
```sisu
# A list used as a condition.
def main():
    let names = ["Aino"]
    if names:
        print("someone is here")
```

<!-- output: examples/reference-false-friends/truthiness.console -->
```console
$ sisu check truthiness.sisu
error[E0302]: a condition must be a `bool`, not `list[str]`; Sisu has no truthiness
  --> truthiness.sisu:4:8
  |
4 |     if names:
  |        ^^^^^
  = note: write the test explicitly, as in `not xs.is_empty()` or `x != 0`

```

### FF-5: `a if c else b`

Sisu's conditional expression is `if c: a else: b`. Python's order is
E0115 with the fix in a `let`, as [R17](diagnostics.md#e0115) shows;
inside a call's parentheses it is E0100.

<!-- file: examples/reference-false-friends/conditional.sisu -->
```sisu
# Python's conditional expression.
def main():
    let n = 3
    print("big" if n > 2 else "small")
```

<!-- output: examples/reference-false-friends/conditional.console -->
```console
$ sisu check conditional.sisu
error[E0100]: expected `)`, found `if`
  --> conditional.sisu:4:17
  |
4 |     print("big" if n > 2 else "small")
  |                 ^^

```

### FF-6: `-7 % 3` read as Rust's -1

`%` takes the sign of the divisor and `//` rounds towards minus
infinity, as in Python. Rust's `%` takes the sign of the dividend, so
code written with Rust's meaning is silently different here: this is the
one row with no check, and the corpus translated from Rust flags it.

<!-- file: examples/reference-false-friends/modulo.sisu -->
```sisu
# `%` and `//` round towards minus infinity, as in Python.
def main():
    print("{-7 % 3} {-7 // 3} {7 % -3}")
```

<!-- output: examples/reference-false-friends/modulo.out -->
```text
2 -3 -2
```

## Python habits that do not compile

Each of these spellings means something in Python and is an error in
Sisu, so a Python habit cannot pass unnoticed. The table says whether
the diagnostic names the Sisu way; where it does not, the section says
what to write, and the finding is in the plan's list for the compiler's
maintainers.

| Habit | Sisu | Code | Names the Sisu way |
|---|---|---|---|
| `xs[-1]` | `xs[^1]` | E0344 | yes, as a fix |
| `7 / 2` | `7 // 2` | E0301 | yes, as a fix |
| `if xs:` | `if not xs.is_empty():` | E0302 | yes, in a note |
| `a if c else b` | `if c: a else: b` | E0115, E0100 | in a `let` only |
| `count = 0` to declare | `var count = 0` | E0200 | no |
| `print(a, b)` | `print("{a} {b}")` | E0305 | no |
| `'text'` | `"text"` | E0016 | no |
| `for c in word` | `for c in word.chars()` | E0325 | no |
| `word[0]` | `word.chars()`, `word.byte_slice(a, b)` | E0345 | yes, in the message |
| `x or default` | `x ?? default` | E0302 | no |
| `", ".join(xs)` | `xs.join(", ")` | E0304 | in a note |
| `(n := f())` | `let n = f()`, `if let` | E0123, E0100 | outside brackets only |
| `for ... else:` | `if let` on a search's result | E0103 | no |
| `a < b > c` | one direction | E0113 | yes, in the message |
| `x is None` | `x == None`, `if let` | E0100 | no |
| `lambda x: e` | `x -> e` | E0117 | no |
| `f"{x}"` | `"{x}"` | E0100 | no |
| `class`, `__init__` | `struct` | E0107, E0215 | no |
| `try:` `except:` | `T ! E`, `?`, `else` | E0117 | no |
| `int(s)`, `str(n)` | `s.parse_int()`, `"{n}"` | E0200 | no; offers `uint` |
| `append`, `strip`, `upper` | `push`, `trim`, `to_uppercase` | E0304 | no |
| `d.get(k, default)` | `d.get(k) ?? default` | E0305 | no |
| `enumerate(xs, start=1)` | `enumerate(xs)` and `i + 1` | E0306 | no |
| `max(xs)` | `max(a, b)`, `sorted(xs)[^1]` | E0308 | no |

### Assignment declares a name

A binding is declared with `let` or `var`; an assignment to a name that
nothing declared is E0200 at each use.

<!-- file: examples/reference-false-friends/assignment.sisu -->
```sisu
# An assignment that should declare a name.
def main():
    count = 0
    print(count)
```

<!-- output: examples/reference-false-friends/assignment.console -->
```console
$ sisu check assignment.sisu
error[E0200]: cannot find `count` in this scope
  --> assignment.sisu:3:5
  |
3 |     count = 0
  |     ^^^^^

error[E0200]: cannot find `count` in this scope
  --> assignment.sisu:4:11
  |
4 |     print(count)
  |           ^^^^^

```

### `print(a, b)`

`print` takes one text; build it with interpolation, `print("Hei,
{name}")`. Several arguments are E0305.

<!-- file: examples/reference-false-friends/print_many.sisu -->
```sisu
# `print` with several arguments.
def main():
    let name = "Aino"
    print("Hei,", name)
```

<!-- output: examples/reference-false-friends/print_many.console -->
```console
$ sisu check print_many.sisu
error[E0305]: wrong number of arguments to `print`: expected 1, found 2
  --> print_many.sisu:4:5
  |
4 |     print("Hei,", name)
  |     ^^^^^^^^^^^^^^^^^^^

```

### `'text'` in single quotes

Single quotes make a `char`; text is between double quotes. E0016.

<!-- file: examples/reference-false-friends/single_quotes.sisu -->
```sisu
# Text between single quotes.
def main():
    print('Hei')
```

<!-- output: examples/reference-false-friends/single_quotes.console -->
```console
$ sisu check single_quotes.sisu
error[E0016]: a character literal holds exactly one character
  --> single_quotes.sisu:3:11
  |
3 |     print('Hei')
  |           ^^^^^

```

### `for letter in word`

A text is not iterated directly, since it can be read as characters,
bytes or lines: `for letter in word.chars()`. E0325.

<!-- file: examples/reference-false-friends/text_loop.sisu -->
```sisu
# A loop over a text.
def main():
    for letter in "hei":
        print(letter)
```

<!-- output: examples/reference-false-friends/text_loop.console -->
```console
$ sisu check text_loop.sisu
error[E0325]: a `for` loop cannot iterate over `str`
  --> text_loop.sisu:3:19
  |
3 |     for letter in "hei":
  |                   ^^^^^

```

### `word[0]`

A `str` has no integer index (D-034); E0345 names `.chars()`, `.bytes()`
and `.byte_slice(a, b)`.

<!-- file: examples/reference-false-friends/text_index.sisu -->
```sisu
# A text indexed by position.
def main():
    let word = "hei"
    print(word[0])
```

<!-- output: examples/reference-false-friends/text_index.console -->
```console
$ sisu check text_index.sisu
error[E0345]: `str` cannot be indexed by position; use `.chars()`, `.bytes()` or `.byte_slice(a, b)`
  --> text_index.sisu:4:11
  |
4 |     print(word[0])
  |           ^^^^^^^

```

### `x or default`

`or` takes two `bool`s. A default for an absent value is `x ?? default`,
and an empty text is tested with `.is_empty()`. E0302.

<!-- file: examples/reference-false-friends/or_default.sisu -->
```sisu
# `or` to give a default.
def main():
    let name = ""
    print(name or "anonymous")
```

<!-- output: examples/reference-false-friends/or_default.console -->
```console
$ sisu check or_default.sisu
error[E0302]: a condition must be a `bool`, not `str`; Sisu has no truthiness
  --> or_default.sisu:4:11
  |
4 |     print(name or "anonymous")
  |           ^^^^
  = note: write the test explicitly, as in `not xs.is_empty()` or `x != 0`

error[E0302]: a condition must be a `bool`, not `str`; Sisu has no truthiness
  --> or_default.sisu:4:19
  |
4 |     print(name or "anonymous")
  |                   ^^^^^^^^^^^
  = note: write the test explicitly, as in `not xs.is_empty()` or `x != 0`

```

### `", ".join(names)`

`join` is a method of an iterator: `names.join(", ")`. E0304, with a
note.

<!-- file: examples/reference-false-friends/join.sisu -->
```sisu
# Python's `str.join`.
def main():
    let names = ["Aino", "Eero"]
    print(", ".join(names))
```

<!-- output: examples/reference-false-friends/join.console -->
```console
$ sisu check join.sisu
error[E0304]: `str` has no method `join`
  --> join.sisu:4:16
  |
4 |     print(", ".join(names))
  |                ^^^^
  = note: `join` is a method of `Iterator`

```

### `:=`

Sisu has no assignment expression: bind with `let`, or use `if let` or a
chained condition. E0123 where a statement starts with it; E0100 inside
brackets, as here.

<!-- file: examples/reference-false-friends/walrus.sisu -->
```sisu
# Python's assignment expression.
def main():
    let names = ["Aino", "Eero", "Roni"]
    if (n := len(names)) > 2:
        print(n)
```

<!-- output: examples/reference-false-friends/walrus.console -->
```console
$ sisu check walrus.sisu
error[E0100]: expected `)`, found `:=`
  --> walrus.sisu:4:11
  |
4 |     if (n := len(names)) > 2:
  |           ^^

```

### `else` on a loop

A loop has no `else` (D-021); search with a function that returns `T?`
and test the result with `if let`. E0103.

<!-- file: examples/reference-false-friends/loop_else.sisu -->
```sisu
# Python's `else` on a loop.
def main():
    for name in ["Aino", "Eero"]:
        if name == "Roni":
            break
    else:
        print("no Roni")
```

<!-- output: examples/reference-false-friends/loop_else.console -->
```console
$ sisu check loop_else.sisu
error[E0103]: expected an expression, found `else`
  --> loop_else.sisu:6:5
  |
6 |     else:
  |     ^^^^

```

### `a < b > c`

A chain of comparisons points one way. E0113.

<!-- file: examples/reference-false-friends/chain.sisu -->
```sisu
# A comparison chain that points two ways.
def main():
    let n = 3
    print(0 < n > 2)
```

<!-- output: examples/reference-false-friends/chain.console -->
```console
$ sisu check chain.sisu
error[E0113]: a chained comparison must point one way; `<` and `>` do not
  --> chain.sisu:4:17
  |
4 |     print(0 < n > 2)
  |                 ^

```

### `x is None`

Sisu has no `is`; compare with `==`, or match the absent value with `if
let` or `??`. E0100.

<!-- file: examples/reference-false-friends/is_none.sisu -->
```sisu
# Python's `is None`.
def main():
    let found: int? = None
    if found is None:
        print("nothing")
```

<!-- output: examples/reference-false-friends/is_none.console -->
```console
$ sisu check is_none.sisu
error[E0100]: expected `:`, found `is`
  --> is_none.sisu:4:14
  |
4 |     if found is None:
  |              ^^

```

### `lambda x: e`

A closure is `x -> e`, or `it` for one parameter, or a `do` block
(D-026). E0117.

<!-- file: examples/reference-false-friends/lambda.sisu -->
```sisu
# Python's `lambda`.
def main():
    let double = lambda n: n * 2
    print(double(4))
```

<!-- output: examples/reference-false-friends/lambda.console -->
```console
$ sisu check lambda.sisu
error[E0117]: unexpected `n` after the end of the statement
  --> lambda.sisu:3:25
  |
3 |     let double = lambda n: n * 2
  |                         ^

```

### `f"..."`

Every string literal interpolates, so there is no prefix
([R1](lexical-structure.md#string-literals)). E0100.

<!-- file: examples/reference-false-friends/f_string.sisu -->
```sisu
# Python's f-string prefix.
def main():
    let n = 3
    print(f"n is {n}")
```

<!-- output: examples/reference-false-friends/f_string.console -->
```console
$ sisu check f_string.sisu
error[E0100]: expected `)`, found a string
  --> f_string.sisu:4:12
  |
4 |     print(f"n is {n}")
  |            ^

```

### `class` and `__init__`

A type with fields is a `struct`, built by calling its name with its
fields; there is no inheritance (D-012). `class` is E0107 and `self`
outside a method E0215.

<!-- file: examples/reference-false-friends/class.sisu -->
```sisu
# Python's `class` and `__init__`.
class Point:
    def __init__(self, x: int):
        self.x = x
```

<!-- output: examples/reference-false-friends/class.console -->
```console
$ sisu check class.sisu
error[E0107]: statements cannot appear at the top level of a module; put them in a function such as `main`
  --> class.sisu:2:1
  |
2 | class Point:
  | ^^^^^

error[E0215]: `self` exists only inside a method
  --> class.sisu:3:18
  |
3 |     def __init__(self, x: int):
  |                  ^^^^

error[E0389]: `self` exists only inside a method
  --> class.sisu:4:9
  |
4 |         self.x = x
  |         ^^^^

```

### `try` and `except`

An error is a value of type `T ! E`, handled with `?`, `else` or `match`
(D-011). E0117.

<!-- file: examples/reference-false-friends/try_except.sisu -->
```sisu
# Python's `try` and `except`.
def main():
    try:
        print("42".parse_int())
    except:
        print("not a number")
```

<!-- output: examples/reference-false-friends/try_except.console -->
```console
$ sisu check try_except.sisu
error[E0117]: unexpected `:` after the end of the statement
  --> try_except.sisu:3:8
  |
3 |     try:
  |        ^

error[E0117]: unexpected `:` after the end of the statement
  --> try_except.sisu:5:11
  |
5 |     except:
  |           ^

```

### `int(text)` and `str(n)`

Text becomes a number with `text.parse_int()`, which says when it
cannot, and a value becomes text by interpolation. E0200, whose fix for
`int` offers `uint`, which is not what you want.

<!-- file: examples/reference-false-friends/conversions.sisu -->
```sisu
# Python's `int(...)` and `str(...)`.
def main():
    let n = int("42")
    print(str(n))
```

<!-- output: examples/reference-false-friends/conversions.console -->
```console
$ sisu check conversions.sisu
error[E0200]: cannot find `int` in this scope
  --> conversions.sisu:3:13
  |
3 |     let n = int("42")
  |             ^^^
  = fix: a similar name is in scope: `uint`

error[E0200]: cannot find `str` in this scope
  --> conversions.sisu:4:11
  |
4 |     print(str(n))
  |           ^^^

```

### Python's method names

Sisu's `list` has `push`, and its `str` has `trim` and `to_uppercase`.
E0304 names no similar method.

<!-- file: examples/reference-false-friends/methods.sisu -->
```sisu
# Python's names for three methods.
def main():
    var names = ["aino"]
    names.append("eero")
    print("  hei  ".strip())
    print("hei".upper())
```

<!-- output: examples/reference-false-friends/methods.console -->
```console
$ sisu check methods.sisu
error[E0304]: `list[str]` has no method `append`
  --> methods.sisu:4:11
  |
4 |     names.append("eero")
  |           ^^^^^^

error[E0304]: `str` has no method `strip`
  --> methods.sisu:5:21
  |
5 |     print("  hei  ".strip())
  |                     ^^^^^

error[E0304]: `str` has no method `upper`
  --> methods.sisu:6:17
  |
6 |     print("hei".upper())
  |                 ^^^^^

```

### `d.get(key, default)`

`get` takes the key only and gives `V?`: `ages.get("Eero") ?? 0`. E0305.

<!-- file: examples/reference-false-friends/dict_get.sisu -->
```sisu
# Python's `dict.get` with a default.
def main():
    let ages = {"Aino": 7}
    print(ages.get("Eero", 0))
```

<!-- output: examples/reference-false-friends/dict_get.console -->
```console
$ sisu check dict_get.sisu
error[E0305]: wrong number of arguments to `get`: expected 1, found 2
  --> dict_get.sisu:4:11
  |
4 |     print(ages.get("Eero", 0))
  |           ^^^^^^^^^^^^^^^^^^^

```

### `enumerate(xs, start=1)`

`enumerate` counts from 0; add 1 where the number is used. E0306.

<!-- file: examples/reference-false-friends/enumerate_start.sisu -->
```sisu
# Python's `enumerate(..., start=1)`.
def main():
    for i, name in enumerate(["Aino", "Eero"], start=1):
        print("{i}. {name}")
```

<!-- output: examples/reference-false-friends/enumerate_start.console -->
```console
$ sisu check enumerate_start.sisu
error[E0306]: `enumerate` has no parameter named `start`
  --> enumerate_start.sisu:3:48
  |
3 |     for i, name in enumerate(["Aino", "Eero"], start=1):
  |                                                ^^^^^

```

### `max(xs)` over a list

Stage 0's `max` and `min` compare two values, and E0308's fix adds a
hole for the second. `sorted(xs)[^1]`, or a loop, gives the largest.
Python's form over a collection is what the prelude list of
[02](../../proposal/02-syntax.md) suggests, and stage 0 does not have
it.

<!-- file: examples/reference-false-friends/max_list.sisu -->
```sisu
# Python's `max` over a list.
def main():
    let scores = [3, 9, 4]
    print(max(scores))
```

<!-- output: examples/reference-false-friends/max_list.console -->
```console
$ sisu check max_list.sisu
error[E0308]: missing the argument `b` of `max`
  --> max_list.sisu:4:11
  |
4 |     print(max(scores))
  |           ^^^^^^^^^^^
  = fix: pass `b` as a hole to fill: `, todo("b")`

```

## Python habits whose meaning Sisu changes

A few Python spellings compile in Sisu and mean something else. Each
difference removes a trap that Python documents, and none hides a bug
that Python would show.

### Default arguments are computed at each call

Python computes a default once, when the function is defined, so a
mutable default is shared between calls. Sisu computes it at each call,
so each call that leaves it out gets a new empty list.

<!-- file: examples/reference-false-friends/defaults.sisu -->
```sisu
# A default value is computed at each call.
def add_name(name: str, own names: list[String] = []) -> list[String]:
    var all = names
    all.push(name.to_string())
    return all

def main():
    print(add_name("Aino"))
    print(add_name("Eero"))
```

<!-- output: examples/reference-false-friends/defaults.out -->
```text
["Aino"]
["Eero"]
```

### Each loop iteration binds its own variable

A closure made in a Python loop sees the loop variable's last value. In
Sisu each iteration binds its own `i`, and an `own` closure takes it, so
the three closures give 0, 1 and 2.

<!-- file: examples/reference-false-friends/closures.sisu -->
```sisu
# Each iteration binds its own `i`, which an `own` closure captures.
def main():
    var steps: list[() -> int] = []
    for i in range(3):
        steps.push(own () -> i)
    print([step() for step in steps])
```

<!-- output: examples/reference-false-friends/closures.out -->
```text
[0, 1, 2]
```

### A closure cannot outlive a borrowed loop variable

Without `own`, the closure borrows `i`, which ends with its iteration,
so keeping the closure is E0406: the late-binding trap is a compile
error rather than a wrong value.

<!-- file: examples/reference-false-friends/closures_borrowed.sisu -->
```sisu
# A closure that borrows the loop's `i` cannot outlive the iteration.
def main():
    var steps: list[() -> int] = []
    for i in range(3):
        steps.push(() -> i)
    print(len(steps))
```

<!-- output: examples/reference-false-friends/closures_borrowed.console -->
```console
$ sisu check closures_borrowed.sisu
error[E0406]: `i` does not live long enough: it is borrowed on line 5, and the borrow is still used on line 5, after `i` is gone
  --> closures_borrowed.sisu:5:28
  |
5 |         steps.push(() -> i)
  |                            ^
  |                    ------- borrowed here by `steps`
  |         ----- the borrow is still used here
  = note: declare `i` where it lives as long as the borrow is used, outside the block or loop it is in now

```

### An `int` has 64 bits

Python's integers grow without limit. Sisu's `int` has 64 bits, and
arithmetic past them panics (D-009), so the difference is loud, at run
time ([R17](diagnostics.md#r0003)). `BigInt` grows without limit when
that is what you want.

<!-- file: examples/reference-false-friends/overflow.sisu -->
```sisu
# An `int` has 64 bits, and an operation past them panics.
def main():
    let big = 4_611_686_018_427_387_904
    print(big + big)
```

<!-- output: examples/reference-false-friends/overflow.console -->
```console
$ sisu run overflow.sisu
panic: integer overflow: 4611686018427387904 + 4611686018427387904
  at main (overflow.sisu:4:11)
```

## Rust habits

Sisu shares much of its meaning with Rust: `T?` and `Some`, `?` that
returns early, `match` that covers every case, ownership and borrowing,
traits. These Rust spellings are not Sisu, and each is an error.

| Habit | Sisu | Code |
|---|---|---|
| `let mut x` | `var x` | E0105 |
| `a && b`, `a \|\| b`, `!a` | `a and b`, `a or b`, `not a` | E0103 |
| `x as f64` | `x.to_f64()` | E0100 |
| `// note` | `# note` | E0117 |
| `x.clone()` | `x.copy()` | E0304 |
| `1 => "one"` | `case 1: "one"` | E0100 |
| `f(&xs)` | `f(xs)` | E0103 |
| `0..3` | `range(3)` | E0100 |
| `Vec<T>` | `list[T]` | E0117 |
| `println!(...)` | `print(...)` | E0117 |
| `7 / 2` is 3 | `7 // 2` | E0301 |
| `-7 % 3` is -1 | 2, as in Python | none: FF-6 |

### `let mut`

A binding that changes is declared with `var`. E0105.

<!-- file: examples/reference-false-friends/let_mut.sisu -->
```sisu
# Rust's `let mut`.
def main():
    let mut total = 1
    total += 1
    print(total)
```

<!-- output: examples/reference-false-friends/let_mut.console -->
```console
$ sisu check let_mut.sisu
error[E0105]: expected a pattern, found `mut`
  --> let_mut.sisu:3:9
  |
3 |     let mut total = 1
  |         ^^^

```

### `&&`, `||` and `!`

The logical operators are the words `and`, `or` and `not` (D-025); `!`
belongs to error sets. E0103.

<!-- file: examples/reference-false-friends/symbols.sisu -->
```sisu
# Rust's `&&` for `and`.
def main():
    let warm = True
    let dry = False
    print(warm && dry)
```

<!-- output: examples/reference-false-friends/symbols.console -->
```console
$ sisu check symbols.sisu
error[E0103]: expected an expression, found `&`
  --> symbols.sisu:5:17
  |
5 |     print(warm && dry)
  |                 ^

```

### `x as f64`

A conversion is a method, `n.to_f64()`, and a lossy one says how:
`round()`, `truncate_to_int()`. E0100.

<!-- file: examples/reference-false-friends/as_cast.sisu -->
```sisu
# Rust's `as` conversion.
def main():
    let n = 3
    print(n as f64)
```

<!-- output: examples/reference-false-friends/as_cast.console -->
```console
$ sisu check as_cast.sisu
error[E0100]: expected `)`, found `as`
  --> as_cast.sisu:4:13
  |
4 |     print(n as f64)
  |             ^^

```

### `//` for a comment

A comment starts with `#`; `//` is integer division, as in Python. Here
E0117 reports the comment's words.

<!-- file: examples/reference-false-friends/slash_comment.sisu -->
```sisu
# Rust's `//` comment.
def main():
    let n = 3 // the count
    print(n)
```

<!-- output: examples/reference-false-friends/slash_comment.console -->
```console
$ sisu check slash_comment.sisu
error[E0117]: unexpected `count` after the end of the statement
  --> slash_comment.sisu:3:22
  |
3 |     let n = 3 // the count
  |                      ^^^^^

```

### `.clone()`

A copy is `.copy()`. E0304 names no similar method.

<!-- file: examples/reference-false-friends/clone.sisu -->
```sisu
# Rust's `.clone()`.
def main():
    let names = ["Aino"]
    let copied = names.clone()
    print(copied)
```

<!-- output: examples/reference-false-friends/clone.console -->
```console
$ sisu check clone.sisu
error[E0304]: `list[str]` has no method `clone`
  --> clone.sisu:4:24
  |
4 |     let copied = names.clone()
  |                        ^^^^^

```

### `=>` in a `match` arm

A `case` arm is `case PATTERN: value`. E0100.

<!-- file: examples/reference-false-friends/match_arrow.sisu -->
```sisu
# Rust's `=>` in a match arm.
def main():
    let n = 1
    let word = match n:
        case 1 => "one"
        case _ => "many"
    print(word)
```

<!-- output: examples/reference-false-friends/match_arrow.console -->
```console
$ sisu check match_arrow.sisu
error[E0100]: expected `:`, found `=`
  --> match_arrow.sisu:5:16
  |
5 |         case 1 => "one"
  |                ^

error[E0100]: expected `:`, found `=`
  --> match_arrow.sisu:6:16
  |
6 |         case _ => "many"
  |                ^

```

### `&x` to borrow

A plain parameter already borrows; an argument is written as it is, and
`ref` and `mut` say the rest ([R7](../README.md#the-reference), when it
is written). E0103.

<!-- file: examples/reference-false-friends/borrow_sign.sisu -->
```sisu
# Rust's `&` to borrow.
def total(prices: list[int]) -> int:
    return sum(prices)

def main():
    let prices = [3, 4]
    print(total(&prices))
```

<!-- output: examples/reference-false-friends/borrow_sign.console -->
```console
$ sisu check borrow_sign.sisu
error[E0103]: expected an expression, found `&`
  --> borrow_sign.sisu:7:17
  |
7 |     print(total(&prices))
  |                 ^

```

### `0..3` for a range

A range is `range(3)` or `range(a, b)`; `..` is the rest of a pattern.
E0100.

<!-- file: examples/reference-false-friends/dot_range.sisu -->
```sisu
# Rust's range `0..3`.
def main():
    for i in 0..3:
        print(i)
```

<!-- output: examples/reference-false-friends/dot_range.console -->
```console
$ sisu check dot_range.sisu
error[E0100]: expected `:`, found `..`
  --> dot_range.sisu:3:15
  |
3 |     for i in 0..3:
  |               ^^

```

### `Vec<T>`

Type arguments go in square brackets, `list[str]`. E0117.

<!-- file: examples/reference-false-friends/angle_generics.sisu -->
```sisu
# Rust's angle brackets for type arguments.
def main():
    let names: Vec<str> = ["Aino"]
    print(names)
```

<!-- output: examples/reference-false-friends/angle_generics.console -->
```console
$ sisu check angle_generics.sisu
error[E0117]: unexpected `<` after the end of the statement
  --> angle_generics.sisu:3:19
  |
3 |     let names: Vec<str> = ["Aino"]
  |                   ^

```

### `println!`

Sisu has no macros; `print("{x}")` prints. E0117.

<!-- file: examples/reference-false-friends/println.sisu -->
```sisu
# Rust's `println!`.
def main():
    println!("{}", 3)
```

<!-- output: examples/reference-false-friends/println.console -->
```console
$ sisu check println.sisu
error[E0117]: unexpected `!` after the end of the statement
  --> println.sisu:3:12
  |
3 |     println!("{}", 3)
  |            ^

```

## Status

The rule that a shared spelling means the same or fails to compile leans
(D-062), and so does the table it keeps; rows added after 1.0 go through
editions ([13](../../proposal/13-evolution-and-governance.md)). The rows
of D-062 are in stage 0 as their examples show, FF-6 being silent by
design. The diagnostics that do not name the Sisu way are findings for
the compiler, listed in the plan; a diagnostic that names it is part of
the row's check.

