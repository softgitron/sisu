# Stage 1: the Sisu front end in Sisu

Stage 1 of the bootstrap ([18](../../proposal/18-next-steps.md),
"Bootstrapping"): the lexer, the lossless syntax tree, the error-tolerant
parser, the AST, the name resolution of one module, the type checker with
its template engine, lowering to SIR and the ownership checker, written in
Sisu, run by the stage-0 interpreter, and checked byte for byte against
the Rust front end (`sisu-lexer`, `sisu-syntax`, `sisu-parser`, and
`ast::build`), against the name, type and ownership diagnostics of `sisu
check`, and against the SIR `sisu sir` prints. What runs SIR (the
interpreter, and with it the evaluation of `comptime` code) is not here.

It is a transcription: every rule of
[docs/architecture.md](../docs/architecture.md) ("Lexing", "Parsing and the
syntax tree", "Error tolerance") and every function of the Rust crates has
its counterpart here, under the same name where the language allows. Where
the Rust behaviour is an accident rather than a rule, stage 1 follows the
documents and the difference is listed in
[DISCREPANCIES.md](DISCREPANCIES.md) until the coordinator decides it
(D-079). None is listed now: the first round's five (three of the
parser's, one of name resolution and one of the type checker's) were
decided for the documents, and stage 0 now behaves as stage 1 does
(docs/architecture.md, decisions 165 to 168 and 179).

## Running it

From this directory, with stage 0 built (`cargo build --release` in
`compiler/`):

```text
sisu test                                   # 88 tests, 50 of them stage 0's own, transcribed
sisu run -- tokens [--trivia] FILE          # what `sisu tokens [--trivia] FILE` prints
sisu run -- parse --tree [--trivia] FILE    # what `sisu parse --tree [--trivia] FILE` prints
sisu run -- parse --print FILE              # the file, reprinted from the tree
sisu run -- parse --ast FILE                # what `sisu parse --ast FILE` prints: the AST, as Rust's `{:#?}`
sisu run -- names FILE                      # the name diagnostics of `sisu check --format=ids FILE`
sisu run -- types FILE                      # the type checker's diagnostics of the same command
sisu run -- batch OUT LIST                  # tokens, trees and ASTs for every file LIST names, into OUT
sisu run -- ast-batch OUT LIST              # the AST of every file LIST names, into OUT
sisu run -- names-batch OUT LIST            # the name diagnostics of every file LIST names, into OUT
sisu run -- types-batch OUT LIST            # the type diagnostics of every file LIST names, into OUT
sisu run -- sir FILE                        # what `sisu sir FILE` prints: the program's SIR
sisu run -- ownership FILE                  # the ownership checker's diagnostics of `sisu check --format=ids FILE`
sisu run -- sir-batch OUT LIST              # both, for every file LIST names, into OUT
sisu run -- mutate SEED COUNT OUT LIST      # mutants of the files LIST names
sisu run -- profile FILE                    # the time lexing, parsing and each dump of FILE take
./check.sh [--mutants N] [--seed S] [--names] [--types] [--sir] [--keep DIR]
```

Stage 1 prints to standard output and its diagnostics to standard error,
in the conformance form (`--format=ids`), as stage 0 does; `names` and
`types` print their diagnostics to standard output, as `sisu check` does,
and `types` then prints on standard error why parts of the file are beyond
the slice it checks; `sir` and `ownership` print on standard error why
the file is beyond the slice of lowering, when it is. It reads the
message catalog at run time, by default `../catalog/messages.catalog`
(`--catalog PATH` names another), and the standard library's sources from
`../std/`, so it is run from this directory; the tests read the catalog and
the proposal's examples the same way.

`check.sh` builds stage 0 in release mode, makes stage 0's outputs for
every file, runs stage 1 once per class of files (`batch`, one interpreter
run for many files), and compares five outputs per file: `tokens` (`sisu
tokens --trivia`, standard output and error), `tree` (`sisu parse --tree`,
both), `trivia-tree` (`sisu parse --tree --trivia`), `print` (stage 1's
reprint against the file itself) and `ast` (`sisu parse --ast`, the AST,
below), where a file whose tree differs as listed may differ in its AST
too. Then it compares `names`: the name diagnostics of `sisu check
--format=ids` (the lines with `E0200`, `E0201`, `E0202`, `E0220`, `E0234`,
`E0389` or `W0002`, with their labels and fixes) against stage 1's (`names-batch`), for the conformance suite's
`names/` cases and stage 1's own cases, and with `--names` for every
program of the corpus that is one module (the conformance cases outside
`packages/`, and the proposal's examples). Then `types`: the type
checker's diagnostics of `sisu check --format=ids` (those whose identifier
begins with `type.`, with their labels and fixes) against stage 1's
(`types-batch`), for the files `types.list` names, each of which must be
within the slice and match, and with `--types` for every program of the
corpus that is one module, where a file within the slice must match and
any other may only report fewer of stage 0's diagnostics. Then `sir` and
`ownership`: what `sisu sir` prints, and the ownership checker's
diagnostics of `sisu check --format=ids` (`E0333`, `E0366`, `E0367`,
`E0392`, `E0393`, `E0397`, `E0400` to `E0410`, `E0416` to `E0420`, `E0428`
and `E0429`, with their labels and fixes), against stage 1's (one run of
`sir-batch` for both), for the files `sir.list` names, each of which must
be within the slice of lowering and match, and with `--sir` for every
program of the corpus that is one module, where a file within the slice
must match and any other is beyond it. It exits non-zero on any
difference that DISCREPANCIES.md does not list ("- Differs: `path`:
outputs"). With `--mutants N` it also compares N mutants of the corpus
(deleted, inserted and swapped characters), where a difference must carry
the mark of a listed discrepancy.

## Results

`./check.sh --names --types`:

```text
stage 1 against stage 0 (tokens, tree, trivia-tree, print, ast):
  conformance   400 files,  400 identical,   0 differing as listed,   0 differing otherwise; stage 1   34.2s (259853 bytes; lex 2943 ms, parse 4500 ms, dumps 6727 ms, ast 10516 ms)
  std            15 files,   15 identical,   0 differing as listed,   0 differing otherwise; stage 1   25.6s (238695 bytes; lex 2029 ms, parse 2816 ms, dumps 4591 ms, ast 7616 ms)
  examples       11 files,   11 identical,   0 differing as listed,   0 differing otherwise; stage 1   11.0s (27495 bytes; lex 256 ms, parse 328 ms, dumps 508 ms, ast 853 ms)
  cases          25 files,   25 identical,   0 differing as listed,   0 differing otherwise; stage 1   10.0s (13816 bytes; lex 175 ms, parse 305 ms, dumps 424 ms, ast 557 ms)
stage 1's name resolution against stage 0's (names):
  names          35 files,   35 identical,   0 differing as listed,   0 differing otherwise; stage 1   14.3s
  corpus-names  364 files,  364 identical,   0 differing as listed,   0 differing otherwise; stage 1   25.3s
stage 1's type checker against stage 0's (types):
  types         368 files,  368 identical,   0 beyond the slice and a subset,   0 differing as listed,   0 otherwise; stage 1  166.0s
  corpus-types  374 files,  365 identical,   9 beyond the slice and a subset,   0 differing as listed,   0 otherwise; stage 1  179.2s
stage 1's lowering against stage 0's (sir):
  sir           329 files,  329 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise; stage 1  513.1s (front end 110s, lowering 267s, ownership 95s)
stage 1's ownership checker against stage 0's (ownership):
  ownership     329 files,  329 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise (the same run)
total: 2250 files checked, 2241 identical, 9 beyond the type checker's slice and a subset, 0 differing (0 listed in DISCREPANCIES.md, 0 not)
```

All 426 files of the conformance suite, `compiler/std/` and
`proposal/examples/` give identical token streams with trivia, trees with
and without trivia, diagnostics, reprints and ASTs. `cases/` holds stage
1's own inputs for the error-tolerance rules the corpus never triggers
(operator mixing and the parentheses its fixes add, Python's conditional
expression, statement, expression, declaration, lexical, indentation,
bracket and structure mistakes, the keyword that opens a missing block, the
token a line misses at its end, the forms of later phases, scripts, CR LF,
Unicode beyond ASCII, type arguments where a value is written), for the
type checker's rules (calls, conversions, control flow), the names in
constant arguments, and the five discrepancies of the first round, which
stage 0 now shares. With `--mutants 2000 --seed 7` all 2000 mutants give
identical outputs, their ASTs included (in the first round 21 differed, by
D1). Of the 364 programs of one module, every one gets the name
diagnostics of stage 0. Of the 374 programs of one module (the 364 and the
conformance suite's `names/` cases), 365 are within the type checker's
slice and get stage 0's type diagnostics exactly; the other 9 get a subset
of them, and each is beyond only for a message of the ownership checker's.
Of the 374 and stage 1's three cases of type rules, 329 are within the
slice of lowering and get stage 0's SIR and the ownership checker's
diagnostics exactly, the 9 among them.

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

A name in a type's constant argument written as an expression
(`Array[int, N * W]`) is looked up as stage 0's folding looks it up
(`consts.rs`): the expression is folded to its first error, and only a
name reached before any other error, that names no generic parameter
(in a body, no `const` one), no local in a body and no item, is `E0200`,
without a fix; what is wrong inside a constant's own value is reported
where that value is checked (`cases/const_argument_names.sisu`).

`check.sh` compares its diagnostics with the name diagnostics of `sisu
check --format=ids` (above), and none differs. Stage 0 reports a mistake
it finds more than once (in the body the cases of a parametrised test
share) once (decision 168), as stage 1 always did.

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

`sisu parse --ast FILE` prints stage 0's AST: the module's `{:#?}`, its
items, doc comments and broken names. `syntax.ast.dump` writes stage 1's
in the one-line notation of Rust's derived `Debug`, field for field and
following the ids, and `pretty` lays it out as `{:#?}` does (each field and
element on a line of its own, four spaces deeper, ending in a comma);
`sisu run -- parse --ast FILE` prints it, and `check.sh` compares the two
for every file. (A small Rust program over the stage-0 crates, `oracle/`,
printed stage 0's AST before stage 0 had the command; it is gone.)

## The type checker's slice

The type checker of stage 0 is `sisu-resolve` (the declarations and their
signatures) and `sisu-typeck` (the bodies), 22 000 lines of Rust. Stage 1
transcribes `sisu-typeck` whole, rule for rule (the checker of `check.rs`,
`expr.rs`, `call.rs` and `pat.rs`, exhaustiveness, contracts and
refinements, `pure`, the rules of tasks, and the driver of `lib.rs`), and of
`sisu-resolve` what the declarations of one file need; and it knows where
that ends: a body that uses anything beyond it is checked to the end but
reports nothing, and says why (`types` prints the reasons; `types-batch`
writes them to `OUT/n.beyond`). So a file with nothing beyond the slice
must get exactly stage 0's type diagnostics, and any other a subset of
them, which `check.sh --types` checks over the corpus.

How a file is checked (`typeck.sisu`): the prelude, the std modules every
module sees (`task`, `log`, `cap`, `live`), `target` and `builtin_impls`
(the impls of the leaf types and tuples, whose text stage 0 writes in
`std_sources.rs` and `typeck.std_sources` writes the same way) are parsed
by stage 1's parser, built into ASTs and resolved once (`typeck.collect`),
and the std modules a program may import are parsed once. Each file's root
module is then declared and resolved on a copy of that program, in stage
0's order: its items, with what the compiler writes for them as it
declares them (a script's `main`, the cases of a parametrised test, the
check of a struct's `invariant` lines, the interceptor and the fake of
each of its traits when its text names `Wrap` or `Fake`, and the module of
its doc examples' tests); then its imports (an imported std module is
declared and resolved with the file, after it, so that its definitions
come after the file's as in stage 0); then the impls of `@derive` and
`@delegate` and the `Debug` every struct and enum has (their headers,
which are what the checker reads) and the generators of `arbitrary[T]`
and `exhaustive[T]`; the predicate of a refinement type is written as the
type is resolved. What stage 0 writes as source text (the interceptors
and fakes, the doc examples' module, whose text keeps every example line
where the file has it) stage 1 writes the same way and parses; the rest it
builds as syntax. The error sets of the `T !` functions are inferred
first, to a fixpoint over the program; then each function body of the
root module is checked by a checker of its own (`typeck.check`,
`typeck.expr`, `typeck.call`, `typeck.pat`, `typeck.contracts`,
`typeck.tasks`, with `typeck.infer`'s unification), which builds the
typed tree (`typeck.thir`, in arenas) that exhaustiveness
(`typeck.exhaustive`), purity (`typeck.purity`) and the captures of
closures and tasks are decided on; its bounds are checked against the
impls once inference is done (`typeck.program`'s solver, `Send` and
`Sync` included); then the doc examples' tests, with their module's own
syntax and text, their diagnostics put at the same lines and columns of
the file; then the defaults of fields and parameters and the constants,
then `main`'s and the tests' signatures. Diagnostics on a line with a
syntax error are dropped (but for the examples', whose module is a file
of its own), and the rest are sorted by start, end and code, a diagnostic
identical to an earlier one dropped (`sort_unique`, decision 168), as the
driver does. Types are interned (`typeck.ty`): a type is an index into a
table, so two types are equal when their indices are.

Two things round 5 of stage 0 added go through every part of this:
`f32`, a type of the prelude whose literals are rounded once from their
text (`float_literal`: `1.5u8` is `E0332`, a literal that rounds to an
infinity in its type `E0374`), which widens to `f64` where one is expected
and beside an `f64` operand (`Coercion.FloatWiden`) and mixes with
nothing else; and a type's constant argument written as an expression,
`Array[T, W * H]` (decisions 173 and 174). The parser reads such an
argument as an expression in a `CONST_ARG` (`at_const_expr`), the AST
holds it as `TypeKind.ConstExpr`, and the checker folds it where the type
is written (`typeck.consts`, from stage 0's `consts.rs`): integers,
`const` parameters, the program's constants and the integer operators
with the run time's rules (`sir.ConstOp.fold`). An operation on `const`
parameters stays one (`TyData.ConstOp`) until substitution or unification
knows its arguments, and equals only the same operation on equal
arguments. A checker of stage 1 holds the syntax of its own module only,
so a constant keeps its value as folding reads it (`typeck.folding`'s
terms) beside its definition.

Templates (08, "Derive" and "Reflection"; stage 0's `template.rs`,
`derive.rs` and `instances.rs`) are unfolded as stage 0 unfolds them
(`typeck.template`, `typeck.instances`): a trait's `comptime def derive`,
the prelude's or the program's own, and a generic function that reflects
on its type parameters run on an evaluator of compile-time code over the
shapes of types (fields, variants, attributes) that writes source text;
the text is parsed, declared in a module of its own whose scope is the
template's (`<derive>/main.sisu`, `<NAME[ARGS]>/main.sisu`) and checked
like any other code, a diagnostic there being at its line and column in
that text. Which instances a program needs is known only once its calls
are typed, so the driver checks the program, declares what its bodies
asked for, and checks again until no new request comes; a generic
function that calls a template with its own type parameters becomes a
template too, and the program is resolved again (stage 0's
`Db::checked`). The impls the standard library derives for its own types
stay headers here (lowering's slice ends there, below).

What it reports: every `type.` message of the catalog but the six that
stage 0's ownership checker reports, on SIR, after the type checker
(`E0333`, `E0366`, `E0367`, `E0392`, `E0393`, `E0397`). The front end's
other messages (`resolve.`, `stage0.`, `comptime.`, `contract.`,
`refine.`, `task.`, `cap.`, `borrow.`, `test.`) are not compared, but
stage 1 notes where stage 0 reports one, for two rules that depend on it:
a local whose type inference left open is reported only in a body without
an error, and the ownership checker runs only when the front end found
none. The names pass's errors come from stage 1's own (`resolve`).

What is beyond it:

- in the module, which then reports nothing at all: a contract of a
  struct other than `invariant`, `@structural` and `@keeps`, variadic
  parameters, a `ref` in a declaration's type, and an import of a std
  module stage 1 does not read;
- the `type.` messages of stage 0's ownership checker: when the front end
  finds no error, a body that may get one is beyond the type checker's
  slice, since `types` does not report them. That is a pure body that
  calls an impure function or function value (`E0333`, `E0392`), uses
  `select` or loops over an `Iterator`, or holds a value whose drop runs a
  `drop` that is not `pure` (`E0393`); a `let` without a value (`E0366`,
  `E0367`); and a call of a generic function with a must-consume type
  argument (`E0397`). Stage 1's ownership checker decides them (below),
  and lowering goes past them.

No choice of stage 1 differs from stage 0 within the slice (the count
`E0305` reports for positional arguments past a bare `*`, D5, is stage
0's now). One difference of order has no effect on what is reported:
stage 0 declares
every module's items before it resolves any, so its definitions of the root
module come before those the prelude's impls and traits make; stage 1
resolves the prelude once, so they come after.

`check.sh` checks the slice twice: `types.list` names the 365 programs of
one module that are within the slice (62 of the 63 of
`conformance/cases/types/` and `wrong/`, all but `pure_violation.sisu`,
whose `E0333` is the ownership checker's) and stage 1's three cases of
type rules, which must match (all 368 do); `--types` checks all 374
programs of one module, of which those 365 are within the slice and the
other 9 report a subset. The reason each of the 9 is beyond is one of the
ownership checker's, by count: a `let` without a value (5, two of which
get `E0366` or `E0367`), a pure body that calls an impure function (2,
both `E0333`), a pure body that drops a value with an impure `drop` (1,
`E0393`), a must-consume type argument (1, `E0397`). Round 5's 18 had
nine more, the derive templates of the programs' own (5) and the generic
functions that reflect on their type parameters (4), which the template
engine brought within the slice.

## Lowering and the ownership checker

`lower.sisu` and `lower/` transcribe stage 0's `sisu-lower` (the functions
of `lib.rs`, `func.rs`, `stmts.rs`, `exprs.rs`, `pats.rs`, `contracts.rs`
and `glue.rs`), `sir.sisu` and `sir/` its SIR (`sisu-sir`'s data,
`types.rs`, `write.rs` and `simplify.rs`), and `borrowck.sisu` and
`borrowck/` the ownership checker (`sisu-borrowck`: `paths.rs`,
`init.rs`, `elaborate.rs`, `loans.rs`, `consume.rs`, `purity.rs`,
`names.rs`, `types.rs`, `bitset.rs` and `check_with` of `lib.rs`), rule
for rule. Together they make what `sisu sir` prints and the ownership
checker's part of `sisu check`, for a program of one module:

1. The type checker keeps what lowering reads (`typeck.Typed`): the
   resolved program, the table of types and the typed bodies, in stage 0's
   order of `typed.bodies`, which decides the order of SIR's functions and
   the numbering of its source files. Stage 0 numbers its definitions as
   it declares them, every module's items before any is resolved; stage 1
   resolves the prelude once, so it records the phase each definition was
   made in (`Program.phases`: declared, a type resolved, a trait, an impl,
   the rest, after the root module) and orders by phase, then by index.
2. Lowering starts from `main` and the tests and lowers what they reach:
   the functions they call or name, the defaults, drops and invariant
   checks of the types they use, and the methods of the impls of the
   traits they call through `call trait` for the types they mention (stage
   0's `lower_reaching`). A body of the standard library is type-checked
   when lowering first reaches it, from the syntax and text the base keeps;
   the drop glue of each declared type that needs one is written from the
   types (`lower.glue`).
3. The ownership checker runs as stage 0's `check_with` does, without its
   memo: per function, the initialisation dataflow and its findings, then
   drop placement; then, for the functions that are not the standard
   library's (`trust_std`, as `sisu check` runs it), the loans and
   must-consume; then purity over the whole program; and a finding that
   repeats one goes. The findings become diagnostics of the program's
   file, sorted as `sisu check` sorts them.
4. Jump threading (`sir.simplify`), and the text form (`sir.write`).

The front end must agree with stage 0 on whether a program has an error,
since then `sisu sir` prints nothing and the ownership checker does not
run: stage 1 notes each error of stage 0's that is not the type checker's
(the resolver's `resolve.` and `borrow.` rules, among them the borrowed
signatures of `borrows.rs`, which `typeck.borrows` transcribes, the
coherence of impls and object safety) without reporting it.

What is beyond the slice of lowering (`sir` and `ownership` print why on
standard error, `sir-batch` writes it to `OUT/n.beyond`): what is beyond
the type checker's slice, but the bodies that may get one of the
ownership checker's `type.` messages, which the ownership checker here
decides; a `comptime` expression, whose value stage 0 computes into the
SIR by running it; a `comptime assert` on `const` parameters, which stage 0
decides at each call; a function with a contract, or a `mut self` method
of a type with an `invariant`, whose test stage 0 writes with generators;
a `@refines` property and a generator of `arbitrary` or `exhaustive`, for
the same reason; an impl of `@delegate`; an impl the standard library
derives for its own types (`Debug` of `Option`), whose methods stage 1
declares as headers; a `Shared` that may cross to another task, which
stage 0's escape analysis (`shared.rs`) tags; a body of the standard
library beyond the type checker's slice; and an ownership finding in a
file other than the program's. Of the 374 programs of one module and
stage 1's three cases of type rules, 329 are within the slice of lowering
and get stage 0's SIR and ownership diagnostics exactly: 185 are lowered,
43 of them with diagnostics of the ownership checker (21 of its messages
between them; `E0392`, `E0419` and `E0420` occur nowhere in the corpus),
and 144 have a front-end error, for which both print nothing. `sir.list`
names the 329. The other 48 are beyond the slice, by the first reason:
an impl the standard library derives (22), a `comptime` expression (9), a
contract's test (6), an impl of `@delegate` (3), a generator of
`arbitrary` or `exhaustive` (3), a `comptime assert` on `const` parameters
(2), a `@refines` property (2), a `Shared` that may cross to another task
(1).

The ownership checker was transcribed whole before it ran, and it matched
stage 0 on every program it checked the first time, the conformance
suite's ownership cases included; what took the work was lowering, whose
SIR depends on everything the front end decided (the order of
definitions, the bodies of the standard library a program reaches, the
names stage 0 gives what it writes). One rule of stage 0's checker shows
in stage 1's own source: a `let` with a tuple pattern and `else` (`let
(name, tys) = found else: continue`) is reported as a use after a move
of the names it binds (item 19 below), so `borrowck.consume` binds them in
two steps; the transcription keeps the rule.

## Module layout

| Module | Lines | What | Stage 0 original |
|---|---:|---|---|
| `syntax.kinds` | 538 | `SyntaxKind` (246 kinds), the keyword and punctuation tables, kind predicates, names in dumps | `sisu-syntax/src/kinds.rs` |
| `syntax.tree` | 338 | `Token`, the arena tree, the builder with checkpoints, the dumps, the reprint, stable identities | `sisu-syntax/src/tree.rs` |
| `diag` | 922 | `Msg` (the catalog identifiers stage 1 reports), diagnostics with labels, fixes and notes, the catalog reader and its templates, positions, `render_ids`, `quote`, the stable sort and `sort_unique` | `sisu-diag` (the parts the front end uses) |
| `text`, `text.unicode` | 369 | UTF-8 decoding, `is_alphanumeric`, Rust's `Debug` quoting, and the Unicode tables it needs (pinned, Unicode 17.0.0) | the host's `char` methods and `{:?}` |
| `lexer` | 262 | `lex`, and stage 0's lexer tests | `sisu-lexer/src/lib.rs`, `tests.rs` |
| `lexer.scan` | 558 | the raw scanner: every byte a token, strings and interpolation as tokens | `sisu-lexer/src/scan.rs` |
| `lexer.layout` | 347 | Python's layout algorithm, continuation, bracket recovery and its fixes | `sisu-lexer/src/layout.rs` |
| `parser` | 517 | `parse`, the diagnostic filters (bracket echoes, structure echoes, one per line), and stage 0's parser tests | `sisu-parser/src/lib.rs`, `tests.rs` |
| `parser.state` | 403 | the parser's state: looking, attaching tokens with their trivia, reporting, the fixes added later, Python's conditional | `sisu-parser/src/parser.rs` |
| `parser.items`, `.stmts`, `.exprs`, `.patterns`, `.types` | 2213 | one function per rule of the grammar | the files of the same names |
| `syntax.ast` | 748 | the AST: items, statements, expressions, types and patterns in arenas, referred to by id | `sisu-syntax/src/ast/mod.rs` |
| `syntax.ast.build` | 2149 | `build.module`, the AST of a tree as a total function, and the decoding of literals | `sisu-syntax/src/ast/build.rs` |
| `syntax.ast.dump` | 1398 | the AST in the notation of Rust's derived `Debug`, laid out as `{:#?}`, for `check.sh` | Rust's `{:#?}`, which `sisu parse --ast` prints |
| `syntax.ast.visit` | 307 | the expressions of items, blocks and expressions, each before the ones inside it | `sisu-syntax/src/ast/visit.rs` |
| `resolve` | 2065 | the names of one module: declarations, imports, scopes, the prelude's and std's names, and the name diagnostics | `sisu-resolve/src/collect.rs` and `types.rs`, and the name lookups of `sisu-typeck` |
| `sir`, `sir.types`, `sir.write`, `sir.simplify` | 1466 | SIR: programs, functions, blocks, statements and types, with the operations a type's constant argument may compute; the types of places; the text form `sisu sir` prints; jump threading | `lib.rs`, `types.rs`, `write.rs` and `simplify.rs` of `sisu-sir` |
| `typeck.ty` | 376 | types, interned in a table | `sisu-resolve/src/ty.rs` |
| `typeck.program` | 1854 | the resolved program, the spelling of types, `Copy`, the trait solver, `Send` and `Sync`, visibility, views and must-consume types | `program.rs`, `ty.rs`, `solve.rs` and `auto.rs` of `sisu-resolve` |
| `typeck.borrows` | 386 | what a signature borrows: the parameters a result may borrow from (`from`, and the elision rules), the borrowed-signature errors | `sisu-resolve/src/borrows.rs` |
| `typeck.std_sources` | 441 | `target` and `builtin_impls`, the std text stage 0 writes, and the import paths of the std modules | `sisu-resolve/src/std_sources.rs` |
| `typeck.collect` | 4003 | declarations, signatures, imports, traits and impls, resolved, and what the compiler writes: derived impls, interceptors and fakes, generators, invariants, predicates, a script's `main`, test cases, doc tests | `collect.rs`, `types.rs`, `traits.rs`, `derive.rs`, `shims.rs`, `arbitrary.rs`, `tests_of.rs` and `doctests.rs` of `sisu-resolve` |
| `typeck.template` | 1972 | the evaluator of compile-time code that unfolds a template (a `comptime def derive`, a generic function that reflects on its type parameters) into source text | `sisu-resolve/src/template.rs` |
| `typeck.instances` | 568 | the impls `@derive` makes for the root module's types and the instances of templates, declared in modules of their own | `derive.rs` and `instances.rs` of `sisu-resolve` |
| `typeck.consts`, `typeck.folding` | 341 | a type's constant argument written as an expression, folded where it is written; an expression as folding reads it | `sisu-resolve/src/consts.rs` |
| `typeck.infer`, `typeck.thir` | 767 | inference variables and unification; the typed tree | `infer.rs` and `thir.rs` of `sisu-typeck` |
| `typeck.check`, `.expr`, `.call`, `.pat` | 6642 | the body checker: statements, expressions, calls, patterns | `check.rs`, `expr.rs`, `call.rs` and `pat.rs` of `sisu-typeck` |
| `typeck.exhaustive` | 572 | exhaustiveness and unreachable arms of `match` | `sisu-typeck/src/exhaustive.rs` |
| `typeck.contracts`, `.purity`, `.tasks` | 1338 | contracts and refinements, `pure`, structured concurrency and generators | `contracts.rs`, `purity.rs` and `tasks.rs` of `sisu-typeck` |
| `typeck` | 1333 | a body's checks, a file's, the inferred error sets, the driver, and the typed bodies lowering reads | `sisu-typeck/src/lib.rs` |
| `lower`, `lower.names`, `.func`, `.stmts`, `.exprs`, `.pats`, `.contracts`, `.glue` | 5572 | lowering the typed bodies to SIR: what `main` and the tests reach, names, functions and closures, statements, expressions, patterns, contracts, drop glue | `lib.rs`, `func.rs`, `stmts.rs`, `exprs.rs`, `pats.rs`, `contracts.rs` and `glue.rs` of `sisu-lower` |
| `borrowck`, `borrowck.paths`, `.init`, `.elaborate`, `.loans`, `.consume`, `.purity`, `.names`, `.types`, `.bitset`, `.finding` | 4877 | the ownership checker: move paths, initialisation and moves, drop placement, loans, must-consume, purity, places in source terms, and the findings as diagnostics | `sisu-borrowck` (but `explain.rs` and `shared.rs`) |
| `mutants` | 109 | a seeded generator and the mutations of stage 0's robustness test | `sisu-driver/src/tests.rs` |
| `main` | 524 | the command line | `sisu-driver/src/cli.rs`, `tokens`, `parse`, `check` and `sir` |

46275 lines in all, about 1100 of them tests; the Rust the lexer and the
parser transcribe (`sisu-lexer`, `sisu-parser`, and `kinds.rs` and
`tree.rs` of `sisu-syntax`) is 5717 lines with its tests, the AST and its
builder 2782, `sisu-resolve` and `sisu-typeck`, of which stage 1
transcribes `sisu-typeck` whole and of `sisu-resolve` what one file needs,
22483, `sisu-lower` 5329 with its tests, the parts of `sisu-sir` stage 1
has (its data, the types of places, the writer and jump threading) 1486,
and `sisu-borrowck` 5299 with its tests, all of it but the explanation of
loans (`explain.rs`) and the `Shared` escape analysis (`shared.rs`).

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
Rust's `is_some_and`. Stage 0's unit tests came along: 50 of them (the
lexer's, the parser's except those of the AST, the AST builder's, the
kinds', the tree's, the source map's columns, the inference table's, and the
type errors of `sisu-typeck`'s tests that are within the slice) are tests of
stage 1 now, beside 38 new ones (among them the template engine's, drop
placement's and the ownership checker's, each against what stage 0
prints for the same program).

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
12. **Compiling the package takes about 8 s** on every `sisu run` (`sisu
    check` of these 46 000 lines takes about as long; of the first 5800,
    0.58 s was the ownership checker's); `batch` and its kin exist so that
    one run covers many files.
13. **A recursive type cost the ownership checker exponential time.**
    `sisu-borrowck`'s `Types::must_consume_depth` followed the types of
    fields with no memory of what it had seen and a depth limit of 32; a
    type with several recursive fields (the Rust AST's `Box<Expr>`, in a
    dozen variants) made `sisu check` of its users take hours. Round 5
    fixed it (decision 180: each type is looked at once); stage 1's AST and
    typed tree stay arenas with typed ids, and its types stay interned, but
    the template engine's shapes nest through lists.
14. **A `dict` key could not hold a list when the program ran** ("List([])
    is not hashable"); round 5 made lists keys as their `Hash` impls
    declare (decision 181). The table of types still finds a type by a
    text that names its parts.
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
18. **The alternatives of `|` cannot bind names** (`E0385`, "yet"): Rust's
    `Stmt::Assign { place, value, .. } | Stmt::AugAssign { place, value, ..
    }` is two arms with the same body, and the type checker has some thirty
    of them.
19. **`let x = xs.first() else:` stops the interpreter when the element is
    a `Copy` struct**: the binding keeps the reference that `first` gives,
    and the first use of a field of `x` is "internal error in the
    interpreter: Field(0) does not apply to Ref(...)". `if let Some(x) =
    xs.first():`, a `match`, or indexing after a length check work; the
    checker never uses the form. A `let` with a tuple pattern and `else`
    (`let (d, args) = found else:`) is misread by the ownership checker
    instead (a name "used before it is assigned", or "moved" at its own
    binding), and is written in two steps.
20. **`sisu test`'s monitor reports passing a borrowed parameter on as a
    `mut` borrow of it**, which ends the loans of what was read through it:
    `case TPatKind.List(prefix, ...)` on `c.pats[...]` and then
    `self.product(c, ...)` is "a reference was used after its loan had
    ended" under `sisu test`, and runs under `sisu run`. Copying what the
    pattern bound before the call satisfies both. Lowering's round met it
    three times more: a `mut` parameter passed on to a view parameter
    stays `mut` to the monitor, so the callee's own copy of it ends the
    loans the callee made through it (the drop placement's decisions, the
    walk of a type's parts in `typeck.borrows`, the checker's loop over the
    functions); the static checker accepts each.
21. **`own`, `def` and `from` are keywords**, so Rust's variables and
    parameters of those names become `owns`, `defaults` and `from_`.

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

The type checker adds its own costs: resolving the prelude, the four
ambient std modules and `builtin_impls`, and parsing the ten std modules a
program may import, takes about 25 s, nearly all of it parsing, and each
file then about half a second (copying the resolved program is 0.13 s of
it; a file that imports a std module declares and resolves it too, as
stage 0 does); `types-batch` checks the 374 programs of the corpus in about
155 s (180 s for 360 before round 5 made the interpreter faster,
decisions 175 to 178; the template engine checks a file again for each
round of instances it declares). Its tests use a small prelude of their
own.

Lowering and the ownership checker add about a second a program:
`sir-batch` takes 513 s for the 329 programs of `sir.list`, of which the
front end is 110 s, lowering 267 s (most of it type-checking, for each
program again, the bodies of the standard library it reaches, since the
table of types is the program's) and the ownership checker with the
writing of the SIR 95 s. With them, `./check.sh --names --types` takes
about 20 minutes.

## What is next

1. **The rest of lowering's corpus**: the impls the standard library
   derives for its own types (the template engine run once on the std
   modules' types, in the base, as stage 0 writes `<std>/<derive>/`),
   `comptime` expressions (an evaluator of SIR, which stage 0 has in its
   interpreter), the generators of `arbitrary` and `exhaustive` with the
   contract tests and `@refines` properties they feed (`arbitrary.rs`),
   the methods of `@delegate`, and the `Shared` escape analysis
   (`shared.rs`). Then **running SIR**, where stage 1 becomes the oracle.
2. **Speed** where it pays: lowering checks the bodies of the standard
   library a program reaches anew for every program (the table of types is
   the program's); the scanner over a byte list, dumps without
   interpolation, and, in the interpreter, views that do not copy.
3. **The discrepancies**: none is listed; the first five were decided for
   the documents and stage 0 changed (D-079). A new one is listed from D6
   on and decided the same way.
