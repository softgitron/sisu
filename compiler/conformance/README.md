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
```

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
| `NAME.doc` | standard output of `sisu check --explain NAME.sisu`: the signatures with their contracts, as `sisu doc` renders them |
| `NAME.tokens` | `sisu tokens NAME.sisu` |
| `NAME.tree` | `sisu parse --tree NAME.sisu` |
| `NAME.defer` | `sisu run --defer-borrow NAME.sisu ARGS...`: standard output, then `--- stderr`, standard error, and `--- status N` |
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
--format=ids`, `NAME.test`, `NAME.out`, `NAME.err`, `NAME.status`,
`NAME.args`, `NAME.stdin`, `NAME.unformatted`, `NAME.watch` for `sisu run
--watch`, `NAME.repl` for `sisu repl` with the package loaded), and these:

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
  "the checker checks itself").
- For a case that runs, `sisu run --record LOG` prints and exits as the
  plain run did, and `sisu replay LOG` reproduces that run from the
  capability log alone (N-04).
- Every failing test in `.test`, `.sim`, `.schedule` and `.select` output
  prints a replay token; `sisu test --replay TOKEN NAME.sisu` must fail
  that test again with the same report.
- For a case with `.test`, `sisu test --jobs 1` (the tests on one
  thread) prints what the default, parallel run printed.
- For a case with `.watch`, `sisu run --watch --record LOG` prints what
  the watched run printed, and `sisu replay LOG` reproduces it, its
  `edit` records standing in for the saves.
- Every file that parses keeps its tree when `sisu fmt` rewrites it:
  `sisu parse --shape` prints the same before and after (the tree's nodes
  and significant tokens, up to what the formatter changes on purpose:
  trailing commas, `Option[T]` written `T?`, `Result[T, E]` written `T !
  E`, unused imports, and the order of imports), and formatting it a
  second time changes nothing. Unless
  the case has a `NAME.unformatted`, `sisu fmt --check` must also accept
  it as it is: the case is canonical text.

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
starts a comment. With `--native LIST` the runner builds each case with
`sisu build --native --output EXE NAME.sisu` (a package case without the
file) in the case's sandbox, runs the executable as it runs `sisu run`
(with the case's `.args`, `.stdin` and `.cwd`, an empty environment and
the time limit), and compares:

- standard output with `NAME.out`;
- the exit status with `NAME.status` (absent means `0`);
- standard error with `NAME.err` (absent means empty), except that when
  the build printed warnings, which a `sisu run` prints before the
  program's own standard error, the program's must be the end of
  `NAME.err`.

The expectations are the interpreter's: the native mode reads no file of
its own and does not bless. With `--native-from-sir LIST` the runner
builds each case from serialised SIR instead, `sisu build --native
--from-sir NAME.sir`, with the case's own `NAME.sir` where it has one and
what `sisu sir` prints for it otherwise (whose warnings stand for the
build's), and compares the same way: the path a self-hosted front end's
SIR takes to native code. `cargo test -p sisu-driver --test native` runs
the list both ways, and CI runs both on every push. `--native-survey
LIST` builds and runs every runnable case the list leaves out (one with
an `.out`, `.err` or `.status`) and prints `pass` or the first thing that
stopped it, which is how the list grows:

```text
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native conformance/native.list
cargo run -p sisu-conformance -- --sisu target/debug/sisu --native-from-sir conformance/native.list
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
  mistake gives one diagnostic, and the rest of the file is still parsed.
- `names/` is name resolution; `types/` is the checker, including cases
  where broken code still gets a typed tree and only the first error in a
  chain is reported.
- `run/` is the semantics: arithmetic and its traps, the sized integers
  from `u8` to `u128` (`sized_integers`, `wide_overflow`), `f32` and its
  one rounding per operation (`f32`; `types/f32` is what it rejects,
  `sir/f32` its SIR), strings and
  their bytes (`text_bytes`: the text methods written in Sisu over the
  bytes; `char_boundary`), formatting (`format_specs`: the spec
  mini-language, written in Sisu),
  collections, `match` (`or_bindings`: alternatives of `|` that bind
  names), loops, closures, errors and traces, options (`let_else_copy`),
  `Drop` order on normal and unwinding paths (`drop_panic_fields`: a
  `Drop` that panics; `drop_chain`: a long chain dropped at constant
  depth), panics and their exit statuses, capabilities, and tests.
- `traits/` is traits and impls (18, Phase 4): provided methods,
  associated types, supertraits, coherence and the orphan rule, sealed
  traits, method resolution and ambiguity, `dyn Trait` and object safety,
  operator traits, `FromLiteral`, derives and delegation, and their
  diagnostics.
- `generics/` is user generics: functions and types with bounds checked
  at the definition, defaults, type arguments at run time, views through
  generic parameters, `Drop` for a generic type, and `const` parameters
  with `Array[T, N]` (`const_generic`, `const_generic_errors`) and
  constant arguments written as expressions, `Array[T, W * H]`
  (`const_expressions`, `const_expression_errors`).
- `comptime/` is compile-time evaluation: constants computed by the
  compiler (`baked` pins the SIR built from literals), `comptime assert`,
  `comptime def`, budgets, the evaluation errors, and derives written as
  compile-time Sisu, with `proposal_derive`, the user-defined derive of
  `proposal/examples/comptime.sisu`, and `proposal_arrays`, its CRC table
  and ring buffer, each kept identical by a test; `generic_assert` is a
  `comptime assert` over a `const` parameter, checked at the call.
- `reflect/` is reflection in generic functions, each made for the types
  it is called with: the `reflect` queries, attributes as data,
  `implements`, `reflect.construct` and `default_of`, promotion of generic
  callers, `comptime panic` and the bound on instances, `embed`,
  `target`, `std.x.cli`'s parser and `json.encode` written over
  reflection in the standard library, and
  `proposal_comptime`, the sections of `proposal/examples/comptime.sisu`
  that stage 0 runs, each kept identical by a test.
- `contracts/` is 07: `requires`, `ensures` with `result` and `old`,
  invariants, refinement types and their chains, the check levels, the
  failure messages with every part's value, `sisu check --explain`
  (`.doc`), contracts as properties, and `proposal_contracts`, which
  starts with `proposal/examples/contracts.sisu`.
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
  order, and `proposal_testing`, which starts with
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
  must respect a program's own `Ord`.
- `concurrency/` is tasks (18, Phase 6), most cases under several
  schedules: joins and their error sets, failures and the scope's result,
  panics in tasks, cancellation at the blocking points, shields and
  deadlines, channels, `select`, races, mutexes across tasks with guards
  dropped by unwinding, the lowest-index rule of `pure` scopes, deadlock
  reports, generators, `log`, and the diagnostics of `Send`, `Sync` and
  what a task may borrow.
- `capabilities/` is std.cap (06): attenuations, lent capabilities and
  their diagnostics, caretakers, interceptors (of the standard
  capabilities and, `wrap_trait`, of a program's own trait), the fakes
  (`fake_rejected`: what `cap.Fake` cannot fake), the test clock,
  the fresh fakes of each test and the real `System` a test may declare
  (and the signatures it may not), the network over `Net.fake`, the
  host's objects, `DateTime`, and what the capability rules reject.
- `http/` is std.x.http over the in-process network, under several
  schedules (a test of the driver runs it under a thousand).
- `ownership/` is the ownership checker (18, Phase 3). Accepted programs
  run (and so run again under the monitor): moves and partial moves, drop
  placement on every path (`drop_torture` is the drop-order torture
  program), the parameter conventions, non-lexical and two-phase borrows,
  returned borrows and `from`, views, closures, the three iteration modes,
  `let` with a pattern and `else` (`let_else_tuple`), a reference passed
  on from a parameter that only reads (`shared_param_passed_on`),
  `@must_consume`, the library types (`Box`, `Shared` and `Weak`,
  `Cell`, a `Mutex` with D-077's poisoning, `Arena` with generational
  keys), `own` closures, and `proposal_core`, `proposal_shared` and
  `proposal_graph`, the sections of `proposal/examples/ownership.sisu`,
  kept identical to the original by tests. Rejected programs pin their diagnostics by code
  and span, and most also pin their counterexample run (`.defer`): the
  monitor's concrete conflict, or "no conflict" with the account of why
  the checker still rejects them (`candidate_false_positive` and
  `untested_path` show the two other outcomes). `explain_borrow` pins
  `sisu explain-borrow`.
- `wrong/` is the wrong-program corpus (18, Phase 2): about forty short
  programs a newcomer might write, each with one intended mistake that its
  first comment names (a misspelling, a missing `mut`, a use after a move,
  a view kept in a plain struct, ...). Each `.diag` pins the one error by
  identifier and span with its fix; a test of the driver
  (`tests/wrong.rs`) holds every case to one error that names a fix, a
  fix with edits or a note, and applies each fix with edits to see that
  the program then checks.
