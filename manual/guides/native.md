# Build a native executable

This guide compiles a program into an executable that runs without
`sisu`, checks that it behaves as the interpreter does, builds it for
release, runs its tests compiled, and says where targets and cross compilation stand. It assumes
a program with a `main`, from chapter 1 of the tutorial, and packages,
from chapter 9.

**See also:** [R16](../reference/sisu-command.md#compiling-and-running-natively---native-and---release)
for `--native`, `--release` and `--output`, and for
[`sisu build`](../reference/sisu-command.md#sisu-build); the tutorial's
[chapter 22](../tutorial/22-web-service.md#a-native-build) (a service
built natively); the proposal's
[12, Interop and targets](../../proposal/12-interop-and-targets.md).

## 1. Build the executable

`sisu build --native FILE` checks the program, compiles it with the
native backend and writes the executable beside the file, named by its
stem. This program counts down to *löyly*, the steam of water thrown on
a sauna's stones:

<!-- file: examples/guide-native/countdown.sisu -->
```sisu
# Counts down from the number it is given, or from 3.
def main(sys: System):
    let args = sys.env.args()
    var n = 3
    if len(args) > 0:
        n = args[0].parse_int() ?? 3
    while n > 0:
        print("{n}...")
        n -= 1
    print("löyly!")
```

<!-- output: examples/guide-native/native.console -->
```console
$ sisu build --native countdown.sisu
built `countdown`
$ sisu run --native countdown.sisu 2
2...
1...
löyly!
$ sisu run countdown.sisu 2
2...
1...
löyly!
```

- After the build, `./countdown 2` runs the executable, with its
  arguments after its name.
- `sisu run --native FILE ARGS` builds the executable and runs it, as
  the second command does; the third runs the same program in the
  interpreter. They must print the same: `sisu-conformance --native`
  builds the compiler's conformance cases natively and compares each
  executable with what the interpreter prints.
- In a package, `sisu build --native` writes `build/debug/NAME` beside
  the lockfile, `NAME` being the package's, and `--output PATH` writes
  the executable at `PATH` instead.

The executable needs no `sisu` and no source: copy the one file to
where it runs, with the C library and `libgcc_s`, which it links
dynamically, on that machine. It receives the real `System`, so it reads
files and the network with the same capabilities that `main` declares.
The link leaves out the code that nothing reaches from `main`, so a
ten-line program is 6.25 MB, nearly all of it the runtime's code and
its debug information, and 1.74 MB once `strip` takes the debug
information out ([R22](../reference/targets.md#binary-size-and-startup)).

## 2. Build for release

`--release` builds with the release profile, which leaves out the
checks that the compiler proves unnecessary; a package's executable then
goes to `build/release/NAME`. `--release` goes with `--native` only,
since the interpreter has no profiles:

<!-- output: examples/guide-native/release.console -->
```console
$ sisu build --native --release --output countdown-fast countdown.sisu
built `countdown-fast`
$ sisu run --native --release countdown.sisu
3...
2...
1...
löyly!
```

Test the debug build and the interpreter first, where the checks are
on, and build for release once the tests pass. The debug build checks
the raw memory of unsafe code as the interpreter does, with the same
panics, and the release build does not
([chapter 27](../tutorial/27-systems.md#raw-memory)). `--contracts=LEVEL`
chooses at build time which contracts the executable checks, as it does
for `sisu run`.

## 3. Run the tests natively

`sisu test --native` compiles a program's tests with the native backend
into one executable and runs each test in a process of its own, so the
tests check the code that ships rather than the interpreter's run of
it. Add `--release` to test the release build:

<!-- file: examples/guide-native/steps.sisu -->
```sisu
# A countdown's lines, with tests that run compiled as well.

## The lines of a countdown from `n` to *löyly*.
def steps(n: int) -> list[String]:
    var lines: list[String] = []
    for k in range(n, 0, -1):
        lines.push("{k}...")
    lines.push("löyly!".to_string())
    return lines

test "a countdown from 2":
    assert steps(2) == ["2...", "1...", "löyly!"]

test "no count at all":
    assert len(steps(0)) == 1
```

<!-- output: examples/guide-native/tests.console -->
```console
$ sisu test steps.sisu
test "a countdown from 2" ... ok
test "no count at all" ... ok
2 passed, 0 failed, 0 blocked
$ sisu test --native steps.sisu
test "a countdown from 2" ... ok
test "no count at all" ... ok
2 passed, 0 failed, 0 blocked
$ sisu test --native --release steps.sisu
test "a countdown from 2" ... ok
test "no count at all" ... ok
2 passed, 0 failed, 0 blocked
```

- The report, the replay tokens and `--explore N` are those of `sisu
  test`; the test cache is not used, so every test runs each time.
- A release build of the tests still checks every contract, as `sisu
  test` always does.
- A failed `expect` records its failure and the test goes on, as in
  the interpreter.
- The schedulers, the `sim` profile's faults (`--sim --faults P`) and
  `sim.always` run natively too, with the interpreter's choices, so a
  seed or a schedule found one way runs the same the other. Here PCT
  and the exhaustive search check that two tasks never put more than
  five logs on the stove:

<!-- file: examples/guide-native/turns.sisu -->
```sisu
# Two tasks add their logs to one stove; the total holds in every order.
import std.sim

def add_logs(stove: Mutex[int], n: int):
    for _ in range(n):
        var g = stove.lock()
        g.value += 1

test "every log reaches the stove":
    let stove = Mutex(0)
    sim.always("never more than five", () -> stove.lock().value <= 5) do:
        _ = task.scope do s:
            s.spawn(() -> add_logs(stove, 2))
            s.spawn(() -> add_logs(stove, 3))
            return Ok(())
    assert stove.lock().value == 5
```

<!-- output: examples/guide-native/schedules.console -->
```console
$ sisu test --native --scheduler pct --explore 20 turns.sisu
test "every log reaches the stove" ... ok
1 passed, 0 failed, 0 blocked
explored 20 schedules of a PCT scheduler of depth 3 per test (seeds 0 to 19)
$ sisu test --native --scheduler exhaustive turns.sisu
test "every log reaches the stove" ... ok
1 passed, 0 failed, 0 blocked
searched 127 schedules (preemption bound 2)
```

- What only the interpreter does is refused with D0134 (a test
  parameter that no fake stands for); run those without `--native`.
  `sisu run --native` takes the schedulers too, `--scheduler pct` and
  `--scheduler exhaustive` with `--depth`, and reports a failing seed or
  schedule as `sisu run` does
  ([R16](../reference/sisu-command.md#choosing-the-schedule)).

## 4. Choose a target

An executable is built for the machine that builds it. Cross
compilation, `sisu build --target TARGET` for another system or
processor, is designed and not in stage 0:

<!-- output: examples/guide-native/target.console -->
```console
$ sisu build --native --target aarch64-linux countdown.sisu
error[D0009]: unknown option `--target` for `sisu build`
```

Until it lands, build on a machine of the target's kind, such as a
build server of each.

**Status.** `--native`, `--release`, `--output` and `sisu test
--native` are in stage 0, as
stage 2 of the bootstrap ([18](../../proposal/18-next-steps.md)).
[12](../../proposal/12-interop-and-targets.md) designs the targets,
their tiers and `--target`, which stage 0 does not have, and
[09](../../proposal/09-modules-packages-and-build.md) a content-addressed
cache of builds. Stage 0 compiles a debug build with the backend written
in Sisu and a release build with Cranelift, which stands in for the
LLVM backend that [10](../../proposal/10-tooling.md) plans for release.
