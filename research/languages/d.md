# D

## Snapshot
| | |
|---|---|
| First appeared | December 2001; D 1.0 January 2007; D2 (const/immutable, ranges, Phobos rewrite) developed 2007-2010, documented in *The D Programming Language* (2010); D1 discontinued end of 2012 |
| Designed by | Walter Bright (compiler author since 1982; Zortech C++); Andrei Alexandrescu co-designed D2 and Phobos from 2006 |
| Current version (Oct 2026) | DMD 2.113.0 (17 August 2026); 2.112.0 (January 2026); LDC (LLVM) and GDC (in GCC since 9) track the same front end. OpenD, a fork started 1 January 2024, ships its own dmd/ldc |
| Governance | D Language Foundation (2015). Walter Bright and Átila Neves are the language maintainers; Alexandrescu stepped away from co-maintainer in 2024 but remains vice-president and treasurer [9]. Changes go through the DIP (D Improvement Proposal) process, decided by the two maintainers |
| Typing | Static, strong, with `auto` inference, templates, transitive `const`/`immutable`, CTFE |
| Memory | Conservative tracing GC by default; `@nogc`, manual allocation, `scope`/DIP1000 escape analysis, `-betterC` (no runtime, no GC) |
| Compilation | AOT native; DMD has its own very fast backend; LDC and GDC for optimisation and targets |
| Primary domains | General-purpose; tooling, finance, games, some systems work |
| Notable users | Weka.io (storage), Symmetry Investments, Funkwerk, Remedy Entertainment (Quantum Break tooling), eBay and Facebook experiments in the 2010s |

## Stated design goals
Per the HOPL IV paper's abstract, D set out "to combine the best features of
C, C++, and other languages" while keeping "the efficiency, low-level access,
and Algol-style syntax of C and C++" and improving "rapid development,
convenience, and syntax"; the C interface was designed to be zero-cost [1].
Bright's recurring goals on the forum are: no forward declarations, a
single pass design that compiles fast, "self-evident code" (attributes that
let the reader know what a function cannot do), and "D is designed with
being forkable in mind" [7]. D was the first serious attempt at a "better
C++" with GC, contracts, unit tests, CTFE and modules in one language; most
of what later languages call "modern" features first shipped together here.

## Strengths

### Compile-time function evaluation and `static if`: the ancestor of comptime
D has evaluated ordinary functions at compile time since 2007 (CTFE), has
`static if` for conditional compilation on types and values, `__traits`
for introspection, `enum` manifest constants, and string `mixin`s that
splice generated source. Together these give template metaprogramming
without a second language: a function written for runtime use can be
called in a `static if` condition or to build a lookup table. Zig's
`comptime`, C++'s `constexpr`/`if constexpr` (C++17) and Rust's `const fn`
all arrive later and implement subsets of what D had; Nim's compile-time VM
is the same idea. The 2.113 changelog still adds CTFE-adjacent features
(`__traits(needsDestruction, T)`, static array length inference) [2].

### Ranges as the iteration abstraction
Alexandrescu's 2009 essay "On Iteration" replaced C++'s iterator pairs
with *ranges* (`empty`/`front`/`popFront`, plus forward, bidirectional,
random-access refinements) checked structurally via `isInputRange!R`
templates, with lazy adaptors in `std.algorithm` and `std.range` [3].
Phobos v3 keeps ranges as "central to the design" (file I/O and formatting
"should work with any range") [10]. Rust's iterator adaptor chains and
C++20 `std::ranges` are the recognisable descendants.

### Built-in unit tests, contracts and coverage
`unittest { }` blocks live next to the code, compile with `-unittest` and
run before `main`; `-cov` produces line coverage from the compiler;
`in`/`out` contracts and class `invariant`s are part of the function
syntax; `assert` is a language primitive. Zig copied `test` blocks
directly, Rust's `#[test]` and `cargo test` serve the same "tests are
zero-friction" goal, and OpenD added named unittests [8]. The lesson that
tests belong in the language's default toolchain, not in a framework, is
D's.

### `scope(exit)`, `scope(success)`, `scope(failure)`
Alexandrescu's C++ ScopeGuard became syntax: a statement registered to run
at scope exit, on success or on exception. Go's `defer` (2009), Zig's
`defer`/`errdefer` and Swift's `defer` are the descendants; `errdefer` in
particular is `scope(failure)`.

### Uniform function call syntax
D defined UFCS (`a.f(b)` resolves to `f(a, b)` when `a` has no member `f`)
in 2010-2012 and gave it the name; it is what makes range pipelines read
`data.filter!(x => x > 0).map!(x => x * 2).array`. Nim adopted it; Kotlin's
extension functions and Swift's extensions solve the same extension problem
by other means.

### Fast compilation and scripting
DMD's single-pass front end and hand-written backend compile large
programs in seconds; `rdmd` compiles-and-runs a file like a script. This
was a deliberate goal (no forward declarations, no textual preprocessor,
modules instead of headers) and it holds up 25 years on; it is also why D
was an early demonstration that modules plus fast compilers remove the
need for a build-time macro layer.

### Attributes that let the reader know what a function cannot do
`pure` (with "weak purity": a pure function may mutate through its
mutable parameters), `nothrow`, `@nogc`, `@safe`/`@trusted`/`@system`,
`scope` and `return` on parameters, and transitive `immutable`. For
templates the compiler *infers* these attributes, so generic code gets
them for free. Bright's "self-evident code" argument is that a `pure
nothrow @nogc @safe` signature is a proof of local reasoning [4]. The
weakness is that for non-template code every attribute is opt-in (below).

### ImportC and `-betterC`
ImportC (2021-) lets `import` consume a C header or source file directly,
with the compiler parsing C11 itself; 2.113 still fixes ImportC corner
cases (`.di` generation, complex types) [2]. `-betterC` strips the runtime
and GC so D can be used as a C replacement on bare metal, with contracts,
CTFE and templates intact. Zig's `@cImport` is the same move.

## Weaknesses

### The GC-optional ambiguity
D's biggest strategic wound. The GC is the default and the standard library
relies on it (exceptions, closures, slices, associative arrays, most of
`std.string`); but D also advertises `@nogc`, `-betterC`, manual allocation
and `scope`. The result is two half-ecosystems: systems-niche users left
because the GC was conservative, stop-the-world and non-concurrent, and
GC-using libraries were unusable from `@nogc` code; GC-niche users found
Phobos increasingly contorted to avoid allocation. The Phobos v3 design
document now states that "the user, not the library, should control memory
strategy", that library code "should avoid allocating where possible",
and that exceptions "use the GC and rule out `nothrow` and `@nogc`", so v3
"minimises" exceptions and prefers error-free return values and
option/sum types [10]. The OpenD fork went the opposite direction:
"embracing the GC and disregarding betterC and nogc" [7]. Twenty-five years
in, the two camps forked rather than resolve the question, which is the
strongest possible evidence that "GC, but optional" is not a stable design
point.

### Too many features
D accumulated contracts, invariants, unittest, CTFE, `static if`, templates
with constraints, template mixins, string mixins, `alias this`, ranges,
`pure`/`nothrow`/`@nogc`/`@safe`/`@trusted`/`@system`, `scope`, `return`,
`ref`, `in`, `out`, `inout`, `shared`, `synchronized`, `immutable`/`const`,
`lazy` parameters, `foreach_reverse`, `opDispatch`, `opApply`,
properties, delegates and function pointers, nested functions, classes
*and* structs with different semantics (reference vs value, inheritance
vs none), `interface`s, `union`s, C-style bit fields (2.11x, behind `-preview`), named arguments,
string interpolation (2024), ImportC and `-betterC`. Almost nothing is
removed. The evidence of cost is in the project's own artefacts: a
graveyard of `-preview=` switches that were never turned on by default
(OpenD lists the ones it enables and calls the rest "unsupported" [8]);
`-preview=bitfields`, which OpenD calls "a mistake to copy into D" [8];
DIP1000 (`scope` escape analysis), proposed in 2016 and still not default
in 2026, with 2.113's changelog still fixing `scope` inference bugs and
asking the compiler to "explain why `scope` was inferred" [2]. Each feature
was defensible alone; the sum is a language no newcomer can hold in their
head and no single maintainer can keep consistent.

### Opt-in safety does not get used
`@safe` (2010) restricts a function to a memory-safe subset; `@trusted`
asserts safety for a body the compiler cannot check; the default is
`@system`. DIP 1028 "Make @safe the Default" (2019-2020) argued "users
expect safety to be opt-out, not opt-in"; it was *accepted* by the
maintainers, then reversed by Bright within days, and is now marked
Rejected [5]. The sticking point was `extern(C)` declarations: a bare C
prototype would have become callable from `@safe` code with no check,
which "almost everyone considered unacceptable" [6]. An August 2026 forum
thread shows the question is still open; one participant's diagnosis is
timing - "the choice should have been made when D2 was first released"
and ten years later "too much breakage in the eco-system" would result
[6]. DIP1000's `scope` semantics are complex enough that the standard
idiom is to wrap pointer code in `@trusted` lambdas, which defeats the
purpose. Meanwhile Rust (unsafe is the exception) shipped in 2015 and is
what "memory-safe systems language" now means. OpenD turned on several
"safe" warnings and `-preview=systemVariables` by default [8]; upstream
"is looking at" those changes [8]. The lesson is the one DIP 1028's
critics state: a safety default is set at birth or not at all.

### Splitting the standard library, twice
2007-2010: Phobos (Bright's) vs Tango (community) split the D1 library
ecosystem; libraries chose one and could not be used from the other, and
the D2 transition was partly an attempt to end it. The Phobos v3 document
records the lesson: "replacing v1 outright broke existing code and caused
lasting damage, which is why v2 is kept alive"; v3 therefore lives under a
new root (`phobos.sys`, `phobos.data`, `phobos.text`, `phobos.net`) while
`std` is "maintained for compatibility only" [10]. The 2024 fork added a
third library direction (`std.v2` in OpenD [7]).

### Autodecoding
Phobos v2 made `string` (an array of UTF-8 `char`) present itself to range
algorithms as a range of decoded `dchar` code points, so `"héllo".front`
is `'h'` but `"héllo"[1]` is a byte. It was an attempt at Unicode
correctness by default that produced an inconsistent, slow, exception-throwing
abstraction. Phobos v3 states flatly that "autodecoding was a mistake
because it is pervasive and hard to disable" and removes it: callers opt
in with `byDchar` [10]. The general lesson is that a "safe by default"
transformation applied implicitly inside a library, rather than in the
type system, cannot be removed once code depends on it.

### Governance: a two-person bottleneck and a fork
The DIP process routes every language change through the two maintainers.
String interpolation (DIP 1027, DIP 1036 and successors) was proposed
and rejected repeatedly over several years before "interpolated
expression sequences" reached upstream in 2024 - after OpenD had shipped
them [8]. On 2 January 2024
GrimMaple and Adam D. Ruppe announced "We are forking D", stating a goal to
"change the way decisions are made" and make contribution "as open-ended
as possible" [7]; in the follow-up thread GrimMaple's grievance is that
Bright "blocks useful contributions over minor issues while pushing
features like ImportC and `@live`" [7]. Bright's reply was that D "is
designed with being forkable in mind" and he asked for "the reciprocal
courtesy" [7]. Alexandrescu, D2's co-designer, had already "stepped away
from his role as co-maintainer" by 2024 [9]. Phobos v3's contribution
rules - reviewer silence within 24 hours counts as approval [10] - are a
direct reaction to "pull requests often stall". D illustrates how a
BDFL model that works at 10 contributors fails at 100.

### `const`/`immutable` transitivity and `shared`
D2's transitive `immutable` (everything reachable is immutable) and
`inout` (a wildcard for const/immutable/mutable) are sound but hard to
use with classes and ranges; code either drops to mutable or duplicates
overloads. `shared` marks data accessible from multiple threads but its
semantics were never fully specified or enforced, so D has a data-race
*marker* without data-race *freedom*. OpenD's treatment of invalid `inout`
as `const` [8] is a workaround, not a fix.

## Designer retrospectives
- Bright, Parker and Alexandrescu wrote "Origins of the D Programming
  Language" for HOPL IV (2020); Bright noted that in researching dates he
  "discovered many of my" recollections were wrong and thanked his
  co-authors for "endless hours spent fixing the mess I originally wrote"
  [1]. The paper's body was not accessible for this research; its
  treatment of the Phobos/Tango split and the D1/D2 transition is
  recalled, not re-verified.
- The Phobos v3 design document is the most concrete written
  retrospective: autodecoding was a mistake; replacing v1 outright caused
  lasting damage; `std.stdio` mixes I/O and formatting; `isSomeChar`-style
  templates are hard to read; the build process discourages contributors;
  PRs stall [10].
- Bright on DIP 1028: accepted, then reversed over `extern(C)` [5][6]. He
  has not published a retrospective on the reversal; the 2026 thread shows
  the community still regards it as the moment D lost the safety race.
- OpenD's changes page is an external retrospective by former
  contributors: zero-initialise floating point and chars, enable the
  `-preview` switches that worked, drop the bitfield feature, make `Object`
  monitor-free, and keep the GC as a first-class design choice [8].

## Influence
- **Copied from others**: C/C++ syntax and ABI; Java's single inheritance
  with interfaces, GC and modules; Python's slices; Eiffel's contracts;
  C++ templates (made saner); ScopeGuard from Alexandrescu's own C++.
- **Copied by others** (strong evidence of what worked): CTFE and
  `static if` -> Zig `comptime`, C++17 `if constexpr`, Rust `const fn`;
  `unittest` blocks -> Zig `test`; `scope(exit/failure)` -> Go/Zig/Swift
  `defer`, Zig `errdefer`; ranges -> Rust iterators, C++20 ranges; UFCS ->
  Nim; ImportC -> Zig `@cImport`; `@safe`/`@trusted`/`@system` -> the
  three-way vocabulary now used in Rust discussions (`unsafe` blocks,
  "trusted" wrappers). What was *not* copied: GC-optional, transitive
  `const`, autodecoding, opt-in safety, the DIP-by-two-maintainers model.

## Lessons for Sisu
- **Adopt**:
  - `unittest` blocks, contracts and coverage as language/toolchain
    features, not a framework.
  - `scope(exit)`/`scope(failure)` (or `defer`/`errdefer`).
  - Compile-time evaluation of ordinary functions plus `static if`; D proved
    this a decade before Zig.
  - Fast single-pass compilation as a design constraint (no forward
    declarations, real modules).
  - Direct C header import (ImportC).
- **Adapt**:
  - Ranges: keep the lazy-adaptor pipeline model but define the protocol
    as a checked interface/trait rather than D's structural `isInputRange!`
    templates, so that errors appear at the definition, not at
    instantiation.
  - Attribute inference for generic code is good; but attributes should
    be *defaults* (safe, nothrow, nogc unless declared otherwise), with
    the opposite spelled out. D's experience is that inference-only
    attributes never propagate to hand-written code.
  - UFCS: adopt the extension property, with D's "member wins over free
    function" rule made explicit.
- **Avoid**:
  - "GC, but you can avoid it". Decide one memory model for the language
    and library; D's two camps ended in a fork [7][10].
  - Opt-in safety. Set the safe default at 1.0; DIP 1028 [5] shows it
    cannot be flipped later.
  - Implicit library-level "correctness" transformations (autodecoding);
    put Unicode semantics in types, not in `front`.
  - Feature accretion without a removal process; keep a `-preview`
    mechanism only with a hard deadline to land or drop.
  - Replacing a standard library wholesale; version it alongside.
  - A two-person approval bottleneck for a community-scale language;
    D's governance crisis cost it its co-designer and a fork within a year.
- **Open questions raised**:
  - Can a language offer *both* a GC'd convenience tier and a no-GC
    systems tier without splitting the library? D says no; Sisu must
    decide whether the no-GC story is the whole language or a documented
    subset with its own library.
  - How much "self-evident" attribute vocabulary (`pure nothrow @nogc
    @safe scope return`) can a signature carry before it is noise? D is
    the upper bound; Sisu needs defaults that make most of them silent.
  - Whether exceptions can coexist with `nothrow`/`@nogc` code at all;
    Phobos v3's answer (prefer error values, minimise exceptions) [10] is
    an argument for one error mechanism.

## Sources
1. W. Bright, A. Alexandrescu, M. Parker, "Origins of the D Programming Language", Proc. ACM Program. Lang. 4, HOPL (2020), doi:10.1145/3386323; HOPL IV page https://hopl4.sigplan.org/details/hopl-4-papers/17/Origins-of-the-D-Programming-Language ; Bright's announcement thread https://digitalmars.com/d/archives/digitalmars/D/announce/Origins_of_the_D_Programming_Language_now_published_by_ACM_58070.html
2. D changelog 2.113.0 (17 August 2026) and index. https://dlang.org/changelog/2.113.0.html ; https://dlang.org/changelog/index.html
3. A. Alexandrescu, "On Iteration", InformIT, 2009. https://www.informit.com/articles/article.aspx?p=1407357
4. W. Bright, "Crafting Self-Evident Code with D", D Blog, 2 October 2023. https://blog.dlang.org/
5. DIP 1028, "Make @safe the Default" (rejected), 2019-2020. https://github.com/dlang/DIPs/blob/master/DIPs/rejected/DIP1028.md
6. digitalmars.D, "Memory Safe Programming", August 2026 (on DIP 1028's rejection and timing). https://lists.puremagic.com/pipermail/digitalmars-d/2026-August/347981.html
7. digitalmars.D, "We are forking D" (2 January 2024) and "The forked elephant in the room" (January 2024). https://digitalmars.com/d/archives/digitalmars/D/We_are_forking_D_371962.html ; https://forum.dlang.org/post/uo4fc0$1egm$1@digitalmars.com
8. OpenD, "Changes from the upstream D Language" and project page. https://opendlang.org/changes.html ; https://opendlang.org/
9. M. Parker, forum post on Alexandrescu stepping away as co-maintainer, 2 August 2024. https://forum.dlang.org/post/deotegblhpepjbsjlqdr@forum.dlang.org
10. dlang/PhobosV3-Design, "Design.md" (Phobos v3 design and lessons from v2). https://github.com/dlang/PhobosV3-Design/blob/main/Design.md
11. D specification: Memory Safety, `-betterC`, Contracts, Unit Tests. https://dlang.org/spec/memory-safe-d.html ; https://dlang.org/spec/betterc.html
