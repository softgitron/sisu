# Lexical structure

This page describes how Sisu reads the text of a program: the source
text, names and the rules for spelling them, keywords, comments, lines
and indentation, every form of literal, the operators and punctuation,
and the layout that `sisu fmt` writes. What the expressions built from
these tokens mean is [R4](../README.md#the-reference), when it is
written.

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
of the file, or a no-break space.

**Stage 0 differs.** [02](../../proposal/02-syntax.md) normalises names
to NFC, so a letter written as a base and a combining mark is the same
name as the letter written as one character. Stage 0 does not: the
combining mark is E0005.

**Status.** In stage 0.

## Names

A name (an identifier) is a Unicode letter or `_`, followed by letters,
digits and `_`. Names are case-sensitive. A keyword is not a name,
except after `.` or `?.` and after `def`, where it is an ordinary name:
`x.or(7)` calls the method `or`, and `def or(...)` declares it.

@@file identifiers.sisu@@

@@output identifiers.out@@

A keyword where a name should be is an error. Where a parameter's name
should be, it is E0124 (`parse.keyword-as-name`); elsewhere, the parser
reports what it expected:

@@file keyword_name.sisu@@

@@output keyword_name.console@@

@@file keyword_parameter.sisu@@

@@output keyword_parameter.console@@

**Stage 0 differs.** The confusable and mixed-script lints of D-048 are
on by default in [02](../../proposal/02-syntax.md); stage 0 has neither,
so a name that mixes Latin and Cyrillic letters that look alike
compiles without a word. A keyword as a binding's name gets E0105 and
E0103, and as a field's name E0100, where E0124 would say that the word
is a keyword.

**Status.** In stage 0; non-ASCII names are decided (D-048).

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

**Stage 0 differs.** [02](../../proposal/02-syntax.md) enforces the
naming rules. Stage 0 enforces them only in patterns, where the case of
a name decides what it means, and for packages and modules: `struct
point`, `def Main()` and `const pi` compile without a word.

**Status.** Designed (D-055); in stage 0 for patterns, packages and
modules.

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

@@file comments.sisu@@

@@output comments.out@@

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

@@file continuation.sisu@@

@@output continuation.out@@

There is no backslash continuation; a `\` at the end of a line is E0015
(`lex.backslash-continuation`), whose message names the three ways to
break a line:

@@file backslash.sisu@@

@@output backslash.console@@

@@file semicolon.sisu@@

@@output semicolon.console@@

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

@@file blocks.sisu@@

@@output blocks.out@@

A line that goes back to an indentation no enclosing block has is
E0002, which names the columns that would match. A line indented more
than the one before, when that one opens no block, is E0125:

@@file dedent.sisu@@

@@output dedent.console@@

A block always starts on the next line. A statement after the `:` on the
same line is E0102, and a line opening a block with nothing indented
after it is E0101:

@@file one_line_block.sisu@@

@@output one_line_block.console@@

**Stage 0 differs.** A one-line `if c: print(x)` is read as an `if`
expression without its `else`, and gets E0116 where E0102 would say
that the block starts on the next line.

**Status.** In stage 0 (D-027).

## Integer literals

An integer literal is written in decimal (`42`), hexadecimal (`0xFF`),
binary (`0b1010`) or octal (`0o17`). `_` may stand between digits and
after the prefix, to group them: `1_000_000`, `0x_ff`. A suffix gives
the literal a type: `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`,
`u32`, `u64`, `u128`, `int` or `uint`; `f32` and `f64` make a decimal
literal a float. Without a suffix, the literal is an `int`, unless the
type expected where it stands is another integer type or an exact type
of `std.num` ([R2](../README.md#the-reference), when it is written).

@@file numbers.sisu@@

@@output numbers.out@@

Letters after a number that are not a suffix, or a prefix without
digits, are E0012 (`lex.invalid-number`); a literal too large for its
type is E0374 ([R17](diagnostics.md#e0374)).

@@file bad_number.sisu@@

@@output bad_number.console@@

**Status.** In stage 0.

## Floating-point literals

A floating-point literal has digits on both sides of its point
(`1.0`, `0.5`), an exponent (`2.5e-3`, `1e3`), or a suffix `f32` or
`f64`. It is an `f64` unless its suffix or the type expected says
otherwise. `1.` and `.5` are not literals: a point needs digits on both
sides, so that `1.method()` reads as a call.

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

@@file characters.sisu@@

@@output characters.out@@

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
  [R4](../README.md#the-reference), when it is written.
- A multi-line literal keeps every character between its quotes,
  including the first line break and the indentation of each line.
- A one-line literal ends on its line. One that is not closed there is
  E0003 ([R17](diagnostics.md#e0003)); a multi-line literal never
  closed is E0004. A single `}` is E0013, an empty interpolation `{}`
  E0121, and a line break inside an interpolation of a one-line
  literal E0018.
- A string literal takes the type expected where it stands when that
  type implements `FromLiteral[str]`, as `Path`, `Url` and `String` do:
  `let home: Path = "/home/aino"` ([R2](../README.md#the-reference),
  when it is written).

@@file strings.sisu@@

@@output strings.out@@

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

@@file collections.sisu@@

@@output collections.out@@

**Stage 0 differs.** Generator expressions and list literals typed as an
`Array` are designed and stage 0 does not have them yet; it builds an
`Array` with `Array.filled`.

**Status.** In stage 0 (D-044), except generator expressions.

## Operators and punctuation

The lexer takes the longest token that matches, so `//=` is one token
and not `//` and `=`.

| Tokens | Meaning ([R4](../README.md#the-reference)) |
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
- a run of imports is sorted, and the imports the file does not use are
  removed.

The formatter checks that the text it writes parses to the same program;
otherwise it leaves the file as it is (D0076). It formats only files
that parse, and never moves a statement to another block.

@@file layout.sisu@@

@@output layout.console@@

**Status.** In stage 0 (D-030).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0001](diagnostics.md#e0001) | `lex.tab-in-indentation` | [Indentation and blocks](#indentation-and-blocks) |
| [E0002](diagnostics.md#e0002) | `lex.inconsistent-dedent` | [Indentation and blocks](#indentation-and-blocks) |
| [E0003](diagnostics.md#e0003) | `lex.unterminated-string` | [String literals](#string-literals) |
| E0004 | `lex.unterminated-block-string` | [String literals](#string-literals) |
| E0005 | `lex.unknown-character` | [Source text](#source-text) |
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
| E0123 | `parse.walrus` | [Operators and punctuation](#operators-and-punctuation) |
| [E0124](diagnostics.md#e0124) | `parse.keyword-as-name` | [Names](#names) |

[R17, Diagnostics](diagnostics.md) lists every code.
