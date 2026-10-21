# Stage 1: the Sisu front end in Sisu

Stage 1 of the bootstrap ([18](../../proposal/18-next-steps.md),
"Bootstrapping"): the lexer, the lossless syntax tree, the error-tolerant
parser, the AST, the name resolution of one module and a first slice of
the type checker, written in Sisu, run by the stage-0 interpreter, and
checked byte for byte against the Rust front end (`sisu-lexer`,
`sisu-syntax`, `sisu-parser`, and `ast::build`) and against the name and
type diagnostics of `sisu check`. The ownership checker and SIR emission
are not here yet.

It is a transcription: every rule of
[docs/architecture.md](../docs/architecture.md) ("Lexing", "Parsing and the
syntax tree", "Error tolerance") and every function of the Rust crates has
its counterpart here, under the same name where the language allows. Where
the Rust behaviour is an accident rather than a rule, stage 1 follows the
documents and the difference is listed in
[DISCREPANCIES.md](DISCREPANCIES.md): five, three of the parser's, none of
which occurs in the corpus, one of name resolution and one of the type
checker's.

## Running it

From this directory, with stage 0 built (`cargo build --release` in
`compiler/`):

```text
sisu test                                   # 73 tests, 49 of them stage 0's own, transcribed
sisu run -- tokens [--trivia] FILE          # what `sisu tokens [--trivia] FILE` prints
sisu run -- parse --tree [--trivia] FILE    # what `sisu parse --tree [--trivia] FILE` prints
sisu run -- parse --print FILE              # the file, reprinted from the tree
sisu run -- ast FILE                        # the AST of FILE, in the notation of Rust's `Debug`
sisu run -- names FILE                      # the name diagnostics of `sisu check --format=ids FILE`
sisu run -- types FILE                      # the type checker's diagnostics of the same command
sisu run -- batch OUT LIST                  # tokens, trees and ASTs for every file LIST names, into OUT
sisu run -- ast-batch OUT LIST              # the AST of every file LIST names, into OUT
sisu run -- names-batch OUT LIST            # the name diagnostics of every file LIST names, into OUT
sisu run -- types-batch OUT LIST            # the type diagnostics of every file LIST names, into OUT
sisu run -- mutate SEED COUNT OUT LIST      # mutants of the files LIST names
sisu run -- profile FILE                    # the time lexing, parsing and each dump of FILE take
./check.sh [--mutants N] [--seed S] [--names] [--types] [--keep DIR]
```

Stage 1 prints to standard output and its diagnostics to standard error,
in the conformance form (`--format=ids`), as stage 0 does; `names` and
`types` print their diagnostics to standard output, as `sisu check` does,
and `types` then prints on standard error why parts of the file are beyond
the slice it checks. It reads the
message catalog at run time, by default `../catalog/messages.catalog`
(`--catalog PATH` names another), and the standard library's sources from
`../std/`, so it is run from this directory; the tests read the catalog and
the proposal's examples the same way.

`check.sh` builds stage 0 in release mode, makes stage 0's outputs for
every file, runs stage 1 once per class of files (`batch`, one interpreter
run for many files), and compares five outputs per file: `tokens` (`sisu
tokens --trivia`, standard output and error), `tree` (`sisu parse --tree`,
both), `trivia-tree` (`sisu parse --tree --trivia`), `print` (stage 1's
reprint against the file itself) and `ast` (the AST, below), where a file
whose tree differs as listed may differ in its AST too. Then it compares
`names`: the
name diagnostics of `sisu check --format=ids` (the lines with `E0200`,
`E0201`, `E0202`, `E0220`, `E0234`, `E0389` or `W0002`, with their labels
and fixes) against stage 1's (`names-batch`), for the conformance suite's
`names/` cases and stage 1's own cases, and with `--names` for every
program of the corpus that is one module (the conformance cases outside
`packages/`, and the proposal's examples). Then `types`: the type
checker's diagnostics of `sisu check --format=ids` (those whose identifier
begins with `type.`, with their labels and fixes) against stage 1's
(`types-batch`), for the files `types.list` names, each of which must be
within the slice and match, and with `--types` for every program of the
corpus that is one module, where a file within the slice must match and
any other may only report fewer of stage 0's diagnostics. It exits non-zero
on any difference that DISCREPANCIES.md does not list ("- Differs: `path`:
outputs"). With `--mutants N` it also compares N mutants of the corpus
(deleted, inserted and swapped characters), where a difference must carry
the mark of a listed discrepancy.

## Results

`./check.sh --mutants 2000 --seed 7 --names --types`:

```text
stage 1 against stage 0 (tokens, tree, trivia-tree, print, ast):
  conformance   386 files,  386 identical,   0 differing as listed,   0 differing otherwise; stage 1   34.2s (250253 bytes; lex 4995 ms, parse 7021 ms, dumps 10894 ms, ast 5838 ms)
  std            15 files,   15 identical,   0 differing as listed,   0 differing otherwise; stage 1   25.6s (234728 bytes; lex 3504 ms, parse 4788 ms, dumps 7744 ms, ast 4460 ms)
  examples       11 files,   11 identical,   0 differing as listed,   0 differing otherwise; stage 1    7.9s (27495 bytes; lex 487 ms, parse 630 ms, dumps 962 ms, ast 567 ms)
  cases          24 files,   21 identical,   3 differing as listed,   0 differing otherwise; stage 1    6.6s (12878 bytes; lex 305 ms, parse 462 ms, dumps 648 ms, ast 272 ms)
  mutants      2000 files, 1979 identical,  21 differing as listed,   0 differing otherwise; stage 1  237.2s (2294140 bytes; lex 40245 ms, parse 55386 ms, dumps 86685 ms, ast 47362 ms)
stage 1's name resolution against stage 0's (names):
  names          34 files,   33 identical,   1 differing as listed,   0 differing otherwise; stage 1   14.6s
  corpus-names  350 files,  349 identical,   1 differing as listed,   0 differing otherwise; stage 1   32.9s
stage 1's type checker against stage 0's (types):
  types          37 files,   36 identical,   0 beyond the slice and a subset,   1 differing as listed,   0 otherwise; stage 1   25.2s
  corpus-types  360 files,  135 identical, 225 beyond the slice and a subset,   0 differing as listed,   0 otherwise; stage 1  102.9s
total: 3217 files checked, 2965 identical, 225 beyond the type checker's slice and a subset, 27 differing (27 listed in DISCREPANCIES.md, 0 not)
```

All 412 files of the conformance suite, `compiler/std/` and
`proposal/examples/` give identical token streams with trivia, trees with
and without trivia, diagnostics, reprints and ASTs. `cases/` holds stage
1's own inputs for the error-tolerance rules the corpus never triggers
(operator mixing and the parentheses its fixes add, Python's conditional
expression, statement, expression, declaration, lexical, indentation,
bracket and structure mistakes, the keyword that opens a missing block, the
token a line misses at its end, the forms of later phases, scripts, CR LF,
Unicode beyond ASCII, type arguments where a value is written), for the
type checker's rules (calls, conversions, control flow) and the five
discrepancies. The 2000 mutants (`--seed 7`) differ only by discrepancy D1,
and their ASTs only where their trees do. Of the 350 programs of one
module, every one but D4's `proposal/examples/testing.sisu` gets the name
diagnostics of stage 0. Of the 360 programs of one module (the 350 and the
conformance suite's `names/` cases), 135 are within the type checker's
slice and get stage 0's type diagnostics exactly; the other 225 get a
subset of them.

## Name resolution

`resolve.sisu` resolves the names of one module, the way `sisu check`
does for a program of one file (docs/architecture.md, "Names and the
standard library"): every name the module writes is found among the
scopes of its bodies, the module's declarations, the prelude's, the
variants they declare, or what its imports bind, or it is reported. The
prelude is read from `../std/prelude.sisu` and parsed by this parser, as
are the std modules every module sees without an import (`task`, `log`,
`cap`, `live`, and `target`) and those the module imports (`import json`,
`from std.time import DateTime`). It reports:

- `E0202`, a name declared twice in the module, labelled with the first;
- `E0220`, an import of a module that does not exist, with the closest
  module path as a fix;
- `E0201`, a type name that names nothing, in signatures, annotations and
  type arguments (`cli.parse[Options]`, `NonEmpty[int].try_from`);
- `E0200`, a value name that names nothing, with the closest name in scope
  as a fix when one is close enough (decision 5: an edit distance that
  counts a swap of neighbours as one edit, at most a third of the name);
- `W0002`, a `let` that shadows a name of an enclosing scope;
- `E0389`, `self` outside a method;
- `E0234`, a method of a trait's impl whose shape differs from the trait's:
  its receiver, its number of parameters, their modes, its own generic
  parameters.

Stage 0 does this work in two places: `sisu-resolve` declares items,
resolves imports and the names in signatures to types, and `sisu-typeck`
looks up the names in bodies while it types them. Stage 1 does it in one
pass over the syntax tree, without types, so it is not a transcription of
one function after another; it reproduces the rules stage 0's two passes
apply, including the error-tolerance rules of names: a function whose
header or body has a syntax error reports no unknown names, names written
in top-level code that did not parse as a declaration are never reported,
a failed import poisons the name it would have bound, and no diagnostic is
kept on a line that has a syntax error. What depends on types is out of
its reach: a method name, a field name, and the variant of an expected
type are not names it looks up, and a signature mismatch of types (rather
than of shape) is the type checker's.

`check.sh` compares its diagnostics with the name diagnostics of `sisu
check --format=ids` (above). One difference is listed: D4, a parametrised
test whose body names something unknown, which stage 0 reports once per
case and stage 1 once.

## The AST

`syntax.ast` is stage 0's AST (`sisu-syntax/src/ast/mod.rs`) and
`syntax.ast.build` its builder (`ast/build.rs`): `build.module` is a total
function of the lossless tree, so a file with syntax errors still has an
AST, with `Error` where a part did not parse and `header_broken` and
`body_broken` on the functions it touched. The literals are decoded as the
Rust builder decodes them (radixes, `_`, suffixes, escapes, `u128` values),
and a module's broken top-level names are kept for the checker's
error-tolerance rule.

Where the Rust AST holds a child in a `Box`, stage 1 holds an id: `Ast`
keeps the items, statements, expressions, types, patterns and the arms
`match` generates in arenas, and an `ExprId` is an index into them. A
recursive type with many boxed fields is what stage 0's ownership checker
handles worst (item 13 below): with the boxed transcription, `sisu check`
of stage 1 did not finish.

No `sisu` command prints stage 0's AST, so `oracle/` does: a small Rust
program over the stage-0 crates (`sisu-diag`, `sisu-parser`, `sisu-syntax`)
that `check.sh` builds into `target/stage1-oracle/` and that writes, for
each file, the module's doc comments, its broken names and each item in
Rust's `Debug` notation. `syntax.ast.dump` writes stage 1's in the same
notation, following the ids. A `sisu parse --ast` in stage 0 would let the
oracle go.

## The type checker's slice

The type checker of stage 0 is `sisu-resolve` (the declarations and their
signatures) and `sisu-typeck` (the bodies), 22 000 lines of Rust. Stage 1
transcribes the part that the simplest programs of `conformance/cases/types/`
and `wrong/` need, rule for rule, and knows where that part ends: a body
that uses anything beyond it is checked to the end but reports nothing, and
says why (`types` prints the reasons; `types-batch` writes them to
`OUT/n.beyond`). So a file with nothing beyond the slice must get exactly
stage 0's type diagnostics, and any other a subset of them, which
`check.sh --types` checks over the corpus.

How a file is checked (`typeck.sisu`): the prelude, the std modules every
module sees (`task`, `log`, `cap`, `live`), `target` and `builtin_impls`
(the impls of the leaf types and tuples, whose text stage 0 writes in
`std_sources.rs` and `typeck.std_sources` writes the same way) are parsed
by stage 1's parser, built into ASTs and resolved once (`typeck.collect`,
about 13 s). Each file's root module is then declared and resolved on a
copy of that program; each function body of the root module is checked by
a checker of its own (`typeck.check`, `typeck.expr`, `typeck.call`, with
`typeck.infer`'s unification), its bounds are checked against the impls
once inference is done (`typeck.program`'s solver), then the defaults of
fields and parameters and the constants, then `main`'s and the tests'
signatures. Diagnostics on a line with a syntax error are dropped, and the
rest are sorted by start, end and code, as the driver does. Types are
interned (`typeck.ty`): a type is an index into a table, so two types are
equal when their indices are.

What it reports: the type checker's own messages, the identifiers that
begin with `type.`. Within the slice that is `E0300` (with the label of the
annotation that asked for the type, and the fixes that unquote a number or
call a method that converts), `E0301` to `E0303`, `E0304` (with the field
or method of a similar name as a fix, as for `E0303`), `E0305` to `E0309`,
`E0310` and its interpolation fix, `E0311`, `E0312` and `E0341` with their
labels and fixes, `E0313`, `E0315` to `E0324`, `E0337` to `E0340`, `E0344`,
`E0345`, `E0347` with its fixes, `E0349` to `E0351`, `E0359` to `E0363`,
`E0365`, `E0368`, `E0369`, `E0372` to `E0376`, `E0378` to `E0380`, `E0386`,
`E0389`, `E0395`, `E0399`, `E0702`, `W0001` and `W0004`.

What the slice covers:

- declarations: functions with generic parameters, bounds, defaults,
  positional-only and keyword-only parameters and `mut` and `own`
  parameters; methods in the bodies of structs and enums and in inherent
  impls; impls of the prelude's traits; structs, enums, error sets, type
  aliases, constants, tests and suites;
- statements: `let` and `var` of a name or `_`, with or without a type;
  `=` and the augmented assignments to locals, fields and elements; `_ =`;
  expression statements; `return`; `if`, `elif` and `else`; `while`;
  `for` over lists, sets, arrays, ranges and iterators; `break`,
  `continue`, `pass`, `assert`, `expect` and `defer`;
- expressions: literals (sized integers and the out-of-range ones
  included), plain and interpolated strings with format specs, names and
  unit variants, tuples and lists, the arithmetic and text operators,
  `and`, `or` and `not`, comparisons and their chains and `in`, unary
  operators, calls of functions (keyword arguments, defaults, holes),
  constructors of structs and of the builtin types, variants with fields,
  methods of a type (and `str`'s through a `String`), static paths
  (`Type.f(...)`, `Enum.Variant`, `log.info(...)`, `T.f()` for a generic
  `T`), fields of structs and tuples, indexing of lists, arrays and `dict`s
  and slices of lists, `if` expressions, `?` on options and results,
  `.copy()` and `ref`;
- the rules between them: the coercions (`Never` to anything, `String` to
  `str`, `ref mut` to `ref`, error-set widening, a `ref` of a `Copy` type to
  its value, integer widening), literals that take the expected type, the
  divergence of statements (unreachable code, a missing `return`), what can
  be assigned and borrowed `mut`, the matching of arguments to parameters,
  and the bounds of generic calls, decided against the impls of the
  prelude, `builtin_impls` and the module.

What is beyond it, and makes a body report nothing:

- in the module, which then reports nothing at all: an import, a trait, a
  derived or delegated impl (stage 0 writes those from templates, which
  stage 1 does not unfold), a refinement type, an invariant, an associated
  constant, a must-consume type, a parametrised test, a doc example (stage
  0's tests), a script's statements, `extern`, `cap.Fake`;
- in a body: `match`, closures and trailing blocks, comprehensions, `dict`
  and `set` literals, `??`, `?.`, `^` outside an index, `comptime`, `with`,
  `select`, `yield`, nested declarations, `let ... else`, `if let`, a `let`
  without a value, patterns other than a name, calls of a trait's methods
  (`x.eq(y)`, `Trait.f(x)`, a trait's function called on a type) and of
  function values, explicit type arguments, functions as arguments,
  inferred error sets (`T !`), contracts, a field of another module's
  struct, coercions to `dyn`, to slices, to capabilities and to
  `Iterable`, the operators of traits, literals of a type that implements
  `FromLiteral`, and a bound the solver cannot decide (an impl stage 0
  derives, `Send` and `Sync`) or a key hashed structurally;
- the `type.` messages that stage 0's ownership checker reports, on SIR
  and after the type checker: a pure function's calls of impure functions
  (`E0333`) and its drops (`E0393`), definite assignment (`E0366`,
  `E0367`), must-consume type arguments (`E0397`). A pure body that calls
  an impure function is beyond, and so is every pure body of a module that
  implements `Drop`.

One choice of stage 1 differs from stage 0 within the slice: D5, the count
`E0305` reports for positional arguments past a bare `*`. And one
difference of order has no effect on what is reported: stage 0 declares
every module's items before it resolves any, so its definitions of the root
module come before those the prelude's impls and traits make; stage 1
resolves the prelude once, so they come after.

`check.sh` checks the slice twice: `types.list` names 34 programs of
`conformance/cases/types/` and `wrong/` and stage 1's three cases of type
rules, which must be within the slice and match (36 do, and D5's case is
listed); `--types` checks all 360 programs of one module, of which 135 are
within the slice and match (49 of the 61 of `types/` and `wrong/`) and 225
report a subset. The first reason each of the 225 is beyond, by count: an
import (32), a trait (28), a method of a trait (19), a derived impl (14),
`match` (13), a trailing block (9), a must-consume type (8), `comptime`
(8), explicit type arguments (7), an invariant (7).

## Module layout

| Module | Lines | What | Stage 0 original |
|---|---:|---|---|
| `syntax.kinds` | 538 | `SyntaxKind` (246 kinds), the keyword and punctuation tables, kind predicates, names in dumps | `sisu-syntax/src/kinds.rs` |
| `syntax.tree` | 338 | `Token`, the arena tree, the builder with checkpoints, the dumps, the reprint, stable identities | `sisu-syntax/src/tree.rs` |
| `diag` | 637 | `Msg` (the catalog identifiers stage 1 reports), diagnostics with labels, fixes and notes, the catalog reader, positions, `render_ids`, `quote`, the stable sort | `sisu-diag` (the parts the front end uses) |
| `text`, `text.unicode` | 369 | UTF-8 decoding, `is_alphanumeric`, Rust's `Debug` quoting, and the Unicode tables it needs (pinned, Unicode 17.0.0) | the host's `char` methods and `{:?}` |
| `lexer` | 262 | `lex`, and stage 0's lexer tests | `sisu-lexer/src/lib.rs`, `tests.rs` |
| `lexer.scan` | 556 | the raw scanner: every byte a token, strings and interpolation as tokens | `sisu-lexer/src/scan.rs` |
| `lexer.layout` | 347 | Python's layout algorithm, continuation, bracket recovery and its fixes | `sisu-lexer/src/layout.rs` |
| `parser` | 517 | `parse`, the diagnostic filters (bracket echoes, structure echoes, one per line), and stage 0's parser tests | `sisu-parser/src/lib.rs`, `tests.rs` |
| `parser.state` | 403 | the parser's state: looking, attaching tokens with their trivia, reporting, the fixes added later, Python's conditional | `sisu-parser/src/parser.rs` |
| `parser.items`, `.stmts`, `.exprs`, `.patterns`, `.types` | 2162 | one function per rule of the grammar | the files of the same names |
| `syntax.ast` | 745 | the AST: items, statements, expressions, types and patterns in arenas, referred to by id | `sisu-syntax/src/ast/mod.rs` |
| `syntax.ast.build` | 2132 | `build.module`, the AST of a tree as a total function, and the decoding of literals | `sisu-syntax/src/ast/build.rs` |
| `syntax.ast.dump` | 1321 | the AST in the notation of Rust's derived `Debug`, for `check.sh` | Rust's `{:?}` |
| `resolve` | 1913 | the names of one module: declarations, imports, scopes, the prelude's and std's names, and the name diagnostics | `sisu-resolve/src/collect.rs` and `types.rs`, and the name lookups of `sisu-typeck` |
| `typeck.ty` | 349 | types, interned in a table | `sisu-resolve/src/ty.rs` |
| `typeck.program` | 1229 | the resolved program, the spelling of types, `Copy`, the trait solver | `program.rs`, `ty.rs` and `solve.rs` of `sisu-resolve` |
| `typeck.std_sources` | 308 | `target` and `builtin_impls`, the std text stage 0 writes | `sisu-resolve/src/std_sources.rs` |
| `typeck.collect` | 1544 | declarations, signatures, traits and impls, resolved | `collect.rs`, `types.rs` and `traits.rs` of `sisu-resolve` |
| `typeck.infer`, `typeck.thir` | 359 | inference variables and unification; the typed tree as far as the slice reads it | `infer.rs` and `thir.rs` of `sisu-typeck` |
| `typeck.check`, `.expr`, `.call` | 3455 | the body checker: statements, expressions, calls | `check.rs`, `expr.rs` and `call.rs` of `sisu-typeck` |
| `typeck` | 440 | a body's checks, a file's, and the driver | `sisu-typeck/src/lib.rs` |
| `mutants` | 109 | a seeded generator and the mutations of stage 0's robustness test | `sisu-driver/src/tests.rs` |
| `main` | 300 | the command line | `sisu-driver/src/cli.rs`, `tokens`, `parse` and `check` |

20333 lines in all, about 800 of them tests; the Rust the lexer and the
parser transcribe (`sisu-lexer`, `sisu-parser`, and `kinds.rs` and
`tree.rs` of `sisu-syntax`) is 5717 lines with its tests, the AST and its
builder 2782, and `sisu-resolve` and `sisu-typeck`, of which the type
checker's slice transcribes a part, 22053.

## How the port went

### What transcribed directly

The grammar, rule by rule: each parser function is the Rust function with
the same name and the same control flow, and the tree builder's checkpoint
mechanism (wrap already-attached children without backtracking) carries
over unchanged. The scanner, the layout pass and the three diagnostic
filters are line-for-line transcriptions. So are the diagnostics' data
(identifier, parameters, primary span, labels, notes), the stable sort by
start, end and code, and the conformance form. Sisu's enums with payloads
and `match` with or-patterns and guards made this the easy part: the
parser's `match self.current(): case IntNumber | FloatNumber | ...:` reads
as the Rust does, `Mode::Interp { depth, triple }` is `Interp(depth: int,
triple: bool)`, and `while let`, `if let` and chained conditions
(`if let Some(next) = next_indentation(...), next > logical_width:`) cover
Rust's `is_some_and`. Stage 0's unit tests came along: 49 of them (the
lexer's, the parser's except those of the AST, the AST builder's, the
kinds', the tree's, the source map's columns, the inference table's, and the
type errors of `sisu-typeck`'s tests that are within the slice) are tests of
stage 1 now, beside 24 new ones.

The rules were complete enough to transcribe: the lexer and the parser
matched stage 0 on all 370 files the first time they ran. What the
documents did not say, the Rust code did; the differences found are its
accidents (DISCREPANCIES.md), not gaps in the rules.

### What took another shape in Sisu

- **Tables become lookups built once.** `KEYWORDS` and `PUNCTUATION` stay
  the tables (`const` lists in `syntax.kinds`), but the scanner turns them
  into `dict`s when it starts: a `const` is rebuilt at every use, so
  scanning the 47 keywords for every name cost about 100 µs per name, and
  the longest-match scan over 63 operators more. Longest match becomes
  "try three bytes, then two, then one", which a test checks against the
  table's order.
- **Slices of kinds become `match`.** Rust's `self.at_any(&[R_PAREN,
  NEWLINE, EOF])` allocates a list per call in Sisu; the sets are
  or-patterns in small predicates instead (`at_list_end`, `is_expr_sync`,
  `is_aug_assign`).
- **A method passed as a value becomes an enum.** `member_block(..., member:
  fn(&mut Self))` dispatches on `Members` (`StructMembers`, `Variants`,
  `Methods`, `Items`).
- **Shared slices become owned copies and index ranges.** The Rust parser
  borrows the lexer's `&[Token]` while the builder copies them; in Sisu the
  builder owns the tokens and the parser keeps its own list of the
  significant ones. The layout pass's physical lines are index ranges into
  the raw tokens rather than vectors of copies.
- **Message constants become an enum, codes come from the catalog.** Rust's
  generated `msg::LEX_SEMICOLON` is `Msg.LexSemicolon` with its catalog
  identifier; the code and the kind are read from `catalog/messages.catalog`
  at run time, so the catalog stays the one source (D-065). `embed` cannot
  read it, since it may not leave the package's directory.
- **Text is bytes.** The scanner reads `str.byte_at` and decodes a character
  only beyond ASCII (`text.char_at`); `chars()` is used where a whole text
  is walked once.
- **What the host gave Rust, stage 1 pins.** `char::is_alphanumeric` (names
  may hold `Nd`, `Nl` and `No` characters) and the escaping of Rust's
  `Debug` (the tree dump's quoting of token texts) have no counterpart in
  the prelude; `text.unicode` holds the two tables, taken from Rust 1.97
  (Unicode 17.0.0) as fixed-width records that a lookup binary-searches in
  place. `char.is_alphabetic` and `is_uppercase` are the prelude's, which
  are the host's (docs/port-debt.md, "Unicode tables").

### What the interpreter and the language made awkward

Candidates for docs/port-debt.md and for the language's open questions, most
consequential first.

1. **A view of a `String` copies it.** Calling a `str` method on a `String`
   makes the view by copying the whole text (port debt, "`str` is a copy"),
   so `text.byte_at(i)` on a 136 KB `String` costs about 4 µs where it
   costs 1 µs on a `str`, and on a 1 MB text 68 µs. Nothing in the source
   shows the cost: the same call is cheap or quadratic depending on whether
   the receiver is a parameter or a field. Stage 1 takes one `str` view per
   walk (`let text: str = self.text`) and passes it down. A view should be a
   pointer and a length; until then, a lint for `str` methods on a `String`
   in a loop would catch it.
2. **There is nothing between a `const` and a parameter.** A `const` is
   rebuilt at each use, and the string literals in a `const` table borrow
   from that temporary (returning one is `E0407`). There are no
   module-level values computed once, so a lookup table must be built by
   its user and threaded through as a value (the scanner's dicts, the
   dumps' `KindNames` cache passed `mut` through the recursion).
3. **Enum variants are bare in patterns but qualified in expressions.** In a
   pattern a variant resolves from the scrutinee's type in any module; in
   an expression only the declaring module sees it bare, others write
   `SyntaxKind.LParen` (a local `type K = SyntaxKind` shortens it), and a
   variant whose name is also a declaration (`Eq` the trait, `Slice` the
   type) must be qualified even there. 02 does not say which rule is meant.
4. **A child module shadows an imported item of the same name.** In
   `stage1.lexer`, `from stage1.lexer.scan import scan` is `E0202` ("defined
   more than once") because the child module `scan` is already a name
   there; stage 1 calls `scan.scan(...)`, and the module of mutations is
   `mutants`, not `mutate`.
5. **`dict[String, V].get` needs a `String`.** Looking a `str` up allocates
   (`d.get(word.to_string())`); a borrowed-key lookup (Rust's `Borrow`)
   would remove it.
6. **Interpolation is the slow way to build text.** A line built with
   `"{a} {b}..{c}\n"` costs about 8.5 µs, a pushed literal 0.7 µs, and the
   dumps build a line per token. Pushing pieces helps; a cheaper path for
   interpolating an `int` would help more.
7. **`print` always adds a line break**, and there is no way to write
   standard output without one; stage 1 prints the text without its last
   `\n` and so cannot print an empty text exactly (it prints nothing).
8. **Lists lack `truncate` and `drain`**, so the builder's "take the
   children from `start` on" is a copy and a loop of `_ = pop()`; `pop`'s
   `T?` must be used even when the element is not wanted.
9. **Small checker and parser surprises.** A move inside `assert` is
   reported as "moved in an earlier iteration of the loop"; `??` does not
   unify `ref String` with `str`; `??` binds looser than `>`, so `x ?? -1 >
   w` is a type error that reads like a precedence question (02's table
   places `??` below `or`, which is right, but the message could say so); a
   `match` arm with a value takes one expression on its line, and an `if`
   expression must fit on one line, so a multi-line condition must be named
   first.
10. **The formatter's splits.** `sisu fmt` splits a long method chain inside
    the last call's brackets (`.with(\n "open", "("\n).label(...)`) rather
    than before the dots that 02 makes continuation operators, breaks a
    long one-line `if` expression at an `and` inside its condition, and
    splits an or-pattern of tuples inside a tuple. Stage 1's source avoids
    these shapes (short builder chains, named conditions).
11. **Tests that need a file need the real `System`.** The parser tests read
    the catalog, so each takes `sys: System`, runs alone, and depends on the
    working directory. A test-only way to name a file of the package (an
    `embed` for tests) would make them hermetic.
12. **Compiling the package takes about 5 s** on every `sisu run` (`sisu
    check` of these 20 000 lines takes 4.5 s; of the first 5800, 0.58 s was
    the ownership checker's); `batch` and its kin exist so that one run
    covers many files.
13. **A recursive type costs the ownership checker exponential time.**
    `sisu-borrowck`'s `Types::must_consume_depth` follows the types of
    fields with no memory of what it has seen and a depth limit of 32, at
    every drop and at every intrinsic call with a moved argument; a type
    with several recursive fields (the Rust AST's `Box<Expr>`, in a dozen
    variants) makes `sisu check` of its users take hours, and a directly
    recursive enum (`enum E: Lit(int) Add(lhs: E, rhs: E)`, which 03 says
    needs a `Box`, but nothing reports) makes it never return. Stage 1's
    AST and typed tree are arenas with typed ids, and its types are
    interned.
14. **A `dict` key cannot hold a list when the program runs.** A type with
    a list in it derives `Hash` and type-checks as a key, and then the
    interpreter stops at the first insertion: "List([]) is not hashable".
    The table of types finds a type by a text that names its parts.
15. **Borrowing through a `ref` field borrows the whole struct.** A view
    struct with `prog: ref Program` cannot call a `mut self` method while it
    holds anything read through `prog` (`E0405`), though the program is not
    the struct's to change; two fields the struct owns can be borrowed
    apart. The checker is given the program, the syntax and the text as a
    parameter (`cx: Cx`) instead.
16. **A parameter named `range` hides the prelude's `range`**, and the loop
    over `range(n)` below it is "a value of type `TextRange` cannot be
    called"; the type checker's ranges are named `at`.
17. **Changing an enum's payload in place needs `ref mut` on the
    scrutinee** (`if let Struct(s) = ref mut defs[i].kind:`), which 02 does
    not show; without it the binding is a copy, and assigning to it is
    `E0341`.

What could have been hard and was not: recursion depth (the parser goes
through fourteen rule functions per operand without trouble), `mut`
arguments marked at the call site, and the ownership rules for the parser's
state: the parser is a `view struct` over the source text that owns
everything else, and only a handful of `own` parameters and `.copy()`s were
needed.

## Where the time goes

Stage 1 is about 180 times slower than stage 0: on `std/prelude.sisu` (136
KB, 3742 lines) stage 0 lexes, parses and dumps the tree in about 26 ms,
stage 1 lexes in 1.7 s, parses in 2.0 s and dumps the tree in 1.0 s (the
trivia dump 1.5 s, the token dump 0.9 s, the reprint 0.2 s), after a start of
about 5 s spent compiling stage 1. Over the corpus, about a quarter of the
time is lexing, under a third parsing, and half writing the four dumps.

Costs measured on the stage-0 interpreter (release build):

| Operation | Cost |
|---|---:|
| a call of a small function | 1 µs |
| `str.byte_at` on a `str`, with `to_int` | 1 µs |
| the same on a 136 KB `String` (a view is a copy) | 4 µs |
| `xs[i]` on a `list[int]`, in a loop | 0.7 µs |
| a `chars()` loop, per character | 0.9 µs |
| pushing a literal onto a `String` | 0.7 µs |
| pushing an interpolated line (`"{a}..{b}"`) | 8.5 µs |
| a `dict` lookup with a `String` key | 2 to 3 µs |
| a `match` over 250 payload-free variants, last arm | 38 µs |

The lexer spends its time reading bytes (several reads per byte, each a
method call and an intrinsic call); the parser in the precedence climb
(fourteen rule functions and a dozen kind tests per operand); the dumps in
building their lines. Reading the text into a `list[int]` once, with
sentinels past the end, would roughly halve the lexer; it was not done, to
keep the scanner a transcription.

The type checker's slice adds its own costs: resolving the prelude, the
four ambient std modules and `builtin_impls` takes about 13 s, nearly all of
it parsing their 220 KB, and each file then about a quarter of a second, of
which copying the resolved program is 0.13 s; `types-batch` checks the 360
programs of the corpus in about 100 s. Its tests use a small prelude of
their own.

## What is next

1. **The type checker, slice by slice**, until every program of
   `conformance/cases/types/` and `wrong/` is within it: imports (the std
   modules a program names), static paths (`Type.f`, `Enum.Variant`), traits
   and the calls of their methods, `match` with its patterns and the
   exhaustiveness rules, closures, and the inferred error sets (`T !`, a
   fixpoint over the program). `check.sh --types` shows the next blocker by
   its count.
2. Then the ownership checker, which also reports the `type.` messages of
   purity and definite assignment, then **SIR emission** against the `.sir`
   files of the suite and `sisu sir`, which is where stage 1 becomes the
   oracle.
3. **Speed** where it pays: the scanner over a byte list, dumps without
   interpolation, a prelude resolved once for many runs, and, in the
   interpreter, views that do not copy.
4. **The discrepancies**: once the coordinator has decided D1 to D5, either
   stage 0 changes (and `check.sh` stops listing them) or stage 1 does.
