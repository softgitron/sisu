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
--unsafe`, `std.rt`, and the runtime's first slice in Sisu.

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
sisu test                                   # 139 tests, 76 of them stage 0's own, transcribed
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
sisu run -- sir [--partial] [--contracts=LEVEL] [PATH]  # what `sisu sir [--contracts=LEVEL] [PATH]` prints: the whole program's SIR
sisu run -- ownership FILE                  # the ownership checker's diagnostics of `sisu check --format=ids FILE`
sisu run -- sir-batch OUT LIST [--contracts=LEVEL]  # both, for every file LIST names, into OUT (with a level, the SIR alone)
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
./sisu1 (check | sir | build | run | fmt | doc | audit) [PATH] ...  # stage 1 as the compiler's front end, from anywhere
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
built with its tests, as `sisu check` builds it. `sir --partial` prints
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

With `--native` it builds stage 1 once with stage 0's native backend
(`sisu build --native --output WORK/stage1`, about 5 s for its 2450 SIR
functions) and runs that executable for every batch instead of `sisu run
--` on the interpreter; nothing else changes, but for the class `build`,
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
documentation's, and now `behaviour`, takes eight and a half to twelve
minutes, as the machine goes (this round's last, below, eight and a
half); the
interpreter's without them (but `human`, `cases-check` and `fmt`) about
thirteen on a quiet machine, and the formatter's classes add about five
(the documentation's were not timed there).

## Results

`./check.sh --native --names --types --sir` (`--native` implies `--self` and
`--check`):

```text
stage 1 against stage 0 (tokens, tree, trivia-tree, print, ast):
  conformance   536 files,  534 identical,   2 differing as listed,   0 differing otherwise; stage 1    1.9s (370902 bytes; lex 101 ms, parse 107 ms, dumps 510 ms, ast 377 ms)
  std            31 files,   28 identical,   3 differing as listed,   0 differing otherwise; stage 1    1.0s (453452 bytes; lex 62 ms, parse 65 ms, dumps 299 ms, ast 280 ms)
  examples       11 files,   10 identical,   1 differing as listed,   0 differing otherwise; stage 1    0.1s (27495 bytes; lex 4 ms, parse 5 ms, dumps 30 ms, ast 25 ms)
  cases          86 files,   66 identical,  20 differing as listed,   0 differing otherwise; stage 1    0.2s (60705 bytes; lex 12 ms, parse 11 ms, dumps 57 ms, ast 38 ms)
stage 1's name resolution against stage 0's (names):
  names          96 files,   85 identical,  11 differing as listed,   0 differing otherwise; stage 1    0.2s
  corpus-names  491 files,  490 identical,   1 differing as listed,   0 differing otherwise; stage 1    0.5s
stage 1's type checker against stage 0's (types):
  types         542 files,  539 identical,   0 beyond the slice and a subset,   3 differing as listed,   0 otherwise; stage 1    4.0s
  corpus-types  527 files,  525 identical,   0 beyond the slice and a subset,   2 differing as listed,   0 otherwise; stage 1    4.6s
stage 1's lowering against stage 0's (sir):
  sir           542 files,  540 identical,   0 beyond the slice,   2 differing as listed,   0 otherwise; stage 1   13.3s (front end 4s, lowering 3s, ownership 4s)
  corpus-sir    527 files,  526 identical,   0 beyond the slice,   1 differing as listed,   0 otherwise; stage 1   13.9s (front end 4s, lowering 4s, ownership 4s)
stage 1's ownership checker against stage 0's (ownership):
  ownership     542 files,  542 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise (the same run)
  corpus-own    527 files,  527 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise (the same run)
stage 1's lowering at the contract levels of a build against stage 0's (contracts):
  contracts-off              16 files,   16 identical,   0 differing as listed,   0 otherwise; stage 1    1.5s
  contracts-preconditions    16 files,   16 identical,   0 differing as listed,   0 otherwise; stage 1    1.8s
stage 1's SIR run against the cases' expectations (behaviour: interpreted and native, from `sisu sir` as stage 1 prints it):
  behaviour     206 programs,  206 as expected interpreted and natively,   0 otherwise; stage 1    9.0s, the runs   20.3s
stage 1's check against stage 0's (every diagnostic, in the human and the conformance form):
  human          93 files,   93 identical,   0 beyond the slice,   0 differing as listed,   0 otherwise; stage 1    0.9s
  corpus-check  527 files,  523 identical,   0 beyond the slice,   4 differing as listed,   0 otherwise; stage 1    8.8s
  corpus-human  527 files,  523 identical,   0 beyond the slice,   4 differing as listed,   0 otherwise (the same run)
  cases-check    64 files,   43 identical,   0 beyond the slice,  21 differing as listed,   0 otherwise; stage 1    1.6s
  cases-human    64 files,   43 identical,   0 beyond the slice,  21 differing as listed,   0 otherwise (the same run)
  expected       19 files,   19 as their .diag or .audit expects,   0 otherwise
stage 1's formatter against stage 0's (fmt: the text `sisu fmt` leaves, what `sisu fmt --check` prints, and its fixed point):
  fmt           748 files,  748 identical,   0 differing as listed,   0 otherwise; stage 1    5.7s
  fmt-check     748 files,  737 identical,  11 differing as listed,   0 otherwise (the same run)
  fmt-again     748 files,  748 unchanged,   0 otherwise (the same run)
  fmt-joined    748 files,  737 identical,  11 differing as listed,   0 otherwise; stage 1   15.2s
  fmt-spread    748 files,  737 identical,  11 differing as listed,   0 otherwise; stage 1   15.2s
stage 1's documentation against stage 0's (doc: what `sisu doc` prints and its status, and the pages `--output` writes):
  doc           597 programs,  571 identical,  26 differing as listed,   0 otherwise; stage 1   15.8s
  doc-output    597 programs,  571 identical,  26 differing as listed,   0 otherwise (the same run)
stage 1's build against stage 0's (build: the executables, the lockfile and what each prints):
  build          16 builds,   16 identical,   0 otherwise; both   14.3s
stage 1 over its own package against stage 0 (self: names, types, the SIR by source file):
  self-names   identical
  self-types   identical
  self-sir      116 files,  116 identical,   0 beyond the slice,   0 otherwise; stage 1   28.1s
the fixed point (fixed: stage1-A built by stage 0, stage1-B by the backend from stage1-A's SIR):
  fixed-sir0   identical: stage1-A's SIR of its own package and stage 0's (16768354 bytes)
  fixed-sir    identical: stage1-B's SIR of the package and stage1-A's (16768354 bytes)
  fixed-exe    identical: stage1-B and stage1-A (23909392 bytes)
  stage1-A 29.9s, the backend 6.6s, stage1-B 29.0s
total: 11061 files checked, 10880 identical, 181 differing (181 listed in RULES.md or DISCREPANCIES.md, 0 not)
```

The 181 differences (each a `listed:` line of the run, left out above) are
those of RULES.md's rules, for the programs each touches as RULES.md
lists them: R1's three cases of `cases/channels/`, and R2 to R9's, the
cases of `cases/unsafe/` and `cases/runtime/`, the two package cases with
`unsafe` levels, the std modules only stage 1 reads (`stage1/std/`, whose
syntax stage 0 does not parse, and what `doc --std` writes of them), the
conformance suite's `packages/trust.pkg` and the proposal's
`examples/ffi.sisu`, whose unsafe code stage 1 checks, and the three of
stage 1's older cases that showed `unsafe` as a later phase's. Every
runnable program, those stage 0 refuses among them, does what its case
says, from stage 1's SIR, interpreted and natively (`behaviour`).

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
exactly; `types.list` and `sir.list` name the 525 and stage 1's 17 cases
of one module (three of type rules, ten of diagnostics no program of the
corpus has, `doc/layout.sisu`, and the three of `cases/channels/`, which
show R1 of RULES.md). Of the 542 of `sir.list`, 346 are lowered by stage
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
  stage1-A   23356960 bytes  sha256 8fc44e21b955904b729e12261869a589e10addbc6012a85243d0615c7456700f  built by stage 0 (7.5 s)
  A.sir      16269545 bytes  sha256 adaa0fe804428cc7af0c8e937bed98c27098a3154758c64b2da4ad0f5f2f9ba2  printed by stage1-A (23.8 s)
  stage1-B   23356960 bytes  sha256 8fc44e21b955904b729e12261869a589e10addbc6012a85243d0615c7456700f  built by the backend from A.sir (3.4 s)
  B.sir      16269545 bytes  sha256 adaa0fe804428cc7af0c8e937bed98c27098a3154758c64b2da4ad0f5f2f9ba2  printed by stage1-B (24.4 s)
  stage0     16269545 bytes  sha256 adaa0fe804428cc7af0c8e937bed98c27098a3154758c64b2da4ad0f5f2f9ba2  printed by stage 0 (sisu sir)

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
sisu1 sir [--contracts=LEVEL] [PATH]          the whole program's SIR, as `sisu sir` prints it
sisu1 build [--release] [--contracts=LEVEL] [--locked] [FEATURES] [PATH] [--output OUT]
                                              what `sisu build --native` makes, and where
sisu1 run [FEATURES] [PATH] [-- ARGS...]      the program built into a temporary directory and run
sisu1 fmt [--check] [PATH...]                 each file formatted in place, or what would change, as
                                              `sisu fmt` does it, with its exit status
sisu1 doc [--locked] [FEATURES] [PATH] [--output DIR]
                                              the documentation, a Markdown page per module, printed
                                              or written into DIR, as `sisu doc` makes it
sisu1 doc --std [MODULE...]                   the standard library's pages, or those of the modules named
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
package declares counted toward the caller's level; `@export`, the header
import and `@blocking`; `sisu publish`'s refusal of undocumented unsafe
code; the teaching profile's and hot replacement's rules, which stage 1
has neither of; and, of the runtime in Sisu, everything past R9's slice
(`RawBuf` under `list` and `String`, `Box`, `Shared`, the stack pool and
the scheduler), which waits for the intrinsics above.

## Module layout

| Module | Lines | What | Stage 0 original |
|---|---:|---|---|
| `syntax.kinds` | 547 | `SyntaxKind` (246 kinds), the keyword and punctuation tables, kind predicates, names in dumps | `sisu-syntax/src/kinds.rs` |
| `syntax.tree` | 357 | `Token`, the arena tree, the builder with checkpoints, the dumps, the reprint, stable identities | `sisu-syntax/src/tree.rs` |
| `diag` | 1667 | `Msg` (the catalog identifiers stage 1 reports), diagnostics with labels, fixes and notes, the catalog reader and its templates, positions, `render_ids`, the human form (`render_human_files`: source lines, labels, notes and fixes), `quote`, the stable sort and `sort_unique` | `sisu-diag` (the parts the front end uses, and `render.rs`) |
| `text`, `text.unicode` | 369 | UTF-8 decoding, `is_alphanumeric`, Rust's `Debug` quoting, and the Unicode tables it needs (pinned, Unicode 17.0.0) | the host's `char` methods and `{:?}` |
| `lexer` | 262 | `lex`, and stage 0's lexer tests | `sisu-lexer/src/lib.rs`, `tests.rs` |
| `lexer.scan` | 558 | the raw scanner: every byte a token, strings and interpolation as tokens | `sisu-lexer/src/scan.rs` |
| `lexer.layout` | 347 | Python's layout algorithm, continuation, bracket recovery and its fixes | `sisu-lexer/src/layout.rs` |
| `parser` | 516 | `parse`, the diagnostic filters (bracket echoes, structure echoes, one per line), and stage 0's parser tests | `sisu-parser/src/lib.rs`, `tests.rs` |
| `parser.state` | 403 | the parser's state: looking, attaching tokens with their trivia, reporting, the fixes added later, Python's conditional | `sisu-parser/src/parser.rs` |
| `parser.items`, `.stmts`, `.exprs`, `.patterns`, `.types` | 2249 | one function per rule of the grammar | the files of the same names |
| `syntax.ast` | 765 | the AST: items, statements, expressions, types and patterns in arenas, referred to by id | `sisu-syntax/src/ast/mod.rs` |
| `syntax.ast.build` | 2160 | `build.module`, the AST of a tree as a total function, and the decoding of literals | `sisu-syntax/src/ast/build.rs` |
| `syntax.ast.dump` | 1425 | the AST in the notation of Rust's derived `Debug`, laid out as `{:#?}`, for `check.sh` | Rust's `{:#?}`, which `sisu parse --ast` prints |
| `syntax.ast.visit` | 311 | the expressions of items, blocks and expressions, each before the ones inside it | `sisu-syntax/src/ast/visit.rs` |
| `resolve` | 2491 | the names of a program of one module or of several modules and packages: declarations, imports, scopes, the prelude's and std's names, and the name diagnostics | `sisu-resolve/src/collect.rs` and `types.rs`, and the name lookups of `sisu-typeck` |
| `toml`, `manifest`, `package` | 2279 | the TOML subset of manifests, the manifest's tables and their diagnostics, and a program's layout: its packages, their dependencies and trust, the module trees, and where its lockfile goes | `toml.rs`, `manifest.rs` and `package.rs` of `sisu-driver` |
| `cfg` | 257 | conditional compilation: the items, test items and methods a build leaves out by their `@cfg`s, with `not(...)` (decision 250), and the problems of the conditions | `sisu-driver/src/cfg.rs` |
| `sir`, `sir.types`, `sir.write`, `sir.simplify` | 1654 | SIR: programs, functions, blocks, statements and types, with the operations a type's constant argument may compute; the types of places; the text form `sisu sir` prints; jump threading | `lib.rs`, `types.rs`, `write.rs` and `simplify.rs` of `sisu-sir` |
| `typeck.ty` | 451 | types, interned in a table | `sisu-resolve/src/ty.rs` |
| `typeck.program` | 2393 | the resolved program, the spelling of types, `Copy`, the trait solver, `Send` and `Sync` (`Send` as a bound owning what it holds, RULES.md's R1), visibility, views and must-consume types | `program.rs`, `ty.rs`, `solve.rs` and `auto.rs` of `sisu-resolve` |
| `typeck.borrows` | 388 | what a signature borrows: the parameters a result may borrow from (`from`, and the elision rules), the borrowed-signature errors | `sisu-resolve/src/borrows.rs` |
| `typeck.std_sources` | 578 | `target` and `builtin_impls`, the std text stage 0 writes, the import paths of the std modules, and those `sisu doc --std` documents | `sisu-resolve/src/std_sources.rs` |
| `typeck.collect` | 5045 | declarations, signatures, imports, traits and impls of a program of several modules and packages, resolved, and what the compiler writes: interceptors and fakes, invariants, predicates, a script's `main`, test cases, doc tests | `collect.rs`, `types.rs`, `traits.rs`, `shims.rs`, `tests_of.rs` and `doctests.rs` of `sisu-resolve` |
| `typeck.template` | 2347 | the evaluator of compile-time code that unfolds a template (a `comptime def derive`, a generic function that reflects on its type parameters) into source text | `sisu-resolve/src/template.rs` |
| `typeck.instances` | 1175 | the impls `@derive` and `@delegate` make, the program's and the standard library's, and the instances of templates, declared in modules of their own | `derive.rs` and `instances.rs` of `sisu-resolve` |
| `typeck.arbitrary` | 1386 | the generators of property tests, and the tests of contracts and `@refines` | `sisu-resolve/src/arbitrary.rs` |
| `typeck.consts`, `typeck.folding` | 341 | a type's constant argument written as an expression, folded where it is written; an expression as folding reads it | `sisu-resolve/src/consts.rs` |
| `typeck.infer`, `typeck.thir` | 771 | inference variables and unification; the typed tree | `infer.rs` and `thir.rs` of `sisu-typeck` |
| `typeck.check`, `.expr`, `.call`, `.pat` | 7398 | the body checker: statements, expressions, calls, patterns | `check.rs`, `expr.rs`, `call.rs` and `pat.rs` of `sisu-typeck` |
| `typeck.exhaustive` | 572 | exhaustiveness and unreachable arms of `match` | `sisu-typeck/src/exhaustive.rs` |
| `typeck.contracts`, `.purity`, `.tasks` | 1561 | contracts and refinements, `pure`, structured concurrency and generators, and what a channel carries (R1) | `contracts.rs`, `purity.rs` and `tasks.rs` of `sisu-typeck` |
| `typeck.safety` | 437 | unsafe code (R2, R3, R5): the blocks and expressions that permit the unsafe operations, the calls that are ones, the `# safety:` comments and `Safety:` lists, and the lints | stage 1's own (22) |
| `typeck.generic_asserts` | 319 | `comptime assert` on `const` parameters, decided at each call | `sisu-typeck/src/generic_asserts.rs` |
| `typeck` | 2244 | a body's checks, a program's, the inferred error sets, the driver, the typed bodies lowering reads, and the standard library's bodies, checked against the base and kept once per process; a file's declarations resolved alone, for the formatter | `sisu-typeck/src/lib.rs` |
| `lower`, `lower.names`, `.func`, `.stmts`, `.exprs`, `.pats`, `.contracts`, `.glue` | 6315 | lowering the typed bodies to SIR: what `main` and the tests reach, names, functions and closures (with the value wrappers of decision 225), statements, expressions, patterns, contracts, drop glue, and the standard library's functions and what names them kept once per process | `lib.rs`, `func.rs`, `stmts.rs`, `exprs.rs`, `pats.rs`, `contracts.rs` and `glue.rs` of `sisu-lower` |
| `borrowck`, `borrowck.paths`, `.init`, `.elaborate`, `.loans`, `.consume`, `.purity`, `.shared`, `.names`, `.types`, `.bitset`, `.finding` | 6474 | the ownership checker: move paths, initialisation and moves, drop placement (the standard library's kept once per process), loans, must-consume, purity, the `Shared` escape analysis, places in source terms, and the findings as diagnostics | `sisu-borrowck` (but `explain.rs`) |
| `interp` | 1972 | compile-time evaluation: the `comptime` functions run on SIR and their values baked into it, or their failures, with the panics' messages | `sisu-interp/src/comptime.rs`, with what it needs of `exec.rs`, `place.rs`, `ops.rs`, `dispatch.rs`, `intrinsics.rs`, `sized.rs` and `format.rs` |
| `fmt`, `fmt.doc`, `fmt.lines`, `fmt.prune`, `fmt.shape`, `fmt.space` | 2142 | the formatter: logical lines with the comments and blank lines between them, sorted and pruned imports, the printer of groups, the spaces between tokens, and the shape its self-check compares | `sisu-fmt` (`lib.rs`, `doc.rs`, `lines.rs`, `prune.rs`, `shape.rs`, `space.rs`, `tests.rs`) |
| `format` | 57 | the formatter with the names it needs: which written types name error sets, in a file resolved alone | `sisu-driver/src/format.rs` |
| `doc` | 1164 | `sisu doc`: a Markdown page per module of a program or of the standard library, from the modules' syntax, the resolved names and the inferred error sets, and its tests | `sisu-driver/src/doc.rs` |
| `diff` | 278 | line diffs: the hunks `fmt --check` prints and the change `--locked` prints, from the longest common subsequence's table kept as rows of bits | `sisu-interp/src/diff.rs` |
| `mutants` | 204 | a seeded generator and the mutations of stage 0's robustness test; the files laid out otherwise that the formatters are compared on | `sisu-driver/src/tests.rs` |
| `main` | 3191 | the command line, and the driver: `check` in both forms, the SIR of a whole program, `build --native` through stage 0's backend, `fmt`, `doc`, `audit --unsafe`, and the packages' `unsafe` levels (R6) | `sisu-driver/src/cli.rs`, `tokens`, `parse`, `check`, `sir`, `build`, `fmt` and `doc` |
| `audit` | 420 | `audit --unsafe` (R7): the unsafe code of a program by package and module, with the modules' hashes | stage 1's own (22) |
| `stage1/std/mem`, `ffi`, `rt` | 1023 | `std.mem`, `std.ffi` and `std.rt` (R4, R8), the standard library's modules only stage 1 reads, and the runtime's first slice in Sisu (R9) | stage 1's own (22) |

68240 lines in all, about 1690 of them tests, beside the 1023 of `stage1/std/`; the Rust the lexer and the
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

## What is next

Every part of 18's stage table for stage 1 exists, `sisu doc` the last
(above, "Documentation"); what remains is beyond it, or is speed.

1. **`sisu doc`'s parts that need evaluation**: 10's rendering of an
   example with its values, `--open` and `--digest` (decision 258 leaves
   them out, port-debt.md), which stage 0 does not have either.
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
   (R1 to R9 now), since stage 0's front end no longer changes.
6. **The documents R1 touches**, outside stage 1: decision 269 for
   docs/architecture.md (RULES.md has its text), the prelude's
   documentation of `channel` and `Send`, 06's "Channels and select", and
   architecture.md's owners' bullet, "Send, Sync and what a task
   captures" and known over-rejections 13 and 14.
7. **Unsafe code** (above, "Unsafe code"): the intrinsics of `std.mem`
   and `std.rt` in stage 0's interpreter (with the checked memory model of
   D-085) and native backend, after which the raw memory operations run and
   compile-time code may use them; the runtime in Sisu over them (D-086)
   past its first slice (R9: the structural hash of decision 256, equality,
   copy and drop, as generic code), `RawBuf` under `list` and `String`
   first; reflection that takes tuples and arrays apart and builds an
   enum's variant, for the slice's last kinds of value; the conformance
   suite's `packages/trust.diag` and the documents of R2 to R9 (decisions
   271 to 278), outside stage 1.
