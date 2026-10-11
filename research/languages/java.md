# Java

## Snapshot
| | |
|---|---|
| First appeared | Announced 23 May 1995; JDK 1.0 released 23 January 1996 (Sun Microsystems) |
| Designed by | James Gosling with Mike Sheridan and Patrick Naughton (1991-95); language specification by Gosling, Bill Joy, Guy Steele, Gilad Bracha; libraries shaped by Joshua Bloch and Doug Lea. Since the Oracle era: Brian Goetz (Java Language Architect), Mark Reinhold (Chief Architect, Java Platform Group), Ron Pressler and Alan Bateman (Loom), Dan Smith (Valhalla) |
| Current version (Oct 2026) | JDK 27 (GA 15 September 2026, non-LTS); JDK 25 is the current LTS (16 September 2025); JDK 28 due March 2027 with value objects in preview [1][2][3] |
| Governance | Oracle leads OpenJDK (GPLv2+CPE); each platform release ratified through the Java Community Process (JCP); features proposed as JEPs; six-month cadence since 2018 (JEP 322), LTS every two years (17, 21, 25) |
| Typing | Static, nominal, class-based with interfaces; generics by erasure with wildcards (2004); `var` for locals (2018); records (2021), sealed types (2021), pattern matching for `switch` (2023). No null safety in the language (JSpecify annotations; a nullness-markers JEP is still a Draft) [4][5] |
| Memory | Tracing GC with several collectors: G1 (default everywhere since JDK 27, JEP 523), generational ZGC (JEP 439), generational Shenandoah (JEP 521; default mode in 28, JEP 535), Parallel, Serial. Compact object headers default in 27 (JEP 534). No value types until JEP 401 (JDK 28 preview) [1][3] |
| Compilation | Source to bytecode; HotSpot tiered JIT (C1/C2); GraalVM Native Image AOT; Project Leyden AOT cache (JEP 483/514/515) and AOT code compilation targeted at 28 (JEP 544) [2][3] |
| Primary domains | Enterprise and web backends, Android (now Kotlin-first), big data (Hadoop, Kafka, Flink), finance, games (Minecraft) |
| Notable users | Google, Amazon (Corretto), Netflix, Alibaba, LinkedIn, Apple services, most banks and governments |

Usage: 29.4 % of all respondents and 29.6 % of professional developers in the 2025 Stack Overflow survey (n = 31,771), third among general-purpose languages after Python and JavaScript/TypeScript [6]. JetBrains' 2025 ecosystem survey (n = 24,534) does not publish per-language shares in its summary page; its narrative notes Kotlin among languages in "slow, steady ascent" [7].

## Stated design goals

The 1996 white paper ("The Java Language Environment", Gosling and McGilton) lists the goals as "simple, object-oriented, and familiar; robust and secure; architecture-neutral and portable; high performance; interpreted, threaded, and dynamic" [8]. Several of those words carried specific commitments that shaped everything after:

- *Familiar* meant C++ syntax without C++'s unsafety: no pointers, no manual memory, no multiple inheritance of implementation, no operator overloading. Gosling's stated aim was to remove features that caused bugs in C++ programs.
- *Robust* was to be delivered by static typing, garbage collection, bounds checking, and checked exceptions: the compiler would force callers to confront failure modes.
- *Architecture-neutral* meant the bytecode and the JVM ("write once, run anywhere"), which made the virtual machine, not the language, the durable asset.
- *Secure* meant the sandbox and the `SecurityManager`, both now abandoned (JEP 411 deprecated it in 17, JEP 486 permanently disabled it in 24).

The modern goal, stated repeatedly by Goetz and Reinhold, is compatibility first: Java is explicitly "a conservative language" where "the cost of a feature is borne forever", and the Amber, Loom, Valhalla and Leyden projects are each framed as fitting new capability into the existing model without breaking it [9][10]. Reinhold's 2017 rationale for the six-month cadence was that a two-year feature-driven schedule made every feature "either rushed or late" [11].

## Timeline of design-relevant decisions

| Year | Event | Why it matters |
|---|---|---|
| 1996 | JDK 1.0: GC, bytecode, checked exceptions, `null`, `synchronized` on every object | The core commitments; all later retrofits work around these |
| 1997 | JDK 1.1: serialization, reflection, inner classes | Serialization later called a mistake by its own stewards [12] |
| 1998 | Collections framework (Bloch); HotSpot JIT (1999) | The JVM becomes the asset |
| 2004 | Java 5: generics by erasure, annotations, enums, autoboxing, `java.util.concurrent`, the Java Memory Model (JSR 133) | Erasure chosen for migration compatibility [13]; first mainstream formal memory model |
| 2006 | OpenJDK released under GPL | Governance moves to open source, still vendor-led |
| 2010 | Oracle acquires Sun | Licensing and trust become recurring issues |
| 2014 | Java 8: lambdas, streams, `Optional`, `java.time`; Valhalla and Amber start | Functional style arrives; checked exceptions collide with lambdas; `Optional` as a return-type-only retrofit [14] |
| 2017 | Java 9: module system (JPMS) after the JCP executive committee initially voted it down (May 2017) | Only time the JCP visibly overruled Oracle; modules adopted slowly |
| 2018 | Six-month cadence (JEP 322); preview features (JEP 12); `var` (JEP 286, JDK 10); JDK 11 LTS and Oracle JDK licence change | Evolution mechanism; licensing pushes users to other builds |
| 2020-21 | Text blocks (JEP 378), records (JEP 395), pattern `instanceof` (JEP 394), sealed classes (JEP 409), JDK 17 LTS | Verbosity attacked; sum types arrive as sealed + records |
| 2021 | Oracle v. Google decided by the US Supreme Court (API reimplementation is fair use) | Ecosystem risk removed |
| 2022 | Virtual threads preview (JEP 425, JDK 19); structured concurrency incubator (JEP 428) | Loom chooses user-mode threads over async/await |
| 2023 | JDK 21 LTS: virtual threads (JEP 444), pattern matching for `switch` (JEP 441), record patterns (JEP 440), generational ZGC (JEP 439), sequenced collections (JEP 431); Oracle moves to per-employee pricing (Jan) | Colourless concurrency and ADTs complete; licensing shock |
| 2024 | FFM API final (JEP 454, JDK 22); unnamed variables (JEP 456); JSpecify 1.0 nullness annotations | JNI replacement; null annotations standardised outside the language |
| 2025 | JDK 24: `synchronized` no longer pins virtual threads (JEP 491), `SecurityManager` disabled (JEP 486), AOT class loading (JEP 483). JDK 25 LTS: compact source files (JEP 512), module imports (JEP 511), flexible constructors (JEP 513), scoped values (JEP 506), compact headers (JEP 519), generational Shenandoah (JEP 521) [2] | Loom's main limitation fixed; "hello world" ceremony removed after 29 years |
| 2026 | JDK 26 (March); JDK 27 (Sept): G1 default everywhere (JEP 523), compact headers default (JEP 534), structured concurrency seventh preview (JEP 533), post-quantum TLS (JEP 527) [1] | |
| 2027 | JDK 28 (planned): value objects preview (JEP 401, integrated Aug 2026), structured concurrency final (JEP 543), AOT code compilation (JEP 544), Simple JSON API incubator (JEP 540) [3] | Valhalla ships its first piece after twelve years |

## Coverage of the 15 evaluation dimensions

1. **Syntax and readability.** C-family; historically the most ceremonious mainstream language (`public static void main(String[] args)`, getters/setters, anonymous inner classes). Since 2018 the ceremony has been cut steadily: `var`, text blocks, records, switch expressions, pattern matching, unnamed variables, compact source files. Still no properties, named arguments or default parameters.
2. **Type system.** Static, nominal; generics by erasure with use-site variance (wildcards); local inference only; records and sealed interfaces give algebraic data types; pattern matching with exhaustiveness over sealed hierarchies. No null safety, no value types (until 28), primitives outside the generic system.
3. **Memory management and safety.** Tracing GC is the strongest in any mainstream runtime (G1, ZGC, Shenandoah, all with sub-millisecond pause options). Prevents memory corruption entirely in pure Java; `Unsafe` is being phased out (JEP 471). Null dereference is a run-time `NullPointerException`.
4. **Mutability, aliasing and value semantics.** Mutable by default; `final` is shallow; everything but primitives is a heap reference with identity. Records give shallow immutability; immutable collections exist (`List.of`). No value semantics for user types until Valhalla.
5. **Error handling.** Exceptions, divided into checked (declared in signatures) and unchecked. The checked variant is the field's canonical failed experiment (see Weaknesses). `Optional` for absent results only.
6. **Concurrency and parallelism.** Platform threads, `java.util.concurrent`, a formal memory model, `synchronized` monitors on every object, and since 21 virtual threads (colourless); structured concurrency finalising in 28; scoped values (25). Data races are legal and undetected statically.
7. **Polymorphism.** Single class inheritance, multiple interface inheritance with default methods (8), erased generics with bounded wildcards, sealed hierarchies for closed sets. Dynamic dispatch everywhere by default (the JIT devirtualises).
8. **Metaprogramming.** Annotations plus annotation processors (compile-time code generation, e.g. Lombok, Dagger), reflection, `invokedynamic`, bytecode manipulation (ASM, ByteBuddy). No macros; Project Babylon (code reflection) is exploratory.
9. **Modules, packages, build.** Packages and the module system (JPMS, 9); no standard build tool or package manager in the JDK; Maven and Gradle compete, Maven Central is the de facto registry. Reproducibility depends on the build tool.
10. **Tooling.** IntelliJ IDEA and Eclipse are among the best IDEs in any language; `javac` is fast; JFR and JMX give production observability; `jshell` REPL; debugger and profiler support are mature. Documentation via Javadoc (now with Markdown, JEP 467).
11. **Interop and platforms.** JNI (painful) replaced by the Foreign Function and Memory API (JEP 454, 22); the JVM hosts Kotlin, Scala, Clojure, Groovy; runs on every server platform; Android uses a different runtime (ART) and lags the language. Bytecode is the stable ABI.
12. **Performance and compilation model.** Tiered JIT with speculative optimisation and deoptimisation; peak throughput near C for many workloads; slow startup and high baseline memory, being addressed by Leyden, CDS and Native Image. Escape analysis is limited by identity semantics (Valhalla's motivation).
13. **Standard library and ecosystem.** Enormous, mostly backward-compatible since 1996, with notable rotting corners (`Date`, `Vector`, serialization). Ecosystem (Spring, Jakarta EE, Kafka) is the largest for server software.
14. **Learnability.** Verbose but regular; beginners struggle with `==` versus `equals`, `Integer` caching, checked exceptions, generics wildcards, and the build tools. Compact source files (25) finally make the first program one line.
15. **Evolution and governance.** Six-month releases with preview and incubator stages; multi-year projects (Amber, Loom, Valhalla, Leyden, Panama) with public design notes; Oracle-led with JCP ratification; strong compatibility culture with enhanced deprecation (JEP 277).

## Strengths

### The JVM is the asset: GC engineering, JIT, observability
- **Feature.** Decades of garbage-collector research shipped as production options: G1 (region-based, now default in every environment, JEP 523), ZGC (concurrent, sub-millisecond pauses, generational since 21), Shenandoah (generational, default mode in 28); compact object headers cut per-object overhead from 96 to 64 bits (JEP 450/519/534) [1][2][3]. The HotSpot JIT speculates, inlines across virtual calls, and deoptimises on misprediction. Java Flight Recorder (open-sourced in 11) and JMX give low-overhead production profiling.
- **Why it is good.** Applications get memory safety, high throughput and observability without language-level effort, and the platform hosts other languages (Kotlin, Scala, Clojure, Groovy) that reuse all of it.
- **Evidence.** Kotlin, Scala and Clojure chose the JVM for the runtime, not the language; JEP 444's design is possible only because the JVM could change stack representation underneath unchanged code [15]. GC options are copied by .NET (regions in .NET 7) and studied by every runtime team.

### Backward compatibility as a thirty-year product
- **Feature.** Binary and source compatibility are design constraints on every change; enhanced deprecation (JEP 277, 2017) distinguishes "for removal" and gives multi-release warning. Erasure was chosen in 2004 precisely so that generifying a library needed no "flag day" [13].
- **Why it is good.** Code and skills retain value; library ecosystems compound. Goetz: "it was not considered acceptable to orphan all the code out there" [13].
- **Evidence.** Software compiled for JDK 1.1 generally still runs on JDK 27. The ecosystem's size (Maven Central) is the compounding result. The cost side is covered under Weaknesses.

### Virtual threads: the colourless answer, retrofitted successfully
- **Feature.** JEP 444 (JDK 21) adds user-mode threads scheduled by the JDK, so blocking code in the thread-per-request style scales to millions of concurrent requests with no `async`/`await` [15].
- **Why it is good.** The JEP's own argument: the asynchronous style "gives up loops and `try/catch` blocks as sequential composition tools", stack traces lose context, "debuggers can't step through request logic, and profilers can't attribute costs to callers" [15]. Virtual threads keep all of that and are a non-goal-preserving change ("not a goal to change the basic concurrency model of Java").
- **Evidence.** The design cites goroutines and Erlang processes as prior successes [15]. The main limitation (pinning under `synchronized`) was removed two releases later (JEP 491, JDK 24). Structured concurrency, previewed nine times from JDK 19 (JEP 428) to JDK 27 (JEP 533), is proposed to finalise "without further change" in JDK 28 (JEP 543), with scoped values (JEP 506) final in 25 [2][16]. This is the most careful rollout of a concurrency model in any mainstream language.

### Preview features and the six-month cadence as an evolution engine
- **Feature.** JEP 12 defines preview features: complete but not permanent, enabled with `--enable-preview`, with no compatibility promise until final. Releases every March and September (JEP 322); LTS every two years.
- **Why it is good.** Designs are tested on real code for one to nine releases before being frozen; mistakes are corrected (structured concurrency changed `fork` to return `Subtask` in 21, replaced constructors with factories in 25, added an exception type parameter in 27 [16]).
- **Evidence.** Every major Amber and Loom feature went through at least two previews. The cadence has shipped on schedule for 18 consecutive releases since 2018.

### Records, sealed types and pattern matching: ADTs in an OO language
- **Feature.** Records (JEP 395) are nominal tuples with value-based `equals`/`hashCode`; sealed interfaces (JEP 409) close a hierarchy; `switch` with type and record patterns (JEP 441, 440) checks exhaustiveness over sealed hierarchies; primitive patterns are in preview (JEP 532).
- **Why it is good.** Sum types and product types fit the existing object model with no new runtime concept; Goetz's "data-oriented programming" guidance gives idioms. Deconstruction patterns nest.
- **Evidence.** Adopted immediately in new code; Lombok's `@Value` is displaced by records. C# followed with `closed` hierarchies in C# 15 (2026) [17], and Kotlin's sealed classes predate both.

### A formal memory model and `java.util.concurrent`
- **Feature.** JSR 133 (2004) specified happens-before semantics for the language; Doug Lea's `java.util.concurrent` shipped the same year.
- **Why it is good.** Programmers and JIT writers have a shared contract; lock-free code is possible with defined semantics.
- **Evidence.** C++11, Go and Rust all adopted memory models modelled on Java's; `ConcurrentHashMap`, `ForkJoinPool` and `CompletableFuture` are the reference designs for their categories.

### Tooling and observability
- **Feature.** IntelliJ IDEA, JFR, JMX, async-profiler, `jcmd`, heap dumps, a fast `javac`, `jshell`, `jlink`/`jpackage`.
- **Why it is good.** Production diagnosis is routine rather than heroic; refactoring at scale is safe because the IDE understands the whole program.
- **Evidence.** IntelliJ's success led JetBrains to build Kotlin, whose main pitch was IDE-grade tooling on day one. JFR-style always-on profiling has been copied by .NET's EventPipe.

### Documentation convention
- **Feature.** Javadoc (1996), and Markdown doc comments since JDK 23 (JEP 467).
- **Evidence.** Every subsequent mainstream language (C# XML docs, Python docstrings' tooling, Rust `///`, Go doc comments, Kotlin KDoc) adopted structured doc comments rendered to HTML and shown by the IDE.

## Weaknesses

### Checked exceptions: the canonical failed experiment
- **Feature.** Exceptions extending `Exception` but not `RuntimeException` must be declared in `throws` clauses and caught or re-declared by every caller.
- **Failure.** Hejlsberg's 2003 critique names the two mechanisms: *versionability* ("adding a new exception to a throws clause in a new version breaks client code") and *scalability* ("in the large, checked exceptions become such an irritation that people completely circumvent the feature"), by declaring `throws Exception` or writing empty catch blocks to "come back and deal with... later" [18]. Lambdas made it structurally worse: the `java.util.function` interfaces declare no checked exceptions, so a stream pipeline cannot propagate an `IOException` without wrapping; JDK 8 itself added `UncheckedIOException` to cope. Spring, Hibernate and most frameworks translate checked exceptions into unchecked ones at their boundaries.
- **Evidence.** No mainstream language designed after Java adopted checked exceptions: C# rejected them explicitly [18], Kotlin "does not have checked exceptions" and must annotate `@Throws` for Java callers [19], Scala and Groovy ignore them. Bloch's *Effective Java* devotes an item to "avoid unnecessary use of checked exceptions". The closest modern relatives, Swift's typed throws (2024) and Rust's `Result`, are opt-in and compose with generics, which Java's version cannot.
- **Fixes attempted.** None in the language; the "fix" was the ecosystem abandoning them. This is strong evidence that signature-level error declarations fail when they do not compose with higher-order functions and when adding a case is a breaking change.

### Null: the billion-dollar mistake, with `Optional` as a partial retrofit
- **Feature.** Every reference type includes `null`; dereferencing throws `NullPointerException`.
- **Failure.** Hoare's 2009 description of null references as "my billion-dollar mistake" was about ALGOL W, but Java is its largest deployment. The compiler offers no help; nullness is documented in prose or annotations.
- **Fixes attempted.** `Optional` (JDK 8): Goetz's own description is that it was intended as "a limited mechanism for library method return types where there needed to be a clear way to represent 'no result'", not a general null replacement, and it is not serializable and not meant for fields or parameters [14]. Helpful NPE messages (JEP 358, JDK 14) improve diagnosis, not prevention. JSpecify 1.0 (2024) standardised `@Nullable`/`@NonNull` annotations across tools after a decade of incompatible ones (JSR 305, Checker Framework, JetBrains, Android). The language-level answer, "Null-Restricted and Nullable Types" (`Foo!`, `Foo?`), is a JEP Draft last updated July 2025, tied to Valhalla because null-free fields are what allow flattened layouts [5]. Thirty years after 1.0, null safety is still not in the language. Kotlin shipped it in 2016 on the same runtime, which demonstrates the gap is design, not platform.

### Type erasure and its consequences
- **Feature.** Generic type arguments are erased at compile time; `List<String>` and `List<Integer>` share one class.
- **Failure.** Primitives cannot be type arguments (`List<int>` is illegal; boxing costs memory and indirection); no runtime type information (`new T[]` is impossible, `instanceof List<String>` is illegal); heap pollution via raw types and unchecked casts; overloads cannot differ by type argument; arrays remain covariant and reified, so `Object[] a = new String[1]; a[0] = 1;` compiles and throws `ArrayStoreException`.
- **Evidence.** Goetz's defence is that in 2004 "the costs and benefits were out of line by several orders of magnitude" for reification and that gradual migration was non-negotiable [13]. The costs he lists are exactly the ones above. Valhalla's "generics over primitives" (JEP 218) remains future work twelve years later [4].
- **Fixes attempted.** None possible without a new type system; Valhalla's approach is to make boxing cheap rather than to reify.

### Project Valhalla: value types still not shipped after twelve years
- **Feature.** Value classes (`value class Point`) with no identity, enabling flattening, scalarisation and `==` by field values; started 2014 as "State of the Values".
- **Failure.** JEP 401 was created in August 2020 and integrated into the JDK 28 mainline in August 2026 as a preview that is "disabled by default", with flattening explicitly "done at the discretion of the JVM" and null-restricted types left to a separate draft [4][5]. Press reports describe the integration as over 197,000 lines across 1,816 files and quote Goetz as expecting it to remain in preview through the next LTS; these figures are from secondary sources and were not verified against OpenJDK records [20].
- **Why it took so long.** Identity is baked into every object: monitors (`synchronized`), `==`, `System.identityHashCode`, weak references, serialization, and the boxed `Integer` cache all assume it. Migrating `Integer`, `LocalDate` and friends compatibly (an explicit JEP 401 goal) required the whole platform to tolerate both kinds of object.
- **Lesson.** The identity/value distinction is foundational; a language that starts with identity everywhere pays for over a decade to add values. C# (structs from 1.0) and Go (values by default) never had this problem.

### Verbosity and ceremony, and what has been fixed
- **Failure.** Until 2025 the smallest program needed a class, a `public static void main(String[] args)`, and `System.out.println`. Data classes needed constructor, accessors, `equals`, `hashCode`, `toString`; the Lombok annotation processor exists because of this, and is widely used in enterprise code (share not verified here). No properties, named arguments or default parameters, so builders proliferate.
- **Fixes shipped.** `var` (JEP 286, 10), switch expressions (JEP 361, 14), text blocks (JEP 378, 15), records (JEP 395, 16), pattern `instanceof` (JEP 394, 16), sealed (JEP 409, 17), record patterns and `switch` patterns (JEP 440/441, 21), unnamed variables `_` (JEP 456, 22), module import declarations (JEP 511, 25), compact source files with instance `main` (JEP 512, 25), flexible constructor bodies (JEP 513, 25) [2]. The 2025 LTS is the first in which a beginner's first program is `void main() { println("hi"); }`.
- **Remaining.** Properties and named/default parameters are still absent; derived record creation (`with` expressions) was drafted (JEP 468) but has not shipped. Kotlin's data classes, properties and default arguments remain the comparison point that Java has only partly caught up to.

### The cost of compatibility: `Date`, `Vector`, serialization, `synchronized`
- **`java.util.Date` and `Calendar`** (1996-97): mutable, zero-based months, replaced by `java.time` (JSR 310, JDK 8) but never removed; two date APIs coexist forever.
- **`Vector`, `Hashtable`, `Enumeration`, `StringBuffer`**: synchronised legacy collections superseded in 1998 by the collections framework, still present and still used in old code.
- **Serialization** (1997): Goetz's 2019 assessment is that it "makes nearly every mistake imaginable", "constitutes an invisible but public constructor", "completely subverts the integrity of the object model", and is "just too hard to secure... for trusted code" [12]. Reinhold is widely reported to have called it a "horrible mistake" responsible for a large share of Java vulnerabilities (wording not re-verified here). The proposed replacement (data-only serialization through constructors and deconstructors) has not shipped as of JDK 27.
- **`synchronized` on every object**: a monitor word in every header and a `wait`/`notify` on `Object`; the cost is the reason compact object headers took until 2025-26 and why virtual threads initially pinned (JEP 444, fixed by JEP 491).
- **Removed, slowly**: `Thread.stop` (deprecated 1998, made to throw `UnsupportedOperationException` in JDK 20, 2023), `SecurityManager` (deprecated 17, disabled 24), finalization (deprecated for removal, JEP 421), applets. Each removal took a decade or more.

### No canonical build tool or package manager
- **Failure.** The JDK ships `javac`, `jar`, `jlink` and `jpackage` but no dependency manager. Maven (2004, XML, convention-over-configuration) and Gradle (Groovy/Kotlin DSL, incremental, more complex) split the ecosystem; both resolve against Maven Central. Reproducible builds, lockfiles and supply-chain verification are build-tool-specific add-ons; neither has a transparency log. JPMS (JDK 9) added a module system but not versioning, so "jar hell" (two versions of a library on one classpath) is still resolved by build tools, and library adoption of modules has been slow.
- **Evidence.** The Jigsaw JSR was initially voted down by the JCP executive committee in May 2017 partly over interaction with existing build and OSGi tooling. JetBrains' surveys consistently show Maven ahead of Gradle (shares not re-verified in this research). Go, Rust, Dart and Swift all learned from this by shipping a package manager with the language.

### Startup, footprint and the JIT trade-off
- **Failure.** Class loading, bytecode verification and JIT warm-up make a Spring Boot service take seconds to reach peak performance and hundreds of megabytes of baseline memory, which matters for serverless and container density.
- **Fixes attempted.** Class-data sharing; GraalVM Native Image (a different compilation model with closed-world assumptions that break reflection-heavy frameworks); Project Leyden's AOT cache (JEP 483 in 24, 514/515 in 25) and AOT-compiled code (JEP 544, targeted 28); compact object headers; generational ZGC/Shenandoah [2][3]. Progress is real but each is a mitigation of a model chosen in 1995.

### Shared mutable state and `==`
- Mutable by default, shallow `final`, and reference identity semantics: `==` on `Integer` compares references and is true only inside the -128..127 cache; `String` comparison with `==` is a classic beginner bug.
- Data races are permitted by the memory model and not detected; `synchronized` is coarse; immutability requires discipline (records help).
- Integer arithmetic wraps silently; `Math.addExact` is opt-in.

### Governance and licensing trust
- Oracle owns the trademark, leads OpenJDK, and (per its own contributor statistics) authors the large majority of commits; the JCP ratifies but rarely overrules. Trust has been damaged repeatedly: the 2010-21 Oracle v. Google litigation over the API, the 2019 Oracle JDK licence change that made production use of Oracle's builds commercial, and the January 2023 move to per-employee pricing for the Java SE subscription, which pushed users to Eclipse Temurin, Amazon Corretto and other OpenJDK builds. The technical governance (JEPs, public design notes, mailing lists) is exemplary; the commercial layer is not.

### Problem-to-fix map
Per the evidence standards, a shipped fix is an admission of the original problem. This table collects the Java ones in one place for the proposal's decision log.

| Problem | Fix shipped, proposed, or absent |
|---|---|
| Ceremony | JEP 286 `var` (10), 361 switch expressions (14), 378 text blocks (15), 395 records (16), 456 unnamed variables (22), 511 module imports and 512 compact source files (25) |
| No closed sum types | JEP 409 sealed (17), 440/441 patterns (21), 532 primitive patterns (preview) |
| Null | JEP 358 helpful NPE messages (14); `Optional` (8) for returns only; JSpecify 1.0 (2024); JEP Draft 8303099 (nullness markers) |
| Checked exceptions | No language fix; `UncheckedIOException` (8); ecosystem wraps them |
| Erasure and boxing | No fix; Valhalla JEP 401 (28 preview) makes boxes cheaper; JEP 218 generics over primitives still future |
| Identity on every object | JEP 390 value-based class warnings (16); JEP 401 value objects (28 preview) |
| Serialization | Goetz's 2019 design note; nothing shipped; filtering (JEP 290, 9) mitigates |
| Async style | JEP 444 virtual threads (21), 491 no pinning (24), 506 scoped values (25), 543 structured concurrency (28) |
| Security sandbox | JEP 411 deprecate (17), 486 permanently disable (24) |
| Finalization | JEP 421 deprecate for removal (18); Cleaner API (9) |
| Startup and footprint | CDS; JEP 483 AOT cache (24), 514/515 (25), 544 AOT code (28); JEP 450/519/534 compact headers; GraalVM Native Image |
| JNI | JEP 454 Foreign Function and Memory API (22) |
| Build tool and packaging | No fix in the JDK; `jlink` (9), `jpackage` (16) cover deployment only |
| Release model | JEP 322 six-month cadence (2018), JEP 12 preview features |

### Three of the traps, concretely
```java
// 1. Checked exceptions do not compose with lambdas.
List<String> lines = paths.stream()
    .map(p -> Files.readString(p))   // compile error: unreported IOException
    .toList();

// 2. Identity leaks through boxing.
Integer a = 127, b = 127, c = 128, d = 128;
System.out.println(a == b);          // true  (cached)
System.out.println(c == d);          // false (two objects)

// 3. Erased generics plus covariant arrays.
Object[] arr = new String[1];
arr[0] = 42;                         // compiles; ArrayStoreException at run time
List<String>[] bad = new List<String>[1];  // does not compile at all
```

### Did Java catch up with Kotlin and C#?
The assignment asks this directly. Kotlin (1.0, 2016) and C# are the two languages whose users most often cite what Java lacks; the table records the state in October 2026.

| Capability | Kotlin | C# | Java, Oct 2026 |
|---|---|---|---|
| Null safety in the type system | Errors since 1.0 | Warnings since C# 8 (2019) | Not in the language; JSpecify annotations; JEP Draft 8303099 [5] |
| Data classes | `data class` since 1.0 | Records, C# 9 (2020) | Records, JDK 16 (2021): caught up, minus `with`-style copies (JEP 468 drafted, not shipped) |
| Closed hierarchies with exhaustive matching | `sealed` + `when`, 1.0 | `closed`/`union`, C# 15 preview (2026) | Sealed (17) + pattern `switch` (21): caught up, and ahead of C# until 2026 |
| Properties | Yes | Yes | No |
| Named and default arguments | Yes | Yes (C# 4) | No |
| Extension functions | Yes | Yes (C# 3, 14) | No |
| String interpolation | Yes | Yes (C# 6) | No; string templates were previewed (JDK 21-22) and withdrawn in 2024 |
| Multi-line strings | Yes | Yes | Text blocks (15): caught up |
| Concise entry point | `fun main()` | Top-level statements (C# 9) | Compact source files (JDK 25): caught up |
| Lightweight concurrency | `suspend` coroutines (coloured) | `async`/`await` (coloured) | Virtual threads (21): colourless, ahead of both |
| Checked exceptions | None | None | Still present |
| User-defined value types | `value class` (single field) | `struct` since 1.0 | JEP 401 preview in JDK 28 (2027) |
| Unsigned integers, operator overloading | Yes | Yes | No |
| Local type inference | `val`/`var`, lambda returns | `var` | `var` (10); lambdas need target typing |

Verdict: on data modelling (records, sealed, patterns, text blocks, compact programs) Java has caught up or passed; on concurrency it is ahead; on null safety, properties, named arguments, extensions and value types it has not caught up, and null safety is the gap that costs the most. Google's Kotlin-first Android policy (May 2019) shows what happens when the gap persists on one platform.

## Designer retrospectives

- **Brian Goetz, "Background: How We Got the Generics We Have" (2020) [13].** Defends erasure as the only choice compatible with migrating the 2004 ecosystem, while listing its permanent costs: no primitives as type arguments, heap pollution, no runtime type information. The document exists because Valhalla keeps having to re-justify the decision.
- **Brian Goetz, "Towards Better Serialization" (2019) [12].** "Java's serialization makes nearly every mistake imaginable"; the fix is to "support serialization of data, not objects" through constructors and deconstruction patterns.
- **Mark Reinhold on the release cadence (2017) [11].** The two-year feature-driven model was abandoned because it made features "either rushed or late"; the six-month train is the correction.
- **Brian Goetz on `Optional` (2014) [14].** It was "a limited mechanism for library method return types", not a general solution to null; the broader solution is still a Draft JEP.
- **James Gosling** has defended checked exceptions in interviews over the years while conceding they are widely misused; the Java team's practical retrospective is in its own APIs (`UncheckedIOException`, functional interfaces that declare nothing).
- **JEP 444 [15]** is itself a retrospective on a decade of asynchronous Java (`CompletableFuture`, reactive frameworks): the asynchronous style is judged to have cost readability, debuggability and observability, and the platform chose to fix the runtime rather than the language.
- **JEP 401 [4]** lists among its goals migrating `Integer` and `LocalDate` "compatibly", an admission that identity everywhere was a mistake the platform must now carry both ways.

## Influence

**Copied from Java (evidence the ideas work):**
- The managed-runtime model (bytecode, JIT, GC, bounds checking, no pointers): .NET/C# 1.0 was explicitly Java-shaped; Dart, Kotlin and Scala inherited it.
- The formal memory model (JSR 133): C++11, Go, Rust, Swift.
- `java.util.concurrent` designs: `ConcurrentHashMap`, executors, fork/join copied into .NET TPL and elsewhere.
- Javadoc-style doc comments: every language since.
- Interfaces (from Objective-C protocols, per Gosling) became the mainstream alternative to multiple inheritance; default methods (8) were copied by C# 8.
- Annotations (5) copied C# attributes (2002) and were in turn copied by Kotlin, Dart, Rust (`#[derive]`) in spirit.
- Checked exceptions: copied by no mainstream later language; the strongest negative-adoption evidence in this study.
- Virtual threads (21): the first mainstream retrofit of user-mode threads onto an existing threaded language; being watched by .NET (which evaluated and shelved green threads in 2023) and Python.

**Copied by Java from others:** C++ syntax and class model; Smalltalk's VM, GC and everything-is-an-object; Objective-C interfaces; Mesa/Modula-3/CLU exception declarations; Pizza/GJ (Odersky, Wadler) for generics; C# for `var`, lambdas syntax (`->`), `switch` expressions and records (C# 9 and Java 16 were contemporaneous); Scala and Kotlin for sealed types, pattern matching and data classes; Go and Erlang for virtual threads (acknowledged in JEP 444); LINQ for streams; Scala/Guava for `Optional`; Kotlin for text blocks and `_` unnamed variables.

## Lessons for Sisu

- **Adopt**
  - *Preview features with an explicit opt-in flag and no compatibility promise*, plus a fixed release train. Nine previews of structured concurrency produced a better API than one design review could.
  - *A formal memory model from day one* (JSR 133 is the template every later language used).
  - *Records, sealed types and exhaustive pattern matching as the ADT trio*, including nested deconstruction. Java proves these fit a nominal, object-shaped language; Sisu should make them the native representation rather than a retrofit.
  - *Colourless concurrency via runtime-scheduled threads with structured scopes*: the Loom design (scope, fork, join, cancellation propagation, scoped values) is the most carefully reviewed in the industry.
  - *Enhanced deprecation* (JEP 277): "deprecated for removal" with a stated horizon, machine-checkable.
  - *Always-on, low-overhead production profiling* (JFR) as a runtime feature, not a third-party tool.
  - *Structured doc comments* rendered by the IDE and the documentation tool.
  - *Public multi-year design notes* for big projects (Amber, Valhalla, Loom), so decisions are traceable.

- **Adapt**
  - *Backward compatibility*: keep Java's seriousness about it, but pair it with a real removal mechanism (editions or a documented multi-release deprecation cycle that actually removes). Java shows the value of compatibility and also that "never remove" means `Date`, `Vector` and serialization live forever.
  - *Generics*: decide reification versus monomorphisation versus erasure before 1.0; Goetz's argument for erasure was about migrating an existing ecosystem, which Sisu will not have. Primitives must be first-class type arguments.
  - *Null*: non-nullable by default with an option type, as Kotlin did on the same runtime; Java shows the retrofit (annotations, `Optional`, a Draft JEP) takes decades and still is not done.
  - *Value types*: value semantics must be in the core type system from the start (identity should be opt-in, not default). Valhalla's twelve years are the cost of the opposite choice.
  - *Exceptions*: keep unchecked exceptions (or panics) for bugs; replace checked exceptions with a typed error channel that composes with generics and higher-order functions (effect polymorphism or `Result`-style unions). Hejlsberg's two failure modes, versionability and scalability, are the test any design must pass.
  - *Serialization*: never automatic; derive explicit, versioned, constructor-routed serializers (Goetz's "data, not objects").
  - *Release cadence*: six months with LTS is proven; a smaller team might choose twelve.

- **Avoid**
  - *Checked exceptions as designed*: non-polymorphic, breaking on addition, invisible to lambdas.
  - *Erasure*, if the ecosystem does not already exist.
  - *Identity (monitor, hash, `==`) on every object*. Reserve identity for types that ask for it.
  - *Array covariance* and other soundness holes accepted for convenience.
  - *Shipping without a build tool and package manager.* The Maven/Gradle split and the absence of a supply-chain story are the direct result.
  - *`==` meaning reference equality on boxed values.* Sisu should have one equality operator with value semantics and an explicit identity comparison.
  - *Silent integer wraparound* with opt-in checked arithmetic (`addExact`); invert the default.
  - *A commercial governance layer that can change the licence of the reference implementation.* Put the reference implementation and trademark under a foundation.

- **Open questions raised**
  - *Can declared errors work if they compose?* Java's version failed; Swift's typed throws, Rust's `Result` with `?`, and effect systems (Koka, Effekt) are attempts at a polymorphic version. The research question for Sisu is whether declared errors can be made cheap enough to keep, since the benefit (knowing what can fail) is real.
  - *Preview versus stability*: nine previews of structured concurrency is thorough, but it also means five years in which libraries could not depend on it. Where is the right cut-off?
  - *JIT versus AOT for a managed language*: Java's JIT delivers peak throughput that AOT rarely matches, at the cost of startup and footprint that Leyden is still repairing in 2026. Sisu must choose which to optimise, or how to get both (Leyden's "shift computation earlier or later" framing is the best current articulation).
  - *LTS cadence*: two-year LTS plus six-month features has worked for Java; it also leads to most production systems lagging the language by years. Is a single rolling track with editions better for a new language?

## Sources

1. OpenJDK, JDK 27 project page (GA 15 Sept 2026; JEPs 523, 527, 531, 532, 533, 534, 536, 537, 538). https://openjdk.org/projects/jdk/27/ (2026)
2. OpenJDK, JDK 25 project page (GA 16 Sept 2025; JEPs 470-521). https://openjdk.org/projects/jdk/25/ (2025)
3. OpenJDK, JDK 28 project page (targeted JEPs 401, 535, 539, 540, 541, 542, 544; proposed 543, 545, 546). https://openjdk.org/projects/jdk/28/ (accessed 7 Oct 2026)
4. JEP 401: Value Objects (Preview), status Integrated, release 28, created 2020-08-13, updated 2026-08-07. https://openjdk.org/jeps/401
5. JEP draft 8303099: Null-Restricted and Nullable Types (Preview), status Draft, updated 2025-07-24. https://openjdk.org/jeps/8303099
6. Stack Overflow Developer Survey 2025, Technology section. https://survey.stackoverflow.co/2025/technology/ (2025)
7. JetBrains, The State of Developer Ecosystem 2025 (n = 24,534, April-June 2025). https://devecosystem-2025.jetbrains.com/ (2025)
8. James Gosling and Henry McGilton, "The Java Language Environment: A White Paper", Sun Microsystems, 1996. https://www.oracle.com/java/technologies/introduction-to-java.html
9. Brian Goetz, talks and posts on Java language stewardship, e.g. "Java Language Futures" (Devoxx, various years); Project Amber design notes. https://openjdk.org/projects/amber/
10. Mark Reinhold, "The State of the Module System" and Java platform keynotes; OpenJDK project pages for Loom, Valhalla, Leyden. https://openjdk.org/projects/
11. Mark Reinhold, "Moving Java Forward Faster", Sept 2017. https://mreinhold.org/blog/forward-faster
12. Brian Goetz, "Towards Better Serialization", June 2019 (rev. 2021). https://openjdk.org/projects/amber/design-notes/towards-better-serialization
13. Brian Goetz, "Background: How We Got the Generics We Have (Or, how I learned to stop worrying and love erasure)", June 2020 (rev. Dec 2021). https://openjdk.org/projects/valhalla/design-notes/in-defense-of-erasure
14. Brian Goetz, answer on the intent of `Optional`, Stack Overflow, 2014. https://stackoverflow.com/a/26328555
15. JEP 444: Virtual Threads, delivered in JDK 21 (2023). https://openjdk.org/jeps/444
16. JEP 543: Structured Concurrency (proposed to target JDK 28; history of JEPs 428, 437, 453, 462, 480, 499, 505, 525, 533). https://openjdk.org/jeps/543 ; JEP 491: Synchronize Virtual Threads without Pinning (JDK 24). https://openjdk.org/jeps/491
17. Microsoft, "What's new in C# 15" (closed hierarchies, union types), Sept 2026. https://learn.microsoft.com/en-us/dotnet/csharp/whats-new/csharp-15
18. Bill Venners with Bruce Eckel, "The Trouble with Checked Exceptions: A Conversation with Anders Hejlsberg, Part II", Artima, 18 Aug 2003. https://www.artima.com/articles/the-trouble-with-checked-exceptions
19. Kotlin documentation, "Calling Kotlin from Java: Checked exceptions". https://kotlinlang.org/docs/java-to-kotlin-interop.html#checked-exceptions (accessed Oct 2026)
20. Secondary coverage of the JEP 401 integration (ADTmag, Aug 2026; InfoQ roundups), reporting about 197,000 changed lines and Goetz's expectation of continued preview status; not verified against OpenJDK primary records.
21. JEP 322: Time-Based Release Versioning (2018). https://openjdk.org/jeps/322 ; JEP 12: Preview Features. https://openjdk.org/jeps/12 ; JEP 277: Enhanced Deprecation (JDK 9). https://openjdk.org/jeps/277
22. JEPs for the verbosity reductions: 286 (`var`), 361 (switch expressions), 378 (text blocks), 394, 395 (records), 409 (sealed), 440, 441 (patterns), 456 (unnamed variables), 467 (Markdown doc comments), 511, 512, 513. https://openjdk.org/jeps/
23. JEP 486: Permanently Disable the Security Manager (JDK 24); JEP 421: Deprecate Finalization for Removal (JDK 18); JEP 454: Foreign Function and Memory API (JDK 22). https://openjdk.org/jeps/
24. Tony Hoare, "Null References: The Billion Dollar Mistake", QCon London 2009. https://www.infoq.com/presentations/Null-References-The-Billion-Dollar-Mistake-Tony-Hoare/
25. JSpecify 1.0 nullness annotations (2024). https://jspecify.dev/
