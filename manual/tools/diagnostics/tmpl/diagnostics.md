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

@@file e0001_tab.sisu@@

@@output e0001_tab.console@@

The note asks for spaces in place of each tab; `sisu fmt` cannot do it,
because it formats only files that parse.

@@file e0001_tab_fixed.sisu@@

@@output e0001_tab_fixed.out@@

Shown in [chapter 1](../tutorial/01-hello.md).

### E0002

`lex.inconsistent-dedent`: this line's indentation matches no enclosing
block; the enclosing blocks start at column {columns}.

A line that is indented less than the one before closes blocks until its
indentation matches an enclosing block's. When it matches none, the
message names the columns that would.

@@file e0002_dedent.sisu@@

@@output e0002_dedent.console@@

Indent the line to one of the columns the message names: the block it
belongs to.

@@file e0002_dedent_fixed.sisu@@

@@output e0002_dedent_fixed.out@@

### E0003

`lex.unterminated-string`: this string literal is not closed before the
end of the line.

A one-line string literal ends on the line where it starts. A multi-line
literal is written between `"""` marks.

@@file e0003_open_string.sisu@@

@@output e0003_open_string.console@@

The fix closes the literal on its line, before the brackets that the
line opened ahead of the literal: here before the `)`, where the quote
belongs.

@@file e0003_open_string_fixed.sisu@@

@@output e0003_open_string_fixed.out@@

Shown in [chapter 1](../tutorial/01-hello.md).

### E0011

`lex.semicolon`: Sisu has no semicolons; put each statement on its own
line.

A statement takes one line, and a line break ends it; Sisu has no
semicolons (D-020).

@@file e0011_semicolon.sisu@@

@@output e0011_semicolon.console@@

Put each statement on its own line.

@@file e0011_semicolon_fixed.sisu@@

@@output e0011_semicolon_fixed.out@@

### E0012

`lex.invalid-number`: `{text}` is not a valid number literal.

A number literal is digits, with `_` between them, a prefix `0x`, `0b`
or `0o`, and a suffix that names a numeric type
([R1](lexical-structure.md#integer-literals)).

@@file e0012_number.sisu@@

@@output e0012_number.console@@

Write the number without the letters, or with a suffix such as `u8` or
`f32`.

@@file e0012_number_fixed.sisu@@

@@output e0012_number_fixed.out@@

### E0015

`lex.backslash-continuation`: Sisu has no backslash continuation; break
the line inside brackets, after a comma or before an operator.

A logical line continues inside brackets, after a comma or an open
bracket, and before a binary operator; a backslash does not continue it
(D-020).

@@file e0015_backslash.sisu@@

@@output e0015_backslash.console@@

Join the lines, or break the line in one of the three ways the message
names; `sisu fmt` joins it again when it fits in 100 columns.

@@file e0015_backslash_fixed.sisu@@

@@output e0015_backslash_fixed.out@@

### E0016

`lex.bad-char-literal`: a character literal holds exactly one character.

Single quotes make a `char`, one character; text is written between
double quotes.

@@file e0016_char_literal.sisu@@

@@output e0016_char_literal.console@@

The diagnostic offers no fix. Use double quotes for text, and keep
single quotes for one character, such as `'A'`.

@@file e0016_char_literal_fixed.sisu@@

@@output e0016_char_literal_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

## Parsing

Codes E0100 to E0126. The parser builds the syntax tree. Its errors name
what it expected and what it found, and several name a habit from
another language and the Sisu way to write it.

### E0100

`parse.expected`: expected {expected}, found {found}.

The parser expected a particular token and found another. The message
names both.

@@file e0100_no_colon.sisu@@

@@output e0100_no_colon.console@@

The fix writes the missing token where the parser expected it, the `:`
that opens the body here.

@@file e0100_no_colon_fixed.sisu@@

@@output e0100_no_colon_fixed.out@@

Shown in [chapter 1](../tutorial/01-hello.md).

### E0102

`parse.block-on-same-line`: a block starts on the next line, indented;
nothing may follow `:` on the line that opens it.

A block starts on the line after the `:` that opens it, indented
(D-027).

@@file e0102_same_line.sisu@@

@@output e0102_same_line.console@@

Move what follows the `:` to the next line and indent it.

@@file e0102_same_line_fixed.sisu@@

@@output e0102_same_line_fixed.out@@

### E0103

`parse.expected-expression`: expected an expression, found {found}.

An expression was expected, and something else stood there: the end of
the line, a keyword, or a statement. A line ends a statement unless a
bracket is open.

@@file e0103_no_operand.sisu@@

@@output e0103_no_operand.console@@

There is no fix to offer, since only you know the missing expression. To
break a long expression, break it inside brackets.

@@file e0103_no_operand_fixed.sisu@@

@@output e0103_no_operand_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0105

`parse.expected-pattern`: expected a pattern, found {found}.

A pattern was expected: a literal, a name, `_`, a variant, a tuple, a
list or a struct pattern, or alternatives with `|`. An operator such as
`not` is not a pattern.

@@file e0105_pattern.sisu@@

@@output e0105_pattern.console@@

Write the case as a pattern; a `case _` arm, or a guard `case n if n !=
0`, says "any other value".

@@file e0105_pattern_fixed.sisu@@

@@output e0105_pattern_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0107

`parse.top-level-statement`: statements cannot appear at the top level
of a module; put them in a function such as `main`.

A module holds declarations; statements belong in functions. A script, a
file that starts with `#!`, is the exception: its top-level statements
are its `main`. Python's `class` is reported here too, since it is not a
declaration.

@@file e0107_top_level.sisu@@

@@output e0107_top_level.console@@

Move the statement into `main` or another function, or make the file a
script.

@@file e0107_top_level_fixed.sisu@@

@@output e0107_top_level_fixed.out@@

### E0108

`parse.mixed-and-or`: `and` and `or` cannot be mixed without
parentheses.

`and` and `or` do not mix without parentheses, so that no reader has to
remember which binds tighter.

@@file e0108_and_or.sisu@@

@@output e0108_and_or.console@@

Add the parentheses that say what you mean.

@@file e0108_and_or_fixed.sisu@@

@@output e0108_and_or_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0113

`parse.mixed-chain`: a chained comparison must point one way; `{first}`
and `{second}` do not.

A chain of comparisons such as `0 < x <= 5` reads as one range, so all
its operators point the same way.

@@file e0113_chain.sisu@@

@@output e0113_chain.console@@

Write the chain one way, or join two comparisons with `and`.

@@file e0113_chain_fixed.sisu@@

@@output e0113_chain_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0115

`parse.python-conditional`: `a if c else b` is not Sisu; write `if c: a
else: b`.

Sisu's conditional expression starts with `if`: `if c: a else: b`.
Python's `a if c else b` does not compile, so that it never means
something else (D-062).

@@file e0115_python_if.sisu@@

@@output e0115_python_if.console@@

The fix rewrites the expression in Sisu's order, which is what you want.

@@file e0115_python_if_fixed.sisu@@

@@output e0115_python_if_fixed.out@@

Shown in [chapter 3](../tutorial/03-functions.md).

### E0116

`parse.if-expression-else`: an `if` expression needs an `else` branch,
because it must have a value.

An `if` used as an expression has a value on every path, so it has an
`else` branch.

@@file e0116_if_without_else.sisu@@

@@output e0116_if_without_else.console@@

Add the `else` branch with the value for the other case.

@@file e0116_if_without_else_fixed.sisu@@

@@output e0116_if_without_else_fixed.out@@

Shown in [chapter 3](../tutorial/03-functions.md).

### E0117

`parse.trailing-tokens`: unexpected {found} after the end of the
statement.

After a statement ends, only a line break may follow. The parser reports
the first token it did not expect, which often stands for a habit from
another language: a Rust comment `//`, `lambda`, `println!`.

@@file e0117_trailing.sisu@@

@@output e0117_trailing.console@@

End the statement where it ends, and put what follows on a line of its
own, in Sisu's spelling.

@@file e0117_trailing_fixed.sisu@@

@@output e0117_trailing_fixed.out@@

### E0124

`parse.keyword-as-name`: `{keyword}` is a keyword and cannot be used as
a name here.

A keyword is not a name, except after `.`, `?.` and `def`
([R1](lexical-structure.md#keywords)).

@@file e0124_keyword.sisu@@

@@output e0124_keyword.console@@

Choose another name. A keyword elsewhere where a name should be gets the
parser's E0100, E0103 or E0105 instead.

@@file e0124_keyword_fixed.sisu@@

@@output e0124_keyword_fixed.out@@

## Names and declarations

Codes E0200 to E0266. Name resolution finds what each name means and
checks declarations: undefined and duplicate names, imports and
visibility, derives, traits and impls.

### E0200

`resolve.undefined-name`: cannot find `{name}` in this scope.

A name is used that nothing in scope declares: no binding, parameter,
item, import or prelude name. A binding is declared with `let` or `var`,
not by assigning to a new name.

@@file e0200_misspelt.sisu@@

@@output e0200_misspelt.console@@

When a name in scope is spelt alike, the fix offers it. When the name is
new, declare it with `let` (or `var`) before its first use.

@@file e0200_misspelt_fixed.sisu@@

@@output e0200_misspelt_fixed.out@@

Shown in [chapter 1](../tutorial/01-hello.md).

### E0202

`resolve.duplicate-definition`: `{name}` is defined more than once in
this module.

A module declares each name once. Sisu has no overloading (D-050), so
two functions with one name are an error even when their parameters
differ.

@@file e0202_twice.sisu@@

@@output e0202_twice.console@@

The note asks you to rename one or remove the one not wanted; a name
that says what each computes is the usual way.

@@file e0202_twice_fixed.sisu@@

@@output e0202_twice_fixed.out@@

Shown in [chapter 3](../tutorial/03-functions.md).

### E0211

`resolve.ambiguous-variant`: `{name}` is a variant of more than one type
({candidates}); qualify it, as in `{example}`.

A variant's own name is enough where only one enum has a variant of that
name. Where several have, the bare name is ambiguous.

@@file e0211_ambiguous_variant.sisu@@

@@output e0211_ambiguous_variant.console@@

Qualify the variant with its type, as the message shows, or let a known
type decide it: a parameter of type `Light` takes `Red`.

@@file e0211_ambiguous_variant_fixed.sisu@@

@@output e0211_ambiguous_variant_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0215

`resolve.self-outside-method`: `self` exists only inside a method.

`self` is the first parameter of a method, in the body of a `struct`,
`enum`, `trait` or `impl`; as a parameter of any other function it is
E0215, and as a value outside a method E0389.

@@file e0215_self.sisu@@

@@output e0215_self.console@@

Leave `self` out of a free function, or move the function into the
type's body as a method.

@@file e0215_self_fixed.sisu@@

@@output e0215_self_fixed.out@@

### E0231

`resolve.trait-as-type`: `{name}` is a trait, not a type; a value of a
type that implements it is `dyn {name}`.

A trait is not a type. A value of some type that implements it is a type
parameter with the trait as its bound, or a `dyn` value behind `ref` or
`Box`.

@@file e0231_trait_as_type.sisu@@

@@output e0231_trait_as_type.console@@

The fix writes `dyn Priced`, which then needs `ref` or `Box` (E0395); a
bounded type parameter is the usual choice.

@@file e0231_trait_as_type_fixed.sisu@@

@@output e0231_trait_as_type_fixed.out@@

Shown in [chapter 14](../tutorial/14-traits.md).

### E0232

`resolve.missing-trait-method`: the impl of `{trait}` for `{ty}` is
missing {methods}.

An `impl` of a trait writes every method the trait declares without a
body.

@@file e0232_missing_method.sisu@@

@@output e0232_missing_method.console@@

Write the method the note names, with the signature it gives.

@@file e0232_missing_method_fixed.sisu@@

@@output e0232_missing_method_fixed.out@@

Shown in [chapter 14](../tutorial/14-traits.md).

### E0235

`resolve.overlapping-impl`: `{trait}` is implemented for `{ty}` more
than once; a type implements a trait at most once.

A type implements a trait at most once, so that a call never has two
meanings (D-013).

@@file e0235_overlap.sisu@@

@@output e0235_overlap.console@@

Keep one impl. For a second way to show the value, write a method, or
wrap the value in a type of its own.

@@file e0235_overlap_fixed.sisu@@

@@output e0235_overlap_fixed.out@@

Shown in [chapter 14](../tutorial/14-traits.md).

### E0241

`resolve.not-object-safe`: `{trait}` cannot be used as `dyn {trait}`:
its method `{method}` cannot be called through it, because {reason}.

A trait can be a `dyn` type only when every method can be called through
it: each takes `self` or `mut self`, and none names `Self` elsewhere
(D-017).

@@file e0241_object_safety.sisu@@

@@output e0241_object_safety.console@@

Move the method out of the trait, into the type or another trait, or use
a type parameter instead of `dyn`.

@@file e0241_object_safety_fixed.sisu@@

@@output e0241_object_safety_fixed.out@@

Shown in [chapter 15](../tutorial/15-generics.md).

### E0242

`resolve.supertrait-missing`: `{ty}` implements `{trait}`, which
requires `{super}`, but `{ty}` does not implement `{super}`.

A trait that names a supertrait, as `trait Labelled(Display)`, can be
implemented only for types that implement the supertrait.

@@file e0242_supertrait.sisu@@

@@output e0242_supertrait.console@@

Implement the supertrait for the type as well.

@@file e0242_supertrait_fixed.sisu@@

@@output e0242_supertrait_fixed.out@@

Shown in [chapter 14](../tutorial/14-traits.md).

### E0243

`resolve.not-derivable`: `{name}` cannot be derived; `@derive` takes
`Copy` and the traits that have a `comptime def derive`: `Debug`, `Eq`,
`PartialOrd`, `Ord`, `Hash`, and any trait that declares one.

`@derive` takes `Copy` and the traits that declare a `comptime def
derive`; the message lists them. `Display` has none, since only you know
how your type should read.

@@file e0243_not_derivable.sisu@@

@@output e0243_not_derivable.console@@

Write an `impl` of the trait.

@@file e0243_not_derivable_fixed.sisu@@

@@output e0243_not_derivable_fixed.out@@

Shown in [chapter 14](../tutorial/14-traits.md).

### E0244

`resolve.derive-field`: `{ty}` cannot derive `{trait}`: its field
`{field}` has type `{fty}`, which does not implement `{trait}`.

A derive needs every field to have the trait it derives. A `String` is
not `Copy`, since copying it takes new memory, which Sisu does only when
asked.

@@file e0244_derive_copy.sisu@@

@@output e0244_derive_copy.console@@

Leave `Copy` out and copy with `.copy()` where a copy is wanted, or make
the field a type that is `Copy`.

@@file e0244_derive_copy_fixed.sisu@@

@@output e0244_derive_copy_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

## Types

Codes E0300 to E0399. The type checker checks every expression's type:
mismatches, calls, operators, `match`, mutability, indexing, traits and
generics.

### E0300

`type.mismatch`: expected `{expected}`, found `{found}`.

A value of one type stands where another is expected. A label points at
what made the other type expected.

@@file e0300_mismatch.sisu@@

@@output e0300_mismatch.console@@

The fix depends on the types; here it writes the number without its
quotes. Where text must become a number, `parse_int()` converts it and
says when it cannot.

@@file e0300_mismatch_fixed.sisu@@

@@output e0300_mismatch_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0301

`type.integer-division`: `/` divides floats; for integers use `//`, or
convert to `f64` first.

`/` divides floating-point numbers; integers are divided with `//`,
which rounds towards negative infinity, and `%` gives the remainder.

@@file e0301_division.sisu@@

@@output e0301_division.console@@

The fix writes `//`. To divide with a fraction, convert first:
`pieces.to_f64() / 2.0`.

@@file e0301_division_fixed.sisu@@

@@output e0301_division_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0302

`type.not-bool`: a condition must be a `bool`, not `{found}`; Sisu has
no truthiness.

A condition is a `bool`. Sisu has no truthiness: a number, a text or a
collection is not true or false by itself (D-025).

@@file e0302_truthiness.sisu@@

@@output e0302_truthiness.console@@

Write the test you mean, such as `count != 0` or `not names.is_empty()`.

@@file e0302_truthiness_fixed.sisu@@

@@output e0302_truthiness_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0304

`type.unknown-method`: `{ty}` has no method `{name}`.

A method call names a method of the value's type or of a trait it
implements. A note names the trait when another type has the method, as
`join` of `Iterator`.

@@file e0304_method.sisu@@

@@output e0304_method.console@@

Use the type's method; [R19](false-friends.md) lists the Python and Rust
names that differ.

@@file e0304_method_fixed.sisu@@

@@output e0304_method_fixed.out@@

Shown in [chapter 15](../tutorial/15-generics.md).

### E0305

`type.argument-count`: wrong number of arguments to `{name}`: expected
{expected}, found {found}.

A call passes as many arguments as the function has parameters, less
those with defaults. `print` takes one text.

@@file e0305_argument_count.sisu@@

@@output e0305_argument_count.console@@

There is no fix to offer. For `print`, build one text with
interpolation.

@@file e0305_argument_count_fixed.sisu@@

@@output e0305_argument_count_fixed.out@@

Shown in [chapter 1](../tutorial/01-hello.md).

### E0306

`type.unknown-keyword`: `{name}` has no parameter named `{arg}`.

A keyword argument names a parameter of the function.

@@file e0306_keyword_argument.sisu@@

@@output e0306_keyword_argument.console@@

Use the parameter's name as the function declares it.

@@file e0306_keyword_argument_fixed.sisu@@

@@output e0306_keyword_argument_fixed.out@@

### E0308

`type.missing-argument`: missing the argument `{arg}` of `{name}`.

A parameter without a default needs an argument at every call.

@@file e0308_missing_argument.sisu@@

@@output e0308_missing_argument.console@@

The fix passes a typed hole, `todo("height")`, which compiles and stops
the program when it is reached; fill it with the value.

@@file e0308_missing_argument_fixed.sisu@@

@@output e0308_missing_argument_fixed.out@@

Shown in [chapter 5](../tutorial/05-collections.md).

### E0310

`type.binary-operator`: `{op}` cannot be applied to `{lhs}` and `{rhs}`.

A binary operator is defined for some pairs of types only. `+` joins two
texts, and adds two numbers of one type.

@@file e0310_operator.sisu@@

@@output e0310_operator.console@@

For a text and a value, the fix writes the value into the text by
interpolation.

@@file e0310_operator_fixed.sisu@@

@@output e0310_operator_fixed.out@@

Shown in [chapter 8](../tutorial/08-errors.md).

### E0312

`type.assign-immutable`: `{name}` is immutable; declare it with `var` to
assign to it.

A `let` binding keeps its value; only a `var` binding, a `mut` parameter
or a field of one can be assigned.

@@file e0312_assign_let.sisu@@

@@output e0312_assign_let.console@@

The fix declares the binding with `var`. For a parameter, it makes the
parameter `mut`, which changes every call; a local `var` copy of it is
often what you want instead.

@@file e0312_assign_let_fixed.sisu@@

@@output e0312_assign_let_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0314

`type.non-exhaustive`: this `match` does not cover {missing}.

A `match` covers every value of its subject's type, so that adding a
variant shows every `match` that must handle it.

@@file e0314_non_exhaustive.sisu@@

@@output e0314_non_exhaustive.console@@

Add an arm for the case the note names. A `case _` arm also covers it,
and every variant added later.

@@file e0314_non_exhaustive_fixed.sisu@@

@@output e0314_non_exhaustive_fixed.out@@

Shown in [chapter 4](../tutorial/04-control-flow.md).

### E0315

`type.missing-return`: `{name}` returns `{ty}`, but control can reach
the end of its body without a `return`.

A function with a result type returns a value on every path; a value
leaves a function only through `return` (D-056).

@@file e0315_missing_return.sisu@@

@@output e0315_missing_return.console@@

Return a value on the path that has none, often with a last `return` at
the end of the body.

@@file e0315_missing_return_fixed.sisu@@

@@output e0315_missing_return_fixed.out@@

Shown in [chapter 3](../tutorial/03-functions.md).

### E0318

`type.unused-result`: this `{ty}` is not used; handle it, or discard it
with `_ = ...`.

A value that can hold an error or an absence must be used, so that no
failure passes unseen.

@@file e0318_unused_result.sisu@@

@@output e0318_unused_result.console@@

Handle it, with `?`, `else` or `match`, or discard it on purpose with `_
= ...`, which the fix writes.

@@file e0318_unused_result_fixed.sisu@@

@@output e0318_unused_result_fixed.out@@

Shown in [chapter 8](../tutorial/08-errors.md).

### E0320

`type.try-context`: `?` on `{ty}` needs a function that returns
`{needed}`, but this one returns `{found}`.

`?` returns early from the function with the absence or the error, so
the function's result must be able to hold it: `T?` for an absence, `T !
E` for an error.

@@file e0320_try_context.sisu@@

@@output e0320_try_context.console@@

Give the function a result type that can hold it, as `def main() -> ! E`
for an error, or handle the value where it is, with `else`.

@@file e0320_try_context_fixed.sisu@@

@@output e0320_try_context_fixed.out@@

Shown in [chapter 8](../tutorial/08-errors.md).

### E0325

`type.not-iterable`: a `for` loop cannot iterate over `{ty}`.

A `for` loop takes a value that can be iterated: a collection, a range
or an iterator. A text is not, because it can be read as characters,
bytes or lines.

@@file e0325_not_iterable.sisu@@

@@output e0325_not_iterable.console@@

Say which: `.chars()`, `.bytes()` or `.lines()`.

@@file e0325_not_iterable_fixed.sisu@@

@@output e0325_not_iterable_fixed.out@@

Shown in [chapter 6](../tutorial/06-text.md).

### E0327

`type.unknown-variant`: `{ty}` has no variant `{name}`.

A capitalised name in a pattern is a variant of the subject's type, and
that type has none of that name.

@@file e0327_unknown_variant.sisu@@

@@output e0327_unknown_variant.console@@

Name a variant the type has, or add it to the enum. A lowercase name in
a pattern binds the value instead.

@@file e0327_unknown_variant_fixed.sisu@@

@@output e0327_unknown_variant_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0328

`type.pattern-field-count`: wrong number of fields in the pattern for
`{name}`: expected {expected}, found {found}.

A struct or variant pattern gives a pattern for every field, by position
or by keyword, unless it ends with `..`, which stands for the fields it
leaves out.

@@file e0328_field_count.sisu@@

@@output e0328_field_count.console@@

Add `..` at the end of the pattern, or a pattern for each field; `_` for
a field matches any value.

@@file e0328_field_count_fixed.sisu@@

@@output e0328_field_count_fixed.out@@

### E0329

`type.refutable-let`: this pattern might not match; add `else:` with a
block that leaves the scope.

The pattern of a plain `let` matches every value. A pattern that might
not match needs `else:` with a block that leaves the scope.

@@file e0329_refutable_let.sisu@@

@@output e0329_refutable_let.console@@

Add `else:` with a `return`, `break`, `continue` or a panic, or use `if
let` or `match`.

@@file e0329_refutable_let_fixed.sisu@@

@@output e0329_refutable_let_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0337

`type.missing-field`: missing the field `{field}` of `{ty}`.

Building a struct gives a value to every field that has no default.

@@file e0337_missing_field.sisu@@

@@output e0337_missing_field.console@@

The fix passes a typed hole for the field; give the value, or give the
field a default in the struct.

@@file e0337_missing_field_fixed.sisu@@

@@output e0337_missing_field_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0338

`type.move-out-of-borrow`: cannot move `{what}` out of a borrowed value;
copy it with `.copy()`.

Indexing reads through a borrow of the collection, so it cannot move an
element that is not `Copy` out of it: the collection would be left with
a hole.

@@file e0338_move_out_of_borrow.sisu@@

@@output e0338_move_out_of_borrow.console@@

The fix copies the element, which costs a copy. To take it out instead,
use a method that removes it, such as `pop` or `remove`, on a `var`
collection.

@@file e0338_move_out_of_borrow_fixed.sisu@@

@@output e0338_move_out_of_borrow_fixed.out@@

Shown in [chapter 10](../tutorial/10-ownership.md).

### E0339

`type.mut-argument-missing`: `{param}` is a `mut` parameter, so the call
must say `mut` before the argument.

A call passes an argument to a `mut` parameter with `mut` before it, so
that the reader of the call sees what can change (D-006).

@@file e0339_mut_at_call.sisu@@

@@output e0339_mut_at_call.console@@

The fix writes `mut` before the argument.

@@file e0339_mut_at_call_fixed.sisu@@

@@output e0339_mut_at_call_fixed.out@@

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0341

`type.not-mutable`: cannot modify `{what}`; it is not declared with
`var`.

A method that changes its value (`mut self`), or a change to a field,
needs a value that can change: a `var` binding, a `mut` parameter, or a
`mut` borrow.

@@file e0341_not_mutable.sisu@@

@@output e0341_not_mutable.console@@

The fix declares the binding with `var`. For a method of your own that
changes `self`, the first parameter is `mut self`.

@@file e0341_not_mutable_fixed.sisu@@

@@output e0341_not_mutable_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0344

`type.constant-negative-index`: a constant negative index does not
compile; count from the end with `^`: `{fix}`.

Sisu counts from the end with `^`: `xs[^1]` is the last element. A
constant negative index does not compile, so that Python's `xs[-1]`
never means something else (D-062).

@@file e0344_negative_index.sisu@@

@@output e0344_negative_index.console@@

The fix writes `^1`; an index computed at run time that is negative
panics instead.

@@file e0344_negative_index_fixed.sisu@@

@@output e0344_negative_index_fixed.out@@

Shown in [chapter 5](../tutorial/05-collections.md).

### E0345

`type.str-not-indexable`: `{ty}` cannot be indexed by position; use
`.chars()`, `.bytes()` or `.byte_slice(a, b)`.

A `str` has no integer index, since its characters take different
numbers of bytes (D-034).

@@file e0345_str_index.sisu@@

@@output e0345_str_index.console@@

Iterate its `.chars()` or `.bytes()`, or take a part with
`.byte_slice(a, b)` at known character boundaries.

@@file e0345_str_index_fixed.sisu@@

@@output e0345_str_index_fixed.out@@

Shown in [chapter 6](../tutorial/06-text.md).

### E0347

`type.bound-not-satisfied`: `{ty}` does not implement `{bound}`.

A type is used where a trait is required, and does not implement it.
`==` needs `Eq`, sorting needs `Ord`, a set or a dict key needs `Hash`.

@@file e0347_bound.sisu@@

@@output e0347_bound.console@@

Derive the trait with `@derive(...)` on the type, or write an `impl` of
it.

@@file e0347_bound_fixed.sisu@@

@@output e0347_bound_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0368

`type.not-ordered`: `{ty}` has no order, so it cannot be compared with
`{op}` or sorted.

`<`, `<=`, `>`, `>=`, `sorted`, `min` and `max` need an order, the trait
`Ord`.

@@file e0368_not_ordered.sisu@@

@@output e0368_not_ordered.console@@

Derive `Ord` (with `Eq`), which compares the fields in the order they
are declared, or write an `impl`.

@@file e0368_not_ordered_fixed.sisu@@

@@output e0368_not_ordered_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0373

`type.mixed-numbers`: `{op}` cannot mix `{lhs}` and `{rhs}`; convert one
of them explicitly.

An arithmetic operator takes two numbers of one type; Sisu converts
between `int` and `f64` only when you say so.

@@file e0373_mixed_numbers.sisu@@

@@output e0373_mixed_numbers.console@@

The fix converts the integer with `.to_f64()`; to go the other way,
`round()` or `truncate_to_int()` says how.

@@file e0373_mixed_numbers_fixed.sisu@@

@@output e0373_mixed_numbers_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0374

`type.literal-out-of-range`: `{text}` does not fit in `{ty}`.

An integer literal fits in the type it gets. The note gives the type's
range.

@@file e0374_out_of_range.sisu@@

@@output e0374_out_of_range.console@@

Use a value in the range, or a wider type such as `int`.

@@file e0374_out_of_range_fixed.sisu@@

@@output e0374_out_of_range_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### E0385

`type.bindings-in-or-pattern`: `{name}` is bound in some alternatives of
`\|` but not in all; every alternative must bind the same names.

The alternatives of `|` run the same arm, so every alternative binds the
same names, with the same types.

@@file e0385_or_bindings.sisu@@

@@output e0385_or_bindings.console@@

Bind the name in every alternative, or give the alternatives arms of
their own.

@@file e0385_or_bindings_fixed.sisu@@

@@output e0385_or_bindings_fixed.out@@

### E0389

`type.no-self`: `self` exists only inside a method.

`self` names the value a method is called on, so it exists only inside a
method.

@@file e0389_self.sisu@@

@@output e0389_self.console@@

Pass the value as a parameter, or move the function into the type's body
as a method.

@@file e0389_self_fixed.sisu@@

@@output e0389_self_fixed.out@@

### E0395

`type.unsized-dyn`: `{ty}` has no size of its own; hold it behind `ref`
or in a `Box`.

A `dyn Trait` value can be of any size, since any type that implements
the trait can stand there, so it is held behind `ref` or in a `Box`.

@@file e0395_bare_dyn.sisu@@

@@output e0395_bare_dyn.console@@

Hold it behind `ref`, or in a `Box` to own it.

@@file e0395_bare_dyn_fixed.sisu@@

@@output e0395_bare_dyn_fixed.out@@

Shown in [chapter 15](../tutorial/15-generics.md).

## Ownership

Codes E0400 to E0437. The ownership checker checks moves, borrows,
views, tasks and must-consume values ([R7](../README.md#the-reference),
when it is written).

### E0400

`borrow.use-after-move`: `{name}` was moved on line {line} and cannot be
used afterwards.

A value has one owner. Assigning a value that is not `Copy` moves it,
and the old name can no longer be used.

@@file e0400_moved.sisu@@

@@output e0400_moved.console@@

The note offers two ways: copy the value before the move, which costs a
copy, or use the old name before the move. Reading through a borrow
instead of moving is the third.

@@file e0400_moved_fixed.sisu@@

@@output e0400_moved_fixed.out@@

Shown in [chapter 5](../tutorial/05-collections.md).

### E0401

`borrow.use-after-partial-move`: `{name}` cannot be used as a whole,
because `{part}` was moved out of it on line {line}.

Moving a field out of a struct leaves the struct without it: the other
fields can still be used, and the struct as a whole cannot.

@@file e0401_partial_move.sisu@@

@@output e0401_partial_move.console@@

Use the whole struct before the move, as the note says, or copy the
field with `.copy()` and leave the struct whole.

@@file e0401_partial_move_fixed.sisu@@

@@output e0401_partial_move_fixed.out@@

Shown in [chapter 10](../tutorial/10-ownership.md).

### E0402

`borrow.modify-while-borrowed`: `{name}` is borrowed on line {line} and
cannot be modified until that borrow ends on line {end}.

A value that is borrowed cannot change until the borrow's last use. A
borrow ends at its last use, not at the end of its block.

@@file e0402_modify_while_borrowed.sisu@@

@@output e0402_modify_while_borrowed.console@@

Move the change after the line the note names, or work on a copy.

@@file e0402_modify_while_borrowed_fixed.sisu@@

@@output e0402_modify_while_borrowed_fixed.out@@

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0405

`borrow.mut-while-borrowed`: `{name}` is already borrowed on line
{line}, so it cannot be borrowed with `mut` until that borrow ends on
line {end}.

A value has either one `mut` borrow or any number of shared ones at a
time. A call of a `mut self` method borrows with `mut`.

@@file e0405_mut_while_borrowed.sisu@@

@@output e0405_mut_while_borrowed.console@@

Finish with the shared borrow first, as the note says, or copy what it
reads.

@@file e0405_mut_while_borrowed_fixed.sisu@@

@@output e0405_mut_while_borrowed_fixed.out@@

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0406

`borrow.does-not-live-long-enough`: `{name}` does not live long enough:
it is borrowed on line {line}, and the borrow is still used on line
{end}, after `{name}` is gone.

A borrow cannot be used after what it borrows is gone. The message names
the line of the borrow and the line of its last use.

@@file e0406_short_life.sisu@@

@@output e0406_short_life.console@@

Declare the borrowed value where it lives long enough, as the note says,
or move a copy into what outlives it: an `own` closure, or `.copy()`.

@@file e0406_short_life_fixed.sisu@@

@@output e0406_short_life_fixed.out@@

### E0407

`borrow.return-local`: this returns a borrow of `{name}`, which belongs
to `{function}` and is gone when it returns.

A function's locals are gone when it returns, so it cannot return a
borrow of one; `str` is a borrow, and an interpolated literal is a local
`String`.

@@file e0407_return_local.sisu@@

@@output e0407_return_local.console@@

Return an owned value: here, make the result type `String`.

@@file e0407_return_local_fixed.sisu@@

@@output e0407_return_local_fixed.out@@

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0411

`borrow.view-field-in-plain-struct`: `{ty}` holds `{field}: {fty}`,
which borrows; only a `view struct` may hold a borrow.

A plain struct owns all its fields; a field that borrows, such as `str`,
could outlive what it borrows. Only a `view struct` may hold a borrow.

@@file e0411_view_field.sisu@@

@@output e0411_view_field.console@@

The fix makes the struct a `view struct`, which limits where its values
can go. Usually the field should own its value: `String` for `str`.

@@file e0411_view_field_fixed.sisu@@

@@output e0411_view_field_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

### E0413

`borrow.from-required`: `{name}` returns `{ret}`, which borrows, and has
more than one borrowed parameter ({params}); name the ones it borrows
from with `from`.

A function that returns a borrow and has one borrowed parameter returns
a borrow of it. With more than one, `from` names those the result may
borrow from, so a caller knows what must outlive the result.

@@file e0413_from.sisu@@

@@output e0413_from.console@@

The fix names every borrowed parameter; name fewer when the result
borrows from fewer.

@@file e0413_from_fixed.sisu@@

@@output e0413_from_fixed.out@@

Shown in [chapter 11](../tutorial/11-borrowing.md).

### E0417

`borrow.must-consume`: `{name}` is never consumed: a `{ty}` must be
passed to an `own` parameter before it goes out of scope.

A `@must_consume` type cannot be dropped silently: each value is passed
to an `own` parameter before it goes out of scope (D-054).

@@file e0417_must_consume.sisu@@

@@output e0417_must_consume.console@@

Consume it with one of the methods the note names, each of which says
what ending it means.

@@file e0417_must_consume_fixed.sisu@@

@@output e0417_must_consume_fixed.out@@

Shown in [chapter 13](../tutorial/13-sharing.md).

## Warnings

Codes W0001 to W0014. A warning does not stop a program: `sisu run`
prints it and runs.

### W0001

`type.hole`: typed hole `{name}` of type `{ty}`.

A typed hole, `todo("name")`, stands for code still to be written
(D-061). The program compiles and runs until it reaches the hole, which
panics; `sisu test` reports a test that reaches one as blocked.

@@file w0001_hole.sisu@@

@@output w0001_hole.console@@

Write the code the hole stands for.

@@file w0001_hole_fixed.sisu@@

@@output w0001_hole_fixed.out@@

Shown in [chapter 1](../tutorial/01-hello.md).

### W0002

`resolve.shadow-outer`: `{name}` shadows a binding of an enclosing
scope.

A binding in an inner block with the name of a binding of an enclosing
block hides it until the inner block ends (D-024). Rebinding in the same
block does not warn.

@@file w0002_shadow.sisu@@

@@output w0002_shadow.console@@

Give the inner binding a name of its own, or assign to the outer one if
it is a `var`.

@@file w0002_shadow_fixed.sisu@@

@@output w0002_shadow_fixed.out@@

Shown in [chapter 2](../tutorial/02-values.md).

### W0003

`type.unreachable-arm`: this `case` arm is unreachable; the arms above
it cover every value it matches.

Arms are tried from the top, so an arm whose values the arms above it
all cover never runs.

@@file w0003_unreachable.sisu@@

@@output w0003_unreachable.console@@

Move the specific arm above the general one, or remove it.

@@file w0003_unreachable_fixed.sisu@@

@@output w0003_unreachable_fixed.out@@

Shown in [chapter 7](../tutorial/07-structs-and-enums.md).

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

@@file r0001_panic.sisu@@

@@output r0001_panic.console@@

When the case can happen, return `T?` or `T ! E` and let the caller
decide.

@@file r0001_panic_fixed.sisu@@

@@output r0001_panic_fixed.out@@

### R0002

`panic.index-out-of-bounds`: index out of bounds: the length is {len}
but the index is {index}.

An index is checked against the length when it is used, and an index
past the end panics, naming both.

@@file r0002_index.sisu@@

@@output r0002_index.console@@

Check the index first, or use `.get(i)`, which gives `None` past the
end.

@@file r0002_index_fixed.sisu@@

@@output r0002_index_fixed.out@@

### R0003

`panic.overflow`: integer overflow: {expr}.

Integer arithmetic that leaves its type's range panics, naming the
operation and its operands (D-009).

@@file r0003_overflow.sisu@@

@@output r0003_overflow.console@@

Use a wider type, or say what should happen with the wrapping (`+%`) or
saturating (`+|`) operators.

@@file r0003_overflow_fixed.sisu@@

@@output r0003_overflow_fixed.out@@

### R0004

`panic.divide-by-zero`: division by zero.

An integer division or remainder by zero panics.

@@file r0004_division.sisu@@

@@output r0004_division.console@@

Check the divisor before dividing, or state it as a precondition with
`requires`.

@@file r0004_division_fixed.sisu@@

@@output r0004_division_fixed.out@@

### R0006

`panic.assert`: assertion failed: {expr}.

`assert` checks a condition and panics when it is false, showing the
value of each part of the expression.

@@file r0006_assert.sisu@@

@@output r0006_assert.console@@

Make the condition hold, or correct the assertion. In a test, a failed
`assert` fails the test.

@@file r0006_assert_fixed.sisu@@

@@output r0006_assert_fixed.out@@

### R0010

`panic.missing-key`: the key {key} is not in the dict.

Indexing a dict with a key it does not hold panics, naming the key.

@@file r0010_missing_key.sisu@@

@@output r0010_missing_key.console@@

Look the key up with `.get(key)`, which gives `None`, or test `key in
ages` first.

@@file r0010_missing_key_fixed.sisu@@

@@output r0010_missing_key_fixed.out@@

### R0012

`panic.zip-lengths`: zip: the arguments have different lengths.

`zip` expects its arguments to be equally long, and panics when one runs
out first.

@@file r0012_zip.sisu@@

@@output r0012_zip.console@@

Make them equally long, or use `zip_shortest` when stopping at the
shorter is what you want.

@@file r0012_zip_fixed.sisu@@

@@output r0012_zip_fixed.out@@

### R0013

`panic.hole`: reached the typed hole {hole}.

A run that reaches a typed hole panics with the hole's name; the
compiler warned about it with W0001.

@@file r0013_hole.sisu@@

@@output r0013_hole.console@@

Write the code the hole stands for.

@@file r0013_hole_fixed.sisu@@

@@output r0013_hole_fixed.out@@

### R0018

`run.error-returned`: error: {error}.

An error that `main` returns ends the program: it is printed with the
place that passed it on, and the exit status is 1.

@@file r0018_main_error.sisu@@

@@output r0018_main_error.console@@

Return the error when ending the program is right; handle it with
`match`, `else` or `.or` when the program can go on.

@@file r0018_main_error_fixed.sisu@@

@@output r0018_main_error_fixed.out@@

### R0100

`contract.requires-failed`: precondition of `{function}` failed:
{clause}.

A `requires` clause states what a function needs of its caller; a call
that breaks it panics with the clause and the values of its parts.

@@file r0100_precondition.sisu@@

@@output r0100_precondition.console@@

The caller is wrong: check the condition before the call.

@@file r0100_precondition_fixed.sisu@@

@@output r0100_precondition_fixed.out@@

### R0101

`contract.ensures-failed`: postcondition of `{function}` failed:
{clause}.

An `ensures` clause states what a function promises; a return that
breaks it panics with the clause and the values of its parts.

@@file r0101_postcondition.sisu@@

@@output r0101_postcondition.console@@

The function is wrong: correct its body, or its promise.

@@file r0101_postcondition_fixed.sisu@@

@@output r0101_postcondition_fixed.out@@

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

@@file r0115_char_boundary.sisu@@

@@output r0115_char_boundary.console@@

Slice at character boundaries, or iterate `.chars()`.

@@file r0115_char_boundary_fixed.sisu@@

@@output r0115_char_boundary_fixed.out@@

## The driver

Codes D0001 to D0122. The messages of the `sisu` command itself
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
| E0106 | `parse.expected-declaration` | expected a declaration, found {found} |
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
| [E0215](#e0215) | `resolve.self-outside-method` | `self` exists only inside a method |
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
| [E0231](#e0231) | `resolve.trait-as-type` | `{name}` is a trait, not a type; a value of a type that implements it is `dyn {name}` |
| [E0232](#e0232) | `resolve.missing-trait-method` | the impl of `{trait}` for `{ty}` is missing {methods} |
| E0233 | `resolve.not-a-trait-method` | `{name}` is not a method of `{trait}` |
| E0234 | `resolve.method-signature` | `{name}` does not have the signature `{trait}` gives it: expected `{expected}`, found `{found}` |
| [E0235](#e0235) | `resolve.overlapping-impl` | `{trait}` is implemented for `{ty}` more than once; a type implements a trait at most once |
| E0236 | `resolve.orphan-impl` | `{trait}` for `{ty}` can be implemented only in the package of the trait or of the type, or in the root package |
| E0237 | `resolve.missing-assoc-type` | the impl of `{trait}` for `{ty}` does not say what `{name}` is; add `type {name} = ...` |
| E0238 | `resolve.unknown-assoc-type` | `{trait}` has no associated type `{name}` |
| E0239 | `resolve.no-assoc-type` | `{ty}` has no associated type `{name}` among its bounds |
| E0240 | `resolve.sealed-trait` | `{trait}` is implemented only by the standard library, so `{ty}` cannot implement it |
| [E0241](#e0241) | `resolve.not-object-safe` | `{trait}` cannot be used as `dyn {trait}`: its method `{method}` cannot be called through it, because {reason} |
| [E0242](#e0242) | `resolve.supertrait-missing` | `{ty}` implements `{trait}`, which requires `{super}`, but `{ty}` does not implement `{super}` |
| [E0243](#e0243) | `resolve.not-derivable` | `{name}` cannot be derived; `@derive` takes `Copy` and the traits that have a `comptime def derive`: `Debug`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that declares one |
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
| [E0304](#e0304) | `type.unknown-method` | `{ty}` has no method `{name}` |
| [E0305](#e0305) | `type.argument-count` | wrong number of arguments to `{name}`: expected {expected}, found {found} |
| [E0306](#e0306) | `type.unknown-keyword` | `{name}` has no parameter named `{arg}` |
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
| [E0328](#e0328) | `type.pattern-field-count` | wrong number of fields in the pattern for `{name}`: expected {expected}, found {found} |
| [E0329](#e0329) | `type.refutable-let` | this pattern might not match; add `else:` with a block that leaves the scope |
| E0330 | `type.else-not-fallible` | `else` after `let` needs a `T?`, a `T ! E` or a pattern that can fail; this value is `{ty}` |
| E0331 | `type.else-must-diverge` | the block after `else:` must leave the scope with `return`, `break`, `continue` or a panic |
| E0332 | `type.float-suffix` | a float literal takes the suffix `f32` or `f64`, not `{suffix}` |
| E0333 | `type.impure-call` | `{callee}` is not pure, so the pure function `{name}` cannot call it |
| E0334 | `type.impure-capability` | the pure function `{name}` cannot use the capability `{ty}` |
| E0335 | `type.impure-argument` | this function is passed where a pure function is required, but it calls `{callee}`, which is not pure |
| [E0337](#e0337) | `type.missing-field` | missing the field `{field}` of `{ty}` |
| [E0338](#e0338) | `type.move-out-of-borrow` | cannot move `{what}` out of a borrowed value; copy it with `.copy()` |
| [E0339](#e0339) | `type.mut-argument-missing` | `{param}` is a `mut` parameter, so the call must say `mut` before the argument |
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
| E0398 | `type.structural-only` | {callee} is hashed and compared structurally in stage 0, and `{ty}` has an impl of `{trait}` of its own; derive `Hash` and `Eq` for it, or key by a field |
| E0399 | `type.bad-format-spec` | `{spec}` is not a format spec; it is `[[fill]align][sign][#][0][width][,\|_][.precision][type]`, as in `>8.2f`, with the type one of `d x X o b e E f F % s ?` |

### Ownership, every code

| Code | Identifier | Message |
|---|---|---|
| [E0400](#e0400) | `borrow.use-after-move` | `{name}` was moved on line {line} and cannot be used afterwards |
| [E0401](#e0401) | `borrow.use-after-partial-move` | `{name}` cannot be used as a whole, because `{part}` was moved out of it on line {line} |
| [E0402](#e0402) | `borrow.modify-while-borrowed` | `{name}` is borrowed on line {line} and cannot be modified until that borrow ends on line {end} |
| E0403 | `borrow.use-while-mut-borrowed` | `{name}` is borrowed with `mut` on line {line} and cannot be used until that borrow ends on line {end} |
| E0404 | `borrow.move-while-borrowed` | `{name}` is borrowed on line {line} and cannot be moved until that borrow ends on line {end} |
| [E0405](#e0405) | `borrow.mut-while-borrowed` | `{name}` is already borrowed on line {line}, so it cannot be borrowed with `mut` until that borrow ends on line {end} |
| [E0406](#e0406) | `borrow.does-not-live-long-enough` | `{name}` does not live long enough: it is borrowed on line {line}, and the borrow is still used on line {end}, after `{name}` is gone |
| [E0407](#e0407) | `borrow.return-local` | this returns a borrow of `{name}`, which belongs to `{function}` and is gone when it returns |
| E0408 | `borrow.return-not-from` | this returns a borrow of the parameter `{param}`, which is not in the `from` clause of `{function}` |
| E0409 | `borrow.store-in-plain-type` | `{ty}` cannot hold this value, because it borrows `{name}`; only a `view struct` holds borrows |
| E0410 | `borrow.store-into-caller` | this value borrows `{name}`, so it cannot be stored in `{param}`, which belongs to the caller of `{function}` |
| [E0411](#e0411) | `borrow.view-field-in-plain-struct` | `{ty}` holds `{field}: {fty}`, which borrows; only a `view struct` may hold a borrow |
| E0412 | `borrow.view-in-enum` | `{ty}` cannot hold `{fty}`, which borrows; an enum has no view form, so store an owned value such as `String` |
| [E0413](#e0413) | `borrow.from-required` | `{name}` returns `{ret}`, which borrows, and has more than one borrowed parameter ({params}); name the ones it borrows from with `from` |
| E0414 | `borrow.from-not-a-parameter` | `{name}` in the `from` clause is not a borrowed parameter of `{function}` |
| E0415 | `borrow.from-without-borrow` | `{function}` returns `{ty}`, which does not borrow, so it takes no `from` clause |
| E0416 | `borrow.move-out-of-drop` | cannot move `{part}` out of `{name}`, because `{ty}` implements `Drop`, which needs the whole value |
| [E0417](#e0417) | `borrow.must-consume` | `{name}` is never consumed: a `{ty}` must be passed to an `own` parameter before it goes out of scope |
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
| D0120 | `cli.bad-lower` | `--lower` takes `stage2` (the lowering written in Sisu, the default) or `rust` (the Rust lowering, the oracle), not `{value}` |
| D0121 | `native.stage2-build` | building stage 2 in `{dir}` failed; `--lower=rust` lowers with the Rust backend: {detail} |
| D0122 | `native.stage2-failed` | internal compiler error: stage 2 failed to lower the program (status {status}); `--lower=rust` lowers with the Rust backend: {detail} |

