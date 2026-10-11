# Memory management

## The question

How a program's memory is reclaimed and how aliasing of mutable state is
controlled, which together determine memory safety, data-race safety,
determinism, runtime size, and most of the learning curve.

## Why it matters more than anything else

The industry data in [../evidence/memory-safety.md](../evidence/memory-safety.md)
is the single strongest result in this research: about 70% of severe
vulnerabilities in C and C++ codebases are memory-safety bugs (Microsoft
2019, Chromium 2020), and Android's share fell from 76% in 2019 to under
20% by late 2025 as new code moved to memory-safe languages, with Rust
code showing roughly a thousand times fewer vulnerabilities per line than
C/C++ and four times fewer rollbacks ([rust.md](../languages/rust.md)).
Government guidance (US ONCD 2024, CISA) now names memory-unsafe
languages as a product-security bad practice. A new language that is not
memory-safe by default is answering a question nobody is asking.

## Approaches found

Ovadia's taxonomy ([vale.md](../languages/vale.md)) is used as the
checklist; for each approach the table records what patterns it cannot
express without an escape hatch, which is the test that matters.

| Approach | Languages | Guarantees | Cost | Cannot express without escape hatch |
|---|---|---|---|---|
| Manual with debug checks | C, Zig, Odin, Jai, C3 | None in release (Zig `ReleaseFast` is UB) | Programmer discipline | Nothing; safety is the escape hatch |
| Tracing GC | Java, Go, C#, OCaml, Haskell, JS, Lisp, Erlang (per-process) | Memory safety; not data-race safety (Go's Uber study: 46% missing locks) | Runtime, pauses or barriers, 2-5x memory headroom (Hertz & Berger 2005), non-deterministic destruction, unsuitable for the smallest targets | Deterministic resource release (needs `defer`/`using`/`try-with-resources`) |
| Reference counting | Swift, Python, Nim, Objective-C | Memory safety modulo cycles; deterministic release | Retain/release traffic (Swift: measurable on pointer-heavy code; atomics for thread safety), cycles need `weak` or a collector; no aliasing control | Cycles (leak), data-race freedom (needs separate discipline; Swift 6 added it) |
| RC with reuse (Perceus) | Koka, Lean 4, Roc | As RC, with in-place mutation when unique | Same cycle story; atomic ops when shared; untested on large shared server heaps | Same as RC |
| Ownership + borrowing with lifetimes | Rust | Memory safety and data-race freedom, zero runtime | Named lifetimes, learning curve (the obstacle named by non-adopters and ex-users; one current user in ten calls the borrow checker a big problem), self-referential structs (`Pin`), async interaction | Graphs, observers, intrusive structures (need `Rc<RefCell>`, arenas or `unsafe`) |
| Mutable value semantics, second-class references | Hylo; Swift partially; SPARK's ownership subset | Memory and race safety with no lifetime syntax | Immature; stored references need a second tier | Iterators, cursors, observers, back-references (projections cover some) |
| Argument conventions + inferred origins | Mojo | Rust-class call-boundary safety with readable signatures | "Mutable aliasing" still unresolved on the roadmap | General application code (unknown yet) |
| Linear types | Austral, Idris 2 (QTT) | Resource and memory safety | Every value used exactly once: hostile to loops and early return | Plain data (needs affine default) |
| Generational references | Vale (archived) | Memory safety with aliasing freedom | 5-11% runtime; project changed model three times then stalled; successor Valen moved to "group borrowing" | Nothing; cost is runtime checks |
| Regions / arenas with handles | Odin, Jai, Zig practice; Rust slotmaps; Verona (research) | Temporal safety by construction within the arena | Handles instead of pointers; lifetime of the arena | Fine-grained individual frees |
| Per-isolate heaps | Erlang, Pony (ORCA), Dart isolates | Pause isolation, race freedom by no sharing | Copy on send; no shared large data (Dart is now adding sharing back) | Shared mutable caches |
| Modes orthogonal to types | OxCaml (Jane Street) | Locality, uniqueness, portability inferred with legacy defaults | Nine modal axes and counting | Unknown; live experiment |
| Hardware capabilities | CHERI (Morello, CHERIoT) | Spatial and some temporal safety for C | Hardware availability | Everything until the hardware ships |

## What the evidence says

1. **Opt-in safety is never adopted.** D's `@safe` (DIP 1028 accepted then
   reverted), C++ safety profiles (six years, "none successful", now
   targeting C++29), Carbon ranking safety fourth then reversing in 2025,
   Zig's `ReleaseFast`: in every case the safe mode exists and is not the
   default and is therefore not used ([d.md](../languages/d.md),
   [cpp.md](../languages/cpp.md), [carbon.md](../languages/carbon.md),
   [zig.md](../languages/zig.md)). The safe subset must be the default from
   version 1 with `unsafe` as the opt-out.
2. **"GC but optional" is not a stable design point.** D split into two
   camps and then forked; Nim changed its default memory manager three
   times ([d.md](../languages/d.md), [nim.md](../languages/nim.md)). A
   language needs one memory model for its standard library.
3. **Rust's safety benefit is measured; its attribution is not.** The
   Android data cannot separate the contribution of the borrow checker from
   bounds checks, no-UB-in-safe-code and sum types
   ([rust.md](../languages/rust.md)). Zig's bet is that the latter three
   give most of the benefit; nobody has the data. This is the central
   unknown for any design that keeps safety while simplifying the checker.
4. **The learning curve is attributable to first-class references and
   named lifetimes, not to ownership.** Rust surveys, Graydon Hoare's
   retrospective, Hylo's papers, Mojo's design and Borretti's analysis
   (most references are function arguments) all point the same way
   ([rust.md](../languages/rust.md), [hylo.md](../languages/hylo.md),
   [mojo.md](../languages/mojo.md), [austral.md](../languages/austral.md)).
   Single ownership and moves are the cheap part. Swift's `~Escapable`,
   Mojo's inferred origins, SPARK's lifetime-free ownership subset and
   OxCaml's modes are all moves toward "ownership without named lifetimes".
5. **Moves and destructors without a sound checker are a performance
   model, not a safety model.** Nim's 2.2.x releases were still fixing
   use-after-free bugs in the compiler's own move inference
   ([nim.md](../languages/nim.md)).
6. **Value semantics is the part of Swift everyone keeps.** Its designers
   (Abrahams, to Hylo), its imitators (Mojo, Kotlin's value-class KEEPs),
   and its own retrospectives agree that value semantics by default with an
   explicit reference category is the right core; hidden ARC cost and
   classes-in-the-core are what they drop ([swift.md](../languages/swift.md)).
7. **Stored references are the unsolved residue.** Every lifetime-free
   design (Hylo, SPARK, Mojo so far) handles function-call borrowing and
   struggles with iterators, observers, graphs and back-references. The
   practical answers in use are arenas with generational handles (games,
   compilers, Rust's `slotmap`/`petgraph`), shared ownership with weak
   back-references (Swift), and indices. No empirical study measures how
   often real code needs more ([hylo.md](../languages/hylo.md)).
8. **Reference counting with reuse is competitive for immutable data and
   untested for shared server heaps.** Lean and Koka benchmarks are
   compute-bound ([koka-effekt.md](../languages/koka-effekt.md),
   [idris-agda-lean.md](../languages/idris-agda-lean.md)); Roc's bugs
   cluster where hosts and FFI share ownership ([roc.md](../languages/roc.md)).
9. **Per-domain isolation beats per-reference capabilities for
   usability.** Pony's six capabilities on every reference were too much
   ceremony; Verona retreated to regions; Swift 6.2 retreated to a
   sequential default ([pony.md](../languages/pony.md),
   [swift.md](../languages/swift.md)). Rust's `Send`/`Sync`, which only
   bite at thread boundaries, are the accepted point on that ladder.

## Recommendation for Sisu

Ownership with value semantics; borrow-by-default parameter conventions
(`x`, `mut x`, `own x`) with the mutation visible at the call site;
second-class references (`ref T`) with Rust's aliasing rule and non-lexical
scopes but no named lifetimes; `view struct` as the single-lifetime stored-
reference tier; `Shared[T]` (atomic RC with Perceus-style reuse, `Weak` for
cycles) for shared ownership; `Arena[T]` with generational handles as the
graph tier; deterministic destruction at scope end; `unsafe` as a block and
a package permission; no tracing GC. See
[proposal/04](../../proposal/04-memory-and-ownership.md); decisions D-004
through D-009.

The bet, stated plainly: that Hylo/Mojo/SPARK-style lifetime-free
ownership plus views plus arenas covers what Rust's lifetimes cover in
practice, at a fraction of the learning cost, while keeping all of Rust's
measured safety benefit. Validating it is the first job of the prototype
([proposal/16](../../proposal/16-validation-plan.md)).

## Open questions

- Q-1 optional GC tier for application code; Q-2 `Shared` variants and
  cycle collection; Q-3 implicit copies; Q-4 allocation failure; Q-18
  convention naming; Q-37 linear resource types.
