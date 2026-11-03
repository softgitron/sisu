# Stage 2: the backend in Sisu

Step 1 of D-087's order of work ([15](../../proposal/15-decision-log.md),
D-087 and D-089): a lowering, written in Sisu, from SIR to LIR, the
backend-neutral low IR that holds every decision the Rust backend
(`sisu-codegen`) makes between SIR and Cranelift IR, checked by
translating LIR one to one into Cranelift IR inside the Rust backend, so
that an executable built from LIR is byte for byte the one built from the
program's SIR. Step 2, the x86-64 debug backend in Sisu, compiles the same
LIR to machine code in one pass; the LLVM release backend of D-088 lowers
it to LLVM IR.

## Running it

```text
./sisu2 lir FILE.sir [--release] [--output FILE.lir]   # the LIR of a serialised SIR program
./sisu2 sir FILE.sir                                   # the SIR read and written again
sisu build --native --from-lir FILE.lir [--output EXE] # the executable, through Cranelift
sisu run --native --from-lir FILE.lir [ARGS...]        # built and run
```

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
LIR in 0.07 s; the SIR of stage 1's own package (493 018 lines, 2 616
function instances) to 4 026 377 lines in 6.0 s, and in 15 s in the
release profile, whose facts about integers it computes. Built from that
LIR, stage 1 is the same 23 MB executable its SIR gives, in both
profiles.

## The default

`sisu build --native` (and `sisu run --native`) lowers every program
through stage 2 (decisions 304 and 305): the driver writes the SIR to a
file, `compiler/target/stage2/sisu2 lir` lowers it, and the Rust backend
translates the LIR. A construct stage 2 does not lower is the program's
`D0103`; nothing falls back to the Rust lowering, which lowers only when
`--lower=rust` asks for it (the oracle the check below compares with, and
the seed that builds stage 1 and stage 2 themselves).

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
- The translation and the check: all 205 cases of `native.list` pass
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
  --tests`, decision 285) and leaves its tests' tags and its benchmarks
  out, since stage 1's SIR types do not hold them yet. When they do, the
  reader fills them: it makes a `stage1.sir.Program` naming every field.
