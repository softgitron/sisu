# Dart

## Snapshot
| | |
|---|---|
| First appeared | Unveiled 10 October 2011 (GOTO Aarhus); 1.0 November 2013; 2.0 August 2018; 3.0 10 May 2023 |
| Designed by | Lars Bak and Kasper Lund (V8 authors) at Google; Gilad Bracha wrote the Dart 1 spec; the Dart 2/3 language is led by Bob Nystrom, Lasse Nielsen, Erik Ernst, Leaf Petersen and Vijay Menon (Google) |
| Current version (Oct 2026) | Dart 3.13 (12 August 2026; 3.13.5 patch); 3.12 May 2026; 3.11 February 2026 |
| Governance | Google (the Dart team inside Google); language work tracked publicly in `dart-lang/language`; ECMA-408 standard (2014) is dormant |
| Typing | Static, sound, nominal, null-safe by default since 2.12/3.0; local inference; generics reified (not erased); `dynamic` as an explicit escape hatch; flow-sensitive type promotion |
| Memory | Generational tracing GC per isolate; no shared mutable heap between isolates |
| Compilation | Both modes: JIT (development, hot reload) and AOT (production native code, ARM/x64/RISC-V); dart2js and dart2wasm (WasmGC) for the web |
| Primary domains | Flutter apps (mobile, desktop, web, embedded), Dart CLI/server (Dart Frog, Serverpod, Cloud Functions since 2026) |
| Notable users | Google Pay, Alibaba, BMW, Toyota, Nubank, eBay, Sonos, Capital One (flutter.dev); Stack Overflow 2025: 5.9% of 31,771 respondents |

## Stated design goals
Dart 1 (2011) was pitched as "a structured web programming language", a JavaScript replacement with
a VM that Google intended to ship in Chrome, optional types for "big web apps", and the dynamic feel
of JavaScript with better tooling. That niche failed (below). Dart 2 (2018) restated the goal as
"optimized for client-side development" with a sound type system, and since Flutter 1.0 (December
2018) the de-facto goal has been to be the best language for Flutter: fast iteration (hot reload),
fast startup (AOT), predictable performance (sound types, no JIT on iOS), and a small enough
language that Flutter developers from JavaScript, Java and Swift can learn it in days. Bob Nystrom's
null-safety document states the design principle: "Code should be safe by default", with unsafe
behaviour opt-in and visible at the use site.

## Strengths

### The sound null-safety retrofit: the best-documented ecosystem migration
Dart 2.12 (3 March 2021) made types non-nullable by default with `?`, flow analysis, `late`, `!`,
and shipped `dart migrate`, an interactive tool that proposes nullability for an existing package,
explains each inference, and accepts hint comments (`/*?*/`) to steer it. The migration was *gradual
by language version*: each package declares a language version in `pubspec.yaml`, so null-safe and
legacy code could mix in one program ("unsound null safety"), and soundness arrived only when every
library had migrated. At launch 98% of the top-100, 78% of the top-250 and 57% of the top-500
pub.dev packages had migrated (the team migrated the core first and asked authors to migrate leaves
before roots); by December 2022 about 85% of `flutter run` executions used sound null safety; by May
2023, 99% of the top 1,000 packages did. Dart 3 then removed legacy mode entirely ("pubspec files
with an SDK constraint below 2.12 no longer resolve"), with the stated reason that three modes
"added overhead to the compilers and runtimes, which slowed SDK evolution". The payoff was measured:
a trivial getter went from 26 instructions (Dart 1 profile-guided JIT) to 10 (Dart 2 AOT) to 3 (Dart
2.12 sound null safety), because the compiler could drop null checks and inline. Two years from
opt-in to mandatory, with tooling, per-package versioning, and a hard cut at the end, is the model
other languages (Python's typing, TypeScript's strictness flags) have not managed.

### Sound static types after optional types failed
Dart 2 (August 2018) replaced Dart 1's optional, unsound types ("similar to typed JavaScript
dialects such as TypeScript or Flow") with a sound system plus runtime checks where static checks
cannot reach. The team's own account of why: with unsound types "static types may be incorrect at
runtime", so the compiler "couldn't assume an object had the expected field layout" and AOT code had
to keep speculative checks; `new List.from(anyIterable)` could trick the analyzer; implicit
downcasts were universally disliked ("most users never liked implicit downcasts") and were removed
with null safety. Strong mode shipped as an opt-in analyzer mode in 1.19 (2016) to test the water
before it became the only mode. The reversal is the clearest industrial admission that optional
typing, in the Bracha sense (types that never affect runtime), does not deliver either the safety or
the performance that justify writing the annotations.

### Hot reload as the adoption feature
Flutter's stateful hot reload compiles only the changed libraries to kernel files, sends them to the
running Dart VM, which swaps the library definitions and rebuilds the widget tree without rerunning
`main()` or `initState()`, preserving navigation and form state. It works because Dart ships a JIT
in debug mode alongside AOT for release; it is the single most-cited reason developers pick Flutter
over native toolchains, and the reason the Dart team killed macros (below) rather than accept slower
reload. Flutter web also supports hot reload and hot restart per current docs (the date it
stabilised on web was not verified in this pass). Limitations are documented and honest:
enum-to-class changes, generic signature changes, global initialisers and `main()` edits need a hot
restart.

### Isolates: no shared mutable memory
All Dart code runs in isolates, each with its own heap and event loop; communication is by message
passing (objects are copied, except immutable/transferable ones); `Isolate.run()` (2.19) gives a
one-call "compute this on another core" API, and isolate groups (2.15) share code and allow cheap
spawning. The documentation's claim is that "mutexes, locks, and data races won't occur in Dart",
which is true within the model. Combined with `async`/`await` and `Stream` on a single-threaded
event loop, this is the simplest concurrency model of the four platform languages, and the one with
the fewest documented production disasters.

### Both JIT and AOT from one source
The VM supports JIT with hot reload for development and AOT native compilation for release, plus
dart2js and dart2wasm (3.4, 2024) for the web, all from one kernel IR. No other mainstream language
offers stateful hot reload in development and iOS-compatible AOT in production from the same
compiler; the cost is a large VM team and an unusual compiler architecture, which Google funds.

### Records, patterns and sealed types (Dart 3)
Dart 3 (May 2023) added records (`(double x, double y)`, structural value types), patterns with
destructuring in `switch`, `if-case` and declarations, switch expressions, exhaustiveness checking
over `sealed` hierarchies, and class modifiers (`interface`, `base`, `final`, `sealed`) that let
library authors control subtyping. These arrived together as one coherent batch rather than
piecemeal, and the design documents are public. Dart 3.13's primary constructors (August 2026) and
the planned value classes are the follow-up to the most-requested feature, data classes; Nystrom's
design post is explicit that "primary constructors add no new capability" and that the team weighed
"the added language complexity against clearer code".

### Honest cancellation of macros
After roughly three years of public prototyping (static metaprogramming was on the roadmap from 2021
and previewed at Google I/O 2024), the team announced on 29 January 2025 that "we've made the
difficult decision to stop our work on macros". The reasons given: "each time we solved one major
technical hurdle, a new one appeared"; the chosen design (deep semantic introspection of types)
meant that any change to code forced recomputation of macro output, which "introduce[d] large
compile-time costs which made it difficult to keep stateful hot reload *hot*"; "we're simply too far
away from shipping macros with the developer-time performance we require." The replacement plan is
narrower features (augmentations, primary constructors, value classes, better `build_runner`).
Cancelling a flagship feature publicly, with reasons, is itself a strength of the process; it is
listed under Weaknesses for what it says about macros.

## Weaknesses

### The original mission failed and the language survived only by pivot
Dart 1 was meant to replace JavaScript with a VM in Chrome; the plan drew public opposition from
other browser vendors and web standards people, the Chrome VM was dropped with Dart 1.9 in March
2015, and Dart's only route to the web became compilation to JavaScript, where it competed with
TypeScript and lost. The language was rescued by Flutter (2017-2018), an internal Google project
that needed a language with AOT, JIT, a GC tuned for UI allocation patterns, and a team willing to
change the language for the framework. The web, Dart's original target, is now its weakest platform:
no isolates (web workers with copying only), Flutter web renders to canvas, and non-Flutter web
frameworks are niche (AngularDart is no longer developed by Google's Angular team). Dart's fate is
now Flutter's fate: the 2024 reports of Flutter team layoffs at Google and the Flock fork
announcement (October 2024) show that dependence cutting both ways (widely reported; not verified
against primary sources here). Lesson: a language without an owned niche does not survive on design
merit alone.

### Optional typing was the wrong bet
Dart 1 spent five years (2011-2016) with types that the runtime ignored. The team's own performance
post shows what it cost: double compilation, speculative checks, and no viable AOT story (which iOS
requires). Dart 2 broke every 1.x program ("Dart 1.x code had to be migrated to Dart 2"). The
reversal cost a full major version and a mandatory migration, and it happened only because Flutter
needed AOT; without that forcing function, the unsound system might have persisted (as TypeScript's
does).

### Macros: three years, then cancelled
The macro design wanted hygienic, type-aware, in-language macros that could generate
`fromJson`/`toJson`, data classes and more, replacing the
`build_runner`/`json_serializable`/`freezed` code-generation ecosystem that every Flutter project
carries. The cancellation post's diagnosis is the clearest statement in any language's record of why
type-introspecting macros are hard: semantic introspection makes macro output depend on the whole
program's types, so incremental compilation and hot reload lose their incrementality. Swift's macros
(out-of-process, syntax-only, over swift-syntax) made the opposite trade and got a build-time
problem instead. The two together say: macros that see types are expensive at every edit; macros
that do not see types are weak and need a big syntax library. As of 2026 the `macros` package is
discontinued at 0.0.1, augmentations are still unshipped as a user-facing feature, and serialisation
still goes through `build_runner`.

### Isolates are too coarse for the work people now want to do
The Dart team's own "Shared Memory Multithreading for Dart" proposal (`dart-lang/language`,
working/333) lists what isolates cannot do: "transferring data between isolates is asynchronous and
incurs copying costs", an isolate is "a bag of state guarded by a mutex" covering all static state,
and native code cannot call Dart synchronously from an arbitrary thread (audio threads, UI threads)
without blocking. The proposal adds shared static fields (`@pragma('vm:shared')`), shared isolates
(`Isolate.runShared`) that may touch only shared state, `NativeCallable.shared`, and eventually a
`dart:concurrent` library with `Thread`, `Lock`, atomics, with a JavaScript/Go-style memory model.
This is the same trajectory as Kotlin/Native (restrictive sharing model, then relaxation) in the
other direction: Dart started with no sharing and is adding it because FFI and parallel workloads
demand it. The web has no isolates at all (web workers with copying only), so cross-platform code
cannot rely on them.

### Code generation as a permanent tax
Because the language has no macros, no reflection in AOT (`dart:mirrors` is unavailable in Flutter),
and until 2026 no data classes, every Flutter project of size depends on `build_runner` with
`json_serializable`, `freezed`, `riverpod_generator` and the like, generating `.g.dart` files that
must be committed or rebuilt. `build_runner` is slow on large monorepos (the macros cancellation
commentary cites analyzer slowness as already painful), and generated code is a frequent source of
merge conflicts. Dart 3.13's primary constructors remove one use case; serialisation remains open.

### `dynamic` and `late` are visible escape hatches that get used
`dynamic` disables checking per expression; `late` defers a non-null guarantee to runtime (Nystrom
admits it is "more 'magical' than using `?`" because "a use can fail with no visible marker at the
use site"); `!` is a cast that "comes with a loss of static safety". These are the right escape
hatches, but `late` in particular is overused in Flutter widget state (`late final Controller c;`
initialised in `initState`) and produces the `LateInitializationError` crashes that null safety was
meant to end. The team chose `late` over Kotlin's `lateinit` restrictions to cover more cases; the
trade is more convenience, more runtime failures.

### Single-vendor language tied to a single framework
Google funds the Dart team and the Flutter team; the ECMA standard is dormant; there is no
foundation, no language committee with external members, and the language roadmap is set by
Flutter's needs. This has been efficient (null safety, records, patterns, and three releases per
year) and risky (the 2024 Flutter staffing reports produced immediate community anxiety and the
Flock fork attempt). The transparency of `dart-lang/language` (every feature has a public issue and
spec) is the mitigation.

## Designer retrospectives
- **Bob Nystrom**, "Understanding null safety" (2021): "It is not `null` that is bad, it is having
  `null` go where you don't expect it"; soundness was chosen over Kotlin-style static-only checks
  because "the compiler can drop redundant null checks, which yields smaller, faster code"; "Even
  so, the rules do cause friction"; `Map[]` stayed nullable because changing it "would have set the
  Dart ecosystem aflame"; "There is no one true solution to this problem".
- **Vijay Menon**, "Dart and the performance benefits of sound types" (January 2021): "Without
  soundness, type checking can only go so far, and static types may be incorrect at runtime"; the 47
  to 3 instruction progression.
- **Vijay Menon**, "An update on Dart macros & data serialization" (29 January 2025): "we are not
  seeing macros converging anytime soon toward a feature we are comfortable shipping"; "too far away
  from shipping macros with the developer-time performance we require".
- **Michael Thomsen**, "The road to Dart 3" (December 2022): legacy null-safety modes "added
  overhead to the compilers and runtimes, which slowed SDK evolution and made new features more
  costly".
- **Bob Nystrom**, "Bringing primary constructors to Dart" (August 2026): data classes were the most
  requested feature but "most commenters cared more about the easy way to define a class with a
  constructor and state than about value semantics"; the team avoided "syntactic cliffs" where a
  small addition forces a rewrite.
- The Dart 1 designers' optional-typing rationale (Gilad Bracha's "pluggable types") has no
  published retrospective from Bracha himself; the Dart 2 announcement and the performance post are
  the team's implicit verdict.

## Influence
- **Copied from Dart:** the staged, per-package-versioned null-safety migration with an interactive
  migration tool is the reference example cited in later migration plans (Python's and TypeScript's
  strict-mode discussions reference it); Flutter's hot reload set the expectation that Compose,
  SwiftUI previews and React Native had to meet; Dart's `extension type` (3.3) is a zero-cost
  wrapper similar to Kotlin's value classes and Swift's wrapper structs; the public cancellation of
  macros is now cited (including in Swift forums) as evidence about macro cost.
- **Dart copied from:** JavaScript (event loop, `async`/`await`, `Future`/`Promise`), Java/C# (class
  syntax, generics, `interface`), Kotlin and Swift (null safety design, smart casts, `late` after
  `lateinit`/`lazy`, `sealed`), Swift and Rust (patterns, records, exhaustiveness), Erlang (isolates
  as share-nothing processes), Strongtalk/Smalltalk (Bak and Bracha's background: optional types,
  mirrors, everything-is-an-object), and Go (`dart format`-style canonical formatting).

## Lessons for Sisu
- **Adopt**
  - Sound, non-nullable-by-default types with flow analysis, where "sound" means the compiler may
    rely on them for codegen. Dart measured the cost of unsoundness (double compilation, speculative
    checks, no AOT) and the benefit of soundness (3-instruction getters); no other language has
    published both numbers.
  - Per-package language versioning as the mechanism for every breaking change. It made a two-year
    ecosystem migration possible and is cheap to build in from the start.
  - An interactive migration tool as part of any breaking language change, shipped the same day as
    the change, with core libraries migrated first and a published progress metric.
  - JIT-for-development, AOT-for-release from one compiler, if Sisu wants hot reload. Dart shows the
    developer-experience payoff is large enough to drive adoption on its own.
  - Records and patterns shipped as one coherent batch with exhaustiveness over sealed types.
- **Adapt**
  - Share-nothing isolates are the right default for application code but Sisu needs the escape
    hatch Dart is now bolting on: a typed notion of shareable (immutable or synchronised) data and
    the ability for native threads to call in. Design the shared tier alongside the isolate tier,
    not after.
  - `late`: keep a declaration-site "initialised later" form, but make the use-site failure visible
    (a distinct type, or a required `!`-like marker) rather than Dart's invisible runtime check.
  - Macros: if Sisu has them, decide first whether they see types. Dart (type-aware, in-language)
    could not make them fast enough for hot reload; Swift (syntax-only, out-of-process) made them
    slow to build. The working middle ground in other languages is compile-time execution over a
    fixed, already-type-checked subset (see Zig/Jai analyses), not general introspection.
  - Code generation: assume that whatever the language does not provide (serialisation, equality,
    builders) will be provided by a generator, and budget for making that generator fast and
    conflict-free, or provide the features.
- **Avoid**
  - Optional (runtime-ignored) typing. Dart 1 is the industrial trial and it was reversed at the
    cost of a major version.
  - Launching without an owned niche. Dart 1's "replace JavaScript" mission died in 2015; the
    language lived only because Flutter adopted it.
  - Promising a flagship feature for years in public before its cost model is known. The macros
    cancellation was handled well, but three years of ecosystem waiting (and designs built around
    "macros will fix this") were lost.
  - Dependence on a single framework and a single company's staffing decisions without a governance
    body that can outlive them.
- **Open questions raised**
  - Can shared-memory parallelism be added to a share-nothing language without losing the "no data
    races" guarantee that made it attractive? Dart's working/333 proposal is the live experiment.
  - Is "sound" worth the migration cost for an existing ecosystem? Dart says yes once, under
    Flutter's forcing function; TypeScript has chosen never. For a new language the answer is
    clearly yes, but the data on existing ecosystems is one case.
  - How much does hot reload constrain language design? Dart gave up macros for it and keeps a JIT
    in the toolchain for it; a language without hot reload has more freedom and less adoption pull.

## Sources
1. Dart, "What's new" (SDK 3.13, 12 August 2026; 3.12; 3.11; 3.10; 3.9).
   https://dart.dev/resources/whats-new
2. Dart blog index (2025-2026 posts). https://dart.dev/blog
3. Vijay Menon, "An update on Dart macros & data serialization", 29 January 2025.
   https://dart.dev/blog/an-update-on-dart-macros-data-serialization
4. Dart, "Macros" language page (feature discontinued). https://dart.dev/language/macros
5. Bob Nystrom, "Understanding null safety". https://dart.dev/null-safety/understanding-null-safety
6. Dart, "Announcing Dart 2.12", 3 March 2021. https://dart.dev/blog/announcing-dart-2-12
7. Michael Thomsen, "The road to Dart 3: A fully sound, null safe language", 8 December
   2022. https://dart.dev/blog/the-road-to-dart-3-a-fully-sound-null-safe-language
8. Dart, "Announcing Dart 3", 10 May 2023. https://dart.dev/blog/announcing-dart-3
9. Vijay Menon, "Dart and the performance benefits of sound types", 19 January 2021.
   https://dart.dev/blog/dart-and-the-performance-benefits-of-sound-types
10. Bob Nystrom, "Bringing primary constructors to Dart", 20 August 2026.
    https://dart.dev/blog/bringing-primary-constructors-to-dart
11. Dart, "Concurrency in Dart" (isolates, event loop, web limitations).
    https://dart.dev/language/concurrency
12. Dart, language evolution and versioning. https://dart.dev/resources/language/evolution
13. Flutter, "Hot reload" documentation. https://docs.flutter.dev/tools/hot-reload
14. dart-lang/language, "Shared Memory Multithreading for Dart" (working/333).
    https://github.com/dart-lang/language/blob/main/working/333%20-%20shared%20memory%20multithreading/proposal.md
15. Dart, "Sound Dart and strong mode", January 2017.
    https://news.dartlang.org/2017/01/sound-dart-and-strong-mode.html
16. Wikipedia, "Dart (programming language)" (history, Chrome VM dropped with 1.9 in 2015,
    ECMA-408). https://en.wikipedia.org/wiki/Dart_(programming_language)
17. Dominik Roszkowski, "Dart macros are not coming", Occasional Flutter, February 2025 (secondary).
    https://occasionalflutter.substack.com/p/dart-macros-are-not-coming
18. flutter.dev (named users). https://flutter.dev/
19. Stack Overflow Developer Survey 2025, technology section (Dart 5.9%).
    https://survey.stackoverflow.co/2025/technology
