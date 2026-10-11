# Carbon

## Snapshot
| | |
|---|---|
| First appeared | Announced publicly 19 July 2022 (CppNorth, Chandler Carruth); design work inside Google from 2020 |
| Designed by | Carbon leads: Chandler Carruth, Kate Gregory, Richard Smith ("zygoloid"); Google-funded core team plus open contributors |
| Current version (Oct 2026) | No release. Nightly toolchain builds ("very experimental ... many things don't yet work"); 0.1 (MVP for evaluation) targeted "at the soonest, the end of 2026" and described as "very ambitious" [3][4]; the older Explorer interpreter is archived |
| Governance | Three leads decide; a "painter" (Carruth) settles taste; written proposal process with community RFC, blocking issues resolved by leads with a quorum of two; annual roadmap proposals with retrospectives; a foundation is a "potential" 2027-2028 goal [5][3] |
| Typing | Static; checked generics (definition-checked against interfaces) and template generics (C++-style, instantiation-checked); nominal interfaces; facets and named constraints |
| Memory | Undecided in detail. Planned: compile-time temporal and mutation safety via the type system, runtime spatial checks, three build modes (debug / performance / hardened), "erroneous behavior" instead of undefined behaviour in unsafe code [6][7] |
| Compilation | AOT via LLVM; the toolchain is designed to be used "as a drop-in Clang C++ toolchain" so that mixed C++/Carbon builds have one compiler [4] |
| Primary domains | Existing large C++ codebases (performance-critical software) |
| Notable users | None; "Carbon is not ready for use" [1] |

## Stated design goals
Carbon's `goals.md` lists seven goals and says that when they conflict
"we intend to prioritize them in this order": (1) performance-critical
software ("idiomatic code should be fast"); (2) software and language
evolution over decades; (3) code that is easy to read, understand and write
("human reading, understanding, and writing of software, in that order");
(4) practical safety and testing mechanisms ("make unsafe or risky aspects
of Carbon code explicit and syntactically visible"); (5) fast and scalable
development ("syntax should parse with bounded, small look-ahead", separate
compilation); (6) modern platforms only; (7) interoperability with and
migration from existing C++ [2]. The README is blunt about why the language
exists: existing modern languages "Go, Swift, Kotlin, Rust, and many more"
already "provide an excellent developer experience", but their designs
"create significant barriers to adoption and migration from C++" [1].
Carbon is "an experiment to explore a possible, distant future for the
C++ programming language" [2], and the project says it will stop if the
experiment fails.

## Strengths

### Written, prioritised goals and a documented decision process
Every substantive change is a proposal PR with a template that requires
"alternatives considered"; proposals are numbered by PR and archived (the
highest at the time of writing is #7697); leads must explain deferrals and rejections; annual
roadmaps come with retrospectives on the previous year [5][3]. The
prioritised goal list is used as a tie-breaker in debate. This is the most
thorough governance design of any language in this group, and it was in
place *before* the first line of compiler code. The 2023 retrospective
calling 2022's roadmap "wildly optimistic" [3] is evidence the process
produces honest self-assessment, not just documentation.

### The successor strategy, with the reasoning written down
Carbon's "difficulties improving C++" document argues the ISO process is
"aimed at standardization rather than design", runs on multi-year cycles,
charges for participation, decides by live in-person votes, and could reach
"no definitive conclusion" on ABI breakage; and that C++'s commitment to
backward compatibility means features are layered, never replaced [8].
The project therefore chose bidirectional interop (call C++ from Carbon
and Carbon from C++, including templates and inheritance) and automated
migration over a from-scratch language. The explicit position on Rust is
that it is an excellent language whose design creates barriers for
existing C++ code; Carbon's safety strategy doc records that Rust-style
borrow checking "requires design patterns that differ from C++" and would
force redesign or runtime checks on migrated code, which is why it is not
the default [7]. Whether or not Carbon succeeds, this is the clearest
published analysis of why "just use Rust" is not an answer for every C++
codebase.

### Checked generics with definition checking
Carbon has both `template` generics (C++ semantics, for interop and
metaprogramming) and checked generics. A checked generic's body "is type
checked once, using only the requirements in its signature" and call sites
are checked "against the signature alone, without reading the function
body" - the document cites "Rust's Golden Rule" [9]. Interfaces are
nominal; a type implements one with an `impl` block "at most once";
`extend impl` adds the interface's names to the class API while an
out-of-line `impl` leaves it unchanged; conflicting names are reached by
qualified access `song.(Comparable.Less)(other)`; a "facet type" is the
type-of-types that implement an interface; `&` composes interfaces and
`constraint` names a reusable combination with aliasing to resolve
collisions [9]. The design document explicitly contrasts this with C++
templates, where "errors can surface only at instantiation".

### Errors are values; no exceptions
Proposal P0301 sets the principle that errors are "sum-type return values
rather than exceptions or other non-return side channels", with explicit
propagation markers (`?`/`try` cited as examples) [10]. The alternatives
considered are recorded: a Swift-style separate error channel (rejected as
speculative and bad for interop with error-returning C++), and implicit
C++-style propagation (rejected because it "hides control flow") [10].
Carbon chose the Rust/Go model with its eyes open about C++ migration
costs.

### A safety strategy with definitions, build modes and a bar
The safety strategy doc defines memory safety (spatial, temporal), type
safety and data-race safety; distinguishes guarantees, error detection and
hardening; and fixes three build modes: **debug** (detect bugs, useful
diagnostics), **performance** (only checks with no measurable hot-path cost)
and **hardened** (attack resistance at significant cost, non-probabilistic
checks), with "the number of modes is meant to stay at three" [7]. P5914
(2025) sets the bar: temporal, spatial and type errors "rejected at compile
time or detected at runtime with fail-stop behavior"; initialisation errors
may zero-initialise; temporal-safety overhead under 1%, spatial 1-2%;
violations of unsafe-code rules become "erroneous behavior" (well-defined,
diagnosable) rather than UB; and the optimiser "won't assume" things about
C++ code that Rust assumes about `unsafe` [6]. P4880 moved memory safety
into the 0.1 milestone because of "stronger and more specific feedback from
potential users" [11].

### Syntax decisions with recorded alternatives
Carbon's proposals are a catalogue of syntax trade-offs with reasoning:
P0618 chose `var message: String` over `var String message` (parse
ambiguity with labelled parameters) and over `var String: message`
(inconsistent with every other colon language), and `for x in xs` over
`for x : xs` [12]. P0676 chose `:!` to mark generic parameters, square
brackets for *deduced* parameters, and a `template` keyword to make the
"powerful feature visible", rejecting angle brackets as "hard to parse and
heavy on punctuation" and context-dependent defaults as violating the
"low-context-sensitivity principle" [13]. P7254, one of the most recent proposals, then *replaced*
`:!` and `:?` with keywords and contextual defaults [5] - evidence that the
process can revisit its own taste decisions. Other recorded decisions:
`fn` keyword, `impls` replacing `is`, `override fn` replacing `impl fn`,
a `char` redesign, octal literals, and an orphan rule for scopes [5].

## Weaknesses

### Four years in, nothing to evaluate
The 2022 roadmap aimed to "substantially complete the core language
design"; the 2023 retrospective called that "wildly optimistic" [3]; 0.1
was then targeted for 2025 as "a lower bound that could slip" [3], and is
now "the end of 2026" at the earliest, with 0.2 "potentially" 2027-2028
and 1.0 "beyond 2028" [4]. The milestone document itself says evaluation
"can only start" at 0.1 [4]. Carbon has produced the best-documented
unfinished language in this group; whether design-first-for-years yields a
better language than Rust's or Swift's iterate-with-users approach is
exactly what the "experiment" will test, and the odds are not yet known.

### Memory safety was a later goal, and the hard part is still open
In the original goal ordering, safety is fourth, behind performance,
evolution and readability [2]; the 2025 roadmap admits the project
"hasn't focused on the safety side for several years" [3]. The current
plan is a type-system approach "broadly following Rust", with *more*
pointer kinds than Rust (shared mutable and exclusive mutable), modelling
iterator invalidation through "methods that require exclusive mutable
access", and drawing on the Ante language's shared-mutability ideas; the
exact exclusivity rules are "still to be worked out" [6]. Rust's
borrow checker took years to make sound and ergonomic; Carbon is starting
that work in 2025-2026 while also promising sub-1% overhead and gradual
adoption inside unsafe C++ [6]. This is the riskiest bet in the design.

### Interop scope starts with the easy half
The 2025 objective was "non-template C++ APIs ... prioritizing access to
C++ APIs from Carbon", with C++ calling Carbon a stretch goal [3]; templates
and coroutines are deferred, and coroutines/async are "deferred until at
least 0.2" together with threading, effects, SIMD and a cross-version ABI
mechanism [4]. The value proposition rests on template-heavy C++
interop; it is the part not yet demonstrated.

### Single-sponsor dependence and an unproven community
The core team is Google employees; a foundation "separate from funding
corporations" is a 2027-2028 "potential" goal [3]. Google's own
large-scale investment in Rust (Android, Chromium, and Crubit for C++/Rust
interop) is the obvious internal competitor to Carbon's premise, and the
project's survival depends on a sponsor whose priorities may change
before 0.1 ships. A 2026 talk titled "Carbon: graduating from the
experiment" [1] suggests the project intends to argue for continuation;
its content was not available for this research.

### The successor premise may be weakening
Carbon's reason to exist is that Rust cannot be adopted incrementally in
C++ codebases. Since 2022, C++/Rust interop tooling (Crubit, autocxx,
cxx) and government and industry guidance favouring memory-safe languages
have moved the baseline: the question evaluators will ask in 2027 is not
"Carbon vs C++" but "Carbon vs Rust-with-interop", and Carbon's safety
design will be judged against a shipped competitor.

## Designer retrospectives
- "In retrospect wildly optimistic" (2023 roadmap on the 2022 plan) [3].
- The 2024 roadmap "focuses significantly more on implementation than we
  have done in previous years, in response to feedback from people trying
  to evaluate or contribute" [3] - an admission that design-only years
  cost community engagement.
- P4880 moved safety earlier because of user feedback and conceded the
  0.1 date slips "at least a year" as a result [11].
- The safety strategy records why borrow checking is not the default
  (migration cost, compile time) and leaves the door open to "guaranteed
  safety ... adopted later with a migration path" [7], a hedge the
  designers themselves flag.
- P7254 replaces `:!`/`:?` with keywords [5]; P0676 had described the
  choice of `:!` over `:$` as "a matter of taste" [13].

## Influence
- **Copied from others**: Rust (checked generics' "golden rule", errors as
  values, `fn`/`var`/`name: Type`, exclusivity ideas); Swift (protocol
  extensions, `impls`-style conformance, the `painter` role is Swift's
  "core team taste" formalised); Go (build modes and toolchain culture);
  C++ (templates, inheritance, ABI concerns); Ante (shared mutability).
- **Copied by others**: Carbon's governance documents and prioritised
  goals are already cited as a model by newer projects; its "difficulties
  improving C++" analysis [8] is the standard reference for why successor
  languages exist. No language feature has been copied yet, because none
  has shipped.

## Lessons for Sisu
- **Adopt**:
  - A written, *ordered* list of goals used as the tie-breaker in design
    debates [2], and a proposal template that requires alternatives
    considered [5]; Carbon's proposals index is a reusable catalogue of
    syntax decisions and their rejected alternatives [12][13].
  - Checked generics with definition checking as the default, with a
    separately spelled template mode if C++-style duck typing is ever
    needed [9].
  - Errors as sum-type values with explicit propagation [10].
  - Three fixed build modes with "checks identical across modes as far as
    possible" [7], and "erroneous behavior" instead of UB for violated
    unsafe-code contracts [6].
- **Adapt**:
  - Facets and named constraints are a good vocabulary for "type that
    implements these interfaces"; Sisu should keep the concept but avoid
    Carbon's `:!` punctuation (which Carbon itself abandoned [5]).
  - Interop: Carbon's bidirectional-interop requirement is right for a
    C++ successor; Sisu, not being one, should target C ABI interop and
    treat C++ as a secondary, tooling-mediated target.
- **Avoid**:
  - Ranking safety fourth and designing it last [2][3]; Rust, Swift and
    Carbon's own 2025 reversal [11] all show safety shapes the core type
    system and cannot be bolted on.
  - Years of design with no code users can run; Carbon's retrospectives
    document the engagement cost [3].
- **Open questions raised**:
  - Successor-with-interop vs from-scratch: Carbon is the test case and
    the result will not be known before 2027. Sisu, as a from-scratch
    design, should watch whether Carbon's "automated migration then
    incremental refactoring" [6] produces safer code than rewrites.
  - Can "more pointer kinds than Rust" (shared mutable, exclusive
    mutable) give temporal safety at <1% overhead without a full borrow
    checker [6]? This is unproven anywhere.

## Sources
1. carbon-language/carbon-lang, README ("Why build Carbon?", status, 2026 talks). https://github.com/carbon-language/carbon-lang/blob/trunk/README.md
2. Carbon, "Language goals" (`docs/project/goals.md`). https://github.com/carbon-language/carbon-lang/blob/trunk/docs/project/goals.md
3. Carbon, "Roadmap" (`docs/project/roadmap.md`, 2025 objectives and 2026+ outlook); proposals P2551 (roadmap 2023 / retrospective 2022) and P3564 (roadmap 2024 / retrospective 2023). https://github.com/carbon-language/carbon-lang/blob/trunk/docs/project/roadmap.md ; https://docs.carbon-lang.dev/proposals/p2551.html ; https://docs.carbon-lang.dev/proposals/p3564.html
4. Carbon, "Milestones" (`docs/project/milestones.md`: 0.1, 0.2, 1.0). https://github.com/carbon-language/carbon-lang/blob/trunk/docs/project/milestones.md
5. Carbon, "Evolution and governance" (`docs/project/evolution.md`) and the proposals index (P7254 and others). https://github.com/carbon-language/carbon-lang/blob/trunk/docs/project/evolution.md ; https://docs.carbon-lang.dev/proposals/
6. Proposal P5914, "Updating Carbon's safety strategy", 2025. https://docs.carbon-lang.dev/proposals/p005914-updating-carbon-s-safety-strategy.html
7. Carbon, "Safety strategy" principle (`docs/project/principles/safety_strategy.md`). https://docs.carbon-lang.dev/docs/project/principles/safety_strategy.html
8. Carbon, "Difficulties improving C++" (`docs/project/difficulties_improving_cpp.md`). https://github.com/carbon-language/carbon-lang/blob/trunk/docs/project/difficulties_improving_cpp.md
9. Carbon, "Generics: Overview" (`docs/design/generics/overview.md`). https://github.com/carbon-language/carbon-lang/blob/trunk/docs/design/generics/overview.md
10. Proposal P0301, "Principle: Errors are values". https://docs.carbon-lang.dev/proposals/p000301-principle-errors-are-values.html
11. Proposal P4880, "Safety milestones and a 2025 roadmap". https://docs.carbon-lang.dev/proposals/p004880-safety-milestones-and-a-2025-roadmap.html
12. Proposal P0618, "var ordering". https://docs.carbon-lang.dev/proposals/p000618-var-ordering.html
13. Proposal P0676, "`:!` generic syntax". https://docs.carbon-lang.dev/proposals/p000676-generic-syntax.html
