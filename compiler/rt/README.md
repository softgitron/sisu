# The runtime in Sisu

The runtime of natively compiled Sisu is being written in Sisu, piece by
piece, as D-086 orders ([15](../../proposal/15-decision-log.md)); this
package holds what is written so far, and `crates/sisu-rt`, the runtime
in Rust, the rest. `docs/architecture.md` ("Stage 2", "The runtime in
Sisu"; decisions 335, 336 and 350 to 353) describes it, and
`docs/port-debt.md` ("The runtime in Sisu") lists what is left.

## What it is

Each `export_NAME` function of the root module (`src/lib.sisu`) is the
runtime's symbol `sisu_rt_NAME`, of Sisu's convention (the result's
address, then each argument's, and the status byte): the generated code
and the runtime in Rust call it as any function is called. What the
exports call is compiled with them under local symbols. The code is the
runtime's own: it counts no frame, passes no yield point and adds no line
to a stack trace, as drop glue does not; it reaches raw memory through the
operations of std.mem the backend compiles inline, and the runtime in Rust
and C's `memmove` and `memcmp` through foreign calls (`unsafe =
"extern"`).

| Module | What |
|---|---|
| `lib.sisu` | the exports, and `==` of two texts' bytes |
| `types.sisu` | the program's type table, as the runtime in Rust lays it out in memory when the program starts (`crates/sisu-rt/src/desc.rs`) |
| `words.sisu` | the reads and writes of words and bytes of values |
| `host.sisu` | the runtime in Rust's functions this code calls, and C's |
| `value.sisu` | equality, order, the structural hash, `.copy()`, the drop and the count of a copy's handles, each a walk of a value by its type's parts |
| `text.sisu` | text appended to a `String`, an integer's digits, a `String` made from a text |
| `search.sisu` | text search by Two-Way, and a prefix or a suffix |

## How it is built

`compiler/target/rt/rt.o`, which every native program is linked with
before the runtime in Rust, is compiled from the package's seed,
`rt.sir`, the SIR stage 1 prints of it, by the Rust lowering in the
release profile and Cranelift (`sisu_codegen::compile_library`), when it
is missing or older than the seed: the runtime must exist before stage 1,
a native program itself, can compile it. A test holds the seed to what
`sisu sir` prints here. After a change to `src/`:

```text
cd compiler/rt
../target/release/sisu check          # the package's diagnostics, which should be none
../target/release/sisu sir > rt.sir   # the seed again
```

Stage 2 compiles the same seed with the backend written in Sisu (`sisu2
build rt.sir --library --output rt.o`), and its fixed point
(`compiler/stage2/bootstrap.sh`) links the builds it compares with the
objects so made, so that the runtime in Sisu is in the fixed point too.
`SISU_RT=PATH` makes a native build link with the object at `PATH` as it
is.
