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

@@file negative_index.sisu@@

@@output negative_index.console@@

### FF-2: the order of a `dict` or a `set`

A `dict` iterates in the order its keys were inserted, as Python's does
since 3.7, so code that relies on the order means the same (Q-42,
D-074). The row is expected to be empty, and the experiment measures it.

@@file dict_order.sisu@@

@@output dict_order.out@@

### FF-3: `7 / 2` on integers

Python 3 gives 3.5 and Rust gives 3. Sisu gives neither: `/` divides
floats, and on integers it is E0301, whose fix writes `//`.

@@file int_division.sisu@@

@@output int_division.console@@

### FF-4: `if xs:` for a test of emptiness

Sisu has no truthiness (D-025): a condition is a `bool`, so a list, a
text or a number as a condition is E0302, whose note names the explicit
test.

@@file truthiness.sisu@@

@@output truthiness.console@@

### FF-5: `a if c else b`

Sisu's conditional expression is `if c: a else: b`. Python's order is
E0115 with the fix in a `let`, as [R17](diagnostics.md#e0115) shows;
inside a call's parentheses it is E0100.

@@file conditional.sisu@@

@@output conditional.console@@

### FF-6: `-7 % 3` read as Rust's -1

`%` takes the sign of the divisor and `//` rounds towards minus
infinity, as in Python. Rust's `%` takes the sign of the dividend, so
code written with Rust's meaning is silently different here: this is the
one row with no check, and the corpus translated from Rust flags it.

@@file modulo.sisu@@

@@output modulo.out@@

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

@@file assignment.sisu@@

@@output assignment.console@@

### `print(a, b)`

`print` takes one text; build it with interpolation, `print("Hei,
{name}")`. Several arguments are E0305.

@@file print_many.sisu@@

@@output print_many.console@@

### `'text'` in single quotes

Single quotes make a `char`; text is between double quotes. E0016.

@@file single_quotes.sisu@@

@@output single_quotes.console@@

### `for letter in word`

A text is not iterated directly, since it can be read as characters,
bytes or lines: `for letter in word.chars()`. E0325.

@@file text_loop.sisu@@

@@output text_loop.console@@

### `word[0]`

A `str` has no integer index (D-034); E0345 names `.chars()`, `.bytes()`
and `.byte_slice(a, b)`.

@@file text_index.sisu@@

@@output text_index.console@@

### `x or default`

`or` takes two `bool`s. A default for an absent value is `x ?? default`,
and an empty text is tested with `.is_empty()`. E0302.

@@file or_default.sisu@@

@@output or_default.console@@

### `", ".join(names)`

`join` is a method of an iterator: `names.join(", ")`. E0304, with a
note.

@@file join.sisu@@

@@output join.console@@

### `:=`

Sisu has no assignment expression: bind with `let`, or use `if let` or a
chained condition. E0123 where a statement starts with it; E0100 inside
brackets, as here.

@@file walrus.sisu@@

@@output walrus.console@@

### `else` on a loop

A loop has no `else` (D-021); search with a function that returns `T?`
and test the result with `if let`. E0103.

@@file loop_else.sisu@@

@@output loop_else.console@@

### `a < b > c`

A chain of comparisons points one way. E0113.

@@file chain.sisu@@

@@output chain.console@@

### `x is None`

Sisu has no `is`; compare with `==`, or match the absent value with `if
let` or `??`. E0100.

@@file is_none.sisu@@

@@output is_none.console@@

### `lambda x: e`

A closure is `x -> e`, or `it` for one parameter, or a `do` block
(D-026). E0117.

@@file lambda.sisu@@

@@output lambda.console@@

### `f"..."`

Every string literal interpolates, so there is no prefix
([R1](lexical-structure.md#string-literals)). E0100.

@@file f_string.sisu@@

@@output f_string.console@@

### `class` and `__init__`

A type with fields is a `struct`, built by calling its name with its
fields; there is no inheritance (D-012). `class` is E0107 and `self`
outside a method E0215.

@@file class.sisu@@

@@output class.console@@

### `try` and `except`

An error is a value of type `T ! E`, handled with `?`, `else` or `match`
(D-011). E0117.

@@file try_except.sisu@@

@@output try_except.console@@

### `int(text)` and `str(n)`

Text becomes a number with `text.parse_int()`, which says when it
cannot, and a value becomes text by interpolation. E0200, whose fix for
`int` offers `uint`, which is not what you want.

@@file conversions.sisu@@

@@output conversions.console@@

### Python's method names

Sisu's `list` has `push`, and its `str` has `trim` and `to_uppercase`.
E0304 names no similar method.

@@file methods.sisu@@

@@output methods.console@@

### `d.get(key, default)`

`get` takes the key only and gives `V?`: `ages.get("Eero") ?? 0`. E0305.

@@file dict_get.sisu@@

@@output dict_get.console@@

### `enumerate(xs, start=1)`

`enumerate` counts from 0; add 1 where the number is used. E0306.

@@file enumerate_start.sisu@@

@@output enumerate_start.console@@

### `max(xs)` over a list

Stage 0's `max` and `min` compare two values, and E0308's fix adds a
hole for the second. `sorted(xs)[^1]`, or a loop, gives the largest.
Python's form over a collection is what the prelude list of
[02](../../proposal/02-syntax.md) suggests, and stage 0 does not have
it.

@@file max_list.sisu@@

@@output max_list.console@@

## Python habits whose meaning Sisu changes

A few Python spellings compile in Sisu and mean something else. Each
difference removes a trap that Python documents, and none hides a bug
that Python would show.

### Default arguments are computed at each call

Python computes a default once, when the function is defined, so a
mutable default is shared between calls. Sisu computes it at each call,
so each call that leaves it out gets a new empty list.

@@file defaults.sisu@@

@@output defaults.out@@

### Each loop iteration binds its own variable

A closure made in a Python loop sees the loop variable's last value. In
Sisu each iteration binds its own `i`, and an `own` closure takes it, so
the three closures give 0, 1 and 2.

@@file closures.sisu@@

@@output closures.out@@

### A closure cannot outlive a borrowed loop variable

Without `own`, the closure borrows `i`, which ends with its iteration,
so keeping the closure is E0406: the late-binding trap is a compile
error rather than a wrong value.

@@file closures_borrowed.sisu@@

@@output closures_borrowed.console@@

### An `int` has 64 bits

Python's integers grow without limit. Sisu's `int` has 64 bits, and
arithmetic past them panics (D-009), so the difference is loud, at run
time ([R17](diagnostics.md#r0003)). `BigInt` grows without limit when
that is what you want.

@@file overflow.sisu@@

@@output overflow.console@@

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

@@file let_mut.sisu@@

@@output let_mut.console@@

### `&&`, `||` and `!`

The logical operators are the words `and`, `or` and `not` (D-025); `!`
belongs to error sets. E0103.

@@file symbols.sisu@@

@@output symbols.console@@

### `x as f64`

A conversion is a method, `n.to_f64()`, and a lossy one says how:
`round()`, `truncate_to_int()`. E0100.

@@file as_cast.sisu@@

@@output as_cast.console@@

### `//` for a comment

A comment starts with `#`; `//` is integer division, as in Python. Here
E0117 reports the comment's words.

@@file slash_comment.sisu@@

@@output slash_comment.console@@

### `.clone()`

A copy is `.copy()`. E0304 names no similar method.

@@file clone.sisu@@

@@output clone.console@@

### `=>` in a `match` arm

A `case` arm is `case PATTERN: value`. E0100.

@@file match_arrow.sisu@@

@@output match_arrow.console@@

### `&x` to borrow

A plain parameter already borrows; an argument is written as it is, and
`ref` and `mut` say the rest ([R7](../README.md#the-reference), when it
is written). E0103.

@@file borrow_sign.sisu@@

@@output borrow_sign.console@@

### `0..3` for a range

A range is `range(3)` or `range(a, b)`; `..` is the rest of a pattern.
E0100.

@@file dot_range.sisu@@

@@output dot_range.console@@

### `Vec<T>`

Type arguments go in square brackets, `list[str]`. E0117.

@@file angle_generics.sisu@@

@@output angle_generics.console@@

### `println!`

Sisu has no macros; `print("{x}")` prints. E0117.

@@file println.sisu@@

@@output println.console@@

## Status

The rule that a shared spelling means the same or fails to compile leans
(D-062), and so does the table it keeps; rows added after 1.0 go through
editions ([13](../../proposal/13-evolution-and-governance.md)). The rows
of D-062 are in stage 0 as their examples show, FF-6 being silent by
design. The diagnostics that do not name the Sisu way are findings for
the compiler, listed in the plan; a diagnostic that names it is part of
the row's check.

