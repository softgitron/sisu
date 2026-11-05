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
E0115, whose fix writes Sisu's order, wherever it stands: in a `let`,
as [R17](diagnostics.md#e0115) shows, and inside a call's parentheses,
as here.

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
error[E0115]: `a if c else b` is not Sisu; write `if c: a else: b`
  --> conditional.sisu:4:11
  |
4 |     print("big" if n > 2 else "small")
  |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: write it as `if c: a else: b`: `if n > 2: "big" else: "small"`

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
| `xs[-1]` | `xs[^1]` | [E0344](diagnostics.md#e0344) | yes, as a fix |
| `7 / 2` | `7 // 2` | [E0301](diagnostics.md#e0301) | yes, as a fix |
| `if xs:` | `if not xs.is_empty():` | [E0302](diagnostics.md#e0302) | yes, in a note |
| `a if c else b` | `if c: a else: b` | [E0115](diagnostics.md#e0115) | yes, as a fix |
| `count = 0` to declare | `var count = 0` | [E0200](diagnostics.md#e0200) | `let`, as a fix |
| `print(a, b)` | `print("{a} {b}")` | [E0305](diagnostics.md#e0305) | yes, as a fix |
| `'text'` | `"text"` | [E0016](diagnostics.md#e0016) | yes, as a fix |
| `for c in word` | `for c in word.chars()` | [E0325](diagnostics.md#e0325) | yes, as a fix |
| `word[0]` | `word.chars()`, `word.byte_slice(a, b)` | [E0345](diagnostics.md#e0345) | yes, in the message |
| `x or default` | `x ?? default` | [E0302](diagnostics.md#e0302) | no |
| `", ".join(xs)` | `xs.join(", ")` | [E0304](diagnostics.md#e0304) | yes, as a fix |
| `(n := f())` | `let n = f()`, `if let` | [E0123](diagnostics.md#e0123) | yes, in the message |
| `for ... else:` | `if let` on a search's result | [E0127](diagnostics.md#e0127) | yes, in the message |
| `a < b > c` | one direction | [E0113](diagnostics.md#e0113) | yes, in the message |
| `x is None` | `x.is_none()`, `if let` | [E0100](diagnostics.md#e0100) | yes, as a fix |
| `lambda x: e` | `x -> e` | [E0117](diagnostics.md#e0117) | yes, as a fix |
| `f"{x}"` | `"{x}"` | [E0100](diagnostics.md#e0100), [E0117](diagnostics.md#e0117) | in a call only |
| `class`, `__init__` | `struct` | [E0107](diagnostics.md#e0107), [E0215](diagnostics.md#e0215) | `struct`, as a fix |
| `try:` `except:` | `T ! E`, `?`, `else` | [E0117](diagnostics.md#e0117) | yes, in a note |
| `int(s)`, `str(n)` | `s.parse_int()`, `"{n}"` | [E0200](diagnostics.md#e0200) | yes, in a note |
| `append`, `strip`, `upper` | `push`, `trim`, `to_uppercase` | [E0304](diagnostics.md#e0304) | yes, as a fix |
| `d.get(k, default)` | `d.get(k) ?? default` | [E0305](diagnostics.md#e0305) | yes, as a fix |
| `enumerate(xs, start=1)` | `enumerate(xs)` and `i + 1` | [E0306](diagnostics.md#e0306) | yes, in a note |
| `def f(*args)` | a `Slice[T]` parameter | [E0900](diagnostics.md#e0900) | yes, in the message |
| `first, *rest = xs` | `case [first, *rest]:` | [E0900](diagnostics.md#e0900) | yes, in the message |

### Assignment declares a name

A binding is declared with `let` or `var`; an assignment to a name that
nothing declared is E0200, whose fix declares it with `let` (write
`var` instead when the name changes later).

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
  = fix: declare it with `let`: `let `

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
  = fix: write the value into the text: `"Hei, {name}"`

```

### `'text'` in single quotes

Single quotes make a `char`; text is between double quotes. E0016,
whose fix writes them.

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
  = fix: text is written in double quotes: `"`, `"`

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
  = fix: iterate over its characters: `.chars()`
  = fix: iterate over its bytes: `.bytes()`

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

`join` is a method of an iterator: `names.join(", ")`. E0304's note
says so, and its fix writes the call.

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
  = fix: join the items with their own method: `names.join(", ")`

```

### `:=`

Sisu has no assignment expression: bind with `let`, or use `if let` or a
chained condition. E0123, wherever it stands, inside brackets as
here.

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
error[E0123]: Sisu has no `:=`; bind with `let` or use `if let`
  --> walrus.sisu:4:11
  |
4 |     if (n := len(names)) > 2:
  |           ^^

```

### `else` on a loop

A loop has no `else` (D-021); search with a function that returns `T?`
and test the result with `if let`. E0127.

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
error[E0127]: a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:`
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

Sisu has no `is`; compare with `==`, test with `.is_none()`, or match
the absent value with `if let` or `??`. E0100, whose fix writes
`.is_none()`, and `.is_some()` for `is not None`.

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
  = fix: Sisu writes it so: `.is_none()`

```

### `lambda x: e`

A closure is `x -> e`, or `it` for one parameter, or a `do` block
(D-026). E0117, whose fix writes `n ->` (`(a, b) ->` for two
parameters).

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
  = fix: Sisu writes it so: `n ->`

```

### `f"..."`

Every string literal interpolates, so there is no prefix
([R1](lexical-structure.md#string-literals)). E0100, whose fix writes
the string without the `f`. After `=`, `let s = f"n is {n}"`, it is
E0117, with the same fix.

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
  = fix: Sisu writes it so: `"n is {n}"`

```

### `class` and `__init__`

A type with fields is a `struct`, built by calling its name with its
fields; there is no inheritance (D-012). `class` is E0107, whose fix
writes `struct`, and `self` outside a method E0215; the methods go in
an `impl` of the struct, and a constructor is the struct's name called
with its fields, or an associated function.

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
  = fix: Sisu writes it so: `struct`

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
(D-011). E0117, at `try:`, `except:` and `finally:`, with the note
`parse.no-exceptions`, which says so.

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
  = note: Sisu has no `try`: an error is a value, a `Result` or a `T ! E`, which `?` passes on and `match`, `else` or `if let` handles

error[E0117]: unexpected `:` after the end of the statement
  --> try_except.sisu:5:11
  |
5 |     except:
  |           ^
  = note: Sisu has no `except`: an error is a value, a `Result` or a `T ! E`, which `?` passes on and `match`, `else` or `if let` handles

```

### `int(text)` and `str(n)`

Text becomes a number with `text.parse_int()`, which says when it
cannot, and a value becomes text by interpolation. E0200, whose note
names Sisu's methods for each conversion.

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
  = note: Sisu converts with methods: text is read with `s.parse_int()` or `s.parse_float()`, which give an option, a number is made text with `n.to_string()` or `"{n}"`, and one number made another with `x.to_int()` or `n.to_f64()`

error[E0200]: cannot find `str` in this scope
  --> conversions.sisu:4:11
  |
4 |     print(str(n))
  |           ^^^
  = note: Sisu converts with methods: text is read with `s.parse_int()` or `s.parse_float()`, which give an option, a number is made text with `n.to_string()` or `"{n}"`, and one number made another with `x.to_int()` or `n.to_f64()`

```

### Python's method names

Sisu's `list` has `push`, and its `str` has `trim` and `to_uppercase`.
E0304's fix names each.

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
  = fix: Sisu spells it this way: `push`

error[E0304]: `str` has no method `strip`
  --> methods.sisu:5:21
  |
5 |     print("  hei  ".strip())
  |                     ^^^^^
  = fix: Sisu spells it this way: `trim`

error[E0304]: `str` has no method `upper`
  --> methods.sisu:6:17
  |
6 |     print("hei".upper())
  |                 ^^^^^
  = fix: Sisu spells it this way: `to_uppercase`

```

### `d.get(key, default)`

`get` takes the key only and gives `V?`: `ages.get("Eero") ?? 0`. E0305,
whose fix writes it.

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
  = fix: `get` gives an option, whose default `??` gives: `ages.get("Eero") ?? 0`

```

### `enumerate(xs, start=1)`

`enumerate` counts from 0; add 1 where the number is used. E0306, whose
note says so.

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
  = note: `enumerate` counts from 0 and takes no `start`: add the start where the count is used, as `i + 1`

```

### `*args`

A parameter that collects any number of arguments, `*prices: int`, is
E0900: [02](../../proposal/02-syntax.md#declarations) gives it and
[14](../../proposal/14-non-goals.md) rules variadic functions out, and
the message says that it waits for that decision. Meanwhile a
`Slice[int]` parameter takes a list, `total([3, 4, 5])`. The call is
also E0305, counted against a function of no parameters.

<!-- file: examples/reference-false-friends/star_args.sisu -->
```sisu
# Python's `*args`, a parameter that collects any number of arguments.
def total(*prices: int) -> int:
    var sum = 0
    for p in prices:
        sum += p
    return sum

def main():
    print(total(3, 4, 5))
```

<!-- output: examples/reference-false-friends/star_args.console -->
```console
$ sisu check star_args.sisu
error[E0900]: a variadic parameter, `*xs: T`, is not implemented in stage 0 yet; it arrives with the decision between 02, which gives it, and 14, which rules variadic functions out; a `Slice[T]` parameter takes a list meanwhile
  --> star_args.sisu:2:11
  |
2 | def total(*prices: int) -> int:
  |           ^^^^^^^^^^^^

```

### `first, *rest = xs`

Star unpacking in a `let` is designed in
[02](../../proposal/02-syntax.md) and not in stage 0: E0900, whose
message names the list pattern of a `match`, `case [first, *rest]:`,
which takes a list apart now ([R6](patterns.md)).

<!-- file: examples/reference-false-friends/star_unpacking.sisu -->
```sisu
# Python's star unpacking.
def main():
    let first, *rest = ["Aino", "Eero", "Roni"]
    print("{first}, then {len(rest)} more")
```

<!-- output: examples/reference-false-friends/star_unpacking.console -->
```console
$ sisu check star_unpacking.sisu
error[E0900]: star unpacking in a `let`, `let first, *rest = xs`, is not implemented in stage 0 yet; it arrives with a later round; a `match` takes a list apart now, `case [first, *rest]:`
  --> star_unpacking.sisu:3:9
  |
3 |     let first, *rest = ["Aino", "Eero", "Roni"]
  |         ^^^^^^^^^^^^

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
traits. These Rust spellings are not Sisu, and each is an error, most
with a fix that writes the Sisu spelling.

| Habit | Sisu | Code | Names the Sisu way |
|---|---|---|---|
| `let mut x` | `var x` | [E0105](diagnostics.md#e0105) | yes, as a fix |
| `a && b`, `a \|\| b`, `!a` | `a and b`, `a or b`, `not a` | [E0103](diagnostics.md#e0103) | yes, as a fix |
| `i++`, `i--` | `i += 1`, `i -= 1` | [E0103](diagnostics.md#e0103) | yes, as a fix |
| `x as f64` | `x.to_f64()` | [E0100](diagnostics.md#e0100), [E0117](diagnostics.md#e0117) | yes, as a fix |
| `// note` | `# note` | [E0117](diagnostics.md#e0117), [E0200](diagnostics.md#e0200) | yes, as a fix |
| `x.clone()` | `x.copy()` | [E0304](diagnostics.md#e0304) | yes, as a fix |
| `1 => "one"` | `case 1: "one"` | [E0100](diagnostics.md#e0100) | yes, as a fix |
| `f(&xs)` | `f(ref xs)`, or `f(xs)` | [E0103](diagnostics.md#e0103) | yes, as a fix |
| `0..3` | `range(3)` | [E0100](diagnostics.md#e0100), [E0117](diagnostics.md#e0117) | yes, as a fix |
| `Vec<T>` | `list[T]` | [E0117](diagnostics.md#e0117) | yes, as a fix |
| `println!(...)` | `print(...)` | [E0117](diagnostics.md#e0117) | yes, as a fix |
| `pub(crate)` | `pub(package)` | [E0130](diagnostics.md#e0130) | yes, as a fix |
| `7 / 2` is 3 | `7 // 2` | [E0301](diagnostics.md#e0301) | yes, as a fix |
| `-7 % 3` is -1 | 2, as in Python | none: FF-6 | |

### `let mut`

A binding that changes is declared with `var`. E0105, whose fix writes
it.

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
  = fix: Sisu writes it so: `var`

```

### `&&`, `||` and `!`

The logical operators are the words `and`, `or` and `not` (D-025); `!`
belongs to error sets. E0103, whose fix writes the word; `i++` gets the
same code and the fix `i += 1`.

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
  = fix: Sisu writes it so: `and`

```

### `x as f64`

A conversion is a method, `n.to_f64()`, and a lossy one says how:
`round()`, `truncate_to_int()`. Inside a call's parentheses, as here,
it is E0100, and after a value that ends a statement, `let x = n as
f64`, E0117; the fix of each writes `.to_f64()`.

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
  = fix: Sisu writes it so: `.to_f64()`

```

### `//` for a comment

A comment starts with `#`; `//` is integer division, as in Python. Here
E0117 reports the comment's words, and its fix writes `#`, as on a line
of its own; a one-word comment after a value, `print(n) // done`, is a
division by a name, E0200, with the same fix.

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
  = fix: Sisu writes it so: `#`

```

### `.clone()`

A copy is `.copy()`, which E0304's fix writes.

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
  = fix: Sisu spells it this way: `copy`

```

### `=>` in a `match` arm

A `case` arm is `case PATTERN: value`. E0100, whose fix writes the `:`.

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
  = fix: Sisu writes it so: `:`

error[E0100]: expected `:`, found `=`
  --> match_arrow.sisu:6:16
  |
6 |         case _ => "many"
  |                ^
  = fix: Sisu writes it so: `:`

```

### `&x` to borrow

A plain parameter already borrows; an argument is written as it is, and
`ref` and `mut` say the rest ([R7](ownership.md)). E0103, whose fix
writes `ref`, which compiles; `total(prices)` says the same.

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
  = fix: Sisu writes it so: `ref `

```

### `0..3` for a range

A range is `range(3)` or `range(a, b)`; `..` is the rest of a pattern.
In a `for` header, as here, it is E0100, and after a value that ends a
statement, `let r = 0..3`, E0117; the fix of each writes `range(0,
3)`.

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
  = fix: Sisu writes it so: `range(0, 3)`

```

### `Vec<T>`

Type arguments go in square brackets, `list[str]`. E0117, whose fix
writes them.

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
  = fix: Sisu writes it so: `list[str]`

```

### `println!`

Sisu has no macros; `print("{x}")` prints. E0117, whose fix writes
`print`.

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
  = fix: Sisu writes it so: `print`

```

### `pub(crate)`

An item is `pub` to every package or `pub(package)` to the modules of
its own ([R13](modules.md#visibility)). Any other name between the
parentheses is E0130 (`parse.visibility-scope`), whose fix writes
`pub(package)`.

<!-- file: examples/reference-false-friends/pub_crate.files/pub_crate.sisu -->
```sisu
# Rust's `pub(crate)`.
pub(crate) def total(prices: list[int]) -> int:
    return sum(prices)

def main():
    print(total([3, 4]))
```

<!-- output: examples/reference-false-friends/pub_crate.console -->
```console
$ sisu check pub_crate.sisu
error[E0130]: `pub(crate)` is not Sisu's: an item is `pub` to every package, or `pub(package)` to the modules of its own
  --> pub_crate.sisu:2:5
  |
2 | pub(crate) def total(prices: list[int]) -> int:
  |     ^^^^^
  = fix: make it visible to its package: `package`

```

## Status

The rule that a shared spelling means the same or fails to compile leans
(D-062), and so does the table it keeps; rows added after 1.0 go through
editions ([13](../../proposal/13-evolution-and-governance.md)). The rows
of D-062 are in stage 0 as their examples show, FF-6 being silent by
design. The diagnostics that do not name the Sisu way are findings for
the compiler, listed in the plan; a diagnostic that names it is part of
the row's check.

