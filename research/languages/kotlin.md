# Kotlin

## Snapshot
| | |
|---|---|
| First appeared | Announced July 2011; 1.0 February 2016 |
| Designed by | Andrey Breslav (lead 2010-2020) and the JetBrains Kotlin team; Roman Elizarov (coroutines, lead 2020-2023); Mikhail Zarechenskii (Lead Language Designer since 2023) |
| Current version (Oct 2026) | 2.4.21 (8 October 2026); 2.4.0 language release June 2026; 2.0 (K2 compiler) May 2024 |
| Governance | JetBrains develops the language; the Kotlin Foundation (JetBrains, Google, Meta, Touchlab, AWS, independents) appoints the Lead Language Designer and a three-person Language Committee that must approve incompatible changes but "does not directly make decisions about the course of the project"; proposals are KEEPs |
| Typing | Static, strong, nominal, null-safe by default; local type inference; declaration-site variance; reified generics only in `inline` functions (JVM erasure otherwise); smart casts via flow analysis |
| Memory | JVM GC on JVM/Android; tracing GC (concurrent mark-sweep default since 2.4) on Kotlin/Native; JS/Wasm engines' GC |
| Compilation | JVM bytecode; JavaScript; WebAssembly (WasmGC); native via LLVM (Kotlin/Native); K2 compiler frontend (FIR) since 2.0 |
| Primary domains | Android (Google's "Kotlin-first" since 2019), JVM backends (Spring, Ktor), Kotlin Multiplatform shared mobile code, Gradle build scripts |
| Notable users | Google (Android, Jetpack), Netflix, Meta, Amazon, Slack, Lyft, Airbnb; Stack Overflow 2025: 10.8% of respondents use Kotlin; JetBrains 2025: 6% want to adopt it next |

## Stated design goals
Breslav's 2015 Curry On abstract: a statically typed language "aimed at industrial use", "focused on
readability, safety, performance and interoperability with existing ecosystems (including Java)".
The official evolution principles call Kotlin "a pragmatic tool" whose design is "cast in stone, but
this stone is reasonably soft". The niche was explicit: JetBrains wanted a better language for its
own million-line Java codebase that could be adopted file by file, compile to the same bytecode, and
be supported by the IDE from day one (Kotlin began as an IntelliJ plugin that could parse and
highlight before it could compile; Breslav: "I could show off the language as if it existed").
Android adoption was accidental (a developer asked "does Kotlin work on Android?", the toolchain
crashed, and the fix made Android the main market).

## Strengths

### Null safety that is pragmatic rather than pure
Types are non-nullable by default; `T?` is nullable; `?.`, `?:` (elvis) and `!!` give short
safe-call, default and assert forms; flow-sensitive smart casts turn `if (x != null)` into a
non-null `x` in the branch with no rebinding. Java values arrive as *platform types* (`String!`,
"String or String?, we don't know which"), so unannotated Java code compiles without ceremony and
nullability is checked lazily (assertions are inserted when a platform value is assigned to a
non-null type). Nullability annotations (JetBrains, JSpecify, JSR-305, Android) upgrade platform
types to real Kotlin types. The design trades soundness for interop: NPEs can still come from Java,
but they are confined to the boundary. Evidence it works: Google claims apps using Kotlin are "20%
less likely to crash" (developer.android.com); Dart's null-safety designer surveyed Kotlin and
adopted smart casts and `!`; Java itself is now adding null-restricted types (JEP drafts) after
Kotlin and C# demonstrated the demand. K2 (2.0) extended smart casts to local `val isCat = animal is
Cat`, `||` chains, inline-lambda captures, function-typed properties and `catch`/`finally`, which
were the most-reported gaps.

### Interop-driven adoption
Kotlin compiles to the same bytecode, calls Java without wrappers, and lets Java call Kotlin (with
`@JvmStatic`, `@JvmOverloads`, `@Throws` where Java conventions differ). This let Android teams
convert one file at a time, which is why Google could announce official support in 2017 with "over
50% of professional Android developers" using it by May 2019 (Kotlin-first announcement) and "over
60%" today. No other new language has displaced an incumbent on a major platform this fast without a
platform-owner mandate, and the mandate came after adoption, not before. The cost of the strategy is
listed under Weaknesses.

### Coroutines as one keyword plus a library
The language has exactly one concurrency primitive, the `suspend` modifier, compiled by
continuation-passing-style transformation into state machines ("the compiler is only responsible for
support of suspending functions, suspending lambdas" per the KEEP). Everything else (`launch`,
`async`, `Flow`, channels, dispatchers, `select`) is the kotlinx.coroutines library, so generators,
async/await, reactive streams and Go-style channels are all expressible without further language
changes. The KEEP's argument against C#/JS-style async: "async-style function composition is more
verbose and error prone", suspending functions are "sequential by default" and concurrency must be
written explicitly, and futures allocate per call so "async-style is not well-suited for very
fine-grained decomposition". Structured concurrency arrived in kotlinx.coroutines 0.26 (2018): every
coroutine lives in a `CoroutineScope`, children are cancelled with their parent, a failing child
cancels siblings unless a `SupervisorJob` is used, and `GlobalScope` is explicitly discouraged ("for
rare cases"). Elizarov's own KotlinConf 2018 slides show the pre-structured version launching into
`GlobalScope` and losing child failures, which is the admission that the first design was wrong.
Swift's SE-0304 and Java's Loom StructuredTaskScope follow the same shape.

### Extension functions and lambdas with receiver
`fun String.shout() = uppercase()` adds a method statically; no monkey-patching, resolved at compile
time, visible through imports. Combined with lambdas whose receiver is a type (`fun html(block:
HTML.() -> Unit)`), this produces type-safe builders: Gradle's Kotlin DSL, Ktor routing,
kotlinx.html, Jetpack Compose. The feature is cheap to implement (static dispatch), easy to teach,
and has been copied by Scala 3 (`extension`), C# (origin), Swift (extensions predate), and Dart
(extension methods, 2.7).

### Data classes and sealed hierarchies
`data class` generates `equals`/`hashCode`/`toString`/`copy`/`componentN`; `sealed class`/`sealed
interface` (1.5, May 2021) plus exhaustive `when` give closed algebraic data types. Java copied both
as records (JEP 395, Java 16) and sealed classes (JEP 409, Java 17), and Dart copied `sealed` in
3.0. The limits (reference identity, shallow immutability, `copy` nesting) are the subject of the
2025-26 value-class KEEPs (below).

### `val`/`var` and read-only collection interfaces
Immutability is the shorter spelling (`val`), and the standard library separates `List` (read-only
view) from `MutableList`. This is a convention, not a guarantee (a `List` may be a `MutableList`
underneath), which the KEEP-0453 motivation admits ("`val` and read-only interfaces give only
shallow guarantees"). Still, the default steers code toward immutability more effectively than
Java's `final` ever did.

### Disciplined evolution process
The evolution principles define Experimental/Alpha/Beta/Stable levels with mandatory opt-in
annotations (`@OptIn`) for pre-stable APIs, deprecation cycles (warn in version A, error in B,
remove later), `-language-version`/`-api-version` flags that keep at least three prior versions
compilable, and a Language Committee that must approve incompatible changes. The K2 rollout is the
model case: a four-year rewrite (announced 2021, alpha 1.7, stable 2.0 in May 2024) validated on
"about 10 million lines of code" with "18,000 developers tested K2 across 80,000 projects" before it
became default, with no source breaks for stable features.

### K2 compiler rewrite paid off
The old frontend was PSI-based and had accumulated a decade of special cases; K2 introduced FIR
(frontend IR), unified all backends (JVM, JS, Native, Wasm) and made compiler plugins possible on a
stable API. JetBrains' benchmarks: Anki-Android clean build 57.7 s to 29.7 s (1.9.23 to 2.0.0),
Exposed 5.8 s to 3.22 s, "up to 94%" in the best case, "potentially doubling compilation speed" on
average with "some projects ... a bit slower". Incremental builds gained only ~20%. The decision to
rewrite rather than patch is a data point in favour of designing the compiler architecture for
plugins and multiple backends from the start.

### Multiplatform that actually shipped
Kotlin Multiplatform was declared stable in November 2023 (Kotlin 1.9.20) after five years of
alpha/beta, with Netflix, Philips, McDonald's and Google's Jetpack libraries as users; Compose
Multiplatform for iOS went stable in May 2025; 2.4 adds Swift export (alpha) and Swift Package
import. The `expect`/`actual` mechanism for platform-specific declarations is simpler than C
preprocessor conditionals and has not needed redesign.

## Weaknesses

### Inherited Java constraints
Interop set the ceiling. Generics are erased (no `List<Int>` at runtime; `reified` works only in
`inline` functions); nullable primitives box (`Int?` is `java.lang.Integer`); unsigned integers took
until 1.5 (May 2021) to stabilise and are implemented as inline classes over signed ones; value
classes could wrap only one property until the 2025-26 KEEPs (0453, 0454, 0468) because JVM has no
value types (Project Valhalla is still not shipped); exceptions are unchecked because Java's checked
exceptions were judged a failure ("you can catch exceptions, but you don't need to explicitly handle
or declare them"), which left Kotlin with *no* signal in a signature that a call can fail. The
rich-errors KEEP (0441, 2025) states this directly: "there are no signals about potential throws",
`null` is ambiguous between "no value" and "error", sealed-class encodings "need a wrapper for
success cases and a lot of repeated boilerplate", and `kotlin.Result` "is tied to `Throwable`" and
"was designed mainly for coroutines". After fifteen years the proposed fix is error union types
(`fun load(): User | NotFound`, `error class`, `|.` propagation operator, KEEP-0462, in public
discussion as of 2026), i.e. an admission that unchecked exceptions alone were not enough and that
the language needs a second, typed, value-based error channel.

### Function colouring via `suspend`
`suspend` functions can only be called from other `suspend` functions or a coroutine builder;
blocking libraries cannot be called without `withContext(Dispatchers.IO)` wrappers; every library
ends up with suspending and non-suspending variants (JDBC vs R2DBC, OkHttp vs Ktor client).
Elizarov's defence (in "How do you color your functions?", 2020, not re-fetched for this pass) is
that the colour makes blocking visible and that Loom-style virtual threads hide a cost rather than
remove it; Java 21's virtual threads nonetheless mean that on the JVM the uncoloured alternative now
exists, and Spring and Vert.x offer both. The KEEP's own trade-off table concedes "async-style is
easier to consume from Java/JS". Kotlin's data point: colouring was accepted on Android (where the
alternative was callbacks) and is contested on the server (where the alternative is now Loom).

### Kotlin/Native's first memory model failed
From Kotlin/Native's 2017 start until 2022, objects could cross threads only if *frozen* (deeply
immutable, enforced at runtime by `InvalidMutabilityException`) or fully detached. JetBrains' 2020
roadmap post admits the model, "conceptually appealing", had "a number of deficiencies that hinder
wider Kotlin/Native adoption": "a steep learning curve", inability to share concurrent code with
Kotlin/JVM, and that the multithreaded coroutines port "still suffers from memory leaks in some
concurrent execution scenarios" for which "we don't have a clear solution". The replacement, a
conventional tracing GC with unrestricted sharing, previewed in August 2021 (initially "5 times
regression on some of our examples"), shipped experimental in 1.6 (November 2021), became default in
1.7.20 (29 September 2022), and freezing was deprecated then and deleted in 1.9.20. This is a clean
retrospective on a static-sharing restriction: a model that was safe and simple to implement was
rejected by users because it did not match the JVM mental model and because the restrictions leaked
into every library. Sisu's concurrency design should treat it as the strongest evidence that
"immutable-to-share" enforced at runtime, without type-system support, does not survive contact with
a mobile ecosystem.

### The context receivers saga
Context receivers (implicit receivers injected by callers, KEEP in 1.6.20, 2022) were experimental
for three years and used in "a non-trivial amount of code"; in 2025 JetBrains announced "we've
decided not to continue with it" and replaced them with context parameters (named, not receivers;
2.1.20 experimental, 2.2 beta, 2.4.0 stable), with removal of the old feature "around 2.3". The
published reason is that implicit receivers made member resolution ambiguous and code hard to read;
the named form forces `ctx.log(...)`. Cost: every early adopter refactored twice. Lesson: a feature
that Scala had already shipped and regretted (implicits as ambient context) was re-tried as an
experiment, which the stability-level system allowed, and the system worked as designed (revoked
before stable), but "experimental" features used in production still break people.

### Compile and build times
Before K2, Kotlin compiled noticeably slower than Java and Gradle's configuration overhead made
Android builds a standing complaint; K2 roughly doubled frontend speed but the 2026 roadmap still
lists "Improve KAPT performance to be comparable with Java APT" and "Kotlin Toolchain: a single
entry point into Kotlin", i.e. annotation processing and toolchain setup remain slow and fragmented.
Kotlin Symbol Processing (KSP) exists because KAPT (Java annotation processing over generated stubs)
was too slow, a pattern (two generations of codegen tooling) that Dart repeated with build_runner.

### Design regrets the lead designer names
Breslav (Pragmatic Engineer podcast, February 2026): omitting the ternary operator is his "biggest
design regret"; he removed it to free `?` and `:` for nullable types and type annotations, expecting
`if` as an expression to suffice, and "by the time he agreed it was a mistake ... you can't retrofit
the ternary operator in the current syntax". He has also said he is unhappy with how composition by
delegation (`by`) works (Kotlin Slack discussion of a 2025 podcast; details not verified). In a 2020
interview (Habr) his bigger regret was organisational: not building a team early, and shipping 1.0
in 2016 "after Java 8", too late.

### Smart casts stop at the type system's edge
Smart casts only apply to stable values: local `val`s, `val` properties without custom getters in
the same module, not `var` properties, not open properties, not anything that could change between
check and use. This is sound but surprising; the standard workaround (`?.let { }`) is noise, and the
K2 improvements list is a catalogue of cases users hit. Dart's "why can't I promote this field"
guide documents the identical complaint, which suggests flow typing without ownership/immutability
information has a natural ceiling.

### Governance centred on one company
The Lead Language Designer and the Foundation president are JetBrains employees; the Foundation's
own page says it "does not directly make decisions about the course of the project". Google's
membership (and Android's dependence) provides a counterweight, but no process allows the community
to block or force a feature. The IDE is JetBrains' product; a Kotlin LSP for VS Code appears on the
2026 roadmap (KT-80322) a decade after the language shipped, and KAPT/Gradle integration has always
been best in IntelliJ. This is a conflict-of-interest structure that has so far been managed well,
but the dependence is total.

### Feature growth
Kotlin 2.2-2.4 added guard conditions in `when`, non-local `break`/`continue`, multi-dollar
interpolation, nested type aliases, explicit backing fields, context parameters, collection literals
(experimental), name-based destructuring (experimental), an unused-return-value checker
(experimental), with rich errors, static extensions, value classes and `lateinit val` in design.
Each is justified individually; the sum is a language that is visibly larger than the 2016 one, and
JetBrains has not published a complexity budget. Breslav's successors have not (yet) had a
Scala-style "Lean Kotlin" moment, but the trajectory is the one Scala followed.

## Designer retrospectives
- **Andrey Breslav** (creator): the ternary omission was his biggest regret and cannot be
  retrofitted; delegation-by-`by` is a second; organisationally he regrets not building a team and a
  management mentor earlier and shipping after Java 8. He also stresses that Kotlin's success came
  from the IDE-first approach and from interop, not from language novelty (Pragmatic Engineer,
  February 2026; Habr, May 2020). His next project, CodeSpeak, abandons conventional language design
  in favour of specification-level programming with LLMs, which is itself a retrospective verdict
  that "the next Kotlin" is not another Kotlin.
- **Roman Elizarov** (coroutines): the original `launch`-anywhere design lost child failures and
  leaked work; structured concurrency (2018) made scopes mandatory and `GlobalScope` an escape
  hatch. He has defended `suspend` colouring as a feature.
- **JetBrains Kotlin/Native team** (2020-2022): the frozen-object model had "a steep learning
  curve", could not share code with JVM and leaked; replaced wholesale.
- **JetBrains language team** (2025): context receivers abandoned; rich-errors motivation lists what
  is wrong with exceptions, nullable encodings, sealed encodings and `Result`; value-classes
  motivation lists what is wrong with data classes (reference semantics, shallow immutability,
  nested `copy`).
- **Evolution principles**: the team writes down that it will apply compiler bug fixes quickly even
  when technically incompatible "to stop bad code patterns from spreading", an explicit choice of
  correctness over stability that most languages leave implicit.

## Influence
- **Copied from Kotlin:** Java's records (JEP 395), sealed classes (JEP 409), `var` local inference
  (JEP 286), text blocks, pattern matching for `switch` and, prospectively, null-restricted types,
  all follow Kotlin's demonstration on the JVM; Dart 3's `sealed` and Dart's null safety (smart
  casts, `!`, `late` after `lateinit`); Swift's `guard let` idiom has a Kotlin twin in `?: return`;
  Scala 3's `extension` syntax; Java 21's StructuredTaskScope and Swift's task groups after
  kotlinx's structured concurrency; Jetpack Compose's lambda-with-receiver DSL style influenced
  SwiftUI's result builders only indirectly, but Compose Multiplatform competes with Flutter
  directly.
- **Kotlin copied from:** Scala (case classes to data classes, `val`/`var`, trailing lambdas,
  `object` singletons, declaration-site variance; Breslav has said Scala was the language they would
  have used if it compiled fast enough), C# (extension functions, properties, `?.`, `??`-as-`?:`,
  LINQ-free collection pipelines), Groovy (builders, safe navigation), Gosu (smart casts, per the
  Pragmatic Engineer write-up), Java (everything else), and Go/C# for the coroutine machinery being
  library-level.

## Lessons for Sisu
- **Adopt**
  - Null safety with `?`, `?.`, `?:`, flow-sensitive smart casts, and a deliberate, bounded unsafe
    form (`!!`) that is greppable. Kotlin plus Dart plus Swift is enough evidence that this is the
    solved shape; no language has gone back.
  - Extension functions with static resolution and import-scoped visibility. Cheap, teachable,
    copied everywhere.
  - Data classes and sealed hierarchies with exhaustive matching, but with value semantics (see
    Swift) rather than Kotlin's reference identity.
  - Stability levels with compiler-enforced opt-in for pre-stable features, written deprecation
    cycles, and a language committee that must sign off incompatible changes. The K2 rollout and the
    context-receivers revocation show the process absorbing a rewrite and a reversal without
    breaking stable code.
  - Concurrency primitives as a minimal language core (one keyword) with the policy in the library,
    so that generators, async, streams and channels do not each need syntax.
  - Structured concurrency with mandatory scopes, parent cancellation, and supervisor scopes for
    isolating failures.
- **Adapt**
  - Platform types are the right idea for FFI (do not force annotation of a foreign world before
    anything compiles) but should be a visible, grep-able type in Sisu rather than "non-denotable";
    the boundary should be inspectable.
  - Error handling: take Kotlin's 2025 conclusion as the starting point, not its 2016 one. A typed,
    value-based recoverable-error channel (union-like, cheap, exhaustively matched, with a
    propagation operator) plus a separate unrecoverable panic/exception channel. Do not ship
    unchecked exceptions alone and add the typed channel fifteen years later.
  - Function colouring: accept that `suspend`-style colouring is tolerated when the alternative is
    callbacks and contested when the runtime offers cheap blocking threads. Sisu should decide based
    on its runtime; if it has green threads, colour is optional and Kotlin's data does not apply.
  - Smart casts: Kotlin and Dart both hit the "cannot promote a mutable/open property" ceiling. If
    Sisu has immutability and uniqueness in types, flow typing can be extended further; design the
    two together.
  - Value types: Kotlin is retrofitting multi-field value classes and deep immutability because the
    JVM did not have them. A language without JVM constraints should make these the default from the
    start.
- **Avoid**
  - Choosing an interop target that fixes the type system's ceiling (erasure, boxing, no value
    types, no unsigned). Interop is the adoption engine but should be a boundary, not the model. If
    Sisu interoperates with C or a VM, let the VM's limits stop at the FFI.
  - Runtime-enforced "frozen to share" as the concurrency safety model without type-system support.
    Kotlin/Native's five-year experiment ended in total replacement.
  - Letting a feature sit in "experimental" for three years while production code adopts it (context
    receivers). Either time-box experiments or make them unusable from published libraries.
  - Dropping a syntax element (ternary) to free punctuation for another feature without measuring
    the cost of the replacement idiom. Breslav's regret is that the verbose form was accepted on
    paper and rejected in practice.
  - Single-IDE tooling. Kotlin's LSP arriving in year ten is a warning for any language built by a
    tools company.
- **Open questions raised**
  - Colouring vs green threads: Kotlin's `suspend` and Java's Loom now coexist on the same VM, which
    is the closest thing to a controlled experiment; results (library ecosystem splitting,
    performance in practice) are still arriving.
  - Can rich errors (error unions) be added to a language with exceptions without producing three
    incompatible conventions (exceptions, nullable, unions)? Kotlin is about to find out; Sisu
    should design for one convention from the start.
  - Does a language need a "complexity budget"? Kotlin's feature growth since 2.0 is steady and each
    feature is well argued; whether the cumulative result repeats Scala's trajectory is unresolved.
  - Is a single corporate steward with a weak foundation acceptable if the steward behaves well?
    Kotlin's decade says yes so far; Akka's licence change in the Scala ecosystem says the risk is
    real.

## Sources
1. Kotlin releases list (2.4.21, 8 October 2026; 2.4.0; 2.3.0).
   https://kotlinlang.org/docs/releases.html
2. "What's new in Kotlin 2.4.0". https://kotlinlang.org/docs/whatsnew24.html
3. "What's new in Kotlin 2.0.0" (K2 stable, smart-cast improvements, 10M LOC / 18,000 developers /
   80,000 projects). https://kotlinlang.org/docs/whatsnew20.html
4. "What's new in Kotlin 1.7.20" (new Kotlin/Native memory manager default, freezing deprecated, 29
   September 2022). https://kotlinlang.org/docs/whatsnew1720.html
5. "What's new in Kotlin 1.5.0" (unsigned types, sealed interfaces, value classes stable; 5 May
   2021). https://kotlinlang.org/docs/whatsnew15.html
6. Kotlin evolution principles (stability levels, deprecation, Language Committee).
   https://kotlinlang.org/docs/kotlin-evolution-principles.html
7. Kotlin language features and proposals (status table, snapshot 4 September 2026).
   https://kotlinlang.org/docs/kotlin-language-features-and-proposals.html
8. Kotlin roadmap (August 2026). https://kotlinlang.org/docs/roadmap.html
9. Calling Java from Kotlin: null-safety and platform types.
   https://kotlinlang.org/docs/java-interop.html
10. Kotlin exceptions documentation (all exceptions unchecked).
    https://kotlinlang.org/docs/exceptions.html
11. Kotlin Foundation structure (board, Lead Language Designer, Language Committee).
    https://kotlinfoundation.org/structure/
12. KEEP: Kotlin coroutines design document (revision 3.3, Kotlin 1.3).
    https://github.com/Kotlin/KEEP/blob/master/proposals/coroutines.md
13. KEEP-0441, "Rich errors: motivation and rationale" (2025).
    https://github.com/Kotlin/KEEP/blob/main/proposals/KEEP-0441-rich-errors-motivation.md
14. KEEP-0462, "Rich errors" (error union types; public discussion, 2026).
    https://github.com/Kotlin/KEEP/blob/main/proposals/KEEP-0462-rich-errors.md
15. KEEP-0453, "Better immutability: value classes, motivation" (2025-26).
    https://github.com/Kotlin/KEEP/blob/main/proposals/KEEP-0453-better-immutability-value-classes-motivation.md
16. KEEP proposals index (KEEP-0412 unused return value checker, 0416 collection literals, 0438
    name-based destructuring, 0448 explicit context arguments, 0449 companion extensions).
    https://github.com/Kotlin/KEEP/tree/main/proposals
17. JetBrains, "Update on context parameters", April 2025.
    https://blog.jetbrains.com/kotlin/2025/04/update-on-context-parameters/
18. JetBrains, "Kotlin/Native Memory Management Roadmap", July 2020.
    https://blog.jetbrains.com/kotlin/2020/07/kotlin-native-memory-management-roadmap/
19. JetBrains, "Kotlin/Native Memory Management Update", May 2021.
    https://blog.jetbrains.com/kotlin/2021/05/kotlin-native-memory-management-update/
20. JetBrains, "Try the New Kotlin/Native Memory Manager Development Preview", August 2021.
    https://blog.jetbrains.com/kotlin/2021/08/try-the-new-kotlin-native-memory-manager-development-preview/
21. JetBrains, "The Road to the K2 Compiler", October 2021.
    https://blog.jetbrains.com/kotlin/2021/10/the-road-to-the-k2-compiler/
22. JetBrains, "K2 Compiler Performance Benchmarks and How to Measure Them on Your Projects", 2024.
    https://blog.jetbrains.com/kotlin/2024/04/k2-compiler-performance-benchmarks-and-how-to-measure-them-on-your-projects/
23. JetBrains, "Celebrating Kotlin 2.0: Fast, Smart, and Multiplatform", May 2024.
    https://blog.jetbrains.com/kotlin/2024/05/celebrating-kotlin-2-0-fast-smart-and-multiplatform/
24. JetBrains, "Kotlin Multiplatform Is Stable and Production-Ready", November 2023.
    https://blog.jetbrains.com/kotlin/2023/11/kotlin-multiplatform-stable/
25. JetBrains, "Compose Multiplatform 1.8.0: Compose Multiplatform for iOS is Stable", May 2025.
    https://blog.jetbrains.com/kotlin/2025/05/compose-multiplatform-1-8-0-released-compose-multiplatform-for-ios-is-stable-and-production-ready/
26. Roman Elizarov, "Structured concurrency", Medium, 2018 (not re-fetched; paraphrased from
    KotlinConf 2018 slides and secondary sources).
    https://elizarov.medium.com/structured-concurrency-722d765aa952
27. Roman Elizarov, "Kotlin Coroutines in Practice", KotlinConf 2018 slides.
    https://www.slideshare.net/elizarov/kotlin-coroutines-in-practice-kotlinconf-2018
28. Roman Elizarov, "How do you color your functions?", Medium, 2020 (not re-fetched).
    https://elizarov.medium.com/how-do-you-color-your-functions-a6bb423d936d
29. Andrey Breslav, "Kotlin: challenges in language design", Curry On / ECOOP 2015 (abstract).
    https://2015.ecoop.org/event/curryon-kotlin-challenges-in-language-design
30. The Pragmatic Engineer, "The programming language after Kotlin, with the creator of Kotlin", 12
    February 2026. https://newsletter.pragmaticengineer.com/p/the-programming-language-after-kotlin
31. Habr, "Мы обречены" interview series with Andrey Breslav, 26 May 2020 (Russian).
    https://habr.com/ru/post/503956
32. Android Developers, Kotlin page ("over 60% of professional Android developers", "20% less likely
    to crash"). https://developer.android.com/kotlin
33. Android Developers Blog, Google I/O 2019 ("Android development will become increasingly
    Kotlin-first"; "over 50% of professional Android developers"), 7 May 2019.
    https://android-developers.googleblog.com/2019/05/google-io-2019-empowering-developers-to-build-experiences-on-Android-Play.html
34. JetBrains, "State of Developer Ecosystem 2025" (24,534 respondents; Kotlin 6% desired next).
    https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/
35. Stack Overflow Developer Survey 2025, technology section (Kotlin 10.8%).
    https://survey.stackoverflow.co/2025/technology
