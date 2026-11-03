# Stage 2: the backend in Sisu

Step 1 of D-087's order of work ([15](../../proposal/15-decision-log.md),
D-087 and D-089): a lowering, written in Sisu, from SIR to LIR, a
backend-neutral low IR that holds every decision the Rust backend
(`sisu-codegen`) makes between SIR and Cranelift IR, checked by
translating LIR one to one into Cranelift IR inside the Rust backend, so
that an executable built from LIR is byte for byte the one built from the
program's source. Step 2, the x86-64 debug backend in Sisu, compiles the
same LIR to machine code in one pass; the LLVM release backend of D-088
lowers it to LLVM IR.

It is a transcription of `crates/sisu-codegen/src` (`lib.rs`, `func.rs`,
`layout.rs`, `facts.rs`, the part of `debug.rs` that describes types) and
of the runtime's ABI that code shares (`crates/sisu-rt/src/abi.rs`): every
rule has its counterpart here, under the same name where the language
allows. Where the Rust code calls Cranelift's function builder, this
package writes a line of LIR, and the translation in Rust
(`crates/sisu-codegen/src/lir.rs` and `translate.rs`) makes the same call;
so the two paths give the same Cranelift IR, function by function, and the
same object file.

The package depends on stage 1 (`../stage1`, a path dependency) for the
SIR types (`stage1.sir`); stage 1 writes the serialised form but has no
reader of it, so `read.sisu` here is a transcription of
`crates/sisu-sir/src/read.rs`.

## Running it

```text
./sisu2 lir FILE.sir [--release] [--output FILE.lir]   # the LIR of a serialised SIR program
sisu build --native --from-lir FILE.lir [--output EXE] # the executable, through Cranelift
```

`sisu2` builds the package natively on first use, and after a source
under `src/` changed, into `compiler/target/stage2/sisu2`, as stage 1's
`sisu1` builds stage 1. `lir` prints the LIR on standard output, or
writes it to `--output`; `--release` lowers for the release profile
(decision 232: the facts of decision 270 and the scalars of decision 246
in variables). A construct the lowering does not handle yet stops it with
`lir: not lowered: WHAT (in FUNCTION)` on standard error and status 1, as
`D0103` stops the Rust backend.

The check is the conformance runner's mode `--native-from-lir`
(`compiler/conformance/README.md`, "The native mode"): every case of
`conformance/native.list` lowered by stage 1 to SIR, by stage 2 to LIR,
translated and run, compared with the case's expectations, and its
executable compared byte for byte with the one `sisu build --native`
builds from the case's source.

## LIR

LIR is the program as the backend's single pass sees it: functions with
their frames laid out (every SIR local in a slot of the frame, its size
decided), basic blocks of typed machine operations, calls by address
that return the status byte (decision 189), the entry probe, the count of
nested calls and the yield points (decisions 190, 203, 204 and 234),
drop glue, dispatchers and thunks as ordinary functions, the runtime's
entry points by their symbols, read-only data with its relocations, a
source location for every instruction, and what the debug information
needs beyond the code: each function's name, file and line, its named
locals with their slots, and a description of their types. What it does
not hold is what only machine code knows: where each slot lies in the
frame, which register holds a value, the encoding, the line table's
addresses and the call frame information, which the backend makes from
the code it writes.

### The text form

One construct per line; blank lines and lines starting with `#` are
ignored, and leading spaces are not significant. The lowering writes it
deterministically: the same SIR gives the same bytes.

```text
lir 1
profile debug                                   # or `release`
source 0 "hello.sisu"                           # the program's files, as SIR numbers them
source 1 "<std>/prelude.sisu"
data d0 import SISU_RT_DEPTH                    # a data symbol of the runtime
data d4 bytes "main\x00"                         # read-only bytes
data d9 align 8 u32 12 2 0 5 3 3                # 32-bit words, little-endian
data d40 align 8 zeros 16 func 0 sisu$P#drop$7 func 8 sisu$L#drop$9   # words holding function addresses
declare sisu$main$1 local sisu 0                # a function of Sisu's convention with 0 arguments
declare sisu_rt_alloc import (i64) -> (i64)     # a runtime entry point
declare main export (i32, i64) -> (i32)
function sisu$main$1 probed name "main" source 0 line 1
    slot s0 24                                  # the frame, in the order the slots were made
    vars 2                                      # variables x0 and x1, each 64 bits
    local "x" t0 s0 line 2                      # a named local, for the debugger
block b0
    v1 = get_stack_pointer
    ...
    brif v5, b5, b6
block b5 cold
    ...
end
types
type t0 base "int" signed 8
...
stats 1 33 0 0 0 0 0
```

The items between the sources and `types` come in the order the lowering
made them, which is the order of the object file: a symbol is declared
before anything refers to it, and the functions are defined in this order.

- `data dN import NAME` declares the runtime's data symbol `NAME`; `data
  dN [align A] bytes "..."` defines read-only bytes (the quoted text is
  bytes: printable ASCII as itself, `\\`, `\"` and `\xHH` for any other
  byte); `u32 W...` defines 32-bit words; `zeros N` defines `N` zero bytes
  in which each `func OFFSET SYMBOL` is a word holding a function's
  address. Data is numbered `d0`, `d1`, ... in the order it is declared,
  imports included.
- `declare SYMBOL LINKAGE SIGNATURE` declares a function: `local` (this
  program's), `import` (the runtime's) or `export` (C's `main`). The
  signature `sisu N` is Sisu's convention: `N + 1` 64-bit addresses (the
  result's, then each argument's) and the 8-bit status, 0 or 1 when a
  panic is leaving (decision 189); otherwise `(T, ...) -> (T, ...)`, its
  parameters' and results' types.
- `function SYMBOL FRAME name "SHOWN" [source S line L]` defines a declared
  function, up to `end`. `FRAME` is how its frame counts toward the probe
  reserve of a task's stack (decisions 203 and 234): `probed` (its entry
  probes the stack: a function compiled from SIR), `unprobed` (a
  dispatcher or a thunk, whose whole frame may lie below the limit) or
  `outside` (it never runs on a task's stack: C's `main`, a stub). `SHOWN`
  is the name the debugger shows, `source` and `line` where it is
  declared. Its first lines are the frame: `slot sN SIZE` for each slot in
  order (`SIZE` bytes, at least 8, aligned to a word), `vars N`, the
  number of its variables (`x0`, `x1`, ...; each 64 bits), and `local
  "NAME" tK sN [indirect] [param] [line L]` for each named local (its
  type, its slot, `indirect` when the slot holds the address of the value,
  as a parameter's does, `param` for a parameter). Then its blocks.
- `block bN [cold]` starts block `N`, which runs until its terminator;
  `cold` blocks are those only a panic, a failed check or unwinding
  reaches, which the backend places after the rest (decision 270). Blocks
  are numbered in the order the lowering made them, which need not be the
  order they are written in; the first written is the entry, whose
  parameters are the function's (`v0` to `vP-1`, the result's address
  first).
- `types` starts the debug types: `type tN ...`, one per Sisu type a named
  local has (below). `stats` ends the file: the function instances
  compiled, the SIR statements and terminators, and the release profile's
  index checks, those proven, overflow checks, those proven, and tests
  decided (decision 270), which `SISU_NATIVE_TIMES` reports.

### Operations

Values are `vN`, numbered in the order the function defines them (its
parameters first); each is defined once and used where its definition
dominates. Types are `i8`, `i32`, `i64`, `f32` and `f64`. An operation
that defines values starts with them (`v7 = ...`, `v8, v9 = ...`).

| Operation | Meaning |
|---|---|
| `iconst.T N` | the constant `N` (a float's constant is its bits, as an `i64`) |
| `iadd`, `isub`, `imul`, `sdiv`, `srem`, `ishl`, `sshr`, `band`, `bor`, `bxor`, `band_not` `a, b` | integer arithmetic, wrapping; `sdiv` and `srem` signed, truncating |
| `iadd_imm`, `imul_imm`, `bxor_imm`, `bor_imm` `a, N` | the same with a constant |
| `ineg a`, `bnot a` | negation and complement |
| `icmp.CC a, b`, `icmp_imm.CC a, N` | an integer comparison giving an `i8`, 1 or 0; `CC` is `eq ne slt sle sgt sge ult ule ugt uge` |
| `fcmp.CC a, b` | a float comparison; `CC` is `eq ne lt le gt ge` |
| `sadd_overflow`, `ssub_overflow`, `smul_overflow` `a, b` | the wrapped result and whether it overflowed, as two values |
| `select c, a, b` | `a` when `c` is not 0, else `b` |
| `uextend.T`, `ireduce.T`, `bitcast.T`, `fpromote.T`, `fdemote.T` `a` | conversions to `T` |
| `fadd`, `fsub`, `fmul`, `fdiv` `a, b`; `fneg a` | float arithmetic |
| `load.T a, OFF`; `uload8 a, OFF`, `sload8 a, OFF` | a load from address `a + OFF` (the 8-bit ones extended to 64 bits) |
| `store v, a, OFF` | a store to `a + OFF` |
| `stack_addr sN`, `stack_load sN`; `stack_store v, sN` | a slot's address, its first word read or written |
| `symbol_value dN`, `func_addr SYMBOL` | the address of data or of a function |
| `get_stack_pointer` | the stack pointer |
| `call SYMBOL(a, ...)` | a call; its results are the callee's |
| `call_indirect N, f(a, ...)` | a call of the code at `f`, of the signature `sisu N` |
| `memcpy d, s, n` | copies `n` bytes from `s` to `d` (the C library's) |
| `use xN`; `def xN, v` | a variable's current value; a new value for it |
| `param bN` | adds a 64-bit parameter to block `N`, where its jumps give it |
| `loc LINE COL` | the source location of what follows, until the next |
| `probe_reserve` | the probe reserve of a task's stack, an `i64` the backend computes from the frames it laid out (decision 234) |
| `jump bN(a, ...)` | to block `N`, with its parameters' values (the parentheses only when it has some) |
| `brif c, bA(...), bB(...)` | to `A` when `c` is not 0, else to `B` |
| `switch v, bD, [K bA, ...] made M` | to the block of the case `v` equals (each `K` a 64-bit pattern written signed), else to `D`; `made M` says the function had made `M` blocks when it wrote the switch, after which a backend that numbers its blocks as it makes them makes those it needs for the switch |
| `return a, ...` | returns its operands |
| `trap N` | stops the program (the runtime has reported why) |

Every load and store is of an aligned address that cannot fault. A
function's references to other functions and data are the backend's to
make: the Cranelift translation makes one per symbol, at its first use.

### Debug types

A named local's type is described once, `type tN KIND ...`, in the order
the debug information first needs it (the functions in order, their
locals in order, each type before the types it holds, as the DWARF writer
walks them); the description says what the writer of decision 227 needs,
in the conventions gdb's Rust support reads:

```text
type t0 base "int" signed 8              # a base type, shared by its name: signed, unsigned, float, boolean, utf, uchar
type t1 str                              # `&str`: the address of the bytes and their length
type t2 struct "Point" 16 { "x" t0 @0, "y" t0 @8 }
type t3 tuple "(int, str)" 24 { 0 t0 @0, 1 t1 @8 }
type t4 pointer "ref Point" t2           # a reference or a box
type t5 text "String" 24                 # `String`, `Path`, `bytes`: their text as `&str`, and the capacity
type t6 list "list[int]" 24 "&[int]" t0  # its items as a slice of the element's type, and the capacity
type t7 enum "Option[int]" 16 @0 { } { "Some" { "__0" t0 @8 }, "None" { } }
type t8 enum "IoError" 40 @16 { "set" @0, "trace" @8 } { "NotFound" { "path" t5 @24 } }
type t9 errset "{A, B}" 48 { 17 "A" t10, 18 "B" t11 }   # by the number of the error's type
type t12 alias t8                        # a union of one error set: that set's description
type t13 opaque "dyn Shape" 16           # words the debugger shows as words
type t14 word "Scope"                    # one word of the runtime's
```

An enum's description gives the offset of its tag, its leading words (an
error's type number and return trace) and its variants with their fields;
a struct's and a tuple's fields are those whose offsets the layout gives.

## Status

What is done, and what is not, is kept here as the work goes.

- Task 1, LIR and its documentation: this file and
  `docs/architecture.md` ("LIR"; decisions 271 onward).
- Task 2, the lowering: in progress.
- Task 3, the translation and the check: in progress.
