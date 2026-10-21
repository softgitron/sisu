# Stage 1: the Sisu front end in Sisu

Stage 1 of the bootstrap ([18](../../proposal/18-next-steps.md),
"Bootstrapping"): the lexer, the lossless syntax tree and the
error-tolerant parser of Sisu, written in Sisu, run by the stage-0
interpreter, and checked byte for byte against the Rust front end
(`sisu-lexer`, `sisu-syntax`, `sisu-parser`). Name resolution, the type
checker, the ownership checker and SIR emission are not here yet.

It is a transcription: every rule of
[docs/architecture.md](../docs/architecture.md) ("Lexing", "Parsing and the
syntax tree", "Error tolerance") and every function of the Rust crates has
its counterpart here, under the same name where the language allows. Where
the Rust behaviour is an accident rather than a rule, stage 1 follows the
documents and the difference is listed in
[DISCREPANCIES.md](DISCREPANCIES.md) (three, none of which occurs in the
corpus).

## Running it

From this directory, with stage 0 built (`cargo build --release` in
`compiler/`):

```text
sisu test                                   # 55 tests, 43 of them stage 0's own, transcribed
sisu run -- tokens [--trivia] FILE          # what `sisu tokens [--trivia] FILE` prints
sisu run -- parse --tree [--trivia] FILE    # what `sisu parse --tree [--trivia] FILE` prints
sisu run -- parse --print FILE              # the file, reprinted from the tree
sisu run -- batch OUT LIST                  # the above for every file LIST names, into OUT
sisu run -- mutate SEED COUNT OUT LIST      # mutants of the files LIST names
./check.sh [--mutants N] [--seed S] [--keep DIR]
```

Stage 1 prints to standard output and its diagnostics to standard error,
in the conformance form (`--format=ids`), as stage 0 does. It reads the
message catalog at run time, by default `../catalog/messages.catalog`
(`--catalog PATH` names another), so it is run from this directory; the
tests read the catalog and the proposal's examples the same way.

`check.sh` builds stage 0 in release mode, makes stage 0's outputs for
every file, runs stage 1 once per class of files (`batch`, one interpreter
run for many files), and compares four outputs per file: `tokens` (`sisu
tokens --trivia`, standard output and error), `tree` (`sisu parse --tree`,
both), `trivia-tree` (`sisu parse --tree --trivia`) and `print` (stage 1's
reprint against the file itself). It exits non-zero on any difference that
DISCREPANCIES.md does not list ("- Differs: `path`: outputs"). With
`--mutants N` it also compares N mutants of the corpus (deleted, inserted
and swapped characters), where a difference must carry the mark of a
listed discrepancy.

## Results

```text
  conformance   344 files,  344 identical,   0 differing as listed,   0 differing otherwise; stage 1   18.9s (241319 bytes; lex 3991 ms, parse 5231 ms, dumps 8469 ms)
  std            15 files,   15 identical,   0 differing as listed,   0 differing otherwise; stage 1   14.4s (231662 bytes; lex 2990 ms, parse 3899 ms, dumps 6444 ms)
  examples       11 files,   11 identical,   0 differing as listed,   0 differing otherwise; stage 1    2.6s (27495 bytes; lex 401 ms, parse 489 ms, dumps 826 ms)
  cases          15 files,   12 identical,   3 differing as listed,   0 differing otherwise; stage 1    1.6s (6704 bytes; lex 119 ms, parse 172 ms, dumps 252 ms)
  mutants      2000 files, 1975 identical,  25 differing as listed,   0 differing otherwise; stage 1  176.2s (2638460 bytes; lex 39116 ms, parse 50672 ms, dumps 83522 ms)
total: 2385 files checked, 2357 identical, 28 differing (28 listed in DISCREPANCIES.md, 0 not)
```

All 370 files of the conformance suite, `compiler/std/` and
`proposal/examples/` give identical token streams with trivia, trees with
and without trivia, diagnostics and reprints. `cases/` holds stage 1's own
inputs for the error-tolerance rules the corpus never triggers (operator
mixing, statement, expression, declaration, lexical, indentation, bracket
and structure mistakes, the forms of later phases, scripts, CR LF, Unicode
beyond ASCII) and the three discrepancies. The 2000 mutants (`--seed 7`)
differ only by discrepancy D1.

## Module layout

| Module | Lines | What | Stage 0 original |
|---|---:|---|---|
| `syntax.kinds` | 538 | `SyntaxKind` (246 kinds), the keyword and punctuation tables, kind predicates, names in dumps | `sisu-syntax/src/kinds.rs` |
| `syntax.tree` | 338 | `Token`, the arena tree, the builder with checkpoints, the dumps, the reprint, stable identities | `sisu-syntax/src/tree.rs` |
| `diag` | 343 | `Msg` (the catalog identifiers stage 1 reports), diagnostics with labels and notes, the catalog reader, positions, `render_ids`, `quote`, the stable sort | `sisu-diag` (the parts the front end uses) |
| `text`, `text.unicode` | 369 | UTF-8 decoding, `is_alphanumeric`, Rust's `Debug` quoting, and the Unicode tables it needs (pinned, Unicode 17.0.0) | the host's `char` methods and `{:?}` |
| `lexer` | 262 | `lex`, and stage 0's lexer tests | `sisu-lexer/src/lib.rs`, `tests.rs` |
| `lexer.scan` | 536 | the raw scanner: every byte a token, strings and interpolation as tokens | `sisu-lexer/src/scan.rs` |
| `lexer.layout` | 325 | Python's layout algorithm, continuation, bracket recovery | `sisu-lexer/src/layout.rs` |
| `parser` | 439 | `parse`, the diagnostic filters (bracket echoes, structure echoes, one per line), and stage 0's parser tests | `sisu-parser/src/lib.rs`, `tests.rs` |
| `parser.state` | 293 | the parser's state: looking, attaching tokens with their trivia, reporting | `sisu-parser/src/parser.rs` |
| `parser.items`, `.stmts`, `.exprs`, `.patterns`, `.types` | 2110 | one function per rule of the grammar | the files of the same names |
| `mutants` | 109 | a seeded generator and the mutations of stage 0's robustness test | `sisu-driver/src/tests.rs` |
| `main` | 175 | the command line | `sisu-driver/src/cli.rs`, `tokens` and `parse` |

5837 lines in all, about 700 of them tests; the Rust it transcribes
(`sisu-lexer`, `sisu-parser`, and `kinds.rs` and `tree.rs` of `sisu-syntax`)
is 5508 lines with its tests.

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
Rust's `is_some_and`. Stage 0's unit tests came along: 43 of them (the
lexer's, the parser's except those of the AST, the kinds', the tree's and
the source map's columns) are tests of stage 1 now, beside 12 new ones.

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
12. **Compiling the package takes about a second** on every `sisu run`, 0.58 s
    of it the ownership checker on these 5800 lines; `batch` exists so that
    one run covers many files.

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
about 1 s spent compiling stage 1. Over the corpus, about a quarter of the
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

## What is next

1. **Name resolution for one module**, against `sisu check --format=ids` on
   the `names/` cases: declare the module's items and the prelude's (parsed
   by this parser), resolve every name in bodies, signatures and patterns
   with scopes, report `E0200`/`E0201`/`E0202` (with the similar-name fix and
   the first-defined label), `W0002` shadowing and poisoning. The first step
   is the AST: stage 0's `ast::build` (1961 lines) as a total function of
   this tree.
2. **The type checker in Sisu**, against the `types/` cases, then the
   ownership checker, then **SIR emission** against the `.sir` files of the
   suite and `sisu sir`, which is where stage 1 becomes the oracle.
3. **Speed** where it pays: the scanner over a byte list, dumps without
   interpolation, and, in the interpreter, views that do not copy.
4. **The discrepancies**: once the coordinator has decided D1 to D3, either
   stage 0 changes (and `check.sh` stops listing them) or stage 1 does.
