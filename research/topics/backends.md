# Compiler backends

## The question

Which code generator should compile Sisu once the toolchain is to be
written in Sisu alone, and in what order should it be built? The front end
already is Sisu: stage 1 (`compiler/stage1`, about 66,000 lines) gives the
Rust front end's output byte for byte and reaches the bootstrap's fixed
point (D-078, D-079). The backend is not: `compiler/crates/sisu-codegen` is
6,542 lines of Rust over Cranelift 0.123, gimli and object, and
`compiler/crates/sisu-rt` is a runtime of 12,340 lines of Rust that uses
corosensei for the context switch and libc for its memory calls (counted
in this repository, November 2026). The owner accepts a release backend
that depends on a compiled library such as LLVM through its C API, as Rust
and Swift depend on LLVM, if the evidence supports it; the lean to test is
a debug backend written in Sisu (D-038; [10](../../proposal/10-tooling.md),
"A non-LLVM backend for debug builds"; [09](../../proposal/09-modules-packages-and-build.md),
"Profiles"). Research conducted in November 2026; sources accessed on
2 November 2026 unless dated otherwise.

What Sisu brings to the question:

- **Targets** ([12](../../proposal/12-interop-and-targets.md)): tier 1 is
  Linux, macOS and Windows on x86-64 and AArch64, and `wasm32-wasi`; tier 2
  is RISC-V 64 Linux, `wasm32-browser`, bare-metal Cortex-M, RISC-V 32 and
  x86-64 kernels, iOS and Android. Cross-compilation from any host to any
  target is built in.
- **Compile speed** ([10](../../proposal/10-tooling.md)): a clean debug
  build of 100 kLOC in under 10 s on a laptop, a clean release build in
  under 60 s, a debug rebuild and relink after a one-function edit in under
  1 s.
- **Debugging** (10): DWARF with full types, code that single-steps
  predictably, symbolised traces, `sisu run --profile`, hot function
  replacement through a per-definition indirection point (D-053) and frames
  that stay reifiable.
- **Foreign calls** ([12](../../proposal/12-interop-and-targets.md),
  [22](../../proposal/22-unsafe.md)): calling a C function is an `unsafe`
  operation allowed only in a package whose manifest declares
  `unsafe = "extern"` and which the root lists under `[trust]`; there is no
  inline assembly, and the runtime's three machine intrinsics (`rt.switch`,
  `rt.stack_pointer`, `rt.thread_slot`) are lowered by the backend. A
  library backend reached through its C API lives in such a package; a
  backend run as a separate program needs no foreign call at all.
- **What stage 2 already fixed** ([architecture.md](../../compiler/docs/architecture.md),
  "Stage 2"): unwinding is SIR's cleanup blocks run as ordinary control
  flow, every function returning a status byte, so there is no unwinder, no
  landing pad and no personality routine; tasks run on pooled stacks kept in
  bounds by a probe at every function entry, with yield points at entries
  and back edges (the Phase 1 verdict, decisions 203 to 206); every part of
  a value is a word (decision 188); there is no tracing collector, so no
  stack maps. Each makes a backend simpler than a C++ or Go backend, and
  each is an ABI or stack rule the backend must be free to emit.
- **What stage 2 measured** (architecture.md, "Compile time and run time";
  [port-debt.md](../../compiler/docs/port-debt.md), "Stage 2"): stage 1 and
  the standard library, about 75,000 lines, build natively in 5.9 to 6.4 s
  on 4 cores (about 0.08 s per thousand lines), of which code generation is
  about 2 s and the link 0.08 s; code generation, bounded by Cranelift's
  compiling, is about 8 s of the build's 18 s of processor time. The release profile (Cranelift's `speed`
  level) builds stage 1 in 4.5 s instead of 4.0 s and runs it as fast,
  because half of stage 1's instructions are in the runtime's intrinsic
  dispatch (decisions 232, 233); arithmetic loops gain 1.3 to 2.9 times once
  scalars live in registers (decision 246). Stage 1 compiled natively prints
  the SIR of its own package in about 30 s, where stage 0 (Rust, compiled by
  LLVM) takes 4 to 7 s ([stage1/README.md](../../compiler/stage1/README.md),
  "Where the time goes").

## Approaches found

| Approach | Examples | Buys | Costs | Evidence |
|---|---|---|---|---|
| LLVM, through its C API, its C++ API or bitcode | Rust, Swift, Zig (release builds), Odin, Crystal, Julia, GHC `-fllvm` | The best code of any candidate, every target of 12, full DWARF and CodeView, unwind tables for every platform, thousands of contributors | Slow even at `-O0`; a 118 to 137 MiB library; C API functions removed in every release from 15 to 21; a major release every six months; C++ | [9][10][12][16][57] |
| Cranelift | Wasmtime, `rustc_codegen_cranelift`, Sisu's stage 2 | Fast compiles, code about 14% behind LLVM on Wasm benchmarks, unwind information, a verified instruction selector in progress | Rust only with no C API; a breaking release every month; 64-bit targets only; no Wasm output; no DWARF of its own | [22][23][24][29] |
| QBE | Hare, cproc | 16,000 lines of C, 63 to 70% of `gcc -O2` on CoreMark, MIT | amd64, arm64 and rv64 only; line tables only; no unwind tables; assembly text for an external assembler; one principal author | [33][34][35] |
| libgccjit and GCC | `rustc_codegen_gcc` | GCC's code and its many architectures; a C API with a stable ABI | GPLv3 makes the toolchain GPL; one GCC build per target; no Wasm; a patched fork needed | [36][37][38] |
| MIR | Makarov's `c2m` C compiler and JIT | Compiles about 100 times faster than `gcc -O2` with code at 0.91 to 0.96 of it on small benchmarks; MIT; 23,000 lines | A JIT: no object files, no debug information; one author | [39] |
| C as the intermediate | Nim, V, Vala, Chicken, Zig's C backend (its bootstrap) | Any target with a C compiler; a portable bootstrap seed | A C compiler's time in every build; the debugger sees C; C's undefined behaviour to avoid | [7][40][41][42] |
| A backend written in the language | Go, Zig (x86-64, AArch64, Wasm), Roc's dev backend, DMD, TCC, Plan 9's compilers; TPDE as the research point | Compile speed, freedom over the ABI and the stack, no dependency | Work per target and per object format; slower code; the debug information and the linker are the project's own | [1][2][3][4][32][43][44][45][46] |
| WebAssembly as a target or as the intermediate | Zig's and Go's Wasm backends; Binaryen under Emscripten, Kotlin/Wasm, AssemblyScript; Wasmtime | One portable format; Binaryen's optimiser; Cranelift through Wasmtime's C API | Not native: 45 to 55% slower in browsers; threads at phase 4 and stack switching at phase 3; a runtime inside every program | [52][53][54] |
| Tiered: own debug backend, LLVM for release | Zig, Rust with Cranelift, D (DMD against LDC and GDC), GHC (NCG against LLVM), Jai | Fast debug builds and fast release code | Two backends whose bugs differ | [2][20][29][55] |

## What the evidence says

1. **LLVM is the reference for code quality and targets.** Every
   comparison below is stated against it: Cranelift's code is about 14%
   slower than WAVM's (an LLVM-based Wasm engine) [22][31], QBE reaches 63%
   of `gcc -O2` on CoreMark and 70% with key routines inlined by hand [34],
   TPDE's single-pass code is 1.54 times (x86-64) and 1.77 times (AArch64)
   slower than LLVM `-O1` [32], and Zig's own x86-64 backend "is also
   currently known to emit slower machine code than the LLVM backend" [2].
   The Ubuntu build of LLVM 18 on this machine registers 44 targets,
   among them every one of 12's: x86-64, AArch64, `wasm32`, RISC-V 32 and
   64, and Thumb for Cortex-M [57]. It writes DWARF 5, CodeView and every
   platform's unwind tables. Sisu's ABI rules are plain IR: the status
   return and the cleanup blocks need no landing pads, the yield point is a
   counter, and the probe compares the stack pointer with the task's limit;
   the probe reserve needs each function's frame size, which LLVM can report
   (`-stack-size-section`).

2. **LLVM's compile time is why the debug backends exist.** Zig's
   self-hosted x86-64 backend took `zig build-exe hello.zig` from 918 ms to
   275 ms and a debug build of the Zig compiler from 75 s to 20 s when it
   became the default in June 2025 [1]; the 0.15.1 notes call it "around a
   5x decrease compared to LLVM in most cases" [2]. In 2017 Odin's author
   measured LLVM at "85%+ of the total compilation time; even for
   non-optimized code" [15]. On LLVM IR from SPEC CPU 2017, TPDE compiles
   8 to 24 times faster than LLVM `-O0` (geomean 16.2 and 18.7 times on
   optimised IR) and 80 to 86 times faster than `-O1`, so LLVM's `-O1` back
   end costs about five times its `-O0` [32]. GHC's users' guide names "a
   significant increase in compilation times" as the price of `-fllvm` [20].
   Julia, which runs LLVM when a program first calls a method, cut GLMakie's
   time to first execution from 64.62 s (1.7) to 1.66 s (1.9), mainly by
   caching native code [19] ([`julia.md`](../languages/julia.md)).

3. **The C API breaks; bitcode does not.** LLVM's policy calls the C API "a
   'best effort' for stability", limited "by the abstractness of the
   interface and the stability of the C++ API that it wraps", so that
   "things like 'create debug info' or 'create this type of instruction'
   are likely to be less stable than 'take this IR file and JIT it for my
   current machine'". It also says that "the current LLVM version supports
   loading any bitcode since version 3.0" while "the textual format is not
   backwards compatible" [9]. The release notes bear the first rule out:
   15 deprecated ten functions for opaque pointers and 16 removed
   them; 17 removed the legacy pass manager's C API
   (`LLVMPassManagerBuilderRef` and the pass-adding functions); 18 removed
   sixteen constant-expression functions; 19 deleted the four DIBuilder
   functions that inserted debug intrinsics, replacing them with debug-record
   versions; 20 and 21 removed more [10]. Crystal, which binds the C API from
   a language that is not C++, called its opaque-pointer migration "a rather
   huge effort" (October 2022 to March 2023) [12], and could drop its C++
   shim only when "LLVM 18 now provides everything we need in the upstream C
   API" [13]. Odin calls the C API from C++ and supports LLVM 17 to 22 [14].
   Rust binds 251 LLVM-C functions and 131 shims of its own, written in
   about 4,100 lines of C++ (`compiler/rustc_llvm/llvm-wrapper`) [16]. The
   policy's warning that "debug metadata is ... dropped during upgrades" is
   narrower in the code: the reader strips debug information only when the
   module's "Debug Info Version" differs from its own, which has been 3 since
   LLVM 3.7 (2015), or when the verifier rejects it [11]. Zig moved from the
   IRBuilder API to writing bitcode itself (issue 13265, closed by PR 19031 in
   February 2024), treating the bitcode format as the interface, which
   shrinks the API surface and makes builds without LLVM possible; a first
   measurement showed a 1.16 times wall-time speedup, which Kelley called
   unfair because the old path did redundant work [5]; its writer is 19,518 lines (`Builder.zig`, `ir.zig`,
   `bitcode_writer.zig`) [8]. Roc's rewrite reuses that MIT-licensed writer
   because "changing to use bitcode generation instead of LLVM's APIs
   requires a complete rewrite of LLVM code gen" [21]. GHC passes textual IR
   to `opt` and `llc` as separate programs and pins a range of LLVM versions
   (13 up to 21) [20]. LLVM 13 added the new pass manager to the C API
   (`LLVMRunPasses`), so a bitcode file can be optimised and emitted through
   a handful of coarse entry points of the stable kind [10].

4. **The dependency is large, and Go refused it.** `libLLVM` is 123.7 MB
   (LLVM 17), 123.2 MB (18) and 143.5 MB (20) on this machine [57]. A Zig
   executable without LLVM is 4.4 MiB, with it 169 MiB [5]; issue 16270 (June
   2023, still open, milestone "urgent") would shrink the compiler from about
   150 MiB to 5 MiB, make it "trivial to build from source and to bootstrap
   with only a C compiler", and leave `.bc` files to "Clang ... installed
   separately" [6]. Zig tracks "over 60" upstream LLVM bugs [2], and its 0.16
   and 0.17 roadmaps keep "transitioning from a library dependency on LLVM to
   a process dependency on Clang" [3][4]. The Go FAQ: "we considered using
   LLVM for gc but decided it was too large and slow to meet our performance
   goals. More important in retrospect, starting with LLVM would have made it
   harder to introduce some of the ABI and related changes, such as stack
   management, that Go requires" [45]. Sisu's tasks, probes, yield points,
   hot replacement and reifiable frames are exactly such changes. Zig is
   taking Clang out of its header import as well: `translate-c` now runs on
   Aro, a C front end written in Zig [56]; 12's header import needs a C
   front end in Sisu for the same reason, whatever the backend. LLVM's
   licence, Apache-2.0 with LLVM exceptions, lets compiled programs carry
   embedded LLVM code without attribution and is compatible with Sisu's
   Apache-2.0 OR Unlicense; a toolchain that ships LLVM ships its licence and
   notices [9].

5. **How the LLVM languages use it** [12][13][14][16][17][18][20][5][21]:

   | Language | Interface | LLVM | Note |
   |---|---|---|---|
   | Rust | LLVM-C plus 131 C++ shims | its own fork (branch `rustc/23.1-2026-07-22`); external LLVM 22 or newer | Cranelift and GCC as optional backends |
   | Swift | the C++ API, Clang as the C importer | `swiftlang/llvm-project`, a fork | |
   | Zig | bitcode it writes itself, compiled by a linked LLVM | upstream, 20.1.8 in 0.15, 22.1.8 in 0.17 | own backends for debug builds |
   | Odin | LLVM-C, called from C++ | upstream, 17 to 22 | a custom backend attempted since 2017 |
   | Crystal | LLVM-C through its own `lib` bindings; C++ shim gone from LLVM 18 | upstream | the closest model for Sisu's `extern "C"` |
   | Julia | the C++ API, the ORC JIT, its own passes | its own fork (`julia-23.1.4-1`) | compile at run time; native code caching |
   | GHC | textual IR to `opt` and `llc` processes | upstream, 13 up to 21 | the native code generator is the default |
   | Roc (rewrite) | bitcode through Zig's writer | | the dev backend becomes an interpreter |

   Swift is a C++ user of LLVM, not a C API one. The languages here that
   reach LLVM with no C++ of their own are Crystal (since LLVM 18) and GHC,
   which runs it as separate programs; Zig still keeps a 550-line C++
   wrapper (`src/zig_llvm.cpp`) for running passes and emitting objects,
   which issue 16270 would remove [6][8].

6. **Zig's move away from LLVM for debug builds is real and incomplete
   after five years.** The x86-64 backend passed 1,984 of 2,008 behaviour
   tests against LLVM's 1,977 in 0.15.1 and has "superior debug
   information, and inferior machine code quality" in 0.16 [2][3]. It is not
   the default on Windows "because more COFF linker work needs to be done"
   (still the 0.17 roadmap, October 2026) [1][4]. The AArch64 backend passed
   79% of the behaviour tests in July 2025 and 84% in 0.15.1 [1][2],
   "currently crashes when running the behavior tests" in 0.16 [3], and is
   "blocked on Linker enhancements" in 0.17 [4]. The Wasm backend passes all
   of LLVM's behaviour tests but is not the debug default "due to lack of
   debug info support" [4]. Parallel code generation took the compiler's own
   build from 13.8 s to 10.0 s [2]. The hard parts were linkers, object
   formats and debug information, not instruction selection; the x86-64
   instruction selector alone is 191,905 lines with 5,681 pattern tables
   [8] ([`zig.md`](../languages/zig.md)).

7. **Cranelift is fast and good, but cannot be called from Sisu.** Its
   README: four backends (x86-64, AArch64, s390x, RISC-V 64) with System V
   and Windows x64 conventions and Apple AArch64; code "~2% slower than that
   of V8 (TurboFan), and ~14% slower than WAVM (LLVM)", compiling "an order
   of magnitude faster than WAVM"; "Cranelift's APIs are not yet considered
   stable"; it needs the latest stable Rust [22]. regalloc2 made it about 20%
   faster to compile and up to 10 to 20% faster in code [25]. A breaking
   version ships every month, 0.116 in January 2025 to 0.137 in October 2026;
   Sisu's 0.123 is fourteen behind [23]. The Rust closure Sisu builds is
   about 170,000 lines of Rust and 42,800 of ISLE in fifteen crates, with
   regalloc2's 12,700, gimli's 49,600 and object's 55,100 [24]. For Rust
   itself the gain is smaller than 10 says: Cargo's debug build took 49.93 s
   against 54.64 s (2020) [27]; Cranelift's own build 29.6 s against 37.5 s
   wall and 125 against 211 processor seconds (2024) [28]; in February 2026,
   Zed's incremental builds 5% faster without debug information and 12% with
   line tables, and single crates of rustc's suite 10 to 15% on average [29].
   rustc's front end is most of a build. Unwinding is "experimental and not
   supported on Windows and macOS" and Windows on AArch64 is unsupported in
   `rustc_codegen_cranelift` [26]. To keep it, a Sisu toolchain would keep a
   Rust crate exporting a C ABI over `cranelift-frontend`,
   `cranelift-module` and `cranelift-object` (the unofficial `craneliftc` last
   released 0.5.1 in July 2023 [30]), cargo and rustc to build it on every
   host, and a monthly upgrade. Wasmtime's official C API compiles Wasm with
   Cranelift, but what it makes runs only inside Wasmtime, so it is not a
   native backend.

8. **QBE is the small optimising backend, and it stops short of Sisu's
   needs.** "QBE is a compiler backend that aims to provide 70% of the
   performance of industrial optimizing compilers in 10% of the code" [33];
   1.3 (June 2026) is 16,104 lines of C in the core, does sparse conditional
   constant propagation, GVN and GCM, "registerization of small stack slots"
   and a "linear register allocator with hinting", and adds an experimental
   Windows x64 ABI [33][34]. Its targets are amd64 (System V, Apple, and
   Windows experimentally), arm64 (Linux and Apple) and rv64; there is no
   32-bit or Wasm target. Its debug support is `dbgfile` and `dbgloc` (1.2),
   which print `.file` and `.loc` for the assembler; it writes no unwind
   tables, and LWN calls the lack of "native source-level debugging-information
   generation" a "major unresolved limitation" [33][34]. Hare uses it and
   therefore supports exactly x86_64, aarch64 and riscv64, and no proprietary
   system upstream [35]. QBE is MIT-licensed with 38 contributors, one "by far
   the most prolific" [34]. No published measurement of its compile speed was
   found. At 16,000 lines it is the one optimising backend small enough to
   transcribe into Sisu, as stage 1 transcribed the front end.

9. **libgccjit has the best C API and the wrong licence.** It is "an API
   for embedding GCC inside programs and libraries" with a pure C header,
   ahead-of-time output and a compatibility policy LLVM lacks: "API
   compatibility is achieved by extending the API rather than changing it",
   with symbol versioning (ABI tags 0 to 38) [36]. But "it links to
   `libgccjit` which is under the GPLv3+ and thus, the resulting toolchain
   (rustc + GCC codegen) will need to be released under the GPL license"
   (programs it compiles need not be) [37]; that rules out offering the
   toolchain under the Unlicense, and `rustc_codegen_gcc` needs a patched
   GCC [37]. GCC has no WebAssembly target [38], each target is a separate
   GCC build, and its `cc1` is 32.2 MB [39]. Its use to Rust is "platforms
   unsupported by LLVM" [37], which Sisu's target list does not contain. The
   other way into GCC is a front end inside GCC's tree, as gccrs is for Rust:
   C++ under the GPL at GCC's release pace, and gccrs's June 2026 report
   still names compiling Rust's `core` as its milestone [58].

10. **MIR is fast in both senses and is not an ahead-of-time backend.** On
    a C sieve it compiles in 249 microseconds against `gcc -O2`'s 27.1 ms and
    runs in 1.74 s against 1.6 s; over 15 small benchmarks its code is 0.96
    (mean) and 0.91 (geomean) of `gcc -O2`; it is 23,400 lines and a 557 KB
    binary, MIT-licensed, on x86-64, AArch64, ppc64le, s390x and RISC-V 64
    Linux and x86-64 and AArch64 macOS [39]. It is a JIT: the README names no
    object-file output and no debug information, and it has one author [39].

11. **A C backend buys portability with the debugger's view and the C
    compiler's time.** Nim emits C, C++ or Objective-C, with `--lineDir` for
    `#line` directives and `--debugger:native` for gdb [40]; V reports about
    100,000 lines per second through Clang and 700,000 through tcc (its own
    figures) [41]; TinyCC 0.9.22 compiled 859,000 lines per second against GCC
    3.2 `-O0`'s 98,000 on a 2.4 GHz Pentium 4; it is LGPL, and its author is
    "no longer working on TCC" [42]. A debugger on generated C sees C names
    and C types; GHC's C code generator, "generally not included any more",
    "produces much slower code" and serves only to port GHC [20]. Sisu's
    status-byte unwinding maps onto C without `setjmp`, but overflow checks
    need compiler builtins, the context switch needs assembly, and a debug
    build pays a C front end. The strongest use is the bootstrap: Zig's
    `zig1.wasm` (2.6 MiB, 637 KB compressed) runs through a 4,000-line
    `wasm2c` and the C backend (8,108 lines) so that "only a C compiler on the
    host" is needed [7][8].

12. **A backend written in the language: what it costs and what it gives.**
    Go's SSA backend (proposed 2015) cut its benchmarks' CPU time by 5 to 35%
    on amd64 in 1.7 and by 20 to 30% on 32-bit ARM in 1.8, while the
    compiler, "still slower than we would like", got about 15% faster in 1.8
    [44]; the 1.5 translation from C had made builds "slower by a factor of
    about two" [44]. In go1.24.7 the `ssa` package is 293,239 lines, of which
    210,905 are generated from 15,730 lines of rules; the assemblers are
    77,443 lines and the linker 42,250, for about a dozen architectures [46]
    ([`go.md`](../languages/go.md): "good but not LLVM-class"). Plan 9's
    compilers "compile quickly, load slowly, and produce medium quality
    object code"; a new machine took "a couple of weeks' work"; a 12,600-line
    program compiled in 4.49 s unoptimised and 15.07 s optimised, against the
    vendor's 14.83 s and 43.59 s, and ran in 89.96 s against the vendor's best
    71.16 s [43]. Roc had direct
    machine-code dev backends and is replacing them with an interpreter,
    which it expects to be not much slower because that machine code is
    naive [21]
    ([`roc.md`](../languages/roc.md)). DMD's own backend gives D "very fast
    compilation speeds" beside LDC's and GDC's "strong optimization" [55]
    ([`d.md`](../languages/d.md)); Jai's beta measured 250,000 lines per
    second with its own x64 backend ([`odin-jai-c3.md`](../languages/odin-jai-c3.md)).
    The research point is TPDE: one analysis pass, then single-pass code
    generation, in "less than 8k lines of code" for an LLVM-IR backend of
    which "about 1.7k are architecture-specific for x86-64 and AArch64
    combined", on its framework and encoder libraries; 8 to 24 times faster
    than LLVM `-O0` with run time "similar" to it (within 9%), and on Wasm
    4.27 times faster than Cranelift with code 1.64 times slower [32].
    Copy-and-patch compilation, on TPC-H queries, generates code "two orders
    of magnitude faster than LLVM -O0" that runs 14% faster than it [31]. For Sisu, a
    single-pass backend for two architectures with object writers and DWARF
    is an estimated 20,000 to 35,000 lines of Sisu (TPDE's backend plus
    encoders, three object formats, DWARF and CodeView), a third to a half of
    stage 1; an estimate, not a measurement.

13. **Register allocation: local for debug, linear scan before graph
    colouring.** A debug build that keeps every SIR local in its frame slot
    (decision 246 keeps the debug profile that way) needs registers only for
    temporaries, which a strictly local greedy allocator handles: TPDE
    allocates "alongside code generation", spills round-robin, and spills
    unfixed values at branches to merge blocks [32]; Zig's register manager
    allocates per instruction and spills on demand [8]; Plan 9 assigned
    registers greedily by estimated cost, each use weighted three times per
    level of loop nesting [43]. Linear scan
    is "within 12%" of graph colouring's code and several times faster to run
    (Poletto and Sarkar 1999) [47]; on SSA it needs no data-flow analysis for
    lifetimes and absorbs SSA deconstruction (Wimmer and Franz 2010, in
    HotSpot's client compiler) [48]; Go's production allocator is "a version
    of a linear scan register allocator" that treats the function as one long
    block and "spills the value whose next use is farthest in the future"
    [46]; QBE's is linear with hints [33]. Graph colouring (Chaitin 1982;
    George and Appel's iterated coalescing 1996) [49] gives the last few
    percent at a higher compile cost; LLVM left linear scan in 3.0 for a
    greedy allocator with live-range splitting whose code is "1-2% smaller,
    and up to 10% faster" [50], and Cranelift's backtracking regalloc2 is
    over 10,000 lines [25]. That class of allocator belongs to the release
    backend, which LLVM supplies.

14. **Debug information and unwinding are most of an own backend's
    platform work.** With status-byte unwinding Sisu needs no language data
    area and no personality routine, only call-frame information for
    debuggers, profilers and the platform. Windows requires a `.pdata` entry
    for "all functions that allocate stack space or call another function",
    or "unreliable exception handling and debugging of processes" follows
    [51], and its debuggers read CodeView, which `lld-link` turns into a PDB.
    With a frame pointer in every function, TPDE's unwind information is one
    template after the prologue and around each epilogue [32]; Go keeps frame
    pointers for profilers at about 2% [44]. Sisu already writes DWARF 4 line
    tables, subprograms with typed locals and `.debug_frame` (decision 227,
    677 lines over gimli, 0.1 s of a stage-1 build), and claims the language
    is Rust so that gdb prints its values. Zig's experience (item 6) is that
    the Windows linker, a second architecture and Wasm debug information each
    took longer than the code generator they served.

15. **WebAssembly is a target, not Sisu's intermediate.** Wasm 3.0
    (September 2025) has memory64, multiple memories, GC, typed references,
    tail calls and exception handling [52]; threads are at phase 4, stack
    switching at phase 3 and shared-everything threads at phase 1, while JS
    Promise Integration is standardised (phase 5) [52]. Sisu's `single`
    profile runs tasks as coroutines on one thread, which needs stack
    switching, JSPI in browsers, or Binaryen's Asyncify, whose binaries are
    "around 50% larger on average" and "at worst 20% slower" when 20% larger,
    with SQLite about 5 times slower [53]. As an intermediate for native
    code, Wasm costs 45% (Firefox) to 55% (Chrome) on SPEC against native,
    with peaks of 2.08 and 2.5 times [54], plus Cranelift's 14% [22] and a
    Wasm runtime in every program. Emitting Wasm directly is a small backend
    (Zig's is 8,096 lines [8]); Binaryen, with "a simple C API in a single
    header" and an Apache-2.0 licence, optimises it for Emscripten, J2CL,
    Kotlin/Wasm, Dart and AssemblyScript [53]; LLVM's Wasm target serves
    `wasm32-wasi` in both profiles meanwhile.

16. **Tiering is the convergent answer, and its cost is checkable.** Zig
    (own backends for debug, LLVM for release), Rust (Cranelift as an option
    for debug), D (DMD, LDC, GDC over one front end), GHC (the native code
    generator by default, LLVM for numeric code) and Jai all split the
    profiles between a fast backend and an optimising one [2][20][29][55]. The
    cost Zig names is fast debug builds and slow release builds "with
    different bugs" ([`zig.md`](../languages/zig.md)). Sisu's conformance
    runner already compares the interpreter, debug and release native builds
    and builds from serialised SIR on every push (architecture.md,
    "Coverage"), which is the harness that catches a backend's private bug.

17. **Sisu's own numbers say which backend the targets depend on.** The
    backend is about a third of today's build, so a 100 kLOC debug build
    with today's Rust front end and Cranelift is about 8 s, of which code
    generation is about 2.7 s of wall time and 10.6 s of processor time on 4
    cores. Cranelift's back end is, roughly, two to four times faster than
    LLVM `-O0`'s (the whole-build savings in item 7 put a lower bound of
    about 1.7 on the ratio; TPDE's 4.27 times over Cranelift and 16 to 19
    times over `-O0` an upper one near 4), so LLVM `-O0` would make the same
    build about 11 to 16 s and miss 10's target, while a single-pass backend
    four times faster than Cranelift would make its share under a second.
    The larger term is the front end: compiled by today's backend, stage 1
    is four to seven times slower than stage 0, and the release level does
    not help, because the time is in the runtime's intrinsic dispatch and
    its walks of the type table (architecture decisions 232, 233, 246 and
    262). A
    self-hosted toolchain meets 10's targets only when the compiler itself
    is fast, which needs a runtime whose fast paths an optimiser can inline
    (written in Sisu and compiled with the program, not linked as a Rust
    library) and a release backend that optimises across that boundary. A
    release build of 100 kLOC with LLVM's optimising pipeline, at about five
    times `-O0`'s back-end cost (item 2), comes to about 30 to 60 s on 4
    cores: inside 10's 60 s, but not by a margin to rely on (Q-60). These
    are derived estimates, not measurements.

## Scores

Each candidate against the nine criteria, for Sisu: 3 good, 2 adequate,
1 poor, 0 fails a requirement. Compile speed is scored against 10's debug
target and then its release target.

| Candidate | From Sisu | Compile speed (debug / release) | Run speed | Debug info and unwinding | Targets | Licence | Dependency size | Maintenance, bus factor | Documentation |
|---|---|---|---|---|---|---|---|---|---|
| LLVM | 2 | 1 / 2 | 3 | 3 | 3 | 3 | 1 | 2 | 2 |
| Cranelift | 0 | 3 / 3 | 2 | 2 | 1 | 3 | 2 | 1 | 2 |
| QBE | 1 | unmeasured / 2 | 2 | 0 | 1 | 3 | 3 | 1 | 2 |
| libgccjit, GCC | 2 | 1 / 2 | 3 | 3 | 1 | 0 | 1 | 2 | 3 |
| MIR | 1 | 3 / 3 | 2 | 0 | 1 | 3 | 3 | 0 | 1 |
| C backend (clang, gcc or tcc) | 2 | 1 / 2 | 3 | 1 | 3 | 3 | 2 | 2 | 3 |
| Own backend in Sisu, single pass | 3 | 3 / 3 | 1 | 2 | 1 | 3 | 3 | 1 | 2 |
| Wasm as the intermediate | 2 | 2 / 2 | 1 | 1 | 2 | 3 | 1 | 2 | 3 |
| Tiered: own debug backend, LLVM release | 3 / 2 | 3 / 2 | 3 | 3 | 3 | 3 | 2 | 1 | 2 |

Reading the rows:

- **LLVM**: reachable from Sisu without FFI (bitcode and a process) or
  through LLVM-C under `unsafe` in a `[trust]`-listed wrapper (Crystal's
  route); not writable in Sisu. Misses the debug target (items 2, 17); best
  code, debug information and targets; 118 to 137 MiB; churn every six
  months but a very large community.
- **Cranelift**: fails the Sisu-only goal (Rust only, no C API), otherwise a
  strong debug backend; no 32-bit, Thumb or Wasm output; monthly breaking
  releases; DWARF is the user's to write (Sisu's is).
- **QBE**: a C program fed text and followed by an assembler, or a
  transcription; no debug information beyond lines and no unwind tables;
  three architectures; one principal author.
- **libgccjit**: the GPL fails the licence criterion for the toolchain;
  every target is its own GCC build and Wasm is absent.
- **MIR**: no object files and no debug information, so it fails as an
  ahead-of-time backend despite its speed; one author.
- **C backend**: portable and fast at `-O2`, but debug builds pay a C
  compiler and the debugger sees C.
- **Own backend**: the only candidate written in Sisu; fastest to compile;
  code like LLVM `-O0`; every target, object format and debug format is
  Sisu's to write, so the target score stays low until each arrives.
- **Wasm as intermediate**: not native, and Sisu's tasks need proposals not
  yet standardised.
- **Tiered**: takes the own backend's scores for debug builds and LLVM's for
  release builds and for every target the own backend lacks; maintenance is
  two backends, checked by one runner.

## Recommendation for Sisu

Recorded as D-087 and D-088 (leaning); 18's "Bootstrapping" gains an
addendum.

- **Debug builds: a backend written in Sisu.** SIR is lowered, in Sisu, to
  a backend-neutral low IR (LIR) that carries what `sisu-codegen` decides
  today: layouts, calls by address with a status byte, entry probes and
  yield points, drop glue and dispatchers. A single-pass code generator in
  TPDE's manner compiles LIR to x86-64 and then AArch64 machine code: one
  analysis pass for liveness and loops, every SIR local in its frame slot,
  a local greedy allocator for temporaries, a frame pointer in every
  function, its own instruction encoders, ELF and then Mach-O and COFF
  writers, DWARF line, type and frame information as architecture decision
  227 writes it,
  CodeView and `.pdata` for Windows, and the system linker as today.
  Functions compile in parallel through `std.par` (D-069) with a fixed
  merge order, so the output stays deterministic for the fixed point. Hot
  replacement and reifiable frames (D-053) are this backend's to give.
- **Release builds: LLVM, through bitcode.** The release backend writes
  LLVM bitcode from LIR (textual IR first, as the prototype that measures
  Q-60) and runs a pinned LLVM as a separate program, one process per
  codegen unit; an in-process path may later use only LLVM-C's coarse entry
  points (parse bitcode, run a pipeline, emit an object). It never builds IR
  through the C API or the C++ API. LLVM also compiles debug builds at `-O0`
  for every target the Sisu backend does not cover: RISC-V 64 and 32,
  Cortex-M, Wasm and Windows on AArch64 (iOS, Android and x86-64 kernels
  follow from the Sisu backend's two architectures and its object writers).
- **Not adopted.** Cranelift beyond the transition (Rust only, no C API);
  libgccjit (GPL); MIR (no ahead-of-time output); WebAssembly as the
  intermediate; a C backend, unless Q-58 chooses C as the bootstrap seed.
  QBE is the fallback if LLVM is ever refused: an MIT backend small enough to
  transcribe into Sisu for amd64, arm64 and rv64 release builds.
- **Order of work.** (1) LIR in Sisu, checked by translating it one to one
  into Cranelift IR inside the Rust backend, so that executables stay byte
  for byte the same. (2) The x86-64 Linux debug backend, linked against the
  Rust runtime unchanged, done when the native conformance list and stage 1's
  native checks pass and the backend compiled by itself compiles itself to
  the same bytes. (3) The runtime in Sisu under `unsafe` (D-086), with the
  context switch emitted by the backend and per-type copy, drop, equality and hashing
  generated rather than walked from the type table; Rust retires on x86-64
  Linux at the whole toolchain's fixed point. (4) The LLVM release backend,
  which emits 22's machine intrinsics as inline assembly in the IR it writes;
  its textual-IR prototype can start right after step 1. (5) AArch64 on Linux
  and macOS, then COFF on Windows, in the Sisu backend; Cranelift retires
  when both architectures are covered on Linux and macOS.
- **A correction for 10.** "Zig's self-hosted x86 backend and Cranelift for
  Rust show 5-10x faster debug compilation than LLVM" holds for Zig (three
  to five times the whole build) and for back ends measured alone, not
  for Rust's whole builds, where Cranelift saves 5 to 30% (item 7).

## Open questions

- Q-58: what seeds the toolchain once no Rust remains, and whether Sisu
  writes WebAssembly itself (which would make both a Zig-style seed and the
  `wasm32-wasi` debug backend, and raises how tasks switch on Wasm).
- Q-59: how LLVM ships, which versions the release backend supports, and
  what `--release` does without it.
- Q-60: what LLVM buys on Sisu's own programs, before and after the runtime
  is in Sisu, and whether the debug backend must keep scalars in registers.
- Q-61: which linker the toolchain uses, and how a debug rebuild relinks in
  under a second.

## Sources

1. Zig devlog 2025: Kelley, "Self-Hosted x86 Backend is Now Default in Debug Mode" (8 June 2025); Kelley and Young, "New Aarch64 Backend" (23 July 2025); Lugg, "Parallel Self-Hosted Code Generation" (14 June 2025). https://ziglang.org/devlog/2025/
2. Zig 0.15.1 release notes (19 August 2025). https://ziglang.org/download/0.15.1/release-notes.html
3. Zig 0.16.0 release notes (13 April 2026). https://ziglang.org/download/0.16.0/release-notes.html
4. Zig 0.17.0 release notes (1 October 2026). https://ziglang.org/download/0.17.0/release-notes.html
5. ziglang/zig issue 13265, "directly output LLVM bitcode rather than using LLVM's IRBuilder API" (opened 22 October 2022; closed by PR 19031, 24 February 2024). https://github.com/ziglang/zig/issues/13265
6. ziglang/zig issue 16270, "make the main zig executable no longer depend on LLVM, LLD, and Clang libraries" (opened 29 June 2023; open). https://github.com/ziglang/zig/issues/16270
7. Kelley, "Goodbye to the C++ Implementation of Zig" (7 December 2022). https://ziglang.org/news/goodbye-cpp/
8. Zig sources on Codeberg, master, line counts measured 2 November 2026: `src/codegen/x86_64/CodeGen.zig` (191,905), `src/codegen/aarch64/`, `src/codegen/c.zig` (8,108), `src/codegen/wasm/CodeGen.zig` (8,096), `lib/std/zig/llvm/Builder.zig` (16,766), `ir.zig` (2,317), `bitcode_writer.zig` (435), `src/zig_llvm.cpp` (550), `src/register_manager.zig`. https://codeberg.org/ziglang/zig
9. LLVM Developer Policy: "IR Backwards Compatibility", "C API Changes", "Copyright, License, and Patents". https://llvm.org/docs/DeveloperPolicy.html
10. LLVM release notes, "Changes to the C API", 13.0.0 (2021) to 21.1.0 (2025). https://releases.llvm.org/13.0.0/docs/ReleaseNotes.html ; https://releases.llvm.org/15.0.0/docs/ReleaseNotes.html ; https://releases.llvm.org/16.0.0/docs/ReleaseNotes.html ; https://releases.llvm.org/17.0.1/docs/ReleaseNotes.html ; https://releases.llvm.org/18.1.0/docs/ReleaseNotes.html ; https://releases.llvm.org/19.1.0/docs/ReleaseNotes.html ; https://releases.llvm.org/20.1.0/docs/ReleaseNotes.html ; https://releases.llvm.org/21.1.0/docs/ReleaseNotes.html
11. LLVM sources: `DEBUG_METADATA_VERSION` in `llvm/include/llvm/IR/Metadata.h` (1 in release/3.5.x, 2 in 3.6.x, 3 from 3.7.x to main) and `llvm::UpgradeDebugInfo` in `llvm/lib/IR/AutoUpgrade.cpp` (main). https://github.com/llvm/llvm-project
12. HertzDevil, "LLVM opaque pointer support has landed", Crystal blog (23 March 2023). https://crystal-lang.org/2023/03/23/llvm-opaque-pointers
13. "Crystal 1.11.0 is released" (8 January 2024). https://crystal-lang.org/2024/01/08/1.11.0-released/
14. Odin, "Getting Started: Install" (LLVM 17 to 22) and `src/llvm_backend.hpp` (the `llvm-c` headers). https://odin-lang.org/docs/install/ ; https://github.com/odin-lang/Odin
15. Ginger Bill, "Working on a Custom Backend for Odin" (9 March 2017). https://odin.handmade.network/blog/p/2210-working_on_a_custom_backend_for_odin
16. rust-lang/rust, main: `compiler/rustc_codegen_llvm/src/llvm/ffi.rs` (251 LLVM-C and 131 `LLVMRust` declarations), `compiler/rustc_llvm/llvm-wrapper/` (`RustWrapper.cpp` 1,940 lines, `PassWrapper.cpp` 1,662, `CoverageMappingWrapper.cpp` 217, `SymbolWrapper.cpp` 182, `LLVMWrapper.h` 54, `Linker.cpp` 43), `.gitmodules` (`rust-lang/llvm-project`, branch `rustc/23.1-2026-07-22`), `src/bootstrap/src/core/build_steps/llvm.rs` (external LLVM 22 or newer). https://github.com/rust-lang/rust
17. Swift's fork of llvm-project, README. https://github.com/swiftlang/llvm-project
18. JuliaLang/julia `deps/llvm.version` (LLVM 23.1.4, branch `julia-23.1.4-1`); "Working with LLVM", Julia developer documentation. https://github.com/JuliaLang/julia ; https://docs.julialang.org/en/v1/devdocs/llvm/
19. "Julia 1.9 Highlights" (April 2023). https://julialang.org/blog/2023/04/julia-1.9-highlights/
20. GHC User's Guide, "GHC Backends" (latest). https://downloads.haskell.org/ghc/latest/docs/users_guide/codegens.html
21. Feldman, on rewriting the Roc compiler in Zig (gist; first comments 4 February 2025). https://gist.github.com/rtfeldman/77fb430ee57b42f5f2ca973a3992532f
22. Cranelift README, wasmtime main. https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/README.md
23. crates.io, `cranelift-codegen` versions (0.116.0, 20 January 2025, to 0.137.0, 20 October 2026). https://crates.io/crates/cranelift-codegen/versions
24. Cranelift 0.123.19, regalloc2 0.12.2, gimli 0.32.3 and object 0.37.3 as this repository's lockfile fetches them; line counts measured 2 November 2026 (`cranelift-codegen` 119,041 lines of Rust and 42,160 of ISLE).
25. Fallin, "Cranelift, Part 4: A New Register Allocator" (9 June 2022). https://cfallin.org/blog/2022/06/09/cranelift-regalloc2/
26. `rustc_codegen_cranelift` README. https://github.com/rust-lang/rustc_codegen_cranelift
27. Nelson, "Using rustc_codegen_cranelift for debug builds", Inside Rust blog (15 November 2020). https://blog.rust-lang.org/inside-rust/2020/11/15/Using-rustc_codegen_cranelift/
28. Alden, "Cranelift code generation comes to Rust", LWN (15 March 2024). https://lwn.net/Articles/964735/
29. Rust project goals 2026, "Improve cg_clif performance". https://goals.rust-lang.org/2026/improve-cg_clif-performance.html
30. `craneliftc`, unofficial C bindings to Cranelift (0.5.1, July 2023). https://lib.rs/crates/craneliftc
31. Xu and Kjolstad, "Copy-and-Patch Compilation: A fast compilation algorithm for high-level languages and bytecode", OOPSLA 2021 (arXiv 2011.13127, 2020 to 2021). https://arxiv.org/abs/2011.13127
32. Schwarz, Kamm and Engelke, "TPDE: A Fast Adaptable Compiler Back-End Framework", arXiv 2505.22610 (28 May 2025), CGO 2026. https://arxiv.org/abs/2505.22610
33. QBE home page, IL documentation and releases (1.0, 22 June 2022; 1.1, 1 February 2023; 1.2, 16 February 2024; 1.3, 1 June 2026); `qbe-1.3.tar.xz` (SHA-256 d587905d...6e320; 16,104 lines of C in the core, measured 2 November 2026). https://c9x.me/compile/ ; https://c9x.me/compile/releases.html
34. Aromal, "QBE 1.3: metaprogramming, performance, and cross-platform support", LWN (10 July 2026). https://lwn.net/Articles/1080519/
35. Hare FAQ and installation guide. https://harelang.org/documentation/faq.html ; https://harelang.org/documentation/install/
36. libgccjit documentation (17.0.0, experimental): introduction, tutorial part 5 (ahead-of-time), "ABI and API compatibility". https://gcc.gnu.org/onlinedocs/jit/
37. `rustc_codegen_gcc` README. https://github.com/rust-lang/rustc_codegen_gcc
38. GCC, "Status of Supported Architectures from Maintainers' Point of View". https://gcc.gnu.org/backends.html
39. MIR README (vnmakarov/mir). https://github.com/vnmakarov/mir
40. Nim Compiler User Guide (`--debugger:native`, `--lineDir`). https://nim-lang.org/docs/nimc.html
41. V home page (self-reported compile speeds). https://vlang.io/
42. Bellard, Tiny C Compiler home page. https://bellard.org/tcc/
43. Thompson, "Plan 9 C Compilers", Proceedings of the Summer 1990 UKUUG Conference. https://plan9.io/sys/doc/compiler.html
44. Go 1.5 (August 2015), 1.7 (August 2016) and 1.8 (February 2017) release notes. https://go.dev/doc/go1.5 ; https://go.dev/doc/go1.7 ; https://go.dev/doc/go1.8
45. Go FAQ, "What compiler technology is used to build the compilers?". https://go.dev/doc/faq
46. Go sources: `src/cmd/compile/internal/ssa/regalloc.go` (master); line counts of go1.24.7's `cmd/compile`, `cmd/internal/obj` and `cmd/link`, measured 2 November 2026. https://github.com/golang/go
47. Poletto and Sarkar, "Linear Scan Register Allocation", ACM TOPLAS 21(5), 1999. https://www.cse.iitm.ac.in/~krishna/cs3300/linearscan.pdf
48. Wimmer and Franz, "Linear Scan Register Allocation on SSA Form", CGO 2010. https://bernsteinbear.com/assets/img/wimmer-linear-scan-ssa.pdf
49. Chaitin, "Register Allocation and Spilling via Graph Coloring", SIGPLAN Symposium on Compiler Construction, 1982. https://web.eecs.umich.edu/~mahlke/courses/583w23/reading/chaitin82.pdf ; George and Appel, "Iterated Register Coalescing", ACM TOPLAS 18(3), 1996 (Princeton TR-498-95). https://www.cs.princeton.edu/research/techreps/TR-498-95
50. Olesen, "Greedy Register Allocation in LLVM 3.0", LLVM blog (18 September 2011). https://blog.llvm.org/2011/09/greedy-register-allocation-in-llvm-30.html
51. Microsoft, "x64 exception handling" (updated May 2026). https://learn.microsoft.com/en-us/cpp/build/exception-handling-x64
52. WebAssembly, "Wasm 3.0 Completed" (17 September 2025); WebAssembly proposals by phase. https://webassembly.org/news/2025-09-17-wasm-3.0/ ; https://github.com/WebAssembly/proposals
53. Binaryen README; Zakai, "Pause and Resume WebAssembly with Binaryen's Asyncify" (16 July 2019). https://github.com/WebAssembly/binaryen ; https://kripken.github.io/blog/wasm/2019/07/16/asyncify.html
54. Jangda, Powers, Berger and Guha, "Not So Fast: Analyzing the Performance of WebAssembly vs. Native Code", USENIX ATC 2019. https://www.usenix.org/conference/atc19/presentation/jangda
55. D, "Downloads" (DMD, GDC, LDC). https://dlang.org/download.html
56. ziglang/zig issue 16268, "upstream Aro and use it for translate-c instead of clang". https://github.com/ziglang/zig/issues/16268
57. Measured on this repository's machine (Ubuntu 24.04, x86-64, 2 November 2026): `libLLVM-17.so.1` 123,671,544 bytes, `libLLVM.so.18.1` 123,215,144, `libLLVM.so.20.1` 143,545,784; `llc --version` (LLVM 18.1.3) lists 44 registered targets.
58. Rust-GCC, "2026-06 monthly report" (gccrs). https://raw.githubusercontent.com/Rust-GCC/Reporting/refs/heads/main/2026/2026-06-monthly-report.org
