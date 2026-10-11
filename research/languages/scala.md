# Scala

## Snapshot
| | |
|---|---|
| First appeared | 20 January 2004 (design from 2001); 2.0 March 2006; 2.8 2010; 2.10 2013; 2.13 2019; 3.0 13 May 2021 |
| Designed by | Martin Odersky, EPFL Programming Methods Laboratory (LAMP); Scala 3 (Dotty) by Odersky with Nicolas Stucki, Guillaume Martres, Sébastien Doeraene and others |
| Current version (Oct 2026) | Scala 3.9.0 LTS (3 September 2026; three-year support, succeeds 3.3 LTS whose last release is 3.3.8, June 2026); Scala 2.13.18 (November 2025) and 2.12.21 maintained by VirtusLab since October 2026 |
| Governance | Scala Center (EPFL non-profit, 2016) plus LAMP for the language; SIP committee for language changes; VirtusLab does most compiler releases; Lightbend (ex-Typesafe, founded 2011; rebranded as Akka) no longer plays a stewardship role after its 2022 Akka relicensing |
| Typing | Static, unified object-functional type system on the DOT calculus; local type inference; higher-kinded types, path-dependent types, union/intersection types, match types, opaque types; implicits replaced by `given`/`using`; experimental capture checking |
| Memory | JVM GC; Scala.js (JS engine GC); Scala Native (Boehm/Immix GC) |
| Compilation | AOT to JVM bytecode with TASTy sidecar files; Scala.js to JavaScript/WebAssembly (stable Wasm backend in Scala.js 1.22, 2026); Scala Native via LLVM |
| Primary domains | Data engineering (Apache Spark, Kafka tooling, Flink APIs), backend services (Play, Akka/Pekko, ZIO, http4s), finance, DSL-heavy libraries |
| Notable users | Twitter/X (since 2009), Databricks, LinkedIn (historically), The Guardian, Netflix (tooling), Tesla, Apple, Zalando, Disney Streaming; Stack Overflow 2025: 2.6% of respondents; JetBrains 2025: 2% primary language, but "leads among the top-paid developers" |

## Stated design goals
Odersky's goal from 2001 was a *scalable* language ("Scala" = scalable): one that grows with the
user from scripts to large systems, by fusing object-oriented and functional programming on a small
core, so that libraries rather than language features provide most abstractions (collections,
actors, parsers, futures were all libraries). Everything is an object; every operation is a method
call; `for` is a library-defined comprehension; implicits were the mechanism by which the language
could "program itself". Scala 3's goals (announced 2018, shipped 2021) were explicitly corrective: a
sound foundation (DOT), removal of "puzzlers", "focus on intent rather than mechanism" for
implicits, simpler syntax (optional braces, `enum`), and binary compatibility across versions via
TASTy. Odersky's 2024 "Lean Scala" post adds a third goal: a recommended simple style, because
"varying coding styles and over-abstraction" had become the language's main reputational problem.

## Strengths

### The OOP/FP fusion produced the idioms everyone now uses
Case classes, pattern matching, `Option`, immutable collections with `map`/`flatMap`/`filter`,
`for`-comprehensions over monads, traits with default implementations and linearised mixins,
first-class functions on the JVM: these were Scala's demonstration (2006-2012) that mainstream
developers would use functional data modelling if the syntax was object-friendly. The evidence is
imitation: Kotlin's data classes, `val`/`var`, trailing lambdas and sealed classes; Java's records,
sealed classes, `var`, pattern matching and streams; Swift's enums with associated values and
`guard`; C#'s records. Whether the fusion is *coherent* is a different question (see Weaknesses);
the pieces were right.

### Higher-kinded types work in industry
Scala is the only mainstream language where typeclass-based programming over `F[_]` (cats, scalaz,
ZIO's `ZIO[R, E, A]`, fs2 streams, tagless-final) has been used at scale by ordinary engineering
teams, not just by researchers. JetBrains' 2025 Scala plugin survey puts ZIO at 32% of respondents
(up from 27%) and ScalaTest/MUnit/ZIO Test as standard. This is evidence that HKTs are usable, with
two qualifications: the encoding (implicits as typeclass instances) was painful enough to be
redesigned, and most users consume HKT-based libraries rather than write them. Typed,
effect-tracking libraries (ZIO, cats-effect) also showed that structured concurrency and resource
safety can be done in a library on top of the JVM, which Kotlin's coroutines and Java's Loom later
did in language/runtime.

### Scala 3 fixed implicits by naming intents
The official reference calls implicits "Scala's most distinguished and most controversial feature"
and says "being very powerful, implicits are easily over-used and mis-used". Scala 2 had one keyword
covering implicit parameters, implicit conversions, typeclass instances, extension methods (via
implicit classes), and context passing. Scala 3 replaced the mechanism with four intent-named
features: `given` instances, `using` clauses, `extension` methods, and explicitly-imported implicit
conversions (`scala.Conversion`, now gated by `into`, SIP-71, stable in 3.9). The design principle,
"focus on intent rather than mechanism", is the clearest published fix of a "too powerful" feature
in any language. Kotlin's context parameters (2.4, 2026) are the same idea arriving by a different
route.

### TASTy and a real compatibility policy
Scala 2 broke binary compatibility at every minor version (2.10/2.11/2.12/2.13 each required
cross-publishing every library), because erased class files could not represent Scala types and
because the compiler's output changed. Scala 3 writes TASTy (typed abstract syntax trees) next to
class files; the compiler reads TASTy for separate compilation, macros, and tooling, and guarantees
backward compatibility across all 3.x releases (a library built on 3.x works on 3.y for y >= x),
with LTS lines (3.3, now 3.9) giving forward compatibility too. Scala 3 can consume Scala 2.13
libraries (except macros) and 2.13 can read Scala 3 TASTy up to 3.7. This ended the cross-build
treadmill for the 3.x line and is the model for any language with a rich type system targeting a
type-poor VM.

### Scala 3 removed what did not work
The dropped-features list is a retrospective in itself: existential types ("violate a type soundness
principle on which DOT and Scala 3 are constructed"), general type projections (unsound),
`DelayedInit`, procedure syntax, XML literals, symbol literals, auto-application, early
initialisers, class shadowing, the 22-arity limit, weak conformance, `private[this]`, Scala 2 macros
(replaced by principled `inline`/quotes/splices over TASTy), and nonlocal returns (deprecated). Few
languages have removed sixteen features in one release; it was possible because TASTy, migration
rewrites (`-source:3.x-migration -rewrite`) and the LTS policy gave users a path.

### Capture checking: the most serious attempt to track effects in a mainstream language
The Caprese project (EPFL, Odersky, Boruch-Gruszecki, Brachthäuser, Lhoták, Xu, Bračevac; TOPLAS
2023 "Capturing Types", OOPSLA 2025 "What's in the Box") extends the type system so that a type
`T^{c1, c2}` records which capabilities a value may retain; `A -> B` is a pure function, `A => B` is
`A ->{cap} B`. Checked exceptions become a `CanThrow` capability in scope; resources (a
`FileOutputStream^`) cannot escape the scope that owns them; async, region allocation and effect
polymorphism are stated goals. The 2025 OOPSLA paper reports migrating the standard collections and
the Gears async library "with minimal changes" using reach capabilities. Odersky's 2025 keynotes
frame it as the way to make "functional and imperative code safer together" and to get direct-style
(no monads) concurrency and errors with static guarantees. It is still "highly experimental and
unstable" after four years (3.9 release notes), but it is the only live industrial experiment in
effect tracking via capabilities rather than monads or effect rows, and directly relevant to Sisu's
effect-system choice.

### Tooling consolidation around Scala CLI and Metals
Scala 3.5 (August 2024) made Scala CLI the `scala` command: one tool to compile, run, test, package
and publish single-module projects with `//> using` directives, without sbt; Metals gained
best-effort compilation so completion works on code that does not compile; sbt 2 shipped in June
2026 after a decade on 1.x. Late, but the right correction to the sbt problem.

## Weaknesses

### Complexity and "too many ways to write it"
The standing critique, and the designer agrees. Paul Phillips (the most prolific Scala committer
until 2013, Typesafe co-founder per secondary sources) titled his PNW Scala 2013 keynote "We're
Doing It All Wrong": he had "lost faith in the language", said some problems "can't be fixed and
others won't be", that "there's still plenty of very basic stuff in the compiler which doesn't
work", and that "the leadership of scala has shown itself unfit stewards of the core technologies";
he left to write his own collections library. Odersky's 2024 "Lean Scala" post concedes the
user-facing half: Scala's "flexibility and expressiveness can also lead to challenges, such as
varying coding styles and over-abstraction", and proposes an official simple style ("simple,
understandable, concise, and expressive") for teams "without special requirements, newcomers, and
educators". The language itself has not shrunk (Scala 3 added union/intersection types, match types,
opaque types, enums, context functions, polymorphic function types, `inline`, quotes/splices, named
tuples, `into`), so the fix is cultural, and the 2024 Scalar recap notes the audience "recognising
the problem that there are too many ways to write Scala".

### Implicits, as shipped, were the canonical too-powerful feature
Scala 2 implicits enabled typeclasses, DSLs, magnet patterns, implicit conversions that silently
changed types, and compile-time behaviour that depended on import order. Error messages ("could not
find implicit value", "ambiguous implicit values") were a rite of passage; implicit conversions
between library types were the source of the "Scala puzzlers" genre. The Scala 3 redesign is an
admission; the fact that *undisciplined* implicit conversions are now behind a language flag and
`into` (SIP-71) took until 3.9 to stabilise shows how long it takes to retract a feature that
libraries depend on.

### Compile times
scalac has been slow since the 2.8 collections redesign; Phillips' critique and the Hacker News
commentary on it centre on compiler internals ("non-ideal" generated code, "best practices broken to
achieve performance"). Scala 3's compiler is a rewrite but not a fast one; implicit search,
type-level programming (match types, inductive givens) and macro expansion remain super-linear, and
the best-effort compilation mode exists because full compilation is too slow for interactive use.
Zinc incremental compilation and parallel module builds are mitigations, not fixes.

### Binary compatibility and the migration treadmill (Scala 2)
Each Scala 2 minor version was a new binary target; library authors cross-published for
2.10/2.11/2.12/2.13 for years (`%%` in sbt exists for this), and Spark, the ecosystem's largest
user, lagged each transition by years. Apache Spark 4.2 (2026) still supports Scala 2.13 only
("Since Spark 4.0.0, it's Scala 2.13"), five years after Scala 3.0. Scala 2 macros (experimental for
a decade, widely used) have no Scala 3 equivalent except rewriting, which blocked migration of
macro-heavy libraries. The JetBrains 2025 plugin survey (446 respondents) finds 59% "use Scala 3
regularly" and a further 15% have switched but still use 2 at work, four years after release: the
migration is succeeding slowly, which is itself the cost measurement.

### sbt
sbt (2008-) is a Turing-complete build DSL in Scala with its own key/scope/task algebra, slow
startup, and a plugin ecosystem that had to be "reforested" for sbt 2 (Scala Center blog, September
2026). The Scala user forum thread "Is Scala too complex?" answers that the syntax is fine but "some
things, like SBT, are too complex". Mill, Scala CLI and Bleep exist because of it; the Scala 3.5
decision to make Scala CLI the default runner is the designers' answer. sbt also had a
remote-execution vulnerability fixed in August 2026. A language whose build tool is harder than the
language is a recurring adoption blocker that Rust (cargo) and Go (go build) avoided by making the
build tool boring.

### Governance disputes and the Akka relicensing
The Phillips departure (2013) exposed the Typesafe-era split between EPFL research direction and the
commercial company; the Scala Center (2016) was created to give the open-source side an institution.
In September 2022 Lightbend changed Akka's licence from Apache 2.0 to BSL 1.1 (commercial licence
required for production use above US$25M revenue, reverting to Apache after three years), citing the
"tragedy of the commons". The ecosystem forked Akka into Apache Pekko; the JetBrains survey shows
Akka usage falling from 35% (2023) to 30% to 28% (2025) and Pekko rising from 15% to 22%. Lightbend
subsequently rebranded as Akka and plays no Scala stewardship role; VirtusLab (a consultancy) now
ships both Scala 2 and Scala 3 releases. The lesson is about the fragility of an ecosystem whose
flagship framework belongs to one company.

### Adoption decline from the peak
Scala's peak (roughly 2013-2017: Twitter, LinkedIn, Spark's rise, "Scala School") was followed by a
long decline in share. JetBrains' 2021 survey already said "Ruby, Objective-C, and Scala have all
decreased in popularity over the last 5 years"; the 2025 survey puts it at 2% as primary language,
with "learned in the past year" at 3% (5% in 2019); Stack Overflow 2025 has 2.6% usage. Widely
reported moves away (LinkedIn's 2016 statement on Scala, Twitter's increasing Java use) could not be
verified from primary sources in this pass and are omitted as evidence. Causes visible in the
record: compile times, the 2.x binary treadmill, sbt, the split between "better Java" and "Haskell
on the JVM" camps with incompatible library stacks (Akka/Play vs cats/scalaz vs ZIO), Kotlin taking
the "better Java" niche with a faster compiler and IDE, and Spark's Python API overtaking its Scala
API. The salary data (Scala leads top-paid developers) suggests the remaining users are experts,
which is consistent with a language that stopped recruiting beginners.

### The OOP/FP fusion is coherent in the core and incoherent in the ecosystem
The language can express both styles; the libraries chose sides. A Play/Akka codebase and a
cats-effect codebase share syntax and almost nothing else: different error handling (exceptions vs
`Either`/`IO`), different concurrency (actors/futures vs fibers), different type-level conventions.
Odersky's direct-style push (Gears, capture checking, `boundary`/`break`, Lean Scala) is an attempt,
nearly twenty years in, to make the core language's own style the default. "Both paradigms" without
a recommended default produced two ecosystems.

### Capture checking is still not shipped
First prototyped in 2021-22, the feature is "highly experimental and unstable, and it evolves
quickly" in the 3.9 LTS cycle; the standard library has a capture-checked variant but the ordinary
one does not; `Gears` (direct-style async) is experimental; the SIP for a stable subset has not been
accepted. Four-plus years from prototype to "not yet" is the realistic cost estimate for a
capability-based effect system in a production language, and the OOPSLA 2025 paper's own motivation
("their expressiveness has been insufficient for tracking capabilities embedded in generic data
structures, preventing them from scaling to the standard collections library") says the first design
did not scale.

## Designer retrospectives
- **Martin Odersky** on implicits: "Scala's most distinguished and most controversial feature";
  "easily over-used and mis-used"; Scala 3 "focuses on intent rather than mechanism" (Scala 3
  reference and book).
- **Odersky** on complexity (Lean Scala, April 2024): flexibility "can also lead to challenges, such
  as varying coding styles and over-abstraction"; the remedy is a recommended lean style rather than
  removing features.
- **Odersky** on soundness: existential types and general type projections were dropped because they
  "violate a type soundness principle on which DOT and Scala 3 are constructed"; Scala 3 was built
  on a proven core precisely because Scala 2's was not.
- **Odersky** on capabilities (Scala Days and Scala Workshop keynotes, August and October 2025):
  object capabilities "are not widely used because they are inconvenient, since passing them along
  long call chains is tedious, and unsafe" when enforced at runtime; EPFL's answer is `using`
  clauses and context functions for convenience and capture tracking in types for safety; "the
  project is two years in, aiming to make these ideas usable at scale".
- **Paul Phillips** (2013): the compiler internals and the process ("features added first and SIPs
  written afterward") were the problem; "I failed for half a decade to change that from the inside."
- **Scala Center / VirtusLab** (2024-2026): making Scala CLI the `scala` command, sbt 2, and the LTS
  policy are institutional admissions that tooling, not the type system, was the adoption blocker.

## Influence
- **Copied from Scala:** Kotlin (case classes to data classes, `val`/`var`, `object`, trailing
  lambdas, declaration-site variance, `by`-delegation from Scala traits; Breslav cites Scala as the
  language JetBrains would have used if it compiled fast enough), Java (records, sealed types,
  `var`, switch patterns, streams, `Optional`), Swift (enums with associated values, `guard` as an
  early-exit answer to nested matches, protocol extensions resembling implicit classes), Rust
  (traits with associated types via Haskell, `?`-style propagation resembles for-comprehensions'
  intent), C# (records, pattern matching), TypeScript and Ceylon (union types, which Scala 3 then
  took back), Flix and Unison (effect tracking lineage), Kotlin's context parameters (from Scala's
  `using`/context functions), Gleam and Elixir (nothing; they rejected the fusion deliberately).
- **Scala copied from:** Java (platform, syntax base), ML/Haskell (type inference ambitions,
  typeclasses as implicits, `for` as `do`), Erlang (actors via Akka), Smalltalk/Self (everything is
  an object, uniform access), Pizza/GJ (Odersky's own Java generics work), Ceylon and TypeScript
  (union/intersection types in Scala 3), Python (optional braces/indentation syntax in Scala 3), E
  and Wyvern (object capabilities for Caprese).

## Lessons for Sisu
- **Adopt**
  - Intent-named context passing (`given`/`using`, context functions) instead of one general
    implicit mechanism. Scala paid fifteen years to learn this and Kotlin arrived at the same design
    independently.
  - A typed intermediate format (TASTy) and a written compatibility policy with LTS lines, from the
    first release. Scala 2's binary treadmill is the strongest evidence that a rich type system over
    an erasing target needs its own interchange format.
  - Case classes, pattern matching, `Option`/`Either`, immutable collections as the default library
    shape; these are the parts of Scala every later language took.
  - A boring build tool shipped with the language (Scala CLI's directives model), not a
    Turing-complete DSL.
  - A published "recommended style" from day one so that there is one idiomatic ecosystem, not two.
- **Adapt**
  - Capability-based effect tracking (capture checking) is the most promising direction for Sisu's
    effects, but Scala's four years show the prototype must be designed with generics and
    collections from the start (reach capabilities), the notation must be mostly inferred, and it
    must ship as part of the core, not as an experimental import that libraries cannot depend on.
  - Higher-kinded types: provide them (industry can use them) but make the common cases
    (`map`/`flatMap` over a container, error propagation) available without typeclass encodings, so
    that HKTs are a library author's tool rather than a user-facing requirement.
  - Implicit conversions: allow them only at declared, opt-in boundaries (`into`-style), never
    globally.
  - Feature removal: Scala 3 shows sixteen features can be removed if the migration tooling
    (`-rewrite`) and compatibility format exist; Sisu should plan for removals and build the rewrite
    tooling early.
- **Avoid**
  - Shipping an experimental feature (Scala 2 macros, implicits-as-everything) that libraries then
    depend on for a decade; Scala 3's macro gap blocked migration longer than anything else.
  - A build tool more complex than the language.
  - A type system whose core was not proven sound before features were piled on; Scala 3's DOT work
    was a decade-long correction.
  - Letting the flagship framework belong to a single company with no licence commitment; Akka's
    relicensing split the ecosystem at its weakest moment.
  - Compiler performance as an afterthought; it is the single most repeated reason users cite for
    leaving Scala, and Kotlin won the "better Java" niche partly on it.
- **Open questions raised**
  - Can capability/capture tracking give effect safety with acceptable annotation burden in ordinary
    code? Scala's answer is "not yet, but closer each year"; it is the question Sisu's effect design
    must answer.
  - Can a language be both "better Java" and "typed FP" without producing two ecosystems? Scala's
    data says no unless the language picks a default; Odersky's direct-style push is the live
    attempt to pick one.
  - Is binary/TASTy compatibility across minor versions compatible with fast language evolution?
    Scala 3's LTS/Next split is the compromise; whether it slows evolution is unmeasured.
  - How much language complexity do expert users want? Scala's remaining base is well paid and
    productive; the language lost newcomers, not experts. A language can choose to be Scala-shaped
    on purpose, but it should know it is choosing.

## Sources
1. Scala download page (3.9.0 Next/LTS, 3.3.8, 2.13.18, 2.12.21).
   https://www.scala-lang.org/download/
2. "Scala 3.9 LTS released!", Wojciech Mazur (VirtusLab), 3 September 2026.
   https://www.scala-lang.org/news/3.9/
3. scala/scala3 release notes 3.9.0 (capture checking experimental, `into` stable, Scala.js 1.22
   Wasm). https://github.com/scala/scala3/releases/tag/3.9.0
4. Scala blog index 2026 ("sbt 2 is now available!" 29 June 2026; "VirtusLab assumes responsibility
   for Scala 2 maintenance" 7 October 2026; "Reforesting the sbt plugin ecosystem" 16 September
   2026). https://www.scala-lang.org/blog/
5. "Scala 3.5.0 released" (Scala CLI as `scala`, named tuples experimental, best-effort
   compilation), 22 August 2024.
   https://www.scala-lang.org/blog/2024/08/22/scala-3.5.0-released.html
6. Scala 3 reference, "Contextual Abstractions" (implicits "most distinguished and most
   controversial"). https://docs.scala-lang.org/scala3/reference/contextual
7. Scala 3 book, contextual abstractions intro ("intent rather than mechanism").
   https://docs.scala-lang.org/scala3/book/ca-contextual-abstractions-intro.html
8. Scala 3 reference, dropped features index and pages (existential types, DelayedInit).
   https://docs.scala-lang.org/scala3/reference/dropped-features/index.html
9. Scala 3 reference, "Capture Checking" (experimental).
   https://docs.scala-lang.org/scala3/reference/experimental/cc.html
10. Scala 3 guide, "TASTy overview". https://docs.scala-lang.org/scala3/guides/tasty-overview.html
11. "Binary Compatibility of Scala Releases".
    https://docs.scala-lang.org/overviews/core/binary-compatibility-of-scala-releases.html
12. Boruch-Gruszecki, Odersky, Lee, Lhoták, Brachthäuser, "Capturing Types", ACM TOPLAS
    2023. https://plg.uwaterloo.ca/~olhotak/pubs/toplas23.pdf
13. Xu, Bračevac, Pham, Odersky, "What's in the Box: Reach capabilities for capture checking",
    OOPSLA 2025 (arXiv 2509.07609). https://arxiv.org/abs/2509.07609
14. Odersky, "Where Are We With Scala's Capabilities?", Scala Workshop / IWACO keynote, 14 October
    2025 (abstract).
    https://2025.workshop.scala-lang.org/details/scala-2025/15/Where-Are-We-With-Scala-s-Capabilities-
15. Scala Days 2025 talks: "Scala capabilities" (Odersky), "The first steps towards practical
    capture checking" (Pham), "Hands-on capture checking".
    https://scaladays.org/editions/2025/talks/scala-capabilities
16. The New Stack, "Scala Creator Proposes 'Lean Scala' for Simpler Code", 17 April 2024 (secondary;
    Odersky's original post not re-fetched).
    https://thenewstack.io/scala-creator-proposes-lean-scala-for-simpler-code/
17. SoftwareMill, "Discover the highlights: Scalar Conference 2024" (Odersky's Lean Scala talk).
    https://softwaremill.com/discover-the-highlights-scalar-conference-2024/
18. Paul Phillips, "We're Doing It All Wrong", PNW Scala 2013 keynote slides.
    https://www.slideshare.net/slideshow/keynote-pnw-scala-2013/27366252
19. Hacker News discussion of Phillips' talk (quotes on process and compiler), 2014.
    https://news.ycombinator.com/item?id=8276765
20. Jonas Bonér, "Why We Are Changing the License for Akka", 7 September 2022.
    https://akka.io/blog/why-we-are-changing-the-license-for-akka
21. JetBrains, "The IntelliJ Scala Plugin in 2025" (survey: 446 respondents, 59% Scala 3 regularly,
    Akka 28%, Pekko 22%, ZIO 32%), 27 January 2026.
    https://blog.jetbrains.com/scala/2026/01/27/intellij-scala-plugin-in-2025/
22. JetBrains, "State of Developer Ecosystem 2025" (Scala 2% primary; top-paid).
    https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/
23. JetBrains, "The State of Java 2025" (Scala learned-in-past-year series).
    https://lp.jetbrains.com/the-state-of-java-2025
24. Apache Spark 4.2 documentation ("Since Spark 4.0.0, it's Scala 2.13").
    https://spark.apache.org/docs/latest/
25. sbt site (sbt 2.0.9 latest stable). https://www.scala-sbt.org/
26. Scala Users forum, "Is Scala too complex?".
    https://users.scala-lang.org/t/is-scala-too-complex/9279
27. Wikipedia, "Scala (programming language)" (dates, Typesafe founding, users).
    https://en.wikipedia.org/wiki/Scala_(programming_language)
28. Stack Overflow Developer Survey 2025, technology section (Scala 2.6%).
    https://survey.stackoverflow.co/2025/technology
