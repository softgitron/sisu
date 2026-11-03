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

Shown in [chapter 2](../tutorial/02-values.md).

## Parsing

Codes E0100 to E0127. The parser builds the syntax tree. Its errors name
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

Shown in [chapter 2](../tutorial/02-values.md).

### E0106

`parse.expected-declaration`: expected a declaration, found {found}.

Attributes and a visibility, `@NAME` and `pub`, stand before a
declaration; with none after them, the parser has nothing to attach
them to. An attribute before a statement is the common case
([R14](attributes.md#writing-an-attribute)).

<!-- file: examples/reference-diagnostics/e0106_attribute_alone.sisu -->
```sisu
# An attribute before a statement, where no declaration follows.
def main():
    @deprecated("use `total`")
    let total = 3 + 4
    print(total)
```

<!-- output: examples/reference-diagnostics/e0106_attribute_alone.console -->
```console
$ sisu run e0106_attribute_alone.sisu
error[E0106]: expected a declaration, found `let`
  --> e0106_attribute_alone.sisu:4:5
  |
4 |     let total = 3 + 4
  |     ^^^

error[D0006]: the program was not run: errors found (1)
```

Put the attribute before a declaration, or leave it out: a statement
takes none.

<!-- file: examples/reference-diagnostics/e0106_attribute_alone_fixed.sisu -->
```sisu
# The attribute stands before a declaration.
@deprecated("use `total`")
def seven() -> int:
    return 3 + 4

def main():
    print(seven())
```

<!-- output: examples/reference-diagnostics/e0106_attribute_alone_fixed.out -->
```text
7
```

Shown in [R14](attributes.md#writing-an-attribute).

### E0107

`parse.top-level-statement`: statements cannot appear at the top level
of a module; put them in a function such as `main`.

A module holds declarations; statements belong in functions. A script, a
file that starts with `#!`, is the exception: its top-level statements
are its `main`. Python's `class` is reported here too, since it is not a
declaration.

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
another language: a Rust comment `//`, `lambda`, `println!`.

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

## Names and declarations

Codes E0200 to E0269. Name resolution finds what each name means and
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

Shown in [chapter 1](../tutorial/01-hello.md).

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

error[D0006]: the program was not run: errors found (1)
```

Name the module by its file's path, here `src/geo/point.sisu`. The
diagnostic has no fix that names the similar module, which E0220 has
for a package's own name.

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
  = fix: write `dyn Priced`: `dyn `

error[D0006]: the program was not run: errors found (1)
```

The fix writes `dyn Priced`, which then needs `ref` or `Box` (E0395); a
bounded type parameter is the usual choice.

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
wrap the value in a type of its own.

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
in the package that declares the trait or the type (D-013); the root
package of a build, which nothing depends on, may implement any trait
for any type. Here the library `digits` implements the prelude's
`Len` for `int`, and the program `year` depends on it:

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

error[D0006]: the program was not run: errors found (1)
```

Wrap the value in a type of the library's own, and implement the trait
for that.

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
    let bench = Bench(["Aino"])
    print(len(bench.sitters))
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

Add the line the message gives.

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
its method `{method}` cannot be called through it, because {reason}.

A trait can be a `dyn` type only when every method can be called through
it: each takes `self` or `mut self`, and none names `Self` elsewhere
(D-017).

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
error[E0241]: `Shape` cannot be used as `dyn Shape`: its method `unit` cannot be called through it, because it does not take `self` or `mut self`
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
`Copy` and the traits that have a `comptime def derive`: `Debug`, `Eq`,
`PartialOrd`, `Ord`, `Hash`, and any trait that declares one.

`@derive` takes `Copy` and the traits that declare a `comptime def
derive`; the message lists them. `Display` has none, since only you know
how your type should read.

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
error[E0243]: `Display` cannot be derived; `@derive` takes `Copy` and the traits that have a `comptime def derive`: `Debug`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that declares one
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
made. The follow-on E0310 says that the operator then has no impl.

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

### E0266

`resolve.bad-cfg`: `@cfg` takes a condition: `test`, `feature="NAME"`,
`not` and a condition, or `all(...)` or `any(...)` of conditions; this
is none of them.

`@cfg` reads conditions on the build, not expressions, and keeps its
declaration when the condition is none of those. A test of the target
is written in a body, with `if comptime`, whose branches are both
checked; the message does not say so.

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
`join` of `Iterator`.

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

Use the parameter's name as the function declares it.

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

Shown in [chapter 5](../tutorial/05-collections.md).

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
error[E0335]: this function is passed where a pure function is required, but it calls `noisy_double`, which is not pure
  --> e0335_impure_argument.sisu:10:23
   |
10 |     print(apply_twice(noisy_double, 3))
   |                       ^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Pass a `pure` function. The message names the function passed,
`noisy_double`, rather than the `print` inside it that makes it impure.

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
  = fix: copy it with `.copy()`: `.copy()`

error[D0006]: the program was not run: errors found (1)
```

The fix copies the element, which costs a copy. To take it out instead,
use a method that removes it, such as `pop` or `remove`, on a `var`
collection.

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

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

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

Shown in [chapter 2](../tutorial/02-values.md).

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

## Ownership

Codes E0400 to E0437. The ownership checker checks moves, borrows,
views, tasks and must-consume values
([R7](ownership.md)).

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
  = note: copy it with `.copy()` before it is moved, or move it later

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
borrow ends at its last use, not at the end of its block.

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
reads.

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
  = note: return an owned value instead, such as a copy made with `.copy()`

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

error[D0006]: the program was not run: errors found (1)
```

The fix names every borrowed parameter; name fewer when the result
borrows from fewer.

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

`borrow.must-consume`: `{name}` is never consumed: a `{ty}` must be
passed to an `own` parameter before it goes out of scope.

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
error[E0417]: `note` is never consumed: a `Draft` must be passed to an `own` parameter before it goes out of scope
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
unconsumed: a `{ty}` must be passed to an `own` parameter first.

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
error[E0418]: this leaves the scope with `draft` unconsumed: a `Draft` must be passed to an `own` parameter first
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
  = note: move it into the task instead: write `own () -> ...` (or `do own:`), or copy what the task needs into a local before the `spawn`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Make the value outside the scope. The note's `own` task moves the value
into one task, which leaves none for a second.

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

## Lowering

Codes E0500 and E0501. Lowering writes the checked program as SIR,
the form that the interpreter and the native backend run, and checks
what it wrote; an error here is the compiler's mistake, which the
message reports without a place in the source.

### E0501

`sir.invalid`: invalid SIR in `{function}`: {detail}.

Lowering checks the SIR it writes before anything runs it; an invalid
program is the compiler's mistake, not the program's, and the message
has no place in the source. In stage 0 a default for a field of a
generic struct is one such case.

<!-- file: examples/reference-diagnostics/e0501_invalid_sir.sisu -->
```sisu
# A default for a field of a generic struct.
struct Shelf[T]:
    items: list[T] = []

def main():
    let shelf = Shelf[int]()
    print(len(shelf.items))
```

<!-- output: examples/reference-diagnostics/e0501_invalid_sir.console -->
```console
$ sisu run e0501_invalid_sir.sisu
error[E0501]: invalid SIR in `main`: the type of _2 names a type parameter the function does not have
```

Until it is fixed, build the value in an associated function.

<!-- file: examples/reference-diagnostics/e0501_invalid_sir_fixed.sisu -->
```sisu
# An associated function builds the empty shelf.
struct Shelf[T]:
    items: list[T]

    def empty() -> Shelf[T]:
        return Shelf(items=[])

def main():
    let shelf = Shelf[int].empty()
    print(len(shelf.items))
```

<!-- output: examples/reference-diagnostics/e0501_invalid_sir_fixed.out -->
```text
0
```

Shown in [chapter 15](../tutorial/15-generics.md).

## Compile-time evaluation

Codes E0600 to E0615. The compiler runs `comptime` code, templates and
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

### E0604

`comptime.not-kept`: {what} made at compile time cannot be kept for run
time; numbers, text, booleans, characters, and lists, sets, dicts,
tuples, options, results, structs and enums of them can.

The program keeps a compile-time value as a literal, so the value needs
a literal form; a function, a reference or a handle has none.

<!-- file: examples/reference-diagnostics/e0604_not_kept.sisu -->
```sisu
# A closure made at compile time, which a program cannot keep.
def adder(n: int) -> (int) -> int:
    return own x -> x + n

const ADD_TWO: (int) -> int = comptime adder(2)

def main():
    print(ADD_TWO(5))
```

<!-- output: examples/reference-diagnostics/e0604_not_kept.console -->
```console
$ sisu run e0604_not_kept.sisu
error[E0604]: a function made at compile time cannot be kept for run time; numbers, text, booleans, characters, and lists, sets, dicts, tuples, options, results, structs and enums of them can
  --> e0604_not_kept.sisu:5:31
  |
5 | const ADD_TWO: (int) -> int = comptime adder(2)
  |                               ^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

Compute the data at compile time, and make the function when the
program runs.

<!-- file: examples/reference-diagnostics/e0604_not_kept_fixed.sisu -->
```sisu
# The number is computed while compiling, the closure while running.
def adder(n: int) -> (int) -> int:
    return own x -> x + n

const STEP: int = comptime 1 + 1

def main():
    let add_two = adder(STEP)
    print(add_two(5))
```

<!-- output: examples/reference-diagnostics/e0604_not_kept_fixed.out -->
```text
7
```

Shown in [R12](compile-time.md#what-compile-time-code-may-do).

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

### E0609

`comptime.instance-failed`: making `{name}` stopped at compile time:
{reason}.

A template is made for each list of types it is called with; one that
stops, by `comptime panic` or a query that does not apply, is reported
at the call that needed it, with a label where the template stopped.

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

### E0812

`refine.change-through-base`: `{what}` is a `{ty}`, which a change
through its base could take outside `{predicate}`; change a copy of the
base and check it again with `try_from`.

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
error[E0812]: `picks` is a `Short`, which a change through its base could take outside `len(self) <= 3`; change a copy of the base and check it again with `try_from`
  --> e0812_change_through_base.sisu:6:5
  |
6 |     picks.push(3)
  |     ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Change a copy typed as the base, and check it again. A copy made with
`.copy()` alone is still a `Short`, and gets E0812 too.

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

Codes E0900 to E0990. What stage 0 does not implement yet, its internal
errors, the check that a `sim` build cannot escape, and the rules of
unsafe code and foreign functions
([R7](ownership.md#unsafe-code-and-raw-pointers)).

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

## Packages and manifests

Codes E1000 to E1022. The driver reads `sisu.toml`, `sisu.lock` and
the module tree of a package before anything is parsed, and reports a
mistake at its place in the manifest ([R13](modules.md#the-manifest)).

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
error[E1002]: `[package]` needs `version`
  --> sisu.toml:1:2
  |
1 | [package]
  |  ^^^^^^^

error[E1003]: `version` takes a version `MAJOR.MINOR.PATCH`, as in `"0.3.1"`, found `"1.0"`
  --> sisu.toml:3:11
  |
3 | version = "1.0"
  |           ^^^^^

```

Stage 0 also reports E1002 (`package.missing-key`), though the key is
there. Write the version in three parts.

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

## Warnings

Codes W0001 to W0016. A warning does not stop a program: `sisu run`
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

## Run time

Codes R0001 to R0153. Messages of a running program: panics, errors that
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

## The driver

Codes D0001 to D0126. The messages of the `sisu` command itself
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
| [E0002](#e0002) | `lex.inconsistent-dedent` | this line's indentation matches no enclosing block; the enclosing blocks start at column {columns} |
| [E0003](#e0003) | `lex.unterminated-string` | this string literal is not closed before the end of the line |
| E0004 | `lex.unterminated-block-string` | this multi-line string literal is never closed |
| E0005 | `lex.unknown-character` | `{char}` cannot appear in Sisu source outside a string or comment |
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
| E0110 | `parse.bitwise-arithmetic` | `{op}` cannot be mixed with arithmetic or shifts without parentheses |
| E0111 | `parse.shift-arithmetic` | a shift cannot be mixed with arithmetic without parentheses |
| E0112 | `parse.negative-power` | `-x ** y` is ambiguous; write `-(x ** y)` or `(-x) ** y` |
| [E0113](#e0113) | `parse.mixed-chain` | a chained comparison must point one way; `{first}` and `{second}` do not |
| E0114 | `parse.assignment-in-expression` | assignment is a statement and has no value; to compare, write `==` |
| [E0115](#e0115) | `parse.python-conditional` | `a if c else b` is not Sisu; write `if c: a else: b` |
| [E0116](#e0116) | `parse.if-expression-else` | an `if` expression needs an `else` branch, because it must have a value |
| [E0117](#e0117) | `parse.trailing-tokens` | unexpected {found} after the end of the statement |
| E0118 | `parse.arm-statement` | a `case` arm on one line holds a single expression; put statements in an indented block |
| E0119 | `parse.chain-operator` | `{op}` cannot be chained; use parentheses or `and` |
| E0121 | `parse.empty-interpolation` | an interpolation needs an expression between `{` and `}` |
| E0122 | `parse.closure-parameter` | a closure parameter must be a name |
| E0123 | `parse.walrus` | Sisu has no `:=`; bind with `let` or use `if let` |
| [E0124](#e0124) | `parse.keyword-as-name` | `{keyword}` is a keyword and cannot be used as a name here |
| E0125 | `parse.unexpected-indent` | this line is indented more than the line before it, which does not open a block |
| E0126 | `parse.expected-member` | a struct body holds fields, methods and `pass`; this line is none of them |
| [E0127](#e0127) | `parse.loop-else` | a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:` |

### Names and declarations, every code

| Code | Identifier | Message |
|---|---|---|
| [E0200](#e0200) | `resolve.undefined-name` | cannot find `{name}` in this scope |
| E0201 | `resolve.undefined-type` | cannot find a type named `{name}` |
| [E0202](#e0202) | `resolve.duplicate-definition` | `{name}` is defined more than once in this module |
| [E0203](#e0203) | `resolve.unknown-module` | there is no module `{name}` |
| E0204 | `resolve.not-in-module` | module `{module}` has no item `{name}` |
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
| E0220 | `resolve.import-unknown` | cannot import `{name}`: it names no module of this package, of a dependency in `sisu.toml`, or of the standard library |
| E0221 | `resolve.field-default-order` | a field with a default must not use the value of another field |
| E0222 | `resolve.parameter-default-order` | a parameter without a default cannot follow one with a default |
| E0223 | `resolve.not-a-module` | `{name}` is not a module |
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
| [E0241](#e0241) | `resolve.not-object-safe` | `{trait}` cannot be used as `dyn {trait}`: its method `{method}` cannot be called through it, because {reason} |
| [E0242](#e0242) | `resolve.supertrait-missing` | `{ty}` implements `{trait}`, which requires `{super}`, but `{ty}` does not implement `{super}` |
| [E0243](#e0243) | `resolve.not-derivable` | `{name}` cannot be derived; `@derive` takes `Copy` and the traits that have a `comptime def derive`: `Debug`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that declares one |
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
| E0255 | `resolve.private-item` | `{name}` is private to the module `{module}`; declare it `pub` to use it from another module |
| [E0256](#e0256) | `resolve.package-item` | `{name}` is `pub(package)` in `{module}`, so only the modules of the package `{package}` can use it |
| E0257 | `resolve.not-reexported` | `{name}` is imported into `{module}`, not declared there; import it from `{origin}`, or re-export it with `pub from {origin} import {name}` |
| [E0258](#e0258) | `resolve.private-field` | the field `{field}` of `{ty}` is private to the module `{module}`; declare it `pub` to use it from another module |
| E0259 | `resolve.private-method` | the method `{name}` of `{ty}` is private to the module `{module}`; declare it `pub` to call it from another module |
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
| E0333 | `type.impure-call` | `{callee}` is not pure, so the pure function `{name}` cannot call it |
| E0334 | `type.impure-capability` | the pure function `{name}` cannot use the capability `{ty}` |
| [E0335](#e0335) | `type.impure-argument` | this function is passed where a pure function is required, but it calls `{callee}`, which is not pure |
| [E0337](#e0337) | `type.missing-field` | missing the field `{field}` of `{ty}` |
| [E0338](#e0338) | `type.move-out-of-borrow` | cannot move `{what}` out of a borrowed value; copy it with `.copy()` |
| [E0339](#e0339) | `type.mut-argument-missing` | `{param}` is a `mut` parameter, so the call must say `mut` before the argument |
| [E0340](#e0340) | `type.mut-argument-unexpected` | `{param}` is not a `mut` parameter, so the argument takes no `mut` |
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
| [E0385](#e0385) | `type.bindings-in-or-pattern` | `{name}` is bound in some alternatives of `\|` but not in all; every alternative must bind the same names |
| E0386 | `type.variant-needs-fields` | `{name}` has fields; write `{name}(...)` |
| E0387 | `type.star-outside-list` | `*name` collects the rest of a list and appears only in a list pattern |
| E0388 | `type.rest-outside-arguments` | `..` skips the remaining fields and appears only inside `Name(...)` |
| [E0389](#e0389) | `type.no-self` | `self` exists only inside a method |
| E0390 | `type.not-a-struct-pattern` | `{name}` cannot be matched with fields |
| E0391 | `type.const-pattern` | a constant cannot be used as a pattern yet; compare with `==` in a guard |
| E0392 | `type.impure-value-call` | `{name}` is pure, so it cannot call a function value whose type is not `pure` |
| E0393 | `type.impure-drop` | dropping `{value}` here runs the `Drop` of `{ty}`, which is not pure, so the pure function `{name}` cannot do it |
| E0394 | `type.ambiguous-method` | `{name}` is a method of more than one trait `{ty}` implements ({traits}); call it on the trait, as in `Trait.{name}(x)` |
| [E0395](#e0395) | `type.unsized-dyn` | `{ty}` has no size of its own; hold it behind `ref` or in a `Box` |
| E0396 | `type.view-to-dyn` | `{ty}` borrows, so it cannot become `{dyn}`, which owns its value |
| E0397 | `type.must-consume-argument` | `{ty}` is must-consume, so it cannot be the argument of the generic parameter `{param}`, whose values the body may drop |
| [E0398](#e0398) | `type.structural-only` | {callee} is hashed and compared structurally in stage 0, and `{ty}` has an impl of `{trait}` of its own; derive `Hash` and `Eq` for it, or key by a field |
| E0399 | `type.bad-format-spec` | `{spec}` is not a format spec; it is `[[fill]align][sign][#][0][width][,\|_][.precision][type]`, as in `>8.2f`, with the type one of `d x X o b e E f F % s ?` |

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
| [E0417](#e0417) | `borrow.must-consume` | `{name}` is never consumed: a `{ty}` must be passed to an `own` parameter before it goes out of scope |
| [E0418](#e0418) | `borrow.must-consume-exit` | this leaves the scope with `{name}` unconsumed: a `{ty}` must be passed to an `own` parameter first |
| E0419 | `borrow.move-out-of-place` | cannot move out of `{name}` through a reference or an index; copy it with `.copy()` |
| E0420 | `borrow.must-consume-intrinsic` | `{callee}` may drop values that `{name}` holds, and a `{ty}` is must-consume: its values must be passed to an `own` parameter |
| E0421 | `borrow.view-in-owner` | `{ty}` borrows, so it cannot be put in `{owner}`, which can outlive what it borrows (04, rule 3) |
| E0422 | `borrow.must-consume-in-owner` | `{ty}` is must-consume, so it cannot be put in `{owner}`, which can drop it where the checker does not see |
| E0423 | `type.own-closure-capture` | an `own` closure keeps a copy of `{name}`, and `{ty}` is not `Copy`; stage 0 copies captures (a capture by move arrives with tasks) |
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

### Lowering, every code

| Code | Identifier | Message |
|---|---|---|
| E0500 | `sir.parse` | SIR: {detail} |
| [E0501](#e0501) | `sir.invalid` | invalid SIR in `{function}`: {detail} |

### Compile-time evaluation, every code

| Code | Identifier | Message |
|---|---|---|
| E0600 | `comptime.not-constant` | `comptime` evaluates what is known at compile time, and `{name}` is known only at run time |
| [E0601](#e0601) | `comptime.io` | the compile-time evaluation of this expression used {what}; nothing reaches the world at compile time |
| E0602 | `comptime.panicked` | the compile-time evaluation of this expression panicked: {message} |
| E0603 | `comptime.budget` | the compile-time evaluation in `{item}` used up its budget of {steps} steps; raise it with `@comptime_budget(steps=N)` on `{item}` |
| [E0604](#e0604) | `comptime.not-kept` | {what} made at compile time cannot be kept for run time; numbers, text, booleans, characters, and lists, sets, dicts, tuples, options, results, structs and enums of them can |
| E0605 | `comptime.only` | `{name}` is a `comptime def`: it can be called only at compile time, inside `comptime` |
| [E0606](#e0606) | `comptime.budget-attribute` | `@comptime_budget` takes `steps=N`, a positive number of steps |
| E0607 | `comptime.derive-template` | a trait's derive is `comptime def derive(T: type) -> impl {trait} for T`, whose body is `return impl {trait} for T:` with the impl's methods |
| E0608 | `comptime.derive-failed` | deriving `{trait}` for `{ty}` stopped at compile time: {reason} |
| [E0609](#e0609) | `comptime.instance-failed` | making `{name}` stopped at compile time: {reason} |
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
| [E0800](#e0800) | `contract.misplaced` | `{keyword}` is a function's contract and belongs at the start of its body, before its first statement |
| E0801 | `contract.not-pure` | a contract only reads what it is given, and `{callee}` is not `pure` |
| E0802 | `contract.changes` | a contract only reads what it is given, and this changes it |
| E0803 | `contract.old-outside` | `old(...)` is a value as it was on entry, which only an `ensures` clause can ask for |
| [E0804](#e0804) | `contract.old-arguments` | `old` takes one expression, as in `old(self.balance)` |
| [E0805](#e0805) | `contract.generator-ensures` | a generator has no `ensures`: what it returns is its iterator |
| E0806 | `contract.invariant-field` | `{what}` is part of a `{ty}`, whose invariant only its own methods may change and check again; change it through one of them |
| [E0807](#e0807) | `contract.not-in-test` | a test has no contract; check what it needs with `assert` |
| E0810 | `refine.not-satisfied` | `{value}` is not a `{ty}`: `{predicate}` is false for it |
| E0811 | `refine.needs-try-from` | expected `{expected}`, found `{found}`; a value becomes a `{expected}` where its predicate is checked, with `{name}.try_from(...)` |
| [E0812](#e0812) | `refine.change-through-base` | `{what}` is a `{ty}`, which a change through its base could take outside `{predicate}`; change a copy of the base and check it again with `try_from` |
| [E0813](#e0813) | `refine.own-impl` | `{ty}` refines `{base}` and has its reading impls; it cannot have an impl of `{trait}` of its own |
| [E0814](#e0814) | `refine.predicate-type` | the predicate of `{ty}` must be a `bool`, found `{found}` |
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
| [E0989](#e0989) | `unsafe.export-signature` | `{name}` is `@export`ed, and foreign code that calls it cannot keep `{ty}` alive: an exported function's signature holds no reference, view, `Shared`, closure or capability |
| [E0990](#e0990) | `unsafe.blocking-not-foreign` | `@blocking` marks a foreign function whose call may block its thread, and `{name}` is not a foreign function |

### Packages and manifests, every code

| Code | Identifier | Message |
|---|---|---|
| E1000 | `package.toml-syntax` | this is not TOML that `sisu.toml` reads: expected {expected}, found {found} |
| E1001 | `package.toml-duplicate` | `{key}` is given more than once in `{table}` |
| E1002 | `package.missing-key` | `{table}` needs `{key}` |
| [E1003](#e1003) | `package.bad-value` | `{key}` takes {expected}, found {found} |
| E1004 | `package.bad-name` | `{name}` cannot name a package: {why} |
| E1005 | `package.no-manifest` | the dependency `{name}` has no `sisu.toml` at `{path}` |
| E1006 | `package.renamed-dependency` | the dependency `{name}` is the package `{found}`; a path dependency is named by its own `[package] name` |
| [E1007](#e1007) | `package.cycle` | the packages depend on each other in a cycle: {cycle}; packages build in dependency order, so their graph has no cycle |
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
| [W0005](#w0005) | `package.unknown-key` | `{key}` is not a key of `{table}` that stage 0 knows; it is ignored |
| W0006 | `resolve.unknown-feature` | no `[features]` of this package declares `{name}`, so `feature="{name}"` never holds |
| W0007 | `unsafe.no-safety-comment` | this {what} has no `# safety:` comment above it saying why each obligation it takes on holds |
| W0008 | `unsafe.no-safety-list` | `{name}` is an `unsafe {kind}` without a `Safety:` list in its `##` comment naming the obligations its {who} take |
| W0009 | `unsafe.unused` | this {what} holds no unsafe operation |
| W0010 | `unsafe.unnamed-obligation` | the `# safety:` comment names no obligation of `{callee}`, which are {obligations} |
| W0011 | `unsafe.unexplained-purity` | this {what} calls `{callee}`, which is not `pure`, in the `pure` function `{name}`: its `# safety:` comment says why the function stays observationally pure, under a `pure:` part |
| W0012 | `unsafe.level` | the package declares `unsafe = "{level}"`, {why} |
| W0013 | `unsafe.auto-trait-bound` | `unsafe impl {trait}` claims `{trait}` for every `{param}`, which is not bounded by `{trait}` |
| W0014 | `unsafe.ambient-authority` | `{name}` is public and safe and reaches the foreign function `{callee}` without taking a capability, so its callers need no authority for what the call does |
| W0015 | `lint.wildcard-own-enum` | `case _` matches {variants} of `{ty}`, an enum of this package, so a variant added to `{ty}` later is not an error here |
| W0016 | `resolve.shadow-prelude-set` | the error set `{name}` hides the prelude's in this module, and the two are different sets: what the prelude's functions return is not this one |

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
| R0103 | `contract.refinement-failed` | `{value}` is not a `{ty}`: `{clause}` is false for it |
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
| D0120 | `cli.bad-lower` | `--lower` takes `stage2` (the lowering written in Sisu, the default) or `rust` (the Rust lowering, the oracle), not `{value}` |
| D0121 | `native.stage2-build` | building stage 2 in `{dir}` failed; `--lower=rust` lowers with the Rust backend: {detail} |
| D0122 | `native.stage2-failed` | internal compiler error: stage 2 failed to lower the program (status {status}); `--lower=rust` lowers with the Rust backend: {detail} |
| D0123 | `cli.bad-backend` | `--backend` takes `sisu` (the x86-64 backend written in Sisu, the debug profile's default) or `cranelift` (the Rust backend, the release profile's default and the oracle), not `{value}` |
| D0124 | `native.backend-lowering` | `--backend=sisu` compiles the LIR stage 2 lowers, so it does not go with `--lower=rust` |
| D0125 | `native.sisu-failed` | internal compiler error: the Sisu backend failed to compile the program (status {status}); `--backend=cranelift` compiles it with the Rust backend: {detail} |
| D0126 | `native.rt-build` | building the runtime in Sisu from `{seed}` failed: {detail} |

