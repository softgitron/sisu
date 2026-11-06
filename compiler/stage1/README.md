# Stage 1: the Sisu front end in Sisu

Stage 1 of the bootstrap ([18](../../proposal/18-next-steps.md),
"Bootstrapping"): the lexer, the lossless syntax tree, the error-tolerant
parser, the AST, the name resolution of a program of one module or of
several modules and packages, the type checker with its template engine,
lowering to SIR, the ownership checker with the `Shared` escape analysis,
the evaluation of `comptime` code, the formatter, a `pure` function of
the lossless syntax tree, and `sisu doc`, a `pure` function of the syntax
trees, the resolved names and the inferred error sets, written in Sisu,
run by the stage-0 interpreter or compiled by stage 0's native backend.
Since this round it is Sisu's front end: the Rust one is frozen, and a
rule of the language changes here only (below, "Stage 1 is the front
end"). It is checked against what the conformance suite's programs must
do, run from the SIR it writes by stage 0's interpreter and native
backend, and byte for byte against the frozen Rust front end (`sisu-lexer`,
`sisu-syntax`, `sisu-parser`, and `ast::build`), against the name, type
and ownership diagnostics of `sisu check`, against the SIR `sisu sir`
prints, against what `sisu fmt` writes, and against the pages `sisu doc`
writes, for the whole corpus and for stage 1's own package, but where
[RULES.md](RULES.md) lists a rule stage 1 decides otherwise. With `sisu
doc`, every part of stage 1 that [18](../../proposal/18-next-steps.md)'s
stage table names exists: the lexer, the parser, name resolution, the
type checker, the ownership checker, the emission of serialised SIR, the
formatter and `sisu doc`. What runs a program's SIR is stage 0's
interpreter; stage 1 has the part of it that compile-time code needs
(`interp`).

It is a transcription: every rule of
[docs/architecture.md](../docs/architecture.md) ("Lexing", "Parsing and the
syntax tree", "Error tolerance") and every function of the Rust crates has
its counterpart here, under the same name where the language allows. Where
the Rust behaviour is an accident rather than a rule, stage 1 follows the
documents and the difference is listed in
[DISCREPANCIES.md](DISCREPANCIES.md) until the coordinator decides it
(D-079). None is listed now: the first round's five (three of the
parser's, one of name resolution and one of the type checker's) and the
formatter's two (D6 and D7, below, "The formatter") were decided for the
documents, and stage 0 now behaves as stage 1 does (docs/architecture.md,
decisions 165 to 168, 179 and 267). A rule that changes now changes in
stage 1 alone, and [RULES.md](RULES.md) lists it with its cases: R1
(decision 269), what a channel carries is `Send`, as the bound asks it,
so that a generic function makes a channel of its type parameter and a
program a channel of closures; and R2 to R9 (decisions 271 to 278),
unsafe code as [22](../../proposal/22-unsafe.md) defines it (below,
"Unsafe code"): the three unsafe operations and the markers that oblige,
the safety comments and the lints, raw pointers, `std.mem` and `std.ffi`,
`pure` and compile time, the level a package declares, `sisu1 audit
--unsafe`, `std.rt`, and the runtime's first slice in Sisu; R10 to R16
(decisions 291 to 297), what the manual's writers found missing in the
front end (below, "The manual's findings"): a non-exhaustive `match`
naming every pattern it misses, `sisu lint` with D-043's lint of `case _`
over an enum of the program's own package, naming enforced, a trait that
yields to a variant of its name in a value's place, an optional chain that
reads in an interpolation, an error set that hides the prelude's warning,
and the fixes and messages the writers found missing; R17 and R18
(decisions 298 and 299), the type arguments of an intrinsic that no
parameter tells and those an associated function takes, `Layout.of[T]()`,
with a body of the standard library that does not type-check an internal
error; R19 (decision 300), what the front end checks of `@export` and
`@blocking`; R20 (decision 313), a float's `Debug` with a format spec;
and R21 to R26 (decisions 320 to 325), the manual's second round (below,
"The second round: soundness first"): a collection holds values or views,
never references; what the checker let through and what lowering wrote
that a program did not say; the fixes, notes and forms the later chapters
found missing; and a generic struct's construction; and R27 to R36
(decisions 340 to 349), the manual's third round (below, "The third
round: what ships"): typed holes, which a release build rejects, a run
shows the closure of and a blocked suite's status says; `Atomic` and a
`par` loop's body in `pure` code; a trait's contract, which binds its
impls; `@deprecated` and attributes nothing reads; `embed` from the
package's root; `json.decode`'s validation; exact numbers as written and
the floating-point attributes; `comptime for`'s fix; `max` and `min` of
a collection; and `@derive(Encode, Decode)`; and R37 to R46 (decisions
361 to 370), the fourth round (below, "The fourth round: the third
round's rest"): trait associated constants; `expect`, which lets a test
go on; a trait's contract on an impl of another module; deprecated
types, constants and variants, and attributes that stand where they mean
nothing; `@derive(Arbitrary)`; generator expressions; the forms,
`@cfg` on members, fixes and notes of the manual's findings; and no SIR
with its tests of a program with an error; and R47 to R55 (decisions 400
to 408), the fifth round (below, "The fifth round: the merge's rest, the
driver's one run, the open items"): `sir --tests`'s warnings and
options, a bound from the end on a `Slice`, a module's type with its
arguments and `E0413`'s order, a generator expression's first iterable, a
type's constant as a constant argument, deprecated traits and fields, a
generic `@derive(Arbitrary)` and an impl in another module, an attribute
before a statement, and the manual's syntax and type-checker findings.

## Stage 1 is the front end

The Rust front end (`sisu-lexer`, `sisu-parser`, `sisu-resolve`,
`sisu-typeck`, `sisu-lower`, `sisu-borrowck` and the front-end commands
of `sisu-driver`) is frozen and will be retired; a rule of the language is
implemented in stage 1 only, which no longer has to match stage 0's front
end where the rule changed. Stage 0's interpreter and native backend stay
as the oracles of what a program does: they run SIR (`sisu run
--from-sir`, `sisu build --native --from-sir`, decision 212), whichever
front end wrote it. So `check.sh` checks stage 1 two ways.

- **What the programs do**, the class `behaviour`, the primary oracle
  from now on: every runnable program of the corpus and of stage 1's own
  cases (one with an `.out`, `.err` or `.status`: the 199 of the
  conformance suite, which are its native list, and seven of stage 1's:
  two of `cases/channels/`, three of `cases/unsafe/`,
  `cases/packages/unsafe_trusted` and `cases/runtime/structural`) is put
  in a sandbox as the conformance runner puts a
  case (the file and what its `.files/` holds, or a package case's
  directory, run in the one its `.cwd` names), lowered there by stage 1
  (one run of `sir-batch` over the sandboxes; the sandbox's paths it
  writes into the SIR, the trace texts among them, become the bare names
  `sisu sir NAME.sisu` writes there), and run from that SIR with the
  case's `.args` and `.stdin` and an empty environment, by stage 0's
  interpreter and by its native backend: each run must print the case's
  `.out` and `.err` (after the front end's warnings, which a run from SIR
  does not print) and exit with its `.status`, and natively leave no
  allocation live at exit. It is the conformance runner's
  `--interpreted-from-sir` and `--native-from-sir` with stage 1's SIR in
  place of `sisu sir`'s, and it is what shows a rule's behaviour where
  stage 0's front end refuses the program. It runs with or without
  `--native` (stage 0's backend builds the programs either way), in about
  half a minute natively (stage 1's batch about 8 s, then each program
  run, built and run again natively, four at a time, about 20 s).
- **Against the frozen Rust front end**, byte for byte, as before: the
  classes `tokens` to `ast`, `names`, `types`, `sir`, `ownership`, the
  `contracts-` classes, `human`, the `cases-` and `corpus-` classes, the
  formatter's (`fmt`, `fmt-check` and the others) and the
  documentation's (`doc`, `doc-output`), and `self` and `fixed`. These
  are expected to differ where a rule of RULES.md changes what stage 1
  reports, prints or writes, exactly for the programs the rule touches.
  RULES.md is DISCREPANCIES.md's successor for differences that are
  decisions rather than accidents: each rule has its reasons from the
  documents, what stage 1 does, its cases and the decision text for
  docs/architecture.md, and under it a line ``- Differs: `path`:
  outputs`` for each case it changes, which `check.sh` accepts as it accepts
  DISCREPANCIES.md's lines (the outputs `ids` and `human` are `sisu
  check --format=ids` and `sisu check`, `doc` and `doc-output` the
  documentation's). Any other difference fails the check. Where no rule
  changed, stage 0's SIR stays an oracle too, through `sir` and
  `corpus-sir`.

A new rule is written in stage 1, with a case of each form it accepts
(with its `.out`, so that `behaviour` runs it) and of each it refuses, its
entry in RULES.md with the `- Differs:` lines its cases need, and the
decision text the coordinator copies into docs/architecture.md.

## Running it

From this directory, with stage 0 built (`cargo build --release` in
`compiler/`):

```text
sisu test                                   # 144 tests, 76 of them stage 0's own, transcribed
sisu run -- tokens [--trivia] FILE          # what `sisu tokens [--trivia] FILE` prints
sisu run -- parse --tree [--trivia] FILE    # what `sisu parse --tree [--trivia] FILE` prints
sisu run -- parse --print FILE              # the file, reprinted from the tree
sisu run -- parse --ast FILE                # what `sisu parse --ast FILE` prints: the AST, as Rust's `{:#?}`
sisu run -- names FILE                      # the name diagnostics of `sisu check --format=ids FILE`
sisu run -- types FILE                      # the `type.` diagnostics of the same command
sisu run -- batch OUT LIST                  # tokens, trees and ASTs for every file LIST names, into OUT
sisu run -- ast-batch OUT LIST              # the AST of every file LIST names, into OUT
sisu run -- names-batch OUT LIST            # the name diagnostics of every file LIST names, into OUT
sisu run -- types-batch OUT LIST            # the type diagnostics of every file LIST names, into OUT
sisu run -- sir [--partial] [--contracts=LEVEL] [--tests [--format=ids] [--modules]] [--locked] [--features NAMES] [--no-default-features] [PATH]  # what `sisu sir [--contracts=LEVEL] [--tests] [PATH]` prints: the whole program's SIR (with `--tests`, its warnings on standard error, R47)
sisu run -- ownership FILE                  # the ownership checker's diagnostics of `sisu check --format=ids FILE`
sisu run -- sir-batch OUT LIST [--contracts=LEVEL | --tests]  # both, for every file LIST names, into OUT (with a level or `--tests`, the SIR alone)
sisu run -- check [--format=ids] [--locked] [--features NAMES] [--no-default-features] [PATH]  # what `sisu check` prints: every diagnostic
sisu run -- check-batch OUT LIST            # both forms, for every file LIST names, into OUT
sisu run -- build [--native [--release] [--contracts=LEVEL] [--output OUT]] [--locked] [--features NAMES] [--no-default-features] [PATH]  # what `sisu build` makes
sisu run -- fmt [--check] [PATH...]         # what `sisu fmt [--check] [PATH...]` does (below, "The formatter")
sisu run -- fmt-batch OUT LIST              # what `sisu fmt --check` prints of every file LIST names, and the text `sisu fmt` leaves, into OUT
sisu run -- fmt-profile LIST                # where the formatter's time goes over the files LIST names
sisu run -- doc [PATH] [--output DIR]       # what `sisu doc [PATH] [--output DIR]` does (below, "Documentation")
sisu run -- doc --std [MODULE...]           # the standard library's pages, as `sisu doc --std` writes them
sisu run -- doc-batch OUT LIST              # what `doc` prints of every program LIST names, its status, and the pages `--output` writes, into OUT
sisu run -- mutate SEED COUNT OUT LIST      # mutants of the files LIST names
sisu run -- deform OUT LIST                 # the files LIST names laid out otherwise, for the formatters
sisu run -- profile FILE                    # the time lexing, parsing and each dump of FILE take
./check.sh [--native] [--self] [--mutants N] [--seed S] [--names] [--types] [--sir] [--check] [--keep DIR]
./bootstrap.sh [--keep DIR]                 # the fixed point alone (below, "The bootstrap")
sisu run -- audit --unsafe [--std] [PATH]   # the program's unsafe code by package and module, with the lints (below, "Unsafe code")
sisu run -- lint [--format=ids] [PATH]      # `sisu lint`: the program's lints, which `check` does not print (below, "The manual's findings")
sisu run -- test [OPTIONS] [PATH]           # the tests, built by stage 1 and run by stage 0's runner, with R27's status (below, "The third round")
sisu run -- query KIND FILE [LINE COL [NEW]] [--text PATH]  # the language server's questions of a file and a position, as tab-separated lines (RULES.md, R164, R169)
sisu run -- check --holes [--format=json] [PATH]  # each typed hole with its type, what fits it, its types, its contract and its tests (R174, R181)
sisu run -- fix FILE [--dry-run] [--all]    # each diagnostic's preferred fix applied to the file, checked again until none applies (R176), status 1 when errors are left (R199)
sisu run -- eval FILE [--to json|toml|yaml] # a configuration module's constants, evaluated at compile time and written as JSON, TOML or YAML (R165)
sisu run -- check --diary PATH [PATH]       # `check`, its records also appended to the diary PATH (R186)
sisu run -- explain-borrow FILE:LINE        # every borrow live on the line, in the driver's words (R187)
sisu run -- query explain FILE LINE [--ranges]  # its loans as an answer, with `--ranges` each loan's start and last use as ranges (R187, R211)
sisu run -- audit (--ownership FILE... | --caps [FILE...])  # the escape hatches of each file, a program's capabilities (R188)
sisu run -- sir --defer-borrow [--tests] [PATH]  # the SIR of a program whose only errors are the ownership rules' (R189)
sisu run -- build --emit-sir [--release] [--contracts=LEVEL] [--output OUT] [PATH]  # each program's SIR the native build hands its backend, after `# executable PATH` (R195)
sisu run -- query values FILE               # a worksheet's values and output by line, from stage 1's evaluator (R196)
sisu run -- grammar                         # the parser's grammar, from its `##!` comments (R202)
sisu run -- doc [--std] --digest [PATH] [--output DIR]  # the API as one plain text, each item's declaration and its doc's first paragraph (R207)
sisu run -- sir --tests (--mutants | --mutant=K) [PATH]  # each mutant of `sisu test --mutate` after a line `# mutant K: FILE:LINE:COL: WHAT` (R208)
sisu run -- sir --tests --cover [PATH]      # the SIR, then a line `# region FILE:SPAN KIND FUNCTION bbB:S` for each statement and branch, for `sisu test --cover` (R216)
sisu run -- sir --tests --module-fixtures [PATH]  # each test's fixtures on its line, a module's fixture the test's `ref T` parameter (R219)
sisu run -- sir --tests --std               # outside a package: the standard library's doc examples alone, as tests of an empty `examples.sisu` (R221)
sisu run -- sir --tests --transcripts [PATH]  # the world of transcripts kept whatever is beside the program, for the driver (R223)
./sisu1 (check | sir | build | run | fmt | doc | audit | lint | test) [PATH] ...  # stage 1 as the compiler's front end, from anywhere
```

A `FILE` of `names`, `types`, `sir` and `ownership` (and a line of a
batch's `LIST`) may also be the directory of a package, with its
`sisu.toml`, or of a workspace: stage 1 reads the package as `sisu check`
does when run in that directory (below, "Programs of several modules"),
and for a workspace prints each member's diagnostics and its one
application's SIR. `sir` without a path reads the package of the working
directory or of the nearest directory above it, as `sisu sir` does, so
`sisu run -- sir` here prints stage 1's own SIR. `sir` lowers the program
as `sisu sir` builds it, without its tests (decision 243: its `test`,
`suite` and `bench` items and its doc examples left out, and `@cfg(test)`
false), with every contract checked or those `--contracts=LEVEL` names
(07), while the ownership checker's diagnostics are those of the program
built with its tests, as `sisu check` builds it. `sir --tests` lowers it
with them, as `sisu sir --tests` does (decision 285), but for a program
with an error of any phase, of which it prints no SIR and, on standard
error, why `sisu test` does not run it (RULES.md, R46), and for one without
an error its warnings on standard error, as `check` prints them, from one
check of the program (R47; `--format=ids`, `--features`,
`--no-default-features` and `--locked` as `check` takes them): a line `test "NAME"
test#K` for each test, its `@tag`s after `tags`, then a line `bench "NAME"
bench#K` for each benchmark, whose functions lowering keeps as roots after
the rest, so that `sisu test --from-sir` and `sisu bench --from-sir` run
what stage 1 lowered (`sir.Program.test_tags` and `benches`,
`Program.test_tags` of the resolved program, `typeck.collect.test_tags`);
the class `sir-tests` of `check.sh` compares it with stage 0's for every
program of `sir.list` that has tests or benchmarks, and the class
`transcripts` that of each program with transcripts, in a sandbox with
its `tests/`, whose transcripts the driver then runs from it (RULES.md,
R132). `sir --partial` prints
the SIR of what lowering reached even for a program beyond its slice,
which it then names (the class `self` compares stage 1's own package that
way, module by module).

Stage 1 prints to standard output and its diagnostics to standard error,
in the conformance form (`--format=ids`), as stage 0 does; `names` and
`types` print their diagnostics to standard output, as `sisu check` does,
and `types` then prints on standard error why parts of the file are beyond
the slice it checks; `sir` and `ownership` print on standard error why
the file is beyond the slice of lowering, when it is. It reads the
message catalog and the standard library's sources at run time, from the
compiler's directory, `--home DIR`, by default `..`
(`DIR/catalog/messages.catalog`, which `--catalog PATH` replaces, and
`DIR/std/`), so it is run from this directory unless `--home` says where
that is; the tests read the catalog and the proposal's examples the same
way. `check`, `build` and `sisu1` are the driver (below, "Stage 1 as the
compiler's front end").

`check.sh` builds stage 0 in release mode, makes stage 0's outputs for
every file (four at a time), runs stage 1 once per class of files
(`batch`, one run for many files), and compares five outputs per file: `tokens` (`sisu
tokens --trivia`, standard output and error), `tree` (`sisu parse --tree`,
both), `trivia-tree` (`sisu parse --tree --trivia`), `print` (stage 1's
reprint against the file itself) and `ast` (`sisu parse --ast`, the AST,
below), where a file whose tree differs as listed may differ in its AST
too. Then it compares `names`: the name diagnostics of `sisu check
--format=ids` (the lines with `E0200`, `E0201`, `E0202`, `E0220`, `E0234`,
`E0389` or `W0002`, with their labels and fixes) against stage 1's
(`names-batch`), for the conformance suite's `names/` cases, stage 1's own
cases (but those of `cases/checker_names/`, whose names stage 0's type
checker reports rather than its resolver, which the classes of `check`
compare) and the programs of several modules (the conformance suite's
packages, each a directory `NAME.pkg`, or the one its `NAME.cwd` names
inside it, which stage 0 checks from inside it, its scripts, and stage 1's
own `cases/packages/`), and with `--names` for every program of the corpus
that is one module (the conformance cases outside `packages/` and
`names/`, and the proposal's examples). Then `types`: the type checker's
diagnostics of `sisu check --format=ids` (those whose identifier begins
with `type.`, with their labels and fixes) against stage 1's
(`types-batch`), for the programs `types.list` names (a file, or the
directory of a package or workspace), each of which must be within the
slice and match, and with `--types` for every program of the corpus (the
conformance cases outside `packages/`, the proposal's examples and the
programs of several modules), where a file within the slice must match and
any other may only report fewer of stage 0's diagnostics. Then `sir` and
`ownership`: what `sisu sir` prints, and the ownership checker's
diagnostics of `sisu check --format=ids` (`E0333`, `E0366`, `E0367`,
`E0392`, `E0393`, `E0397`, `E0400` to `E0410`, `E0416` to `E0420`, `E0428`
and `E0429`, with their labels and fixes), against stage 1's (one run of
`sir-batch` for both), for the programs `sir.list` names, each of which
must be within the slice of lowering and match, and with `--sir` for every
program of the corpus, where a file within the slice must match and any
other is beyond it. Then `contracts-off` and `contracts-preconditions`:
what `sisu sir --contracts=LEVEL` prints (07; the levels a release build
may check, decision 232) against `sir-batch OUT LIST --contracts=LEVEL`,
for the programs of `sir.list` with a contract, each of which must match.
Then the formatter's: `fmt`, the text `sisu fmt` leaves of each `.sisu`
file of the corpus, the standard library, the examples, stage 1's own
sources and its cases (stage 0 formats a copy), and `fmt-check`, what
`sisu fmt --check` prints of it and its exit status, against stage 1's
`fmt-batch`; `fmt-again`, stage 1's text formatted again, which must be
itself; with `--native`, `fmt-joined` and `fmt-spread`, all three for
the same files laid out otherwise; and with `--mutants N`, `fmt-mutants`,
all three for the mutants (below, "The formatter"). Then `doc`: what
`sisu doc` prints of every program of the corpus and of stage 1's own
cases (a file run in its directory, a package inside it) and, with
`--self`, of stage 1's own package, with its exit status, then of the
standard library (`--std`, all its modules and some) and of arguments
stage 0 refuses (`D0113`, an unknown option), against stage 1's
`doc-batch`; and `doc-output`, the pages `sisu doc --output DIR` writes
into DIR, file for file, with what it prints and its status (below,
"Documentation"). And `behaviour`: every runnable program of the corpus
and of stage 1's cases lowered by stage 1 and run from its SIR by the
interpreter and natively, against the case's `.out`, `.err` and `.status`
(above, "Stage 1 is the front end").
With `--self` (which `--native` implies) it checks
stage 1 over its own package, the class `self`: its name and type
diagnostics against those of `sisu check --format=ids` run in this
directory, and its SIR (`sir --partial`) against what `sisu sir` prints of
the package, split by the source file each function comes from (with
what is not a function as `(declarations)`), so that a difference names
the module it is in. It exits non-zero on any difference that neither
RULES.md nor DISCREPANCIES.md lists ("- Differs: `path`: outputs"), and on
any program of `behaviour` that does not do what its case says. With
`--mutants N` it also compares N mutants of the corpus (deleted, inserted
and swapped characters), where a difference must carry the mark of a
listed discrepancy. Stage 0 writes a package's `sisu.lock` when it checks
it; `check.sh` removes the ones its run wrote.

With `--native` it runs the stage 1 the driver builds for every `sisu`
command (the release profile with every contract checked, decision 860;
`sisu1` asks the driver for it, RULES.md, R150), copied into the work
directory, for every batch instead of `sisu run --` on the interpreter,
and builds stage1-A, the debug build the fixed point is about (`sisu
build --native --output WORK/stage1-A`), for the classes `self` and
`fixed`; nothing else changes, but for the class `build`,
which needs the executable: stage 0's `sisu build` and stage 1's `build`,
one after the other, of copies of a file, of a package whose manifest
names its `[profile.release] contracts`, of a workspace and of a package
with features, in the work directory, in the debug and the release
profile, with `--contracts`, with `--locked`, and with `--features` and
`--no-default-features` (one naming no feature), each compared by the
executable it makes, the lockfile it writes and what it prints. The two paths give the same
outputs: a run of each over every class but `self`, kept (`--keep`) and
compared file by file, differs in nothing but the timings of the summary
(below; this round's two runs, the formatter's classes among them, differ
in no file either; `corpus-check` was compared natively only), and stage
1 over its own package on the interpreter (`names`, `types` and `sir
--partial`, 83 s, 460 s and 1123 s, at a peak of 2.5 GB of memory) prints
what the native executable prints. The native run, with the classes
`self`, `fixed`, `build`, those of `check`, the formatter's and the
documentation's, `behaviour`, and now `sir-tests`, takes eight and a
half to twelve minutes on a machine of its own (this round's last, below,
took fourteen and a half on four cores that other work kept busy, a load
of 5 to 13); the
interpreter's without them (but `human`, `cases-check` and `fmt`) about
thirteen on a quiet machine, and the formatter's classes add about five
(the documentation's were not timed there).

## Results

The twenty-fifth round's run (`SISU_FRONT_END=rust SISU_LOWER=rust`, at
a load of about 4 on four cores, 13 minutes) ended `total: 16111 files
checked, 14214 identical, 1897 differing (1897 listed in RULES.md or
DISCREPANCIES.md, 0 not)`, with `served` 12 request files and `fix` 4
files as expected, `expected` 198 files, `behaviour` 378 programs (377 as
expected, 358 natively too, 1 refused as listed) and `fixed-sir`
identical. The twenty-fourth round's ended `total: 15947 files checked,
14079 identical, 1868 differing (1868 listed ..., 0 not)`. The run below is an earlier
round's, kept for its breakdown of the classes.

`./check.sh --native --names --types --sir` (`--native` implies `--self` and
`--check`), run this round, as the rounds before it, with
`SISU_FRONT_END=rust`, so that `sisu`, whose front end is stage 1 since
decision 268, gives the Rust front end's outputs to compare with, and
`SISU_LOWER=rust`, the Rust lowering of the native backend, where `sisu
build --native` lowers through stage 2 by default (decision 304):

```text
stage 1 against stage 0 (tokens, tree, trivia-tree, print, ast):
  conformance   576 files,  550 identical,  26 differing as listed,   0 differing otherwise; stage 1    2.9s (411770 bytes; lex 160 ms, parse 166 ms, dumps 1121 ms, ast 567 ms)
  std            32 files,   29 identical,   3 differing as listed,   0 differing otherwise; stage 1    3.2s (477525 bytes; lex 492 ms, parse 101 ms, dumps 752 ms, ast 490 ms)
  examples       11 files,   10 identical,   1 differing as listed,   0 differing otherwise; stage 1    0.5s (27495 bytes; lex 33 ms, parse 11 ms, dumps 69 ms, ast 35 ms)
  cases         179 files,  140 identical,  39 differing as listed,   0 differing otherwise; stage 1    0.7s (126363 bytes; lex 38 ms, parse 35 ms, dumps 262 ms, ast 243 ms)
stage 1's name resolution against stage 0's (names):
  names         182 files,  153 identical,  29 differing as listed,   0 differing otherwise; stage 1    0.5s
  corpus-names  530 files,  504 identical,  26 differing as listed,   0 differing otherwise; stage 1    0.9s
stage 1's type checker against stage 0's (types):
  types         547 files,  539 identical,   0 beyond the slice and a subset,   8 differing as listed,   0 otherwise; stage 1    6.1s
  corpus-types  572 files,  559 identical,   0 beyond the slice and a subset,  13 differing as listed,   0 otherwise; stage 1    6.1s
stage 1's lowering against stage 0's (sir):
  sir           547 files,  506 identical,   0 beyond the slice,  41 differing as listed,   0 otherwise; stage 1   40.0s (front end 11s, lowering 11s, ownership 14s)
  corpus-sir    572 files,  499 identical,   0 beyond the slice,  73 differing as listed,   0 otherwise; stage 1   19.7s (front end 6s, lowering 5s, ownership 6s)
stage 1's ownership checker against stage 0's (ownership):
  ownership     547 files,  546 identical,   0 beyond the slice,   1 differing as listed,   0 otherwise (the same run)
  corpus-own    572 files,  571 identical,   0 beyond the slice,   1 differing as listed,   0 otherwise (the same run)
stage 1's lowering at the contract levels of a build against stage 0's (contracts):
  contracts-off              16 files,    6 identical,  10 differing as listed,   0 otherwise; stage 1    2.2s
  contracts-preconditions    16 files,    6 identical,  10 differing as listed,   0 otherwise; stage 1    2.3s
stage 1's lowering of the programs built with their tests against stage 0's (sir-tests):
  sir-tests                  54 files,   36 identical,  18 differing as listed,   0 otherwise; stage 1    5.0s
stage 1's SIR run against the cases' expectations (behaviour: interpreted and native, from `sisu sir` as stage 1 prints it):
  behaviour     280 programs,  279 as expected interpreted (261 of them natively too),   1 refused or changed as listed,   0 otherwise; stage 1   15.9s, the runs   38.8s
stage 1's check against stage 0's (every diagnostic, in the human and the conformance form):
  human          93 files,   80 identical,   0 beyond the slice,  13 differing as listed,   0 otherwise; stage 1    1.4s
  corpus-check  572 files,  526 identical,   0 beyond the slice,  46 differing as listed,   0 otherwise; stage 1   16.3s
  corpus-human  572 files,  507 identical,   0 beyond the slice,  65 differing as listed,   0 otherwise (the same run)
  cases-check   144 files,   61 identical,   0 beyond the slice,  83 differing as listed,   0 otherwise; stage 1    4.9s
  cases-human   144 files,   59 identical,   0 beyond the slice,  85 differing as listed,   0 otherwise (the same run)
  expected       79 files,   79 as their .diag, .audit, .lint, .sir, .sir-tests, .release or .tests expects,   0 otherwise
stage 1's formatter against stage 0's (fmt: the text `sisu fmt` leaves, what `sisu fmt --check` prints, and its fixed point):
  fmt           885 files,  885 identical,   0 differing as listed,   0 otherwise; stage 1    5.5s
  fmt-check     885 files,  854 identical,  31 differing as listed,   0 otherwise (the same run)
  fmt-again     885 files,  885 unchanged,   0 otherwise (the same run)
  fmt-joined    885 files,  854 identical,  31 differing as listed,   0 otherwise; stage 1   14.0s
  fmt-spread    885 files,  854 identical,  31 differing as listed,   0 otherwise; stage 1   15.5s
stage 1's documentation against stage 0's (doc: what `sisu doc` prints and its status, and the pages `--output` writes):
  doc           722 programs,  597 identical, 125 differing as listed,   0 otherwise; stage 1   33.3s
  doc-output    722 programs,  597 identical, 125 differing as listed,   0 otherwise (the same run)
stage 1's build against stage 0's (build: the executables, the lockfile and what each prints):
  build          16 builds,   16 identical,   0 otherwise; both   12.2s
stage 1 over its own package against stage 0 (self: names, types, the SIR by source file):
  self-names   identical
  self-types   identical
  self-sir      120 files,  120 identical,   0 beyond the slice,   0 otherwise; stage 1   54.1s
the fixed point (fixed: stage1-A built by stage 0, stage1-B by the backend from stage1-A's SIR):
  fixed-sir0   identical: stage1-A's SIR of its own package and stage 0's (18630206 bytes)
  fixed-sir    identical: stage1-B's SIR of the package and stage1-A's (18630206 bytes)
  fixed-exe    identical: stage1-B and stage1-A (28022096 bytes)
  stage1-A 51.0s, the backend 3.1s, stage1-B 50.2s
differences:
total: 12847 files checked, 11912 identical, 935 differing (935 listed in RULES.md or DISCREPANCIES.md, 0 not)
```

The 935 differences (each a `listed:` line of the run, left out above) are
those of RULES.md's rules, for the programs each touches as RULES.md
lists them: R1's three cases of `cases/channels/`; R2 to R9's, the
cases of `cases/unsafe/` and `cases/runtime/`, the two package cases with
`unsafe` levels, the std modules only stage 1 reads (`stage1/std/`, whose
syntax stage 0 does not parse, and what `doc --std` writes of them), the
conformance suite's `packages/trust.pkg` and the proposal's
`examples/ffi.sisu`, whose unsafe code stage 1 checks, and the three of
stage 1's older cases that showed `unsafe` as a later phase's; R10 to
R19's, their cases (`cases/findings/`, `cases/lowering/`, `cases/calls/`,
and `exports` and `layout_type_arguments` of `cases/unsafe/`), the five
conformance programs whose diagnostics they change
(`types/list_exhaustiveness`, `generics/function_value_errors`,
`ownership/view_field`, `types/try_context` and `wrong/view_in_struct`)
and the two older cases whose tree R16's statements change; and R20's,
the SIR of the 43 programs that reach a float's `Debug` impl (`sir`,
`corpus-sir`, `sir-tests` and the contract levels); and R21 to R26's,
their cases (`cases/references/`, `cases/soundness/`, `cases/habits/`
and `cases/packages/reexport_import.pkg`) and the conformance programs
whose diagnostics or SIR they change, as RULES.md lists them (the dozen
of `E0400`'s new note among them), with `unsafe/layouts` and
`unsafe/raw_buffer`, which joined the conformance suite, under R4; and
R27 to R36's, their cases (`cases/holes/`, `cases/purity/`,
`cases/contracts/`, `cases/attributes/`, `cases/json/`, `cases/numbers/`,
`cases/comptime/plain_for` and `cases/packages/embed_root.pkg`) and the
conformance programs whose SIR they change (`types/typed_hole` and
`run/testing`, which print a hole's closure, `std/num_exact`'s `Decimal`
literal, and `reflect/json`'s decoder), with `types/typed_hole`'s run held
to `expected/` ("changed as listed"); and R37 to R46's, their cases
(`cases/traits/`, the new ones of `cases/testing/` and
`cases/attributes/`, `cases/generators/`, `cases/findings4/`, and the
package cases `assoc_consts.pkg` and `trait_contract_elsewhere.pkg`), the
conformance programs and stage 1's older cases whose diagnostics R45's
fixes and notes change (`syntax/unicode_escape_sign`,
`types/argument_count`, `types/keyword_only_count`,
`wrong/own_closure_captures_text`, `ownership/own_closure_errors` and
`ownership/shared_immutable`; `lexical_mistakes`, `unicode_escape_sign`,
`type_calls` and `type_arguments_in_values`), and the SIR with its tests
of `testing/parametrised_once_ownership`, which R46 refuses; and R47 to
R55's, their cases (`cases/findings5/`, and the new ones of
`cases/attributes/`, `cases/generators/`, `cases/testing/`,
`cases/traits/` and `cases/packages/arbitrary_elsewhere.pkg`), the
conformance suite's `testing/unsafe_level.pkg`, `run/debug_forms` and
`std/decimal_literals`, which the merge of the fourth round with stage
0's third brought under R6, R20 and R33, and the fixes of
`ownership/from_required` and `wrong/missing_from` (R48), whose `.diag`
files are blessed again. Every
runnable program, those stage 0 refuses among them, does what its case
says, from stage 1's SIR, interpreted, and natively when it is stage 1's
or the conformance suite's native list names it (`behaviour`; the cases
of the checked memory model that joined the suite expect what only the
interpreter reports); `run/iterators`, which R21 refused, writes the
owned spelling RULES.md proposed since the soundness round's expectations
and runs as its case says, so `types/typed_hole` is the one "changed as
listed".

On the interpreter, `./check.sh --names --types --sir` (without
`--self`) gave the same counts as the native run, the classes `build`,
`fmt-joined` and `fmt-spread` aside, in about forty minutes on a machine
other work shared (about thirteen on a quiet one, before the formatter's
classes); this is its run of round 9, over the corpus before round 10's
74 programs:

```text
stage 1 against stage 0 (tokens, tree, trivia-tree, print, ast):
  conformance   458 files,  458 identical,   0 differing as listed,   0 differing otherwise; stage 1   44.3s (342061 bytes; lex 4649 ms, parse 7198 ms, dumps 10179 ms, ast 15260 ms)
  std            28 files,   28 identical,   0 differing as listed,   0 differing otherwise; stage 1   43.8s (412948 bytes; lex 4258 ms, parse 6467 ms, dumps 10046 ms, ast 14440 ms)
  examples       11 files,   11 identical,   0 differing as listed,   0 differing otherwise; stage 1    8.6s (27495 bytes; lex 270 ms, parse 458 ms, dumps 662 ms, ast 910 ms)
  cases          55 files,   55 identical,   0 differing as listed,   0 differing otherwise; stage 1   12.0s (26940 bytes; lex 364 ms, parse 552 ms, dumps 810 ms, ast 1063 ms)
stage 1's name resolution against stage 0's (names):
  names          73 files,   73 identical,   0 differing as listed,   0 differing otherwise; stage 1   15.4s
  corpus-names  418 files,  418 identical,   0 differing as listed,   0 differing otherwise; stage 1   45.2s
stage 1's type checker against stage 0's (types):
  types         463 files,  463 identical,   0 beyond the slice and a subset,   0 differing as listed,   0 otherwise; stage 1  215.3s
  corpus-types  450 files,  450 identical,   0 beyond the slice and a subset,   0 differing as listed,   0 otherwise; stage 1  201.9s
stage 1's lowering against stage 0's (sir):
  sir           463 files,  463 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise; stage 1  541.2s (front end 178s, lowering 103s, ownership 164s)
  corpus-sir    450 files,  450 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise; stage 1  510.7s (front end 167s, lowering 99s, ownership 156s)
stage 1's ownership checker against stage 0's (ownership):
  ownership     463 files,  463 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise (the same run)
  corpus-own    450 files,  450 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise (the same run)
stage 1's lowering at the contract levels of a build against stage 0's (contracts):
  contracts-off              13 files,   13 identical,   0 otherwise; stage 1   82.2s
  contracts-preconditions    13 files,   13 identical,   0 otherwise; stage 1   85.2s
stage 1's check against stage 0's (every diagnostic, in the human and the conformance form):
  human          41 files,   41 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise; stage 1   69.3s
  cases-check    45 files,   45 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise; stage 1   98.7s
  cases-human    45 files,   45 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise (the same run)
stage 1's formatter against stage 0's (fmt: the text `sisu fmt` leaves, what `sisu fmt --check` prints, and its fixed point):
  fmt           633 files,  631 identical,   2 differing as listed,   0 otherwise; stage 1  284.2s
  fmt-check     633 files,  631 identical,   2 differing as listed,   0 otherwise (the same run)
  fmt-again     633 files,  633 unchanged,   0 otherwise (the same run)
total: 5838 files checked, 5834 identical, 4 differing (4 listed in DISCREPANCIES.md, 0 not)
```

Every file of the conformance suite, `compiler/std/` and
`proposal/examples/` gives identical token streams with trivia, trees with
and without trivia, diagnostics, reprints and ASTs. `cases/` holds stage
1's own inputs for the error-tolerance rules the corpus never triggers
(operator mixing and the parentheses its fixes add, Python's conditional
expression, statement, expression, declaration, lexical, indentation,
bracket and structure mistakes, the keyword that opens a missing block, the
token a line misses at its end, the forms of later phases, scripts, CR LF,
Unicode beyond ASCII, type arguments where a value is written), for the
type checker's rules (calls, conversions, control flow), the names in
constant arguments, the five discrepancies of the first round, which
stage 0 now shares, programs of several packages (`cases/packages/`), and
the diagnostics no program of the corpus has that stage 1 rendered last
(below, "Stage 1 as the compiler's front end"): a label in another
module than its error, an error in a file the compiler wrote, and the
names stage 0's type checker reports (`cases/checker_names/`); for the
formatter's rules (`cases/fmt/`, below, "The formatter"), among them the
two that showed D6 and D7, which stage 0 now shares; for `sisu doc`'s
layout (`cases/doc/` and `cases/packages/doc_pages.pkg`, below,
"Documentation"); and for the rules stage 1 decides (`cases/channels/`,
RULES.md's R1: a generic pipeline and a channel of closures, with the
`.out` the class `behaviour` holds their runs to, and the forms the rule
refuses). With `--mutants 2000 --seed 7` all 2000 mutants give
identical outputs, their ASTs and their formatting included (in the first
round 21 differed, by D1).

The corpus is 527 programs: the 501 of one module (the conformance cases
outside `packages/` and its package cases, and the proposal's examples),
the conformance suite's 20 programs of several modules or with a manifest
(its packages, `doc/package.pkg` among them, workspaces and scripts) and
stage 1's own six package cases. Every one gets stage 0's name
diagnostics, its type diagnostics (the ownership checker's `type.`
messages among them), its SIR, its ownership diagnostics, the whole of
what `sisu check` prints, in both forms, and the pages `sisu doc` writes,
exactly; `types.list` and `sir.list` name the 525 and stage 1's 22 cases
of one module (three of type rules, ten of diagnostics no program of the
corpus has, `doc/layout.sisu`, the three of `cases/channels/`, which
show R1 of RULES.md, `comptime/evaluator.sisu` and `comptime/sieve.sisu`,
whose compile-time code the evaluator now runs to stage 0's SIR, and the
three of `cases/testing/` whose build with their tests R46 refuses). Of the 542 of `sir.list` before it, 346 are lowered by stage
1, 85 of them with diagnostics of the ownership checker, and 196 have a
front-end error, for which both print nothing; stage 0 refuses two of
the 346, those R1 lets in, whose differences RULES.md lists. And stage 1 over
its own package, 68 000 lines in 84 modules, gets stage 0's name and type
diagnostics (none), its pages (84) and its SIR, function for function,
for all 116 of its source files: its own 84, the 28 the compiler writes
for their derived impls, the 3 of the standard library it reaches (the
prelude, its derived impls and the builtin impls), and the declarations.
Built without its tests, as `sisu sir` builds it since decision 243, the
package reaches fewer derived impls and std modules than the 135 files
its SIR had before.

## The bootstrap: the fixed point

D-078's fixed point holds: stage 1 compiled with itself as its front end is
the executable stage 0 compiles, byte for byte. `bootstrap.sh` checks it
alone, and `check.sh --self` (which `--native` implies) as its class
`fixed`:

1. stage 0 builds stage 1 natively (`sisu build --native`: the Rust front
   end, then the backend): stage1-A;
2. stage1-A prints the SIR of its own package (`sir`, run in this
   directory, which without a path reads the package of the working
   directory as `sisu sir` does): the whole program in one file, every
   module and the standard library's functions it reaches in stage 0's
   order, the types, impls and intrinsics, exactly what `sisu sir` prints
   of the package;
3. the backend alone builds that SIR (`sisu build --native --from-sir`,
   decision 212): stage1-B, stage 1 compiled with stage 1 as its front end;
4. stage1-B prints the SIR of the package again, which is stage1-A's byte
   for byte;
5. and stage1-B is stage1-A byte for byte: the backend is deterministic,
   and nothing in the executable depends on when it was built (its build
   ID is a hash of its contents), nor on where but through the paths it
   holds: a source file of a crate the runtime links, and the directories
   the debug information names (decision 227), the one the build ran in
   and the standard library's, so both scripts build stage1-B in this
   directory, as stage1-A is built.

```text
  stage1-A   25377848 bytes  sha256 5e4bd9ae6cec930799c72081bfdd8763a72db4758df21375af92ea8eeb5c2890  built by stage 0 (14.0 s)
  A.sir      18015195 bytes  sha256 ed08c5ec16ba7cec598de67b76fbaceff650956f4ff15b952b37cbe9f9626f96  printed by stage1-A (34.1 s)
  stage1-B   25377848 bytes  sha256 5e4bd9ae6cec930799c72081bfdd8763a72db4758df21375af92ea8eeb5c2890  built by the backend from A.sir (7.8 s)
  B.sir      18015195 bytes  sha256 ed08c5ec16ba7cec598de67b76fbaceff650956f4ff15b952b37cbe9f9626f96  printed by stage1-B (38.2 s)
  stage0     18015195 bytes  sha256 ed08c5ec16ba7cec598de67b76fbaceff650956f4ff15b952b37cbe9f9626f96  printed by stage 0 (sisu sir)

the SIR, A.sir and B.sir: identical
the executables, stage1-A and stage1-B: identical
stage 1's SIR and stage 0's, A.sir and sisu sir: identical
the fixed point holds
```

Since the two executables are one file, the compiler compiled by itself
gives the output of the compiler compiled by stage 2 on everything, on
itself and on the corpus, which is how D-078 states the fixed point; and
stage 1's SIR is stage 0's for every program of the corpus (the classes
`sir` and `corpus-sir`, whose package and workspace cases are whole
programs of several modules, printed as `sir PACKAGE` prints one).

What the fixed point covers is the front end: lexing, parsing, names,
types and templates, lowering, the ownership checker with drop placement,
jump threading and compile-time evaluation, which stage 1 runs on itself.
What stage 0 still does that stage 1 does not:

- **the backend and the runtime**, in Rust (`sisu-codegen` over Cranelift,
  and `sisu-rt`), as the plan expects of stage 2;
- **running SIR**: the interpreter (`sisu run`, `sisu test` with the
  ownership monitor, `sisu bench`, the counterexample mode, replay,
  simulation, hot replacement), of which stage 1 has the part compile-time
  code needs; stage 1's own tests run on it (`sisu test`), since the
  backend builds programs, not test runs;
- **the driver's other commands**: the language server, the REPL and
  worksheets, `explain-borrow` (stage 1 has the ownership checker but its
  `explain.rs`), `audit` (`--ownership`, and `--caps`, decision 240),
  `new`, `replay`, and `sisu test`'s cache of the tests that passed
  (decision 242), which belongs to running them (`check --explain` is
  retired, decision 258, and `sisu doc` is stage 1's now);
- **speed**: stage1-A prints the SIR of its 66 000 lines in about 30 s
  (of 61 000 lines in about 28 s a round ago, about 34 s before that
  round's work on the loans dataflow, below, and 55 s before stage 0's
  backend ran the commonest intrinsics inline, decision 253), stage 0 in
  4 to 7 s (below, "Where the time goes").

Since this round stage 1 builds as `sisu build` builds: in the release
profile with `--release`, with the contracts its manifest's
`[profile.release]` names or `--contracts` asks for, every member of a
workspace, the libraries with the line that says so, and the lockfile
written as stage 0 writes it; and it renders every error of stage 0's
front end that the corpus and stage 1's own cases show, those whose
label is in another file than the error among them (below).

## Stage 1 as the compiler's front end

`sisu1` (a script in this directory) runs stage 1 as the front end of a
`sisu`-like command, from any directory:

```text
sisu1 check [--format=ids] [--locked] [FEATURES] [PATH]  every diagnostic, as `sisu check` prints them
sisu1 sir [--contracts=LEVEL] [--tests] [PATH]  the whole program's SIR, as `sisu sir` prints it
sisu1 sir --repl SESSION INPUT --kept OUT [--load]
                                              the SIR of a REPL session's program with one more
                                              input, or that input's diagnostics (R79)
sisu1 sir --worksheet FILE                    the SIR of a worksheet, each statement marked and
                                              shown, or its diagnostics (R79)
sisu1 build [--release] [--contracts=LEVEL] [--locked] [FEATURES] [PATH] [--output OUT]
                                              what `sisu build --native` makes, and where
sisu1 run [FEATURES] [PATH] [-- ARGS...]      the program built into a temporary directory and run
sisu1 fmt [--check] [PATH...]                 each file formatted in place, or what would change, as
                                              `sisu fmt` does it, with its exit status
sisu1 doc [--locked] [FEATURES] [PATH] [--output DIR]
                                              the documentation, a Markdown page per module, printed
                                              or written into DIR, as `sisu doc` makes it
sisu1 doc --std [MODULE...]                   the standard library's pages, or those of the modules named
sisu1 lint [--format=ids] [FEATURES] [PATH]   the program's lints, which `check` does not print (R11)
sisu1 test [OPTIONS] [FEATURES] [PATH]        the tests, checked and lowered by stage 1 and run by stage 0's
                                              runner (`sisu test --from-sir`), exiting 1 when one failed
                                              and 3 when none did and one is blocked by a hole (R27)
```

`FEATURES` is `--features NAMES` (the root package's features to turn on
besides `default`, separated by commas or spaces, `DEP/FEATURE` for a
dependency's, any number of times) and `--no-default-features`, which
leaves the root package's `default` off, as `sisu` takes them (decision
250: `package.FeatureRequest`, `turn_on_features`); a name that is no
feature of the package it names is `D0112` (`cli.unknown-feature`),
reported before anything is written, and so is any name for a single
file.

`PATH` is a file or the directory of a package or workspace, by default
the package of the working directory or of the nearest directory above
it. The first use builds stage 1 with stage 0 (`sisu build --native`) into
`compiler/target/stage1/`, and so does the first after a source of stage 1
changed; stage 1's other commands pass through. `check`, `sir` and
`build` are stage 1's own commands, and each writes the lockfile of the
package or workspace as stage 0 writes it before it works on it (09:
`sisu.lock` beside the root manifest or its workspace's, every package
the build resolved, sorted; rewritten when the manifests imply another,
and with `--locked` that is the error `D0065`, with what would change).
`build` builds each program the path names, a file's, a package's or
each member's of a workspace in turn, as `sisu build` does: it checks the
program as `check` does, but built without its tests (decision 243),
stops where `sisu build` stops (at the front end's errors, with the line
`error[D0006]: the program was not run: errors found (N)`, then at a
`comptime` expression that fails, with `D0006` too, then at the ownership
checker's, with `D0021`), and lowers it with every contract checked, or in
the release profile (`--release`, decision 232) with those of the root
package's `[profile.release] contracts`, preconditions when it names
none, or with those `--contracts=LEVEL` names (07); a library, which has
no `main`, is then built, and says so (`built the package NAME VERSION
(modules: N)`), and a program with a `main` has its SIR written beside the
executable and handed to stage 0's backend (`sisu build --native
--from-sir`, with `--release` in the release profile), which prints
`built PATH`. The options are checked as `sisu build` checks them
(`--output` and `--contracts` go with `--native`, a level is `off`,
`preconditions` or `all`, and `--release` without `--native` is
`D0111`); `sisu1 build` is `build --native`. `run` builds and runs the
executable with the arguments, standard streams and exit status of the
command. The executable goes where stage 0 puts it (decision 231): a
package's to `build/PROFILE/NAME` in the directory of its lockfile, its
workspace's or its own (`package.workspace_of`), `PROFILE` being `debug`
or `release`, a file's beside it, named by its stem, and its directory is
made when it is missing. For a program of one file the executable is the
one stage 0 builds from the source, byte for byte, as the fixed point has
it for stage 1 itself, and so for the packages and the workspace of the
class `build` in both profiles. Stage 0 does the rest: the backend, the
runtime and the link.

`check` prints what `sisu check` prints: the syntax diagnostics of each
module, the manifests' problems, `cap.untrusted`, the problems of the
`@cfg`s (`E0266`, `W0006`), the name pass's and the type checker's
diagnostics, sorted and without repeats, and, when the front
end found no error, the ownership checker's (with the note of `sisu
check`'s driver that names the methods consuming a must-consume value),
in the order of stage 0's files (the modules', the manifests', then those
the compiler wrote). With `--format=ids` it prints the conformance form on
standard output; otherwise the human form on standard error, the
diagnostics with their source lines, labels (a label in another file says
which), notes and fixes, `diag.render_human_files`, transcribed from
`sisu-diag`'s `render_human`. Rendering it brought the parts of a
diagnostic the conformance form never printed under test: the notes of the
type checker (`type.missing-patterns`, the ranges of a literal's type,
`type.truthiness` and a dozen more) and of the name pass
(`resolve.rename-duplicate`), and the parameters of the fixes
(`type.add-hole`'s `name`, `type.add-bound`'s `param` and `bound`), which
stage 1 did not have and now does. A program exits with status 1 only by
returning an error from `main`, which the run time then prints, so a
failing `check` or `build` ends with the line `error: Failed`.

The front end of stage 0 reports errors that are neither the name pass's
nor the type checker's: the resolver's checks of declarations, imports,
impls and signatures (`resolve.`, its `borrow.` rules, `stage0.unsupported`),
those of contracts and refinements (`contract.`, `refine.`), of tasks
(`task.`), of tests and their generators (`test.`), of templates and
compile-time code (`comptime.`), and `cap.`. Stage 1 renders each as the
transcription of stage 0's renders it, with its labels, notes and fixes:
a private item named from another module says whether it is
re-exported, package-private or private; an alias or a trait cycle, an
impl that overlaps another (labelled at the first), a trait that is not
object safe (with the method in the way and why), a template whose derive
or instance fails (labelled at the request), the bound on a template's
instances, a constant that a generic `comptime assert` rejects; and the
failures of compile-time evaluation, which run after the ownership
checker: a panic, with the message the catalog gives stage 0's interpreter
(an index out of bounds, an overflow and the expression that overflowed,
a failed `assert` with the values of its parts drawn under it or the two
sides it compared, a contract's broken clause), a constant a refinement
rejects (`refine.not-satisfied`), a spent budget, a use of the world, and
a value with no literal form. Those whose label or whose place is in
another file than the program's own modules, which stage 1 noted without
rendering until this round, it renders too, each with a case of its own
that the classes `types`, `sir` and `cases-check` compare: a label in
another module than its error, drawn in that module's file (an impl
that overlaps one of another module of the package,
`cases/packages/overlapping_impl.pkg`; a template of another module that
stops making an instance, `cases/packages/template_elsewhere.pkg`; the
prelude's derive of `Default`, which stops for an enum,
`derive_stopped_in_prelude`); the bound on a template's instances, at the
template in the standard library's file (`json.encode`,
`instances_unbounded_in_std`); a syntax error in a doc example, at its
line of the module's file (`doc_example_syntax`), and in code the
compiler wrote, in that code's file (an instance whose header the
template's text breaks, `instance_syntax_error`); and a generator's site
in code the compiler wrote, a doc example's or an instance's, which
stands for the generator's call as a site in the module does (stage 1
wrote that call for the module's own sites only, and noted
`test.no-generator` at the others; `doc_example_generator`,
`generator_in_template`). The names stage 0's type checker reports
rather than its resolver (in code the compiler wrote, and in a body's
annotation, with the whole path and without a fix) are reported by stage
1's checker the same way (`cases/checker_names/`). Stage 1 keeps a note
only for what no program is known to reach (a label, or the bound on
instances, in a module with no file among the program's, and a
generator's site that its walk of a module's expressions misses): a
program with one would be beyond what `check` renders, and `check` would
print what it has, name the rule on standard error (`beyond the slice:
stage 0 reports test.no-generator, which stage 1 notes without rendering
it`) and fail, and `build` would stop.

`check.sh` compares both forms. The class `human`, the 93 wrong programs
of `conformance/cases/wrong/`, each a single mistake written to read well
in the human form, compares what `sisu check FILE` prints on standard
error with what stage 1 prints, and every one must be within the slice and
match; so must the 49 of stage 1's own cases outside `cases/packages/`
(the formatter's ten of `cases/fmt/` and `cases/doc/layout.sisu` among
them), in both forms
(`cases-check` and `cases-human`), but the three of `cases/channels/`,
whose diagnostics are R1's (RULES.md). With `--check` (which
`--native` implies) the classes `corpus-check` and `corpus-human` compare
the whole of `sisu check --format=ids` and the human form for every
program of the corpus: a program within the slice must match, and one
beyond it may only report fewer of stage 0's diagnostics. All 525 are
within it and match. Run alone, one process a program, each of the 450
of round 9's corpus gets the SIR and the ownership diagnostics it gets in a batch, and in a
batch run backwards: what a run keeps from one program to the next
(below, "Where the time goes") does not depend on which program came
first.

## Name resolution

`resolve.sisu` resolves the names of a module, the way `sisu check` does
(docs/architecture.md, "Names and the standard library"; a program of
several modules below): every name the module writes is found among the
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

### Programs of several modules

A package (09, "Modules" and "Package manifest") is read as stage 0's
driver reads it: `toml.sisu` is the TOML subset of stage 0's `toml.rs`
(tables, strings, integers, booleans, arrays, inline tables, and the
positions of what it reports, and `quote`, which the lockfile writes
with), `manifest.sisu` its `manifest.rs` (the `[package]`,
`[dependencies]` with a dependency's `features` and `default-features`,
`[features]`, `[trust]`, `[workspace]`, `[profile.release]` and
`[target.*]` tables, each with its diagnostics), and `package.sisu` its
`package.rs`: a single file, a script with its inline metadata, a package
with its path dependencies, or a workspace's members, each with its
module tree, which mirrors `src/` (the root module `src/main.sisu` or
`src/lib.sisu`, a directory a module of its own beside a file of the
same name), and its package's index, trust, dependencies and features
(the root's `default`, each dependency's unless every entry that reaches
it says `default-features = false`, those the entries list and what each
feature that is on turns on, `turn_on_features`, with `E1018` for a name
that is no feature; decision 243), and the lockfile it writes
(`lockfile_of`). Each module is then filtered as the build compiles it
(`cfg.sisu`, stage 0's `cfg.rs`): an item, an item of a suite or an
`extern` block, or a method whose `@cfg` does not hold is left out before
names are resolved, as are the `test`, `suite` and `bench` items of a
build without its tests, and a `@cfg` that is no condition is `E0266`,
a feature its package does not declare `W0006`. The names pass then
resolves the whole program at once (`resolve.resolve_program`): every
package's modules are declared before an import is resolved, an import
names a module of the importing package (by the package's own name), of
a dependency (by the dependency's name) or of the standard library
(`std.core` the prelude, and `std.string` and `std.fmt` modules that name
its items, decision 245), `pub(package)` reaches across modules of a
package but not beyond, a dependency's tests are not its user's, and a
name a module failed to import or did not parse is not reported again
anywhere. The type checker does the same with the resolved program
(`Resolver.add_program`, stage 0's `resolve_session`): each package is a
package of `Program.packages`, the root package first and the standard
library second, with the orphan rule for impls and the doc examples of
the root package as its tests when the program is built with them. A
diagnostic is rendered with its file's name, relative to the root
package's directory, as stage 0's conformance form writes it for a
program of several files. Stage 0 writes a package's lockfile when it
checks it, and so does stage 1's `check`, `sir` and `build` (not its
batches); `check.sh` removes the lockfiles its run wrote.

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
`sisu-resolve` what the declarations of a program need; and it knows where
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
`@delegate` and the `Debug` every struct and enum has, and the
generators of `arbitrary[T]` and `exhaustive[T]`, with the tests of the
contracts and of `@refines` (below); the predicate of a refinement type
is written as the type is resolved. What stage 0 writes as source text (the interceptors
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
the prelude's or the program's own, and a template, a function or a
method whose body runs code at compile time by unfolding (a `reflect`
query, `x.[f]`, a `comptime for` or `comptime if` among statements or
arms, or a `comptime` expression over a type parameter; decision 244: a
free function, generic or not, a method of a type or an inherent impl;
decision 251: a method of a trait impl, made once as generic as the
impl, a trait's provided method, made in each impl that does not write
it, a test, a `comptime def` and its `T: type` parameters, a constant's
value and a default), run on an evaluator of compile-time code over the
shapes of types (fields, with whether each has a default, variants,
attributes, methods) that writes source text; the text is parsed, declared in a module of its own whose scope is
the template's (`<derive>/main.sisu`, `<NAME[ARGS]>/main.sisu`, a
template without type parameters `<NAME>/main.sisu`) and checked like any
other code, a diagnostic there being at its line and column in that
text. A method's instance is a method of an inherent impl of its type,
whose compile-time code names the type `Self`; the types the code names
that are not type parameters (`reflect.fields_of(Point)`) are known to
the unfolding without being replaced in its text; an instance's module
is never a template's, so code an instance cannot unfold is the
instance's error; and the one instance of a template without type
parameters takes the template's name in SIR, so whatever names the
template runs it. Which instances a program needs is known only once its
calls are typed, so the driver checks the program, declares what its
bodies asked for (and, once, each template without type parameters,
whether anything calls it or not), and checks again until no new request
comes; a generic function or method that calls a template with its own
type parameters becomes a template too, and the program is resolved
again (stage 0's `Db::checked`). Compile-time code where nothing unfolds
it (a refinement's `where` clause, an `invariant`) is `E0900`, which names
the place.
The standard library's own derived impls are unfolded the
same way, into modules of the standard library (`<derive>/prelude.sisu`,
whose file is `<std>/<derive>/prelude.sisu`): the base unfolds those of
the modules it resolves, and, on a copy of itself that loads them all,
those of every module a program may import, whose text a program that
imports one then declares (`Resolver.std_derived`), since the prelude's
templates are the base's and are not copied for each file. Their bodies,
like the standard library's others, are checked when lowering reaches
them. An `@delegate` writes its forwarding impl as text too
(`delegate_text`, into `<delegate>/main.sisu`).

The generators of property tests (17, D-070; stage 0's `arbitrary.rs`,
transcribed in `typeck.arbitrary`) are written as stage 0 writes them,
into `<arbitrary>/main.sisu`: `__arbitrary_T` for each struct, enum and
refinement type of the module whose parts have one (an integer
refinement's predicate bounds what it draws), `__arbitrary_site_N` for
each type written in `arbitrary[T]` or `exhaustive[T](...)`, which the
checker puts at the site as `Gen.of(...)` or `Gen.every(..., max_len=n)`,
the cases and the test `contract of NAME` of each function with a
contract (or a `mut self` method of a type with an `invariant`), and the
cases and the test of each `@refines(reference)`. A diagnostic in that
file repeats one of the program's, so it goes when the program has another
error, as stage 0's driver drops it.

A `comptime assert` on `const` generic parameters (stage 0's
`generic_asserts.rs`, in `typeck.generic_asserts`) is decided at each call
that gives them constants, once the program is typed: its condition is
folded with the call's arguments (integer and boolean arithmetic and
comparisons), and a false one is the call's `comptime.panicked`. Each
body keeps its asserts' conditions as trees of their own, apart from its
typed tree, which the checker keeps only for lowering.

What it reports: every `type.` message of the catalog. Six of them are
stage 0's ownership checker's, reported on SIR after the type checker
(`E0333`, `E0366`, `E0367`, `E0392`, `E0393`, `E0397`): when the front end
finds no error and a body may get one, `types` lowers the program and runs
stage 1's ownership checker (below), and reports its `type.` findings with
the type checker's, in the order `sisu check` sorts them. The front end's
other messages (`resolve.`, `stage0.`, `comptime.`, `contract.`,
`refine.`, `task.`, `cap.`, `borrow.`, `test.`) are not this class's but
`check`'s (above, "Stage 1 as the compiler's front end"), and two rules
here depend on them: a local whose type inference left open is reported
only in a body without an error, and the ownership checker runs only when
the front end found none. The names pass's errors come from stage 1's own
(`resolve`).

What is beyond it:

- in the module, which then reports nothing at all: a contract of a
  struct other than `invariant`, `@structural` and `@keeps`, variadic
  parameters, a `ref` in a declaration's type, and an import of a std
  module stage 1 does not read;
- the `type.` messages of stage 0's ownership checker, for a program that
  lowering leaves part of: a pure body that calls an impure function or
  function value (`E0333`, `E0392`), uses `select` or loops over an
  `Iterator`, or holds a value whose drop runs a `drop` that is not `pure`
  (`E0393`); a `let` without a value (`E0366`, `E0367`); and a call of a
  generic function with a must-consume type argument (`E0397`). For any
  other program, the ownership checker decides them.

No choice of stage 1 differs from stage 0 within the slice but the rules
RULES.md lists (the count `E0305` reports for positional arguments past a
bare `*`, D5, is stage 0's now). One difference of order has no effect on what is reported:
stage 0 declares
every module's items before it resolves any, so its definitions of the root
module come before those the prelude's impls and traits make; stage 1
resolves the prelude once, so they come after.

`check.sh` checks the slice twice: `types.list` names the 542 programs
within it, which must match (all do, but
`cases/channels/channel_refusals.sisu`, whose `E0347`s are R1's): the 525
programs of the corpus (501 of one module, the 20 package and script
cases of the conformance suite and stage 1's own four package cases) and
stage 1's 17 other cases; `--types` checks the 525 of the corpus again. Round 6's 9
programs beyond the slice, each for one of the ownership checker's
messages (a `let` without a value, 5; a pure body that calls an impure
function, 2; one that drops a value with an impure `drop`, 1; a
must-consume type argument, 1), are within it now that `types` runs the
ownership checker.

## Lowering and the ownership checker

`lower.sisu` and `lower/` transcribe stage 0's `sisu-lower` (the functions
of `lib.rs`, `func.rs`, `stmts.rs`, `exprs.rs`, `pats.rs`, `contracts.rs`
and `glue.rs`), `sir.sisu` and `sir/` its SIR (`sisu-sir`'s data,
`types.rs`, `write.rs` and `simplify.rs`), and `borrowck.sisu` and
`borrowck/` the ownership checker (`sisu-borrowck`: `paths.rs`,
`init.rs`, `elaborate.rs`, `loans.rs`, `consume.rs`, `purity.rs`,
`shared.rs`, `names.rs`, `types.rs`, `bitset.rs` and `check_with` of
`lib.rs`), rule for rule, and `interp.sisu` the evaluation of `comptime`
code (`sisu-interp`'s `comptime.rs`, with the part of its interpreter that
compile-time code needs). Together they make what `sisu sir` prints and
the ownership checker's part of `sisu check`:

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
   when lowering first reaches it, from the syntax and text the base keeps,
   and lowered, once per process for a module the base resolved
   (`typeck.StdBodies` and `lower.StdLowered`, below); the drop glue of
   each declared type that needs one is written
   from the types (`lower.glue`). A `comptime` expression is lowered to a
   function of nothing, of kind `comptime N`, with the budget of steps its
   item's `@comptime_budget(steps=N)` gives (a million without one).
3. The ownership checker runs as stage 0's `check_with` does, without its
   memo: per function, the initialisation dataflow and its findings, then
   drop placement; then, for the functions that are not the standard
   library's (`trust_std`, as `sisu check` runs it), the loans and
   must-consume; then purity over the whole program; and a finding that
   repeats one goes. The findings become diagnostics of the program's
   files, sorted as `sisu check` sorts them. Last, the `Shared` escape
   analysis (`borrowck.shared`) tags each site that makes a `Shared`
   task-local or may-cross, which SIR writes as `Shared.new` or
   `Shared.new_atomic`.
4. Jump threading (`sir.simplify`).
5. When the ownership checker finds nothing, the `comptime` functions run
   (`interp.evaluate`, stage 0's `comptime::evaluate`), each after those
   whose values it uses, and each that finishes becomes a `value`
   function whose body builds the value it returned from literals; one
   that panics, spends its budget, uses the world or returns what has no
   literal form keeps its body, and is an error of the program at the
   expression (stage 0 still prints the SIR): `comptime.panicked` with
   the panic's message, rendered from the catalog as stage 0's
   interpreter renders it (an index, an overflow and its expression, a
   division by zero, a failed `assert` with its parts' values drawn under
   it as `format::parts_diagram` draws them, a contract's broken clause),
   `refine.not-satisfied` for a constant a refinement's predicate
   rejects, `comptime.budget`, `comptime.io` and `comptime.not-kept`; one
   that uses a failed one is not run, its error being that one's. The
   evaluator runs SIR as stage 0's interpreter
   runs it, a statement a step: values are trees, each aggregate owning
   its parts, and a reference is a path from a frame's local (stage 0's
   `value.rs` and `place.rs`), with the integers, the sized ones, `f64`
   and `f32`, text, lists, tuples, structs and enums, ranges, closures,
   calls through traits, the intrinsics compile-time code uses, and the
   structural `Debug` and `Display` of values (`format.rs`). One that
   needs more makes the program beyond the slice of lowering, naming what
   it needed; none in the corpus does.
6. The text form (`sir.write`).

Stage 0's rounds 7 and 8 changed lowering and the checks, and stage 1
follows them (decisions 220 to 233, as they bear on the front end): drop
elaboration opens a place into its parts only along paths a statement
moves out of (the "split" paths of decision 220); a pattern that moves
fields out of an error value moves the value whole into a local of its
set's type first, and a union of error sets is read through a reference of
the set's declared type (221); the operands of a statement are held for
dropping while a later one may leave, and a struct with an invariant is
registered for dropping while its check runs (222); a function used as a
value is its wrapper `f#value` when it is generic or borrows a parameter,
a generic one so used is instantiated, a generic intrinsic so used is
`E0350`, and one whose result borrows from a parameter the value would
take is `E0352` (225); and a channel's element type is checked as an
owner's, `E0421` and `E0422` (230), which R1 of RULES.md (decision 269)
since widens: what a channel carries is `Send`, as the bound asks it,
which a type parameter is by its bounds and a closure where it is sent
(`check_channel_type`, `check_sends`). The conformance suite's four new cases
(`concurrency/channel_owner_errors`, `generics/function_value_errors`,
`generics/function_values` and `run/drop_moved_parts`) are in
`types.list` and `sir.list`.

Stage 0's next round changed rules of the front end again (decisions 234
to 246), and stage 1 follows them. A value whose type holds a capability
handle or a `Cap` is dropped, as is one that is `Copy` only by a bound
(237: lowering's `needs_drop` asks `holds_capability`, without the
bounds); a temporary of such a type is moved into what takes it, an
argument a callee borrows is a copy of a place (`lent_copy`, by
`owned_params`) and a comparison borrows it (`compare_operand`); SIR's
drop glue drops the handles (`drops_something`); a parameter that holds
one counts as borrowed for the elision rules (`is_borrowed_param`), only a
`TestClock` lending nothing of its own (`is_run_handle`); and the
ownership checker gives a call's result a loan of the place it copied a
handle from (`handle_sources`), which a `return` may carry for a
parameter in its `from`. `@derive(Default)` is the prelude's trait, whose
template leaves out the fields with defaults and whose field check asks
`Default` of the others (238). `Atomic`, `RwLock` and `ReadGuard` are
known where `Mutex`, `MutexGuard` and `Cell` are (239: the auto traits,
views, smart pointers, a read guard's value that nobody may change, with
the note `type.read-guard-value`, the owner checks, `is_structural`, SIR's
drops and `place_ty`, the glue's host types and the ownership checker's
lists). A test takes nothing, the real `System`, or a fake of each
capability it names, once each, and lowering keeps `System.fake` and
`Net.fake` for it (241). Then `@cfg` and features, with `sir` and `build`
building a program without its tests (243, above, "Programs of several
modules"); templates beyond generic functions (244, above, "The type
checker's slice"); and `std.core`, `std.string` and `std.fmt` (245).
Decisions 234 to 236, 240, 242 and 246 change no rule of the front end.
The conformance suite's 19 new cases are in `types.list` and `sir.list`.

Round 9 of stage 0 changed rules of the front end once more (decisions 247
to 256), and stage 1 follows them. An iterator of a type of the program's
own given to an `Iterable[T]` parameter is passed as `iter_from(x)`, the
prelude's generator that takes its items, where the checker accepts the
argument (247: `Checker.iter_from`, from the coercion and from a call's
`Iterable[T]` argument; a reference to one is `E0300`). A function value
goes with what its environment holds (248): `holds_capability` holds for
a function type, so lowering drops and counts function values as it does
handles, and SIR's `drops_something` holds for one. `not(feature="x")` is
a call of `not` in an attribute's arguments, which the parser reads as a
name called and `@cfg` takes as the negation of its one condition (250:
`not_expr`, `cfg.Filter.holds`), and `check` and `build` take `--features
NAMES` and `--no-default-features` (below, "Stage 1 as the compiler's front
end"). Reflection reaches every place 08 allows (251): a test whose body
reflects is a template made once, each case of a parametrised one too,
whose instance begins by binding its case; a constant's value and a
field's or a parameter's default that reflects is unfolded once every
declaration is resolved, from a text of its own (`<NAME>/MODULE.sisu`, a
module of the compiler's with no items, `Resolver.unfold_value`) where it
is checked and whose file its body is in (`typeck.body_module`); a
`comptime def` that reflects, or has `T: type` parameters, is a template
whose type arguments a call writes among its arguments; a trait may
declare `comptime def`s, `Trait.name(...)` (`TraitInfo.comptime_fns`); a
trait's provided method that reflects is made in each impl that does not
write it (`trait_templates`, `provided_copies`); a method of a trait impl
is made once, as generic as the impl (`FnInfo.made_once`, `impl[A, ...]
__S`, `Resolver.shape_names`, `Kind.Param`), and an impl for every type
stops its instance (`E0609`); `reflect.methods_of` and `reflect.is_trait`
see a trait's methods or a type's own (`MethodShape`, `Value.Method`,
`Value.Param`); a closure's block may hold `comptime for` and `comptime
if`; `W0004` is not reported in an instance, and `E0900` names a
refinement's `where` clause or an `invariant`. A std module's generic
function that calls a template with its own type parameters is promoted,
as stage 0 finds checking every body of a program, the standard
library's among them: the base checks those of the modules a program may
import once (`typeck.std_promotions`, `json.decode` today), and the first
round of a program that loads one asks for their promotion; a request's
types are carried into the table of the next resolution
(`Types.carried`). The modules of 11's table that std lacked (`par`, `io`,
`fs`, `rand`, `encoding`, `hash`, `sort`, `search`, `process`, `env`,
`regex`) are importable (252: `std_sources`, `resolve.std_module`), `x in
xs` on a type that implements the prelude's `Contains[T]` is its
`contains` (`Checker.contains_call`), the expected type does not drive a
call whose result holds an associated type of a type its arguments have
not told yet, and lowering counts the types a call takes as type
arguments among those its function mentions (`reach_from`). A parameter
declared without a convention whose type is a reference is that
reference (255: `param_local_ty`, which the checker's own now calls), and
`result` of an `ensures` is a returned reference itself
(`result_borrows`). A type parameter bounded by `Escapable` holds no loan
(256, in stage 0's code only: `can_borrow_in`, for the solver's
`Escapable` and the owner checks). Decisions 249, 253 and 254 change no
rule of the front end. The conformance suite's 28 new cases are in
`types.list` and `sir.list`.

Round 10 of stage 0 (decisions 258 to 267) changed three rules of the
front end and decided the formatter's two discrepancies, and stage 1
follows. A test's instance stays a test once it is resolved (259): the
resolver keeps each instance's template (`Resolver.instance_of`, stage
0's), and an instance whose template is a test takes the test's kind when
its `FnInfo` is made again, so a `requires` in it is `E0807` and a value it
returns `E0381` (`reflect/test_instances`). A statement's operands are
used in order (261): an operand that uses a value an earlier operand of
its statement moved, or a part of it, is `E0400` (or `E0401` for the whole
after a part), the move labelled at the statement, which the
initialisation pass finds before it checks the use against the state
before the statement (`borrowck.init.moved_by_statement`,
`wrong/moved_twice_in_call`). Twenty diagnostics name a fix where the
wrong-ownership corpus had none (263): a note each (`borrow.copy-part`,
`type.assign-every-path`, `task.make-inside` and the others, eighteen
messages of the catalog new to `diag.Msg`) and the fix `type.declare-var`
on `E0367`, at the sites of the resolver, the type checker and the
ownership checker that report them, which the human form shows. And
`Option[Option[T]]` and `Option[ T ]` are formatted as stage 1 formatted
them (267, above, "The formatter"). Decision 258 adds `sisu doc` (below,
"Documentation"); 260, 262 and 264 to 266 are the backend's and the
runtime's. The conformance suite's 74 new programs (52 of the
wrong-ownership corpus, which `human` compares in the human form, now 93
programs; the 13 of `ownership/rustc/`; `doc/`'s three, one of them the
package `doc/package.pkg`, the first package case outside `packages/`,
which `check.sh` now finds anywhere in the suite; and six others) are in
`types.list` and `sir.list`.

The front end must agree with stage 0 on whether a program has an error,
since then `sisu sir` prints nothing and the ownership checker does not
run: stage 1 reports each error of stage 0's that is not the type
checker's (the resolver's `resolve.` and `borrow.` rules, among them the
borrowed signatures of `borrows.rs`, which `typeck.borrows` transcribes,
the coherence of impls and object safety), each rendered (above, "Stage 1
as the compiler's front end").

What is beyond the slice of lowering (`sir` and `ownership` print why on
standard error, `sir-batch` writes it to `OUT/n.beyond`): what is beyond
the type checker's slice, but the bodies that may get one of the
ownership checker's `type.` messages, which the ownership checker here
decides; a body of the standard library beyond the type checker's slice;
an ownership finding in a file other than the program's; and a
`comptime` expression that needs what stage 1's evaluator does not run.
Nothing in the corpus is: all 525 programs and stage 1's 17 other cases
get stage 0's SIR and ownership diagnostics exactly (but the SIR of the
two R1 lets in, which stage 0's front end refuses, and which the class
`behaviour` runs instead), and `sir.list` names the 542. Round 6's 48 programs beyond the slice came in
by what this round added: the standard library's derived impls written
from the templates (22; with them `<std>/<derive>/prelude.sisu` in the SIR
of nearly every program), the `comptime` evaluator (9), the generators
with the tests of contracts (6), of `arbitrary` and `exhaustive` (4,
`failing_test.pkg` among them) and of `@refines` (2), `@delegate` written
as text (3), the folding of `comptime assert` on `const` parameters (2,
the second of which needed the evaluator too) and the escape analysis
(2). Four of them then showed a difference the reason had hidden, each a
rule of stage 0's that stage 1 lacked: the checks of an `impl Drop`
(`resolve.drop-target`, `drop-generic` and `drop-signature`, with
`refine.own-impl`), the names of benchmarks (`bench#N`), the text of a
refinement's predicate in its failure message, and the range of a
generator's site (its index expression, not the call around it).

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

## The formatter

`fmt.sisu` and `fmt/` transcribe stage 0's formatter, `sisu-fmt` (10,
"Formatter"; 02, "Formatting"; docs/architecture.md, "The formatter"),
function for function: `fmt.lines` regroups the lossless tree's tokens
into logical lines (what the lexer ends with `NEWLINE`), places the
comments and blank lines between them, sorts a run of imports and leaves
out the unused ones (`fmt.prune` says which: an import is used when the
file spells its name outside the imports, or in a doc comment); `fmt.doc`
lays each line out with a printer of groups (Wadler's, as Prettier
implements it: a group prints flat when it fits in the rest of its line
and breaks otherwise); `fmt.space` says where a space goes between two
tokens; and `fmt.shape` what formatting must keep, for the self-check:
the result must parse without error to the input's shape, or the file is
refused (`D0076`). `format` and `format_with` are `pure` functions of the
text, which they parse with stage 1's parser; a file with a syntax error
is not formatted, its errors being the refusal.

Whether the `E` of a `Result[T, E]` names an error set, so that it is
written `T ! E`, is not in the syntax. Stage 0's formatter asks its caller
through a closure each time it meets one; stage 1's takes the answers,
the set of the types as written (`E`, `json.JsonError`) that name error
sets, and says what it would ask (`written_errors`). `format.sisu` is the
driver's part, `sisu-driver/src/format.rs`: as stage 0 does, a file that
writes no `Result[` is not resolved, and one that does is resolved as a
program of its own, a module of a package without the package's other
modules (`typeck.resolve_file`: its declarations and imports against the
standard library, as a build with its tests resolves them; stage 0 checks
the bodies too, which change none of the names the answers read), and
each type it writes is looked up there (`names_error_set`: the first name
in the file's module or the prelude, each further one in the module the
name before it names). The standard library is resolved once a run, when
the first file that needs it comes. The hunks `--check` prints are those
of the interpreter's line diff (`diff`, stage 0's
`sisu-interp/src/diff.rs`, with the whole diff a lockfile's change under
`--locked` prints, which `main` had).

`fmt [--check] [PATH...]` is `sisu fmt`: each file, and each `.sisu` file
under each directory (but hidden ones and `target`), or, without a path,
the package's, formatted in place, or with `--check` what would change,
with stage 0's messages (`D0001`, `D0009`, `D0013`, `D0064`, `D0074` to
`D0077`, the syntax errors in the human form) and exit statuses. A Sisu
program exits with status 1 only, by returning an error from `main`, so
`fmt` returns the status stage 0 exits with as the error
`Exit(status=N)`, which the run time prints as the last line of standard
error; `sisu1 fmt` takes that line away and exits with `N`: 1 when a file
is not formatted or could not be, 2 for a usage error, 70 for a file the
formatter refused as changed. `fmt-batch OUT LIST` writes, for each file,
what `fmt --check` prints of it and its status, the text `fmt` leaves of
it, and whether formatting that text again changes it.

What took another shape in Sisu:

- **The documents of a line are an arena** (`fmt.doc.Docs`): a document is
  a number, and what a group or an indentation holds is a list of
  numbers, kept in the arena too, as the AST and the typed tree are kept;
  the printer's stack holds numbers where the Rust's holds references
  into a tree of `Vec<Doc>`, and a group forced by a comment inside it is
  written over in place.
- **The tree is read through a view** (`fmt.lines.Src`, the tree and one
  view of its text, taken once a walk, since a view of a `String` copies it
  in the interpreter); the walker and a line's layout are view structs
  over it, given what they change as parameters.
- **The printer holds back the spaces at the end of its text** rather than
  popping them when a line ends (a `String` has no `pop`): what it has
  written never ends in a space, and the spaces it holds are written only
  when something else follows on the line.
- **The answers about error sets are a set**, as above.
- **What changes nothing is done once** (stage 1's own, for speed, each
  with the same output): a text that comes out as it went in is not
  parsed again for the self-check, since it parses to the tree it was
  parsed to; the shapes are written straight into one text, only the
  parts of a file and of a `from` import kept apart to be put in order,
  where the Rust makes each node's shape a text of its own and copies it
  into its parent's; the input's unused imports are found once for its
  layout and its shape; and they are found in one walk of the tokens
  beside the imports' ranges, which follow one another as the tokens do.
- **The line diff keeps its table as rows of bits.** The Rust fills the
  table of the longest common subsequence of the two texts' lines a cell
  at a time, which for a file whose every bracket changed is millions of
  cells; stage 1 numbers the lines and computes the same lengths in
  Hyyrö's bit-parallel form (a row a word of bits for every 62 lines, made
  in a few operations a word, a length a count of clear bits), and a test
  compares its marks with the table's on random lists.

`check.sh` compares the text `sisu fmt` leaves of each file (class `fmt`;
stage 0 formats a copy) and what `sisu fmt --check` prints of it with its
exit status (`fmt-check`) for every `.sisu` file of the conformance suite,
the standard library, the proposal's examples, stage 1's own sources and
its cases, the refusals of the files that do not parse among them; and
that formatting stage 1's text again changes nothing (`fmt-again`). The
corpus is mostly formatted already, so it shows the formatter keeping
what it should keep; what it must break, join and move shows in the same
files laid out otherwise, which stage 1's `deform` writes and the classes
`fmt-joined` (the lines inside each bracket joined into one, long ones)
and `fmt-spread` (a line break after each comma inside brackets) compare
the same way; and `fmt-mutants`, with `--mutants N`, the mutants: of
`--mutants 2000 --seed 7` over round 9's corpus, 1482 do not parse and
are refused, their syntax errors rendered as stage 0 renders them, and
518 are formatted (over this round's, all 2000 match too).
Stage 1's cases in `cases/fmt/` show the rules the corpus
leaves out: comments and blank lines, long lines and the magic trailing
comma, `T?`, `T ! E` (with an error set of the file and one of an
imported module), imports sorted and pruned, spacing, and a refusal. All
match, the two that showed D6 and D7 among them: stage 0's formatter
overflowed its stack on `Option[Option[T]]` and refused its own output
for `Option[ T ]` written with spaces, where decision 152 has them
written `Option[T?]` and `T?`; decision 267 decided both for the
documents, and stage 0's `option_sugar_asked` is what stage 1's was
(`fmt.lines.option_sugar_asked`, renamed after it).

The tests transcribe `sisu-fmt`'s (its twelve, the printer's three and
the spacing's one) and the line diff's two.

## Documentation

`doc.sisu` transcribes stage 0's `sisu doc` (`sisu-driver/src/doc.rs`;
09, "Documentation"; decision 258; docs/architecture.md,
"Documentation"), function for function: `doc.pages` is a `pure`
function of what the front end hands it (`doc.Docs`: each module's
syntax as the build has it, its text and its tokens, the resolved
program and its table of types, and the error sets the checker
inferred), and makes a page per module, as decision 258 lays it out byte
for byte: the module's path, its `##!` lines, the modules under it, the
names it re-exports (a package module's `pub` imports, or the prelude's
names a std module names), a list of its items with the first paragraph
of each one's doc comment, and a section per function, struct, enum,
error set, trait, constant and type alias, its declaration as written (a
header over several lines joined into one, its contracts keeping their
lines, a struct's fields, an enum's variants and an inherent impl's
constants with their own `##` lines, and the headers of the type's
impls), its doc text, and what the compiler knows that the declaration
does not say: the error set inferred for a `T !` function (`# inferred`,
a member of another module after that module's path), its typed holes
(the calls of the prelude's `todo`, as written, once each) and the
parameters that hold capabilities (decision 240's rule read from the
resolved signature, through references, tuples, type arguments, the
fields of structs and enums, a `Lent` and an operations trait's bound).
A type's methods and a trait's follow it, each a section of its own; the
impls of types the module does not declare close the page. A package's
pages show what another package may name; a single file's and the
standard library's show everything.

`doc [PATH] [--output DIR]` and `doc --std [MODULE...]` in stage 1's
`main` are the driver's part (stage 0's `doc` and `doc_pages` in
`cli.rs`): the program the path names (a file, a package, each member of
a workspace, with `--locked`, `--features` and `--no-default-features`
as `check` takes them) is read and checked as `sisu build` builds it,
without its tests (decision 243); one with an error is reported as `sisu
check` reports it, then `D0114` (`doc.errors-found`), and nothing is
written (`front_report`, the front end's part of what `check` prints,
which `report_of` now starts with). The pages are the root package's
modules', each lexed once more, parsed from those tokens and filtered by
its `@cfg`s, as stage 0 takes it from the build; with `--std` the
standard library's, the
prelude's (`std.core`) and those of the modules of 11 but `target` (a
program that imports each, `std_root`, is checked so that each is
loaded), or of those the paths name (`D0113`, `doc.std-path`, for a path
that is no module of it). They are printed each after a line `<!--
PATH.md -->`, or written into `DIR`, made when it is missing. The exit
statuses are stage 0's (2 for a usage error), which `doc` returns as
`Exit(status=N)` and `sisu1 doc` turns into its own, as for `fmt`.

What took another shape in Sisu: a `Writer` is a view of the
documentation and the index of its module among the sources, with one
`str` view of the module's text for the whole page; the Rust's closures
over iterators are loops; `join`, which takes back a list's trailing
comma and the space before it, keeps the pieces of the line in a list
until the line is done, since a `String` has no `pop`; and the
capability types and operations traits are `match`es rather than tables.

`check.sh` compares stage 1's pages with stage 0's (the classes `doc` and
`doc-output`, above), for every program of the corpus, single files and
packages, stage 1's own cases, its own package (82 pages) and the
standard library (`--std`, 28 pages, four of them by name, and the
prelude's alone), and all match; the conformance suite's `doc/` cases and the two of `contracts/`
whose `.doc` is `sisu doc`'s output are among them. Stage 1's own cases
show the layout rules the corpus leaves out (`cases/doc/layout.sisu`:
headers and contracts over several lines, attributes, inline code with
backticks in it, the sets that are empty or of one member, the impls of
`T?` and `ref T`, an inherent impl's constants, a refinement with
parameters, capabilities through a `Lent`, a `dyn`, a tuple, an option
and an enum's payload; `cases/packages/doc_pages.pkg`: re-exports of
every kind, of a module, under another name, of a dependency's and of the
standard library's, a set inferred from errors of other modules and of a
dependency, an item a feature that is off leaves out, and members another
package may not name). The tests check the helpers (inline code, doc
text, summaries) and a page made against a small prelude.

## Unsafe code

[22](../../proposal/22-unsafe.md) (D-080 to D-086) is implemented here,
and only here (D-089): the Rust front end parses `unsafe:` and `unsafe e`
and reports them as `E0900`, and parses neither `unsafe def`, `unsafe
trait` nor `unsafe impl`. RULES.md has each rule (R2 to R9), its cases and
its decision text (271 to 278); in short:

- **The syntax.** `unsafe def` (after `pure`, before `def`, as an item or
  a method), `unsafe trait`, `unsafe impl`, beside `unsafe:` and `unsafe
  e`; the AST's `is_unsafe` (written by the dump only when set, so that
  every other file's AST stays the Rust one), `StmtKind.Unsafe`,
  `ExprKind.Unsafe` and `TypeKind.PtrMut`. The formatter and `sisu doc`
  needed nothing: their rules are the tree's and the header as written
  (`sisu doc` shows `unsafe def` and the `Safety:` list as written).
- **The three unsafe operations** (`typeck.safety`): a call of an `unsafe
  def` or of a foreign function outside an `unsafe:` block or expression
  is `E0981` or `E0982`, labelled with the callee's obligations; neither
  is a function value (`E0983`); `unsafe impl` is how `Send`, `Sync` and an
  `unsafe trait` are implemented (`E0984` to `E0986`). An `extern` block
  declares foreign functions, whose calls lower to `extern.c.NAME`, as
  SIR marks a foreign call. Lowering passes the blocks through: the SIR of
  unsafe code is the SIR of its calls. `@intrinsic` and the other
  attributes only the standard library writes are `E0225` on a program's
  methods as on its items (stage 0 checks items only, and stage 1 had not
  transcribed the check), since an intrinsic body in a program would be an
  unsafe operation that needs no `unsafe`.
- **The comments and the lints** (`W0007` to `W0014`): the `# safety:`
  comment above each block, expression, `unsafe impl` and `extern` block,
  whose parts name the obligations they discharge; the `Safety:` list of
  each `unsafe def` and `unsafe trait` (a trait impl's `unsafe def` has the
  trait method's); unused unsafe, the bound of an `unsafe impl Send`, a
  public safe function reaching a foreign call without a capability.
- **`pure` and compile time.** An impure call in an `unsafe` block of a
  `pure` function is an assertion the ownership checker's purity check
  accepts (the ranges travel as `TBody.asserted` and
  `sir.Program.asserted`), explained under a `pure:` part; compile-time
  evaluation stops at a foreign call or a machine intrinsic, code it writes
  holds no `unsafe`, and neither does a contract.
- **Packages.** `unsafe = "memory" | "extern"` in `[package]`, checked
  against each package's code (`E1019`, `E1020`, `W0012`); a dependency
  that declares one needs the root's `[trust] unsafe` (`E1021`); the
  lockfile records the level and a rewrite that raises it needs
  `--accept-unsafe` (`E1022`, the test of `main`). Stage 0 rewrites a
  lockfile without the levels, so `check.sh` runs it on a package case
  through `in_package`, which leaves the case's lockfile as stage 1 wrote
  it.
- **The standard library's layer.** `std.mem` (raw pointers and their
  `unsafe def`s, `as_ptr`, `Layout`, `Allocator`, `Global`, `Region`,
  `RawBuf` with its safe `grow`, `Plain` and its derive, `bit_cast`,
  `view_plain`, `transmute`), `std.ffi` (the C types, `CString`, `CStr`)
  and `std.rt` (the machine intrinsics, internal to `std`) are in
  `stage1/std/`, since the frozen Rust front end parses none of their
  `unsafe` declarations and reads and formats every file of `std/`; stage
  1 reads them as the standard library's (`std_sources.std_file`).
  Making, growing and dropping a `RawBuf` (and a `Region`) are `pure`, so
  that `pure` code can use what is built on one. The pure Sisu of them
  runs (the derive of `Plain`, `Layout`, `align_up`;
  `cases/unsafe/plain_data`); the rest is checked (`raw_pointers`,
  `foreign_strings`).
- **The runtime's first slice** (R9): `std.mem`'s `structural_hash`,
  `structural_eq`, `structural_copy`, `drop` and `f64_bits`, the work
  `sisu-rt` does by type table (decision 256's hash, equality, copy) as
  generic code that reflection takes apart per type, with no machine
  support; `cases/runtime/structural.sisu` checks them against the
  interpreter's answers, and natively.
- **The audit.** `audit --unsafe [PATH]` lists the unsafe code by package,
  with its declared level and the program's consent, and by module, with
  its hash, each block with its function and comment, each declaration
  with its list, the foreign functions with what their callers hold, and
  the lints; `--std` lists the standard library's.

What the interpreter and the native backend run of unsafe code today:
`unsafe def`s with Sisu bodies, `unsafe impl`s, the blocks and the
`pure` assertion (`cases/unsafe/operations.sisu`, from stage 1's SIR),
and the runtime's slice. What they do not: the raw memory operations,
whose bodies are the intrinsics below, and foreign calls, which stage 0's
interpreter does not make (Q-55).

What SIR needs for the raw memory intrinsics. No new statement, operand
or terminator, and the text form does not change: every raw memory
operation is a call of an intrinsic, with the type arguments SIR already
writes (the impl's first, then the method's: `mem.ptr.cast[u8, i8]`), and
a foreign call is the intrinsic `extern.LIBRARY.NAME`, without `pure`, as
`sim.rs` reads it. What the interpreter and the backend must add:

1. the types `ptr[T]` and `ptr_mut[T]` as builtins: lowering declares
   them as the other builtins are declared (`struct ptr[T] {  }`), and a
   value of one is a block, an offset and the tag of the loan it came
   from in the interpreter's checked model (D-085), a machine word
   natively, with `ptr[T]?` laid out as a nullable pointer;
2. a layout per type (size, alignment, the offsets of a `@repr(c)`
   struct's fields), which `mem.size_of`, `mem.align_of`, `Layout.of`
   and the checked model's bounds and alignment checks read;
3. a place that is a raw location (a block and an offset), so that
   `mem.ptr.as_ref` and `as_mut` give a reference the monitor tracks with
   the origin the checker gave it, and `read`, `write` and `drop_in_place`
   move into and out of it;
4. the allocations of `Global` (`malloc` and `free` natively, blocks
   with an identity interpreted), each slot's initialisation, and the
   bits' validity for the type read;
5. the purity assertions of `unsafe` blocks (`sir.Program.asserted`),
   which SIR's text does not carry: stage 1's ownership checker applies
   them; a consumer that checks purity again from the text would need a
   line for each (`asserted SOURCE @L:C-L:C`).

The intrinsics the stage-0 round implements, each `pure` unless said:

| Intrinsic | Signature | Meaning |
|---|---|---|
| `mem.ptr.of`, `mem.ptr.of_mut` | `(x: ref T) -> ptr[T]`, `(mut x: T) -> ptr_mut[T]` | a pointer to a place; the borrow ends at once |
| `mem.ptr.dangling` | `() -> ptr[T]` (and `ptr_mut[T]`) | aligned, never used: an empty buffer's |
| `mem.ptr.from_address` | `(a: int) -> ptr[T]`, impure | the device or foreign memory at `a` |
| `mem.ptr.cast`, `mem.ptr.with_addr`, `mem.ptr.as_const` | `[T, U](p) -> ptr[U]`, `(p, a: int)`, `(p: ptr_mut[T]) -> ptr[T]` | the same provenance, another type or address |
| `mem.ptr.addr` | `(p) -> int`, impure | the address |
| `mem.ptr.offset`, `mem.ptr.cast_mut` | `(p, n: int)`, `(p: ptr[T]) -> ptr_mut[T]` | `n` values on, in bounds; from a `ptr mut` |
| `mem.ptr.read`, `mem.ptr.write` | `(p) -> T`, `(p: ptr_mut[T], own v: T)` | move out (the slot then holds none); write without dropping |
| `mem.ptr.drop_in_place` | `(p: ptr_mut[T])`, impure | run `T`'s drop; the slot then holds none |
| `mem.ptr.copy_from` | `(p: ptr_mut[T], src: ptr[T], n: int)` | copy `n` slots as bits, each with what it holds (a value or none), as through a buffer |
| `mem.ptr.as_ref`, `mem.ptr.as_mut` | `(p) -> ref T`, `(p: ptr_mut[T]) -> ref mut T` | a reference with the origin its use asks for |
| `mem.ptr.read_volatile`, `mem.ptr.write_volatile` | `(p) -> T`, `(p: ptr_mut[T], v: T)`, impure | never removed or merged |
| `mem.slice.as_ptr` | `[T](xs: Slice[T]) -> ptr[T]` | the pointer to a slice's first value (aligned and unused for an empty one) |
| `mem.size_of`, `mem.align_of` | `[T]() -> int` | the layout of `T` |
| `mem.global.acquire`, `mem.global.release` | `(g: Global, l: Layout) -> ptr_mut[u8]?`, `(g, b: ptr_mut[u8], l: Layout)` | the process allocator (`malloc` natively, the checked model interpreted) |
| `mem.bit_cast`, `mem.as_bytes`, `mem.read_plain`, `mem.view_plain` | `[U, T](x: T) -> U`, `[T](x: T) -> bytes`, `[T](data: bytes, at: int) -> T`, `[T](data: bytes) -> ref T?` | reinterpretation of plain data (sizes equal for `bit_cast`; `None` from `view_plain` when short or misaligned) |
| `mem.transmute`, `mem.slice_from_raw` | `[U, T](own x: T) -> U`, `[T](p: ptr[T], n: int) -> Slice[T]` | unsafe reinterpretation; a view of raw parts |
| `rt.switch`, `rt.stack_pointer`, `rt.thread_slot` | `(save: ptr_mut[Context], load: ptr[Context])`, `() -> int`, `() -> ptr_mut[int]`, impure | the machine intrinsics of D-086 |
| `extern.LIBRARY.NAME` | the declaration's, impure | a foreign call (`extern.c.strlen`) |

What remains: the checked memory model (D-085) in the interpreter and,
with it, raw memory operations at compile time (stage 1's `interp` stops
at a `mem.*` intrinsic, which makes the program beyond the slice of its
lowering); `bit_cast`'s and `transmute`'s compile-time check of sizes;
the derive of `Plain`'s refusal of padding, of an `invariant` and of a
refinement (stage 1 has no layout at compile time, and reflection does
not show invariants); `p == q` on pointers (impure, 22, which an `Eq` impl, whose `eq` is
`pure`, cannot be; `addr` is there); `xs.as_ptr()` and `Cell.as_ptr()` as
methods (the prelude, which the Rust front end reads, cannot name a
pointer type; `mem.as_ptr` takes a slice instead); `Slice.from_raw` (the
function is `mem.slice_from_raw`); a foreign call to a function another
package declares counted toward the caller's level; `@export` and
`@blocking` past the front end, which R19 has check an export's signature
and where `@blocking` goes; the header import; `sisu publish`'s refusal
of undocumented unsafe code; the teaching profile's and hot replacement's rules, which stage 1
has neither of; and, of the runtime in Sisu, everything past R9's slice
(`RawBuf` under `list` and `String`, `Box`, `Shared`, the stack pool and
the scheduler), which waits for the intrinsics above.

## The manual's findings

The manual's writers list every place where stage 0 printed something
the proposal does not say (manual/README.md, "What the plan found in
stage 0"); most are the front end's, and stage 1 decides each from the
documents. RULES.md has each rule (R10 to R16), its cases (in
`cases/findings/`) and its decision text (291 to 297); in short:

- **Missing patterns** (R10; 03): `E0314` names every pattern no arm
  covers, up to eight, a part no arm names any constructor of as `_`.
- **`sisu lint`** (R11; D-043, 10): a command of its own, the lints beyond
  the compiler's warnings; its first, `W0015`, is a `case _` over an enum
  of the same package, with the variants it matches as the fix. `check`
  prints no lint (`Checked.lints` takes them out of its diagnostics).
- **Naming** (R12; 02): `E0267` to `E0269`, a declaration not in its
  kind's case, with the name so written as the fix (`naming`).
- **A trait is no value** (R13): `Add(left, right)` makes a variant named
  like the prelude's trait where no type is expected.
- **An interpolation reads** (R14): `{u.team?.name}` reads through a place,
  link by link, rather than moving out of it.
- **An error set that hides the prelude's** (R15; D-024): `W0016`.
- **Fixes and messages** (R16; 10, 02, D-061): `String` for a `str` field,
  `mut self`, `@derive` with the supertraits a type lacks, a copy of a
  borrowed list of non-`Copy` items with a note on `Copy`, `Path.new`,
  `-> ! E` and `return Ok(())` for `?` in a function returning nothing,
  `.chars()` and `.bytes()`, a character written as text,
  `print("{a} {b}")`, `let` or `var` for a name declared by assignment
  (whose uses are then not reported), `Fs` for `open`, `E0127` for a
  loop's `else`, `E0118` with its block for an assignment on a `case`
  line, and a hole of type `()` for a `todo` statement.

R17 (decision 298) is the coordinator's, for the stage-0 round that runs
`std.mem`: an intrinsic's call writes its type arguments when one of its
own generic parameters is in no parameter (`mem.size_of[int]()`;
`cases/lowering/size_of_type_arguments.sisu`, whose `.sir` the class
`expected` holds `sir` to).

`{x=:.1f}` ignoring the precision turned out to be the front end's: the
`Debug` impls of `f64` and `f32` are text stage 1 writes among
`builtin_impls`, and since R20 (decision 313) they apply a spec as their
`Display` impls do (`cases/run/float_debug_precision.sisu`). The `Debug`
form of an error `main` returns is the run time's, and is left to it. One
finding is not decided here, since the documents disagree: whether a
public type has a `Debug` form without asking. 10 says the derived one is
"opt-in for public" types, where the same sentence says "a `Debug`
rendering exists for every type", 11's principle 4 says every type has
`Debug`, and docs/architecture.md's decision 66 gives every struct and
enum the derived one; and `{x=}`, `{x:?}`, `assert`'s values and the error
`main` returns all print a value's `Debug` form, so taking it from public
types would break what prints them. It waits for the owner.

### The second round: soundness first

The writers of chapters 10 to 22 and of the reference found more, and
this round read every entry of their lists and sorted it: a hole of
soundness or of the specification (a program the checker accepted that
the documents refuse, or one whose run does what its text does not say),
or a missing convenience (a fix, a message, a form refused that the
documents give). RULES.md has the rules (R21 to R26, decisions 320 to
325), their cases and their decision texts:

- **A collection holds values or views** (R21; 04, rule 3): `list[ref
  String]`, written or inferred (`push(ref x)`, `[ref a]`, a `collect()`
  of an iterator over a borrowed list), is `E0438`, with the fix that holds
  the values or collects copies (`cases/references/`). Three conformance
  programs made such a list, which their cases will have to change.
- **What the checker let through** (R22): an impl's method with other
  types than its trait's (`E0234`, which stage 1 compared by shape only),
  `impl Copy` by hand (`E0270`), a `dyn` of a trait that cannot be one in
  a body's type (`E0241`), a trait method whose impl nothing decides
  (`E0324`), and an `assert` that moves the value it tests, which showed
  its parts by evaluating them again (`cases/soundness/`).
- **What lowering wrote that the program did not say** (R23): a sized
  integer's compound assignment at the width of `int` (`+%=` left 260 in a
  `u8`), and a `Copy` instance of a tuple or option of the function's type
  parameters passed by value where the function took it by reference.
- **Fixes and notes** (R24): Python's `", ".join(names)`, `append`,
  `clone` and the other spellings, `.iter()` before an iterator's method on
  a list, the bound a type parameter lacks for a method, `.to_string()`
  rather than `.to_lowercase()`, `E0340` and `E0341` agreeing, a `String`
  result for `E0413` and `E0407` on text, `E0400`'s remedies with their
  costs, `Default` in `E0243`'s list, and `from pkg import x` of a name
  `pkg` re-exports (`cases/habits/`, `cases/packages/reexport_import.pkg`).
- **Forms the documents give** (R25): `with x:` without `as`, `ref` of a
  temporary, `len` of a `Box`, `print` of a `Slice`, a `Key[T]` as a set
  element, a trait method's type arguments (so `collect[set[int]]()` is
  `E0349` where it made a list), and messages for `sorted(xs, key=len)`, an
  untyped closure in a `let`, and `import *`.
- **A generic struct's construction** (R26): the type expected gives its
  type arguments (`Reading(21.5, "°C")` as a `Reading[f64, String]`), and
  a field's default is as generic as its struct, where `Shelf[int]()` was
  invalid SIR (`E0501`).

What the documents give but the front end alone cannot do waits for the
runtime: an `own` closure that moves a capture that is not `Copy` (04)
lowers and checks as a task's does, but the native runtime does not drop
what such a closure's environment owns (four allocations live at exit in
a trial), so `E0423` stays; `expect`'s non-fatal failure (17) needs the
test runner; `Shared.same` (02) an identity intrinsic. The findings this
round left, and why, are in "What is next".

The run of the conformance suite's own cases changed too: `check.sh`'s
`behaviour` runs natively only the cases of the suite's native list
(`conformance/native.list`), since the cases of the checked memory model
that joined the suite expect what the interpreter alone reports, and a
case a rule refuses is counted "refused as listed".

### The third round: what ships

The writers of Part V, the guides and the explanations found more, and
this round took the correctness of what ships first: what lets a hole, a
broken contract or an impure body through. RULES.md has the rules (R27 to
R36, decisions 340 to 349), their cases and their decision texts:

- **Typed holes** (R27; D-061, 10, 17): a release build (`sisu build
  --native --release`, `sisu run --native --release`, `sisu1 build
  --release`) rejects each hole with `E0445`, naming it and the
  `panic("...")` that may ship; a run that reaches a hole prints its
  closure first, `in scope at the typed hole `todo("area")`:` and `NAME =
  VALUE` for each binding that holds its whole value there (the
  initialisation dataflow says which), holds no loan or is a parameter,
  has the host's `Debug` and holds no capability, a closure's captures
  after its own (`borrowck.holes`, written into the SIR after the
  ownership checker and compile-time evaluation, from the bindings
  lowering names, `sir.Function.shown`); and `sisu1 test` runs the tests
  stage 1 built with stage 0's runner and exits with 3 when none failed
  and one is blocked (1 when one failed). `sisu test` itself is stage 0's
  runner and front end, whose status and routing are the stage-0 round's.
- **`pure` code** (R28; 06): an `Atomic`'s methods, `load` among them, are
  impure there (`E0333` names `Atomic.load`), and the body of `for x in
  par.iter(xs):` is `pure`: it calls only pure functions (`E0446`) and
  changes nothing declared before the loop (`E0447`).
- **A trait's contract** (R29; 07): an impl's method checks the trait's
  `requires` before its own and its `ensures` after, under the trait's
  names of the parameters, a failure naming the trait's method, through
  any call; a trait's method whose body is its contract alone is
  required. An impl in another module than the trait's is warned that
  stage 1 does not hold it to the contract yet (`W0021`).
- **Attributes** (R30; 13, D-046): a call of a `@deprecated` function or
  method warns with the attribute's text (`W0023`, `W0025`); an attribute
  Sisu does not read warns with the fix when it misspells one (`W0024`),
  and when no compile-time code of the program reads attributes
  (`W0022`).
- **`embed`** (R31; 08) resolves from the package's root.
- **`json.decode`** (R32; 11, 05) validates a refinement's predicate and a
  struct's invariant at the boundary, a `Mismatch` naming where, not a
  type error in the decoder nor a panic, in stage 1's copy of `std.json`
  (`stage1/std/json.sisu`), with reflection's `is_refinement`, `base_of`
  and `has_invariant`.
- **Numbers** (R33; 03, D-076): a float literal reaches a `Decimal` with
  its places (`3.50`), `/` between `Decimal`s names `div` and `to_ratio`,
  `@float(fast)` is accepted (strict floating point meets its permission)
  and takes nothing else, `@diagnostic_name` is `E0900` until Q-27, and a
  `list` has `is_sorted`.
- **Reflection** (R34; 08): a plain `for` over a compile-time list gets
  the fix that writes `comptime for`. `reflect.fields_of(int)` stays an
  empty list, as the conformance suite's `reflect/methods` has it.
- **`max` and `min` of a collection** (R35; 02): one argument is Python's
  form, the largest or smallest item (the prelude's `max_of`, `min_of`).
- **`@derive(Encode, Decode)`** (R36; 11): `std.json`'s traits, which
  `encode` and `decode` use for a type that implements them; the derive
  gives a type the form of its shape, an impl by hand one of its own.

The compile-time evaluator (`interp`) runs what the explanations' writer
found it stopping at, each a transcription of stage 0's interpreter, to
the same SIR (`cases/comptime/evaluator.sisu`, in `sir.list`): a `for`
over a list, a set or a dict (`iter.ref`, `iter.own` and their `next`),
sets and dicts themselves (their literals, `[k]` read and written, `in`,
`len`, `add`, `get`, `insert`, `remove`, `items`, `keys`, `values`, their
`Debug` and their literal form in SIR), `sum` (an `f64`'s pairwise, as the
run time sums), `int.to_f64`, `String.push`, `list.join`, `list.new` (a
comprehension), `str.byte_at`, and a typed hole, which is the expression's
panic (`E0602`) as it is stage 0's, where each was "beyond the slice".
Those are not rules, since the Rust front end ran them.

`check.sh` holds three new kinds of expectation: a case's `NAME.release`
(what `build --native --release` prints), its `NAME.tests` (what `test`
prints, the replay tokens' hashes written `HASH`), and, for a case of the
conformance suite whose expectation a rule changes, the file of the same
path under `expected/` (`expected/conformance/cases/types/typed_hole.err`,
R27's closure), which the class `behaviour` holds the run to and counts as
"changed as listed" when RULES.md lists the case's output `behaviour`.

### The fourth round: the third round's rest

This round took what the third left open and the manual's findings of
the front end that were stage 1's to decide. RULES.md has the rules (R37
to R46, decisions 361 to 370), their cases and their decision texts:

- **Trait associated constants** (R37; 03): `const SIDES: int` in a
  trait, given by each impl or by the trait's value, read as `T.SIDES`,
  `Self.SIDES` or `Square.SIDES`; each is the trait's `pure` function
  without `self` that gives it (`FnInfo.assoc_const`), so a read through
  a bound is the trait's call made for the instance. An impl gives each
  constant the trait leaves without a value, no other, with the trait's
  type, as a constant (`E0232`, `E0271` to `E0273`); `Shape.SIDES` is
  `E0449`, and a trait with a constant is no `dyn` type.
- **`expect`** (R38; 17): in a program built with its tests, a failed
  `expect` calls `expect.fail`, `expect.fail_compare` or
  `expect.fail_parts`, which take `assert`'s arguments and return, and the
  test goes on; `assert` stops as before, and so does `expect` outside a
  build with its tests. The interpreter and the runner implement the
  three intrinsics, which is the stage-0 round's ("What is next").
- **A trait's contract across modules** (R39; 07): an impl of another
  module than the trait's is held to the trait's `requires` and `ensures`,
  checked in the syntax of the trait's module (its names, `old`, the
  parts a failure shows), the check standing at the impl's method's name;
  `W0021` stays for a trait whose module's syntax a check does not have.
- **Deprecation and attributes** (R40; 13, D-046): a deprecated type,
  constant or variant warns where it is used, but in its own declaration;
  an attribute Sisu reads that stands where it means nothing warns
  (`W0026`); a suite's `@tag`s tag its tests.
- **`@derive(Arbitrary)`** (R41; 17): the prelude's `Arbitrary`, which a
  type implements by hand to be drawn its own way (`arbitrary[T]` and the
  generators of the types that hold it use the impl) or derives to be
  drawn by its shape, so that a bound `T: Arbitrary` draws it.
- **Generator expressions** (R42; 02): `(f(x) for x in xs if c(x))` is a
  lazy `Iter`, the generator that runs the clauses around a `yield` of
  the element, task-like as a generator function's body is, borrowing
  what it reads while it lives.
- **Forms the documents give** (R43): `xs[1:^1]`, a constant in an enum's
  body, `N.to_f32()` on a `const` parameter, sized integers compared as
  they add, `2.0 * v` through `impl Mul[Vector] for f64`; and refused, a
  supertrait cycle through other traits and a field or a variant declared
  twice (`E0205`, `E0206`, as stage 0 has them), which stage 1 let
  through.
- **`@cfg` on members** (R44; 08): a field, a variant or a method in a
  struct's or an enum's own body is left out where its condition does not
  hold.
- **Fixes and notes** (R45): double quotes for `'Aino'`, `E0327` for
  `Light.Gren`, Python's conversions, `Some(5)`, `.to_f64()`, `repeat`,
  the operator's trait, a misspelt keyword argument, keyword-only
  parameters, `.or(...)` for `??` on a result, `?.` after a result, a copy
  of a parameter the body changes, `remove` for an item moved out of a
  list, a `Shared`'s field, and an `own` closure's capture.
- **No SIR of a refused program with its tests** (R46; decision 357):
  `sir --tests`, from which `sisu test` runs, prints no SIR of a program
  with an error of any phase, the ownership checker's and a failed
  `comptime` expression's among them, and says why as `sisu test` does
  (`D0006`, `D0021`), where `sisu test` ran the tests of the manual's
  exercises `winner` and `journeys` in their start states.

`check.sh` holds a case to a new kind of expectation, its
`NAME.sir-tests`, what `sir --tests` prints of it (R38's intrinsics and
R40's tags), and what it prints on standard error when there is any
(R46's refusals). Two of the conformance suite's `.diag` files are blessed
again, with R45's fixes: `syntax/unicode_escape_sign` and
`types/argument_count`. Two items of the round are not rules: the compile-time
evaluator's speed, and the base cache's write (below, "Where the time
goes" and "Starting up").

### The fifth round: the merge's rest, the driver's one run, the open items

This round took the differences the merge of the fourth round with stage
0's third left unlisted, what the driver needs to run stage 1 once for a
test run, the three items the stage-0 round assigned to stage 1, the
fourth round's open items, and the front-end findings of the manual's
list that were stage 1's. RULES.md has the rules (R47 to R55, decisions 400
to 408), their cases and their decision texts:

- **The merge's nine differences** were rules' already: the conformance
  suite's new `testing/unsafe_level.pkg` declares `unsafe = "memory"` for
  an `unsafe:` block over `std.mem` (R6, with R2 and R4: its AST, names,
  SIR, diagnostics and pages), `run/debug_forms` reaches a float's `Debug`
  (R20: its SIR calls `Spec.float`) and `std/decimal_literals` lowers its
  literals from their text (R33). Each is listed there; nothing in stage
  1 was wrong.
- **`sir --tests` prints the warnings** (R47): from one check of the
  program, the SIR on standard output and the program's warnings on
  standard error, as `check` prints them (`--format=ids` too), and `sir`
  takes `--features`, `--no-default-features` and `--locked` as `check`
  does. RULES.md says what the driver may rely on, so that `sisu test`
  drops its run of `check`; stage 1's own `test` checks once too
  (`report_with`).
- **The stage-0 round's three** (R48): `xs[^1]` and `xs[1:^1]` of a `Slice`
  and of `bytes` (the prelude's `at_end` and `cut_end`, the base evaluated
  once), `collections.Deque[int].new()`, and `E0413`'s `from` clause first
  when the body returns a borrowed parameter or a part of one.
- **The fourth round's open items**: a generator expression's first
  iterable evaluated where the expression is, kept by the generator, a `?`
  in it passing its error on (R49); a type's constant as a constant
  argument, `Array[int, Square.SIDES]`, folded from the impl's or the
  type's value, and through a type parameter decided against, with a note
  that names the `const` parameter (R50); deprecated traits and fields
  (R51); `@derive(Arbitrary)` of a generic type and an impl of `Arbitrary`
  in another module than its type's (R52); and an attribute before a
  statement, `E0129`, the loop directives noted as 19's planned N-22
  (R53), where taking them on a loop and checking nothing is what 19
  rules out.
- **The manual's findings** that were stage 1's: the fix that writes
  Sisu's spelling at a parse error over another language's (`&&`, `!`,
  `&x`, `i++`, `let mut`, `as f64`, `println!`, `0..3`, `Vec<int>`,
  `switch`, `is None`, `=>`, `//`, `nil`), an attribute on its declaration's
  line (`E0131`), `pub(crate)` (`E0130`), a one-line `if` (`E0102`) and a
  keyword as a field's name (`E0124`) (R54); and the type checker's notes
  and fixes for a `var` that holds a borrow, a borrowed parameter given to
  `own`, a narrowing conversion, `let Total`, and `E0417`, `E0806` and
  `E0810` without an article (R55).

Decided against this round: registering an interpolation's buffer in
the lowering (decision 381's other way), since it changes the SIR of
every program whose interpolation has a value that may fail, which
`check.sh`'s `sir` classes hold to the frozen Rust front end's, for frees
the backend's bookkeeping already makes; a trait's constant through a
type parameter as a constant argument (R50); and the loop directives on a
statement (R53). Two of the conformance suite's `.diag` files are blessed
again, with R48's order: `ownership/from_required` and
`wrong/missing_from`. `check.sh` holds a package case to its
`NAME.pkg.tests` too (R52).

### The sixth round: the driver's version, the stage-0 round's items, the open findings

This round took the difference the merge left unlisted, what the driver
needs to stop probing stage 1, the base cache's atomic write, the items
the stage-0 rounds assigned to stage 1, the fifth round's open items and
the manual's findings that were stage 1's. RULES.md has the rules (R56 to
R64, decisions 440 to 448) and their decision texts:

- **The merge's difference**: the conformance suite's new
  `run/stopped_by_errors` (decision 427) expects its errors to stop the
  run. `check.sh`'s `behaviour` now knows such a case, one whose `.err`
  ends with `D0006` or `D0021` and whose status is 1: stage 1 prints no
  SIR of it, and what stage 1's `build --for-run` prints of it, the errors
  and that line, must be the case's `.err`; it counts as expected. No
  rule is needed: stage 1 does what the case says.
- **`sisu1 --version`** (R56) prints `sisu1 0.6.0`, one line, before it
  reads anything; the minor number is the round that last changed what
  the driver may rely on, so the driver can ask it rather than probe.
- **The base cache** (`main.write_base_cache`) is written aside,
  `base-ID.cache.N.aside` with `N` drawn at random, and renamed over the
  old one with `Fs.rename` (decision 420): a reader finds the old cache or
  the new one. What a crashed write left aside goes with the next write
  of another executable's cache. Not a rule: it changes no output.
- **The stage-0 rounds' items**: a hole's closure shows each `old(...)`
  once (R58); `sisu build` and `sisu sir` of a program with errors end
  with `D0132`, "the program was not compiled", and the build a run asks
  for (`build --for-run`, which the driver's `sisu run` now passes) with
  `D0006` as before (R57); the audit names `sisu audit --unsafe --std`.
  The doubled module path of an impl's method,
  `kiuas.meter.Dial.kiuas.meter.Meter.read`, is the SIR name both front
  ends give the method (stage 0's SIR of the same package names it so,
  checked by hand): the name of a method of a trait impl is the
  qualified type, the qualified trait and the method, unique and the
  oracle's, which the classes `sir` hold stage 1 to; what reads oddly is
  the run time's trace, which prints the SIR name, and is stage 0's to
  shorten (`Dial.read`, or the trait in brackets). `R0103` (a
  refinement's check, `contract.fail` of kind 3) is raised: a constant
  given to a refinement is checked at compile time, in a `comptime`
  expression whose failure is `E0810`, and that expression is kept in
  the SIR `sisu sir` prints of a refused program, whose run from that SIR
  panics with R0103's text (`cases/contracts/refinement_check.sisu`, its
  `.sir`, the same as stage 0's, and `.diag`). Nothing else lowers a
  refinement's check: any other value becomes one only through
  `try_from`, which returns an error.
- **The fifth round's open items and the manual's findings**:
  `debug_assert` (R59); `import a.b.Item`, `E0223` with the fix
  `from a.b import Item` (R60); a `Slice` given to `sorted`, `{b.note ??
  "-"}` in an interpolation and `comptime name(bool)` (R61, the last a
  regression of stage 1's name pass, which the Rust front end took);
  `@derive(Decode)` on an enum stopped at the derive (R62); the fixes and
  notes for `map(f)` over references, `enumerate(start=1)`, `1.` and `.5`,
  Python's conditional and walrus inside an expression, `E0231` for a
  parameter's type, `E0801`'s `pure def` and a manifest's key reported
  twice (R63); a variadic parameter and star unpacking, `E0900` saying so
  (R64).

Decided against this round, from the documents: a `u32` shifted by an
`int` widens to `int`, as D-016's implicit lossless widening has every
mix of a sized integer and an `int` (03 and 02 say nothing of a shift's
type, so a shift is an operator as the others); the trace's names (stage
0's, above); lowering star unpacking, whose refutable pattern 02 gives no
`else` for, and a variadic parameter, which 02 and 14 disagree on (R64);
and adapting a function given as `map(f)` to references by itself, which
would be an implicit conversion (R63 offers the closure instead).

Open: a derived impl's diagnostics are still placed in `<derive>/` (only
R62's derive stops at the derive's argument; the others need each
template's diagnostics mapped back to the attribute); a call of a
function with a variadic parameter is also `E0305` after R64's `E0900`;
`E0260` for a dependency whose `when` excludes the target, and the
derives' messages; and the rest of "What is next", 11.

Two of the conformance suite's `.diag` files are blessed again, with R63:
`contracts/contract_errors` (`E0801`'s fix) and
`packages/manifest_errors` (no `E1002` beside `E1003`).

### The seventh round: black-box tests, the merge's differences, the open items

This round took the differences the merge of stage 0's fifth round left,
the black-box tests the driver needs stage 1 to compile, the
compile-time evaluator's `Shared`, the sixth round's open items and the
manual's findings of step 16 that were stage 1's. RULES.md has the rules
(R65 to R72, decisions 480 to 487) and their decision texts:

- **The merge's differences**: the conformance suite's new
  `run/shared_equality` (decision 464) writes `d.get(b) == Some(ref 1)`,
  the borrow of a temporary, which R25 lets stage 1 lower and the frozen
  Rust front end refuses with `E0313`: its six differences (types, sir,
  ids, human, doc, doc-output) are R25's, listed there; the lowering of
  `==` on `Shared` and std's new `impl[T: Hash] Hash for Shared[T]` agree
  (its run, interpreted and native, prints the case's `.out`). And
  stage 1's `cases/holes/blocked.tests` and `failed.tests` are blessed
  again: since decision 466 a test's output is captured and shown only
  with its failure, and a blocked test's hole closure, which the run time
  prints as the test's output, goes with it. That drops what R27 says a
  blocked test prints (its text says so now); stage 0 is asked to show a
  blocked test's captured output under its line.
- **Black-box tests** (R65): a file of a package's `tests/` is a program
  whose dependencies are the package, its `pub` items only, and the
  package's dev-dependencies; `sir --tests PKG/tests/NAME.sisu` prints its
  SIR and its own tests' lines, without the package's tests, and `sisu1
  test PKG` runs each after the package's own. Stage 1's version is
  0.7.0: the driver may rely on it.
- **The evaluator's `Shared`** (R66): held as its value, so `==`, `hash`,
  a set and a dict follow it to the value, as decision 464 does at run
  time; its identity is beyond the evaluator, and a `Shared` kept for run
  time is `E0604`, as stage 0 has it.
- **The sixth round's open items**: a derived impl's diagnostics placed at
  the derive's argument with a note that gives the impl's line, and the
  derives' messages (R70); `E0275` for a dependency that `when` excludes
  and a variadic function's call that is no `E0305` (R71).
- **The manual's findings**: R54's fix inside a call, in a `for` header
  and for a `//` comment at a line's end, and a fix that names its
  replacement once (R67); the five messages without "a `{ty}`" and
  `E0274` for a trait's constant where a type goes (R68); `sisu lint
  --help` is `D0009`, the audit heads a file "no level", and `fmt`
  formats a file whose one error is `E0130`, whose tree is whole (R69);
  Python's `lambda`, `f"..."`, `class`, `try:`, Go's `go f()` and a
  keyword as a binding's name (R72).

Decided against this round: showing a blocked test's closure from stage 1
(the runner that captures it is stage 0's, above); offering a fix that
inserts `from std.json import Encode` (a note names it: where the import
goes is the program's choice); a derive's diagnostic placed in the
trait's template (the derive's argument is what the program's writer
wrote; the note gives the template's line as the impl has it); reading
`print(a // b)` with a name after `//` as a comment anywhere but at the
end of a line; formatting a file with any other parse diagnostic.

Open: the rest of "What is next", 11 (below); stage 0's side of R65, the
driver's `sisu test` running the black-box files; and the closure of a
blocked test.

The conformance suite's four `.defer` files that hold `R0069` are blessed
again with R68: `traits/must_consume_generic`,
`ownership/must_consume_dropped`, `ownership/must_consume_container` and
`ownership/must_consume_try`.

### The eighth round: the merge's differences, item 11, dev-dependencies, `expect`'s words

This round took the differences the merge of stage 0's sixth round left,
the rest of "What is next", 11, as far as the documents decide it, the
dev-dependency a package's code could import, the words of a failed
`expect` and the manual's findings that were stage 1's. RULES.md has the
rules (R73 to R78, decisions 520 to 525) and their decision texts:

- **The merge's differences**: the conformance suite's new
  `testing/blocked_closure` (decision 507) reaches a hole, whose closure
  stage 1's SIR prints (R27's, listed there: `sir`); stage 1's
  `cases/contracts/trait_contracts` and
  `cases/packages/trait_contract_elsewhere`, whose traces name the impl's
  method `Counter.add for Tally` and `Counter.add for Sloppy` since
  decision 504, and `cases/holes/blocked.tests` and `failed.tests`, which
  show a blocked test's closure in its block again since decision 507,
  are blessed again: each new trace and block is what the decisions say.
- **Item 11**: a test's name is fixed text, `E0132` for an interpolation
  in it with the fix that doubles its braces (R73); a module's name gives
  way to an import named like it, and `E0304` on a local
  that hides an imported module says so (R74); `else as e` on a
  statement, `d.get(k, default)`, a positional-only parameter passed by
  name, `JsonError` alone and `E0981`'s `ptr mut T` (R76); a template's
  instance's diagnostics at the call that made it (code `comptime for`
  unrolls, `reflect.construct` of a plain parameter), a derive's hole
  once, a template's interpolated literal kept a `String`, and the notes
  of `it` in a list, `result` in `ensures`, `sorted`'s references and a
  change inside a loop (R77); a literal beside a `BigInt` or a `Decimal`,
  and an explicit discriminant `E0900` where it was `E0117` (R78).
- **Dev-dependencies** (R75): a package's code imports one only by a
  `@cfg(test)` import; any other is `E0276`, in every build, so that
  `check` says it as a build without the tests would fail.
- **`expect`'s words**: the lowering of `expect` (R38) passes the
  condition's text and the message, as `assert`'s does; the words
  `panic: assertion failed:` are the run time's, which renders a recorded
  failure as `assert`'s panic (`PANIC_EXPLICIT` around `R0006` or `R0007`,
  in `sisu-interp`'s `expect_fail` and `sisu-rt`'s intrinsic). No change of
  the front end gives other words, and 17 names none, so the change is
  stage 0's: a failure `expect` records in a test is written
  `expectation failed: {expr}` (and `\n  message: {message}`), with its
  sides or parts and frames as now and without `panic: `, a catalog entry
  of its own (`run.expect-failed`, `run.expect-failed-message`); the
  test's block lists it as now, and a panic that then ends the test is
  still `panic: ...`; outside a test `expect` is `assert`'s panic, as
  decision 362 has it.
- **The manual's findings**: the items "assigned to stage 1" are settled:
  the black-box tests (R65, decision 508), a derived impl's diagnostics
  and a derive's hole (R70, R77), the habits (R72, and `d.get` here, R76),
  R54's fix in a call, a `for` header and a line's end (R67). The places
  the fifth manual round named for a spelling fix, `E0100` (`f"..."`,
  `print(n as f64)`, `for i in 0..3:`), `E0117` (`lambda`, `try:`, `go
  f()`, `//` at a line's end), `E0107` (`class`) and `E0305` (`d.get(k,
  default)`), all have it now.

One of the conformance suite's `.diag` files is blessed again, with R77:
`reflect/test_instances`, whose `E0807` and `E0381` in a test's instance
are at the test's name (its comment says so now).

Stage 1's version stays 0.7.0: nothing the driver relies on changed (a
program that imports a dev-dependency outside `@cfg(test)` is refused
with its diagnostics, as any program with an error is).

Decided against this round, from the documents: `it` in a list of
functions (02 makes `it` a closure of a call's argument only; the note
says to write the closure); the `own self` method that skips the
invariant (07 checks construction and the exits of `mut self` methods
only; checking a method that returns `Self` is a change of 07, the
owner's); `zip`'s temporaries (04: a temporary lives to the end of its
statement and `zip` borrows what it is given; `E0406`'s note says to bind
them); lowering explicit discriminants (nothing in stage 1's SIR carries
a variant's value, and `@repr`'s layouts are the backend's, so `E0900`
says so); a test's name that interpolates compile-time constants (a name
is what the runner filters and replays by; 02's interpolation is a run
time `String`).

Open: `E0335` naming the operation responsible (`print`) rather than the
function passed (naming it needs a walk of that function's checked body,
which the check of the call, in another function's body, does not have); `E0204` naming the
`@cfg(feature=...)` that left an item out (the items a `@cfg` removes are
gone before the type checker sees the module, so the removal would have
to be recorded per module); a field's `ty` in a derive's template
(`E0608`), which needs the derive's template to see the fields' types as
a generic function's reflection does.

### The ninth round: the REPL and worksheets, the stage 0 round's items, the open items

This round took what the REPL and worksheets need of stage 1, the items
the stage 0 round assigned to stage 1, the eighth round's open items and
the manual's findings that were stage 1's. RULES.md has the rules (R79 to
R94, decisions 580 to 595) and their decision texts:

- **The REPL and worksheets** (R79): `sir --repl SESSION INPUT --kept OUT
  [--load]` builds a session's program with one more input, as the
  driver's `repl.rs` writes it (an expression alone shown through
  `__repl_show`), and prints its SIR, writing the session with the input
  kept to OUT, or the diagnostics of the phase that stopped it, those in
  the input drawn in its text as `<input>`; `sir --worksheet FILE`
  builds the worksheet with each statement marked by the line it ends on
  in the file as written and its values shown, keeping `System.fake`. The
  classes `repl` and `ws` of `check.sh` run them (`cases/repl/`), the
  worksheets against `sisu ws` itself. Stage 1's version is 0.8.0: the
  driver may rely on both.
- **The stage 0 round's items**: a sized integer shifted by an amount of
  any integer type keeps its type (R80), which reverses the sixth round's
  widening of `u32 << int`, so that std can write `rotate_left` and the
  other bit methods; `expect cond, "message"` (R81); an assertion whose
  call takes a call shows its values, evaluated once (R82); the
  `@refines` test at the attribute (R83); a typed hole reached at compile
  time left to run time, which blocks what reaches it (R84); `E1022`
  followed by `D0006` or `D0132` (R85); an `edition` the compiler knows
  (R86); `sisu fmt` keeps an unused import, which `sisu lint` reports as
  `W0027` with the fix that removes it (R87: a formatter changes no
  meaning, 10); `E0402` of tasks a loop spawns that change a captured
  value and `E0428`'s note (R88); a `?` in a scope's body that a `T !`
  function returns (R89).
- **The eighth round's open items**: `E0335` names the operation
  responsible (R90), `E0204` the `@cfg` that left an item out (R91), and
  a derive's template sees a struct field's `ty` as written (R92).
- **The manual's findings**: a function named `type` (R93).

Decided against this round: removing unused imports in the formatter (09's
table says it does; 10 makes the formatter change the layout only, and the
lint's fix removes them where the writer asks); a warning of `check` for
an unused import (every program that imports what it does not use yet
would warn; `sisu lint` has it); giving a derive the resolved types of
fields (a derive runs before the types are resolved, R92 reads them as
written); `E0241`'s wording for a trait's constant ("its method `SIDES`"),
whose message is the catalog's, which the Rust front end and the manual's
transcripts share: the catalog's text is the owner's to change, `its
`{method}` cannot be used through it` being the proposal.

Open: `:why NAME` in the REPL (above, "What is next", 12); the bit
methods themselves (`count_ones`, `leading_zeros`, `rotate_left`), std's,
which the standard library writes once the Rust front end no longer
checks it (the Rust front end refuses `u64 << int`); the rest of the
manual's "Diagnostics without fixes" (`E0203`'s and `E0271`'s similar
names, `E0310` of an `i16` and a `u32`, `E0812`'s copy, and the others the
list names) and `E0005`'s invisible characters.

The conformance suite's `testing/refines.test` is blessed again, with R83:
its two failures stand at the `@refines` lines; and stage 1's
`cases/testing/expect.sir-tests`, with R82. Two items of the manual round
came late and are R94: `parse --shape` refuses what `fmt` refuses (a test's
name that interpolates, `E0132`), for the driver to route `parse --shape`
to stage 1, and the note of a template instance's diagnostic writes the
type arguments as the source does (`User`, not `__T0`).

### The tenth round: the merge's differences, the driver's REPL, the open items, purity

This round took the differences the merge of stage 0's eighth round left,
what the driver's REPL still needed of stage 1, the ninth round's open
items, purity through an iterator's adapters, and the manual's findings
that were stage 1's. RULES.md has the rules (R95 to R103, decisions 600
to 608) and their decision texts:

- **The merge's differences**: the conformance suite's three new cases of
  decision 561, `unsafe/malloc_bounds`, `unsafe/unterminated` and
  `unsafe/free_twice`, import `std.ffi` and `std.mem` and declare an
  `extern` block, which only stage 1 reads: their seven differences each
  (ast, names, sir, ids, human, doc, doc-output) are R4's, listed there,
  and their runs, interpreted and native, print the cases' `.out` and
  `.err`. The twenty-second, `std/pure_iterators`'s `sir`, is R20's: the
  program reaches a float's `Debug`, whose impl stage 1 writes otherwise.
- **The driver's REPL** (R95): `sir --repl` and `sir --worksheet` take
  `--dir DIR`, where the program stands (`embed`, the inline metadata),
  and `--name NAME`, what its diagnostics and SIR call it; `parse
  --bindings INPUT` gives what `:why NAME` reads of an input (each
  statement, what it binds, its initialiser's bytes, the names that reads,
  the names it changes), so the driver needs no parser for it. Stage 1's
  version is 0.9.0: the driver may rely on the three (RULES.md says the
  forms exactly).
- **The ninth round's open items**: a variant's fields' `ty` in a derive
  and the kinds of the program's own types there (R96); `E0203` and
  `E0271` offer the similar name (R97); `E0310` of an `i16` and a `u32`
  offers `.to_int()` (R98); `E0812` gives the copy typed as the base
  (R99); `E0005` names an invisible character, a byte-order mark among
  them (R100); and of the rest of "Diagnostics without fixes", R101.
- **Purity** (R102): decided against computing it over the monomorphised
  call graph (03 keeps `pure` a property of a function's type, and that
  graph for the resource attributes and capture kinds); `E0333` at an
  adapter notes why and names `fold` with a `pure` function.
- **The manual round's late items** (R103): `sisu lint` leaves an import
  the resolver refused to its error, and a worksheet shows an option or a
  result an expression statement gives.

R85's case: none still. A raised `unsafe` level needs a lockfile that
records a lower one, and `check.sh`'s runs write each package's lockfile
themselves, so a case would need a lockfile the run does not rewrite,
which no class of `check.sh` provides; the line after `E1022` is the
commands' own, as for `E1019`, and `main`'s unit test of `raised_level`
holds the message.

Decided against this round: purity over the call graph (R102); skipping a
byte-order mark as trivia, which would change the file's tokens and the
formatter's output (R100 names it instead); `E0241`'s wording and
`E0117`'s "unexpected a string", which are the catalog's; a fix for
`sorted` of a `list[f64]` in a library other than `impl Ord for f64` (03's
`f64.total_order()` is missing, a finding of "Types and traits").

Open: of "Diagnostics without fixes", `E0338` calling a local list "a
borrowed value" (not reproduced: `let first = queue[0]` of a `list[int]`
compiles), `sorted`'s fix in a library (above), `E0421`'s note for a lent
capability, a `TestClock` given where a `System` goes, `E0806`'s method,
a script's dependency by version (`E0220` before `E1010`), `E0237`'s
cascade, `E0501`'s place, `E0301`'s float division, `E0402` in a nested
loop, `E0258`'s associated function, a variant imported by name, the
`[trust]` order, `E0310` of a `+=` in unrolled code and `E0333`'s `D0021`
(stage 0's).

Stage 0's side: the driver's `repl.rs` passes `--dir` and `--name` to `sir
--repl` and `sir --worksheet`, and reads `:why NAME`'s inputs from `parse
--bindings`; the manual's transcripts that show the five fixes R101
renames are made again.

The conformance suite's two `.defer` files that show `type.add-copy`'s fix
are blessed again with R101: `ownership/rustc/move_out_of_vec_tail` and
`ownership/rustc/overloaded_index_move`; and so are stage 1's
`cases/findings7/derive_messages.diag` and
`cases/traits/assoc_const_refusals.diag`.

### The eleventh round: the manual's findings, coherence, conditional imports

This round took what the manual's findings left to stage 1: the tenth
round's open items of "Diagnostics without fixes", coherence for a
library's operators, conditional imports, `arbitrary[T]` under a bound,
"Types and traits" and part of "Metaprogramming", the compile-time
evaluator's speed, and two items of the stage 0 round. RULES.md has the
rules (R104 to R110, decisions 660 to 666) and their decision texts:

- **Diagnostics without fixes** (R104): `E0338` of a local list's item,
  `E0421` of a lent capability, a `TestClock` where a `System` is taken,
  `E0806`'s methods, `E1010` without `E0220`, `E0237` without its
  `E0347`s, `E0301`'s float division, `E0402` at a binding the loop makes
  anew, `E0258`'s associated function, a variant imported by name,
  `E1021` without the `E0703`s, and `E0310` naming `+=`. `E0501` is
  decided against: it is the SIR verifier's, an internal error.
- **Coherence** (R105): an impl of a foreign trait for a foreign type is
  the package's own when one of the trait's type arguments is its type,
  `impl Mul[Vector] for f64` beside `Vector`, Rust's first-local-type
  form.
- **Conditional imports** (R106): an import's `@cfg` may test the target;
  a name it would bind is not reported in an `if comptime` branch on the
  target that the build leaves out, and is `E0200` naming the `@cfg`
  elsewhere. Stage 1 checked such a branch before; the exception is for
  the names the build does not have. An import in a body is `E0277`, with
  the fix that moves it.
- **`arbitrary[T]` under a bound** (R107): drawn through the bound's impl.
- **Types and traits** (R108): `F[_]`'s note; `checked_add`,
  `checked_sub`, `checked_mul`, `round_to`, `f64.total_order()` and
  `max_by` and `min_by` over a `PartialOrd` key in the prelude, which
  `E0347` for a float names; `@derive(Plain)` asks for `@repr(c)` itself.
- **Metaprogramming** (R109): `E0235` at a derive labels the written impl.
- **The stage 0 round's items** (R110): a generic method of the prelude
  may ask `reflect` of its type parameters, and `json.decode`'s
  `Mismatch` has no article.
- **The evaluator's speed**: the finding is stale (below, "The
  compile-time evaluator").

Decided against or left: `E0501` in `sisu check`; a padded `@repr(c)`
struct under `Plain`, which waits for the layouts; `ptrsize`, `Sized`,
`Decimal`'s `/`, `graphemes`, `@derive(Into, From)`, the `Array` literal
and `FromLiteral` for `Url`; the label at the template line `comptime
for` unrolled, `@preview`'s warning and statements before `return impl`;
and the invariant of a method that takes `own self` and returns `Self`,
the owner's question (RULES.md, R108).

Stage 1's version stays 0.9.0: no form the driver relies on changed. The
package cases of the round are in `cases/packages/`, where `check.sh`
reads package cases. Blessed again: `cases/json/decode_validates.out`
(R110), `cases/run/build_stopped.release`,
`cases/packages/unsafe_levels.diag` and `.audit` (R104),
`cases/unsafe/plain_refusals.diag` (R108), and the conformance suite's
`types/integer_division.diag`, `run/stopped_by_errors.diag` and `.err`
(R104) and `std/doc_examples.select` (R108's doc examples).

### The twelfth round: the merge's differences, the REPL served, lookups by `str`, the open items

This round took the differences the merge of stage 0's tenth round left,
the REPL's cost per input (decision 641), the manual's findings that were
stage 1's, and the eleventh round's open items. RULES.md has the rules
(R111 to R119, decisions 700 to 708) and their decision texts:

- **The merge's differences** (R111): a prelude type's constants are
  named `u8.MAX` in the SIR, as stage 0 names them, where stage 1 wrote
  `prelude.u8.MAX` (`std/int_bounds`'s `sir`); `std/instant_monotonic`'s
  four formatter classes are R87's, an import the Rust formatter removes.
- **The REPL served** (R112): `sir --repl-serve` makes the base once and
  answers each request, a command's arguments separated by tabs on a
  line, on standard error, framed by the lengths of what the command alone
  prints. An input took 0.83 to 0.91 s as a process of its own and takes
  0.041 s served (the session of `cases/repl/session.repl`, natively, on
  four busy cores); the answers are the commands' byte for byte.
- **Lookups by `str`** (R113): the prelude's `Lookup[K]`, which `str`
  implements for `String` keys, lets `get`, `contains`, `remove` and a
  `dict`'s index take a `str`; `E0300` of an optional reference compared
  with an optional value offers `.copied()`.
- **The ownership checker's findings** (R114): `E0343` for a change
  through a `Shared`; the `zip` temporaries and known over-rejection 15
  decided against, with what each would take.
- **The eleventh round's open items** (R115): `@preview` warns at a use;
  `truncate_to` to 128 bits is exact.
- **`target.runtime`** (R116): `--target-runtime native|interpreter`.
- **A field's documentation** (R117): `f.doc`, which `cli.help_text`
  prints under each option.
- **`@export` at the boundary** (R118): `E0991` for a type C has no
  layout for.
- **A file beside its modules** (R119): `E0220`'s note says that a
  package makes them one program.

Decided against: the `zip` temporaries and over-rejection 15 (R114), a
directory without a manifest as a package (R119), and `Decimal` as
`Copy` (03 makes it a scaled `BigInt`), a question for the owner.

Open: the label at the template's line that `comptime for` unrolls,
statements before `return impl` (R115), `@cfg` on an import and `when`
under `--target-runtime` (R116), an item's own `##` text to reflection
(R117), and the lookup by `str` without its `String`, the runtime's.

Stage 1's version is 0.10.0: the driver may rely on `sir --repl-serve`
and `--target-runtime` (RULES.md, R112 and R116). The new cases are in
`cases/findings12/`.

### The thirteenth round: what stage 0's eleventh round asks, the base cache, over-rejection 15, the open items

This round took what stage 0's eleventh round left to stage 1 (decisions
683, 687, 690, 693 and 694), the base cache (decision 641), the call on
known over-rejection 15 and the bootstrap's comparison with stage 0, and
the twelfth round's open items. RULES.md has the rules (R120 to R126,
decisions 740 to 746) and their decision texts:

- **`@export` and transcripts** (R120): `sir` writes `export "SYMBOL"
  FUNCTION` after the entry and lowers the function as a root; `sir
  --tests` of a program with transcripts keeps `transcript_world` and
  `transcript_system`.
- **Attributes nothing checks** (R121): `@no_alloc`, `@no_block` and
  `@stack(...)` are `W0030`.
- **The runtime profile** (R122): `[package] runtime`, `full`, `single`,
  or `freestanding`, which is `E1023`; `runtime single` in the SIR after
  the entry; `target.runtime_profile`.
- **The REPL's lines** (R123): a loaded file's functions at the file's
  own lines, a typed input's at `<input>`'s.
- **Over-rejection 15** (R124): a read of a `mut` parameter is a shared
  reborrow, `ref _1.*`, so a loop over it may read it; `bootstrap.sh`
  reports how `A.sir` compares with stage 0's and fails only when the
  fixed point does not hold.
- **The base cache** (R125): its form 2 keeps which importable modules
  hold templates and `unsafe` code; a std text's lines are found when
  lowering reaches it. `check` of a two-line program: 4.64 G instructions
  before, 4.15 G after; what dominates is the parse of what the base
  declares (below, "Starting up").
- **The open items** (R126): `E0258` alone for a private field of
  another module's struct.

Decided against: a cache of the collected items themselves (R125),
`TotalF64` as a key and `Lookup`'s doc example by `str` (R126). Open: the
label at an unrolled template's line, an item's own `##` in reflection,
and `@cfg` on an import and `when` under `--target-runtime` (R126).

Stage 1's version is 0.11.0: the driver may rely on the SIR's `export`
and `runtime` lines and on `sir --tests` keeping the world of transcripts
(RULES.md, R120 and R122). The new cases are in `cases/findings13/`,
`cases/packages/` (`private_field_hole`, `runtime_single`,
`runtime_freestanding`) and `cases/repl/loaded_panic.repl`.

### The fourteenth round: the lexer's cost, the runtime of imports, the open items, the transcripts' class

This round took the lexer's cost that R125 measured, R116's open item
(imports and dependencies under `--target-runtime`), the thirteenth
round's open items, a class of `check.sh` for the transcripts' roots, and
the call on D-048's Unicode names. RULES.md has the rules (R127 to R133,
decisions 760 to 766) and their decision texts:

- **The lexer and the AST builder** (R127): punctuation and keywords are
  known by their bytes, with nothing looked up or made, and each byte is
  read once; a tree without an `ERROR` node is walked once, and an item is
  found among its siblings by a binary search. `sisu1 tokens` of the
  prelude lexes in 284 M instructions where it took 543 M (1,470 a byte
  where it was 2,800); a cached `check` of a two-line program takes 3.42 G
  where it took 4.15 G, the lexer 0.53 G of it (1.03 G) and the AST builder
  0.54 G (0.77 G). `types-batch` of `types.list` takes 5.1 s of user time
  where it took 7.6 s, `sir-batch` of `sir.list` 18.6 s where it took
  23.9 s (the same machine, the better of two runs each, at a load of 8 to
  10 on four cores). The round's last native run of `check.sh --native
  --names --types --sir` took 16 minutes, where the thirteenth round's
  took 21, on the same four cores at a like load (`types-batch` 7.2 s,
  `sir-batch` 24.1 s, `doc-batch` and `check-batch` as before).
- **The runtime of imports and dependencies** (R128): an import's
  `@cfg(target.runtime == ...)` and a dependency's `when = { runtime =
  ... }` read the runtime `--target-runtime` names.
- **Documentation to reflection** (R129): `reflect.doc_of(T)`, a variant's
  field's `f.doc`, and `cli.about[T]()`, a struct's own comment, which a
  tool prints before `cli.help_text`.
- **Checks before `return impl`** (R130): a derive's template may begin
  with `comptime assert`, `comptime panic` and `comptime if` of them.
- **The template's line** (R131): an error of an instance placed at its
  call labels the template's statement that wrote its line.
- **The transcripts' class** (R132): `check.sh`'s `transcripts` compares
  the SIR of `testing/shop`, built with its tests, with stage 0's and runs
  its transcripts through the driver from stage 1's SIR.
- **Unicode names** (R133): NFC and the confusable and mixed-script lints
  decided against for this round, with what their tables would take.

Decided against: a `let` before a derive's `return impl` (R130), and
D-048's names and lints for this round (R133). Open: the parser's own
cost (0.91 G of the two-line `check`, below, "Starting up"), `sisu doc`'s
rendering of an example with its values (below, "What is next", item 1),
and the lookup by `str` without its `String`, whose intrinsics are stage
0's next round's (decision 729).

Stage 1's version stays 0.11.0: the driver relies on no new form (it
already passes `--target-runtime`, which now reaches imports and
dependencies too). The new cases are in `cases/findings14/` and
`cases/packages/runtime_import.pkg`.

### The fifteenth round: lookups by `str`, the parser's cost, Unicode names, the planned helpers, names despite a parse error

This round took the lookup by `str` without its `String` (decisions 729
and 784), the parser's own cost that R127 left, D-048's names and lints,
the library's planned helpers, G4's names despite a parse error, `sisu
doc`'s values and the call on a public type's `Debug`. RULES.md has the
rules (R134 to R141, decisions 800 to 807) and their decision texts:

- **Lookups by `str`** (R134): `get`, `remove` and the index of a
  `dict[String, V]` and `contains` and `remove` of a `set[String]` by a
  `str` pass the `str` to decision 784's intrinsics. 30,000 lookups take
  370 M instructions on the interpreter before, 260 M after; natively they
  make 31 allocations where they made 30,031, but take 124 M instructions
  where they took 98 M, since the backend lowers `dict.get` in place and
  the `_str` intrinsics through the runtime's dispatch.
- **The parser's cost** (R135): the lines from the tokens, fewer calls a
  token. The prelude's parse takes 353 M instructions where it took 464 M;
  a cached two-line `check` 3.23 G where it took 3.43 G, the parser 0.70 G
  of it (0.91 G).
- **Unicode names** (R136): names in NFC, a combining mark in a name, and
  `W0031` and `W0032`, from tables `tools/unicode_tables.py` generates
  (`src/unicode_tables.sisu`, 821 lines), which `check.sh` keeps fresh.
- **The planned helpers** (R137): `checked_add`, `checked_sub`,
  `checked_mul` and `checked_pow` of the sized integers, `int.checked_pow`,
  `Result.or_else`, `Iter.try_map` and `list[Result[T, E]].try_collect`.
- **Names despite a parse error** (R138): a body with a syntax error
  reports the names outside its broken statements, and none that they
  write.
- **`sisu doc`'s values** (R139): decided against for the round.
- **A public type's `Debug`** (R140): every type keeps it; 10's sentence is
  changed in the decision text.
- **The merge's differences** (R141): the conformance suite's
  `runtime_single`, whose package `check.sh` now lowers with the base of
  its own profile.

Decided against: `sisu doc`'s values of an example (R139), a string
literal's key and `dict.contains` by `str` without their `String` (R134),
and the trials of R135. Open: the backend's in-place lowering of the
`_str` intrinsics (stage 0's), `try_collect` after an adapter's `map`
(R137), the borrow of a `str` index by `dict.index_str` (R134), a batch's
base for each runtime profile (R141), the confusables and scripts of
Unicode's own files (R136), and the
profile stage 1 is built with (R135: the release profile parses the
prelude in 373 M instructions, the debug profile in 988 M).

Stage 1's version stays 0.11.0: the driver relies on no new form. The new
cases are in `cases/findings15/`.

### The sixteenth round: a test's module, reflection's errors and functions, `try_collect` after `map`, the resolver's cost, Hangul

This round took decision 828's request (each test's module in `sir
--tests`), the manual's fourteenth round's findings of reflection and of
`E0607`, the framework of 08's reflection, R137's and R134's open items,
the resolver's and the trait check's cost, and the derives of 03 with
R136's Hangul. RULES.md has the rules (R142 to R149, decisions 840 to
847) and their decision texts:

- **A test's module** (R142): `sir --tests --modules` writes ` in
  "FILE"` on each `test` and `bench` line after its function, so that the
  driver can run one module's tests (`sisu test src/MODULE.sisu`); `sir
  --tests` alone is unchanged, since stage 0's SIR reader drops a `test`
  line's tags after a word it does not know. Stage 1's version is 0.12.0.
- **Reflection's errors** (R143): `reflect.name_of(int)` and the other
  scalar types in a function that takes no type parameter, and one
  `E0609` at a query's argument that compile time knows no type by, where
  it was `E0900` with an `E0200` in the instance's module.
- **`E0607` alone** (R144): no `E0243` at its `@derive`s, no `E0351` or
  `E0304` where the functions it would have written are used.
- **The framework's reflection** (R145): `reflect.functions_of(module)`,
  each function with its name, parameters, result type, attributes and
  comment, written in run-time code as the function itself, so a table of
  routed functions is made at compile time.
- **`try_collect` and the borrowed index** (R146): `Map[I, Result[T,
  E]].try_collect`, appended to the prelude, and a `dict`'s index by a
  `str` that is only borrowed read through `dict.index_str`.
- **The resolver's cost** (R147): a derived impl found among its trait's
  impls, and each trait's impls kept in place. A cached two-line `check`
  takes 3.34 G instructions where it took 3.39 G, the trait check 0.10 G
  where it took 0.15 G; resolving stays 0.38 G, spread over the base's
  signatures with no site above 3 % of the command.
- **Hangul, and `@derive(Into, From)`** (R148): jamo compose into
  syllables in a name's NFC; the derives decided against, since the
  prelude has no `From`, `Into` promises a lossless conversion and a
  derive's template takes no trait argument.
- **The merge's differences** (R149): the conformance suite's
  `unsafe/cstring`, which differs from the Rust front end as the other
  cases of `unsafe/` do.

Decided against: `@derive(Into, From)` (R148), and a `test` line's
module written by default (R142). Open: `try_collect` after `Filter`,
`Take` and the other adapters, whose items are their inner iterator's
(R146); a compile-time table of function values written `comptime
[...]`, which the evaluator after lowering cannot keep (R145); an
attribute's arguments as values rather than their text (R145); the
resolver's spread cost (R147, below, "Starting up"); and the driver's
reading of ` in "FILE"` and the reader's (stage 0's, R142).

Stage 1's version is 0.12.0: the driver may rely on `sir --tests
--modules` (RULES.md, R142). The new cases are in `cases/findings16/`,
`cases/packages/test_modules.pkg` and `cases/packages/routed_functions.pkg`.

### The seventeenth round: stage 1's builds in the release profile, compile-time function values and attribute arguments, the index's panic, the small types of 03

This round took decision 860's notes for stage 1 (its own builds in the
release profile), the sixteenth round's open items of reflection, the
manual's fifteenth round's missing key, the narrowed "missing beside 03"
item, R146's open item, the first step of compile-time types and
`Decimal`'s quotient. RULES.md has the rules (R150 to R157, decisions
880 to 887) and their decision texts:

- **The release profile** (R150): `sisu1` runs the driver's build of
  stage 1 and asks the driver for it, so the two no longer build over
  each other; `check.sh --native`'s batches run it, and `self` and
  `fixed` a debug stage1-A (below, "Where the time goes").
- **Function values and attribute arguments** (R151): a table of
  function values written `comptime [...]` is kept as the constants that
  name them, and `reflect.attributes_of` gives each argument as a value,
  with `keys` for their keywords.
- **The index's panic** (R152): a `dict`'s entry read inside an
  expression is read at the index, so a missing key's panic names the
  index for a literal key too.
- **The small types** (R153): `ptrsize`, `uptrsize` and `Sized` in the
  prelude, a list literal where an `Array[T, N]` is expected, and
  `FromLiteral[str] for Url`.
- **`try_collect` after every adapter** (R154): decided against; the
  chain ends in its `map`, or in `map(r -> r)`.
- **Compile-time types** (R155): the plan, a `comptime def ... -> type`
  and a type position written `comptime f(args)`, whose syntax waits for
  the owner.
- **`Decimal`'s `/`** (R156): decided against for the round, until a
  `Ratio` of `BigInt`s.
- **The merge's differences** (R157): the conformance suite's
  `unsafe/export_by_pointer`.

Decided against: `try_collect` after the other adapters (R154),
`Decimal`'s `/` giving a `Ratio` (R156), and `@derive(Into, From)`, the
owner's Q-80, left. Open: compile-time types and `@diagnostic_name`
(R155), a closure in a `comptime` result (R151), and the manual's
reference pages that show an attribute's arguments as text (R151) and
the missing key at `4:5` (R152).

Stage 1's version stays 0.12.0: the driver relies on no new form. The new
cases are in `cases/findings17/` and `cases/packages/routed_comptime.pkg`.

### The eighteenth round: a server for every command, an instance's syntax error, closures at compile time, members' attributes, the normalisation forms, `BigRatio`

This round took the base's cost of every command but the REPL's (R112's
server), two findings of the manual's sixteenth round, R151's open item
of closures, the narrowed framework finding of members' attributes, and
the "missing beside 03" item's normalisation forms, graphemes and
`Decimal`'s `/`. RULES.md has the rules (R158 to R163, decisions 920 to
925) and their decision texts:

- **A server for every command** (R158): `sir --repl-serve` answers any
  `sir`, `check`, `parse`, `fmt`, `lint`, `doc` or `tokens` request, run
  in the directory its `--dir DIR` names, with R112's frame; the base is
  made again when a std text, the catalog or the request's runtime
  changed, and a server whose executable was built again answers `stale`
  and ends. A `check` of a two-line program takes 0.009 s served where it
  takes 0.22 s as a process (below, "Where the time goes"). Stage 1's
  version is 0.13.0.
- **An instance's syntax error and `E0609`'s words** (R159): a syntax
  error in a template's body is reported once, in the template's file, not
  again for each unrolled copy; compile-time code of a function without
  type parameters that stops is `E0616`, "the compile-time code of `main`
  stopped", where it was `E0609`'s "making `main`".
- **Closures at compile time** (R160): a closure in a `comptime` result
  that captures nothing or only constants is kept; one that captures more
  is `E0604` with a note on what a result keeps.
- **Members' attributes** (R161): `f.attributes` of a field and a variant,
  `reflect.attributes_of(m)` of a method, and `x.[m]` of a method, so a
  template routes by a method's `@route`.
- **The normalisation forms and graphemes** (R162): `std.string`'s
  `Normalize` (`nfc`, `nfd`, `nfkc`, `nfkd`) and `Graphemes`
  (`graphemes()`) for `str`, from tables `tools/unicode_tables.py`
  writes; the forms agree with Python's for 18,464 texts.
- **`BigRatio`** (R163): an exact fraction of `BigInt`s in `std.num`, and
  `Decimal`'s `/`, which gives one.

Decided against: keeping a closure that captures a tuple, a struct or a
list (R160). Open: the driver's side of the server (one server per
working directory, its lifetime, reading `stale`), which is stage 0's;
UAX #29's `Prepend`, conjunct and pictograph rules (R162); one `Ratio`
of any size in place of `Ratio` and `BigRatio`, the owner's (R163); and
compile-time types (R155) and the sharing of instances with
`--explain-size`.

Stage 1's version is 0.13.0: the driver may rely on the requests of R158.
The new cases are in `cases/findings18/`.

### The nineteenth round: the language server's questions, `eval`, a template's builtins, the graphemes' gaps

This round took the Rust front end's last role in `sisu lsp`, 08's
configuration files, a finding of the manual's seventeenth round and
R162's open item, and the merge's case `std/slices`. RULES.md has the
rules (R164 to R168, decisions 960 to 964) and their decision texts:

- **The language server's questions** (R164): `query diagnostics`,
  `symbols`, `hover`, `definition`, `references` and `completions` of a
  file and a position, answered from stage 1's check of the program that
  holds the file, a line per item of tab-separated fields, a file with a
  syntax error answered where its tree allows, `--text` for an editor's
  buffer; served, a hover takes 0.009 s (above, "Where the time goes").
  `check.sh`'s new class `served` asks a case's `.requests` through the
  server against its `.answers`.
- **`eval`** (R165): a configuration module's `const` items evaluated by
  the compile-time evaluator and written as JSON, TOML or YAML (`--to`, as
  08 spells it), a constant whose evaluation stops being that error at
  its name and one the form cannot write the new `E0617`.
- **A template's builtins** (R166): `range`, `zip`, `reversed`, `sorted`,
  `min`, `max`, `sum` and text's `to_lowercase`, `to_uppercase`,
  `to_string` and `byte_len` in a template's compile-time code, so
  `comptime for i in range(len(a.args))` unrolls; `E0609`'s reason lists
  them.
- **The graphemes' gaps** (R167): decided against for the round; Python's
  `unicodedata` has no `Grapheme_Cluster_Break`, `Indic_Conjunct_Break` or
  `Extended_Pictographic`, and RULES.md records what a table from
  Unicode's own files would add.
- **The merge's differences** (R168): the conformance suite's
  `std/slices`, which imports stage 1's `std.mem`.

Decided against: rendering `eval` through each type's `Encode` (R165), and
UAX #29's rules beyond R162's tables (R167). Open: the driver's routing of
`sisu lsp`'s requests and of `sisu eval` to stage 1, which is stage 0's;
the questions the driver keeps (selection ranges, inlay hints, inline
values); `max` and `min` of the prelude in a constant `eval` evaluates,
which stage 1's evaluator does not run (an intrinsic it lacks); `W0022`
for an attribute that a template reads only through `f.attributes`; and
compile-time types (R155).

Stage 1's version is 0.14.0: the driver may rely on `query` and `eval`
(R164, R165). The new cases are in `cases/findings19/` and
`cases/packages/query_modules.pkg`.

### The twentieth round: the language server's other answers, the prelude's names, `W0022`, a slice of a slice, the evaluator's prelude, `check --holes`, `sisu run` served

This round took what the driver's language server still answered from
the Rust front end, the manual's finding on the prelude's width and its
eighteenth round's two, R165's and R164's open items, 10's `check
--holes`, and decision 981's served `sisu run`. RULES.md has the rules
(R169 to R175, decisions 1000 to 1006) and their decision texts:

- **The language server's other answers** (R169): `query rename`,
  `actions`, `inlay`, `selection`, `format` and `holes`, and the names
  of import lines among a definition's references, so that `sisu lsp`
  can answer every request but the worksheets' values from stage 1.
- **`sisu run` served** (R170): `build --for-run --emit-sir` writes the
  SIR the build would hand its backend on standard output, with the
  build's diagnostics, and the server answers it.
- **`W0022` and members' attributes** (R171): `f.attributes` and
  `v.attributes` are code that reads attributes.
- **A slice of a slice** (R172): what a call borrows through a reference
  to a `Slice` is what the slice borrows, so `towels.as_slice()[:2]`
  borrows `towels`.
- **The evaluator's prelude** (R173): `min`, `max`, `abs`, `any`, `all`,
  `sorted`, `enumerate`, `zip` and `reversed` in a constant.
- **`check --holes`** (R174): each typed hole with its type, its
  function, the bindings in scope and those of its type.
- **The prelude's visible names** (R175): the prelude marks `pub` the
  names 09 and 11 give it, a program that names one of its 43 helpers is
  `E0278`, and completions offer the `pub` names.

Decided against: hiding `Formatter`, `Spec` and `Hasher`, which the
traits' methods take, and `iterable_list`, which a conformance case calls
(R175), and 10's report of a hole beyond its type
and bindings (R174). Open: the driver's routing of the new queries, of
`check --holes` and of `sisu run` through R170, which is stage 0's; the
worksheets' inline values and `sisu.explainBorrow`, which the driver
keeps; a hole's types, contracts, tests and fits (R174); a keyword
argument's name among a field's references; and compile-time types
(R155).

Stage 1's version is 0.15.0: the driver may rely on the queries of R169,
the request of R170 and `check --holes` (R174). The new cases are in
`cases/findings20/`.

### The twenty-first round: `fix`, constants at compile time, the whole hole report, keyword arguments, TOML's tables, `Plain`'s padding, `days_in_month`

This round took 10's `sisu fix` and `@fix_inline`, the manual's
nineteenth round's two findings (a constant computed only at run time,
and TOML's one line of inline tables with `E0617` at the constant),
R174's and R169's open items, the manual's padded `Plain` struct, and
R175's decided-against home for `days_in_month`. RULES.md has the rules
(R176 to R184, decisions 1040 to 1048) and their decision texts:

- **`fix`** (R176): `fix FILE [--dry-run] [--all]` applies each
  diagnostic's preferred fix, with `--all` each lint's, checks the text
  again until no fix applies or a round would undo another, says each fix
  it applied, and writes the file once; `check.sh`'s new class `fix` runs
  it as a process on a copy of each case with a `.fixed`.
- **`@fix_inline`** (R177): a call of such a function warns as a
  deprecated one's, with the fix that writes the expression its one
  `return` gives in the call's place, where that means what the call
  meant; any other body is `W0033`.
- **Constants at compile time** (R178): every command that checks a
  program evaluates its constants, and a failed invariant, panic,
  refinement or budget is an error at the constant's name; the SIR still
  computes the value where the program reads it.
- **Keyword arguments** (R179): `rows` of `Bed(rows=2)` is a reference of
  the field, which `references` lists and `rename` renames.
- **TOML's tables and `E0617`'s field** (R180): a struct or a mapping is a
  table, a list of them an array of tables, after the inline values; the
  error names `LIMITS.retry.backoff`.
- **The whole hole report** (R181): the expressions that fit to depth
  two, the definitions of the types the hole's type names, the contract
  in force, the tests that reach it, and `--format=json`.
- **`Plain`'s padding** (R182): a `@repr(c)` struct whose fields C's
  layout leaves a gap between or after cannot derive `Plain` (`E0279`,
  `E0280`).
- **`days_in_month`** (R183): `std.time` has it as a `pub` function, and
  `E0278`'s fix imports it.
- **The merge's differences** (R184): the conformance suite's
  `std/bytes`, `std/narrow` and `unsafe/narrow_pointer`, which differ from
  the Rust front end under R48, R152 and R4.

Decided against: 10's ownership-aware inliner of a body of several
statements, methods and `--deps` (R177); the values a test sees at a hole,
which need the tests run (R181). Open: the driver's routing of `sisu fix`
and `sisu check --holes --format=json`, which is stage 0's; `--edition`
and `--deps`; a name in a struct pattern among a field's references; and
compile-time types (R155).

Stage 1's version is 0.16.0: the driver may rely on `fix` (R176), the
holes' JSON form and `query holes`'s four new fields (R181). The new cases
are in `cases/findings21/`.

### The twenty-second round: the Rust front end's remaining roles, a named function in `filter`, three diagnostics' words, a pattern's fields, `Decode`'s enum field

This round took the roles port-debt.md's "The Rust front end's roles still
to replace" leaves to stage 1, as commands of stage 1 in the driver's
forms, and the manual's findings in "The standard library" and
"Diagnostics without fixes", R179's open item and the coordinator's G7.
RULES.md has the rules (R185 to R194, decisions 1080 to 1089) and their
decision texts:

- **`sir --tests --std` and `test --std`** (R185): the standard library's
  doc examples are tests, `std.core/zip (doc example)`, in stage 0's order
  and before the program's own. There are 197 of them, each once, where
  the Rust front end lists `std.net`'s two twice, and all pass from stage
  1's SIR.
- **`check --diary PATH`** (R186): the driver's `check`, `diagnostic` and
  `resolved` records, with the texts kept by hash in `PATH.d/`. A diary
  either front end writes reads the same to the other, but for the
  toolchain.
- **`explain-borrow FILE:LINE` and `query explain`** (R187): the loans live
  on a line, from stage 1's ownership checker, in the driver's words.
- **`audit --ownership` and `audit --caps`** (R188): the driver's lines,
  from stage 1's tokens and SIR. They match on the corpus but where a rule
  of RULES.md changes a case's errors.
- **`--defer-borrow`** (R189): `sir` and `build --for-run --emit-sir` print
  the SIR of a program whose only errors are the ownership rules', with
  its diagnostics on standard error and status 3.
- **A named function in `filter`** (R190): its fix, the closure that calls
  it, now compiles for a `str` parameter too. Accepting the function
  itself is decided against.
- **Three diagnostics' words** (R191): `E0117`'s kind of token without its
  article, `E0241`'s member, and one `E0333` per chain of adapters.
- **A struct pattern's field names** (R192) are references of the field.
- **`Decode` of a struct with an enum field** (R193) is `E0608` at the
  derive, where the run stopped at `E0501`. The instance that failed was
  asked from the derived module, whose errors the program does not show.

Decided against: a function value of another parameter type in `filter`
(R190), and narrowing methods beside `truncate_to` (R194). `D0181` and
`D0183`, which nothing named whole, are now named by the hole report
(R181's note). Open: the driver's routing of these commands, which is stage
0's; the `.select` of `std/doc_examples`, which is blessed again once the
driver sends `test --std` to stage 1; compile-time types (R155).

Stage 1's version is 0.17.0. The driver may rely on `sir --tests --std`,
`check --diary`, `explain-borrow` and `query explain`, `audit --ownership`
and `--caps`, and `--defer-borrow`, each served. The new cases are in
`cases/findings22/`.

### The twenty-third round: a served `sisu build`, worksheets' inline values, three of the manual's findings, `@fixture`, the parser's grammar

This round took what decision 1062 left stage 1 to add for a served
`sisu build`, the last role of the language server on the Rust front end
(decision 1060), the manual's twenty-first round's findings of
`@fix_inline`, `Plain`'s padding and `sisu fix`, 17's fixtures, R108's
sharing of instances and the manual's R20. RULES.md has the rules (R195 to
R203, decisions 1120 to 1128) and their decision texts:

- **`build --emit-sir`** (R195): without `--for-run` it is the native
  build stopped where it hands its backend the SIR, `--native` implied,
  with `--release` and `--contracts`; each program's SIR follows a line
  `# executable PATH`, a comment of the SIR, which R170's and R189's
  answers now carry too; the server answers it and a `build` without
  `--native`; `check --holes` takes a package's directory or none.
- **`query values FILE`** (R196): a worksheet's values and printed lines
  by line, from the program `sir --worksheet` writes run by stage 1's
  evaluator, which keeps what it prints and marks where it stops
  (`stopped`, `panicked`).
- **`@fix_inline`'s parentheses** (R197): only where precedence needs
  them, so `to_celsius(sauna + 4.0)` becomes `celsius_of(sauna + 4.0)`;
  an argument such as `half - 1` is no longer taken for a negative
  number.
- **`E0279` and `E0280`** (R198): the gap's offsets name its last byte,
  and the note's fields are numbers, `_pad1: u8, _pad2: u16`, which are
  `Copy` where `Array[u8, 3]` is not.
- **`fix`'s status** (R199): 1 with `D0190` when errors without a fix are
  left.
- **`@fixture`** (R200): a test's parameter named after a fixture of its
  module is a `let` of the fixture's call at the start of its body, torn
  down by its drop, so the runner needs nothing new; `E0281` for a fixture
  that takes something.
- **`grammar`** (R202): the parser's productions from its `##!` comments,
  which now hold the declarations and statements they lacked.
- **The merge's differences** (R203): `holes/typed_holes`'s SIR, under
  R27.

Decided against: 17's `Expectation`, which needs a reflected expression
08 does not have (R200), and `--explain-size`'s instances per package,
which needs the SIR to name who requests each instance (R201). Open:
`scope="module"`, which waits for the runner (R200's "What stage 0 must
do"); the driver's routing of `sisu build`, `sisu run --native` of a
package and the inline values, which is stage 0's; compile-time types
(R155). The merge brought the conformance cases `holes/typed_holes`, whose SIR
differs under R27 (R203), `holes/holes_refused` and `eval/usage`.

Stage 1's version is 0.18.0. The driver may rely on `build --emit-sir`
with its executable's line, the same line before `--for-run --emit-sir`'s
SIR, `query values`, `fix`'s status 1, and `check --holes` of a package,
each served. The new cases are in `cases/findings23/`, with
`cases/packages/served_build.pkg` and `cases/repl/values.ws.sisu`.

### The twenty-fourth round: `Decode` through a field's own impl, Unicode's own data files, the tz database, `sisu doc --digest`, mutants, module fixtures

This round took the manual's findings of `Decode` (G7), of the lints'
tables and the grapheme rules, and of `Zone.named` (G9), 10's digest,
17's mutants and R200's `scope="module"`. RULES.md has the rules (R204 to
R210, decisions 1160 to 1166) and their decision texts:

- **`@derive(Decode)` through a field type's own impl** (R204): a field
  of an enum with an `impl Decode` of its own decodes with it, `E0608`
  only where the enum has none; `Encode` had no such gap.
- **Unicode's own data files** (R205): `tools/unicode_tables.py` reads
  Unicode 15.1.0's `GraphemeBreakProperty.txt`, `DerivedCoreProperties.txt`,
  `emoji-data.txt`, `Scripts.txt` and `confusables.txt`, fetched by
  pinned URLs into `compiler/target/unicode/`. `graphemes()` follows
  every rule of UAX #29 and passes `GraphemeBreakTest.txt` whole, 1,187
  lines, so `"a\u{200d}b"` is two graphemes and `"\u{915}\u{94d}\u{937}"`
  one; `W0031` reads `Scripts.txt` and `W0032` UTS #39's skeleton over
  the 5,147 confusables a name can hold.
- **The tz database** (R206): `tools/tzdata.py` writes the history of
  `Zone.named`'s zones from the system's TZif files (tz 2026c), 50 zones
  and 3,545 changes in 50 KB of `std/time.sisu`, so a past date has the
  offset its zone's clocks showed; the whole database (447 zones, 278 KB)
  and `std.x.tzdata` are left out.
- **`sisu doc --digest`** (R207): the pages as one plain text, each
  item's declaration with `##` lines of its first paragraph,
  capabilities, holes and first example; the standard library's is 147
  KB, about 37,000 tokens. `--open` is the driver's.
- **Mutants** (R208): `sir --tests --mutants` prints each mutant of the
  program's own code, a boundary moved, an operator flipped, a negation
  or a statement removed, as a line `# mutant K: FILE:LINE:COL: WHAT` and
  its SIR, with `print`, `eprint` and `log` left alone; `--mutant=K` one.
  The runner's half is stage 0's.
- **`@fixture(scope="module")`** (R209) keeps R200's lowering: the SIR has
  no state of a module and only the runner knows a module's first and
  last test; what the runner must do is in the rule.
- **The merge's differences** (R210): none: the worktree is at stage 1's own head, and every difference `check.sh` reports is a rule's.

Decided against: a module-level cell for a module's fixture (R209),
`std.x.tzdata` and the whole tz database (R206), and `--open` (R207).
Open: the runner's half of `--mutate` and of `scope="module"`, and the
driver's routing of `doc --digest`, which are stage 0's; compile-time
types (R155). Found on the way and left: a program's own function named
`check` with a contract is what the contract's generated test calls in
place of the prelude's `check` (`E0305` in `<arbitrary>/main.sisu`).

Stage 1's version is 0.19.0. The driver may rely on `doc --digest` and
`doc --std --digest`, with `--output DIR`'s one file, and on `sir --tests
--mutants` and `--mutant=K` with their lines, each served. The new cases
are in `cases/findings24/`.

### The twenty-fifth round: the driver's three asks, `E0702`'s words, `check` shadowed, the grammar against the parser, `@no_alloc`, `--cover`'s regions, fixtures on the test lines

This round took what decisions 1104, 1106 and 1147 ask of stage 1, the
manual's findings of `E0702`, R20's grammar and `@no_alloc`, the
twenty-fourth round's `check` shadowed in a contract's test, and 17's
`--cover`. RULES.md has the rules (R211 to R219, decisions 1180 to 1188)
and their decision texts:

- **The driver's three asks** (R211): `build --for-run --emit-sir
  --defer-borrow` writes the errors that stop it in the human form;
  `query explain FILE LINE --ranges` ends each loan's line with its start
  and last use as ranges, `21:21-21:22`; and each deferred finding is
  followed by a line `  function NAME`, its SIR function.
- **`E0702`'s words** (R212) name a module's `@fixture` among what a
  test's parameter may be, and a fixture `E0281` refuses is that error
  alone.
- **`check` shadowed** (R213): the tests the compiler writes call the
  prelude's `check` and `within_ulps` by names bound in the module it
  writes, `__prelude_check`.
- **The grammar** (R214): the `##!` comments say what the parser reads
  of an attribute's dotted name, an error set's generic parameters, the
  members of an enum, an error set, an impl, a trait and an `extern`
  block, every form of a type and a trait's constant; two cases write
  each form.
- **`@no_alloc`** (R215) is checked on the program's SIR through its call
  graph, the standard library's functions included: `E0282` at a
  statement that makes, grows, copies or boxes what owns memory, or calls
  a function that may, or through a function value; the paths that only
  panic are exempt.
- **`--cover`** (R216): `sir --tests --cover` prints after the SIR a
  line `# region FILE:SPAN KIND FUNCTION bbB:S` for each statement and
  branch of the program's own code; the runner's half is stage 0's.
- **The merge's differences** (R218): `unsafe/tuple_pointer`, under R4
  and R5.
- **Fixtures on the test lines** (R219): `sir --tests
  --module-fixtures` writes ` fixture NAME FUNCTION test` and ` fixture
  NAME FUNCTION module K` on a test's line and lowers a module's fixture
  as the test's `ref T` parameter K; such a fixture returns a `Sync`
  value (`E0283`).

Decided against: `sisu doc`'s examples with their values (R217), which
need an example built and instrumented as a worksheet is and an
evaluator that runs against the fakes. Open: the runner's halves of
`--cover`, of `--mutate` and of a module's fixture (R216, R208, R219),
stage 0's; `@no_alloc` at each instantiation of a generic function
(R215); compile-time types (R155).

Stage 1's version is 0.20.0. The driver may rely on the human form of
`build --for-run --emit-sir --defer-borrow`'s stopping errors, `query
explain --ranges`, the line `  function NAME` after each deferred
finding, `sir --tests --cover` with its lines, and `sir --tests
--module-fixtures` with its groups, each served. The new cases are in
`cases/findings25/`.

### The twenty-sixth round: a memo between a server's requests, the examples alone, `@no_alloc` at each instance, the transcripts' world, `test --std`'s speed, D-060's allocators planned

This round took what decision 1200 asks of stage 1 (a save of `sisu run
--watch` as fast as the Rust front end's kept queries), decision 1144's
leftover, R215's open item, port-debt.md's transcripts, `test --std`'s
3.1 s and the manual's allocators. RULES.md has the rules (R220 to R226,
decisions 1220 to 1226) and their decision texts:

- **A memo between a server's requests** (R220): the server's base parses
  every std module a program may import once, for the resolver and the
  name pass; each module's parse is the name pass's and the checker's,
  and the server keeps it while its text is unchanged; the standard
  library's bodies as a request typed, lowered and placed them are kept
  for the next request, as a batch keeps them for its next program; and
  `build` takes its SIR from the one check that made its report, without
  copying the served base. A save of `live/swap` takes MEASURE_SWAP
  served where it took MEASURE_SWAP_BEFORE; each answer is a fresh
  request's bytes.
- **The examples alone** (R221): `sir --tests --std` and `test --std`
  naming no path outside a package lower and run the standard library's
  doc examples as the tests of an empty `examples.sisu`.
- **`@no_alloc` at each instance** (R222): a generic `@no_alloc`
  function is checked as written for what allocates whatever its
  arguments are, and at each instance the program makes for what depends
  on them, `total[Big]` named with a note where the program makes it.
- **The transcripts' world** (R223): `sir --tests --transcripts` keeps
  `transcript_world` and `transcript_system` on the driver's word; the
  round found the driver already runs every transcript from stage 1's
  SIR but for its fallbacks, which the option lets it delete.
- **`test --std`'s speed** (R224): the examples' modules are made once
  for a check, where each round of resolution made them again.
- **D-060's allocators** (R225), planned in five steps; the first,
  the defaulted parameter on the prelude's collections, decided against
  for the round, since it changes the prelude stage 0's agent shares.
- **The merge's differences** (R226): none; the conformance suite's `testing/mutants` differs in nothing.

Decided against: a memo of the resolver's, the checker's, lowering's and
the ownership checker's results for the unchanged modules (R220), which
need each body's dependencies on the other modules' declarations, which
no phase records; and step 1 of the allocators for the round (R225).
Open: the runner's halves of `--cover`, `--mutate` and a module's
fixture (R216, R208, R219), stage 0's; compile-time types (R155); the
allocators' steps (R225).

Stage 1's version is 0.21.0. The driver may rely on `sir --tests --std`
and `test --std` with no path outside a package, their first source
`examples.sisu` (R221), and `sir --tests --transcripts` (R223), each
served; the memo (R220) changes no answer. The new cases are in
`cases/findings26/`.

## Module layout

| Module | Lines | What | Stage 0 original |
|---|---:|---|---|
| `syntax.kinds` | 547 | `SyntaxKind` (246 kinds), the keyword and punctuation tables, kind predicates, names in dumps | `sisu-syntax/src/kinds.rs` |
| `syntax.tree` | 357 | `Token`, the arena tree, the builder with checkpoints, the dumps, the reprint, stable identities | `sisu-syntax/src/tree.rs` |
| `diag` | 1816 | `Msg` (the catalog identifiers stage 1 reports), diagnostics with labels, fixes and notes, the catalog reader and its templates, positions, `render_ids`, the human form (`render_human_files`: source lines, labels, notes and fixes), `quote`, the stable sort and `sort_unique` | `sisu-diag` (the parts the front end uses, and `render.rs`) |
| `text`, `text.unicode` | 369 | UTF-8 decoding, `is_alphanumeric`, Rust's `Debug` quoting, and the Unicode tables it needs (pinned, Unicode 17.0.0) | the host's `char` methods and `{:?}` |
| `lexer` | 262 | `lex`, and stage 0's lexer tests | `sisu-lexer/src/lib.rs`, `tests.rs` |
| `lexer.scan` | 567 | the raw scanner: every byte a token, strings and interpolation as tokens | `sisu-lexer/src/scan.rs` |
| `lexer.layout` | 347 | Python's layout algorithm, continuation, bracket recovery and its fixes | `sisu-lexer/src/layout.rs` |
| `parser` | 516 | `parse`, the diagnostic filters (bracket echoes, structure echoes, one per line), and stage 0's parser tests | `sisu-parser/src/lib.rs`, `tests.rs` |
| `parser.state` | 407 | the parser's state: looking, attaching tokens with their trivia, reporting, the fixes added later, Python's conditional | `sisu-parser/src/parser.rs` |
| `parser.items`, `.stmts`, `.exprs`, `.patterns`, `.types` | 2341 | one function per rule of the grammar | the files of the same names |
| `parser.habits` | 162 | the fix that writes Sisu's spelling where a parse error stops at another language's (RULES.md, R54) | — |
| `syntax.ast` | 775 | the AST: items, statements, expressions, types and patterns in arenas, referred to by id | `sisu-syntax/src/ast/mod.rs` |
| `syntax.ast.build` | 2172 | `build.module`, the AST of a tree as a total function, and the decoding of literals | `sisu-syntax/src/ast/build.rs` |
| `syntax.ast.dump` | 1454 | the AST in the notation of Rust's derived `Debug`, laid out as `{:#?}`, for `check.sh` | Rust's `{:#?}`, which `sisu parse --ast` prints |
| `syntax.ast.visit` | 311 | the expressions of items, blocks and expressions, each before the ones inside it | `sisu-syntax/src/ast/visit.rs` |
| `resolve` | 2883 | the names of a program of one module or of several modules and packages: declarations, imports, scopes, the prelude's and std's names, and the name diagnostics, with the names 02 enforces (R12), the error sets that hide the prelude's (R15) and a name declared by assignment (R16) | `sisu-resolve/src/collect.rs` and `types.rs`, and the name lookups of `sisu-typeck` |
| `toml`, `manifest`, `package` | 2279 | the TOML subset of manifests, the manifest's tables and their diagnostics, and a program's layout: its packages, their dependencies and trust, the module trees, and where its lockfile goes | `toml.rs`, `manifest.rs` and `package.rs` of `sisu-driver` |
| `cfg` | 290 | conditional compilation: the items, test items, methods, fields and variants a build leaves out by their `@cfg`s (R44), with `not(...)` (decision 250), and the problems of the conditions | `sisu-driver/src/cfg.rs` |
| `sir`, `sir.types`, `sir.write`, `sir.simplify` | 1673 | SIR: programs, functions, blocks, statements and types, with the operations a type's constant argument may compute; the types of places; the text form `sisu sir` prints; jump threading | `lib.rs`, `types.rs`, `write.rs` and `simplify.rs` of `sisu-sir` |
| `typeck.ty` | 451 | types, interned in a table | `sisu-resolve/src/ty.rs` |
| `typeck.program` | 2488 | the resolved program, the spelling of types, `Copy`, the trait solver, `Send` and `Sync` (`Send` as a bound owning what it holds, RULES.md's R1), visibility, views and must-consume types | `program.rs`, `ty.rs`, `solve.rs` and `auto.rs` of `sisu-resolve` |
| `typeck.borrows` | 388 | what a signature borrows: the parameters a result may borrow from (`from`, and the elision rules), the borrowed-signature errors | `sisu-resolve/src/borrows.rs` |
| `typeck.std_sources` | 606 | `target` and `builtin_impls`, the std text stage 0 writes, the import paths of the std modules, and those `sisu doc --std` documents | `sisu-resolve/src/std_sources.rs` |
| `typeck.collect` | 5694 | declarations, signatures, imports, traits and impls of a program of several modules and packages, resolved, and what the compiler writes: interceptors and fakes, invariants, predicates, a script's `main`, test cases, doc tests | `collect.rs`, `types.rs`, `traits.rs`, `shims.rs`, `tests_of.rs` and `doctests.rs` of `sisu-resolve` |
| `typeck.template` | 2364 | the evaluator of compile-time code that unfolds a template (a `comptime def derive`, a generic function that reflects on its type parameters) into source text | `sisu-resolve/src/template.rs` |
| `typeck.instances` | 1214 | the impls `@derive` and `@delegate` make, the program's and the standard library's, and the instances of templates, declared in modules of their own | `derive.rs` and `instances.rs` of `sisu-resolve` |
| `typeck.arbitrary` | 1468 | the generators of property tests, with `Arbitrary`'s impls written by hand and derived (R41), and the tests of contracts and `@refines` | `sisu-resolve/src/arbitrary.rs` |
| `typeck.consts`, `typeck.folding` | 341 | a type's constant argument written as an expression, folded where it is written; an expression as folding reads it | `sisu-resolve/src/consts.rs` |
| `typeck.infer`, `typeck.thir` | 789 | inference variables and unification; the typed tree | `infer.rs` and `thir.rs` of `sisu-typeck` |
| `typeck.check`, `.expr`, `.call`, `.pat` | 8605 | the body checker: statements, expressions, calls, patterns | `check.rs`, `expr.rs`, `call.rs` and `pat.rs` of `sisu-typeck` |
| `typeck.exhaustive` | 662 | exhaustiveness and unreachable arms of `match`, every missing pattern (R10), and `sisu lint`'s wildcard over an enum of the package (R11) | `sisu-typeck/src/exhaustive.rs` |
| `typeck.contracts`, `.purity`, `.tasks` | 1746 | contracts and refinements, `pure`, structured concurrency and generators, and what a channel carries (R1) | `contracts.rs`, `purity.rs` and `tasks.rs` of `sisu-typeck` |
| `typeck.safety` | 437 | unsafe code (R2, R3, R5): the blocks and expressions that permit the unsafe operations, the calls that are ones, the `# safety:` comments and `Safety:` lists, and the lints | stage 1's own (22) |
| `typeck.fixtures` | 125 | a test's fixtures (17; R200): the parameters named after its module's `@fixture` functions lowered into `let`s of their calls at the start of its body | stage 1's own |
| `typeck.generic_asserts` | 319 | `comptime assert` on `const` parameters, decided at each call | `sisu-typeck/src/generic_asserts.rs` |
| `typeck` | 2606 | a body's checks, a program's, the inferred error sets, the driver, the typed bodies lowering reads, and the standard library's bodies, checked against the base and kept once per process (an internal error when one does not type-check, R18); a base made whole or from its cache (`BaseCache`); a file's declarations resolved alone, for the formatter | `sisu-typeck/src/lib.rs` |
| `lower`, `lower.names`, `.func`, `.stmts`, `.exprs`, `.pats`, `.contracts`, `.glue` | 6497 | lowering the typed bodies to SIR: what `main` and the tests reach, names, functions and closures (with the value wrappers of decision 225), statements, expressions, patterns, contracts, drop glue, and the standard library's functions and what names them kept once per process | `lib.rs`, `func.rs`, `stmts.rs`, `exprs.rs`, `pats.rs`, `contracts.rs` and `glue.rs` of `sisu-lower` |
| `borrowck`, `borrowck.paths`, `.init`, `.elaborate`, `.loans`, `.consume`, `.purity`, `.shared`, `.names`, `.types`, `.bitset`, `.finding` | 6541 | the ownership checker: move paths, initialisation and moves, drop placement (the standard library's kept once per process), loans, must-consume, purity, the `Shared` escape analysis, places in source terms, and the findings as diagnostics | `sisu-borrowck` (but `explain.rs`) |
| `borrowck.holes` | 360 | the closure of a typed hole, written into the checked SIR (R27) | stage 1's own (10, 17) |
| `interp` | 2466 | compile-time evaluation: the `comptime` functions run on SIR and their values baked into it, or their failures, with the panics' messages | `sisu-interp/src/comptime.rs`, with what it needs of `exec.rs`, `place.rs`, `ops.rs`, `dispatch.rs`, `intrinsics.rs`, `sized.rs` and `format.rs` |
| `fmt`, `fmt.doc`, `fmt.lines`, `fmt.prune`, `fmt.shape`, `fmt.space` | 2190 | the formatter: logical lines with the comments and blank lines between them, sorted imports (the unused ones `sisu lint` reports, R87), the printer of groups, the spaces between tokens, and the shape its self-check compares | `sisu-fmt` (`lib.rs`, `doc.rs`, `lines.rs`, `prune.rs`, `shape.rs`, `space.rs`, `tests.rs`) |
| `format` | 57 | the formatter with the names it needs: which written types name error sets, in a file resolved alone | `sisu-driver/src/format.rs` |
| `doc` | 1171 | `sisu doc`: a Markdown page per module of a program or of the standard library, from the modules' syntax, the resolved names and the inferred error sets, and its tests | `sisu-driver/src/doc.rs` |
| `diff` | 278 | line diffs: the hunks `fmt --check` prints and the change `--locked` prints, from the longest common subsequence's table kept as rows of bits | `sisu-interp/src/diff.rs` |
| `mutants` | 204 | a seeded generator and the mutations of stage 0's robustness test; the files laid out otherwise that the formatters are compared on | `sisu-driver/src/tests.rs` |
| `mutation` | 309 | the mutants of `sisu test --mutate`: a boundary moved, an operator flipped, a negation or a statement removed, the arid calls left alone (RULES.md, R208) | (none) |
| `main` | 3761 | the command line, and the driver: `check` in both forms, the SIR of a whole program (with its tests, `--tests`), `build --native` through stage 0's backend, `fmt`, `doc`, `audit --unsafe`, `lint` (R11), the packages' `unsafe` levels (R6), and the cache of the base beside the native executable | `sisu-driver/src/cli.rs`, `tokens`, `parse`, `check`, `sir`, `build`, `fmt` and `doc` |
| `session` | 188 | the programs of `sisu repl` and `sisu ws` (R79): an input wrapped to be shown, a loaded file's preamble, a worksheet's markers | `sisu-driver/src/repl.rs`'s `Entry`, `split_preamble` and `evaluate_sheet` |
| `query` | 1333 | the language server's questions (R164, R169): what a position names (an import line's names among them), its hover text, declaration and uses, the completions there, a file's declarations, a rename's edits, the inlay hints, the selection ranges, the formatter's edits, and the typed holes (R174), as tab-separated lines | `sisu-driver/src/lsp.rs` and `assist.rs` |
| `config` | 396 | `eval` (R165): a configuration module's constants as data, written as JSON, TOML or YAML | stage 1's own (08) |
| `naming` | 107 | the case each kind of name is written in, and a name written so (R12) | stage 1's own (02) |
| `audit` | 420 | `audit --unsafe` (R7): the unsafe code of a program by package and module, with the modules' hashes | stage 1's own (22) |
| `stage1/std/mem`, `ffi`, `rt` | 1023 | `std.mem`, `std.ffi` and `std.rt` (R4, R8), the standard library's modules only stage 1 reads, and the runtime's first slice in Sisu (R9) | stage 1's own (22) |
| `stage1/std/json` | 463 | stage 1's copy of `std.json`, whose decoder validates refinements and invariants (R32) | `compiler/std/json.sisu` |

73548 lines in all, about 1700 of them tests, beside the 1537 of `stage1/std/`; the Rust the lexer and the
parser transcribe (`sisu-lexer`, `sisu-parser`, and `kinds.rs` and
`tree.rs` of `sisu-syntax`) is 5821 lines with its tests, the AST and its
builder 2809, `sisu-resolve` and `sisu-typeck`, of which stage 1
transcribes `sisu-typeck` whole and of `sisu-resolve` what a program of
several modules and packages needs, 22982, `sisu-lower` 5913 with its
tests, the parts of `sisu-sir` stage 1 has (its data, the types of
places, the writer and jump threading) 1620, `sisu-borrowck` 5686 with
its tests, all of it but the explanation of loans (`explain.rs`), the
driver's `toml.rs`, `manifest.rs` and `package.rs` 1822,
`sisu-interp`'s `comptime.rs` 496, beside the 13 000 lines of the
interpreter it runs on, of which stage 1 has what compile-time code
needs, `sisu-fmt` 1891 with its tests, all of it, with the driver's
`format.rs` 55 and the interpreter's `diff.rs` 108, and the driver's
`doc.rs` 943, all of it, with its part of `cli.rs`, the command `doc`
and `doc_pages`, about 220.

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
stage 1 now, as are the eight of the driver's reading of manifests and
packages and the formatter's eighteen (`sisu-fmt`'s twelve, its
printer's three and its spacing's one, and the line diff's two), beside
new ones (among them the template engine's, drop placement's, the
ownership checker's and the folding of `comptime assert`, each against
what stage 0 gives for the same input, and the line diff's rows of bits
against its table).

The formatter matched stage 0 on every file of the corpus the first time
it ran, and on the same files laid out otherwise; the two differences
found (D6 and D7) were stage 0's, on inputs the corpus did not have, and
were decided for the documents (decision 267).

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
18. **The alternatives of `|` could not bind names** (`E0385`, "yet"):
    Rust's `Stmt::Assign { place, value, .. } | Stmt::AugAssign { place,
    value, .. }` was two arms with the same body, and the type checker has
    some thirty of them. Round 6 lets them (decision 182), and the code
    written since uses them (`case Value.Str(s) | Value.String(s):`).
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
    parameters of those names become `owns`, `defaults` and `from_`; so is
    `comptime`, so the module of compile-time evaluation is `interp`,
    after its crate.
22. **A struct dropped in native code did not free its lists.** Compiled
    by the native backend, dropping a value of a struct with a `list` field
    left the list allocated: 2000 rounds of making and dropping a struct
    that holds a `list[bool]` of 10 000 elements peaked at 159 MB natively
    and 29 MB on the interpreter, 20 000 rounds at 1.5 GB. Lowering stage
    1's own package natively ran out of memory until the initialisation
    dataflow kept its sets as `BitSet`s (as stage 0's `init.rs` does) and
    `sir --partial` left out the loans, must-consume and purity, whose
    findings change no statement, and then peaked at about 3.4 GB. The
    native runtime frees them now: both programs peak at 11 MB, and stage
    1 prints the SIR of its own package, the loans and all, at a peak of
    about 690 MB.
23. **Two walks over types were exponential on stage 1's own types.**
    Whether a drop may run impure code (`drops_impure`) and whether a type
    must be consumed (`must_consume_ty`) followed fields with no memory of
    what they had seen, as stage 0's did before decision 180; `types` of
    stage 1's own package took over ten minutes and 7 GB. A set of the
    types seen per query, and a memo of the depth cleared per query, make
    it 12 s, with the same answers.
24. **`let ... else` on a borrowed place moves out of it** (`E0419`): `let
    Call(dest, ...) = s.kind else:` with `s` a `ref` binds by move, where
    `if let` binds by reference. And `f(x)?.g()` reads as an optional chain
    `?.` on `f(x)`, not as `?` and then a call of `g`; the result is named
    first.

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
building their lines. Reading the text into a `list[int]` once was tried,
and gains nothing measurable, natively (about 50 ms for the prelude either
way) or on the interpreter (about a second): building the list costs what
the reads it replaces did, so the scanner stays a transcription.

The type checker adds its own costs (measured on the interpreter two
rounds ago, with the ten std modules a program could import then):
resolving the prelude, the four ambient std modules and
`builtin_impls`, parsing the std modules a program may import and
unfolding the impls they derive takes about 20 s, after the 12 s `sisu
run` takes to start stage 1, and each file then a fraction of a second (a
file that imports a std module declares and resolves it too, as stage 0
does); `types-batch` checks the 401 programs of the corpus of then in
about 90 s (195 s for 397 before that round's work and stage 0's rounds 7
and 8, 155 s for the 374 of round 6, 180 s for 360 before round 5 made the
interpreter faster, decisions 175 to 178; the template engine checks a
file again for each round of instances it declares). Its tests use a small
prelude of their own.

Lowering and the ownership checker add about half a second a program on
the interpreter: `sir-batch` lowered those 401 programs and checked their
ownership in about 220 s (the front end 64 s, lowering 47 s, the ownership
checker and the writing of the SIR 75 s; 580 s before that round's work
and stage 0's rounds 7 and 8).

Compiled by stage 0's native backend (`sisu build --native`, `check.sh
--native`), stage 1 runs 20 to 60 times faster: the base (the prelude and
the std modules resolved, and their derived impls unfolded) takes about a
second, `types-batch` checks the 450 programs of the corpus in about 4.5 s,
and `sir-batch` lowers them and runs the ownership checker in about 13.5 s
(the front end 3.5 s, lowering 3.5 s, the ownership checker and the
writing of the SIR 4.5 s); stage 1 prints the SIR of its own package in
about 30 s (stage 0 in 4 to 7 s), half of it the ownership checker over
its SIR functions, at a peak of about 920 MB of memory (`sir
--partial`, which leaves out the loans, must-consume and purity, at
555 MB). `./check.sh --native --names --types --sir` takes about eight
and a half minutes, one of them the fixed point.

A body of the standard library is type-checked when lowering first
reaches it. Each program's table of types is its own, so a body typed for
one program cannot serve the next as it is; `typeck.StdBodies` keeps, for
each body of a module the base resolved (whose definitions every program
shares), the body typed against the base's table, with the types its
check made past the base's in the order it made them, and each program
makes those types again in its own table, which holds the base's first,
and reads the body's types through them (`remap_body`). So the standard
library's bodies are checked once per process, as stage 0 parses the
standard library once and keeps each item's checked body, and the
answers are the same; it took lowering's part of `sir-batch` from 8.7 s
to 5.8 s natively. The lines of the std modules' texts are counted once
too, in the base. Such a body is checked against the base alone
(`check_std_base`: its definitions and its table of types), since nothing
a program adds can reach it: a program's impls are of its own types or
traits, its inferred error sets its own functions'. Checked against the
first program that reached it, as it was, a body met that program's impls
in a table of types without their types: `stage1 sir` of
`conformance/cases/examples/capabilities.sisu` alone panicked, and the
corpus passed only because an earlier program had reached the body first.
Each of the 450 programs gets the same SIR and ownership diagnostics
alone, in a batch and in a batch run backwards; a test lowers two programs
in both orders.

This round's work went to the batches, where each program does again
what only its own definitions can change (the first item of "What is
next" of the round before): profiled with valgrind's callgrind over a
quarter of the corpus (132 programs), `types-batch` spent about 4% of its
instructions walking definitions and impls that are the base's and the
same in every program. Now the walks start at the program's own: the
base notes where its definitions end and which of its functions every
program makes an instance of whether anything calls them
(`Resolver.base_defs`, `base_eager`, `mark_base`), so each round of
instances looks at those and the program's own definitions rather than
every definition (`check_round`), and the bodies of templates are sought
among the program's own (`template_bodies`); a derived impl is sought
among the impls made after the base (`check_derives`, since an impl made
before a type cannot be of it); and an impl is held against the earlier
impls of its own trait (`check_coherence`, from `Program.impls_of`,
which lists them in their order among all the impls) rather than against
every earlier impl. `sisu doc` lexes each module once, for its tokens and
its parse (`parse_lexed`). The outputs are the same (every class of
`check.sh`, and the four batches of the corpus, `types-batch`,
`check-batch`, `sir-batch` and `doc-batch`, compared file for file
before and after). Over those 132 programs `types-batch` executes
6.31 G instructions against 6.60 G (callgrind's count; the base, the
same 2.4 G in both, among them), and over the corpus (the 539 programs of
`types.list`, five runs of each build in turn on a machine whose speed
varied by a fifth) it takes 5.2 s against 5.9 s (medians), `sir-batch`
15.8 s against 16.5 s and `check-batch` 14.6 s against 15.0 s (three
runs each), and `sisu doc` of stage 1's own package 12.5 s against 13.6
s. Of what each program does again, the batches spend most now on the copy
of the base's resolver for each resolution (about 6% of `types-batch`),
the second parse of each module (about 6%), and the text of the derived
impls of the program's types and of the std modules it imports, unfolded
or taken as the base made it, parsed and declared for each program
(about 5%); the base itself is about 2.4 G instructions, a third of a
second, once a run.

The round before took stage 1's SIR of its own package (stage1-A, in the class
`fixed`) from about 34 s to about 28 s, and that of its source files
(`sir --partial`, in the class `self-sir`) from about 21 s to about 18 s,
with the same outputs: every class of `check.sh`, and each program of the
corpus alone and in a batch run backwards. The times are medians of the
old and the new executable run in turn, three times each, on a machine
whose speed varied by a fifth meanwhile; the classes of the corpus, whose
time goes to the front end and to lowering more than to the dataflows,
moved within that (`corpus-sir` 14.9 s and 15.3 s, `corpus-check` 13.3 s
and 12.3 s). In profiles of the self SIR, the loans dataflow and its
checks went from about 10.5 s to 7.5 s, and the purity of the drops in
`pure` bodies from about 1.2 s to 0.5 s. The peak memory of the SIR of the
package went from about 840 MB to about 920 MB, the references of the
loans state being rows of words now (below); `sir --partial` stays at
about 555 MB. The work:

- **the loans dataflow's states copied at once**: since decision 253 the
  native run time copies a value that owns nothing as its bytes, so that a
  copy of a list of integers costs a twentieth of a `while` loop that
  assigns its words; `Rows.assign` and `BitSet.assign` copy the list
  rather than assign in place, and the references of a state
  (`State.direct`, a set per local, which the run time copied a set at a
  time) are rows of words as `holds` is, with the locals whose reference
  is known as a set (`State.known`). A state is copied without its log
  (`detached`), and a visit takes its block's entry, or a copy of it,
  rather than making a state to replace;
- **liveness from each block's summary**: what a block reads before it
  writes it and what it writes (`gen` and `kill`, found once a function)
  give what is live at its start in each round of the loans dataflow's
  liveness, rather than a walk of its statements; the live loans of a
  statement are added row by row in one walk (`Rows.add_rows_to`), a
  use's two-phase loans are activated through a mask (`two_phase`) rather
  than by listing its row, and a write to a local whose type can hold no
  loan no longer asks the type (its row is left out, so nothing is
  written);
- **the initialisation dataflow's joins**: the state after each
  statement's moves is joined straight into the unwinding block's entry,
  only the words of its sets the statements changed after the first join
  (`push_words`), rather than copied and joined after the statements; a
  switch's facts (the path switched on, its cases and its payloads'
  paths) are found once a visit rather than for each edge
  (`switch_facts`), and the state is copied only for a switch's edges;
  must-consume's states are met in place;
- **constants written out where a bit set is touched**: natively, a
  constant is read through a call, as a function is called (about 9 ns),
  so the operations on a member of a `BitSet` or of `Rows` write `WORD`
  out (62), and test a slot's sign rather than compare it with
  `LEFT_OUT`;
- **substitution in a type without parameters**: the table of types notes
  whether a type mentions a parameter as it makes it (`has_params` reads
  that), and `subst` gives such a type back as it is rather than making it
  again, part by part, as it did for the field types of every struct the
  purity of a `pure` body's drops looks at (`drops_impure_in`);
- **lookups made once a program**: the traits by qualified name
  (`lower.traits_by_name`), which lowering looked a `Trait.method` up in
  by a walk of every definition, for each `call trait`; the base's
  intrinsics by name (`StdSlots.intrinsic_defs`); the intrinsics the
  ownership checker asks about, by name in its table (`Types.intrinsic`);
  and the impls nothing dispatches to are passed by before their names
  and types are made.

A round before that took stage 1's SIR of its own package from about 75 s
to about 55 s, and the corpus's `sir-batch` from 14 s to 10.5 s, with the
same outputs (every class of `check.sh`; the times of `check.sh
--native`'s classes before and after: `types` 4.8 s and 4.2 s, `sir`
14.8 s and 10.9 s, `corpus-check` 13.6 s and 10.3 s, `self-sir` 54.2 s
and 49.5 s, stage1-A 75.1 s and 56.7 s, the whole run 5 min 53 s and
4 min 47 s, on a machine other builds shared), by:

- **the standard library's functions lowered once per process**
  (`lower.StdLowered`, as `typeck.StdBodies` keeps their typed bodies and
  `borrowck.Placed` their drops; stage 0 keeps a program's lowered bodies
  from one lowering of it to the next): a body of a module the base
  resolved lowers to the same functions in every program but for the
  names of the prelude's functions the program's own hide (`std.NAME`) and
  the instances of std templates it declares, so an entry is taken again
  by a program that agrees on those (`std_agreement`), with its functions'
  source renumbered; and what `lower` finds of the base's definitions (the
  declared types by name, the names of the base's impls' methods, the
  bodies in their order and their functions' names) is found once too
  (`lower.StdSlots`). The first alone took lowering's part of the corpus's
  `sir-batch` from 5.0 s to 4.1 s;
- **the unwinding state joined a statement's changes only**: a transfer
  records the locals whose rows and references it wrote
  (`State.touched`), and a block's unwinding state joins those after each
  statement (`join_touched`), straight into the unwinding block's entry
  rather than into a copy (unless that block is the block itself or one of
  its other successors, when the copy is kept, in stage 0's order); the
  liveness pass asks once a local whether dropping it runs a user `Drop`
  and reuses each statement's uses of locals, must-consume asks once a
  local whether its value must be consumed, and purity finds a function
  in the table of functions by name rather than by a walk of the program;
- **walks that were quadratic made linear**: the move paths a
  discriminant read makes for each variant, and the variants a switch's
  edge leaves active, as sets (an enum of hundreds of variants has as
  many paths and cases); the type checker's impls by trait
  (`Program.impls_of`), which `implements` and `trait_methods_for` walk
  instead of every impl; lowering's types gathered per function stopping
  at a type met before, and the names of tests, benchmarks and instances
  found in tables;
- **the resolved program's definitions sharing what they define**: a
  program checked against the base copies the base's resolver, each
  definition's kind (its parameters, generics, fields and variants) with
  it; a `Def` now holds its kind as a `Shared` (`Def.kind`, `set_kind`),
  so the copy is another owner and a change makes the kind the program's
  own;
- compile-time evaluation running all of a program's `comptime`
  functions with one interpreter, its table of functions made once.

Together they took the parts of the self SIR from 9.7 s, 12.2 s and
46.7 s (the front end, lowering, the ownership checker) to 8.8 s, 7.9 s
and 30 s, and those of the corpus's `sir-batch` from 3.5 s, 5.0 s and
3.9 s to 2.7 s, 2.4 s and 3.6 s (measured one build against the other on
the same machine, a run of each in turn). Flattening the loans state's
references (`direct`, a set per local) into one buffer, as `holds` is,
gained nothing measurable then, while the run time copied a list a word
at a time; this round keeps it (above).

The round before it took stage 1's SIR of its own package from over 190 s
to about 85 s, and `check.sh --native` with its new classes from twelve
and a half minutes to seven, with the same outputs, by:

- **the loans dataflow as stage 0 keeps it since decision 219**: a row of
  its bit sets only for each local whose type can hold a loan, the others
  left out (`bitset.Rows`, `LEFT_OUT`), with the slots shared by every
  state of a function (stage 0's `Arc`, a `Shared` here); states joined in
  place, and a block's unwinding state copied only for a block that
  unwinds; each statement's uses of locals worked out once a function; a
  function found by name in a `dict` (`Types.function`); must-consume's
  state holding only the iterators it follows;
- **loops without intrinsics**: natively, each step of a `for` loop calls
  the intrinsic `Iter.next`, and `is_some` is one too, each through the
  run time's dispatch, which costs more than the rest of a step; the bit
  sets, the dataflows' walks over a block's statements and the join of two
  states are `while` loops over indices and `match`es;
- **the standard library's initialisation and drop placement once per
  process** (`borrowck.Placed`, as `typeck.StdBodies` keeps the typed
  bodies): a function of the standard library whose SIR and the
  declarations its places reach are the same as for an earlier program
  takes the body placed then, which took the ownership checker's part of
  `sir-batch` from 7.2 s to 5.3 s;
- **the resolver copied for each program and each round of instances
  shares what no program changes**: the std modules a program may import,
  parsed once, and the message catalog, as `Shared` values, whose copy is
  another owner, which took the front end's part of `sir-batch` from
  about 5.5 s to 4.3 s;
- `diag.Lines` finds the line and column of a position in ASCII text
  without decoding it.

A profile of stage 1 printing its own SIR (gdb samples, a thousand of
them) shows where the rest goes: the ownership checker half of it (the
loans dataflow and its checks under a third, spread over the transfer of
statements, the joins at the edges, the copies that make a successor's
entry and the liveness; the initialisation dataflow a tenth;
must-consume and drop placement a twentieth each), the type checker under
a quarter, lowering an eighth, parsing and building the AST a twelfth,
and writing the SIR a thirtieth. No function takes more than 3% of the
samples of its own. The run time's copies and drops of values are about a
tenth, its allocations a tenth, and its intrinsics, each called through
its dispatch (`Shared.get` among them, about 40 ns, which every row of a
loans state reads its slot through), a little more.

In the batches of the corpus, whose programs are small, the time goes to
what each program does again: `types-batch` spends a tenth of its time
copying the base's resolver for each program and each round of instances
(about 1 ms: its definitions half of it, its methods and table of types
most of the rest), a twentieth parsing each program a second time (the
name pass parses it, and the type checker again), and about as much on
the walks over every definition, the base's among them, that each round
makes (`fn_info` through `Shared.get`). The second parse was taken away
this round to see (each module of an input carrying the parse the name
pass made, to the type checker's `check_program`): natively it is about a
quarter of a second of the corpus's `sir-batch` of sixteen, less than the
machine's speed varied from one run to the next, so the change, which
the copies of an input a build makes would have to carry too, was left
out.

The formatter, compiled natively, formats the conformance suite, the
standard library, the proposal's examples and its own sources (578
files) with `--check` in about 4.5 to 5 s, where stage 0's `sisu fmt
--check` takes about 3.3 to 3.7 s (the two run in turn, five times, on a
machine other work shared): the conformance suite in 0.43 s (stage 0's
0.27 s), the standard library in 1.2 s (0.41 s), the examples in 0.08 s
(0.02 s), and stage 1's sources in 2.8 s (2.6 s). A file that is
formatted already is parsed and laid out once, and that is all its time
(over stage 1's sources, `fmt-profile`: parsing 1.3 s, the layout 1.8 s);
the standard library's is mostly the base, resolved once a run for the
prelude, which writes `Result[` (stage 0 checks the prelude as a program
of its own too, in less time). A file that changes costs more: over
stage 1's sources with the lines inside their brackets joined, parsing
takes 1.1 s, the layout 1.5 s, the self-check's second parse 1.1 s and
its two shapes 1.1 s and 1.0 s, and `--check`'s hunks 0.3 s, about 7 s
in all, where stage 0 takes about 3.6 s. As first transcribed, the same
took 16 s, the hunks 2.7 s of it (the table of the longest common
subsequence filled a cell at a time, where stage 1 now keeps it as rows
of bits) and each shape 1.5 s (each node's a text of its own, copied into
its parent's); and the corpus, mostly formatted, took about three times
as long, since a text that came out as it went in was parsed again and
its two shapes written (above, "The formatter").

In the round that made stage 1 the front end (RULES.md), a profile of
`types-batch` over a quarter of the corpus (callgrind, every fourth of
the programs of `types.list`, 135) put the three items of "What is next"
at about 4.6% of its instructions for the type checker's second parse of
each module, 1.6% for each import of a std module (its derived impls'
text parsed again among it), and the copy of the base's resolver inside
each round of resolution; and it showed a cost no list had: every parse
made its text's table of lines twice, once for the parser
(`Parser.new`) and once more to keep one syntax diagnostic per line
(`one_per_line`), 1.5% of the batch. The second is gone: the parser hands
its table back with the tree (`Parser.finish`), and `one_per_line` reads
it. Over those 135 programs `types-batch` executes 6.231 G instructions
against 6.321 G (1.4% fewer), and the syntax batch over the standard
library 5.105 G against 5.140 G, with the same outputs (`types-batch`,
`sir-batch` and `check-batch` of the 542 programs of `sir.list` compared
file for file, before and after, and every class of `check.sh`). Writing `TreeBuilder.finish_node`'s loops as `while` loops
over indices saved 0.27% more and was left out, and giving its list of
children their room up front saved nothing (a node has a handful). The
three items stay where they were: each is a few per cent of a batch at
most, less than the machine's speed varies from one run to the next, and
each a change to what an input carries through the driver.

The thirteenth round measured the batches again (`check.sh --native`, on
four cores that other work kept at a load of 3 to 11): `types-batch` of
the 549 programs of `types.list` 7.9 s, `sir-batch` of `sir.list` 23.9 s
(the front end 7 s, lowering 6 s, the ownership checker and the writing
of the SIR 8 s), `check-batch` of the corpus 24.4 s, `doc-batch` 47.9 s,
the behaviour's batch 22.8 s, stage 1 over its own package (`names`,
`types`, `sir --partial`) 76 s, and the whole native run 21 minutes. A
batch makes its base whole once, so R125's cache does not reach it; the
catalog's read and the lexer's lookup of punctuation do, by less than the
runs vary. What remains is what the items below name: the base's
resolver copied for each program, each module parsed again by each pass,
and the lexer and the parser themselves, which a batch pays for every
program's text and every command for the base's (above; and below,
"Starting up").

The seventeenth round runs the batches on the release profile (RULES.md,
R150): `sisu1` and `check.sh --native` take the stage 1 the driver builds
for every command (decision 860, Cranelift's `speed` level, every contract
checked), where they built the debug profile's. Measured one build
against the other on the same machine, each batch in turn and twice,
`types-batch` of the 549 programs of `types.list` takes 5.2 to 5.9 s
where the debug build took 7.3 to 7.9 s, and `sir-batch` of `sir.list`
17.5 to 18.6 s where it took 22.9 to 23.9 s: a quarter and a fifth less,
less than the 2.6 times fewer instructions of R135's parse of the prelude,
since a batch's time is mostly the run time's copies, drops, allocations
and intrinsics, which are the same code in both. The whole `check.sh
--native --names --types --sir` took 955 s, against 1990 s before on a
machine twice as loaded (the same classes listed below R150); the
classes `self` and `fixed` keep the debug stage1-A the fixed point is
about, `self-sir` 67 s of it.

### Starting up

A command of stage 1 that checks one program (`check`, `sir`, `build`,
`run`, `doc`, `fmt`, `lint`, `audit`, `types`, `ownership`) started by
making the base again: the prelude, the ambient modules, `target` and
`builtin_impls` parsed, resolved and their derived impls unfolded, and
then every std module a program may import parsed and loaded on a copy
of the base, for their derived impls and the generic functions their
programs promote (`Resolver.base_with_all`, `typeck.std_promotions`); and
the name pass parsed the prelude and the ambient modules once more for
its environment. A `check` of a two-line file took 0.53 s of CPU time
(3.14 G instructions under callgrind, 81% of them the base and 14% the
second parse), where the Rust front end takes 0.08 s.

Since this round the base keeps what the second half computes in a cache
beside the native executable, `compiler/target/stage1/base-ID.cache`
(`main.base_of`): the derived impls of every std module, as text, and the
promotions of each module a program may import (`typeck.BaseCache`, about
60 KB), with a key made of the running executable's size and time and
each std source's name, size and time, and the key again on its last
line, so that a file half written is no cache. A base made from the cache
(`typeck.base_cached`, `Resolver.base_cached`) is the same base: the base
modules declare the cached text of their derived impls rather than
unfolding it again, as a module a program imports always did, and the
modules a program may import are parsed when one imports them
(`Resolver.std_sources`). The name pass's environment comes from the
base's own parses (`Base.environment`, `ModuleNames` of the prelude and
the ambient modules), for every command that makes a base. A miss makes
the base whole, as before, and writes the cache, removing the caches of
other executables; stage 1 run on the interpreter, whose executable is
stage 0's `sisu`, keeps none, and a batch makes its base whole, once for
all its programs. The same `check` now takes 0.23 s of CPU time (1.48 G
instructions); what remains is mostly parsing the prelude, `cap` and
`builtin_impls` and building their ASTs (60% of it), which only a cache
of the parsed and resolved base itself would remove. Each of the 506
programs of one file of the corpus gets from `sisu1 check --format=ids`
and `sisu1 sir`, made with the cache, what `check-batch` and `sir-batch`
give it; `check.sh`'s classes `build` and `self` run the single commands
with the cache too.

The fourth round measured it again, on a machine other work kept at a
load of 6 to 10 on four cores (the least user time of six runs each):
`check --format=ids` of a two-line file takes 0.32 s with the cache and
0.58 s without it (a miss, which makes the base whole and writes the
cache), and callgrind, under which the key never matches (the running
executable is valgrind's), counts 3.99 G instructions for either, where
it counted 3.14 G for the miss a round ago: the standard library and the
front end both grew. The write is not atomic and cannot be made so:
std's `Fs` has no rename (`cap.FsOps`: `read`, `write`, `read_to_string`,
`write_text`, `exists`, `metadata`, `remove`, `list_dir`, `create_dir`,
`sync`), so a sibling file could not be put in the cache's place. What
holds without it: a reader of a file half written finds no key on its
last line and makes the base whole (a miss), and two runs that write at
once write the same bytes, since the cache of one executable and one
standard library is one text. A rename is the runtime's to add (a host
call of the interpreter and of the native runtime, `Fs.rename`), after
which `write_base_cache` writes `base-ID.cache.PID` and renames it. The
sixth round does: the cache is written aside, `base-ID.cache.N.aside`,
and renamed over the old one (decision 420).

The thirteenth round measured a cached `check --format=ids` of a
two-line file under callgrind (the cache written under valgrind first,
since the running executable names it): 4.64 G instructions, 4.11 G of
them `base_of`. Of the base, 1.94 G is lexing and parsing what it declares
(the prelude, the ambient modules, `target`, `builtin_impls` and the text
of the derived impls; the lexer 1.03 G, about 3,500 instructions a byte),
0.56 G building their ASTs, 0.37 G resolving them and 0.15 G checking the
traits, with the copies and drops of the values throughout; outside the
base, the message catalog took 0.30 G (each line split into all its
words) and the lines of every std text 0.15 G. R125 keeps in the cache
which importable modules hold templates and `unsafe` code (0.14 G of text
searches), finds a std text's lines when lowering first reaches it, reads
a catalog line's first four words alone and looks punctuation up by its
bytes: 4.15 G now, 3.79 G of it the base. What remains is the parse of
the base's modules; a cache of the collected items, the resolver's
program and types with the ASTs lowering reads the std bodies from, was
decided against (RULES.md, R125).

The fourteenth round measured the lexer natively (callgrind on `sisu1
tokens` of the prelude, then on the same cached `check`): a punctuation
was looked up in a `dict` up to three times, a key made of its bytes for
each length (59 M of the prelude's 543 M), every name was made a `String`
and looked up among the keywords (27 M), the scanner built both tables
for each text it scanned, and each byte was read by a call of
`Scanner.byte`, some twice, at about 200 instructions a read. R127
dispatches on the bytes, compares a name's bytes with the keywords' of
its first byte and length, and reads the bytes of a loop in place: the
prelude lexes in 284 M, about 1,470 instructions a byte, and the two-line
`check` takes 3.42 G, the lexer 0.53 G of it. The same profile showed the
AST builder walking every function's body again for an `ERROR` node no
std text has (0.1 G) and walking a block's children from the first to
find each item's doc comments, quadratic in an `impl` of many methods
(0.16 G): it builds in 0.54 G where it took 0.77 G. What remains of a
command's start is the parser's own 0.91 G, the resolution of the base
(0.37 G) and the copies and drops of the values throughout; the lexer's
cost a byte is now mostly the native code's for each operation, a load,
a compare and a branch each a few instructions more than they need.

The fifteenth round profiled the parser (callgrind on `sisu1 parse
--print` of the prelude, 988 M instructions): the table of lines was made
twice, by `Parser.new` and by `render_ids_in` of no diagnostic, 45 M each
at about 230 instructions a byte, and the parser's most frequent calls
(`at`, `current`, `nth`, `flush_trivia`, `tokens_attached`) cost 100 to
250 instructions each, most of it the call. R135 finds the lines from the
tokens, makes no table for no diagnostic, and reads the current token in
place: the prelude's parse takes 353 M instructions where it took 464 M,
and the two-line `check` 3.23 G where it took 3.43 G. What is left of a
command's start is the parser's 0.70 G, the lexer's 0.53 G and the AST
builder's 0.54 G, and the code of the debug profile, in which `sisu1` and
`check.sh` built stage 1: the release profile parses the prelude in 373 M
instructions, 2.6 times fewer, which makes the profile the larger lever
(the driver's and `sisu1`'s to choose; the driver chose it in decision
860, and `sisu1` and `check.sh` run its build since R150).

The sixteenth round profiled resolving and the trait check of the same
cached `check` (3.392 G instructions, the base 3.03 G of it): resolving
the base's declarations (`resolve_pending`) took 0.38 G and checking its
traits (`check_traits`) 0.15 G. The trait check looked for each derive's
impl among every impl of the program, comparing two module names'
prefixes for each, quadratic in the base's 508 impls (`check_derives`,
88 M), and `resolve_impl` copied each trait's list of impls to add one.
R147 searches the trait's own impls and adds in place: the trait check
takes 0.10 G, the command 3.34 G. Resolving itself has no site above 3 %
of the command: 1,323 functions and 622 impls at about 155,000
instructions each, their types resolved at about 37,000 each
(`def_as_type` and the interning key of `Types.adt`), the walk of each
body for compile-time code (`unfolds`, 27 M) and the copies of `FnInfo`
in `Def.set_kind`, so what would cut it is a base whose resolved
declarations are kept rather than made again, the cache R125 decided
against.

The eighteenth round took the other way out of the base's cost: a
process that keeps it. `sir --repl-serve`, which served the REPL's
commands since R112, serves every command that checks a program
(`sir`, `check`, `parse`, `fmt`, `lint`, `doc`, `tokens`), each run in the
directory its request names (R158). Measured natively on a machine at a
load of 10 to 12 on four cores, a `check` of a two-line file takes 0.22 s
of CPU time as a process (the least of ten) and 0.009 s served (the
median of 50, after a first answer of 0.44 s that makes the base); under
callgrind 1.95 G instructions as a process, and 0.095 G for each request
served after the base's 1.75 G. What a served request still pays is its
own: `base.environment()` and the copy of the base's resolver for the
program (R147's spread), then the program's check and lowering. The
driver's use of it, one server per working directory, is stage 0's next
round.

The nineteenth round's questions of the language server (R164) are
served the same way and cost what a `check` does: a hover of a 30-line
program takes 0.22 s of CPU time as a process and 0.009 s served (the
median of 50, after a first answer of 0.36 s that makes the base), a
completion 0.013 s and a search of references 0.012 s, measured natively
at a load of 5 to 8 while `check.sh` ran beside them. A completion after
a `.` checks the program twice (with the `.` and without it), and a
search of references parses each module of the program's own packages
again for its names.

The twentieth round's questions (R169) cost the same: served, a rename of
a local in the 24-line `cases/findings20/query_assist.sisu` takes 0.014 s
of wall time (the mean of 30 after the first request, which makes the
base, 0.29 s), its inlay hints 0.009 s, at a load of 2 to 4 on four
cores; a rename is a search of references, which parses each module of
the program's own packages again and now each import line's module path
too. `check --holes` as a process takes 0.29 s, a `check`'s cost.

The twenty-second round's commands cost what a `check` does too,
natively as processes at a load of about 5: `explain-borrow` of
`cases/findings22/explain_borrow.sisu` 0.28 s against its `check`'s 0.30
s, `audit --caps` 0.27 s, `check --diary` 0.26 s, and `audit --ownership`,
which reads tokens and parses but checks nothing, 0.02 s. `sir --tests
--std` takes 3.3 s where `sir --tests` takes 0.26 s. It checks and lowers
197 examples and every std module `doc --std` documents, parsing each
module's text again for its items' starts, since the base keeps no syntax
of its modules.

The twenty-third round's commands cost what a build or a check does,
natively at a load of 12 to 13 on four cores while `check.sh` ran beside
them (the median of five as processes): `build --emit-sir` of
`cases/findings23/lamp.sisu` 0.46 s against its `check`'s 0.36 s, and
0.047 s served after the first request; `check --holes` of the package
`served_build.pkg` 0.34 s; `query values` of `cases/repl/sheet.ws.sisu`
0.47 s, where `sir --worksheet` takes 0.44 s, and 0.12 s served. It checks
the worksheet as written and its instrumented program, as `sir
--worksheet` does, and the evaluator's run of the worksheet is a few
thousand steps. `grammar` reads five files, 0.016 s.

The twenty-first round's commands cost what a `check` does, measured
natively as processes at a load of about 4: `fix --dry-run --all` of the
13-line `cases/findings21/fix_assist.sisu`, which checks it twice (the
round that fixes and the one that finds nothing), 0.36 to 0.45 s, its
`check` 0.35 to 0.41 s, `check --holes` of `holes_report.sisu` with its
whole report 0.31 to 0.37 s, and `check` of `const_fine.sisu`, which
evaluates its four constants, 0.35 to 0.42 s: a constant of a
configuration costs the evaluator's steps, a few thousand, against the
check's tens of millions of instructions.

The twenty-fourth round's commands cost what a check does, natively as
processes at a load of about 3 on four cores: `doc --std --digest` 0.82
to 0.91 s, as `doc --std`'s pages (0.86 to 0.89 s), since the digest is
made from them; `sir --tests --mutants` of `cases/findings24/mutants.sisu`
0.35 to 0.40 s for its seven mutants, against `sir --tests`'s 0.29 to
0.33 s, each mutant a check and a lowering of the program from the base;
`check` of `confusables_whole.sisu` 0.28 to 0.29 s, as R136's case's,
the skeleton a binary search a character over the 5,147 records. A
program of all 1,187 lines of `GraphemeBreakTest.txt`, built natively,
runs in 0.017 s, and `Zone.named` reads its zone's line of the history
once per call.

The twenty-fifth round's additions cost nothing a check notices,
natively as processes at a load of about 1 (three runs each): `check` of
`cases/findings25/no_alloc_kept.sisu` 0.26 to 0.28 s after the first
run, as the same program without its `@no_alloc` (0.27 to 0.29 s), the
pass walking the calls of one function into the standard library once,
each function's answer kept; `sir --tests --cover` of `cover.sisu` 0.26
to 0.28 s against `sir --tests`'s 0.27 to 0.32 s, the table one parse of
each module and a read of the SIR's text.

The twenty-sixth round measured a served save and the standard library's
examples. A save of `live/swap` (`build --for-run --emit-sir`, served)
took 1.18 G instructions under callgrind: the copy and the drop of the
served base 0.23 G, a second check, lowering and ownership check of the
same program for its SIR 0.31 G, `std.time` parsed three times (once by
the name pass, once in each check's import) 0.12 G, and the std bodies
typed and lowered again 0.14 G. R220 takes all four: the save takes
MEASURE_SWAP where it took MEASURE_SWAP_BEFORE (the mean of 20 after the
first, at a load of about 8 to 10). A `query diagnostics` of a module of
`compiler/rt`, 18 modules and 7,624 lines, takes MEASURE_RT where it took
MEASURE_RT_BEFORE: the parse of its modules was 1 G of its 16 G
instructions; the rest is the program's own type check (3.7 G), lowering
and ownership check (11 G), done again at each request, which only a
memo of each body's results, with the dependencies no phase records,
would shorten. `sir --tests --std` of no program took 28.6 G
instructions, the base made whole among them: its check 7.5 G, 4.2 G of
it declaring the examples in each of two rounds of resolution, lowering
8.9 G (the 815 std bodies the examples reach typed, 3.6 G, and lowered,
3.6 G) and the ownership checker 5.5 G (its elaboration 2.2 G). R224
makes the examples' modules once a check: 26.9 G, and natively 2.9 to
3.3 s of user time where it took 3.4 to 3.6 s; what is left is the
examples' own reach into the standard library, typed, lowered and
checked once a process (a server's later request takes them from its
`Kept`), and the parse of every documented module for the examples'
places (1.8 G), which a base that kept its modules' syntax would give.

### The compile-time evaluator

A sieve of 20,000 numbers at compile time took 90 s, a round ago: the
evaluator (`interp`) copied a list to read its length at each index,
`xs[k]` and `xs[k] = v` alike (`resolve_mode`'s bounds check), copied it
to push or pop an item (`list.push` read it whole and wrote it back), and
copied an iterator of items made up front at each step (`Iter.next` read
it, and took and wrote back its items). Each is quadratic in a loop. The
evaluator now measures a list or a dict where it is (`list_len_at`,
`extent_at`, `entry_at`, which read through `value_at`'s reference),
changes a list, a `String` and an iterator in place (`edit_at` with
`InPlace.ListPush`, `ListPop`, `TextPush` and `PopItem`, the items of an
iterator made up front kept in reverse so that each step pops the next),
and measures a collection behind a reference where it is for `len`. The
sieve of 20,000 now takes 1.1 s for the whole `sir` command (about 0.35 s
of it the base and the rest of the front end), of 4,000 0.8 s (3.7 s a
round ago); the SIR of the sieve, `cases/comptime/sieve.sisu` (in
`sir.list` and `types.list`, with a loop over a list's length besides),
of `cases/comptime/evaluator.sisu` and of every program of the corpus is
stage 0's as before. What remains
quadratic: a read through a reference of a whole value (`target`, which
an intrinsic's argument goes through), and the values themselves, trees
a copy of which is a copy of every node.

The eleventh round measured the chapter's programs again (`sisu1 sir`,
native, on a machine other work kept busy): `table.sisu`, the sieve of
100, 1.2 s, the sieve of 4,000 1.2 s and of 20,000 1.8 s, where the Rust
front end takes 0.5, 0.8 and 1.4 s. The manual's 3.7 s and 90 s were the
ninth round's; the sieve's evaluation is now below the start-up's
noise for 4,000 numbers and about 0.6 s for 20,000, so nothing
dominates that a change of the evaluator would remove without the base
cache of item 8 below, and the round changed nothing there.

## What is next

Every part of 18's stage table for stage 1 exists, `sisu doc` the last
(above, "Documentation"); what remains is beyond it, or is speed.

1. **`sisu doc`'s parts that need evaluation**: 10's rendering of an
   example with its values (decision 258 leaves it out, port-debt.md),
   which stage 0 does not have either, and which R217 leaves for an
   evaluator that runs against the fakes; `--digest` is R207's, and
   `--open` the driver's (R207).
2. **Running SIR**: the rest of the interpreter (tasks, capabilities, the
   host), of which `interp` has what compile-time code needs, where stage
   1 becomes the oracle; the backend and the runtime stay stage 0's, as
   the plan expects of stage 2.
3. **What stage 1 still notes without rendering**, which no program is
   known to reach (above, "Stage 1 as the compiler's front end"): a label,
   or the bound on instances, in a module with no file among the
   program's, and a generator's site that its walk of a module's
   expressions misses. Each error it noted before round 8 is rendered,
   with a case of its own.
4. **Speed** where it pays (above, "Where the time goes"): in the
   batches, the base's resolver, copied for each program and each round
   of resolution, whose definitions could be shared whole, the program's
   own after them, now that reading through a `Shared` costs a load in
   native code (decision 260: `Shared.get` no longer goes through the
   run time's dispatch); the parse of each module, made by the name pass,
   the type checker and, for `sisu doc`, the pages, which a package as
   big as stage 1's own pays for three times (each module of an input
   could carry the parse the name pass made, which the copies of an
   input would have to carry too); and the derived impls of the std
   modules a program imports, whose text the base keeps, parsed again for
   each program that imports one. In the native runtime, which is stage
   0's: a list of values that own something (a list of optional sets,
   say) copied faster than a value at a time (decision 262 made a start);
   in the interpreter, views that do not copy.
5. **The discrepancies**: none is listed; the first five and the
   formatter's two (D6 and D7) were decided for the documents and stage 0
   changed. A new one is listed from D8 on and decided the same way
   (D-079). A rule stage 1 changes is not a discrepancy but RULES.md's
   (R1 to R20 now), since stage 0's front end no longer changes.
6. **The documents the rules touch**, outside stage 1: the conformance
   suite's `.diag` files of `types/list_exhaustiveness` (R10),
   `generics/function_value_errors`, `ownership/view_field`,
   `types/try_context` and `wrong/view_in_struct` (R16), which stage 1's
   `check --format=ids` now gives; 10's `sisu lint` (R11) and the driver's
   routing of it; the decisions 291 to 300 and 313 (RULES.md has their
   text); and the question whether a public type's `Debug` is opt-in
   (above, "The manual's findings"). R1's: decision 269 for
   docs/architecture.md (RULES.md has its text), the prelude's
   documentation of `channel` and `Send`, 06's "Channels and select", and
   architecture.md's owners' bullet, "Send, Sync and what a task
   captures" and known over-rejections 13 and 14.
   Of the second round (R21 to R26, decisions 320 to 325): the
   conformance cases `run/iterators`, `contracts/properties` and
   `generics/loan_through_own`, which make a collection of references
   (R21 refuses them; RULES.md proposes the owned spellings), the `.diag`
   files of `ownership/from_required`, `wrong/missing_from` and
   `traits/method_needs_bound` (R24's fixes), and the manual's
   transcripts that show the diagnostics R24 changes (`E0400` and
   `E0401`'s note in about a dozen, `E0413`, `E0407`, `E0243`, `E0304`,
   `E0300` and `E0340`), which `sisu`, whose front end is stage 1, now
   prints otherwise.
7. **The manual's other findings** (manual/README.md, "What the plan
   found in stage 0"): item 11 has those of the front end the fifth round
   left. Decided against: `enumerate(own xs)` (04 writes `mut` at a call,
   never `own`); NFC names and the confusable lints (D-048) are R136's.
8. **Start-up and speed** (above, "Starting up" and "The compile-time
   evaluator"): the base's prelude, `cap` and `builtin_impls`, parsed and
   resolved again by each command, which a cache of the parsed and
   resolved base itself would remove; and the evaluator's reads through a
   reference of a whole value (`target`). The base cache's write is
   atomic since the sixth round (`Fs.rename`). The thirteenth round's
   cache (R125) took a tenth off a command's start; the parse of the
   base's modules, half of it, is left; the fourteenth round's (R127)
   made the lexer twice as fast and the AST builder a third faster, and
   the parser itself, 0.91 G of a two-line `check`, was left; the
   fifteenth round's (R135) took it to 0.70 G, the command to 3.23 G.
9. **Unsafe code** (above, "Unsafe code"): since round 11 the intrinsics
   of `std.mem` run in stage 0's interpreter, on the checked memory model
   of D-085 with the foreign calls of `extern` blocks emulated, and
   natively (decisions 301 and 302; the conformance suite's
   `cases/unsafe/`, whose differences from the Rust front end R4 lists,
   and the held `layouts` and `raw_buffer`, which R17 and R18 let join
   it), and `packages/trust.diag` holds stage 1's diagnostics. What
   remains: the machine intrinsics of `std.rt` (a panic in the
   interpreter), compile-time code that uses raw memory, the runtime in
   Sisu over them (D-086) past its first slice (R9: the structural hash of
   decision 256, equality, copy and drop, as generic code), `RawBuf` under
   `list` and `String` first; reflection that takes tuples and arrays
   apart and builds an enum's variant, for the slice's last kinds of
   value; and the documents of R2 to R9 (decisions 271 to 278), outside
   stage 1.
10. **The fourth round's rest**, which the fifth round took but for what
    is stage 0's: the driver's `sisu test` and `sisu bench`, which may now
    run stage 1 once (`sir --tests` with the program's warnings and the
    build's options, R47; RULES.md says what the driver may rely on), and
    until then print a program's warnings twice (the driver now runs
    stage 1 once when its probe finds a stage 1 that reports, decision
    424, and may ask `sisu1 --version` instead, R56), and its `sisu
    test` of a package's black-box tests, which stage 1 compiles since
    R65. The host's equality of `Shared` values follows them to their
    values since decision 464, and stage 1's evaluator does at compile
    time since R66. Of stage 1's, left: Python's bare generator as a call's
    one argument, `sum(x for x in xs)`, which 02's parentheses rule out;
    a trait's constant through a type parameter as a constant argument
    (R50 decides against it); the loop directives (R53, 19's N-22).
11. **The manual's findings left** (after R54 to R93), of the front end:
    lowering a variadic parameter and star unpacking (R64 refuses them
    with `E0900`, the first waiting for the owner, since 02 and 14
    disagree), explicit discriminants (R78 refuses them with `E0900`
    until the layouts of `@repr` enums are lowered); the rest of
    "Diagnostics without fixes" that the tenth round left (above, "The
    tenth round") and "Types and traits". The eleventh round took the
    rest of "Diagnostics without fixes" (R104, `E0501` decided against),
    coherence through a trait's arguments (R105), conditional imports
    and an import in a body (R106), `arbitrary[T]` under a bound
    (R107), most of "Types and traits" (R108) and `E0235` at a derive
    (R109); left: a padded `@repr(c)` struct under `Plain` and the
    invariant of a method that returns `Self`, the owner's. The twelfth
    round took the lookups by `str` and `.copied()` (R113), `E0343` for a
    `Shared` (R114; the `zip` temporaries and over-rejection 15 decided
    against), `@preview`'s warning and the exact `truncate_to` (R115),
    `target.runtime` in a native build (R116), `f.doc` for
    `cli.help_text` (R117), the `@export` boundary (R118) and `E0220`
    beside a module (R119); left: the label at an unrolled template's
    line and statements before `return impl`. The thirteenth round took
    over-rejection 15 (R124) and `E0258` alone for a private field
    (R126); decided against: `TotalF64` as a key, which the runtime's
    structural hashing refuses (`E0398`), and `Lookup`'s doc example by
    `str`; left: the label at an unrolled template's line, an item's own
    `##` in reflection, `@cfg` on an import and `when` under
    `--target-runtime`, and statements before `return impl`. The fourteenth
    round took `@cfg` on an import and `when` under `--target-runtime`
    (R128), an item's own `##` and a variant's fields' in reflection
    (R129), checks before `return impl` (R130; a `let` there decided
    against) and the label at an unrolled template's line (R131); decided
    against for the round: NFC names and the confusable lints (R133). The
    fifteenth round took NFC names and the lints (R136), the planned helpers
    (R137), names despite a parse error (R138) and a public type's `Debug`
    (R140, every type keeps it); decided against for the round: `sisu
    doc`'s values (R139). The sixteenth round took reflection on the
    scalar types and its one error (R143), `E0607` alone (R144),
    `reflect.functions_of` (R145), `try_collect` after `map` and the
    borrowed index by a `str` (R146) and Hangul names (R148); decided
    against: `@derive(Into, From)` (R148); left: `try_collect` after the
    other adapters and a compile-time table of function values written
    `comptime`. The seventeenth round took the table of function values
    and an attribute's arguments as values (R151), the missing key's
    panic at the index (R152) and `ptrsize`, `uptrsize`, `Sized`, a list
    literal as an `Array` and a `Url` literal (R153); decided against:
    `try_collect` after the other adapters (R154) and `Decimal`'s `/`
    giving a `Ratio` (R156); left: compile-time types (R155, planned),
    `graphemes` and the normalisation forms, and the sharing of instances
    with `--explain-size`. The eighteenth round took an instance's syntax
    error once and `E0616` for a plain function's compile-time code (R159), closures
    in a `comptime` result (R160), members' attributes (R161), the
    normalisation forms and `graphemes()` (R162) and `Decimal`'s `/` with
    `BigRatio` (R163); decided against: a closure that captures more than
    constants (R160); left: compile-time types (R155), UAX #29's rules
    beyond the tables (R162) and the sharing of instances with
    `--explain-size`. The nineteenth round took `range(len(a.args))` and the
    other builtins in a template's compile-time code (R166); decided
    against for the round: UAX #29's rules beyond the tables (R167); left:
    compile-time types (R155), the sharing of instances with
    `--explain-size`, and `W0022` of an attribute read only through
    `f.attributes`. The twentieth round took `W0022` of members'
    attributes (R171), a slice of a slice in one expression (R172), the
    prelude's functions in a constant (R173), `sisu check --holes` (R174)
    and the prelude's width (R175); decided against: hiding `Formatter`,
    `Spec` and `Hasher`, which the traits take (R175); left: compile-time
    types (R155), the sharing of instances with `--explain-size`, and 10's
    report of a hole past its type and bindings (R174). The twenty-first
    round took `@fix_inline` (R177), constants at compile time (R178),
    TOML's tables and `E0617`'s field (R180), the rest of the hole report
    (R181), a padded `@repr(c)` struct under `Plain` (R182) and
    `days_in_month`'s home (R183); decided against: the inliner of a body
    of several statements (R177) and a test's values at a hole (R181);
    left: compile-time types (R155) and the sharing of instances with
    `--explain-size`. The twenty-second round took the closure for a
    named function in `filter` (R190), `E0117`'s, `E0241`'s and
    `E0333`'s words (R191) and `Decode` of a struct with an enum field
    (R193); decided against: a function value taken for another
    parameter type (R190) and narrowing methods beside `truncate_to`
    (R194); left: compile-time types (R155) and the sharing of instances
    with `--explain-size`. The twenty-third round took `@fix_inline`'s
    parentheses (R197), `E0279`'s and `E0280`'s offsets and fill (R198),
    `sisu fix`'s status (R199), 17's fixtures (R200) and the parser's
    grammar as a command (R202); decided against: 17's `Expectation`,
    which waits for reflected expressions (R200), and the sharing of
    instances with `--explain-size`, which waits for the SIR to name the
    packages that request them (R201); left: compile-time types (R155) and
    a fixture's `scope="module"`, the runner's. The twenty-fourth round
    took `Decode` through a field type's own impl (R204), UAX #29's every
    rule and D-048's lints from Unicode's own data files (R205), the tz
    database's history of `Zone.named`'s zones (R206), `sisu doc
    --digest` (R207) and the mutants of `--mutate` (R208); decided
    against: `std.x.tzdata` and the whole database (R206), `--open`
    (R207), and a module's fixture kept in the SIR (R209); left:
    compile-time types (R155), and the runner's halves of `--mutate` and
    of `scope="module"`, stage 0's. The twenty-fifth round took `E0702`'s
    words (R212), the prelude's `check` in a contract's test (R213), the
    printed grammar against the parser (R214), `@no_alloc` (R215) and the
    fixtures on the test lines (R219); decided against again: `sisu
    doc`'s values (R217); left: compile-time types (R155), `@no_alloc` at
    each instantiation of a generic function (R215), and the runner's
    halves of `--cover`, `--mutate` and a module's fixture, stage 0's. The
    twenty-sixth round took `@no_alloc` at each instance of a generic
    function (R222) and planned D-060's allocators (R225); decided against
    for the round: the allocators' first step, the defaulted parameter on
    the prelude's collections (R225); left: compile-time types (R155), the
    allocators' steps (R225), and the runner's halves of `--cover`,
    `--mutate` and a module's fixture, stage 0's. The tenth round took a variant's
    fields' `ty` and the program's kinds in a derive (R96), `E0203`'s and
    `E0271`'s similar names (R97), `E0310` of sized integers (R98),
    `E0812`'s copy (R99), `E0005`'s invisible characters (R100), most of
    "Diagnostics without fixes" (R101) and the adapters' purity (R102,
    decided against). The ninth round took the REPL's and the
    worksheets' programs (R79), a sized integer shifted by an `int`
    (R80), `expect`'s message (R81), an assertion's call in a call
    (R82), the `@refines` test's place (R83), a hole reached at compile
    time (R84), `E1022`'s line (R85), the manifest's `edition` (R86),
    the formatter's unused imports (R87), `E0402` and `E0428` of tasks
    (R88), a `?` in a scope's body of a `T !` function (R89), `E0335`'s
    operation (R90), `E0204`'s `@cfg` (R91), a field's `ty` in a derive
    (R92) and a function named `type` (R93). Decided against: `it`
    outside a call's argument, the `own self` method and `zip`'s
    temporaries (above, "The eighth round").
12. **The driver's `sisu repl` and `sisu ws` on stage 1** (R79, R95),
    which is stage 0's: the driver may pass `--dir` and `--name` to `sir
    --repl` and `sir --worksheet`, and read `:why NAME`'s inputs from
    `parse --bindings`; the language server's inline values of a worksheet
    (`evaluate_sheet`) can take `query values` (R196), whose evaluator
    stops at the world and at the intrinsics it lacks (`list.first`,
    `list.slice`), where the driver's interpreter goes on.
