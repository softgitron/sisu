# Lexical structure

This page describes how Sisu reads the text of a program: the source
text, names and the rules for spelling them, keywords, comments, lines
and indentation, every form of literal, the operators and punctuation,
and the layout that `sisu fmt` writes. What the expressions built from
these tokens mean is [R4](expressions.md).

**See also:** the tutorial's [chapter 1](../tutorial/01-hello.md)
(comments, indentation), [chapter 2](../tutorial/02-values.md) (number
literals and naming) and [chapter 6](../tutorial/06-text.md) (text
literals); the proposal's [02, Syntax](../../proposal/02-syntax.md),
"Lexical structure", "Literals", "Formatting" and "Departures from
Python"; the compiler's [architecture
notes](../../compiler/docs/architecture.md), "Lexing" and "The
formatter".

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains: **In stage 0** for what the compiler does as the examples
show, **Designed** for what the proposal decides and no compiler has,
**Leaning** for a decision that may still change. A paragraph that
starts **Stage 0 differs** says where the compiler does something other
than what the proposal says.

## Source text

A source file is UTF-8 text, and a line ends with a line feed or a
carriage return and line feed. Outside strings and comments, a
character that no token uses is E0005 (`lex.unknown-character`): a
control character, a symbol such as `$`, a byte-order mark at the start
of the file, or a no-break space. A character that shows as a space or
as nothing (a no-break space, a zero-width space, a soft hyphen, the
marks of the text's direction, a byte-order mark) is written as its
escape, with a note that names it and a fix that writes a space or
removes it:

<!-- file: examples/reference-lexical-structure/no_break_space.sisu -->
```sisu
# A no-break space, pasted from a web page, between `=` and the value.
def main():
    let heat = 80
    print(heat)
```

<!-- output: examples/reference-lexical-structure/no_break_space.console -->
```console
$ sisu check no_break_space.sisu
error[E0005]: `\u{a0}` cannot appear in Sisu source outside a string or comment
  --> no_break_space.sisu:3:15
  |
3 |     let heat = 80
  |               ^
  = note: it is a no-break space, which looks like a space
  = fix: write a space: ` `

```

The line in the message looks right, since the character looks like a
space; the escape `\u{a0}` and the note say what it is.

**Status.** In stage 0.

## Names

A name (an identifier) is a Unicode letter or `_`, followed by letters,
digits and `_`. Names are case-sensitive. A keyword is not a name,
except after `.` or `?.` and after `def`, where it is an ordinary name:
`x.or(7)` calls the method `or`, and `def or(...)` declares it.

<!-- file: examples/reference-lexical-structure/identifiers.sisu -->
```sisu
# Names are Unicode letters, digits and `_`, not starting with a digit.
def main():
    let päivä = "maanantai"
    let _unused = 0
    let room2 = "sauna"
    let maybe: int? = None
    print("{päivä} {room2} {maybe.or(7)}")
```

<!-- output: examples/reference-lexical-structure/identifiers.out -->
```text
maanantai sauna 7
```

A letter may be followed by marks that combine with it, such as an
accent typed as a character of its own, and a name is read in NFC
([02](../../proposal/02-syntax.md), D-048): its letters and marks are
composed as Unicode's normal form C composes them, so `e` followed by
the combining acute accent `\u{301}` and the one letter `é` name the
same binding. The text of the file stays as it was written. In this
program, the binding is spelt the first way and its use the second:

<!-- file: examples/reference-lexical-structure/nfc.sisu -->
```sisu
# One name, spelt with `e` and a combining accent, then with `é`.
def main():
    let café = "kahvila"
    print(café)
```

<!-- output: examples/reference-lexical-structure/nfc.out -->
```text
kahvila
```

Korean is composed by Unicode's rule for Hangul: a leading consonant,
a vowel and a trailing consonant written as three jamo name the same
binding as the one syllable they make:

<!-- file: examples/reference-lexical-structure/hangul.sisu -->
```sisu
# One name, spelt with the jamo ᄀ ᅡ ᆨ, then with the syllable 각.
def main():
    let 각 = "a corner of the sauna"
    print(각)
```

<!-- output: examples/reference-lexical-structure/hangul.out -->
```text
a corner of the sauna
```

Two lints, on by default, catch a name that is not what it looks like.
W0031 (`resolve.mixed-script`) warns of a declared name whose letters
come from more than one script, Latin and Cyrillic, say; digits, `_`
and letters of no script do not count, and the scripts of Chinese,
Japanese and Korean count as one. W0032 (`resolve.confusable`) warns
of a declared name that reads as another name in scope, a module's
item or a local of an enclosing scope, when one of the two holds a
letter beyond ASCII; its label stands at the module's declaration. A
program whose names are all ASCII pays nothing for either. This one
has a Cyrillic `а` (`\u{430}`) in its local `pаrt`:

<!-- file: examples/reference-lexical-structure/lookalikes.sisu -->
```sisu
# A name with a Cyrillic `а`, beside the function `part`.
def part() -> int:
    return 1

def main():
    let pаrt = 2
    print(pаrt + part())
```

<!-- output: examples/reference-lexical-structure/lookalikes.console -->
```console
$ sisu run lookalikes.sisu
warning[W0031]: `pаrt` mixes letters of the Latin and Cyrillic scripts
  --> lookalikes.sisu:6:9
  |
6 |     let pаrt = 2
  |         ^^^^

warning[W0032]: `pаrt` looks like `part`, another name in scope
  --> lookalikes.sisu:6:9
  |
6 |     let pаrt = 2
  |         ^^^^
  |
2 | def part() -> int:
  |     ---- `part` is declared here

3
```

W0032 compares the names' skeletons, as Unicode's security
mechanisms (UTS #39) define them: each character written as what
Unicode's `confusables.txt` maps it to, which may be more than one
character, so `rn` reads as `m` and `1` as `l`. Once one of the two
names holds a letter beyond ASCII, an ASCII look-alike counts too:

<!-- file: examples/reference-lexical-structure/skeleton.sisu -->
```sisu
# `rnоdern`, with a Cyrillic `о`, reads as `modern`: `rn` reads as `m`.
def modern() -> int:
    return 1

def main():
    let rnоdern = 2
    print(rnоdern + modern())
```

<!-- output: examples/reference-lexical-structure/skeleton.console -->
```console
$ sisu run skeleton.sisu
warning[W0031]: `rnоdern` mixes letters of the Latin and Cyrillic scripts
  --> skeleton.sisu:6:9
  |
6 |     let rnоdern = 2
  |         ^^^^^^^

warning[W0032]: `rnоdern` looks like `modern`, another name in scope
  --> skeleton.sisu:6:9
  |
6 |     let rnоdern = 2
  |         ^^^^^^^
  |
2 | def modern() -> int:
  |     ------ `modern` is declared here

3
```

A keyword where a name should be is an error. Where a function's, a
parameter's or a field's name should be, it is E0124
(`parse.keyword-as-name`), and the line declares nothing; elsewhere, the parser reports what it expected:

<!-- file: examples/reference-lexical-structure/keyword_name.sisu -->
```sisu
# A keyword used as a binding's name.
def main():
    let type = "csv"
    print(type)
```

<!-- output: examples/reference-lexical-structure/keyword_name.console -->
```console
$ sisu run keyword_name.sisu
error[E0105]: expected a pattern, found `type`
  --> keyword_name.sisu:3:9
  |
3 |     let type = "csv"
  |         ^^^^
  = note: `type` is a keyword, which names no binding: give it another name

error[E0103]: expected an expression, found `type`
  --> keyword_name.sisu:4:11
  |
4 |     print(type)
  |           ^^^^

error[D0006]: the program was not run: errors found (2)
```

<!-- file: examples/reference-lexical-structure/keyword_parameter.sisu -->
```sisu
# A keyword used as a parameter's name.
def score(match: str) -> int:
    return 0

def main():
    print(score("final"))
```

<!-- output: examples/reference-lexical-structure/keyword_parameter.console -->
```console
$ sisu run keyword_parameter.sisu
error[E0124]: `match` is a keyword and cannot be used as a name here
  --> keyword_parameter.sisu:2:11
  |
2 | def score(match: str) -> int:
  |           ^^^^^

error[D0006]: the program was not run: errors found (1)
```

The lints' tables are generated from Unicode 15.1.0's own data files:
a letter's or a mark's script is the one `Scripts.txt` gives it, 373
ranges of 156 scripts, where `Common`, `Inherited` and `Unknown` count
as no script, so a modifier letter such as `ʰ` is Latin and `αʰ` mixes
two scripts; and the confusables are the 5,147 records of
`confusables.txt` whose character can stand in a name.

**Stage 0 differs.** A keyword as a binding's name gets E0105, whose
note `parse.keyword-name` says that the word is a keyword, and each use
of it E0103, where E0124 reports the declaration alone.

**Status.** In stage 0 (D-048), with the differences noted.

## Naming rules

Sisu's names follow PEP 8:

| Kind of item | Spelling | Examples |
|---|---|---|
| Values, bindings, parameters, functions, methods, fields, modules, packages | `snake_case` | `word_count`, `read_to_string` |
| Types, traits, enum variants, error sets | `CamelCase` | `String`, `Display`, `NotANumber` |
| `const` items | `SCREAMING_CASE` | `MAX_SIZE` |
| The prelude's constants | `CamelCase` | `True`, `False`, `None`, `Some`, `Ok`, `Err` |
| Builtin types with a literal or primitive form | lowercase | `int`, `f64`, `bool`, `char`, `str`, `list`, `dict`, `set` |

Every other type is `CamelCase`: `String`, `Array`, `Option`, `Shared`.
Because a variant is always capitalised, a capitalised name in a pattern
is a constant and a lowercase one binds a value (R6, Patterns, when it is
written). A package is named in lowercase letters, digits and `_`
([R16, sisu new](sisu-command.md#sisu-new)).

**Status.** In stage 0 (D-055).

## Keywords

These 47 words are reserved, and are names only after `.`, `?.` and
`def`:

| | | | | | | |
|---|---|---|---|---|---|---|
| `and` | `as` | `assert` | `break` | `case` | `comptime` | `const` |
| `continue` | `def` | `defer` | `do` | `dyn` | `elif` | `else` |
| `enum` | `error` | `extern` | `for` | `from` | `if` | `impl` |
| `import` | `in` | `let` | `match` | `mut` | `not` | `or` |
| `own` | `pass` | `pub` | `ref` | `return` | `select` | `self` |
| `Self` | `struct` | `test` | `trait` | `type` | `unsafe` | `var` |
| `view` | `where` | `while` | `with` | `yield` | | |

These 11 are contextual keywords, keywords only where they have a
meaning and names everywhere else: `requires`, `ensures`, `invariant`,
`decreases`, `old` and `result` in contracts; `it` in a closure; `bench`
and `suite` at the start of a declaration; `expect` in a test; and
`pure` before `def` and in a function type.

**Status.** In stage 0.

## Comments and doc comments

- `#` starts a comment, which runs to the end of the line.
- `##` starts a doc comment. On a line of its own, it documents the item
  or field that follows; at the end of a declaration's line, it
  documents that line, such as a field. The examples in a doc comment,
  in fenced blocks, are tests that `sisu test` runs.
- `##!` documents the module that the comment is in.

`//` is not a comment: it is integer division, as in Python.

<!-- file: examples/reference-lexical-structure/comments.sisu -->
```sisu
##! A module of points. This comment documents the module.

# A plain comment, which the documentation leaves out.

## A point on a grid.
struct Point:
    ## How far right the point is.
    x: int
    y: int  ## How far up the point is.

## The point `n` steps right of `p`.
def right(p: Point, n: int) -> Point:
    return Point(p.x + n, p.y)  # a comment at the end of a line

def main():
    print("{right(Point(1, 2), 3):?}")
```

<!-- output: examples/reference-lexical-structure/comments.out -->
```text
Point(x=4, y=2)
```

`sisu doc` makes pages of the doc comments ([R16, sisu
doc](sisu-command.md#sisu-doc)); the plain comments are for the reader of
the source only.

**Status.** In stage 0.

## Lines and statements

A statement takes one logical line, and a line break ends it. There are
no semicolons: `;` is E0011 (`lex.semicolon`). A logical line continues
onto the next line:

- inside `()`, `[]` and `{}`, until the bracket closes;
- after a line that ends with `,` or an open bracket;
- when the next line starts with a binary operator, `.`, `?.` or `??`.
  The binary operators include `and`, `or`, `in` and `not in`.

A line that continues may be indented as you like; `sisu fmt` joins it
to the line before when the whole fits in 100 columns. A bracket that
closes nothing is E0008, one never closed E0009, and one closed by the
wrong bracket E0010.

<!-- file: examples/reference-lexical-structure/continuation.sisu -->
```sisu
# A logical line continues inside brackets and before an operator.
def main():
    let prices = [
        12,
        5,
        30,
    ]
    let total = prices[0]
        + prices[1]
        + prices[2]
    let words = "hei maailma hei"
        .split_whitespace()
        .count()
    let ready = total > 40
        and words == 3
    print("{total} {words} {ready}")
```

<!-- output: examples/reference-lexical-structure/continuation.out -->
```text
47 3 True
```

There is no backslash continuation; a `\` at the end of a line is E0015
(`lex.backslash-continuation`), whose message names the three ways to
break a line:

<!-- file: examples/reference-lexical-structure/backslash.sisu -->
```sisu
# A backslash at the end of a line.
def main():
    let total = 1 + \
        2
    print(total)
```

<!-- output: examples/reference-lexical-structure/backslash.console -->
```console
$ sisu run backslash.sisu
error[E0015]: Sisu has no backslash continuation; break the line inside brackets, after a comma or before an operator
  --> backslash.sisu:3:21
  |
3 |     let total = 1 + \
  |                     ^

error[D0006]: the program was not run: errors found (1)
```

<!-- file: examples/reference-lexical-structure/semicolon.sisu -->
```sisu
# Two statements on one line.
def main():
    let a = 1; let b = 2
    print(a + b)
```

<!-- output: examples/reference-lexical-structure/semicolon.console -->
```console
$ sisu run semicolon.sisu
error[E0011]: Sisu has no semicolons; put each statement on its own line
  --> semicolon.sisu:3:14
  |
3 |     let a = 1; let b = 2
  |              ^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0 (D-020).

## Indentation and blocks

A logical line that ends with `:` opens a block: the lines after it that
are indented more than the line that opened it. The lexer turns
indentation into INDENT and DEDENT tokens with a stack of widths, as
Python's reference does, without asking the parser:

- A line indented more than the one before opens a level, and a line
  indented less closes every level wider than it.
- Blank lines and lines holding only a comment do not count.
- Indentation is ASCII spaces only. A tab in indentation is E0001, a
  form feed E0014, and a no-break space E0005.
- Any width is a block, as long as the lines of one block agree; `sisu
  fmt` writes four spaces.
- `pass` is the empty block.

<!-- file: examples/reference-lexical-structure/blocks.sisu -->
```sisu
# Any consistent width of indentation opens a block.
def main():
  let n = 2
  if n > 1:
     print("a block indented by three more spaces")
  else:
     pass
```

<!-- output: examples/reference-lexical-structure/blocks.out -->
```text
a block indented by three more spaces
```

A line that goes back to an indentation no enclosing block has is
E0002, which names the columns that would match. A line indented more
than the one before, when that one opens no block, is E0125:

<!-- file: examples/reference-lexical-structure/dedent.sisu -->
```sisu
# A line that matches no enclosing block's indentation.
def main():
    let n = 2
    if n > 1:
        print("big")
      print("done")
```

<!-- output: examples/reference-lexical-structure/dedent.console -->
```console
$ sisu run dedent.sisu
error[E0002]: this line's indentation matches no enclosing block; the enclosing blocks start at column 1 or 5
  --> dedent.sisu:6:1
  |
6 |       print("done")
  | ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

A block always starts on the next line. A statement after the `:` on the
same line is E0102, and a line opening a block with nothing indented
after it is E0101:

<!-- file: examples/reference-lexical-structure/one_line_block.sisu -->
```sisu
# A block on the line that opens it.
def main():
    for n in [1, 2]: print(n)
```

<!-- output: examples/reference-lexical-structure/one_line_block.console -->
```console
$ sisu run one_line_block.sisu
error[E0102]: a block starts on the next line, indented; nothing may follow `:` on the line that opens it
  --> one_line_block.sisu:3:22
  |
3 |     for n in [1, 2]: print(n)
  |                      ^^^^^

error[D0006]: the program was not run: errors found (1)
```

A one-line `if c: print(x)` is E0102 as well: Python's one-line `if`
is a statement whose block is on its line, not an `if` expression
without its `else` (E0116).

**Status.** In stage 0 (D-027).

## Integer literals

An integer literal is written in decimal (`42`), hexadecimal (`0xFF`),
binary (`0b1010`) or octal (`0o17`). `_` may stand between digits and
after the prefix, to group them: `1_000_000`, `0x_ff`. A suffix gives
the literal a type: `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`,
`u32`, `u64`, `u128`, `int` or `uint`; `f32` and `f64` make a decimal
literal a float. Without a suffix, the literal is an `int`, unless the
type expected where it stands is another integer type or an exact type
of `std.num` ([R2](types.md#integers)).

<!-- file: examples/reference-lexical-structure/numbers.sisu -->
```sisu
# Integer and floating-point literals.
def main():
    print("{1_000_000} {0xFF} {0b1010} {0o17} {0x_ff}")
    let small = 42u8
    let wide = 7i128
    let unsigned = 3uint
    print("{small} {wide} {unsigned}")
    print("{2.5e-3} {1e3} {1.5f32} {3f64} {1_000.5}")
```

<!-- output: examples/reference-lexical-structure/numbers.out -->
```text
1000000 255 10 15 255
42 7 3
0.0025 1000.0 1.5 3.0 1000.5
```

Letters after a number that are not a suffix, or a prefix without
digits, are E0012 (`lex.invalid-number`); a literal too large for its
type is E0374 ([R17](diagnostics.md#e0374)).

<!-- file: examples/reference-lexical-structure/bad_number.sisu -->
```sisu
# A number with letters that are not a suffix.
def main():
    print(12abc)
```

<!-- output: examples/reference-lexical-structure/bad_number.console -->
```console
$ sisu run bad_number.sisu
error[E0012]: `12abc` is not a valid number literal
  --> bad_number.sisu:3:11
  |
3 |     print(12abc)
  |           ^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0.

## Floating-point literals

A floating-point literal has digits on both sides of its point
(`1.0`, `0.5`), an exponent (`2.5e-3`, `1e3`), or a suffix `f32` or
`f64`. It is an `f64` unless its suffix or the type expected says
otherwise. `1.` and `.5` are not literals: a point needs digits on both
sides, so that `1.method()` reads as a call. Python's `1.` is E0100 and
`.5` E0103, each with the fix that writes `1.0` or `0.5`.

**Status.** In stage 0.

## Boolean literals

`True` and `False`, of type `bool`, capitalised like the other
constants of the prelude.

**Status.** In stage 0.

## Character literals

A character literal is one Unicode character between single quotes, of
type `char`: `'a'`, `'ä'`. Its escapes are `\n` (line feed), `\t` (tab),
`\r` (carriage return), `\0` (the zero character), `\\`, `\'`, `\"`, and
`\u{...}`, one to six hexadecimal digits naming a Unicode scalar value.

<!-- file: examples/reference-lexical-structure/characters.sisu -->
```sisu
# Character literals and their escapes.
def main():
    let letters = ['a', 'ä', '\n', '\t', '\\', '\'', '"', '\0']
    print("{letters:?}")
    print('\u{1F600}')
```

<!-- output: examples/reference-lexical-structure/characters.out -->
```text
['a', 'ä', '\n', '\t', '\\', '\'', '"', '\u{0}']
😀
```

A literal that is not closed on its line is E0006, one that holds no
character or more than one is E0016 ([R17](diagnostics.md#e0016)), and
an escape that is not one of these is E0007. Text is written between
double quotes, never single ones.

**Status.** In stage 0.

## String literals

A string literal is text between double quotes. Its forms are these:

| Form | Example | Escapes | Interpolation | Type |
|---|---|---|---|---|
| Plain | `"Hei"` | yes | no | `str` |
| Interpolated | `"Hei, {name}"` | yes | yes | `String` |
| Multi-line | `"""..."""` | yes | yes | as the plain and interpolated forms |
| Raw | `r"C:\temp"`, `r"""..."""` | no | no | `str` |
| Bytes | `b"\x00\xff"` | yes, and `\xNN` | no | `bytes` |

- The escapes are those of character literals. A bytes literal also
  takes `\xNN`, any byte.
- `{expr}` in a plain or multi-line literal is an interpolation: the
  expression's value, written into the text. `{expr:spec}` gives a
  format spec, `{expr=}` writes the expression's text and `=` before its
  `Debug` value, and `{{` and `}}` are literal braces. There is no `f`
  prefix: every literal can interpolate, and one that does is a
  `String`. What the format specs mean is
  [R4](expressions.md#interpolation-and-format-specs).
- A multi-line literal keeps every character between its quotes,
  including the first line break and the indentation of each line.
- A one-line literal ends on its line. One that is not closed there is
  E0003 ([R17](diagnostics.md#e0003)); a multi-line literal never
  closed is E0004. A single `}` is E0013, an empty interpolation `{}`
  E0121, and a line break inside an interpolation of a one-line
  literal E0018.
- A string literal takes the type expected where it stands when that
  type implements `FromLiteral[str]`, as `Path` and `String` do:
  `let home: Path = "/home/aino"`
  ([R2](types.md#literals-that-take-the-type-expected)).

<!-- file: examples/reference-lexical-structure/strings.sisu -->
```sisu
# The forms of a string literal.
def main():
    let name = "Aino"
    print("Hei, {name}! {{braces}} \"quotes\"\ttab")
    print(r"raw: \n {name}")
    print("""two
lines, {name}""")
    let data = b"bytes\x00\xff"
    print("{data:?} has {len(data)} bytes")
```

<!-- output: examples/reference-lexical-structure/strings.out -->
```text
Hei, Aino! {braces} "quotes"	tab
raw: \n {name}
two
lines, Aino
b"bytes\x00\xff" has 7 bytes
```

**Status.** In stage 0 (D-044; `{expr=}` leans, D-071).

## Collection literals

| Literal | Example | Type |
|---|---|---|
| List | `[1, 2, 3]` | `list[T]`, or `Array[T, N]` or `Slice[T]` where the expected type says so |
| Dict | `{"a": 1, "b": 2}`, `{}` | `dict[K, V]`; `{}` is the empty dict |
| Set | `{1, 2, 3}` | `set[T]`; the empty set is `set()` |
| Tuple | `(1, "a")`, `(x,)` | `(int, str)`; one element needs the comma |
| Unit | `()` | `()` |
| Comprehension | `[x * x for x in xs if x > 0]`, `{k: v for k, v in pairs}`, `{x for x in xs}` | `list`, `dict`, `set` |
| Generator expression | `(x * x for x in xs)` | a lazy `Iter[T]` |

The elements of a literal are separated by commas, and a comma after the
last one is allowed. An empty literal takes its element types from the
type expected where it stands.

<!-- file: examples/reference-lexical-structure/collections.sisu -->
```sisu
# List, dict, set and tuple literals, and comprehensions.
def main():
    let scores = [3, 1, 4]
    let ages = {"Aino": 7, "Eero": 9}
    let nothing: dict[str, int] = {}
    let vowels = {'a', 'e', 'i'}
    let pair = (1, "one")
    let squares = [n * n for n in scores if n > 1]
    let by_name = {name: age + 1 for name, age in ages.items()}
    print("{scores} {ages} {nothing} {len(vowels)} {pair}")
    print("{squares} {by_name} {set[int]()} {()}")
```

<!-- output: examples/reference-lexical-structure/collections.out -->
```text
[3, 1, 4] {"Aino": 7, "Eero": 9} {} 3 (1, "one")
[9, 16] {"Aino": 8, "Eero": 10} set() ()
```

**Status.** In stage 0 (D-044).

## Operators and punctuation

The lexer takes the longest token that matches, so `//=` is one token
and not `//` and `=`.

| Tokens | Meaning ([R4](expressions.md#precedence)) |
|---|---|
| `+` `-` `*` `/` `//` `%` `**` | arithmetic: `/` divides floats, `//` integers; `-x` negates |
| `+%` `-%` `*%` | arithmetic that wraps around |
| `+\|` `-\|` `*\|` | arithmetic that saturates |
| `==` `!=` `<` `<=` `>` `>=` | comparison, chained in one direction |
| `&` `\|` `^` `<<` `>>`, and `~x` | bits and shifts; `~x` flips the bits of `x` |
| `=` and `+=` `-=` `*=` `/=` `//=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `+%=` `-%=` `*%=` `+\|=` `-\|=` `*\|=` | assignment and augmented assignment |
| `?` `?.` `??` | propagate, the field of an optional value, the fallback |
| `!` | an error set in a type, `T ! E` |
| `->` | a result type, a closure |
| `.` `..` | a field or method; the rest in a pattern, an open error set |
| `^` before an index | counting from the end, `xs[^1]` |
| `(` `)` `[` `]` `{` `}` `,` `:` `@` | brackets, separators, a block, an attribute |

`and`, `or`, `not` and `in` are keywords rather than symbols, and Sisu
has no `is` (D-025). `:=` is E0123
(`parse.walrus`), `;` is E0011 and `\` at the end of a line E0015.

**Status.** In stage 0 (D-022, D-025).

## The formatter's layout

`sisu fmt` writes one layout, with no options (D-030):

- four spaces of indentation, and lines of at most 100 columns;
- a bracket that does not fit is broken with its contents on lines of
  their own, then one element per line with a comma after the last
  (Black's bracket-first splitting); a comma after the last element in
  the source keeps one element per line;
- a long expression breaks before its binary operators, and a
  comprehension before its clauses;
- one blank line around each top-level definition and at most one
  elsewhere; every comment keeps its text and its line;
- `Option[T]` is written `T?`, and `Result[T, E]` is written `T ! E`
  when `E` is an error set;
- a run of imports is sorted, and every import is kept, the ones the
  file does not use among them, which `sisu lint` names (W0027).

The formatter checks that the text it writes parses to the same program;
otherwise it leaves the file as it is (D0076). It formats only files
that parse, and never moves a statement to another block.

<!-- file: examples/reference-lexical-structure/layout.sisu -->
```sisu
# Code as `sisu fmt` finds it.
from std.time import Duration
from std.math import clamp
from std.collections import Deque
def first_word(text: str) -> Option[String]:
  for word in text.split_whitespace():
    return Some(word.to_string())
  return None
def main():
  var queue = Deque[str].new()
  queue.push_back("Aino",)
  print(first_word("hei maailma") ?? "nothing")
  print("{queue.pop_front() ?? "empty"} {clamp(1.5, 0.0, 1.0)}")
```

<!-- output: examples/reference-lexical-structure/layout.console -->
```console
$ sisu fmt --check layout.sisu
`layout.sisu` is not formatted; `sisu fmt` would change it:
@@ line 1 @@
  # Code as `sisu fmt` finds it.
- from std.time import Duration
- from std.math import clamp
  from std.collections import Deque
- def first_word(text: str) -> Option[String]:
-   for word in text.split_whitespace():
-     return Some(word.to_string())
-   return None
+ from std.math import clamp
+ from std.time import Duration
+ 
+ def first_word(text: str) -> String?:
+     for word in text.split_whitespace():
+         return Some(word.to_string())
+     return None
+ 
  def main():
-   var queue = Deque[str].new()
-   queue.push_back("Aino",)
-   print(first_word("hei maailma") ?? "nothing")
-   print("{queue.pop_front() ?? "empty"} {clamp(1.5, 0.0, 1.0)}")
+     var queue = Deque[str].new()
+     queue.push_back(
+         "Aino",
+     )
+     print(first_word("hei maailma") ?? "nothing")
+     print("{queue.pop_front() ?? "empty"} {clamp(1.5, 0.0, 1.0)}")
$ sisu fmt layout.sisu
$ sisu fmt --check layout.sisu
$ sisu lint layout.sisu
warning[W0027]: `Duration` is imported and not used in this file
  --> layout.sisu:4:1
  |
4 | from std.time import Duration
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: remove the import

```

The formatted file keeps `Duration`, which it does not use, and `sisu
lint` names it, with the fix that removes it
([R16](sisu-command.md#sisu-lint)).

**Status.** In stage 0 (D-030).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0001](diagnostics.md#e0001) | `lex.tab-in-indentation` | [Indentation and blocks](#indentation-and-blocks) |
| [E0002](diagnostics.md#e0002) | `lex.inconsistent-dedent` | [Indentation and blocks](#indentation-and-blocks) |
| [E0003](diagnostics.md#e0003) | `lex.unterminated-string` | [String literals](#string-literals) |
| E0004 | `lex.unterminated-block-string` | [String literals](#string-literals) |
| [E0005](diagnostics.md#e0005) | `lex.unknown-character` | [Source text](#source-text) |
| E0006 | `lex.unterminated-char` | [Character literals](#character-literals) |
| E0007 | `lex.invalid-escape` | [Character literals](#character-literals) |
| E0008, E0009, E0010 | `lex.unmatched-close`, `lex.unclosed-bracket`, `lex.mismatched-bracket` | [Lines and statements](#lines-and-statements) |
| [E0011](diagnostics.md#e0011) | `lex.semicolon` | [Lines and statements](#lines-and-statements) |
| [E0012](diagnostics.md#e0012) | `lex.invalid-number` | [Integer literals](#integer-literals) |
| E0013 | `lex.lone-close-brace` | [String literals](#string-literals) |
| E0014 | `lex.bad-indent-char` | [Indentation and blocks](#indentation-and-blocks) |
| [E0015](diagnostics.md#e0015) | `lex.backslash-continuation` | [Lines and statements](#lines-and-statements) |
| [E0016](diagnostics.md#e0016) | `lex.bad-char-literal` | [Character literals](#character-literals) |
| E0018 | `lex.newline-in-interpolation` | [String literals](#string-literals) |
| E0101, [E0102](diagnostics.md#e0102), E0125 | `parse.expected-block`, `parse.block-on-same-line`, `parse.unexpected-indent` | [Indentation and blocks](#indentation-and-blocks) |
| E0121 | `parse.empty-interpolation` | [String literals](#string-literals) |
| [E0123](diagnostics.md#e0123) | `parse.walrus` | [Operators and punctuation](#operators-and-punctuation) |
| [E0124](diagnostics.md#e0124) | `parse.keyword-as-name` | [Names](#names) |
| [W0027](diagnostics.md#w0027) | `lint.unused-import` | [The formatter's layout](#the-formatters-layout) |
| [W0031](diagnostics.md#w0031), [W0032](diagnostics.md#w0032) | `resolve.mixed-script`, `resolve.confusable` | [Names](#names) |

[R17, Diagnostics](diagnostics.md) lists every code.
