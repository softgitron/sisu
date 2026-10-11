# Mojo

## Snapshot
| | |
|---|---|
| First appeared | Announced 2 May 2023 by Modular (closed, hosted playground); standard library open-sourced 28 March 2024; 1.0 on 11 August 2026 (Modular 26.5); compiler and toolchain open-sourced 18 August 2026 |
| Designed by | Chris Lattner (LLVM, Clang, MLIR, Swift) with the Modular compiler team; co-founder Tim Davis |
| Current version (Oct 2026) | 1.1.0 (17 September 2026); 1.2 nightlies; Modular reportedly acquired by Qualcomm in 2026 (secondary sources only; not verified against a primary announcement) |
| Governance | Single vendor. Public design proposals and a forum since 2024; compiler contributions "not being accepted yet", target end of 2026 [3]; Apache 2.0 with LLVM exceptions for stdlib (2024) and compiler (2026) |
| Typing | Static. Mandatory argument types; nominal traits; parametric (compile-time) generics checked against trait bounds; `PythonObject` as a dynamic escape hatch |
| Memory | Ownership with argument conventions (`imm`/`read`, `mut`, `var`, `ref`, `out`, `deinit`), ASAP destruction at last use, compile-time "origins" (lifetimes) tracked by a lifetime checker; no GC (Python objects are refcounted by CPython) |
| Compilation | AOT via MLIR and LLVM; a JIT exists for `mojo run`; targets CPUs and NVIDIA/AMD GPUs from one language |
| Primary domains | AI kernels and inference engines (Modular MAX), GPU programming, high-performance numeric code |
| Notable users | Modular's own MAX platform ("hundreds of thousands of lines" of Mojo kernels open-sourced in 25.3 [3]); early external adopters are mostly individuals and start-ups |

## Stated design goals
The 2023 "Why Mojo" document (no longer at its original URL; summarised from
contemporaneous coverage [1]) positioned Mojo as the language that would end
the "two-world problem" in AI: Python for research, C++/CUDA for production.
Its most quoted sentence was that Mojo was intended to be "a superset of
Python". The August 2025 vision document reframes this: Mojo is "the one
programming language developers can use to target all the diverse
hardware"; Python compatibility is secondary - Mojo "should feel familiar to
Python developers" and "will support more Python features over time", and
"Mojo may or may not evolve into a full superset of Python, and it's okay if
it doesn't" [2]. Design principles listed: "zero-cost abstractions", "you
can't afford implicit performance overhead in inner loops", "defer syntactic
sugar", and "every Mojo feature must work reliably in all situations" [2].
The three "architectural bets" are parametric metaprogramming, MLIR, and
integration with the MAX framework [2].

## Strengths

### Ownership expressed as argument conventions, not lifetime annotations
Every argument has a convention: the default is an immutable reference
(spelled `read` in 0.x, `imm` preferred in 1.0 [5]); `mut` is a mutable
reference with enforced exclusivity ("a mutable reference can't alias any
other reference to the same value"); `var` takes ownership (callers move
with the postfix `^` or copy if `Copyable`); `ref` is parametrically
mutable, following the referent; `out` is an uninitialised slot the callee
must fill (used for `self` in `__init__`); `deinit` is initialised on entry
and consumed by the callee (used in `__deinit__` and consuming moves) [4].
The design is Swift's `borrowing`/`inout`/`consuming` made mandatory and
visible at the declaration, with Rust's exclusivity rule but without
`&'a` syntax in ordinary code. Because the convention is on the
*parameter* and the compiler infers the rest, most Mojo code carries no
lifetime annotations at all.

### ASAP destruction
Values are destroyed at their *last use*, not at scope end: the compiler's
dataflow analysis inserts the `__deinit__` call immediately after the final
read [6]. This removes the C++/Rust surprise of a destructor running after
the value is logically dead, allows "register-passable" trivial types to be
treated as values, and makes NRVO-style initialisation (`out`) natural. The
documented cost is that a wildcard-origin pointer "effectively disables
Mojo's ASAP destruction" for values in its scope while live [6], i.e. the
optimisation interacts with unsafe code.

### Origins arrived later, mostly inferred, and already catch iterator invalidation
Lifetimes were renamed "origins" and are "tracked symbolically at compile
time"; most are created automatically, with `origin_of()`, inferred
origins from arguments, static origins for literals, untracked origins for
manual memory, and (discouraged) wildcard origins [6]. `ref` return values
tie the result to `self`'s origin and mutability: a `__getitem__` returns a
mutable reference on a mutable container and an immutable one otherwise,
with one definition [6]. 1.0 added experimental **interior origins**: `List`,
`Dict` and `String` element references are bound to the container, and a
mutation such as `List.append` invalidates them, so the lifetime checker
rejects later use [5] - the canonical Rust borrow-check case, reached
without explicit lifetimes.

### Parametric metaprogramming with one language
`[]` holds compile-time *parameters*, `()` holds run-time *arguments*;
"'parameter' always means a compile-time value" [7]. Parameters can be
types or values; `comptime` declares compile-time constants and type
aliases; `comptime if` / `comptime for` / `comptime assert` do conditional
compilation and unrolling; parameters are inferred from argument types; the
compiler "doesn't instantiate functions during parsing" (unlike C++), so a
`rebind()` escape exists for cases inference cannot settle [7]. This is Zig
`comptime` with trait-bounded, nominally typed generics, and it is the same
mechanism used for SIMD widths, GPU tiling and dtype specialisation.

### Nominal traits with defaults, refinement and composition
Conformance is explicit ("a struct with a matching method doesn't conform
unless it declares the trait"); traits have required and provided (default)
methods, associated `comptime` values and types, refinement (`trait B(A)`),
composition (`T: A & B`) and conditional conformance; `Some[Trait]` names
an anonymous conforming type [8]. Limitations are the usual nominal ones:
you cannot add a trait to a type you do not own, and conflicting defaults
must be resolved by hand [8].

### MLIR as the backend: one language for CPU and GPU
Mojo is built on MLIR Core and "is already able to target CPUs and GPUs
from different vendors"; the vision document claims it can "match or
exceed vendor BLAS libraries across a range of CPUs" [2]. The concrete
evidence is that Modular's own inference stack is written in Mojo rather
than CUDA/C++, and that the GPU id accessors, atomics orderings and tile
tensor types are ordinary library code [5].

### Python interop in both directions
Mojo embeds the unmodified CPython runtime (`libpython`) and imports any
Python module; conversely, Mojo functions and types can be exported to
Python with "no extra compilation step" [9]. 1.0 routed `PythonObject`
operators through CPython protocols (about 12x faster) and added NumPy
array copies [5]. This is what remains of "superset": not source
compatibility but ABI-level use of the Python ecosystem.

### Fast, decisive evolution with migration tooling
Over three years Mojo removed `let`, `borrowed`, `inout`, `owned`,
`@parameter`, `@register_passable`, `__del__`, `__moveinit__`/
`__copyinit__` and finally `fn`, each with deprecation warnings and
compiler fix-its [5][10]. 1.0 declares that during 1.x "changes should
primarily be additive", that only a named set of stdlib APIs is stable so
far (`Copyable`/`Movable`/`Deinitable`, `Array`, `List`, `Span`, `String`,
`Bool`, `Optional`) and that stability is marked per API [5]. Bounds
checking is on by default on CPU as of 1.0 [10].

## Weaknesses

### "Python superset" was the pitch and was abandoned
The 2023 message was a superset; the 2025 vision says "may or may not ...
and it's okay if it doesn't" [2]; a team member then clarified this only
meant not looking "too far into the future" [2]. The concrete design
decisions say more than the prose: 1.0 removed negative indexing (`x[-1]`
is a compile-time error) [10]; `def` no longer implicitly raises and every
argument must be typed [11]; classes, inheritance and untyped variables
are "Phase 3: not started" on the roadmap [12]; `PythonObject` is the only
dynamic type. Mojo today is a statically typed systems language with
Python's surface syntax and a CPython bridge. The lesson is that
"compatible with X but fast" collapses to "looks like X" as soon as the
semantics that make X slow (dynamic attributes, duck typing, exceptions
everywhere, refcounted everything) are removed; the migration story now
rests on "AI-assisted coding tools" [2].

### The dual `fn`/`def` mode did not survive
Mojo launched with two function kinds: `fn` (strict: typed arguments,
no implicit raises, declared variables) and `def` (Python-like: untyped
arguments allowed, implicit raises, implicit declarations). 0.24.4 (2024)
made `def` arguments borrowed by default; 0.26.2 (March 2026) unified them
("`def` ... has the same semantics as `fn`"); 1.0.0b1 deprecated `fn`
entirely [10][11]. Implicit variable declarations are now a warning with a
fix-it [5]. Two years of "which keyword do I use" ended with the strict
mode winning under the lenient keyword. This is strong evidence that a
language cannot sustain a dynamic dialect and a static dialect side by
side; users choose one and the other becomes dead weight.

### Three years of closed development and a still-frozen compiler
The compiler was proprietary from 2023 until 18 August 2026, with the
standard library open since March 2024; even after open-sourcing,
"compiler and tooling contributions are not being accepted yet", with
end-2026 as the target [3]. The company "previously built the language
using public design proposals" but states that "small design teams are best
for shaping a language's core" [3]. Governance is undefined beyond Modular,
and the vendor itself changed hands in 2026 (reported acquisition; see
Snapshot). For a language that asks users to port their kernels, this is
the main adoption risk and it is unmitigated.

### Churn: the cost of iterating in public on a 0.x line
The renames are legion: `borrowed` -> `read` -> `imm`; `inout` -> `mut`;
`owned` -> `var`/`deinit`; "lifetimes" -> "origins"; `alias` -> `comptime`;
`@parameter if` -> `comptime if`; `__del__` -> `__deinit__`;
`__moveinit__` -> `__init__(take=...)`; `Consistency` -> `Ordering`;
`.🔥`/`📦` file extensions removed; the REPL and Jupyter kernel removed in
25.3 [5][10][11]. Each change is individually defensible; the sum is that
code written in 2024 does not compile in 2026 and that only a handful of
stdlib types are marked stable at 1.0 [5].

### The language is still missing the application-programming half
The roadmap's Phase 2 ("systems application programming") lists as *not
started*: first-class `async`, algebraic data types and pattern matching,
existentials/dynamic traits, struct extensions, reflection, `private`
access control, and "memory safety: finalizing internal origins and
supporting mutable aliasing" [12]. The 1.0 announcement confirms "a robust
async programming model, pattern matching, and unions" are still ahead
[5]. Mojo 1.0 is a kernel language with a stability promise, not a
general-purpose language.

### Memory safety is not yet a guarantee
`UnsafePointer` with explicit origins is pervasive in the standard
library; `UnsafeAnyOrigin` widening was only deprecated in 1.0 [5];
mutable aliasing is unresolved [12]; `alloc` aborts on failure [10]; and
interior origins are "experimental" [5]. The exclusivity rule exempts
register-passable trivial types [4]. Mojo has Rust's *vocabulary* of safety
and much of its ergonomics, but no published soundness argument and
several documented gaps; the roadmap's own wording ("finalizing") is the
admission.

### Interop through an embedded CPython is a ceiling, not a bridge
Because Python objects are real CPython objects, every `PythonObject` call
crosses into the interpreter with its GIL and dynamic dispatch; a Mojo
function exported to Python is a C-extension in effect. Interop needs
Python 3.10-3.14 [9]. Nothing in Mojo makes *existing* Python faster; it
makes it callable.

### Platform and ecosystem breadth
Linux and macOS are the supported hosts (native Windows support was
absent through 0.x; its 1.x status was not verified in this research).
The package ecosystem is Modular's; a third-party package index was not
part of 1.0.

## Designer retrospectives
- The vision document's "may or may not evolve into a full superset" [2]
  is the retrospective on the 2023 positioning; Simon Willison's summary -
  "Mojo is now its own language" - is the external reading [1].
- The `def`/`fn` unification (0.26.2) and `fn` deprecation (1.0.0b1) [10][11]
  are the retrospective on dual modes: the team concluded the distinction
  "added complexity without benefit" (paraphrase of the unification
  changelog; the proposal's exact wording was not retrieved). The earlier
  removal of `let` (2024, from memory of the proposal; not re-verified)
  followed the same reasoning.
- Lattner's team wrote that language design "creates value that compounds
  over time" and that the forum exists "to avoid an echo chamber" [2][3] -
  an acknowledgement that closed design has costs they are now paying.
- The 1.0 announcement's "Breaking changes may still be made, but will be
  managed with care", modelled on "how mature languages like C++ evolve"
  [5], signals that even the stable line will not be frozen.

## Influence
- **Copied from others**: Python's syntax and `__dunder__` protocols;
  Swift's argument conventions (`inout`/`borrowing`/`consuming`), value
  semantics and protocol-style traits; Rust's ownership, exclusivity,
  `^` move (from Swift's `consume`), `where` clauses and nominal traits;
  Zig's `comptime` (as `[]` parameters and `comptime if/for`); C++'s
  parametric specialisation; MLIR from Lattner's own work.
- **Copied by others**: too early for language-level copying. Mojo is the
  most visible demonstration that Rust-style ownership can be delivered
  without lifetime syntax in most user code, and the clearest recent
  failure of the "superset of a dynamic language" strategy.

## Lessons for Sisu
- **Adopt**:
  - Argument conventions as keywords on parameters (`imm`/`mut`/`var`/
    `ref`/`out`/`deinit`) with a safe immutable default and a visible move
    operator [4]; this gives Rust's exclusivity with Swift's readability.
  - ASAP destruction at last use [6]; it is simpler to explain than scope
    end and removes a class of lifetime extension bugs.
  - The strict `[]` parameter / `()` argument split and `comptime if/for`
    [7]; one language for compile time and run time, with generics checked
    against nominal trait bounds.
  - Interior origins for container element references [5]: the compiler,
    not the programmer, tracks "this reference dies when the container
    mutates".
  - Bounds checking on by default, negative indexing rejected statically
    [10], deprecations with compiler fix-its [5].
- **Adapt**:
  - Origins: infer by default, require explicit origin parameters only at
    API boundaries that store references (as Mojo's "a field cannot hide
    `UnsafeAnyOrigin`" rule does [5]).
  - Traits: nominal conformance is right; Sisu should additionally allow
    conformance declared outside the type's module (orphan rules permitting)
    to avoid Mojo's "cannot add a trait to `Int`" limitation [8].
  - Python-style interop: embed the host runtime for *calling* libraries,
    but never promise source compatibility.
- **Avoid**:
  - Promising to be a superset of an existing dynamic language [1][2];
    the promise sets expectations the semantics cannot meet.
  - Dual dynamic/static function modes [10][11]; pick one.
  - Years of closed development followed by an open-source release with
    contributions still frozen [3]; design in public from the first
    release, with a documented decision process.
  - Renaming core keywords repeatedly on a public pre-1.0 line; either
    stay private until names settle or adopt tiered stability from the
    first public release.
- **Open questions raised**:
  - Can ownership-without-lifetime-syntax scale to general application
    code (async, graphs, self-referential structures), or only to the
    kernel-shaped code Mojo has so far? The roadmap's unresolved "mutable
    aliasing" [12] says this is not yet known.
  - Is "looks like Python" worth anything once semantics diverge? Mojo's
    users are kernel authors who could have learned any syntax; the
    familiarity dividend has not been measured.
  - Whether a single-vendor language can earn trust before its governance
    is independent; Swift took a decade.

## Sources
1. S. Willison, "Mojo is now open source", 18 August 2026 (summarising Modular's positioning change). https://simonwillison.net/2026/Aug/18/mojo-is-now-open-source/
2. Modular, "Mojo Vision" document and forum post "Mojo Vision Document and Roadmap", 26 August 2025. https://mojolang.org/docs/vision ; https://forum.modular.com/t/mojo-vision-document-and-roadmap/2187
3. Modular, "Mojo is now open source" (compiler and toolchain, Apache 2.0 with LLVM exceptions), 18 August 2026. https://www.modular.com/blog/mojo-open-source
4. Mojo manual, "Ownership" (argument conventions, transfer, exclusivity). https://mojolang.org/docs/manual/values/ownership
5. Modular, "Modular 26.5: Mojo 1.0 is here", 11 August 2026, and Mojo 1.0.0 release notes. https://www.modular.com/blog/modular-26-5-mojo-1-0-is-here ; https://mojolang.org/releases/v1.0.0
6. Mojo manual, "Lifetimes, origins and references" (ASAP destruction, origins, `ref`). https://mojolang.org/docs/manual/values/lifetimes
7. Mojo manual, "Parameterization: compile-time metaprogramming". https://mojolang.org/docs/manual/parameters/
8. Mojo manual, "Traits". https://mojolang.org/docs/manual/traits
9. Mojo manual, "Python interoperability". https://mojolang.org/docs/manual/python/
10. Mojo 1.0.0b1 release notes (May 2026): `fn` deprecated, `owned` removed, negative indexing removed, bounds checks default. https://mojolang.org/releases/v1.0.0b1
11. Mojo 0.26.2 release notes (March 2026): `def`/`fn` unification, `comptime if/for`, origin changes. https://mojolang.org/releases/v0.26.2
12. Mojo roadmap (phases 0-3, status as of 1.x). https://mojolang.org/docs/roadmap
13. Modular, "The Next Big Step in Mojo Open Source" (standard library, 28 March 2024). https://www.modular.com/blog/the-next-big-step-in-mojo-open-source
14. Mojo releases index (version dates). https://mojolang.org/docs/changelog
