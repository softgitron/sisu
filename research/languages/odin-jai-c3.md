# Odin, Jai and C3

Three "C successors that reject both C++ and Rust", treated together because they share a
position: manual memory with better defaults (zero-initialisation, arenas, bounds-checked slices,
explicit allocators), errors as values, no RAII, no borrow checker, and a stated preference for
the programmer's judgement over compiler-enforced safety. This file states that counter-argument
as its proponents make it and assesses the evidence.

## Snapshot
| | Odin | Jai | C3 |
|---|---|---|---|
| First appeared | July 2016 | Design started 2014; closed beta from about Jan 2020 | About 2019 |
| Designed by | Bill Hall ("gingerBill") | Jonathan Blow (Thekla, Inc.) | Christoffer Lernö |
| Current version (Oct 2026) | `dev-2026-10` (6 Oct 2026); monthly `dev-YYYY-MM` tags; no 1.0 (a July 2026 newsletter item claiming "Odin 1.0" could not be confirmed on odin-lang.org or the release list) | Closed, invite-only beta; still not public as of Oct 2026; Blow's game *Order of the Sinking Star*, written in Jai, shipped 8 Oct 2026, and Thekla "has plans to release the programming language and engine for free after the game's released" | 0.8.4 (11 Sep 2026, "latest"); 0.8.5 pre-release 10 Oct 2026; roadmap: 0.9 on 2027-06-01, 1.0 on 2028-06-01 [23] |
| Governance | BDFL (Bill Hall); no foundation; no RFC process | Single company, closed development; community wiki and Discord | BDFL (Lernö); public GitHub; yearly 0.x releases, minor releases compatible within a series |
| Typing | Static, strong, "distinctly typed", few implicit conversions; parametric polymorphism with `$T`; no methods, no interfaces | Static; polymorphic procedures with `$T`; types as values at compile time | Static; C's type system plus generic modules, distinct types, optionals; `any` dynamic interfaces |
| Memory | Manual; implicit `context.allocator`/`temp_allocator`; bounds checks on by default; zero-initialised variables | Manual; implicit context allocator; `---` to skip initialisation | Manual; explicit allocators and temp allocator; ZII ("Zero Is Initialization") |
| Compilation | AOT via LLVM | AOT; own x64 backend (250k lines/s measured on the beta; goal 1M lines/s) plus LLVM | AOT via LLVM; full C ABI |
| Primary domains | Games, graphics tools, simulation | Games (engine and game for Thekla) | "General-purpose applications", the space of C++/Swift/Java per the designer's 2026 post |
| Notable users | JangaFX (EmberGen, LiquiGen, IlluGen, GeoGen), Spall profiler, OLS language server, a dozen Steam games | Thekla's engine and *Order of the Sinking Star* (Switch 2, PS5, Windows); beta users shipping on Steam | Small; the designer gives no named users |

## Stated design goals

**Odin.** The home page: "The Data-Oriented Language for People Who Ship"; "Odin is the C
alternative for the Joy of Programming"; "simplicity ... readability, scalability, and
orthogonality of concepts"; "clear is better than clever"; "low-level control over the memory
layout" [1]. The overview: "The design goals of Odin were explicitness and simplicity" [2]. The
FAQ: "We believe that data and code should be separate concepts; data should not have
'behaviour'"; "Odin will allow the programmer to do unsafe things if they so wish" [3].

**Jai.** The community-maintained philosophy page (Blow has said the older "JaiPrimer" does not
describe Jai well; the quotations below are from the beta community wiki) [4]: "Explicit
Control, nothing should happen invisibly behind your back"; "You should feel the joy of
programming new and interesting functionality"; "Programming should not be a matter of filling
out income tax forms all day"; "The RAII acronym makes no sense"; "Garbage collection causes a
lot of friction in games"; "The problem is that Rust does not have respect for the kinds of
things you need to do in a game engine" (attributed to Blow); "If I got rid of full arbitrary
compile time execution, it wouldn't be the same programming language" (Blow). His 2014
motivation, per Wikipedia: C++ "makes [software development] a lot harder than it should be" [5].

**C3.** The design-goals page, verbatim: "Procedural language, with a pragmatic ethos to get
work done. Minimalistic, no feature should be unnecessary or redundant. Stay close to C - only
change where there is a significant need. Learning C3 should be easy for a C programmer.
Seamless C integration. Ergonomic common patterns. Data is inert. Zero Is Initialization (ZII).
Avoid 'big ideas'." [6]. The home page: "an evolution, not a revolution: the speed of C, shaped
for modern software" [7].

## Strengths

### Odin's implicit `context`: dependency injection for allocation, logging and randomness
Every Odin-convention procedure receives a hidden pointer to a `Context` struct holding
`allocator`, `temp_allocator`, `logger`, `random_generator`, `assertion_failure_proc`,
`user_ptr`, `user_index` and a reserved `_internal` field [2][8]. Bill's 2025 explanation of the
purpose: "to intercept third-party code, and to change how it does things" (how it allocates,
logs, asserts), the problem that C libraries addressed with `MY_MALLOC` macro overrides that
"fail across LIB/DLL boundaries" [8]. Design details that matter: it is passed as a parameter
rather than a thread-local (works across DLLs, freestanding targets, fibers and async code); it
has copy-on-write scoping so a callee's changes do not propagate back; its layout is fixed and
not user-extensible so the ABI is stable [8]. Non-Odin calling conventions must set `context =
runtime.default_context()` explicitly [2]. The idea comes from Jai's context [8] and is the
implicit counterpart of Zig's explicit `Allocator`/`Io` parameters. Bill's own caveat: it is
"Odin's most misunderstood feature", people mark procedures `contextless` because they "don't
actually know if the code needs it or not", and generic explicit allocator passing in his C
experience "led to more allocation bugs" [8].

### Error values with `or_return`/`or_else`, and C3's Optionals
Odin: procedures return multiple values; the last may be an error enum/union; `x := f() or_return`
pops the final value, checks it and returns early (all results must be named for a bare
`return`); `v := m["k"] or_else default` and `.?` for `Maybe(T)` [2]. Bill's 2018 position:
"There will never be software exceptions in the traditional sense. I hate the entire philosophy
behind the concept"; catch-all blocks mean "you cannot know where the error comes from"; and he
criticises Go's `if err != nil` for encouraging passing errors up the stack: "Handle errors there
and then and don't pass them up the stack. You make your mess; you clean it" [9]. `or_return`,
added later, is the admission that propagation is needed anyway.

C3: a function returning `int?` returns either a value or an "excuse" (a `fault` value, e.g.
`io::FILE_NOT_FOUND~`); `x!` rethrows; `??` supplies a default; `if (catch err = x)` and
`if (try v = x)` branch; using an Optional in an expression makes the result Optional, and a
call with Optional arguments short-circuits to the first excuse [10]. The C ABI mapping (excuse
in the return register, result by reference, pointer-sized `c3fault_t`) is what the site calls
"zero overhead errors" [7][10]. Unifying "no value" and "error" into one type with a mandatory
reason is a genuinely different design from Zig's `E!T` and Rust's `Result`.

### Zero-is-initialisation and "make the zero value useful"
All three initialise to zero by default with an explicit opt-out (`---`). Odin's FAQ: constructors
are unnecessary because "the zero value is meant to be useful" [3]; C3 lists ZII as a goal [6];
Jai's `---` skips initialisation "for performance" [11]. Bill's 2026 claim that zero-initialisation
"has worked out really well" and that null is "empirically the easiest class of invalid memory
addresses to catch at runtime" [12] is the designer's case against non-null pointer types; see
Weaknesses for the counter-evidence.

### Built-in structure-of-arrays and array programming (Odin)
`#soa` arrays and slices, `soa_zip`/`soa_unzip`, `append_soa`, fixed-size array arithmetic and
swizzling are language features aimed at SIMD/data-oriented layouts [2]. Jai had a built-in
`SOA` keyword and removed it: the community wiki records that Struct-of-Arrays "was removed as a
built-in feature" and is "now supported through `#insert` directives" because it was "too narrow
for a general language feature" [4][11]. That is a useful data point: SoA is valuable but the
right level may be metaprogramming, not syntax.

### Compile-time execution as the build system (Jai) and semantic macros (C3)
Jai's `#run` executes arbitrary code during compilation, exposes the compiler's message loop and
AST to user code, and the build is itself a procedure run via `#run`: "all you would ever need to
compile your code is the compiler itself, no external dependencies" [4][11]. C3 has `$if`/`$foreach`
compile-time constructs, macros that "read like functions" with lazy parameters and trailing
bodies, and compile-time and runtime reflection [7][13]. Odin deliberately has none: "If Odin Had
Macros" (2025) answers "No", because "every problem I came across, I find a better solution that
could be solved with a different language construct or idiom" and "giving into one whim does
lead to giving into another whim" [14]. The three bracket the design space: full CTFE (Jai),
constrained macros (C3), none (Odin).

### Gradual contracts (C3)
`@require`, `@ensure`, `@param [in|out|inout|own|drop|init]`, `@pure`; checked by runtime
asserts in safe mode, reported at compile time when constant-folded, and "violating a contract
is unspecified behavior, so a compiler may optimize as if the contract always holds"; "a
barebones compiler may ignore contracts entirely" [15]. 0.8.4 added parameter-pinpointing for
`@require` failures [16]. This is the closest any of the three comes to a machine-checkable
safety story, and it is explicitly optional.

### Explicitness, and production evidence that it ships
Odin has explicit procedure groups instead of implicit overloading (it "complicates the scoping
system"), no operator overloading ("very easily abused"), no UFCS, no methods [2][3]; Jai:
"nothing should happen invisibly behind your back" [4]; C3: "Data is inert" [6]. JangaFX ships
four commercial VFX tools in Odin [1] and *Order of the Sinking Star* shipped on three platforms
in Jai [5]: real products from small expert teams, the population these languages target, though
neither is safety-critical or multi-million-line.

### No build system, no package manager, by design (Odin)
`odin build .` builds a project; linking lives in source via the `foreign` system; the FAQ
recommends "copying and vendoring each package manually, and fixing the specific versions down"
[17]. Bill's 2025 essay: package managers automate "dependency hell" ("not everything needs to be
automated, especially hell"); "each dependency is a potential liability" [17].

## Weaknesses

### No memory safety, and the argument against ownership is not supported by data
Bill's "The Fatal Flaw of Ownership Semantics" (2020) [18] is the most developed written case
against the borrow checker from this camp. Its claims: ownership semantics and OOP are duals
("a linear value hierarchy of responsibility" versus "of behaviour"); "data is more commonly
graph-like (non-linear) than tree-like (linear) for most problems"; the borrow checker's affine
logic "cannot be used to express non-linear problems", so Rust programmers "bypass the borrow
checker" with `Rc`, `RefCell` and indices; use-after-free is "usually a symptom of another larger
problem"; the alternative is subsystems that hand out generational handles and "less
responsibility to begin with" (POD data, useful zero values). Assessment: the first two points
are real and measurable (Rust's own survey: 9.5% "big problem" plus 42.8% "could be improved"
for "borrow checker not allowing valid code" [see rust.md]). The third is asserted without data,
and the Android dataset points the other way for large codebases: memory-safety bugs fell from
76% to under 20% of vulnerabilities as new code moved to Rust [see rust.md]. Generational handles
are a runtime check (stale handle = sentinel + error) only as safe as the discipline to use them
everywhere. None of the three languages has published defect data.

### Nullable pointers by default
Odin keeps `nil` pointers ("a very rare problem" [12]); non-nullability "is only achievable
through maybe types or explicit initialization of every element", and enforcing initialisation is
"quite a high cost" [12]. Bill's broader claim that Hoare's billion-dollar figure is "close to a
rounding error" next to the performance cost of "the individual-element mindset" [12] is an
opinion, not a measurement. Jai has no null-safety story. C3 relies on ZII and `[&in]` contract
annotations. Every managed and most modern systems languages (Swift, Kotlin, Rust, Zig, Dart,
TypeScript-strict) went the other way, which is the adoption evidence against this position.

### No traits/interfaces, weak constraints on generics
Odin: "there is no separate entry on interfaces or traits"; methods are rejected as a "rabbit
hole" of dispatch/inheritance/interface questions [3]; `where` clauses on parapoly procedures are
the only constraint mechanism [2]. Jai: polymorphic procedures, no interfaces. C3: generic
modules with contract blocks "to improve compile errors", plus `any`/dynamic interfaces [13].
Compared with Rust traits, all three give worse error messages at generic call sites and no way
to document "what a `$T` must provide" that tools can check; this is the same weakness documented
for Zig (`anytype`), and Zig's community thread on it applies here [see zig.md].

### Jai: closed, unverifiable, dependent on one studio
Twelve years after the first talk the compiler is still invite-only [4]; in July 2024 Blow said
Thekla "could not afford to pay employees to work on the Jai compiler" after *Braid, Anniversary
Edition* sold poorly [5]; documentation is community-written and Blow has disowned the best-known
primer [11]. Outsiders cannot evaluate the design beyond user reports ("jai is what c++ should
have been", a 2026 beta user [19]). On concurrency the wiki says Blow is unsure how Jai should
handle it and offers "only basic threading primitives" [4].

### Odin: no stability promise, no package manager, no 1.0 date
Releases are monthly `dev-` tags; the FAQ says the language "is pretty much done" while the
compiler, toolchain and core library are not, and self-hosting waits for "after version 1.0" and
a specification [3]. `core:os` was being replaced in late 2025 with a transition period [20]:
"done" applies to syntax, not the library. Vendoring pushes updates and security patching onto
every user with no tooling; the supply-chain risks Bill names are real, but Rust's answer
(immutable registry, lockfile, audits) addresses them without giving up automation.

### C3: churn before the freeze, and a shifting identity
0.8.4 (Sept 2026) renamed `faultdef` to `excuse`, `attrdef` to `attrmacro`, `constdef` to
`constset` "ahead of the 0.9.0 syntactic freeze"; 0.8.0 re-ordered `??`/`?:` precedence and
"kill[ed] off half the builtins" [16]. "Unsigned sizes: a five year mistake" (May 2026) made sizes
signed, removed implicit signed/unsigned conversions, and concludes "casts should mean something"
[21]. "I thought I was building a C replacement. I was wrong" (Aug 2026) re-positions C3 from "a
C-like for people who like C" to "refreshing C to make it a pleasant general-purpose application
language again", because "people write databases in C, not their next video editor or game"; it
also concedes C3 "has methods because they avoid the need for function overloading" [22]. The user
base is small and unnamed.

### Context pitfalls (Odin, Jai)
Copy-on-write scoping means "changes to `context` inside a loop are not visible as expected" [2];
C callbacks must re-establish a context; every Odin-convention call carries an extra pointer; and
the designer says the feature works "*because* people misunderstand it, and thus generally leave
it alone" [8]. Jai's context "allows custom fields and back-propagation" [8], trading ABI stability
for flexibility. Neither has a static story for which procedures need which context fields.

### Error propagation and concurrency are thin
Odin's `or_return` works only on the final return value and forces named results [2]; there is
no error trace or standard context chaining. C3's excuse is one pointer-sized fault value with
no payload (inferred from the ABI description [10]). Jai documents no public error model. On
concurrency: Odin has threads and `core:sync`, Jai "basic threading primitives" [4], C3 threads
and channels; none has data-race freedom, structured concurrency or an async model. Next to
Zig's `std.Io` or Rust's `Send`/`Sync`, this is where "trust the programmer" has least to show.

## Designer retrospectives

- **Bill Hall (Odin):** exceptions, never (2018) [9]; ownership semantics are "a duality" of OOP
  with a linear-hierarchy flaw (2020) [18]; macros, "No" (2025) [14]; package managers "automate
  dependency hell" (2025) [17]; `context` exists to intercept third-party code (2025) [8]; null
  was not a billion-dollar mistake and Odin "could not 'just add' non-nil pointers" (2026) [12].
  He has published nothing he would change in Odin itself; the `core:os` rewrite is the nearest
  admission.
- **Jonathan Blow (Jai):** the 2024 funding statement [5]; the removal of built-in SoA in favour
  of `#insert` [4]; "Temperance": compile-time execution "should be used conservatively because
  it runs slower than machine code" [4]; uncertainty about concurrency [4].
- **Christoffer Lernö (C3):** unsigned sizes were "a five year mistake" [21]; C3 was marketed
  wrong [22]; half the builtins were unnecessary [16]; methods were kept for overloading
  avoidance despite preferring procedural style [22].

## Influence

Jai (2014-) is upstream of the family: the implicit context, `using`, `#run` compile-time
execution, `---`, SoA layouts and the "joy of programming" rhetoric all reappear in Odin (Bill
names Jai's context as the inspiration [8]); Zig's `comptime` is in the same lineage. Odin took
`defer`, multiple returns and directory packages from Go, pointer syntax from Pascal, array
programming from APL-family languages; Lernö credits Odin with "getting programmers into a
better mindset" [22]. C3 took `defer`, slices, `!` error propagation and compile-time constructs
from Zig and Odin, and contracts from Eiffel/Ada/D. Outward influence is limited so far: no
mainstream language has copied `or_return`, the fixed-ABI context, or C3's unified
optional/excuse type.

## Lessons for Sisu

- **Adopt**
  - Zero-is-initialisation with an explicit opt-out, and the discipline of designing types so
    the zero value is valid. It removes the constructor problem without a language feature.
  - `defer`-style visible cleanup, SoA as a library/metaprogramming facility rather than
    syntax (Jai's reversal), and bounds-checked slices as the default array type.
  - The "intercept third-party code" requirement that `context` solves: Sisu's standard library
    must let a caller substitute allocator, logger, assertion handler and entropy for code it
    does not own, across dynamic-library boundaries.
  - Gradual contracts in the style of C3: optional, checked in safe builds, available to the
    optimiser, with pinpointed failures.

- **Adapt**
  - Implicit `context` versus Zig's explicit parameters: carry the context as a typed,
    statically tracked capability (an effect or compiler-known implicit parameter) so that
    "does this procedure need the allocator?" is answered by the type system, not convention.
  - C3's single Optional type with a mandatory reason is attractive; Sisu should keep "absent"
    and "failed" distinguishable while sharing one propagation operator, and allow a payload.
  - `or_return`: adopt the operator, but on a proper sum type rather than on "the last of N
    return values".
  - Generational handles and arenas: provide them in the standard library as a first-class
    strategy, since skilled practitioners reach for them; do not make them the only safety story.

- **Avoid**
  - Nullable pointers by default. The designers' case is an argument from taste against the
    whole industry's movement and against the Android data.
  - Closed development (Jai): a language whose design cannot be examined cannot be adopted,
    funded or specified.
  - No package manager: the supply-chain concern is valid, the remedy (manual vendoring) is
    worse. Use an immutable registry, lockfiles and audit tooling instead.
  - Relying on "trust the programmer" for concurrency; none of the three has a credible
    data-race story.

- **Open questions raised**
  - Is the counter-argument right for its niche? For small expert teams writing games and
    tools, the JangaFX and Thekla results suggest manual memory management with good defaults
    is productive and shippable. Whether that generalises to large teams or hostile input is
    exactly what no one in this camp has measured.
  - How much of Rust's borrow-checker friction is intrinsic to safety and how much is the
    choice of first-class references and named lifetimes? Bill's graph-versus-tree argument
    targets the latter; Sisu could keep safety and lose the tree constraint (regions, handles,
    mutable value semantics) if a sound design exists.
  - Where does compile-time execution stop? Jai (anything), C3 (macros with constraints),
    Odin (nothing) all have shipped products; Blow's "Temperance" note and Bill's "slippery
    slope" argument both say the cost is in unbounded use, not in the feature itself.

## Sources
1. Odin home page (goals, showcase including JangaFX products). https://odin-lang.org/
2. Odin Language Overview (context system, `or_return`/`or_else`, `#soa`, `using`, procedure groups, `defer`, parametric polymorphism, `where`, `Maybe`, bounds checking). https://odin-lang.org/docs/overview/
3. Odin FAQ (no exceptions/constructors/destructors/methods/operator overloading/UFCS; self-hosting after 1.0). https://odin-lang.org/docs/faq/
4. Jai Community Library wiki: home, "Philosophy of Jai", "Overview" (beta-member maintained; Blow quotations as given there), accessed Oct 2026. https://github.com/Jai-Community/Jai-Community-Library/wiki
5. Wikipedia, "Jai (programming language)" / "Jonathan Blow" (timeline, July 2024 funding statement) and "Order of the Sinking Star" (released 8 Oct 2026; Thekla's plan to release the language). https://en.wikipedia.org/wiki/Jai_(programming_language) and https://en.wikipedia.org/wiki/Order_of_the_Sinking_Star
6. C3, "Design goals". https://c3-lang.org/getting-started/design-goals/
7. C3 home page. https://c3-lang.org/
8. Bill Hall, "context - Odin's Most Misunderstood Feature", 15 Dec 2025. https://www.gingerbill.org/article/2025/12/15/odins-most-misunderstood-feature-context/
9. Bill Hall, "Exceptions - And Why Odin Will Never Have Them", 5 Sept 2018. https://www.gingerbill.org/article/2018/09/05/exceptions---and-why-odin-will-never-have-them/
10. C3, "Optionals (essential)". https://c3-lang.org/language-common/optionals-essential/
11. BSVino, "JaiPrimer" (community document based on Blow's 2014-2015 talks; Blow has said it does not describe Jai well). https://github.com/BSVino/JaiPrimer/blob/master/JaiPrimer.md
12. Bill Hall, "Was it really a Billion Dollar Mistake?" (2 Jan 2026) and "Mitigating the Billion Dollar Mistake" (11 Jan 2026). https://www.gingerbill.org/article/2026/01/02/was-it-really-a-billion-dollar-mistake/ and https://www.gingerbill.org/article/2026/01/11/mitigating-the-billion-dollar-mistake/
13. C3, "Generics" (generic modules) and "Comparisons with other languages". https://c3-lang.org/generic-programming/generics/ and https://c3-lang.org/faq/compare-languages/
14. Bill Hall, "If Odin Had Macros", 31 Jul 2025. https://www.gingerbill.org/article/2025/07/31/if-odin-had-macros/
15. C3, "Contracts". https://c3-lang.org/language-common/contracts/
16. C3 blog: "0.8.0 The Core Language Is Settling" (14 May 2026), "0.8.4 The last renaming" (18 Sept 2026), "Let's kill off half the builtins in 0.8.0" (9 May 2026). https://c3-lang.org/blog/
17. Bill Hall, "Package Managers are Evil", 8 Sept 2025, and the Odin FAQ on vendoring. https://www.gingerbill.org/article/2025/09/08/package-managers-are-evil/
18. Bill Hall, "The Fatal Flaw of Ownership Semantics", 21 Jun 2020. https://www.gingerbill.org/article/2020/06/21/the-ownership-semantics-flaw/
19. demonkingswarn, "I am really enjoying the Jai language", 30 Apr 2026 (beta user report). https://demonkingswarn.is-a.dev/blog/2026/04/30/i-am-really-enjoying-the-jai-language
20. Odin news, "Moving Towards a New core:os", 31 Oct 2025, and the Odin GitHub releases page (`dev-2026-10`, 6 Oct 2026). https://odin-lang.org/news/ and https://github.com/odin-lang/Odin/releases
21. Christoffer Lernö, "Unsigned sizes: a five year mistake", 2 May 2026. https://c3-lang.org/blog/unsigned-sizes-a-five-year-mistake/
22. Christoffer Lernö, "I thought I was building a C replacement. I was wrong", 16 Aug 2026. https://c3-lang.org/blog/i_thought_i_was_building_a_c_replacement/
23. C3 roadmap (0.9 on 2027-06-01, 1.0 on 2028-06-01) and c3c GitHub releases (0.8.4, 11 Sept 2026). https://c3-lang.org/getting-started/roadmap/ and https://github.com/c3lang/c3c/releases
