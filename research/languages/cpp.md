# C++

## Snapshot
| | |
|---|---|
| First appeared | 1979 as "C with Classes" (Bell Labs); renamed C++ 1983; Cfront 1.0 released 1985; ISO C++98 |
| Designed by | Bjarne Stroustrup; since 1990 evolved by ANSI X3J16 / ISO WG21 |
| Current version (Oct 2026) | C++23 (ISO/IEC 14882:2024) is the published standard. C++26 was technically completed by WG21 on 28 Mar 2026 (Croydon, 411 national-body comments resolved) and is in DIS ballot per the convenor's report N5056 (28 Jul 2026); ISO publication expected late 2026. C++29 work began at Brno (Jun 2026). Compilers: GCC 16 (Apr 2026) and LLVM 22 (Feb 2026) per their release schedules (not individually verified); MSVC 14.50 shipped with Visual Studio 2026 (Nov 2025), which defaults to C++20. |
| Governance | ISO/IEC JTC1/SC22/WG21: three-year release train, plenary consensus votes, ~20 study groups (SG23 = Safety and Security). Membership reported at roughly 400 (TechRepublic; secondary). A "Direction Group" (P2000) publishes non-binding priorities. |
| Typing | Static, nominal, strong with many implicit conversions; templates (structurally checked at instantiation) constrained by concepts since C++20; `auto` inference; no language sum types (`std::variant` is a library); nullability not expressed in types |
| Memory | Manual with RAII and smart pointers (`unique_ptr`, `shared_ptr`); no language-enforced memory safety; hardened library and "profiles" under development |
| Compilation | AOT, textual `#include` plus C++20 modules; heavy monomorphisation; separate compilation with a frozen de facto ABI (Itanium C++ ABI, MSVC ABI) |
| Primary domains | Browsers, game engines, HFT/finance, HPC and ML framework cores, databases, embedded, compilers, CAD/EDA, OS components |
| Notable users | Chromium, LLVM, Unreal Engine, Qt, PyTorch/TensorFlow cores, MongoDB, ClickHouse, Adobe, Bloomberg. Stack Overflow 2025: 23.5% of respondents used C++ in the past year; JetBrains 2025 (24,534 devs): 27%; TIOBE Sep 2026: #3 at 8.67%. |

## Stated design goals
Stroustrup's *The Design and Evolution of C++* (1994) lists the rules the language was built
under, including "C++ must be compatible with C", "no implicit violations of the static type
system", "leave no room for a lower-level language below C++ (except assembler)", and the
**zero-overhead principle**, restated in Sutter's P0709 as "you don't pay for what you don't
use" and "when you do use it you can't reasonably write it more efficiently by hand". The
language was to be "a better C" that supports data abstraction, object-oriented programming and
generic programming without a runtime.

The Direction Group's P2000R4 (Hinnant, Orr, Stroustrup, Vandevoorde, Wong, Oct 2022) restates
the long-term aims: "Stability: Useful code 'lives' for decades" against "Evolution: The world
changes and C++ must change to face new challenges", with the admission that "There is an
inherent tension here." Its stated ideal is "Complete type-safety and resource-safety (no
memory corruption and no leaks)", immediately qualified: this "cannot be achieved while
accepting every legal C++ program (as we must for compatibility), so external tools will" be
needed.

## Strengths

### RAII and deterministic destruction: the most-copied idea in systems languages
Resource acquisition is initialisation (constructors acquire, destructors release, scope exit
is the release point, exceptions unwind through destructors) is C++'s single most influential
contribution. It makes resource safety (files, locks, sockets, memory) a *type* property
instead of a discipline. Evidence by imitation: Rust's `Drop` and move semantics are RAII with
compiler-checked ownership; D copied it; Swift's `deinit` under ARC provides the same guarantee
for class instances; Zig, Go and C2Y's `defer` are weaker, statement-level versions of the same
idea. Combined with C++11 move semantics (`unique_ptr`, rvalue references), it is the basis of
the C++ Core Guidelines' "resource safety" and the reason modern C++ code can be leak-free
without a GC.

### The zero-overhead principle as a design filter
Every abstraction must be free when unused and hand-code-competitive when used. This filter is
why C++ remains the performance yardstick for games, browsers and HFT and why its abstractions
(iterators, `std::sort` beating `qsort`, `std::vector`, lambdas inlined into algorithms) are
not a tax. Sutter's P0709 shows how seriously the community takes it: the only two features
that violate it (exceptions and RTTI) are the two that projects routinely disable.

### Generic programming via templates and the STL
Stepanov's STL (1994) demonstrated that generic algorithms over iterators could be *faster*
than hand-written loops because templates monomorphise. Templates became a Turing-complete
compile-time facility (template metaprogramming, 1994 onward), then were gradually civilised:
`constexpr` functions (C++11), relaxed `constexpr` (C++14), `if constexpr` (C++17), concepts
(P0734R0, C++20), `consteval`/`constinit` (C++20), and static reflection (P2996, C++26), which
Sutter calls "by far the biggest upgrade for C++ development that we've shipped since the
invention of templates". Evidence by imitation: D's CTFE, Zig's `comptime`, Rust's `const
fn`/const generics and Mojo's parametric metaprogramming are all answers to "what if C++'s
compile-time power had been designed on purpose".

### Value semantics by default
Objects are values; copying is deep unless the type says otherwise; references and pointers are
explicit. This is the opposite of Java/C#/Python reference semantics and is what makes local
reasoning about aliasing possible at all. Rust and Swift (structs) adopted value-by-default;
Swift's designers cite it explicitly.

### A memory model that others adopted
C++11 (with C11) defined the first mainstream formal memory model for threads and atomics
(sequential consistency by default, acquire/release/relaxed orderings). Rust's atomics are the
C++11 model verbatim; Go, Java (`VarHandle`) and Swift aligned to it.

### Backward compatibility and longevity
Code from 1998 compiles today. For infrastructure with 30-year lifetimes this is a decisive
adoption property, and it is why "rewrite it in X" has only ever happened at the margins. (It
is also the trap; see ABI below.)

### Tooling depth
Three independent production compilers (GCC, Clang, MSVC) that agree on the standard to a
remarkable degree; sanitizers (ASan, TSan, UBSan, MSan) were invented for C/C++; clangd,
clang-format and clang-tidy give modern IDE behaviour. The weakness is not tooling quality but
tooling *integration* (build, packages, modules).

## Weaknesses

### Complexity: "a much smaller and cleaner language struggling to get out"
Stroustrup, D&E (1994), p. 207: "Within C++, there is a much smaller and cleaner language
struggling to get out." Thirty years later P2000R4 says: "C++ is complicated, too complicated,
yet we cannot remove significant facilities and changing them is very hard." The same paper
warns "C++ has a major problem with learning and teaching. This could send the C++ community
into decline over the next few years. C++ is expert-friendly, but it cannot be just
expert-friendly without losing important streams of new talent."

Concrete manifestations: at least six initialisation syntaxes (`T x = v;`, `T x(v);`, `T
x{v};`, `T x = {v};`, `auto x = T{v};`, `T x{};`) with different semantics, the "most vexing
parse", and `std::initializer_list` hijacking brace-init for containers; four kinds of casts
plus C casts; `const`/`constexpr`/`consteval`/`constinit`; `typename`/`template`
disambiguators; overload resolution with user-defined implicit conversions; value categories
(lvalue, xvalue, prvalue, glvalue, rvalue) that are needed to explain `std::move` and
forwarding references. Google's style guide pins code to C++20 and advises against C++23
features; most shops ban a subset. The committee's own direction paper now says "We discourage
isolated 'cute' proposals" because "their addition inevitably leads to an impression of
instability".

### Templates vs. concepts: late, deep, long diagnostics
Pre-C++20 templates are checked only at instantiation, so a misuse of `std::sort` on a
`std::list` or a missing `operator<` is reported from inside the library, after the
instantiation stack, as a "no match for operator" error several levels deep. This was the
motivating example of Stroustrup's P0557 "Concepts: The Future of Generic Programming" (2017)
and of merging the Concepts TS into C++20 (P0734R0). Concepts move the check to the call site
and name the failed requirement, which is a genuine improvement. But diagnostics remain long
because overload resolution still prints every candidate with its unsatisfied constraints,
because constraint subsumption nests, and because most existing libraries are unconstrained.
Concepts also arrived 15 years after the first design (C++0x "concepts" were removed in 2009 at
Frankfurt as too complex), which is itself evidence about how hard it is to retrofit
constraints onto duck-typed generics. (I did not re-measure diagnostic lengths for this
research; the qualitative pattern is well documented, exact line counts are not.)

### The exceptions debate: a language whose primary error mechanism half its users ban
Sutter's P0709R4 "Zero-overhead deterministic exceptions: Throwing values" (2019) documents the
rift with numbers:
- "In [SC++F 2018], 52% of C++ developers reported that exceptions were banned in part or all
  of their project code".
- Enabling exception support in a project that throws nothing "commonly incurs significant
  binary space overhead; I regularly hear +15% reported ... and +16% reported by Ben Craig ...
  and I have recently seen other Windows internal examples with +38% bloat, down from +52%".
- Exceptions "violate the zero-overhead principle" because "throw requires dynamic allocation
  and catch of a type requires RTTI"; on the Itanium ABI exceptions are heap-allocated;
  matching handlers via RTTI "has statically unpredictable cost". This is "the primary reason
  exceptions are banned in many real-time and/or safety-critical environments" (JSF++ 2005,
  Mars rover flight software).
- The standard library itself proliferates dual interfaces: `<filesystem>` functions "provide
  two overloads, one that throws an exception ... and another that sets an error_code".
- Google's style guide: "On their face, the benefits of using exceptions outweigh the costs,
  especially in new projects", but "Because most existing C++ code at Google is not prepared to
  deal with exceptions" they stay banned. Every compiler ships `-fno-exceptions`.

Sutter concludes: "This is an intolerable rift: Large numbers of 'C++' projects are not
actually using standard C++." P0709 proposed `throws` functions returning a statically typed
`std::error` by value through the normal return path (two registers), with allocation failure
handled separately. No revision after R4 (Aug 2019) was found; what the committee actually
shipped was `std::expected` (C++23), i.e. a library result type, so C++ now has *three* error
channels (exceptions, error codes, `expected`) and no rule for choosing. Boost.Outcome
implemented P0709's `std::error` experimentally.

### Modules: proposed 2004, standardised 2020, still not routinely usable in 2026
The build-system/compiler coupling that C's `#include` model avoided is exactly what modules
require: a dependency scan before compilation, compiler-specific binary module interfaces
(BMIs), and a build graph that CMake, Ninja and the three compilers all had to learn. State as
of CMake 4.x documentation (2026): module dependency scanning works with MSVC 14.34+, Clang 16+
and GCC 14+; `import std;` needs Clang 18.1.2+, MSVC 14.36+ or GCC 15+ **and** the experimental
gate `CMAKE_EXPERIMENTAL_CXX_IMPORT_STD`; only Ninja generators support `import std` (Visual
Studio generators cannot build BMIs for imported targets); header units are "not supported ...
for any generator"; Ubuntu releases before 26.04 ship a broken `libstdc++.modules.json`.
Boost's modules prototype (2025) is on hold until CMake's `import std` is stable. A CppCon 2026
abstract claims modules "stopped being experimental somewhere around 2025" (secondary; the
session page could not be fetched). Six years after C++20, the standard library cannot be
imported as a module from a stable, generator-independent build on all three compilers. Lesson:
a module system is a *build system* feature; if the language does not own the build graph and
the module cache, the ecosystem takes a decade to catch up.

### Build systems and package management: the top two pain points for a decade
The ISO C++ Foundation's annual surveys consistently rank the same items first: in the 2023
survey, 47% called managing third-party libraries a major pain point, 43% build times, 31%
managing CMake projects (via JetBrains' summary; the 2026 "Lite" survey summary exists but
could not be fetched). There is no standard build description and no standard package manager;
CMake is de facto, vcpkg (2,691 ports, Oct 2025) and Conan compete, and most large projects
vendor. Build times are structural (headers re-parsed per TU, template instantiation repeated
per TU); Activision cut Call of Duty's build from 28 to 14 minutes using MSVC Build Insights,
which illustrates both the scale and that the fix is tooling, not language.

### ABI stability trapping evolution
In Prague (Feb 2020) WG21 polled the ABI question raised by Titus Winters' papers (P1863 "ABI -
Now or Never" and P2028, numbers from memory) and, per Corentin Jabot's account, found "a clear
majority but no consensus" that WG21 was *not* in favour of an ABI break in C++23, *was* in
favour of one in some future version, and "wants to prioritize performance over stability".
Nothing has happened since. Known costs of the frozen Itanium/MSVC ABIs: `std::unique_ptr`
cannot be passed in a register (so it is not zero-overhead versus a raw pointer; Chandler
Carruth's CppCon 2019 talk "There Are No Zero-cost Abstractions"), `std::regex` is so slow that
Jabot notes "it is currently faster to launch PHP to execute a regex", `std::unordered_map` is
locked into a node-based layout, `std::string`/`std::vector` layouts cannot change. Jabot
estimates a 5-10% performance loss growing over time and calls not breaking ABI in C++23 "the
worst mistake the committee ever made". The committee's workaround is to add new names
(`std::flat_map`, `std::hive`) rather than fix old ones. The last real break (libstdc++'s C++11
`std::string` in GCC 5, 2015) was painful enough that distributions still carry dual-ABI
symbols.

### Committee process: "Remember the Vasa!"
Stroustrup's P0977R0 (2018), responding to the C++20 pipeline: "We are on a path to disaster
though enthusiasm and design-by-committee (or rather 'design-by-committees')." "The foundation
begun in C++11 is not yet complete, and C++17 did little to make our foundation more solid,
regular, and complete. Instead, it added significant surface complexity and increased the
number of features people need to learn." "We now have about 150 cooks; that's not a good way
to get a tasty and balanced meal." "We are on the path to something that could destroy C++. We
must get off that path!" The Direction Group (P2000) was the institutional response, but it is
advisory. Observable symptoms since: C++26 contracts adopted 114-12-3 "despite disquiet over
their value" (DevClass) and over Stroustrup's objection; trivial relocatability removed from
C++26 at the last meeting due to implementation bugs; `std::execution` (P2300) adopted in 2025
after four years and ten revisions; coroutines shipped in C++20 as a bare framework with no
standard task type until C++26's `std::execution::task` (P3552; from memory).

### The 2024-2026 safety debate: profiles vs. Safe C++
Pressure: the ONCD report "Back to the Building Blocks" (26 Feb 2024) and the CISA/FBI/NSA
roadmap guidance (deadline 1 Jan 2026, non-binding) named C/C++ as the problem; vendor data
(Microsoft ~70% of CVEs, Chromium ~70% of high/critical bugs, Android below 20% after moving
new code to Rust; see `c.md`) supplied the evidence.

Two responses inside WG21:
1. **Safety Profiles** (Stroustrup's Core Guidelines lineage; Sutter's P3081 "Core safety
   profiles", R0 Oct 2024, R2 2025; Stroustrup's P3984 type-safety profile, P4222
   initialization profile; Dos Reis's framework): opt-in, annotation-free, backward-compatible
   sets of rules enforced by the compiler and static analysis, following P2000R4's claim that
   "any single definition of safety will be wrong for a significant user population". Outcome
   so far: Hagenberg (Feb 2025) found no consensus to put profiles in C++26; P4186R0 (Bindels,
   Apr 2026) lists P3081R2, P3589R2, P3700R0 and P3984R0 as attempts that were "so far none
   successful", and proposes a plan targeting a Committee Draft in 2028 and DIS in 2029 for
   C++29. SG23 took the lead at Croydon (Mar 2026) and in Brno (Jun 2026) decided to create a
   profiles specification "which would be merged into C++29 if it arrives in time" or otherwise
   published as a white paper. WebKit's hardening of ~4 million lines (P4158R0) was presented
   as evidence that bounds and type profiles pay off in practice.
2. **Safe C++** (Sean Baxter, P3390, Sept 2024, with the C++ Alliance): a superset enabled
   per-file with `#feature on safety`; `safe` functions in which raw pointer dereference,
   pointer arithmetic, union access and calls to unsafe functions are rejected; checked
   references `T^` with MIR-level borrow checking ("This analysis is done at compile time. It
   has no impact on your binary's size or execution speed"); destructive relocation instead of
   moves; `choice` types and exhaustive `match`; a new `std2` library; runtime bounds checks
   that panic; `send`/`sync` for data-race freedom; "`unsafe` is an auditable token where the
   user expressly takes responsibility". Implemented in Baxter's Circle compiler. At Wrocław
   (Nov 2024) the SG23 priority poll was 19 Profiles : 11 Both : 6 Neutral : 9 Safe C++; Safe
   C++ was reportedly abandoned by Sept 2025 (per the State of C++ 2026 newsletter citing
   InfoWorld/The Register; not verified against a WG21 record). Baxter's position: profiles
   cannot deliver lifetime safety without borrow-checker-style annotations; Sutter's:
   annotations are unadoptable for existing code and the big four categories (type, bounds,
   initialisation, lifetime) can be attacked incrementally.

What C++26 actually ships for safety (from memory of the papers; verify numbers): erroneous
behaviour for reads of uninitialised automatic variables (P2795) replacing UB, standard library
hardening (P3471) making `operator[]` etc. checked in a hardened mode, and contracts (P2900).
What it does not ship: any lifetime safety, any data-race freedom, any safe subset.

### Why Carbon and Circle exist
Carbon (Google, announced at CppNorth in July 2022 by Chandler Carruth) is explicitly a C++
successor rather than a C++ evolution because its authors judged that the ABI freeze and the
committee process would not deliver the changes they needed; its selling point is bidirectional
C++ interop by compiling C++ with its own Clang-based toolchain. Status Oct 2026: still "an
experiment"; the 0.1 milestone slipped from 2025 to "at the soonest, the end of 2026", because
the team added "a concrete design for memory safe Carbon" to 0.1; full memory safety is
deferred to 0.2. Circle (Baxter, 2019) is a one-person, closed-source C++ compiler used as a
vehicle for extensions (pattern matching, reflection, Safe C++); Baxter has said he will
publish the frontend "when it's viable". Sutter's own Cppfront/cpp2 (2022) is a third "syntax 2
for C++" experiment. Three successors in four years, from the committee chair, a Google team
and the most prolific independent compiler author, is the strongest evidence that the language
cannot fix itself in place.

### Concurrency: a model without a safety net
Threads, atomics and the memory model are excellent; everything above them is the programmer's
problem. There is no data-race freedom, no structured concurrency, no standard executor until
`std::execution` in C++26, and C++20 coroutines are a compiler transformation with no
scheduler, no standard `task<T>`, and heap-allocated frames unless the optimiser elides them.
Function colouring arrives with coroutines as in every async design.

### Compile times
Templates and headers compound: each TU re-parses all headers and re-instantiates all templates
it touches. This is structural and the reason modules were pursued for 20 years. Zig and Rust
both list C++ compile times as a motivating negative; Rust then reproduced the problem via
monomorphisation.

## Designer retrospectives
- Stroustrup, D&E (1994): the "smaller and cleaner language struggling to get out" line; also
  the acknowledgement that C compatibility rules forced implicit conversions, arrays decaying
  to pointers and the preprocessor into C++.
- Stroustrup, P0977R0 (2018): the Vasa warning quoted above; "We should not spend most our time
  creating increasingly complicated facilities for experts, such as ourselves."
- Stroustrup, interviews (2018): "C++17 did little to make our foundation more solid, regular,
  and complete. Instead, it added significant surface complexity" (also in P0977).
- Sutter, P0709R4 (2019): "a large fraction of the C++ community is not actually using 'real'
  C++, but are using a language dialect"; exceptions and RTTI are the two features "so widely
  disabled that [P0941R0] proposes special feature test macros for testing the absence of only
  those C++98 language features."
- Direction Group, P2000R4 (2022): "C++ is complicated, too complicated"; "C++ has a major
  problem with learning and teaching"; type and resource safety "cannot be achieved while
  accepting every legal C++ program".
- Stroustrup's FAQ: "There are only two kinds of languages: the ones people complain about and
  the ones nobody uses" and "C makes it easy to shoot yourself in the foot; C++ makes it
  harder, but when you do it blows your whole leg off" (both confirmed as his on his FAQ).

## Influence
Given to others: RAII/destructors (Rust, D, Swift, partially Zig/Go via `defer`); value
semantics and moves (Rust, Swift); templates and monomorphised generics (D, Rust, Mojo, Zig's
comptime types; Java generics were designed in reaction, with erasure); `constexpr`
compile-time evaluation (Zig `comptime`, Rust `const fn`, C23 `constexpr` objects); the C++11
memory model (Rust, Go, Java 9+); `auto`, lambdas, range-for (adopted into C23 as `auto`, into
Java/C#); exceptions (Java, C#; both later hit the same "when to use" arguments); operator
overloading; the STL iterator/algorithm separation (Rust iterators, Swift sequences); smart
pointers (Rust `Box`/`Rc`/`Arc` are `unique_ptr`/`shared_ptr` with checked ownership).

Taken from others: Simula 67 (classes, virtual functions, inheritance), ALGOL 68 (operator
overloading, references, declarations anywhere), Ada (generics as the model for templates,
exceptions, namespaces as packages), CLU (exceptions, iterators), ML/Haskell (concepts resemble
type classes; `std::optional`/`variant`/`expected` are ADTs by other names), C (everything
else, including the problems).

## Lessons for Sisu
- **Adopt**:
  - *RAII with move semantics and deterministic destruction*, as the resource model, not an
    add-on. Rust proves it composes with ownership checking; it is the only leak-free model
    without a GC.
  - *The zero-overhead principle as a written, enforced rule* for every abstraction Sisu ships:
    if a feature cannot be free when unused, it goes into a library or is rejected. C++'s two
    violations (exceptions, RTTI) became its two most-disabled features.
  - *Value semantics by default* with explicit references; never Java-style hidden reference
    semantics.
  - *The C++11 memory model* for atomics, verbatim. It is the industry standard and every other
    language converged on it.
  - *Compile-time evaluation designed in from day one* (`constexpr`-class capability) rather
    than discovered as a template accident; reflection should be in the first version, not the
    fourteenth.
  - *Non-owning view types* (`span`, `string_view`) as the only way to pass "pointer plus
    length".
- **Adapt**:
  - *Generics must be constrained by construction*: concepts/traits are the only way to write
    generic code; no unconstrained duck-typed templates, so instantiation-time errors cannot
    exist. C++'s 15-year delay getting concepts and the remaining diagnostic length show the
    cost of doing this second.
  - *Error handling*: take P0709's idea (a statically typed error returned on the normal path,
    with `throws`/`try` sugar so that constructors and operators can fail) rather than C++'s
    dynamic exceptions, and do *not* also ship error codes and a library `expected` as parallel
    channels. One mechanism, visible in the signature, zero-overhead on the happy path.
  - *Modules must own the build graph*: the compiler (or a first-party driver) performs
    dependency scanning, caches interfaces and builds the DAG; third-party build systems
    integrate with that, not the other way round. C++'s six-years-and-counting is the
    cautionary tale.
  - *Safety profiles are the right instinct in the wrong place*: the idea of a menu of
    guarantees (bounds, type, initialisation, lifetime) is sound, but in Sisu the safe subset
    must be the *default* and the baseline from version 1, with `unsafe` as the opt-out, as
    Safe C++ proposed. Retrofitting safety as opt-in has not converged in C++ after roughly a
    decade of Core Guidelines work.
  - *ABI*: separate the stable, versioned, C-shaped export ABI from the internal ABI, which is
    explicitly allowed to change between compiler versions. Do not promise std-library layout
    stability; make the dual-ABI/linking story a designed feature instead of a 2015-style
    emergency.
  - *Governance*: a small core design team with a public RFC process and a published direction
    document that is *binding*, not advisory. "150 cooks" and a three-year train optimised for
    shipping features is the documented failure mode.
- **Avoid**:
  - *Being a superset of C* (or of anything). Every C-compatibility rule in D&E (implicit
    conversions, array decay, preprocessor, `int` promotion) is a current C++ weakness.
  - *Multiple initialisation syntaxes, user-defined implicit conversions, and implicit
    narrowing.*
  - *Dynamic exceptions with RTTI-based matching and heap-allocated exception objects.* The 52%
    ban rate is the verdict.
  - *Textual inclusion.* Even C++ gave up on it; it just could not finish the job.
  - *Freezing the standard library's ABI* and then adding renamed replacements (`flat_map` for
    `map`, `hive`, `jthread` for `thread`) instead of fixing the originals.
  - *Dual error-reporting overloads* in the standard library.
  - *Library-first sum types and optionals.* `std::variant`/`std::optional`/`std::expected`
    without language pattern matching are the clearest example of a feature that is only usable
    when it is syntax.
  - *Shipping a coroutine mechanism without a scheduler, task type and cancellation model.*
- **Open questions raised**:
  - *Lifetime safety without annotations?* Sutter bets yes (profiles), Baxter and Rust bet no.
    The profiles result (2028-2029 CD/DIS) will be the experiment; Sisu should assume
    annotations are needed for cross-function lifetimes but should minimise them with elision
    rules and local inference, and should measure how far whole-program analysis within a
    module can go.
  - *Can "throws returning a value" unify exceptions and result types?* P0709 was never
    adopted, so there is no production evidence either way; Swift's typed throws (Swift 6) is
    the closest live experiment and should be studied by the Swift researcher.
  - *How much monomorphisation?* Templates buy C++ its performance and cost it compile time and
    binary size; Rust inherited both. Sisu needs a stated policy (monomorphise by default with
    opt-in dictionary passing? the reverse?) rather than discovering it.
  - *Contracts*: C++26 adopted them amid "disquiet over their value". Whether contracts are
    runtime assertions (Ada), a proof surface (SPARK), or an optimiser hint (and whether
    violating one is UB, erroneous behaviour, or a panic) is unresolved in C++ and must be
    decided explicitly in Sisu.
  - *Successor economics*: Carbon has slipped years with Google funding; Circle is one person;
    cpp2 is a side project. A language that must interoperate with C++ at the template level
    pays for a C++ front end. Sisu should decide early whether "C++ interop" means C ABI only
    (cheap, limiting) or Clang-backed header import (expensive, decisive for adoption).

## Sources
1. B. Stroustrup, *The Design and Evolution of C++*, Addison-Wesley, 1994 (p. 207 quote
   confirmed via https://en.wikiquote.org/wiki/Bjarne_Stroustrup).
2. B. Stroustrup, "Remember the Vasa!", P0977R0, 2018.
   https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2018/p0977r0.pdf
3. H. Hinnant, R. Orr, B. Stroustrup, D. Vandevoorde, M. Wong, "Direction for ISO C++",
   P2000R4, Oct 2022. https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2022/p2000r4.pdf
4. H. Sutter, "Zero-overhead deterministic exceptions: Throwing values", P0709R4, Aug 2019.
   https://www.open-std.org/JTC1/SC22/wg21/docs/papers/2019/p0709r4.pdf
5. Boost.Outcome experimental `std::error` implementation (P1028/P1095 lineage):
   https://www.boost.org/doc/libs/1_87_0/libs/outcome/doc/html/experimental.html
6. Google C++ Style Guide, Exceptions section:
   https://google.github.io/styleguide/cppguide.html (fetched Oct 2026)
7. H. Sutter, "C++26 is done! Trip report: March 2026 ISO C++ standards meeting (London
   Croydon, UK)", 29 Mar 2026.
https://herbsutter.com/2026/03/29/c26-is-done-trip-report-march-2026-iso-c-standards-meeting-london-croydon-uk/
8. H. Sutter, "Trip report: June 2026 ISO C++ meeting (Brno, Czechia)", 13 Jun 2026.
   https://herbsutter.com/2026/06/13/brno-trip-report/
9. WG21 convenor's report N5056, 28 Jul 2026 (C++26 in DIS ballot).
   https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2026/n5056.pdf
10. DevClass, "Contracts are in C++26 despite disquiet over their value", 1 Apr 2026.
https://www.devclass.com/development/2026/04/01/contracts-are-in-c26-despite-disquiet-over-their-value/5213555
11. P. Bindels, "A proposed plan for profiles in C++", P4186R0, 13 Apr 2026.
    https://open-std.org/JTC1/SC22/WG21/docs/papers/2026/p4186r0.html
12. H. Sutter, "Core safety profiles", P3081R2 (2025):
    https://isocpp.org/files/papers/P3081R2.pdf ; B. Stroustrup, "A type-safety profile",
    P3984R0 (2025).
13. S. Baxter, "Safe C++", P3390R0 / draft D3390 (2024). https://safecpp.org/draft.html
14. H. Sutter, "Trip report: November 2024 ISO C++ standards meeting (Wroclaw, Poland)" (SG23
    priority poll 19:11:6:9). https://herbsutter.com/2024/11/
15. "State of C++ 2026" (devnewsletter; secondary source for Safe C++ abandonment, VS 2026
    defaults, JetBrains 2025 figures, vcpkg port count, Activision build times):
    https://devnewsletter.com/p/state-of-cpp-2026/
16. CMake `cmake-cxxmodules(7)` manual (compiler/generator requirements for modules and `import
    std`): https://manpages.debian.org/unstable/cmake/cmake-cxxmodules.7.en.html
17. C++ Alliance, "C++20 modules and Boost: a prototype" (2025):
    https://www.cppal-dev.boost.cppalliance.org/news/c20-modules-and-boost-a-prototype/
18. JetBrains, "How to address 7 major C++ pain points with CLion" (reporting ISO C++
    Foundation 2023 survey figures), Jul 2023.
    https://blog.jetbrains.com/clion/2023/07/how-to-address-7-major-cpp-pain-points-with-clion/
19. ISO C++ Foundation, "Results summary: 2026 Annual C++ Developer Survey 'Lite'", May 2026
    (page returned 403 during research). https://isocpp.org/blog/2026/05/2026-survey-summary
20. C. Jabot, "The Day The Standard Library Died" (Prague 2020 ABI polls), 2020.
    https://cor3ntin.github.io/posts/abi/
21. T. Winters, "ABI - Now or Never", P1863 (2019-2020) and "What is ABI, and What Should WG21
    Do About It?", P2028 (2020); paper numbers from memory, not re-fetched.
22. C. Carruth, "There Are No Zero-cost Abstractions", CppCon 2019 (unique_ptr in registers);
    from memory.
23. Carbon project milestones and roadmap:
    https://docs.carbon-lang.dev/docs/project/milestones.html ;
    https://github.com/carbon-language/carbon-lang/blob/trunk/docs/project/roadmap.md ; "Safety
    milestones and a 2025 roadmap" (P4880): https://docs.carbon-lang.dev/proposals/p4880.html
24. The New Stack, "Can the Safe C++ Proposal Copy Rust's Memory Safety?" (2024; Circle
    background): https://thenewstack.io/can-the-safe-c-proposal-copy-rusts-memory-safety/
25. ONCD, "Back to the Building Blocks", 26 Feb 2024; CISA/FBI roadmap guidance; Microsoft,
    Chromium and Android memory-safety data: see `c.md` sources 13-16.
26. Stack Overflow Developer Survey 2025: https://survey.stackoverflow.co/2025/technology ;
    TIOBE Index Sep 2026: https://www.tiobe.com/tiobe-index/
27. B. Stroustrup, "Concepts: The Future of Generic Programming", P0557R0 (2017); A. Sutton et
    al., "Wording Paper, C++ extensions for Concepts", P0734R0 (2017).
    https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2017/
28. Wikipedia, "C++26" (feature list, 28 Mar 2026 completion, trivial relocatability removal):
    https://en.wikipedia.org/wiki/C%2B%2B26


