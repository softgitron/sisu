# The conformance suite

Language-agnostic tests for any Sisu implementation
([18](../../proposal/18-next-steps.md), "Bootstrapping": "input files,
expected SIR, expected diagnostics by catalog identifier"). The runner
drives an implementation only through the `sisu` command line, so the
stage-1 compiler written in Sisu is checked by the same files as the Rust
stage 0.

Run it with `cargo test -p sisu-driver --test conformance` (the stage-0
`sisu` is built first), or against any executable:

```text
cargo run -p sisu-conformance -- --sisu path/to/sisu conformance/cases
cargo run -p sisu-conformance -- --sisu path/to/sisu --front-end=rust conformance/cases
```

**The front end.** The `sisu` of `compiler/` runs stage 1, the front end
written in Sisu, compiled natively (`compiler/target/stage1/sisu1`, built
from `compiler/stage1` by the Rust front end when a source of it is newer),
for `check`, `sir`, `build`, `run`, `test`, `bench`, `fmt`, `doc`,
`tokens` and `parse` (`--shape` too, decision 622): their diagnostics,
SIR and pages are stage 1's, and
a program runs from the SIR stage 1 serialised, in the interpreter or
natively, as its tests and benchmarks do
([architecture.md](../docs/architecture.md), decisions 268 and 357). The
commands stage 1 does not serve yet run on the Rust front end: `test
--std`, `test --defer-borrow`, the tests of a program with transcripts,
`run --watch`, `run --record`, `replay`, `run --defer-borrow` and
`explain-borrow` ([port-debt.md](../docs/port-debt.md), "The Rust front
end's roles"); `repl` and `ws` build their programs with stage 1's `sir
--repl` and `sir --worksheet` once it says it serves them (`sisu1 0.8.0`,
decisions 563 and 620; from `sisu1 0.9.0` with `--dir` and `--name`, and
`:why` from `parse --bindings`, decision 640), and `cargo test` runs the
suite's sessions and
worksheet through stage 1 too, with `SISU_STAGE1_REPL=1`.
`cargo test` runs the suite so, with stage 1 as the front end.
`--front-end=stage1` or `--front-end=rust` asks the executable for that
front end, through `SISU_FRONT_END` in the empty environment the runner
gives every command; `--front-end=rust` checks the Rust front end, the
oracle, against the same files, which CI's `stage1` workflow does, allowed
to fail, since new rules are written in stage 1 only and the Rust front
end falls behind (decision 268).

`--bless` (or `SISU_BLESS=1` for the cargo test) rewrites the expected
files from what the implementation printed instead of failing. Review the
diff before committing a blessed file: blessing records behaviour, it does
not check it.

## A case

A case is a file `NAME.sisu` anywhere under `cases/`. Files beside it with
the same stem state what an implementation must print. Every expectation
is compared byte for byte.

| File | Compared with |
|---|---|
| `NAME.diag` | `sisu check --format=ids NAME.sisu`; absent means no diagnostics |
| `NAME.out` | standard output of `sisu run NAME.sisu ARGS...` |
| `NAME.err` | standard error of that run; absent means empty |
| `NAME.status` | exit status of that run; absent means `0` |
| `NAME.args` | not compared: the run's arguments, one per line |
| `NAME.stdin` | not compared: what every run of the program reads as standard input |
| `NAME.files/` | not compared: copied into the run's working directory (fixtures, `NAME.snap` snapshots, `tests/*.transcript` transcripts) |
| `NAME.sir` | `sisu sir NAME.sisu`, the serialised SIR |
| `NAME.test` | standard output of `sisu test NAME.sisu` |
| `NAME.doc` | standard output of `sisu doc NAME.sisu`, the documentation's pages, or of `sisu doc ARGS` when a line `# doc ARGS` gives the arguments (`# doc --std std.time`) |
| `NAME.tokens` | `sisu tokens NAME.sisu` |
| `NAME.tree` | `sisu parse --tree NAME.sisu` |
| `NAME.defer` | `sisu run --defer-borrow NAME.sisu ARGS...`: standard output, then `--- stderr`, standard error, and `--- status N` |
| `NAME.monitor` | `sisu run --monitor NAME.sisu ARGS...`: standard output, then `--- stderr`, standard error, and `--- status N`, for a case whose run under the monitor stops where the plain run does not (below) |
| `NAME.explain` | `sisu explain-borrow NAME.sisu:N`, where `N` is given by a line `# explain-borrow N` in the case |
| `NAME.explore` | `sisu test --explore N NAME.sisu`, where `N` is given by a line `# explore N` in the case |
| `NAME.sim` | `sisu test --sim ARGS NAME.sisu`, where `ARGS` (such as `--faults 0.05 --explore 5`) is given by a line `# sim ARGS` |
| `NAME.schedule` | `sisu test ARGS NAME.sisu`, where `ARGS` (such as `--scheduler pct --depth 2 --explore 20`) is given by a line `# schedule ARGS` |
| `NAME.select` | `sisu test ARGS NAME.sisu`, where `ARGS` (such as `--tag slow --filter x`) is given by a line `# select ARGS` |
| `NAME.watch` | `sisu run --watch NAME.sisu ARGS...`: standard output, then `--- stderr`, standard error, and `--- status N`; the program saves new versions of its own file from `NAME.files/` to drive the swaps |
| `NAME.repl` | `sisu repl NAME.sisu` with `NAME.stdin` as the session's inputs: standard output, then `--- stderr`, standard error, and `--- status N` |
| `NAME.ws` | `sisu ws NAME.sisu`, the worksheet: standard output, then `--- stderr`, standard error, and `--- status N` |
| `NAME.unformatted` | not compared: why the case is not in the form `sisu fmt` writes (its layout is what it tests, or it holds a proposal example unchanged); without it the case must be |

A line `# explore N` also runs a case that runs again as `sisu run
--explore N NAME.sisu ARGS...`: the program under the schedule seeds 0 to
N-1, each of which must give the expected output and status.

A case with `NAME.doc` also runs `sisu doc` again with `--output DIR`,
which must write exactly the pages the first run printed, each into the
file its line `<!-- FILE -->` names, and exit as it did.

A case is run when any of `.out`, `.err` or `.status` exists.

Every command runs in a fresh temporary directory holding only
`NAME.sisu` and the contents of `NAME.files/`, with an empty environment,
and names the file by its bare name, so paths in messages are the same on
every machine. A program that writes files writes them there.

## A package case

A directory `NAME.pkg/` is a package case (09): a package with its
`sisu.toml` and `src/` tree, a workspace, or several packages beside each
other. Every command runs in it, or in the directory `NAME.cwd` names
(relative to `NAME.pkg/`), without naming a file, as a programmer runs
them. The files beside it are a file case's (`NAME.diag` for `sisu check
--format=ids`, `NAME.doc` for `sisu doc`, `NAME.test`, `NAME.out`,
`NAME.err`, `NAME.status`, `NAME.args`, `NAME.stdin`, `NAME.unformatted`,
`NAME.watch` for `sisu run --watch`, `NAME.repl` for `sisu repl` with the
package loaded), and these:

| File | Compared with |
|---|---|
| `NAME.lock` | the `sisu.lock` that `sisu check` wrote, beside the package's manifest or its workspace's |
| `NAME.build` | `sisu build`: standard output, then `--- stderr`, standard error, and `--- status N` |
| `NAME.cwd` | not compared: where the commands run |

The program's arguments follow `--` (`sisu run -- ARGS...`). The checks
below apply to every `.sisu` file of the package.

## Checks without an expectation file

These checks apply to every case without an expectation file:

- `sisu parse --print NAME.sisu` reproduces the file byte for byte (the
  syntax tree is lossless).
- For a case that runs, the SIR is a complete program: `sisu sir` written
  to a file and run with `sisu run --sir` gives the same standard output,
  standard error and exit status as running the source (minus any
  compile-time warnings that precede the program's own output).
- For a case that runs, `sisu run --monitor` (the dynamic ownership
  monitor) prints exactly what `sisu run` printed and exits the same way.
  A case that runs was accepted by the ownership checker, so any
  difference is the monitor stopping a program the checker accepted: a
  soundness hole in the checker, which fails the suite (18, principle 5,
  "the checker checks itself"). The one exception is unsafe code, which
  no static checker checks: a case with a `NAME.monitor` expectation is
  one whose aliasing violation the monitor's checked memory model stops
  (D-085), and its monitored run is compared with that file instead
  (decision 303).
- For a case that runs, `sisu run --record LOG` prints and exits as the
  plain run did, and `sisu replay LOG` reproduces that run from the
  capability log alone (N-04); a run through stage 1 keeps the SIR it
  ran in the log, which the replay runs without a front end (decision
  303). A program its errors stop is not run and writes no log, so it
  has nothing to replay: a case pins what its run prints with `.err` and
  `.status` (decision 427).
- Every failing test in `.test`, `.sim`, `.schedule` and `.select` output
  prints a replay token; `sisu test --replay TOKEN NAME.sisu` must fail
  that test again with the same report.
- For a case with `.test`, `sisu test --jobs 1` (the tests on one
  thread) prints what the default, parallel run printed.
- Every run of `sisu test` compared with a `.test`, `.explore`, `.sim`,
  `.schedule` or `.select` exits as its report says (decisions 340 and
  380): 1 when a test failed, 3 when none failed and one is blocked by a
  typed hole, 0 when every test passed, counted over its summary lines; a
  run that printed none, or an error of the program, is held to nothing
  here.
- For a case with `.watch`, `sisu run --watch --record LOG` prints what
  the watched run printed, and `sisu replay LOG` reproduces it, its
  `edit` records standing in for the saves.
- Every file that parses (`sisu parse --shape` exits 0; it exits 1 for a
  syntax error, and any other status is a failure) keeps its tree when
  `sisu fmt` rewrites it: `sisu parse --shape` prints the same before and
  after (the tree's nodes and significant tokens, up to what the
  formatter changes on purpose: trailing commas, `Option[T]` written
  `T?`, `Result[T, E]` written `T ! E`, unused imports, and the order of
  imports), and formatting it a second time changes nothing. Unless the
  case has a `NAME.unformatted`, `sisu fmt --check` must also accept it
  as it is: the case is canonical text.

The summary line counts the cases (and the package cases among them),
the checks, the files formatted, the runs under `--watch`, the runs
repeated under the monitor,
the runs recorded and replayed, the failing tests replayed from their
tokens, the test runs repeated on one thread, the counterexample runs
(`.defer`), and the cases explored under several schedules with the
number of schedules.

## The diagnostic form

`--format=ids` prints one line per diagnostic, then one indented line per
label and fix, naming messages by code and catalog identifier, never by
English text:

```text
3:11-3:16 E0200 resolve.undefined-name name="cuont"
  fix resolve.similar-name 3:11-3:16 "count"
```

Positions are `line:column-line:column`, 1-based, columns counted in
characters. Parameters follow in the order the implementation gives them;
the catalog (`compiler/catalog/messages.catalog`) defines which
parameters each identifier takes. The lines are sorted by position, then
code, and a diagnostic identical to another (the same identifier,
parameters, positions, labels and fixes) is printed once, however many
times the implementation found it (the cases of a parametrised test share
one body: `testing/parametrised_once`).

## The native mode

`native.list` names the cases the native backend (stage 2;
[architecture.md](../docs/architecture.md), "Stage 2") must build and
run, one per line as the runner names them (`run/arithmetic`); `#`
starts a comment. A case marked `checked` after its name (`unsafe/released
checked`) stops with the checked memory model's report, which a debug
build gives as the interpreter does (decisions 543, 544, 560 and 561):
`--native-release`, whose raw memory is unchecked, builds it and does not
run it. With `--native LIST` the runner builds each case with
`sisu build --native --output EXE NAME.sisu` (a package case without the
file) in the case's sandbox, runs the executable as it runs `sisu run`
(with the case's `.args`, `.stdin` and `.cwd`, the time limit, and an
environment holding only `SISU_RT_ALLOCS=1`), and compares:

- standard output with `NAME.out`;
- the exit status with `NAME.status` (absent means `0`);
- standard error with `NAME.err` (absent means empty), except that when
  the build printed warnings, which a `sisu run` prints before the
  program's own standard error, the program's must be the end of
  `NAME.err`;
- the runtime's report of its allocations live at exit, the last line of
  the program's standard error (`allocations: N of the runtime's live at
  exit`), which must say 0 and is left out of the comparison above; a
  program that aborts (status 134) ends without one.

The expectations are the interpreter's: the native mode reads no file of
its own and does not bless, except `NAME.release` (below). A native build lowers SIR through stage 2, the
lowering written in Sisu, and through nothing else unless `--lower=rust`
asks for the Rust lowering, the oracle (decisions 304 and 305), and a debug
build's LIR is compiled by the x86-64 backend written in Sisu, a release
build's by Cranelift (decision 331): the modes below that build with the
default lowering (`--native`, `--native-from-sir`, `--native-release` and
`--native-cranelift`) run each build with `SISU_NATIVE_TIMES=1` and fail a
case whose build does not say that stage 2 lowered it (`native: lowered by
stage 2 in T ms`), and the two debug ones (`--native`, `--native-from-sir`)
one whose build does not say that the Sisu backend compiled it (`native:
compiled by the Sisu backend in T ms`); their summary counts those cases
and the time each took over them. With `--native-from-sir LIST` the runner
builds each case from serialised SIR instead, `sisu build --native
--from-sir NAME.sir`, with the case's own `NAME.sir` where it has one and
what `sisu sir` prints for it otherwise, and compares the same way (a
case whose `NAME.diag` holds a warning, or whose `sisu sir` printed one,
has the front end's warnings before the program's standard error, as a
`sisu run` prints them; stage 1's `sir` prints none): the path a self-hosted front end's
SIR takes to native code. With `--native-release LIST` it builds each case
with the release profile, `sisu build --native --release
--contracts=all`: every contract checked, as the interpreter checks them,
so that what is compared is what Cranelift's optimisations do; a case with
a `NAME.release` file is one a release build refuses (a typed hole,
decision 340), and the runner compares the build's standard error with it
instead of running the program. With
`--interpreted-from-sir LIST` it runs each case's SIR, the file
`--native-from-sir` builds, in the interpreter instead (`sisu run
--from-sir NAME.sir ARGS...`), and compares standard output, the status
and standard error as above, without the report of allocations: the path
a self-hosted front end's SIR takes to be run rather than compiled.
With `--native-from-lir LIST` it lowers each case's SIR, the file
`--native-from-sir` builds, with stage 2 (`compiler/stage2`, the backend
written in Sisu: `sisu2 lir NAME.sir --output NAME.lir`), builds the LIR
with `sisu build --native --from-lir NAME.lir`, runs the executable and
compares as above, and requires the executable to be byte for byte the
one `--from-sir --lower=rust` builds from the same SIR through the Rust
lowering, the oracle, and the release profile's (`sisu2 lir --release`)
the one `--release --from-sir --lower=rust` builds; a
construct stage 2 does not lower yet fails the case with stage 2's
message. Stage 2 is the
executable `--sisu2 PATH` names, else the runner builds
`ROOT/../../stage2` with the `sisu` it tests (`sisu build --native`)
before the cases.
The LIR is translated by Cranelift there (`--backend=cranelift`), whose
executable is the Rust lowering's. With `--native-cranelift LIST` it
builds each case with Cranelift, the oracle of the Sisu backend (`sisu
build --native --backend=cranelift`, decision 331), and compares the runs
as `--native` does, so that every case is held to the interpreter's
behaviour under both backends; the executables of the two are their own,
so nothing is compared byte for byte between them.
A case of the list that pins `sisu test` (`NAME.test`, and the runs that
`# sim`, `# schedule`, `# select` and `# explore N` with `NAME.explore`
ask for) has its tests run natively too, by `sisu test --native` with the
mode's flags (decision 460): `--native` and `--native-from-sir` build them
as they are, from stage 1's `sir --tests` (whatever the mode, a test's
program is built from SIR), `--native-release` with `--release`
(a test checks every contract), `--native-cranelift` with `--backend=cranelift`,
`--native-from-lir` through the Rust lowering and Cranelift (`--lower=rust
--backend=cranelift`), and `--interpreted-from-sir` in the interpreter;
each report is compared with the expectation, held to the status it
gives, and each failure replayed natively from its token. PCT, the bounded
exhaustive search, the `sim` profile's faults, `sim.choose`, `sim.always`
and `expect` run natively as in the interpreter (decisions 500 to 503), so
every such run of a case is one a native test makes, a property's
budget of statements counted as yield points (decision 540). A case that
has only such runs is built for nothing else. A case whose program its errors
stop (`run/stopped_by_errors`) passes when the build, or `sisu sir`,
prints its `.err` and exits with its `.status`, as `sisu run --native`
does (decision 463).
`cargo test -p sisu-driver --test native` runs `native.list` all six ways,
and CI runs them on every push, each through the front end the executable
uses (stage 1 by default, so a case built from source or from `sisu sir`
is built from stage 1's SIR). `--native-survey
LIST` builds and runs every runnable case the list leaves out (one with
an `.out`, `.err` or `.status`, or a run of `sisu test` it pins) and
prints `pass` or the first thing that stopped it, which is how the list
grows:

```text
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native conformance/native.list
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native-from-sir conformance/native.list
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native-release conformance/native.list
cargo run -p sisu-conformance -- --sisu target/debug/sisu --interpreted-from-sir conformance/native.list
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native-from-lir conformance/native.list
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native-cranelift conformance/native.list
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native-survey conformance/native.list
```

(from `compiler/`). The summary line counts the cases and their lines,
and the time spent building and running them.

## Groups

- `examples/` holds copies of `proposal/examples/hello.sisu` and
  `wordcount.sisu`, kept identical to the originals by a test, and of
  `traits.sisu`, `errors.sisu`, `capabilities.sisu` and
  `concurrency.sisu` with what they leave undeclared and a `main` (or a
  test) appended, kept starting with the originals.
- `syntax/` is lexing and parsing, with the error-tolerance cases: one
  mistake gives one diagnostic, and the rest of the file is still parsed;
  and how `sisu fmt` writes options (`option_nested` and `option_spaced`
  spell them out, `option_sugar` as it writes them: decision 267).
- `names/` is name resolution; `types/` is the checker, including cases
  where broken code still gets a typed tree and only the first error in a
  chain is reported.
- `run/` is the semantics: arithmetic and its traps, the sized integers
  from `u8` to `u128` (`sized_integers`, `wide_overflow`), `f32` and its
  one rounding per operation (`f32`; `types/f32` is what it rejects,
  `sir/f32` its SIR), strings and
  their bytes (`text_bytes`: the text methods written in Sisu over the
  bytes; `char_boundary`), formatting (`format_specs`: the spec
  mini-language, written in Sisu; `debug_forms`: `Debug` and `Display` of
  every kind of value, natively by the runtime in Sisu; `shared_display`: a
  `Shared` shown as its value; `shared_equality`: two `Shared` equal by
  their values, and a `Shared` a table's key by its value), the moves of
  lists (`list_moves`), copies of counted handles put in a `Box` or a
  `dyn` (`counted_copies`),
  collections, `match` (`or_bindings`: alternatives of `|` that bind
  names), loops, closures, errors and traces, options (`let_else_copy`),
  `Drop` order on normal and unwinding paths (`drop_panic_fields`: a
  `Drop` that panics; `drop_chain`: a long chain dropped at constant
  depth; `drop_moved_parts`: what is left of a value once a part moves
  out, error values a pattern takes fields out of, and the operands a `?`
  or a panic abandons; `drop_parts`: tuples within tuples, last to first,
  a panic among their parts, and values whose drop only frees memory,
  natively by their parts), panics and their exit statuses, a recursion that
  overflows the stack (`stack_overflow`: a panic naming the function, its
  drops and `defer`s run, the trace's ends), constants read where they are
  used, natively too, with the schedule and the overflow a call of them
  gives (`constant_reads`), what a call leaves of an
  iterator given to an `Iterable[T]` parameter (`iterable_rest`: dropped
  when it returns and when it unwinds), an iterator written in Sisu given
  to one (`iterable_iterator`: lazy, through a generator), the checks a
  release build leaves out because its facts prove them and the one it
  keeps (`proven_checks`: indices below a length nothing changes,
  additions that cannot overflow, a postcondition its preconditions
  decide, and a list that shrinks in its loop), capabilities, and tests.
- `traits/` is traits and impls (18, Phase 4): provided methods,
  associated types, supertraits, coherence and the orphan rule, sealed
  traits, method resolution and ambiguity, `dyn Trait` and object safety,
  operator traits, `FromLiteral`, `in` through `Contains` (`contains`),
  derives and delegation, and their diagnostics.
- `generics/` is user generics: functions and types with bounds checked
  at the definition, defaults, type arguments at run time, views through
  generic parameters, `Drop` for a generic type, functions as values
  (`function_values`: generic ones and ones that borrow a parameter;
  `function_value_errors`), and `const` parameters
  with `Array[T, N]` (`const_generic`, `const_generic_errors`) and
  constant arguments written as expressions, `Array[T, W * H]`
  (`const_expressions`, `const_expression_errors`).
- `comptime/` is compile-time evaluation: constants computed by the
  compiler (`baked` pins the SIR built from literals), `comptime assert`,
  `comptime def` (`comptime_defs`: ones that reflect, take a type, `T:
  type`, or belong to a trait), constants and defaults that reflect
  (`reflecting_values`), budgets, the evaluation errors, and derives
  written as compile-time Sisu (`derive_default` and
  `derive_default_errors`: the prelude's `Default`), with
  `proposal_derive`, the user-defined derive of
  `proposal/examples/comptime.sisu`, and `proposal_arrays`, its CRC table
  and ring buffer, each kept identical by a test; `generic_assert` is a
  `comptime assert` over a `const` parameter, checked at the call.
- `reflect/` is reflection in generic functions, each made for the types
  it is called with: the `reflect` queries, attributes as data,
  `implements`, `reflect.construct` and `default_of`, promotion of generic
  callers, reflection in methods (`methods`), in a trait's own methods and
  the methods of trait impls, generic ones too (`trait_methods`), in
  tests, parametrised ones too (`tests`, with its `.test`; a test that
  reflects is still a test to the checker, `test_instances`), in closures
  (`closures`), over methods (`methods_of`), the places it cannot unfold
  (`methods_rejected`), `comptime panic` and the bound on instances,
  `embed`, `target`, `std.x.cli`'s parser and `json.encode` written over
  reflection in the standard library, and `proposal_comptime`, the
  sections of `proposal/examples/comptime.sisu` that stage 0 runs, each
  kept identical by a test.
- `contracts/` is 07: `requires`, `ensures` with `result` and `old`,
  invariants, refinement types and their chains, the check levels, the
  failure messages with every part's value, contracts as `sisu doc`
  shows them (`.doc`), contracts as properties, and `proposal_contracts`,
  which starts with `proposal/examples/contracts.sisu`.
- `doc/` is `sisu doc` (10; decision 258), each case's pages pinned by
  its `.doc`: `items`, a single file with every kind of item (contracts,
  an inferred error set, a typed hole, capability parameters, a type's
  impls and methods, an impl of a type it does not declare), `package`, a
  package whose pages show only what another package may name, with
  modules under modules, re-exports and a dependency it does not
  document, and `std_pages`, the pages of four modules of the standard
  library (`# doc --std ...`).
- `replay/` is the capability log and replay tokens: programs reading
  standard input, files and the network, recorded and replayed, and
  failing tests run again from their tokens.
- `sim/` is the `sim` profile and the schedulers (`# sim`, `# schedule`):
  a lost update found by the bounded exhaustive search, a depth-2 bug
  found by PCT, crashes of a crash-consistent file system, nodes and
  partitions, `choose` and `always`, faults from declared error sets, and
  HTTP over simulated nodes with faults.
- `testing/` is `sisu test`'s surface (17): parametrised tests, suites
  and tags (`# select`), snapshots, transcripts, `@refines`, fakes of the
  program's traits, tests in parallel with their output in declaration
  order, holes that block a test (`hole_in_task`, whose run exits with 3),
  the output of the schedule that failed under `--explore`
  (`explore_output`), programs whose errors stop `sisu test` before any
  test runs (`refused_program`, `refused_comptime`), a package that
  declares its unsafe code, whose lockfile keeps the level
  (`unsafe_level`), and `proposal_testing`, which starts with
  `proposal/examples/testing.sisu`.
- `sir/` pins the serialised SIR of small programs.
- `live/` is 10's tools that run programs: hot replacement under `sisu
  run --watch` (`.watch`: a swap of changed bodies in epochs, a rejected
  save, restarts for a changed signature and a changed layout, tasks
  adopting epochs at their boundaries), a REPL session with `:back` and
  `:why` (`.repl`), and a worksheet (`.ws`).
- `packages/` is 09, mostly package cases: modules from the file tree
  (a file and a directory of the same name are one module), `import` and
  `from ... import`, `pub`, `pub(package)` and re-exports, manifests and
  their errors, path dependencies with the lockfile, `[trust]`, the orphan
  rule across packages, dependency cycles, a workspace with one lockfile
  (and one whose member has no manifest), `sisu build`, a failing test
  replayed from its token in a package, scripts (`#!/usr/bin/env sisu`,
  top-level statements, inline metadata), and a package run under
  `--watch` and loaded into `sisu repl`.
- `std/` is the standard library as modules (std.collections, std.num,
  std.time and the rest, imported by their paths), its doc examples run
  as tests by `sisu test --std`, and the functions of the prelude that
  must respect a program's own `Ord`; a case for each module of 11 that
  is written in Sisu over the prelude (decision 252): `num_exact`
  (`BigInt`, `Decimal`; `decimal_literals`, a float literal's places kept
  natively as in the interpreter), `par`, `io`, `fs`, `rand`, `encoding`, `hash`
  (each digest against its reference), `sort`, `search`, `process_env`,
  `json_decode`, `regex` (and its rejected patterns) and `text_math`
  (`Builder`, `std.math`, `SortedSet`); a `DateTime`'s parts
  (`time_parts`), `Instant` and time zones (`time_zones`), the `pure`
  consumers of `Iter[T]` (`pure_iterators`), std.log's levels and fields
  (`log_levels`), and the bit methods of the sized integers and `int`
  (`bit_methods`).
- `concurrency/` is tasks (18, Phase 6), most cases under several
  schedules: joins and their error sets, failures and the scope's result,
  panics in tasks, a task's stack overflow (`stack_overflow`),
  cancellation at the blocking points, shields and deadlines, channels
  (`unbounded_channel`: one whose `send` never waits),
  `select`, races, mutexes across tasks with guards dropped by unwinding,
  the lowest-index rule of `pure` scopes, deadlock reports, generators
  (`generator_close`: one dropped before its end runs its drops and
  `defer`s; `generator_close_panic`: a panic there aborts;
  `generator_driven`: one inside another iterator or given to an
  intrinsic runs to its `yield` there; `generator_driven_wait`: a wait
  there aborts), the sync types beside `Mutex` (`sync_types`: `Atomic`,
  `Once`, `Condvar` and `RwLock`; `rwlock_writers`: a waiting writer
  goes before new readers; `sync_errors`), `log`, and
  the diagnostics of `Send`, `Sync`, what a task may borrow and what a
  channel may carry (`channel_owner_errors`: neither a view nor
  must-consume).
- `capabilities/` is std.cap (06): attenuations, lent capabilities and
  their diagnostics, caretakers, interceptors (of the standard
  capabilities and, `wrap_trait`, of a program's own trait), the fakes
  (`fake_rejected`: what `cap.Fake` cannot fake), the test clock,
  the fresh fakes of each test and the real `System` a test may declare
  (and the signatures it may not), the network over `Net.fake`, the
  host's objects, `DateTime`, objects dropped with the last copies of
  their handles (`object_drops`) and with the last copies of the function
  values whose `own` closures hold them (`closure_drops`), and what the
  capability rules reject.
- `http/` is std.x.http over the in-process network, under several
  schedules (a test of the driver runs it under a thousand), with the
  network's error a refusal gives (`Network(cause)`), and a `FakeNet`
  that scripts any response and a slow one (`scripted`).
- `ownership/` is the ownership checker (18, Phase 3). Accepted programs
  run (and so run again under the monitor): moves and partial moves, drop
  placement on every path (`drop_torture` is the drop-order torture
  program), the parameter conventions, non-lexical and two-phase borrows,
  returned borrows and `from`, views, closures, the three iteration modes,
  `let` with a pattern and `else` (`let_else_tuple`), a reference passed
  on from a parameter that only reads (`shared_param_passed_on`),
  `@must_consume`, the library types (`Box`, `Shared` and `Weak`,
  `Cell`, a `Mutex` with D-077's poisoning, `Arena` with generational
  keys), `own` closures, borrows of a capability's object through its
  handle (`handle_borrows`), and `proposal_core`, `proposal_shared` and
  `proposal_graph`, the sections of `proposal/examples/ownership.sisu`,
  kept identical to the original by tests. Rejected programs pin their diagnostics by code
  and span, and most also pin their counterexample run (`.defer`): the
  monitor's concrete conflict, or "no conflict" with the account of why
  the checker still rejects them (`candidate_false_positive` and
  `untested_path` show the two other outcomes). `explain_borrow` pins
  `sisu explain-borrow`. `rustc/` is a translated slice of rustc's
  borrow-checker UI tests (18, Phase 3): a dozen of their shapes rewritten
  in Sisu by hand (a move while borrowed, two closures that change one
  variable, a closure that escapes with a borrow, a borrow of a local
  returned, a use before assignment, a use after a move and a new value,
  a list changed in the loop over it, moves out of a list's element, a
  field assigned while borrowed with `mut`, a `mut self` method called
  while borrowed, a value read whole while a field is borrowed with
  `mut`), each pinning its code and its counterexample run (`.defer`), and
  the two-phase borrow they accept (`two_phase_allowed`), which runs.
- `unsafe/` is unsafe code (22; stage 1's RULES.md, R2 to R9) as the
  interpreter's checked memory model runs it (D-085, decision 301) and
  native code does (decision 302): a vector on memory of `Global`, grown
  by copying its slots, its values moved out, referred to and dropped in
  place, and a bump `Region` (`vector`); pointers to the program's places
  and into a C struct (`pointers`); plain data, its bit casts, bytes and
  views, and `transmute` (`plain_casts`); foreign calls the interpreter
  emulates, `strlen`, `memcpy`, `malloc`, `free`, `getenv` and `write`
  (`foreign`); and a refusal of the model for each obligation it checks,
  each a panic naming it (`out_of_bounds`, `released`, `uninitialised`,
  `misaligned`, `released_twice`, `release_layout`, `release_other`,
  `shared_write`, `invalid_bits`, `overwrite`, `leaked`, `gone`,
  `no_memory`, `mistyped`, `bit_cast_sizes`, `read_plain_short`,
  `foreign_unknown`, and of C's memory `malloc_bounds`, `free_twice` and
  `unterminated`), the two that only the monitor sees with their
  `.monitor` (`stale_pointer`, a write through a pointer whose borrow an
  assignment ended, and `reference_behind`, a reference used after the
  value behind it was dropped through the pointer). The behaviour cases
  are in `native.list`, and seventeen of the refusals, marked `checked`,
  whose panics a debug build's checks of `Global`'s blocks, of the
  places pointers were made to and of the memory of the C functions the
  interpreter emulates give (decisions 543, 544, 560 and 561);
  `reference_behind` and `foreign_unknown` are the interpreter's alone (a
  reference read after a drop, and the C functions it emulates).
  `../held/unsafe/` holds the cases that wait for two changes of stage 1
  (port-debt.md, "Shortcuts in the interpreter"): a stack on std.mem's
  `RawBuf` and the layouts `size_of` and `align_of` give, with
  their expectations; the runner checks them with `cargo run -p
  sisu-conformance -- --sisu target/debug/sisu conformance/held`, and
  they move here when stage 1 has changed.
- `wrong/` is the wrong-program corpus (18, Phases 2 and 3): ninety-three
  short programs a newcomer might write, each with one intended mistake
  that its first comment names (a misspelling, a missing `mut`, a use
  after a move, a view kept in a plain struct, ...). Sixty-two of them
  are the wrong-ownership corpus of Phase 3, a mistake against the
  ownership model each: moves, partial moves and moves in loops, borrows
  that conflict or outlive what they borrow, views in owners, `from`
  clauses, must-consume values, closures, tasks and what they may share,
  and `mut` at calls. Each `.diag` pins the one error by identifier and
  span; a test of the driver (`tests/wrong.rs`) holds every case to one
  error that names its fix, a fix with edits or a note, applies each fix
  with edits to see that the program then checks, and counts the
  ownership corpus, which must hold sixty.
