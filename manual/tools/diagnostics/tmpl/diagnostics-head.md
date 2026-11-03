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

@@output reading.console@@

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
