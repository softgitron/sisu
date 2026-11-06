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
  capability logs it writes, and `sisu --version` prints it.
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
  options when there is no file, as in `sisu run -- ARGS`, and a `--`
  right after the file, `sisu run FILE -- ARGS`, does the same: the
  program does not receive it, and a second `--` is the program's.
- An option that the command does not take is D0009, a command that
  does not exist is D0003, and an option given a word where it needs a
  number is D0023. A command without the argument it needs says what it
  needs: a file (D0004), a code for `sisu explain` (D0129) or a name for
  `sisu new` (D0130). Each is a usage error, with exit status 2.
- `sisu help` (also `sisu --help` and `sisu -h`) prints the list of
  commands to standard output, with status 0. `sisu` alone prints the
  same list to standard error, with status 2.
- `sisu --version` (also `sisu version`) prints the toolchain's version
  (D0131), the one that replay tokens and capability logs record.

<!-- output: examples/reference-sisu-command/usage.console -->
```console
$ sisu frob
error[D0003]: unknown command `frob`; run `sisu help`
$ sisu run --fast hello.sisu
error[D0009]: unknown option `--fast` for `sisu run`
$ sisu run --explore=many hello.sisu
error[D0023]: `--explore` needs a number, not `many`
$ sisu explain
error[D0129]: `sisu explain` needs a diagnostic's code, such as `E0300`, or its name, such as `type.mismatch`
$ sisu new
error[D0130]: `sisu new` needs the name of the package to create
$ sisu --version
sisu 0.0.1
```

The list that `sisu help` prints is the compiler's own summary of this
page, and its last paragraph names the commands whose front end is stage
1's ([Choosing the front end](#choosing-the-front-end)):

<details>
<summary>What <code>sisu help</code> prints</summary>

<!-- output: examples/reference-sisu-command/help.console -->
```console
$ sisu help
usage: sisu <command> [options] [<file>]

Without a file, a command works on the package of the current directory (`sisu.toml`).

commands:
  run [<file>] [args...]  check and run a program (`--watch` swaps in each save, `--sir` runs a SIR file, `--monitor` checks ownership while it runs, `--defer-borrow` runs a program the ownership checker rejected under the monitor, `--contracts=off|preconditions|all` sets what it checks, `--record LOG` writes its capability log, `--stats` reports the scheduler's numbers, `--stats stack-depth` a histogram of the frame depths, `--stats shared` each `Shared` construction site, `--native` compiles it to an executable and runs that, `--release` with the release profile, `--from-sir FILE` runs a file of serialised SIR, natively with `--native`, and `--native --from-lir FILE` a file of LIR, stage 2's lowering of SIR)
  check [<file>]         report diagnostics (`--format=ids` for the conformance form, `--diary LOG` appends them and what resolved the last check's to a JSONL log, `--holes <file>` lists each typed hole of the program with the type its place needs and the bindings in scope that fit it)
  doc [<path>]           the documentation, a Markdown page per module, printed each after a line naming its file (`--output DIR` writes the pages into DIR instead, `--std [MODULE...]` documents the standard library, or the modules named, as `std.time`)
  build [<file>]         check and lower a package, writing its lockfile (`--native` also compiles a program to an executable: a package's to `build/debug/NAME` beside its lockfile, `build/release/NAME` with `--release`, a file's beside it, named by its stem, or at `--output PATH`; `--native --from-sir` compiles a file of serialised SIR, `--native --from-lir` a file of LIR)
  test [<file>]          run the tests and transcripts under the ownership monitor (`--no-monitor` without it, `--no-cache` runs the tests that passed before too, `--jobs N`, `--filter TEXT`, `--tag T`, `--skip-tag T`, `--update` for snapshots and transcripts, `--replay TOKEN`, `--sim` with `--faults P`, `--std` for the standard library's doc examples, `--from-sir FILE` runs the tests of a file `sisu sir --tests` printed, `--native` compiles them and runs each natively, `--release` with the release profile)
  bench [<file>]         run the benchmarks (`--iterations N`, `--samples K` times each after a run not timed, `--from-sir FILE` those of a file `sisu sir --tests` printed)
  replay <log>           run a recorded program again against its capability log
  fmt [<path>...]        format files in place (`--check` reports what would change)
  repl [<file>|<dir>]    an interactive session, with a package loaded from its directory (`--echo` writes each input after its prompt, for a session read from a pipe)
  ws <file>              run a worksheet, showing each statement's value (`--run` with the real `System`)
  lsp                    the language server, over standard input and output
  new <name>             create a package (`--lib` for a library)
  sir [<file>]           print the serialised SIR (`--tests`: of the program built with its tests and benchmarks)
  tokens <file>          print the token stream (`--trivia` includes trivia)
  parse <file>           `--tree` prints the syntax tree, `--ast` the AST built from it, `--print` reprints the source, `--shape` prints what formatting keeps
  explain <code>         describe a diagnostic code
  explain-borrow <file>:<line>  show every borrow live on a line
  audit --ownership <file>...  count the uses of `Shared`, `Weak`, `Cell`, `Arena`, `Key`, `.copy()` and `unsafe` in each file
  audit --caps [<file>...]  the capability parameters of each function that does I/O, the depth from `main` of each use of a capability, and the structs bundling three or more (without a file, of the package's program)
  audit --unsafe [--std] [<path>]  every `unsafe` block and function of a program by module, with its `# safety:` reason, and its package's level
  lint [<path>]          the lints beyond the compiler's warnings, with their fixes
  fix <file>             apply the preferred fix of each diagnostic of the file, checking it again until none applies (`--dry-run` prints the edits and leaves the file, `--all` fixes the lints too), from stage 1
  eval <file>            the constants of a configuration module, as JSON (`--to toml` or `--to yaml` for TOML or YAML), from stage 1's compile-time evaluator

`run`, `check`, `build`, `doc`, `test` and `bench` take `--locked`, which keeps `sisu.lock` as it is, and `--features NAMES` (the package's features to turn on, separated by commas, `DEP/FEATURE` for a dependency's) and `--no-default-features`. `run` and `test` take `--schedule-seed N`, `--explore N` (N seeds), `--scheduler random|pct|exhaustive` and `--depth N`.

The front end is stage 1's, the one written in Sisu, for `check`, `sir`, `build`, `run`, `test`, `bench`, `fmt`, `doc`, `tokens`, `parse` (`--shape` too), `repl`, `ws` and `eval`, which only it has; `--front-end=rust` before a command's file, or `SISU_FRONT_END=rust` for a whole process, runs the Rust front end, the oracle, instead.
```

</details>

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
  on every member in turn (`sisu test` names each before its list,
  D0137) and end with the worst status of the members,
  where a member's failure outweighs another's blocked tests (3);
  `sisu run`, `sir` and `bench` work on the one member that is an
  application, and stop with D0067 when there is more than one.

Every command that checks a package also writes its lockfile,
`sisu.lock`, when the manifests imply another one; `--locked` refuses
to, as [Package options](#package-options) says.

This package, `till`, adds up the prices given to it:

<!-- file: examples/reference-sisu-command/till.pkg/sisu.toml -->
```toml
[package]
name = "till"
version = "0.1.0"
edition = "2027"

[dependencies]
```

<!-- file: examples/reference-sisu-command/till.pkg/src/main.sisu -->
```sisu
##! A till that adds up the prices it is given.

def total(prices: list[int]) -> int:
    return sum(prices)

def main(sys: System):
    var prices: list[int] = []
    for arg in sys.env.args():
        prices.push(arg.parse_int().or(0))
    print("total {total(prices)}")

test "an empty basket costs nothing":
    assert total([]) == 0
```

In the package's directory, each command works on the whole package:

<!-- output: examples/reference-sisu-command/till.console -->
```console
$ sisu check
$ sisu run -- 12 5 30
total 47
$ sisu test
test "an empty basket costs nothing" ... ok
1 passed, 0 failed, 0 blocked
$ sisu build
built the package `till` 0.1.0 (modules: 1)
$ sisu doc
<!-- till.md -->
# `till`

A till that adds up the prices it is given.
$ sisu fmt --check
```

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

<!-- file: examples/reference-sisu-command/script.sisu -->
```sisu
#!/usr/bin/env sisu
# A script: its top-level statements run in order, with `sys` in scope.
for name in sys.env.args():
    print("Hei, {name}!")
```

<!-- output: examples/reference-sisu-command/script.console -->
```console
$ sisu script.sisu Aino Eero
Hei, Aino!
Hei, Eero!
$ sisu run script.sisu Roni
Hei, Roni!
```

**Status.** In stage 0.

### Exit statuses

Every command ends with one of these statuses:

| Status | Means |
|---|---|
| 0 | The command did what it was asked: the program ran and ended normally, every test passed, the file is formatted. |
| 1 | The program has errors (and did not run), a test failed, `main` returned an error, a replay diverged, or a file could not be read or written. |
| 2 | A usage error: an unknown command or option, a missing argument, a value of the wrong form, or no package where one is needed. |
| 3 | `sisu test` found no failure, and a test is blocked by a typed hole (D-061). |
| 70 | An internal compiler error (D0012, D0076, D0118): a bug in `sisu`, never in your program. |
| 101 | The program panicked. |
| 134 | The program aborted: a panic while unwinding from a panic, a panic leaving a function that C called through `@export`, or the ownership monitor stopping a run at a conflict. |

A program that panics prints the panic to standard error and ends
with status 101. This one prints its first line, then the panic:

<!-- file: examples/reference-sisu-command/panics.sisu -->
```sisu
# A panic ends the program with exit status 101.
def main():
    let scores = [3, 1, 4]
    print("the first score is {scores[0]}")
    print("the fourth score is {scores[3]}")
```

<!-- output: examples/reference-sisu-command/panics.err -->
```text
panic: index out of bounds: the length is 3 but the index is 3
  at main (panics.sisu:5:33)
```

An error that `main` returns is printed to standard error, with the
place that passed it on, and the status is 1:

<!-- file: examples/reference-sisu-command/fails.sisu -->
```sisu
# An error that `main` returns ends the program with exit status 1.
error ConfigError:
    Missing(key: String)

def port(key: str) -> int ! ConfigError:
    return Err(Missing(key.to_string()))

def main() -> ! ConfigError:
    print("starting")
    print("port {port("port")?}")
    return Ok(())
```

<!-- output: examples/reference-sisu-command/fails.err -->
```text
error: Missing(key="port")
  at fails.sisu:10:18
```

The error is printed in its `Debug` form, `Missing(key="port")`.

**Status.** In stage 0.

### Choosing the front end

- `sisu COMMAND --front-end=stage1|rust ...`
- `SISU_FRONT_END=stage1|rust sisu COMMAND ...`

The front end reads, checks and lowers a program; the rest of `sisu`
runs, tests and compiles what it produces. Stage 0 has two: stage 1,
written in Sisu and compiled natively, which is the default for
`check`, `sir`, `build`, `run`, `test`, `bench`, `fmt`, `doc`, `tokens`,
`parse` (`--shape` too), `repl` and `ws`, and the only one for `eval`,
as `sisu help` says, and the Rust front end, the oracle that stage 1 is checked against. `--front-end=rust` among a command's options, or
`SISU_FRONT_END=rust` in the environment, chooses the Rust one. `sisu
lint`, `sisu audit --unsafe`, `sisu eval`, `sisu fix` and `sisu check
--holes` are stage 1's alone, and the Rust front end refuses them; `sisu lsp` answers from both
([sisu lsp](#sisu-lsp)). The other commands, and `run --watch` and
`--defer-borrow`, `test --std` and `--defer-borrow`, and `check
--diary` use the Rust front end whichever you choose. A program's
transcripts run on the front end its tests run on, stage 1 unless you
choose the Rust one (with a stage 1 older than 0.11.0, on the Rust
front end, D0128). `sisu
test` and `sisu bench` run stage 1 once for each program, and print its
warnings once, before the report.

<!-- output: examples/reference-sisu-command/front_end.console -->
```console
$ sisu run --front-end=rust hello.sisu
Hei, maailma!
$ sisu run --front-end=fast hello.sisu
error[D0115]: `--front-end` takes `stage1` (the front end written in Sisu, the default) or `rust` (the Rust front end, the oracle), not `fast`
```

A value other than `stage1` or `rust` is D0115. When stage 1 is missing
(D0116), fails to build (D0117) or stops as a compiler may not (D0118),
the message names `--front-end=rust` as the way on. Stage 1's own
executable, `sisu1`, says its version with `sisu1 --version` (`sisu1
0.15.0`, its minor number the round of its rules), which the driver asks
to learn what that stage 1 can do; `sisu --version` is the toolchain's.

**Status.** In stage 0, for the compiler's own development (its
decision 268 in the [architecture
notes](../../compiler/docs/architecture.md)); the proposal has one
front end.

### The server of stage 1

- `SISU_STAGE1_SERVE=0 sisu COMMAND ...`

A command whose front end is stage 1 asks a server of stage 1 rather
than starting stage 1 for itself. The first command that finds no
server starts one, a process of `sisu` in the background whose socket
is under `target/stage1/serve/` beside the driver, and every later
command of any directory sends it its words and reads its answer as it
would read a process's output. The server keeps up to one process of
stage 1 for each core, each with the standard library's base already
made, and ends 300 seconds after its last request
(`SISU_STAGE1_SERVE_IDLE` sets the seconds) or when `sisu` is built
again; a process of stage 1 that was built again is replaced. A two-line `sisu check` takes 0.014 seconds served
where it takes 0.244 as a process of its own.

Served are `check`, `sir`, `parse`, `fmt`, `lint`, `doc`, `tokens`,
the builds of `test` and `bench`, `eval`, `check --holes`, `fix`, the
language server's questions ([sisu lsp](#sisu-lsp)), and from `sisu1
0.15.0` the build that gives `sisu run` its SIR, also for `sisu run
--native` of a file: a two-line `sisu run` takes 0.039 seconds served
where it takes 0.246 as a process, and `sisu run --native` 0.121 where
it takes 0.347 (with a release build of `sisu`). `sisu build`, with
`--native` too, and `sisu run --native` of a package still start stage
1 as a process of their own, since stage 1 serves only the build for a
run. Whatever goes wrong with the server, the command runs stage 1
as a process of its own, and `SISU_STAGE1_SERVE=0` makes it do so every
time. What a command prints is the same either way.

**Status.** In stage 0, from `sisu1 0.13.0` (decisions 942, 980 and 1061 of
the [architecture notes](../../compiler/docs/architecture.md)).

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

<!-- file: examples/reference-sisu-command/hello.sisu -->
```sisu
# The program most examples on this page run.
def main():
    print("Hei, maailma!")
```

<!-- output: examples/reference-sisu-command/hello.out -->
```text
Hei, maailma!
```

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

<!-- file: examples/reference-sisu-command/watch.sisu -->
```sisu
# Under `sisu run --watch`, a save swaps in the bodies that changed.
from std.time import Ticker

def frame(n: int) -> String:
    return "frame {n}: a circle"

def save_next_version(sys: System) -> ! IoError:
    let text = sys.fs.read_to_string("next.sisu")?
    sys.fs.write_text("watch.sisu", text)?
    return Ok(())

def main(sys: System) -> !:
    let start = DateTime.parse("2026-01-01T00:00:00Z")?
    var ticker = Ticker.every(Clock.frozen_at(start), Duration.millis(16))
    for n in range(3):
        ticker.wait()?
        print(frame(n))
        if n == 0:
            save_next_version(sys)?
    return Ok(())
```

`sisu run --watch watch.sisu` prints this to standard output, and to
standard error, after the first frame, a line that names the epoch and
the functions it replaced:

<!-- output: examples/reference-sisu-command/watch.watch -->
```text
frame 0: a circle
frame 1: a square
frame 2: a square
```

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

<!-- output: examples/reference-sisu-command/monitor.console -->
```console
$ sisu run --monitor hello.sisu
Hei, maailma!
```

**Status.** In stage 0, a tool of the experiment (18, Phase 3).

### Running a rejected program: --defer-borrow

`sisu run --defer-borrow` runs a program that the ownership checker
rejected, under the monitor, to find out whether the rejection is a
real conflict. The run is the counterexample: it stops at the first
conflict with the monitor's account and the blocks of the SIR of the
function where it happened that the conflict passes through (R0051),
the loan's, its end's and the use's, with `...` for the blocks left
out, and ends with status 134; or it ends without one, with
the account of why the checker still rejects the program.

<!-- file: examples/reference-sisu-command/moved.sisu -->
```sisu
# A list used after it moved.
def main():
    let names = ["Aino", "Eero"]
    let team = names
    print("{len(team)} in the team, {len(names)} names")
```

<details>
<summary>What <code>sisu run</code> and <code>sisu run --defer-borrow</code> print</summary>

<!-- output: examples/reference-sisu-command/moved.console -->
```console
$ sisu run moved.sisu
error[E0400]: `names` was moved on line 4 and cannot be used afterwards
  --> moved.sisu:5:42
  |
5 |     print("{len(team)} in the team, {len(names)} names")
  |                                          ^^^^^
  |
4 |     let team = names
  |                ----- `names` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
$ sisu run --defer-borrow moved.sisu
counterexample run: the ownership checker rejected this program; its errors are deferred to the monitor
ownership violation: `names` was used while it holds no value: it was moved out, or never assigned
  used at: moved.sisu:5:42 (`_10 = ref _1`)
  at main (moved.sisu:5:42)
SIR of `main`, its blocks that the conflict passes through:
fn main(0) source 0 @2:5-2:9 {
  ...
  bb3 (unwind bb4):
    _4 = call intrinsic fmt.new() @5:11-5:56
    _5 = ref mut _4 @5:11-5:56
    _7 = ref _2 @5:17-5:21
    _6 = call intrinsic len(copy _7) @5:13-5:22
    _8 = call intrinsic fmt.display(copy _5, copy _6, const "") @5:13-5:22
    _8 = call intrinsic fmt.text(copy _5, const " in the team, ") @5:11-5:56
    _10 = ref _1 @5:42-5:47
    _9 = call intrinsic len(copy _10) @5:38-5:48
    _8 = call intrinsic fmt.display(copy _5, copy _9, const "") @5:38-5:48
    _8 = call intrinsic fmt.text(copy _5, const " names") @5:11-5:56
    goto bb5
  ...
}
outcome: conflict reached; the run stopped at a concrete ownership conflict
```

</details>

`sisu test --defer-borrow` runs a rejected program's tests in the same
way.

**Status.** In stage 0, a tool of the experiment (18, Phase 3).

### What a run checks: --contracts

`--contracts=all|preconditions|off` sets the contracts a run checks
([R11](contracts.md#check-levels); [07](../../proposal/07-contracts.md)):
`all`, the default, checks `requires`, `ensures` and invariants;
`preconditions` checks `requires` only; `off` checks none. Another
value is D0028. `sisu test` always checks every contract, so `sisu test
--contracts` is D0029.

<!-- file: examples/reference-sisu-command/contracts.sisu -->
```sisu
# A postcondition that a bug breaks, and a precondition that a call breaks.
def rounded_up(amount: int, step: int) -> int:
    requires step > 0
    ensures result >= amount
    return amount // step * step

def main():
    print(rounded_up(7, 5))
    print(rounded_up(7, 0))
```

<!-- output: examples/reference-sisu-command/contracts.console -->
```console
$ sisu run contracts.sisu
panic: postcondition of `rounded_up` failed: result >= amount
    result >= amount
    │         │
    5         7
  at rounded_up (contracts.sisu:4:13)
  at main (contracts.sisu:8:11)
$ sisu run --contracts=preconditions contracts.sisu
5
panic: precondition of `rounded_up` failed: step > 0
    step > 0
    │
    0
  at rounded_up (contracts.sisu:3:14)
  at main (contracts.sisu:9:11)
$ sisu run --contracts=off contracts.sisu
5
panic: division by zero
  at rounded_up (contracts.sisu:5:12)
  at main (contracts.sisu:9:11)
```

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

<!-- file: examples/reference-sisu-command/reader.sisu -->
```sisu
# Reads a file through the file system it is given.
def main(sys: System) -> ! IoError:
    let text = sys.fs.read_to_string("poem.txt")?
    for (i, line) in enumerate(text.lines()):
        print("{i + 1}: {line}")
    return Ok(())
```

With a file `poem.txt` of two lines:

<!-- output: examples/reference-sisu-command/reader.console -->
```console
$ sisu run --record run.log reader.sisu
1: Hei
2: maailma
$ sisu replay run.log
1: Hei
2: maailma
$ sisu run --record quiet.log --redact fs reader.sisu
1: Hei
2: maailma
$ sisu replay quiet.log
error[D0037]: the replay diverged from the log: record 2 of the log is a call to `fs` whose result was redacted, but the program made the call read_file "poem.txt"
```

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

<!-- output: examples/reference-sisu-command/stats.console -->
```console
$ sisu run --stats hello.sisu
Hei, maailma!
stats: 1 tasks, at most 1 at once, 0 switches
stats: scheduling latency p50 0 p99 0 max 0 statements; p50 0 p99 0 microseconds
stats shared: 0 cells from task-local sites, 0 from may-cross sites, 0 reached by another task
$ sisu run --stats stack-depth hello.sisu
Hei, maailma!
stats stack-depth: 1 calls, the deepest stack 1 frames, about 96 bytes of native stack there (16 per SIR local and 64 per frame)
stats stack-depth: 1 calls at a depth of 1 to 1 frames
$ sisu run --stats shared hello.sisu
Hei, maailma!
stats shared: 0 of 0 `Shared` construction sites tagged task-local, 0 may-cross
```

**Status.** In stage 0, for the experiment's measurements (18, Phase 6).

### Compiling and running natively: --native and --release

`sisu run --native` compiles the program to an executable with the
native backend, as `sisu build --native` does, and runs it.
`--release` builds with the release profile, which leaves out the
checks it can prove unnecessary; it goes with `--native` only (D0111),
since the interpreter has no profiles. A native run takes the schedule
options, `--scheduler pct` and `--scheduler exhaustive` with `--depth`
among them ([Choosing the schedule](#choosing-the-schedule)), and
`--stats` with no word after it; it does not combine with `--sir`,
`--monitor`, `--defer-borrow`, `--watch` or `--record` (D0109). A
native debug build checks raw memory as the interpreter does, with the
same panics: the blocks of `Global` and of C's `malloc`, the text of
`getenv`, and pointers to the program's own variables, whose frame
must not have returned and which a shared borrow made read-only. It
checks a pointer by its address, where the interpreter knows the borrow
it came from, so it lets pass a few accesses that the interpreter
stops; the release build checks none
([chapter 27](../tutorial/27-systems.md#raw-memory)).

<!-- output: examples/reference-sisu-command/native.console -->
```console
$ sisu build --native hello.sisu
built `hello`
$ sisu run --native hello.sisu
Hei, maailma!
$ sisu build --native --release --output hei hello.sisu
built `hei`
$ sisu run --release hello.sisu
error[D0111]: `--release` builds with the native backend's release profile (09); it goes with `--native`
```

**Stage 0 differs.** The native backend compiles every program that the
conformance suite runs, and reports what it does not compile yet with
D0103. The proposal's debug backend is its own and its release backend
is LLVM ([10](../../proposal/10-tooling.md)); stage 0 compiles a debug
build with its backend written in Sisu and a release build with
Cranelift, which stands in for LLVM (`--backend=cranelift` asks for
Cranelift in a debug build too).

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

<!-- output: examples/reference-sisu-command/sir.console -->
```console
$ sisu sir hello.sisu
sir 0
source 0 "hello.sisu"
entry main

intrinsic print

fn main(0) source 0 @2:5-2:9 {
    let _0: ()
    let _1: ()
  bb0:
    _1 = call intrinsic print(const "Hei, maailma!") @3:5-3:27
    _0 = const ()
    return
}
$ sisu run --sir hello.sir
Hei, maailma!
$ sisu run --from-sir hello.sir
Hei, maailma!
```

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
([R9](concurrency.md#the-scheduler);
[06](../../proposal/06-concurrency-and-capabilities.md)).

- `--schedule-seed N` runs with seed `N`; the default is 0.
- `--explore N` runs `N` schedules, from the first seed on, and stops at
  the first run that fails, naming its seed (D0025); a run of `sisu
  run` that exits as the first seed's did but prints other standard
  output stops it too (D0127, [R17](diagnostics.md#d0127)), since its
  output depends on the schedule; what is printed is that run's output.
  When all succeed it says so (D0024).
  Under `sisu test` every test runs under each schedule (D0026, D0027),
  and a test that fails shows what it printed under the schedule that
  failed.
- `--scheduler random` is the default, a random choice at each yield
  point. `--scheduler pct` is probabilistic concurrency testing, which
  finds bugs of a small depth with a known probability; `--depth N` is
  that depth, 3 by default (D0042 to D0045). `--scheduler exhaustive`
  searches every schedule with at most `--depth N` preemptions, 2 by
  default, up to 10,000 schedules unless `--explore N` says otherwise
  (D0046 to D0050). Another scheduler is D0041.

This program expects the first task it spawns to finish first, which
some schedules break:

<!-- file: examples/reference-sisu-command/race.sisu -->
```sisu
# The program expects Aino to arrive first, which some schedules break.
def arrive(order: Mutex[list[String]], name: str):
    task.yield_now()
    order.lock().push(name.to_string())

def main():
    let order = Mutex(list[String]())
    task.scope do s:
        s.spawn(() -> arrive(order, "Aino"))
        s.spawn(() -> arrive(order, "Eero"))
    let arrived = order.into_inner()
    print(arrived)
    assert arrived[0] == "Aino"
```

<!-- output: examples/reference-sisu-command/race.console -->
```console
$ sisu run --schedule-seed 1 race.sisu
["Aino", "Eero"]
$ sisu run --schedule-seed 1 --explore 5 race.sisu
["Eero", "Aino"]
panic: assertion failed: arrived[0] == "Aino"
    arrived[0] == "Aino"
    │
    "Eero"
  at main (race.sisu:13:5)
schedule seed 3 of 5 failed with exit status 101; `--schedule-seed 3` runs it again
$ sisu run --scheduler pct --explore 5 race.sisu
["Eero", "Aino"]
panic: assertion failed: arrived[0] == "Aino"
    arrived[0] == "Aino"
    │
    "Eero"
  at main (race.sisu:13:5)
schedule seed 1 of 5 failed with exit status 101 under a PCT scheduler of depth 3; `--scheduler pct --depth 3 --schedule-seed 1` runs it again
$ sisu run --scheduler exhaustive race.sisu
["Eero", "Aino"]
panic: assertion failed: arrived[0] == "Aino"
    arrived[0] == "Aino"
    │
    "Eero"
  at main (race.sisu:13:5)
schedule 0.0.1.0.0.0 failed with exit status 101: run 5 of a bounded exhaustive search (preemption bound 2)
$ sisu run --native --scheduler pct --explore 5 race.sisu
["Eero", "Aino"]
panic: assertion failed: arrived[0] == "Aino"
    arrived[0] == "Aino"
    │
    "Eero"
  at main (race.sisu:13:5)
schedule seed 1 of 5 failed with exit status 101 under a PCT scheduler of depth 3; `--scheduler pct --depth 3 --schedule-seed 1` runs it again
$ sisu run --native --scheduler exhaustive race.sisu
["Eero", "Aino"]
panic: assertion failed: arrived[0] == "Aino"
    arrived[0] == "Aino"
    │
    "Eero"
  at main (race.sisu:13:5)
schedule 0.0.1.0.0.0 failed with exit status 101: run 5 of a bounded exhaustive search (preemption bound 2)
```

Each failure names the options that run it again. `sisu run race.sisu`,
with seed 0, fails at once. A native run, `sisu run --native`, takes the
same options and runs the executable under the same schedulers, so a
seed or a schedule names the same order both ways, as the last two
commands show.

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
- `--accept-unsafe`, which `sir` takes too, accepts a package whose
  `unsafe` level is higher than the one `sisu.lock` records, and records
  the new level (E1022, [R13](modules.md#trust)).

`@cfg(feature="NAME")` reads the features ([R13](modules.md#features);
[09](../../proposal/09-modules-packages-and-build.md)).

**Status.** In stage 0.

## Checking and building

### sisu check

- `sisu check [--format=human|ids] [--diary LOG] [PACKAGE OPTIONS] [FILE]`
- `sisu check --holes [--format=human|json] FILE`

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
- `--holes` reports the typed holes instead, each with what fills it
  ([sisu check --holes](#sisu-check---holes)).

<!-- file: examples/reference-sisu-command/broken.sisu -->
```sisu
# A misspelt name stops the program before it runs.
def main():
    let total = 3
    print("{totl} items")
```

<!-- output: examples/reference-sisu-command/check.console -->
```console
$ sisu check hello.sisu
$ sisu check broken.sisu
error[E0200]: cannot find `totl` in this scope
  --> broken.sisu:4:13
  |
4 |     print("{totl} items")
  |             ^^^^
  = fix: a similar name is in scope: `total`

$ sisu check --format=ids broken.sisu
4:13-4:17 E0200 resolve.undefined-name name="totl"
  fix resolve.similar-name 4:13-4:17 "total"
$ sisu check --diary friction.jsonl broken.sisu
error[E0200]: cannot find `totl` in this scope
  --> broken.sisu:4:13
  |
4 |     print("{totl} items")
  |             ^^^^
  = fix: a similar name is in scope: `total`

```

**Stage 0 differs.** [10](../../proposal/10-tooling.md) sets a target
of under 300 ms for `sisu check` after an edit of one function in a
package of 100,000 lines. Stage 0 checks the program from nothing each
time, keeping only the standard library's base made in [the server of
stage 1](#the-server-of-stage-1), so a `sisu check` of a two-line
program takes 0.014 seconds served and 0.244 as a process of its own;
the language server, `--watch` and the REPL keep what they checked,
and recheck such an edit within the target.

**Status.** In stage 0.

### sisu check --holes

- `sisu check --holes [--format=human|json] FILE`

`sisu check --holes` reports each typed hole `todo("...")` of the
program's own modules, by module and position, with what the code that
fills it has to work with: the type its place needs and the function it
is in; the bindings in scope, with their types; the expressions of its
type, to depth two, a binding, a field of one, a method of one that
takes nothing else, or a function of the program of one binding; each
struct and enum of the program that its type names, with its fields or
variants; its function's `requires` as what is given and `ensures` as
what to keep; and the tests that reach it, the property test of its
contract among them. `--format=json` writes the same as an array of
one object for each hole. The status is 0, or 1 with `sisu check`'s
report when the program has an error.

<!-- file: examples/reference-sisu-command/holes.sisu -->
```sisu
# A hole with a contract around it, for `sisu check --holes`.
struct Chore:
    name: String
    hours: int

    def doubled(self) -> int:
        return self.hours * 2

## The hours to plan for `chore`, with a margin.
def planned(chore: Chore) -> int:
    requires chore.hours >= 0
    ensures result >= chore.hours
    return todo("add a margin")

test "a margin is never negative":
    assert planned(Chore("sauna", 2)) >= 2
```

<!-- output: examples/reference-sisu-command/holes.console -->
```console
$ sisu check --holes holes.sisu
holes.sisu:13:12: the typed hole `todo("add a margin")` of type `int`, in `planned`
  in scope: `chore: ref Chore`
  of its type: `chore.doubled()`, `chore.hours`
  given: `chore.hours >= 0`
  to keep: `result >= chore.hours`
  reached by the tests "a margin is never negative", "contract of planned"
$ sisu check --holes --format=json holes.sisu
[
  {"at": "holes.sisu:13:12", "name": "add a margin", "type": "int", "function": "planned", "scope": ["chore: ref Chore"], "fits": ["chore.doubled()", "chore.hours"], "types": [], "requires": ["chore.hours >= 0"], "ensures": ["result >= chore.hours"], "tests": ["a margin is never negative", "contract of planned"]}
]
```

A file that cannot be read is D0001, another option D0009, and no
file, a directory or more than one path prints the command's usage
(D0151). Without `--holes`, `sisu check` reports each hole as warning
W0001, and `sisu test` reports a test that reaches one as blocked.

**Stage 0 differs.** `sisu check --holes` is stage 1's, from `sisu1
0.15.0`, and the whole report and `--format=json` from `sisu1 0.16.0`;
the Rust front end answers it with D0009. 10's report also shows the
values each test saw at the hole, which wait for `sisu test` to record
them, and the bindings' capabilities, which their types show here.

**Status.** In stage 0 (D-061; [10](../../proposal/10-tooling.md),
"AI-assisted development"), with the differences noted. `sisu build
--release` rejects a hole (E0445); `sisu publish`, which would too, is
designed.

### sisu build

- `sisu build [PACKAGE OPTIONS] [FILE]`
- `sisu build --native [--release] [--contracts=LEVEL] [--explain-size] [--output PATH] [FILE]`
- `sisu build --native --from-sir FILE.sir | --from-lir FILE.lir`

`sisu build` checks and lowers the program without its tests, writes
the lockfile, and says what it built (D0066 for a package, D0070 for a
file); with an error it prints the diagnostics and D0132, "the program
was not compiled", where `sisu run` ends with D0006. With
`--native` it also compiles the program to an executable: a package's
to `build/debug/NAME` beside its lockfile, or `build/release/NAME` with
`--release`; a file's beside it, named by its stem; or to `--output
PATH`. The line it prints names the executable's path relative to the
working directory, as `build/debug/NAME`. A release build refuses a
program with a typed hole, E0445 at each hole
([R17](diagnostics.md#e0445)), where `sisu check` and a debug build
warn with W0001, so that a hole never ships. The [native
example](#compiling-and-running-natively---native-and---release) builds
one.

<!-- output: examples/reference-sisu-command/build.console -->
```console
$ sisu build hello.sisu
built `hello.sisu`
$ sisu build broken.sisu
error[E0200]: cannot find `totl` in this scope
  --> broken.sisu:4:13
  |
4 |     print("{totl} items")
  |             ^^^^
  = fix: a similar name is in scope: `total`

error[D0132]: the program was not compiled: errors found (1)
```

`--explain-size` says, after the `built` line, how large the
executable is (D0139) and what its bytes are, in seven parts that add
up to its size, each with its share (D0140 to D0146): the program's own
code (its functions, the standard library's instances among them), the
runtime written in Sisu, the runtime written in Rust, Rust's standard
library and the rest of the code, the debug information, the symbols,
and the data, the unwinding tables and the headers. The driver reads
them from the linked file. The numbers change with the runtime and
with how the `sisu` command itself was built (a debug build of `sisu`
links a runtime that is not optimised), so this output, from a debug
build of `sisu`, is an example rather than a check:

<!-- unchecked: the sizes differ between builds of the runtime and of sisu -->
```console
$ sisu build --native --explain-size hello.sisu
built `hello`
`hello` is 20819000 bytes:
        1106 bytes (0.0%), the program's own code
      526944 bytes (2.5%), the runtime in Sisu's code
      517827 bytes (2.5%), the runtime in Rust's code
      393626 bytes (1.9%), Rust's standard library's code and the rest of the code
    18286972 bytes (87.8%), the debug information
      516541 bytes (2.5%), the symbols
      575984 bytes (2.8%), the data, the unwinding tables and the headers
```

The option explains a native executable, so it goes with `--native`
(D0148, `cli.explain-size-native`, status 2), and a file that is not a
64-bit ELF executable, which is what the driver reads, is D0147:

<!-- output: examples/reference-sisu-command/explain_size.console -->
```console
$ sisu build --explain-size hello.sisu
error[D0148]: `--explain-size` explains the size of a native executable: it goes with `--native`
```

**Stage 0 differs.** [09](../../proposal/09-modules-packages-and-build.md)
gives builds a content-addressed cache and `--target` for cross
compilation, which stage 0 does not have.

**Status.** In stage 0.

### sisu sir

- `sisu sir [--contracts=LEVEL] [--tests] [FILE]`

`sisu sir` prints the program's SIR, the intermediate form that the
interpreter runs and the native backend compiles, in its text form, with
every drop placed and every `comptime` value computed. It prints the SIR
of a program that the ownership checker rejects as well; a program with
other errors gets its diagnostics and D0132, as a build does. `--tests`
includes the program's tests and benchmarks, and `--contracts` sets the
contracts the SIR checks. The [SIR example](#running-serialised-sir---sir---from-sir-and---from-lir)
shows the SIR of `hello.sisu`.

**Status.** In stage 0, for implementations of Sisu
([18](../../proposal/18-next-steps.md), "Bootstrapping").

### sisu eval

- `sisu eval [--to json|toml|yaml] FILE`

`sisu eval` evaluates each `const` of the module FILE with the
compile-time evaluator and writes the constants on standard output as
JSON, or as TOML or YAML with `--to`, keyed by their names, for a
configuration file ([R12](compile-time.md#configuration-files) has the
rules of each form). A constant whose evaluation stops is that error at
its name, and one that holds a value the form cannot write is E0617;
either way nothing is written and the status is 1. Without a file, or
with a form it does not write, it prints its usage (D0151) with status
2, as every command does whose usage stage 1 checks.

<!-- file: examples/reference-sisu-command/settings.sisu -->
```sisu
# A configuration: the constants are its keys.
struct Stove:
    kilowatts: int
    wood: bool = True

const SAUNA: String = "Kota"
const STOVE: Stove = Stove(kilowatts=9)
```

<!-- output: examples/reference-sisu-command/eval.console -->
```console
$ sisu eval settings.sisu
{
  "SAUNA": "Kota",
  "STOVE": {
    "kilowatts": 9,
    "wood": true
  }
}
$ sisu eval settings.sisu --to yaml
SAUNA: "Kota"
STOVE:
  kilowatts: 9
  wood: true
$ sisu eval
usage: sisu eval <file>

the constants of a configuration module, as JSON (`--to toml` or `--to yaml` for TOML or YAML), from stage 1's compile-time evaluator
```

**Stage 0 differs.** `sisu eval` is stage 1's, from `sisu1 0.14.0`,
and served as `sisu check` is ([The server of stage
1](#the-server-of-stage-1)); the Rust front end answers it with D0003.
08 renders the values through
`Encode`, where stage 0 writes each value's own structure.

**Status.** In stage 0, with the differences noted; rendering through
`Encode` leans (D-063; [08](../../proposal/08-metaprogramming.md#configuration-files)).
[Generate configuration](../guides/config.md) is the guide.

## Testing

### sisu test

- `sisu test [OPTIONS] [FILE]`

`sisu test` runs the program's tests: its `test` blocks, the examples in
its doc comments, the properties its contracts imply, and the
transcripts in the `tests/` directory beside the program. Each runs under the ownership
monitor, with fresh fakes for the capabilities it names, and ends `ok`,
`FAILED`, or `blocked by` a typed hole that it reached. Tests
run in parallel and are reported in the order they are declared. Each
failure is shown again after the list, with what the test printed
(D0135; a test that passes prints nothing) and a replay token, and so is
each blocked test, with what it printed, its hole's closure; the last
line counts the outcomes. The program's warnings come before the list,
as `sisu run` prints them, and a program with an error runs no test:
`sisu test` prints its diagnostics and the line D0006, or D0021 for the
ownership checker's errors. The status is 0 when every test passed, 1
when a test failed or the program has an error, and 3 when no test
failed and one is blocked.

<!-- file: examples/reference-sisu-command/tests.sisu -->
```sisu
# Tests that pass, fail and wait for a hole.
def double(x: int) -> int:
    return x * 2

def halve(x: int) -> int:
    return todo("halve")

test "doubles a number":
    assert double(21) == 42

test "doubles" for x, y in [(-1, -2), (-3, -5)]:
    assert double(x) == y

test "halves a number":
    assert halve(42) == 21

suite "large numbers":
    @tag("slow")
    test "doubles a large number":
        assert double(1 << 40) == 1 << 41
```

<!-- output: examples/reference-sisu-command/tests.console -->
```console
$ sisu test tests.sisu
warning[W0001]: typed hole `halve` of type `int`
  --> tests.sisu:6:12
  |
6 |     return todo("halve")
  |            ^^^^^^^^^^^^^

test "doubles a number" ... ok
test "doubles (-1, -2)" ... ok
test "doubles (-3, -5)" ... FAILED
test "halves a number" ... blocked by todo("halve")
test "large numbers/doubles a large number" ... ok

test "doubles (-3, -5)" ... FAILED
panic: assertion failed: double(x) == y
  left:  -6
  right: -5
  at test "doubles (-3, -5)" (tests.sisu:12:5)
replay: sisu test --replay 1-ed778be29924c09d-0.0.1-t2-s0-r0 tests.sisu

test "halves a number" ... blocked by todo("halve")
what the test printed:
  in scope at the typed hole `todo("halve")`:
    x = 42

3 passed, 1 failed, 1 blocked
$ sisu test --filter large tests.sisu
warning[W0001]: typed hole `halve` of type `int`
  --> tests.sisu:6:12
  |
6 |     return todo("halve")
  |            ^^^^^^^^^^^^^

test "large numbers/doubles a large number" ... ok
1 passed, 0 failed, 0 blocked
4 not selected
$ sisu test --tag slow tests.sisu
warning[W0001]: typed hole `halve` of type `int`
  --> tests.sisu:6:12
  |
6 |     return todo("halve")
  |            ^^^^^^^^^^^^^

test "large numbers/doubles a large number" ... ok
1 passed, 0 failed, 0 blocked
4 not selected
$ sisu test --skip-tag slow --jobs 1 --no-monitor --no-cache tests.sisu
warning[W0001]: typed hole `halve` of type `int`
  --> tests.sisu:6:12
  |
6 |     return todo("halve")
  |            ^^^^^^^^^^^^^

test "doubles a number" ... ok
test "doubles (-1, -2)" ... ok
test "doubles (-3, -5)" ... FAILED
test "halves a number" ... blocked by todo("halve")

test "doubles (-3, -5)" ... FAILED
panic: assertion failed: double(x) == y
  left:  -6
  right: -5
  at test "doubles (-3, -5)" (tests.sisu:12:5)
replay: sisu test --replay 1-ed778be29924c09d-0.0.1-t2-s0-r0 tests.sisu

test "halves a number" ... blocked by todo("halve")
what the test printed:
  in scope at the typed hole `todo("halve")`:
    x = 42

2 passed, 1 failed, 1 blocked
1 not selected
$ sisu test --replay 1-ed778be29924c09d-0.0.1-t2-s0-r0 tests.sisu
warning[W0001]: typed hole `halve` of type `int`
  --> tests.sisu:6:12
  |
6 |     return todo("halve")
  |            ^^^^^^^^^^^^^

test "doubles (-3, -5)" ... FAILED

test "doubles (-3, -5)" ... FAILED
panic: assertion failed: double(x) == y
  left:  -6
  right: -5
  at test "doubles (-3, -5)" (tests.sisu:12:5)
replay: sisu test --replay 1-ed778be29924c09d-0.0.1-t2-s0-r0 tests.sisu

0 passed, 1 failed, 0 blocked
```

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
- `FILE`, a module under a package's `src/`, runs that module's tests
  only, with the package compiled as `sisu test` compiles it, so the
  module imports the modules beside it; the replay line names the
  module's file; a token of the module's run given to the package's
  is D0149, and one of the package's run given to the module's D0150,
  each naming the run that replays it
  ([R15](testing.md#the-tests-of-one-module)).
- `--replay TOKEN` runs the failing test that the token names, with the
  same seeds and the same shrunk case, so that it fails in the same
  way. A token for other code is D0033, one for a test that is not
  there D0035, and one from another toolchain D0034; text that is not a
  token is D0032. A test that takes the real `System` has no token
  (D0031).
- `--update` accepts what the snapshots and transcripts now show,
  rewriting them (D0054, D0055, D0063).
- `--sim` runs the tests that name their capabilities in the `sim`
  runtime profile, against simulated ones; `--faults P` sets the
  probability that a capability call fails with an error of its
  declared set, 0.01 by default (D0051, and D0052 without `--sim`). A
  test that takes the real `System` gets the real one, which no fault
  reaches, and its failure has no token (D0138).
- `--std` runs the standard library's doc examples besides the
  program's tests; outside a package and without a file, it runs them
  alone.
- `--native` compiles the tests into one executable and runs each in a
  process of its own, and `--release` compiles them with the release
  profile ([R15](testing.md#native-tests)), with the schedulers,
  `--faults` and `sim.always` as the interpreter runs them; what only
  the interpreter does, a test parameter that no fake stands for, is
  D0134.
- `--defer-borrow` runs the tests of a program that the ownership
  checker rejected, as [`sisu run
  --defer-borrow`](#running-a-rejected-program---defer-borrow) does.
- The schedule options and the package options are as for `sisu run`.

A snapshot test stores what it saw the first time `--update` accepts it,
in `FILE.snap` beside a file:

<!-- file: examples/reference-sisu-command/snapshot.sisu -->
```sisu
# A snapshot test compares a value with the snapshot stored for it.
from std.test import assert_snapshot

def greeting(names: list[str]) -> String:
    var text = String.new()
    for name in names:
        text += "Hei, {name}! "
    return text.trim().to_string()

test "greets everyone":
    assert_snapshot(greeting(["Aino", "Eero", "Roni"]))
```

<!-- output: examples/reference-sisu-command/snapshot.console -->
```console
$ sisu test snapshot.sisu
test "greets everyone" ... FAILED

test "greets everyone" ... FAILED
panic: there is no snapshot `greets everyone` yet (`sisu test --update` stores this one):
Hei, Aino! Hei, Eero! Hei, Roni!
  at test "greets everyone" (snapshot.sisu:11:5)
replay: sisu test --replay 1-d80205220117e448-0.0.1-t0-s0-r0 snapshot.sisu

0 passed, 1 failed, 0 blocked
$ sisu test --update snapshot.sisu
test "greets everyone" ... ok
1 passed, 0 failed, 0 blocked
stored the new snapshot `greets everyone`:
Hei, Aino! Hei, Eero! Hei, Roni!
$ sisu test snapshot.sisu
test "greets everyone" ... ok
1 passed, 0 failed, 0 blocked
```

A test that names the file system it uses, `fs: Fs`, writes to a fake
one when it runs, and to a simulated one under `--sim`, where some
writes fail. A test that takes the real `System` gets the real one in
both runs: it runs alone, after the others, and no fault reaches it:

<!-- file: examples/reference-sisu-command/sim.sisu -->
```sisu
# Under `--sim`, a test that names the file system gets a simulated one,
# and a test that takes the real `System` gets the real one.
def save_notes(fs: Fs, count: int) -> int ! IoError:
    for i in range(count):
        fs.write_text(Path.new("note{i}.txt"), "note {i}")?
    return Ok(count)

test "every note is saved"(fs: Fs):
    assert save_notes(fs, 10) == Ok(10)

test "the real disk keeps them too"(sys: System):
    assert save_notes(sys.fs, 3) == Ok(3)
```

<!-- output: examples/reference-sisu-command/sim.console -->
```console
$ sisu test sim.sisu
test "every note is saved" ... ok
test "the real disk keeps them too" ... ok
2 passed, 0 failed, 0 blocked
$ sisu test --sim --faults 0.2 sim.sisu
test "every note is saved" ... FAILED
test "the real disk keeps them too" ... ok

test "every note is saved" ... FAILED
panic: assertion failed: save_notes(fs, 10) == Ok(10)
  left:  Err(InvalidData(path="note0.txt"))
  right: Ok(10)
  at test "every note is saved" (sim.sisu:9:5)
replay: sisu test --replay 1-e3c3fb213982d560-0.0.1-t0-s0-r0-m200000 sim.sisu

1 passed, 1 failed, 0 blocked
$ sisu test --faults 0.2 sim.sisu
error[D0052]: `--faults` sets the fault rate of the `sim` profile; add `--sim`
```

[R15](testing.md) describes the tests themselves;
[17](../../proposal/17-testing.md) is their design.

**Stage 0 differs.** [17](../../proposal/17-testing.md) has `sisu test`
write a JSON event stream, with TAP and JUnit XML as `--format` options;
stage 0 prints the human form only, and has none of `--fuzz`, `--cover`
and `--mutate` ([Commands that are designed](#commands-that-are-designed)).

**Status.** In stage 0. `--sim`, `--faults` and replay tokens are
prototypes of the experiment (Q-49, D-070).

### sisu bench

- `sisu bench [--iterations N] [--samples K] [PACKAGE OPTIONS] [FILE]`

`sisu bench` runs each of the program's `bench` blocks once without
timing it, to warm up, and then times `--samples K` samples (10 by
default; fewer than 2 is D0136, since the interval needs two), each
`--iterations N` turns of its loop (100 by default). Its
line (D0057) gives the median time of one iteration over the samples,
the 95% interval of the mean of the samples that are not outliers, by
Student's t, and how many samples were outliers: further from the
median than three scaled median absolute deviations (1.4826 times the
MAD). The last line counts the benchmarks and the failures. A benchmark
that panics fails, with status 1.

<!-- file: examples/reference-sisu-command/bench.sisu -->
```sisu
# A benchmark runs its loop many times and reports the time of one.
from std.test import Bench, black_box

def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

bench "fib of 15"(b: Bench):
    for _ in b.loop():
        black_box(fib(15))
```

`sisu bench bench.sisu` prints a line like this one, with times that
differ from run to run:

<!-- unchecked: a benchmark's time differs on every run -->
```text
bench "fib of 15" ... 1.8 ms an iteration, the median of 10 samples of 100 iterations (95% interval of the mean 1.7 ms to 2.0 ms, 3 outliers)
1 benchmarks, 0 failed (stage 0 times the interpreter: compare benchmarks with each other, not with compiled code)
```

A wide interval, or many outliers, says that the machine was busy
while it timed: run again with more samples before comparing.

In a package, `sisu bench` runs the benchmarks of all its modules, and
`sisu bench src/MODULE.sisu` those that one module declares, the
package compiled as the whole run compiles it, so the module's imports
of the package's other modules resolve, as `sisu test src/MODULE.sisu`
does ([sisu test](#sisu-test)); each benchmark of a module other than
the root is named after its module, `matched/a match`
([G27](../guides/bench.md#3-run-one-modules-benchmarks) runs one).

**Stage 0 differs.** Stage 0 times the interpreter, and has no
`--native`. [17](../../proposal/17-testing.md) also reports the results
in a stable JSON form that CI can follow, which is not begun.

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

<!-- file: examples/reference-sisu-command/greet.sisu -->
```sisu
##! Greetings in Finnish.

## The greeting for `name`.
##
## ```
## assert greeting("Aino") == "Hei, Aino!"
## ```
pub def greeting(name: str) -> String:
    return "Hei, {name}!"

def main():
    print(greeting("Aino"))
```

<!-- output: examples/reference-sisu-command/greet.console -->
```console
$ sisu test greet.sisu
test "greeting (doc example)" ... ok
1 passed, 0 failed, 0 blocked
$ sisu doc --output pages greet.sisu
$ sisu doc --std std.nothing
error[D0113]: `std.nothing` is not a module of the standard library; `sisu doc --std` takes the paths an import names them by, as `std.time`
$ sisu test --std --filter std.time greet.sisu
test "std.time/Ticker (doc example)" ... ok
test "std.time/Timeline (doc example)" ... ok
test "std.time/Instant (doc example)" ... ok
test "std.time/Zone (doc example)" ... ok
test "std.time/days_in_month (doc example)" ... ok
5 passed, 0 failed, 0 blocked
195 not selected
```

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
imports, each kept whether the file uses it or not ([`sisu
lint`](#sisu-lint) names the unused ones), one blank line between items, and
the sugar spelled one way (`T?`, `T ! E`). It has no options of layout
(D-030). A path that is a directory formats every `.sisu` file under it
(D0077 when there is none); without a path, the package's. `--check`
changes nothing and prints, for each file that is not formatted, what
`sisu fmt` would change, with status 1.

<!-- file: examples/reference-sisu-command/messy.sisu -->
```sisu
def main():
  print("Hei, maailma!")
```

<!-- output: examples/reference-sisu-command/messy.console -->
```console
$ sisu fmt --check messy.sisu
`messy.sisu` is not formatted; `sisu fmt` would change it:
@@ line 1 @@
  def main():
-   print("Hei, maailma!")
+     print("Hei, maailma!")
$ sisu fmt messy.sisu
$ sisu fmt --check messy.sisu
```

A file with syntax errors is reported and left as it is (D0075). The
formatter keeps the program: a formatted file that would parse to
another program is left as it is, and is an internal error (D0076, status
70).

**Status.** In stage 0.

## Interactive tools

### sisu repl

- `sisu repl [--echo] [FILE | DIR]`

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
earlier inputs neither print nor touch anything again. Stage 1 checks
and lowers each input's program, as for `sisu run`, and one process of
it serves the whole session, so an input answers in about a twentieth
of a second, and an input takes every form a script takes. Its
diagnostics and its panics name each line by the input it came from: a
line of the input being run as `<input>:LINE`, a line of the file the
session starts from by the file's name and its own line, as
`repl.sisu:3`, and a place among the other inputs kept before it as
`<repl>`. The program stands in the working directory, so an input's
`embed("notes.txt")` and a script's inline metadata name their files
from where the session runs. `:why` asks stage 1 what each input binds,
reads and changes (its own command `parse --bindings`, which `sisu
parse` does not take). `--front-end=rust` runs the session on the
Rust front end instead, which gives the same transcript.

Given this file and these inputs on standard input:

<!-- file: examples/reference-sisu-command/repl.sisu -->
```sisu
# The session's first input: a function to call from the prompt.
def area(width: int, height: int) -> int:
    return width * height
```

<!-- file: examples/reference-sisu-command/repl.stdin -->
```text
let side = 4
area(side, 3)
let wrong: int = "text"
var sizes = [side]
sizes.push(area(2, 5))
sizes
:why sizes
:back 1
sizes
```

`sisu repl repl.sisu` prints the values to standard output, after its
prompts:

<!-- output: examples/reference-sisu-command/repl.repl -->
```text
sisu> sisu> 12
sisu> sisu> sisu> sisu> [4, 10]
sisu> sizes = [4, 10]  <- [side]  (input 4)
  side = 4  <- 4  (input 2)
  then sizes.push(area(2, 5))  (input 5)
sisu> forgot the last inputs (1); the session keeps 5
sisu> [4, 10]
sisu> 
```

The input `let wrong: int = "text"` does not check, so its E0300 goes to
standard error, at `<input>:1:18`, and the session is as it was before
it.

A terminal shows what you type after each prompt, and a session read
from a pipe does not. `--echo` writes each input after its prompt, and
a line of continuation after its `...`, so that the same inputs read as
a terminal session, standard error in its place among them:

<!-- output: examples/reference-sisu-command/repl_echo.console -->
```console
$ sisu repl --echo repl.sisu < repl.stdin
sisu repl: each input runs as the next line of a script; `:back N` forgets the last N inputs, `:why NAME` shows where a value came from, `:quit` ends
sisu> let side = 4
sisu> area(side, 3)
12
sisu> let wrong: int = "text"
error[E0300]: expected `int`, found `str`
  --> <input>:1:18
  |
1 | let wrong: int = "text"
  |                  ^^^^^^
  |            --- expected because of this

sisu> var sizes = [side]
sisu> sizes.push(area(2, 5))
sisu> sizes
[4, 10]
sisu> :why sizes
sizes = [4, 10]  <- [side]  (input 4)
  side = 4  <- 4  (input 2)
  then sizes.push(area(2, 5))  (input 5)
sisu> :back 1
forgot the last inputs (1); the session keeps 5
sisu> sizes
[4, 10]
sisu> 
```

An input that panics is not kept either. The panic's trace names the
loaded file's function at the file's own line, and the input that
called it as `<input>:1`:

<!-- file: examples/reference-sisu-command/repl_panic.sisu -->
```sisu
# A function to call from the prompt, which panics on no people.
def share(total: int, people: int) -> int:
    return total // people
```

<!-- output: examples/reference-sisu-command/repl_panic.console -->
```console
$ sisu repl --echo repl_panic.sisu < repl_panic.stdin
sisu repl: each input runs as the next line of a script; `:back N` forgets the last N inputs, `:why NAME` shows where a value came from, `:quit` ends
sisu> share(90, 3)
30
sisu> share(90, 0)
panic: division by zero
  at share (repl_panic.sisu:3:12)
  at main (<input>:1:1)
the input did not run to its end, so the session is as it was before it
sisu> let guests = [3]
sisu> guests[2]
panic: index out of bounds: the length is 1 but the index is 2
  at main (<input>:1:1)
the input did not run to its end, so the session is as it was before it
sisu> 
```

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

<!-- file: examples/reference-sisu-command/sheet.sisu -->
```sisu
#!/usr/bin/env sisu
# A worksheet: `sisu ws` writes each statement's value after it.
let prices = [12, 5, 30]
let total = sum(prices)
print("total {total}")
total // len(prices)
total.to_f64() / 3.0
```

`sisu ws sheet.sisu` prints:

<!-- output: examples/reference-sisu-command/sheet.ws -->
```text
#!/usr/bin/env sisu
# A worksheet: `sisu ws` writes each statement's value after it.
let prices = [12, 5, 30]  # => [12, 5, 30]
let total = sum(prices)  # => 47
print("total {total}")
# | total 47
total // len(prices)  # => 15
total.to_f64() / 3.0  # => 15.666666666666666
```

**Stage 0 differs.** A worksheet is tooling after 0.1 (Q-48), which the
language server shows as values beside the lines; stage 0 has it now,
on the command line and in the language server. `sisu ws` checks and
lowers the worksheet with stage 1, in one run of it, so it takes every
form `sisu run` takes, `^` in a slice's bound and a generator
expression among them, and shows stage 1's diagnostics as they are. A top-level expression
whose value is an optional value or a result is shown too, `Some(7)` or
`Err(Negative)`, where a program, the same file under `sisu run`
among them, refuses it as unused (E0318);
the language server's worksheets still run on the Rust front end
([Choosing the front end](#choosing-the-front-end)).

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
starts it, as [Set up your editor](../guides/editor.md) shows. It takes
no options (D0009).

**Stage 0 differs.** The proposal has one front end. With stage 1 built
and at `sisu1 0.15.0` or later, stage 1 answers every request but two,
each a query of its own sent to [the server of
stage 1](#the-server-of-stage-1), a hover in about 9 ms: the
diagnostics, hover, go-to-definition, references, completion, document
symbols, selection ranges, rename and its `prepareRename`, code
actions, inlay hints, and the formatting of a document or a range. A
file with a syntax error still answers what its tree allows, such as
its whole items' symbols. Stage 1's completion offers the prelude's
`pub` names only ([R13](modules.md#names-every-module-sees)), and its
references count the names in import lines, where hover and
go-to-definition answer too; from `sisu1 0.16.0` they count the keyword
of a field in a struct's construction, `Sauna(turn=1)`, which a rename
of the field renames. A worksheet's values and `sisu.explainBorrow`
stay with the Rust front end, which answers every question when stage 1
is not built (the server does not build it under an editor) or
`--front-end=rust` is given.

**Status.** In stage 0, with the differences noted. The proposal's
always-on evaluation, structural editing and stepper come after 0.1
([10](../../proposal/10-tooling.md), "Language server").

## Packages

### sisu new

- `sisu new [--lib] NAME`

`sisu new` creates a package in a new directory `NAME`: a manifest,
`sisu.toml`, and a root module, `src/main.sisu` for an application or,
with `--lib`, `src/lib.sisu` for a library with a test. A package is
named in lowercase letters, digits and `_`, starting with a letter, and
not as a module of the standard library (D0072, status 2); a directory
that exists already is D0073 (status 1).

<!-- output: examples/reference-sisu-command/new.console -->
```console
$ sisu new kala
created the package `kala`, with its manifest `sisu.toml` and its root module `src/main.sisu`
$ sisu new --lib kirja
created the package `kirja`, with its manifest `sisu.toml` and its root module `src/lib.sisu`
$ sisu new Kala
error[D0072]: `Kala` cannot name a package: a package is named in lowercase letters, digits and `_`, starting with a letter, and not as a standard-library module
$ sisu new kala
error[D0073]: `kala` exists already
```

**Status.** In stage 0.

## Explaining and auditing

### sisu explain

- `sisu explain CODE | IDENTIFIER`

`sisu explain` prints a diagnostic's entry in the message catalog: its
code, its kind (error, warning, runtime or cli), its identifier and its
message as a reader sees it, each part that the diagnostic fills in
named in braces, as `{name}`. It takes a code or an identifier. A code
that does not exist is D0010, with status 1, and no code at all D0129.

<!-- output: examples/reference-sisu-command/explain.console -->
```console
$ sisu explain E0200
E0200 (error) resolve.undefined-name: cannot find `{name}` in this scope
$ sisu explain resolve.undefined-name
E0200 (error) resolve.undefined-name: cannot find `{name}` in this scope
$ sisu explain W0002
W0002 (warning) resolve.shadow-outer: `{name}` shadows a binding of an enclosing scope
$ sisu explain E0121
E0121 (error) parse.empty-interpolation: an interpolation needs an expression between `{` and `}`
$ sisu explain R0003
R0003 (runtime) panic.overflow: integer overflow: {expr}
$ sisu explain D0006
D0006 (cli) cli.errors-found: the program was not run: errors found ({count})
$ sisu explain E9999
error[D0010]: there is no diagnostic `E9999`
```

**Stage 0 differs.** [10](../../proposal/10-tooling.md) gives each code
a long explanation with examples; stage 0 prints the catalog's line
(Q-66).

**Status.** In stage 0.

### sisu lint

- `sisu lint [--format=ids] [PATH]`

`sisu lint` checks a program as `sisu check` does and prints its lints,
the warnings that `sisu check` leaves out, each with its fix; a program
with errors gets its errors with them, and status 1. It has two lints.
W0015 (`lint.wildcard-own-enum`) is a `case _` over an enum of the same
package, which would hide a variant added later
([R6](patterns.md)); its fix lists the variants the wildcard matches.
W0027 (`lint.unused-import`) is an import whose name the file does not
use ([R13](modules.md#imports)); its fix removes the import, or the
name and its comma from a `from` that keeps others. `sisu fmt` keeps
such an import, so a file can import what it does not use yet. An
import that does not resolve, a name its module lacks (E0204) or a path
that names no module, is reported by its error alone, not as unused
too; the other names of a `from` beside it are still W0027 when the
file does not use them.

<!-- file: examples/reference-sisu-command/lint.sisu -->
```sisu
# A `case _` over an enum of the program's own, which `sisu lint` names.
enum Stove:
    Wood
    Electric
    Smoke

def heat(stove: Stove) -> int:
    return match stove:
        case Wood: 80
        case _: 70

def main():
    print(heat(Stove.Smoke))
```

The second file imports two names it does not use yet:

<!-- file: examples/reference-sisu-command/unused_import.sisu -->
```sisu
# Imports that the file does not use yet, which `sisu lint` names.
import std.json
from std.math import clamp, sqrt

def main():
    print(clamp(1.5, 0.0, 1.0))
```

<!-- output: examples/reference-sisu-command/lint.console -->
```console
$ sisu lint lint.sisu
warning[W0015]: `case _` matches `Electric`, `Smoke` of `Stove`, an enum of this package, so a variant added to `Stove` later is not an error here
  --> lint.sisu:10:14
   |
10 |         case _: 70
   |              ^
  = fix: list them: `Electric | Smoke`

$ sisu lint unused_import.sisu
warning[W0027]: `json` is imported and not used in this file
  --> unused_import.sisu:2:1
  |
2 | import std.json
  | ^^^^^^^^^^^^^^^
  = fix: remove the import

warning[W0027]: `sqrt` is imported and not used in this file
  --> unused_import.sisu:3:29
  |
3 | from std.math import clamp, sqrt
  |                             ^^^^
  = fix: remove the import

$ sisu lint --help
error[D0009]: unknown option `--help` for `sisu lint`
```

`sisu check` prints nothing for either file, and both programs run.
Without a file, `sisu lint` works on the package of the current
directory, D0064 outside one. An option it does not take, `--help` among them, is D0009
with status 2, as for the other commands; `sisu help` lists them.

**Status.** In stage 0 (10), stage 1's alone (its decisions 292 and 588
in the [architecture notes](../../compiler/docs/architecture.md)).

### sisu fix

- `sisu fix [--dry-run] [--all] FILE`

`sisu fix` applies the preferred fix of each diagnostic of FILE, the
first fix the diagnostic prints, and checks the file again, until no
fix applies. Each fix it applies is a line, D0175, with the code it
fixed and what the fix does; then the count (D0177), and the file is
written once. `--dry-run` prints each fix's edit under its line, the
range it replaces and the text, and leaves the file as it is (D0178);
`--all` fixes the lints of [sisu lint](#sisu-lint) too. The status is 0
whether or not something was fixed, so read the dry run first: the
preferred fix is the cheapest to write, not always the one you want.

<!-- file: examples/reference-sisu-command/tally.sisu -->
```sisu
# Three mistakes, each with a fix that `sisu fix` can apply.
def add_turn(mut turns: list[String], name: String):
    turns.push(name)

def main():
    var turns: list[String] = []
    add_turn(turns, "Aino")
    print("{len(turn)} in the sauna")
```

<!-- output: examples/reference-sisu-command/fix.console -->
```console
$ sisu fix --dry-run tally.sisu
tally.sisu:3:16: E0300 fixed: copy it: `.copy()`
  3:20-3:20 ".copy()"
tally.sisu:7:14: E0339 fixed: write `mut` before the argument: `mut `
  7:14-7:14 "mut "
tally.sisu:8:17: E0200 fixed: a similar name is in scope: `turns`
  8:17-8:21 "turns"
fixes that would be applied to `tally.sisu`: 3, and `--dry-run` leaves it as it is
$ sisu fix tally.sisu
tally.sisu:3:16: E0300 fixed: copy it: `.copy()`
tally.sisu:7:14: E0339 fixed: write `mut` before the argument: `mut `
tally.sisu:8:17: E0200 fixed: a similar name is in scope: `turns`
fixes applied to `tally.sisu`: 3
$ sisu run tally.sisu
1 in the sauna
$ sisu fix tally.sisu
nothing in `tally.sisu` has a fix to apply
```

The first fix copies `name` where `own name: String` would move it
in, the second fix E0300 offers; the dry run is where you choose. A
file with nothing to fix is D0176. A diagnostic whose first fix has no
edit is left as it is, and a fix that would undo an earlier round's is
D0179, which stops the rounds. An option `sisu fix` does not take,
`--edition` and `--deps` among them, is D0009 with status 2, and no
file, or more than one, prints the command's usage (D0151).

**Status.** In stage 0, stage 1's alone, from `sisu1 0.16.0` (its
decisions 1040 and 1064 in the [architecture
notes](../../compiler/docs/architecture.md)); the Rust front end
answers it with D0003. 10's `--edition` and `--deps` are designed
([sisu fix --edition and --deps](#sisu-fix---edition-and---deps)).

### sisu explain-borrow

- `sisu explain-borrow FILE:LINE`

`sisu explain-borrow` lists the borrows live on a line: whether each is
shared or `mut`, what it borrows, where it starts, the names that hold
it, and where it is last used. A program with errors is reported
instead. The argument is a file and a line, separated by a colon
(D0018 otherwise).

<!-- file: examples/reference-sisu-command/borrows.sisu -->
```sisu
# The words borrow the text they come from.
def main():
    var text = String.new("hei maailma")
    let words = [w for w in text.split_whitespace()]
    print(words)
    text.push("!")
    print(text)
```

<!-- output: examples/reference-sisu-command/borrows.console -->
```console
$ sisu explain-borrow borrows.sisu:5
borrows live on line 5 of borrows.sisu, in `main`:
  a shared borrow of `text` from 4:29, held by `words`, used until 5:11
  a shared borrow of `words` from 5:11, held by a temporary, used until 5:5
$ sisu explain-borrow borrows.sisu:3
no borrow is live on line 3 of borrows.sisu
```

**Status.** In stage 0 (18, Phase 3), and in the language server as
`sisu.explainBorrow`.

### sisu audit

- `sisu audit --ownership FILE...`
- `sisu audit --caps [FILE...]`
- `sisu audit --unsafe [--std] [PATH]`

`sisu audit --ownership` and `--caps` report, as one line of data per
item, the experiment's counts (18, Phases 3 and 6):

- `--ownership` counts, for each file, its lines and its uses of the
  ownership model's escape hatches: `Shared`, `Weak`, `Cell`, `Arena`,
  `Key`, `.copy()` and `unsafe`. A file need not check to be counted.
- `--caps` reads a program that checks and prints each function that
  does I/O, with the parameters that hold a capability and its fewest
  calls from `main`; each use of a capability's operation; each struct
  that bundles three or more capability types; and a summary. A file it
  names is a program of its own; without a file, it audits the program
  of the current directory's package, each member of a workspace in
  turn, every module of it and what `main` reaches of its dependencies,
  each line starting with `.`, the package's directory, as
  [G24](../guides/audit.md#3-give-it-only-what-it-needs) shows.

`--unsafe` is the reviewer's list of a program's unsafe code: for each
package its level and where the root consents to it, for each module
its `unsafe` blocks, unsafe functions and `extern` blocks with the
start of each `# safety:` reason, and the foreign functions with the
safe functions that call them. The standard library is one line, its
level, and `--std` audits it instead. A one-file program declares no
level ([chapter 27](../tutorial/27-systems.md#packages-that-contain-unsafe-code)
and [G19](../guides/c-library.md#4-test-it-and-run-it-in-the-interpreter-and-natively)
show packages that do).

Without one of the three, `sisu audit` is D0022.

<!-- file: examples/reference-sisu-command/audit.sisu -->
```sisu
# A program for the audits: what it shares, copies and reaches.
def save(fs: Fs, name: str, text: str) -> ! IoError:
    fs.write_text(Path.new(name), text)?
    return Ok(())

def main(sys: System) -> ! IoError:
    let team = Shared(["Aino".to_string(), "Eero".to_string()])
    let names = team.copy()
    save(sys.fs, "team.txt", "{names:?}")?
    return Ok(())
```

<!-- output: examples/reference-sisu-command/audit.console -->
```console
$ sisu audit --ownership audit.sisu borrows.sisu
audit.sisu lines=8 Shared=1 Weak=0 Cell=0 Arena=0 Key=0 copy=1 unsafe=0
borrows.sisu lines=6 Shared=0 Weak=0 Cell=0 Arena=0 Key=0 copy=0 unsafe=0
$ sisu audit --caps audit.sisu
audit.sisu function main caps=1 io=calls depth=0
audit.sisu function save caps=1 io=direct depth=1
audit.sisu use save FsOps.write_text depth=1
audit.sisu summary functions=2 median=1 max=1 uses=1 deepest=1 structs=0
$ sisu audit --unsafe audit.sisu
audit.sisu   no level   a program of one file, which may hold any unsafe code
  no unsafe code
std (toolchain)   unsafe = "extern"   see `sisu audit --unsafe --std`
```

`audit.sisu` has no unsafe code. A program of one file declares no
level, so its `--unsafe` report heads it "no level", a file that may
hold any unsafe code, and lists what it holds, here nothing, and the
standard library's level.

**Status.** In stage 0; `--unsafe` is stage
1's (its decision 425 in the [architecture
notes](../../compiler/docs/architecture.md)). The proposal's `sisu
audit`, which lists the capability summary, holes and advisories as
well, is [designed](#sisu-audits-other-reports).

## Inspecting the front end

### sisu tokens

- `sisu tokens [--trivia] FILE`

`sisu tokens` prints the lexer's tokens, one per line: the kind, the
byte range and the text. `INDENT` and `DEDENT` are the layout's tokens.
`--trivia` includes spaces and comments.

<!-- output: examples/reference-sisu-command/tokens.console -->
```console
$ sisu tokens hello.sisu
DEF_KW 46..49 "def"
IDENT 50..54 "main"
L_PAREN 54..55 "("
R_PAREN 55..56 ")"
COLON 56..57 ":"
NEWLINE 57..58 "\n"
INDENT 62..62 ""
IDENT 62..67 "print"
L_PAREN 67..68 "("
STRING_START 68..69 "\""
STRING_TEXT 69..82 "Hei, maailma!"
STRING_END 82..83 "\""
R_PAREN 83..84 ")"
NEWLINE 84..85 "\n"
DEDENT 85..85 ""
EOF 85..85 ""
```

**Status.** In stage 0, for implementations of Sisu.

### sisu parse

- `sisu parse --print|--tree|--shape|--ast FILE`

`sisu parse` prints the result of the parser. `--print` reprints the
source from the syntax tree, which keeps every byte; `--tree` prints
the tree with byte ranges; `--shape` prints what formatting keeps, the
tree without ranges, trivia and what the formatter changes on purpose;
and `--ast` prints the abstract syntax tree built from it. `--shape`
exits with status 1 for a file with a syntax error.

<!-- output: examples/reference-sisu-command/parse.console -->
```console
$ sisu parse --print hello.sisu
# The program most examples on this page run.
def main():
    print("Hei, maailma!")
$ sisu parse --tree hello.sisu
SOURCE_FILE 0..85
  FN_DEF 46..85
    DEF_KW 46..49 "def"
    IDENT 50..54 "main"
    PARAM_LIST 54..56
      L_PAREN 54..55 "("
      R_PAREN 55..56 ")"
    COLON 56..57 ":"
    BLOCK 57..85
      NEWLINE 57..58 "\n"
      INDENT 62..62 ""
      EXPR_STMT 62..85
        CALL_EXPR 62..84
          NAME_REF 62..67
            IDENT 62..67 "print"
          ARG_LIST 67..84
            L_PAREN 67..68 "("
            ARG 68..83
              STRING_EXPR 68..83
                STRING_START 68..69 "\""
                STRING_TEXT 69..82 "Hei, maailma!"
                STRING_END 82..83 "\""
            R_PAREN 83..84 ")"
        NEWLINE 84..85 "\n"
      DEDENT 85..85 ""
  EOF 85..85 ""
$ sisu parse --shape hello.sisu
SOURCE_FILE
  FN_DEF
    DEF_KW
    IDENT "main"
    PARAM_LIST
      L_PAREN
      R_PAREN
    COLON
    BLOCK
      NEWLINE
      INDENT
      EXPR_STMT
        CALL_EXPR
          NAME_REF
            IDENT "print"
          ARG_LIST
            L_PAREN
            ARG
              STRING "\"Hei, maailma!\""
            R_PAREN
        NEWLINE
      DEDENT
  EOF
```

**Status.** In stage 0, for implementations of Sisu and for the
conformance suite's checks of the formatter.

## Commands that are designed

The proposal designs these commands and options, and stage 0 does not
have them yet. Stage 0 answers each with D0003 or D0009, as this session
shows; when one lands, the session changes, and so does this section.

<!-- output: examples/reference-sisu-command/designed.console -->
```console
$ sisu add json
error[D0003]: unknown command `add`; run `sisu help`
$ sisu remove json
error[D0003]: unknown command `remove`; run `sisu help`
$ sisu update
error[D0003]: unknown command `update`; run `sisu help`
$ sisu publish
error[D0003]: unknown command `publish`; run `sisu help`
$ sisu review main..HEAD
error[D0003]: unknown command `review`; run `sisu help`
$ sisu fix --edition 2030 hello.sisu
error[D0009]: unknown option `--edition` for `sisu fix`
$ sisu why send hello.sisu:3
error[D0003]: unknown command `why`; run `sisu help`
$ sisu fill hello.sisu
error[D0003]: unknown command `fill`; run `sisu help`
$ sisu doc --digest hello.sisu
error[D0009]: unknown option `--digest` for `sisu doc`
$ sisu run --profile hello.sisu
error[D0009]: unknown option `--profile` for `sisu run`
$ sisu test --cover hello.sisu
error[D0009]: unknown option `--cover` for `sisu test`
$ sisu build --target wasm32-wasi hello.sisu
error[D0009]: unknown option `--target` for `sisu build`
$ sisu learn
error[D0003]: unknown command `learn`; run `sisu help`
```

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

`sisu audit` lists the capability summary and the pure label
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

### sisu fix --edition and --deps

`sisu fix --edition YEAR` rewrites a package for a new edition, and
`sisu fix --deps` replaces calls of a dependency's deprecated items with
their `@fix_inline` bodies in the packages that depend on it. [sisu
fix](#sisu-fix) of a file applies the fix of `@fix_inline` in the
file's own program.

**Status.** Designed ([10](../../proposal/10-tooling.md), "Migration";
[13](../../proposal/13-evolution-and-governance.md)); `--deps` leans
(D-067).

### sisu why

`sisu why FACT FILE:LINE` explains an inferred fact (an error set, `Send`
or `Sync`, `pure`, a capability's reach, the choice of an impl, a
compile-time budget), in a human form and a JSON form.

**Status.** Leaning (D-065).

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
  reports what monomorphisation cost, the copies of each package, beside
  the parts of the executable that stage 0's `--explain-size` gives
  ([sisu build](#sisu-build)). Designed
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

The driver's messages have codes D0001 to D0187. These are the ones that
a command line can produce, by what they report:

| Codes | Report |
|---|---|
| D0001, D0013 | A file that cannot be read or written. |
| D0002, [D0003](diagnostics.md#d0003), [D0004](diagnostics.md#d0004), [D0009](diagnostics.md#d0009), [D0023](diagnostics.md#d0023), [D0129](diagnostics.md#d0129), [D0130](diagnostics.md#d0130), [D0131](diagnostics.md#d0131), [D0151](diagnostics.md#d0151) | The usage, a command's usage, an unknown command, a missing argument, an unknown option, a value that is not a number, the version. |
| D0005, D0069 | A program without `main`; a library run as an application. |
| [D0006](diagnostics.md#d0006), [D0021](diagnostics.md#d0021), D0114, [D0132](diagnostics.md#d0132) | A program with errors that did not run, was not documented, or was not compiled. |
| D0007, D0008, D0053, D0060 to D0063, [D0128](diagnostics.md#d0128), [D0135](diagnostics.md#d0135), [D0137](diagnostics.md#d0137) | The lines of `sisu test`: a test's outcome, the counts, the tests not selected, transcripts, what a failing test printed, the program a list is of. |
| [D0010](diagnostics.md#d0010), D0011 | `sisu explain`. |
| D0012, D0076, D0118 | Internal errors (status 70). |
| D0014 to D0020 | `sisu explain-borrow`. |
| D0022 | `sisu audit` without an audit. |
| D0024 to D0027, D0041 to D0050, D0127 | Schedules explored, and the schedulers. |
| [D0028](diagnostics.md#d0028), [D0029](diagnostics.md#d0029) | `--contracts`. |
| D0030 to D0040 | Replay tokens, capability logs and replays. |
| D0051, [D0052](diagnostics.md#d0052) | `--faults`. |
| D0054 to D0056 | Snapshots. |
| D0057 to D0059, [D0136](diagnostics.md#d0136) | `sisu bench`. |
| D0064 to D0067, D0112 | Packages, lockfiles, workspaces and features. |
| D0070 to [D0073](diagnostics.md#d0073) | `sisu build` and `sisu new`. |
| D0139 to D0147, [D0148](diagnostics.md#d0148) | `sisu build --native --explain-size`. |
| D0170 to D0174, D0180 to D0187 | The lines of `sisu check --holes` ([sisu check --holes](#sisu-check---holes)). |
| D0175 to D0179 | The lines of `sisu fix` ([sisu fix](#sisu-fix)). |
| D0074, D0075, D0077 | `sisu fmt`. |
| D0078 | `--watch`. |
| D0079 to D0093 | `sisu repl` and `sisu ws`. |
| D0094 | `--stats`. |
| D0095 to D0102 | The language server's answers. |
| D0103 to [D0111](diagnostics.md#d0111), D0119 to D0126, [D0134](diagnostics.md#d0134) | The native backend, its lowering, the runtime in Sisu, and `sisu test --native`. |
| [D0113](diagnostics.md#d0113) | `sisu doc --std`. |
| [D0115](diagnostics.md#d0115) to D0118 | The front end. |

`sisu explain CODE` prints any of them, and [R17, Diagnostics](diagnostics.md#the-driver)
has an entry for each one that the manual shows, with the fix.
