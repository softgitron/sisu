# Build a native executable

This guide compiles a program into an executable that runs without
`sisu`, checks that it behaves as the interpreter does, builds it for
release, and says where targets and cross compilation stand. It assumes
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
where it runs. It receives the real `System`, so it reads files and the
network with the same capabilities that `main` declares.

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
on, and build for release once the tests pass. `--contracts=LEVEL`
chooses at build time which contracts the executable checks, as it does
for `sisu run`.

## 3. Choose a target

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

**Status.** `--native`, `--release` and `--output` are in stage 0, as
stage 2 of the bootstrap ([18](../../proposal/18-next-steps.md)).
[12](../../proposal/12-interop-and-targets.md) designs the targets,
their tiers and `--target`, which stage 0 does not have, and
[09](../../proposal/09-modules-packages-and-build.md) a content-addressed
cache of builds. Stage 0 compiles a debug build with the backend written
in Sisu and a release build with Cranelift, which stands in for the
LLVM backend that [10](../../proposal/10-tooling.md) plans for release.
