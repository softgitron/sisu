# Diagnostics

A diagnostic is what Sisu says when something is wrong: a code, a
message, the place, and often a fix. This page explains the parts of a
diagnostic, lists every code by range, and gives an entry for each code
that the manual's pages show: the rule it guards, a wrong program, what
Sisu prints for it, and the program with the fix. The entries grow with
the manual; a code without one is still in the tables of
[Every code](#every-code).

**See also:** [chapter 1](../tutorial/01-hello.md) of the tutorial
(reading a diagnostic and breaking a program on purpose); the proposal's
[10, Tooling](../../proposal/10-tooling.md), "Diagnostics"; the compiler's
[message catalog](../../compiler/catalog/messages.catalog) and its
[architecture notes](../../compiler/docs/architecture.md), "Diagnostics
and the message catalog".

## How to read this page

Each entry has the code as its heading, so a link to a code is
`diagnostics.md#e0200`. Under it come the catalog identifier and the
message, with `{name}` for each part that the diagnostic fills in; the
rule the code guards; the wrong program and what `sisu run` prints for
it; and the program with the fix and what it prints. Every program ran
when the manual was last checked.

The messages are those of stage 0, the compiler in this repository.
**Status.** The catalog, the codes, the identifiers, the human form and
the identifier form are in stage 0 (D-065). The long explanations that
[10](../../proposal/10-tooling.md) gives `sisu explain`, the consequence
line on each of several fixes, and the teaching profile's gloss of dense
signatures are designed, not in stage 0 (Q-66, D-073).

## The parts of a diagnostic

`sisu run`, `sisu check` and the other commands print a diagnostic in its
human form:

<!-- output: examples/reference-diagnostics/reading.console -->
```console
$ sisu run e0200_misspelt.sisu
error[E0200]: cannot find `prnt` in this scope
  --> e0200_misspelt.sisu:3:5
  |
3 |     prnt("Hei, maailma!")
  |     ^^^^
  = fix: a similar name is in scope: `print`

error[D0006]: the program was not run: errors found (1)
$ sisu check --format=ids e0200_misspelt.sisu
3:5-3:9 E0200 resolve.undefined-name name="prnt"
  fix resolve.similar-name 3:5-3:9 "print"
$ sisu explain E0200
E0200 (error) resolve.undefined-name: cannot find `{name}` in this scope
```

- **The code and the message.** `error[E0200]` is the severity and the
  code; the message after it says what is wrong, naming the thing, not
  the stage of the compiler that noticed it. The code is the
  diagnostic's identity: it never changes and is never reused, while the
  English text may be reworded or translated.
- **The place.** `--> e0200_misspelt.sisu:3:5` is the file, the line and
  the column, counted in characters from 1. The source line follows,
  with `^` under the span that is wrong.
- **Labels.** A line marked with `-` under another span says how it is
  involved, such as `` `names` is moved here `` or `expected because of
  this`.
- **The fix.** `= fix:` says what the fix does, then the text it writes,
  between backquotes. The language server offers it as a code action. A
  diagnostic without a fix may have a `= note:` that says what to do, or
  a `= help:`.
- **The last line.** A command that stopped because of errors says so,
  with a code of the driver: D0006 for errors, D0021 for errors of the
  ownership checker only.

`sisu check --format=ids` prints the same diagnostics in the identifier
form: one line per diagnostic with its span (`line:column-line:column`),
code, catalog identifier and parameters, and an indented line per label
and fix. It is the form the conformance suite compares, which no
rewording changes. `sisu explain CODE` prints the catalog's line for a
code or an identifier.

A diagnostic reports the first error of a chain only: when a name has an
unknown type, its uses are not reported again (D-065). The checker types
broken code, so the language server keeps working in it.

## Codes and their ranges

A code is a letter and four digits. The letter is the kind, and the
first digits the stage that reports it:

| Range | Reports |
|---|---|
| E00xx | lexing: characters, strings, brackets, indentation |
| E01xx | parsing: the syntax of statements and expressions |
| E02xx | names and declarations: scopes, imports, visibility, derives, traits and impls |
| E03xx | types: expressions, calls, operators, `match`, mutability, traits and generics |
| E04xx | ownership: moves, borrows, views, tasks, must-consume values |
| E05xx | lowering to SIR |
| E06xx | compile-time evaluation |
| E07xx | capabilities and `[trust]` |
| E08xx | contracts and refinement types |
| E09xx | stage 0's limits and internal errors |
| E10xx | packages, manifests, lockfiles and the module tree |
| W0xxx | warnings, which do not stop a program |
| R0xxx, R01xx | run time: panics, errors from `main`, test failures |
| D0xxx, D01xx | the driver, the `sisu` command itself |

An error stops `sisu run` before the program runs; a warning is printed
and the program runs. A run-time message is printed as the program
fails, without its code. The driver's messages report usage errors and
the results of commands.

## Lexing

Codes E0001 to E0018. The lexer reads the text into tokens and the
indentation into blocks. Its errors are about characters: tabs, unclosed
strings and brackets, escapes, number literals, and what Sisu does not
spell (`;`, a backslash at the end of a line).

### E0001

`lex.tab-in-indentation`: a tab in indentation; Sisu indents with spaces
only.

A line is indented with spaces only. A tab in the indentation is an
error wherever it stands, since its width is a matter of the editor.

<!-- file: examples/reference-diagnostics/e0001_tab.sisu -->
```sisu
# A tab in the indentation.
def main():
	print("Hei, maailma!")
```

<!-- output: examples/reference-diagnostics/e0001_tab.console -->
```console
$ sisu run e0001_tab.sisu
error[E0001]: a tab in indentation; Sisu indents with spaces only
  --> e0001_tab.sisu:3:1
  |
3 | 	print("Hei, maailma!")
  | ^
  = note: replace each tab with spaces

error[D0006]: the program was not run: errors found (1)
```

The note asks for spaces in place of each tab; `sisu fmt` cannot do it,
because it formats only files that parse.

<!-- file: examples/reference-diagnostics/e0001_tab_fixed.sisu -->
```sisu
# Indented with four spaces.
def main():
    print("Hei, maailma!")
```

<!-- output: examples/reference-diagnostics/e0001_tab_fixed.out -->
```text
Hei, maailma!
```

Taught in [chapter 1](../tutorial/01-hello.md).

### E0003

`lex.unterminated-string`: this string literal is not closed before the
end of the line.

A one-line string literal ends on the line where it starts. A multi-line
literal is written between `"""` marks.

<!-- file: examples/reference-diagnostics/e0003_open_string.sisu -->
```sisu
# A string literal that is not closed.
def main():
    print("Hei, maailma!)
```

<!-- output: examples/reference-diagnostics/e0003_open_string.console -->
```console
$ sisu run e0003_open_string.sisu
error[E0003]: this string literal is not closed before the end of the line
  --> e0003_open_string.sisu:3:11
  |
3 |     print("Hei, maailma!)
  |           ^^^^^^^^^^^^^^^
  = fix: close it on its line: `"`

error[D0006]: the program was not run: errors found (1)
```

The fix closes the literal at the end of the line, which is right when
the text ends there; here the quote belongs before the `)`.

<!-- file: examples/reference-diagnostics/e0003_open_string_fixed.sisu -->
```sisu
# The string literal closed on its line.
def main():
    print("Hei, maailma!")
```

<!-- output: examples/reference-diagnostics/e0003_open_string_fixed.out -->
```text
Hei, maailma!
```

Taught in [chapter 1](../tutorial/01-hello.md).

### E0016

`lex.bad-char-literal`: a character literal holds exactly one character.

Single quotes make a `char`, one character; text is written between
double quotes.

<!-- file: examples/reference-diagnostics/e0016_char_literal.sisu -->
```sisu
# Single quotes around a word.
def main():
    let name = 'Aino'
    print(name)
```

<!-- output: examples/reference-diagnostics/e0016_char_literal.console -->
```console
$ sisu run e0016_char_literal.sisu
error[E0016]: a character literal holds exactly one character
  --> e0016_char_literal.sisu:3:16
  |
3 |     let name = 'Aino'
  |                ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

The diagnostic offers no fix. Use double quotes for text, and keep
single quotes for one character, such as `'A'`.

<!-- file: examples/reference-diagnostics/e0016_char_literal_fixed.sisu -->
```sisu
# Text between double quotes.
def main():
    let name = "Aino"
    print(name)
```

<!-- output: examples/reference-diagnostics/e0016_char_literal_fixed.out -->
```text
Aino
```

Taught in [chapter 2](../tutorial/02-values.md).

## Parsing

Codes E0100 to E0126. The parser builds the syntax tree. Its errors name
what it expected and what it found, and several name a habit from
another language and the Sisu way to write it.

### E0100

`parse.expected`: expected {expected}, found {found}.

The parser expected a particular token and found another. The message
names both.

<!-- file: examples/reference-diagnostics/e0100_no_colon.sisu -->
```sisu
# A block opened without its colon.
def main()
    print("Hei, maailma!")
```

<!-- output: examples/reference-diagnostics/e0100_no_colon.console -->
```console
$ sisu run e0100_no_colon.sisu
error[E0100]: expected `:`, found the end of the line
  --> e0100_no_colon.sisu:2:11
  |
2 | def main()
  |           ^
  = fix: write the missing token: `:`

error[D0006]: the program was not run: errors found (1)
```

The fix writes the missing token where the parser expected it, the `:`
that opens the body here.

<!-- file: examples/reference-diagnostics/e0100_no_colon_fixed.sisu -->
```sisu
# The colon that opens the block.
def main():
    print("Hei, maailma!")
```

<!-- output: examples/reference-diagnostics/e0100_no_colon_fixed.out -->
```text
Hei, maailma!
```

Taught in [chapter 1](../tutorial/01-hello.md).

### E0103

`parse.expected-expression`: expected an expression, found {found}.

An expression was expected, and something else stood there: the end of
the line, a keyword, or a statement. A line ends a statement unless a
bracket is open.

<!-- file: examples/reference-diagnostics/e0103_no_operand.sisu -->
```sisu
# An operator without its right operand.
def main():
    let total = 3 +
    print(total)
```

<!-- output: examples/reference-diagnostics/e0103_no_operand.console -->
```console
$ sisu run e0103_no_operand.sisu
error[E0103]: expected an expression, found the end of the line
  --> e0103_no_operand.sisu:3:20
  |
3 |     let total = 3 +
  |                    ^

error[D0006]: the program was not run: errors found (1)
```

There is no fix to offer, since only you know the missing expression. To
break a long expression, break it inside brackets.

<!-- file: examples/reference-diagnostics/e0103_no_operand_fixed.sisu -->
```sisu
# Both operands, on one line.
def main():
    let total = 3 + 4
    print(total)
```

<!-- output: examples/reference-diagnostics/e0103_no_operand_fixed.out -->
```text
7
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0105

`parse.expected-pattern`: expected a pattern, found {found}.

A pattern was expected: a literal, a name, `_`, a variant, a tuple, a
list or a struct pattern, or alternatives with `|`. An operator such as
`not` is not a pattern.

<!-- file: examples/reference-diagnostics/e0105_pattern.sisu -->
```sisu
# `not` in a pattern.
def describe(n: int) -> str:
    return match n:
        case 0: "zero"
        case not 0: "other"

def main():
    print(describe(0))
```

<!-- output: examples/reference-diagnostics/e0105_pattern.console -->
```console
$ sisu run e0105_pattern.sisu
error[E0105]: expected a pattern, found `not`
  --> e0105_pattern.sisu:5:14
  |
5 |         case not 0: "other"
  |              ^^^

error[D0006]: the program was not run: errors found (1)
```

Write the case as a pattern; a `case _` arm, or a guard `case n if n !=
0`, says "any other value".

<!-- file: examples/reference-diagnostics/e0105_pattern_fixed.sisu -->
```sisu
# `_` matches every other value.
def describe(n: int) -> str:
    return match n:
        case 0: "zero"
        case _: "other"

def main():
    print(describe(0))
```

<!-- output: examples/reference-diagnostics/e0105_pattern_fixed.out -->
```text
zero
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0108

`parse.mixed-and-or`: `and` and `or` cannot be mixed without
parentheses.

`and` and `or` do not mix without parentheses, so that no reader has to
remember which binds tighter.

<!-- file: examples/reference-diagnostics/e0108_and_or.sisu -->
```sisu
# `and` and `or` mixed without parentheses.
def main():
    let raining = True
    let cold = False
    let windy = True
    if raining and cold or windy:
        print("stay in")
```

<!-- output: examples/reference-diagnostics/e0108_and_or.console -->
```console
$ sisu run e0108_and_or.sisu
error[E0108]: `and` and `or` cannot be mixed without parentheses
  --> e0108_and_or.sisu:6:25
  |
6 |     if raining and cold or windy:
  |                         ^^

error[D0006]: the program was not run: errors found (1)
```

Add the parentheses that say what you mean.

<!-- file: examples/reference-diagnostics/e0108_and_or_fixed.sisu -->
```sisu
# Parentheses say which goes first.
def main():
    let raining = True
    let cold = False
    let windy = True
    if (raining and cold) or windy:
        print("stay in")
```

<!-- output: examples/reference-diagnostics/e0108_and_or_fixed.out -->
```text
stay in
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0113

`parse.mixed-chain`: a chained comparison must point one way; `{first}`
and `{second}` do not.

A chain of comparisons such as `0 < x <= 5` reads as one range, so all
its operators point the same way.

<!-- file: examples/reference-diagnostics/e0113_chain.sisu -->
```sisu
# A chained comparison that points two ways.
def main():
    let size = 3
    print(0 < size > 5)
```

<!-- output: examples/reference-diagnostics/e0113_chain.console -->
```console
$ sisu run e0113_chain.sisu
error[E0113]: a chained comparison must point one way; `<` and `>` do not
  --> e0113_chain.sisu:4:20
  |
4 |     print(0 < size > 5)
  |                    ^

error[D0006]: the program was not run: errors found (1)
```

Write the chain one way, or join two comparisons with `and`.

<!-- file: examples/reference-diagnostics/e0113_chain_fixed.sisu -->
```sisu
# A chain that points one way.
def main():
    let size = 3
    print(0 < size < 5)
```

<!-- output: examples/reference-diagnostics/e0113_chain_fixed.out -->
```text
True
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0115

`parse.python-conditional`: `a if c else b` is not Sisu; write `if c: a
else: b`.

Sisu's conditional expression starts with `if`: `if c: a else: b`.
Python's `a if c else b` does not compile, so that it never means
something else (D-062).

<!-- file: examples/reference-diagnostics/e0115_python_if.sisu -->
```sisu
# Python's conditional expression.
def main():
    let size = 3
    let label = "big" if size > 2 else "small"
    print(label)
```

<!-- output: examples/reference-diagnostics/e0115_python_if.console -->
```console
$ sisu run e0115_python_if.sisu
error[E0115]: `a if c else b` is not Sisu; write `if c: a else: b`
  --> e0115_python_if.sisu:4:17
  |
4 |     let label = "big" if size > 2 else "small"
  |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: write it as `if c: a else: b`: `if size > 2: "big" else: "small"`

error[D0006]: the program was not run: errors found (1)
```

The fix rewrites the expression in Sisu's order, which is what you want.

<!-- file: examples/reference-diagnostics/e0115_python_if_fixed.sisu -->
```sisu
# Sisu's `if` expression.
def main():
    let size = 3
    let label = if size > 2: "big" else: "small"
    print(label)
```

<!-- output: examples/reference-diagnostics/e0115_python_if_fixed.out -->
```text
big
```

Taught in [chapter 3](../tutorial/03-functions.md).

### E0116

`parse.if-expression-else`: an `if` expression needs an `else` branch,
because it must have a value.

An `if` used as an expression has a value on every path, so it has an
`else` branch.

<!-- file: examples/reference-diagnostics/e0116_if_without_else.sisu -->
```sisu
# An `if` expression without an `else`.
def main():
    let size = 3
    let label = if size > 2: "big"
    print(label)
```

<!-- output: examples/reference-diagnostics/e0116_if_without_else.console -->
```console
$ sisu run e0116_if_without_else.sisu
error[E0116]: an `if` expression needs an `else` branch, because it must have a value
  --> e0116_if_without_else.sisu:4:35
  |
4 |     let label = if size > 2: "big"
  |                                   ^

error[D0006]: the program was not run: errors found (1)
```

Add the `else` branch with the value for the other case.

<!-- file: examples/reference-diagnostics/e0116_if_without_else_fixed.sisu -->
```sisu
# Both branches give a value.
def main():
    let size = 3
    let label = if size > 2: "big" else: "small"
    print(label)
```

<!-- output: examples/reference-diagnostics/e0116_if_without_else_fixed.out -->
```text
big
```

Taught in [chapter 3](../tutorial/03-functions.md).

## Names and declarations

Codes E0200 to E0266. Name resolution finds what each name means and
checks declarations: undefined and duplicate names, imports and
visibility, derives, traits and impls.

### E0200

`resolve.undefined-name`: cannot find `{name}` in this scope.

A name is used that nothing in scope declares: no binding, parameter,
item, import or prelude name. A binding is declared with `let` or `var`,
not by assigning to a new name.

<!-- file: examples/reference-diagnostics/e0200_misspelt.sisu -->
```sisu
# A misspelt name.
def main():
    prnt("Hei, maailma!")
```

<!-- output: examples/reference-diagnostics/e0200_misspelt.console -->
```console
$ sisu run e0200_misspelt.sisu
error[E0200]: cannot find `prnt` in this scope
  --> e0200_misspelt.sisu:3:5
  |
3 |     prnt("Hei, maailma!")
  |     ^^^^
  = fix: a similar name is in scope: `print`

error[D0006]: the program was not run: errors found (1)
```

When a name in scope is spelt alike, the fix offers it. When the name is
new, declare it with `let` (or `var`) before its first use.

<!-- file: examples/reference-diagnostics/e0200_misspelt_fixed.sisu -->
```sisu
# The name as it is declared.
def main():
    print("Hei, maailma!")
```

<!-- output: examples/reference-diagnostics/e0200_misspelt_fixed.out -->
```text
Hei, maailma!
```

Taught in [chapter 1](../tutorial/01-hello.md).

### E0202

`resolve.duplicate-definition`: `{name}` is defined more than once in
this module.

A module declares each name once. Sisu has no overloading (D-050), so
two functions with one name are an error even when their parameters
differ.

<!-- file: examples/reference-diagnostics/e0202_twice.sisu -->
```sisu
# Two functions with one name.
def area(width: int) -> int:
    return width * width

def area(width: int) -> int:
    return width * 2

def main():
    print(area(3))
```

<!-- output: examples/reference-diagnostics/e0202_twice.console -->
```console
$ sisu run e0202_twice.sisu
error[E0202]: `area` is defined more than once in this module
  --> e0202_twice.sisu:5:5
  |
5 | def area(width: int) -> int:
  |     ^^^^
  |
2 | def area(width: int) -> int:
  |     ---- first defined here
  = note: rename one of them, or remove the one not wanted

error[D0006]: the program was not run: errors found (1)
```

The note asks you to rename one or remove the one not wanted; a name
that says what each computes is the usual way.

<!-- file: examples/reference-diagnostics/e0202_twice_fixed.sisu -->
```sisu
# Each function with a name of its own.
def square_area(width: int) -> int:
    return width * width

def strip_area(width: int) -> int:
    return width * 2

def main():
    print(square_area(3))
```

<!-- output: examples/reference-diagnostics/e0202_twice_fixed.out -->
```text
9
```

Taught in [chapter 3](../tutorial/03-functions.md).

### E0211

`resolve.ambiguous-variant`: `{name}` is a variant of more than one type
({candidates}); qualify it, as in `{example}`.

A variant's own name is enough where only one enum has a variant of that
name. Where several have, the bare name is ambiguous.

<!-- file: examples/reference-diagnostics/e0211_ambiguous_variant.sisu -->
```sisu
# A variant name that two enums share.
enum Light:
    Red
    Green

enum Wine:
    Red
    White

def main():
    let colour = Red
    print("{colour:?}")
```

<!-- output: examples/reference-diagnostics/e0211_ambiguous_variant.console -->
```console
$ sisu run e0211_ambiguous_variant.sisu
error[E0211]: `Red` is a variant of more than one type (Light, Wine); qualify it, as in `Light.Red`
  --> e0211_ambiguous_variant.sisu:11:18
   |
11 |     let colour = Red
   |                  ^^^

error[D0006]: the program was not run: errors found (1)
```

Qualify the variant with its type, as the message shows, or let a known
type decide it: a parameter of type `Light` takes `Red`.

<!-- file: examples/reference-diagnostics/e0211_ambiguous_variant_fixed.sisu -->
```sisu
# The variant named with its type.
enum Light:
    Red
    Green

enum Wine:
    Red
    White

def main():
    let colour = Light.Red
    print("{colour:?}")
```

<!-- output: examples/reference-diagnostics/e0211_ambiguous_variant_fixed.out -->
```text
Red
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0244

`resolve.derive-field`: `{ty}` cannot derive `{trait}`: its field
`{field}` has type `{fty}`, which does not implement `{trait}`.

A derive needs every field to have the trait it derives. A `String` is
not `Copy`, since copying it takes new memory, which Sisu does only when
asked.

<!-- file: examples/reference-diagnostics/e0244_derive_copy.sisu -->
```sisu
# `Copy` for a struct whose field is not `Copy`.
@derive(Copy)
struct Person:
    name: String

def main():
    let aino = Person("Aino")
    print(aino.name)
```

<!-- output: examples/reference-diagnostics/e0244_derive_copy.console -->
```console
$ sisu run e0244_derive_copy.sisu
error[E0244]: `Person` cannot derive `Copy`: its field `name` has type `String`, which does not implement `Copy`
  --> e0244_derive_copy.sisu:2:9
  |
2 | @derive(Copy)
  |         ^^^^

error[D0006]: the program was not run: errors found (1)
```

Leave `Copy` out and copy with `.copy()` where a copy is wanted, or make
the field a type that is `Copy`.

<!-- file: examples/reference-diagnostics/e0244_derive_copy_fixed.sisu -->
```sisu
# Without `Copy`; `.copy()` copies on request.
struct Person:
    name: String

def main():
    let aino = Person("Aino")
    print(aino.name)
```

<!-- output: examples/reference-diagnostics/e0244_derive_copy_fixed.out -->
```text
Aino
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

## Types

Codes E0300 to E0399. The type checker checks every expression's type:
mismatches, calls, operators, `match`, mutability, indexing, traits and
generics.

### E0300

`type.mismatch`: expected `{expected}`, found `{found}`.

A value of one type stands where another is expected. A label points at
what made the other type expected.

<!-- file: examples/reference-diagnostics/e0300_mismatch.sisu -->
```sisu
# A text where a number is expected.
def main():
    let count: int = "3"
    print(count)
```

<!-- output: examples/reference-diagnostics/e0300_mismatch.console -->
```console
$ sisu run e0300_mismatch.sisu
error[E0300]: expected `int`, found `str`
  --> e0300_mismatch.sisu:3:22
  |
3 |     let count: int = "3"
  |                      ^^^
  |                --- expected because of this
  = fix: write the number without quotes: `3`

error[D0006]: the program was not run: errors found (1)
```

The fix depends on the types; here it writes the number without its
quotes. Where text must become a number, `parse_int()` converts it and
says when it cannot.

<!-- file: examples/reference-diagnostics/e0300_mismatch_fixed.sisu -->
```sisu
# A number.
def main():
    let count: int = 3
    print(count)
```

<!-- output: examples/reference-diagnostics/e0300_mismatch_fixed.out -->
```text
3
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0301

`type.integer-division`: `/` divides floats; for integers use `//`, or
convert to `f64` first.

`/` divides floating-point numbers; integers are divided with `//`,
which rounds towards negative infinity, and `%` gives the remainder.

<!-- file: examples/reference-diagnostics/e0301_division.sisu -->
```sisu
# `/` between two integers.
def main():
    let pieces = 7
    print(pieces / 2)
```

<!-- output: examples/reference-diagnostics/e0301_division.console -->
```console
$ sisu run e0301_division.sisu
error[E0301]: `/` divides floats; for integers use `//`, or convert to `f64` first
  --> e0301_division.sisu:4:11
  |
4 |     print(pieces / 2)
  |           ^^^^^^^^^^
  = fix: use `//` for integer division: `//`

error[D0006]: the program was not run: errors found (1)
```

The fix writes `//`. To divide with a fraction, convert first:
`pieces.to_f64() / 2.0`.

<!-- file: examples/reference-diagnostics/e0301_division_fixed.sisu -->
```sisu
# `//` divides integers.
def main():
    let pieces = 7
    print(pieces // 2)
```

<!-- output: examples/reference-diagnostics/e0301_division_fixed.out -->
```text
3
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0302

`type.not-bool`: a condition must be a `bool`, not `{found}`; Sisu has
no truthiness.

A condition is a `bool`. Sisu has no truthiness: a number, a text or a
collection is not true or false by itself (D-025).

<!-- file: examples/reference-diagnostics/e0302_truthiness.sisu -->
```sisu
# A number used as a condition.
def main():
    let count = 3
    if count:
        print("some")
```

<!-- output: examples/reference-diagnostics/e0302_truthiness.console -->
```console
$ sisu run e0302_truthiness.sisu
error[E0302]: a condition must be a `bool`, not `int`; Sisu has no truthiness
  --> e0302_truthiness.sisu:4:8
  |
4 |     if count:
  |        ^^^^^
  = note: write the test explicitly, as in `not xs.is_empty()` or `x != 0`

error[D0006]: the program was not run: errors found (1)
```

Write the test you mean, such as `count != 0` or `not names.is_empty()`.

<!-- file: examples/reference-diagnostics/e0302_truthiness_fixed.sisu -->
```sisu
# The test written out.
def main():
    let count = 3
    if count != 0:
        print("some")
```

<!-- output: examples/reference-diagnostics/e0302_truthiness_fixed.out -->
```text
some
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0305

`type.argument-count`: wrong number of arguments to `{name}`: expected
{expected}, found {found}.

A call passes as many arguments as the function has parameters, less
those with defaults. `print` takes one text.

<!-- file: examples/reference-diagnostics/e0305_argument_count.sisu -->
```sisu
# Two arguments to a function that takes one.
def main():
    print("Hei,", "maailma!")
```

<!-- output: examples/reference-diagnostics/e0305_argument_count.console -->
```console
$ sisu run e0305_argument_count.sisu
error[E0305]: wrong number of arguments to `print`: expected 1, found 2
  --> e0305_argument_count.sisu:3:5
  |
3 |     print("Hei,", "maailma!")
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

There is no fix to offer. For `print`, build one text with
interpolation.

<!-- file: examples/reference-diagnostics/e0305_argument_count_fixed.sisu -->
```sisu
# One text, built by interpolation.
def main():
    let world = "maailma"
    print("Hei, {world}!")
```

<!-- output: examples/reference-diagnostics/e0305_argument_count_fixed.out -->
```text
Hei, maailma!
```

Taught in [chapter 1](../tutorial/01-hello.md).

### E0308

`type.missing-argument`: missing the argument `{arg}` of `{name}`.

A parameter without a default needs an argument at every call.

<!-- file: examples/reference-diagnostics/e0308_missing_argument.sisu -->
```sisu
# A call without one of its arguments.
def area(width: int, height: int) -> int:
    return width * height

def main():
    print(area(3))
```

<!-- output: examples/reference-diagnostics/e0308_missing_argument.console -->
```console
$ sisu run e0308_missing_argument.sisu
error[E0308]: missing the argument `height` of `area`
  --> e0308_missing_argument.sisu:6:11
  |
6 |     print(area(3))
  |           ^^^^^^^
  = fix: pass `height` as a hole to fill: `, todo("height")`

error[D0006]: the program was not run: errors found (1)
```

The fix passes a typed hole, `todo("height")`, which compiles and stops
the program when it is reached; fill it with the value.

<!-- file: examples/reference-diagnostics/e0308_missing_argument_fixed.sisu -->
```sisu
# Every argument given.
def area(width: int, height: int) -> int:
    return width * height

def main():
    print(area(3, 4))
```

<!-- output: examples/reference-diagnostics/e0308_missing_argument_fixed.out -->
```text
12
```

Taught in [chapter 5](../tutorial/05-collections.md).

### E0310

`type.binary-operator`: `{op}` cannot be applied to `{lhs}` and `{rhs}`.

A binary operator is defined for some pairs of types only. `+` joins two
texts, and adds two numbers of one type.

<!-- file: examples/reference-diagnostics/e0310_operator.sisu -->
```sisu
# `+` between a text and a number.
def main():
    let age = 3
    print("age " + age)
```

<!-- output: examples/reference-diagnostics/e0310_operator.console -->
```console
$ sisu run e0310_operator.sisu
error[E0310]: `+` cannot be applied to `str` and `int`
  --> e0310_operator.sisu:4:11
  |
4 |     print("age " + age)
  |           ^^^^^^^^^^^^
  = fix: write the value into the text: `"age {age}"`

error[D0006]: the program was not run: errors found (1)
```

For a text and a value, the fix writes the value into the text by
interpolation.

<!-- file: examples/reference-diagnostics/e0310_operator_fixed.sisu -->
```sisu
# The number interpolated into the text.
def main():
    let age = 3
    print("age {age}")
```

<!-- output: examples/reference-diagnostics/e0310_operator_fixed.out -->
```text
age 3
```

Taught in [chapter 8](../tutorial/08-errors.md).

### E0312

`type.assign-immutable`: `{name}` is immutable; declare it with `var` to
assign to it.

A `let` binding keeps its value; only a `var` binding, a `mut` parameter
or a field of one can be assigned.

<!-- file: examples/reference-diagnostics/e0312_assign_let.sisu -->
```sisu
# An assignment to a `let` binding.
def main():
    let count = 1
    count = 2
    print(count)
```

<!-- output: examples/reference-diagnostics/e0312_assign_let.console -->
```console
$ sisu run e0312_assign_let.sisu
error[E0312]: `count` is immutable; declare it with `var` to assign to it
  --> e0312_assign_let.sisu:4:5
  |
4 |     count = 2
  |     ^^^^^
  |
3 |     let count = 1
  |         ----- `count` is defined here
  = fix: declare `count` with `var`

error[D0006]: the program was not run: errors found (1)
```

The fix declares the binding with `var`. For a parameter, it makes the
parameter `mut`, which changes every call; a local `var` copy of it is
often what you want instead.

<!-- file: examples/reference-diagnostics/e0312_assign_let_fixed.sisu -->
```sisu
# A `var` binding can be assigned.
def main():
    var count = 1
    count = 2
    print(count)
```

<!-- output: examples/reference-diagnostics/e0312_assign_let_fixed.out -->
```text
2
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0314

`type.non-exhaustive`: this `match` does not cover {missing}.

A `match` covers every value of its subject's type, so that adding a
variant shows every `match` that must handle it.

<!-- file: examples/reference-diagnostics/e0314_non_exhaustive.sisu -->
```sisu
# A `match` that misses a variant.
enum Light:
    Red
    Amber
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case Green: "go"

def main():
    print(action(Amber))
```

<!-- output: examples/reference-diagnostics/e0314_non_exhaustive.console -->
```console
$ sisu run e0314_non_exhaustive.sisu
error[E0314]: this `match` does not cover `Amber`
  --> e0314_non_exhaustive.sisu:8:12
  |
8 |     return match light:
  |            ^^^^^^^^^^^
  = note: add an arm for `Amber`, or a `case _` arm

error[D0006]: the program was not run: errors found (1)
```

Add an arm for the case the note names. A `case _` arm also covers it,
and every variant added later.

<!-- file: examples/reference-diagnostics/e0314_non_exhaustive_fixed.sisu -->
```sisu
# An arm for every variant.
enum Light:
    Red
    Amber
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case Amber: "wait"
        case Green: "go"

def main():
    print(action(Amber))
```

<!-- output: examples/reference-diagnostics/e0314_non_exhaustive_fixed.out -->
```text
wait
```

Taught in [chapter 4](../tutorial/04-control-flow.md).

### E0315

`type.missing-return`: `{name}` returns `{ty}`, but control can reach
the end of its body without a `return`.

A function with a result type returns a value on every path; a value
leaves a function only through `return` (D-056).

<!-- file: examples/reference-diagnostics/e0315_missing_return.sisu -->
```sisu
# A path that falls off the end of the body.
def sign(n: int) -> int:
    if n < 0:
        return -1
    elif n > 0:
        return 1

def main():
    print(sign(5))
```

<!-- output: examples/reference-diagnostics/e0315_missing_return.console -->
```console
$ sisu run e0315_missing_return.sisu
error[E0315]: `sign` returns `int`, but control can reach the end of its body without a `return`
  --> e0315_missing_return.sisu:2:5
  |
2 | def sign(n: int) -> int:
  |     ^^^^
  = note: return a value on that path too, or end the body with a `return`

error[D0006]: the program was not run: errors found (1)
```

Return a value on the path that has none, often with a last `return` at
the end of the body.

<!-- file: examples/reference-diagnostics/e0315_missing_return_fixed.sisu -->
```sisu
# A `return` on every path.
def sign(n: int) -> int:
    if n < 0:
        return -1
    elif n > 0:
        return 1
    return 0

def main():
    print(sign(5))
```

<!-- output: examples/reference-diagnostics/e0315_missing_return_fixed.out -->
```text
1
```

Taught in [chapter 3](../tutorial/03-functions.md).

### E0318

`type.unused-result`: this `{ty}` is not used; handle it, or discard it
with `_ = ..`.

A value that can hold an error or an absence must be used, so that no
failure passes unseen.

<!-- file: examples/reference-diagnostics/e0318_unused_result.sisu -->
```sisu
# A result that nothing uses.
def main():
    "42".parse_int()
    print("done")
```

<!-- output: examples/reference-diagnostics/e0318_unused_result.console -->
```console
$ sisu run e0318_unused_result.sisu
error[E0318]: this `int?` is not used; handle it, or discard it with `_ = ...`
  --> e0318_unused_result.sisu:3:5
  |
3 |     "42".parse_int()
  |     ^^^^^^^^^^^^^^^^
  = fix: discard it explicitly: `_ = `

error[D0006]: the program was not run: errors found (1)
```

Handle it, with `?`, `else` or `match`, or discard it on purpose with `_
= ...`, which the fix writes.

<!-- file: examples/reference-diagnostics/e0318_unused_result_fixed.sisu -->
```sisu
# The result used.
def main():
    let n = "42".parse_int() else: 0
    print("done with {n}")
```

<!-- output: examples/reference-diagnostics/e0318_unused_result_fixed.out -->
```text
done with 42
```

Taught in [chapter 8](../tutorial/08-errors.md).

### E0320

`type.try-context`: `?` on `{ty}` needs a function that returns
`{needed}`, but this one returns `{found}`.

`?` returns early from the function with the absence or the error, so
the function's result must be able to hold it: `T?` for an absence, `T !
E` for an error.

<!-- file: examples/reference-diagnostics/e0320_try_context.sisu -->
```sisu
# `?` in a function that returns nothing.
def main():
    let n = "42".parse_int()?
    print(n)
```

<!-- output: examples/reference-diagnostics/e0320_try_context.console -->
```console
$ sisu run e0320_try_context.sisu
error[E0320]: `?` on `int?` needs a function that returns `T?`, but this one returns `()`
  --> e0320_try_context.sisu:3:13
  |
3 |     let n = "42".parse_int()?
  |             ^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Give the function a result type that can hold it, as `def main() -> ! E`
for an error, or handle the value where it is, with `else`.

<!-- file: examples/reference-diagnostics/e0320_try_context_fixed.sisu -->
```sisu
# A fallback where there is no caller to return to.
def main():
    let n = "42".parse_int() else: 0
    print(n)
```

<!-- output: examples/reference-diagnostics/e0320_try_context_fixed.out -->
```text
42
```

Taught in [chapter 8](../tutorial/08-errors.md).

### E0325

`type.not-iterable`: a `for` loop cannot iterate over `{ty}`.

A `for` loop takes a value that can be iterated: a collection, a range
or an iterator. A text is not, because it can be read as characters,
bytes or lines.

<!-- file: examples/reference-diagnostics/e0325_not_iterable.sisu -->
```sisu
# A `for` loop over a text.
def main():
    for letter in "hei":
        print(letter)
```

<!-- output: examples/reference-diagnostics/e0325_not_iterable.console -->
```console
$ sisu run e0325_not_iterable.sisu
error[E0325]: a `for` loop cannot iterate over `str`
  --> e0325_not_iterable.sisu:3:19
  |
3 |     for letter in "hei":
  |                   ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Say which: `.chars()`, `.bytes()` or `.lines()`.

<!-- file: examples/reference-diagnostics/e0325_not_iterable_fixed.sisu -->
```sisu
# A loop over the text's characters.
def main():
    for letter in "hei".chars():
        print(letter)
```

<!-- output: examples/reference-diagnostics/e0325_not_iterable_fixed.out -->
```text
h
e
i
```

Taught in [chapter 6](../tutorial/06-text.md).

### E0327

`type.unknown-variant`: `{ty}` has no variant `{name}`.

A capitalised name in a pattern is a variant of the subject's type, and
that type has none of that name.

<!-- file: examples/reference-diagnostics/e0327_unknown_variant.sisu -->
```sisu
# A pattern that names a variant the enum does not have.
enum Light:
    Red
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case Blue: "wait"
        case _: "go"

def main():
    print(action(Red))
```

<!-- output: examples/reference-diagnostics/e0327_unknown_variant.console -->
```console
$ sisu run e0327_unknown_variant.sisu
error[E0327]: `Light` has no variant `Blue`
  --> e0327_unknown_variant.sisu:9:14
  |
9 |         case Blue: "wait"
  |              ^^^^

error[D0006]: the program was not run: errors found (1)
```

Name a variant the type has, or add it to the enum. A lowercase name in
a pattern binds the value instead.

<!-- file: examples/reference-diagnostics/e0327_unknown_variant_fixed.sisu -->
```sisu
# Only the enum's own variants.
enum Light:
    Red
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"
        case _: "go"

def main():
    print(action(Red))
```

<!-- output: examples/reference-diagnostics/e0327_unknown_variant_fixed.out -->
```text
stop
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0329

`type.refutable-let`: this pattern might not match; add `else:` with a
block that leaves the scope.

The pattern of a plain `let` matches every value. A pattern that might
not match needs `else:` with a block that leaves the scope.

<!-- file: examples/reference-diagnostics/e0329_refutable_let.sisu -->
```sisu
# A `let` whose pattern might not match.
def main():
    let maybe: int? = Some(3)
    let Some(n) = maybe
    print(n)
```

<!-- output: examples/reference-diagnostics/e0329_refutable_let.console -->
```console
$ sisu run e0329_refutable_let.sisu
error[E0329]: this pattern might not match; add `else:` with a block that leaves the scope
  --> e0329_refutable_let.sisu:4:9
  |
4 |     let Some(n) = maybe
  |         ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Add `else:` with a `return`, `break`, `continue` or a panic, or use `if
let` or `match`.

<!-- file: examples/reference-diagnostics/e0329_refutable_let_fixed.sisu -->
```sisu
# `else` says what happens when it does not match.
def main():
    let maybe: int? = Some(3)
    let Some(n) = maybe else:
        return
    print(n)
```

<!-- output: examples/reference-diagnostics/e0329_refutable_let_fixed.out -->
```text
3
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0337

`type.missing-field`: missing the field `{field}` of `{ty}`.

Building a struct gives a value to every field that has no default.

<!-- file: examples/reference-diagnostics/e0337_missing_field.sisu -->
```sisu
# A struct built without one of its fields.
struct Point:
    x: int
    y: int

def main():
    let p = Point(x=1)
    print(p.x)
```

<!-- output: examples/reference-diagnostics/e0337_missing_field.console -->
```console
$ sisu run e0337_missing_field.sisu
error[E0337]: missing the field `y` of `Point`
  --> e0337_missing_field.sisu:7:13
  |
7 |     let p = Point(x=1)
  |             ^^^^^^^^^^
  = fix: pass `y` as a hole to fill: `, y=todo("y")`

error[D0006]: the program was not run: errors found (1)
```

The fix passes a typed hole for the field; give the value, or give the
field a default in the struct.

<!-- file: examples/reference-diagnostics/e0337_missing_field_fixed.sisu -->
```sisu
# Every field given.
struct Point:
    x: int
    y: int

def main():
    let p = Point(x=1, y=2)
    print(p.x)
```

<!-- output: examples/reference-diagnostics/e0337_missing_field_fixed.out -->
```text
1
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0341

`type.not-mutable`: cannot modify `{what}`; it is not declared with
`var`.

A method that changes its value (`mut self`), or a change to a field,
needs a value that can change: a `var` binding, a `mut` parameter, or a
`mut` borrow.

<!-- file: examples/reference-diagnostics/e0341_not_mutable.sisu -->
```sisu
# A change to a `let` binding's list.
def main():
    let names = ["Aino"]
    names.push("Eero")
    print(names)
```

<!-- output: examples/reference-diagnostics/e0341_not_mutable.console -->
```console
$ sisu run e0341_not_mutable.sisu
error[E0341]: cannot modify `names`; it is not declared with `var`
  --> e0341_not_mutable.sisu:4:5
  |
4 |     names.push("Eero")
  |     ^^^^^
  |
3 |     let names = ["Aino"]
  |         ----- `names` is defined here
  = fix: declare `names` with `var`

error[D0006]: the program was not run: errors found (1)
```

The fix declares the binding with `var`. For a method of your own that
changes `self`, the first parameter is `mut self`.

<!-- file: examples/reference-diagnostics/e0341_not_mutable_fixed.sisu -->
```sisu
# A `var` binding can change.
def main():
    var names = ["Aino"]
    names.push("Eero")
    print(names)
```

<!-- output: examples/reference-diagnostics/e0341_not_mutable_fixed.out -->
```text
["Aino", "Eero"]
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0344

`type.constant-negative-index`: a constant negative index does not
compile; count from the end with `^`: `{fix}`.

Sisu counts from the end with `^`: `xs[^1]` is the last element. A
constant negative index does not compile, so that Python's `xs[-1]`
never means something else (D-062).

<!-- file: examples/reference-diagnostics/e0344_negative_index.sisu -->
```sisu
# Python's last element.
def main():
    let names = ["Aino", "Eero"]
    print(names[-1])
```

<!-- output: examples/reference-diagnostics/e0344_negative_index.console -->
```console
$ sisu run e0344_negative_index.sisu
error[E0344]: a constant negative index does not compile; count from the end with `^`: `names[^1]`
  --> e0344_negative_index.sisu:4:17
  |
4 |     print(names[-1])
  |                 ^^
  = fix: count from the end with `^`: `^1`

error[D0006]: the program was not run: errors found (1)
```

The fix writes `^1`; an index computed at run time that is negative
panics instead.

<!-- file: examples/reference-diagnostics/e0344_negative_index_fixed.sisu -->
```sisu
# `^1` counts from the end.
def main():
    let names = ["Aino", "Eero"]
    print(names[^1])
```

<!-- output: examples/reference-diagnostics/e0344_negative_index_fixed.out -->
```text
Eero
```

Taught in [chapter 5](../tutorial/05-collections.md).

### E0345

`type.str-not-indexable`: `{ty}` cannot be indexed by position; use
`.chars()`, `.bytes()` or `.byte_slice(a, b)`.

A `str` has no integer index, since its characters take different
numbers of bytes (D-034).

<!-- file: examples/reference-diagnostics/e0345_str_index.sisu -->
```sisu
# A text indexed by position.
def main():
    let word = "hei"
    print(word[0])
```

<!-- output: examples/reference-diagnostics/e0345_str_index.console -->
```console
$ sisu run e0345_str_index.sisu
error[E0345]: `str` cannot be indexed by position; use `.chars()`, `.bytes()` or `.byte_slice(a, b)`
  --> e0345_str_index.sisu:4:11
  |
4 |     print(word[0])
  |           ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Iterate its `.chars()` or `.bytes()`, or take a part with
`.byte_slice(a, b)` at known character boundaries.

<!-- file: examples/reference-diagnostics/e0345_str_index_fixed.sisu -->
```sisu
# The first character, from the characters.
def main():
    let word = "hei"
    let first = word.chars().next() else: ' '
    print(first)
```

<!-- output: examples/reference-diagnostics/e0345_str_index_fixed.out -->
```text
h
```

Taught in [chapter 6](../tutorial/06-text.md).

### E0347

`type.bound-not-satisfied`: `{ty}` does not implement `{bound}`.

A type is used where a trait is required, and does not implement it.
`==` needs `Eq`, sorting needs `Ord`, a set or a dict key needs `Hash`.

<!-- file: examples/reference-diagnostics/e0347_bound.sisu -->
```sisu
# `==` on a struct that does not implement `Eq`.
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) == Point(1, 2))
```

<!-- output: examples/reference-diagnostics/e0347_bound.console -->
```console
$ sisu run e0347_bound.sisu
error[E0347]: `Point` does not implement `Eq`
  --> e0347_bound.sisu:7:11
  |
7 |     print(Point(1, 2) == Point(1, 2))
  |           ^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Derive the trait with `@derive(...)` on the type, or write an `impl` of
it.

<!-- file: examples/reference-diagnostics/e0347_bound_fixed.sisu -->
```sisu
# `@derive(Eq)` gives the struct `==`.
@derive(Eq)
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) == Point(1, 2))
```

<!-- output: examples/reference-diagnostics/e0347_bound_fixed.out -->
```text
True
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0368

`type.not-ordered`: `{ty}` has no order, so it cannot be compared with
`{op}` or sorted.

`<`, `<=`, `>`, `>=`, `sorted`, `min` and `max` need an order, the trait
`Ord`.

<!-- file: examples/reference-diagnostics/e0368_not_ordered.sisu -->
```sisu
# `<` on a struct that has no order.
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) < Point(2, 1))
```

<!-- output: examples/reference-diagnostics/e0368_not_ordered.console -->
```console
$ sisu run e0368_not_ordered.sisu
error[E0368]: `Point` has no order, so it cannot be compared with `<` or sorted
  --> e0368_not_ordered.sisu:7:11
  |
7 |     print(Point(1, 2) < Point(2, 1))
  |           ^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Derive `Ord` (with `Eq`), which compares the fields in the order they
are declared, or write an `impl`.

<!-- file: examples/reference-diagnostics/e0368_not_ordered_fixed.sisu -->
```sisu
# `@derive(Eq, Ord)` orders the struct field by field.
@derive(Eq, Ord)
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) < Point(2, 1))
```

<!-- output: examples/reference-diagnostics/e0368_not_ordered_fixed.out -->
```text
True
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0373

`type.mixed-numbers`: `{op}` cannot mix `{lhs}` and `{rhs}`; convert one
of them explicitly.

An arithmetic operator takes two numbers of one type; Sisu converts
between `int` and `f64` only when you say so.

<!-- file: examples/reference-diagnostics/e0373_mixed_numbers.sisu -->
```sisu
# An integer and a float in one operation.
def main():
    let price = 3
    print(price * 1.5)
```

<!-- output: examples/reference-diagnostics/e0373_mixed_numbers.console -->
```console
$ sisu run e0373_mixed_numbers.sisu
error[E0373]: `*` cannot mix `int` and `f64`; convert one of them explicitly
  --> e0373_mixed_numbers.sisu:4:11
  |
4 |     print(price * 1.5)
  |           ^^^^^^^^^^^
  = fix: make the `int` an `f64`: `.to_f64()`

error[D0006]: the program was not run: errors found (1)
```

The fix converts the integer with `.to_f64()`; to go the other way,
`round()` or `truncate_to_int()` says how.

<!-- file: examples/reference-diagnostics/e0373_mixed_numbers_fixed.sisu -->
```sisu
# The integer converted first.
def main():
    let price = 3
    print(price.to_f64() * 1.5)
```

<!-- output: examples/reference-diagnostics/e0373_mixed_numbers_fixed.out -->
```text
4.5
```

Taught in [chapter 2](../tutorial/02-values.md).

### E0374

`type.literal-out-of-range`: `{text}` does not fit in `{ty}`.

An integer literal fits in the type it gets. The note gives the type's
range.

<!-- file: examples/reference-diagnostics/e0374_out_of_range.sisu -->
```sisu
# A literal too large for its type.
def main():
    let level: u8 = 300
    print(level)
```

<!-- output: examples/reference-diagnostics/e0374_out_of_range.console -->
```console
$ sisu run e0374_out_of_range.sisu
error[E0374]: `300` does not fit in `u8`
  --> e0374_out_of_range.sisu:3:21
  |
3 |     let level: u8 = 300
  |                     ^^^
  = note: `u8` holds 0 to 255

error[D0006]: the program was not run: errors found (1)
```

Use a value in the range, or a wider type such as `int`.

<!-- file: examples/reference-diagnostics/e0374_out_of_range_fixed.sisu -->
```sisu
# A value the type holds.
def main():
    let level: u8 = 250
    print(level)
```

<!-- output: examples/reference-diagnostics/e0374_out_of_range_fixed.out -->
```text
250
```

Taught in [chapter 2](../tutorial/02-values.md).

## Ownership

Codes E0400 to E0437. The ownership checker checks moves, borrows,
views, tasks and must-consume values ([R7](../README.md#the-reference),
when it is written).

### E0400

`borrow.use-after-move`: `{name}` was moved on line {line} and cannot be
used afterwards.

A value has one owner. Assigning a value that is not `Copy` moves it,
and the old name can no longer be used.

<!-- file: examples/reference-diagnostics/e0400_moved.sisu -->
```sisu
# A list used after it moved.
def main():
    let names = ["Aino"]
    let team = names
    print(names)
    print(team)
```

<!-- output: examples/reference-diagnostics/e0400_moved.console -->
```console
$ sisu run e0400_moved.sisu
error[E0400]: `names` was moved on line 4 and cannot be used afterwards
  --> e0400_moved.sisu:5:11
  |
5 |     print(names)
  |           ^^^^^
  |
4 |     let team = names
  |                ----- `names` is moved here
  = note: copy it with `.copy()` before it is moved, or move it later

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

The note offers two ways: copy the value before the move, which costs a
copy, or use the old name before the move. Reading through a borrow
instead of moving is the third.

<!-- file: examples/reference-diagnostics/e0400_moved_fixed.sisu -->
```sisu
# A copy for the second owner.
def main():
    let names = ["Aino"]
    let team = names.copy()
    print(names)
    print(team)
```

<!-- output: examples/reference-diagnostics/e0400_moved_fixed.out -->
```text
["Aino"]
["Aino"]
```

Taught in [chapter 5](../tutorial/05-collections.md).

### E0411

`borrow.view-field-in-plain-struct`: `{ty}` holds `{field}: {fty}`,
which borrows; only a `view struct` may hold a borrow.

A plain struct owns all its fields; a field that borrows, such as `str`,
could outlive what it borrows. Only a `view struct` may hold a borrow.

<!-- file: examples/reference-diagnostics/e0411_view_field.sisu -->
```sisu
# A borrowed text in a plain struct.
struct Order:
    item: str

def main():
    let order = Order("bread")
    print(order.item)
```

<!-- output: examples/reference-diagnostics/e0411_view_field.console -->
```console
$ sisu run e0411_view_field.sisu
error[E0411]: `Order` holds `item: str`, which borrows; only a `view struct` may hold a borrow
  --> e0411_view_field.sisu:3:5
  |
3 |     item: str
  |     ^^^^^^^^^
  |
2 | struct Order:
  |        ----- `Order` is declared here
  = fix: declare it `view struct Order`: `view `

error[D0006]: the program was not run: errors found (1)
```

The fix makes the struct a `view struct`, which limits where its values
can go. Usually the field should own its value: `String` for `str`.

<!-- file: examples/reference-diagnostics/e0411_view_field_fixed.sisu -->
```sisu
# The struct owns its text.
struct Order:
    item: String

def main():
    let order = Order("bread")
    print(order.item)
```

<!-- output: examples/reference-diagnostics/e0411_view_field_fixed.out -->
```text
bread
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

## Warnings

Codes W0001 to W0014. A warning does not stop a program: `sisu run`
prints it and runs.

### W0001

`type.hole`: typed hole `{name}` of type `{ty}`.

A typed hole, `todo("name")`, stands for code still to be written
(D-061). The program compiles and runs until it reaches the hole, which
panics; `sisu test` reports a test that reaches one as blocked.

<!-- file: examples/reference-diagnostics/w0001_hole.sisu -->
```sisu
# A typed hole in a function's body.
def double(x: int) -> int:
    return todo("double")

def main():
    print("ready")
```

<!-- output: examples/reference-diagnostics/w0001_hole.console -->
```console
$ sisu run w0001_hole.sisu
warning[W0001]: typed hole `double` of type `int`
  --> w0001_hole.sisu:3:12
  |
3 |     return todo("double")
  |            ^^^^^^^^^^^^^^

ready
```

Write the code the hole stands for.

<!-- file: examples/reference-diagnostics/w0001_hole_fixed.sisu -->
```sisu
# The hole filled.
def double(x: int) -> int:
    return x * 2

def main():
    print("ready, {double(21)}")
```

<!-- output: examples/reference-diagnostics/w0001_hole_fixed.out -->
```text
ready, 42
```

Taught in [chapter 1](../tutorial/01-hello.md).

### W0002

`resolve.shadow-outer`: `{name}` shadows a binding of an enclosing
scope.

A binding in an inner block with the name of a binding of an enclosing
block hides it until the inner block ends (D-024). Rebinding in the same
block does not warn.

<!-- file: examples/reference-diagnostics/w0002_shadow.sisu -->
```sisu
# An inner binding that hides an outer one.
def main():
    let total = 1
    if total > 0:
        let total = 2
        print(total)
    print(total)
```

<!-- output: examples/reference-diagnostics/w0002_shadow.console -->
```console
$ sisu run w0002_shadow.sisu
warning[W0002]: `total` shadows a binding of an enclosing scope
  --> w0002_shadow.sisu:5:13
  |
5 |         let total = 2
  |             ^^^^^

2
1
```

Give the inner binding a name of its own, or assign to the outer one if
it is a `var`.

<!-- file: examples/reference-diagnostics/w0002_shadow_fixed.sisu -->
```sisu
# Each binding with a name of its own.
def main():
    let total = 1
    if total > 0:
        let doubled = 2
        print(doubled)
    print(total)
```

<!-- output: examples/reference-diagnostics/w0002_shadow_fixed.out -->
```text
2
1
```

Taught in [chapter 2](../tutorial/02-values.md).

### W0003

`type.unreachable-arm`: this `case` arm is unreachable; the arms above
it cover every value it matches.

Arms are tried from the top, so an arm whose values the arms above it
all cover never runs.

<!-- file: examples/reference-diagnostics/w0003_unreachable.sisu -->
```sisu
# An arm that the arm above it covers.
def describe(n: int) -> str:
    return match n:
        case _: "a number"
        case 0: "zero"

def main():
    print(describe(0))
```

<!-- output: examples/reference-diagnostics/w0003_unreachable.console -->
```console
$ sisu run w0003_unreachable.sisu
warning[W0003]: this `case` arm is unreachable; the arms above it cover every value it matches
  --> w0003_unreachable.sisu:5:14
  |
5 |         case 0: "zero"
  |              ^

a number
```

Move the specific arm above the general one, or remove it.

<!-- file: examples/reference-diagnostics/w0003_unreachable_fixed.sisu -->
```sisu
# The specific arm first.
def describe(n: int) -> str:
    return match n:
        case 0: "zero"
        case _: "a number"

def main():
    print(describe(0))
```

<!-- output: examples/reference-diagnostics/w0003_unreachable_fixed.out -->
```text
zero
```

Taught in [chapter 7](../tutorial/07-structs-and-enums.md).

## Run time

Codes R0001 to R0130. Messages of a running program: panics, errors that
`main` returns, and the failures that `sisu test` reports. They are
printed without their code, as `panic: ...` or `error: ...`, followed by
the trace.

### R0001

`panic.explicit`: {message}.

`panic(message)` stops the task with the message. A panic is for a bug,
a state the program should never reach; an input that can be wrong is an
error or an absent value instead.

<!-- file: examples/reference-diagnostics/r0001_panic.sisu -->
```sisu
# A panic for a case the program cannot handle.
def weekday(n: int) -> str:
    if n < 1 or n > 7:
        panic("no weekday {n}")
    return ["ma", "ti", "ke", "to", "pe", "la", "su"][n - 1]

def main():
    print(weekday(3))
    print(weekday(8))
```

<!-- output: examples/reference-diagnostics/r0001_panic.console -->
```console
$ sisu run r0001_panic.sisu
ke
panic: no weekday 8
  at weekday (r0001_panic.sisu:4:9)
  at main (r0001_panic.sisu:9:11)
```

When the case can happen, return `T?` or `T ! E` and let the caller
decide.

<!-- file: examples/reference-diagnostics/r0001_panic_fixed.sisu -->
```sisu
# An absent value for a number that names no weekday.
def weekday(n: int) -> str?:
    if n < 1 or n > 7:
        return None
    return Some(["ma", "ti", "ke", "to", "pe", "la", "su"][n - 1])

def main():
    print(weekday(3) ?? "none")
    print(weekday(8) ?? "none")
```

<!-- output: examples/reference-diagnostics/r0001_panic_fixed.out -->
```text
ke
none
```

### R0002

`panic.index-out-of-bounds`: index out of bounds: the length is {len}
but the index is {index}.

An index is checked against the length when it is used, and an index
past the end panics, naming both.

<!-- file: examples/reference-diagnostics/r0002_index.sisu -->
```sisu
# An index past the end of a list.
def main():
    let scores = [3, 1, 4]
    print(scores[3])
```

<!-- output: examples/reference-diagnostics/r0002_index.console -->
```console
$ sisu run r0002_index.sisu
panic: index out of bounds: the length is 3 but the index is 3
  at main (r0002_index.sisu:4:5)
```

Check the index first, or use `.get(i)`, which gives `None` past the
end.

<!-- file: examples/reference-diagnostics/r0002_index_fixed.sisu -->
```sisu
# `.get` gives an absent value past the end.
def main():
    let scores = [3, 1, 4]
    print(scores.get(3) ?? 0)
```

<!-- output: examples/reference-diagnostics/r0002_index_fixed.out -->
```text
0
```

### R0003

`panic.overflow`: integer overflow: {expr}.

Integer arithmetic that leaves its type's range panics, naming the
operation and its operands (D-009).

<!-- file: examples/reference-diagnostics/r0003_overflow.sisu -->
```sisu
# An addition past the largest `int`.
def main():
    let big = 9_223_372_036_854_775_807
    print(big + 1)
```

<!-- output: examples/reference-diagnostics/r0003_overflow.console -->
```console
$ sisu run r0003_overflow.sisu
panic: integer overflow: 9223372036854775807 + 1
  at main (r0003_overflow.sisu:4:11)
```

Use a wider type, or say what should happen with the wrapping (`+%`) or
saturating (`+|`) operators.

<!-- file: examples/reference-diagnostics/r0003_overflow_fixed.sisu -->
```sisu
# `+%` wraps around on purpose.
def main():
    let big = 9_223_372_036_854_775_807
    print(big +% 1)
```

<!-- output: examples/reference-diagnostics/r0003_overflow_fixed.out -->
```text
-9223372036854775808
```

### R0004

`panic.divide-by-zero`: division by zero.

An integer division or remainder by zero panics.

<!-- file: examples/reference-diagnostics/r0004_division.sisu -->
```sisu
# A division by zero.
def share(total: int, people: int) -> int:
    return total // people

def main():
    print(share(10, 0))
```

<!-- output: examples/reference-diagnostics/r0004_division.console -->
```console
$ sisu run r0004_division.sisu
panic: division by zero
  at share (r0004_division.sisu:3:12)
  at main (r0004_division.sisu:6:11)
```

Check the divisor before dividing, or state it as a precondition with
`requires`.

<!-- file: examples/reference-diagnostics/r0004_division_fixed.sisu -->
```sisu
# The zero handled before dividing.
def share(total: int, people: int) -> int:
    if people == 0:
        return 0
    return total // people

def main():
    print(share(10, 0))
```

<!-- output: examples/reference-diagnostics/r0004_division_fixed.out -->
```text
0
```

### R0006

`panic.assert`: assertion failed: {expr}.

`assert` checks a condition and panics when it is false, showing the
value of each part of the expression.

<!-- file: examples/reference-diagnostics/r0006_assert.sisu -->
```sisu
# An assertion that does not hold.
def main():
    let names = ["Aino", "Eero"]
    assert len(names) == 3
    print("three names")
```

<!-- output: examples/reference-diagnostics/r0006_assert.console -->
```console
$ sisu run r0006_assert.sisu
panic: assertion failed: len(names) == 3
    len(names) == 3
    │   │
    2   ["Aino", "Eero"]
  at main (r0006_assert.sisu:4:5)
```

Make the condition hold, or correct the assertion. In a test, a failed
`assert` fails the test.

<!-- file: examples/reference-diagnostics/r0006_assert_fixed.sisu -->
```sisu
# An assertion that holds.
def main():
    let names = ["Aino", "Eero", "Roni"]
    assert len(names) == 3
    print("three names")
```

<!-- output: examples/reference-diagnostics/r0006_assert_fixed.out -->
```text
three names
```

### R0010

`panic.missing-key`: the key {key} is not in the dict.

Indexing a dict with a key it does not hold panics, naming the key.

<!-- file: examples/reference-diagnostics/r0010_missing_key.sisu -->
```sisu
# A key that the dict does not hold.
def main():
    let ages = {"Aino": 7, "Eero": 9}
    print(ages["Roni"])
```

<!-- output: examples/reference-diagnostics/r0010_missing_key.console -->
```console
$ sisu run r0010_missing_key.sisu
panic: the key "Roni" is not in the dict
  at main (r0010_missing_key.sisu:4:5)
```

Look the key up with `.get(key)`, which gives `None`, or test `key in
ages` first.

<!-- file: examples/reference-diagnostics/r0010_missing_key_fixed.sisu -->
```sisu
# `.get` gives an absent value for a missing key.
def main():
    let ages = {"Aino": 7, "Eero": 9}
    print(ages.get("Roni") ?? 0)
```

<!-- output: examples/reference-diagnostics/r0010_missing_key_fixed.out -->
```text
0
```

### R0012

`panic.zip-lengths`: zip: the arguments have different lengths.

`zip` expects its arguments to be equally long, and panics when one runs
out first.

<!-- file: examples/reference-diagnostics/r0012_zip.sisu -->
```sisu
# `zip` over two lists of different lengths.
def main():
    for name, age in zip(["Aino", "Eero"], [7]):
        print("{name} {age}")
```

<!-- output: examples/reference-diagnostics/r0012_zip.console -->
```console
$ sisu run r0012_zip.sisu
Aino 7
panic: zip: the arguments have different lengths
  at main (r0012_zip.sisu:3:5)
```

Make them equally long, or use `zip_shortest` when stopping at the
shorter is what you want.

<!-- file: examples/reference-diagnostics/r0012_zip_fixed.sisu -->
```sisu
# `zip_shortest` stops at the shorter list.
def main():
    for name, age in zip_shortest(["Aino", "Eero"], [7]):
        print("{name} {age}")
```

<!-- output: examples/reference-diagnostics/r0012_zip_fixed.out -->
```text
Aino 7
```

### R0013

`panic.hole`: reached the typed hole {hole}.

A run that reaches a typed hole panics with the hole's name; the
compiler warned about it with W0001.

<!-- file: examples/reference-diagnostics/r0013_hole.sisu -->
```sisu
# A run that reaches a typed hole.
def double(x: int) -> int:
    return todo("double")

def main():
    print(double(21))
```

<!-- output: examples/reference-diagnostics/r0013_hole.console -->
```console
$ sisu run r0013_hole.sisu
warning[W0001]: typed hole `double` of type `int`
  --> r0013_hole.sisu:3:12
  |
3 |     return todo("double")
  |            ^^^^^^^^^^^^^^

panic: reached the typed hole `todo("double")`
  at double (r0013_hole.sisu:3:12)
  at main (r0013_hole.sisu:6:11)
```

Write the code the hole stands for.

<!-- file: examples/reference-diagnostics/r0013_hole_fixed.sisu -->
```sisu
# The hole filled.
def double(x: int) -> int:
    return x * 2

def main():
    print(double(21))
```

<!-- output: examples/reference-diagnostics/r0013_hole_fixed.out -->
```text
42
```

### R0018

`run.error-returned`: error: {error}.

An error that `main` returns ends the program: it is printed with the
place that passed it on, and the exit status is 1.

<!-- file: examples/reference-diagnostics/r0018_main_error.sisu -->
```sisu
# `main` returns the error of a failed parse.
error InputError:
    NotANumber(text: String)

def number(text: str) -> int ! InputError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def main() -> ! InputError:
    print(number("kolme")?)
    return Ok(())
```

<!-- output: examples/reference-diagnostics/r0018_main_error.console -->
```console
$ sisu run r0018_main_error.sisu
error: NotANumber(text="kolme")
  at r0018_main_error.sisu:11:11
```

Return the error when ending the program is right; handle it with
`match`, `else` or `.or` when the program can go on.

<!-- file: examples/reference-diagnostics/r0018_main_error_fixed.sisu -->
```sisu
# The error handled where it happens.
error InputError:
    NotANumber(text: String)

def number(text: str) -> int ! InputError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def main():
    match number("kolme"):
        case Ok(n): print(n)
        case Err(e): print("not a number: {e:?}")
```

<!-- output: examples/reference-diagnostics/r0018_main_error_fixed.out -->
```text
not a number: NotANumber(text="kolme")
```

### R0100

`contract.requires-failed`: precondition of `{function}` failed:
{clause}.

A `requires` clause states what a function needs of its caller; a call
that breaks it panics with the clause and the values of its parts.

<!-- file: examples/reference-diagnostics/r0100_precondition.sisu -->
```sisu
# A call that breaks its function's precondition.
def average(total: int, count: int) -> int:
    requires count > 0
    return total // count

def main():
    print(average(10, 0))
```

<!-- output: examples/reference-diagnostics/r0100_precondition.console -->
```console
$ sisu run r0100_precondition.sisu
panic: precondition of `average` failed: count > 0
    count > 0
    │
    0
  at average (r0100_precondition.sisu:3:14)
  at main (r0100_precondition.sisu:7:11)
```

The caller is wrong: check the condition before the call.

<!-- file: examples/reference-diagnostics/r0100_precondition_fixed.sisu -->
```sisu
# The call checks what the function requires.
def average(total: int, count: int) -> int:
    requires count > 0
    return total // count

def main():
    let count = 0
    if count > 0:
        print(average(10, count))
    else:
        print("nothing to average")
```

<!-- output: examples/reference-diagnostics/r0100_precondition_fixed.out -->
```text
nothing to average
```

### R0101

`contract.ensures-failed`: postcondition of `{function}` failed:
{clause}.

An `ensures` clause states what a function promises; a return that
breaks it panics with the clause and the values of its parts.

<!-- file: examples/reference-diagnostics/r0101_postcondition.sisu -->
```sisu
# A body that breaks its own postcondition.
def rounded_up(amount: int, step: int) -> int:
    requires step > 0
    ensures result >= amount
    return amount // step * step

def main():
    print(rounded_up(7, 5))
```

<!-- output: examples/reference-diagnostics/r0101_postcondition.console -->
```console
$ sisu run r0101_postcondition.sisu
panic: postcondition of `rounded_up` failed: result >= amount
    result >= amount
    │         │
    5         7
  at rounded_up (r0101_postcondition.sisu:4:13)
  at main (r0101_postcondition.sisu:8:11)
```

The function is wrong: correct its body, or its promise.

<!-- file: examples/reference-diagnostics/r0101_postcondition_fixed.sisu -->
```sisu
# A body that keeps its promise.
def rounded_up(amount: int, step: int) -> int:
    requires step > 0
    ensures result >= amount
    return (amount + step - 1) // step * step

def main():
    print(rounded_up(7, 5))
```

<!-- output: examples/reference-diagnostics/r0101_postcondition_fixed.out -->
```text
10
```

### R0115

`panic.char-boundary`: the byte index {index} is inside a character of
the text.

A text is its UTF-8 bytes, and `ä` takes two of them. A byte index
inside a character panics.

<!-- file: examples/reference-diagnostics/r0115_char_boundary.sisu -->
```sisu
# A byte slice that cuts a character in two.
def main():
    let word = "äiti"
    print(word.byte_slice(0, 1))
```

<!-- output: examples/reference-diagnostics/r0115_char_boundary.console -->
```console
$ sisu run r0115_char_boundary.sisu
panic: the byte index 1 is inside a character of the text
  at main (r0115_char_boundary.sisu:4:11)
```

Slice at character boundaries, or iterate `.chars()`.

<!-- file: examples/reference-diagnostics/r0115_char_boundary_fixed.sisu -->
```sisu
# A slice that ends at a character boundary.
def main():
    let word = "äiti"
    print(word.byte_slice(0, 2))
```

<!-- output: examples/reference-diagnostics/r0115_char_boundary_fixed.out -->
```text
ä
```

## The driver

Codes D0001 to D0119. The messages of the `sisu` command itself
([R16](sisu-command.md)): usage errors, the lines of `sisu test` and
`sisu bench`, replays, packages and the native backend.

### D0003

`cli.unknown-command`: unknown command `{command}`; run `sisu help`.

The first word of the command line is not a command of `sisu`.

Spell the command as `sisu help` lists it; a file to run is named after
`run`, or first when it ends in `.sisu`.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0004

`cli.missing-file`: `sisu {command}` needs a file.

The command needs an argument that the command line does not give: a
file for `sisu ws`, a log for `sisu replay`, a code for `sisu explain`,
a name for `sisu new`.

Give the argument. The message says "a file" for every command, also
where a code or a name is needed.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0006

`cli.errors-found`: the program was not run: errors found ({count}).

The program had errors, so the command did not run it (or build or
document it). It follows the diagnostics and counts the errors; warnings
do not stop a run.

Fix the errors above it. Every session on this page that runs a wrong
program ends with it.

Example: [E0200](#e0200).

### D0009

`cli.unknown-option`: unknown option `{option}` for `sisu {command}`.

An option that the command does not take, or an argument after the file
where the command takes none.

Check the command's options in [R16](sisu-command.md); options come
before the file.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0010

`cli.explain-unknown`: there is no diagnostic `{code}`.

`sisu explain` was given a code or an identifier that the catalog does
not have.

Copy the code from the diagnostic, as `E0200`, or the identifier, as
`resolve.undefined-name`.

Example: [R16](sisu-command.md#sisu-explain).

### D0021

`cli.ownership-errors`: the program was not run: the ownership checker
found errors ({count}); `--defer-borrow` runs it under the monitor.

The ownership checker found errors, and nothing else did, so the program
did not run. It names `--defer-borrow`, which runs the program under the
ownership monitor to see whether the conflict happens.

Fix the ownership errors above it; `sisu run --defer-borrow` shows a run
that reaches the conflict.

Example: [E0400](#e0400).

### D0023

`cli.bad-option-value`: `{option}` needs a number, not `{value}`.

An option that takes a number was given something else.

Give a whole number, as in `--explore 20`.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0037

`cli.replay-diverged`: the replay diverged from the log: {detail}.

A replay reached a capability call that the log does not hold the result
of: the log was redacted, or the program does something other than what
was recorded.

Record without `--redact` for the capabilities the replay needs.

Example: [R16](sisu-command.md#recording-a-run---record-and---redact).

### D0052

`cli.faults-without-sim`: `--faults` sets the fault rate of the `sim`
profile; add `--sim`.

`--faults` sets the fault rate of the `sim` profile, and the command
line has no `--sim`.

Add `--sim`.

Example: [R16](sisu-command.md#sisu-test).

### D0072

`cli.bad-package-name`: `{name}` cannot name a package: a package is
named in lowercase letters, digits and `_`, starting with a letter, and
not as a standard-library module.

A package name that is not lowercase letters, digits and `_` starting
with a letter, or that is the name of a standard-library module.

Choose a name such as `word_count`.

Example: [R16](sisu-command.md#sisu-new).

### D0073

`cli.exists`: `{path}` exists already.

`sisu new` does not write into a directory that exists.

Choose another name, or remove the directory first.

Example: [R16](sisu-command.md#sisu-new).

### D0111

`native.release`: `--release` builds with the native backend's release
profile (09); it goes with `--native`.

`--release` chooses the native backend's release profile, and the
command line has no `--native`; the interpreter has no profiles.

Add `--native`, or leave `--release` out.

Example: [R16](sisu-command.md#compiling-and-running-natively---native-and---release).

### D0113

`doc.std-path`: `{path}` is not a module of the standard library; `sisu
doc --std` takes the paths an import names them by, as `std.time`.

`sisu doc --std` was given a path that names no module of the standard
library.

Name the module as an import names it, as `std.time`.

Example: [R16](sisu-command.md#sisu-doc).

### D0115

`cli.bad-front-end`: `--front-end` takes `stage1` (the front end written
in Sisu, the default) or `rust` (the Rust front end, the oracle), not
`{value}`.

`--front-end` takes `stage1` or `rust`.

Use one of the two, or leave the option out for stage 1.

Example: [R16](sisu-command.md#choosing-the-front-end).

## Every code

The tables list every code of the catalog in this version, by range, with
its identifier and its message; `{name}` stands for a part of the message
that the diagnostic fills in. A code with an entry on this page links to
it. `sisu explain CODE` prints the same line.

### Lexing, every code

| Code | Identifier | Message |
|---|---|---|
| [E0001](#e0001) | `lex.tab-in-indentation` | a tab in indentation; Sisu indents with spaces only |
| E0002 | `lex.inconsistent-dedent` | this line's indentation matches no enclosing block; the enclosing blocks start at column {columns} |
| [E0003](#e0003) | `lex.unterminated-string` | this string literal is not closed before the end of the line |
| E0004 | `lex.unterminated-block-string` | this multi-line string literal is never closed |
| E0005 | `lex.unknown-character` | `{char}` cannot appear in Sisu source outside a string or comment |
| E0006 | `lex.unterminated-char` | this character literal is not closed |
| E0007 | `lex.invalid-escape` | `\{escape}` is not an escape sequence; write `\\` for a backslash |
| E0008 | `lex.unmatched-close` | `{close}` closes nothing |
| E0009 | `lex.unclosed-bracket` | `{open}` is never closed |
| E0010 | `lex.mismatched-bracket` | `{close}` does not close the `{open}` before it |
| E0011 | `lex.semicolon` | Sisu has no semicolons; put each statement on its own line |
| E0012 | `lex.invalid-number` | `{text}` is not a valid number literal |
| E0013 | `lex.lone-close-brace` | a single `}` in a string closes nothing; write `}}` for a literal brace |
| E0014 | `lex.bad-indent-char` | `{char}` in indentation; Sisu indents with ASCII spaces only |
| E0015 | `lex.backslash-continuation` | Sisu has no backslash continuation; break the line inside brackets, after a comma or before an operator |
| [E0016](#e0016) | `lex.bad-char-literal` | a character literal holds exactly one character |
| E0018 | `lex.newline-in-interpolation` | an interpolation in a one-line string must end on the same line |

### Parsing, every code

| Code | Identifier | Message |
|---|---|---|
| [E0100](#e0100) | `parse.expected` | expected {expected}, found {found} |
| E0101 | `parse.expected-block` | expected an indented block after `{keyword}` on line {line} |
| E0102 | `parse.block-on-same-line` | a block starts on the next line, indented; nothing may follow `:` on the line that opens it |
| [E0103](#e0103) | `parse.expected-expression` | expected an expression, found {found} |
| E0104 | `parse.expected-type` | expected a type, found {found} |
| [E0105](#e0105) | `parse.expected-pattern` | expected a pattern, found {found} |
| E0106 | `parse.expected-declaration` | expected a declaration, found {found} |
| E0107 | `parse.top-level-statement` | statements cannot appear at the top level of a module; put them in a function such as `main` |
| [E0108](#e0108) | `parse.mixed-and-or` | `and` and `or` cannot be mixed without parentheses |
| E0109 | `parse.mixed-bitwise` | `{first}` and `{second}` cannot be mixed without parentheses |
| E0110 | `parse.bitwise-arithmetic` | `{op}` cannot be mixed with arithmetic or shifts without parentheses |
| E0111 | `parse.shift-arithmetic` | a shift cannot be mixed with arithmetic without parentheses |
| E0112 | `parse.negative-power` | `-x ** y` is ambiguous; write `-(x ** y)` or `(-x) ** y` |
| [E0113](#e0113) | `parse.mixed-chain` | a chained comparison must point one way; `{first}` and `{second}` do not |
| E0114 | `parse.assignment-in-expression` | assignment is a statement and has no value; to compare, write `==` |
| [E0115](#e0115) | `parse.python-conditional` | `a if c else b` is not Sisu; write `if c: a else: b` |
| [E0116](#e0116) | `parse.if-expression-else` | an `if` expression needs an `else` branch, because it must have a value |
| E0117 | `parse.trailing-tokens` | unexpected {found} after the end of the statement |
| E0118 | `parse.arm-statement` | a `case` arm on one line holds a single expression; put statements in an indented block |
| E0119 | `parse.chain-operator` | `{op}` cannot be chained; use parentheses or `and` |
| E0121 | `parse.empty-interpolation` | an interpolation needs an expression between `{` and `}` |
| E0122 | `parse.closure-parameter` | a closure parameter must be a name |
| E0123 | `parse.walrus` | Sisu has no `:=`; bind with `let` or use `if let` |
| E0124 | `parse.keyword-as-name` | `{keyword}` is a keyword and cannot be used as a name here |
| E0125 | `parse.unexpected-indent` | this line is indented more than the line before it, which does not open a block |
| E0126 | `parse.expected-member` | a struct body holds fields, methods and `pass`; this line is none of them |

### Names and declarations, every code

| Code | Identifier | Message |
|---|---|---|
| [E0200](#e0200) | `resolve.undefined-name` | cannot find `{name}` in this scope |
| E0201 | `resolve.undefined-type` | cannot find a type named `{name}` |
| [E0202](#e0202) | `resolve.duplicate-definition` | `{name}` is defined more than once in this module |
| E0203 | `resolve.unknown-module` | there is no module `{name}` |
| E0204 | `resolve.not-in-module` | module `{module}` has no item `{name}` |
| E0205 | `resolve.duplicate-field` | `{name}` is declared more than once in `{owner}` |
| E0206 | `resolve.duplicate-variant` | `{owner}` has more than one variant named `{name}` |
| E0207 | `resolve.duplicate-parameter` | the parameter `{name}` appears more than once |
| E0208 | `resolve.type-argument-count` | wrong number of type arguments for `{name}`: expected {expected}, found {found} |
| E0209 | `resolve.not-a-type` | `{name}` is not a type |
| [E0211](#e0211) | `resolve.ambiguous-variant` | `{name}` is a variant of more than one type ({candidates}); qualify it, as in `{example}` |
| E0212 | `resolve.not-an-error-set` | `{name}` is not an error set, so it cannot follow `!` |
| E0213 | `resolve.drop-target` | `Drop` can be implemented only for a struct or enum declared in this module |
| E0215 | `resolve.self-outside-method` | `self` exists only inside a method |
| E0216 | `resolve.drop-signature` | `Drop` has one method, `def drop(mut self)` |
| E0217 | `resolve.duplicate-method` | `{owner}` has more than one method named `{name}` |
| E0218 | `resolve.self-not-first` | `self` must be the first parameter |
| E0219 | `resolve.const-type-required` | a `const` needs a type: `const {name}: T = ...` |
| E0220 | `resolve.import-unknown` | cannot import `{name}`: it names no module of this package, of a dependency in `sisu.toml`, or of the standard library |
| E0221 | `resolve.field-default-order` | a field with a default must not use the value of another field |
| E0222 | `resolve.parameter-default-order` | a parameter without a default cannot follow one with a default |
| E0223 | `resolve.not-a-module` | `{name}` is not a module |
| E0224 | `resolve.missing-body` | `{name}` has no body; only standard-library intrinsics are declared without one |
| E0225 | `resolve.std-only-attribute` | `@{name}` is reserved for the standard library |
| E0226 | `resolve.inferred-set-position` | an inferred error set (`T !`) can only be the result type of a function |
| E0227 | `resolve.union-outside-result` | `{A, B}` is a union of error sets and can only follow `!` |
| E0228 | `resolve.alias-cycle` | the type alias `{name}` refers to itself |
| E0229 | `resolve.self-type-outside` | `Self` exists only inside a type's declaration or `impl` |
| E0230 | `resolve.not-a-trait` | `{name}` is not a trait |
| E0231 | `resolve.trait-as-type` | `{name}` is a trait, not a type; a value of a type that implements it is `dyn {name}` |
| E0232 | `resolve.missing-trait-method` | the impl of `{trait}` for `{ty}` is missing {methods} |
| E0233 | `resolve.not-a-trait-method` | `{name}` is not a method of `{trait}` |
| E0234 | `resolve.method-signature` | `{name}` does not have the signature `{trait}` gives it: expected `{expected}`, found `{found}` |
| E0235 | `resolve.overlapping-impl` | `{trait}` is implemented for `{ty}` more than once; a type implements a trait at most once |
| E0236 | `resolve.orphan-impl` | `{trait}` for `{ty}` can be implemented only in the package of the trait or of the type, or in the root package |
| E0237 | `resolve.missing-assoc-type` | the impl of `{trait}` for `{ty}` does not say what `{name}` is; add `type {name} = ...` |
| E0238 | `resolve.unknown-assoc-type` | `{trait}` has no associated type `{name}` |
| E0239 | `resolve.no-assoc-type` | `{ty}` has no associated type `{name}` among its bounds |
| E0240 | `resolve.sealed-trait` | `{trait}` is implemented only by the standard library, so `{ty}` cannot implement it |
| E0241 | `resolve.not-object-safe` | `{trait}` cannot be used as `dyn {trait}`: its method `{method}` cannot be called through it, because {reason} |
| E0242 | `resolve.supertrait-missing` | `{ty}` implements `{trait}`, which requires `{super}`, but `{ty}` does not implement `{super}` |
| E0243 | `resolve.not-derivable` | `{name}` cannot be derived; `@derive` takes `Copy` and the traits that have a `comptime def derive`: `Debug`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that declares one |
| [E0244](#e0244) | `resolve.derive-field` | `{ty}` cannot derive `{trait}`: its field `{field}` has type `{fty}`, which does not implement `{trait}` |
| E0245 | `resolve.generic-default-order` | a type parameter without a default cannot follow one with a default |
| E0246 | `resolve.impl-default` | the type parameters of an `impl` or a function take no defaults; a default belongs to a type's or a trait's parameter |
| E0247 | `resolve.trait-cycle` | `{name}` cannot be its own supertrait |
| E0248 | `resolve.assoc-outside-trait-impl` | an associated type belongs in an impl of a trait |
| E0249 | `resolve.assoc-needs-type` | `type {name}` in an impl needs a type: `type {name} = ...` |
| E0250 | `resolve.not-delegable` | `{name}` cannot be delegated; `@delegate` takes `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Display`, `Debug` and `Len` |
| E0251 | `resolve.delegate-to` | `@delegate` needs `to="field"`, naming a field of `{ty}` |
| E0252 | `resolve.delegate-self` | `{trait}` returns a `{ty}`, so `{ty}` can delegate it only when `{field}` is its one field |
| E0253 | `resolve.delegate-field` | `{ty}` cannot delegate `{trait}` to `{field}`: its type `{fty}` does not implement `{trait}` |
| E0254 | `resolve.drop-generic` | a `Drop` for `{ty}` must apply to every `{name}`: write `{expected}`, with the type's parameters in order and only the bounds the type declares |
| E0255 | `resolve.private-item` | `{name}` is private to the module `{module}`; declare it `pub` to use it from another module |
| E0256 | `resolve.package-item` | `{name}` is `pub(package)` in `{module}`, so only the modules of the package `{package}` can use it |
| E0257 | `resolve.not-reexported` | `{name}` is imported into `{module}`, not declared there; import it from `{origin}`, or re-export it with `pub from {origin} import {name}` |
| E0258 | `resolve.private-field` | the field `{field}` of `{ty}` is private to the module `{module}`; declare it `pub` to use it from another module |
| E0259 | `resolve.private-method` | the method `{name}` of `{ty}` is private to the module `{module}`; declare it `pub` to call it from another module |
| E0260 | `resolve.not-a-dependency` | cannot import `{name}`: `{package}` is not a dependency of `{this}`; add it to `[dependencies]` in that package's `sisu.toml` |
| E0261 | `resolve.script-main` | a script's statements at the top level are its `main`, so it cannot also declare `main` |
| E0262 | `resolve.const-argument` | `{param}` is a `const` parameter of `{name}`, so it takes a constant, such as `{name}[..., 8]`, not a type |
| E0263 | `resolve.type-argument` | `{param}` is a type parameter of `{name}`, so it takes a type, not a constant |
| E0264 | `resolve.const-expression` | a type's constant argument is computed where the type is written, from integers, `const` parameters, constants written that way and the operators `+ - * // % ** << >> & \| ^`; this is none of them |
| E0265 | `resolve.const-not-int` | a type's constant argument is an `int`, and this is a `{ty}` |
| E0266 | `resolve.bad-cfg` | `@cfg` takes a condition: `test`, `feature="NAME"`, `not` and a condition, or `all(...)` or `any(...)` of conditions; this is none of them |

### Types, every code

| Code | Identifier | Message |
|---|---|---|
| [E0300](#e0300) | `type.mismatch` | expected `{expected}`, found `{found}` |
| [E0301](#e0301) | `type.integer-division` | `/` divides floats; for integers use `//`, or convert to `f64` first |
| [E0302](#e0302) | `type.not-bool` | a condition must be a `bool`, not `{found}`; Sisu has no truthiness |
| E0303 | `type.unknown-field` | `{ty}` has no field `{name}` |
| E0304 | `type.unknown-method` | `{ty}` has no method `{name}` |
| [E0305](#e0305) | `type.argument-count` | wrong number of arguments to `{name}`: expected {expected}, found {found} |
| E0306 | `type.unknown-keyword` | `{name}` has no parameter named `{arg}` |
| E0307 | `type.duplicate-argument` | the argument `{arg}` is given more than once |
| [E0308](#e0308) | `type.missing-argument` | missing the argument `{arg}` of `{name}` |
| E0309 | `type.not-callable` | a value of type `{ty}` cannot be called |
| [E0310](#e0310) | `type.binary-operator` | `{op}` cannot be applied to `{lhs}` and `{rhs}` |
| E0311 | `type.unary-operator` | `{op}` cannot be applied to `{ty}` |
| [E0312](#e0312) | `type.assign-immutable` | `{name}` is immutable; declare it with `var` to assign to it |
| E0313 | `type.not-a-place` | this expression cannot be assigned to |
| [E0314](#e0314) | `type.non-exhaustive` | this `match` does not cover {missing} |
| [E0315](#e0315) | `type.missing-return` | `{name}` returns `{ty}`, but control can reach the end of its body without a `return` |
| E0316 | `type.return-value-in-unit` | `{name}` returns nothing, so its `return` takes no value |
| E0317 | `type.return-needs-value` | this `return` needs a value of type `{ty}` |
| [E0318](#e0318) | `type.unused-result` | this `{ty}` is not used; handle it, or discard it with `_ = ...` |
| E0319 | `type.try-needs-fallible` | `?` needs a `T ! E` or a `T?`, found `{ty}` |
| [E0320](#e0320) | `type.try-context` | `?` on `{ty}` needs a function that returns `{needed}`, but this one returns `{found}` |
| E0321 | `type.error-not-in-set` | `{from}` errors cannot leave this function, whose error set is `{to}` |
| E0322 | `type.break-outside-loop` | `break` is only valid inside a loop |
| E0323 | `type.continue-outside-loop` | `continue` is only valid inside a loop |
| E0324 | `type.cannot-infer` | the type of `{name}` cannot be inferred here; add a type annotation |
| [E0325](#e0325) | `type.not-iterable` | a `for` loop cannot iterate over `{ty}` |
| E0326 | `type.pattern-mismatch` | this pattern cannot match a value of type `{ty}` |
| [E0327](#e0327) | `type.unknown-variant` | `{ty}` has no variant `{name}` |
| E0328 | `type.pattern-field-count` | wrong number of fields in the pattern for `{name}`: expected {expected}, found {found} |
| [E0329](#e0329) | `type.refutable-let` | this pattern might not match; add `else:` with a block that leaves the scope |
| E0330 | `type.else-not-fallible` | `else` after `let` needs a `T?`, a `T ! E` or a pattern that can fail; this value is `{ty}` |
| E0331 | `type.else-must-diverge` | the block after `else:` must leave the scope with `return`, `break`, `continue` or a panic |
| E0332 | `type.float-suffix` | a float literal takes the suffix `f32` or `f64`, not `{suffix}` |
| E0333 | `type.impure-call` | `{callee}` is not pure, so the pure function `{name}` cannot call it |
| E0334 | `type.impure-capability` | the pure function `{name}` cannot use the capability `{ty}` |
| E0335 | `type.impure-argument` | this function is passed where a pure function is required, but it calls `{callee}`, which is not pure |
| [E0337](#e0337) | `type.missing-field` | missing the field `{field}` of `{ty}` |
| E0338 | `type.move-out-of-borrow` | cannot move `{what}` out of a borrowed value; copy it with `.copy()` |
| E0339 | `type.mut-argument-missing` | `{param}` is a `mut` parameter, so the call must say `mut` before the argument |
| E0340 | `type.mut-argument-unexpected` | `{param}` is not a `mut` parameter, so the argument takes no `mut` |
| [E0341](#e0341) | `type.not-mutable` | cannot modify `{what}`; it is not declared with `var` |
| E0342 | `type.it-outside-closure` | `it` is the parameter of an implicit closure and exists only where a one-parameter function is expected |
| [E0344](#e0344) | `type.constant-negative-index` | a constant negative index does not compile; count from the end with `^`: `{fix}` |
| [E0345](#e0345) | `type.str-not-indexable` | `{ty}` cannot be indexed by position; use `.chars()`, `.bytes()` or `.byte_slice(a, b)` |
| [E0347](#e0347) | `type.bound-not-satisfied` | `{ty}` does not implement `{bound}` |
| E0349 | `type.not-generic` | `{name}` does not take type arguments |
| E0350 | `type.not-a-value` | `{name}` is a {kind}, not a value |
| E0351 | `type.unknown-associated` | `{ty}` has no associated function `{name}` |
| E0352 | `type.value-returns-borrow` | `{name}` cannot be a value: it returns a borrow of its parameter `{param}`, which a function value takes by value |
| E0353 | `type.closure-parameter-count` | wrong number of closure parameters: expected {expected}, found {found} |
| E0356 | `type.open-set-needs-wildcard` | `{ty}` is an open error set; matching it needs a `case _` arm |
| E0358 | `type.let-needs-type` | `let {name}` without a value needs a type annotation |
| E0359 | `type.no-constructor` | `{ty}` cannot be constructed in code |
| E0360 | `type.not-indexable` | a value of type `{ty}` cannot be indexed |
| E0361 | `type.method-needs-receiver` | `{name}` is a method; call it on a value, as in `x.{name}()` |
| E0362 | `type.assoc-not-method` | `{name}` has no `self`; call it on the type, as in `{ty}.{name}()` |
| E0363 | `type.tuple-index` | the tuple `{ty}` has no element {index} |
| E0364 | `type.unpack-count` | wrong number of names to unpack into: the value has size {found}, the pattern has size {expected} |
| E0365 | `type.if-branches` | the branches of this `{keyword}` have different types: `{first}` and `{second}` |
| E0366 | `type.uninitialized` | `{name}` is used before it is assigned |
| E0367 | `type.reassign-let` | `{name}` is a `let` binding and is already assigned |
| [E0368](#e0368) | `type.not-ordered` | `{ty}` has no order, so it cannot be compared with `{op}` or sorted |
| E0369 | `type.comparison-types` | `{lhs}` and `{rhs}` cannot be compared |
| E0370 | `type.optional-chain` | `?.` needs an optional value, found `{ty}` |
| E0371 | `type.coalesce` | `??` needs an optional value on its left, found `{ty}` |
| E0372 | `type.float-floor-division` | `//` divides integers; for floats use `/` |
| [E0373](#e0373) | `type.mixed-numbers` | `{op}` cannot mix `{lhs}` and `{rhs}`; convert one of them explicitly |
| [E0374](#e0374) | `type.literal-out-of-range` | `{text}` does not fit in `{ty}` |
| E0375 | `type.type-arguments-needed` | `{name}` needs a type argument, as in `{name}[T]` |
| E0376 | `type.positional-after-keyword` | a positional argument cannot follow a keyword argument |
| E0378 | `type.main-signature` | `main` must take nothing or one `System` parameter, and return nothing or `! E` |
| E0379 | `type.assert-message` | the message of `assert` must be a string |
| E0380 | `type.no-display` | `{ty}` has no text rendering for `{spec}` |
| E0381 | `type.return-in-test` | a test returns nothing |
| E0382 | `type.self-type-outside` | `Self` exists only inside a type's declaration |
| E0383 | `type.from-end-outside-index` | `^` counts from the end and appears only inside an index, as in `xs[^1]` |
| E0384 | `type.arm-needs-expression` | an arm of a `match` that has a value holds one expression, not a block |
| E0385 | `type.bindings-in-or-pattern` | `{name}` is bound in some alternatives of `\|` but not in all; every alternative must bind the same names |
| E0386 | `type.variant-needs-fields` | `{name}` has fields; write `{name}(...)` |
| E0387 | `type.star-outside-list` | `*name` collects the rest of a list and appears only in a list pattern |
| E0388 | `type.rest-outside-arguments` | `..` skips the remaining fields and appears only inside `Name(...)` |
| E0389 | `type.no-self` | `self` exists only inside a method |
| E0390 | `type.not-a-struct-pattern` | `{name}` cannot be matched with fields |
| E0391 | `type.const-pattern` | a constant cannot be used as a pattern yet; compare with `==` in a guard |
| E0392 | `type.impure-value-call` | `{name}` is pure, so it cannot call a function value whose type is not `pure` |
| E0393 | `type.impure-drop` | dropping `{value}` here runs the `Drop` of `{ty}`, which is not pure, so the pure function `{name}` cannot do it |
| E0394 | `type.ambiguous-method` | `{name}` is a method of more than one trait `{ty}` implements ({traits}); call it on the trait, as in `Trait.{name}(x)` |
| E0395 | `type.unsized-dyn` | `{ty}` has no size of its own; hold it behind `ref` or in a `Box` |
| E0396 | `type.view-to-dyn` | `{ty}` borrows, so it cannot become `{dyn}`, which owns its value |
| E0397 | `type.must-consume-argument` | `{ty}` is must-consume, so it cannot be the argument of the generic parameter `{param}`, whose values the body may drop |
| E0398 | `type.structural-only` | {callee} is hashed and compared structurally in stage 0, and `{ty}` has an impl of `{trait}` of its own; derive `Hash` and `Eq` for it, or key by a field |
| E0399 | `type.bad-format-spec` | `{spec}` is not a format spec; it is `[[fill]align][sign][#][0][width][,\|_][.precision][type]`, as in `>8.2f`, with the type one of `d x X o b e E f F % s ?` |

### Ownership, every code

| Code | Identifier | Message |
|---|---|---|
| [E0400](#e0400) | `borrow.use-after-move` | `{name}` was moved on line {line} and cannot be used afterwards |
| E0401 | `borrow.use-after-partial-move` | `{name}` cannot be used as a whole, because `{part}` was moved out of it on line {line} |
| E0402 | `borrow.modify-while-borrowed` | `{name}` is borrowed on line {line} and cannot be modified until that borrow ends on line {end} |
| E0403 | `borrow.use-while-mut-borrowed` | `{name}` is borrowed with `mut` on line {line} and cannot be used until that borrow ends on line {end} |
| E0404 | `borrow.move-while-borrowed` | `{name}` is borrowed on line {line} and cannot be moved until that borrow ends on line {end} |
| E0405 | `borrow.mut-while-borrowed` | `{name}` is already borrowed on line {line}, so it cannot be borrowed with `mut` until that borrow ends on line {end} |
| E0406 | `borrow.does-not-live-long-enough` | `{name}` does not live long enough: it is borrowed on line {line}, and the borrow is still used on line {end}, after `{name}` is gone |
| E0407 | `borrow.return-local` | this returns a borrow of `{name}`, which belongs to `{function}` and is gone when it returns |
| E0408 | `borrow.return-not-from` | this returns a borrow of the parameter `{param}`, which is not in the `from` clause of `{function}` |
| E0409 | `borrow.store-in-plain-type` | `{ty}` cannot hold this value, because it borrows `{name}`; only a `view struct` holds borrows |
| E0410 | `borrow.store-into-caller` | this value borrows `{name}`, so it cannot be stored in `{param}`, which belongs to the caller of `{function}` |
| [E0411](#e0411) | `borrow.view-field-in-plain-struct` | `{ty}` holds `{field}: {fty}`, which borrows; only a `view struct` may hold a borrow |
| E0412 | `borrow.view-in-enum` | `{ty}` cannot hold `{fty}`, which borrows; an enum has no view form, so store an owned value such as `String` |
| E0413 | `borrow.from-required` | `{name}` returns `{ret}`, which borrows, and has more than one borrowed parameter ({params}); name the ones it borrows from with `from` |
| E0414 | `borrow.from-not-a-parameter` | `{name}` in the `from` clause is not a borrowed parameter of `{function}` |
| E0415 | `borrow.from-without-borrow` | `{function}` returns `{ty}`, which does not borrow, so it takes no `from` clause |
| E0416 | `borrow.move-out-of-drop` | cannot move `{part}` out of `{name}`, because `{ty}` implements `Drop`, which needs the whole value |
| E0417 | `borrow.must-consume` | `{name}` is never consumed: a `{ty}` must be passed to an `own` parameter before it goes out of scope |
| E0418 | `borrow.must-consume-exit` | this leaves the scope with `{name}` unconsumed: a `{ty}` must be passed to an `own` parameter first |
| E0419 | `borrow.move-out-of-place` | cannot move out of `{name}` through a reference or an index; copy it with `.copy()` |
| E0420 | `borrow.must-consume-intrinsic` | `{callee}` may drop values that `{name}` holds, and a `{ty}` is must-consume: its values must be passed to an `own` parameter |
| E0421 | `borrow.view-in-owner` | `{ty}` borrows, so it cannot be put in `{owner}`, which can outlive what it borrows (04, rule 3) |
| E0422 | `borrow.must-consume-in-owner` | `{ty}` is must-consume, so it cannot be put in `{owner}`, which can drop it where the checker does not see |
| E0423 | `type.own-closure-capture` | an `own` closure keeps a copy of `{name}`, and `{ty}` is not `Copy`; stage 0 copies captures (a capture by move arrives with tasks) |
| E0424 | `type.own-closure-changes` | an `own` closure keeps a copy of `{name}` and cannot change it; borrow it with a plain closure instead |
| E0425 | `task.closure-literal` | `{function}` takes a closure written at the call (or a `do` block), so that what it captures can be checked; a function value is not one |
| E0426 | `task.not-send` | `{name}` moves into the task, but `{ty}` is not `Send` |
| E0427 | `task.not-sync` | `{name}` is shared with the task by reference, but `{ty}` is not `Sync` |
| E0428 | `task.borrows-body-local` | the task borrows `{name}`, which belongs to the scope's body: the body ends before the scope joins its tasks |
| E0429 | `task.borrows-elsewhere` | the task borrows `{name}`, but only a task spawned on a scope in that scope's body may borrow, and only what the body's environment borrows |
| E0430 | `task.scope-escapes` | a scope's body cannot return `{ty}`: a scope and the handles of its tasks end with the scope |
| E0431 | `task.race-types` | the tasks of a race return one type: this one returns `{found}`, an earlier one `{expected}` |
| E0432 | `task.result-unknown` | the type of the task's result must be known where it is joined |
| E0433 | `task.result-not-send` | the task returns `{ty}`, which is not `Send`, so it cannot cross back to the task that joins it |
| E0434 | `task.select-arm` | a `select` arm waits for a `recv`, `send` or `join` call, a `Timer` or a `Cancellation`, not `{ty}` |
| E0435 | `type.yield-outside-generator` | `yield` belongs in the body of a generator, a function that returns `Iter[T]` (not in a closure) |
| E0436 | `type.generator-result` | `{name}` yields, so it must return `Iter[T]`, not `{ty}` |
| E0437 | `task.not-send-mut` | `{name}` is changed by the task through a reference, but `{ty}` is not `Send` |

### Lowering, every code

| Code | Identifier | Message |
|---|---|---|
| E0500 | `sir.parse` | SIR: {detail} |
| E0501 | `sir.invalid` | invalid SIR in `{function}`: {detail} |

### Compile-time evaluation, every code

| Code | Identifier | Message |
|---|---|---|
| E0600 | `comptime.not-constant` | `comptime` evaluates what is known at compile time, and `{name}` is known only at run time |
| E0601 | `comptime.io` | the compile-time evaluation of this expression used {what}; nothing reaches the world at compile time |
| E0602 | `comptime.panicked` | the compile-time evaluation of this expression panicked: {message} |
| E0603 | `comptime.budget` | the compile-time evaluation in `{item}` used up its budget of {steps} steps; raise it with `@comptime_budget(steps=N)` on `{item}` |
| E0604 | `comptime.not-kept` | {what} made at compile time cannot be kept for run time; numbers, text, booleans, characters, and lists, sets, dicts, tuples, options, results, structs and enums of them can |
| E0605 | `comptime.only` | `{name}` is a `comptime def`: it can be called only at compile time, inside `comptime` |
| E0606 | `comptime.budget-attribute` | `@comptime_budget` takes `steps=N`, a positive number of steps |
| E0607 | `comptime.derive-template` | a trait's derive is `comptime def derive(T: type) -> impl {trait} for T`, whose body is `return impl {trait} for T:` with the impl's methods |
| E0608 | `comptime.derive-failed` | deriving `{trait}` for `{ty}` stopped at compile time: {reason} |
| E0609 | `comptime.instance-failed` | making `{name}` stopped at compile time: {reason} |
| E0610 | `comptime.instances-unbounded` | `{name}` needs more than {count} instances: its instances call it with ever more types |
| E0611 | `comptime.template-value` | `{name}` reflects on its type parameters, so it is made for each call's types and cannot be used as a value |
| E0612 | `comptime.embed-failed` | cannot embed `{path}`: {reason} |
| E0613 | `comptime.embed-path` | `embed` takes the path of a file as a string literal |
| E0614 | `comptime.no-fake` | `cap.Fake[{name}]` needs a trait of the program whose methods can all be forwarded: a `self` or `mut self` receiver, no type parameters, and no borrow or `Self` in a signature |
| E0615 | `comptime.unsafe-written` | code that compile-time evaluation writes holds no `unsafe`, so that every unsafe block stays in a source file, where `sisu audit --unsafe` shows it |

### Capabilities, every code

| Code | Identifier | Message |
|---|---|---|
| E0700 | `cap.not-shareable` | `{ty}` cannot become `{cap}`: a capability may reach every task, and `{ty}` is not `{auto}` |
| E0701 | `cap.internal` | `{name}` is internal to the standard library; a program reaches the world only through the capabilities it is given |
| E0702 | `type.test-signature` | a test takes nothing, the real `System` as its one parameter (`test "name"(sys: System):`), or fresh fakes of the capabilities it names, one of each (`test "name"(fs: Fs, clock: Clock):`) |
| E0703 | `cap.untrusted` | the package `{package}` is not trusted with {what}; a dependency may contain `unsafe` and `extern` only when `[trust] unsafe` in the root package's `sisu.toml` lists it |

### Contracts, every code

| Code | Identifier | Message |
|---|---|---|
| E0800 | `contract.misplaced` | `{keyword}` is a function's contract and belongs at the start of its body, before its first statement |
| E0801 | `contract.not-pure` | a contract only reads what it is given, and `{callee}` is not `pure` |
| E0802 | `contract.changes` | a contract only reads what it is given, and this changes it |
| E0803 | `contract.old-outside` | `old(...)` is a value as it was on entry, which only an `ensures` clause can ask for |
| E0804 | `contract.old-arguments` | `old` takes one expression, as in `old(self.balance)` |
| E0805 | `contract.generator-ensures` | a generator has no `ensures`: what it returns is its iterator |
| E0806 | `contract.invariant-field` | `{what}` is part of a `{ty}`, whose invariant only its own methods may change and check again; change it through one of them |
| E0807 | `contract.not-in-test` | a test has no contract; check what it needs with `assert` |
| E0810 | `refine.not-satisfied` | `{value}` is not a `{ty}`: `{predicate}` is false for it |
| E0811 | `refine.needs-try-from` | expected `{expected}`, found `{found}`; a value becomes a `{expected}` where its predicate is checked, with `{name}.try_from(...)` |
| E0812 | `refine.change-through-base` | `{what}` is a `{ty}`, which a change through its base could take outside `{predicate}`; change a copy of the base and check it again with `try_from` |
| E0813 | `refine.own-impl` | `{ty}` refines `{base}` and has its reading impls; it cannot have an impl of `{trait}` of its own |
| E0814 | `refine.predicate-type` | the predicate of `{ty}` must be a `bool`, found `{found}` |
| E0820 | `test.no-generator` | `{ty}` has no generator for property tests: {why} |
| E0821 | `test.refines-mismatch` | `@refines({reference})` on `{name}` cannot be checked: {why} |

### Stage 0's limits, every code

| Code | Identifier | Message |
|---|---|---|
| E0900 | `stage0.unsupported` | {construct} is not implemented in stage 0 yet; it arrives with {phase} |
| E0901 | `stage0.internal` | internal compiler error in {stage}: {detail} |
| E0971 | `sim.unmodelled-extern` | this `sim` build reaches `{callee}`, an `extern` call with no simulation model, in `{function}` at {at} |
| E0981 | `unsafe.call` | `{name}` is an `unsafe def`: call it inside an `unsafe:` block or an `unsafe` expression |
| E0982 | `unsafe.foreign-call` | `{name}` is a foreign function, whose signature is a claim the compiler cannot check: call it inside an `unsafe:` block or an `unsafe` expression |
| E0983 | `unsafe.fn-value` | `{name}` is {what}, which cannot be a function value: a call through the value would need no `unsafe` |
| E0984 | `unsafe.impl-needed` | `{trait}` is an `unsafe trait`, whose implementors keep a promise the compiler cannot check: its impl is written `unsafe impl` |
| E0985 | `unsafe.impl-promises-nothing` | {what}, so `unsafe impl` promises nothing: write `impl` |
| E0986 | `unsafe.never-asserted` | `{trait}` is never asserted: the compiler decides it from a type's parts, and nothing may claim it otherwise |
| E0987 | `unsafe.not-ptr` | `ptr mut T` is the raw pointer of `std.mem`, and `{name}` here names {found} |
| E0988 | `unsafe.in-contract` | a contract holds no `unsafe`: it may call safe functions built on unsafe code, and unsafe code relies on no contract |

### Packages and manifests, every code

| Code | Identifier | Message |
|---|---|---|
| E1000 | `package.toml-syntax` | this is not TOML that `sisu.toml` reads: expected {expected}, found {found} |
| E1001 | `package.toml-duplicate` | `{key}` is given more than once in `{table}` |
| E1002 | `package.missing-key` | `{table}` needs `{key}` |
| E1003 | `package.bad-value` | `{key}` takes {expected}, found {found} |
| E1004 | `package.bad-name` | `{name}` cannot name a package: {why} |
| E1005 | `package.no-manifest` | the dependency `{name}` has no `sisu.toml` at `{path}` |
| E1006 | `package.renamed-dependency` | the dependency `{name}` is the package `{found}`; a path dependency is named by its own `[package] name` |
| E1007 | `package.cycle` | the packages depend on each other in a cycle: {cycle}; packages build in dependency order, so their graph has no cycle |
| E1008 | `package.no-root-module` | the package `{name}` has no root module; add `src/main.sisu` (an application) or `src/lib.sisu` (a library) |
| E1009 | `package.two-root-modules` | the package `{name}` has both `src/main.sisu` and `src/lib.sisu`; a package has one root module |
| E1010 | `package.not-a-path` | the dependency `{name}` is not a `path` dependency; stage 0 has no registry, so a dependency is `{name} = { path = "../{name}" }` |
| E1011 | `package.bad-when` | `when` takes the build target's facts `os`, `arch` and `runtime`, each a string or a list of strings, as in `when = { os = "windows" }` |
| E1012 | `package.module-name` | `{file}` is not a module: `{name}` is not a module name; a module is named in lowercase letters, digits and `_` |
| E1013 | `package.no-member` | the workspace member `{path}` has no `sisu.toml` |
| E1014 | `package.script-metadata` | a script's metadata starts with `# /// script` and ends with a line `# ///`; this block has no end |
| E1015 | `package.trust-unknown` | `[trust] unsafe` names `{name}`, which is not a dependency of this package |
| E1016 | `package.no-package` | `{path}` has `[workspace]` but no `[package]`, so it cannot be a dependency |
| E1017 | `package.reserved-name` | `{name}` is the name of a standard-library module, which `import {name}` names; a package of the program needs another name |
| E1018 | `package.unknown-feature` | `{name}` is not a feature of `{package}`; a package's features are the keys of its `[features]` |
| E1019 | `package.unsafe-undeclared` | the package `{package}` contains {what}, and its `sisu.toml` declares no `unsafe`: add `unsafe = "{level}"` to its `[package]` |
| E1020 | `package.unsafe-level` | the package `{package}` declares `unsafe = "memory"`, which allows no {what}: declare `unsafe = "extern"` |
| E1021 | `package.unsafe-untrusted` | the dependency `{name}` declares `unsafe = "{level}"`, and `[trust] unsafe` in the root package's `sisu.toml` does not list it |
| E1022 | `package.unsafe-raised` | `sisu.lock` records `{name}` with {old}, and it now declares `unsafe = "{level}"`: accept the change with `--accept-unsafe` |

### Warnings, every code

| Code | Identifier | Message |
|---|---|---|
| [W0001](#w0001) | `type.hole` | typed hole `{name}` of type `{ty}` |
| [W0002](#w0002) | `resolve.shadow-outer` | `{name}` shadows a binding of an enclosing scope |
| [W0003](#w0003) | `type.unreachable-arm` | this `case` arm is unreachable; the arms above it cover every value it matches |
| W0004 | `type.unreachable-code` | this code is unreachable |
| W0005 | `package.unknown-key` | `{key}` is not a key of `{table}` that stage 0 knows; it is ignored |
| W0006 | `resolve.unknown-feature` | no `[features]` of this package declares `{name}`, so `feature="{name}"` never holds |
| W0007 | `unsafe.no-safety-comment` | this {what} has no `# safety:` comment above it saying why each obligation it takes on holds |
| W0008 | `unsafe.no-safety-list` | `{name}` is an `unsafe {kind}` without a `Safety:` list in its `##` comment naming the obligations its {who} take |
| W0009 | `unsafe.unused` | this {what} holds no unsafe operation |
| W0010 | `unsafe.unnamed-obligation` | the `# safety:` comment names no obligation of `{callee}`, which are {obligations} |
| W0011 | `unsafe.unexplained-purity` | this {what} calls `{callee}`, which is not `pure`, in the `pure` function `{name}`: its `# safety:` comment says why the function stays observationally pure, under a `pure:` part |
| W0012 | `unsafe.level` | the package declares `unsafe = "{level}"`, {why} |
| W0013 | `unsafe.auto-trait-bound` | `unsafe impl {trait}` claims `{trait}` for every `{param}`, which is not bounded by `{trait}` |
| W0014 | `unsafe.ambient-authority` | `{name}` is public and safe and reaches the foreign function `{callee}` without taking a capability, so its callers need no authority for what the call does |

### Run time, every code

| Code | Identifier | Message |
|---|---|---|
| [R0001](#r0001) | `panic.explicit` | {message} |
| [R0002](#r0002) | `panic.index-out-of-bounds` | index out of bounds: the length is {len} but the index is {index} |
| [R0003](#r0003) | `panic.overflow` | integer overflow: {expr} |
| [R0004](#r0004) | `panic.divide-by-zero` | division by zero |
| R0005 | `panic.shift-out-of-range` | a shift by {amount} is out of range; shifts take 0 to 63 |
| [R0006](#r0006) | `panic.assert` | assertion failed: {expr} |
| R0007 | `panic.assert-message` | assertion failed: {expr} message: {message} |
| R0008 | `panic.expect` | {message}: {value} |
| R0009 | `panic.unreachable` | entered unreachable code: {message} |
| [R0010](#r0010) | `panic.missing-key` | the key {key} is not in the dict |
| R0011 | `panic.negative-exponent` | a negative exponent on an integer: {expr} |
| [R0012](#r0012) | `panic.zip-lengths` | zip: the arguments have different lengths |
| [R0013](#r0013) | `panic.hole` | reached the typed hole {hole} |
| R0014 | `panic.during-unwind` | panic while unwinding: {message} the first panic: {first} a panic during unwinding aborts the program |
| R0015 | `panic.negative-index` | the index {index} is negative; count from the end with `^` |
| R0016 | `panic.slice-bounds` | the slice bounds {start}:{end} are negative |
| R0017 | `panic.unordered` | NaN has no order |
| [R0018](#r0018) | `run.error-returned` | error: {error} |
| R0019 | `run.panic-header` | panic: {message} |
| R0020 | `run.frame` | at {function} ({location}) |
| R0021 | `panic.range-step` | the step of a range must not be zero |
| R0023 | `panic.conversion` | cannot convert {value} to `{ty}` |
| R0024 | `run.internal` | internal error in the interpreter: {detail} |
| R0025 | `run.assert-left` | left:  {value} |
| R0026 | `run.assert-right` | right: {value} |
| R0029 | `panic.unreachable-plain` | entered unreachable code |
| R0030 | `panic.expect-none` | called `expect` on `None` |
| R0031 | `panic.expect-error` | called `expect` on an error: {value} |
| R0032 | `panic.stack-overflow` | stack overflow in `{function}`: the calls nest deeper than the stack holds |
| R0033 | `panic.moved-value` | use of a value after it was moved |
| R0034 | `panic.dangling` | use of a reference to a value that no longer exists |
| R0037 | `panic.insert-out-of-bounds` | insert index {index} is out of bounds for a list of length {len} |
| R0038 | `run.step-limit` | the program ran past its limit of {steps} statements |
| R0039 | `run.trace-line` | at {location} |
| R0040 | `run.no-entry` | the program has no `main` function |
| R0041 | `run.trace-header` | error return trace: |
| R0042 | `monitor.header` | ownership violation: {what} |
| R0043 | `monitor.use-ended-loan` | a reference was used after its loan had ended |
| R0044 | `monitor.use-dangling` | a reference was used after `{name}` was gone |
| R0045 | `monitor.use-moved` | `{name}` was used while it holds no value: it was moved out, or never assigned |
| R0046 | `monitor.loan` | the loan: {kind} borrow of `{place}` in `{function}`, created at {location} by `{stmt}` |
| R0047 | `monitor.ended-by` | ended by: {operation} of `{place}` at {location} (`{stmt}`) |
| R0048 | `monitor.used-at` | used at: {location} (`{stmt}`) |
| R0049 | `monitor.value` | value: `{place}` = {value} |
| R0050 | `monitor.counts` | loans of `{place}` not ended yet: {shared} shared, {exclusive} with `mut` |
| R0051 | `monitor.sir-header` | SIR of `{function}`: |
| R0052 | `monitor.op-write` | a modification |
| R0053 | `monitor.op-read` | a use |
| R0054 | `monitor.op-move` | a move |
| R0055 | `monitor.op-drop` | a drop |
| R0056 | `monitor.op-dead` | the end of the scope |
| R0057 | `monitor.op-borrow-mut` | a `mut` borrow |
| R0058 | `monitor.kind-shared` | a shared |
| R0059 | `monitor.kind-mut` | a `mut` |
| R0060 | `defer.header` | counterexample run: the ownership checker rejected this program; its errors are deferred to the monitor |
| R0061 | `defer.conflict` | outcome: conflict reached; the run stopped at a concrete ownership conflict |
| R0062 | `defer.no-conflict` | outcome: no conflict; this run ended without an ownership conflict |
| R0063 | `defer.reached` | the checker still rejects line {line} ({code} {id}), which this run reached without a conflict (count: {count}) |
| R0064 | `defer.not-reached` | the checker still rejects line {line} ({code} {id}), which this run never reached |
| R0065 | `defer.false-positive` | every rejected statement was reached without a conflict: a candidate false positive |
| R0066 | `defer.inconclusive` | a rejected statement was never reached: inconclusive, an untested path |
| R0067 | `defer.signature` | the checker still rejects line {line} ({code} {id}), a rule about declarations that no run can test |
| R0068 | `monitor.no-value` | <no value> |
| R0069 | `monitor.unconsumed` | `{name}` was dropped holding a `{ty}`, which must be passed to an `own` parameter |
| R0070 | `defer.declarations-only` | only rules about declarations reject the program, and the run reached no conflict: a candidate false positive |
| R0071 | `monitor.op-return` | the return from the frame |
| R0072 | `panic.stale-key` | stale arena key: slot {index}, generation {generation}, no longer holds its value |
| R0073 | `panic.mutex-held` | the lock is already held by this task, which would wait for itself forever |
| R0074 | `panic.mutex-poisoned` | the lock is poisoned: a task panicked holding it and left its value's invariant broken |
| R0076 | `panic.scope-ended` | a task was spawned on a scope that has ended |
| R0077 | `panic.joined-twice` | the task was joined already: its result was taken by an earlier `join` |
| R0078 | `panic.empty-race` | a race whose body spawned no task has no winner |
| R0079 | `panic.unobserved-error` | a task failed with {error} and nobody joined it, and the scope's result cannot carry it |
| R0080 | `run.deadlock` | deadlock: every task is waiting, and no timer can wake any of them |
| R0081 | `run.deadlock-task` | {task} waits for {wait} at {location} |
| R0082 | `run.task-root` | the root task |
| R0083 | `panic.channel-capacity` | a channel's capacity must be at least 1, not {capacity} |
| R0084 | `run.log-record` | {level}: {message} |
| R0085 | `run.log-record-task` | {level} [task {task}]: {message} |
| R0086 | `run.stats-tasks` | stats: {tasks} tasks, at most {max} at once, {switches} switches |
| R0087 | `run.stats-latency` | stats: scheduling latency p50 {p50} p99 {p99} max {max} statements; p50 {p50us} p99 {p99us} microseconds |
| R0088 | `run.stats-shared` | stats shared: {local} cells from task-local sites, {crossing} from may-cross sites, {crossed} reached by another task |
| R0089 | `monitor.shared-crossed` | a `Shared` whose construction site the escape analysis tagged task-local was reached by another task |
| R0090 | `panic.into-inner-poisoned` | `into_inner` of a poisoned lock: a task panicked holding it and left its value's invariant broken |
| R0091 | `panic.nested-recv` | a receiver inside another iterator would have to wait; iterate the receiver itself |
| R0092 | `run.task-named` | task {task} |
| R0093 | `panic.shift-out-of-range-sized` | a shift of a `{ty}` by {amount} is out of range; shifts take 0 to {max} |
| R0094 | `panic.bad-port` | {port} is not a port; ports are 0 to 65535 |
| [R0100](#r0100) | `contract.requires-failed` | precondition of `{function}` failed: {clause} |
| [R0101](#r0101) | `contract.ensures-failed` | postcondition of `{function}` failed: {clause} |
| R0102 | `contract.invariant-failed` | invariant of `{ty}` failed: {clause} |
| R0103 | `contract.refinement-failed` | `{value}` is not a `{ty}`: `{clause}` is false for it |
| R0104 | `run.case-too-long` | the case ran past its budget of {steps} statements |
| R0105 | `sim.always-failed` | `sim.always` does not hold: {what} |
| R0106 | `sim.choose-none` | `sim.choose` needs at least one option |
| R0107 | `test.snapshot-changed` | the snapshot `{name}` changed (`sisu test --update` accepts the new one): {diff} |
| R0108 | `test.snapshot-missing` | there is no snapshot `{name}` yet (`sisu test --update` stores this one): {value} |
| R0109 | `run.live-epoch` | watch: epoch {epoch} replaces {functions} |
| R0110 | `run.live-rejected` | watch: the save of {files} does not check, so the program keeps running epoch {epoch}: {diagnostics} |
| R0111 | `run.live-restart-signature` | watch: the signature of `{function}` changed, so the program starts again |
| R0112 | `run.live-restart-layout` | watch: the layout of `{ty}` changed, so the program starts again |
| R0114 | `run.live-restart-impls` | watch: the impls of `{trait}` changed, so the program starts again |
| [R0115](#r0115) | `panic.char-boundary` | the byte index {index} is inside a character of the text |
| R0116 | `run.stats-depth` | stats stack-depth: {calls} calls, the deepest stack {max} frames, about {bytes} bytes of native stack there (16 per SIR local and 64 per frame) |
| R0117 | `run.stats-depth-bucket` | stats stack-depth: {calls} calls at a depth of {from} to {to} frames |
| R0118 | `run.stats-shared-sites` | stats shared: {local} of {total} `Shared` construction sites tagged task-local, {crossing} may-cross |
| R0119 | `run.stats-shared-site` | stats shared: {at} in `{function}`, {tag}: {cells} cells made, {crossed} reached by another task |
| R0120 | `run.stats-tag-local` | task-local |
| R0121 | `run.stats-tag-crossing` | may-cross |
| R0122 | `run.native-internal` | internal error in the native runtime: {detail} |
| R0124 | `run.stats-latency-native` | stats: scheduling latency p50 {p50us} p99 {p99us} microseconds (native code counts no statements) |
| R0125 | `run.live-allocations` | allocations: {count} of the runtime's live at exit |
| R0126 | `run.peak-allocations` | allocations: at most {count} of the runtime's live at once |
| R0127 | `run.trace-elided` | ... (frames left out) |
| R0128 | `panic.during-abandon` | panic while a generator dropped before its end cleans up: {message} a panic there aborts the program |
| R0129 | `panic.abandoned-wait` | a generator dropped before its end cannot wait while it cleans up |
| R0130 | `panic.driven-wait` | a generator inside another iterator, or given to `sum`, `zip` and the other functions of an `Iterable[T]`, cannot wait; iterate it with `for` |

### The driver, every code

| Code | Identifier | Message |
|---|---|---|
| D0001 | `cli.cannot-read` | cannot read `{path}`: {reason} |
| D0002 | `cli.usage` | the list of commands that `sisu help` prints |
| [D0003](#d0003) | `cli.unknown-command` | unknown command `{command}`; run `sisu help` |
| [D0004](#d0004) | `cli.missing-file` | `sisu {command}` needs a file |
| D0005 | `cli.no-main` | `{file}` has no `main` function |
| [D0006](#d0006) | `cli.errors-found` | the program was not run: errors found ({count}) |
| D0007 | `cli.test-result` | test {name} ... {outcome} |
| D0008 | `cli.test-summary` | {passed} passed, {failed} failed, {blocked} blocked |
| [D0009](#d0009) | `cli.unknown-option` | unknown option `{option}` for `sisu {command}` |
| [D0010](#d0010) | `cli.explain-unknown` | there is no diagnostic `{code}` |
| D0011 | `cli.explain` | {code} ({kind}) {id}: {template} |
| D0012 | `cli.ice` | internal compiler error in {stage}: {detail} the intermediate form was written to {dump} |
| D0013 | `cli.cannot-write` | cannot write `{path}`: {reason} |
| D0014 | `cli.explain-borrow-header` | borrows live on line {line} of {file}, in `{function}`: |
| D0015 | `cli.explain-borrow-loan` | {kind} borrow of `{place}` from {start}, held by {holders}, used until {end} |
| D0016 | `cli.explain-borrow-temporary` | {kind} borrow of `{place}` from {start}, held by a temporary, used until {end} |
| D0017 | `cli.explain-borrow-none` | no borrow is live on line {line} of {file} |
| D0018 | `cli.explain-borrow-usage` | `sisu explain-borrow` takes FILE:LINE, as in `sisu explain-borrow main.sisu:12` |
| D0019 | `cli.explain-borrow-shared` | a shared |
| D0020 | `cli.explain-borrow-mut` | a `mut` |
| [D0021](#d0021) | `cli.ownership-errors` | the program was not run: the ownership checker found errors ({count}); `--defer-borrow` runs it under the monitor |
| D0022 | `cli.audit-usage` | `sisu audit` needs one of its audits: `sisu audit --ownership FILE...` or `sisu audit --caps FILE...` |
| [D0023](#d0023) | `cli.bad-option-value` | `{option}` needs a number, not `{value}` |
| D0024 | `cli.explore-ok` | explored {count} schedules (seeds {first} to {last}): every run exited with status 0 |
| D0025 | `cli.explore-failed` | schedule seed {seed} of {count} failed with exit status {status}; `--schedule-seed {seed}` runs it again |
| D0026 | `cli.explore-test-failed` | with schedule seed {seed}; `--schedule-seed {seed}` runs it again |
| D0027 | `cli.explore-summary` | explored {count} schedules per test (seeds {first} to {last}) |
| D0028 | `cli.bad-contract-level` | `--contracts` takes `off`, `preconditions` or `all`, not `{value}` |
| D0029 | `cli.test-checks-contracts` | `sisu test` checks every contract; `--contracts` sets what `sisu run` and `sisu sir` check |
| D0030 | `cli.replay-token` | replay: sisu test --replay {token} {file} |
| D0031 | `cli.replay-no-token` | no replay token: the test takes the real `System`, which a replay cannot stand in for |
| D0032 | `cli.bad-replay-token` | `{token}` is not a replay token |
| D0033 | `cli.token-other-code` | the token is for other code (content hash {expected}); `{file}` has content hash {found} |
| D0034 | `cli.other-toolchain` | {what} was made by toolchain {expected}, and this is {found} |
| D0035 | `cli.token-no-test` | the token names test {index}, and `{file}` has {count} |
| D0036 | `cli.bad-log` | `{path}` is not a capability log: {reason} |
| [D0037](#d0037) | `cli.replay-diverged` | the replay diverged from the log: {detail} |
| D0038 | `cli.record-once` | `--record` records one run; it does not combine with `{option}` |
| D0039 | `cli.bad-redact` | `--redact` takes capability names (fs, net, clock, rand, env, proc, stdin), not `{value}` |
| D0040 | `cli.replay-status` | the replay exited with status {found}, and the recorded run with {expected} |
| D0041 | `cli.bad-scheduler` | `--scheduler` takes `random`, `pct` or `exhaustive`, not `{value}` |
| D0042 | `cli.pct-failed` | schedule seed {seed} of {count} failed with exit status {status} under a PCT scheduler of depth {depth}; `--scheduler pct --depth {depth} --schedule-seed {seed}` runs it again |
| D0043 | `cli.pct-test-failed` | with schedule seed {seed} of a PCT scheduler of depth {depth}; `--scheduler pct --depth {depth} --schedule-seed {seed}` runs it again |
| D0044 | `cli.pct-summary` | explored {count} schedules of a PCT scheduler of depth {depth} per test (seeds {first} to {last}) |
| D0045 | `cli.pct-ok` | explored {count} schedules of a PCT scheduler of depth {depth} (seeds {first} to {last}): every run exited with status 0 |
| D0046 | `cli.exhaustive-failed` | schedule {schedule} failed with exit status {status}: run {count} of a bounded exhaustive search (preemption bound {bound}) |
| D0047 | `cli.exhaustive-ok` | searched {count} schedules (preemption bound {bound}): every run exited with status 0{stopped} |
| D0048 | `cli.exhaustive-test-failed` | with schedule {schedule} of a bounded exhaustive search (preemption bound {bound}) |
| D0049 | `cli.exhaustive-summary` | searched {count} schedules (preemption bound {bound}){stopped} |
| D0050 | `cli.exhaustive-stopped` | , stopping at {cap} schedules |
| D0051 | `cli.bad-fault-rate` | `--faults` takes a probability from 0 to 1, as in `--faults 0.02`, not `{value}` |
| [D0052](#d0052) | `cli.faults-without-sim` | `--faults` sets the fault rate of the `sim` profile; add `--sim` |
| D0053 | `cli.tests-unselected` | {count} not selected |
| D0054 | `cli.snapshot-updated` | updated the snapshot `{name}`: {diff} |
| D0055 | `cli.snapshot-new` | stored the new snapshot `{name}`: {text} |
| D0056 | `cli.snapshot-unwritable` | cannot write the snapshots to `{path}`: {reason} |
| D0057 | `cli.bench-result` | bench {name} ... {iterations} iterations, {each} each |
| D0058 | `cli.bench-failed` | bench {name} ... FAILED {message} |
| D0059 | `cli.bench-summary` | {count} benchmarks, {failed} failed (stage 0 times the interpreter: compare benchmarks with each other, not with compiled code) |
| D0060 | `cli.transcript-result` | transcript {name} ... {outcome} |
| D0061 | `cli.transcript-differs` | what the program printed differs from the transcript (`sisu test --update` accepts it): {diff} |
| D0062 | `cli.transcript-no-main` | a transcript runs the program's `main`, and `{file}` has none |
| D0063 | `cli.transcript-updated` | updated the transcript {name}: {diff} |
| D0064 | `cli.no-package` | there is no `sisu.toml` in `{dir}` or a directory above it; `sisu {command}` without a file works on a package |
| D0065 | `cli.locked` | `sisu.lock` is not up to date, and `--locked` keeps it as it is: {what} |
| D0066 | `cli.built` | built the package `{name}` {version} (modules: {modules}) |
| D0067 | `cli.workspace-run` | `sisu {command}` in a workspace needs one member's application; name the member: {members} |
| D0069 | `cli.no-application` | the package `{name}` is a library (`src/lib.sisu`); `sisu run` runs an application, whose root module is `src/main.sisu` |
| D0070 | `cli.built-file` | built `{file}` |
| D0071 | `cli.created` | created the package `{name}`, with its manifest `sisu.toml` and its root module `{root}` |
| [D0072](#d0072) | `cli.bad-package-name` | `{name}` cannot name a package: a package is named in lowercase letters, digits and `_`, starting with a letter, and not as a standard-library module |
| [D0073](#d0073) | `cli.exists` | `{path}` exists already |
| D0074 | `cli.fmt-would-change` | `{file}` is not formatted; `sisu fmt` would change it: {diff} |
| D0075 | `cli.fmt-syntax` | `{file}` has syntax errors, so it was not formatted |
| D0076 | `cli.fmt-unstable` | internal error: the formatted `{file}` would not parse to the same program (from byte {at}); it was left as it is |
| D0077 | `cli.fmt-nothing` | there is no `.sisu` file to format in `{path}` |
| D0078 | `cli.watch-alone` | `--watch` runs the program once, swapping in each save; it does not combine with `{option}` |
| D0079 | `cli.repl-banner` | sisu repl: each input runs as the next line of a script; `:back N` forgets the last N inputs, `:why NAME` shows where a value came from, `:quit` ends |
| D0080 | `cli.repl-loaded` | the package `{name}` is loaded: import its modules by their paths (`from {name} import ...`) |
| D0081 | `cli.repl-prompt` | sisu> |
| D0082 | `cli.repl-more` | ... |
| D0083 | `cli.repl-not-kept` | the input did not run to its end, so the session is as it was before it |
| D0084 | `cli.repl-back` | forgot the last inputs ({forgotten}); the session keeps {count} |
| D0085 | `cli.repl-back-too-far` | there are not that many inputs to forget; the session has {count} |
| D0086 | `cli.repl-bad-command` | `:{command}` is not a command of the REPL; the commands are `:back N`, `:why NAME` and `:quit` |
| D0087 | `cli.repl-why-value` | {name} = {value}  <- {expr}  (input {input}) |
| D0088 | `cli.repl-why-call` | {call}  (log #{record}) |
| D0089 | `cli.repl-why-change` | then {text}  (input {input}) |
| D0090 | `cli.repl-why-unbound` | no input of the session binds `{name}` |
| D0091 | `cli.repl-diverged` | the session's earlier inputs did not replay as they ran ({detail}); its values may differ from what they were |
| D0092 | `cli.ws-value` | # => {value} |
| D0093 | `cli.ws-output` | # \| {text} |
| D0094 | `cli.bad-stats` | `--stats` takes `stack-depth` or `shared` (or nothing, for the scheduler's numbers), not `{value}` |
| D0095 | `lsp.not-open` | the document is not open |
| D0096 | `lsp.not-served` | `{method}` is not served |
| D0097 | `lsp.not-a-command` | `{command}` is not a command of this server |
| D0098 | `lsp.not-a-name` | `{name}` is not a name |
| D0099 | `lsp.cannot-rename` | nothing here can be renamed: a local, or a definition of the program's own |
| D0100 | `lsp.not-analysed` | the document is not analysed |
| D0101 | `lsp.has-errors` | the program has errors: fix them to see its borrows |
| D0102 | `lsp.keyword` | keyword |
| D0103 | `native.unsupported` | the native backend does not compile {what} yet (in `{function}`); `sisu run` without `--native` runs the program |
| D0104 | `native.no-runtime` | the native runtime `libsisu_rt.a` is not beside `sisu` (in `{dir}`); `cargo build` in `compiler/` builds it |
| D0105 | `native.link-failed` | linking `{output}` failed: {detail} |
| D0106 | `native.built` | built `{output}` |
| D0107 | `native.internal` | internal error in the native backend: {detail} |
| D0108 | `native.cannot-run` | cannot run `{path}`: {reason} |
| D0109 | `native.alone` | `--native` builds a native executable and runs it; it does not combine with `{option}` |
| D0110 | `native.from-sir` | `sisu build --from-sir FILE` compiles a serialised SIR program with the native backend; it goes with `--native`, and `sisu run --from-sir FILE` interprets it |
| [D0111](#d0111) | `native.release` | `--release` builds with the native backend's release profile (09); it goes with `--native` |
| D0112 | `cli.unknown-feature` | `--features` names `{name}`, which is not a feature of `{package}`; a package's features are the keys of its `[features]` |
| [D0113](#d0113) | `doc.std-path` | `{path}` is not a module of the standard library; `sisu doc --std` takes the paths an import names them by, as `std.time` |
| D0114 | `doc.errors-found` | the documentation was not written: errors found ({count}) |
| [D0115](#d0115) | `cli.bad-front-end` | `--front-end` takes `stage1` (the front end written in Sisu, the default) or `rust` (the Rust front end, the oracle), not `{value}` |
| D0116 | `front.no-stage1` | the stage-1 front end is not in `{dir}`; `--front-end=rust` uses the Rust front end |
| D0117 | `front.stage1-build` | building the stage-1 front end in `{dir}` failed; `--front-end=rust` uses the Rust front end: {detail} |
| D0118 | `front.stage1-failed` | internal compiler error: the stage-1 front end stopped with status {status}; `--front-end=rust` uses the Rust front end |
| D0119 | `native.from-lir` | `--from-lir FILE` builds a LIR program (what stage 2's `sisu2 lir` writes) with the native backend; it goes with `--native` |

