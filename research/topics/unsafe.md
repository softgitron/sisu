# Unsafe code

## The question

Where a memory-safe language draws the line between the code its compiler
checks and the code it trusts; which operations sit on the trusted side;
how that side is marked, kept small, reviewed and tested; and what Sisu
needs so that its runtime and compiler backend, today `sisu-rt` and
`sisu-codegen` in Rust, can be written in Sisu and the Rust
implementation retired. The givens: ownership with second-class
references and single-lifetime views (D-004); views, scopes and suspended
task frames that are never leaked (D-058); capabilities as values, with
`unsafe` and `extern` gated by the root manifest's `[trust]` (D-015,
D-032); a deterministic `pure` (D-074); comptime without I/O (D-018); an
interpreter whose dynamic monitor checks the ownership rules as the
program runs (D-052, [18](../../proposal/18-next-steps.md) Phase 3); the
bootstrap rule of D-078; and the owner's goal of a Sisu-only toolchain in
which `unsafe` is as small as possible, used only where it is truly
needed, declared in the package manifest, visible to dependents and
listed by `sisu audit`.

## Approaches found

| Approach | Languages | Marker and granularity | Audit story | Evidence |
|---|---|---|---|---|
| Block that permits, function and trait markers that oblige, a written soundness contract | Rust; C# 15's new model; D (`@safe`/`@trusted`/`@system`, per function) | `unsafe { }`, `unsafe fn`, `unsafe trait`/`impl`, `unsafe extern`, unsafe attributes | `// SAFETY:` per block, `# Safety` per function, cargo-geiger, cargo-vet, Miri | [1]-[27], [31], [43] |
| Expression marker as a diagnostic | Swift 6.2 strict memory safety (opt-in) | `@unsafe`/`@safe` declarations, `unsafe` expression like `try`, warnings only | "audit the uses of unsafe constructs within an entire project" | [28]-[30] |
| Module granularity | Modula-3 (`UNSAFE MODULE`), Austral (`pragma Unsafe_Module`), Safe Haskell (`Safe`/`Trustworthy`/`Unsafe` plus package trust) | the module; a safe module cannot import an unsafe interface | the build lists unsafe modules transitively | [45], [47], [52] |
| Pseudo-package or pseudo-module | Go (`unsafe`), Oberon (`SYSTEM`) | importing it; rules written as patterns | go-geiger, `go vet`, `-d=checkptr` | [32]-[38], [46] |
| No marker, checks by build mode | Zig (illegal behaviour checked in Debug and ReleaseSafe), Odin, Nim | none at use sites | allocator leak checks, debug fills | [39]-[42] |
| Naming convention | Haskell (`unsafePerformIO`), OCaml (`Obj.magic`, `-unsafe`), Ada (`Unchecked_Conversion`) | the name | grep; Ada's `pragma Restrictions` | [44], [53], [54] |
| Checked library API instead of raw memory | Java FFM (`MemorySegment`, `Arena`), .NET `Span<T>`, Swift `Span` | restricted methods only | run-time bounds and lifetime checks | [30], [31], [55] |
| Separated or sandboxed foreign memory | Vale's Fearless FFI (planned), Cyclone (no escape; runtime in C) | none in the language | sandboxing, scrambled references | [48], [51] |
| Build-level permission per package | Pony `--safe`, Java `--enable-native-access`, C# `AllowUnsafeBlocks`, Cargo `[lints] unsafe_code`, Tock capsules, Safe Haskell package trust, Sisu's `[trust]` | granted by whoever builds the program | the grant list is the audit's first page | [9], [31], [50], [52], [55], [56] |

## Language by language

### Rust

The Reference lists what only unsafe code may do: dereference a raw
pointer, read or write a mutable or unsafe external static, read a union
field, call an unsafe function, call a `target_feature` function from one
without it, implement an unsafe trait, declare an `extern` block and apply
an unsafe attribute [1]. Everything else, raw pointers included, can be
created and passed in safe code. The contract is one sentence, "No matter
what, Safe Rust clients can't cause Undefined Behavior", with an
asymmetry: safe code trusts the unsafe code it calls, but unsafe code
"cannot trust Safe Rust chosen or supplied by its clients", which is why
`BTreeMap` must stay sound under an `Ord` that is not total, and why
unsafe traits exist: they shift the obligation to the implementor [2].
Raw pointers are neither `Send` nor `Sync`, "more of a lint", so that a
type that holds one does not become thread-safe by accident [3].
Dereferencing a raw pointer yields a reference with an unbounded lifetime
that "becomes as big as context demands", the textbook use-after-free of
unsafe Rust [3]. Unsafe code must be "exception safe to the point of not
violating memory safety": a panic between `set_len` and the
initialisation of the elements exposes uninitialised memory to the drop
[3].

Three Rust decisions are revisions, and each is evidence. In 2015 the
`thread::scoped` API was found unsound because a guard could be leaked,
and RFC 1066 made `mem::forget` safe instead of making leaks impossible
[7]; Sisu took the other branch (D-058). The 2024 edition makes
`unsafe_op_in_unsafe_fn` warn by default, because the `unsafe` on a
function "was performing two roles" (an obligation on callers and a
permission in the body) and the second "was determined to be too risky
without explicit `unsafe` blocks"; it requires `unsafe extern` blocks,
since "it is the responsibility of the author of the `extern` block to
ensure that the signatures are correct", with per-item `safe` and
`unsafe` qualifiers; and it marks attributes such as `no_mangle` unsafe
[4]. Rust 1.84 (January 2025) stabilised the strict-provenance API
(`addr`, `with_addr`, `map_addr`, `expose_provenance`,
`with_exposed_provenance`, `without_provenance`, `dangling`), code that
is "easier to analyze for the compiler, and also benefits tools like Miri
and architectures like CHERI" [5], after a 2022 essay headed
"Integer-To-Pointer Casts Are The Devil" [6]. Unsafe fields (RFC 3458),
which make writing a field that carries a safety invariant, such as a
`Vec`'s length, an unsafe operation, were accepted after more than a
decade of discussion and are a 2026 stabilisation goal [8].

The audit ecosystem exists because the boundary is explicit. Cargo's
`[lints]` table (respected as of 1.74, November 2023) lets a manifest say
`unsafe_code = "forbid"` [9]; clippy's `undocumented_unsafe_blocks` and
`missing_safety_doc` require the comments [10]; the Linux kernel requires
that "Any `unsafe` block must be preceded by a `// SAFETY:` comment" and
that "Unsafe functions must document their safety preconditions under a
`# Safety` section" [11]; cargo-geiger counts unsafe across a dependency
tree as "statistical input to auditing" [12]; Google's audits grade
crates from ub-risk-0 ("do not contain unsafe Rust code") to ub-risk-4
("very dangerous unsafe rust code") and ask for two reviewers for a
thorough soundness review unless one is very experienced [13]. In
Android about 4% of the Rust code is inside `unsafe{}`; the first
potential memory-safety vulnerability in Android's Rust (CVE-2025-48530,
in the CrabbyAVIF decoder) was a linear overflow in unsafe code that
Scudo's guard pages made non-exploitable, and Google reports that
treating a line of unsafe Rust like a line of C "significantly
overestimates the risk", in part because "unsafe{} doesn't actually
disable all or even most of Rust's safety checks" [27]. The cost is
real: "writing correct unsafe code" is a big problem for 12.6% of 2024
survey respondents and 11.0% in 2025
([surveys.md](../evidence/surveys.md), [rust.md](../languages/rust.md)),
and the aliasing model it must obey is still research
([rust.md](../languages/rust.md)).

### Swift

Swift has the `Unsafe*Pointer` family and scoped access through closures
(`withUnsafeBufferPointer`, `withUnsafeMutablePointer(to:)`). SE-0447
(`Span`, Swift 6.2) lists what is wrong with the closure form: the
pointer "is unsafe and unmanaged", its subscript "is only bounds-checked
in debug builds of client code", and it "might escape the duration of the
closure"; `Span` is non-escapable and "performs bounds-checking on every
access" [30]. SE-0458 (Swift 6.2) adds opt-in strict memory safety: five
dimensions (lifetime, bounds, type, initialisation and thread safety);
`@unsafe` declarations, implicit when a signature mentions an unsafe
type; `@safe` for a declaration with unsafe types in its signature that
is safe to use, such as `Array.withUnsafeBufferPointer`; an `unsafe`
expression that "is much like `try` and `await`" but "doesn't propagate
outward"; `for unsafe` loops; and `@unsafe` conformances. "All of the
memory-safety diagnostics produced by the strict memory safety mode will
be warnings" [28]. The vision behind it concedes that the "unsafe"
naming convention "is not entirely sufficient for identifying all Swift
code that makes use of unsafe constructs", and its acceptance (January
2025) says it "does not expect that strict safety will be enabled in a
future language mode" [29]. Swift's marker is a lint: it makes unsafe
code auditable in the modules that opt in and guarantees nothing
elsewhere.

### C# and .NET

C# has had `unsafe` contexts, pointers, `fixed` (pinning) and
`stackalloc` since 1.0, and a project must set `AllowUnsafeBlocks` to
compile any of it [31]. The C# 15 preview (.NET 11, October 2026) starts
"a multirelease effort to redefine memory safety": the goal is "to tie
the `unsafe` context to the operations that actually access unmanaged
memory, rather than to the existence of pointer types". Declaring a
pointer, taking an address, `fixed` and `sizeof` no longer need
`unsafe`; dereference, `->`, element access and calls through function
pointers still do; `unsafe` on a member now marks it *requires-unsafe*,
so "the audit obligation flows to the caller"; a `safe` keyword marks
`extern` members that are safe to call; an `unsafe(expression)` form
covers places a block cannot go; the assembly records which rules it was
compiled under, and a compatibility mode treats an old assembly's
pointer-typed signatures as requires-unsafe [31]. The research file
calls this a redefinition of a 1.0 keyword after 24 years
([csharp.md](../languages/csharp.md)). It converges on Rust's shape:
creating pointers is safe, using them is not, and an obligation on
callers is a declaration of its own.

### Go

Go has no `unsafe` keyword; it has a package. `unsafe.Pointer` converts
between pointer types and `uintptr` under six documented patterns, and
"Code not using these patterns is likely to be invalid today or to become
invalid in the future"; `go vet` finds some violations, but "silence from
'go vet' is not a guarantee that the code is valid" [32]. Go 1.14 added
`-d=checkptr`, run-time checks that a converted pointer is aligned and
that arithmetic stays inside one heap object, on by default under
`-race` and `-msan`, with the runtime exempt [33]. The most common bad
patterns got dedicated functions: `unsafe.Add` and `unsafe.Slice` in 1.17,
`unsafe.String`, `unsafe.StringData` and `unsafe.SliceData` in 1.20 [32].
Passing pointers to C has its own rules, checked at run time by
`cgocheck`: Go memory handed to C may not contain unpinned Go pointers,
and C may not keep a Go pointer after the call unless it is pinned with
`runtime.Pinner` [34]. Studies find `unsafe` in 24% of 2,438 popular Go
projects, mostly for operating-system and C interop, with risky
conversions in 6% of them [37], and in 38% of 500 top projects directly
and 91% counting dependencies [38].

Go's runtime is the precedent for a runtime written in the language it
runs. It needs a system stack (`systemstack`, `mcall`) on which code "is
implicitly non-preemptible", `//go:nosplit` functions that must explain
themselves, directives that only the runtime may use
(`go:systemstack`, `go:nowritebarrier`, `go:nowritebarrierrec`,
`go:uintptrkeepalive`), rules for unmanaged memory obtained with
`sysAlloc`, `persistentalloc` and `fixalloc` [35], and assembly for the
context switch itself. Its escape hatches leak when they are reachable:
Go 1.23's linker refuses `//go:linkname` references to standard-library
internals not marked for it, because packages had come to depend on the
runtime's internals [36]. And the runtime calls the operating system the
supported way: since Go 1.11 it uses `libSystem.dylib` on macOS and iOS
"instead of calling the kernel directly" [36].

### Zig

Zig has no `unsafe`. "Many operations in Zig trigger what is known as
Illegal Behavior"; most of it is safety-checked in the Debug and
ReleaseSafe modes and unchecked in ReleaseFast and ReleaseSmall,
`@setRuntimeSafety` toggles the checks per scope, unchecked illegal
behaviour at comptime is a compile error, and undefined memory is filled
with `0xaa` in checked modes ([zig.md](../languages/zig.md), [39]); the
reference renamed "undefined behavior" to "illegal behavior" in February
2025 [39]. Reinterpretation is explicit (`@ptrCast`, `@ptrFromInt`,
`@alignCast`), and allocators are values (`std.mem.Allocator`) whose
debug implementation catches use-after-free and double free at run time
[39], [40]. One assessment: Zig "removes some of the most egregious
footguns from c" but "does not nearly approach the level of systematic
prevention of memory unsafety that rust achieves" [40]. Allocators as
values are worth taking; safety that depends on a build flag is not
(D-038's profiles have no unchecked mode).

### Odin and Nim

Odin has `transmute` (a bit cast between types of the same size),
`rawptr`, multi-pointers `[^]T` for arithmetic (a plain `^T` has none),
per-block `#no_bounds_check` and an implicit `context` that carries the
allocator [41]; "Odin will allow the programmer to do unsafe things if
they so wish" ([odin-jai-c3.md](../languages/odin-jai-c3.md)). Nim's
manual says untraced `ptr` references "are _unsafe_" but "for certain
low-level operations (accessing the hardware) untraced references are
unavoidable", that "the usage of addr is an _unsafe_ feature", and that
type casts "are only needed for low-level programming and are inherently
unsafe" [42], yet none of them is marked where it is used, and Nim's
ARC/ORC releases kept fixing use-after-free in the compiler's own move
inference ([nim.md](../languages/nim.md)). A feature documented as unsafe
but unmarked at its uses cannot be audited.

### D

D marks functions: "@safe functions have a number of restrictions"
(no casts that break the type system, no pointer arithmetic, no address
of a local); "@trusted functions must have a safe interface, as they can
be called from @safe functions. Internally they have all the
capabilities of @system functions"; and `@system`, the default, "may not
be called directly from @safe functions" [43]. DIP 1028, which would have
made `@safe` the default, was accepted and then reversed because bare
`extern(C)` declarations would have become callable from safe code, and
the idiom of wrapping pointer code in `@trusted` lambdas "defeats the
purpose" ([d.md](../languages/d.md), [43]). Two lessons: the trusted
function with a safe interface is the right unit, and a foreign
declaration must never be implicitly safe.

### Ada and SPARK

Ada names every unchecked operation (`Ada.Unchecked_Conversion`,
`Ada.Unchecked_Deallocation`, `'Unchecked_Access`), which makes them a
grep away, and a program can forbid them outright with
`pragma Restrictions (No_Dependence => Ada.Unchecked_Conversion)` and
`No_Unchecked_Access` [44]. SPARK excludes them and proves the absence of run-time errors in
what remains; `SPARK_Mode => Off` marks the bodies the prover does not
analyse, which is SPARK's trust boundary, and general access types and
cycles are pushed into that unsafe subset
([ada-spark.md](../languages/ada-spark.md)). Restrictions per program and
proof per body are the strong end of the spectrum.

### Modula-3 and Oberon

Modula-3 put the boundary on modules: "The keyword UNSAFE can precede the
declaration of any interface or module to indicate that it is unsafe",
and `LOOPHOLE`, `ADR`, address arithmetic, `DISPOSE` and untraced
references to traced types are "forbidden in safe interfaces and
modules". "It is a static error for a safe interface to import an unsafe
one" or "for a safe module to import or export an unsafe interface", and
"If any of the modules that export a safe interface are unsafe, it is the
programmer, rather than the compiler, who makes the guarantee" [45]. That
is the safe-wrapper pattern written into the module system in 1989.
Oberon kept low-level operations in a pseudo-module, `SYSTEM` (`ADR`,
`SIZE`, `BIT`, `GET`, `PUT`, `COPY`), because "they may corrupt a system
(e.g. PUT)"; Wirth recommends restricting them "to specific low-level
modules, as such modules are inherently non-portable and not 'type-safe'.
However, they are easily recognized due to the identifier SYSTEM
appearing in the module's import lists" [46].

### Austral

"An **unsafe module** is a module that can access FFI features": it may
import from `Austral.Memory`, which "contains types and functions for
manipulating pointers and memory", and use the `External_Name` pragma,
and it says so with `pragma Unsafe_Module`. The rationale is the audit:
"a list of unsafe modules in all dependencies (including transitive ones)
can be collected by the build system", and "only code at the FFI boundary
needs to be audited" [47]. Its author records that unsafe modules and
capabilities are not unified, and that the stricter design with an
`Unsafe` capability was "much more tedious to use"
([austral.md](../languages/austral.md)).

### Vale and Hylo

Vale's Fearless FFI (June 2022) separates safe and unsafe memory: C code
runs on a separate stack (prototyped on macOS), receives references
scrambled with compile-time constants that Vale generation-checks when
they come back, gets immutable data by copy, and would be allowed per
dependency with a whitelist flag and optionally sandboxed through wasm2c
or a subprocess; copying and scrambling were implemented, the rest
planned or in progress [48]. Its successor Valen "includes unsafe code
like Rust" ([vale.md](../languages/vale.md)). Hylo offers unsafe
constructs by "explicit, auditable opt-in", with pointer types in the
standard library built on `Builtin.Pointer`, intended for writing safe
abstractions ([hylo.md](../languages/hylo.md), [49]). Both value-semantics
designs end where Sisu does: a small unsafe layer in the library, and
safe types above it.

### Pony

Using the C FFI means "basically declaring that you trust the C code
that's being executed", and `ponyc --safe=files:net:process` lets only
the listed packages make FFI calls: "Any other packages that try to use
C-FFI calls will result in a compile-time error" [50]. Authority is an
object capability (`AmbientAuth`) passed from `main`, the same split as
Sisu's: capabilities for authority, a build-level list for trust.

### Cyclone

Cyclone, a safe dialect of C, has no `unsafe` escape: fat pointers carry
bounds, arithmetic is only allowed on them, casts are checked, and C
functions are called with Cyclone types and inserted null checks; its
garbage collector is the Boehm collector, in C, and its overhead runs
from near zero for I/O-bound programs to 2.85 times C on one compute
benchmark [51]. Porting C to Cyclone changed about 8% of the code [51]. A
language with no escape hatch still runs on a trusted runtime written in
another language, which is the situation Sisu wants to leave.

### Haskell and OCaml

`unsafePerformIO` needs a `NOINLINE` pragma ("If the call is inlined, the
I/O may be performed more than once") and breaks type safety: "it is
possible to write `coerce :: a -> b` with the help of
`unsafePerformIO`" [53]; bytestring's `accursedUnutterablePerformIO` "is
in fact a malevolent agent of chaos" [53], a name doing a type's job.
Safe Haskell (GHC 7.2, 2011) marks modules `Safe`, `Trustworthy` or
`Unsafe`; with `-fpackage-trust` a `Trustworthy` module counts only if its
package is trusted by the user building the program, and that trust can
be withdrawn when a package is found wanting [52]. It is the closest
precedent for Sisu's `[trust]`, and its outcome is a warning: in a 2022
discussion on deprecating it, maintainers said "the Safe ecosystem failed
to materialise", partly because many forms of deriving could not be used
in a `Safe` module, so library authors ignored it [52]. OCaml's `Obj` is
"Not for the casual user" and `Obj.magic : 'a -> 'b` has no further
documentation; `-unsafe` turns bounds checking off and then "anything can
happen" [54].

### Java and .NET foreign memory

JEP 471 (JDK 23) deprecates the memory-access methods of
`sun.misc.Unsafe` (79 of its 87 methods) for removal, because "They can
lead to undefined behavior, including JVM crashes", in favour of
`VarHandle` and the FFM API, which "guarantee no undefined behavior",
with warnings by JDK 25 and exceptions after [55]. The FFM API (JEP 454,
JDK 22) gives memory segments spatial and temporal bounds, and arenas
(global, automatic, confined, shared) that decide deallocation, so that
"a memory segment cannot be accessed after its backing region of memory
is deallocated"; what it cannot check (a downcall's signature, a
user-asserted size through `reinterpret`) are *restricted methods* [55].
JEP 472 (JDK 24) then makes JNI follow the same rule: loading native code
warns unless the application grants the module native access with
`--enable-native-access`, under the principle that the application
developer must approve operations that can break the platform's integrity
[55]. .NET's `Span<T>` and `ref struct` remove most of the reasons C#
code used pointers ([csharp.md](../languages/csharp.md)). A checked API
covers most raw-memory needs; what stays unchecked is granted by the
application, per module.

### Kernels and runtimes in safe languages

Tock never allowed unsafe language features in its drivers (capsules);
unsafe is confined to the few places that touch hardware [56]. Asterinas
(USENIX ATC 2025) confines all unsafe Rust to a framework, OSTD, and
reports a trusted computing base of 14.0% of its code, against 43.8% for
Tock, 62.4% for Theseus and 66.1% for RedLeaf, with a retrofitted Miri
(KernMiri) checking the framework [57]. In Singularity the kernel and the
runtime libraries were the only parts with unsafe code, and most of
their code was safe C# [58]. A runtime or a kernel can keep its unsafe
code to one framework, provided the framework offers the abstractions
(stacks, frames, raw memory, devices) that everything above it needs.

## What the research literature says

- **Soundness can be proved, and proving it finds bugs.** RustBelt (POPL
  2018) gives a machine-checked semantic soundness proof for a realistic
  subset of Rust, in which each library that uses unsafe features must
  meet a verification condition to count as a safe extension [14]. While
  proving `Mutex` sound, Ralf Jung found that `MutexGuard<T>` was `Sync`
  whenever `T` was `Send`, a data race reachable from safe code
  (CVE-2017-20004), fixed in Rust 1.19 [14].
- **The aliasing model is the hard part.** Tree Borrows (PLDI 2025)
  replaces Stacked Borrows' per-location stack with a tree; on the 30,000
  most widely used crates it rejects 54% fewer test cases, and its proofs
  keep most optimisations and add read-read reorderings [15].
- **An interpreter can check unsafe code.** Miri (POPL 2026) interprets
  MIR as an implementation of Rust's abstract machine, tracking pointer
  provenance, validating type invariants, detecting data races and
  exploring weak memory. Of 100,778 crates, 59,136 had runnable tests:
  51.8% passed fully, 7.8% showed undefined behaviour, and 21.7% hit
  operations Miri does not support, mostly calls into C libraries; 70.3%
  of 1.57 million tests passed. It runs 3,000 to 7,000 times slower than
  unoptimised native code, is part of the standard library's CI, and has
  found uninitialised reads, unaligned reads in `rand`, a violated
  `posix_memalign` precondition in the standard library, out-of-bounds
  pointer arithmetic in TiKV and an alignment mismatch at deallocation in
  rkyv [16].
- **The bugs have a few shapes.** Rudra (SOSP 2021) scanned 43,000
  crates in 6.5 hours and found 264 previously unknown memory-safety bugs
  (76 CVEs, 112 advisories, two in the standard library) in three
  patterns: panic safety, higher-order invariants (unsafe code trusting a
  trait implementation it does not control) and `Send`/`Sync` variance
  (manual impls missing a bound) [17]. Xu et al. classify 186 memory-safety
  bugs, among them every Rust CVE to the end of 2020: every one needs
  unsafe code,
  and they fall into automatic memory reclaim (a drop running on
  uninitialised or freed memory), unsound functions (a safe wrapper with
  an unchecked precondition) and unsound generic or trait bounds [18].
  Qin et al. (PLDI 2020) find that all memory-safety bugs in their corpus
  involve unsafe code, as do all non-blocking thread-safety bugs [19]
  ([studies.md](../evidence/studies.md)).
- **Unsafe is rare per package and common transitively.** Fewer than 30%
  of Rust libraries use `unsafe`, but more than half cannot be checked
  entirely because of unsafe somewhere in their call chain [20]; the
  Rust Foundation counts 19.11% of 127,000 crates using `unsafe` and
  34.35% calling directly into a crate that does, mostly to reach C or
  C++ [22]. "Most unsafe code is simple and well-encapsulated", but
  unsafe features "are used extensively, especially for interoperability
  with other languages" [21]. Developers reach for it "when they
  perceived that there was no alternative", and find foreign aliasing and
  concurrency patterns hard to encapsulate [23]. The Go figures above
  (24%, 38%, 91%) say the same [37], [38].
- **Safety documentation drifts unless it is structured.** Cui et al.
  (ICSE 2024) define 19 safety properties, label 416 unsafe APIs of the
  Rust standard library and find its safety documentation inconsistent and
  incomplete [24]; a 2025 paper proposes tags in five categories (layout,
  pointer validity, content, alias, miscellaneous) that cover 96.1% of the
  standard library's unsafe APIs, intrinsics excluded, and a Rust pre-RFC
  proposes `#[safety]` attributes that a linter can check against call
  sites [25].
- **Verifying a standard library is a long road.** The Rust Foundation
  and AWS contest (November 2024 on) produced 725 manual Kani harnesses,
  694 of them backed by function contracts, and more than 50 VeriFast
  proofs; manual work plateaued at a small fraction of the library, and
  automatic harness generation brought the strongest guarantees to 989
  functions [26].

## What the evidence says

1. **A hard boundary from the first version works; a lint or an opt-in
   does not get adopted.** Rust's boundary is respected by an ecosystem of
   tools because it is a rule [9]-[13]; Swift's strict mode is a warning
   that its vision does not expect to become the default [28], [29]; D's
   safe default came too late (DIP 1028) and Safe Haskell never found an
   ecosystem [43], [52]. Sisu's `unsafe` must be a compile error outside
   its form, everywhere, from 0.1.
2. **The permission belongs on operations, not types.** C# 15 moved
   `unsafe` from pointer types to the operations that access memory [31];
   Rust always let safe code create raw pointers [1]. Creating, copying and
   comparing a raw pointer harms nothing; reading, writing and deriving a
   reference from one is what needs the block.
3. **A function's obligation and a block's permission are different
   things.** Rust 2024 split them [4]; C# 15's *requires-unsafe* members
   carry only the obligation [31]; D's `@trusted` is the inverse marker
   [43]. An obligation marker that also opened its body to unsafe
   operations hid them, which is why Rust changed it.
4. **Foreign declarations are never implicitly safe.** D's DIP 1028
   failed on exactly this [43]; Rust 2024 requires `unsafe extern` [4];
   Java restricts downcalls whose signatures it cannot check [55].
5. **Unsafe is small per package and transitive, and most of it is
   foreign interop** [20]-[22], [37], [38]. The audit therefore has to be
   per package and transitive, and binding a C library deserves first-class
   support: a declaration in the manifest, a wrapper idiom, and a separate
   line in the audit.
6. **The soundness bugs have known shapes: panic safety, trusting
   supplied safe code, auto-trait impls, unsound wrappers, drops of
   uninitialised memory** [2], [3], [14], [17], [18]. A contract that names
   them helps review, and two of them can be linted (an `unsafe impl Send`
   whose generic parameters lack a bound; an undocumented block).
7. **Dynamic checking finds what review misses.** Miri runs 70% of the
   ecosystem's tests and has found real bugs in the standard library [16];
   Go's checkptr catches pattern violations in tests [33]; Asterinas built
   KernMiri for its framework [57]. Sisu already has an interpreter over a
   MIR-like SIR with a run-time ownership monitor; that is where such a
   checker belongs.
8. **The common unsafe patterns get safe APIs.** Go added `unsafe.String`
   and friends for its commonest conversions [32]; Rust added strict
   provenance [5]; Java's FFM and Swift's `Span` replaced raw pointers with
   checked views [30], [55]. Reinterpreting bytes as plain data and
   building a growable buffer are the two patterns most worth a safe
   library type.
9. **Integer-to-pointer casts break analysis, provenance and CHERI** [5],
   [6]. A new language can have strict provenance from the first day.
10. **A small trusted base is achievable for kernels and runtimes** [27],
    [56]-[58], and a self-hosted runtime needs a handful of primitives
    outside the ordinary language: a system stack, functions that must not
    be preempted or grow the stack, a context switch, memory outside the
    managed heap [35]. Escape hatches that are reachable get used beyond
    their purpose, as Go's `linkname` was [36].
11. **The permission to be trusted is granted by whoever builds the
    program, per package.** Pony's `--safe` [50], Java's
    `--enable-native-access` [55], Safe Haskell's package trust [52],
    Austral's unsafe-module list [47], C#'s `AllowUnsafeBlocks` [31],
    Cargo's `[lints]` [9] and Tock's capsules [56] are seven versions of
    one idea, which Sisu's `[trust]` already is.
12. **Structured safety comments are checkable; free text drifts** [11],
    [24], [25].

## What Sisu's runtime and backend need

`sisu-rt` is 12,340 lines of Rust in 19 files with 253 `unsafe` blocks,
150 `unsafe fn`s, 31 entry points and 4 statics exported to the generated
code (`#[unsafe(no_mangle)]`), 6 transmutes of addresses to function types
and 2 `unsafe impl`s, each block with a `SAFETY:` comment (architecture
decision 187 of [`compiler/docs/architecture.md`](../../compiler/docs/architecture.md)).
It depends on corosensei, whose context switch is inline assembly per
architecture [60], on `libc` for `mmap`, `madvise` and `sysconf`, and on
Rust's standard library for `malloc`, sockets, files, processes,
thread-locals, hash maps and locks, none of which the 253 blocks count.
Sorted by what they do:

| Kind | Where (blocks) | What it is | In a Sisu runtime | Unsafe left |
|---|---|---|---|---|
| The ABI boundary | `entry.rs` (35), `intrinsics.rs` (37), `tasks.rs` (most of 61), `value.rs` (25), `dict.rs` (12), `caps.rs` (10), `iter.rs` (9), `host.rs` (7), `bytes.rs`, `sized.rs`, `wide.rs`, `views.rs` (11) | reading and writing values at raw addresses by the type table: arguments, results, the structural drop, copy, equality, order, hash, `Display` and `Debug` | the runtime is compiled with the program: its functions take typed values, the structural operations are generic functions over comptime reflection (as `derive` is), and the type table and the C entry points go | none |
| Raw heap memory | `mem.rs` (13), `cells.rs` (20), part of `caps.rs` | `malloc`, `realloc`, `free`, `memmove`; list and text buffers; `Shared` and `Weak` headers at negative offsets; capability cells with counts | a global allocator over a foreign allocator or `mmap`; a typed raw buffer for `list`, `String`, `dict` and `Deque` storage; `Box`, `Shared` and `Weak` cells as typed header structs; capability handles as ordinary counted cells | about 15 |
| Stacks and switching | `stack.rs` (5), `sched.rs` (7) | the pooled stacks (`mmap`, `madvise`, `sysconf`); corosensei's `suspend`, `resume`, `on_parent_stack`, `force_reset`; the `Stack` trait impl; an `AssertSend` wrapper | the pool over foreign calls; one context-switch primitive; the scheduler, channels, mutexes, generators and scopes as ordinary Sisu on top | about 8 |
| Code addresses | 6 transmutes | drop glue from a function table, `main`'s entry, task bodies | typed function values; only the process entry is called from C | 1 |
| Host calls | through Rust's standard library today | stdout, files, sockets, processes, clock, random bits, exit | foreign declarations of the C library's functions, one wrapper each, behind the capability objects | about 25 |

The estimate, about 50 blocks against 253, is a sorting of the existing
blocks by whether their operation has a safe expression once runtime and
program are one Sisu program; it is an estimate, not a port. What it shows
is where the unsafe code would be: half of it in one module of foreign
bindings, the rest in raw memory, cells and the switch, with the trusted
base visible in one language rather than spread over Rust's standard
library and an assembly crate.

The operations the task list names, against that runtime:

| Operation | Needed for | Sisu form |
|---|---|---|
| Allocation and release | every heap type | a global allocator in `std.mem`; allocation is safe, release is not |
| Pointer arithmetic | buffers, cells | `p.offset(n)`, unsafe, within one allocation; typed header structs instead of negative offsets |
| Uninitialised memory | spare capacity, ring buffers | only behind raw pointers; reading it is undefined, and the interpreter checks |
| Stack switching | tasks, generators, the system stack | one machine intrinsic that saves and loads a `Context` (Cranelift's `stack_switch` on x86-64 Linux [59], the backend's own sequence elsewhere) |
| Atomics | `Shared` counts, the scheduler's flags | `Atomic[T]` fields in typed headers, already safe; an unsafe constructor over raw memory |
| Volatile and memory-mapped I/O | `freestanding` drivers only | unsafe volatile reads and writes, and a pointer from an address |
| Foreign calls | every host effect | `extern "C"` declarations, unsafe to call, `@blocking` when they can block |
| Inline assembly | the switch, reading the stack pointer, the thread slot (taskbench's probe uses `asm!` for both) | none: the same three needs as machine intrinsics |
| Reinterpretation | bytes as plain values; addresses as functions | a safe bit cast for plain data; an unsafe transmute; no address-to-function conversion outside the process entry |
| Manual drops | values moved aside, cells | an unsafe drop in place, typed |
| Per-thread runtime state | panics in flight, the yield budget, stack limits (today `thread_local!` and four `no_mangle` statics) | one word of thread-local storage read through an intrinsic, pointing at the thread's runtime value; no mutable globals |
| Entry from C | `main`, `argv` | one unsafe block in the runtime's start |

The backend needs nothing beyond ordinary code. A code generator, a
register allocator, an object and DWARF writer and a linker driver
(through `Proc`) are byte-buffer programs. The interpreter, rewritten in
Sisu, holds values as Sisu enums, and even a checked memory model for
unsafe code (below) is safe code over byte arrays. The one exception
would be native hot replacement (D-053) in an in-process JIT, which maps
executable memory and calls through code addresses; it can wait, since
hot replacement runs in the interpreter today.

## The soundness contract, given Sisu's model

Rust's contract carries over: safe code cannot cause undefined behaviour
whatever it does, and an `unsafe` block is its author's promise that the
operations inside keep that true for every input, including every safe
code its callers supply [2]. Sisu's model changes it in five places.

- **References are second-class, so the unbounded lifetime cannot
  happen.** A reference made from a raw pointer can be bound to a local,
  passed down, or returned tied to a borrowed parameter; it cannot be
  stored in a struct, so its worst case is the frame or the declared
  `from` parameter, never "as big as context demands" [3]. The obligation
  is that the memory stays valid, and the aliasing rule holds, for that
  origin.
- **Views, scopes and suspended frames are never leaked (D-058), so
  unsafe code inherits the obligation.** Completion I/O lends a caller's
  buffer to the kernel on the strength of that rule; unsafe code that
  moved a view into raw memory and never dropped it would break every
  safe caller. The same holds for `@must_consume` values (D-054).
- **Capabilities are the only authority, so unsafe code that reaches the
  world must not hand authority out for free.** A foreign call that
  affects the world must be reachable only from code that holds the
  capability for that authority; a trusted wrapper that turned `open(2)`
  into a safe function taking no capability would silently restore
  ambient authority. Rust has no such clause because it has no
  capabilities; Austral notes the gap as its "faultline"
  ([austral.md](../languages/austral.md)).
- **`pure`, comptime, replay and simulation are deterministic, so unsafe
  code may not let observable behaviour depend on an address.** D-074
  already makes address-derived values errors in `pure` code; unsafe code
  must not reintroduce them (alignment arithmetic on an aligned base is
  the exception, since it gives the same result in every run).
- **Contracts can be compiled out ([07](../../proposal/07-contracts.md)),
  so soundness never rests on one.** Unsafe code may rely on invariants
  that its own module maintains through privacy, never on another
  function's `ensures` or on a check that a release profile may drop; the
  Nomicon's `BTreeMap` rule [2] extends to contracts.

The task model adds three obligations that already exist in safe code
and become the unsafe author's: nothing crosses `spawn` that is not
`Send` or is shared that is not `Sync`, so an `unsafe impl Send` is a
promise about every instance (the RustBelt bug [14] and Rudra's third
pattern [17]); a raw pointer into a task's stack stays valid while the
frame lives, even while the task is parked, because Sisu never moves
stacks and decommits only below the parked stack pointer (architecture
decision 203); and code that must not be interleaved with another task on
its thread is `@no_block`, which suppresses yield points (D-059), as Go's
system stack is non-preemptible [35]. A foreign call that can block must
say `@blocking`, or it stalls the scheduler.

## Interactions with the rest of Sisu

- **The dynamic monitor.** The interpreter already gives every borrow a
  run-time loan and checks each access against the loan tree, the tree
  Tree Borrows formalises [15]. Extending it to raw memory is Miri's
  design [16]: an allocation is a block with an identity, a raw pointer is
  a block, an offset and the tag of the loan it came from, and every read
  checks liveness, bounds, alignment, initialisation and the validity of
  the bits for the type read, while writes through a pointer derived from
  a shared loan, or any access that conflicts with a live `mut` loan, are
  violations. Since SIR is already the MIR Miri interprets, this is an
  extension of the interpreter, not a new tool; its cost falls only on raw
  operations, and Miri's 3,000-7,000-times figure [16] is an upper bound
  for code that is all raw operations. Unwinding is ordinary control flow
  in both the interpreter and native code, so the interpreter can also
  force a panic at each potential panic site of a function that contains
  unsafe code and check the drops that follow, which tests Rudra's first
  pattern [17] mechanically.
- **The simulation profile.** A `sim` build rejects reachable code that
  could escape the simulation ([17](../../proposal/17-testing.md)). Raw
  memory operations cannot escape it; foreign calls and the runtime's
  machine intrinsics can. Separating the two in the manifest (below) makes
  the check exact, and stage 0's check already looks only at foreign calls
  (`compiler/crates/sisu-driver/src/sim.rs`).
- **`pure`.** Today's text says a `pure` function "contains no `unsafe`"
  ([06](../../proposal/06-concurrency-and-capabilities.md)). Read
  literally, that makes every function built on a standard collection
  implemented in Sisu impure. The rule that keeps the property is about
  operations: raw memory operations touch only memory reachable from the
  function's arguments or allocated by it, which the contract requires,
  and are pure; foreign calls, machine intrinsics and address observation
  are not. A `pure` function whose unsafe block makes an impure call
  asserts observational purity, which is what N-63 asks a trusted package
  to declare for `std.x.incr`'s caches, so one existing form covers it.
- **Comptime.** [08](../../proposal/08-metaprogramming.md) evaluates
  functions with "no capabilities, no `unsafe`, and no `Shared`". The
  standard library's collections, written in Sisu over raw memory, must
  still work at compile time; Rust's const evaluation runs raw pointer code
  in its own checked machine. The interpreter's checked model is the
  same: comptime runs raw memory operations in it, and stops with an
  error at a foreign call, a machine intrinsic, an address observation or
  a raw pointer in a constant's value. Code produced by comptime (a
  `derive`) should contain no `unsafe` at all, so every block stays
  visible in a source file for the audit.
- **`Escapable`, `Send`, `Sync`.** A raw pointer to `T` should be
  `Escapable` exactly when `T` is, so that a buffer of views built on raw
  pointers is itself a view without a phantom marker; `Escapable` must
  never be asserted, since D-058 rests on it. Raw pointers should be
  neither `Send` nor `Sync`, as in Rust and for Rust's reason [3]; a
  standard raw buffer that asserts both with the right bounds gives every
  container built on it the right auto traits structurally, which removes
  most of the manual impls in which Rudra found its third pattern [17].
- **Hot replacement and the teaching profile.** A body that relies on
  its module's invariants should not be swapped under live frames, so a
  module containing `unsafe` is not hot-replaced (D-068); the teaching
  profile excludes `unsafe` at every level (D-073).

## Keeping it minimal

- **One form permits.** The `unsafe:` block (and `unsafe e` for one
  expression, already in [02](../../proposal/02-syntax.md)) is the only
  place an unsafe operation can occur. `unsafe def` (callers take an
  obligation) and `unsafe impl` of an `unsafe trait` (the implementor
  takes one) permit nothing in their bodies, which is Rust 2024's lesson
  made a rule from the start [4] and C# 15's *requires-unsafe* [31].
  Without an obligation marker a helper with a precondition is either
  unsound or duplicated, so the alternative to `unsafe def` is Modula-3's
  module granularity [45], which makes every line of the module the audit
  unit.
- **The operations are library functions, not syntax.** Reading,
  writing, offsetting and dereferencing a raw pointer, allocating raw
  memory, transmuting and the machine intrinsics are `unsafe def`s in the
  standard library with intrinsic bodies, as Oberon's `SYSTEM` procedures
  [46] and Go's `unsafe` functions [32] are; the language adds no
  dereference operator, no address-of syntax and no pointer arithmetic
  operators.
- **Safe wrappers come with the library.** A typed raw buffer with
  bounds-checked slots leaves its owner only the initialisation
  obligations; a plain-data marker gives safe reinterpretation of bytes;
  views from raw parts and C strings cover foreign data. Go's 1.17 and
  1.20 additions and Java's FFM [32], [55] are the model.
- **A declaration per package, consent per program.** The package says
  in its manifest that it contains unsafe code and at what level (raw
  memory only, or also foreign calls); the root's `[trust]` consents; the
  lockfile records the level, so an update that raises it is an authority
  delta that `sisu update` refuses without a flag. The level is not a
  security boundary (raw memory can be made to do anything by a malicious
  author), it is a review signal and the simulation profile's input.
- **An audit that lists every block with its reason.** `sisu audit`
  groups blocks by module (the Nomicon's point that unsafe code relying on
  a field's invariant "pollutes a whole module" makes the module the review
  unit), prints each block's safety comment, checks that the comment names
  every obligation of every unsafe callee (the tags of [25] reduced to the
  names in each callee's documentation), and hashes the module for the
  review ledger (N-43), as cargo-vet records audits as deltas [13].
- **Lints**: an `unsafe` block with no unsafe operation (Rust's
  `unused_unsafe`); a block, `unsafe def` or `unsafe impl` without its
  comment [10], [11]; a declared level above what the package uses; an
  `unsafe impl Send` or `Sync` on a generic type whose parameters lack the
  bound (Rudra's check [17]); and a public safe function that reaches a
  foreign call without taking a capability.
- **What is left out**: inline assembly (three intrinsics cover the
  runtime's needs), mutable globals (one thread slot covers the runtime's
  per-thread state), integer-to-pointer casts (strict provenance [5]),
  unions, unsafe fields [8] (privacy and module review instead, for now),
  a global switch that turns checks off (OCaml's `-unsafe` [54], Zig's
  ReleaseFast [39]), and a safe `forget`.

## The capability angle

Should `unsafe` be a capability, a value handed down from `main` like
`Fs`, or a manifest permission? Austral tried both and called the
capability "much more tedious to use"
([austral.md](../languages/austral.md)); the analysis for Sisu goes
further:

1. **It is not authority over the world.** A `list` growing its buffer
   needs raw memory and touches nothing outside the process. A capability
   parameter for it would travel from `main` to every function that
   pushes to a list, which is the colouring problem in a new place.
2. **It cannot be attenuated, revoked or confined.** Code that can write
   arbitrary memory can forge any capability object, read any other, or
   call the operating system through a corrupted function pointer. An
   `Unsafe` value would be all-or-nothing, which is the shape of a
   permission, not a capability (E's and Austral's capabilities are
   meaningful because they can be narrowed;
   [capabilities-and-effects.md](capabilities-and-effects.md)).
3. **The decision belongs to whoever builds the program, and it is made
   once, at build time.** That is Pony's `--safe`, Java's
   `--enable-native-access`, Safe Haskell's package trust and Sisu's
   `[trust]` [50], [55], [52]; a run-time value adds nothing a build-time
   list does not already decide.

What is a capability matter is what trusted code does with foreign
calls. The runtime turns them into the root capabilities (`System`), and
a trusted wrapper that performs I/O must take the capability whose
authority it exercises, so that from safe code the capability is still
the only way to the world. That clause can be linted (a public safe
function that reaches a foreign call and takes no capability) but not
proved, and the audit lists the foreign calls of each package separately
for that reason.

## What Sisu should take

- **Adopt**: Rust's boundary and contract (operations need the block,
  creating pointers is safe, unsafe code must not trust supplied safe
  code, minimal panic safety) [1]-[3]; Rust 2024's split of permission and
  obligation and its unsafe foreign declarations [4]; strict provenance
  from the first day [5]; per-block safety comments and per-function
  safety sections [10], [11]; raw pointers that are neither `Send` nor
  `Sync` [3]; Java's and Pony's per-package grant by the program's builder
  [50], [55] as the existing `[trust]`; Miri's interpreter-based checking
  [16] as an extension of the existing monitor; Go's runtime structure (a
  system stack, non-preemptible runtime code, unmanaged memory rules) [35]
  through existing attributes (`@no_block`, `@no_alloc`, `@stack`); Zig's
  allocators as values [39] (already D-060).
- **Adapt**: Modula-3's safe interface over an unsafe implementation [45]
  as the audit's grouping by module rather than as module-level
  permission; safety tags [25] as obligation names checked by a lint
  rather than a new attribute language; Swift's `Span` and Java's segments
  [30], [55] as a typed raw buffer and views from raw parts in the
  standard library; Go's dedicated conversion functions [32] as a
  plain-data marker with a safe bit cast; Oberon's and Go's
  operations-as-library-functions [32], [46] inside Rust's block
  discipline.
- **Avoid**: unsafe as a lint or an opt-in mode (Swift [29]); safety by
  build mode (Zig [39], OCaml [54]); foreign declarations that are safe
  by default (D [43]); unsafe function bodies that permit unsafe
  operations (Rust before 2024 [4]); integer-to-pointer casts [6]; a safe
  `forget` that breaks scoped borrowing (Rust 2015 [7]); runtime-only
  escape hatches that other packages can reach (Go's `linkname` [36]); a
  safe subset too small for everyday code, which made Safe Haskell's
  ecosystem fail to materialise [52]; an `Unsafe` capability (Austral).

The design that follows is [proposal/22](../../proposal/22-unsafe.md),
decisions D-080 to D-086.

## Open questions

- Q-54: whether a closed set of machine intrinsics is enough, or Sisu
  needs inline assembly.
- Q-55: how the interpreter runs unsafe code and foreign calls, and what
  the checked memory model may cost.
- Q-56: which aliasing rules raw pointers follow.
- Q-57: whether the runtime's machine layer becomes public for
  `freestanding` programs.
- Q-58: whether safe reinterpretation of plain data ships in version 1,
  and as a derive or an inferred trait.
- Q-59: whether safety obligations are checked names, and whether
  fields can carry them.

## Sources

1. The Rust Reference, "Unsafety". https://doc.rust-lang.org/reference/unsafety.html (2026)
2. The Rustonomicon, "How Safe and Unsafe Interact". https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html (2026)
3. The Rustonomicon, "Send and Sync", "Unbounded Lifetimes", "Exception Safety", "Leaking". https://doc.rust-lang.org/nomicon/send-and-sync.html ; https://doc.rust-lang.org/nomicon/unbounded-lifetimes.html ; https://doc.rust-lang.org/nomicon/exception-safety.html ; https://doc.rust-lang.org/nomicon/leaking.html (2026)
4. The Rust Edition Guide, Rust 2024: "`unsafe_op_in_unsafe_fn` warning", "Unsafe `extern` blocks", "Unsafe attributes". https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html ; https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-extern.html ; https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html (February 2025)
5. Rust Blog, "Announcing Rust 1.84.0" (strict provenance APIs). https://blog.rust-lang.org/2025/01/09/Rust-1.84.0/ (9 January 2025)
6. A. Beingessner, "Rust's Unsafe Pointer Types Need An Overhaul". https://faultlore.com/blah/fix-rust-pointers/ (19 March 2022)
7. RFC 1066, "Alter `mem::forget` to be safe". https://rust-lang.github.io/rfcs/1066-safe-mem-forget.html (April 2015)
8. Rust project goals, "Stabilize unsafe fields" (RFC 3458). https://rust-lang.github.io/rust-project-goals/2026/unsafe-fields.html (2026)
9. The Cargo Book, "The `[lints]` section" (respected as of 1.74); Rust Blog, "Announcing Rust 1.74.0". https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section ; https://blog.rust-lang.org/2023/11/16/Rust-1.74.0/ (16 November 2023)
10. Clippy lint list: `undocumented_unsafe_blocks`, `multiple_unsafe_ops_per_block`, `missing_safety_doc`. https://rust-lang.github.io/rust-clippy/master/index.html (2026)
11. The Linux kernel documentation, "Rust: Coding Guidelines" (comments, `SAFETY:`, `# Safety`). https://docs.kernel.org/rust/coding-guidelines.html (2026)
12. cargo-geiger README (0.13.0). https://github.com/geiger-rs/cargo-geiger (2025)
13. Google, rust-crate-audits, "Auditing standards" (ub-risk-0 to ub-risk-4). https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md (2026)
14. Jung, Jourdan, Krebbers, Dreyer, "RustBelt: Securing the Foundations of the Rust Programming Language", POPL 2018. https://plv.mpi-sws.org/rustbelt/popl18/ ; R. Jung, "How to prove that MutexGuard is not Sync" (the `MutexGuard` bug). https://www.ralfj.de/blog/2017/06/09/mutexguard-sync.html (9 June 2017); rust-lang/rust#41622, CVE-2017-20004. https://github.com/rust-lang/rust/issues/41622
15. Villani, Hostert, Dreyer, Jung, "Tree Borrows", PLDI 2025 (distinguished paper). https://pldi25.sigplan.org/details/pldi-2025-papers/42/Tree-Borrows ; https://www.ralfj.de/blog/2025/07/07/tree-borrows-paper.html (7 July 2025)
16. Jung, Kimock, Poveda, Sánchez Muñoz, Scherer, Wang, "Miri: Practical Undefined Behavior Detection for Rust", PACMPL 10 (POPL 2026), article 48. https://research.ralfj.de/papers/2026-popl-miri.pdf (January 2026)
17. Bae, Kim, Askar, Lim, Kim, "Rudra: Finding Memory Safety Bugs in Rust at the Ecosystem Scale", SOSP 2021. https://github.com/sslab-gatech/Rudra ; https://cc.gatech.edu/news/new-bugs-identified-popular-programming-language (2021)
18. Xu, Chen, Sun, Zhou, Lyu, "Memory-Safety Challenge Considered Solved? An In-Depth Study with All Rust CVEs", ACM TOSEM. https://arxiv.org/abs/2003.03296 (2021, revised)
19. Qin, Chen, Yu, Song, Zhang, "Understanding Memory and Thread Safety Practices and Issues in Real-World Rust Programs", PLDI 2020. https://doi.org/10.1145/3385412.3386036 (2020); summarised in [`../evidence/studies.md`](../evidence/studies.md)
20. Evans, Campbell, Soffa, "Is Rust Used Safely by Software Developers?", ICSE 2020. https://arxiv.org/abs/2007.00752 (2020)
21. Astrauskas, Matheja, Poli, Müller, Summers, "How Do Programmers Use Unsafe Rust?", OOPSLA 2020 (PACMPL 4, article 136). https://doi.org/10.1145/3428204 (November 2020)
22. Rust Foundation, "Unsafe Rust in the Wild: Notes on the Current State of Unsafe Rust". https://rustfoundation.org/media/unsafe-rust-in-the-wild-notes-on-the-current-state-of-unsafe-rust/ (21 May 2024)
23. McCormack et al., "A Mixed Methods Study on the Implications of Unsafe Rust for Interoperation, Encapsulation, and Tooling" (FSE 2026 journal-first). https://arxiv.org/abs/2404.02230 (2024)
24. Cui et al., "Is unsafe an Achilles' Heel? A Comprehensive Study of Safety Requirements in Unsafe Rust Programming", ICSE 2024. https://arxiv.org/abs/2308.04785 (2023-2024)
25. "Annotating and Auditing the Safety Properties of Unsafe Rust", arXiv 2504.21312 (2025); Rust internals, "Pre-RFC: Safety Property System". https://arxiv.org/abs/2504.21312 ; https://internals.rust-lang.org/t/pre-rfc-safety-property-system/23252 (2025)
26. AWS Open Source Blog, "Verify the Safety of the Rust Standard Library" (November 2024), and the Rust Foundation's progress report on the contest (2026). https://aws.amazon.com/blogs/opensource/verify-the-safety-of-the-rust-standard-library/
27. J. Vander Stoep, "Rust in Android: move fast and fix things", Google. https://blog.google/security/rust-in-android-move-fast-fix-things/ (13 November 2025)
28. Swift Evolution SE-0458, "Opt-in Strict Memory Safety Checking" (implemented in Swift 6.2). https://github.com/swiftlang/swift-evolution/blob/main/proposals/0458-strict-memory-safety.md (2025)
29. Swift Evolution vision, "Optional Strict Memory Safety for Swift", and its acceptance by the Language Steering Group. https://github.com/swiftlang/swift-evolution/blob/main/visions/memory-safety.md ; https://forums.swift.org/t/accepted-vision-optional-strict-memory-safety-for-swift/77223 (15 January 2025)
30. Swift Evolution SE-0447, "`Span`: Safe Access to Contiguous Storage" (Swift 6.2). https://github.com/swiftlang/swift-evolution/blob/main/proposals/0447-span-access-shared-contiguous-storage.md (2024-2025)
31. Microsoft Learn, "What's new in C# 15" (memory safety preview), 19 October 2026; "C# Compiler Options: language feature rules" (`AllowUnsafeBlocks`, the updated memory safety rules), September 2026. https://learn.microsoft.com/en-us/dotnet/csharp/whats-new/csharp-15 ; https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/compiler-options/language
32. Go standard library, package `unsafe` (the six patterns; `Add` and `Slice` in 1.17; `SliceData`, `String`, `StringData` in 1.20). https://pkg.go.dev/unsafe (2026)
33. M. Dempsky, "cmd/compile: add -d=checkptr to validate unsafe.Pointer rules" (Go 1.14), and golang/go#34964. https://go.googlesource.com/go/+/80a6fedea05dbdab2e55b2ba922faeaf4155a981 ; https://golang.org/issue/34964 (2019-2020)
34. Go command documentation, "cgo: Passing pointers". https://pkg.go.dev/cmd/cgo (2026)
35. The Go runtime's `HACKING.md` (system stack, `nosplit`, runtime-only directives, unmanaged memory). https://go.googlesource.com/go/+/HEAD/src/runtime/HACKING.md (2026)
36. Go 1.23 release notes ("Linker": `//go:linkname`), August 2024, and golang/go#67401 on why; Go 1.11 release notes (`libSystem.dylib` on macOS and iOS), August 2018. https://go.dev/doc/go1.23 ; https://github.com/golang/go/issues/67401 ; https://go.dev/doc/go1.11
37. Costa, Mujahid, Abdalkareem, Shihab, "Breaking Type Safety in Go: An Empirical Study on the Usage of the unsafe Package", IEEE TSE. https://arxiv.org/abs/2006.09973 (2021)
38. Lauinger, Baumgärtner, Wickert, Mezini, "Uncovering the Hidden Dangers: Finding Unsafe Go Code in the Wild", TrustCom 2020. https://arxiv.org/abs/2010.11242 (2020)
39. Zig Language Reference (master): Illegal Behavior, Build Mode, `@setRuntimeSafety`, `@ptrCast`, `@ptrFromInt`, Memory. https://ziglang.org/documentation/master/ (2026); "langref: embrace the term illegal behavior" (22 February 2025)
40. J. Brandon, "How (memory) safe is zig?". https://www.scattered-thoughts.net/writing/how-safe-is-zig/ (2021, updated 21 September 2022)
41. Odin overview: `transmute`, `rawptr`, multi-pointers, `#no_bounds_check`, `context`. https://odin-lang.org/docs/overview/ (2026)
42. Nim Manual: "Reference and pointer types", "The addr operator", "Type casts". https://nim-lang.org/docs/manual.html (2026)
43. D Language Specification, "Memory-Safe-D-Spec" (`@safe`, `@trusted`, `@system`); DIP 1028, "Make @safe the Default" (rejected). https://dlang.org/spec/memory-safe-d.html ; https://github.com/dlang/DIPs/blob/master/DIPs/rejected/DIP1028.md (2019-2020, 2026)
44. Ada 2022 Reference Manual, 13.9 "Unchecked Type Conversions", 13.11.2 "Unchecked Storage Deallocation", 13.12.1 "Language-Defined Restrictions and Profiles", H.4 "High Integrity Restrictions"; SPARK User's Guide, "SPARK_Mode". http://www.ada-auth.org/standards/22rm/html/RM-13-9.html ; https://docs.adacore.com/spark2014-docs/html/ug/en/source/spark_mode.html (2023-2026); see also [`../languages/ada-spark.md`](../languages/ada-spark.md)
45. Modula-3 Reference (after Cardelli et al., "Modula-3 Report (revised)", DEC SRC Research Report 52, 1989): "Safety" and "Unsafe operations". https://modula3.github.io/cm3/reference/safety.html ; https://modula3.github.io/cm3/reference/unsafe.html
46. N. Wirth, "The Programming Language Oberon", revision 1.10.2013 / 3.5.2016, section 11.1 "The Module SYSTEM". https://people.inf.ethz.ch/wirth/Oberon/Oberon07.Report.pdf (2016)
47. The Austral Language Specification, "Unsafe Modules" and `Austral.Memory`. https://austral-lang.org/spec/spec.html (2026)
48. E. Ovadia, "Vale's Fearless FFI, for Safer Dependencies and Supply-Chain Attack Mitigation". https://verdagon.dev/blog/fearless-ffi (21 June 2022)
49. Hylo homepage and specification. https://www.hylo-lang.org/ ; https://github.com/hylo-lang/specification/blob/main/spec.md (2026)
50. Pony Tutorial, "Trust Boundary" (`ponyc --safe`). https://tutorial.ponylang.io/object-capabilities/trust-boundary (2026)
51. Jim, Morrisett, Grossman, Hicks, Cheney, Wang, "Cyclone: A safe dialect of C", USENIX ATC 2002; Grossman et al., "Region-Based Memory Management in Cyclone", PLDI 2002. https://www.usenix.org/event/usenix02/full_papers/jim/jim_html ; https://www.cs.cornell.edu/Projects/cyclone/papers/cyclone-regions-tr.pdf (2002)
52. Terei, Mazières, Marlow, Peyton Jones, "Safe Haskell", Haskell Symposium 2012; GHC User's Guide, "Safe Haskell"; "Deprecating Safe Haskell, or heavily investing in it?" (December 2022). https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/safe_haskell.html ; https://discourse.haskell.org/t/deprecating-safe-haskell-or-heavily-investing-in-it/5489
53. Haskell `base`, `System.IO.Unsafe`; `bytestring`, `Data.ByteString.Internal` (`accursedUnutterablePerformIO`). https://hackage-content.haskell.org/package/base-4.22.0.0/docs/System-IO-Unsafe.html ; https://hackage-content.haskell.org/package/bytestring-0.12.2.0/docs/Data-ByteString-Internal.html (2026)
54. The OCaml Manual: module `Obj`; "Native-code compilation" (`-unsafe`). https://ocaml.org/manual/latest/api/Obj.html ; https://ocaml.org/manual/latest/native.html (2026)
55. JEP 454, "Foreign Function & Memory API" (JDK 22, 2024); JEP 471, "Deprecate the Memory-Access Methods in sun.misc.Unsafe for Removal" (JDK 23, 2024); JEP 472, "Prepare to Restrict the Use of JNI" (JDK 24, 2025). https://openjdk.org/jeps/454 ; https://openjdk.org/jeps/471 ; https://openjdk.org/jeps/472
56. Levy et al., "Multiprogramming a 64 kB Computer Safely and Efficiently", SOSP 2017; Schuermann et al. on Tock's evolution, SOSP 2025. https://web.eecs.umich.edu/~prabal/pubs/papers/levy17tock.pdf ; https://www.cs.virginia.edu/~bjc8c/papers/schuermann25tock.pdf
57. Peng et al., "Asterinas: A Linux ABI-Compatible, Rust-Based Framekernel OS with a Small and Sound TCB", USENIX ATC 2025. https://www.usenix.org/conference/atc25/presentation/peng-yuke ; https://arxiv.org/abs/2506.03876 (2025)
58. Hunt, Larus et al., "Singularity: Rethinking the Software Stack", Operating Systems Review 41(2) (2007), and "The Singularity System", CACM (2010). https://cacm.acm.org/research/the-singularity-system
59. Cranelift, x64 `stack_switch` instruction ("only supported on x64 Linux at the moment"), Wasmtime documentation. https://docs.wasmtime.dev/api/src/cranelift_codegen/isa/x64/inst/stack_switch.rs.html (2026)
60. corosensei 0.3.4, per-architecture stack switching in inline assembly. https://docs.rs/crate/corosensei/latest (2025)
