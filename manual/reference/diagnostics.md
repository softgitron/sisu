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
| E04xx | ownership: moves, borrows, views, tasks, must-consume values; and, from E0445, types again: holes in a release build, parallel loops, `@float` |
| E05xx | lowering to SIR: errors of the compiler's own, which a run reports and `sisu check` never does |
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

Shown in [chapter 1](../tutorial/01-hello.md).

### E0002

`lex.inconsistent-dedent`: this line's indentation matches no enclosing
block; the enclosing blocks start at column {columns}.

A line that is indented less than the one before closes blocks until its
indentation matches an enclosing block's. When it matches none, the
message names the columns that would.

<!-- file: examples/reference-diagnostics/e0002_dedent.sisu -->
```sisu
# A line that matches no enclosing block's indentation.
def main():
    let n = 2
    if n > 1:
        print("big")
      print("done")
```

<!-- output: examples/reference-diagnostics/e0002_dedent.console -->
```console
$ sisu run e0002_dedent.sisu
error[E0002]: this line's indentation matches no enclosing block; the enclosing blocks start at column 1 or 5
  --> e0002_dedent.sisu:6:1
  |
6 |       print("done")
  | ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Indent the line to one of the columns the message names: the block it
belongs to.

<!-- file: examples/reference-diagnostics/e0002_dedent_fixed.sisu -->
```sisu
# The line back at the indentation of its block.
def main():
    let n = 2
    if n > 1:
        print("big")
    print("done")
```

<!-- output: examples/reference-diagnostics/e0002_dedent_fixed.out -->
```text
big
done
```

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

The fix closes the literal on its line, before the brackets that the
line opened ahead of the literal: here before the `)`, where the quote
belongs.

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

Shown in [chapter 1](../tutorial/01-hello.md).

### E0005

`lex.unknown-character`: `{char}` cannot appear in Sisu source outside
a string or comment.

Outside strings and comments, a source file holds only the characters
its tokens use ([R1](lexical-structure.md#source-text)). A character
that shows as a space or as nothing, often pasted from a web page or a
word processor, is written as its escape, with the note
`lex.invisible-character` that names it, and a fix that writes a space
(`lex.write-space`) or removes it (`lex.remove-character`). A
byte-order mark at the start of a file is named as one.

<!-- file: examples/reference-diagnostics/e0005_invisible.sisu -->
```sisu
# A zero-width space after a name, invisible in an editor.
def main():
    let sauna​ = 80
    print(sauna)
```

<!-- output: examples/reference-diagnostics/e0005_invisible.console -->
```console
$ sisu run e0005_invisible.sisu
error[E0005]: `\u{200b}` cannot appear in Sisu source outside a string or comment
  --> e0005_invisible.sisu:3:14
  |
3 |     let sauna​ = 80
  |              ^
  = note: it is a zero-width space, which shows as nothing
  = fix: remove it

error[D0006]: the program was not run: errors found (1)
```

The line looks right, since the character shows as nothing; the escape
`\u{200b}` and the note say what is there. Remove it, as the fix does.

<!-- file: examples/reference-diagnostics/e0005_invisible_fixed.sisu -->
```sisu
# The name without the character that showed as nothing.
def main():
    let sauna = 80
    print(sauna)
```

<!-- output: examples/reference-diagnostics/e0005_invisible_fixed.out -->
```text
80
```

Shown in [R1](lexical-structure.md#source-text).

### E0011

`lex.semicolon`: Sisu has no semicolons; put each statement on its own
line.

A statement takes one line, and a line break ends it; Sisu has no
semicolons (D-020).

<!-- file: examples/reference-diagnostics/e0011_semicolon.sisu -->
```sisu
# Two statements on one line.
def main():
    let a = 1; let b = 2
    print(a + b)
```

<!-- output: examples/reference-diagnostics/e0011_semicolon.console -->
```console
$ sisu run e0011_semicolon.sisu
error[E0011]: Sisu has no semicolons; put each statement on its own line
  --> e0011_semicolon.sisu:3:14
  |
3 |     let a = 1; let b = 2
  |              ^

error[D0006]: the program was not run: errors found (1)
```

Put each statement on its own line.

<!-- file: examples/reference-diagnostics/e0011_semicolon_fixed.sisu -->
```sisu
# One statement per line.
def main():
    let a = 1
    let b = 2
    print(a + b)
```

<!-- output: examples/reference-diagnostics/e0011_semicolon_fixed.out -->
```text
3
```

### E0012

`lex.invalid-number`: `{text}` is not a valid number literal.

A number literal is digits, with `_` between them, a prefix `0x`, `0b`
or `0o`, and a suffix that names a numeric type
([R1](lexical-structure.md#integer-literals)).

<!-- file: examples/reference-diagnostics/e0012_number.sisu -->
```sisu
# A number with letters that are not a suffix.
def main():
    print(12abc)
```

<!-- output: examples/reference-diagnostics/e0012_number.console -->
```console
$ sisu run e0012_number.sisu
error[E0012]: `12abc` is not a valid number literal
  --> e0012_number.sisu:3:11
  |
3 |     print(12abc)
  |           ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write the number without the letters, or with a suffix such as `u8` or
`f32`.

<!-- file: examples/reference-diagnostics/e0012_number_fixed.sisu -->
```sisu
# A number, and a suffix that names a type.
def main():
    print(12u8)
```

<!-- output: examples/reference-diagnostics/e0012_number_fixed.out -->
```text
12
```

### E0015

`lex.backslash-continuation`: Sisu has no backslash continuation; break
the line inside brackets, after a comma or before an operator.

A logical line continues inside brackets, after a comma or an open
bracket, and before a binary operator; a backslash does not continue it
(D-020).

<!-- file: examples/reference-diagnostics/e0015_backslash.sisu -->
```sisu
# A backslash at the end of a line.
def main():
    let total = 1 + \
        2
    print(total)
```

<!-- output: examples/reference-diagnostics/e0015_backslash.console -->
```console
$ sisu run e0015_backslash.sisu
error[E0015]: Sisu has no backslash continuation; break the line inside brackets, after a comma or before an operator
  --> e0015_backslash.sisu:3:21
  |
3 |     let total = 1 + \
  |                     ^

error[D0006]: the program was not run: errors found (1)
```

Join the lines, or break the line in one of the three ways the message
names; `sisu fmt` joins it again when it fits in 100 columns.

<!-- file: examples/reference-diagnostics/e0015_backslash_fixed.sisu -->
```sisu
# The expression on one line, as `sisu fmt` writes it.
def main():
    let total = 1 + 2
    print(total)
```

<!-- output: examples/reference-diagnostics/e0015_backslash_fixed.out -->
```text
3
```

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
  = fix: text is written in double quotes: `"`, `"`

error[D0006]: the program was not run: errors found (1)
```

The fix writes double quotes in place of the single ones, which is
what text needs; keep single quotes for one character, such as `'A'`.

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

Shown in [chapter 2](../tutorial/02-values.md).

## Parsing

Codes E0100 to E0132. The parser builds the syntax tree. Its errors name
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
that opens the body here. A generator expression as a call's only
argument without parentheses of its own, Python's `sum(p * p for p in
primes)`, gets the note `parse.generator-parens` and the fix that adds
them ([chapter 16](../tutorial/16-closures-and-iterators.md)). `F[_]`
in a generic parameter list, the syntax that 03 reserves for
higher-kinded types, gets the note `parse.higher-kinded`, which says
that a generic parameter is a type, `F`
([R2](types.md#what-the-type-system-leaves-out)).

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

Shown in [chapter 1](../tutorial/01-hello.md).

### E0102

`parse.block-on-same-line`: a block starts on the next line, indented;
nothing may follow `:` on the line that opens it.

A block starts on the line after the `:` that opens it, indented
(D-027).

<!-- file: examples/reference-diagnostics/e0102_same_line.sisu -->
```sisu
# A block on the line that opens it.
def main():
    for n in [1, 2]: print(n)
```

<!-- output: examples/reference-diagnostics/e0102_same_line.console -->
```console
$ sisu run e0102_same_line.sisu
error[E0102]: a block starts on the next line, indented; nothing may follow `:` on the line that opens it
  --> e0102_same_line.sisu:3:22
  |
3 |     for n in [1, 2]: print(n)
  |                      ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Move what follows the `:` to the next line and indent it.

<!-- file: examples/reference-diagnostics/e0102_same_line_fixed.sisu -->
```sisu
# The block on the next line, indented.
def main():
    for n in [1, 2]:
        print(n)
```

<!-- output: examples/reference-diagnostics/e0102_same_line_fixed.out -->
```text
1
2
```

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

Shown in [chapter 2](../tutorial/02-values.md).

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
  = note: a pattern says what a value is, not what it is not: write the case it is first (`case 0:`), then the rest with `case _:`, or test it in a guard, `case n if n != 0:`

error[D0006]: the program was not run: errors found (1)
```

The note `parse.pattern-not` says what a pattern is: what a value is,
never what it is not. Write the case as a pattern; a `case _` arm, or a
guard `case n if n != 0`, says "any other value". A keyword used as a
binding's name, `let type = "csv"`, gets the note `parse.keyword-name`
instead ([R1](lexical-structure.md#names)).

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0106

`parse.expected-declaration`: expected a declaration, found {found}.

Attributes and a visibility, `@NAME` and `pub`, stand before a
declaration; with none after them, the parser has nothing to attach
them to. A `pub` before a `let`, at the top level of a module, is the
common case: a module gives other modules functions, types and
constants, and a `let` is a statement of a function. An attribute before
a statement inside a function is E0129.

<!-- file: examples/reference-diagnostics/e0106_pub_let.sisu -->
```sisu
# A visibility before a line that declares nothing.
pub let total = 3 + 4

def main():
    print(total)
```

<!-- output: examples/reference-diagnostics/e0106_pub_let.console -->
```console
$ sisu run e0106_pub_let.sisu
error[E0106]: expected a declaration, found `let`
  --> e0106_pub_let.sisu:2:5
  |
2 | pub let total = 3 + 4
  |     ^^^

error[D0006]: the program was not run: errors found (1)
```

Declare the value a constant, whose type is written and whose name is
in capitals ([R3](declarations.md)):

<!-- file: examples/reference-diagnostics/e0106_pub_let_fixed.sisu -->
```sisu
# A constant is what a module gives other modules as a value.
pub const TOTAL: int = 3 + 4

def main():
    print(TOTAL)
```

<!-- output: examples/reference-diagnostics/e0106_pub_let_fixed.out -->
```text
7
```

### E0107

`parse.top-level-statement`: statements cannot appear at the top level
of a module; put them in a function such as `main`.

A module holds declarations; statements belong in functions. A script, a
file that starts with `#!`, is the exception: its top-level statements
are its `main`. Python's `class` is reported here too, since it is not a
declaration, with the fix that writes `struct`.

<!-- file: examples/reference-diagnostics/e0107_top_level.sisu -->
```sisu
# A statement outside every function.
print("Hei")

def main():
    print("maailma")
```

<!-- output: examples/reference-diagnostics/e0107_top_level.console -->
```console
$ sisu run e0107_top_level.sisu
error[E0107]: statements cannot appear at the top level of a module; put them in a function such as `main`
  --> e0107_top_level.sisu:2:1
  |
2 | print("Hei")
  | ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Move the statement into `main` or another function, or make the file a
script.

<!-- file: examples/reference-diagnostics/e0107_top_level_fixed.sisu -->
```sisu
# The statement inside `main`.
def main():
    print("Hei")
    print("maailma")
```

<!-- output: examples/reference-diagnostics/e0107_top_level_fixed.out -->
```text
Hei
maailma
```

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0110

`parse.bitwise-arithmetic`: `{op}` cannot be mixed with arithmetic or
shifts without parentheses.

`&`, `^` and `|` are not mixed with arithmetic or shifts without
parentheses: languages order them differently (C puts `&` below `==`,
Python above it), so the parentheses say which one a reader should not
have to remember ([R4](expressions.md#precedence)).

<!-- file: examples/reference-diagnostics/e0110_bitwise.sisu -->
```sisu
# A bit mask mixed with arithmetic, without parentheses.
def main():
    let flags = 6
    print(flags & 1 + 2)
```

<!-- output: examples/reference-diagnostics/e0110_bitwise.console -->
```console
$ sisu run e0110_bitwise.sisu
error[E0110]: `&` cannot be mixed with arithmetic or shifts without parentheses
  --> e0110_bitwise.sisu:4:24
  |
4 |     print(flags & 1 + 2)
  |                        ^
  = fix: add parentheses: `(`, `)`

error[D0006]: the program was not run: errors found (1)
```

The fix adds the parentheses around the operation the parser took first,
here `1 + 2`. Move them if the program meant the other order, as the
fixed program does.

<!-- file: examples/reference-diagnostics/e0110_bitwise_fixed.sisu -->
```sisu
# Parentheses say which operation comes first.
def main():
    let flags = 6
    print((flags & 1) + 2)
```

<!-- output: examples/reference-diagnostics/e0110_bitwise_fixed.out -->
```text
2
```

Shown in [R4](expressions.md#precedence).

### E0112

`parse.negative-power`: `-x ** y` is ambiguous; write `-(x ** y)` or
`(-x) ** y`.

In mathematics and in Python, `-n ** 2` is `-(n ** 2)`; in other
languages a minus sign binds tighter. Sisu asks for parentheses, so the
expression means one thing to every reader.

<!-- file: examples/reference-diagnostics/e0112_negative_power.sisu -->
```sisu
# A minus sign before a power, without parentheses.
def main():
    let n = 3
    print(-n ** 2)
```

<!-- output: examples/reference-diagnostics/e0112_negative_power.console -->
```console
$ sisu run e0112_negative_power.sisu
error[E0112]: `-x ** y` is ambiguous; write `-(x ** y)` or `(-x) ** y`
  --> e0112_negative_power.sisu:4:18
  |
4 |     print(-n ** 2)
  |                  ^
  = fix: add parentheses: `(`, `)`

error[D0006]: the program was not run: errors found (1)
```

The fix writes the parentheses of one reading; choose the one the
program means. The fixed program prints both.

<!-- file: examples/reference-diagnostics/e0112_negative_power_fixed.sisu -->
```sisu
# Parentheses say whether the minus applies before the power.
def main():
    let n = 3
    print(-(n ** 2))
    print((-n) ** 2)
```

<!-- output: examples/reference-diagnostics/e0112_negative_power_fixed.out -->
```text
-9
9
```

Shown in [R4](expressions.md#precedence).

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

Shown in [chapter 2](../tutorial/02-values.md).

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

Shown in [chapter 3](../tutorial/03-functions.md).

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

Shown in [chapter 3](../tutorial/03-functions.md).

### E0117

`parse.trailing-tokens`: unexpected {found} after the end of the
statement.

After a statement ends, only a line break may follow. The parser reports
the first token it did not expect, which often stands for a habit from
another language: a Rust comment `//`, `lambda`, `println!`. Where it
stops at such a habit, a fix writes Sisu's spelling (`#`, `x ->`,
`print`) or a note says what Sisu has instead (`try:`, Go's `go f()`);
[R19](false-friends.md) shows each. An `else` after a call that is no
`let`, `save(4) else as e:`, gets the note that `else` belongs to a
`let`, and with `as e` the fix `if let Err(e) = save(4)`, the statement
that handles the error alone. Python's `f"..."` gets the fix that drops
the `f` both as a call's argument and after `=`, `let s = f"n is {n}"`.

<!-- file: examples/reference-diagnostics/e0117_trailing.sisu -->
```sisu
# Two statements on one line, without a semicolon.
def main():
    let n = 3 print(n)
```

<!-- output: examples/reference-diagnostics/e0117_trailing.console -->
```console
$ sisu run e0117_trailing.sisu
error[E0117]: unexpected `print` after the end of the statement
  --> e0117_trailing.sisu:3:15
  |
3 |     let n = 3 print(n)
  |               ^^^^^

error[D0006]: the program was not run: errors found (1)
```

End the statement where it ends, and put what follows on a line of its
own, in Sisu's spelling.

<!-- file: examples/reference-diagnostics/e0117_trailing_fixed.sisu -->
```sisu
# One statement per line.
def main():
    let n = 3
    print(n)
```

<!-- output: examples/reference-diagnostics/e0117_trailing_fixed.out -->
```text
3
```

### E0123

`parse.walrus`: Sisu has no `:=`; bind with `let` or use `if let`.

Sisu has no `:=`: a name is declared with `let` or `var`, and a value
bound inside a condition is written with `if let` or a chained condition
([R5](statements.md)). The statement that `:=` would have declared is
not read, so each use of its name is E0200 too.

<!-- file: examples/reference-diagnostics/e0123_walrus.sisu -->
```sisu
# `:=` from Go, which declares a name and gives it a value.
def main():
    count := 3
    print("{count} saunas")
```

<!-- output: examples/reference-diagnostics/e0123_walrus.console -->
```console
$ sisu run e0123_walrus.sisu
error[E0123]: Sisu has no `:=`; bind with `let` or use `if let`
  --> e0123_walrus.sisu:3:11
  |
3 |     count := 3
  |           ^^

error[E0200]: cannot find `count` in this scope
  --> e0123_walrus.sisu:4:13
  |
4 |     print("{count} saunas")
  |             ^^^^^

error[D0006]: the program was not run: errors found (2)
```

Write `let`, or `var` for a name that changes. Python's walrus in a
condition, `if (n := len(xs)) > 2:`, becomes `let n = len(xs)` on the
line before the `if`.

<!-- file: examples/reference-diagnostics/e0123_walrus_fixed.sisu -->
```sisu
# `let` declares a name and gives it a value.
def main():
    let count = 3
    print("{count} saunas")
```

<!-- output: examples/reference-diagnostics/e0123_walrus_fixed.out -->
```text
3 saunas
```

Shown in the guide [Coming from Go](../guides/from-go.md).

### E0124

`parse.keyword-as-name`: `{keyword}` is a keyword and cannot be used as
a name here.

A keyword is not a name, except after `.`, `?.` and `def`
([R1](lexical-structure.md#keywords)).

<!-- file: examples/reference-diagnostics/e0124_keyword.sisu -->
```sisu
# A keyword used as a parameter's name.
def score(match: str) -> int:
    return 0

def main():
    print(score("final"))
```

<!-- output: examples/reference-diagnostics/e0124_keyword.console -->
```console
$ sisu run e0124_keyword.sisu
error[E0124]: `match` is a keyword and cannot be used as a name here
  --> e0124_keyword.sisu:2:11
  |
2 | def score(match: str) -> int:
  |           ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Choose another name. A keyword elsewhere where a name should be gets the
parser's E0100, E0103 or E0105 instead.

<!-- file: examples/reference-diagnostics/e0124_keyword_fixed.sisu -->
```sisu
# A parameter named with a name.
def score(game: str) -> int:
    return len(game)

def main():
    print(score("final"))
```

<!-- output: examples/reference-diagnostics/e0124_keyword_fixed.out -->
```text
5
```

### E0127

`parse.loop-else`: a loop has no `else` in Sisu; keep what the loop
finds in a variable and test it after the loop, as in `if let Some(i) =
found:`.

Python runs a loop's `else` when the loop ends without `break`. Sisu
has no `else` on a loop (D-021), and says so where one starts.

<!-- file: examples/reference-diagnostics/e0127_loop_else.sisu -->
```sisu
# Python's `else` after a loop, for when no item was found.
def main():
    let temps = [62, 75, 81]
    for t in temps:
        if t > 90:
            print("too hot: {t}")
            break
    else:
        print("no reading above 90")
```

<!-- output: examples/reference-diagnostics/e0127_loop_else.console -->
```console
$ sisu run e0127_loop_else.sisu
error[E0127]: a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:`
  --> e0127_loop_else.sisu:8:5
  |
8 |     else:
  |     ^^^^

error[D0006]: the program was not run: errors found (1)
```

Keep what the loop finds in an optional value, and test it after the
loop.

<!-- file: examples/reference-diagnostics/e0127_loop_else_fixed.sisu -->
```sisu
# What the loop finds is kept, and tested after it.
def main():
    let temps = [62, 75, 81]
    var found: int? = None
    for t in temps:
        if t > 90:
            found = Some(t)
            break
    if let Some(t) = found:
        print("too hot: {t}")
    else:
        print("no reading above 90")
```

<!-- output: examples/reference-diagnostics/e0127_loop_else_fixed.out -->
```text
no reading above 90
```

Shown in [chapter 4](../tutorial/04-control-flow.md).

### E0128

`parse.glob-import`: Sisu has no glob imports: name what the module
gives, as in `from std.collections import Deque, Heap`, or import the
module and write `collections.Deque`.

An import names each item it brings in, or the module itself, so that a
reader of a file can find where each name comes from: glob imports make
name resolution unreadable, as
[09](../../proposal/09-modules-packages-and-build.md) found in Rust and
Python, and a variant is written unqualified where its type is known
instead ([R13](modules.md#imports)).

<!-- file: examples/reference-diagnostics/e0128_glob_import.sisu -->
```sisu
# A glob import, which Sisu does not have.
from std.math import *

def main():
    print(floor(sqrt(20.0)))
```

<!-- output: examples/reference-diagnostics/e0128_glob_import.console -->
```console
$ sisu run e0128_glob_import.sisu
error[E0128]: Sisu has no glob imports: name what the module gives, as in `from std.collections import Deque, Heap`, or import the module and write `collections.Deque`
  --> e0128_glob_import.sisu:2:22
  |
2 | from std.math import *
  |                      ^

error[E0200]: cannot find `floor` in this scope
  --> e0128_glob_import.sisu:5:11
  |
5 |     print(floor(sqrt(20.0)))
  |           ^^^^^

error[E0200]: cannot find `sqrt` in this scope
  --> e0128_glob_import.sisu:5:17
  |
5 |     print(floor(sqrt(20.0)))
  |                 ^^^^

error[D0006]: the program was not run: errors found (3)
```

Name the items the module gives, as the message shows, or import the
module and write its name before each item, as `math.sqrt`. Each name
the glob would have brought in is E0200 where it is used.

<!-- file: examples/reference-diagnostics/e0128_glob_import_fixed.sisu -->
```sisu
# The import names what the module gives.
from std.math import floor, sqrt

def main():
    print(floor(sqrt(20.0)))
```

<!-- output: examples/reference-diagnostics/e0128_glob_import_fixed.out -->
```text
4.0
```

Shown in [R13](modules.md).

### E0129

`parse.attribute-on-statement`: `@{name}` stands before a statement; an
attribute stands on the line before a declaration (a function, a type,
a constant, a test), and none applies to a statement.

An attribute stands before a declaration; inside a block, the line
after it is a statement, which no attribute applies to. The parser
names the attribute and reads the statement as it would without it.
`@tile`, `@unroll` and `@vectorize` before a loop get a note: they are
the checked directives that
[19](../../proposal/19-novel-features.md) plans with the native
backend, not in the language yet.

<!-- file: examples/reference-diagnostics/e0129_statement_attribute.sisu -->
```sisu
# An attribute before a statement, which no attribute applies to.
def main():
    @deprecated("use `total`")
    let total = 3 + 4
    print(total)
```

<!-- output: examples/reference-diagnostics/e0129_statement_attribute.console -->
```console
$ sisu run e0129_statement_attribute.sisu
error[E0129]: `@deprecated` stands before a statement; an attribute stands on the line before a declaration (a function, a type, a constant, a test), and none applies to a statement
  --> e0129_statement_attribute.sisu:3:5
  |
3 |     @deprecated("use `total`")
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Put the attribute before a declaration, or leave it out: a statement
takes none.

<!-- file: examples/reference-diagnostics/e0129_statement_attribute_fixed.sisu -->
```sisu
# The attribute stands before a declaration.
@deprecated("use `total`")
def seven() -> int:
    return 3 + 4

def main():
    print(seven())
```

<!-- output: examples/reference-diagnostics/e0129_statement_attribute_fixed.console -->
```console
$ sisu run e0129_statement_attribute_fixed.sisu
warning[W0023]: `seven` is deprecated: use `total`
  --> e0129_statement_attribute_fixed.sisu:7:11
  |
7 |     print(seven())
  |           ^^^^^^^

7
```

The call of `seven` now warns (W0023), which is what the attribute is
for.

Shown in [R14](attributes.md#writing-an-attribute).

### E0130

`parse.visibility-scope`: `pub({name})` is not Sisu's: an item is `pub`
to every package, or `pub(package)` to the modules of its own.

`pub(package)` is the one name that stands between the parentheses of
`pub` ([R13](modules.md#visibility)). Rust's `pub(crate)` would mean
the same as `pub(package)`, and `pub(super)` and `pub(self)` have no
meaning in Sisu, so each is an error rather than a guess.

<!-- file: examples/reference-diagnostics/e0130_pub_crate.files/e0130_pub_crate.sisu -->
```sisu
# Rust's visibility of a crate.
pub(crate) def heat() -> int:
    return 80

def main():
    print(heat())
```

<!-- output: examples/reference-diagnostics/e0130_pub_crate.console -->
```console
$ sisu run e0130_pub_crate.sisu
error[E0130]: `pub(crate)` is not Sisu's: an item is `pub` to every package, or `pub(package)` to the modules of its own
  --> e0130_pub_crate.sisu:2:5
  |
2 | pub(crate) def heat() -> int:
  |     ^^^^^
  = fix: make it visible to its package: `package`

error[D0006]: the program was not run: errors found (1)
```

The fix writes `package` between the parentheses, which is what
`pub(crate)` means in Rust; write `pub` for an item that other packages
use.

<!-- file: examples/reference-diagnostics/e0130_pub_crate_fixed.sisu -->
```sisu
# `pub(package)` shows the function to its own package only.
pub(package) def heat() -> int:
    return 80

def main():
    print(heat())
```

<!-- output: examples/reference-diagnostics/e0130_pub_crate_fixed.out -->
```text
80
```

Shown in [R19](false-friends.md#pubcrate).

### E0131

`parse.attribute-line`: an attribute stands on a line of its own, before
its declaration.

An attribute is a line of its own, so that a declaration's first line
starts with its keyword wherever it has attributes
([R14](attributes.md#writing-an-attribute)). The parser reads the
declaration after the attribute as if the line were broken, so the
program's other errors are still found.

<!-- file: examples/reference-diagnostics/e0131_attribute_line.sisu -->
```sisu
# An attribute on the line of its declaration.
@derive(Eq) struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) == Point(1, 2))
```

<!-- output: examples/reference-diagnostics/e0131_attribute_line.console -->
```console
$ sisu run e0131_attribute_line.sisu
error[E0131]: an attribute stands on a line of its own, before its declaration
  --> e0131_attribute_line.sisu:2:13
  |
2 | @derive(Eq) struct Point:
  |             ^^^^^^
  = fix: put the declaration on the next line

error[D0006]: the program was not run: errors found (1)
```

The fix breaks the line before the declaration.

<!-- file: examples/reference-diagnostics/e0131_attribute_line_fixed.sisu -->
```sisu
# The attribute stands on the line before its declaration.
@derive(Eq)
struct Point:
    x: int
    y: int

def main():
    print(Point(1, 2) == Point(1, 2))
```

<!-- output: examples/reference-diagnostics/e0131_attribute_line_fixed.out -->
```text
True
```

Shown in [R14](attributes.md#writing-an-attribute).

### E0132

`parse.interpolated-name`: the name of a test, a suite or a benchmark
is fixed text, and `{` starts no interpolation in it; a brace is
written `{{` or `}}`.

A name is what `sisu test` reports, chooses by (`--filter`) and replays
by, so no value of the running program can be part of it
([R15](testing.md#the-forms-of-a-test)). Braces in a test's,
a suite's or a benchmark's name start no interpolation, and what they
hold is not read as an expression.

<!-- file: examples/reference-diagnostics/e0132_interpolated_name.sisu -->
```sisu
# Braces in a test's name, as if it were interpolated.
test "len({3, 1}) counts each item once":
    assert len({3, 1, 3}) == 2
```

<!-- output: examples/reference-diagnostics/e0132_interpolated_name.console -->
```console
$ sisu test e0132_interpolated_name.sisu
error[E0132]: the name of a test, a suite or a benchmark is fixed text, and `{` starts no interpolation in it; a brace is written `{{` or `}}`
  --> e0132_interpolated_name.sisu:2:11
  |
2 | test "len({3, 1}) counts each item once":
  |           ^^^^^^
  = fix: write the braces as text: `{{3, 1}}`

error[D0006]: the program was not run: errors found (1)
```

The fix writes each brace twice, as text, which the name then shows
once. An escaped quote, `\"`, is text in a name too.

<!-- file: examples/reference-diagnostics/e0132_interpolated_name_fixed.sisu -->
```sisu
# A brace in a test's name is written twice.
test "len({{3, 1}}) counts each item once":
    assert len({3, 1, 3}) == 2
```

<!-- output: examples/reference-diagnostics/e0132_interpolated_name_fixed.test -->
```text
test "len({3, 1}) counts each item once" ... ok
1 passed, 0 failed, 0 blocked
```

Shown in [chapter 19](../tutorial/19-testing.md#suites-tags-and-choosing-tests).

## Names and declarations

Codes E0200 to E0281. Name resolution finds what each name means and
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
new, declare it with `let` (or `var`) before its first use. A type that
nothing declares, called with type arguments, `Deque[int]()` without
its import, is one E0200, for `Deque`: its arguments are not read as
values.

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

A name that an import under a `@cfg` on the target binds is missing
from a build the condition leaves out, and a use of it outside an `if
comptime` branch on the same fact is E0200 with the note
`resolve.cfg-import-absent`, which names the `@cfg` and the branch that
keeps the use where the import is
([R13](modules.md#target-conditions)):

<!-- file: examples/reference-diagnostics/e0200_cfg_import.files/sauna/src/main.sisu -->
```sisu
# A name that a `@cfg` import binds, used where the build may lack it.
@cfg(target.os == "windows")
import winlog

def main():
    winlog.record("the stove is on")
```

<!-- output: examples/reference-diagnostics/e0200_cfg_import.console -->
```console
$ sisu run sauna
error[E0200]: cannot find `winlog` in this scope
  --> src/main.sisu:6:5
  |
6 |     winlog.record("the stove is on")
  |     ^^^^^^
  = note: `winlog` is imported under `@cfg(target.os == "windows")`, which this build leaves out: use it inside `if comptime target.os == "windows":`, a branch that is built only where the import is

error[D0006]: the program was not run: errors found (1)
```

The use inside the branch is the fix, as [E0275](#e0275) shows.

Shown in [chapter 1](../tutorial/01-hello.md).

### E0201

`resolve.undefined-type`: cannot find a type named `{name}`.

Where a type is expected, a name must be a type that is declared,
imported or in the prelude. The prelude holds the types every program
uses; the others come from their modules.

<!-- file: examples/reference-diagnostics/e0201_undefined_type.sisu -->
```sisu
# A type from the standard library, used without its import.
def waiting(queue: Deque[str]) -> int:
    return len(queue)

def main():
    print("no queue yet")
```

<!-- output: examples/reference-diagnostics/e0201_undefined_type.console -->
```console
$ sisu run e0201_undefined_type.sisu
error[E0201]: cannot find a type named `Deque`
  --> e0201_undefined_type.sisu:2:20
  |
2 | def waiting(queue: Deque[str]) -> int:
  |                    ^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Import the type from its module, as `from std.collections import Deque`,
or correct its spelling. A standard module's error set named alone,
`JsonError`, gets the fix `json.JsonError` when the module is imported,
and otherwise a note that names its import. When an expression names the type too, as
`Deque[str].new()`, that use gets E0200.

<!-- file: examples/reference-diagnostics/e0201_undefined_type_fixed.sisu -->
```sisu
# The type imported from its module.
from std.collections import Deque

def waiting(queue: Deque[str]) -> int:
    return len(queue)

def main():
    var queue = Deque[str].new()
    queue.push_back("Aino")
    print(waiting(queue))
```

<!-- output: examples/reference-diagnostics/e0201_undefined_type_fixed.out -->
```text
1
```

Shown in [chapter 17](../tutorial/17-modules-and-packages.md).

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

Shown in [chapter 3](../tutorial/03-functions.md).

### E0203

`resolve.unknown-module`: there is no module `{name}`.

A path of an import names modules of a package by its files: the
module `atlas.geo.point` is the file `src/geo/point.sisu` of the
package `atlas`. A path whose first name is the package but whose rest
names no file is E0203.

<!-- file: examples/reference-diagnostics/e0203_unknown_module.files/src/main.sisu -->
```sisu
# An import of a module that the package does not have.
from atlas.geo.points import Point

def main():
    print(Point(65.0, 25.5).lat)
```

<!-- output: examples/reference-diagnostics/e0203_unknown_module.console -->
```console
$ sisu run
error[E0203]: there is no module `atlas.geo.points`
  --> src/main.sisu:2:6
  |
2 | from atlas.geo.points import Point
  |      ^^^^^^^^^^^^^^^^
  = fix: a module with a similar name exists: `atlas.geo.point`

error[D0006]: the program was not run: errors found (1)
```

Name the module by its file's path, here `src/geo/point.sisu`. When a
module of the package has a similar name, the fix
`resolve.similar-module` writes the path with it, as E0220 does for a
package's own name.

<!-- file: examples/reference-diagnostics/e0203_unknown_module_fixed.files/src/main.sisu -->
```sisu
# An import of the module `src/geo/point.sisu`.
from atlas.geo.point import Point

def main():
    print(Point(65.0, 25.5).lat)
```

<!-- output: examples/reference-diagnostics/e0203_unknown_module_fixed.console -->
```console
$ sisu run
65.0
```

Shown in [R13](modules.md#imports).

### E0204

`resolve.not-in-module`: module `{module}` has no item `{name}`.

`from MODULE import NAME` names an item that the module declares `pub`:
a type, a function, a constant or an error set. A variant is reached
through its type, as `BookingError.Taken`, and is not an item of the
module: an import of a variant by its name, `from ledger import Taken`,
has the note `resolve.variant-of-type`, which names the type to import
and the variant through it ([G32](../guides/library-api.md)). When the module has the item and a `@cfg` of it leaves the
item out of this build, the note `resolve.cfg-left-out` names the
condition, `@cfg(feature="vat")` or `@cfg(test)`
([chapter 17](../tutorial/17-modules-and-packages.md#features)).

<!-- file: examples/reference-diagnostics/e0204_not_in_module.sisu -->
```sisu
# An import of an item that the module does not have.
from std.collections import Stack

def main():
    var stack = Stack[int]()
    stack.push(3)
    print(stack.pop())
```

<!-- output: examples/reference-diagnostics/e0204_not_in_module.console -->
```console
$ sisu run e0204_not_in_module.sisu
error[E0204]: module `std.collections` has no item `Stack`
  --> e0204_not_in_module.sisu:2:29
  |
2 | from std.collections import Stack
  |                             ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Check the module's items with `sisu doc --std std.collections`, or in
its source; the diagnostic offers no similar name. The use of the item
that the import did not bind, `Stack[int]()`, adds no error of its
own: its type argument is not read as a value.

<!-- file: examples/reference-diagnostics/e0204_not_in_module_fixed.sisu -->
```sisu
# A `Deque` is a stack, pushed and popped at its back.
from std.collections import Deque

def main():
    var stack = Deque[int].new()
    stack.push_back(3)
    print(stack.pop_back())
```

<!-- output: examples/reference-diagnostics/e0204_not_in_module_fixed.out -->
```text
Some(3)
```

Shown in [R13](modules.md).

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0215

`resolve.self-outside-method`: `self` exists only inside a method.

`self` is the first parameter of a method, in the body of a `struct`,
`enum`, `trait` or `impl`; as a parameter of any other function it is
E0215, and as a value outside a method E0389.

<!-- file: examples/reference-diagnostics/e0215_self.sisu -->
```sisu
# A `self` parameter in a function that is not a method.
def describe(self) -> String:
    return "a thing"

def main():
    print(describe())
```

<!-- output: examples/reference-diagnostics/e0215_self.console -->
```console
$ sisu run e0215_self.sisu
error[E0215]: `self` exists only inside a method
  --> e0215_self.sisu:2:14
  |
2 | def describe(self) -> String:
  |              ^^^^

error[D0006]: the program was not run: errors found (1)
```

Leave `self` out of a free function, or move the function into the
type's body as a method.

<!-- file: examples/reference-diagnostics/e0215_self_fixed.sisu -->
```sisu
# A function without `self`.
def describe() -> String:
    return "a thing"

def main():
    print(describe())
```

<!-- output: examples/reference-diagnostics/e0215_self_fixed.out -->
```text
a thing
```

### E0217

`resolve.duplicate-method`: `{owner}` has more than one method named
`{name}`.

A name is one function: Sisu has no overloading by parameters (D-050),
so a type's methods have names of their own.

<!-- file: examples/reference-diagnostics/e0217_duplicate_method.sisu -->
```sisu
# Two methods with one name.
struct Stove:
    celsius: int

    def heat(mut self):
        self.celsius += 10

    def heat(mut self, degrees: int):
        self.celsius += degrees

def main():
    var stove = Stove(60)
    stove.heat()
    print(stove.celsius)
```

<!-- output: examples/reference-diagnostics/e0217_duplicate_method.console -->
```console
$ sisu run e0217_duplicate_method.sisu
error[E0217]: `Stove` has more than one method named `heat`
  --> e0217_duplicate_method.sisu:8:9
  |
8 |     def heat(mut self, degrees: int):
  |         ^^^^

error[D0006]: the program was not run: errors found (1)
```

Give the parameter that varies a default, or give the second method
another name.

<!-- file: examples/reference-diagnostics/e0217_duplicate_method_fixed.sisu -->
```sisu
# One method, with a default for the degrees.
struct Stove:
    celsius: int

    def heat(mut self, degrees: int = 10):
        self.celsius += degrees

def main():
    var stove = Stove(60)
    stove.heat()
    stove.heat(20)
    print(stove.celsius)
```

<!-- output: examples/reference-diagnostics/e0217_duplicate_method_fixed.out -->
```text
90
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0220

`resolve.import-unknown`: cannot import `{name}`: it names no module of
this package, of a dependency in `sisu.toml`, or of the standard
library.

An import names a module of the package, of a dependency in `sisu.toml`,
or of the standard library. A single file is a package of one module, so
it can import only the standard library.

<!-- file: examples/reference-diagnostics/e0220_import_unknown.sisu -->
```sisu
# An import of a module whose name is misspelt.
import std.colections

def main():
    let sizes: colections.Deque[int] = colections.Deque.new()
    print(sizes.is_empty())
```

<!-- output: examples/reference-diagnostics/e0220_import_unknown.console -->
```console
$ sisu run e0220_import_unknown.sisu
error[E0220]: cannot import `std.colections`: it names no module of this package, of a dependency in `sisu.toml`, or of the standard library
  --> e0220_import_unknown.sisu:2:8
  |
2 | import std.colections
  |        ^^^^^^^^^^^^^^
  = fix: a module with a similar name exists: `std.collections`

error[D0006]: the program was not run: errors found (1)
```

The fix writes the module with the most similar name, when there is one.
For a dependency, add it to `sisu.toml` first
([R13](modules.md#the-manifest)). For a module whose file or directory
stands beside the file, the note `resolve.module-beside` says that only
a package makes them one program, as
[chapter 17](../tutorial/17-modules-and-packages.md#a-module-for-each-file)
shows. A module under a package's `src/` is not alone: `sisu test
src/NAME.sisu` compiles the package and runs that module's tests
([R15](testing.md#the-tests-of-one-module)).

<!-- file: examples/reference-diagnostics/e0220_import_unknown_fixed.sisu -->
```sisu
# The module's name spelt as the standard library spells it.
import std.collections

def main():
    let sizes: collections.Deque[int] = collections.Deque.new()
    print(sizes.is_empty())
```

<!-- output: examples/reference-diagnostics/e0220_import_unknown_fixed.out -->
```text
True
```

Shown in the guide [Write a script with dependencies](../guides/script.md).

### E0223

`resolve.not-a-module`: `{name}` is not a module.

`import` names a module, and `from MODULE import NAME` an item of one.
An `import` whose path ends in an item, a type, a function or a
constant, rather than a module is E0223; where a module of the
standard library has no such item at all, it is E0204.

<!-- file: examples/reference-diagnostics/e0223_not_a_module.sisu -->
```sisu
# An item of a module imported as if it were a module.
import std.collections.Deque

def main():
    var queue = Deque.new()
    queue.push_back("Aino")
    print(len(queue))
```

<!-- output: examples/reference-diagnostics/e0223_not_a_module.console -->
```console
$ sisu run e0223_not_a_module.sisu
error[E0223]: `std.collections.Deque` is not a module
  --> e0223_not_a_module.sisu:2:8
  |
2 | import std.collections.Deque
  |        ^^^^^^^^^^^^^^^^^^^^^
  = fix: import the item from its module: `from std.collections import Deque`

error[D0006]: the program was not run: errors found (1)
```

The fix writes the `from` form, which binds the item's name. The uses
of the name are not reported again.

<!-- file: examples/reference-diagnostics/e0223_not_a_module_fixed.sisu -->
```sisu
# An item imported from its module.
from std.collections import Deque

def main():
    var queue = Deque.new()
    queue.push_back("Aino")
    print(len(queue))
```

<!-- output: examples/reference-diagnostics/e0223_not_a_module_fixed.out -->
```text
1
```

Shown in [R13](modules.md#imports).

### E0225

`resolve.std-only-attribute`: `@{name}` is reserved for the standard
library.

Seven attributes, `@intrinsic`, `@builtin`, `@structural`, `@keeps`,
`@sealed`, `@marker` and `@internal`, describe the standard library's
own source; in a program's module they would make an unsafe operation a
safe call, or close a trait to the program that declares it.

<!-- file: examples/reference-diagnostics/e0225_reserved_attribute.sisu -->
```sisu
# An attribute that only the standard library may write.
@sealed
trait Shape:
    def area(self) -> f64

def main():
    print("no shapes yet")
```

<!-- output: examples/reference-diagnostics/e0225_reserved_attribute.console -->
```console
$ sisu run e0225_reserved_attribute.sisu
error[E0225]: `@sealed` is reserved for the standard library
  --> e0225_reserved_attribute.sisu:2:1
  |
2 | @sealed
  | ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Leave the attribute out.

<!-- file: examples/reference-diagnostics/e0225_reserved_attribute_fixed.sisu -->
```sisu
# The trait without the standard library's attribute.
trait Shape:
    def area(self) -> f64

def main():
    print("no shapes yet")
```

<!-- output: examples/reference-diagnostics/e0225_reserved_attribute_fixed.out -->
```text
no shapes yet
```

Shown in [R14](attributes.md#attributes-of-the-standard-library).

### E0231

`resolve.trait-as-type`: `{name}` is a trait, not a type; a value of a
type that implements it is `dyn {name}`.

A trait is not a type. A value of some type that implements it is a type
parameter with the trait as its bound, or a `dyn` value behind `ref` or
`Box`.

<!-- file: examples/reference-diagnostics/e0231_trait_as_type.sisu -->
```sisu
# A trait used where a type should be.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show(item: Priced) -> String:
    return "{item.price()} cents"

def main():
    print(show(Bun("cardamom")))
```

<!-- output: examples/reference-diagnostics/e0231_trait_as_type.console -->
```console
$ sisu run e0231_trait_as_type.sisu
error[E0231]: `Priced` is a trait, not a type; a value of a type that implements it is `dyn Priced`
  --> e0231_trait_as_type.sisu:12:16
   |
12 | def show(item: Priced) -> String:
   |                ^^^^^^
  = note: a function that takes a value of any type implementing `Priced` is generic: `def f[T: Priced](x: T)`, made for each type it is called with
  = fix: use the trait through a reference: `ref dyn `

error[D0006]: the program was not run: errors found (1)
```

The fix writes `ref dyn Priced`, a borrow of a value of any type that
implements the trait; the note names the other way, a bounded type
parameter, which is the usual choice and the fixed program's.

<!-- file: examples/reference-diagnostics/e0231_trait_as_type_fixed.sisu -->
```sisu
# A type parameter bounded by the trait.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show[T: Priced](item: T) -> String:
    return "{item.price()} cents"

def main():
    print(show(Bun("cardamom")))
```

<!-- output: examples/reference-diagnostics/e0231_trait_as_type_fixed.out -->
```text
280 cents
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0232

`resolve.missing-trait-method`: the impl of `{trait}` for `{ty}` is
missing {methods}.

An `impl` of a trait writes every method the trait declares without a
body.

<!-- file: examples/reference-diagnostics/e0232_missing_method.sisu -->
```sisu
# An impl without one of the trait's required methods.
trait Priced:
    def price(self) -> int
    def name(self) -> String

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def main():
    print(Bun("cardamom").price())
```

<!-- output: examples/reference-diagnostics/e0232_missing_method.console -->
```console
$ sisu run e0232_missing_method.sisu
error[E0232]: the impl of `Priced` for `Bun` is missing `name`
  --> e0232_missing_method.sisu:9:1
  |
9 | impl Priced for Bun:
  | ^^^^^^^^^^^^^^^^^^^
  = note: write `def name(self) -> String` in the impl

error[D0006]: the program was not run: errors found (1)
```

Write the method the note names, with the signature it gives.

<!-- file: examples/reference-diagnostics/e0232_missing_method_fixed.sisu -->
```sisu
# Every required method written.
trait Priced:
    def price(self) -> int
    def name(self) -> String

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

    def name(self) -> String:
        return "a {self.filling} bun"

def main():
    print(Bun("cardamom").price())
```

<!-- output: examples/reference-diagnostics/e0232_missing_method_fixed.out -->
```text
280
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0233

`resolve.not-a-trait-method`: `{name}` is not a method of `{trait}`.

An `impl Trait for Type` holds the trait's methods and nothing else, so
that every method in it keeps a promise of the trait.

<!-- file: examples/reference-diagnostics/e0233_not_trait_method.sisu -->
```sisu
# An impl with a method that its trait does not declare.
trait Priced:
    def price(self) -> int

struct Coffee:
    cups: int

impl Priced for Coffee:
    def price(self) -> int:
        return 350 * self.cups

    def discount(self) -> int:
        return 50

def main():
    print(Coffee(2).price())
```

<!-- output: examples/reference-diagnostics/e0233_not_trait_method.console -->
```console
$ sisu run e0233_not_trait_method.sisu
error[E0233]: `discount` is not a method of `Priced`
  --> e0233_not_trait_method.sisu:12:9
   |
12 |     def discount(self) -> int:
   |         ^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

A method of the type's own goes in an `impl Type:` of its own, or in the
type's body.

<!-- file: examples/reference-diagnostics/e0233_not_trait_method_fixed.sisu -->
```sisu
# The extra method goes in the type's own impl.
trait Priced:
    def price(self) -> int

struct Coffee:
    cups: int

impl Priced for Coffee:
    def price(self) -> int:
        return 350 * self.cups

impl Coffee:
    def discount(self) -> int:
        return 50

def main():
    let order = Coffee(2)
    print(order.price() - order.discount())
```

<!-- output: examples/reference-diagnostics/e0233_not_trait_method_fixed.out -->
```text
650
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0235

`resolve.overlapping-impl`: `{trait}` is implemented for `{ty}` more
than once; a type implements a trait at most once.

A type implements a trait at most once, so that a call never has two
meanings (D-013).

<!-- file: examples/reference-diagnostics/e0235_overlap.sisu -->
```sisu
# Two impls of one trait for one type.
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100} €")

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("€{self.cents // 100}")

def main():
    print(Money(1250))
```

<!-- output: examples/reference-diagnostics/e0235_overlap.console -->
```console
$ sisu run e0235_overlap.sisu
error[E0235]: `Display` is implemented for `Money` more than once; a type implements a trait at most once
  --> e0235_overlap.sisu:9:1
  |
9 | impl Display for Money:
  | ^^^^^^^^^^^^^^^^^^^^^^
  |
5 | impl Display for Money:
  | ---------------------- the other impl is here

error[D0006]: the program was not run: errors found (1)
```

Keep one impl. For a second way to show the value, write a method, or
wrap the value in a type of its own. An impl written beside a derive of
the same trait is E0235 at the derive's argument, `Eq` in
`@derive(Eq)`, with the label `resolve.first-impl` at the written impl,
as for two written impls, and the note `type.derived-code`, which
quotes the derived impl's first line
([R14](attributes.md#deriving-traits)): keep the derive or the impl.

<!-- file: examples/reference-diagnostics/e0235_overlap_fixed.sisu -->
```sisu
# One impl.
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100} €")

def main():
    print(Money(1250))
```

<!-- output: examples/reference-diagnostics/e0235_overlap_fixed.out -->
```text
12 €
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0236

`resolve.orphan-impl`: `{trait}` for `{ty}` can be implemented only
in the package of the trait or of the type, or in the root package.

A type implements a trait at most once in a program, so an impl stands
in the package that declares the trait, the type, or one of the
trait's type arguments (D-013), as `Vector` in `impl Mul[Vector] for
f64`; the root package of a build, which nothing depends on, may
implement any trait for any type. Here the library `digits` implements
the prelude's `Len` for `int`, and the program `year` depends on it:

<!-- file: examples/reference-diagnostics/e0236_orphan.files/digits/src/lib.sisu -->
```sisu
##! The digits of numbers.

## The number of decimal digits of an `int`, as `len` gives it.
impl Len for int:
    def len(self) -> int:
        return len("{self}")
```

<!-- output: examples/reference-diagnostics/e0236_orphan.console -->
```console
$ sisu run year
error[E0236]: `Len` for `int` can be implemented only in the package of the trait or of the type, or in the root package
  --> ../digits/src/lib.sisu:4:1
  |
4 | impl Len for int:
  | ^^^^^^^^^^^^^^^^
  = note: the impl is this package's when `int` is a type of this package, `Len` a trait of it, or one of the trait's type arguments a type of its own, as `Vector` in `impl Mul[Vector] for f64`

error[D0006]: the program was not run: errors found (1)
```

The note (`resolve.orphan-local-type`) names the three ways an impl
is the package's own. `Len` has no type argument, so wrap the value in
a type of the library's own, and implement the trait for that.

<!-- file: examples/reference-diagnostics/e0236_orphan_fixed.files/digits/src/lib.sisu -->
```sisu
##! The digits of numbers.

## A number whose `len` is its count of decimal digits.
pub struct Number:
    pub value: int

impl Len for Number:
    def len(self) -> int:
        return len("{self.value}")
```

<!-- file: examples/reference-diagnostics/e0236_orphan_fixed.files/year/src/main.sisu -->
```sisu
# A program that uses the library's own type.
from digits import Number

def main():
    print(len(Number(2026)))
```

<!-- output: examples/reference-diagnostics/e0236_orphan_fixed.console -->
```console
$ sisu run year
4
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0237

`resolve.missing-assoc-type`: the impl of `{trait}` for `{ty}` does
not say what `{name}` is; add `type {name} = ...`.

A trait's associated type is chosen by each impl, which says what it is
with `type NAME = T`.

<!-- file: examples/reference-diagnostics/e0237_missing_assoc_type.sisu -->
```sisu
# An impl that does not say what its associated type is.
trait Source:
    type Item
    def first(self) -> Self.Item

struct Bench:
    sitters: list[String]

impl Source for Bench:
    def first(self) -> String:
        return self.sitters[0].copy()

def main():
    print(Bench(["Aino"]).first())
```

<!-- output: examples/reference-diagnostics/e0237_missing_assoc_type.console -->
```console
$ sisu run e0237_missing_assoc_type.sisu
error[E0237]: the impl of `Source` for `Bench` does not say what `Item` is; add `type Item = ...`
  --> e0237_missing_assoc_type.sisu:9:1
  |
9 | impl Source for Bench:
  | ^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

The impl's associated type is the error's from there on, so a use of
it, the `print` of what `first()` returns, is not reported again as a
type without `Display` (E0347): E0237 stands alone. Add the line the
message gives.

<!-- file: examples/reference-diagnostics/e0237_missing_assoc_type_fixed.sisu -->
```sisu
# The impl says what its associated type is.
trait Source:
    type Item
    def first(self) -> Self.Item

struct Bench:
    sitters: list[String]

impl Source for Bench:
    type Item = String

    def first(self) -> String:
        return self.sitters[0].copy()

def main():
    print(Bench(["Aino"]).first())
```

<!-- output: examples/reference-diagnostics/e0237_missing_assoc_type_fixed.out -->
```text
Aino
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0241

`resolve.not-object-safe`: `{trait}` cannot be used as `dyn {trait}`:
its member `{method}` cannot be used through it, because {reason}.

A trait can be a `dyn` type only when every member can be used through
it: each method takes `self` or `mut self`, and none names `Self`
elsewhere (D-017), and the trait has no constant, which each impl gives
its own value. The member it names is a method or such a constant.

<!-- file: examples/reference-diagnostics/e0241_object_safety.sisu -->
```sisu
# A trait used as `dyn` with a method that takes no `self`.
trait Shape:
    def unit() -> Self
    def area(self) -> f64

struct Square:
    side: f64

impl Shape for Square:
    def unit() -> Square:
        return Square(1.0)

    def area(self) -> f64:
        return self.side * self.side

def total(shapes: list[Box[dyn Shape]]) -> f64:
    return sum([shape.area() for shape in shapes])

def main():
    print(total([Box(Square(2.0))]))
```

<!-- output: examples/reference-diagnostics/e0241_object_safety.console -->
```console
$ sisu run e0241_object_safety.sisu
error[E0241]: `Shape` cannot be used as `dyn Shape`: its member `unit` cannot be used through it, because it does not take `self` or `mut self`
  --> e0241_object_safety.sisu:16:28
   |
16 | def total(shapes: list[Box[dyn Shape]]) -> f64:
   |                            ^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Move the method out of the trait, into the type or another trait, or use
a type parameter instead of `dyn`.

<!-- file: examples/reference-diagnostics/e0241_object_safety_fixed.sisu -->
```sisu
# The method without `self` moved out of the trait.
trait Shape:
    def area(self) -> f64

struct Square:
    side: f64

    def unit() -> Square:
        return Square(1.0)

impl Shape for Square:
    def area(self) -> f64:
        return self.side * self.side

def total(shapes: list[Box[dyn Shape]]) -> f64:
    return sum([shape.area() for shape in shapes])

def main():
    print(total([Box(Square(2.0)), Box(Square.unit())]))
```

<!-- output: examples/reference-diagnostics/e0241_object_safety_fixed.out -->
```text
5.0
```

Shown in [chapter 15](../tutorial/15-generics.md).

### E0242

`resolve.supertrait-missing`: `{ty}` implements `{trait}`, which
requires `{super}`, but `{ty}` does not implement `{super}`.

A trait that names a supertrait, as `trait Labelled(Display)`, can be
implemented only for types that implement the supertrait.

<!-- file: examples/reference-diagnostics/e0242_supertrait.sisu -->
```sisu
# An impl of a trait whose supertrait the type lacks.
trait Labelled(Display):
    def label(self) -> String:
        return "[ {self} ]"

struct Weight:
    grams: int

impl Labelled for Weight:
    pass

def main():
    print(Weight(90).label())
```

<!-- output: examples/reference-diagnostics/e0242_supertrait.console -->
```console
$ sisu run e0242_supertrait.sisu
error[E0242]: `Weight` implements `Labelled`, which requires `Display`, but `Weight` does not implement `Display`
  --> e0242_supertrait.sisu:9:1
  |
9 | impl Labelled for Weight:
  | ^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Implement the supertrait for the type as well.

<!-- file: examples/reference-diagnostics/e0242_supertrait_fixed.sisu -->
```sisu
# The supertrait implemented too.
trait Labelled(Display):
    def label(self) -> String:
        return "[ {self} ]"

struct Weight:
    grams: int

impl Display for Weight:
    def fmt(self, mut f: Formatter):
        f.write("{self.grams} g")

impl Labelled for Weight:
    pass

def main():
    print(Weight(90).label())
```

<!-- output: examples/reference-diagnostics/e0242_supertrait_fixed.out -->
```text
[ 90 g ]
```

Shown in [chapter 14](../tutorial/14-traits.md).

### E0243

`resolve.not-derivable`: `{name}` cannot be derived; `@derive` takes
`Copy` and the traits that have a `comptime def derive`: `Debug`,
`Default`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that
declares one.

`@derive` takes `Copy` and the traits that declare a `comptime def
derive`; the message lists them. `Display` has none, since only you know
how your type should read. A trait whose derive [E0607](#e0607) refuses
declares one, so its `@derive`s are not this error.

<!-- file: examples/reference-diagnostics/e0243_not_derivable.sisu -->
```sisu
# A derive of a trait that has no derive.
@derive(Debug, Display)
struct Money:
    cents: int

def main():
    print("{Money(1250):?}")
```

<!-- output: examples/reference-diagnostics/e0243_not_derivable.console -->
```console
$ sisu run e0243_not_derivable.sisu
error[E0243]: `Display` cannot be derived; `@derive` takes `Copy` and the traits that have a `comptime def derive`: `Debug`, `Default`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that declares one
  --> e0243_not_derivable.sisu:2:16
  |
2 | @derive(Debug, Display)
  |                ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write an `impl` of the trait.

<!-- file: examples/reference-diagnostics/e0243_not_derivable_fixed.sisu -->
```sisu
# `Display` written by hand.
@derive(Debug)
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100} €")

def main():
    print(Money(1250))
```

<!-- output: examples/reference-diagnostics/e0243_not_derivable_fixed.out -->
```text
12 €
```

Shown in [chapter 14](../tutorial/14-traits.md).

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0245

`resolve.generic-default-order`: a type parameter without a default
cannot follow one with a default.

A use that leaves out a defaulted type parameter leaves out the last
ones, so the parameters with defaults come after the others (D-060).

<!-- file: examples/reference-diagnostics/e0245_default_order.sisu -->
```sisu
# A type parameter without a default after one with a default.
struct Reading[U = String, T]:
    value: T
    unit: U

def main():
    print(Reading("°C", 80).value)
```

<!-- output: examples/reference-diagnostics/e0245_default_order.console -->
```console
$ sisu run e0245_default_order.sisu
error[E0245]: a type parameter without a default cannot follow one with a default
  --> e0245_default_order.sisu:2:28
  |
2 | struct Reading[U = String, T]:
  |                            ^

error[D0006]: the program was not run: errors found (1)
```

Put the parameter with a default last.

<!-- file: examples/reference-diagnostics/e0245_default_order_fixed.sisu -->
```sisu
# The parameter with a default comes last.
struct Reading[T, U = String]:
    value: T
    unit: U

def main():
    print(Reading(80, "°C").value)
```

<!-- output: examples/reference-diagnostics/e0245_default_order_fixed.out -->
```text
80
```

Shown in [chapter 15](../tutorial/15-generics.md).

### E0252

`resolve.delegate-self`: `{trait}` returns a `{ty}`, so `{ty}` can
delegate it only when `{field}` is its one field.

An operator trait's method returns `Self`, which `@delegate` builds
from the field's result, so a struct with other fields could not be
made. The follow-on E0310 says that the operator then has no impl, and
its note names the impl that would give it one.

<!-- file: examples/reference-diagnostics/e0252_delegate_self.sisu -->
```sisu
# An operator delegated to one of two fields.
@delegate(Add, to="metres")
struct Run:
    metres: f64
    runner: String

def main():
    let total = Run(400.0, "Aino") + Run(800.0, "Aino")
    print(total.metres)
```

<!-- output: examples/reference-diagnostics/e0252_delegate_self.console -->
```console
$ sisu run e0252_delegate_self.sisu
error[E0252]: `Add` returns a `Run`, so `Run` can delegate it only when `metres` is its one field
  --> e0252_delegate_self.sisu:2:11
  |
2 | @delegate(Add, to="metres")
  |           ^^^

error[E0310]: `+` cannot be applied to `Run` and `Run`
  --> e0252_delegate_self.sisu:8:17
  |
8 |     let total = Run(400.0, "Aino") + Run(800.0, "Aino")
  |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: `+` on a `Run` is the method of the trait `Add`: an `impl Add[Run] for Run` gives it one

error[D0006]: the program was not run: errors found (2)
```

Give the value a one-field type of its own that delegates the operator,
and keep that type as the field.

<!-- file: examples/reference-diagnostics/e0252_delegate_self_fixed.sisu -->
```sisu
# The operator is delegated by a one-field struct, held as a field.
@delegate(Add, to="value")
struct Metres:
    value: f64

struct Run:
    distance: Metres
    runner: String

def main():
    let total = Run(Metres(400.0), "Aino").distance + Metres(800.0)
    print(total.value)
```

<!-- output: examples/reference-diagnostics/e0252_delegate_self_fixed.out -->
```text
1200.0
```

Shown in [R14](attributes.md#delegating-traits-to-a-field).

### E0255

`resolve.private-item`: `{name}` is private to the module `{module}`;
declare it `pub` to use it from another module.

An item of a module, a function, a type, a constant or an error set, is
private to the module unless it is `pub` ([R13](modules.md#visibility)).
Another module of the same package may import only its `pub` items, and
another package only those of the package's API.

<!-- file: examples/reference-diagnostics/e0255_private_item.files/src/heat.sisu -->
```sisu
# How a sauna's heat feels; `band` is this module's own.
def band(celsius: int) -> int:
    return celsius // 10 * 10

## How a sauna at `celsius` degrees feels.
pub def feel(celsius: int) -> str:
    return if band(celsius) >= 80: "hot" else: "warm"
```

<!-- file: examples/reference-diagnostics/e0255_private_item.files/src/main.sisu -->
```sisu
# A function that another module keeps private.
from sauna.heat import band, feel

def main():
    print("{band(85)}: {feel(85)}")
```

<!-- output: examples/reference-diagnostics/e0255_private_item.console -->
```console
$ sisu run
error[E0255]: `band` is private to the module `sauna.heat`; declare it `pub` to use it from another module
  --> src/main.sisu:2:24
  |
2 | from sauna.heat import band, feel
  |                        ^^^^

error[D0006]: the program was not run: errors found (1)
```

Declare the item `pub`, with a doc comment, if it belongs to what the
module offers, as the fixed module does; otherwise keep it private and
give the other module what it needs through a `pub` function.
`pub(package)` makes an item visible to the package's own modules only.

<!-- file: examples/reference-diagnostics/e0255_private_item_fixed.files/src/heat.sisu -->
```sisu
# How a sauna's heat feels, by bands of ten degrees.

## `celsius` rounded down to tens.
pub def band(celsius: int) -> int:
    return celsius // 10 * 10

## How a sauna at `celsius` degrees feels.
pub def feel(celsius: int) -> str:
    return if band(celsius) >= 80: "hot" else: "warm"
```

<!-- output: examples/reference-diagnostics/e0255_private_item_fixed.console -->
```console
$ sisu run
80: hot
```

Shown in [chapter 17](../tutorial/17-modules-and-packages.md).

### E0256

`resolve.package-item`: `{name}` is `pub(package)` in `{module}`, so
only the modules of the package `{package}` can use it.

`pub(package)` exports an item to the modules of its own package only
(D-045). Here the application `booking` calls a function of the library
`sauna` that the library keeps for itself:

<!-- file: examples/reference-diagnostics/e0256_package_item.files/sauna/src/lib.sisu -->
```sisu
##! How hot a sauna is.

## `celsius` rounded down to tens, for this package's own modules.
pub(package) def round_to_ten(celsius: int) -> int:
    return celsius // 10 * 10

## How a sauna at `celsius` degrees feels.
pub def feel(celsius: int) -> str:
    return if round_to_ten(celsius) >= 70: "hot" else: "warm"
```

<!-- file: examples/reference-diagnostics/e0256_package_item.files/booking/src/main.sisu -->
```sisu
# An application that uses a `pub(package)` function of a library.
import sauna

def main():
    print(sauna.round_to_ten(84))
```

<!-- output: examples/reference-diagnostics/e0256_package_item.console -->
```console
$ sisu run booking
error[E0256]: `round_to_ten` is `pub(package)` in `sauna`, so only the modules of the package `sauna` can use it
  --> src/main.sisu:5:17
  |
5 |     print(sauna.round_to_ten(84))
  |                 ^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Use the library's `pub` API, or, in the library, declare the item
`pub` if other packages are to use it.

<!-- file: examples/reference-diagnostics/e0256_package_item_fixed.files/booking/src/main.sisu -->
```sisu
# An application that uses the library's `pub` function.
import sauna

def main():
    print(sauna.feel(84))
```

<!-- output: examples/reference-diagnostics/e0256_package_item_fixed.console -->
```console
$ sisu run booking
hot
```

Shown in [chapter 17](../tutorial/17-modules-and-packages.md).

### E0258

`resolve.private-field`: the field `{field}` of `{ty}` is private to
the module `{module}`; declare it `pub` to use it from another module.

A field is private to its type's module unless it is `pub`. A
construction names every field, so a struct with a private field is
built only where that field is seen.

<!-- file: examples/reference-diagnostics/e0258_private_field.files/src/geo/point.sisu -->
```sisu
# A point whose longitude is private to its module.
pub struct Point:
    pub lat: f64
    lon: f64
```

<!-- file: examples/reference-diagnostics/e0258_private_field.files/src/main.sisu -->
```sisu
# A struct built from another module, which cannot see one of its fields.
from atlas.geo.point import Point

def main():
    let oulu = Point(65.0, 25.5)
    print(oulu.lat)
```

<!-- output: examples/reference-diagnostics/e0258_private_field.console -->
```console
$ sisu run
error[E0258]: the field `lon` of `Point` is private to the module `atlas.geo.point`; declare it `pub` to use it from another module
  --> src/main.sisu:5:16
  |
5 |     let oulu = Point(65.0, 25.5)
  |                ^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

The fix declares the field `pub`, which makes it part of the module's
API. Where the field should stay private, an associated function of
the type, such as `Point.new(lat, lon)`, builds the value instead.
When the type has such a function that this module may call, the note
`resolve.made-by` names it, `new` first: with `pub def
on_meridian(lat: f64) -> Point` in `point.sisu`, it reads
"`Point.on_meridian(lat)` makes a `Point` from another module".

<!-- file: examples/reference-diagnostics/e0258_private_field_fixed.files/src/geo/point.sisu -->
```sisu
# A point whose fields are both public.
pub struct Point:
    pub lat: f64
    pub lon: f64
```

<!-- output: examples/reference-diagnostics/e0258_private_field_fixed.console -->
```console
$ sisu run
65.0
```

Shown in [chapter 17](../tutorial/17-modules-and-packages.md).

### E0259

`resolve.private-method`: the method `{name}` of `{ty}` is private to
the module `{module}`; declare it `pub` to call it from another module.

A method is private to its type's module unless it is `pub`, as a field
is; the type itself can be `pub` while some of its methods are the
module's own ([R13](modules.md#visibility)).

<!-- file: examples/reference-diagnostics/e0259_private_method.files/src/stove.sisu -->
```sisu
# A stove whose method `cool` is private to its module.
pub struct Stove:
    pub celsius: int

    def cool(mut self):
        self.celsius -= 10

    ## Lets the stove rest, which cools it by ten degrees.
    pub def rest(mut self):
        self.cool()
```

<!-- file: examples/reference-diagnostics/e0259_private_method.files/src/main.sisu -->
```sisu
# A method that the type's module keeps private.
from kiuas.stove import Stove

def main():
    var stove = Stove(celsius=90)
    stove.cool()
    print(stove.celsius)
```

<!-- output: examples/reference-diagnostics/e0259_private_method.console -->
```console
$ sisu run
error[E0259]: the method `cool` of `kiuas.stove.Stove` is private to the module `kiuas.stove`; declare it `pub` to call it from another module
  --> src/main.sisu:6:11
  |
6 |     stove.cool()
  |           ^^^^

error[D0006]: the program was not run: errors found (1)
```

Call the method the type offers, as the fixed program does with `rest`,
or declare the method `pub` if it belongs to the type's API.

<!-- file: examples/reference-diagnostics/e0259_private_method_fixed.files/src/main.sisu -->
```sisu
# The method that the type's module offers.
from kiuas.stove import Stove

def main():
    var stove = Stove(celsius=90)
    stove.rest()
    print(stove.celsius)
```

<!-- output: examples/reference-diagnostics/e0259_private_method_fixed.console -->
```console
$ sisu run
80
```

Shown in [R13](modules.md#visibility).

### E0266

`resolve.bad-cfg`: `@cfg` takes a condition: `test`, `feature="NAME"`,
`not` and a condition, or `all(...)` or `any(...)` of conditions; this
is none of them.

`@cfg` reads conditions on the build, not expressions, and keeps its
declaration when the condition is none of those. On an import, a
condition may test the target, `@cfg(target.os == "windows")`
([R13](modules.md#target-conditions)); on any other item a test of the
target is written in a body, with `if comptime`, which the note
`resolve.cfg-target` names.

<!-- file: examples/reference-diagnostics/e0266_bad_cfg.sisu -->
```sisu
# A condition of `@cfg` that is an expression.
@cfg(target.os == "linux")
def separator() -> str:
    return "/"

def main():
    print("usr{separator()}bin")
```

<!-- output: examples/reference-diagnostics/e0266_bad_cfg.console -->
```console
$ sisu run e0266_bad_cfg.sisu
error[E0266]: `@cfg` takes a condition: `test`, `feature="NAME"`, `not` and a condition, or `all(...)` or `any(...)` of conditions; this is none of them
  --> e0266_bad_cfg.sisu:2:6
  |
2 | @cfg(target.os == "linux")
  |      ^^^^^^^^^^^^^^^^^^^^
  = note: a body tests the target with `if comptime target.os == "linux":`, whose other branch is left out of the build

error[D0006]: the program was not run: errors found (1)
```

<!-- file: examples/reference-diagnostics/e0266_bad_cfg_fixed.sisu -->
```sisu
# The target is tested with `if comptime`, in a body.
def separator() -> str:
    return if comptime target.os == "windows": "\\" else: "/"

def main():
    print("usr{separator()}bin")
```

<!-- output: examples/reference-diagnostics/e0266_bad_cfg_fixed.out -->
```text
usr/bin
```

Shown in [R14](attributes.md#conditional-compilation).

### E0271

`resolve.not-a-trait-constant`: `{name}` is not a constant of
`{trait}`.

An impl of a trait gives the constants that the trait declares, and no
other ([R3](declarations.md#impls)): a constant that the trait does not
name, here a misspelling of `WARM_UP`, is refused at its name.

<!-- file: examples/reference-diagnostics/e0271_not_trait_constant.sisu -->
```sisu
# An impl gives a constant that its trait does not declare.
trait Stove:
    const MAX_HEAT: int
    const WARM_UP: int = 30

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    const MAX_HEAT: int = 110
    const WARMUP: int = 60

def main():
    print("{WoodStove.MAX_HEAT} {WoodStove.WARM_UP}")
```

<!-- output: examples/reference-diagnostics/e0271_not_trait_constant.console -->
```console
$ sisu run e0271_not_trait_constant.sisu
error[E0271]: `WARMUP` is not a constant of `Stove`
  --> e0271_not_trait_constant.sisu:11:11
   |
11 |     const WARMUP: int = 60
   |           ^^^^^^
  = fix: a similar name is in scope: `WARM_UP`

error[D0006]: the program was not run: errors found (1)
```

Spell the constant as the trait does: when the trait declares one of a
similar name, the fix `resolve.similar-name` writes it. A constant that
belongs to the type alone goes in the type's body or in an `impl WoodStove:`, outside
the trait's impl.

<!-- file: examples/reference-diagnostics/e0271_not_trait_constant_fixed.sisu -->
```sisu
# The impl names the trait's constant as the trait spells it.
trait Stove:
    const MAX_HEAT: int
    const WARM_UP: int = 30

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    const MAX_HEAT: int = 110
    const WARM_UP: int = 60

def main():
    print("{WoodStove.MAX_HEAT} {WoodStove.WARM_UP}")
```

<!-- output: examples/reference-diagnostics/e0271_not_trait_constant_fixed.out -->
```text
110 60
```

[R3](declarations.md#impls) describes it.

### E0272

`resolve.constant-type`: the constant `{name}` has the type `{found}`
here, and `{trait}` declares it with the type `{expected}`.

An impl gives a trait's constant the type that the trait declares, so
that code generic over the trait reads one type whichever impl gives
the value ([R3](declarations.md#impls)).

<!-- file: examples/reference-diagnostics/e0272_constant_type.sisu -->
```sisu
# An impl gives a trait's constant a type other than the trait's.
trait Stove:
    const MAX_HEAT: int

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    const MAX_HEAT: f64 = 110.0

def main():
    print(WoodStove.MAX_HEAT)
```

<!-- output: examples/reference-diagnostics/e0272_constant_type.console -->
```console
$ sisu run e0272_constant_type.sisu
error[E0272]: the constant `MAX_HEAT` has the type `f64` here, and `Stove` declares it with the type `int`
  --> e0272_constant_type.sisu:9:21
  |
9 |     const MAX_HEAT: f64 = 110.0
  |                     ^^^

error[D0006]: the program was not run: errors found (1)
```

Give the constant the trait's type, and a value of that type.

<!-- file: examples/reference-diagnostics/e0272_constant_type_fixed.sisu -->
```sisu
# The impl gives the constant the type that the trait declares.
trait Stove:
    const MAX_HEAT: int

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    const MAX_HEAT: int = 110

def main():
    print(WoodStove.MAX_HEAT)
```

<!-- output: examples/reference-diagnostics/e0272_constant_type_fixed.out -->
```text
110
```

[R3](declarations.md#impls) describes it.

### E0273

`resolve.const-or-method`: `{name}` is {what} of `{trait}`, and this
impl gives it as {given}.

A trait's constant is given as a constant, and its method as a method
([R3](declarations.md#impls)): a constant has one value for the type,
where a method works one out for each value. This impl writes the
constant `MAX_HEAT` as a method that reads a field, which E0268 refuses
for its name as well. E0268 offers no rename here, which would leave
the constant missing: its note `resolve.const-as-method` says how a
trait's constant is given.

<!-- file: examples/reference-diagnostics/e0273_const_or_method.sisu -->
```sisu
# An impl gives a trait's constant as a method.
trait Stove:
    const MAX_HEAT: int

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    def MAX_HEAT(self) -> int:
        return 100 + self.logs

def main():
    print(WoodStove.MAX_HEAT)
```

<!-- output: examples/reference-diagnostics/e0273_const_or_method.console -->
```console
$ sisu run e0273_const_or_method.sisu
error[E0268]: the name of a method is written in snake_case, and `MAX_HEAT` is not
  --> e0273_const_or_method.sisu:9:9
  |
9 |     def MAX_HEAT(self) -> int:
  |         ^^^^^^^^
  = note: a name in capitals is a constant's: a trait's constant is given as `const MAX_HEAT: T = ...`

error[E0273]: `MAX_HEAT` is a constant of `Stove`, and this impl gives it as a method
  --> e0273_const_or_method.sisu:9:9
  |
9 |     def MAX_HEAT(self) -> int:
  |         ^^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

Give the constant one value for the type. A value that depends on each
stove's fields is a method, which the trait would declare as one.

<!-- file: examples/reference-diagnostics/e0273_const_or_method_fixed.sisu -->
```sisu
# The impl gives the constant as a constant, with one value for the type.
trait Stove:
    const MAX_HEAT: int

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    const MAX_HEAT: int = 110

def main():
    print(WoodStove.MAX_HEAT)
```

<!-- output: examples/reference-diagnostics/e0273_const_or_method_fixed.out -->
```text
110
```

[R3](declarations.md#impls) describes it.

### E0274

`resolve.const-not-type`: `{name}` is a constant of `{trait}`, which
bounds `{ty}`, not a type.

A trait's constant read through a type parameter, `T.SIDES` where a
type's constant argument goes (`Array[int, T.SIDES]`), is a value of the
instance, known where the function runs, while a type's constant
argument is known where the type is written. The note
`resolve.const-through-param` names the fix: a `const` parameter that
carries the value, which a caller gives as `Square.SIDES`.

Example: [R3](declarations.md#traits) shows it, and the `const`
parameter that is its fix.

### E0275

`resolve.dependency-excluded`: cannot import `{name}`: the dependency
`{package}` of `{this}` is not built for this target, which its `when`
in that package's `sisu.toml` excludes.

A dependency with `when` is part of the build only on the targets its
condition holds for ([R13](modules.md#target-conditions)). Here the
program `sauna` names `winlog` for Windows only, and imports it on
every target:

<!-- file: examples/reference-diagnostics/e0275_excluded.files/sauna/sisu.toml -->
```toml
[package]
name = "sauna"
version = "0.1.0"
edition = "2027"

[dependencies]
winlog = { path = "../winlog", when = { os = "windows" } }
```

<!-- file: examples/reference-diagnostics/e0275_excluded.files/sauna/src/main.sisu -->
```sisu
# A program that imports a dependency built for Windows only.
import winlog

def main():
    winlog.record("the stove is on")
```

<!-- output: examples/reference-diagnostics/e0275_excluded.console -->
```console
$ sisu run sauna
error[E0275]: cannot import `winlog`: the dependency `winlog` of `sauna` is not built for this target, which its `when` in that package's `sisu.toml` excludes
  --> src/main.sisu:2:8
  |
2 | import winlog
  |        ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

A dependency that every target can build, as `winlog` can, has no
`when`. One that only some targets build is imported under the same
condition, `@cfg(target.os == "windows")`, which binds `winlog` only
where it is built, and its uses stand in an `if comptime` branch on the
same fact, which the build leaves out elsewhere:

<!-- file: examples/reference-diagnostics/e0275_excluded_cfg.files/sauna/src/main.sisu -->
```sisu
# The import tests the target, and so does the code that uses it.
@cfg(target.os == "windows")
import winlog

def main():
    if comptime target.os == "windows":
        winlog.record("the stove is on")
    else:
        print("the stove is on")
```

<!-- output: examples/reference-diagnostics/e0275_excluded_cfg.console -->
```console
$ sisu run sauna
the stove is on
```

A use outside such a branch is [E0200](#e0200), whose note names the
`@cfg`. Where the program does not need the dependency to stay
Windows-only, drop its `when`:

<!-- file: examples/reference-diagnostics/e0275_excluded_fixed.files/sauna/sisu.toml -->
```toml
[package]
name = "sauna"
version = "0.1.0"
edition = "2027"

[dependencies]
winlog = { path = "../winlog" }
```

<!-- output: examples/reference-diagnostics/e0275_excluded_fixed.console -->
```console
$ sisu run sauna
event log: the stove is on
```

### E0276

`resolve.dev-dependency`: cannot import `{name}` here: `{package}` is a
dev-dependency of `{this}`, which its tests import and its code does
not.

A dev-dependency is for a package's `test` blocks, its benchmarks and
the black-box tests of `tests/`
([R13](modules.md#dependencies-and-dev-dependencies)). A package that
another one depends on is built without its dev-dependencies, so its
own code imports one only under `@cfg(test)`, which a build without the
tests leaves out. The check is made in every build, `sisu check`
included. Here the library `ledger` uses its test fixtures in a public
function:

<!-- file: examples/reference-diagnostics/e0276_dev_dependency.files/ledger/sisu.toml -->
```toml
[package]
name = "ledger"
version = "0.1.0"
edition = "2027"

[dev-dependencies]
fixtures = { path = "../fixtures" }
```

<!-- file: examples/reference-diagnostics/e0276_dev_dependency.files/ledger/src/lib.sisu -->
```sisu
##! A sauna's takings, whose code imports the tests' fixtures.
from fixtures import bookings

## The takings of some bookings, in euros.
pub def takings(paid: list[int]) -> int:
    return sum(paid)

## The takings of the sample bookings.
pub def sample() -> int:
    return takings(bookings())

test "three bookings":
    assert takings(bookings()) == 36
```

<!-- output: examples/reference-diagnostics/e0276_dev_dependency.console -->
```console
$ sisu test ledger
error[E0276]: cannot import `fixtures` here: `fixtures` is a dev-dependency of `ledger`, which its tests import and its code does not
  --> src/lib.sisu:2:6
  |
2 | from fixtures import bookings
  |      ^^^^^^^^
  = note: a dev-dependency is for `test` blocks, benchmarks and the black-box tests of `tests/`: import it under `@cfg(test)`, or list it under `[dependencies]`

error[D0006]: the program was not run: errors found (1)
```

The note gives the two fixes. A dependency that the code needs belongs
under `[dependencies]`; one that only the tests need is imported under
`@cfg(test)`, and the code that is no test does without it:

<!-- file: examples/reference-diagnostics/e0276_dev_dependency_fixed.files/ledger/src/lib.sisu -->
```sisu
##! A sauna's takings, whose tests import their fixtures.
@cfg(test)
from fixtures import bookings

## The takings of some bookings, in euros.
pub def takings(paid: list[int]) -> int:
    return sum(paid)

test "three bookings":
    assert takings(bookings()) == 36
```

<!-- output: examples/reference-diagnostics/e0276_dev_dependency_fixed.console -->
```console
$ sisu test ledger
test "three bookings" ... ok
1 passed, 0 failed, 0 blocked
```

[R13](modules.md#dependencies-and-dev-dependencies) states the rule.

### E0277

`resolve.import-in-body`: an import stands at the top level of a
module, before its declarations, and its names are the whole module's;
a function's body holds none.

An import binds names for the whole module, so it is written at the
top level, as Python allows inside a function and Sisu does not. The
names it would bind are not reported again where the body uses them.

<!-- file: examples/reference-diagnostics/e0277_import_in_body.sisu -->
```sisu
# An import inside a function's body.
def main():
    let side = 9.0
    import std.math
    print(math.sqrt(side))
```

<!-- output: examples/reference-diagnostics/e0277_import_in_body.console -->
```console
$ sisu run e0277_import_in_body.sisu
error[E0277]: an import stands at the top level of a module, before its declarations, and its names are the whole module's; a function's body holds none
  --> e0277_import_in_body.sisu:4:5
  |
4 |     import std.math
  |     ^^^^^^^^^^^^^^^
  = fix: move it to the top of the module: `import std.math`

error[D0006]: the program was not run: errors found (1)
```

The fix, `resolve.move-import`, moves the import before the module's
first line that is not a comment, and the body keeps its use of the
name.

<!-- file: examples/reference-diagnostics/e0277_import_in_body_fixed.sisu -->
```sisu
# The import at the top of the module, where its name is the module's.
import std.math

def main():
    let side = 9.0
    print(math.sqrt(side))
```

<!-- output: examples/reference-diagnostics/e0277_import_in_body_fixed.out -->
```text
3.0
```

[R13](modules.md#imports) states the rule.

### E0278

`resolve.prelude-internal`: `{name}` is the prelude's own, for the
standard library's code: a program sees only the names it marks `pub`
(09).

Every module sees the prelude's fixed list of names
([R13](modules.md#names-every-module-sees)). The prelude declares more,
helpers that the standard library's own code calls, such as
`days_in_month` of the calendar or `DrawState` of the property tests,
and a program that names one is refused, where an unknown name would
be E0200 or E0201:

<!-- file: examples/reference-diagnostics/e0278_prelude_internal.sisu -->
```sisu
# A name that the prelude keeps for the standard library's own code.
def main():
    print(days_in_month(2028, 2))
```

<!-- output: examples/reference-diagnostics/e0278_prelude_internal.console -->
```console
$ sisu run e0278_prelude_internal.sisu
error[E0278]: `days_in_month` is the prelude's own, for the standard library's code: a program sees only the names it marks `pub` (09)
  --> e0278_prelude_internal.sisu:3:11
  |
3 |     print(days_in_month(2028, 2))
  |           ^^^^^^^^^^^^^
  = fix: import it from `std.time`: `from std.time import days_in_month`

error[D0006]: the program was not run: errors found (1)
```

Where a module of the standard library has the helper as a `pub` item,
the fix, `resolve.import-from-std`, imports it from there:
`days_in_month` is `pub` in `std.time`. The prelude's other helpers are
its own algorithms, with no fix; declare the function the program
needs. Here the import is enough:

<!-- file: examples/reference-diagnostics/e0278_prelude_internal_fixed.sisu -->
```sisu
# The program imports the function from the module that has it.
from std.time import days_in_month

def main():
    print(days_in_month(2028, 2))
```

<!-- output: examples/reference-diagnostics/e0278_prelude_internal_fixed.out -->
```text
29
```

E0278 is stage 1's; the Rust front end still lets a program name the
prelude's helpers.

### E0279

`resolve.plain-padding`: `{ty}` cannot derive `Plain`: C's layout
leaves {bytes} bytes of padding between `{before}` and `{after}`, at
offsets {start} to {end}, and a plain type has none (22).

A plain type's bytes are all its value's, so `as_bytes` and
`read_plain` read no byte that a field does not own. C's layout, which
`@repr(c)` gives a struct ([R14](attributes.md#the-layout-of-a-type)),
puts each field at the next offset its alignment allows, and a narrow
field before a wider one leaves a gap. The offsets name the gap's
first byte and its last, and the note names fields of numbers that
would fill it at C's offsets, each the widest that its offset allows:

<!-- file: examples/reference-diagnostics/e0279_plain_padding.sisu -->
```sisu
# A plain struct whose fields C's layout leaves a gap between.
from std.mem import Plain, size_of

@repr(c)
@derive(Plain, Copy)
struct Sample:
    sensor: u8
    value: i32

def main():
    print("{size_of[Sample]()} bytes")
```

<!-- output: examples/reference-diagnostics/e0279_plain_padding.console -->
```console
$ sisu run e0279_plain_padding.sisu
error[E0279]: `Sample` cannot derive `Plain`: C's layout leaves 3 bytes of padding between `sensor: u8` and `value: i32`, at offsets 1 to 3, and a plain type has none (22)
  --> e0279_plain_padding.sisu:5:9
  |
5 | @derive(Plain, Copy)
  |         ^^^^^
  = note: fields of their own there, `_pad1: u8, _pad2: u16`, make those bytes the struct's, or the fields ordered from the widest leave none

error[D0006]: the program was not run: errors found (1)
```

The note gives two ways out. Ordering the fields from the widest leaves
no gap between them, and two narrow fields that fill the rest of the
word leave none at the end:

<!-- file: examples/reference-diagnostics/e0279_plain_padding_fixed.sisu -->
```sisu
# The widest field first leaves C's layout no gap between the fields.
from std.mem import Plain, read_plain, size_of

@repr(c)
@derive(Plain, Copy, Debug)
struct Sample:
    value: i32
    sensor: u8
    level: u8
    flags: u16

def main():
    let data = b"\x2d\x00\x00\x00\x02\x07\x00\x00"
    print("{size_of[Sample]()} bytes")
    print("{read_plain[Sample](data, 0):?}")
```

<!-- output: examples/reference-diagnostics/e0279_plain_padding_fixed.out -->
```text
8 bytes
Sample(value=45, sensor=2, level=7, flags=0)
```

Shown in [chapter 27](../tutorial/27-systems.md#data-laid-out-as-c-lays-it-out).

### E0280

`resolve.plain-padding-end`: `{ty}` cannot derive `Plain`: C's layout
leaves {bytes} bytes of padding after `{before}`, at offsets {start} to
{end}, the end of the struct, and a plain type has none (22).

C rounds a struct's size up to a multiple of its widest field's
alignment, so fields that end before that multiple leave a gap at the
end, which an array of the struct repeats:

<!-- file: examples/reference-diagnostics/e0280_plain_padding_end.sisu -->
```sisu
# A plain struct that C's layout pads at its end.
from std.mem import Plain, size_of

@repr(c)
@derive(Plain, Copy)
struct Sample:
    value: i32
    sensor: u8

def main():
    print("{size_of[Sample]()} bytes")
```

<!-- output: examples/reference-diagnostics/e0280_plain_padding_end.console -->
```console
$ sisu run e0280_plain_padding_end.sisu
error[E0280]: `Sample` cannot derive `Plain`: C's layout leaves 3 bytes of padding after `sensor: u8`, at offsets 5 to 7, the end of the struct, and a plain type has none (22)
  --> e0280_plain_padding_end.sisu:5:9
  |
5 | @derive(Plain, Copy)
  |         ^^^^^
  = note: fields of their own there, `_pad5: u8, _pad6: u16`, make those bytes the struct's, or the fields ordered from the widest leave none

error[D0006]: the program was not run: errors found (1)
```

Give the bytes fields of their own, as the note names them, so that
they are the struct's and `as_bytes` writes them. A `u8` and a `u16` are
`Copy`, which `Plain` needs (E0244), where an `Array[u8, 3]` is not:

<!-- file: examples/reference-diagnostics/e0280_plain_padding_end_fixed.sisu -->
```sisu
# Fields of its own make the bytes at the end the struct's.
from std.mem import Plain, read_plain, size_of

@repr(c)
@derive(Plain, Copy, Debug)
struct Sample:
    value: i32
    sensor: u8
    _pad5: u8
    _pad6: u16

def main():
    let data = b"\x2d\x00\x00\x00\x02\x00\x00\x00"
    print("{size_of[Sample]()} bytes")
    print("{read_plain[Sample](data, 0).sensor}")
```

<!-- output: examples/reference-diagnostics/e0280_plain_padding_end_fixed.out -->
```text
8 bytes
2
```

Shown in [chapter 27](../tutorial/27-systems.md#data-laid-out-as-c-lays-it-out).

### E0281

`type.fixture-signature`: `{name}` is a `@fixture`, which a test names
as a parameter to receive what it returns, so it is a free function
that takes nothing.

A test's parameter that names a `@fixture` function gets what a call of
it returns, and the test gives the call no arguments, so a fixture with
a parameter, or a method marked `@fixture`, could never be called. A
test that names it is E0702 as well, a parameter the runner cannot
give:

<!-- file: examples/reference-diagnostics/e0281_fixture_signature.sisu -->
```sisu
# A fixture takes nothing, so no test could give it a parameter.
@fixture
def readings(count: int) -> list[int]:
    return [70 + i for i in range(count)]

test "three readings"(readings: list[int]):
    assert len(readings) == 3
```

<!-- output: examples/reference-diagnostics/e0281_fixture_signature.console -->
```console
$ sisu test e0281_fixture_signature.sisu
error[E0281]: `readings` is a `@fixture`, which a test names as a parameter to receive what it returns, so it is a free function that takes nothing
  --> e0281_fixture_signature.sisu:2:2
  |
2 | @fixture
  |  ^^^^^^^

error[E0702]: a test takes nothing, the real `System` as its one parameter (`test "name"(sys: System):`), or fresh fakes of the capabilities it names, one of each (`test "name"(fs: Fs, clock: Clock):`)
  --> e0281_fixture_signature.sisu:6:23
  |
6 | test "three readings"(readings: list[int]):
  |                       ^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

Make the value from nothing, with what it needs written in the body, or
as a constant of the module; a test that needs another value calls a
plain function itself:

<!-- file: examples/reference-diagnostics/e0281_fixture_signature_fixed.sisu -->
```sisu
# The fixture makes its value from nothing; a test names it to get one.
@fixture
def readings() -> list[int]:
    return [70 + i for i in range(3)]

test "three readings"(readings: list[int]):
    assert len(readings) == 3
```

<!-- output: examples/reference-diagnostics/e0281_fixture_signature_fixed.test -->
```text
test "three readings" ... ok
1 passed, 0 failed, 0 blocked
```

Shown in [chapter 19](../tutorial/19-testing.md#fixtures-that-tests-ask-for).

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
says when it cannot. A value given to a `ref dyn Trait` parameter whose
trait its type implements, `total(bun)`, gets the fix `type.lend-it`,
which writes `ref ` before it (`mut ` for a `mut dyn` parameter)
([chapter 15](../tutorial/15-generics.md)). A capability given where a
whole `System` is taken, a test's `TestClock`, has the note
`type.narrow-capability`: narrow the parameter to the capability,
`clock: Clock`, and let a caller that has a `System` pass `sys.clock`
([chapter 18](../tutorial/18-capabilities.md#exercise-182-a-clock-to-test-with)).

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

Shown in [chapter 2](../tutorial/02-values.md).

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
  = fix: divide them as floats: `.to_f64()`, `.0`

error[D0006]: the program was not run: errors found (1)
```

The first fix, `type.use-floor-division`, writes `//`, which keeps the
division in whole numbers. The second, `type.float-division`, divides
with a fraction: it makes both sides `f64`, with `.to_f64()` after a
name and `.0` after a literal, `pieces.to_f64() / 2.0`.

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

Shown in [chapter 2](../tutorial/02-values.md).

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0304

`type.unknown-method`: `{ty}` has no method `{name}`.

A method call names a method of the value's type or of a trait it
implements. A note names the trait when another type has the method, as
`join` of `Iterator`, and says when the value is a local that hides a
module of the same name, `fs: Fs` beside `import std.fs`
([R13](modules.md#imports)).

<!-- file: examples/reference-diagnostics/e0304_method.sisu -->
```sisu
# A method that `str` does not have.
def main():
    print("  hei  ".strip())
```

<!-- output: examples/reference-diagnostics/e0304_method.console -->
```console
$ sisu run e0304_method.sisu
error[E0304]: `str` has no method `strip`
  --> e0304_method.sisu:3:21
  |
3 |     print("  hei  ".strip())
  |                     ^^^^^
  = fix: Sisu spells it this way: `trim`

error[D0006]: the program was not run: errors found (1)
```

Use the type's method; [R19](false-friends.md) lists the Python and Rust
names that differ.

<!-- file: examples/reference-diagnostics/e0304_method_fixed.sisu -->
```sisu
# `trim` removes the spaces at both ends.
def main():
    print("  hei  ".trim())
```

<!-- output: examples/reference-diagnostics/e0304_method_fixed.out -->
```text
hei
```

Shown in [chapter 15](../tutorial/15-generics.md).

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
  = fix: write the value into the text: `"Hei, maailma!"`

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

Shown in [chapter 1](../tutorial/01-hello.md).

### E0306

`type.unknown-keyword`: `{name}` has no parameter named `{arg}`.

A keyword argument names a parameter of the function.

<!-- file: examples/reference-diagnostics/e0306_keyword_argument.sisu -->
```sisu
# A keyword argument that the function does not have.
def greet(name: str, polite: bool = False) -> String:
    return if polite: "Good day, {name}" else: "Hei, {name}"

def main():
    print(greet("Aino", formal=True))
```

<!-- output: examples/reference-diagnostics/e0306_keyword_argument.console -->
```console
$ sisu run e0306_keyword_argument.sisu
error[E0306]: `greet` has no parameter named `formal`
  --> e0306_keyword_argument.sisu:6:25
  |
6 |     print(greet("Aino", formal=True))
  |                         ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Use the parameter's name as the function declares it. A positional-only
parameter, one before `/`, has a name that no call may give: E0306 for
it has a note that says so and a fix that drops the name
([chapter 3](../tutorial/03-functions.md#passing-arguments-by-name)).

<!-- file: examples/reference-diagnostics/e0306_keyword_argument_fixed.sisu -->
```sisu
# The parameter's own name.
def greet(name: str, polite: bool = False) -> String:
    return if polite: "Good day, {name}" else: "Hei, {name}"

def main():
    print(greet("Aino", polite=True))
```

<!-- output: examples/reference-diagnostics/e0306_keyword_argument_fixed.out -->
```text
Good day, Aino
```

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
interpolation. Two sized integers neither of which holds the other's
values, an `i16` and a `u32`, get the fix `type.convert-to-int`, which
makes the left one an `int`, beside which the other widens
([chapter 24](../tutorial/24-numbers.md#integers-of-every-size)). A
number on the left of a type of the program's own, `3.0 * step`, gets
the note `type.operator-trait-left`, which names the impl the operator
needs, `impl Mul[Step] for f64`, which the package that declares `Step`
may write ([E0236](#e0236)), and the order that needs none, `step * 3.0`
([chapter 14](../tutorial/14-traits.md)). A compound assignment is
named as written: `total += name` with an `int` and a `String` is
"`+=` cannot be applied", in code that `comptime for` unrolls too.

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

Shown in [chapter 8](../tutorial/08-errors.md).

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0313

`type.not-a-place`: this expression cannot be assigned to.

An assignment changes a place: a `var`, a field, an index or a key. A
call's result, such as `len(temps)`, is a value with no place to change.

<!-- file: examples/reference-diagnostics/e0313_not_a_place.sisu -->
```sisu
# An assignment to a value that is not a place.
def main():
    var temps = [62, 75, 81]
    len(temps) = 0
    print(temps)
```

<!-- output: examples/reference-diagnostics/e0313_not_a_place.console -->
```console
$ sisu run e0313_not_a_place.sisu
error[E0313]: this expression cannot be assigned to
  --> e0313_not_a_place.sisu:4:5
  |
4 |     len(temps) = 0
  |     ^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Change the value through a method that changes it, here `clear`.

<!-- file: examples/reference-diagnostics/e0313_not_a_place_fixed.sisu -->
```sisu
# A method changes the list, which is a place.
def main():
    var temps = [62, 75, 81]
    temps.clear()
    print(temps)
```

<!-- output: examples/reference-diagnostics/e0313_not_a_place_fixed.out -->
```text
[]
```

Shown in [R5](statements.md#assignment).

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

Shown in [chapter 4](../tutorial/04-control-flow.md).

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

Shown in [chapter 3](../tutorial/03-functions.md).

### E0318

`type.unused-result`: this `{ty}` is not used; handle it, or discard it
with `_ = ...`.

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

Shown in [chapter 8](../tutorial/08-errors.md).

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

Shown in [chapter 8](../tutorial/08-errors.md).

### E0324

`type.cannot-infer`: the type of `{name}` cannot be inferred here; add a
type annotation.

Inside a body a binding's type comes from its value and its uses (D-003).
An empty list that nothing fills has no element type.

<!-- file: examples/reference-diagnostics/e0324_untyped_list.sisu -->
```sisu
# Nothing says what the list holds.
def main():
    let names = []
    print(len(names))
```

<!-- output: examples/reference-diagnostics/e0324_untyped_list.console -->
```console
$ sisu run e0324_untyped_list.sisu
error[E0324]: the type of `names` cannot be inferred here; add a type annotation
  --> e0324_untyped_list.sisu:3:9
  |
3 |     let names = []
  |         ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write the type, as the message says.

<!-- file: examples/reference-diagnostics/e0324_untyped_list_fixed.sisu -->
```sisu
# A type annotation says what the list holds.
def main():
    let names: list[str] = []
    print(len(names))
```

<!-- output: examples/reference-diagnostics/e0324_untyped_list_fixed.out -->
```text
0
```

Shown in [R2](types.md#static-types-and-local-inference).

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
  = fix: iterate over its characters: `.chars()`
  = fix: iterate over its bytes: `.bytes()`

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

Shown in [chapter 6](../tutorial/06-text.md).

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0328

`type.pattern-field-count`: wrong number of fields in the pattern for
`{name}`: expected {expected}, found {found}.

A struct or variant pattern gives a pattern for every field, by position
or by keyword, unless it ends with `..`, which stands for the fields it
leaves out.

<!-- file: examples/reference-diagnostics/e0328_field_count.sisu -->
```sisu
# A struct pattern that leaves out a field without `..`.
struct Point:
    x: int
    y: int

def describe(p: Point) -> str:
    return match p:
        case Point(x=0): "on the y axis"
        case _: "elsewhere"

def main():
    print(describe(Point(0, 4)))
```

<!-- output: examples/reference-diagnostics/e0328_field_count.console -->
```console
$ sisu run e0328_field_count.sisu
error[E0328]: wrong number of fields in the pattern for `Point`: expected 2, found 1
  --> e0328_field_count.sisu:8:14
  |
8 |         case Point(x=0): "on the y axis"
  |              ^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Add `..` at the end of the pattern, or a pattern for each field; `_` for
a field matches any value.

<!-- file: examples/reference-diagnostics/e0328_field_count_fixed.sisu -->
```sisu
# `..` stands for the fields the pattern does not name.
struct Point:
    x: int
    y: int

def describe(p: Point) -> str:
    return match p:
        case Point(x=0, ..): "on the y axis"
        case _: "elsewhere"

def main():
    print(describe(Point(0, 4)))
```

<!-- output: examples/reference-diagnostics/e0328_field_count_fixed.out -->
```text
on the y axis
```

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0333

`type.impure-call`: `{callee}` is not pure, so the pure function
`{name}` cannot call it.

A `pure` function gives the same result for the same arguments and does
nothing else, so it calls only `pure` functions. `print` writes to
standard output, which is not `pure`, and every method of an `Atomic`
but its construction observes the schedule, which a note then says
([R9](concurrency.md#atomics-and-orderings)). An adapter of an
iterator that takes a function, `map` or `filter`, is not `pure`,
whatever the function given, and E0333 at one has the note
`type.pure-adapter`, which names `fold` with a `pure` function and the
loop ([chapter 18](../tutorial/18-capabilities.md#code-that-takes-no-capability));
the calls after it in the same chain, the `count()` of
`filter(...).count()`, are no error of their own. The ownership checker reports it, so the run ends with D0021.

<!-- file: examples/reference-diagnostics/e0333_impure_call.sisu -->
```sisu
# A pure function that prints.
pure def feel(celsius: int) -> String:
    print("checking {celsius}")
    return if celsius > 80: "hot" else: "warm"

def main():
    print(feel(85))
```

<!-- output: examples/reference-diagnostics/e0333_impure_call.console -->
```console
$ sisu run e0333_impure_call.sisu
error[E0333]: `print` is not pure, so the pure function `feel` cannot call it
  --> e0333_impure_call.sisu:3:5
  |
3 |     print("checking {celsius}")
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
2 | pure def feel(celsius: int) -> String:
  |          ---- `feel` is declared `pure` here

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Keep the output in the caller, as the fixed program does, or remove
`pure` if the function needs to print ([chapter
26](../tutorial/26-purity.md)).

<!-- file: examples/reference-diagnostics/e0333_impure_call_fixed.sisu -->
```sisu
# The pure function computes, and its caller prints.
pure def feel(celsius: int) -> String:
    return if celsius > 80: "hot" else: "warm"

def main():
    print("checking 85")
    print(feel(85))
```

<!-- output: examples/reference-diagnostics/e0333_impure_call_fixed.out -->
```text
checking 85
hot
```

Shown in [chapter 26](../tutorial/26-purity.md).

### E0334

`type.impure-capability`: the pure function `{name}` cannot use the
capability `{ty}`.

A capability reaches the world, whose answers change from call to call,
so a `pure` function takes none ([R10](capabilities.md)).

<!-- file: examples/reference-diagnostics/e0334_impure_capability.sisu -->
```sisu
# A pure function that takes a file system.
pure def has_notes(fs: Fs) -> bool:
    return fs.exists("notes.txt")

def main(sys: System):
    print(has_notes(sys.fs))
```

<!-- output: examples/reference-diagnostics/e0334_impure_capability.console -->
```console
$ sisu run e0334_impure_capability.sisu
error[E0334]: the pure function `has_notes` cannot use the capability `Fs`
  --> e0334_impure_capability.sisu:2:20
  |
2 | pure def has_notes(fs: Fs) -> bool:
  |                    ^^^^^^
  = fix: remove `pure`

error[D0006]: the program was not run: errors found (1)
```

The fix removes `pure`. Where the work itself is pure, read the file in
the caller and pass the text to a `pure` function instead.

<!-- file: examples/reference-diagnostics/e0334_impure_capability_fixed.sisu -->
```sisu
# The function that reaches the file system is not pure.
def has_notes(fs: Fs) -> bool:
    return fs.exists("notes.txt")

def main(sys: System):
    print(has_notes(sys.fs))
```

<!-- output: examples/reference-diagnostics/e0334_impure_capability_fixed.out -->
```text
False
```

Shown in [chapter 18](../tutorial/18-capabilities.md).

### E0335

`type.impure-argument`: this function is passed where a pure function
is required, but it calls `{callee}`, which is not pure.

A parameter of a `pure` function type accepts only functions that are
`pure` (D-074), so the code that receives it can rely on the same
result for the same arguments.

<!-- file: examples/reference-diagnostics/e0335_impure_argument.sisu -->
```sisu
# A function that prints, passed where a pure function is required.
def apply_twice(f: pure (int) -> int, x: int) -> int:
    return f(f(x))

def noisy_double(n: int) -> int:
    print("doubling {n}")
    return n * 2

def main():
    print(apply_twice(noisy_double, 3))
```

<!-- output: examples/reference-diagnostics/e0335_impure_argument.console -->
```console
$ sisu run e0335_impure_argument.sisu
error[E0335]: this function is passed where a pure function is required, but it calls `print`, which is not pure
  --> e0335_impure_argument.sisu:10:23
   |
10 |     print(apply_twice(noisy_double, 3))
   |                       ^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Pass a `pure` function. The place is the function passed,
`noisy_double`, and the message names the operation inside it that
makes it impure, `print`. When that operation is in a function that
the one passed calls, the note `type.impure-path` gives the way, such
as "`helper` calls `noisy_double`, which calls `print`".

<!-- file: examples/reference-diagnostics/e0335_impure_argument_fixed.sisu -->
```sisu
# A pure function, which prints nothing.
def apply_twice(f: pure (int) -> int, x: int) -> int:
    return f(f(x))

pure def double(n: int) -> int:
    return n * 2

def main():
    print(apply_twice(double, 3))
```

<!-- output: examples/reference-diagnostics/e0335_impure_argument_fixed.out -->
```text
12
```

Shown in [chapter 16](../tutorial/16-closures-and-iterators.md).

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0338

`type.move-out-of-borrow`: cannot move `{what}` out of a borrowed value;
copy it with `.copy()`.

Indexing reads through a borrow of the collection, so it cannot move an
element that is not `Copy` out of it: the collection would be left with
a hole.

<!-- file: examples/reference-diagnostics/e0338_move_out_of_borrow.sisu -->
```sisu
# An element moved out of a list that is borrowed.
def main():
    let queue: list[String] = ["Aino", "Eero"]
    let first = queue[0]
    print("{first} is first of {len(queue)}")
```

<!-- output: examples/reference-diagnostics/e0338_move_out_of_borrow.console -->
```console
$ sisu run e0338_move_out_of_borrow.sisu
error[E0338]: cannot move `queue[0]` out of a borrowed value; copy it with `.copy()`
  --> e0338_move_out_of_borrow.sisu:4:17
  |
4 |     let first = queue[0]
  |                 ^^^^^^^^
  = note: `queue` owns its list, which keeps the item `queue[0]` reads and only lends it: copy the item, or take it out of the list
  = fix: copy it: `.copy()`

error[D0006]: the program was not run: errors found (1)
```

For an item of a list the function owns, as here, the note
`type.item-stays-in-list` says that the list keeps the item and only
lends it. The fix copies the element, which costs a copy. To take it
out instead, use a method that removes it, such as `pop` or `remove`,
on a `var` collection: where `queue` is a `var`, a second fix,
`type.take-out`, writes `queue.remove(0)`.

<!-- file: examples/reference-diagnostics/e0338_move_out_of_borrow_fixed.sisu -->
```sisu
# A copy of the element, which the list keeps.
def main():
    let queue: list[String] = ["Aino", "Eero"]
    let first = queue[0].copy()
    print("{first} is first of {len(queue)}")
```

<!-- output: examples/reference-diagnostics/e0338_move_out_of_borrow_fixed.out -->
```text
Aino is first of 2
```

Shown in [chapter 10](../tutorial/10-ownership.md).

### E0339

`type.mut-argument-missing`: `{param}` is a `mut` parameter, so the call
must say `mut` before the argument.

A call passes an argument to a `mut` parameter with `mut` before it, so
that the reader of the call sees what can change (D-006).

<!-- file: examples/reference-diagnostics/e0339_mut_at_call.sisu -->
```sisu
# A `mut` parameter called without `mut` at the call.
def deposit(mut balance: int, amount: int):
    balance += amount

def main():
    var balance = 100
    deposit(balance, 50)
    print(balance)
```

<!-- output: examples/reference-diagnostics/e0339_mut_at_call.console -->
```console
$ sisu run e0339_mut_at_call.sisu
error[E0339]: `balance` is a `mut` parameter, so the call must say `mut` before the argument
  --> e0339_mut_at_call.sisu:7:13
  |
7 |     deposit(balance, 50)
  |             ^^^^^^^
  = fix: write `mut` before the argument: `mut `

error[D0006]: the program was not run: errors found (1)
```

The fix writes `mut` before the argument.

<!-- file: examples/reference-diagnostics/e0339_mut_at_call_fixed.sisu -->
```sisu
# The call says `mut` too.
def deposit(mut balance: int, amount: int):
    balance += amount

def main():
    var balance = 100
    deposit(mut balance, 50)
    print(balance)
```

<!-- output: examples/reference-diagnostics/e0339_mut_at_call_fixed.out -->
```text
150
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0340

`type.mut-argument-unexpected`: `{param}` is not a `mut` parameter, so
the argument takes no `mut`.

`mut` at a call says that the callee changes the argument, and it is
written exactly where the parameter is `mut` (D-006), so a reader of the
call knows what may change.

<!-- file: examples/reference-diagnostics/e0340_extra_mut.sisu -->
```sisu
# `mut` at a call whose parameter only reads.
def total(prices: list[int]) -> int:
    return sum(prices)

def main():
    var prices = [350, 280]
    print(total(mut prices))
```

<!-- output: examples/reference-diagnostics/e0340_extra_mut.console -->
```console
$ sisu run e0340_extra_mut.sisu
error[E0340]: `prices` is not a `mut` parameter, so the argument takes no `mut`
  --> e0340_extra_mut.sisu:7:17
  |
7 |     print(total(mut prices))
  |                 ^^^^^^^^^^
  = fix: remove `mut`

error[D0006]: the program was not run: errors found (1)
```

The fix removes `mut`: the call lends the list to read. When the
function is meant to change it, make the parameter `mut` instead.

<!-- file: examples/reference-diagnostics/e0340_extra_mut_fixed.sisu -->
```sisu
# The call lends the list to read, without `mut`.
def total(prices: list[int]) -> int:
    return sum(prices)

def main():
    var prices = [350, 280]
    print(total(prices))
```

<!-- output: examples/reference-diagnostics/e0340_extra_mut_fixed.out -->
```text
630
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0342

`type.it-outside-closure`: `it` is the parameter of an implicit closure
and exists only where a one-parameter function is expected.

`it` is the parameter of an implicit closure: `map(it + 1)` is `map(x ->
x + 1)`. It exists only in an argument where a function of one parameter
is expected ([R4](expressions.md#closures)).

<!-- file: examples/reference-diagnostics/e0342_it_outside_closure.sisu -->
```sisu
# `it` where no function is expected.
def main():
    let ages = [7, 34, 61]
    let next = it + 1
    print(next)
```

<!-- output: examples/reference-diagnostics/e0342_it_outside_closure.console -->
```console
$ sisu run e0342_it_outside_closure.sisu
error[E0342]: `it` is the parameter of an implicit closure and exists only where a one-parameter function is expected
  --> e0342_it_outside_closure.sisu:4:16
  |
4 |     let next = it + 1
  |                ^^
  = note: `it` makes a closure of a call's argument only; elsewhere, as for an item of a list of functions, write the closure with its parameter: `x -> ...`

error[D0006]: the program was not run: errors found (1)
```

Pass the expression where a function is expected, or, as the note
says, write the closure with a named parameter, `x -> x + 1`. A method that a list lacks, such
as `map`, is reached through `iter()` first.

<!-- file: examples/reference-diagnostics/e0342_it_outside_closure_fixed.sisu -->
```sisu
# `it` is the parameter of a closure passed where a function is expected.
def main():
    let ages = [7, 34, 61]
    let next = ages.iter().map(it + 1).collect()
    print(next)
```

<!-- output: examples/reference-diagnostics/e0342_it_outside_closure_fixed.out -->
```text
[8, 35, 62]
```

Shown in [chapter 16](../tutorial/16-closures-and-iterators.md).

### E0343

`type.shared-not-mutable`: cannot modify `{what}`: a `Shared` holds it,
and a shared value is not changed in place, even through a `var`.

Every owner of a `Shared` sees the value it holds, so none of them
changes it in place: a field assigned through a `Shared` is E0343,
whether the binding is a `let` or a `var`
([R7](ownership.md#sharing-cells-and-graphs)).

<!-- file: examples/reference-diagnostics/e0343_shared_not_mutable.sisu -->
```sisu
# A change to a field of a value that a `Shared` holds.
struct Stove:
    celsius: int

def main():
    var stove = Shared(Stove(60))
    stove.celsius = 80
    print(stove.celsius)
```

<!-- output: examples/reference-diagnostics/e0343_shared_not_mutable.console -->
```console
$ sisu run e0343_shared_not_mutable.sisu
error[E0343]: cannot modify `stove.celsius`: a `Shared` holds it, and a shared value is not changed in place, even through a `var`
  --> e0343_shared_not_mutable.sisu:7:5
  |
7 |     stove.celsius = 80
  |     ^^^^^^^^^^^^^
  = note: what a `Shared` holds is shared, so it is not changed in place: put the part that changes in a `Cell` or a `Mutex` inside it, and change it through that

error[D0006]: the program was not run: errors found (1)
```

The note `type.shared-changes` names the way out: the part that changes
goes in a `Cell`, or a `Mutex` when tasks share it, and every owner
changes it through that and sees the change.

<!-- file: examples/reference-diagnostics/e0343_shared_not_mutable_fixed.sisu -->
```sisu
# The part that changes is a `Cell` inside the `Shared`.
struct Stove:
    celsius: Cell[int]

def main():
    let stove = Shared(Stove(Cell(60)))
    let other = stove.copy()
    other.celsius.set(80)
    print(stove.celsius.get())
```

<!-- output: examples/reference-diagnostics/e0343_shared_not_mutable_fixed.out -->
```text
80
```

Shown in [chapter 13](../tutorial/13-sharing.md#values-with-several-owners).

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

Shown in [chapter 5](../tutorial/05-collections.md).

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

Shown in [chapter 6](../tutorial/06-text.md).

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
  = fix: derive it: `@derive(Eq)`

error[D0006]: the program was not run: errors found (1)
```

Derive the trait with `@derive(...)` on the type, or write an `impl` of
it. In code that a derive wrote, E0347 stands at the derive's argument
that asked for the impl, with the note `type.derived-code`, which
quotes the line of the derived impl, and without a fix, since that line
is not in the program ([chapter 25](../tutorial/25-compile-time.md)
shows it). Where a derived impl needs the trait of a type argument,
`==` of a `Pair[Plain]` whose `Pair` derives `Eq`, the fix
`type.derive-trait-for` derives it for that type: "derive it for
`Plain`". For an `f64` that is not `Ord`, `sorted` of a `list[f64]`,
the note `type.total-order` names `x.total_order()`, a value in a total
order, and `max_by` and `min_by`, which take the `f64` itself
([chapter 24](../tutorial/24-numbers.md#nan-and-the-order-of-floats));
no fix writes an `impl Ord for f64`, which only the root package may
write.

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0351

`type.unknown-associated`: `{ty}` has no associated function `{name}`.

`Type.name(...)` calls an associated function: a function of the type's
body, or of an impl of it, without `self`. A name that the type has
neither as an associated function nor as a constant is this error
([R3](declarations.md)), except one that a derive refused by
[E0607](#e0607) would have written, which that error stands for.

<!-- file: examples/reference-diagnostics/e0351_unknown_associated.sisu -->
```sisu
# An associated function that the type does not have.
struct Point:
    x: int
    y: int

def main():
    let p = Point.origin()
    print(p.x)
```

<!-- output: examples/reference-diagnostics/e0351_unknown_associated.console -->
```console
$ sisu run e0351_unknown_associated.sisu
error[E0351]: `Point` has no associated function `origin`
  --> e0351_unknown_associated.sisu:7:19
  |
7 |     let p = Point.origin()
  |                   ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Declare the function in the type's body, as the fixed program does, or
call one the type has. A variant that an enum does not have,
`Light.Blue`, is E0327 instead, which says that `Light` has no variant
`Blue`.

<!-- file: examples/reference-diagnostics/e0351_unknown_associated_fixed.sisu -->
```sisu
# The associated function, declared in the type's body.
struct Point:
    x: int
    y: int

    def origin() -> Point:
        return Point(0, 0)

def main():
    let p = Point.origin()
    print(p.x)
```

<!-- output: examples/reference-diagnostics/e0351_unknown_associated_fixed.out -->
```text
0
```

Shown in the entry for [E0608](#e0608).

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
  = fix: derive it: `@derive(PartialOrd)`

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

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
  = fix: convert the `int`: `.to_f64()`

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0374

`type.literal-out-of-range`: `{text}` does not fit in `{ty}`.

An integer literal fits in the type it gets. The note gives the type's
range. For a literal beyond `int`, a second note, `type.bigint-literal`,
names `BigInt.parse` and the literal beside a `BigInt`, which becomes
one ([chapter 24](../tutorial/24-numbers.md#integers-of-any-size)).

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

Shown in [chapter 2](../tutorial/02-values.md).

### E0385

`type.bindings-in-or-pattern`: `{name}` is bound in some alternatives of
`\|` but not in all; every alternative must bind the same names.

The alternatives of `|` run the same arm, so every alternative binds the
same names, with the same types.

<!-- file: examples/reference-diagnostics/e0385_or_bindings.sisu -->
```sisu
# An alternative that binds a name the other does not.
enum Reading:
    Celsius(f64)
    Missing

def value(r: Reading) -> f64:
    return match r:
        case Celsius(t) | Missing: t

def main():
    print(value(Missing))
```

<!-- output: examples/reference-diagnostics/e0385_or_bindings.console -->
```console
$ sisu run e0385_or_bindings.sisu
error[E0385]: `t` is bound in some alternatives of `|` but not in all; every alternative must bind the same names
  --> e0385_or_bindings.sisu:8:27
  |
8 |         case Celsius(t) | Missing: t
  |                           ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Bind the name in every alternative, or give the alternatives arms of
their own.

<!-- file: examples/reference-diagnostics/e0385_or_bindings_fixed.sisu -->
```sisu
# Each alternative in an arm of its own.
enum Reading:
    Celsius(f64)
    Missing

def value(r: Reading) -> f64:
    return match r:
        case Celsius(t): t
        case Missing: 0.0

def main():
    print(value(Missing))
```

<!-- output: examples/reference-diagnostics/e0385_or_bindings_fixed.out -->
```text
0.0
```

### E0389

`type.no-self`: `self` exists only inside a method.

`self` names the value a method is called on, so it exists only inside a
method.

<!-- file: examples/reference-diagnostics/e0389_self.sisu -->
```sisu
# `self` used in a function that is not a method.
def describe() -> String:
    return "{self}"

def main():
    print(describe())
```

<!-- output: examples/reference-diagnostics/e0389_self.console -->
```console
$ sisu run e0389_self.sisu
error[E0389]: `self` exists only inside a method
  --> e0389_self.sisu:3:14
  |
3 |     return "{self}"
  |              ^^^^

error[D0006]: the program was not run: errors found (1)
```

Pass the value as a parameter, or move the function into the type's body
as a method.

<!-- file: examples/reference-diagnostics/e0389_self_fixed.sisu -->
```sisu
# The value passed as a parameter.
def describe(name: str) -> String:
    return "{name}"

def main():
    print(describe("Aino"))
```

<!-- output: examples/reference-diagnostics/e0389_self_fixed.out -->
```text
Aino
```

### E0392

`type.impure-value-call`: `{name}` is pure, so it cannot call a function
value whose type is not `pure`.

A `pure` function calls only `pure` functions, and a function value of
type `(int) -> int` may be any function, `print` included. The type
`pure (int) -> int` admits only pure ones
([R2](types.md#function-types)).

<!-- file: examples/reference-diagnostics/e0392_impure_value_call.sisu -->
```sisu
# A pure function that calls a function value that may not be pure.
pure def apply(f: (int) -> int, x: int) -> int:
    return f(x) + 1

def main():
    print(apply(n -> n * 3, 4))
```

<!-- output: examples/reference-diagnostics/e0392_impure_value_call.console -->
```console
$ sisu run e0392_impure_value_call.sisu
error[E0392]: `apply` is pure, so it cannot call a function value whose type is not `pure`
  --> e0392_impure_value_call.sisu:3:12
  |
3 |     return f(x) + 1
  |            ^^^^
  |
2 | pure def apply(f: (int) -> int, x: int) -> int:
  |          ----- `apply` is declared `pure` here

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Write `pure` in the parameter's type, as the fixed program does; a
caller that passes a function that is not pure then gets the error at
its call.

<!-- file: examples/reference-diagnostics/e0392_impure_value_call_fixed.sisu -->
```sisu
# The parameter's type says that the function it takes is pure.
pure def apply(f: pure (int) -> int, x: int) -> int:
    return f(x) + 1

def main():
    print(apply(n -> n * 3, 4))
```

<!-- output: examples/reference-diagnostics/e0392_impure_value_call_fixed.out -->
```text
13
```

Shown in [chapter 26](../tutorial/26-purity.md).

### E0395

`type.unsized-dyn`: `{ty}` has no size of its own; hold it behind `ref`
or in a `Box`.

A `dyn Trait` value can be of any size, since any type that implements
the trait can stand there, so it is held behind `ref` or in a `Box`.

<!-- file: examples/reference-diagnostics/e0395_bare_dyn.sisu -->
```sisu
# A `dyn` value passed by itself.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show(item: dyn Priced) -> String:
    return "{item.price()} cents"

def main():
    print(show(Bun("cardamom")))
```

<!-- output: examples/reference-diagnostics/e0395_bare_dyn.console -->
```console
$ sisu run e0395_bare_dyn.sisu
error[E0395]: `dyn Priced` has no size of its own; hold it behind `ref` or in a `Box`
  --> e0395_bare_dyn.sisu:12:10
   |
12 | def show(item: dyn Priced) -> String:
   |          ^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Hold it behind `ref`, or in a `Box` to own it.

<!-- file: examples/reference-diagnostics/e0395_bare_dyn_fixed.sisu -->
```sisu
# The `dyn` value behind `ref`.
trait Priced:
    def price(self) -> int

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def show(item: ref dyn Priced) -> String:
    return "{item.price()} cents"

def main():
    let bun = Bun("cardamom")
    print(show(ref bun))
```

<!-- output: examples/reference-diagnostics/e0395_bare_dyn_fixed.out -->
```text
280 cents
```

Shown in [chapter 15](../tutorial/15-generics.md).

### E0398

`type.structural-only`: {callee} is hashed and compared structurally in
stage 0, and `{ty}` has an impl of `{trait}` of its own; derive `Hash`
and `Eq` for it, or key by a field.

Stage 0 hashes and compares a `dict`'s keys and a `set`'s elements
structurally, field by field, so a key type whose `Hash` or `Eq` is
written by hand would be treated otherwise than its impl says; it is
refused instead.

<!-- file: examples/reference-diagnostics/e0398_own_hash.sisu -->
```sisu
# A dict key whose type has an `Eq` and a `Hash` of its own.
struct Room:
    number: int

impl Eq for Room:
    def eq(self, other: Room) -> bool:
        return self.number == other.number

impl Hash for Room:
    def hash(self, mut h: Hasher):
        self.number.hash(mut h)

def main():
    let guests = {Room(1): "Aino", Room(2): "Eero"}
    print(guests[Room(2)])
```

<!-- output: examples/reference-diagnostics/e0398_own_hash.console -->
```console
$ sisu run e0398_own_hash.sisu
error[E0398]: a `dict` or `set` key is hashed and compared structurally in stage 0, and `Room` has an impl of `Hash` of its own; derive `Hash` and `Eq` for it, or key by a field
  --> e0398_own_hash.sisu:14:18
   |
14 |     let guests = {Room(1): "Aino", Room(2): "Eero"}
   |                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Derive `Eq` and `Hash`, which compare field by field, or key the
`dict` by a field such as `number`.

<!-- file: examples/reference-diagnostics/e0398_own_hash_fixed.sisu -->
```sisu
# Derived `Eq` and `Hash`, which the host compares structurally.
@derive(Eq, Hash)
struct Room:
    number: int

def main():
    let guests = {Room(1): "Aino", Room(2): "Eero"}
    print(guests[Room(2)])
```

<!-- output: examples/reference-diagnostics/e0398_own_hash_fixed.out -->
```text
Eero
```

Shown in [chapter 13](../tutorial/13-sharing.md).

### E0399

`type.bad-format-spec`: `{spec}` is not a format spec; it is
`[[fill]align][sign][#][0][width][,|_][.precision][type]`, as in
`>8.2f`, with the type one of `d x X o b e E f F % s ?`.

The text after `:` in an interpolation is a format spec, in the form the
message gives: a fill and an alignment, a sign, a width, grouping, a
precision and a type letter
([R4](expressions.md#interpolation-and-format-specs)).

<!-- file: examples/reference-diagnostics/e0399_bad_format_spec.sisu -->
```sisu
# A format spec with a type letter that Sisu does not have.
def main():
    let price = 4.5
    print("{price:>8.2z}")
```

<!-- output: examples/reference-diagnostics/e0399_bad_format_spec.console -->
```console
$ sisu run e0399_bad_format_spec.sisu
error[E0399]: `>8.2z` is not a format spec; it is `[[fill]align][sign][#][0][width][,|_][.precision][type]`, as in `>8.2f`, with the type one of `d x X o b e E f F % s ?`
  --> e0399_bad_format_spec.sisu:4:13
  |
4 |     print("{price:>8.2z}")
  |             ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Use one of the type letters the message lists: `f` for a fixed number of
decimals, `e` for an exponent, `%` for a percentage, `?` for the `Debug`
form.

<!-- file: examples/reference-diagnostics/e0399_bad_format_spec_fixed.sisu -->
```sisu
# `f` writes a float with a fixed number of decimals.
def main():
    let price = 4.5
    print("{price:>8.2f}")
```

<!-- output: examples/reference-diagnostics/e0399_bad_format_spec_fixed.out -->
```text
    4.50
```

Shown in [R4](expressions.md#interpolation-and-format-specs).

## Ownership

Codes E0400 to E0449. The ownership checker checks moves, borrows,
views, tasks and must-consume values
([R7](ownership.md)). E0445 to E0448, numbered after its codes, are the
type checker's: a typed hole in a release build, the body of a parallel
loop, and the argument of `@float`.

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
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

The note offers three ways, each with its cost: copy the value before
the move, which makes a second value; use the old name before the move,
which costs nothing; or, where what the value moves into only reads it,
take it there without `own`, so that the call lends it.

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

Shown in [chapter 5](../tutorial/05-collections.md).

### E0401

`borrow.use-after-partial-move`: `{name}` cannot be used as a whole,
because `{part}` was moved out of it on line {line}.

Moving a field out of a struct leaves the struct without it: the other
fields can still be used, and the struct as a whole cannot.

<!-- file: examples/reference-diagnostics/e0401_partial_move.sisu -->
```sisu
# A struct used whole after a field moved out of it.
struct Order:
    customer: String
    items: list[String]

def main():
    let order = Order("Aino", ["coffee", "bun"])
    let items = order.items
    print("{len(items)} items")
    print("{order:?}")
```

<!-- output: examples/reference-diagnostics/e0401_partial_move.console -->
```console
$ sisu run e0401_partial_move.sisu
error[E0401]: `order` cannot be used as a whole, because `order.items` was moved out of it on line 8
  --> e0401_partial_move.sisu:10:13
   |
10 |     print("{order:?}")
   |             ^^^^^
   |
 8 |     let items = order.items
   |                 ----------- `order.items` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Use the whole struct before the move, as the note says, or copy the
field with `.copy()` and leave the struct whole.

<!-- file: examples/reference-diagnostics/e0401_partial_move_fixed.sisu -->
```sisu
# The struct used before the field moves out.
struct Order:
    customer: String
    items: list[String]

def main():
    let order = Order("Aino", ["coffee", "bun"])
    print("{order:?}")
    let items = order.items
    print("{len(items)} items")
```

<!-- output: examples/reference-diagnostics/e0401_partial_move_fixed.out -->
```text
Order(customer="Aino", items=["coffee", "bun"])
2 items
```

Shown in [chapter 10](../tutorial/10-ownership.md).

### E0402

`borrow.modify-while-borrowed`: `{name}` is borrowed on line {line} and
cannot be modified until that borrow ends on line {end}.

A value that is borrowed cannot change until the borrow's last use. A
borrow ends at its last use, not at the end of its block. Tasks that a
loop spawns, each changing a captured `var`, get E0402 too, with the
note `task.shared-change`, which says that the tasks run at once and
names a `Mutex` ([chapter 20](../tutorial/20-concurrency.md#sharing-a-value-that-changes)).

<!-- file: examples/reference-diagnostics/e0402_modify_while_borrowed.sisu -->
```sisu
# A list changed while a borrow of an element is still used.
def main():
    var names: list[String] = ["Aino", "Eero", "Roni"]
    let first = ref names[0]
    names[1] = "Ella"
    print("{first} is first")
```

<!-- output: examples/reference-diagnostics/e0402_modify_while_borrowed.console -->
```console
$ sisu run e0402_modify_while_borrowed.sisu
error[E0402]: `names` is borrowed on line 4 and cannot be modified until that borrow ends on line 6
  --> e0402_modify_while_borrowed.sisu:5:5
  |
5 |     names[1] = "Ella"
  |     ^^^^^^^^^^^^^^^^^
  |
4 |     let first = ref names[0]
  |                 ------------ borrowed here by `first`
  |
6 |     print("{first} is first")
  |             ----- the borrow is still used here
  = note: move this after line 6, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Move the change after the line the note names, or work on a copy.
Where the binding is one a loop makes anew, its variable in `for turn
in turns:` while `who = turn.name` still borrows the one before, the
note is `borrow.bound-anew`: moving the change is no way out, so it
says to keep a copy that owns its value, `turn.name.copy()` in a
`String`.

<!-- file: examples/reference-diagnostics/e0402_modify_while_borrowed_fixed.sisu -->
```sisu
# The change after the borrow's last use.
def main():
    var names: list[String] = ["Aino", "Eero", "Roni"]
    let first = ref names[0]
    print("{first} is first")
    names[1] = "Ella"
```

<!-- output: examples/reference-diagnostics/e0402_modify_while_borrowed_fixed.out -->
```text
Aino is first
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0403

`borrow.use-while-mut-borrowed`: `{name}` is borrowed with `mut` on line
{line} and cannot be used until that borrow ends on line {end}.

A borrow with `mut` excludes every other use of the value until its last
use, so nothing reads the value while it is being changed.

<!-- file: examples/reference-diagnostics/e0403_use_while_mut.sisu -->
```sisu
# A list read while a `mut` borrow of it is still in use.
def main():
    var queue = ["Aino", "Eero"]
    let back = ref mut queue
    print(len(queue))
    back.push("Roni")
```

<!-- output: examples/reference-diagnostics/e0403_use_while_mut.console -->
```console
$ sisu run e0403_use_while_mut.sisu
error[E0403]: `queue` is borrowed with `mut` on line 4 and cannot be used until that borrow ends on line 6
  --> e0403_use_while_mut.sisu:5:15
  |
5 |     print(len(queue))
  |               ^^^^^
  |
4 |     let back = ref mut queue
  |                ------------- borrowed here by `back`
  |
6 |     back.push("Roni")
  |     ---- the borrow is still used here
  = note: move this after line 6, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Move the read after the last use of the borrow, as the note says, or
read a copy.

<!-- file: examples/reference-diagnostics/e0403_use_while_mut_fixed.sisu -->
```sisu
# The read comes after the last use of the `mut` borrow.
def main():
    var queue = ["Aino", "Eero"]
    let back = ref mut queue
    back.push("Roni")
    print(len(queue))
```

<!-- output: examples/reference-diagnostics/e0403_use_while_mut_fixed.out -->
```text
3
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0404

`borrow.move-while-borrowed`: `{name}` is borrowed on line {line} and
cannot be moved until that borrow ends on line {end}.

A value that is borrowed stays where it is until the borrow's last use,
since the borrow points at it.

<!-- file: examples/reference-diagnostics/e0404_move_while_borrowed.sisu -->
```sisu
# A list moved while a borrow of it is still in use.
def main():
    let names = ["Aino", "Eero"]
    let first = ref names[0]
    let team = names
    print("{first} {len(team)}")
```

<!-- output: examples/reference-diagnostics/e0404_move_while_borrowed.console -->
```console
$ sisu run e0404_move_while_borrowed.sisu
error[E0404]: `names` is borrowed on line 4 and cannot be moved until that borrow ends on line 6
  --> e0404_move_while_borrowed.sisu:5:16
  |
5 |     let team = names
  |                ^^^^^
  |
4 |     let first = ref names[0]
  |                 ------------ borrowed here by `first`
  |
6 |     print("{first} {len(team)}")
  |             ----- the borrow is still used here
  = note: move this after line 6, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Use the borrow before the move, as here, or move a copy.

<!-- file: examples/reference-diagnostics/e0404_move_while_borrowed_fixed.sisu -->
```sisu
# The borrow's last use comes before the move.
def main():
    let names = ["Aino", "Eero"]
    let first = ref names[0]
    print(first)
    let team = names
    print(len(team))
```

<!-- output: examples/reference-diagnostics/e0404_move_while_borrowed_fixed.out -->
```text
Aino
2
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0405

`borrow.mut-while-borrowed`: `{name}` is already borrowed on line
{line}, so it cannot be borrowed with `mut` until that borrow ends on
line {end}.

A value has either one `mut` borrow or any number of shared ones at a
time. A call of a `mut self` method borrows with `mut`.

<!-- file: examples/reference-diagnostics/e0405_mut_while_borrowed.sisu -->
```sisu
# A method that changes a list called while it is borrowed.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    scores.push(10)
    print("first {first}, of {len(scores)}")
```

<!-- output: examples/reference-diagnostics/e0405_mut_while_borrowed.console -->
```console
$ sisu run e0405_mut_while_borrowed.sisu
error[E0405]: `scores` is already borrowed on line 4, so it cannot be borrowed with `mut` until that borrow ends on line 6
  --> e0405_mut_while_borrowed.sisu:5:5
  |
5 |     scores.push(10)
  |     ^^^^^^^^^^^^^^^
  |
4 |     let first = ref scores[0]
  |                 ------------- borrowed here by `first`
  |
6 |     print("first {first}, of {len(scores)}")
  |                   ----- the borrow is still used here
  = note: move this after line 6, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Finish with the shared borrow first, as the note says, or copy what it
reads. When the change stands inside a loop over the value, the borrow
lasts through the loop, and the note says so: change the value after
the loop, or loop over a copy.

<!-- file: examples/reference-diagnostics/e0405_mut_while_borrowed_fixed.sisu -->
```sisu
# The borrow's last use before the change.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    print("first {first}")
    scores.push(10)
    print("{len(scores)} scores")
```

<!-- output: examples/reference-diagnostics/e0405_mut_while_borrowed_fixed.out -->
```text
first 7
4 scores
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0406

`borrow.does-not-live-long-enough`: `{name}` does not live long enough:
it is borrowed on line {line}, and the borrow is still used on line
{end}, after `{name}` is gone.

A borrow cannot be used after what it borrows is gone. The message names
the line of the borrow and the line of its last use.

<!-- file: examples/reference-diagnostics/e0406_short_life.sisu -->
```sisu
# A closure that borrows the loop's `i` outlives the iteration.
def main():
    var steps: list[() -> int] = []
    for i in range(3):
        steps.push(() -> i)
    print(len(steps))
```

<!-- output: examples/reference-diagnostics/e0406_short_life.console -->
```console
$ sisu run e0406_short_life.sisu
error[E0406]: `i` does not live long enough: it is borrowed on line 5, and the borrow is still used on line 5, after `i` is gone
  --> e0406_short_life.sisu:5:28
  |
5 |         steps.push(() -> i)
  |                            ^
  |                    ------- borrowed here by `steps`
  |         ----- the borrow is still used here
  = note: declare `i` where it lives as long as the borrow is used, outside the block or loop it is in now

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Declare the borrowed value where it lives long enough, as the note says,
or move a copy into what outlives it: an `own` closure, or `.copy()`.

<!-- file: examples/reference-diagnostics/e0406_short_life_fixed.sisu -->
```sisu
# An `own` closure takes its own copy of `i`.
def main():
    var steps: list[() -> int] = []
    for i in range(3):
        steps.push(own () -> i)
    print(len(steps))
```

<!-- output: examples/reference-diagnostics/e0406_short_life_fixed.out -->
```text
3
```

### E0407

`borrow.return-local`: this returns a borrow of `{name}`, which belongs
to `{function}` and is gone when it returns.

A function's locals are gone when it returns, so it cannot return a
borrow of one; `str` is a borrow, and an interpolated literal is a local
`String`.

<!-- file: examples/reference-diagnostics/e0407_return_local.sisu -->
```sisu
# A function that returns a borrow of its own local.
def greeting(name: str) -> str:
    let text = "Hei, {name}!"
    return text

def main():
    print(greeting("Aino"))
```

<!-- output: examples/reference-diagnostics/e0407_return_local.console -->
```console
$ sisu run e0407_return_local.sisu
error[E0407]: this returns a borrow of `text`, which belongs to `greeting` and is gone when it returns
  --> e0407_return_local.sisu:4:5
  |
4 |     return text
  |     ^^^^^^^^^^^
  |            ---- borrowed here
  = note: a `str` views text, and a copy of it is the same view: return a `String`, which owns its text (`-> String`, or `list[String]` for views in a list)

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Return an owned value: here, make the result type `String`.

<!-- file: examples/reference-diagnostics/e0407_return_local_fixed.sisu -->
```sisu
# The function returns the owned text.
def greeting(name: str) -> String:
    let text = "Hei, {name}!"
    return text

def main():
    print(greeting("Aino"))
```

<!-- output: examples/reference-diagnostics/e0407_return_local_fixed.out -->
```text
Hei, Aino!
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0408

`borrow.return-not-from`: this returns a borrow of the parameter
`{param}`, which is not in the `from` clause of `{function}`.

A `from` clause names the parameters that a returned borrow may borrow
from, and callers rely on it: they keep only those alive while they use
the result.

<!-- file: examples/reference-diagnostics/e0408_not_from.sisu -->
```sisu
# A returned borrow of a parameter that the `from` clause leaves out.
def longer(a: str, b: str) -> str from a:
    if len(b) > len(a):
        return b
    return a

def main():
    print(longer("sauna", "lauteet"))
```

<!-- output: examples/reference-diagnostics/e0408_not_from.console -->
```console
$ sisu run e0408_not_from.sisu
error[E0408]: this returns a borrow of the parameter `b`, which is not in the `from` clause of `longer`
  --> e0408_not_from.sisu:4:9
  |
4 |         return b
  |         ^^^^^^^^
  |
2 | def longer(a: str, b: str) -> str from a:
  |                    ------ `b` is declared here
  = fix: name `b` in the `from` clause

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

The fix names the parameter in the clause, which tells callers that the
result may borrow from either argument.

<!-- file: examples/reference-diagnostics/e0408_not_from_fixed.sisu -->
```sisu
# The `from` clause names both parameters the result may borrow.
def longer(a: str, b: str) -> str from a, b:
    if len(b) > len(a):
        return b
    return a

def main():
    print(longer("sauna", "lauteet"))
```

<!-- output: examples/reference-diagnostics/e0408_not_from_fixed.out -->
```text
lauteet
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

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
  = fix: own the text instead: `String`
  = fix: declare it a view: `view `

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0412

`borrow.view-in-enum`: `{ty}` cannot hold `{fty}`, which borrows; an
enum has no view form, so store an owned value such as `String`.

A struct that holds a view can be a `view struct`, but an enum or an
error set has no view form, so each of its fields owns its value.

<!-- file: examples/reference-diagnostics/e0412_view_in_enum.sisu -->
```sisu
# An enum with a field that borrows.
enum Reading:
    Missing(name: str)
    Celsius(value: int)

def main():
    print("{Reading.Missing("kiuas"):?}")
```

<!-- output: examples/reference-diagnostics/e0412_view_in_enum.console -->
```console
$ sisu run e0412_view_in_enum.sisu
error[E0412]: `Reading` cannot hold `str`, which borrows; an enum has no view form, so store an owned value such as `String`
  --> e0412_view_in_enum.sisu:3:19
  |
3 |     Missing(name: str)
  |                   ^^^
  |
2 | enum Reading:
  |      ------- `Reading` is declared here
  = note: declare the field with an owned type, such as `String` for `str`

error[D0006]: the program was not run: errors found (1)
```

Give the field an owned type, `String` for `str`.

<!-- file: examples/reference-diagnostics/e0412_view_in_enum_fixed.sisu -->
```sisu
# The field owns its text.
enum Reading:
    Missing(name: String)
    Celsius(value: int)

def main():
    print("{Reading.Missing("kiuas".to_string()):?}")
```

<!-- output: examples/reference-diagnostics/e0412_view_in_enum_fixed.out -->
```text
Missing(name="kiuas")
```

Shown in [chapter 12](../tutorial/12-views.md).

### E0413

`borrow.from-required`: `{name}` returns `{ret}`, which borrows, and has
more than one borrowed parameter ({params}); name the ones it borrows
from with `from`.

A function that returns a borrow and has one borrowed parameter returns
a borrow of it. With more than one, `from` names those the result may
borrow from, so a caller knows what must outlive the result.

<!-- file: examples/reference-diagnostics/e0413_from.sisu -->
```sisu
# A returned borrow that could come from either parameter.
def longer(a: str, b: str) -> str:
    return if len(a) >= len(b): a else: b

def main():
    print(longer("sauna", "sisu"))
```

<!-- output: examples/reference-diagnostics/e0413_from.console -->
```console
$ sisu run e0413_from.sisu
error[E0413]: `longer` returns `str`, which borrows, and has more than one borrowed parameter (a, b); name the ones it borrows from with `from`
  --> e0413_from.sisu:2:31
  |
2 | def longer(a: str, b: str) -> str:
  |                               ^^^
  |            ------ `a` is a borrowed parameter
  |                    ------ `b` is a borrowed parameter
  = fix: name `a, b` in the `from` clause: ` from a, b`
  = fix: return owned text instead: `String`

error[D0006]: the program was not run: errors found (1)
```

The fixes come in the order the body asks for. Here a `return` gives a
borrowed parameter, so the first names every borrowed parameter in
`from`; name fewer when the result borrows from fewer. The second makes
the result owned text, a `String`, which is right when the result
borrows from neither parameter, such as text the function reads or
makes, and comes first for such a body; here it would copy.

<!-- file: examples/reference-diagnostics/e0413_from_fixed.sisu -->
```sisu
# `from` names where the borrow comes from.
def longer(a: str, b: str) -> str from a, b:
    return if len(a) >= len(b): a else: b

def main():
    print(longer("sauna", "sisu"))
```

<!-- output: examples/reference-diagnostics/e0413_from_fixed.out -->
```text
sauna
```

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0417

`borrow.must-consume`: `{name}` is never consumed: a value of `{ty}`
must be passed to an `own` parameter before it goes out of scope.

A `@must_consume` type cannot be dropped silently: each value is passed
to an `own` parameter before it goes out of scope (D-054).

<!-- file: examples/reference-diagnostics/e0417_must_consume.sisu -->
```sisu
# A must-consume value that goes out of scope unconsumed.
@must_consume
struct Draft:
    text: String

    def discard(own self):
        pass

def main():
    let note = Draft("a first try")
    print("drafted {note.text}")
```

<!-- output: examples/reference-diagnostics/e0417_must_consume.console -->
```console
$ sisu run e0417_must_consume.sisu
error[E0417]: `note` is never consumed: a value of `Draft` must be passed to an `own` parameter before it goes out of scope
  --> e0417_must_consume.sisu:11:33
   |
11 |     print("drafted {note.text}")
   |                                 ^
   |
10 |     let note = Draft("a first try")
   |         ---- `note` is declared here
  = note: consume it with one of its methods that take `own self`: `discard`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Consume it with one of the methods the note names, each of which says
what ending it means.

<!-- file: examples/reference-diagnostics/e0417_must_consume_fixed.sisu -->
```sisu
# The value consumed by a method that takes `own self`.
@must_consume
struct Draft:
    text: String

    def discard(own self):
        pass

def main():
    let note = Draft("a first try")
    print("drafted {note.text}")
    note.discard()
```

<!-- output: examples/reference-diagnostics/e0417_must_consume_fixed.out -->
```text
drafted a first try
```

Shown in [chapter 13](../tutorial/13-sharing.md).

### E0418

`borrow.must-consume-exit`: this leaves the scope with `{name}`
unconsumed: a value of `{ty}` must be passed to an `own` parameter
first.

A value of a `@must_consume` type ends in an `own` parameter on every
path (D-054), so a `return`, `break` or `?` that leaves it behind is
refused.

<!-- file: examples/reference-diagnostics/e0418_left_unconsumed.sisu -->
```sisu
# An early return that leaves a must-consume value behind.
@must_consume
struct Draft:
    text: String

    def publish(own self) -> String:
        return self.text

def post(text: str) -> String:
    let draft = Draft(text.to_string())
    if text == "":
        return "nothing to post"
    return draft.publish()

def main():
    print(post("Hei Aino"))
```

<!-- output: examples/reference-diagnostics/e0418_left_unconsumed.console -->
```console
$ sisu run e0418_left_unconsumed.sisu
error[E0418]: this leaves the scope with `draft` unconsumed: a value of `Draft` must be passed to an `own` parameter first
  --> e0418_left_unconsumed.sisu:12:9
   |
12 |         return "nothing to post"
   |         ^^^^^^^^^^^^^^^^^^^^^^^^
   |
10 |     let draft = Draft(text.to_string())
   |         ----- `draft` is declared here
  = note: consume it with one of its methods that take `own self`: `publish`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Consume the value before the early exit, or make it only on the path
that consumes it.

<!-- file: examples/reference-diagnostics/e0418_left_unconsumed_fixed.sisu -->
```sisu
# The value is made only on the path that consumes it.
@must_consume
struct Draft:
    text: String

    def publish(own self) -> String:
        return self.text

def post(text: str) -> String:
    if text == "":
        return "nothing to post"
    let draft = Draft(text.to_string())
    return draft.publish()

def main():
    print(post("Hei Aino"))
```

<!-- output: examples/reference-diagnostics/e0418_left_unconsumed_fixed.out -->
```text
Hei Aino
```

Shown in [chapter 13](../tutorial/13-sharing.md).

### E0421

`borrow.view-in-owner`: `{ty}` borrows, so it cannot be put in
`{owner}`, which can outlive what it borrows (04, rule 3).

A view, such as a `str` or a `Lent[Fs]`, borrows what it shows. A
`Shared`, a `Box`, a field of a struct that is not a view, and a
capability that is kept can all outlive the frame that owns what the
view borrows, so none of them may hold one ([R7](ownership.md#views)).

<!-- file: examples/reference-diagnostics/e0421_view_in_owner.sisu -->
```sisu
# Text that borrows, put in a value that has shared owners.
def main():
    let name = "Aino"
    let shared = Shared(name)
    let other = shared.copy()
    print("{Shared.count(other)} owners of the name")
```

<!-- output: examples/reference-diagnostics/e0421_view_in_owner.console -->
```console
$ sisu run e0421_view_in_owner.sisu
error[E0421]: `str` borrows, so it cannot be put in `Shared`, which can outlive what it borrows (04, rule 3)
  --> e0421_view_in_owner.sisu:4:18
  |
4 |     let shared = Shared(name)
  |                  ^^^^^^^^^^^^
  = note: put an owned value in it instead, such as a `String` made with `.to_string()`

error[D0006]: the program was not run: errors found (1)
```

Put an owned value in it, such as a `String` made with `.to_string()`,
as the note says. For a lent capability the note is another,
`borrow.lent-capability`, which says what to do instead: take the
capability itself as a parameter, or do the work inside the call that
lends it ([R10](capabilities.md#lending)).

<!-- file: examples/reference-diagnostics/e0421_view_in_owner_fixed.sisu -->
```sisu
# A `String` owns its text, so a `Shared` can hold it.
def main():
    let name = "Aino".to_string()
    let shared = Shared(name)
    let other = shared.copy()
    print("{Shared.count(other)} owners of the name")
```

<!-- output: examples/reference-diagnostics/e0421_view_in_owner_fixed.out -->
```text
2 owners of the name
```

Shown in [chapter 18](../tutorial/18-capabilities.md).

### E0423

`type.own-closure-capture`: an `own` closure keeps a copy of `{name}`,
and `{ty}` is not `Copy`; stage 0 copies captures (a capture by move
arrives with tasks).

An `own` closure keeps what it captures, so that it can outlive the
frame that made it. Stage 0 copies each capture into the closure, so a
capture must be `Copy`; a capture by move arrives with tasks
([R7](ownership.md#closures-and-their-captures)).

<!-- file: examples/reference-diagnostics/e0423_own_closure_capture.sisu -->
```sisu
# An `own` closure that would keep a `String`.
def make_greeter(own greeting: String) -> (str) -> String:
    return own (name: str) -> "{greeting}, {name}!"

def main():
    let hei = make_greeter("Hei".to_string())
    print(hei("Aino"))
```

<!-- output: examples/reference-diagnostics/e0423_own_closure_capture.console -->
```console
$ sisu run e0423_own_closure_capture.sisu
error[E0423]: an `own` closure keeps a copy of `greeting`, and `String` is not `Copy`; stage 0 copies captures (a capture by move arrives with tasks)
  --> e0423_own_closure_capture.sisu:3:12
  |
3 |     return own (name: str) -> "{greeting}, {name}!"
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: a plain closure borrows `greeting` instead, while it lives; a copy made first is no more `Copy` than `greeting` is

error[D0006]: the program was not run: errors found (1)
```

The note names what works now: a plain closure, which borrows the value
while it lives, so that the work stays where the value lives and the
values are passed in, as the fixed program does; or capture `Copy`
values only. A copy of the `String` made first is not `Copy` either,
as the note says.

<!-- file: examples/reference-diagnostics/e0423_own_closure_capture_fixed.sisu -->
```sisu
# The closure is called where the greeting lives, and borrows it.
def greet_all(greeting: String, names: list[str]) -> list[String]:
    return [("{greeting}, {name}!") for name in names]

def main():
    for line in greet_all("Hei".to_string(), ["Aino", "Eero"]):
        print(line)
```

<!-- output: examples/reference-diagnostics/e0423_own_closure_capture_fixed.out -->
```text
Hei, Aino!
Hei, Eero!
```

Shown in [chapter 16](../tutorial/16-closures-and-iterators.md).

### E0427

`task.not-sync`: `{name}` is shared with the task by reference, but
`{ty}` is not `Sync`.

A task that borrows a value shares it with the code around it, so the
value's type must be `Sync`; a `Cell` changes without a lock and is
not.

<!-- file: examples/reference-diagnostics/e0427_cell_shared.sisu -->
```sisu
# A `Cell` shared with tasks by reference.
def main():
    let visits = Cell(0)
    task.scope do s:
        for _ in range(3):
            s.spawn do:
                visits.set(visits.get() + 1)
    print(visits.get())
```

<!-- output: examples/reference-diagnostics/e0427_cell_shared.console -->
```console
$ sisu run e0427_cell_shared.sisu
error[E0427]: `visits` is shared with the task by reference, but `Cell[int]` is not `Sync`
  --> e0427_cell_shared.sisu:6:13
  |
6 |             s.spawn do:
  |             ^^^^^^^^^^^
  = note: wrap it: `Mutex[int]` shares mutation between tasks; or move the value with an `own` task

error[D0006]: the program was not run: errors found (1)
```

Use a type made for sharing between tasks: an `Atomic` for a number, a
`Mutex` for anything else.

<!-- file: examples/reference-diagnostics/e0427_cell_shared_fixed.sisu -->
```sisu
# An `Atomic` is shared between tasks.
def main():
    let visits = Atomic(0)
    task.scope do s:
        for _ in range(3):
            s.spawn do:
                _ = visits.fetch_add(1)
    print(visits.load())
```

<!-- output: examples/reference-diagnostics/e0427_cell_shared_fixed.out -->
```text
3
```

Shown in [chapter 20](../tutorial/20-concurrency.md).

### E0428

`task.borrows-body-local`: the task borrows `{name}`, which belongs to
the scope's body: the body ends before the scope joins its tasks.

A task may borrow what the frame around its scope owns, which outlives
the scope, but not a value of the scope's own body.

<!-- file: examples/reference-diagnostics/e0428_body_local.sisu -->
```sisu
# A task borrows a value that the scope's body makes.
def longest(words: list[str]) -> int:
    var best = 0
    for w in words:
        best = max(best, len(w))
    return best

def main():
    task.scope do s:
        let words = ["sauna", "löyly", "vihta"]
        let handle = s.spawn(() -> longest(words))
        print(handle.join())
```

<!-- output: examples/reference-diagnostics/e0428_body_local.console -->
```console
$ sisu run e0428_body_local.sisu
error[E0428]: the task borrows `words`, which belongs to the scope's body: the body ends before the scope joins its tasks
  --> e0428_body_local.sisu:11:22
   |
11 |         let handle = s.spawn(() -> longest(words))
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |                              -------------------- borrowed here
  = note: make `words` before `task.scope`, outside the scope's body, so that it outlives every task; `own () -> ...` (or `do own:`) would move it into the first task and leave none for the next

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Make the value outside the scope, as the note says. An `own` task
would move the value into one task, which leaves none for a second;
the note says so too.

<!-- file: examples/reference-diagnostics/e0428_body_local_fixed.sisu -->
```sisu
# The value is made outside the scope, which outlives its tasks.
def longest(words: list[str]) -> int:
    var best = 0
    for w in words:
        best = max(best, len(w))
    return best

def main():
    let words = ["sauna", "löyly", "vihta"]
    task.scope do s:
        let handle = s.spawn(() -> longest(words))
        print(handle.join())
```

<!-- output: examples/reference-diagnostics/e0428_body_local_fixed.out -->
```text
Ok(5)
```

Shown in [chapter 20](../tutorial/20-concurrency.md).

### E0438

`borrow.ref-in-collection`: `{ty}` would hold references, and a
collection holds values it owns or views, never references (04, rule 3).

A collection's items are values it owns, or views such as `str`, and
never references: a reference cannot be stored, so that no collection
can outlive what its items borrow
([04](../../proposal/04-memory-and-ownership.md#the-decision), rule 3;
[R7](ownership.md)). It holds wherever the program has such a type,
written or inferred.

<!-- file: examples/reference-diagnostics/e0438_ref_in_collection.sisu -->
```sisu
# A list that would hold references.
def main():
    let aino = "Aino".to_string()
    var names: list[ref String] = []
    names.push(ref aino)
    print(len(names))
```

<!-- output: examples/reference-diagnostics/e0438_ref_in_collection.console -->
```console
$ sisu run e0438_ref_in_collection.sisu
error[E0438]: `list[ref String]` would hold references, and a collection holds values it owns or views, never references (04, rule 3)
  --> e0438_ref_in_collection.sisu:4:21
  |
4 |     var names: list[ref String] = []
  |                     ^^^^^^^^^^
  = fix: hold the values themselves: `list[String]`

error[D0006]: the program was not run: errors found (1)
```

The fix holds the values themselves, `list[String]`; then push copies,
made with `.copy()`, or move the values in. A list of views, such as
`list[str]`, holds borrows without references. `sorted` of a list whose
items are not `Copy` makes a list of references too, and its note says
to sort the list in place with `sort()`; deriving `Copy` for a small
type is the other way.

<!-- file: examples/reference-diagnostics/e0438_ref_in_collection_fixed.sisu -->
```sisu
# The list holds values of its own.
def main():
    let aino = "Aino".to_string()
    var names: list[String] = []
    names.push(aino.copy())
    print("{names} and {aino}")
```

<!-- output: examples/reference-diagnostics/e0438_ref_in_collection_fixed.out -->
```text
["Aino"] and Aino
```

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0445

`type.hole-in-release`: a release build rejects the typed hole {hole}
of type `{ty}`: blocked is never a shipped state.

A typed hole is a warning, W0001, in `sisu check`, `sisu run` and a
debug build, so that a program can run while it is being written. A
release build, `sisu build --native --release` or `sisu run --native
--release`, refuses each hole instead, so that no hole ships (D-061;
[17](../../proposal/17-testing.md#assertions)). Nothing is built.

<!-- file: examples/reference-diagnostics/e0445_hole_release.sisu -->
```sisu
# A release build of a program that still has a typed hole.
def area(width: f64, height: f64) -> f64:
    return todo("area")

def main():
    print(area(2.0, 3.0))
```

<!-- output: examples/reference-diagnostics/e0445_hole_release.console -->
```console
$ sisu build --native --release e0445_hole_release.sisu
error[E0445]: a release build rejects the typed hole `todo("area")` of type `f64`: blocked is never a shipped state
  --> e0445_hole_release.sisu:3:12
  |
3 |     return todo("area")
  |            ^^^^^^^^^^^^
  = note: fill the hole before the release build; a placeholder meant to ship is `panic("...")`

error[D0132]: the program was not compiled: errors found (1)
```

The note says what to do: fill the hole, or, for a part of the program
that is meant to stop when it is reached, write `panic("...")`, which a
release build accepts and which says that the stop is on purpose.

<!-- file: examples/reference-diagnostics/e0445_hole_release_fixed.sisu -->
```sisu
# The hole filled, so that the release build goes ahead.
def area(width: f64, height: f64) -> f64:
    return width * height

def main():
    print(area(2.0, 3.0))
```

<!-- output: examples/reference-diagnostics/e0445_hole_release_fixed.console -->
```console
$ sisu build --native --release e0445_hole_release_fixed.sisu
built `e0445_hole_release_fixed`
```

Shown in [Plan with typed holes](../guides/holes.md#5-check-that-no-hole-is-left).

### E0446

`type.par-body-impure`: `{callee}` is not `pure`, so the body of a `par`
loop cannot call it: a `par` body runs on a worker and is `pure` (06).

The body of a parallel loop, `for x in par.iter(xs):`, runs for each
element on a worker, in no order the program can see, so it is `pure`
and calls only `pure` functions: what it does cannot depend on the
schedule, and removing the parallelism changes nothing
([06](../../proposal/06-concurrency-and-capabilities.md#data-parallelism)).
`print` is not `pure`.

<!-- file: examples/reference-diagnostics/e0446_par_print.sisu -->
```sisu
# A parallel loop whose body prints.
import std.par

def main():
    let heats = [70, 80, 90]
    for heat in par.iter(heats):
        print("{heat} degrees")
```

<!-- output: examples/reference-diagnostics/e0446_par_print.console -->
```console
$ sisu run e0446_par_print.sisu
error[E0446]: `print` is not `pure`, so the body of a `par` loop cannot call it: a `par` body runs on a worker and is `pure` (06)
  --> e0446_par_print.sisu:7:9
  |
7 |         print("{heat} degrees")
  |         ^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Compute in parallel and print afterwards: `par.map` gives the values in
the order of the list, and an ordinary loop prints them.

<!-- file: examples/reference-diagnostics/e0446_par_print_fixed.sisu -->
```sisu
# The parallel part computes; the loop that prints is an ordinary one.
import std.par

pure def describe(heat: ref int) -> String:
    return "{heat} degrees"

def main():
    let heats = [70, 80, 90]
    for line in par.map(heats, describe):
        print(line)
```

<!-- output: examples/reference-diagnostics/e0446_par_print_fixed.out -->
```text
70 degrees
80 degrees
90 degrees
```

[Chapter 26](../tutorial/26-purity.md#a-parallel-loop) describes it.

### E0447

`type.par-body-changes`: the body of a `par` loop changes nothing but
its own values, and `{name}` is declared before the loop: compute each
value with `par.map` and combine them with `sum` or a reducer (06).

A parallel loop's body may change its own element and its own
bindings, and nothing declared before the loop: two bodies that ran at
once would race on it, and the result would depend on the schedule.
An assignment, a `mut` argument such as a `push`, and a `ref mut` are
changes.

<!-- file: examples/reference-diagnostics/e0447_par_total.sisu -->
```sisu
# A parallel loop whose body adds to a total declared before it.
import std.par

def main():
    let heats = [70, 80, 90]
    var total = 0
    for heat in par.iter(heats):
        total += heat
    print(total)
```

<!-- output: examples/reference-diagnostics/e0447_par_total.console -->
```console
$ sisu run e0447_par_total.sisu
error[E0447]: the body of a `par` loop changes nothing but its own values, and `total` is declared before the loop: compute each value with `par.map` and combine them with `sum` or a reducer (06)
  --> e0447_par_total.sisu:8:9
  |
8 |         total += heat
  |         ^^^^^

error[D0006]: the program was not run: errors found (1)
```

A reducer combines what each chunk computes, in a fixed order, so the
total is the same on any number of workers.

<!-- file: examples/reference-diagnostics/e0447_par_total_fixed.sisu -->
```sisu
# The total made by a reducer, which merges the chunks' sums in a fixed
# order.
import std.par

def main():
    let heats = [70, 80, 90]
    print(par.reduce[par.Sum[int]](heats))
```

<!-- output: examples/reference-diagnostics/e0447_par_total_fixed.out -->
```text
240
```

Shown in [chapter 26](../tutorial/26-purity.md#a-parallel-loop).

### E0448

`resolve.float-attribute`: `@float` takes `fast`, which allows
contraction and reassociation in the function (03); floating point is
strict elsewhere.

`@float(fast)` is the one form of the attribute
([R14](attributes.md#fast-floating-point)). Without it, floating point
is strict, so there is nothing else for the attribute to ask for.

<!-- file: examples/reference-diagnostics/e0448_float_attribute.sisu -->
```sisu
# `@float` with an argument it does not take.
@float(precise)
def blend(a: f64, b: f64) -> f64:
    return a * 0.5 + b * 0.5

def main():
    print(blend(1.0, 2.0))
```

<!-- output: examples/reference-diagnostics/e0448_float_attribute.console -->
```console
$ sisu run e0448_float_attribute.sisu
error[E0448]: `@float` takes `fast`, which allows contraction and reassociation in the function (03); floating point is strict elsewhere
  --> e0448_float_attribute.sisu:2:1
  |
2 | @float(precise)
  | ^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Remove the attribute: strict floating point is the default.

<!-- file: examples/reference-diagnostics/e0448_float_attribute_fixed.sisu -->
```sisu
# Floating point is strict without an attribute.
def blend(a: f64, b: f64) -> f64:
    return a * 0.5 + b * 0.5

def main():
    print(blend(1.0, 2.0))
```

<!-- output: examples/reference-diagnostics/e0448_float_attribute_fixed.out -->
```text
1.5
```

[R14](attributes.md#fast-floating-point) describes it.

### E0449

`type.trait-constant`: each impl of `{trait}` gives its own `{name}`,
so it is read through a type: `T.{name}` for a type `T` that implements
`{trait}`.

A trait's constant has a value for each type that implements the
trait, and none of the trait's own, even when the trait gives a
default ([R3](declarations.md#traits)), so a read through the trait
names no value.

<!-- file: examples/reference-diagnostics/e0449_trait_constant.sisu -->
```sisu
# A trait's constant read through the trait, which has no value of it.
trait Stove:
    const MAX_HEAT: int

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    const MAX_HEAT: int = 110

def main():
    print(Stove.MAX_HEAT)
```

<!-- output: examples/reference-diagnostics/e0449_trait_constant.console -->
```console
$ sisu run e0449_trait_constant.sisu
error[E0449]: each impl of `Stove` gives its own `MAX_HEAT`, so it is read through a type: `T.MAX_HEAT` for a type `T` that implements `Stove`
  --> e0449_trait_constant.sisu:12:11
   |
12 |     print(Stove.MAX_HEAT)
   |           ^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Read the constant through a type: the type itself, `WoodStove.MAX_HEAT`,
or a type parameter bounded by the trait, as `hottest` does.

<!-- file: examples/reference-diagnostics/e0449_trait_constant_fixed.sisu -->
```sisu
# The constant read through a type, whose impl gives its value.
trait Stove:
    const MAX_HEAT: int

struct WoodStove:
    logs: int

impl Stove for WoodStove:
    const MAX_HEAT: int = 110

def hottest[T: Stove](stove: T) -> int:
    return T.MAX_HEAT

def main():
    print("{WoodStove.MAX_HEAT} {hottest(WoodStove(4))}")
```

<!-- output: examples/reference-diagnostics/e0449_trait_constant_fixed.out -->
```text
110 110
```

Shown in [chapter 14](../tutorial/14-traits.md#types-that-each-impl-chooses).

## Compile-time evaluation

Codes E0600 to E0617. The compiler runs `comptime` code, templates and
derives in its interpreter, with a budget of steps and no way to the
world ([R12](compile-time.md)).

### E0601

`comptime.io`: the compile-time evaluation of this expression used
{what}; nothing reaches the world at compile time.

A build gives the same program every time, so compile-time code reads
no file, clock or environment and writes nothing; the evaluation stops
where it tried.

<!-- file: examples/reference-diagnostics/e0601_comptime_io.sisu -->
```sisu
# Compile-time code that prints.
def greet(name: str) -> String:
    print("greeting {name}")
    return "Hei, {name}!"

const GREETING: String = comptime greet("Aino")

def main():
    print(GREETING)
```

<!-- output: examples/reference-diagnostics/e0601_comptime_io.console -->
```console
$ sisu run e0601_comptime_io.sisu
error[E0601]: the compile-time evaluation of this expression used the standard output; nothing reaches the world at compile time
  --> e0601_comptime_io.sisu:6:26
  |
6 | const GREETING: String = comptime greet("Aino")
  |                          ^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Keep the compile-time part to computing, and print when the program
runs.

<!-- file: examples/reference-diagnostics/e0601_comptime_io_fixed.sisu -->
```sisu
# The compile-time code only computes; the program prints.
def greeting(name: str) -> String:
    return "Hei, {name}!"

const GREETING: String = comptime greeting("Aino")

def main():
    print(GREETING)
```

<!-- output: examples/reference-diagnostics/e0601_comptime_io_fixed.out -->
```text
Hei, Aino!
```

Shown in [chapter 25](../tutorial/25-compile-time.md).

### E0602

`comptime.panicked`: the compile-time evaluation of this expression
panicked: {message}.

Code that runs while compiling, a `comptime` expression, a `comptime
assert`, a constant's value, can panic as code at run time can; the
panic is then an error of the program, with its message, and the program
does not compile ([R12](compile-time.md)).

<!-- file: examples/reference-diagnostics/e0602_comptime_panicked.sisu -->
```sisu
# A check made while compiling, which fails.
const SLOTS: int = 12

def slot(n: int) -> int:
    comptime assert SLOTS & (SLOTS - 1) == 0, "{SLOTS} is not a power of two"
    return n & (SLOTS - 1)

def main():
    print(slot(21))
```

<!-- output: examples/reference-diagnostics/e0602_comptime_panicked.console -->
```console
$ sisu run e0602_comptime_panicked.sisu
error[E0602]: the compile-time evaluation of this expression panicked: assertion failed: SLOTS & (SLOTS - 1) == 0
  message: 12 is not a power of two
    SLOTS & (SLOTS - 1) == 0
          │        │
          8        11
  --> e0602_comptime_panicked.sisu:5:5
  |
5 |     comptime assert SLOTS & (SLOTS - 1) == 0, "{SLOTS} is not a power of two"
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Read the panic's message: here the check holds for a power of two, so
the constant changes. A typed hole reached while compiling is E0602 too,
since it panics.

<!-- file: examples/reference-diagnostics/e0602_comptime_panicked_fixed.sisu -->
```sisu
# The constant that the check needs.
const SLOTS: int = 16

def slot(n: int) -> int:
    comptime assert SLOTS & (SLOTS - 1) == 0, "{SLOTS} is not a power of two"
    return n & (SLOTS - 1)

def main():
    print(slot(21))
```

<!-- output: examples/reference-diagnostics/e0602_comptime_panicked_fixed.out -->
```text
5
```

Shown in [chapter 25](../tutorial/25-compile-time.md).

### E0603

`comptime.budget`: the compile-time evaluation in `{item}` used up its
budget of {steps} steps; raise it with `@comptime_budget(steps=N)` on
`{item}`.

Compile-time evaluation has a budget of steps, a million by default, so
that a loop that never ends cannot hang the build (D-063). An item that
needs more says so with `@comptime_budget(steps=N)`
([R12](compile-time.md#budgets)).

<!-- file: examples/reference-diagnostics/e0603_comptime_budget.sisu -->
```sisu
# A constant computed while compiling, slowly.
def climbs(n: int) -> int:
    if n < 2:
        return 1
    return climbs(n - 1) + climbs(n - 2)

const STAIRS: int = comptime climbs(26)

def main():
    print("{STAIRS} ways up 26 steps")
```

<!-- output: examples/reference-diagnostics/e0603_comptime_budget.console -->
```console
$ sisu run e0603_comptime_budget.sisu
error[E0603]: the compile-time evaluation in `STAIRS` used up its budget of 1000000 steps; raise it with `@comptime_budget(steps=N)` on `STAIRS`
  --> e0603_comptime_budget.sisu:7:21
  |
7 | const STAIRS: int = comptime climbs(26)
  |                     ^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Raise the budget on the item, as the message says, when the work is
worth it; here a loop takes 26 steps where the recursion took hundreds
of thousands, and needs no budget of its own.

<!-- file: examples/reference-diagnostics/e0603_comptime_budget_fixed.sisu -->
```sisu
# A faster way to compute the same constant.
def climbs(n: int) -> int:
    var a = 1
    var b = 1
    for _ in range(n - 1):
        a, b = b, a + b
    return b

const STAIRS: int = comptime climbs(26)

def main():
    print("{STAIRS} ways up 26 steps")
```

<!-- output: examples/reference-diagnostics/e0603_comptime_budget_fixed.out -->
```text
196418 ways up 26 steps
```

Shown in [chapter 25](../tutorial/25-compile-time.md).

### E0604

`comptime.not-kept`: {what} made at compile time cannot be kept for run
time; numbers, text, booleans, characters, and lists, sets, dicts,
tuples, options, results, structs and enums of them can.

The program keeps a compile-time value as a literal, so the value needs
a literal form. A function is kept as the function it names, and a
closure as that closure when it captures nothing or only numbers, text,
booleans and characters; a closure that captures more, a reference or
a handle has no such form. The message names what the value is, such
as "a closure that captures a tuple", and the note
`comptime.kept-functions` says what a result keeps of functions.

<!-- file: examples/reference-diagnostics/e0604_not_kept.sisu -->
```sisu
# A closure that captures a tuple, made at compile time.
def shift(by: (int, int)) -> (int) -> int:
    return own x -> x + by.0 - by.1

const SHIFT: (int) -> int = comptime shift((5, 3))

def main():
    print(SHIFT(5))
```

<!-- output: examples/reference-diagnostics/e0604_not_kept.console -->
```console
$ sisu run e0604_not_kept.sisu
error[E0604]: a closure that captures a tuple made at compile time cannot be kept for run time; numbers, text, booleans, characters, and lists, sets, dicts, tuples, options, results, structs and enums of them can
  --> e0604_not_kept.sisu:5:29
  |
5 | const SHIFT: (int) -> int = comptime shift((5, 3))
  |                             ^^^^^^^^^^^^^^^^^^^^^^
  = note: a function is kept as the function it names, and a closure as that closure when it captures nothing or only numbers, text, booleans and characters; a closure that captures a binding would reach the compile-time code's bindings, which run time does not have

error[D0006]: the program was not run: errors found (1)
```

Compute the data at compile time, and make the closure when the
program runs.

<!-- file: examples/reference-diagnostics/e0604_not_kept_fixed.sisu -->
```sisu
# The number is computed while compiling, the closure while running.
def shift(by: (int, int)) -> (int) -> int:
    return own x -> x + by.0 - by.1

const BY: (int, int) = comptime (5, 3)

def main():
    let shift_two = shift(BY)
    print(shift_two(5))
```

<!-- output: examples/reference-diagnostics/e0604_not_kept_fixed.out -->
```text
7
```

Shown in [R12](compile-time.md#what-compile-time-code-may-do).

### E0605

`comptime.only`: `{name}` is a `comptime def`: it can be called only at
compile time, inside `comptime`.

A `comptime def` runs only while compiling: it may take types as
arguments and build code, which a running program cannot do. Call it
inside `comptime`, in a constant's value or in a `comptime assert`
([R12](compile-time.md)).

<!-- file: examples/reference-diagnostics/e0605_comptime_only.sisu -->
```sisu
# A compile-time function called while the program runs.
comptime def power_of_two(n: int) -> bool:
    return n > 0 and n & (n - 1) == 0

def main():
    print(power_of_two(16))
```

<!-- output: examples/reference-diagnostics/e0605_comptime_only.console -->
```console
$ sisu run e0605_comptime_only.sisu
error[E0605]: `power_of_two` is a `comptime def`: it can be called only at compile time, inside `comptime`
  --> e0605_comptime_only.sisu:6:11
  |
6 |     print(power_of_two(16))
  |           ^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write `comptime` before the call, as the fixed program does, or make the
function a plain `def` if the program needs it while it runs.

<!-- file: examples/reference-diagnostics/e0605_comptime_only_fixed.sisu -->
```sisu
# The call inside `comptime`, so it runs while compiling.
comptime def power_of_two(n: int) -> bool:
    return n > 0 and n & (n - 1) == 0

def main():
    print(comptime power_of_two(16))
```

<!-- output: examples/reference-diagnostics/e0605_comptime_only_fixed.out -->
```text
True
```

Shown in [chapter 25](../tutorial/25-compile-time.md).

### E0606

`comptime.budget-attribute`: `@comptime_budget` takes `steps=N`, a
positive number of steps.

The budget is named, so that the attribute says what its number counts.

<!-- file: examples/reference-diagnostics/e0606_budget_attribute.sisu -->
```sisu
# A budget without `steps=`.
def triangle(n: int) -> int:
    var total = 0
    for i in range(n + 1):
        total += i
    return total

@comptime_budget(2_000_000)
const BIG: int = comptime triangle(100)

def main():
    print(BIG)
```

<!-- output: examples/reference-diagnostics/e0606_budget_attribute.console -->
```console
$ sisu run e0606_budget_attribute.sisu
error[E0606]: `@comptime_budget` takes `steps=N`, a positive number of steps
  --> e0606_budget_attribute.sisu:8:1
  |
8 | @comptime_budget(2_000_000)
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<!-- file: examples/reference-diagnostics/e0606_budget_attribute_fixed.sisu -->
```sisu
# A budget named by `steps=`.
def triangle(n: int) -> int:
    var total = 0
    for i in range(n + 1):
        total += i
    return total

@comptime_budget(steps=2_000_000)
const BIG: int = comptime triangle(100)

def main():
    print(BIG)
```

<!-- output: examples/reference-diagnostics/e0606_budget_attribute_fixed.out -->
```text
5050
```

Shown in [R14](attributes.md#compile-time-budgets).

### E0607

`comptime.derive-template`: a trait's derive is `comptime def
derive(T: type) -> impl {trait} for T`, whose body is `return impl
{trait} for T:` with the impl's methods.

A derive writes one declaration, the impl, as text for each type, so its
body is the `return impl` after checks of the type, if any: `comptime
assert`, `comptime panic`, and `comptime if` whose branches hold only
checks. Any other statement there is the error, and when the body has a
`return impl`, the note `comptime.derive-checks` says what may come
before it ([R12](compile-time.md#the-derive-protocol)). It is the one
error of the mistake: a `@derive` of the trait is not [E0243](#e0243),
and a use of what the derive would have written, `Booking.width()`
here, is not [E0351](#e0351) or [E0304](#e0304):

<!-- file: examples/reference-diagnostics/e0607_derive_template.sisu -->
```sisu
# A derive that computes a value before its `return impl`, and its use.
trait Columns:
    ## The number of the type's fields.
    def width() -> int

    comptime def derive(T: type) -> impl Columns for T:
        let count = len(reflect.fields_of(T))
        return impl Columns for T:
            def width() -> int:
                return count

@derive(Columns)
struct Booking:
    name: String
    hour: int

def main():
    print(Booking.width())
```

<!-- output: examples/reference-diagnostics/e0607_derive_template.console -->
```console
$ sisu check e0607_derive_template.sisu
error[E0607]: a trait's derive is `comptime def derive(T: type) -> impl Columns for T`, whose body is `return impl Columns for T:` with the impl's methods
  --> e0607_derive_template.sisu:6:18
  |
6 |     comptime def derive(T: type) -> impl Columns for T:
  |                  ^^^^^^
  = note: only checks of the type may come before the `return impl`: `comptime assert`, and `comptime panic` and `comptime if` of them, which write no code

```

A `let` before the `return impl` would be a variable of every method
the derive writes, which the engine has only for the variable of a
`comptime for`. Compute the value inside the method that needs it, at
compile time, and keep only checks before the `return impl`:

<!-- file: examples/reference-diagnostics/e0607_derive_template_fixed.sisu -->
```sisu
# The method counts the fields itself, at compile time.
trait Columns:
    ## The number of the type's fields.
    def width() -> int

    comptime def derive(T: type) -> impl Columns for T:
        comptime assert reflect.is_struct(T), "columns are a struct's"
        return impl Columns for T:
            def width() -> int:
                return comptime len(reflect.fields_of(T))

@derive(Columns)
struct Booking:
    name: String
    hour: int

def main():
    print(Booking.width())
```

<!-- output: examples/reference-diagnostics/e0607_derive_template_fixed.out -->
```text
2
```

Shown in [R12](compile-time.md#the-derive-protocol).

### E0608

`comptime.derive-failed`: deriving `{trait}` for `{ty}` stopped at
compile time: {reason}.

A derive is a trait's `comptime def derive`, which writes the impl while
compiling. When it stops, with `comptime panic` or a failed `comptime
assert`, the `@derive` that asked for it is the error, with the derive's
reason, and the type has none of the trait's functions, so each use of
one is E0351 ([R12](compile-time.md)).

<!-- file: examples/reference-diagnostics/e0608_derive_failed.sisu -->
```sisu
# A derive that refuses a type it cannot handle.
trait Columns:
    def columns() -> String

    comptime def derive(T: type) -> impl Columns for T:
        comptime if not reflect.is_struct(T):
            comptime panic("`{reflect.name_of(T)}` is not a struct")
        return impl Columns for T:
            def columns() -> String:
                var names: list[String] = []
                comptime for f in reflect.fields_of(T):
                    names.push(f.name.to_string())
                return names.join(",")

@derive(Columns)
enum Heater:
    Wood
    Electric

def main():
    print(Heater.columns())
```

<!-- output: examples/reference-diagnostics/e0608_derive_failed.console -->
```console
$ sisu run e0608_derive_failed.sisu
error[E0608]: deriving `Columns` for `Heater` stopped at compile time: `Heater` is not a struct
  --> e0608_derive_failed.sisu:15:9
   |
15 | @derive(Columns)
   |         ^^^^^^^
   |
 7 |             comptime panic("`{reflect.name_of(T)}` is not a struct")
   |                      ----------------------------------------------- the derive's template, where it stopped

error[E0351]: `Heater` has no associated function `columns`
  --> e0608_derive_failed.sisu:21:18
   |
21 |     print(Heater.columns())
   |                  ^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

Read the reason, which the derive's author wrote: here the derive lists
a struct's fields, and an enum has none. Derive it for a type it
handles, or write the impl by hand. E0608 is the derive stopping; an
error in the impl it went on to write is that error's own code, placed
at the same `@derive` with a note that quotes the impl's line, as
[E0347](#e0347) is. `std.json`'s `@derive(Decode)` stops so for an
enum, since `decode` cannot make an enum's variants yet, and for a
struct with a field that holds an enum, directly or as a list's or an
option's element, its reason naming the field, when the enum has no
`impl Decode` of its own; with one, the struct's derived decoder calls
it ([G7](../guides/json.md#5-give-a-type-a-json-form-of-its-own)).

<!-- file: examples/reference-diagnostics/e0608_derive_failed_fixed.sisu -->
```sisu
# The derive asked for a struct, whose fields it lists.
trait Columns:
    def columns() -> String

    comptime def derive(T: type) -> impl Columns for T:
        comptime if not reflect.is_struct(T):
            comptime panic("`{reflect.name_of(T)}` is not a struct")
        return impl Columns for T:
            def columns() -> String:
                var names: list[String] = []
                comptime for f in reflect.fields_of(T):
                    names.push(f.name.to_string())
                return names.join(",")

@derive(Columns)
struct Stove:
    kind: String
    celsius: int

def main():
    print(Stove.columns())
```

<!-- output: examples/reference-diagnostics/e0608_derive_failed_fixed.out -->
```text
kind,celsius
```

Shown in the guides [Write a derive](../guides/derive.md) and [Read
and write JSON](../guides/json.md).

### E0609

`comptime.instance-failed`: making `{name}` stopped at compile time:
{reason}.

A template is made for each list of types it is called with; one that
stops, by `comptime panic` or a query that does not apply, is reported
at the call that needed it, with a label where the template stopped.
When it stopped at a plain `for` over a list that only compile time
has, such as `reflect.fields_of(T)`, the fix writes `comptime` before
the `for`, which unrolls it ([chapter 25](../tutorial/25-compile-time.md)).
A reflection query given what compile time knows no type by, inside a
template, is this error too; in a function without type parameters it
is [E0616](#e0616).

<!-- file: examples/reference-diagnostics/e0609_instance_failed.sisu -->
```sisu
# A template that a type cannot be made into.
def columns[T](value: T) -> int:
    if comptime reflect.is_struct(T):
        return len(reflect.fields_of(T))
    else:
        comptime panic("`{reflect.name_of(T)}` has no columns")

def main():
    print(columns(42))
```

<!-- output: examples/reference-diagnostics/e0609_instance_failed.console -->
```console
$ sisu run e0609_instance_failed.sisu
error[E0609]: making `columns[int]` stopped at compile time: `int` has no columns
  --> e0609_instance_failed.sisu:9:11
  |
9 |     print(columns(42))
  |           ^^^^^^^^^^^
  |
6 |         comptime panic("`{reflect.name_of(T)}` has no columns")
  |                  ---------------------------------------------- the template, where it stopped

error[D0006]: the program was not run: errors found (1)
```

Call the template with a type it takes, or give it a branch for the
other types.

<!-- file: examples/reference-diagnostics/e0609_instance_failed_fixed.sisu -->
```sisu
# The template is called with a struct.
struct Booking:
    name: String
    hour: int

def columns[T](value: T) -> int:
    if comptime reflect.is_struct(T):
        return len(reflect.fields_of(T))
    else:
        comptime panic("`{reflect.name_of(T)}` has no columns")

def main():
    print(columns(Booking("Aino", 18)))
```

<!-- output: examples/reference-diagnostics/e0609_instance_failed_fixed.out -->
```text
2
```

Shown in [chapter 25](../tutorial/25-compile-time.md).

### E0612

`comptime.embed-failed`: cannot embed `{path}`: {reason}.

`embed("PATH")` makes a file's text part of the program while it
compiles, so the file must be there and readable, and its path must stay
inside the program's directory: the package's root in a package, where
the path starts, or a single file's directory
([R12](compile-time.md#embedding-a-file)). The reason after the colon
says which.

<!-- file: examples/reference-diagnostics/e0612_embed_failed.sisu -->
```sisu
# A file embedded while compiling, which is not there.
const MENU: str = embed("menu.txt")

def main():
    for line in MENU.lines():
        print(line)
```

<!-- output: examples/reference-diagnostics/e0612_embed_failed.console -->
```console
$ sisu run e0612_embed_failed.sisu
error[E0612]: cannot embed `menu.txt`: there is no such file
  --> e0612_embed_failed.sisu:2:19
  |
2 | const MENU: str = embed("menu.txt")
  |                   ^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Put the file where the path says, relative to the module's directory;
the fixed program has `menu.txt` beside it.

<!-- file: examples/reference-diagnostics/e0612_embed_failed_fixed.sisu -->
```sisu
# The file is beside the program, so the build embeds it.
const MENU: str = embed("menu.txt")

def main():
    for line in MENU.lines():
        print(line)
```

<!-- file: examples/reference-diagnostics/e0612_embed_failed_fixed.files/menu.txt -->
```text
kahvi 3
pulla 2
```

<!-- output: examples/reference-diagnostics/e0612_embed_failed_fixed.out -->
```text
kahvi 3
pulla 2
```

Shown in the guide [Embed a file at compile time](../guides/embed.md).

### E0613

`comptime.embed-path`: `embed` takes the path of a file as a string
literal.

The path of `embed` is a string literal, so that the files a build reads
are in its source for every tool to see, and a build records them
([R12](compile-time.md#embedding-a-file)).

<!-- file: examples/reference-diagnostics/e0613_embed_path.sisu -->
```sisu
# `embed` given a constant's name, not a literal.
const NAME: str = "menu.txt"
const MENU: str = embed(NAME)

def main():
    for line in MENU.lines():
        print(line)
```

<!-- output: examples/reference-diagnostics/e0613_embed_path.console -->
```console
$ sisu run e0613_embed_path.sisu
error[E0613]: `embed` takes the path of a file as a string literal
  --> e0613_embed_path.sisu:3:19
  |
3 | const MENU: str = embed(NAME)
  |                   ^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write the path as a literal in the call. A program that chooses among
files embeds each of them and chooses among the constants.

<!-- file: examples/reference-diagnostics/e0613_embed_path_fixed.sisu -->
```sisu
# The path written as a literal, which the build can read.
const MENU: str = embed("menu.txt")

def main():
    for line in MENU.lines():
        print(line)
```

<!-- file: examples/reference-diagnostics/e0613_embed_path_fixed.files/menu.txt -->
```text
kahvi 3
pulla 2
```

<!-- output: examples/reference-diagnostics/e0613_embed_path_fixed.out -->
```text
kahvi 3
pulla 2
```

Shown in the guide [Embed a file at compile time](../guides/embed.md).

### E0616

`comptime.function-failed`: the compile-time code of `{name}` stopped:
{reason}.

A function without type parameters, `main` or a test among them, is
made once, and its compile-time code runs when it is made. When that
code stops, as a reflection query given a run-time value or a name of
nothing does, the error stands at the function, with a reason that says
what stopped it and the label `comptime.function-here` ("the
compile-time code, where it stopped") at the place. In a template,
made for each list of types, the same stop is E0609
([R12](compile-time.md#reflection)). A type of the prelude, `int` or
`str`, is a type a query takes.

<!-- file: examples/reference-diagnostics/e0616_function_failed.sisu -->
```sisu
# A reflection query in `main` given a value, where it takes a type.
def main():
    let heat = 80
    print(comptime reflect.name_of(heat))
```

<!-- output: examples/reference-diagnostics/e0616_function_failed.console -->
```console
$ sisu run e0616_function_failed.sisu
error[E0616]: the compile-time code of `main` stopped: `reflect.name_of` takes a type (a type parameter, or a type or trait in scope), and compile time knows no type `heat`
  --> e0616_function_failed.sisu:2:5
  |
2 | def main():
  |     ^^^^
  |
4 |     print(comptime reflect.name_of(heat))
  |                                    ---- the compile-time code, where it stopped

error[D0006]: the program was not run: errors found (1)
```

Give the query the type, or a type parameter of a generic function that
takes the value.

<!-- file: examples/reference-diagnostics/e0616_function_failed_fixed.sisu -->
```sisu
# The query given the value's type, which compile time knows.
def main():
    let heat = 80
    print("{heat} is an {comptime reflect.name_of(int)}")
```

<!-- output: examples/reference-diagnostics/e0616_function_failed_fixed.out -->
```text
80 is an int
```

Shown in [R12](compile-time.md#reflection).

### E0617

`eval.no-form`: `{name}` holds {what}, which {form} cannot write.

`sisu eval` writes a module's constants as JSON, TOML or YAML
([R12](compile-time.md#configuration-files)), and `{name}` holds a
value that the form has no way to write: a function or a reference in
any form, an infinite or not-a-number float in JSON, or a `None` inside
a list in TOML. `{name}` is the constant, or the path from it of the
field or item that holds the value, `LIMITS.timeout` or `MAYBE[0]`
([Generate configuration](../guides/config.md#5-keep-to-what-the-form-can-write)
shows one). The error stands at the constant's name, nothing is
written, and the status is 1; `sisu check` and `sisu run` do not report
it, since only a form needs the value written.

<!-- file: examples/reference-diagnostics/e0617_no_form.sisu -->
```sisu
# A configuration constant that holds a function, which JSON cannot write.
pure def doubled(seats: int) -> int:
    return seats * 2

const SEATS: int = 20
const GROW: (int) -> int = doubled
```

<!-- output: examples/reference-diagnostics/e0617_no_form.console -->
```console
$ sisu eval e0617_no_form.sisu
error[E0617]: `GROW` holds a function, which JSON cannot write
  --> e0617_no_form.sisu:6:7
  |
6 | const GROW: (int) -> int = doubled
  |       ^^^^

```

Give the constant the value it stands for, here what the function
gives, or choose a form that writes the value, as TOML and YAML write
an infinite float.

<!-- file: examples/reference-diagnostics/e0617_no_form_fixed.sisu -->
```sisu
# The constant holds what the function gives, which every form writes.
pure def doubled(seats: int) -> int:
    return seats * 2

const SEATS: int = 20
const GROWN: int = doubled(SEATS)
```

<!-- output: examples/reference-diagnostics/e0617_no_form_fixed.console -->
```console
$ sisu eval e0617_no_form_fixed.sisu
{
  "SEATS": 20,
  "GROWN": 40
}
```

Shown in [Generate configuration](../guides/config.md).

## Capabilities

Codes E0700 to E0703. The rules of capabilities, of a test's parameters,
and of `[trust]`, which says which dependencies may hold unsafe and
foreign code ([R10](capabilities.md), [R13](modules.md#trust)).

### E0703

`cap.untrusted`: the package `{package}` is not trusted with {what}; a
dependency may contain `unsafe` and `extern` only when `[trust] unsafe`
in the root package's `sisu.toml` lists it.

A dependency may contain `unsafe` and `extern` code only when the root
package's `[trust] unsafe` lists it, since unsafe code can do what no
capability allows (D-032, D-082). Each block of a dependency that the
root does not trust is an error, and so is a package whose `sisu.toml`
does not declare the unsafe code it holds (E1019).

<!-- file: examples/reference-diagnostics/e0703_untrusted.files/app/sisu.toml -->
```toml
[package]
name = "app"
version = "0.1.0"
edition = "2027"

[dependencies]
readings = { path = "../readings" }
```

<!-- file: examples/reference-diagnostics/e0703_untrusted.files/readings/sisu.toml -->
```toml
[package]
name = "readings"
version = "0.1.0"
edition = "2027"
```

<!-- file: examples/reference-diagnostics/e0703_untrusted.files/readings/src/lib.sisu -->
```sisu
##! The last of a list of readings, read through a raw pointer.
from std.mem import as_ptr

## The last reading of `temps`, which holds at least one.
pub def last(temps: Slice[int]) -> int:
    requires len(temps) > 0
    let first = as_ptr(temps)
    # safety: in_bounds, valid, aligned: the precondition gives the last
    #   index, and `temps` lives, unchanged, until the read.
    return unsafe first.offset(len(temps) - 1).read()
```

<!-- output: examples/reference-diagnostics/e0703_untrusted.console -->
```console
$ sisu run app
error[E0703]: the package `readings` is not trusted with `unsafe`; a dependency may contain `unsafe` and `extern` only when `[trust] unsafe` in the root package's `sisu.toml` lists it
  --> ../readings/src/lib.sisu:10:12
   |
10 |     return unsafe first.offset(len(temps) - 1).read()
   |            ^^^^^^

error[E1019]: the package `readings` contains unsafe code, and its `sisu.toml` declares no `unsafe`: add `unsafe = "memory"` to its `[package]`
  --> ../readings/src/lib.sisu:10:12
   |
10 |     return unsafe first.offset(len(temps) - 1).read()
   |            ^^^^^^

error[D0006]: the program was not run: errors found (2)
```

The dependency's author declares its level, `unsafe = "memory"` for
unsafe blocks or `"extern"` for foreign code, and the root package's
author, having read the code, lists it in `[trust] unsafe`
([R13](modules.md#trust)).

<!-- file: examples/reference-diagnostics/e0703_untrusted_fixed.files/readings/sisu.toml -->
```toml
[package]
name = "readings"
version = "0.1.0"
edition = "2027"
unsafe = "memory"
```

<!-- file: examples/reference-diagnostics/e0703_untrusted_fixed.files/app/sisu.toml -->
```toml
[package]
name = "app"
version = "0.1.0"
edition = "2027"

[dependencies]
readings = { path = "../readings" }

[trust]
unsafe = ["readings"]
```

<!-- output: examples/reference-diagnostics/e0703_untrusted_fixed.console -->
```console
$ sisu run app
92
```

Shown in the guide [Call a C library](../guides/c-library.md).

## Contracts

Codes E0800 to E0821. The checker checks where contract clauses stand
and that they only read, and the predicates and changes of refinement
types ([R11](contracts.md)).

### E0800

`contract.misplaced`: `{keyword}` is a function's contract and belongs
at the start of its body, before its first statement.

A function's `requires` and `ensures` are the first lines of its body,
so that they describe the call as a whole.

<!-- file: examples/reference-diagnostics/e0800_misplaced_clause.sisu -->
```sisu
# A precondition after the body's first statement.
def half(n: int) -> int:
    let h = n // 2
    requires n >= 0
    return h

def main():
    print(half(8))
```

<!-- output: examples/reference-diagnostics/e0800_misplaced_clause.console -->
```console
$ sisu run e0800_misplaced_clause.sisu
error[E0800]: `requires` is a function's contract and belongs at the start of its body, before its first statement
  --> e0800_misplaced_clause.sisu:4:5
  |
4 |     requires n >= 0
  |     ^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Move the clause to the top of the body.

<!-- file: examples/reference-diagnostics/e0800_misplaced_clause_fixed.sisu -->
```sisu
# The precondition is the body's first line.
def half(n: int) -> int:
    requires n >= 0
    let h = n // 2
    return h

def main():
    print(half(8))
```

<!-- output: examples/reference-diagnostics/e0800_misplaced_clause_fixed.out -->
```text
4
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### E0801

`contract.not-pure`: a contract only reads what it is given, and
`{callee}` is not `pure`.

A contract only reads: a build can check its clauses or leave them out
(`--contracts`), and the program must mean the same either way. So a
clause calls only `pure` functions ([R11](contracts.md)).

<!-- file: examples/reference-diagnostics/e0801_contract_not_pure.sisu -->
```sisu
# A contract that calls a function that is not `pure`.
def all_different(hours: Slice[int]) -> bool:
    var seen: set[int] = set()
    for h in hours:
        if seen.contains(h):
            return False
        seen.add(h)
    return True

def book(mut hours: list[int], hour: int):
    requires hour not in hours
    ensures all_different(hours)
    hours.push(hour)

def main():
    var hours = [17, 18]
    book(mut hours, 20)
    print(hours)
```

<!-- output: examples/reference-diagnostics/e0801_contract_not_pure.console -->
```console
$ sisu run e0801_contract_not_pure.sisu
error[E0801]: a contract only reads what it is given, and `all_different` is not `pure`
  --> e0801_contract_not_pure.sisu:12:13
   |
12 |     ensures all_different(hours)
   |             ^^^^^^^^^^^^^^^^^^^^
  = fix: declare it `pure`, if it only reads what it is given: `pure `

error[D0006]: the program was not run: errors found (1)
```

Mark the function `pure` when it only computes, as `all_different` does:
its own `var` changes nothing outside it, which is what the fix does.

<!-- file: examples/reference-diagnostics/e0801_contract_not_pure_fixed.sisu -->
```sisu
# The function the contract calls is `pure`.
pure def all_different(hours: Slice[int]) -> bool:
    var seen: set[int] = set()
    for h in hours:
        if seen.contains(h):
            return False
        seen.add(h)
    return True

def book(mut hours: list[int], hour: int):
    requires hour not in hours
    ensures all_different(hours)
    hours.push(hour)

def main():
    var hours = [17, 18]
    book(mut hours, 20)
    print(hours)
```

<!-- output: examples/reference-diagnostics/e0801_contract_not_pure_fixed.out -->
```text
[17, 18, 20]
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### E0803

`contract.old-outside`: `old(...)` is a value as it was on entry, which
only an `ensures` clause can ask for.

`old(e)` is the value of `e` when the call began. Only an `ensures`
clause, which is checked when the call returns, has an earlier moment to
ask about; a `requires` clause is checked on entry, where `e` is the
value already ([R11](contracts.md)).

<!-- file: examples/reference-diagnostics/e0803_old_outside.sisu -->
```sisu
# `old(...)` in a precondition.
struct Stove:
    celsius: int

    def heat(mut self, degrees: int):
        requires old(self.celsius) < 110
        self.celsius = min(self.celsius + degrees, 110)

def main():
    var stove = Stove(celsius=60)
    stove.heat(30)
    print("{stove.celsius} degrees")
```

<!-- output: examples/reference-diagnostics/e0803_old_outside.console -->
```console
$ sisu run e0803_old_outside.sisu
error[E0803]: `old(...)` is a value as it was on entry, which only an `ensures` clause can ask for
  --> e0803_old_outside.sisu:6:18
  |
6 |         requires old(self.celsius) < 110
  |                  ^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write the expression without `old` in a `requires` clause, and keep
`old` for the postcondition that compares the result with the start.

<!-- file: examples/reference-diagnostics/e0803_old_outside_fixed.sisu -->
```sisu
# A precondition reads the value as it is; `old` is for `ensures`.
struct Stove:
    celsius: int

    def heat(mut self, degrees: int):
        requires self.celsius < 110
        ensures self.celsius == min(old(self.celsius) + degrees, 110)
        self.celsius = min(self.celsius + degrees, 110)

def main():
    var stove = Stove(celsius=60)
    stove.heat(30)
    print("{stove.celsius} degrees")
```

<!-- output: examples/reference-diagnostics/e0803_old_outside_fixed.out -->
```text
90 degrees
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### E0804

`contract.old-arguments`: `old` takes one expression, as in
`old(self.balance)`.

`old(e)` is the value of one expression on entry, which the compiler
copies then.

<!-- file: examples/reference-diagnostics/e0804_old_arguments.sisu -->
```sisu
# `old` given two expressions.
struct Stove:
    celsius: int

    def heat(mut self, degrees: int):
        ensures self.celsius == old(self.celsius, degrees) + degrees
        self.celsius += degrees

def main():
    var stove = Stove(60)
    stove.heat(20)
    print(stove.celsius)
```

<!-- output: examples/reference-diagnostics/e0804_old_arguments.console -->
```console
$ sisu run e0804_old_arguments.sisu
error[E0804]: `old` takes one expression, as in `old(self.balance)`
  --> e0804_old_arguments.sisu:6:33
  |
6 |         ensures self.celsius == old(self.celsius, degrees) + degrees
  |                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Give `old` the one expression whose entry value the clause needs.

<!-- file: examples/reference-diagnostics/e0804_old_arguments_fixed.sisu -->
```sisu
# `old` takes the one expression whose entry value it is.
struct Stove:
    celsius: int

    def heat(mut self, degrees: int):
        ensures self.celsius == old(self.celsius) + degrees
        self.celsius += degrees

def main():
    var stove = Stove(60)
    stove.heat(20)
    print(stove.celsius)
```

<!-- output: examples/reference-diagnostics/e0804_old_arguments_fixed.out -->
```text
80
```

Shown in [R11](contracts.md#postconditions).

### E0805

`contract.generator-ensures`: a generator has no `ensures`: what it
returns is its iterator.

A generator returns at once with an iterator, before its body has run,
so there is no result for a postcondition to check.

<!-- file: examples/reference-diagnostics/e0805_generator_ensures.sisu -->
```sisu
# A generator with a postcondition.
def evens(limit: int) -> Iter[int]:
    requires limit >= 0
    ensures limit >= 0
    for i in range(limit):
        yield i * 2

def main():
    print(evens(4).collect())
```

<!-- output: examples/reference-diagnostics/e0805_generator_ensures.console -->
```console
$ sisu run e0805_generator_ensures.sisu
error[E0805]: a generator has no `ensures`: what it returns is its iterator
  --> e0805_generator_ensures.sisu:4:5
  |
4 |     ensures limit >= 0
  |     ^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Leave the `ensures` out; a `requires` is checked when the generator is
called.

<!-- file: examples/reference-diagnostics/e0805_generator_ensures_fixed.sisu -->
```sisu
# A generator keeps its precondition and has no postcondition.
def evens(limit: int) -> Iter[int]:
    requires limit >= 0
    for i in range(limit):
        yield i * 2

def main():
    print(evens(4).collect())
```

<!-- output: examples/reference-diagnostics/e0805_generator_ensures_fixed.out -->
```text
[0, 2, 4, 6]
```

Shown in [R11](contracts.md#where-clauses-stand).

### E0806

`contract.invariant-field`: `{what}` is part of a value of `{ty}`,
whose invariant only its own methods may change and check again; change
it through one of them.

An invariant holds for every value of its type that the program can see:
construction checks it, and so does each return of a `mut self` method.
A field changed from outside the type's methods would skip the check, so
only those methods assign the fields of a type with an invariant
([R11](contracts.md#invariants)).

<!-- file: examples/reference-diagnostics/e0806_invariant_field.sisu -->
```sisu
# A field of a type with an invariant, changed from outside.
struct Schedule:
    opens: int
    closes: int
    invariant 0 <= self.opens < self.closes <= 24

def main():
    var today = Schedule(opens=16, closes=21)
    today.closes = 25
    print("open {today.opens} to {today.closes}")
```

<!-- output: examples/reference-diagnostics/e0806_invariant_field.console -->
```console
$ sisu run e0806_invariant_field.sisu
error[E0806]: `today.closes` is part of a value of `Schedule`, whose invariant only its own methods may change and check again; change it through one of them
  --> e0806_invariant_field.sisu:9:5
  |
9 |     today.closes = 25
  |     ^^^^^^^^^^^^
  = note: give `Schedule` a method that makes the change, `def NAME(mut self, ...)`, which checks the invariant when it returns, and call it

error[D0006]: the program was not run: errors found (1)
```

Give the type a method that makes the change, as `close_at` does; the
invariant is checked when it returns, so `close_at(25)` panics there.
The diagnostic offers no fix line, since the method is the author's to
write; its note says so (`contract.invariant-method`), or, for a type
that has methods taking `mut self`, lists them
(`contract.invariant-methods`).

<!-- file: examples/reference-diagnostics/e0806_invariant_field_fixed.sisu -->
```sisu
# The type's own method changes the field, and the invariant is checked.
struct Schedule:
    opens: int
    closes: int
    invariant 0 <= self.opens < self.closes <= 24

    def close_at(mut self, hour: int):
        self.closes = hour

def main():
    var today = Schedule(opens=16, closes=21)
    today.close_at(23)
    print("open {today.opens} to {today.closes}")
```

<!-- output: examples/reference-diagnostics/e0806_invariant_field_fixed.out -->
```text
open 16 to 23
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### E0807

`contract.not-in-test`: a test has no contract; check what it needs
with `assert`.

A test is called by `sisu test`, not by code whose arguments a contract
could judge. `sisu run` leaves tests out, so `sisu check`, which builds
the program with its tests, is where this is reported.

<!-- file: examples/reference-diagnostics/e0807_test_contract.sisu -->
```sisu
# A test with a precondition.
def half(n: int) -> int:
    return n // 2

test "halves":
    requires half(4) > 0
    assert half(4) == 2
```

<!-- output: examples/reference-diagnostics/e0807_test_contract.console -->
```console
$ sisu check e0807_test_contract.sisu
error[E0807]: a test has no contract; check what it needs with `assert`
  --> e0807_test_contract.sisu:6:5
  |
6 |     requires half(4) > 0
  |     ^^^^^^^^^^^^^^^^^^^^

```

Write the check as an `assert`.

<!-- file: examples/reference-diagnostics/e0807_test_contract_fixed.sisu -->
```sisu
# A test says what it needs with `assert`.
def half(n: int) -> int:
    return n // 2

test "halves":
    assert half(4) > 0
    assert half(4) == 2
```

<!-- output: examples/reference-diagnostics/e0807_test_contract_fixed.test -->
```text
test "halves" ... ok
1 passed, 0 failed, 0 blocked
```

Shown in [R11](contracts.md#where-clauses-stand).

### E0810

`refine.not-satisfied`: `{value}` is not of type `{ty}`: `{predicate}`
is false for it.

A value whose type is a refinement satisfies its predicate. The compiler
checks a constant while compiling, and a value it cannot know passes
through `try_from` (E0811) ([R11](contracts.md#refinement-types)).

<!-- file: examples/reference-diagnostics/e0810_refinement_constant.sisu -->
```sisu
# A constant that the refinement type does not admit.
type Percent = int where 0 <= self <= 100

def main():
    let after_ladle: Percent = 135
    print("{after_ladle} percent")
```

<!-- output: examples/reference-diagnostics/e0810_refinement_constant.console -->
```console
$ sisu run e0810_refinement_constant.sisu
error[E0810]: `135` is not of type `Percent`: `0 <= self <= 100` is false for it
  --> e0810_refinement_constant.sisu:5:32
  |
5 |     let after_ladle: Percent = 135
  |                                ^^^

error[D0006]: the program was not run: errors found (1)
```

Use a value that satisfies the predicate the message names, or a wider
type where the value can be outside it.

<!-- file: examples/reference-diagnostics/e0810_refinement_constant_fixed.sisu -->
```sisu
# A constant within the refinement.
type Percent = int where 0 <= self <= 100

def main():
    let after_ladle: Percent = 85
    print("{after_ladle} percent")
```

<!-- output: examples/reference-diagnostics/e0810_refinement_constant_fixed.out -->
```text
85 percent
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### E0811

`refine.needs-try-from`: expected `{expected}`, found `{found}`; a value
becomes a `{expected}` where its predicate is checked, with
`{name}.try_from(...)`.

A refinement is a subtype of its base, so a `Percent` is an `int`
wherever one is expected, but an `int` becomes a `Percent` only where
its predicate is checked: in `try_from`, which returns an error for a
value outside it ([R11](contracts.md#refinement-types)).

<!-- file: examples/reference-diagnostics/e0811_needs_try_from.sisu -->
```sisu
# A value that the compiler cannot check, passed as a refinement.
type Percent = int where 0 <= self <= 100

def steam(humidity: Percent) -> str:
    return if humidity < 50: "soft" else: "heavy"

def main():
    let n = "70".parse_int() ?? 0
    print(steam(n))
```

<!-- output: examples/reference-diagnostics/e0811_needs_try_from.console -->
```console
$ sisu run e0811_needs_try_from.sisu
error[E0811]: expected `Percent`, found `int`; a value becomes a `Percent` where its predicate is checked, with `Percent.try_from(...)`
  --> e0811_needs_try_from.sisu:9:17
  |
9 |     print(steam(n))
  |                 ^
  = note: check it where it runs: `Percent.try_from(...)`

error[D0006]: the program was not run: errors found (1)
```

Call `Percent.try_from(n)`, as the message says, and decide what to do
with a value outside the predicate, here with `else`. A reference to
the base where the refinement goes, the `ref int` of a loop over a
`list[int]`, is E0811 too, with the same note.

<!-- file: examples/reference-diagnostics/e0811_needs_try_from_fixed.sisu -->
```sisu
# `try_from` checks the predicate where the value becomes a `Percent`.
type Percent = int where 0 <= self <= 100

def steam(humidity: Percent) -> str:
    return if humidity < 50: "soft" else: "heavy"

def main():
    let n = "70".parse_int() ?? 0
    let humidity = Percent.try_from(n) else:
        print("{n} is not a percentage")
        return
    print(steam(humidity))
```

<!-- output: examples/reference-diagnostics/e0811_needs_try_from_fixed.out -->
```text
heavy
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### E0812

`refine.change-through-base`: `{what}` is of type `{ty}`, which a
change through its base could take outside `{predicate}`; change a copy
of the base and check it again with `try_from`.

A refinement's predicate is checked where a value comes in, so a method
of the base that changes the value, such as `push`, could break it
unseen.

<!-- file: examples/reference-diagnostics/e0812_change_through_base.sisu -->
```sisu
# A refined list changed through its base.
type Short = list[int] where len(self) <= 3

def main():
    var picks: Short = [1, 2]
    picks.push(3)
    print(picks)
```

<!-- output: examples/reference-diagnostics/e0812_change_through_base.console -->
```console
$ sisu run e0812_change_through_base.sisu
error[E0812]: `picks` is of type `Short`, which a change through its base could take outside `len(self) <= 3`; change a copy of the base and check it again with `try_from`
  --> e0812_change_through_base.sisu:6:5
  |
6 |     picks.push(3)
  |     ^^^^^
  = note: a copy of `picks` is a `Short` too: change a copy typed as the base, `var copy: list[int] = picks.copy()`, and check it again with `Short.try_from(copy)`

error[D0006]: the program was not run: errors found (1)
```

Change a copy typed as the base, and check it again, as the note
`refine.copy-as-base` says. A copy made with `.copy()` alone is still a
`Short`, and gets E0812 too.

<!-- file: examples/reference-diagnostics/e0812_change_through_base_fixed.sisu -->
```sisu
# A copy typed as the base is changed, and checked again.
type Short = list[int] where len(self) <= 3

def main():
    let picks: Short = [1, 2]
    var more: list[int] = picks.copy()
    more.push(3)
    print(Short.try_from(more))
```

<!-- output: examples/reference-diagnostics/e0812_change_through_base_fixed.out -->
```text
Ok([1, 2, 3])
```

Shown in [R11](contracts.md#refinement-types).

### E0813

`refine.own-impl`: `{ty}` refines `{base}` and has its reading impls;
it cannot have an impl of `{trait}` of its own.

A refinement is its base wherever the base is expected, with the base's
impls of the traits that read it, so an impl of its own would make the
same value behave two ways.

<!-- file: examples/reference-diagnostics/e0813_refinement_impl.sisu -->
```sisu
# A refinement with an impl of a trait of its own.
type Hour = int where 0 <= self < 24

impl Display for Hour:
    def fmt(self, mut f: Formatter):
        f.write("{self}:00")

def main():
    let opens: Hour = 17
    print(opens)
```

<!-- output: examples/reference-diagnostics/e0813_refinement_impl.console -->
```console
$ sisu run e0813_refinement_impl.sisu
error[E0813]: `Hour` refines `int` and has its reading impls; it cannot have an impl of `Display` of its own
  --> e0813_refinement_impl.sisu:4:1
  |
4 | impl Display for Hour:
  | ^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write a function, or a struct of one field, for the behaviour of its
own.

<!-- file: examples/reference-diagnostics/e0813_refinement_impl_fixed.sisu -->
```sisu
# A function formats the hour; the refinement keeps the base's impls.
type Hour = int where 0 <= self < 24

def clock(hour: Hour) -> String:
    return "{hour}:00"

def main():
    let opens: Hour = 17
    print(clock(opens))
```

<!-- output: examples/reference-diagnostics/e0813_refinement_impl_fixed.out -->
```text
17:00
```

Shown in [R11](contracts.md#refinement-types).

### E0814

`refine.predicate-type`: the predicate of `{ty}` must be a `bool`,
found `{found}`.

Sisu has no truthiness, so a predicate is a `bool` that says when a
value belongs to the type.

<!-- file: examples/reference-diagnostics/e0814_predicate_type.sisu -->
```sisu
# A predicate that is not a `bool`.
type Even = int where self % 2

def main():
    let n: Even = 4
    print(n)
```

<!-- output: examples/reference-diagnostics/e0814_predicate_type.console -->
```console
$ sisu run e0814_predicate_type.sisu
error[E0814]: the predicate of `Even` must be a `bool`, found `int`
  --> e0814_predicate_type.sisu:2:23
  |
2 | type Even = int where self % 2
  |                       ^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<!-- file: examples/reference-diagnostics/e0814_predicate_type_fixed.sisu -->
```sisu
# The predicate says when the value is even.
type Even = int where self % 2 == 0

def main():
    let n: Even = 4
    print(n)
```

<!-- output: examples/reference-diagnostics/e0814_predicate_type_fixed.out -->
```text
4
```

Shown in [R11](contracts.md#refinement-types).

## Stage 0's limits

Codes E0900 to E0991. What stage 0 does not implement yet, its internal
errors, the check that a `sim` build cannot escape, and the rules of
unsafe code and foreign functions
([R7](ownership.md#unsafe-code-and-raw-pointers)).

### E0900

`stage0.unsupported`: {construct} is not implemented in stage 0 yet; it
arrives with {phase}.

Stage 0 parses some of what the proposal designs before it can run it,
and names the construct and the phase it arrives with
([R16](sisu-command.md)). The rest of the program is checked as far as
it can be, so an error that follows from the missing construct, such as
a name that the missing declaration would have made, may come after it.

<!-- file: examples/reference-diagnostics/e0900_unsupported.sisu -->
```sisu
# A function inside a function, which stage 0 does not implement yet.
def main():
    let readings = [78, 85, 92]

    def doubled(r: int) -> int:
        return r * 2

    for r in readings:
        print(doubled(r))
```

<!-- output: examples/reference-diagnostics/e0900_unsupported.console -->
```console
$ sisu run e0900_unsupported.sisu
error[E0900]: a nested `def` is not implemented in stage 0 yet; it arrives with nested declarations (a later phase)
  --> e0900_unsupported.sisu:5:5
  |
5 |     def doubled(r: int) -> int:
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0200]: cannot find `doubled` in this scope
  --> e0900_unsupported.sisu:9:15
  |
9 |         print(doubled(r))
  |               ^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

Write what stage 0 runs: here a closure, which a local name holds as
it would hold the inner function, or a function at the top of the
module ([chapter 16](../tutorial/16-closures-and-iterators.md#closures)).

<!-- file: examples/reference-diagnostics/e0900_unsupported_fixed.sisu -->
```sisu
# A closure, which stage 0 implements, in place of the inner function.
def main():
    let readings = [78, 85, 92]
    let doubled = (r: int) -> r * 2
    for r in readings:
        print(doubled(r))
```

<!-- output: examples/reference-diagnostics/e0900_unsupported_fixed.out -->
```text
156
170
184
```

Shown in [chapter 23](../tutorial/23-contracts.md#what-waits-for-the-verifier).

### E0981

`unsafe.call`: `{name}` is an `unsafe def`: call it inside an `unsafe:`
block or an `unsafe` expression.

An `unsafe def` has obligations that its callers must make true, listed
under `Safety:` in its doc comment, so a call of one is an unsafe
operation: it stands in an `unsafe:` block or an `unsafe` expression,
where the audit finds it
([R7](ownership.md#unsafe-code-and-raw-pointers)). The message names the
builtins by their internal names, such as `ptr_mut.write` for a write
through a `ptr mut T`.

<!-- file: examples/reference-diagnostics/e0981_unsafe_call.sisu -->
```sisu
# An `unsafe def` called outside unsafe code.
from std.mem import as_ptr

## The reading at `i` of `temps`, without the check that there is one.
##
## Safety:
## - `in_range`: `i` is at least 0 and less than `len(temps)`
unsafe def reading_at(temps: Slice[int], i: int) -> int:
    let first = as_ptr(temps)
    # safety: in_bounds, valid, aligned: this function's `in_range`, and
    #   `temps` lives, unchanged, until the read.
    return unsafe first.offset(i).read()

def main():
    let temps = [78, 85, 92]
    print(reading_at(temps, 2))
```

<!-- output: examples/reference-diagnostics/e0981_unsafe_call.console -->
```console
$ sisu run e0981_unsafe_call.sisu
error[E0981]: `reading_at` is an `unsafe def`: call it inside an `unsafe:` block or an `unsafe` expression
  --> e0981_unsafe_call.sisu:16:11
   |
16 |     print(reading_at(temps, 2))
   |           ^^^^^^^^^^^^^^^^^^^^
   |           -------------------- its obligation: `in_range`
  = note: write why they hold in a `# safety:` comment above the block, naming each obligation it discharges

error[D0006]: the program was not run: errors found (1)
```

Write `unsafe` before the call, with a `# safety:` comment above its
line that names each obligation and says why it holds; without the
comment the call is W0007.

<!-- file: examples/reference-diagnostics/e0981_unsafe_call_fixed.sisu -->
```sisu
# The call in an `unsafe` expression, with the reason it is sound.
from std.mem import as_ptr

## The reading at `i` of `temps`, without the check that there is one.
##
## Safety:
## - `in_range`: `i` is at least 0 and less than `len(temps)`
unsafe def reading_at(temps: Slice[int], i: int) -> int:
    let first = as_ptr(temps)
    # safety: in_bounds, valid, aligned: this function's `in_range`, and
    #   `temps` lives, unchanged, until the read.
    return unsafe first.offset(i).read()

def main():
    let temps = [78, 85, 92]
    # safety: in_range: the list holds three readings, so 2 is in range.
    print(unsafe reading_at(temps, 2))
```

<!-- output: examples/reference-diagnostics/e0981_unsafe_call_fixed.out -->
```text
92
```

Shown in [chapter 27](../tutorial/27-systems.md).

### E0989

`unsafe.export-signature`: `{name}` is `@export`ed, and foreign code
that calls it cannot keep `{ty}` alive: an exported function's signature
holds no reference, view, `Shared`, closure or capability.

Foreign code knows nothing of Sisu's borrows, so an exported function
takes and gives only what it can hold without them.

<!-- file: examples/reference-diagnostics/e0989_export_signature.sisu -->
```sisu
# An exported function that takes a view, which C cannot keep alive.
@export("sauna_name_length")
def name_length(name: str) -> int:
    return len(name)

def main():
    print(name_length("Kotiharju"))
```

<!-- output: examples/reference-diagnostics/e0989_export_signature.console -->
```console
$ sisu run e0989_export_signature.sisu
error[E0989]: `name_length` is `@export`ed, and foreign code that calls it cannot keep `str` alive: an exported function's signature holds no reference, view, `Shared`, closure or capability
  --> e0989_export_signature.sisu:3:17
  |
3 | def name_length(name: str) -> int:
  |                 ^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Take plain values, such as numbers, or a pointer and a length.

<!-- file: examples/reference-diagnostics/e0989_export_signature_fixed.sisu -->
```sisu
# The exported function takes plain numbers.
@export("sauna_heat")
def heat(celsius: i32, minutes: i32) -> i32:
    return celsius * minutes

def main():
    print(heat(80, 10))
```

<!-- output: examples/reference-diagnostics/e0989_export_signature_fixed.out -->
```text
800
```

Shown in [R14](attributes.md#exported-functions).

### E0990

`unsafe.blocking-not-foreign`: `@blocking` marks a foreign function
whose call may block its thread, and `{name}` is not a foreign function.

The scheduler sees every blocking point of Sisu code, so only a foreign
function, whose code it cannot see, says that its call may block.

<!-- file: examples/reference-diagnostics/e0990_blocking_not_foreign.sisu -->
```sisu
# `@blocking` on a function of the program.
@blocking
def wait_for_heat() -> int:
    return 80

def main():
    print(wait_for_heat())
```

<!-- output: examples/reference-diagnostics/e0990_blocking_not_foreign.console -->
```console
$ sisu run e0990_blocking_not_foreign.sisu
error[E0990]: `@blocking` marks a foreign function whose call may block its thread, and `wait_for_heat` is not a foreign function
  --> e0990_blocking_not_foreign.sisu:2:1
  |
2 | @blocking
  | ^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<!-- file: examples/reference-diagnostics/e0990_blocking_not_foreign_fixed.sisu -->
```sisu
# A function of the program needs no `@blocking`.
def wait_for_heat() -> int:
    return 80

def main():
    print(wait_for_heat())
```

<!-- output: examples/reference-diagnostics/e0990_blocking_not_foreign_fixed.out -->
```text
80
```

Shown in [R14](attributes.md#foreign-functions-that-block).

### E0991

`unsafe.export-not-c`: `{name}` is `@export`ed, and `{ty}` has no layout
C shares: an exported function takes and returns numbers, `bool`,
`char`, raw pointers and types declared `@repr(c)`.

C calls an exported function with values laid out as C lays them out, so
each parameter and the result is a number of a width C has (`int`,
`f64`, `f32`, `u8` to `u64`, `i8` to `i32`), `bool`, `char`, nothing,
a raw pointer, or a type declared `@repr(c)`
([12](../../proposal/12-interop-and-targets.md#c-interop)). A type that
holds a reference, a view, a `Shared`, a closure or a capability is
E0989 instead.

<!-- file: examples/reference-diagnostics/e0991_export_not_c.sisu -->
```sisu
# An exported function that takes a struct C has no layout for.
struct Reading:
    stove: String
    celsius: i32

@export("sauna_reading_heat")
def reading_heat(reading: Reading) -> i32:
    return reading.celsius

def main():
    print(reading_heat(Reading("Kotiharju", 80)))
```

<!-- output: examples/reference-diagnostics/e0991_export_not_c.console -->
```console
$ sisu run e0991_export_not_c.sisu
error[E0991]: `reading_heat` is `@export`ed, and `Reading` has no layout C shares: an exported function takes and returns numbers, `bool`, `char`, raw pointers and types declared `@repr(c)`
  --> e0991_export_not_c.sisu:7:18
  |
7 | def reading_heat(reading: Reading) -> i32:
  |                  ^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

The fix declares the struct `@repr(c)`, which asks its fields to be
plain values too: a `String` is not, so the stove is named by a number.

<!-- file: examples/reference-diagnostics/e0991_export_not_c_fixed.sisu -->
```sisu
# A struct laid out as C lays it out crosses the boundary.
@repr(c)
struct Reading:
    stove: i32
    celsius: i32

@export("sauna_reading_heat")
def reading_heat(reading: Reading) -> i32:
    return reading.celsius

def main():
    print(reading_heat(Reading(1, 80)))
```

<!-- output: examples/reference-diagnostics/e0991_export_not_c_fixed.out -->
```text
80
```

Shown in [R14](attributes.md#exported-functions).

## Packages and manifests

Codes E1000 to E1023. The driver reads `sisu.toml`, `sisu.lock` and
the module tree of a package before anything is parsed, and reports a
mistake at its place in the manifest ([R13](modules.md#the-manifest)).

### E1002

`package.missing-key`: `{table}` needs `{key}`.

`[package]` needs `name`, `version` and `edition`; the message names the
table and the key that is missing ([R13](modules.md#package)).

<!-- file: examples/reference-diagnostics/e1002_missing_key.files/sisu.toml -->
```toml
[package]
name = "kiuas"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1002_missing_key.console -->
```console
$ sisu run
error[E1002]: `[package]` needs `version`
  --> sisu.toml:1:2
  |
1 | [package]
  |  ^^^^^^^

```

Add the key.

<!-- file: examples/reference-diagnostics/e1002_missing_key_fixed.files/sisu.toml -->
```toml
[package]
name = "kiuas"
version = "0.1.0"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1002_missing_key_fixed.console -->
```console
$ sisu run
kiuas
```

Shown in [R13](modules.md#mistakes-in-the-manifest).

### E1003

`package.bad-value`: `{key}` takes {expected}, found {found}.

Each key of `sisu.toml` takes a value of one kind, and the message says
which.

<!-- file: examples/reference-diagnostics/e1003_bad_value.files/sisu.toml -->
```toml
[package]
name = "stove"
version = "1.0"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1003_bad_value.console -->
```console
$ sisu check
error[E1003]: `version` takes a version `MAJOR.MINOR.PATCH`, as in `"0.3.1"`, found `"1.0"`
  --> sisu.toml:3:11
  |
3 | version = "1.0"
  |           ^^^^^

```

Write the version in three parts.

<!-- file: examples/reference-diagnostics/e1003_bad_value_fixed.files/sisu.toml -->
```toml
[package]
name = "stove"
version = "1.0.0"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1003_bad_value_fixed.console -->
```console
$ sisu run
kiuas
```

Shown in [R13](modules.md#mistakes-in-the-manifest).

### E1007

`package.cycle`: the packages depend on each other in a cycle: {cycle};
packages build in dependency order, so their graph has no cycle.

Packages build in dependency order, so a package cannot depend on
itself through others, while the modules of one package may import each
other freely (D-019).

<!-- file: examples/reference-diagnostics/e1007_cycle.files/stove/sisu.toml -->
```toml
[package]
name = "stove"
version = "0.1.0"
edition = "2027"

[dependencies]
benches = { path = "../benches" }
```

<!-- file: examples/reference-diagnostics/e1007_cycle.files/benches/sisu.toml -->
```toml
[package]
name = "benches"
version = "0.1.0"
edition = "2027"

[dependencies]
stove = { path = "../stove" }
```

<!-- output: examples/reference-diagnostics/e1007_cycle.console -->
```console
$ sisu check stove
error[E1007]: the packages depend on each other in a cycle: `stove` -> `benches` -> `stove`; packages build in dependency order, so their graph has no cycle
  --> ../benches/sisu.toml:7:1
  |
7 | stove = { path = "../stove" }
  | ^^^^^
  ::: sisu.toml:7:1
  |
7 | benches = { path = "../benches" }
  | ------- `stove` depends on `benches` here

```

Remove the dependency that closes the cycle, here the library's on the
application, or move what both need into a third package.

<!-- file: examples/reference-diagnostics/e1007_cycle_fixed.files/benches/sisu.toml -->
```toml
[package]
name = "benches"
version = "0.1.0"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1007_cycle_fixed.console -->
```console
$ sisu run stove
6 seats
```

Shown in [chapter 17](../tutorial/17-modules-and-packages.md).

### E1010

`package.not-a-path`: the dependency `{name}` is not a `path`
dependency; stage 0 has no registry, so a dependency is `{name} = { path
= "../{name}" }`.

Stage 0 has no registry, so a dependency is a package in a directory,
named by its path; a version, as `stones = "0.4"`, waits for the
registry (D-031). E1010 stands alone: the import of the dependency,
which then names nothing, is not reported as E0220 too
([R13](modules.md#dependencies-and-dev-dependencies)); so it is in a
script's inline metadata ([G29](../guides/script.md)).

<!-- file: examples/reference-diagnostics/e1010_not_a_path.files/app/sisu.toml -->
```toml
[package]
name = "app"
version = "0.1.0"
edition = "2027"

[dependencies]
stones = "0.4"
```

<!-- output: examples/reference-diagnostics/e1010_not_a_path.console -->
```console
$ sisu run app
error[E1010]: the dependency `stones` is not a `path` dependency; stage 0 has no registry, so a dependency is `stones = { path = "../stones" }`
  --> sisu.toml:7:10
  |
7 | stones = "0.4"
  |          ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Write the dependency as the message does, with the path of its directory
relative to the manifest.

<!-- file: examples/reference-diagnostics/e1010_not_a_path_fixed.files/app/sisu.toml -->
```toml
[package]
name = "app"
version = "0.1.0"
edition = "2027"

[dependencies]
stones = { path = "../stones" }
```

<!-- output: examples/reference-diagnostics/e1010_not_a_path_fixed.console -->
```console
$ sisu run app
85
```

Shown in [R13](modules.md#mistakes-in-the-manifest).

### E1015

`package.trust-unknown`: `[trust] unsafe` names `{name}`, which is not a
dependency of this package.

`[trust] unsafe` lists dependencies of the package, so a name in it that
is not one is a mistake: a dependency removed, or a name misspelt
([R13](modules.md#trust)).

<!-- file: examples/reference-diagnostics/e1015_trust_unknown.files/sisu.toml -->
```toml
[package]
name = "kiuas"
version = "0.1.0"
edition = "2027"

[trust]
unsafe = ["zlib"]
```

<!-- output: examples/reference-diagnostics/e1015_trust_unknown.console -->
```console
$ sisu run
error[E1015]: `[trust] unsafe` names `zlib`, which is not a dependency of this package
  --> sisu.toml:7:11
  |
7 | unsafe = ["zlib"]
  |           ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Remove the name, as the fixed manifest does, or correct it to the
dependency it means.

<!-- file: examples/reference-diagnostics/e1015_trust_unknown_fixed.files/sisu.toml -->
```toml
[package]
name = "kiuas"
version = "0.1.0"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1015_trust_unknown_fixed.console -->
```console
$ sisu run
kiuas
```

Shown in [R13](modules.md#trust).

### E1017

`package.reserved-name`: `{name}` is the name of a standard-library
module, which `import {name}` names; a package of the program needs
another name.

A package's name is the first part of its modules' paths, as in `import
json`, so it cannot be the name of a module of the standard library,
which the same import names ([R13](modules.md#package)).

<!-- file: examples/reference-diagnostics/e1017_reserved_name.files/sisu.toml -->
```toml
[package]
name = "json"
version = "0.1.0"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1017_reserved_name.console -->
```console
$ sisu run
error[E1017]: `json` is the name of a standard-library module, which `import json` names; a package of the program needs another name
  --> sisu.toml:2:8
  |
2 | name = "json"
  |        ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Choose another name, as `json_tool`.

<!-- file: examples/reference-diagnostics/e1017_reserved_name_fixed.files/sisu.toml -->
```toml
[package]
name = "json_tool"
version = "0.1.0"
edition = "2027"
```

<!-- output: examples/reference-diagnostics/e1017_reserved_name_fixed.console -->
```console
$ sisu run
json_tool
```

Shown in [R13](modules.md#mistakes-in-the-manifest).

### E1019

`package.unsafe-undeclared`: the package `{package}` contains {what},
and its `sisu.toml` declares no `unsafe`: add `unsafe = "{level}"` to
its `[package]`.

A package that holds `unsafe` blocks or foreign code says so in its
`[package]`, with the level the code needs: `"memory"` for unsafe
operations on memory, `"extern"` for foreign functions
([R13](modules.md#trust)). The level is part of what a user of the
package reads before trusting it, and the lockfile records it.

<!-- file: examples/reference-diagnostics/e1019_unsafe_undeclared.files/sisu.toml -->
```toml
[package]
name = "kiuas"
version = "0.1.0"
edition = "2027"
```

<!-- file: examples/reference-diagnostics/e1019_unsafe_undeclared.files/src/main.sisu -->
```sisu
# A package with unsafe code, which its manifest must declare.
from std.mem import ptr

def main():
    var celsius = 80
    let p = ptr.of_mut(mut celsius)
    # safety: valid, aligned, writable: p points to the live local
    #   celsius, which nothing else uses until the block ends.
    unsafe:
        p.write(p.read() + 5)
    print(celsius)
```

<!-- output: examples/reference-diagnostics/e1019_unsafe_undeclared.console -->
```console
$ sisu run
error[E1019]: the package `kiuas` contains unsafe code, and its `sisu.toml` declares no `unsafe`: add `unsafe = "memory"` to its `[package]`
  --> src/main.sisu:9:5
  |
9 |     unsafe:
  |     ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Add the level that the message names to `[package]`.

<!-- file: examples/reference-diagnostics/e1019_unsafe_undeclared_fixed.files/sisu.toml -->
```toml
[package]
name = "kiuas"
version = "0.1.0"
edition = "2027"
unsafe = "memory"
```

<!-- output: examples/reference-diagnostics/e1019_unsafe_undeclared_fixed.console -->
```console
$ sisu run
85
```

Shown in [chapter 27](../tutorial/27-systems.md).

### E1021

`package.unsafe-untrusted`: the dependency `{name}` declares `unsafe =
"{level}"`, and `[trust] unsafe` in the root package's `sisu.toml` does
not list it.

A dependency that declares an unsafe level asks the root package for
consent, which `[trust] unsafe` in the root's `sisu.toml` gives by
naming it ([R13](modules.md#trust)). E1021 stands alone, since it names
the fix: the dependency's `unsafe` blocks and `extern` blocks, which
would be E0703 without the declared level, are not reported again.

<!-- file: examples/reference-diagnostics/e1021_unsafe_untrusted.files/app/sisu.toml -->
```toml
[package]
name = "app"
version = "0.1.0"
edition = "2027"

[dependencies]
readings = { path = "../readings" }
```

<!-- file: examples/reference-diagnostics/e1021_unsafe_untrusted.files/readings/sisu.toml -->
```toml
[package]
name = "readings"
version = "0.1.0"
edition = "2027"
unsafe = "memory"
```

<!-- output: examples/reference-diagnostics/e1021_unsafe_untrusted.console -->
```console
$ sisu run app
error[E1021]: the dependency `readings` declares `unsafe = "memory"`, and `[trust] unsafe` in the root package's `sisu.toml` does not list it
  --> ../readings/sisu.toml:5:10
  |
5 | unsafe = "memory"
  |          ^^^^^^^^
  = note: the root package consents by listing it: `[trust] unsafe = ["readings"]`

error[D0006]: the program was not run: errors found (1)
```

List the dependency in `[trust] unsafe`, as the note says, once you have
read what its unsafe code does; `sisu audit` lists the blocks with their
`# safety:` comments ([R16](sisu-command.md#sisu-audit)).

<!-- file: examples/reference-diagnostics/e1021_unsafe_untrusted_fixed.files/app/sisu.toml -->
```toml
[package]
name = "app"
version = "0.1.0"
edition = "2027"

[dependencies]
readings = { path = "../readings" }

[trust]
unsafe = ["readings"]
```

<!-- output: examples/reference-diagnostics/e1021_unsafe_untrusted_fixed.console -->
```console
$ sisu run app
92
```

Shown in the guide [Call a C library](../guides/c-library.md).

### E1023

`package.freestanding`: the runtime profile `freestanding` has no
runtime yet: this compiler's runtime schedules tasks, so a program is
built for `full` or `single`.

The manifest's `runtime` key chooses the runtime profile
([R22](targets.md#runtime-profiles)), and `freestanding`, the profile
without a scheduler, an allocator or unwinding, has no runtime in this
compiler, so a package that asks for it is refused at the value.

<!-- file: examples/reference-diagnostics/e1023_freestanding.files/sisu.toml -->
```toml
[package]
name = "kiuas"
version = "0.1.0"
edition = "2027"
runtime = "freestanding"
```

<!-- output: examples/reference-diagnostics/e1023_freestanding.console -->
```console
$ sisu check
error[E1023]: the runtime profile `freestanding` has no runtime yet: this compiler's runtime schedules tasks, so a program is built for `full` or `single`
  --> sisu.toml:5:11
  |
5 | runtime = "freestanding"
  |           ^^^^^^^^^^^^^^

```

Ask for `single` for now, the runtime with one thread that every
program of this compiler gets; the program reads its profile as
`target.runtime_profile`.

<!-- file: examples/reference-diagnostics/e1023_freestanding_fixed.files/sisu.toml -->
```toml
[package]
name = "kiuas"
version = "0.1.0"
edition = "2027"
runtime = "single"
```

<!-- output: examples/reference-diagnostics/e1023_freestanding_fixed.console -->
```console
$ sisu run
kiuas on single
```

Shown in [R22](targets.md#runtime-profiles).

## Warnings

Codes W0001 to W0033. A warning does not stop a program: `sisu run`
prints it and runs.

### W0001

`type.hole`: typed hole `{name}` of type `{ty}`.

A typed hole, `todo("name")`, stands for code still to be written
(D-061). The program compiles and runs until it reaches the hole, which
panics ([R0013](#r0013)); `sisu test` reports a test that reaches one as
blocked. `sisu check`, `sisu run` and a debug build warn; a release
build refuses the hole instead ([E0445](#e0445)).

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

Shown in [chapter 1](../tutorial/01-hello.md).

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

Shown in [chapter 2](../tutorial/02-values.md).

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### W0005

`package.unknown-key`: `{key}` is not a key of `{table}` that stage 0
knows; it is ignored.

A key of `sisu.toml` that stage 0 does not read is a warning, so that a
manifest written for a later version still builds, and a misspelt key
is still seen.

<!-- file: examples/reference-diagnostics/w0005_unknown_key.files/sisu.toml -->
```toml
[package]
name = "stove"
version = "0.1.0"
edition = "2027"
author = "Aino"
```

<!-- output: examples/reference-diagnostics/w0005_unknown_key.console -->
```console
$ sisu run
warning[W0005]: `author` is not a key of `[package]` that stage 0 knows; it is ignored
  --> sisu.toml:5:1
  |
5 | author = "Aino"
  | ^^^^^^

kiuas
```

Here the key is `authors`, a list.

<!-- file: examples/reference-diagnostics/w0005_unknown_key_fixed.files/sisu.toml -->
```toml
[package]
name = "stove"
version = "0.1.0"
edition = "2027"
authors = ["Aino"]
```

<!-- output: examples/reference-diagnostics/w0005_unknown_key_fixed.console -->
```console
$ sisu run
kiuas
```

Shown in [R22](targets.md#runtime-profiles).

### W0007

`unsafe.no-safety-comment`: this {what} has no `# safety:` comment above
it saying why each obligation it takes on holds.

Each `unsafe:` block, `unsafe` expression, `unsafe impl` and `extern`
block has a `# safety:` comment on the lines above it that says why each
obligation it takes on holds, so that the audit can list the reason with
the code (D-083). A warning, so the program runs.

<!-- file: examples/reference-diagnostics/w0007_no_safety_comment.sisu -->
```sisu
# Unsafe code without the comment that says why it is sound.
from std.mem import ptr

def main():
    var celsius = 80
    let p = ptr.of_mut(mut celsius)
    unsafe:
        p.write(p.read() + 5)
    print(celsius)
```

<!-- output: examples/reference-diagnostics/w0007_no_safety_comment.console -->
```console
$ sisu run w0007_no_safety_comment.sisu
warning[W0007]: this `unsafe:` block has no `# safety:` comment above it saying why each obligation it takes on holds
  --> w0007_no_safety_comment.sisu:7:5
  |
7 |     unsafe:
  |     ^^^^^^

85
```

Write the comment, naming the obligations as the operations list them
(`valid`, `aligned`, `writable` for a write through a pointer). `sisu
publish` will refuse a package with this warning.

<!-- file: examples/reference-diagnostics/w0007_no_safety_comment_fixed.sisu -->
```sisu
# The comment names each obligation the block takes on.
from std.mem import ptr

def main():
    var celsius = 80
    let p = ptr.of_mut(mut celsius)
    # safety: valid, aligned, writable: p points to the live local
    #   celsius, which nothing else uses until the block ends.
    unsafe:
        p.write(p.read() + 5)
    print(celsius)
```

<!-- output: examples/reference-diagnostics/w0007_no_safety_comment_fixed.out -->
```text
85
```

Shown in [chapter 27](../tutorial/27-systems.md).

### W0010

`unsafe.unnamed-obligation`: the `# safety:` comment names no obligation
of `{callee}`, which are {obligations}.

A `# safety:` comment names the obligations it discharges, each a name
and a `:` (`in_range: ...`), so that a reader can check each against the
`Safety:` list of the function called (Q-72). A comment that names none
of a callee's obligations gets this warning once per callee.

<!-- file: examples/reference-diagnostics/w0010_unnamed_obligation.sisu -->
```sisu
# A safety comment that names none of the callee's obligations.
from std.mem import as_ptr

## The reading at `i` of `temps`, without the check that there is one.
##
## Safety:
## - `in_range`: `i` is at least 0 and less than `len(temps)`
unsafe def reading_at(temps: Slice[int], i: int) -> int:
    let first = as_ptr(temps)
    # safety: in_bounds, valid, aligned: this function's `in_range`, and
    #   `temps` lives, unchanged, until the read.
    return unsafe first.offset(i).read()

def main():
    let temps = [78, 85, 92]
    # safety: the list was made on the line above.
    print(unsafe reading_at(temps, 2))
```

<!-- output: examples/reference-diagnostics/w0010_unnamed_obligation.console -->
```console
$ sisu run w0010_unnamed_obligation.sisu
warning[W0010]: the `# safety:` comment names no obligation of `reading_at`, which are `in_range`
  --> w0010_unnamed_obligation.sisu:16:5
   |
16 |     # safety: the list was made on the line above.
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

92
```

Name the obligations that the message lists, and say why each holds.

<!-- file: examples/reference-diagnostics/w0010_unnamed_obligation_fixed.sisu -->
```sisu
# The comment names the obligation that the call discharges.
from std.mem import as_ptr

## The reading at `i` of `temps`, without the check that there is one.
##
## Safety:
## - `in_range`: `i` is at least 0 and less than `len(temps)`
unsafe def reading_at(temps: Slice[int], i: int) -> int:
    let first = as_ptr(temps)
    # safety: in_bounds, valid, aligned: this function's `in_range`, and
    #   `temps` lives, unchanged, until the read.
    return unsafe first.offset(i).read()

def main():
    let temps = [78, 85, 92]
    # safety: in_range: the list holds three readings, so 2 is in range.
    print(unsafe reading_at(temps, 2))
```

<!-- output: examples/reference-diagnostics/w0010_unnamed_obligation_fixed.out -->
```text
92
```

Shown in [chapter 27](../tutorial/27-systems.md).

### W0014

`unsafe.ambient-authority`: `{name}` is public and safe and reaches the
foreign function `{callee}` without taking a capability, so its callers
need no authority for what the call does.

A function that is public, safe and not `pure`, and that calls a foreign
function, would give every caller the authority of the call without a
capability, which the rest of the language never does
([22](../../proposal/22-unsafe.md), clause 5). It is a warning, and a
library that wraps C answers it by taking the capability the call
exercises.

<!-- file: examples/reference-diagnostics/w0014_ambient_authority.sisu -->
```sisu
# A safe function that reads the environment through C without a capability.
from std.ffi import CString, c_char
from std.mem import ptr

# safety: the signature as <stdlib.h> declares it; it returns promptly.
extern "C":
    def getenv(name: ptr[c_char]) -> ptr[c_char]?

## Whether the environment variable `name` is set.
pub def is_set(name: str) -> bool:
    let c = CString.new(name) else:
        return False
    # safety: valid: `c` lives across the call and ends with its NUL, and
    #   getenv keeps no pointer to it.
    return unsafe getenv(c.as_ptr()).is_some()
```

<!-- output: examples/reference-diagnostics/w0014_ambient_authority.console -->
```console
$ sisu check w0014_ambient_authority.sisu
warning[W0014]: `is_set` is public and safe and reaches the foreign function `getenv` without taking a capability, so its callers need no authority for what the call does
  --> w0014_ambient_authority.sisu:10:9
   |
10 | pub def is_set(name: str) -> bool:
   |         ^^^^^^
  = note: take the capability whose authority the call exercises (22, clause 5), as `std.rt.os` does behind `System`

```

Take the capability, as `env: Env` here for `getenv`, so that only a
caller that holds it can read the environment. A foreign function that
reaches nothing, such as `abs`, belongs in a `pure` function, whose `#
safety:` comment says so under `pure:`. Both programs are only
checked, as [chapter 27](../tutorial/27-systems.md#no-authority-without-a-capability)
checks its own; it runs a `CString` through C's `strlen`.

<!-- file: examples/reference-diagnostics/w0014_ambient_authority_fixed.sisu -->
```sisu
# The function takes `Env`, the authority that the call exercises.
from std.ffi import CString, c_char
from std.mem import ptr

# safety: the signature as <stdlib.h> declares it; it returns promptly.
extern "C":
    def getenv(name: ptr[c_char]) -> ptr[c_char]?

## Whether the variable `name` of the environment `env` reaches is set.
pub def is_set(env: Env, name: str) -> bool:
    let c = CString.new(name) else:
        return False
    # safety: valid: `c` lives across the call and ends with its NUL, and
    #   getenv keeps no pointer to it.
    return unsafe getenv(c.as_ptr()).is_some()
```

<!-- output: examples/reference-diagnostics/w0014_ambient_authority_fixed.console -->
```console
$ sisu check w0014_ambient_authority_fixed.sisu
```

Shown in [chapter 27](../tutorial/27-systems.md).

### W0015

`lint.wildcard-own-enum`: `case _` matches {variants} of `{ty}`, an enum
of this package, so a variant added to `{ty}` later is not an error
here.

A lint, which `sisu lint` prints and `sisu check` does not
([R16](sisu-command.md#sisu-lint)). A `match` over an enum of its own
package should name every variant, so that a variant added later is an
error at each `match` that has to handle it (D-043). The fix
`lint.list-variants` writes the variants the wildcard matches in its
place. A wildcard over an enum of another package, a wildcard nested in
a pattern, and a binding such as `case other:` are not linted.

Example: [R16](sisu-command.md#sisu-lint) shows it.

### W0021

`contract.trait-elsewhere`: the contract of `{method}` binds this impl,
and stage 1 does not have the syntax of the trait's module to check it
in.

A trait's method may declare `requires` and `ensures`, which bind every
impl of the method, in any module or package: the impl's method checks
the trait's clauses in the syntax of the trait's module
([R11](contracts.md#contracts-on-trait-methods)). The warning is kept
for an impl whose trait's module the check cannot read, so that the
contract is not taken for checked; every module of a program can be
read, so a program does not get it.

Example: [R11](contracts.md#contracts-on-trait-methods) shows the
check, for an impl in another module than the trait's.

### W0022

`resolve.unknown-attribute`: `@{name}` is not an attribute Sisu reads,
and no compile-time code of the program reads attributes
(`reflect.attributes_of`), so it means nothing here.

An attribute is data ([R14](attributes.md#attributes-are-data)). One
whose name Sisu does not read means something only to compile-time code
that reads it, and a program with no such code cannot read it at all.
Such code is a call of `reflect.attributes_of`, or a read of a field's
or a variant's `attributes`, `f.attributes` and `v.attributes`
([R14](attributes.md#attributes-are-data) shows one).

<!-- file: examples/reference-diagnostics/w0022_unknown_attribute.sisu -->
```sisu
# An attribute that nothing in the program reads.
@table("saunas")
struct Sauna:
    name: String
    heat: int

def main():
    let home = Sauna("Kotiharju", 80)
    print("{home.name} {home.heat}")
```

<!-- output: examples/reference-diagnostics/w0022_unknown_attribute.console -->
```console
$ sisu run w0022_unknown_attribute.sisu
warning[W0022]: `@table` is not an attribute Sisu reads, and no compile-time code of the program reads attributes (`reflect.attributes_of`), so it means nothing here
  --> w0022_unknown_attribute.sisu:2:2
  |
2 | @table("saunas")
  |  ^^^^^

Kotiharju 80
```

Remove the attribute, or read it: a program whose compile-time code
calls `reflect.attributes_of`, or reads a member's `attributes`, keeps
its own attributes as data, without a word.

<!-- file: examples/reference-diagnostics/w0022_unknown_attribute_fixed.sisu -->
```sisu
# Compile-time code that reads the attribute gives it a meaning.
@table("saunas")
struct Sauna:
    name: String
    heat: int

comptime def stored_in(T: type) -> String:
    var out = String.new(reflect.name_of(T))
    comptime for a in reflect.attributes_of(T):
        out = "{out} @{a.name} {a.args[0]}"
    return out

def main():
    let home = Sauna("Kotiharju", 80)
    print("{home.name} {home.heat}, {comptime stored_in(Sauna)}")
```

<!-- output: examples/reference-diagnostics/w0022_unknown_attribute_fixed.out -->
```text
Kotiharju 80, Sauna @table saunas
```

Shown in [R14](attributes.md#where-an-attribute-may-stand).

### W0023

`type.deprecated`: `{name}` is deprecated: {reason}.

A use of an item marked `@deprecated("...")` warns with the
attribute's text, which names what to use instead
([13](../../proposal/13-evolution-and-governance.md#deprecation)): a
call of a function or a method, or its use as a value, a type named or
built, a constant read, a variant built or matched
([R14](attributes.md#deprecation-and-migrations)). The item's own body
may use it without a word, and so may a type's own declaration and its
impls.

<!-- file: examples/reference-diagnostics/w0023_deprecated.sisu -->
```sisu
# A call of a function that is deprecated.

## Degrees Celsius of `fahrenheit` degrees.
def celsius_of(fahrenheit: f64) -> f64:
    return (fahrenheit - 32.0) * 5.0 / 9.0

## The old name of `celsius_of`.
@deprecated("use `celsius_of`, which this calls")
def to_celsius(fahrenheit: f64) -> f64:
    return celsius_of(fahrenheit)

def main():
    print(to_celsius(212.0))
```

<!-- output: examples/reference-diagnostics/w0023_deprecated.console -->
```console
$ sisu run w0023_deprecated.sisu
warning[W0023]: `to_celsius` is deprecated: use `celsius_of`, which this calls
  --> w0023_deprecated.sisu:13:11
   |
13 |     print(to_celsius(212.0))
   |           ^^^^^^^^^^^^^^^^^

100.0
```

Call what the message names.

<!-- file: examples/reference-diagnostics/w0023_deprecated_fixed.sisu -->
```sisu
# The call names the function that replaces the deprecated one.

## Degrees Celsius of `fahrenheit` degrees.
def celsius_of(fahrenheit: f64) -> f64:
    return (fahrenheit - 32.0) * 5.0 / 9.0

## The old name of `celsius_of`.
@deprecated("use `celsius_of`, which this calls")
def to_celsius(fahrenheit: f64) -> f64:
    return celsius_of(fahrenheit)

def main():
    print(celsius_of(212.0))
```

<!-- output: examples/reference-diagnostics/w0023_deprecated_fixed.out -->
```text
100.0
```

Shown in [chapter 28](../tutorial/28-code-that-lasts.md#keeping-an-old-name).

### W0024

`resolve.misspelt-attribute`: `@{name}` is not an attribute Sisu reads.

An attribute whose name is close to one that Sisu reads is taken for a
misspelling of it, and the fix writes that one.

<!-- file: examples/reference-diagnostics/w0024_misspelt_attribute.sisu -->
```sisu
# A misspelt attribute.

## The old name of `celsius_of`.
@deprecatd("use `celsius_of`")
def to_celsius(fahrenheit: f64) -> f64:
    return (fahrenheit - 32.0) * 5.0 / 9.0

def main():
    print(to_celsius(212.0))
```

<!-- output: examples/reference-diagnostics/w0024_misspelt_attribute.console -->
```console
$ sisu run w0024_misspelt_attribute.sisu
warning[W0024]: `@deprecatd` is not an attribute Sisu reads
  --> w0024_misspelt_attribute.sisu:4:2
  |
4 | @deprecatd("use `celsius_of`")
  |  ^^^^^^^^^
  = fix: write `@deprecated`: `deprecated`

100.0
```

With the attribute spelt as Sisu reads it, the function is deprecated,
and its call warns with W0023.

<!-- file: examples/reference-diagnostics/w0024_misspelt_attribute_fixed.sisu -->
```sisu
# The attribute spelt as Sisu reads it, which makes the call warn.

## The old name of `celsius_of`.
@deprecated("use `celsius_of`")
def to_celsius(fahrenheit: f64) -> f64:
    return (fahrenheit - 32.0) * 5.0 / 9.0

def main():
    print(to_celsius(212.0))
```

<!-- output: examples/reference-diagnostics/w0024_misspelt_attribute_fixed.console -->
```console
$ sisu run w0024_misspelt_attribute_fixed.sisu
warning[W0023]: `to_celsius` is deprecated: use `celsius_of`
  --> w0024_misspelt_attribute_fixed.sisu:9:11
  |
9 |     print(to_celsius(212.0))
  |           ^^^^^^^^^^^^^^^^^

100.0
```

[R14](attributes.md#where-an-attribute-may-stand) describes it.

### W0025

`type.deprecated-plain`: `{name}` is deprecated.

A use of an item marked `@deprecated` without a message warns without
naming a replacement, so the reader has to find one.

<!-- file: examples/reference-diagnostics/w0025_deprecated_plain.sisu -->
```sisu
# A deprecated function whose attribute names no replacement.
@deprecated
def heat_of(name: str) -> int:
    return 80

def main():
    print(heat_of("Kotiharju"))
```

<!-- output: examples/reference-diagnostics/w0025_deprecated_plain.console -->
```console
$ sisu run w0025_deprecated_plain.sisu
warning[W0025]: `heat_of` is deprecated
  --> w0025_deprecated_plain.sisu:7:11
  |
7 |     print(heat_of("Kotiharju"))
  |           ^^^^^^^^^^^^^^^^^^^^

80
```

Give the attribute a message that names the replacement, and call that.

<!-- file: examples/reference-diagnostics/w0025_deprecated_plain_fixed.sisu -->
```sisu
# The attribute names the replacement, and the call uses it.
@deprecated("use `heat`")
def heat_of(name: str) -> int:
    return heat(name)

## The heat of the sauna `name`.
def heat(name: str) -> int:
    return 80

def main():
    print(heat("Kotiharju"))
```

<!-- output: examples/reference-diagnostics/w0025_deprecated_plain_fixed.out -->
```text
80
```

[R14](attributes.md#deprecation-and-migrations) describes it.

### W0026

`resolve.misplaced-attribute`: `@{name}` means something on {places},
so on {what} it does nothing.

Each attribute that Sisu reads means something on some declarations
only ([R14](attributes.md#where-an-attribute-may-stand)): `@derive` on
a struct, an enum or an error set, `@export` on a function, `@tag` on a
test, a benchmark or a suite. On another declaration it would do
nothing, so it warns, naming the declarations it is for, and the
program runs without it.

<!-- file: examples/reference-diagnostics/w0026_misplaced_attribute.sisu -->
```sisu
# An attribute that Sisu reads, on a declaration where it means nothing.
struct Reading:
    celsius: int

@derive(Eq)
def latest() -> Reading:
    return Reading(82)

def main():
    print(latest().celsius)
```

<!-- output: examples/reference-diagnostics/w0026_misplaced_attribute.console -->
```console
$ sisu run w0026_misplaced_attribute.sisu
warning[W0026]: `@derive` means something on a struct, an enum or an error set, so on a function it does nothing
  --> w0026_misplaced_attribute.sisu:5:2
  |
5 | @derive(Eq)
  |  ^^^^^^

82
```

Move the attribute to the declaration it is meant for, here the struct,
whose values `==` then compares, or remove it.

<!-- file: examples/reference-diagnostics/w0026_misplaced_attribute_fixed.sisu -->
```sisu
# The derive moved to the struct, where it gives `==`.
@derive(Eq)
struct Reading:
    celsius: int

def latest() -> Reading:
    return Reading(82)

def main():
    print(latest() == Reading(82))
```

<!-- output: examples/reference-diagnostics/w0026_misplaced_attribute_fixed.out -->
```text
True
```

Shown in [R14](attributes.md#where-an-attribute-may-stand).

### W0027

`lint.unused-import`: `{name}` is imported and not used in this file.

A lint, which `sisu lint` prints and `sisu check` does not
([R16](sisu-command.md#sisu-lint)). The file does not spell the name
an import binds, outside the imports and its doc comments. `sisu fmt`
keeps such an import, so a file can import what it does not use yet,
and the program runs.

<!-- file: examples/reference-diagnostics/w0027_unused_import.sisu -->
```sisu
# An import that the file does not use, which `sisu lint` names.
from std.collections import Deque
from std.math import clamp

def main():
    print(clamp(1.5, 0.0, 1.0))
```

<!-- output: examples/reference-diagnostics/w0027_unused_import.console -->
```console
$ sisu lint w0027_unused_import.sisu
warning[W0027]: `Deque` is imported and not used in this file
  --> w0027_unused_import.sisu:2:1
  |
2 | from std.collections import Deque
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: remove the import

$ sisu run w0027_unused_import.sisu
1.0
```

The fix `lint.remove-import` removes the import; in a `from` that
imports other names too, it removes the name and its comma. An import
that re-exports a name, or holds a comment, is not linted.

<!-- file: examples/reference-diagnostics/w0027_unused_import_fixed.sisu -->
```sisu
# Only the import the file uses is left.
from std.math import clamp

def main():
    print(clamp(1.5, 0.0, 1.0))
```

<!-- output: examples/reference-diagnostics/w0027_unused_import_fixed.out -->
```text
1.0
```

Shown in [R16](sisu-command.md#sisu-lint) and
[R1](lexical-structure.md#the-formatters-layout).

### W0028

`type.preview`: `{name}` is a preview, which may change or be removed
before it is stable: {reason}.

A use of an item marked `@preview("...")` warns with the attribute's
text, as a use of a deprecated item does (W0023): a call of a function,
a type named or built, a constant read
([R14](attributes.md#preview-features)). The item's own declaration
uses it without a word.

<!-- file: examples/reference-diagnostics/w0028_preview.sisu -->
```sisu
# A call of a function that is a preview, with the attribute's text.

## The minutes of each sauna turn for `guests` guests.
@preview("the turns may come to depend on the stove")
def turns(guests: int) -> list[int]:
    return [20 for _ in range(guests)]

def main():
    print(turns(3))
```

<!-- output: examples/reference-diagnostics/w0028_preview.console -->
```console
$ sisu run w0028_preview.sisu
warning[W0028]: `turns` is a preview, which may change or be removed before it is stable: the turns may come to depend on the stove
  --> w0028_preview.sisu:9:11
  |
9 |     print(turns(3))
  |           ^^^^^^^^

[20, 20, 20]
```

The warning goes once the item is stable: its library takes the
attribute away, and the same call says nothing.

<!-- file: examples/reference-diagnostics/w0028_preview_fixed.sisu -->
```sisu
# The function is stable now, so it has no `@preview` and no warning.

## The minutes of each sauna turn for `guests` guests.
def turns(guests: int) -> list[int]:
    return [20 for _ in range(guests)]

def main():
    print(turns(3))
```

<!-- output: examples/reference-diagnostics/w0028_preview_fixed.out -->
```text
[20, 20, 20]
```

Shown in [chapter 28](../tutorial/28-code-that-lasts.md#marking-what-may-still-change).

### W0029

`type.preview-plain`: `{name}` is a preview, which may change or be
removed before it is stable.

A use of an item marked `@preview` without a text warns without saying
what may change.

<!-- file: examples/reference-diagnostics/w0029_preview_plain.sisu -->
```sisu
# A type that is a preview, without a text, used by the program.

## A booking of the sauna, whose fields may still change.
@preview
struct Booking:
    guest: String
    hour: int

def main():
    let booking = Booking("Aino", 18)
    print("{booking.guest} at {booking.hour}")
```

<!-- output: examples/reference-diagnostics/w0029_preview_plain.console -->
```console
$ sisu run w0029_preview_plain.sisu
warning[W0029]: `Booking` is a preview, which may change or be removed before it is stable
  --> w0029_preview_plain.sisu:10:19
   |
10 |     let booking = Booking("Aino", 18)
   |                   ^^^^^^^

Aino at 18
```

<!-- file: examples/reference-diagnostics/w0029_preview_plain_fixed.sisu -->
```sisu
# The type is stable now, so it has no `@preview` and no warning.

## A booking of the sauna.
struct Booking:
    guest: String
    hour: int

def main():
    let booking = Booking("Aino", 18)
    print("{booking.guest} at {booking.hour}")
```

<!-- output: examples/reference-diagnostics/w0029_preview_plain_fixed.out -->
```text
Aino at 18
```

Shown in [R14](attributes.md#preview-features).

### W0030

`resolve.unchecked-attribute`: `@{name}` is accepted and checked by
nothing yet, so nothing ensures {what}.

`@no_alloc`, `@no_block` and `@stack(...)` on a function promise what
the compiler is to check ([R14](attributes.md#resource-limits)), and
no check of them exists yet, so each warns at its name that the promise
is the program's own to keep.

<!-- file: examples/reference-diagnostics/w0030_unchecked_attribute.sisu -->
```sisu
# A promise that the function allocates nothing, which nothing checks.
@no_alloc
def total(readings: Array[int, 3]) -> int:
    var sum = 0
    for i in range(3):
        sum += readings[i]
    return sum

def main():
    let readings: Array[int, 3] = Array.filled(80)
    print(total(readings))
```

<!-- output: examples/reference-diagnostics/w0030_unchecked_attribute.console -->
```console
$ sisu run w0030_unchecked_attribute.sisu
warning[W0030]: `@no_alloc` is accepted and checked by nothing yet, so nothing ensures that the function does not allocate
  --> w0030_unchecked_attribute.sisu:2:2
  |
2 | @no_alloc
  |  ^^^^^^^^

240
```

Keep the attribute if you want the check when it comes, and the warning
with it until then; otherwise state the promise in the function's
comment.

<!-- file: examples/reference-diagnostics/w0030_unchecked_attribute_fixed.sisu -->
```sisu
# The promise kept in the function's comment until a check exists.

## The sum of `readings`, which allocates nothing.
def total(readings: Array[int, 3]) -> int:
    var sum = 0
    for i in range(3):
        sum += readings[i]
    return sum

def main():
    let readings: Array[int, 3] = Array.filled(80)
    print(total(readings))
```

<!-- output: examples/reference-diagnostics/w0030_unchecked_attribute_fixed.out -->
```text
240
```

Shown in [R14](attributes.md#resource-limits) and the tutorial's
[chapter 27](../tutorial/27-systems.md#exercise-273-an-audio-callback-without-allocation).

### W0031

`resolve.mixed-script`: `{name}` mixes letters of the {scripts}
scripts.

A declared name holds letters of more than one script, as a name typed
or pasted with a Cyrillic `а` (`\u{430}`) among Latin letters does
([R1](lexical-structure.md#names)). Digits, `_` and letters of no
script do not count, and the scripts of Chinese, Japanese and Korean
count as one.

<!-- file: examples/reference-diagnostics/w0031_mixed_script.sisu -->
```sisu
# A name typed with a Cyrillic `а` among its Latin letters.
def main():
    let sаuna_heat = 80
    print(sаuna_heat)
```

<!-- output: examples/reference-diagnostics/w0031_mixed_script.console -->
```console
$ sisu run w0031_mixed_script.sisu
warning[W0031]: `sаuna_heat` mixes letters of the Latin and Cyrillic scripts
  --> w0031_mixed_script.sisu:3:9
  |
3 |     let sаuna_heat = 80
  |         ^^^^^^^^^^

80
```

Retype the name in one script. The two names look the same, so the
warning is often the only sign of the stray letter.

<!-- file: examples/reference-diagnostics/w0031_mixed_script_fixed.sisu -->
```sisu
# The name in Latin letters only.
def main():
    let sauna_heat = 80
    print(sauna_heat)
```

<!-- output: examples/reference-diagnostics/w0031_mixed_script_fixed.out -->
```text
80
```

Shown in [R1](lexical-structure.md#names).

### W0032

`resolve.confusable`: `{name}` looks like `{other}`, another name in
scope.

A declared name reads as another name in scope, a module's item or a
local or parameter of an enclosing scope: the two differ, one of them
holds a letter beyond ASCII, and written with the ASCII letters they
look like, they are the same. When the other name is the module's, the
label `resolve.confusable-here` stands at its declaration. A name that
also mixes scripts gets W0031 as well:

<!-- file: examples/reference-diagnostics/w0032_confusable.sisu -->
```sisu
# Two names that look the same: the second has a Greek `ο`.
def main():
    let total = 4
    let tοtal = total + 1
    print(tοtal)
```

<!-- output: examples/reference-diagnostics/w0032_confusable.console -->
```console
$ sisu run w0032_confusable.sisu
warning[W0031]: `tοtal` mixes letters of the Latin and Greek scripts
  --> w0032_confusable.sisu:4:9
  |
4 |     let tοtal = total + 1
  |         ^^^^^

warning[W0032]: `tοtal` looks like `total`, another name in scope
  --> w0032_confusable.sisu:4:9
  |
4 |     let tοtal = total + 1
  |         ^^^^^

5
```

Give the new value a name of its own, which a reader can tell apart:

<!-- file: examples/reference-diagnostics/w0032_confusable_fixed.sisu -->
```sisu
# The new value gets a name of its own.
def main():
    let total = 4
    let with_guest = total + 1
    print(with_guest)
```

<!-- output: examples/reference-diagnostics/w0032_confusable_fixed.out -->
```text
5
```

Shown in [R1](lexical-structure.md#names).

### W0033

`type.fix-inline-body`: `@fix_inline` rewrites a call of `{name}` with
the expression it returns, so its body is one `return`, and its
parameters take no `mut` or `own`: its calls warn without the rewrite.

`@fix_inline` gives each call of a deprecated function a fix that writes
what it returns in the call's place
([R14](attributes.md#deprecation-and-migrations)), which needs a body
that is one `return` of an expression, of a free function whose
parameters are plain borrows. Another body, a method, or a parameter
with `mut` or `own` gets the warning at the attribute, and each call
warns (W0023) without the fix:

<!-- file: examples/reference-diagnostics/w0033_fix_inline_body.sisu -->
```sisu
# A `@fix_inline` function whose body is more than one `return`.

## Degrees Celsius of `fahrenheit` degrees.
def celsius_of(fahrenheit: f64) -> f64:
    return (fahrenheit - 32.0) * 5.0 / 9.0

## The old name of `celsius_of`.
@deprecated("use `celsius_of`")
@fix_inline
def to_celsius(fahrenheit: f64) -> f64:
    let celsius = celsius_of(fahrenheit)
    return celsius

def main():
    print(to_celsius(212.0))
```

<!-- output: examples/reference-diagnostics/w0033_fix_inline_body.console -->
```console
$ sisu run w0033_fix_inline_body.sisu
warning[W0033]: `@fix_inline` rewrites a call of `to_celsius` with the expression it returns, so its body is one `return`, and its parameters take no `mut` or `own`: its calls warn without the rewrite
  --> w0033_fix_inline_body.sisu:9:2
  |
9 | @fix_inline
  |  ^^^^^^^^^^

warning[W0023]: `to_celsius` is deprecated: use `celsius_of`
  --> w0033_fix_inline_body.sisu:15:11
   |
15 |     print(to_celsius(212.0))
   |           ^^^^^^^^^^^^^^^^^

100.0
```

Write the body as one `return`, and the call's warning gets its fix,
which `sisu fix` applies ([R16](sisu-command.md#sisu-fix)):

<!-- file: examples/reference-diagnostics/w0033_fix_inline_body_fixed.sisu -->
```sisu
# The body is one `return`, which each call can be rewritten to.

## Degrees Celsius of `fahrenheit` degrees.
def celsius_of(fahrenheit: f64) -> f64:
    return (fahrenheit - 32.0) * 5.0 / 9.0

## The old name of `celsius_of`.
@deprecated("use `celsius_of`")
@fix_inline
def to_celsius(fahrenheit: f64) -> f64:
    return celsius_of(fahrenheit)

def main():
    print(to_celsius(212.0))
```

<!-- output: examples/reference-diagnostics/w0033_fix_inline_body_fixed.console -->
```console
$ sisu run w0033_fix_inline_body_fixed.sisu
warning[W0023]: `to_celsius` is deprecated: use `celsius_of`
  --> w0033_fix_inline_body_fixed.sisu:14:11
   |
14 |     print(to_celsius(212.0))
   |           ^^^^^^^^^^^^^^^^^
  = fix: write what `to_celsius` returns in the call's place: `celsius_of(212.0)`

100.0
```

Shown in [chapter 28](../tutorial/28-code-that-lasts.md#letting-the-compiler-rewrite-the-callers).

## Run time

Codes R0001 to R0160. Messages of a running program: panics, errors that
`main` returns, and the failures that `sisu test` reports. They are
printed without their code, as `panic: ...` or `error: ...`, followed by
the trace.

### R0001

`panic.explicit`: {message}.

`panic(message)` stops the task with the message, and so does
`expect(message)` on an absent value. A panic is for a bug, a state the
program should never reach; an input that can be wrong is an error or an
absent value instead.

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

Indexing a dict with a key it does not hold panics, naming the key,
at the index itself (`4:11`, `ages["Roni"]` inside the call of
`print`), whatever the type of the key.

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
  at main (r0010_missing_key.sisu:4:11)
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

A run that reaches a typed hole prints the hole's closure, the bindings
in scope there ([R0160](#r0160)), and panics with the hole's name; the
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

in scope at the typed hole `todo("double")`:
  x = 21
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

### R0080

`run.deadlock`: deadlock: every task is waiting, and no timer can wake
any of them.

When no task can run and no timer is set, the program can never go on,
so it stops with this message, a line for each task saying what it
waits for and where, and status 134.

<!-- file: examples/reference-diagnostics/r0080_deadlock.sisu -->
```sisu
# A task waits for a message that is sent only after the scope ends.
def main():
    let tx, rx = channel[String](capacity=1)
    task.scope do s:
        s.spawn do:
            print(rx.recv())
    _ = tx.send("löyly".to_string())
```

<!-- output: examples/reference-diagnostics/r0080_deadlock.console -->
```console
$ sisu run r0080_deadlock.sisu
deadlock: every task is waiting, and no timer can wake any of them
  the root task waits for the end of a scope at r0080_deadlock.sisu:4:5
  task 1 waits for recv at r0080_deadlock.sisu:6:19
```

Here the root task waits for the scope, and the task in it waits for a
message that the root sends only after the scope. Send it from a task
inside the scope.

<!-- file: examples/reference-diagnostics/r0080_deadlock_fixed.sisu -->
```sisu
# The message is sent inside the scope, by a task of its own.
def main():
    let tx, rx = channel[String](capacity=1)
    task.scope do s:
        s.spawn do:
            print(rx.recv())
        s.spawn do:
            _ = tx.send("löyly".to_string())
```

<!-- output: examples/reference-diagnostics/r0080_deadlock_fixed.out -->
```text
Ok("löyly")
```

Shown in [R9](concurrency.md#the-scheduler).

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

### R0102

`contract.invariant-failed`: invariant of `{ty}` failed: {clause}.

A struct's invariant is checked after its construction and after each
`mut self` method returns; the panic names the type and the clause,
shows its values, and traces from the clause through the method.

<!-- file: examples/reference-diagnostics/r0102_invariant_failed.sisu -->
```sisu
# A method that leaves the invariant broken.
struct Gauge:
    level: int
    invariant 0 <= self.level <= 10

    def fill(mut self, amount: int):
        self.level += amount

def main():
    var gauge = Gauge(level=4)
    gauge.fill(9)
    print(gauge.level)
```

<!-- output: examples/reference-diagnostics/r0102_invariant_failed.console -->
```console
$ sisu run r0102_invariant_failed.sisu
panic: invariant of `Gauge` failed: 0 <= self.level <= 10
    0 <= self.level <= 10
         │
         13
  at the invariant of Gauge (r0102_invariant_failed.sisu:4:15)
  at Gauge.fill (r0102_invariant_failed.sisu:7:29)
  at main (r0102_invariant_failed.sisu:11:5)
```

Make the method keep the invariant, here by filling to the top at most.

<!-- file: examples/reference-diagnostics/r0102_invariant_failed_fixed.sisu -->
```sisu
# The method keeps the invariant: it fills to the top at most.
struct Gauge:
    level: int
    invariant 0 <= self.level <= 10

    def fill(mut self, amount: int):
        self.level = min(self.level + amount, 10)

def main():
    var gauge = Gauge(level=4)
    gauge.fill(9)
    print(gauge.level)
```

<!-- output: examples/reference-diagnostics/r0102_invariant_failed_fixed.out -->
```text
10
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### R0108

`test.snapshot-missing`: there is no snapshot `{name}` yet (`sisu test
--update` stores this one): {value}.

A snapshot test ran `assert_snapshot` for a value that has no stored
snapshot yet. The test fails and shows the value it would store.

Read the value, and if it is right, run `sisu test --update`, which
stores it in `NAME.snap` beside the program `NAME.sisu`.

Example: [R16](sisu-command.md#sisu-test).

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

### R0154

`run.expect-failed`: expectation failed: {expr}.

Not a panic: the report of a failed `expect` in a test, which records
the failure and lets the test go on to its end, where it fails with
every failure it recorded ([R15](testing.md#assertions)). The lines
under it are those of a failed `assert`: the two sides of a comparison
and the place. R0155 is the same report with the line `message:` of an
`expect` that has one. Outside a test a failed `expect` is `assert`'s
panic.

Example: [R15](testing.md#assertions) and
[chapter 19](../tutorial/19-testing.md#what-a-failing-assertion-shows).

### R0160

`panic.hole-closure`: in scope at the typed hole {hole}:

A run that reaches a typed hole prints the hole's closure before it
stops ([R0013](#r0013)): this line, then a line `NAME = VALUE` for each
binding in scope there, in the order the function declares them, each
value in its `Debug` form ([17](../../proposal/17-testing.md#assertions)).
A test blocked by a hole prints the same closure, which `sisu test`
shows in the blocked test's block after the list, under `what the test
printed:` ([R15](testing.md#what-a-test-prints)). A binding is left out when it
does not hold its whole value at the hole (it was moved away, as
`names` is here), when it is a local that holds a borrow or a view (as
`greeting`, a `str`, does), when its type writes its own `Debug` or is a
type parameter, and when it holds a capability.

<!-- file: examples/reference-diagnostics/r0160_closure.sisu -->
```sisu
# A run that reaches a hole prints the bindings in scope there.
def plan(own names: list[String], hour: int) -> String:
    let turns = names
    let last = hour + len(turns) - 1
    let greeting = "Hei"
    return todo("the evening's plan")

def main():
    print(plan(["Aino", "Eero"], 18))
```

<!-- output: examples/reference-diagnostics/r0160_closure.console -->
```console
$ sisu run r0160_closure.sisu
warning[W0001]: typed hole `the evening's plan` of type `String`
  --> r0160_closure.sisu:6:12
  |
6 |     return todo("the evening's plan")
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("the evening's plan")`:
  hour = 18
  turns = ["Aino", "Eero"]
  last = 19
panic: reached the typed hole `todo("the evening's plan")`
  at plan (r0160_closure.sisu:6:12)
  at main (r0160_closure.sisu:9:11)
```

The closure is what the code that fills the hole has to work with: here
two turns, from 18 to 19.

<!-- file: examples/reference-diagnostics/r0160_closure_fixed.sisu -->
```sisu
# The hole filled with what its closure showed was there.
def plan(own names: list[String], hour: int) -> String:
    let turns = names
    let last = hour + len(turns) - 1
    let greeting = "Hei"
    return "{greeting}: {len(turns)} turns, from {hour} to {last}"

def main():
    print(plan(["Aino", "Eero"], 18))
```

<!-- output: examples/reference-diagnostics/r0160_closure_fixed.out -->
```text
Hei: 2 turns, from 18 to 19
```

Shown in [chapter 3](../tutorial/03-functions.md#leaving-a-hole).

## The driver

Codes D0001 to D0190. The messages of the `sisu` command itself
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

The command needs a file that the command line does not give: a file
for `sisu ws`, `sisu tokens` or `sisu parse`, a log for `sisu replay`.
`sisu explain` without a code is D0129, and `sisu new` without a name
D0130.

Give the file. The message says "a file" for `sisu replay` too, whose
file is a capability log.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0006

`cli.errors-found`: the program was not run: errors found ({count}).

The program had errors, so the command did not run it (or test or
document it). It follows the diagnostics and counts the errors; warnings
do not stop a run. A build that stops at errors ends with D0132
instead.

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

### D0028

`cli.bad-contract-level`: `--contracts` takes `off`, `preconditions` or
`all`, not `{value}`.

The level of a run names which clauses it checks
([R11](contracts.md#check-levels)).

<!-- file: examples/reference-diagnostics/d0028_bad_level.sisu -->
```sisu
# A contract, checked at the level a run names.
def half(n: int) -> int:
    requires n >= 0
    return n // 2

def main():
    print(half(8))
```

<!-- output: examples/reference-diagnostics/d0028_bad_level.console -->
```console
$ sisu run --contracts=most d0028_bad_level.sisu
error[D0028]: `--contracts` takes `off`, `preconditions` or `all`, not `most`
```

<!-- output: examples/reference-diagnostics/d0028_bad_level_fixed.console -->
```console
$ sisu run --contracts=preconditions d0028_bad_level.sisu
4
```

Shown in [R11](contracts.md#check-levels).

### D0029

`cli.test-checks-contracts`: `sisu test` checks every contract;
`--contracts` sets what `sisu run` and `sisu sir` check.

`sisu test` checks every contract and adds a test for each function's
contract, so it takes no `--contracts`; the level is for `sisu run` and
`sisu sir`, which build the program a user runs
([R11](contracts.md#check-levels)).

<!-- file: examples/reference-diagnostics/d0029_test_contracts.sisu -->
```sisu
# A contract, and a test of the function it guards.
def half(n: int) -> int:
    requires n >= 0
    return n // 2

test "half of eight":
    assert half(8) == 4

def main():
    print(half(8))
```

<!-- output: examples/reference-diagnostics/d0029_test_contracts.console -->
```console
$ sisu test --contracts=all d0029_test_contracts.sisu
error[D0029]: `sisu test` checks every contract; `--contracts` sets what `sisu run` and `sisu sir` check
```

Leave `--contracts` out of `sisu test`, and give it to `sisu run` for
the level the run needs.

<!-- output: examples/reference-diagnostics/d0029_test_contracts_fixed.console -->
```console
$ sisu test d0029_test_contracts.sisu
test "half of eight" ... ok
test "contract of half" ... ok
2 passed, 0 failed, 0 blocked
$ sisu run --contracts=off d0029_test_contracts.sisu
4
```

Shown in [chapter 23](../tutorial/23-contracts.md).

### D0033

`cli.token-other-code`: the token is for other code (content hash
{expected}); `{file}` has content hash {found}.

A replay token names the content hash of the code it was made for, and
replays a run of exactly that code: the token is a fact about one
version of the program ([R15](testing.md#replay-tokens)). After any
change to the file, the hash differs, so the token is refused rather
than replaying something else. A token that another run of the same package made,
of one module's tests or of the package's, is [D0149](#d0149) or
[D0150](#d0150) instead, which names that run.

<!-- file: examples/reference-diagnostics/d0033_other_code.sisu -->
```sisu
# A test that fails, and prints the token that replays it.
def total(prices: list[int]) -> int:
    var sum = 0
    for p in prices[1:]:
        sum += p
    return sum

test "the total of three prices":
    assert total([3, 4, 5]) == 12
```

<!-- output: examples/reference-diagnostics/d0033_other_code.console -->
```console
$ sisu test d0033_other_code.sisu
test "the total of three prices" ... FAILED

test "the total of three prices" ... FAILED
panic: assertion failed: total([3, 4, 5]) == 12
  left:  9
  right: 12
  at test "the total of three prices" (d0033_other_code.sisu:9:5)
replay: sisu test --replay 1-c239940ba84d571f-0.0.1-t0-s0-r0 d0033_other_code.sisu

0 passed, 1 failed, 0 blocked
$ sisu test --replay 1-c239940ba84d571f-0.0.1-t0-s0-r0 d0033_other_code_fixed.sisu
error[D0033]: the token is for other code (content hash c239940ba84d571f); `d0033_other_code_fixed.sisu` has content hash 45b5b32410fd660b
```

Replay the token against the code it was made for, to see the failure
again, and run the tests anew on the changed code, which makes tokens of
its own.

<!-- file: examples/reference-diagnostics/d0033_other_code_fixed.sisu -->
```sisu
# The function fixed, so the token's code is gone.
def total(prices: list[int]) -> int:
    var sum = 0
    for p in prices:
        sum += p
    return sum

test "the total of three prices":
    assert total([3, 4, 5]) == 12
```

<!-- output: examples/reference-diagnostics/d0033_other_code_fixed.console -->
```console
$ sisu test --replay 1-c239940ba84d571f-0.0.1-t0-s0-r0 d0033_other_code.sisu
test "the total of three prices" ... FAILED

test "the total of three prices" ... FAILED
panic: assertion failed: total([3, 4, 5]) == 12
  left:  9
  right: 12
  at test "the total of three prices" (d0033_other_code.sisu:9:5)
replay: sisu test --replay 1-c239940ba84d571f-0.0.1-t0-s0-r0 d0033_other_code.sisu

0 passed, 1 failed, 0 blocked
$ sisu test d0033_other_code_fixed.sisu
test "the total of three prices" ... ok
1 passed, 0 failed, 0 blocked
```

Shown in the guide [Reproduce a failure](../guides/replay.md).

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

### D0103

`native.unsupported`: the native backend does not compile {what} yet
(in `{function}`); `sisu run` without `--native` runs the program.

The program is valid, and the interpreter runs it, but the native
backend has no lowering yet for a part of it, which the message names,
with the function it is in: a `@repr(c)` struct passed by value to an
exported function, "a `Reading` by value (a parameter of the exported
function `sauna_load`)", or a raw pointer to an option of a raw
pointer, "`mem.ptr.of` of a `Option[ptr[u8]]`". A native build lays a
declared struct's fields and a tuple's parts out at C's offsets and a
number at its width, but gives an option of a raw pointer a tag word of
its own where C has one nullable word, so a pointer to such an option
would not see what C sees (D-086).

Run the program without `--native`, or change it to what the backend
compiles: pass a struct to an exported function by a pointer, and hand
C an optional pointer through a foreign call's own `ptr[T]?` parameter,
as [R14](attributes.md#exported-functions) shows.

Example: [R14](attributes.md#exported-functions).

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

### D0127

`cli.explore-differs`: schedule seed {seed} of {count} printed other
output than seed {first}, with the same exit status {status};
`--schedule-seed {seed}` runs it again.

`sisu run --explore N` runs the program under `N` schedule seeds and
compares each run with the first, in its standard output and its exit
status ([R16](sisu-command.md#choosing-the-schedule)). A run that exits
as the first did but prints something else is reported too, since its
output depends on the order in which the tasks ran. Standard error is
not compared. The status is 1.

<!-- file: examples/reference-diagnostics/d0127_explore_differs.sisu -->
```sisu
# The order of the names depends on the schedule.
def arrive(order: Mutex[list[String]], name: str):
    task.yield_now()
    order.lock().push(name.to_string())

def main():
    let order = Mutex(list[String]())
    task.scope do s:
        s.spawn(() -> arrive(order, "Aino"))
        s.spawn(() -> arrive(order, "Eero"))
    print(order.into_inner())
```

<!-- output: examples/reference-diagnostics/d0127_explore_differs.console -->
```console
$ sisu run --explore 5 d0127_explore_differs.sisu
["Aino", "Eero"]
schedule seed 1 of 5 printed other output than seed 0, with the same exit status 0; `--schedule-seed 1` runs it again
```

The output is the reported seed's, seed 1's; seed 0 printed the names
the other way round, as `--schedule-seed 0` shows.
Make the output independent of the schedule, here by sorting the names,
or make the order part of the program, by joining the tasks in turn:

<!-- file: examples/reference-diagnostics/d0127_explore_differs_fixed.sisu -->
```sisu
# Sorted, the names print the same under every schedule.
def arrive(order: Mutex[list[String]], name: str):
    task.yield_now()
    order.lock().push(name.to_string())

def main():
    let order = Mutex(list[String]())
    task.scope do s:
        s.spawn(() -> arrive(order, "Aino"))
        s.spawn(() -> arrive(order, "Eero"))
    var arrived = order.into_inner()
    arrived.sort()
    print(arrived)
```

<!-- output: examples/reference-diagnostics/d0127_explore_differs_fixed.console -->
```console
$ sisu run --explore 5 d0127_explore_differs_fixed.sisu
["Aino", "Eero"]
explored 5 schedules (seeds 0 to 4): every run exited with status 0
```

[R16](sisu-command.md#choosing-the-schedule) describes it.

### D0128

`cli.transcript-front-end`: a transcript runs on the Rust front end's
build of the program, which stops at errors stage 1 does not report:
{diagnostics}.

A program's transcripts run from the SIR that its tests run from,
which stage 1 writes with the world a transcript runs in
([R15](testing.md#transcripts)). They run on the Rust front end's build
instead with a stage 1 older than 0.11.0, or when the environment
variable `SISU_STAGE1_TRANSCRIPTS` is `0`, and then D0128 is what a
program gets that writes a form the frozen Rust front end lacks, such as
`^` in a slice's bound: it passes its check and its tests, and each of
its transcripts fails with the Rust front end's diagnostics. The status
is 1. With this compiler the program's transcript passes, so the
session shows no D0128:

<!-- file: examples/reference-diagnostics/d0128_transcript_front_end.sisu -->
```sisu
# A program with a transcript, and a slice that only stage 1 reads.
def main(sys: System):
    let words = sys.env.args()
    print(words[1:^1])
```

<!-- file: examples/reference-diagnostics/d0128_transcript_front_end.files/tests/middle.transcript -->
```text
$ middle sauna kiuas löyly
["kiuas"]
```

<!-- output: examples/reference-diagnostics/d0128_transcript_front_end.console -->
```console
$ sisu test d0128_transcript_front_end.sisu
transcript "tests/middle.transcript" ... ok
1 passed, 0 failed, 0 blocked
```

Where D0128 still comes, write the forms that both front ends read in
a program that has transcripts: here the slice's end as
`len(words) - 1`.

<!-- file: examples/reference-diagnostics/d0128_transcript_front_end_fixed.sisu -->
```sisu
# The slice written with `len`, which both front ends read.
def main(sys: System):
    let words = sys.env.args()
    print(words[1:len(words) - 1])
```

<!-- output: examples/reference-diagnostics/d0128_transcript_front_end_fixed.console -->
```console
$ sisu test d0128_transcript_front_end_fixed.sisu
transcript "tests/middle.transcript" ... ok
1 passed, 0 failed, 0 blocked
```

[R15](testing.md#transcripts) describes it.

### D0129

`cli.missing-code`: `sisu explain` needs a diagnostic's code, such as
`E0300`, or its name, such as `type.mismatch`.

`sisu explain` was given nothing to explain. The status is 2.

Give the code that the diagnostic shows, or its identifier.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0130

`cli.missing-name`: `sisu new` needs the name of the package to create.

`sisu new` was given no name. The status is 2.

Give the package's name, which is also the name of its new directory:
`sisu new kala`.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0131

`cli.version`: sisu {version}.

Not an error: the line that `sisu --version` and `sisu version` print,
the toolchain's version, which replay tokens and capability logs record
too. The status is 0.

Example: [R16](sisu-command.md#commands-options-and-arguments).

### D0132

`cli.not-compiled`: the program was not compiled: errors found
({count}).

A build of a program with errors, `sisu build` with or without
`--native`, and `sisu sir` of a program it has no SIR of, stopped: it
follows the diagnostics and counts the errors, the typed holes of a
release build (E0445) and the ownership checker's errors among them.
`sisu run` and `sisu test` of the same program end with D0006, or
D0021, since what they did not do is run it. The status is 1.

Fix the errors above it.

Example: [R16](sisu-command.md#sisu-build), and
[E0445](#e0445) for a release build's holes.

### D0134

`native.test-unsupported`: `sisu test --native` does not run {what}
yet; `sisu test` without `--native` runs it.

`sisu test --native` was asked for what only the interpreter's run of
the tests does: a test parameter that no fake stands for, or a
simulated or faked `System` that the program's build does not keep.
The schedulers, injected faults and `sim.always` run natively
([R15](testing.md#native-tests)). The line goes to standard error and
the status is 1.

Run those tests without `--native`.

### D0135

`cli.test-output`: what the test printed:.

Not an error: the line in a failing test's block, after its header,
under which `sisu test` shows what the test wrote to standard output
and standard error, each line indented, and in a blocked test's block,
under which its hole's closure comes. A test that passes prints
nothing.

Example: [R15](testing.md#what-a-test-prints).

### D0136

`cli.bench-samples`: `--samples` takes 2 or more, not {value}: the
interval of the mean needs two samples.

`sisu bench --samples K` was given a `K` below 2, from which no
interval of the mean can be computed
([R16](sisu-command.md#sisu-bench)). The status is 2.

Give 2 or more; the default is 10.

Example: [G27](../guides/bench.md#2-run-the-benchmarks-and-compare).

### D0137

`cli.test-program`: tests of `{program}`:.

`sisu test` ran more than one program, the members of a workspace or a
package and the files of its `tests/`, and this line names the program
whose list follows it: a package or a member by its name, a file of
`tests/` by its path ([R15](testing.md#black-box-tests)). It is no
error: the status is the worst of the programs'. A run of one program
has no such line.

Example: [R15](testing.md#black-box-tests), [chapter
19](../tutorial/19-testing.md#suites-tags-and-choosing-tests).

### D0138

`cli.replay-no-token-sim`: no replay token: the test takes the real
`System`, which `--sim` does not simulate and a replay cannot stand in
for; a test that names the capabilities it uses gets simulated ones.

A test that takes the real `System` failed under `sisu test --sim`, and
this line stands where its replay token would be. Such a test gets the
real `System` under `--sim` too, runs alone and meets no fault, since
the real world can be neither simulated nor replayed; outside `--sim`
the same failure prints D0031. It is no error of its own: the status is
the run's.

Name the capabilities the test uses, `test "..."(fs: Fs):`, and it
runs on simulated ones, which faults reach and a token replays.

Example: [R15](testing.md#simulation).

### D0148

`cli.explain-size-native`: `--explain-size` explains the size of a
native executable: it goes with `--native`.

`sisu build --explain-size` without `--native` builds no executable, so
there is nothing whose size it could explain; the status is 2. With
`--native`, the build prints the executable's size (D0139) and its
seven parts (D0140 to D0146) after the `built` line, and D0147 says
that the file it built is not a 64-bit ELF executable, the one form the
driver reads.

Add `--native`: `sisu build --native --explain-size FILE`.

Example: [R16](sisu-command.md#sisu-build).

### D0149

`cli.token-module-run`: the token is for the run of the tests of
`{module}`; `sisu test --replay {token} {module}` replays it.

`sisu test FILE` of a module under a package's `src/` numbers the
module's tests apart from the package's, so its replay tokens are for
that run alone ([R15](testing.md#the-tests-of-one-module)). The token
was given to the run of the whole package, or of another module, and
the message names the module whose run made it and the command that
replays it. Nothing runs, and the status is 1.

Run the command the message gives, which names the module's file after
the token.

Example: [R15](testing.md#the-tests-of-one-module).

### D0150

`cli.token-package-run`: the token is for the run of the package's
tests, not of `{module}`'s; `sisu test --replay {token} {package}`
replays it.

The token was made by the run of the package's tests, `sisu test` in
its directory, and was given to the run of one module's tests, `sisu
test --replay TOKEN src/MODULE.sisu`. The message names the package's
directory to replay it in, `.` for the one you are in.

Leave the module's file out, or give the package's directory, as the
message's command does.

Example: [R15](testing.md#the-tests-of-one-module).

### D0151

`cli.command-usage`: usage: sisu {form}, and on the lines after it what
the command does.

A command whose words stage 1 reads, such as `sisu eval`, `sisu fix` or
`sisu check --holes`, was given no file where it needs one, more than
one, or a value it does not take, such as `--to xml`. The command prints its own line of
`sisu help` and nothing else, and the status is 2.

Give the command what its usage line names: `sisu eval FILE`.

Example: [R16](sisu-command.md#sisu-eval).

### D0152

`cli.test-cached`: ok (cached).

`sisu test` took the test's result from its cache, since nothing the
test reaches has changed since it passed, and did not run it
([R16](sisu-command.md#sisu-test)). It is no error: the test counts as
passed and the status is a passed test's. `--no-cache` runs it again,
and prints `ok`.

Example: [chapter 19](../tutorial/19-testing.md#what-sisu-test-remembers).

### D0190

`cli.fix-errors-left`: `{file}` does not check after the fixes: errors
that no fix applies to are left ({count}), which `sisu check` shows.

`sisu fix` applied every fix it could, wrote the file, and checked it
again, and errors without a fix are left. The status is 1, as for
`sisu check` at an error, so a script that runs `sisu fix` can tell a
program it mended from one that still does not compile; warnings left
do not count:

<!-- file: examples/reference-diagnostics/d0190_fix_errors_left.sisu -->
```sisu
# One mistake with a fix, and one that no fix can mend.
def heated(celsius: int) -> int:
    return celsius + 10

def main():
    let start = 70
    print(heated(strat))
    print(heated(start, 2))
```

<!-- output: examples/reference-diagnostics/d0190_fix_errors_left.console -->
```console
$ sisu fix d0190_fix_errors_left.sisu
d0190_fix_errors_left.sisu:7:18: E0200 fixed: a similar name is in scope: `start`
fixes applied to `d0190_fix_errors_left.sisu`: 1
error[D0190]: `d0190_fix_errors_left.sisu` does not check after the fixes: errors that no fix applies to are left (1), which `sisu check` shows
$ sisu check d0190_fix_errors_left.sisu
error[E0305]: wrong number of arguments to `heated`: expected 1, found 2
  --> d0190_fix_errors_left.sisu:8:11
  |
8 |     print(heated(start, 2))
  |           ^^^^^^^^^^^^^^^^

```

Mend what is left by hand, from what `sisu check` shows; here the call
with one argument too many:

<!-- file: examples/reference-diagnostics/d0190_fix_errors_left_fixed.sisu -->
```sisu
# The call with too many arguments mended by hand, and the name fixed.
def heated(celsius: int) -> int:
    return celsius + 10

def main():
    let start = 70
    print(heated(start))
    print(heated(start) + 2)
```

<!-- output: examples/reference-diagnostics/d0190_fix_errors_left_fixed.out -->
```text
80
82
```

Example: [R16](sisu-command.md#sisu-fix).

## Every code

The tables list every code of the catalog in this version, by range, with
its identifier and its message; `{name}` stands for a part of the message
that the diagnostic fills in. A code with an entry on this page links to
it. `sisu explain CODE` prints the same line.

### Lexing, every code

| Code | Identifier | Message |
|---|---|---|
| [E0001](#e0001) | `lex.tab-in-indentation` | a tab in indentation; Sisu indents with spaces only |
| [E0002](#e0002) | `lex.inconsistent-dedent` | this line's indentation matches no enclosing block; the enclosing blocks start at column {columns} |
| [E0003](#e0003) | `lex.unterminated-string` | this string literal is not closed before the end of the line |
| E0004 | `lex.unterminated-block-string` | this multi-line string literal is never closed |
| [E0005](#e0005) | `lex.unknown-character` | `{char}` cannot appear in Sisu source outside a string or comment |
| E0006 | `lex.unterminated-char` | this character literal is not closed |
| E0007 | `lex.invalid-escape` | `\{escape}` is not an escape sequence; write `\\` for a backslash |
| E0008 | `lex.unmatched-close` | `{close}` closes nothing |
| E0009 | `lex.unclosed-bracket` | `{open}` is never closed |
| E0010 | `lex.mismatched-bracket` | `{close}` does not close the `{open}` before it |
| [E0011](#e0011) | `lex.semicolon` | Sisu has no semicolons; put each statement on its own line |
| [E0012](#e0012) | `lex.invalid-number` | `{text}` is not a valid number literal |
| E0013 | `lex.lone-close-brace` | a single `}` in a string closes nothing; write `}}` for a literal brace |
| E0014 | `lex.bad-indent-char` | `{char}` in indentation; Sisu indents with ASCII spaces only |
| [E0015](#e0015) | `lex.backslash-continuation` | Sisu has no backslash continuation; break the line inside brackets, after a comma or before an operator |
| [E0016](#e0016) | `lex.bad-char-literal` | a character literal holds exactly one character |
| E0018 | `lex.newline-in-interpolation` | an interpolation in a one-line string must end on the same line |

### Parsing, every code

| Code | Identifier | Message |
|---|---|---|
| [E0100](#e0100) | `parse.expected` | expected {expected}, found {found} |
| E0101 | `parse.expected-block` | expected an indented block after `{keyword}` on line {line} |
| [E0102](#e0102) | `parse.block-on-same-line` | a block starts on the next line, indented; nothing may follow `:` on the line that opens it |
| [E0103](#e0103) | `parse.expected-expression` | expected an expression, found {found} |
| E0104 | `parse.expected-type` | expected a type, found {found} |
| [E0105](#e0105) | `parse.expected-pattern` | expected a pattern, found {found} |
| [E0106](#e0106) | `parse.expected-declaration` | expected a declaration, found {found} |
| [E0107](#e0107) | `parse.top-level-statement` | statements cannot appear at the top level of a module; put them in a function such as `main` |
| [E0108](#e0108) | `parse.mixed-and-or` | `and` and `or` cannot be mixed without parentheses |
| E0109 | `parse.mixed-bitwise` | `{first}` and `{second}` cannot be mixed without parentheses |
| [E0110](#e0110) | `parse.bitwise-arithmetic` | `{op}` cannot be mixed with arithmetic or shifts without parentheses |
| E0111 | `parse.shift-arithmetic` | a shift cannot be mixed with arithmetic without parentheses |
| [E0112](#e0112) | `parse.negative-power` | `-x ** y` is ambiguous; write `-(x ** y)` or `(-x) ** y` |
| [E0113](#e0113) | `parse.mixed-chain` | a chained comparison must point one way; `{first}` and `{second}` do not |
| E0114 | `parse.assignment-in-expression` | assignment is a statement and has no value; to compare, write `==` |
| [E0115](#e0115) | `parse.python-conditional` | `a if c else b` is not Sisu; write `if c: a else: b` |
| [E0116](#e0116) | `parse.if-expression-else` | an `if` expression needs an `else` branch, because it must have a value |
| [E0117](#e0117) | `parse.trailing-tokens` | unexpected {found} after the end of the statement |
| E0118 | `parse.arm-statement` | a `case` arm on one line holds a single expression; put statements in an indented block |
| E0119 | `parse.chain-operator` | `{op}` cannot be chained; use parentheses or `and` |
| E0121 | `parse.empty-interpolation` | an interpolation needs an expression between `{` and `}` |
| E0122 | `parse.closure-parameter` | a closure parameter must be a name |
| [E0123](#e0123) | `parse.walrus` | Sisu has no `:=`; bind with `let` or use `if let` |
| [E0124](#e0124) | `parse.keyword-as-name` | `{keyword}` is a keyword and cannot be used as a name here |
| E0125 | `parse.unexpected-indent` | this line is indented more than the line before it, which does not open a block |
| E0126 | `parse.expected-member` | a struct body holds fields, methods and `pass`; this line is none of them |
| [E0127](#e0127) | `parse.loop-else` | a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:` |
| [E0128](#e0128) | `parse.glob-import` | Sisu has no glob imports: name what the module gives, as in `from std.collections import Deque, Heap`, or import the module and write `collections.Deque` |
| [E0129](#e0129) | `parse.attribute-on-statement` | `@{name}` stands before a statement; an attribute stands on the line before a declaration (a function, a type, a constant, a test), and none applies to a statement |
| [E0130](#e0130) | `parse.visibility-scope` | `pub({name})` is not Sisu's: an item is `pub` to every package, or `pub(package)` to the modules of its own |
| [E0131](#e0131) | `parse.attribute-line` | an attribute stands on a line of its own, before its declaration |
| [E0132](#e0132) | `parse.interpolated-name` | the name of a test, a suite or a benchmark is fixed text, and `{` starts no interpolation in it; a brace is written `{{` or `}}` |

### Names and declarations, every code

| Code | Identifier | Message |
|---|---|---|
| [E0200](#e0200) | `resolve.undefined-name` | cannot find `{name}` in this scope |
| [E0201](#e0201) | `resolve.undefined-type` | cannot find a type named `{name}` |
| [E0202](#e0202) | `resolve.duplicate-definition` | `{name}` is defined more than once in this module |
| [E0203](#e0203) | `resolve.unknown-module` | there is no module `{name}` |
| [E0204](#e0204) | `resolve.not-in-module` | module `{module}` has no item `{name}` |
| E0205 | `resolve.duplicate-field` | `{name}` is declared more than once in `{owner}` |
| E0206 | `resolve.duplicate-variant` | `{owner}` has more than one variant named `{name}` |
| E0207 | `resolve.duplicate-parameter` | the parameter `{name}` appears more than once |
| E0208 | `resolve.type-argument-count` | wrong number of type arguments for `{name}`: expected {expected}, found {found} |
| E0209 | `resolve.not-a-type` | `{name}` is not a type |
| [E0211](#e0211) | `resolve.ambiguous-variant` | `{name}` is a variant of more than one type ({candidates}); qualify it, as in `{example}` |
| E0212 | `resolve.not-an-error-set` | `{name}` is not an error set, so it cannot follow `!` |
| E0213 | `resolve.drop-target` | `Drop` can be implemented only for a struct or enum declared in this module |
| [E0215](#e0215) | `resolve.self-outside-method` | `self` exists only inside a method |
| E0216 | `resolve.drop-signature` | `Drop` has one method, `def drop(mut self)` |
| [E0217](#e0217) | `resolve.duplicate-method` | `{owner}` has more than one method named `{name}` |
| E0218 | `resolve.self-not-first` | `self` must be the first parameter |
| E0219 | `resolve.const-type-required` | a `const` needs a type: `const {name}: T = ...` |
| [E0220](#e0220) | `resolve.import-unknown` | cannot import `{name}`: it names no module of this package, of a dependency in `sisu.toml`, or of the standard library |
| E0221 | `resolve.field-default-order` | a field with a default must not use the value of another field |
| E0222 | `resolve.parameter-default-order` | a parameter without a default cannot follow one with a default |
| [E0223](#e0223) | `resolve.not-a-module` | `{name}` is not a module |
| E0224 | `resolve.missing-body` | `{name}` has no body; only standard-library intrinsics are declared without one |
| [E0225](#e0225) | `resolve.std-only-attribute` | `@{name}` is reserved for the standard library |
| E0226 | `resolve.inferred-set-position` | an inferred error set (`T !`) can only be the result type of a function |
| E0227 | `resolve.union-outside-result` | `{A, B}` is a union of error sets and can only follow `!` |
| E0228 | `resolve.alias-cycle` | the type alias `{name}` refers to itself |
| E0229 | `resolve.self-type-outside` | `Self` exists only inside a type's declaration or `impl` |
| E0230 | `resolve.not-a-trait` | `{name}` is not a trait |
| [E0231](#e0231) | `resolve.trait-as-type` | `{name}` is a trait, not a type; a value of a type that implements it is `dyn {name}` |
| [E0232](#e0232) | `resolve.missing-trait-method` | the impl of `{trait}` for `{ty}` is missing {methods} |
| [E0233](#e0233) | `resolve.not-a-trait-method` | `{name}` is not a method of `{trait}` |
| E0234 | `resolve.method-signature` | `{name}` does not have the signature `{trait}` gives it: expected `{expected}`, found `{found}` |
| [E0235](#e0235) | `resolve.overlapping-impl` | `{trait}` is implemented for `{ty}` more than once; a type implements a trait at most once |
| [E0236](#e0236) | `resolve.orphan-impl` | `{trait}` for `{ty}` can be implemented only in the package of the trait or of the type, or in the root package |
| [E0237](#e0237) | `resolve.missing-assoc-type` | the impl of `{trait}` for `{ty}` does not say what `{name}` is; add `type {name} = ...` |
| E0238 | `resolve.unknown-assoc-type` | `{trait}` has no associated type `{name}` |
| E0239 | `resolve.no-assoc-type` | `{ty}` has no associated type `{name}` among its bounds |
| E0240 | `resolve.sealed-trait` | `{trait}` is implemented only by the standard library, so `{ty}` cannot implement it |
| [E0241](#e0241) | `resolve.not-object-safe` | `{trait}` cannot be used as `dyn {trait}`: its member `{method}` cannot be used through it, because {reason} |
| [E0242](#e0242) | `resolve.supertrait-missing` | `{ty}` implements `{trait}`, which requires `{super}`, but `{ty}` does not implement `{super}` |
| [E0243](#e0243) | `resolve.not-derivable` | `{name}` cannot be derived; `@derive` takes `Copy` and the traits that have a `comptime def derive`: `Debug`, `Default`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that declares one |
| [E0244](#e0244) | `resolve.derive-field` | `{ty}` cannot derive `{trait}`: its field `{field}` has type `{fty}`, which does not implement `{trait}` |
| [E0245](#e0245) | `resolve.generic-default-order` | a type parameter without a default cannot follow one with a default |
| E0246 | `resolve.impl-default` | the type parameters of an `impl` or a function take no defaults; a default belongs to a type's or a trait's parameter |
| E0247 | `resolve.trait-cycle` | `{name}` cannot be its own supertrait |
| E0248 | `resolve.assoc-outside-trait-impl` | an associated type belongs in an impl of a trait |
| E0249 | `resolve.assoc-needs-type` | `type {name}` in an impl needs a type: `type {name} = ...` |
| E0250 | `resolve.not-delegable` | `{name}` cannot be delegated; `@delegate` takes `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Display`, `Debug` and `Len` |
| E0251 | `resolve.delegate-to` | `@delegate` needs `to="field"`, naming a field of `{ty}` |
| [E0252](#e0252) | `resolve.delegate-self` | `{trait}` returns a `{ty}`, so `{ty}` can delegate it only when `{field}` is its one field |
| E0253 | `resolve.delegate-field` | `{ty}` cannot delegate `{trait}` to `{field}`: its type `{fty}` does not implement `{trait}` |
| E0254 | `resolve.drop-generic` | a `Drop` for `{ty}` must apply to every `{name}`: write `{expected}`, with the type's parameters in order and only the bounds the type declares |
| [E0255](#e0255) | `resolve.private-item` | `{name}` is private to the module `{module}`; declare it `pub` to use it from another module |
| [E0256](#e0256) | `resolve.package-item` | `{name}` is `pub(package)` in `{module}`, so only the modules of the package `{package}` can use it |
| E0257 | `resolve.not-reexported` | `{name}` is imported into `{module}`, not declared there; import it from `{origin}`, or re-export it with `pub from {origin} import {name}` |
| [E0258](#e0258) | `resolve.private-field` | the field `{field}` of `{ty}` is private to the module `{module}`; declare it `pub` to use it from another module |
| [E0259](#e0259) | `resolve.private-method` | the method `{name}` of `{ty}` is private to the module `{module}`; declare it `pub` to call it from another module |
| E0260 | `resolve.not-a-dependency` | cannot import `{name}`: `{package}` is not a dependency of `{this}`; add it to `[dependencies]` in that package's `sisu.toml` |
| E0261 | `resolve.script-main` | a script's statements at the top level are its `main`, so it cannot also declare `main` |
| E0262 | `resolve.const-argument` | `{param}` is a `const` parameter of `{name}`, so it takes a constant, such as `{name}[..., 8]`, not a type |
| E0263 | `resolve.type-argument` | `{param}` is a type parameter of `{name}`, so it takes a type, not a constant |
| E0264 | `resolve.const-expression` | a type's constant argument is computed where the type is written, from integers, `const` parameters, constants written that way and the operators `+ - * // % ** << >> & \| ^`; this is none of them |
| E0265 | `resolve.const-not-int` | a type's constant argument is an `int`, and this is a `{ty}` |
| [E0266](#e0266) | `resolve.bad-cfg` | `@cfg` takes a condition: `test`, `feature="NAME"`, `not` and a condition, or `all(...)` or `any(...)` of conditions; this is none of them |
| E0267 | `resolve.not-camel-case` | the name of {kind} is written in CamelCase, and `{name}` is not |
| E0268 | `resolve.not-snake-case` | the name of {kind} is written in snake_case, and `{name}` is not |
| E0269 | `resolve.not-screaming-case` | the name of {kind} is written in SCREAMING_CASE, and `{name}` is not |
| E0270 | `resolve.copy-by-hand` | `Copy` is derived, not implemented: an `impl Copy for {ty}` written by hand leaves `{ty}` moved, and `@derive(Copy)` makes it copied when every field is `Copy` (03) |
| [E0271](#e0271) | `resolve.not-a-trait-constant` | `{name}` is not a constant of `{trait}` |
| [E0272](#e0272) | `resolve.constant-type` | the constant `{name}` has the type `{found}` here, and `{trait}` declares it with the type `{expected}` |
| [E0273](#e0273) | `resolve.const-or-method` | `{name}` is {what} of `{trait}`, and this impl gives it as {given} |
| [E0274](#e0274) | `resolve.const-not-type` | `{name}` is a constant of `{trait}`, which bounds `{ty}`, not a type |
| [E0275](#e0275) | `resolve.dependency-excluded` | cannot import `{name}`: the dependency `{package}` of `{this}` is not built for this target, which its `when` in that package's `sisu.toml` excludes |
| [E0276](#e0276) | `resolve.dev-dependency` | cannot import `{name}` here: `{package}` is a dev-dependency of `{this}`, which its tests import and its code does not |
| [E0277](#e0277) | `resolve.import-in-body` | an import stands at the top level of a module, before its declarations, and its names are the whole module's; a function's body holds none |
| [E0278](#e0278) | `resolve.prelude-internal` | `{name}` is the prelude's own, for the standard library's code: a program sees only the names it marks `pub` (09) |
| [E0279](#e0279) | `resolve.plain-padding` | `{ty}` cannot derive `Plain`: C's layout leaves {bytes} bytes of padding between `{before}` and `{after}`, at offsets {start} to {end}, and a plain type has none (22) |
| [E0280](#e0280) | `resolve.plain-padding-end` | `{ty}` cannot derive `Plain`: C's layout leaves {bytes} bytes of padding after `{before}`, at offsets {start} to {end}, the end of the struct, and a plain type has none (22) |
| [E0281](#e0281) | `type.fixture-signature` | `{name}` is a `@fixture`, which a test names as a parameter to receive what it returns, so it is a free function that takes nothing |

### Types, every code

| Code | Identifier | Message |
|---|---|---|
| [E0300](#e0300) | `type.mismatch` | expected `{expected}`, found `{found}` |
| [E0301](#e0301) | `type.integer-division` | `/` divides floats; for integers use `//`, or convert to `f64` first |
| [E0302](#e0302) | `type.not-bool` | a condition must be a `bool`, not `{found}`; Sisu has no truthiness |
| E0303 | `type.unknown-field` | `{ty}` has no field `{name}` |
| [E0304](#e0304) | `type.unknown-method` | `{ty}` has no method `{name}` |
| [E0305](#e0305) | `type.argument-count` | wrong number of arguments to `{name}`: expected {expected}, found {found} |
| [E0306](#e0306) | `type.unknown-keyword` | `{name}` has no parameter named `{arg}` |
| E0307 | `type.duplicate-argument` | the argument `{arg}` is given more than once |
| [E0308](#e0308) | `type.missing-argument` | missing the argument `{arg}` of `{name}` |
| E0309 | `type.not-callable` | a value of type `{ty}` cannot be called |
| [E0310](#e0310) | `type.binary-operator` | `{op}` cannot be applied to `{lhs}` and `{rhs}` |
| E0311 | `type.unary-operator` | `{op}` cannot be applied to `{ty}` |
| [E0312](#e0312) | `type.assign-immutable` | `{name}` is immutable; declare it with `var` to assign to it |
| [E0313](#e0313) | `type.not-a-place` | this expression cannot be assigned to |
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
| [E0324](#e0324) | `type.cannot-infer` | the type of `{name}` cannot be inferred here; add a type annotation |
| [E0325](#e0325) | `type.not-iterable` | a `for` loop cannot iterate over `{ty}` |
| E0326 | `type.pattern-mismatch` | this pattern cannot match a value of type `{ty}` |
| [E0327](#e0327) | `type.unknown-variant` | `{ty}` has no variant `{name}` |
| [E0328](#e0328) | `type.pattern-field-count` | wrong number of fields in the pattern for `{name}`: expected {expected}, found {found} |
| [E0329](#e0329) | `type.refutable-let` | this pattern might not match; add `else:` with a block that leaves the scope |
| E0330 | `type.else-not-fallible` | `else` after `let` needs a `T?`, a `T ! E` or a pattern that can fail; this value is `{ty}` |
| E0331 | `type.else-must-diverge` | the block after `else:` must leave the scope with `return`, `break`, `continue` or a panic |
| E0332 | `type.float-suffix` | a float literal takes the suffix `f32` or `f64`, not `{suffix}` |
| [E0333](#e0333) | `type.impure-call` | `{callee}` is not pure, so the pure function `{name}` cannot call it |
| [E0334](#e0334) | `type.impure-capability` | the pure function `{name}` cannot use the capability `{ty}` |
| [E0335](#e0335) | `type.impure-argument` | this function is passed where a pure function is required, but it calls `{callee}`, which is not pure |
| [E0337](#e0337) | `type.missing-field` | missing the field `{field}` of `{ty}` |
| [E0338](#e0338) | `type.move-out-of-borrow` | cannot move `{what}` out of a borrowed value; copy it with `.copy()` |
| [E0339](#e0339) | `type.mut-argument-missing` | `{param}` is a `mut` parameter, so the call must say `mut` before the argument |
| [E0340](#e0340) | `type.mut-argument-unexpected` | `{param}` is not a `mut` parameter, so the argument takes no `mut` |
| [E0341](#e0341) | `type.not-mutable` | cannot modify `{what}`; it is not declared with `var` |
| [E0342](#e0342) | `type.it-outside-closure` | `it` is the parameter of an implicit closure and exists only where a one-parameter function is expected |
| [E0343](#e0343) | `type.shared-not-mutable` | cannot modify `{what}`: a `Shared` holds it, and a shared value is not changed in place, even through a `var` |
| [E0344](#e0344) | `type.constant-negative-index` | a constant negative index does not compile; count from the end with `^`: `{fix}` |
| [E0345](#e0345) | `type.str-not-indexable` | `{ty}` cannot be indexed by position; use `.chars()`, `.bytes()` or `.byte_slice(a, b)` |
| [E0347](#e0347) | `type.bound-not-satisfied` | `{ty}` does not implement `{bound}` |
| E0349 | `type.not-generic` | `{name}` does not take type arguments |
| E0350 | `type.not-a-value` | `{name}` is a {kind}, not a value |
| [E0351](#e0351) | `type.unknown-associated` | `{ty}` has no associated function `{name}` |
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
| [E0385](#e0385) | `type.bindings-in-or-pattern` | `{name}` is bound in some alternatives of `\|` but not in all; every alternative must bind the same names |
| E0386 | `type.variant-needs-fields` | `{name}` has fields; write `{name}(...)` |
| E0387 | `type.star-outside-list` | `*name` collects the rest of a list and appears only in a list pattern |
| E0388 | `type.rest-outside-arguments` | `..` skips the remaining fields and appears only inside `Name(...)` |
| [E0389](#e0389) | `type.no-self` | `self` exists only inside a method |
| E0390 | `type.not-a-struct-pattern` | `{name}` cannot be matched with fields |
| E0391 | `type.const-pattern` | a constant cannot be used as a pattern yet; compare with `==` in a guard |
| [E0392](#e0392) | `type.impure-value-call` | `{name}` is pure, so it cannot call a function value whose type is not `pure` |
| E0393 | `type.impure-drop` | dropping `{value}` here runs the `Drop` of `{ty}`, which is not pure, so the pure function `{name}` cannot do it |
| E0394 | `type.ambiguous-method` | `{name}` is a method of more than one trait `{ty}` implements ({traits}); call it on the trait, as in `Trait.{name}(x)` |
| [E0395](#e0395) | `type.unsized-dyn` | `{ty}` has no size of its own; hold it behind `ref` or in a `Box` |
| E0396 | `type.view-to-dyn` | `{ty}` borrows, so it cannot become `{dyn}`, which owns its value |
| E0397 | `type.must-consume-argument` | `{ty}` is must-consume, so it cannot be the argument of the generic parameter `{param}`, whose values the body may drop |
| [E0398](#e0398) | `type.structural-only` | {callee} is hashed and compared structurally in stage 0, and `{ty}` has an impl of `{trait}` of its own; derive `Hash` and `Eq` for it, or key by a field |
| [E0399](#e0399) | `type.bad-format-spec` | `{spec}` is not a format spec; it is `[[fill]align][sign][#][0][width][,\|_][.precision][type]`, as in `>8.2f`, with the type one of `d x X o b e E f F % s ?` |

### Ownership, every code

| Code | Identifier | Message |
|---|---|---|
| [E0400](#e0400) | `borrow.use-after-move` | `{name}` was moved on line {line} and cannot be used afterwards |
| [E0401](#e0401) | `borrow.use-after-partial-move` | `{name}` cannot be used as a whole, because `{part}` was moved out of it on line {line} |
| [E0402](#e0402) | `borrow.modify-while-borrowed` | `{name}` is borrowed on line {line} and cannot be modified until that borrow ends on line {end} |
| [E0403](#e0403) | `borrow.use-while-mut-borrowed` | `{name}` is borrowed with `mut` on line {line} and cannot be used until that borrow ends on line {end} |
| [E0404](#e0404) | `borrow.move-while-borrowed` | `{name}` is borrowed on line {line} and cannot be moved until that borrow ends on line {end} |
| [E0405](#e0405) | `borrow.mut-while-borrowed` | `{name}` is already borrowed on line {line}, so it cannot be borrowed with `mut` until that borrow ends on line {end} |
| [E0406](#e0406) | `borrow.does-not-live-long-enough` | `{name}` does not live long enough: it is borrowed on line {line}, and the borrow is still used on line {end}, after `{name}` is gone |
| [E0407](#e0407) | `borrow.return-local` | this returns a borrow of `{name}`, which belongs to `{function}` and is gone when it returns |
| [E0408](#e0408) | `borrow.return-not-from` | this returns a borrow of the parameter `{param}`, which is not in the `from` clause of `{function}` |
| E0409 | `borrow.store-in-plain-type` | `{ty}` cannot hold this value, because it borrows `{name}`; only a `view struct` holds borrows |
| E0410 | `borrow.store-into-caller` | this value borrows `{name}`, so it cannot be stored in `{param}`, which belongs to the caller of `{function}` |
| [E0411](#e0411) | `borrow.view-field-in-plain-struct` | `{ty}` holds `{field}: {fty}`, which borrows; only a `view struct` may hold a borrow |
| [E0412](#e0412) | `borrow.view-in-enum` | `{ty}` cannot hold `{fty}`, which borrows; an enum has no view form, so store an owned value such as `String` |
| [E0413](#e0413) | `borrow.from-required` | `{name}` returns `{ret}`, which borrows, and has more than one borrowed parameter ({params}); name the ones it borrows from with `from` |
| E0414 | `borrow.from-not-a-parameter` | `{name}` in the `from` clause is not a borrowed parameter of `{function}` |
| E0415 | `borrow.from-without-borrow` | `{function}` returns `{ty}`, which does not borrow, so it takes no `from` clause |
| E0416 | `borrow.move-out-of-drop` | cannot move `{part}` out of `{name}`, because `{ty}` implements `Drop`, which needs the whole value |
| [E0417](#e0417) | `borrow.must-consume` | `{name}` is never consumed: a value of `{ty}` must be passed to an `own` parameter before it goes out of scope |
| [E0418](#e0418) | `borrow.must-consume-exit` | this leaves the scope with `{name}` unconsumed: a value of `{ty}` must be passed to an `own` parameter first |
| E0419 | `borrow.move-out-of-place` | cannot move out of `{name}` through a reference or an index; copy it with `.copy()` |
| E0420 | `borrow.must-consume-intrinsic` | `{callee}` may drop values that `{name}` holds, and a value of `{ty}` is must-consume: its values must be passed to an `own` parameter |
| [E0421](#e0421) | `borrow.view-in-owner` | `{ty}` borrows, so it cannot be put in `{owner}`, which can outlive what it borrows (04, rule 3) |
| E0422 | `borrow.must-consume-in-owner` | `{ty}` is must-consume, so it cannot be put in `{owner}`, which can drop it where the checker does not see |
| [E0423](#e0423) | `type.own-closure-capture` | an `own` closure keeps a copy of `{name}`, and `{ty}` is not `Copy`; stage 0 copies captures (a capture by move arrives with tasks) |
| E0424 | `type.own-closure-changes` | an `own` closure keeps a copy of `{name}` and cannot change it; borrow it with a plain closure instead |
| E0425 | `task.closure-literal` | `{function}` takes a closure written at the call (or a `do` block), so that what it captures can be checked; a function value is not one |
| E0426 | `task.not-send` | `{name}` moves into the task, but `{ty}` is not `Send` |
| [E0427](#e0427) | `task.not-sync` | `{name}` is shared with the task by reference, but `{ty}` is not `Sync` |
| [E0428](#e0428) | `task.borrows-body-local` | the task borrows `{name}`, which belongs to the scope's body: the body ends before the scope joins its tasks |
| E0429 | `task.borrows-elsewhere` | the task borrows `{name}`, but only a task spawned on a scope in that scope's body may borrow, and only what the body's environment borrows |
| E0430 | `task.scope-escapes` | a scope's body cannot return `{ty}`: a scope and the handles of its tasks end with the scope |
| E0431 | `task.race-types` | the tasks of a race return one type: this one returns `{found}`, an earlier one `{expected}` |
| E0432 | `task.result-unknown` | the type of the task's result must be known where it is joined |
| E0433 | `task.result-not-send` | the task returns `{ty}`, which is not `Send`, so it cannot cross back to the task that joins it |
| E0434 | `task.select-arm` | a `select` arm waits for a `recv`, `send` or `join` call, a `Timer` or a `Cancellation`, not `{ty}` |
| E0435 | `type.yield-outside-generator` | `yield` belongs in the body of a generator, a function that returns `Iter[T]` (not in a closure) |
| E0436 | `type.generator-result` | `{name}` yields, so it must return `Iter[T]`, not `{ty}` |
| E0437 | `task.not-send-mut` | `{name}` is changed by the task through a reference, but `{ty}` is not `Send` |
| [E0438](#e0438) | `borrow.ref-in-collection` | `{ty}` would hold references, and a collection holds values it owns or views, never references (04, rule 3) |
| [E0445](#e0445) | `type.hole-in-release` | a release build rejects the typed hole {hole} of type `{ty}`: blocked is never a shipped state |
| [E0446](#e0446) | `type.par-body-impure` | `{callee}` is not `pure`, so the body of a `par` loop cannot call it: a `par` body runs on a worker and is `pure` (06) |
| [E0447](#e0447) | `type.par-body-changes` | the body of a `par` loop changes nothing but its own values, and `{name}` is declared before the loop: compute each value with `par.map` and combine them with `sum` or a reducer (06) |
| [E0448](#e0448) | `resolve.float-attribute` | `@float` takes `fast`, which allows contraction and reassociation in the function (03); floating point is strict elsewhere |
| [E0449](#e0449) | `type.trait-constant` | each impl of `{trait}` gives its own `{name}`, so it is read through a type: `T.{name}` for a type `T` that implements `{trait}` |

### Lowering, every code

| Code | Identifier | Message |
|---|---|---|
| E0500 | `sir.parse` | SIR: {detail} |
| E0501 | `sir.invalid` | invalid SIR in `{function}`: {detail} |

### Compile-time evaluation, every code

| Code | Identifier | Message |
|---|---|---|
| E0600 | `comptime.not-constant` | `comptime` evaluates what is known at compile time, and `{name}` is known only at run time |
| [E0601](#e0601) | `comptime.io` | the compile-time evaluation of this expression used {what}; nothing reaches the world at compile time |
| [E0602](#e0602) | `comptime.panicked` | the compile-time evaluation of this expression panicked: {message} |
| [E0603](#e0603) | `comptime.budget` | the compile-time evaluation in `{item}` used up its budget of {steps} steps; raise it with `@comptime_budget(steps=N)` on `{item}` |
| [E0604](#e0604) | `comptime.not-kept` | {what} made at compile time cannot be kept for run time; numbers, text, booleans, characters, and lists, sets, dicts, tuples, options, results, structs and enums of them can |
| [E0605](#e0605) | `comptime.only` | `{name}` is a `comptime def`: it can be called only at compile time, inside `comptime` |
| [E0606](#e0606) | `comptime.budget-attribute` | `@comptime_budget` takes `steps=N`, a positive number of steps |
| [E0607](#e0607) | `comptime.derive-template` | a trait's derive is `comptime def derive(T: type) -> impl {trait} for T`, whose body is `return impl {trait} for T:` with the impl's methods |
| [E0608](#e0608) | `comptime.derive-failed` | deriving `{trait}` for `{ty}` stopped at compile time: {reason} |
| [E0609](#e0609) | `comptime.instance-failed` | making `{name}` stopped at compile time: {reason} |
| E0610 | `comptime.instances-unbounded` | `{name}` needs more than {count} instances: its instances call it with ever more types |
| E0611 | `comptime.template-value` | `{name}` reflects on its type parameters, so it is made for each call's types and cannot be used as a value |
| [E0612](#e0612) | `comptime.embed-failed` | cannot embed `{path}`: {reason} |
| [E0613](#e0613) | `comptime.embed-path` | `embed` takes the path of a file as a string literal |
| E0614 | `comptime.no-fake` | `cap.Fake[{name}]` needs a trait of the program whose methods can all be forwarded: a `self` or `mut self` receiver, no type parameters, and no borrow or `Self` in a signature |
| E0615 | `comptime.unsafe-written` | code that compile-time evaluation writes holds no `unsafe`, so that every unsafe block stays in a source file, where `sisu audit --unsafe` shows it |
| [E0616](#e0616) | `comptime.function-failed` | the compile-time code of `{name}` stopped: {reason} |
| [E0617](#e0617) | `eval.no-form` | `{name}` holds {what}, which {form} cannot write |

### Capabilities, every code

| Code | Identifier | Message |
|---|---|---|
| E0700 | `cap.not-shareable` | `{ty}` cannot become `{cap}`: a capability may reach every task, and `{ty}` is not `{auto}` |
| E0701 | `cap.internal` | `{name}` is internal to the standard library; a program reaches the world only through the capabilities it is given |
| E0702 | `type.test-signature` | a test takes nothing, the real `System` as its one parameter (`test "name"(sys: System):`), or fresh fakes of the capabilities it names, one of each (`test "name"(fs: Fs, clock: Clock):`) |
| [E0703](#e0703) | `cap.untrusted` | the package `{package}` is not trusted with {what}; a dependency may contain `unsafe` and `extern` only when `[trust] unsafe` in the root package's `sisu.toml` lists it |

### Contracts, every code

| Code | Identifier | Message |
|---|---|---|
| [E0800](#e0800) | `contract.misplaced` | `{keyword}` is a function's contract and belongs at the start of its body, before its first statement |
| [E0801](#e0801) | `contract.not-pure` | a contract only reads what it is given, and `{callee}` is not `pure` |
| E0802 | `contract.changes` | a contract only reads what it is given, and this changes it |
| [E0803](#e0803) | `contract.old-outside` | `old(...)` is a value as it was on entry, which only an `ensures` clause can ask for |
| [E0804](#e0804) | `contract.old-arguments` | `old` takes one expression, as in `old(self.balance)` |
| [E0805](#e0805) | `contract.generator-ensures` | a generator has no `ensures`: what it returns is its iterator |
| [E0806](#e0806) | `contract.invariant-field` | `{what}` is part of a value of `{ty}`, whose invariant only its own methods may change and check again; change it through one of them |
| [E0807](#e0807) | `contract.not-in-test` | a test has no contract; check what it needs with `assert` |
| [E0810](#e0810) | `refine.not-satisfied` | `{value}` is not of type `{ty}`: `{predicate}` is false for it |
| [E0811](#e0811) | `refine.needs-try-from` | expected `{expected}`, found `{found}`; a value becomes a `{expected}` where its predicate is checked, with `{name}.try_from(...)` |
| [E0812](#e0812) | `refine.change-through-base` | `{what}` is of type `{ty}`, which a change through its base could take outside `{predicate}`; change a copy of the base and check it again with `try_from` |
| [E0813](#e0813) | `refine.own-impl` | `{ty}` refines `{base}` and has its reading impls; it cannot have an impl of `{trait}` of its own |
| [E0814](#e0814) | `refine.predicate-type` | the predicate of `{ty}` must be a `bool`, found `{found}` |
| E0820 | `test.no-generator` | `{ty}` has no generator for property tests: {why} |
| E0821 | `test.refines-mismatch` | `@refines({reference})` on `{name}` cannot be checked: {why} |

### Stage 0's limits, every code

| Code | Identifier | Message |
|---|---|---|
| [E0900](#e0900) | `stage0.unsupported` | {construct} is not implemented in stage 0 yet; it arrives with {phase} |
| E0901 | `stage0.internal` | internal compiler error in {stage}: {detail} |
| E0971 | `sim.unmodelled-extern` | this `sim` build reaches `{callee}`, an `extern` call with no simulation model, in `{function}` at {at} |
| [E0981](#e0981) | `unsafe.call` | `{name}` is an `unsafe def`: call it inside an `unsafe:` block or an `unsafe` expression |
| E0982 | `unsafe.foreign-call` | `{name}` is a foreign function, whose signature is a claim the compiler cannot check: call it inside an `unsafe:` block or an `unsafe` expression |
| E0983 | `unsafe.fn-value` | `{name}` is {what}, which cannot be a function value: a call through the value would need no `unsafe` |
| E0984 | `unsafe.impl-needed` | `{trait}` is an `unsafe trait`, whose implementors keep a promise the compiler cannot check: its impl is written `unsafe impl` |
| E0985 | `unsafe.impl-promises-nothing` | {what}, so `unsafe impl` promises nothing: write `impl` |
| E0986 | `unsafe.never-asserted` | `{trait}` is never asserted: the compiler decides it from a type's parts, and nothing may claim it otherwise |
| E0987 | `unsafe.not-ptr` | `ptr mut T` is the raw pointer of `std.mem`, and `{name}` here names {found} |
| E0988 | `unsafe.in-contract` | a contract holds no `unsafe`: it may call safe functions built on unsafe code, and unsafe code relies on no contract |
| [E0989](#e0989) | `unsafe.export-signature` | `{name}` is `@export`ed, and foreign code that calls it cannot keep `{ty}` alive: an exported function's signature holds no reference, view, `Shared`, closure or capability |
| [E0990](#e0990) | `unsafe.blocking-not-foreign` | `@blocking` marks a foreign function whose call may block its thread, and `{name}` is not a foreign function |
| [E0991](#e0991) | `unsafe.export-not-c` | `{name}` is `@export`ed, and `{ty}` has no layout C shares: an exported function takes and returns numbers, `bool`, `char`, raw pointers and types declared `@repr(c)` |

### Packages and manifests, every code

| Code | Identifier | Message |
|---|---|---|
| E1000 | `package.toml-syntax` | this is not TOML that `sisu.toml` reads: expected {expected}, found {found} |
| E1001 | `package.toml-duplicate` | `{key}` is given more than once in `{table}` |
| [E1002](#e1002) | `package.missing-key` | `{table}` needs `{key}` |
| [E1003](#e1003) | `package.bad-value` | `{key}` takes {expected}, found {found} |
| E1004 | `package.bad-name` | `{name}` cannot name a package: {why} |
| E1005 | `package.no-manifest` | the dependency `{name}` has no `sisu.toml` at `{path}` |
| E1006 | `package.renamed-dependency` | the dependency `{name}` is the package `{found}`; a path dependency is named by its own `[package] name` |
| [E1007](#e1007) | `package.cycle` | the packages depend on each other in a cycle: {cycle}; packages build in dependency order, so their graph has no cycle |
| E1008 | `package.no-root-module` | the package `{name}` has no root module; add `src/main.sisu` (an application) or `src/lib.sisu` (a library) |
| E1009 | `package.two-root-modules` | the package `{name}` has both `src/main.sisu` and `src/lib.sisu`; a package has one root module |
| [E1010](#e1010) | `package.not-a-path` | the dependency `{name}` is not a `path` dependency; stage 0 has no registry, so a dependency is `{name} = { path = "../{name}" }` |
| E1011 | `package.bad-when` | `when` takes the build target's facts `os`, `arch` and `runtime`, each a string or a list of strings, as in `when = { os = "windows" }` |
| E1012 | `package.module-name` | `{file}` is not a module: `{name}` is not a module name; a module is named in lowercase letters, digits and `_` |
| E1013 | `package.no-member` | the workspace member `{path}` has no `sisu.toml` |
| E1014 | `package.script-metadata` | a script's metadata starts with `# /// script` and ends with a line `# ///`; this block has no end |
| [E1015](#e1015) | `package.trust-unknown` | `[trust] unsafe` names `{name}`, which is not a dependency of this package |
| E1016 | `package.no-package` | `{path}` has `[workspace]` but no `[package]`, so it cannot be a dependency |
| [E1017](#e1017) | `package.reserved-name` | `{name}` is the name of a standard-library module, which `import {name}` names; a package of the program needs another name |
| E1018 | `package.unknown-feature` | `{name}` is not a feature of `{package}`; a package's features are the keys of its `[features]` |
| [E1019](#e1019) | `package.unsafe-undeclared` | the package `{package}` contains {what}, and its `sisu.toml` declares no `unsafe`: add `unsafe = "{level}"` to its `[package]` |
| E1020 | `package.unsafe-level` | the package `{package}` declares `unsafe = "memory"`, which allows no {what}: declare `unsafe = "extern"` |
| [E1021](#e1021) | `package.unsafe-untrusted` | the dependency `{name}` declares `unsafe = "{level}"`, and `[trust] unsafe` in the root package's `sisu.toml` does not list it |
| E1022 | `package.unsafe-raised` | `sisu.lock` records `{name}` with {old}, and it now declares `unsafe = "{level}"`: accept the change with `--accept-unsafe` |
| [E1023](#e1023) | `package.freestanding` | the runtime profile `freestanding` has no runtime yet: this compiler's runtime schedules tasks, so a program is built for `full` or `single` |

### Warnings, every code

| Code | Identifier | Message |
|---|---|---|
| [W0001](#w0001) | `type.hole` | typed hole `{name}` of type `{ty}` |
| [W0002](#w0002) | `resolve.shadow-outer` | `{name}` shadows a binding of an enclosing scope |
| [W0003](#w0003) | `type.unreachable-arm` | this `case` arm is unreachable; the arms above it cover every value it matches |
| W0004 | `type.unreachable-code` | this code is unreachable |
| [W0005](#w0005) | `package.unknown-key` | `{key}` is not a key of `{table}` that stage 0 knows; it is ignored |
| W0006 | `resolve.unknown-feature` | no `[features]` of this package declares `{name}`, so `feature="{name}"` never holds |
| [W0007](#w0007) | `unsafe.no-safety-comment` | this {what} has no `# safety:` comment above it saying why each obligation it takes on holds |
| W0008 | `unsafe.no-safety-list` | `{name}` is an `unsafe {kind}` without a `Safety:` list in its `##` comment naming the obligations its {who} take |
| W0009 | `unsafe.unused` | this {what} holds no unsafe operation |
| [W0010](#w0010) | `unsafe.unnamed-obligation` | the `# safety:` comment names no obligation of `{callee}`, which are {obligations} |
| W0011 | `unsafe.unexplained-purity` | this {what} calls `{callee}`, which is not `pure`, in the `pure` function `{name}`: its `# safety:` comment says why the function stays observationally pure, under a `pure:` part |
| W0012 | `unsafe.level` | the package declares `unsafe = "{level}"`, {why} |
| W0013 | `unsafe.auto-trait-bound` | `unsafe impl {trait}` claims `{trait}` for every `{param}`, which is not bounded by `{trait}` |
| [W0014](#w0014) | `unsafe.ambient-authority` | `{name}` is public and safe and reaches the foreign function `{callee}` without taking a capability, so its callers need no authority for what the call does |
| [W0015](#w0015) | `lint.wildcard-own-enum` | `case _` matches {variants} of `{ty}`, an enum of this package, so a variant added to `{ty}` later is not an error here |
| W0016 | `resolve.shadow-prelude-set` | the error set `{name}` hides the prelude's in this module, and the two are different sets: what the prelude's functions return is not this one |
| [W0021](#w0021) | `contract.trait-elsewhere` | the contract of `{method}` binds this impl, and stage 1 does not have the syntax of the trait's module to check it in |
| [W0022](#w0022) | `resolve.unknown-attribute` | `@{name}` is not an attribute Sisu reads, and no compile-time code of the program reads attributes (`reflect.attributes_of`), so it means nothing here |
| [W0023](#w0023) | `type.deprecated` | `{name}` is deprecated: {reason} |
| [W0024](#w0024) | `resolve.misspelt-attribute` | `@{name}` is not an attribute Sisu reads |
| [W0025](#w0025) | `type.deprecated-plain` | `{name}` is deprecated |
| [W0026](#w0026) | `resolve.misplaced-attribute` | `@{name}` means something on {places}, so on {what} it does nothing |
| [W0027](#w0027) | `lint.unused-import` | `{name}` is imported and not used in this file |
| [W0028](#w0028) | `type.preview` | `{name}` is a preview, which may change or be removed before it is stable: {reason} |
| [W0029](#w0029) | `type.preview-plain` | `{name}` is a preview, which may change or be removed before it is stable |
| [W0030](#w0030) | `resolve.unchecked-attribute` | `@{name}` is accepted and checked by nothing yet, so nothing ensures {what} |
| [W0031](#w0031) | `resolve.mixed-script` | `{name}` mixes letters of the {scripts} scripts |
| [W0032](#w0032) | `resolve.confusable` | `{name}` looks like `{other}`, another name in scope |
| [W0033](#w0033) | `type.fix-inline-body` | `@fix_inline` rewrites a call of `{name}` with the expression it returns, so its body is one `return`, and its parameters take no `mut` or `own`: its calls warn without the rewrite |

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
| R0051 | `monitor.sir-header` | SIR of `{function}`, its blocks that the conflict passes through: |
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
| R0069 | `monitor.unconsumed` | `{name}` was dropped holding a value of `{ty}`, which must be passed to an `own` parameter |
| R0070 | `defer.declarations-only` | only rules about declarations reject the program, and the run reached no conflict: a candidate false positive |
| R0071 | `monitor.op-return` | the return from the frame |
| R0072 | `panic.stale-key` | stale arena key: slot {index}, generation {generation}, no longer holds its value |
| R0073 | `panic.mutex-held` | the lock is already held by this task, which would wait for itself forever |
| R0074 | `panic.mutex-poisoned` | the lock is poisoned: a task panicked holding it and left its value's invariant broken |
| R0076 | `panic.scope-ended` | a task was spawned on a scope that has ended |
| R0077 | `panic.joined-twice` | the task was joined already: its result was taken by an earlier `join` |
| R0078 | `panic.empty-race` | a race whose body spawned no task has no winner |
| R0079 | `panic.unobserved-error` | a task failed with {error} and nobody joined it, and the scope's result cannot carry it |
| [R0080](#r0080) | `run.deadlock` | deadlock: every task is waiting, and no timer can wake any of them |
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
| [R0102](#r0102) | `contract.invariant-failed` | invariant of `{ty}` failed: {clause} |
| R0103 | `contract.refinement-failed` | `{value}` is not of type `{ty}`: `{clause}` is false for it |
| R0104 | `run.case-too-long` | the case ran past its budget of {steps} statements |
| R0105 | `sim.always-failed` | `sim.always` does not hold: {what} |
| R0106 | `sim.choose-none` | `sim.choose` needs at least one option |
| R0107 | `test.snapshot-changed` | the snapshot `{name}` changed (`sisu test --update` accepts the new one): {diff} |
| [R0108](#r0108) | `test.snapshot-missing` | there is no snapshot `{name}` yet (`sisu test --update` stores this one): {value} |
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
| R0131 | `panic.raw-bounds` | `{op}` of {bytes} bytes at offset {offset} of a block of {size} bytes reaches outside it (`{obligation}`) |
| R0132 | `panic.raw-offset` | `ptr.offset` by {count} from offset {offset} of a block of {size} bytes leaves it (`in_bounds`) |
| R0133 | `panic.raw-misaligned` | `{op}` of `{ty}` at offset {offset} of its block, which is not aligned to {align} (`aligned`) |
| R0134 | `panic.raw-released` | `{op}` of memory that was released (`{obligation}`) |
| R0135 | `panic.raw-gone` | `{op}` through a pointer to a value that no longer exists (`{obligation}`) |
| R0136 | `panic.raw-uninitialised` | `{op}` of `{ty}` at offset {offset} of a block of {size} bytes, which holds no value there (`{obligation}`) |
| R0137 | `panic.raw-mistyped` | `{op}` of `{ty}` at offset {offset}, where the memory holds a value of type `{held}` (`{obligation}`) |
| R0138 | `panic.raw-invalid-bits` | `{op}` gives the bits {bits}, which are not a valid value of type `{ty}` (`{obligation}`) |
| R0139 | `panic.raw-shared-write` | `{op}` through a pointer made from a shared borrow (`{obligation}`) |
| R0140 | `panic.raw-overwrite` | `{op}` over a value of type `{ty}` at offset {offset} that was never dropped (`writable`) |
| R0141 | `panic.raw-no-memory` | `{op}` through a pointer with no memory behind it (`{obligation}`) |
| R0142 | `panic.raw-release-other` | `{op}` of memory that `{allocator}` did not give out, or not at its start (`from_here`) |
| R0143 | `panic.raw-release-twice` | `{op}` of memory that was released already (`once`) |
| R0144 | `panic.raw-release-layout` | `Global.release` with a layout of {size} bytes aligned to {align} of a block acquired with {block_size} bytes aligned to {block_align} (`from_here`) |
| R0145 | `panic.raw-release-live` | `{op}` of memory that still holds a value of type `{ty}` at offset {offset}, which was never dropped: every value is dropped once (22, the soundness contract, clause 3) |
| R0146 | `panic.raw-sizes` | `{op}` from `{from}` of {from_size} bytes to `{to}` of {to_size} bytes: the sizes differ |
| R0147 | `panic.raw-plain-short` | `read_plain` of `{ty}` of {size} bytes at {at} of {len} bytes reaches past their end |
| R0148 | `panic.raw-read-only` | `{op}` of memory that foreign code gave out to be read only (`writable`) |
| R0149 | `panic.foreign-unknown` | the interpreter makes no foreign call: `{name}` is not one of the C functions it emulates ({known}); `sisu run --native` calls it |
| R0150 | `panic.foreign-null` | `{name}` returned a null pointer, which its declared result `{ty}` cannot hold |
| R0151 | `panic.foreign-overlap` | `memcpy` of {bytes} bytes from a range that overlaps the one it writes, which C's `memcpy` does not allow |
| R0152 | `panic.foreign-fd` | `write` to the file descriptor {fd}, which the interpreter does not emulate; it emulates 1 and 2, the standard streams |
| R0153 | `panic.machine-intrinsic` | `{name}` is a machine intrinsic of native code; the interpreter runs its tasks on a scheduler of its own |
| [R0154](#r0154) | `run.expect-failed` | expectation failed: {expr} |
| R0155 | `run.expect-failed-message` | expectation failed: {expr} message: {message} |
| [R0160](#r0160) | `panic.hole-closure` | in scope at the typed hole {hole}: |

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
| [D0028](#d0028) | `cli.bad-contract-level` | `--contracts` takes `off`, `preconditions` or `all`, not `{value}` |
| [D0029](#d0029) | `cli.test-checks-contracts` | `sisu test` checks every contract; `--contracts` sets what `sisu run` and `sisu sir` check |
| D0030 | `cli.replay-token` | replay: sisu test --replay {token} {file} |
| D0031 | `cli.replay-no-token` | no replay token: the test takes the real `System`, which a replay cannot stand in for |
| D0032 | `cli.bad-replay-token` | `{token}` is not a replay token |
| [D0033](#d0033) | `cli.token-other-code` | the token is for other code (content hash {expected}); `{file}` has content hash {found} |
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
| D0057 | `cli.bench-result` | bench {name} ... {each} an iteration, the median of {samples} samples of {iterations} iterations (95% interval of the mean {low} to {high}{outliers}) |
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
| [D0103](#d0103) | `native.unsupported` | the native backend does not compile {what} yet (in `{function}`); `sisu run` without `--native` runs the program |
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
| D0120 | `cli.bad-lower` | `--lower` takes `stage2` (the lowering written in Sisu, the default) or `rust` (the Rust lowering, the oracle), not `{value}` |
| D0121 | `native.stage2-build` | building stage 2 in `{dir}` failed; `--lower=rust` lowers with the Rust backend: {detail} |
| D0122 | `native.stage2-failed` | internal compiler error: stage 2 failed to lower the program (status {status}); `--lower=rust` lowers with the Rust backend: {detail} |
| D0123 | `cli.bad-backend` | `--backend` takes `sisu` (the x86-64 backend written in Sisu, the debug profile's default) or `cranelift` (the Rust backend, the release profile's default and the oracle), not `{value}` |
| D0124 | `native.backend-lowering` | `--backend=sisu` compiles the LIR stage 2 lowers, so it does not go with `--lower=rust` |
| D0125 | `native.sisu-failed` | internal compiler error: the Sisu backend failed to compile the program (status {status}); `--backend=cranelift` compiles it with the Rust backend: {detail} |
| D0126 | `native.rt-build` | building the runtime in Sisu from `{seed}` failed: {detail} |
| [D0127](#d0127) | `cli.explore-differs` | schedule seed {seed} of {count} printed other output than seed {first}, with the same exit status {status}; `--schedule-seed {seed}` runs it again |
| [D0128](#d0128) | `cli.transcript-front-end` | a transcript runs on the Rust front end's build of the program, which stops at errors stage 1 does not report: {diagnostics} |
| [D0129](#d0129) | `cli.missing-code` | `sisu explain` needs a diagnostic's code, such as `E0300`, or its name, such as `type.mismatch` |
| [D0130](#d0130) | `cli.missing-name` | `sisu new` needs the name of the package to create |
| [D0131](#d0131) | `cli.version` | sisu {version} |
| [D0132](#d0132) | `cli.not-compiled` | the program was not compiled: errors found ({count}) |
| [D0134](#d0134) | `native.test-unsupported` | `sisu test --native` does not run {what} yet; `sisu test` without `--native` runs it |
| [D0135](#d0135) | `cli.test-output` | what the test printed: |
| [D0136](#d0136) | `cli.bench-samples` | `--samples` takes 2 or more, not {value}: the interval of the mean needs two samples |
| [D0137](#d0137) | `cli.test-program` | tests of `{program}`: |
| [D0138](#d0138) | `cli.replay-no-token-sim` | no replay token: the test takes the real `System`, which `--sim` does not simulate and a replay cannot stand in for; a test that names the capabilities it uses gets simulated ones |
| D0139 | `native.size-total` | `{output}` is {bytes} bytes: |
| D0140 | `native.size-program` | {bytes} bytes ({share}%), the program's own code |
| D0141 | `native.size-rt-sisu` | {bytes} bytes ({share}%), the runtime in Sisu's code |
| D0142 | `native.size-rt-rust` | {bytes} bytes ({share}%), the runtime in Rust's code |
| D0143 | `native.size-other-code` | {bytes} bytes ({share}%), Rust's standard library's code and the rest of the code |
| D0144 | `native.size-debug` | {bytes} bytes ({share}%), the debug information |
| D0145 | `native.size-symbols` | {bytes} bytes ({share}%), the symbols |
| D0146 | `native.size-rest` | {bytes} bytes ({share}%), the data, the unwinding tables and the headers |
| D0147 | `native.size-unread` | `--explain-size` reads the parts of a 64-bit ELF executable, which `{output}` is not |
| [D0148](#d0148) | `cli.explain-size-native` | `--explain-size` explains the size of a native executable: it goes with `--native` |
| [D0149](#d0149) | `cli.token-module-run` | the token is for the run of the tests of `{module}`; `sisu test --replay {token} {module}` replays it |
| [D0150](#d0150) | `cli.token-package-run` | the token is for the run of the package's tests, not of `{module}`'s; `sisu test --replay {token} {package}` replays it |
| [D0151](#d0151) | `cli.command-usage` | usage: sisu {form}  {about} |
| [D0152](#d0152) | `cli.test-cached` | ok (cached) |
| D0170 | `cli.hole` | {at}: the typed hole {hole} of type `{ty}`, in `{function}` |
| D0171 | `cli.hole-scope` | in scope: {bindings} |
| D0172 | `cli.hole-scope-none` | nothing is in scope |
| D0173 | `cli.hole-fits` | of its type: {names} |
| D0174 | `cli.hole-fits-none` | nothing in scope is of its type |
| D0175 | `cli.fixed` | {at}: {code} fixed: {fix} |
| D0176 | `cli.fix-none` | nothing in `{file}` has a fix to apply |
| D0177 | `cli.fix-done` | fixes applied to `{file}`: {count} |
| D0178 | `cli.fix-would` | fixes that would be applied to `{file}`: {count}, and `--dry-run` leaves it as it is |
| D0179 | `cli.fix-undone` | the fixes at {at} would bring back a text an earlier round fixed, so they are not applied |
| D0180 | `cli.hole-struct` | `{name}` is a struct of {members} |
| D0181 | `cli.hole-struct-empty` | `{name}` is a struct without fields |
| D0182 | `cli.hole-enum` | `{name}` is an enum of {members} |
| D0183 | `cli.hole-enum-empty` | `{name}` is an enum without variants |
| D0184 | `cli.hole-requires` | given: `{clause}` |
| D0185 | `cli.hole-ensures` | to keep: `{clause}` |
| D0186 | `cli.hole-tests` | reached by the tests {tests} |
| D0187 | `cli.hole-tests-none` | no test reaches it |
| [D0190](#d0190) | `cli.fix-errors-left` | `{file}` does not check after the fixes: errors that no fix applies to are left ({count}), which `sisu check` shows |

