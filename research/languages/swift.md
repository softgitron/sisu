# Swift

## Snapshot
| | |
|---|---|
| First appeared | June 2014 (WWDC); 1.0 September 2014; open-sourced 3 December 2015 (Apache 2.0 with runtime exception) |
| Designed by | Chris Lattner (initial design, 2010-2014), with Doug Gregor, John McCall, Ted Kremenek, Joe Groff, Dave Abrahams (standard library and generics) at Apple |
| Current version (Oct 2026) | Swift 6.4.0, released 15 September 2026 (6.3 March 2026, 6.2 September 2025, 6.0 September 2024) |
| Governance | Apple-employed Project Lead (Ted Kremenek) has final authority; Core Team appoints the Language, Ecosystem and Platform Steering Groups; Swift Evolution proposals (SE-nnnn) reviewed publicly on forums.swift.org; workgroups for Server, Windows, Android (2025), Networking (2026) |
| Typing | Static, strong, nominal; bidirectional constraint-based inference per statement; generics with protocols/associated types; optionals as `enum Optional<T>`; no implicit conversions |
| Memory | Automatic reference counting (ARC) for classes/closures; structs/enums are values with copy-on-write library collections; opt-in ownership (`~Copyable`, `borrowing`/`consuming`, `Span`, `UniqueArray`) since 5.9-6.4 |
| Compilation | AOT via LLVM; whole-module optimisation; ABI-stable on Apple platforms since 5.0; Embedded Swift subset for bare metal; WebAssembly SDK |
| Primary domains | Apple platform apps (iOS/macOS/...), increasingly server (Vapor, Hummingbird, Apple internal services), embedded, some CLI/tools |
| Notable users | Apple (OS frameworks, Password Monitoring service, parts of the kernel announced at WWDC26), Uber, Airbnb, Lyft, most iOS apps; Stack Overflow 2025: used by 5.4% of 31,771 respondents |

## Stated design goals
Apple's 2014 pitch was "Objective-C without the C": a language that is "safe, fast and expressive",
with safety meaning that "undefined behaviour is the enemy" and that the default should be the safe
one. Lattner's stated intent was a general-purpose language scaling "from scripting to systems
programming", designed around *progressive disclosure*: a beginner should be able to write
`print("hello")` without meeting generics, ownership or concurrency, and each layer should only
appear when the programmer asks for it. Dave Abrahams' WWDC 2015 session 408 ("Protocol-Oriented
Programming in Swift") stated the second pillar: "at its heart, Swift is protocol-oriented", with
first-class value semantics; "Don't start with a class. Start with a protocol." The practical niche
was to replace Objective-C on Apple platforms while being interoperable with it, and since 2015 to
become a credible cross-platform language.

## Strengths

### Optionals with dedicated syntax
`Optional<T>` is an ordinary enum, but the language gives it sugar that makes the common cases
short: `T?`, optional chaining `a?.b?.c`, nil-coalescing `x ?? default`, `if let x`, `guard let x
else { return }`, and since Swift 5.7 the shorthand `if let x` for `if let x = x` (SE-0345). `guard`
in particular is a small invention that other languages lack: it forces the early-exit path to be
written first and keeps the unwrapped binding in scope afterwards, which removes the "pyramid of
doom" that `if let` nesting produced in Swift 1.x. Nullability is total: Objective-C APIs are
imported with nullability annotations and un-annotated ones as implicitly-unwrapped optionals, so
nothing from the outside world arrives as a silent non-optional. Evidence: Dart's null-safety design
document explicitly surveys Swift and Kotlin and adopts the same `?`/`!`/flow-analysis shape;
TypeScript added `?.` and `??` (ES2020) with Swift's spelling; Optional-returning APIs have become
the idiom across the Apple SDK with no sign of the Java `Optional` backlash.

### Value semantics by default, with copy-on-write collections
Structs and enums are values; `Array`, `Dictionary`, `String` and `Set` are structs that share
storage until mutation (copy-on-write) so that value semantics is affordable. Lattner has said that
arrays initially lacked value semantics in the Swift 1.0 betas and that Abrahams "was very adamant"
that values were right; the team switched before 1.0 (Hacking with Swift interview). Abrahams'
working definition on the forums: with value semantics, "it is impossible for any code to alter any
visible behavior of x unless that code actually mentions x". The WWDC 2015 talk gives the
engineering argument: implicit sharing of mutable class instances leads to "excessive copying, race
conditions, locks, deadlocks, bugs". The struct/class split also makes the concurrency story
cheaper: values are trivially `Sendable`. This is the single most-imitated Swift idea: Hylo
(Abrahams' later language) and Mojo both make mutable value semantics the core model, and Kotlin's
2025-26 value-class KEEPs cite the same motivation.

### Protocol-oriented generics with retroactive conformance
Protocols can have associated types, `Self` requirements, default implementations in extensions, and
conditional conformances (`extension Array: Equatable where Element: Equatable`). Any type,
including types from other modules, can be made to conform after the fact, which gives most of the
power of Haskell/Rust typeclasses without a separate instance language. Protocol extensions provide
statically-dispatched defaults and requirement-based dynamic dispatch (customisation points). Opaque
result types (`some P`, SE-0244) and explicit existentials (`any P`, SE-0335, SE-0309 in Swift 5.7)
finally separated "a generic over P" from "a box holding some P", which had been a major source of
confusion ("protocol can only be used as a generic constraint because it has Self or associated type
requirements" was for years the most-complained-about error). Evidence that the model works: the
standard library and Foundation are built on it; Apple's SwiftUI and Swift Testing APIs are
protocol-first; Rust and Swift independently converged on the same associated-type design.

### Structured concurrency and actors with compile-time data-race checking
Swift 5.5 (2021) shipped `async`/`await` (SE-0296), structured concurrency with task groups and
`async let` (SE-0304, SE-0317), actors (SE-0306) and the `Sendable` marker protocol (SE-0302). Child
tasks cannot outlive their parent scope; cancellation is cooperative and propagates down the tree.
Swift 6 (September 2024) turned the data-race diagnostics from Swift 5.10's
`-strict-concurrency=complete` warnings into errors in the Swift 6 language mode, making Swift one
of only two mainstream languages (with Rust) that reject data races at compile time, and the only
one that does it on top of reference counting rather than ownership. Migrators who finished report
that irreproducible crashes disappeared (dev.to migration write-ups, 2024-25). The design was copied
in spirit by Kotlin's structured concurrency predecessors (Trio, kotlinx 0.26) and in form by Mojo
and Hylo's roadmaps.

### Deterministic memory and server-class performance
ARC gives deterministic destruction, no GC pauses and low memory floors. Apple's Password Monitoring
service migration (swift.org, June 2025) is the strongest public data point: a Java service handling
"multiple billions of requests per day" was rewritten in Swift with about 40% higher throughput,
sub-millisecond 99.9th-percentile latency, "hundreds of megabytes" per instance against "tens of
gigabytes" for the JVM to match, about 50% of the Kubernetes capacity freed, and 85% fewer lines of
code. The stated reasons were G1 pause tuning under load and slow JVM scale-up. This is a single
vendor's result, but it is a well-instrumented one.

### Ownership added as progressive disclosure, not as a rewrite
Rather than making ownership mandatory (Rust) or absent (Go/Kotlin), Swift layered it on:
`borrowing`/`consuming` parameter modifiers (SE-0377, 5.9), `consume` (SE-0366), noncopyable
`~Copyable` structs and enums (SE-0390, 5.9; generalised in 6.0), non-escapable types, `Span` and
`InlineArray` (6.2), and in 6.4 `UniqueBox`, `UniqueArray`, `Ref`/`MutableRef`, borrowing iteration
(`Iterable`, SE-0516) and borrow/mutate accessors (SE-0507). The Ownership Manifesto set the goal in
2017: "reference counting and uniqueness testing do impose some overhead" and programmers "have few
tools" to fix copy hot spots short of unsafe pointers. The approach lets 99% of code stay
ARC-managed while hot paths opt into Rust-like guarantees; `~Copyable` file descriptors get a
`deinit` with no heap allocation. Whether a retrofit can reach Rust's performance is still open (see
Weaknesses), but the shape, copyable-by-default with opt-out, is now being copied by Mojo and
considered by Kotlin (KEEP-0453).

### ABI and module stability with library evolution
Swift 5.0 (March 2019) made the ABI stable on Apple platforms; 5.1 added module stability and the
library-evolution mode (`-enable-library-evolution`, `@frozen`, `@inlinable`). This lets Apple ship
Swift in the OS and lets frameworks evolve without recompiling clients, something no other language
with value types and generics had managed (C++ never did; Rust has deliberately not). The cost is
real (see Weaknesses) but it is opt-in per module, which is the right default: application code pays
nothing.

### Unicode-correct strings
`String` is a collection of `Character` values that are extended grapheme clusters; equality is
canonical equivalence, so `"é"` as one scalar equals `"é"` as e + combining acute. Swift 5 moved
storage to UTF-8 for interop and speed. The String Manifesto states the goal: grapheme-level
operations "match whole-string behavior for equality, ordering, case conversion, and reversal". This
is the correct default for user-facing text and no other mainstream language has it; the price is
paid in ergonomics (below).

### Result builders and macros replacing code generation
Result builders (SE-0289, 5.4) are the mechanism behind SwiftUI's declarative syntax and are general
enough for HTML, regex (SE-0351 regex builders) and test DSLs. Macros (SE-0382 freestanding, SE-0389
attached, 5.9) are hygienic, type-checked, run as separate processes over swift-syntax, and replaced
Sourcery-style codegen for `@Observable`, `#Predicate`, Swift Testing's `#expect`. The design is
sound; its build cost is not (see Weaknesses).

### Open, written-down evolution process
Every language change since 2015 is a numbered proposal with a motivation, detailed design,
source/ABI-compatibility sections, alternatives considered and a public review thread. Vision
documents (e.g. "Improving the approachability of data-race safety", 2025) set direction before
proposals. Compared with Kotlin's KEEPs (often published after design) and Dart's language repo,
Swift's record is the most complete, which is why it is possible to cite exactly what the designers
were thinking at each step.

## Weaknesses

### The type checker is exponential and the user sees it
"The compiler is unable to type-check this expression in reasonable time" is Swift's signature
failure. Matt Gallagher's 2016 analysis ("Exponential time complexity in the Swift type checker")
showed `let a: Double = -(1 + 2) + -(3 + 4) + -(5)` failing in Swift 3 with only five literals and
six operators. The compiler's own TypeChecker.md explains the mechanism: inference is bidirectional,
every overloaded operator becomes a disjunction constraint, every literal is polymorphic (an integer
literal may be `Int`, `Double`, `UInt8`, any `ExpressibleByIntegerLiteral`), and the solver is a
depth-first search that "can take exponential time in the worst case". Mitigations
(connected-component splitting, online scoring, cheaper conversions first, "one-way constraints" for
result builders) have reduced but not removed the problem; a 2024 forum thread ("Using the reduce
operator on a publisher of literal Range smothers the compiler") shows the same failure mode in
Swift 5.10. Workarounds are to annotate literals (`3 as Int`), add result types, or split
expressions, which is the opposite of what inference is for. Lesson: unconstrained operator
overloading plus polymorphic literals plus bidirectional inference is a combination to avoid, not a
tuning problem.

### Compile times, and macros that made them worse
Swift's compile times have been a complaint since 1.0 (type checking above, plus whole-module
optimisation and generic specialisation). Macros added a new fixed cost: every package using a macro
must build swift-syntax from source. Forum threads in 2024 ("Compilation extremely slow since macros
adoption", "Swift Macros: Build Time Overhead Concerns") report a framework going from 257 s to 2004
s clean build after adding one macro, a small app from ~12 s to ~23 s, and "over 4 minutes just for
SwiftSyntax" in release builds; a 2025 analysis measured ~1 ms per macro expansion round-trip to the
plugin process. The design decision to make macros out-of-process plugins over a large syntax
library was taken for hygiene and sandboxing; the cost was not disclosed up front and a prebuilt
swift-syntax took years to arrive. Dart cancelled its macro system in 2025 for an analogous reason
(compile-time cost versus hot reload).

### Swift 6 strict concurrency: the migration was too hard, and 6.2 retreated
The Swift 6 language mode made every potential data race an error. Forum threads from 2024-25 ("My
experience (attempting) a migration to swift 6", "Migration Challenges from Swift 5.x to Swift 6 in
a UIKit") describe brownfield projects "buried in errors", fixes that were "wrong but needed just to
compile", `@MainActor` annotations cascading through protocols and dependencies, and teams staying
in Swift 5 mode or suppressing warnings. The designers agreed. The 2025 vision document "Improving
the approachability of data-race safety" (endorsed by the Language Steering Group) says: "Swift 6
defaults to a presumption of concurrency", so single-threaded code produced false positives for
globals, protocol conformances of main-actor types, deinitialisers and subclass overrides; "Writing
a single-threaded program is surprisingly difficult under the Swift 6 language mode"; and of
SE-0338's rule that nonisolated async functions always hop off the caller's actor, "Over time, we
have learned that this design decision undermines progressive disclosure." Swift 6.2 (September
2025) shipped the retreat: SE-0466 lets a module default all unannotated code to `@MainActor` (Xcode
26 enables it for new app targets), SE-0461 makes nonisolated async functions run on the caller's
actor (`nonisolated(nonsending)`) with explicit `@concurrent` for code that must leave it, SE-0470
adds isolated conformances. The vision document's three phases, "write sequential, single-threaded
code", "write asynchronous code without data-race safety errors", "introduce parallelism to improve
performance", are the clearest published statement of how much static concurrency checking
application developers will accept: full checking is tolerated only when the single-threaded default
makes most code trivially correct. The lesson is not that checking failed, it is that the default
must be sequential.

### ARC costs are hidden and hard to control
Every class reference copy is a retain/release pair, often atomic; the optimiser removes many but
cannot across ABI-stable or polymorphic boundaries (SE-0377 motivation). Reference cycles are not
collected, so `weak`/`unowned` are required in closures and delegate patterns, and `[weak self]`
boilerplate is the most common Swift idiom; forgetting it leaks, using `unowned` wrongly crashes.
Copy-on-write hides the moment a copy actually happens, so a struct holding an array can trigger an
O(n) copy on a seemingly innocent mutation (the "accidental copy" when a value is held in two
places). The Ownership Manifesto (2017) admits that the model gives programmers "few tools" beyond
unsafe pointers, and the 2023-2026 ownership features are the ten-year fix. Dave Abrahams, who led
the standard library and generics, left and built Hylo on the thesis that value semantics should be
the whole model rather than the "hybrid model of Swift" (BLDL-15 keynote abstract, November 2024).

### Early churn: Swift 1-3 broke source every year
Swift 2 (2015) changed error handling and dropped `String`'s `Collection` conformance; Swift 3 (13
September 2016) applied 89 accepted proposals including the "Grand Renaming" (SE-0005 Objective-C
import translation, SE-0006 standard-library naming, SE-0023 API design guidelines) and was "not
source-compatible with Swift 2.2 or 2.3". Xcode's migrator handled the mechanical part; the rest was
manual, and most third-party code had to be re-released. Swift 4 (September 2017) was the first
release to promise source compatibility (via a `-swift-version 3` mode) and Swift 5 the first with
ABI stability. The cost was three years in which libraries, tutorials and Stack Overflow answers
rotted annually; the benefit was that the naming and error-handling design settled early rather than
being frozen wrong. The team's own conclusion was to never do it again: every proposal since carries
a source-compatibility section and breaking changes are gated behind language modes.

### Library evolution has a performance price
The 2020 swift.org post "Library Evolution in Swift" is explicit: enabling resilience "changes your
framework's performance characteristics"; for non-`@frozen` types "the compiler will always
manipulate the value indirectly" through metadata and offset calculations; property access crosses
the boundary via accessors, protocol calls via dispatch thunks; code size grows. Library authors
must learn `@frozen`, `@inlinable`, `@usableFromInline` and the rule that adding or removing
`@frozen` is itself ABI-breaking. Apple's SDK pays this everywhere; third-party frameworks mostly do
not enable it. The alternative (Rust: no stable ABI, everything recompiled) is also a real cost;
neither choice is free.

### String ergonomics
No integer subscripting: `s[3]` does not compile; you write `s[s.index(s.startIndex, offsetBy: 3)]`,
and indices from one string are invalid in another. `Substring` is a separate type, so slicing
requires `String(...)` conversions at API boundaries. The Manifesto counts 205 APIs on `String`
against 80 on `Int` and concludes "Today, `String` fails that test" of being a simple type; it also
notes users "struggle with `Character` as a black box". The `Collection` conformance was removed in
Swift 2 and restored in Swift 4 (SE-0163), another churn episode. Grapheme-correct by default is
right; the missing piece was a cheap, obvious way to say "I know this is ASCII/UTF-8, give me
offsets".

### Errors were untyped for nine years, then typed throws arrived half-finished
Swift 2's `throws` carried `any Error` only, by design: SE-0413's motivation records the original
rationale that "errors are generally propagated and rendered, but rarely handled exhaustively".
Users asked for typed errors continuously (the proposal says the wish "has come up repeatedly");
`Result<T, E>` (SE-0235, 2018) had a typed error parameter that `throws` could not express, so
converting between them lost information. SE-0413 (implemented Swift 6.0) adds `throws(E)`, and
generics can now replace `rethrows` with `throws(E)` where `E` infers to `Never`. But its closure
type-inference section was moved to Future Directions, 2025 forum posts call the feature "only
half-implemented", and the guidance still says untyped is the better default for most code ("Resist
the temptation to use typed throws because there is only a single kind of error"). The honest
reading: Swift's designers believe typed errors are right for libraries and constrained environments
and wrong as a universal requirement, and the language now has both with a seam between them.

### Governance is open in process and Apple-controlled in fact
The Language Steering Group page states that membership is decided by the Core Team alone and that
"final decision-making authority over language evolution rests with the Project Lead", an Apple
engineering director. Release schedules track Xcode and WWDC. Chris Lattner, after leaving Apple in
2017, publicly left the Core Team in early 2022 citing frustration with the environment and
decision-making; this is widely reported (e.g. Michael Tsai's blog, February 2022) but the original
forum post could not be re-fetched for this research and the wording should be checked before
quoting. The counter-evidence is the 2024-26 opening: a separate Ecosystem Steering Group creating
workgroups (Android 2025, Networking June 2026), swiftly and the VS Code extension on Open VSX,
Swift Build as the default SwiftPM backend on all platforms (6.4). Still, no non-Apple organisation
has veto power over anything.

### Cross-platform arrived late and unevenly
Linux has been supported since the 2015 open-sourcing but the open-source Foundation was a
reimplementation that lagged Apple's for years (replaced by swift-foundation in 2024). Windows got
an official toolchain with Swift 5.3 (2020); a Windows Workgroup exists; 6.4 made Swift Build the
default there. Android has a workgroup (mid-2025) whose charter is still "to make Android an
officially supported platform", i.e. it is not yet one. Embedded Swift (preview in 6.0, `String` and
existentials by 6.2-6.4) and the WebAssembly SDK (6.2) are real. The practical effect is that server
Swift (Vapor, Hummingbird, Apple's own services) is viable on Linux, but the ecosystem of pure-Swift
libraries is an order of magnitude smaller than Kotlin's or Go's, and most iOS developers never
target anything else.

## Designer retrospectives
- **Dave Abrahams** (standard library lead 2013-2020, co-designer of the generics system) left
  Apple, worked on Swift for TensorFlow at Google, and now leads Hylo (formerly Val) at Adobe.
  Hylo's own description: a language "built on value semantics" whose model is "simpler than the
  constrained reference model of Rust or the hybrid model of Swift", whose generics are "separately
  type-checked" like Swift's but which "aims to address some of Swift's restrictions and surprising
  behaviors" (BLDL-15 keynote abstract, 2024). His C++Now 2022 keynote ("A Future of Value Semantics
  and Generic Programming") makes the same argument from the C++ side. The implicit verdict: classes
  and ARC in the core model were the mistake; value semantics and generic programming were right.
- **Language Steering Group / Holly Borla** on concurrency (2025 vision document): "we have learned
  that this design decision undermines progressive disclosure"; the per-module main-actor default
  and caller-isolation for async functions followed within a year.
- **SE-0413 authors** on error handling: the original untyped-only design was deliberate and remains
  the recommended default; typed throws exists for the minority of cases where exhaustive handling
  or no-allocation matters.
- **String Manifesto** (Swift 4 era): "`String` has 205 APIs ... Today, `String` fails that test";
  the localized/non-localized mixing is "a correctness trap".
- **Ownership Manifesto** (2017): copies force heap allocation, "reference counting and uniqueness
  testing do impose some overhead", and programmers lack tools; the fix took until 2023-2026.
- **Chris Lattner** (Hacking with Swift interview) on arrays' value semantics being settled only
  just before 1.0, and on the Swift 1-3 period as the time when the team "didn't have the luxury" of
  stability. He left Apple in January 2017 and the Core Team in 2022.

## Influence
- **Copied from Swift:** Optional chaining and nil-coalescing spelling (`?.`, `??`) in
  TypeScript/JavaScript (ES2020) and Dart; `guard`-style early exit appears in Kotlin idioms via `?:
  return` and in Zig's `orelse return`; `some`/`any` vocabulary for opaque vs existential types is
  now standard terminology; structured concurrency with task groups in Mojo; result builders
  inspired Kotlin Compose's DSL shape (indirectly, via SwiftUI); Hylo and Mojo adopt Swift's syntax
  and mutable value semantics; Kotlin's value-class KEEPs cite the struct/class split; Dart's `late`
  cites Swift's `lazy`; Rust's `impl Trait` and Swift's `some` co-evolved.
- **Swift copied from:** Objective-C (ARC, message-style labels, protocols), Rust (ownership
  vocabulary, `Sendable` resembling `Send`, `~Copyable` after `!Copy` discussions), Haskell and Rust
  (protocols with associated types as typeclasses/traits), C# (async/await, extension-like protocol
  extensions, `#if`), Python/Ruby (syntax lightness, trailing closures), Scala/Kotlin-era languages
  (named arguments, `let`/`var`), Erlang-free actor model via Pony/Akka ideas, and Python's
  Trio/Kotlin for structured concurrency.

## Lessons for Sisu
- **Adopt**
  - Optionals as a library enum with dedicated sugar (`?.`, `??`, `if let`, `guard let`). Ten years
    of use, no backlash, copied widely. `guard` specifically: early-exit that keeps the binding in
    scope.
  - Value semantics by default with copy-on-write for collections; a separate, explicit category for
    reference/identity types. Both Swift's designers and the language they left to build agree this
    is the core that works.
  - Structured concurrency with scoped child tasks and cooperative cancellation; actors for
    isolation. This has survived three years of production use and the 6.2 changes left it intact.
  - Opt-in library evolution: application code pays nothing, frameworks choose resilience. Do not
    make resilient layout the default.
  - A numbered, public proposal format with mandatory motivation, source/ABI compatibility and
    alternatives sections, plus vision documents that precede proposal batches.
- **Adapt**
  - Static data-race checking, but with Swift 6.2's lesson built in from day one: the default
    context is sequential (main-actor-like); async functions stay on the caller's context;
    concurrency is explicitly requested; libraries may be stricter than applications. Do not ship
    Swift 6.0's "presumption of concurrency" first.
  - Ownership and borrowing as progressive disclosure, but designed into the calling convention from
    the start so that there is no ABI-breaking retrofit and no "copyable by default except when
    annotated" seam. Swift shows that this can be layered; Rust shows the cost of the alternative;
    neither shows a retrofit reaching parity.
  - Typed errors from day one with inference through generics (`throws(E)` with `E = Never`), so
    that the untyped/typed seam Swift now carries never exists. Keep Swift's guidance: libraries
    should be able to type errors, applications should rarely need to.
  - Grapheme-cluster strings for correctness, but add what Swift lacks: a cheap, obvious ASCII/UTF-8
    byte-offset view with integer indices, and no separate `Substring` type at API boundaries.
  - Macros that are hygienic and type-aware, but with the cost model designed first: expansion must
    be in-process or precompiled, never "build a 100k-line syntax library per package". Dart's
    cancellation and Swift's forum threads are two independent warnings.
- **Avoid**
  - The inference triangle: unrestricted operator overloading + polymorphic literals +
    whole-expression bidirectional inference. Pick at most two. Literals should default quickly;
    operator overload sets should be small or resolved by the left operand's type.
  - Hidden ARC costs in the core model: atomic retain/release on every reference copy, uncollected
    cycles requiring `weak`/`unowned`, copies whose cost is invisible at the call site. If Sisu uses
    reference counting at all, cycles and copy costs must be visible in types or in syntax.
  - Annual source-breaking releases. Swift 1-3 cost three years of ecosystem rot; stabilise naming
    before 1.0 and gate every break behind a language mode.
  - Single-vendor governance with a titular open process. Even with the best proposal process in the
    industry, Apple's final authority and Xcode-driven scheduling limit who invests in the language
    outside Apple's platforms.
  - Shipping a concurrency model, a macro system or an error-typing design in a form the designers
    already know is incomplete. Each of Swift's three retrofits (ownership, typed throws,
    approachable concurrency) cost a multi-year migration.
- **Open questions raised**
  - How much static concurrency checking will developers accept? Swift's data: full checking with a
    concurrent default was rejected by a large fraction of the user base; full checking with a
    sequential default plus explicit `@concurrent` is being accepted. Whether that holds for library
    authors (who still live in the strict world) is unknown.
  - Can a value-semantics-plus-reference-counting language reach systems-language performance by
    opt-in ownership, or does performance require ownership in the core (Rust) or no references at
    all (Hylo)? Swift 6.4's `UniqueArray`/`Span`/`Ref` are the experiment; results are not in.
  - Is ABI stability worth it for a language that is not shipped inside an OS? Swift paid heavily
    and got OS integration; a language without that need might choose Rust's path.
  - Grapheme-cluster-default strings are correct and unpopular; the trade-off between correctness
    and the expectation that `s[i]` works has not been resolved by any language.
  - Typed throws: Swift's designers still recommend untyped by default. Whether a language that
    offers only typed errors (Rust) or only untyped (Go's `error`) produces better libraries is not
    settled by Swift's data; it only shows that having both is messy.

## Sources
1. Swift.org, "Swift 6.4 Released", 15 September 2026.
   https://www.swift.org/blog/swift-6.4-released/
2. Swift.org, "Announcing Swift 6", 17 September 2024.
   https://www.swift.org/blog/announcing-swift-6/
3. Swift.org, "Swift 6.1 Released", 31 March 2025. https://www.swift.org/blog/swift-6.1-released/
4. Swift.org, "Swift 6.2 Released", 15 September 2025.
   https://www.swift.org/blog/swift-6.2-released/
5. Swift.org, "Swift 3.0 Released", 13 September 2016.
   https://www.swift.org/blog/swift-3.0-released/
6. Swift.org, "Swift 4.0 Released", 19 September 2017.
   https://www.swift.org/blog/swift-4.0-released/
7. Swift.org, "Library Evolution in Swift", 13 February 2020.
   https://www.swift.org/blog/library-evolution/
8. Swift.org, "Swift at Apple: Migrating the Password Monitoring service from Java", 2 June 2025.
   https://www.swift.org/blog/swift-at-apple-migrating-the-password-monitoring-service-from-java/
9. Swift.org, Language Steering Group charter. https://www.swift.org/language-steering-group/
10. Swift.org, Android Workgroup charter (2025). https://www.swift.org/android-workgroup/
11. Swift.org, "Announcing the Networking Workgroup", 4 June 2026. https://www.swift.org/blog/
12. Swift Evolution, "Improving the approachability of data-race safety" (vision document), 2025.
    https://github.com/swiftlang/swift-evolution/blob/main/visions/approachable-concurrency.md
13. SE-0466, "Control default actor isolation inference", implemented Swift 6.2.
    https://github.com/swiftlang/swift-evolution/blob/main/proposals/0466-control-default-actor-isolation.md
14. SE-0461, "Run nonisolated async functions on the caller's actor by default", implemented Swift
    6.2.
    https://github.com/swiftlang/swift-evolution/blob/main/proposals/0461-async-function-isolation.md
15. SE-0413, "Typed throws", implemented Swift 6.0.
    https://github.com/swiftlang/swift-evolution/blob/main/proposals/0413-typed-throws.md
16. SE-0390, "Noncopyable structs and enums", implemented Swift 5.9.
    https://github.com/swiftlang/swift-evolution/blob/main/proposals/0390-noncopyable-structs-and-enums.md
17. SE-0377, "`borrowing` and `consuming` parameter ownership modifiers", implemented Swift 5.9.
    https://github.com/swiftlang/swift-evolution/blob/main/proposals/0377-parameter-ownership-modifiers.md
18. Swift Evolution proposals index (SE-0005, 0006, 0023, 0163, 0235, 0244, 0289, 0296, 0302, 0304,
    0306, 0309, 0317, 0335, 0345, 0366, 0382, 0389, 0434, 0470; numbers cited from the repository,
    not re-fetched individually in this pass). https://github.com/swiftlang/swift-evolution
19. Swift compiler, "String Manifesto" (Swift 4 era).
    https://github.com/swiftlang/swift/blob/main/docs/StringManifesto.md
20. Swift compiler, "Ownership Manifesto" (2017).
    https://github.com/swiftlang/swift/blob/main/docs/OwnershipManifesto.md
21. Swift compiler, "Type Checker Design and Implementation".
    https://github.com/swiftlang/swift/blob/main/docs/TypeChecker.md
22. Matt Gallagher, "Exponential time complexity in the Swift type checker", 12 July 2016.
    https://cocoawithlove.com/blog/2016/07/12/type-checker-issues.html
23. swiftlang/swift issue #46253 (expression too complex, heavily overloaded operators).
    https://github.com/swiftlang/swift/issues/46253
24. Swift Forums, "Using the reduce operator on a publisher of literal Range smothers the compiler",
    2024.
    https://forums.swift.org/t/using-the-reduce-operator-on-a-publisher-of-literal-range-smothers-the-compiler/73219
25. Swift Forums, "Compilation extremely slow since macros adoption", 2023-24.
    https://forums.swift.org/t/compilation-extremely-slow-since-macros-adoption/67921
26. Swift Forums, "Swift Macros: Build Time Overhead Concerns", 2024.
    https://forums.swift.org/t/swift-macros-build-time-overhead-concerns/70443
27. Swift Forums, "Analysis of Macro expansion overhead in the Swift Compiler", 2025.
    https://forums.swift.org/t/analysis-of-macro-expansion-overhead-in-the-swift-compiler/80813
28. Swift Forums, "My experience (attempting) a migration to swift 6", 2025.
    https://forums.swift.org/t/my-experience-attempting-a-migration-to-swift-6/79069
29. Swift Forums, "Migration Challenges from Swift 5.x to Swift 6 in a UIKit", 2024.
    https://forums.swift.org/t/migration-challenges-from-swift-5-x-to-swift-6-in-a-uikit/76987
30. Dave Abrahams, "Protocol-Oriented Programming in Swift", WWDC 2015 session 408.
    https://developer.apple.com/videos/play/wwdc2015/408/
31. Dave Abrahams, forum post defining value semantics, Swift Forums "ValueSemantic protocol"
    thread, 2020. https://forums.swift.org/t/valuesemantic-protocol/41686/2
32. Dave Abrahams, keynote "Hylo - The Safe Systems and Generic-programming Language Built on Value
    Semantics", BLDL-15, 27 November 2024 (abstract).
    https://conf.researchr.org/details/bldl-15/bldl-15-papers/2/
33. Dave Abrahams, keynote "A Future of Value Semantics and Generic Programming", C++Now 2022
    (announcement). https://cppnow.org/announcements/2022/04/abrahams-keynote/
34. Hylo language site and repository. https://www.hylo-lang.org/ and
    https://github.com/hylo-lang/hylo
35. Chris Lattner interview, Hacking with Swift, "Were there changes between the first Swift beta
    and release?".
    https://www.hackingwithswift.com/interviews/chris-lattner-were-there-changes-between-the-first-swift-beta-and-release
36. Wikipedia, "Swift (programming language)" (release history, Lattner's 2017 departure).
    https://en.wikipedia.org/wiki/Swift_(programming_language)
37. Stack Overflow Developer Survey 2025, technology section (Swift 5.4% of 31,771 responses).
    https://survey.stackoverflow.co/2025/technology
38. Michael Tsai, "Chris Lattner Leaves Swift Core Team", February 2022 (secondary; original forum
    post not re-fetched). https://mjtsai.com/blog/
