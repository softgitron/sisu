# Targets and runtime profiles

This page states where a Sisu program runs: the targets and their
tiers, the facts of the target that a program can read, cross
compilation, the runtime profiles that decide what the runtime gives a
program (`full`, `single`, `freestanding` and `sim`), how tasks wait for
I/O and use their stacks, the build profiles, Sisu's ABI and the C ABI
that stands in for a stable one, other languages, and the size and
startup of an executable. The commands that build and run an
executable are [R16](sisu-command.md#compiling-and-running-natively---native-and---release).

**See also:** the tutorial's [chapter 22](../tutorial/22-web-service.md)
(a native build), [chapter 25](../tutorial/25-compile-time.md#code-for-one-target-or-for-the-tests)
(`target`) and [chapter 27](../tutorial/27-systems.md) (systems
programming); the guide [Build a native executable](../guides/native.md);
the proposal's [12, Interop and
targets](../../proposal/12-interop-and-targets.md), [06,
"Colourless I/O"](../../proposal/06-concurrency-and-capabilities.md#colourless-io)
and [17, "Simulation and replay"](../../proposal/17-testing.md#simulation-and-replay);
the compiler's [architecture notes](../../compiler/docs/architecture.md),
"Stage 2: the native backend", "The runtime" and "Tasks";
[R9](concurrency.md#the-scheduler) for the scheduler.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Sisu's own ABI is unstable and the C ABI is the stable one
(D-039, Q-31); the targets, the profiles and their tiers are designed
in [12](../../proposal/12-interop-and-targets.md).

## Where a program runs

A program runs in one of two ways, from the same SIR, the program that
the front end writes:

- `sisu run` interprets it, which is how tests, replays and the
  ownership monitor run it too;
- `sisu run --native` and `sisu build --native` compile it to machine
  code, an executable that needs neither `sisu` nor the source.

The two give the same output, the same panics and the same exit status,
and run tasks under the same seeded schedule
([R9](concurrency.md#the-scheduler)). A debug build is compiled by the
backend written in Sisu, a release build by Cranelift with its
optimisations ([Build profiles](#build-profiles)). `--release` goes with
`--native` only (D0111, [R17](diagnostics.md#d0111)), and a construct
that the native backend does not compile yet is D0103
(`native.unsupported`), whose program still runs without `--native`.

**Stage 0 differs.** The release backend of
[10](../../proposal/10-tooling.md) is LLVM (D-087, D-088); Cranelift
stands in for it.

**Status.** In stage 0 (stage 2 of the bootstrap, D-087), with the
difference noted.

## Targets and tiers

| Target | Tier in version 1 | Runtime profile |
|---|---|---|
| Linux, macOS and Windows on x86-64 and AArch64 | tier 1 | `full` |
| RISC-V 64 Linux | tier 2 | `full` |
| `wasm32-wasi` (WASI preview 2, the component model) | tier 1 | `single` |
| `wasm32-browser` (bindings to the JavaScript host) | tier 2 | `single` |
| Bare-metal ARM Cortex-M, RISC-V 32, x86-64 (kernels) | tier 2 | `freestanding` |
| iOS, Android | tier 2, through the C ABI and the platforms' toolchains | `full` |

**Stage 0 differs.** A native build is for the machine that makes it,
and the backend written in Sisu compiles for x86-64 Linux only.

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md#targets)).

## The facts of the target

`target` is a module that every program sees, of constants that
describe what the program is compiled for: `target.os`, `target.arch`
and `target.runtime`. They are known while the program compiles, so
`if comptime target.os == "windows":` chooses code for a system, and
both branches are still checked, so a branch for a system you do not
build for keeps compiling ([R12](compile-time.md#the-facts-of-the-target)). The
manifest's `when` conditions read the same facts
([R13](modules.md#target-conditions)).

<!-- file: examples/reference-targets/facts.sisu -->
```sisu
# The facts of the build target, known while the program compiles.
def separator() -> str:
    return if comptime target.os == "windows": "\\" else: "/"

def main():
    print("{target.os} {target.arch} {target.runtime}")
    print("home{separator()}aino")
```

<!-- output: examples/reference-targets/facts.console -->
```console
$ sisu run facts.sisu
linux x86_64 interpreter
home/aino
$ sisu run --native facts.sisu
linux x86_64 interpreter
home/aino
```

**Stage 0 differs.** `target.runtime` is `interpreter` in a native
build too, and no fact names the runtime profile.

**Status.** In stage 0 ([08](../../proposal/08-metaprogramming.md#conditional-compilation)),
with the difference noted.

## Cross compilation

`sisu build --target TARGET` compiles for another target from any host,
with the C headers and libraries of each target bundled with the
compiler, as Zig does, so a cross build works on a fresh machine and
needs no second toolchain
([09](../../proposal/09-modules-packages-and-build.md)).

**Stage 0 differs.** `--target` is D0009 (`cli.unknown-option`,
[R17](diagnostics.md#d0009)):

<!-- output: examples/reference-targets/cross.console -->
```console
$ sisu build --native --target aarch64-linux facts.sisu
error[D0009]: unknown option `--target` for `sisu build`
```

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md#targets)).

## Runtime profiles

The runtime is what makes tasks and capabilities work, and it is what
embedded and kernel code cannot afford, so it comes in profiles, chosen
in the manifest:

| Profile | The runtime gives |
|---|---|
| `full` | tasks scheduled M:N over threads of the operating system, a global allocator, unwinding on a panic, and capabilities backed by the operating system; the default |
| `single` | one thread with an event loop, on which tasks are coroutines, and capabilities backed by WASI or the JavaScript host; for WebAssembly and for programs that want a deterministic schedule |
| `freestanding` | no scheduler, no global allocator and no unwinding (`panic = abort` or a hook of the program's): `task.scope` does not compile, collections take an explicit allocator, and the program makes its own `System` by implementing the capability traits over its hardware |
| `sim` | a `System` that is simulated whole, for `sisu test --sim`: the test clock, an in-process network, a file system that loses unsynced writes in a crash, and a seeded `Rand` |

The capability traits are the seam between a program and its profile:
`Fs`, `Net`, `Clock` and the rest have the operating system's
implementations in `full` and `single`, simulated ones in `sim`, and the
program's own in `freestanding` ([R10](capabilities.md#the-root-capabilities)).
A `sim` build does not compile if code it reaches could escape the
simulation, which only an `extern` call with no simulation model can
(E0971, `sim.unmodelled-extern`); the simulation itself is
[R15](testing.md#simulation).

**Stage 0 differs.** No profile can be chosen: the manifest has no key
for it, so `runtime = "single"` is W0005 (`package.unknown-key`), and
the interpreter and the native runtime both run every task on one
thread, one at a time, under the seeded scheduler. `sim` is the profile
of `sisu test --sim` only:

<!-- file: examples/reference-targets/profiles.pkg/sisu.toml -->
```toml
[package]
name = "stove"
version = "0.1.0"
edition = "2027"
runtime = "single"

[profile.release]
contracts = "preconditions"
panic = "abort"

[profile.release-size]
contracts = "off"
```

<!-- file: examples/reference-targets/profiles.pkg/src/main.sisu -->
```sisu
# A package that asks for a runtime profile and a way to panic.
def main():
    print("the stove runs on {target.runtime}")
```

<!-- output: examples/reference-targets/profiles.console -->
```console
$ sisu check
warning[W0005]: `runtime` is not a key of `[package]` that stage 0 knows; it is ignored
  --> sisu.toml:5:1
  |
5 | runtime = "single"
  | ^^^^^^^

warning[W0005]: `panic` is not a key of `[profile.release]` that stage 0 knows; it is ignored
  --> sisu.toml:9:1
  |
9 | panic = "abort"
  | ^^^^^

warning[W0005]: `contracts` is not a key of `[profile.release-size]` that stage 0 knows; it is ignored
  --> sisu.toml:12:1
   |
12 | contracts = "off"
   | ^^^^^^^^^

```

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md#runtime-profiles));
the `sim` profile is a prototype of the experiment (D-070).

## Tasks, I/O and stacks

- A call that waits for I/O, such as a read of a file or a request,
  blocks its task, never the thread: the runtime parks the task, runs
  others, and resumes it when the I/O completes. A function has no
  `async` colour, and the same library serves programs with and
  without tasks (D-014, [06](../../proposal/06-concurrency-and-capabilities.md#colourless-io)).
- A foreign call that blocks blocks its thread, so the runtime starts
  another; `@blocking` on a foreign function says that its call may
  block ([R14](attributes.md#foreign-functions-that-block)).
- A task starts with a small stack, reserved in virtual memory and
  committed as it is used, and the compiler computes the bound of a call
  tree without recursion, so that most tasks get an exact stack. Deep
  recursion that exhausts a task's stack panics with R0032
  (`panic.stack-overflow`), which names the function; whether stacks grow
  is open (Q-6). The bodies of `std.par` run on the stacks of its
  workers ([R9](concurrency.md#parallel-computation-over-data)).

**Stage 0 differs.** Tasks run one at a time on one thread, so a
blocking foreign call stops every task. A natively compiled task's
stack is 1 MiB, unless the environment variable `SISU_TASK_STACK` of the
run gives another size, and no stack bound is computed.

**Status.** Designed (D-014, Q-6 open); stacks that panic on overflow
are in stage 0.

## Build profiles

A build profile says how a program is compiled; it is not a runtime
profile ([09](../../proposal/09-modules-packages-and-build.md)):

| Profile | Compiles |
|---|---|
| `debug` | fast, with every check, by Sisu's own backend |
| `release` | optimised, by LLVM, with the checks of overflow and bounds and the contracts that the manifest names |
| `release-size` | for a small executable, for embedded targets and WebAssembly |

No profile turns off the checks of overflow or bounds: safety that can
be turned off in a release build is turned off in release builds. A
profile's `contracts` key sets which contracts it checks
([R13](modules.md#profilerelease)), and `panic = "abort"` makes a panic
abort the process instead of unwinding
([R8](errors.md#aborts)).

**Stage 0 differs.** `--release` compiles with Cranelift's
optimisations and leaves out the checks it proves unnecessary;
`release-size` and the `panic` key are W0005, as
[Runtime profiles](#runtime-profiles) shows.

**Status.** `debug` and `release` in stage 0, with the differences
noted; `release-size` and `panic = "abort"` designed.

## The ABI

Sisu's own ABI, the layout of its values and how its functions are
called, is not stable from one version of the compiler to the next, so
that layout, calling conventions and monomorphisation stay free to
improve (D-039; a stable ABI for dynamic libraries is Q-31, decided:
unstable for now). The C ABI is the stable interface: `@repr(c)` gives a
type C's layout, and `@export` makes a function callable from C
([R14](attributes.md#exported-functions)).

**Stage 0 differs.** A native build gives an exported function no
symbol under its C name ([R14](attributes.md#exported-functions)).

**Status.** Decided (D-039, Q-31).

## Other languages

- Python calls Sisu through the C ABI, with the glue of a CPython
  extension from `std.x.py`, so that Sisu can be the fast layer under
  Python.
- JavaScript and the Web reach Sisu as WebAssembly, with bindings made
  for its exported functions and capabilities over the Web's APIs.
- The JVM and .NET reach it through the C ABI, which is not first-class.
- There is no direct interop with C++, which Sisu reaches through
  `extern "C"` wrappers (Q-24, decided).

**Stage 0 differs.** None of these exists yet.

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md#other-languages)).

## Binary size and startup

- An executable is linked statically, one file to deploy.
- Code that nothing reaches is left out, item by item, which the absence
  of reflection at run time keeps possible.
- A program does nothing at startup beyond making its `System`: there
  are no global constructors, a `const` is computed while compiling,
  and a value made the first time it is used is an explicit `Lazy[T]`.

**Stage 0 differs.** An executable links dynamically to the C library,
`libm` and `libgcc_s`, keeps its debug information, and leaves no code
out: a ten-line program is about 18 MB in either profile. `Lazy[T]` is
E0201 (`resolve.undefined-type`).

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md#binary-size-and-startup)).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [D0009](diagnostics.md#d0009) | `cli.unknown-option` | [Cross compilation](#cross-compilation) |
| [W0005](diagnostics.md#w0005), E0971 | `package.unknown-key`, `sim.unmodelled-extern` | [Runtime profiles](#runtime-profiles) |
| R0032 | `panic.stack-overflow` | [Tasks, I/O and stacks](#tasks-io-and-stacks) |
| [E0201](diagnostics.md#e0201) | `resolve.undefined-type` | [Binary size and startup](#binary-size-and-startup) |
| [D0111](diagnostics.md#d0111), D0103 | `native.release`, `native.unsupported` | [Where a program runs](#where-a-program-runs) |
