# Stage 2: the backend in Sisu

Step 1 of D-087's order of work ([15](../../proposal/15-decision-log.md),
D-087 and D-089): a lowering, written in Sisu, from SIR to LIR, the
backend-neutral low IR that holds every decision the Rust backend
(`sisu-codegen`) makes between SIR and Cranelift IR, checked by
translating LIR one to one into Cranelift IR inside the Rust backend, so
that an executable built from LIR is byte for byte the one built from the
program's SIR. Step 2, the x86-64 debug backend in Sisu, compiles the same
LIR to machine code in one pass (`sisu2 x86`, below), the debug profile's
backend since its fixed point holds; the LLVM release backend of D-088
lowers it to LLVM IR.

## Running it

```text
./sisu2 lir FILE.sir [--release] [--output FILE.lir]   # the LIR of a serialised SIR program
./sisu2 sir FILE.sir                                   # the SIR read and written again
./sisu2 x86 FILE.lir --output FILE.o                   # the LIR compiled by the Sisu backend
./sisu2 build FILE.sir --output FILE.o [--release]     # lowered and compiled in one process
./sisu2 build rt.sir --library --output rt.o           # the runtime in Sisu, compiled as a library
sisu build --native [FILE]                             # lowered by stage 2, compiled by the Sisu backend
sisu build --native --backend=cranelift [FILE]         # lowered by stage 2, translated for Cranelift
sisu build --native --from-lir FILE.lir [--output EXE] # the executable of a LIR program
sisu run --native --from-lir FILE.lir [ARGS...]        # built and run
./bootstrap.sh [--keep DIR] [--sisu PATH]              # the Sisu backend's fixed point
```

`--library` (with `lir` and `build`) lowers the runtime in Sisu
(`compiler/rt`) as a library rather than a program (decision 353): each
`export_NAME` function of its root module the global symbol
`sisu_rt_NAME`, what they call local, and no C `main` or type table, as
`sisu_codegen::compile_library` does.

`sisu2` builds the package natively on first use, and after a source
under `src/` (or stage 1's `sir` module) changed, into
`compiler/target/stage2/sisu2`, as stage 1's `sisu1` builds stage 1:
`sisu build --native` through stage 1, the front end of `sisu`, in about
21 s. `lir` prints the LIR on standard output, or writes it to
`--output`; `--release` lowers for the release profile (decision 232: the
scalars of decision 246 in variables, and the checks the facts of
decision 270 prove left out). A construct the lowering does not
handle stops it with `lir: not lowered: WHAT (in FUNCTION)` on standard
error and status 1, as `D0103` stops the Rust backend. `sir` reads a
program and writes it again with stage 1's writer, which gives the same
bytes back: the reader's check.

## How it is made

The package depends on stage 1 (`../stage1`, a path dependency) for the
SIR types (`stage1.sir`) and their writer. Stage 1 writes the serialised
form but has no reader of it, so `read.sisu` here is a transcription of
`crates/sisu-sir/src/read.rs`. The lowering is a transcription of
`crates/sisu-codegen/src`, rule for rule, under the Rust names:

| Module | Transcribes |
|---|---|
| `read.sisu` | `sisu-sir/src/read.rs`: the serialised SIR read back |
| `abi.sisu` | the part of `sisu-rt/src/abi.rs` the code generator shares: the runtime's symbols and words, the numbers of its operations and intrinsics, the type table and its encoding |
| `lir.sisu` | the function builder: one LIR line per call of Cranelift's |
| `layout.sisu` | `layout.rs`: sizes, type numbers and entries, fields, what needs a drop |
| `gen.sisu` | `lib.rs`: instances, the runtime's entry points, texts and descriptors, trait resolution, dispatchers, thunks, C's `main`, what `main` reaches, and the LIR file |
| `func.sisu` | `func.rs`: a function's frame, entry, probe, yield points, failure paths, blocks, terminators and statements |
| `rvalue.sisu` | `func.rs`: places, operands and rvalues |
| `call.sisu` | `func.rs`: drops, calls and the inline intrinsics |
| `scan.sisu` | `func.rs`: the analyses over SIR (cold blocks, register locals, iterators in the frame) |
| `facts.sisu` | `facts.rs`: what the release profile knows about a function's integers (decision 270) |
| `debug.sisu` | the part of `debug.rs` that describes types |

The backend written in Sisu (D-087, step 2; decisions 306 to 312) is not
a transcription; it compiles LIR to x86-64 in one pass over each function
after one analysis pass, as TPDE does:

| Module | What it is |
|---|---|
| `lirread.sisu` | LIR read back item by item (as `sisu-codegen/src/lir.rs` reads it), each value given its type |
| `x64.sisu` | the encodings of the instructions the backend uses |
| `x86.sisu` | the analysis, the frame, the register allocator, and the code of each operation |
| `dwarf.sisu` | the line table (`.debug_line`, with the unit that names it) |
| `elf.sisu` | the ELF relocatable object: sections, symbols, relocations |
| `object.sisu` | the pass over a program: each datum placed as it is read, the functions compiled in batches on `std.par`'s workers and placed in order |

The program's state and one function's are one struct, `gen.Cg`, whose
methods are spread over the modules, as `lib.rs` and `func.rs` share
`Codegen` and `Cx`; where the Rust calls Cranelift's function builder, this
package writes a line of LIR (`self.b`), and the translation in Rust
(`crates/sisu-codegen/src/lir.rs` and `translate.rs`) makes the same call.

## LIR

LIR's text form, its operations and its descriptions of the debug
information's types are documented in `docs/architecture.md` ("LIR"),
with the decisions behind them (281 to 284). In short: a program is its
sources, then its items in the order of the object file (`data`,
`declare`, `function`), then the descriptions of its types (`types`) and
its counts (`stats`); a function is its frame (`slot`, `vars`, `local`)
and its blocks, each line one call of the function builder.

## How fast

The SIR of `examples/wordcount` (5 065 lines) lowers to 39 108 lines of
LIR in 0.07 s; the SIR of stage 1's own package (521 266 lines, after its
diagnostics round) to 4 263 811 lines in 7.1 to 8.5 s on a loaded
machine, and in 17.8 to 21.1 s in the release profile, whose facts about
integers it computes. Built from that LIR, stage 1 is the same executable
its SIR gives, in both profiles. The Sisu backend compiled that LIR in
15 to 19 s on one thread; on `std.par`'s workers (decisions 332 to 334)
it takes 9.4 s from the LIR file on four cores of a quiet machine (18.3 s
on one worker), to a 39 MB object with its line table, the same bytes
either way, and `sisu2 build` lowers stage 1 in 6.7 s and compiles it in
6.0 s, where Cranelift compiles the LIR in 3.7 s. A whole native build of
stage 1 takes 82.5 s with the Sisu backend and 77.6 s with Cranelift,
stage 1's front end most of both.

## The default

`sisu build --native` (and `sisu run --native`) lowers every program
through stage 2 (decisions 304 and 305): the driver writes the SIR to a
file; a debug build's is lowered and compiled by the Sisu backend in one
process (`compiler/target/stage2/sisu2 build`, decisions 331 and 334), a
release build's lowered by `sisu2 lir` and its LIR translated for
Cranelift by the Rust backend. A construct
stage 2 does not lower is the program's `D0103`; nothing falls back to the
Rust lowering, which lowers only when `--lower=rust` asks for it (the
oracle the check below compares with, and the seed that builds stage 1
and stage 2 themselves). `SISU_STAGE2=PATH` makes native builds use that
stage-2 executable as it is instead of `compiler/target/stage2/sisu2`
(decision 330).

## The Sisu backend

`sisu2 x86 FILE.lir --output FILE.o` writes an ELF object that `cc` links
with the runtime Cranelift's objects are linked with, and `sisu build
--native` does both after stage 2's lowering, unless `--backend=cranelift`
(or `SISU_BACKEND=cranelift`) or `--release` asks for Cranelift (decision
331). Every function has a frame pointer and every frame slot a
fixed offset: the callee-saved registers the allocator uses, LIR's slots,
its variables, and a home for each value that lives past its block or is
spilled. A value used only in the block that defines it stays in one of
`rbx`, `r12` to `r15` (which a call preserves) until its last use, the
furthest one spilled when they run out; one used elsewhere, and every
block's parameter, lives in its home. Calls are System V's, of which
Sisu's convention is an instance. The conformance runner's mode `--native
conformance/native.list` builds every case so and compares its runs with
the interpreter's expectations, and `--native-cranelift` builds them with
Cranelift, its oracle, and compares the same way.

## The fixed point

`./bootstrap.sh` checks D-087's criterion for this backend (decision
330): sisu2-C, stage 2 as the Cranelift path builds it, lowers and
compiles the SIR of stage 2 and of stage 1 with the Sisu backend into
sisu2-A and stage1-A, and the runtime in Sisu's (`compiler/rt`, its seed)
as a library into rt-A.o, which sisu2-A and stage1-A are linked with;
stage1-A prints the three packages' SIR again, and sisu2-A lowers and
compiles it into rt-B.o, sisu2-B and stage1-B, the executables linked with
rt-B.o (decision 353). It holds: each B is its A byte for byte, and
stage1-A prints the SIR `sisu sir` prints, the runtime's seed among it.

```text
  sisu2-C       23536552 bytes  sha256 4ebfa64db1cd453d...  built through the Cranelift path (65.2 s)
  A.stage2.sir   4204001 bytes  sha256 df41a8a5447a9444...  printed by sisu sir
  A.stage1.sir  18015195 bytes  sha256 ddc542c20f268407...  printed by sisu sir
  A.rt.sir        428692 bytes  sha256 4c985bfeab3977d8...  printed by sisu sir (all three 87.4 s)
  rt-A.o          799152 bytes  sha256 b42a1b24b1059549...  built by sisu2-C
  sisu2-A       27655024 bytes  sha256 c803f28b9d79495c...  built by sisu2-C, with rt-A.o (5.8 s)
  stage1-A      54716320 bytes  sha256 32da331b9ba38535...  built by sisu2-C, with rt-A.o (26.5 s)
  B.stage2.sir   4204001 bytes  sha256 df41a8a5447a9444...  printed by stage1-A
  B.stage1.sir  18015195 bytes  sha256 ddc542c20f268407...  printed by stage1-A
  B.rt.sir        428692 bytes  sha256 4c985bfeab3977d8...  printed by stage1-A (all three 224.2 s)
  rt-B.o          799152 bytes  sha256 b42a1b24b1059549...  built by sisu2-A
  sisu2-B       27655024 bytes  sha256 c803f28b9d79495c...  built by sisu2-A, with rt-B.o (13.3 s)
  stage1-B      54716320 bytes  sha256 32da331b9ba38535...  built by sisu2-A, with rt-B.o (53.7 s)
```

It takes six to eight minutes on four cores; `--keep DIR` leaves the files
in `DIR`, `--sisu PATH` drives the builds with another `sisu` (CI's
`bootstrap` job uses the debug one).

## The check

The conformance runner's mode `--native-from-lir`
(`compiler/conformance/README.md`, "The native mode") lowers every case of
`conformance/native.list` with stage 2, builds it from LIR, runs it
against the case's expectations, and compares its executable byte for
byte with the one `sisu build --native --from-sir` builds from the same
SIR, and the release profile's executable (`sisu2 lir --release`) with
the one `--release --from-sir` builds; `cargo test -p sisu-driver --test
native` and CI's `native` job run it.

## Status

What is done, and what is not, is kept here as the work goes.

- LIR: defined and documented (`docs/architecture.md`, "LIR";
  decisions 281 to 284).
- The lowering: both profiles, the release one with its facts about
  integers (`facts.sisu`, transcribing `facts.rs`; decision 270).
- The translation and the check: all 216 cases of `native.list` pass
  `--native-from-lir`, their executables byte for byte the ones from SIR
  in both profiles, the five unsafe ones included.
- Unsafe code (decisions 302 and 305): the intrinsics of std.mem (277 to
  305), `ptr` and `ptr_mut` with their `Ptr` entry, the refusal of a
  pointer to a place whose two layouts differ, foreign calls (`call.sisu`,
  `call_foreign` and `c_argument`) and the machine intrinsics
  (`machine_intrinsic`).
- The default: `sisu build --native` lowers through stage 2 alone, and the
  runner's `--native`, `--native-from-sir` and `--native-release` modes
  fail a case whose build does not say stage 2 lowered it.
- The reader takes the SIR of a program built with its tests (`sisu sir
  --tests`, decision 285), its tests' tags and its benchmarks included.
- The Sisu backend (decisions 306 to 312): every operation of LIR but a
  jump with more than nine arguments and a return of more than one value;
  all 216 cases of `native.list` run from its objects with the
  interpreter's output (`--native`), as they do from Cranelift's
  (`--native-cranelift`); stage 1 compiled by it checks the conformance
  suite's files as the Cranelift build does. What it leaves is in
  `docs/port-debt.md` ("The backend in Sisu").
- Its fixed point holds (decision 330, `./bootstrap.sh`), and it is the
  debug profile's backend, Cranelift the release profile's and the
  oracle (decision 331).
- The runtime in Sisu (decisions 335, 336, 350 to 353, 358, 383 and 385):
  raw memory of a word or a byte compiled inline, and `compiler/rt`,
  compiled once into the object every native program is linked with,
  holding `==` of texts' bytes, the structural operations by a value's
  parts, text building and integers' digits, text search, a float's text,
  `Display` and `Debug` by the type table, and lists over raw memory,
  whose Rust counterparts are gone; stage 2's library mode compiles it
  with the Sisu backend, and the fixed point links its builds with the
  objects so made.
- What native code leaked or refused (decisions 381 and 382): the text an
  interpolation is building when a panic or a `?` in one of its values
  leaves the function is freed on that path (`building_texts` in
  `scan.sisu`, the failure paths of `func.sisu`), and an `own` closure
  that keeps a function value has its environment in a heap cell, which a
  copy of the value copies and its drop frees (`closure` in
  `rvalue.sisu`), both as the Rust lowering does, byte for byte.
- On threads (decisions 332 to 334): the runtime's words the code reaches
  are thread-local (`tls_value`), std.par's `map` runs its bodies on the
  runtime's workers, the backend compiles each batch of functions through
  it, and `sisu2 build` lowers and compiles in one process, each giving
  the bytes one thread gives.
