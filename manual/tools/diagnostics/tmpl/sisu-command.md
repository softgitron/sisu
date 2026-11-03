# The `sisu` command

`sisu` is the one program that runs, checks, builds, tests, documents
and formats Sisu code (D-029). This page describes each of its commands
and options: what it takes, what it does, what it prints, and the exit
status it ends with. The commands that the proposal designs and the
compiler does not have yet come last, each with its status.

**See also:** [chapter 1](../tutorial/01-hello.md) of the tutorial
(running, checking and formatting a file, and the parts of a
diagnostic) and [chapter 9](../tutorial/09-word-counter.md) (a
package); [R17, Diagnostics](diagnostics.md), for every code a command
prints; the proposal's [10, Tooling](../../proposal/10-tooling.md);
the compiler's [architecture notes](../../compiler/docs/architecture.md),
"The query layer and the driver".

## How to read this page

- A command line is written as `sisu run [OPTIONS] [FILE] [ARGS...]`.
  Square brackets mark what may be left out, `...` what may repeat, and
  capitals what you replace.
- Every example ran when the manual was last checked. A session shows
  what a terminal shows: each line that starts with `$ sisu` is a
  command, and the lines after it are its standard output and standard
  error together.
- The page describes the `sisu` of this repository, stage 0 of the
  compiler. Its toolchain calls itself 0.0.1 in the replay tokens and
  capability logs it writes; it has no command that prints its version.
- Each section ends with a status. **In stage 0** means that the
  compiler does what the section says, as its examples show.
  **Designed** means that the proposal decides it and no compiler has it
  yet. **Leaning** means that its decision leans and may still change,
  and names the decision. **Prototype** means a prototype of the
  experiment (Q-49), which may change or go. A paragraph that starts
  **Stage 0 differs** says where the compiler does something other than
  what the proposal says.

## The command line

### Commands, options and arguments

- `sisu COMMAND [OPTIONS] [FILE] [ARGS...]`

The parts of a command line follow these rules:

- The first word is the command.
- Options come after the command and before the file. An option that
  takes a value is written `--option VALUE` or `--option=VALUE`.
- `ARGS` are the program's own arguments, for `sisu run`; `--` ends the
  options when there is no file, as in `sisu run -- ARGS`.
- An option that the command does not take is D0009, a command that
  does not exist is D0003, and an option given a word where it needs a
  number is D0023. Each is a usage error, with exit status 2.
- `sisu help` (also `sisu --help` and `sisu -h`) prints the list of
  commands to standard output, with status 0. `sisu` alone prints the
  same list to standard error, with status 2.

@@output usage.console@@

The list that `sisu help` prints is the compiler's own summary of this
page:

<details>
<summary>What <code>sisu help</code> prints</summary>

@@output help.console@@

</details>

**Stage 0 differs.** `sisu explain` and `sisu new` without their
argument say that they need "a file" (D0004), where they need a code
and a name.

**Status.** In stage 0.

### What a command works on

A command works on one of these:

- **A file.** `sisu run hello.sisu` checks and runs the program in
  `hello.sisu`. A single file is a program with no modules of its own
  besides the standard library's.
- **A package.** Without a file, a command works on the package of the
  current directory, the nearest directory at or above it that holds a
  `sisu.toml`. The package's module tree is its `src/` directory, and
  its root module is `src/main.sisu` for an application or
  `src/lib.sisu` for a library ([chapter 9](../tutorial/09-word-counter.md)).
  Where there is no `sisu.toml`, the command stops with D0064 and status
  2.
- **A workspace.** In a workspace, `sisu check`, `build` and `test` work
  on every member in turn and end with the worst status of the members;
  `sisu run`, `sir` and `bench` work on the one member that is an
  application, and stop with D0067 when there is more than one.

Every command that checks a package also writes its lockfile,
`sisu.lock`, when the manifests imply another one; `--locked` refuses
to, as [Package options](#package-options) says.

This package, `till`, adds up the prices given to it:

@@file till.pkg/sisu.toml@@

@@file till.pkg/src/main.sisu@@

In the package's directory, each command works on the whole package:

@@output till.console@@

**Status.** In stage 0.

### Running a file as a script

- `sisu FILE.sisu [ARGS...]`

A command line whose first word is a file ending in `.sisu` runs that
file, as `sisu run FILE.sisu ARGS` does. A file that starts with the
line `#!/usr/bin/env sisu` is a script: its top-level statements are
its `main`, run in order, with `sys`, the program's `System`, in scope,
and the operating system runs it as `./FILE.sisu` once it is
executable. A script may name its dependencies in a comment block
between `# /// script` and `# ///` ([10](../../proposal/10-tooling.md),
"REPL and scripts"; D-052).

@@file script.sisu@@

@@output script.console@@

**Status.** In stage 0.

### Exit statuses

Every command ends with one of these statuses:

| Status | Means |
|---|---|
| 0 | The command did what it was asked: the program ran and ended normally, every test passed, the file is formatted. |
| 1 | The program has errors (and did not run), a test failed, `main` returned an error, a replay diverged, or a file could not be read or written. |
| 2 | A usage error: an unknown command or option, a missing argument, a value of the wrong form, or no package where one is needed. |
| 70 | An internal compiler error (D0012, D0076, D0118): a bug in `sisu`, never in your program. |
| 101 | The program panicked. |
| 134 | The program aborted: a panic while unwinding from a panic, or the ownership monitor stopping a run at a conflict. |

A program that panics prints the panic to standard error and ends
with status 101. This one prints its first line, then the panic:

@@file panics.sisu@@

@@output panics.err@@

An error that `main` returns is printed to standard error, with the
place that passed it on, and the status is 1:

@@file fails.sisu@@

@@output fails.err@@

The error is printed in its `Debug` form, `Missing(key="port")`.

**Status.** In stage 0.

### Choosing the front end

- `sisu COMMAND --front-end=stage1|rust ...`
- `SISU_FRONT_END=stage1|rust sisu COMMAND ...`

The front end reads, checks and lowers a program; the rest of `sisu`
runs, tests and compiles what it produces. Stage 0 has two: stage 1,
written in Sisu and compiled natively, which is the default for
`check`, `sir`, `build`, `run`, `fmt`, `doc`, `tokens` and `parse`, and
the Rust front end, the oracle that stage 1 is checked against.
`--front-end=rust` among a command's options, or `SISU_FRONT_END=rust`
in the environment, chooses the Rust one. The other commands, and
`run --watch`, `--record` and `--defer-borrow`, `check --diary` and
`parse --shape`, use the Rust front end whichever you choose.

@@output front_end.console@@

A value other than `stage1` or `rust` is D0115. When stage 1 is missing
(D0116), fails to build (D0117) or stops as a compiler may not (D0118),
the message names `--front-end=rust` as the way on.

**Status.** In stage 0, for the compiler's own development (its
decision 268 in the [architecture
notes](../../compiler/docs/architecture.md)); the proposal has one
front end.

## Running programs

### sisu run

- `sisu run [OPTIONS] [FILE] [ARGS...]`
- `sisu run [OPTIONS] -- [ARGS...]`

`sisu run` checks the program, lowers it, runs the ownership checker
and runs it in the interpreter, with `ARGS` as its arguments
(`sys.env.args()`). Warnings are printed and the program runs. With an
error, `sisu run` prints the diagnostics and D0006 and does not run the
program; with ownership errors only, it prints D0021, which names
`--defer-borrow`. The exit status is the program's: 0 when `main`
returns, 1 when `main` returns an error, 101 for a panic, 134 for an
abort; 1 when the program did not run.

@@file hello.sisu@@

@@output hello.out@@

Without a file, `sisu run` runs the package's application;
for a library it stops with D0069.

**Status.** In stage 0.

### Watching for saves: --watch

`sisu run --watch [FILE] [ARGS...]` runs the program once and, while it
runs, checks every save of its files. A save that checks publishes all
the bodies that changed as one epoch; a task adopts the newest epoch
when it starts and at `live.boundary()`, which `Ticker.wait` calls, so
one loop iteration never mixes an old body with a new one. A save that
does not check publishes nothing, and a changed signature or layout
restarts the program. Each swap is reported on standard error.

This program saves its own next version, `next.sisu`, after its first
frame, as an editor would:

@@file watch.sisu@@

`sisu run --watch watch.sisu` prints this to standard output, and to
standard error, after the first frame, a line that names the epoch and
the functions it replaced:

@@output watch.watch@@

`--watch` runs one program once, so it does not combine with
`--explore`, `--defer-borrow`, `--sir`, `--from-sir` or `--scheduler
exhaustive` (D0078).

**Status.** In stage 0. Hot replacement is decided (D-053); its epochs
lean (D-068).

### Checking ownership while running: --monitor

`sisu run --monitor` runs the program under the dynamic ownership
monitor, which checks every move, borrow and drop as it happens. A
program that the ownership checker accepted prints the same with and
without it; a difference is a bug in the checker, which the
conformance suite looks for.

@@output monitor.console@@

**Status.** In stage 0, a tool of the experiment (18, Phase 3).

### Running a rejected program: --defer-borrow

`sisu run --defer-borrow` runs a program that the ownership checker
rejected, under the monitor, to find out whether the rejection is a
real conflict. The run is the counterexample: it stops at the first
conflict with the monitor's account and the SIR of the function where
it happened, and ends with status 134; or it ends without one, with
the account of why the checker still rejects the program.

@@file moved.sisu@@

<details>
<summary>What <code>sisu run</code> and <code>sisu run --defer-borrow</code> print</summary>

@@output moved.console@@

</details>

`sisu test --defer-borrow` runs a rejected program's tests in the same
way.

**Status.** In stage 0, a tool of the experiment (18, Phase 3).

### What a run checks: --contracts

`--contracts=all|preconditions|off` sets the contracts a run checks
([R11](../README.md#the-reference), when it is written; [07](../../proposal/07-contracts.md)):
`all`, the default, checks `requires`, `ensures` and invariants;
`preconditions` checks `requires` only; `off` checks none. Another
value is D0028. `sisu test` always checks every contract, so `sisu test
--contracts` is D0029.

@@file contracts.sisu@@

@@output contracts.console@@

With every contract checked, the bug in `rounded_up` is caught at its
first call. Checking only preconditions lets the wrong result 5 through
and stops the second call at its precondition; checking nothing lets the
second call divide by zero.

`sisu sir` and `sisu build --native` take `--contracts` too, and a
package's `[profile.release] contracts` sets it for a release build.

**Status.** In stage 0.

### Recording a run: --record and --redact

`sisu run --record LOG` runs the program and writes its capability log
to `LOG`: the toolchain, the program's source, the scheduler's seed,
standard input, and every result that a capability returned, in order.
`sisu replay LOG` runs the program again with the log standing in for
the world, so the replay reads no file and no clock.
`--redact CAPS` leaves the results of the named capabilities out of the
log, for a log that would hold secrets: `fs`, `net`, `clock`, `rand`,
`env`, `proc` and `stdin`, separated by commas (D0039 for another
name). A replay of a redacted log diverges at the first call whose
result it does not hold.

@@file reader.sisu@@

With a file `poem.txt` of two lines:

@@output reader.console@@

`--record` records one run, so it does not combine with `--explore`,
`--defer-borrow`, `--sir` or `--from-sir` (D0038).

**Status.** In stage 0, a prototype of the experiment (Q-49, D-070).

### Counting what the run did: --stats

`--stats` reports the scheduler's numbers after the run, on standard
output: the tasks, the most at once, the switches between them, the
latency of scheduling, and the `Shared` cells and whether another task
reached them. `--stats stack-depth` reports the depth of the call stack
instead, and `--stats shared` each place that constructs a `Shared`.
`--stats=` with another word is D0094.

@@output stats.console@@

**Status.** In stage 0, for the experiment's measurements (18, Phase 6).

### Compiling and running natively: --native and --release

`sisu run --native` compiles the program to an executable with the
native backend, as `sisu build --native` does, and runs it.
`--release` builds with the release profile, which leaves out the
checks it can prove unnecessary; it goes with `--native` only (D0111),
since the interpreter has no profiles. A native run takes the schedule
options and `--stats` with no word after it; it does not combine with
`--sir`, `--monitor`, `--defer-borrow`, `--watch`, `--record` or
`--scheduler` (D0109).

@@output native.console@@

**Stage 0 differs.** The native backend compiles every program that the
conformance suite runs, and reports what it does not compile yet with
D0103. The proposal's debug backend is its own and its release backend
is LLVM ([10](../../proposal/10-tooling.md)); stage 0's release profile
uses the same backend with the checks it proves unnecessary left out.

**Status.** In stage 0 (stage 2 of the bootstrap,
[18](../../proposal/18-next-steps.md)).

### Running serialised SIR: --sir, --from-sir and --from-lir

`sisu run --sir FILE.sir` and `sisu run --from-sir FILE.sir [ARGS...]`
run a program from its serialised SIR, the form that `sisu sir`
prints; with `--from-sir`, what follows the file is the program's
arguments. `sisu run --native --from-sir` compiles the SIR natively
and runs it, and `--native --from-lir FILE` does the same for LIR, the
lower form that stage 2 of the bootstrap writes (D0119 without
`--native`).

@@output sir.console@@

`sisu test --from-sir` and `sisu bench --from-sir` run the tests and
benchmarks of SIR that `sisu sir --tests` wrote.

**Status.** In stage 0, for implementations of Sisu: a front end
written in Sisu hands its SIR to these.

### sisu replay

- `sisu replay [--monitor] LOG`

`sisu replay` runs a recorded program again, with the capability log
standing in for the world: each capability call returns the result the
log holds, standard input is the log's, and the scheduler takes the
log's seed. The program is the source the log holds, so the replay
needs only the log. `--monitor` replays under the ownership monitor.
The example of [Recording a run](#recording-a-run---record-and---redact)
shows a replay.

The replay stops with status 1 when the log is not one (D0036), was
made by another toolchain (D0034), or diverges from what the program
does (D0037), and reports a replay that exits with another status than
the recorded run did (D0040).

**Status.** In stage 0, a prototype of the experiment (Q-49, D-070).

### Choosing the schedule

- `sisu run|test --schedule-seed N --explore N --scheduler random|pct|exhaustive --depth N ...`

Tasks run on a cooperative scheduler that is seeded, so that a run with
the same seed takes the same schedule
([R9](../README.md#the-reference), when it is written;
[06](../../proposal/06-concurrency-and-capabilities.md)).

- `--schedule-seed N` runs with seed `N`; the default is 0.
- `--explore N` runs `N` schedules, from the first seed on, and stops at
  the first run that fails, naming its seed (D0025); when all succeed
  it says so (D0024). Under `sisu test` every test runs under each
  schedule (D0026, D0027).
- `--scheduler random` is the default, a random choice at each yield
  point. `--scheduler pct` is probabilistic concurrency testing, which
  finds bugs of a small depth with a known probability; `--depth N` is
  that depth, 3 by default (D0042 to D0045). `--scheduler exhaustive`
  searches every schedule with at most `--depth N` preemptions, 2 by
  default, up to 10,000 schedules unless `--explore N` says otherwise
  (D0046 to D0050). Another scheduler is D0041.

This program expects the first task it spawns to finish first, which
some schedules break:

@@file race.sisu@@

@@output race.console@@

Each failure names the options that run it again. `sisu run race.sisu`,
with seed 0, fails at once.

**Status.** In stage 0. PCT and the exhaustive search are tools of the
experiment ([17](../../proposal/17-testing.md); 18, Phase 6).

### Package options

`run`, `check`, `build`, `doc`, `test` and `bench` take these:

- `--locked` keeps `sisu.lock` as it is: a lockfile that the manifests
  no longer match stops the command with D0065, which says what would
  change. CI runs with it.
- `--features NAMES` turns on the package's features, separated by
  commas; `DEP/FEATURE` turns on a dependency's. A name that is not a
  key of the package's `[features]` is D0112.
- `--no-default-features` leaves out the features that the manifest's
  `default` list turns on.

`@cfg(feature="NAME")` reads the features ([R13](../README.md#the-reference), when
it is written; [09](../../proposal/09-modules-packages-and-build.md)).

**Status.** In stage 0.

## Checking and building

### sisu check

- `sisu check [--format=human|ids] [--diary LOG] [PACKAGE OPTIONS] [FILE]`

`sisu check` reports the program's diagnostics without running it, and
ends with status 1 if any is an error, 0 otherwise. A program without
diagnostics prints nothing.

- `--format=human`, the default, prints each diagnostic as chapter 1
  explains: the code and message, the place, the source line, the
  labels, and the fix.
- `--format=ids` prints one line per diagnostic, naming it by code and
  catalog identifier with its parameters, and a line per label and fix:
  the form that the conformance suite compares, which no rewording of
  a message changes.
- `--diary LOG` appends to `LOG`, a file of JSON lines, a record of the
  check, one of each diagnostic, and one of each diagnostic of the
  previous check that this one no longer has, with the edit that
  resolved it. It is the experiment's friction log (18, Phase 3); a log
  that cannot be written is D0013.

@@file broken.sisu@@

@@output check.console@@

**Stage 0 differs.** [10](../../proposal/10-tooling.md) sets a target
of under 300 ms for `sisu check` after an edit of one function in a
package of 100,000 lines. Stage 0 checks from nothing each time it is
started; the language server, `--watch` and the REPL keep what they
checked, and recheck such an edit within the target.

**Status.** In stage 0.

### sisu build

- `sisu build [PACKAGE OPTIONS] [FILE]`
- `sisu build --native [--release] [--contracts=LEVEL] [--output PATH] [FILE]`
- `sisu build --native --from-sir FILE.sir | --from-lir FILE.lir`

`sisu build` checks and lowers the program without its tests, writes
the lockfile, and says what it built (D0066 for a package, D0070 for a
file); with an error it prints the diagnostics and D0006. With
`--native` it also compiles the program to an executable: a package's
to `build/debug/NAME` beside its lockfile, or `build/release/NAME` with
`--release`; a file's beside it, named by its stem; or to `--output
PATH`. The [native example](#compiling-and-running-natively---native-and---release)
builds one.

@@output build.console@@

**Stage 0 differs.** D0006 says that "the program was not run" after a
failed build. [09](../../proposal/09-modules-packages-and-build.md)
also gives builds a content-addressed cache and `--target` for cross
compilation, which stage 0 does not have.

**Status.** In stage 0.

### sisu sir

- `sisu sir [--contracts=LEVEL] [--tests] [FILE]`

`sisu sir` prints the program's SIR, the intermediate form that the
interpreter runs and the native backend compiles, in its text form, with
every drop placed and every `comptime` value computed. It prints the SIR
of a program that the ownership checker rejects as well. `--tests`
includes the program's tests and benchmarks, and `--contracts` sets the
contracts the SIR checks. The [SIR example](#running-serialised-sir---sir---from-sir-and---from-lir)
shows the SIR of `hello.sisu`.

**Status.** In stage 0, for implementations of Sisu
([18](../../proposal/18-next-steps.md), "Bootstrapping").

## Testing

### sisu test

- `sisu test [OPTIONS] [FILE]`

`sisu test` runs the program's tests: its `test` blocks, the examples in
its doc comments, the properties its contracts imply, and the
transcripts in the `tests/` directory beside the program. Each runs under the ownership
monitor, with fresh fakes for the capabilities it names, and ends `ok`,
`FAILED`, or `blocked by` a typed hole that it reached. Tests run in
parallel and are reported in the order they are declared. Each failure
is shown again after the list, with a replay token; the last line counts
the outcomes. The status is 1 if a test failed, 0 otherwise, even when
a test is blocked.

@@file tests.sisu@@

@@output tests.console@@

- `--filter TEXT` runs the tests whose names contain `TEXT`, `--tag T`
  the tests tagged `T`, and `--skip-tag T` all but those. Each may be
  given more than once, and the last line counts the tests left out
  (D0053). A test in a suite is named `SUITE/TEST`.
- `--jobs N` runs at most `N` tests at once, 8 by default; the tests
  that take the real `System` run one at a time. The output does not
  depend on it.
- `--no-monitor` runs without the ownership monitor.
- `--no-cache` runs every test. Without it, a test that passed is not
  run again while nothing it reaches has changed: the cache is keyed on
  the code each test reaches and on the options that decide a run, and
  a cached test is reported `ok` as a run would report it. The cache is
  `FILE.test-cache` beside a file, or `build/debug/test-cache` in a
  package. A test that takes the real `System` is never cached.
- `--replay TOKEN` runs the failing test that the token names, with the
  same seeds and the same shrunk case, so that it fails in the same
  way. A token for other code is D0033, one for a test that is not
  there D0035, and one from another toolchain D0034; text that is not a
  token is D0032. A test that takes the real `System` has no token
  (D0031).
- `--update` accepts what the snapshots and transcripts now show,
  rewriting them (D0054, D0055, D0063).
- `--sim` runs the tests that take a `System` in the `sim` runtime
  profile, against a simulated one; `--faults P` sets the probability
  that a capability call fails with an error of its declared set,
  0.01 by default (D0051, and D0052 without `--sim`).
- `--std` runs the standard library's doc examples besides the
  program's tests.
- `--defer-borrow` runs the tests of a program that the ownership
  checker rejected, as [`sisu run
  --defer-borrow`](#running-a-rejected-program---defer-borrow) does.
- The schedule options and the package options are as for `sisu run`.

A snapshot test stores what it saw the first time `--update` accepts it,
in `FILE.snap` beside a file:

@@file snapshot.sisu@@

@@output snapshot.console@@

A test that takes a `System` writes real files when it runs, and
simulated ones under `--sim`, where some writes fail:

@@file sim.sisu@@

@@output sim.console@@

[R15](../README.md#the-reference), when it is written, describes the tests themselves;
[17](../../proposal/17-testing.md) is their design.

**Stage 0 differs.** [17](../../proposal/17-testing.md) has `sisu test`
write a JSON event stream, with TAP and JUnit XML as `--format` options;
stage 0 prints the human form only, and has none of `--fuzz`, `--cover`
and `--mutate` ([Commands that are designed](#commands-that-are-designed)).

**Status.** In stage 0. `--sim`, `--faults` and replay tokens are
prototypes of the experiment (Q-49, D-070).

### sisu bench

- `sisu bench [--iterations N] [PACKAGE OPTIONS] [FILE]`

`sisu bench` runs the program's `bench` blocks, each the number of
times `--iterations N` says (100 by default), and prints the time of one
iteration; the last line counts the benchmarks and the failures. A
benchmark that panics fails, with status 1.

@@file bench.sisu@@

`sisu bench bench.sisu` prints a line like this one, with a time that
differs from run to run:

<!-- unchecked: a benchmark's time differs on every run -->
```text
bench "fib of 15" ... 100 iterations, 1.1 ms each
1 benchmarks, 0 failed (stage 0 times the interpreter: compare benchmarks with each other, not with compiled code)
```

**Stage 0 differs.** Stage 0 times the interpreter, and reports a mean.
[17](../../proposal/17-testing.md) reports confidence intervals with
outliers detected, in a stable JSON form that CI can follow.

**Status.** In stage 0.

## Documentation and formatting

### sisu doc

- `sisu doc [--output DIR] [PACKAGE OPTIONS] [PATH]`
- `sisu doc --std [MODULE...] [--output DIR]`

`sisu doc` writes the documentation of a program as Markdown, a page per
module: the module's `##!` comment, a list of its items, and for each
item its signature, its contracts, its inferred error set, the
capabilities it takes, and its `##` comment with the examples in it. A
package's pages show what another package may name, its `pub` items; a
single file's show every item. Each page is printed after a line
`<!-- FILE -->` that names its file, or written into `DIR` with
`--output DIR`. A program with errors is reported as `sisu check`
reports it, and nothing is written (D0114).

`sisu doc --std` documents the standard library, or the modules named
by their import paths, such as `std.time`; a path that names no module
is D0113. The standard library's reference ([R18](#the-standard-librarys-reference))
is these pages.

The examples in doc comments are tests, which `sisu test` runs:

@@file greet.sisu@@

@@output greet.console@@

The [package example](#what-a-command-works-on) shows the page of a
package whose root module has a `##!` comment and no `pub` item.

**Stage 0 differs.** [10](../../proposal/10-tooling.md) renders each
example's expressions with their values and serves the pages with
`--open`; stage 0 writes the examples as they are.

**Status.** In stage 0.

### The standard library's reference

The reference of the standard library is not written by hand: `sisu doc
--std --output DIR` generates it from the `##` comments of
[compiler/std](../../compiler/std/), and `sisu test --std` runs its
examples. It is page R18 of [the plan](../README.md#the-reference).

**Status.** In stage 0.

### sisu fmt

- `sisu fmt [--check] [PATH...]`

`sisu fmt` rewrites each file in the canonical layout: four spaces, at
most 100 columns, trailing commas in constructs that span lines, sorted
imports with the unused ones removed, one blank line between items, and
the sugar spelled one way (`T?`, `T ! E`). It has no options of layout
(D-030). A path that is a directory formats every `.sisu` file under it
(D0077 when there is none); without a path, the package's. `--check`
changes nothing and prints, for each file that is not formatted, what
`sisu fmt` would change, with status 1.

@@file messy.sisu@@

@@output messy.console@@

A file with syntax errors is reported and left as it is (D0075). The
formatter keeps the program: a formatted file that would parse to
another program is left as it is, and is an internal error (D0076, status
70).

**Status.** In stage 0.

## Interactive tools

### sisu repl

- `sisu repl [FILE | DIR]`

`sisu repl` starts a session: each input is the next line of a script,
an expression shows its value, and a declaration stays for the inputs
after it. A file is the session's first input; a package's directory
loads the package, whose modules the session imports by their paths
(D0080). An input that does not check, panics or returns an error is
not kept (D0083). Three commands start with a colon: `:back N` forgets
the last `N` inputs, `:why NAME` shows where a value came from, down to
the capability results it was computed from, and `:quit` ends the
session (D0084 to D0091).

The session reruns the script from the top for each input, with the
capability log of the run before standing in for the world, so the
earlier inputs neither print nor touch anything again.

Given this file and these inputs on standard input:

@@file repl.sisu@@

@@file repl.stdin@@

`sisu repl repl.sisu` prints the values to standard output, after its
prompts:

@@output repl.repl@@

The input `let wrong: int = "text"` does not check, so its E0300 goes to
standard error and the session is as it was before it.

**Status.** In stage 0 ([10](../../proposal/10-tooling.md), "REPL and
scripts").

### sisu ws

- `sisu ws [--run] FILE`

`sisu ws` runs a worksheet, a script, from the top, and prints it with
the value of each top-level statement after it (`# => VALUE`) and what
the statement printed under it (`# | TEXT`). The file is not changed. A
worksheet runs against a test's fake `System`, with files in memory, no
network, the test clock and seeded random numbers; `--run` gives it the
real one.

@@file sheet.sisu@@

`sisu ws sheet.sisu` prints:

@@output sheet.ws@@

**Stage 0 differs.** A worksheet is tooling after 0.1 (Q-48), which the
language server shows as values beside the lines; stage 0 has it now,
on the command line and in the language server.

**Status.** In stage 0.

### sisu lsp

- `sisu lsp`

`sisu lsp` is the language server: it speaks the Language Server
Protocol over standard input and output, with the same queries as
`sisu check` and `sisu run`, so the three never disagree. It serves
diagnostics with their codes, hover with types, error sets,
capabilities and contracts, go-to-definition into the standard library,
document symbols, selection ranges, formatting, completion, rename and
references, the fixes of diagnostics as code actions, inlay hints for
inferred types, error sets and `mut` and `own` at call sites, the
command `sisu.explainBorrow`, and a worksheet's values inline. An editor
starts it; the guide G1, Set up your editor, will show how. It takes no
options (D0009).

**Status.** In stage 0. The proposal's always-on evaluation, structural
editing and stepper come after 0.1 ([10](../../proposal/10-tooling.md),
"Language server").

## Packages

### sisu new

- `sisu new [--lib] NAME`

`sisu new` creates a package in a new directory `NAME`: a manifest,
`sisu.toml`, and a root module, `src/main.sisu` for an application or,
with `--lib`, `src/lib.sisu` for a library with a test. A package is
named in lowercase letters, digits and `_`, starting with a letter, and
not as a module of the standard library (D0072, status 2); a directory
that exists already is D0073 (status 1).

@@output new.console@@

**Status.** In stage 0.

## Explaining and auditing

### sisu explain

- `sisu explain CODE | IDENTIFIER`

`sisu explain` prints a diagnostic's entry in the message catalog: its
code, its kind (error, warning, runtime or cli), its identifier and its
message with the parameters in braces. It takes a code or an
identifier. A code that does not exist is D0010, with status 1.

@@output explain.console@@

**Stage 0 differs.** [10](../../proposal/10-tooling.md) gives each code
a long explanation with examples; stage 0 prints the catalog's line
(Q-66).

**Status.** In stage 0.

### sisu explain-borrow

- `sisu explain-borrow FILE:LINE`

`sisu explain-borrow` lists the borrows live on a line: whether each is
shared or `mut`, what it borrows, where it starts, the names that hold
it, and where it is last used. A program with errors is reported
instead. The argument is a file and a line, separated by a colon
(D0018 otherwise).

@@file borrows.sisu@@

@@output borrows.console@@

**Status.** In stage 0 (18, Phase 3), and in the language server as
`sisu.explainBorrow`.

### sisu audit

- `sisu audit --ownership FILE...`
- `sisu audit --caps FILE...`

`sisu audit` reports, as one line of data per item, the experiment's
counts (18, Phases 3 and 6):

- `--ownership` counts, for each file, its lines and its uses of the
  ownership model's escape hatches: `Shared`, `Weak`, `Cell`, `Arena`,
  `Key`, `.copy()` and `unsafe`. A file need not check to be counted.
- `--caps` reads a program that checks and prints each function that
  does I/O, with the parameters that hold a capability and its fewest
  calls from `main`; each use of a capability's operation; each struct
  that bundles three or more capability types; and a summary.

Without one of the two, `sisu audit` is D0022.

@@file audit.sisu@@

@@output audit.console@@

**Status.** In stage 0. The proposal's `sisu audit`, which lists trusted
packages, `unsafe` blocks, the capability summary, holes and advisories,
is [designed](#sisu-audits-other-reports).

## Inspecting the front end

### sisu tokens

- `sisu tokens [--trivia] FILE`

`sisu tokens` prints the lexer's tokens, one per line: the kind, the
byte range and the text. `INDENT` and `DEDENT` are the layout's tokens.
`--trivia` includes spaces and comments.

@@output tokens.console@@

**Status.** In stage 0, for implementations of Sisu.

### sisu parse

- `sisu parse --print|--tree|--shape|--ast FILE`

`sisu parse` prints the result of the parser. `--print` reprints the
source from the syntax tree, which keeps every byte; `--tree` prints
the tree with byte ranges; `--shape` prints what formatting keeps, the
tree without ranges, trivia and what the formatter changes on purpose;
and `--ast` prints the abstract syntax tree built from it. `--shape`
exits with status 1 for a file with a syntax error.

@@output parse.console@@

**Status.** In stage 0, for implementations of Sisu and for the
conformance suite's checks of the formatter.

## Commands that are designed

The proposal designs these commands and options, and stage 0 does not
have them yet. Stage 0 answers each with D0003 or D0009, as this session
shows; when one lands, the session changes, and so does this section.

@@output designed.console@@

### sisu check --holes

`sisu check --holes` reports each typed hole `todo("...")` with its
context: the expected type and the definitions of the types it
mentions, the bindings in scope with their conventions and
capabilities, the enclosing `requires` as facts and `ensures` as
obligations, the tests that reach it with the values they saw, and the
expressions that fit it, in a human form and a JSON form. `sisu build
--release` and `sisu publish` reject holes, `sisu audit` lists them and
`sisu doc` marks the functions that contain one.

**Status.** Designed (D-061; [10](../../proposal/10-tooling.md),
"AI-assisted development"). Stage 0 reports each hole as warning W0001,
and `sisu test` reports a test that reaches one as blocked.

### sisu lint

`sisu lint` reports lints beyond the compiler's warnings, each with a
fix.

**Status.** Designed ([10](../../proposal/10-tooling.md)).

### sisu add and sisu remove

`sisu add PACKAGE` and `sisu remove PACKAGE` edit the manifest and the
lockfile. Before it touches the lockfile, `sisu add` prints the
dependency's authority delta: the capabilities that become reachable,
new `unsafe` and `extern`, changes to `[trust]`, new dependencies and new
panic sites. It applies a cooldown before resolving a new version and
refuses new packages in agent runs.

**Status.** Designed ([09](../../proposal/09-modules-packages-and-build.md),
"Admission"); the admission rules lean (D-066). Stage 0 has path
dependencies only.

### sisu update

`sisu update` resolves newer versions within the ranges the manifest
allows, and prints the authority delta as `sisu add` does.

**Status.** Designed ([09](../../proposal/09-modules-packages-and-build.md)).

### sisu publish

`sisu publish` uploads a package to the registry: it compares the public
API with the previous version and rejects a breaking change without a
major version (`--breaking` records the reason), runs the previous
version's black-box tests and doc examples against the new one, records
provenance, and rejects typed holes.

**Status.** Designed ([09](../../proposal/09-modules-packages-and-build.md));
behavioural semver is a candidate of [19](../../proposal/19-novel-features.md).

### sisu audit's other reports

`sisu audit` lists the trusted packages, the `unsafe` blocks
(`--unsafe`), the capability summary and the pure label
(`--authority`), the typed holes and the security advisories of the
dependencies; `sisu audit --panics f` lists the panic sites a function
can reach.

**Status.** Designed ([10](../../proposal/10-tooling.md),
[09](../../proposal/09-modules-packages-and-build.md),
[22](../../proposal/22-unsafe.md)); the pure label leans (D-066).

### sisu review

`sisu review BASE..HEAD` lists, beside the code diff, each change that
weakens a promise: a removed or edited `ensures`, `invariant` or
refinement, an added `requires`, a deleted test or a changed expected
value, an updated snapshot, a dropped doc example, `pure` or
`@must_consume`. `--authority` adds the authority delta between the two
revisions.

**Status.** Leaning (D-066).

### sisu fix

`sisu fix --edition YEAR` rewrites a package for a new edition, and
`sisu fix --deps` replaces calls of a dependency's deprecated items with
their `@fix_inline` bodies.

**Status.** Designed ([10](../../proposal/10-tooling.md), "Migration";
[13](../../proposal/13-evolution-and-governance.md)); `--deps` leans
(D-067).

### sisu why

`sisu why FACT FILE:LINE` explains an inferred fact (an error set, `Send`
or `Sync`, `pure`, a capability's reach, the choice of an impl, a
compile-time budget), in a human form and a JSON form.

**Status.** Leaning (D-065).

### sisu eval

`sisu eval FILE` renders a module of `pure` constants as JSON, TOML or
YAML (`--to yaml`), for configuration.

**Status.** Leaning (D-063; [08](../../proposal/08-metaprogramming.md)).

### sisu fill

`sisu fill` asks proposers for fills of a typed hole, checks them, runs
them against the tests, contracts and properties that reach the hole,
and shows the survivors for a person to choose.

**Status.** Later tooling ([10](../../proposal/10-tooling.md),
"AI-assisted development").

### Options of the existing commands

- `sisu doc --digest` writes a plain-text digest of each package's
  public API for a model's context; `sisu doc --open` serves the pages
  locally. Designed ([10](../../proposal/10-tooling.md)).
- `sisu run --profile` samples the run with the platform's profiler.
  Designed ([10](../../proposal/10-tooling.md), "Debugging and
  profiling").
- `sisu test --fuzz`, `--cover` and `--mutate` run coverage-guided
  fuzzing, report uncovered regions and run mutation testing on the
  changed region; `--format` writes TAP or JUnit XML. Designed
  ([17](../../proposal/17-testing.md)).
- `sisu build --target TARGET` cross-compiles, and `--explain-size`
  reports what monomorphisation cost. Designed
  ([09](../../proposal/09-modules-packages-and-build.md),
  [12](../../proposal/12-interop-and-targets.md),
  [03](../../proposal/03-types-and-generics.md)).
- `sisu check --deny-warnings` fails on a warning, for CI. Designed
  ([09](../../proposal/09-modules-packages-and-build.md)).
- `sisu explain --raw-types` shows a type as it is, without the name a
  library gives it with `@diagnostic_name`. Leaning (D-065).

### sisu learn

`sisu learn` would run the tutorial's exercises: copy them into a
working directory, report each as not started, in progress or solved,
rerun the current one on every save, and show hints and solutions.

**Status.** A plan of this manual ([README](../README.md#running-the-exercises)),
open as Q-63.

## Diagnostics of the command line

The driver's messages have codes D0001 to D0119. These are the ones that
a command line can produce, by what they report:

| Codes | Report |
|---|---|
| D0001, D0013 | A file that cannot be read or written. |
| D0002, D0003, D0004, D0009, D0023 | The usage, an unknown command, a missing argument, an unknown option, a value that is not a number. |
| D0005, D0069 | A program without `main`; a library run as an application. |
| D0006, D0021, D0114 | A program with errors that did not run, or was not documented. |
| D0007, D0008, D0053, D0060 to D0063 | The lines of `sisu test`: a test's outcome, the counts, the tests not selected, transcripts. |
| D0010, D0011 | `sisu explain`. |
| D0012, D0076, D0118 | Internal errors (status 70). |
| D0014 to D0020 | `sisu explain-borrow`. |
| D0022 | `sisu audit` without an audit. |
| D0024 to D0027, D0041 to D0050 | Schedules explored, and the schedulers. |
| D0028, D0029 | `--contracts`. |
| D0030 to D0040 | Replay tokens, capability logs and replays. |
| D0051, D0052 | `--faults`. |
| D0054 to D0056 | Snapshots. |
| D0057 to D0059 | `sisu bench`. |
| D0064 to D0067, D0112 | Packages, lockfiles, workspaces and features. |
| D0070 to D0073 | `sisu build` and `sisu new`. |
| D0074, D0075, D0077 | `sisu fmt`. |
| D0078 | `--watch`. |
| D0079 to D0093 | `sisu repl` and `sisu ws`. |
| D0094 | `--stats`. |
| D0095 to D0102 | The language server's answers. |
| D0103 to D0111, D0119 | The native backend. |
| D0113 | `sisu doc --std`. |
| D0115 to D0118 | The front end. |

`sisu explain CODE` prints any of them, and [R17, Diagnostics](diagnostics.md#the-driver)
has an entry for each one that the manual shows, with the fix.
