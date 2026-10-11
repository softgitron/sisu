# Designer retrospectives: what language designers say they got wrong

Compiled October 2026. The research README ranks designer retrospectives as the strongest class of evidence: the
people who made a decision have no incentive to invent problems with it. Each entry gives who, which language, the
claim, the source with year, and a one-line lesson for Sisu. Quotations were re-read from the cited source unless the
entry is marked `[paraphrase; source not re-fetched]`, in which case the wording is from memory of the talk or post
and should be checked before being quoted.

Sources that could not be re-fetched during compilation and are therefore paraphrased: Graydon Hoare's 2023 post
(Dreamwidth captcha), Ritchie's HOPL II paper (all mirrors returned 503), the Lex Fridman transcripts for Lattner and
van Rossum (404), Elm's 2015 news pages (empty responses), Peyton Jones's 2003 "hair shirt" slides (moved),
Kernighan's CSTR 100, and the talk videos by Armstrong, Czaplicki, Blow and Abrahams. Everything else was read from
the primary page or PDF.

---

## Null and error handling

### Tony Hoare, ALGOL W: null references, "my billion-dollar mistake"
- **Claim.** He introduced the null reference in ALGOL W in 1965 "simply because it was so easy to implement"; it has caused "innumerable errors, vulnerabilities, and system crashes" costing on the order of a billion dollars. The alternative (discriminated unions with a test) was known; making null a member of every type forces a check on every use.
- **Source.** "Null References: The Billion Dollar Mistake", QCon London, 2009 (InfoQ recording).
- **Detail.** The talk notes that newer languages (C#, Spec#, Java with annotations) had by 2009 introduced non-null reference parameters checked at compile time, and that the obvious alternative in 1965, discriminated unions with a test, had implementation problems he chose to avoid.
- **Lesson.** Nullability must be opt-in in the type system; the cost of retrofitting it (Kotlin, Swift, C# 8, Dart 2.12/3, TypeScript `strictNullChecks`) shows every later designer agreed.

### Anders Hejlsberg, C#/Java: checked exceptions
- **Claim.** Checked exceptions fail on *versionability* ("Adding a new exception to a throws clause in a new version breaks client code") and *scalability* (in large systems the lists "balloon out of control" to 40 or 80 exceptions, so programmers write `throws Exception` or empty catch blocks: "checked exceptions have actually degraded the quality of the system in the large"). He still values knowing what can be thrown and suggests analysis tools instead.
- **Source.** "The Trouble with Checked Exceptions", interview with Bill Venners and Bruce Eckel, Artima, 18 Aug 2003.
- **Lesson.** Error declarations in a public signature must be designed for evolution (open sets, wrapping, inference), or users will route around them.

### Robert Griesemer and the Go team: no syntax for error handling
- **Claim.** After `check/handle` (2018, "too complicated"), `try` (2019, ~900 comments; hid control flow and was presented as near-final), and `?` (2024, no broad support), the team decided in June 2025 to stop pursuing syntactic changes "for the foreseeable future", admitting "we neither have a shared understanding of the problem, nor do we all agree that there is a problem in the first place", even though "complaints about error handling have topped our annual user surveys for years".
- **Source.** R. Griesemer, "[ On | No ] syntactic support for error handling", go.dev/blog, 3 Jun 2025.
- **Detail.** Informal user studies found most participants guessed the meaning of `?` correctly, yet the proposal still attracted a flood of tweak requests; the team concedes it does not know "whether the core problem is check verbosity or the difficulty of constructing useful errors", notes that adding syntax would merely move the unhappy group to the other side, that "virtually everybody will need to start using it, lest their code become unidiomatic", and that language changes are expensive for a small team. Its summary: "Go has a perfectly fine way to handle errors, even if it may seem verbose at times."
- **Lesson.** Error-handling ergonomics cannot be retrofitted by consensus once there is an installed base; decide the propagation syntax before 1.0.

## Simplicity, complexity and feature creep

### Bjarne Stroustrup, C++: "Remember the Vasa!" and the HOPL retrospective
- **Claim.** 2018: "Many/most people in WG21 are working independently towards non-shared goals. Individually, many (most?) proposals make sense. Together they are insanity to the point of endangering the future of C++" (43 pending papers listed). 2020: the process rewards "isolated features" that "add to the complexity of learning and implementation" and "often turn out to have surprising feature interactions"; "Enthusiasm favors the new"; concepts took "a dozen years"; the absence of a shared foundation library was "the worst mistake of early C++". Sutter's P3081 profiles (2024) are the admission that lifetime safety was never solved in the language proper.
- **Sources.** P0977R0, 6 Mar 2018; "Thriving in a Crowded and Changing World: C++ 2006-2020", HOPL IV, 2020; H. Sutter, P3081R0, Oct 2024.
- **Detail (HOPL IV, section 9).** Stroustrup names the committee's failure modes with examples: *delays* (concepts six years, contracts six years to failure, networking fifteen years and unfinished, constexpr five years, even `nullptr` three years); *isolated features* that "rarely have major impact on how the language is used but add to the complexity of learning and implementation"; *late alternatives* raised by members who "have so far not taken an interest" when a proposal nears a vote (structured bindings, operator dot, modules, coroutines, contracts); *enthusiasm favours the new* ("Yet every new feature has a cost: e.g., design, specification, implementation, revision, deployment, and teaching"; "I fear Thursday afternoons in the evolution working group"); and *overconfidence* from everyday experience that is "inadequate" to the breadth of C++'s users.
- **Lesson.** Language evolution needs a single design authority with a complexity budget and a standard library from day one; committee accretion is the documented failure mode.

### Herb Sutter, C++: safety profiles as the admission
- **Claim.** P3081R0 proposes enforced `type_safety`, `bounds_safety`, `initialization_safety`, `lifetime_safety` and `arithmetic_safety` profiles, with an implementation that should "prioritize adoptability and safety improvement impact, especially to silently fix" serious bugs in existing code; the companion essay "C++ safety, in context" (2024) argues for incremental hardening over a new language. The proposal itself cites Cyclone, CCured and SAL as the prior attempts, none of which reached mainstream C++.
- **Source.** H. Sutter, P3081R0, "Core safety Profiles: Specification, adoptability, and impact", 12 Oct 2024.
- **Lesson.** Retrofitting lifetime safety onto a language with raw pointers has been attempted for 25 years without a mainstream success; safety has to be in the object model, not in an optional profile.

### Niklaus Wirth, Oberon: "A Plea for Lean Software"
- **Claim.** Software grows fat because features are added to satisfy every request and tools hide cost; Oberon shows a whole system can be small; features should be removed when they do not pay for their complexity (Wirth's law: software slows faster than hardware speeds up).
- **Source.** IEEE Computer 28(2), Feb 1995 `[paraphrase; scanned PDF]`.
- **Lesson.** Measure the language by what it leaves out; every feature must justify its implementation and teaching cost.

### Rob Pike, Go: "What We Got Right, What We Got Wrong"
- **Got right.** A formal specification ("A compiler alone is not a specification"); a second implementation (gccgo) that "helped us refine the language and polish the specification"; the Go 1 compatibility promise; a standard library that kept the community on one toolkit; easy parsing enabling tools; gofmt ("showed it could be done well"); interfaces ("one of the best-designed things in Go"); goroutines and channels making concurrency mainstream.
- **Got wrong.** "The process of developing package management for Go was not done well": the team underestimated dependency resolution and the community "felt slighted" by the final (technically excellent) design. Generics: building generic containers into the language without exposing genericity, "we erred in stopping the conversation there". "We should have explained up front" that concurrency was about structuring servers; "we took too long to clarify the difference between parallelism" and concurrency. Documentation "took us too long" to centre on examples. "A theme throughout this talk is poor communication on our part."
- **Source.** GopherCon AU, Nov 2023; commandcenter.blogspot.com, Jan 2024. Also "Go: Ten years and climbing", Sep 2017 (credits the second implementation as "vital"; no regrets listed).
- **Also noted.** Writing the first compiler in C was "the right one for us at the time" even though it offended some; the Creative Commons licence on the gopher mascot led to attribution being "disputed, misassigned, or ignored"; and his closing observation is that Go code "looks and works the same regardless of who's writing it", which he thinks may be a first for a major language.
- **Lesson.** Spec, second implementation, formatter, compatibility promise and a blessed package manager are the parts worth copying; the omissions (generics, dependency management) cost a decade each.

### Russ Cox, Go: versioning and compatibility
- **Claim.** The original `go get` "always fetches the latest copy"; the team "ignored" release tags; relying on copying and on invoking VCS tools fragmented the ecosystem. The 2018 fix: the import compatibility rule, semantic import versioning, minimal version selection ("reproducible builds by default, without a lock file"). On compatibility (2023): "the most important design decision we made for Go 1"; a Go 2 that breaks Go 1 "is never going to happen"; GODEBUG lets programs opt out of behaviour changes for at least two years.
- **Sources.** "Go += Package Versioning", research.swtch.com, 2018; "Backward Compatibility, Go 1.21, and Go 2", go.dev/blog, Aug 2023.
- **Detail.** Cox classifies changes that honour the compatibility promise yet still break programs into output changes (a faster `sort` reordered equal elements; a better `compress/flate` broke reproducible builds), input changes (`strconv.ParseInt` accepting underscores; `net.ParseIP` rejecting leading zeros) and protocol changes (automatic HTTP/2; SHA-1 certificate deprecation). GODEBUG settings are kept for at least two years, follow the `go` line in `go.mod`, can be overridden per binary with `//go:debug`, and where possible report a `runtime/metrics` counter so operators can see whether the old behaviour is still used. He names the cost: forks insulate users from breakage but also from fixes, and the team cannot "be locked in to undocumented implementation details".
- **Lesson.** Versioning is a language-design decision (import paths, version selection), not a tooling afterthought; compatibility with a formal opt-out keeps upgrades "boring".

### Rich Hickey, Clojure: breaking changes
- **Claim.** "There is growth and there is breakage"; growth is accretion, relaxation and fixation; "Breaking changes are broken". Removing or changing a signature is breakage; add a new name instead. SemVer's major version "might just as well change the name" and is "in part, about how to ship breakage"; dependency hell is "mutability hell".
- **Source.** "Spec-ulation", Clojure/conj, 2016 (transcript).
- **Lesson.** Make growth the only kind of change the tooling makes easy; breaking changes get a new name, not a new version number.

### Walter Bright, D: "D at 20: Hits and Misses"
- **Misses.** A `bit` type ("Having more than one pointer type is just a disaster. I should have known better."); octal and binary literals; 80-bit and complex floats; autodecoding strings ("Then We Botched It ... still trying to dig our way out"); postblit; contracts; "Emphasis on GC" (good for batch and CTFE, "Not so good for interactive programs because of pause time", "Uses 3x the memory of manual management"); "Exceptions are the default" ("not zero cost, and never were ... nothrow should be the default"); throwing destructors; "Safety should be default, not opt-in ... I underestimated how strong the desire for it is."
- **Hits.** No preprocessor, UTF-8, fixed-size integers, slices ("Fixes C's biggest mistake" of conflating arrays and pointers), built-in unit tests and documentation, CTFE, simple template syntax, modules, UFCS, scope guards, pure functions, struct/class value/reference dichotomy. "Learned: simple syntax wins; convenience wins; built-in wins over third party tool; everybody hates bloat; everybody loves colored error messages."
- **Sources.** Slides, 2019 (digitalmars.com/articles/hits.pdf); "C's Biggest Mistake", 22 Dec 2009.
- **Lesson.** Safety and `nothrow` as defaults, one string representation, one pointer type, built-in test/doc tooling; D's regrets are almost all defaults chosen for C compatibility.

### Andrey Breslav, Kotlin
- **Claim.** His "biggest design regret" is omitting the ternary operator: `if` was already an expression, which freed `?` and `:` for nullable types and annotations, but the result is verbose and "it was too late because you can't retrofit the ternary operator in the current syntax". Kotlin's first version was an IDE plugin before a compiler existed; Java interoperability was "the biggest challenge for Kotlin" (flexible/platform types).
- **Sources.** The Pragmatic Engineer podcast, 12 Feb 2026; "Flexible Types in Kotlin", JVMLS 2015; "Challenges in Language Design", Curry On 2015.
- **Lesson.** Syntax allocations are permanent; decide the expression-level conditional and the symbol budget early, and build the IDE experience alongside the compiler.

## Type systems

### Gilad Bracha, Strongtalk/Dart: pluggable optional types, and Dart's reversal
- **Claim.** "Mandatory Typing Considered Harmful": mandatory type systems "inhibit software reliability and security", reduce expressible programs and "make systems brittle"; types should be optional, have "no effect on the dynamic semantics", and be pluggable.
- **Counter-evidence that shipped.** Dart 1 followed this position. Dart 2 (2018) switched to a sound static type system; Dart 2.12 (2021) added sound null safety; "Dart has enforced sound null safety since Dart 3, released in May 2023", claiming "fewer bugs, smaller binaries, and faster execution". The language that most fully implemented the position reversed it within seven years.
- **Sources.** G. Bracha, "Pluggable Type Systems", OOPSLA workshop, 27 Oct 2004; dart.dev type-system and null-safety pages (2026).
- **His own cost list.** Bracha grants that mandatory types catch errors early, document interfaces and enable optimisation, and argues the disadvantages are less acknowledged: "a mandatory type system greatly reduces the number of" expressible programs, and "Once a mandatory type system is in place, the temptation to rely upon it is irresistible", which he says makes systems brittle when the type system is wrong or incomplete. Dart's history is the test of that argument.
- **Lesson.** Optional/unsound typing lost its own flagship; soundness is what tooling, performance and null safety are built on.

### Anders Hejlsberg, TypeScript: deliberate unsoundness
- **Claim.** Non-goal 3: "Apply a sound or 'provably correct' type system. Instead, strike a balance between correctness and productivity." Goal 9: "a consistent, fully erasable, structural type system". In 2025 the team ported the compiler to Go for ~10x (VS Code 77.8 s -> 7.5 s; memory roughly halved), an admission that the JavaScript implementation had hit a wall.
- **Sources.** TypeScript Design Goals wiki; "A 10x Faster TypeScript", 11 Mar 2025.
- **Detail.** The other goals explain the shape of every gradual type system since: "Statically identify constructs that are likely to be errors", "Impose no runtime overhead on emitted programs", "Emit clean, idiomatic, recognizable JavaScript code", "Avoid adding expression-level syntax", "Preserve runtime behavior of all JavaScript code"; non-goals include runtime type information and "Exactly mimic the design of existing languages". The 2025 plan keeps a JavaScript-based 6.x line alongside the native 7.0 and moves the editor service to the Language Server Protocol.
- **Lesson.** Unsoundness was the price of retrofitting types onto JavaScript; a fresh language has no such excuse, and checker throughput must be budgeted from the start.

### Martin Odersky, Scala: implicits and capabilities
- **Claim.** Scala 2 implicits were overused and misused; "most applications of implicit conversions have turned out to be of dubious value"; implicit imports "often lead to inscrutable type errors that go away with the right import incantation"; one keyword "conveys mechanism instead of intent"; failed searches gave "very unspecific error messages"; fixing this in the old syntax was "impossible", hence Scala 3's `given`/`using`. His current capture-checking work ("highly experimental") targets resource safety, checked exceptions, effect polymorphism and the sync/async "what colour is your function" problem through capabilities.
- **Sources.** Scala 3 reference, "Contextual Abstractions: Critique of the Status Quo"; "Capture Checking" reference page.
- **Detail.** The five enumerated problems: overuse, worst for conversions; reliance on implicit imports that "can hide anywhere in a long list of imports"; the `implicit` modifier covering many constructs so "the intent is rather indirect and can be easily misread"; implicit-argument application that "looks like a regular application" (so `currentMap("abc")` cannot be written and `.apply` is needed), with only one implicit section allowed and last; and tooling, since available implicits depend on context and Scaladoc cannot show them. Goals of the redesign: "make implicits easier to learn and harder to abuse", improve "the clarity of the 95% of Scala programs that use implicits", and achieve "a better separation of term inference from the rest of the language".
- **Lesson.** Features should express intent, not mechanism; term inference must be scoped and diagnosable; effects and colouring are the next frontier.

### Simon Peyton Jones and the Haskell committee: laziness and success
- **Claim.** "Laziness was undoubtedly the single theme that united the various groups"; it "kept us pure, and thereby motivated a great deal of productive work" on monads and I/O. Admitted costs: space leaks that are astonishingly hard to find; records omitted and never fixed well; some syntax "probably a mistake". On adoption: Haskell's virtue was that it "has not become too successful"; "runaway success, such as that of Java" brings compatibility pressure ("avoid success at all costs").
- **Sources.** Hudak, Hughes, Peyton Jones, Wadler, "A History of Haskell: Being Lazy With Class", HOPL III, 2007; "Wearing the Hair Shirt", POPL 2003 `[not re-fetched]`.
- **Lesson.** Laziness by default buys purity at the cost of predictable performance (42 % of Haskell users cannot reason about performance, `surveys.md`); a language can take purity without laziness.

### José Valim, Elixir: adding types to a dynamic language
- **Claim.** Elixir is adding a gradual, set-theoretic type system incrementally, inferring from patterns "without requiring changes to existing software" and surfacing only as warnings; the design (with Castagna and Duboc) extends semantic subtyping to arity, guards and maps and uses the VM's own type tests instead of inserted casts, giving warnings "more reliable, precise, and with better error messages" than syntactic checks.
- **Sources.** "Elixir v1.17 released", 12 Jun 2024; Castagna, Duboc, Valim, "The Design Principles of the Elixir Type System", 2023/2024.
- **Detail.** The first milestone makes atoms divisible types (`:foo` and `:hello_world` are distinct), models maps and structs as open or closed so the compiler sees which keys exist, and warns on missing keys, calls on non-modules, struct comparison with `<`, comparisons between non-overlapping types and rescuing undefined exception types; tuples, lists, functions, binaries and numbers are still indivisible, and "the Elixir typechecker only infers types from patterns within the same function at the moment". Each release since has widened the inference one construct at a time, the gradualism being the design.
- **Lesson.** Even the most successful dynamic languages of the last decade converge on types; adding them later is a multi-year research programme.

## Memory, performance and runtime

### Guido van Rossum, Python: the GIL and the 2-to-3 transition
- **Claim.** 2007: Greg Stein's 1999 free-threading patch "slowed down single-threaded execution nearly two-fold"; he would accept GIL removal "only if" single-threaded performance "does not decrease". The eventual answer, PEP 703 (accepted 24 Oct 2023), measures a 5-8 % single-thread penalty and was accepted with a rollback proviso; PEP 779 (16 Jun 2025) sets a 15 % ceiling and makes free-threading supported-but-optional in 3.14. On Python 3 his public position is that the transition was far harder and longer than expected `[paraphrase; interviews not re-fetched]`; the shipped evidence is PEP 373 extending 2.7 support to 2020.
- **Sources.** "It isn't Easy to Remove the GIL", artima.com, 10 Sep 2007; PEP 703; PEP 779; PEP 373.
- **Lesson.** Runtime-model decisions are the hardest to change: 25 years for the GIL, 12 for Python 3. Choose the concurrency model before there is an ecosystem.

### Chris Lattner, Swift and Mojo
- **Claim.** Mojo "takes lessons learned and best practices from languages like C++, Rust, Swift, and Zig": arguments are borrowed by default because Rust's move default is harsh for learners; eager destruction removes Rust's drop flags and most need for non-lexical lifetimes; Rust users "often wrap values in Arc, Mutex or Box" to avoid the borrow checker; Mojo claims "much faster compile times than Rust" via MLIR. Lattner's widely reported Swift regrets (the bidirectional type checker's exponential "expression too complex" behaviour, ARC overhead, language growth) `[paraphrase; podcast transcripts not re-fetched]` are corroborated by Swift's own 2025 concurrency vision document.
- **Sources.** Modular blog, "Mojo vs. Rust: is Mojo faster than Rust?", 12 Feb 2024; Lex Fridman interviews 2020 and 2023 `[not re-fetched]`.
- **Detail.** Mojo's `^` transfer operator provides the Rust-style move only on request ("To fully opt into the Rust default of moving an object and losing ownership"); Mojo "removes Rust's drop flags categorically" through eager destruction; self-referential structs need no `Pin` ("you can still update any self-referential fields"); and Rust's move-by-memcpy is cited as a cost that "can be optimized away by LLVM in many cases" but not always.
- **Lesson.** Ownership can be made gentler (borrow by default, eager destruction) without losing its guarantees; type-checker algorithmic complexity is a design decision.

### Swift Language Steering Group: Swift 6 strict concurrency
- **Claim.** Swift 6's data-race safety produced false positives in single-threaded code (globals, protocol conformances, deinitialisers, main-actor SDK calls), forced reflexive `@MainActor` annotation, and confronted beginners "with the concept of data-race safety and actor isolation too early"; nonisolated async functions had confusing semantics; migration was hard. Fix: single-threaded (main-actor) by default per module, isolated conformances, caller-isolation for async functions.
- **Source.** "Improving the approachability of data-race safety", Swift Evolution vision document, 2025.
- **Detail.** Isolated types could not conform to nonisolated protocols without unsafe `assumeIsolated` workarounds; runtime isolation assertions (SE-0423) could break clients who migrated before their dependencies; `nonisolated` meant different things on sync and async functions. The document gates every source-breaking change behind upcoming-feature flags with compiler migration tooling, keeps libraries nonisolated by default while executables default to the main actor, and explicitly leaves task ordering and actor re-entrancy out of scope.
- **Lesson.** Safety checks must be progressive: a program that uses no concurrency should need no concurrency annotations.

### Andrew Kelley, Zig: removing async/await and the Io interface
- **Claim.** Zig 0.10 (2022): "async/await is not done yet" in the new compiler; 0.11 (2023) dropped the old compiler, so async disappeared. 2025 (0.15): "it is settled that there will not be async/await keywords in the language"; their replacement lives "in the Standard Library as part of the Io Interface", passed by the caller like `Allocator`, so packages are "agnostic to the application's concurrency model". Loris Cro: the old design avoided colouring in source but not at run time because stackless coroutines "have a viral calling convention". The accompanying Reader/Writer redesign is "extremely breaking".
- **Sources.** Zig 0.10.0 and 0.15.1 release notes; L. Cro, "Zig's New Async I/O", 13 Jul 2025; Zig devlog, 15 Oct 2025.
- **Detail.** The 0.10 self-hosted compiler was shipped before async was reimplemented ("Although we fully intend to make the self-hosted compiler a strict improvement, that is not the case yet"), with users told to wait or use `-fstage1`; it was 7 % faster and used 3.5x less memory than the C++ compiler. The 0.15 I/O rewrite deprecates every existing reader and writer because "The old interface was generic, poisoning structs that contain them"; the buffer moves into the interface, and the notes call the changes "extremely breaking". Kelley's October 2025 devlog shows DNS resolution that "behaves optimally" under threads or an event loop and still "works when using single-threaded, blocking Io".
- **Lesson.** Async as a keyword couples the compiler to one execution model; injecting the I/O runtime as a value is the latest attempt to avoid colouring, at the cost of threading a parameter.

### Graydon Hoare, Rust: "The Rust I Wanted Had No Future"
- **Claim.** The Rust he wanted differed from the Rust that shipped on many axes: a slower, more dynamic language with green threads and no first-class async, simpler non-monomorphised generics, checked integer arithmetic by default, less emphasis on zero-cost abstraction; the community that gathered wanted a C++ replacement and pushed every decision toward performance. He concludes his version would not have attracted that community and so "had no future". `[paraphrase; page returned a captcha on re-fetch]`
- **Source.** graydon2.dreamwidth.org/307291.html, 2023.
- **Lesson.** A language's design is decided by the audience it attracts as much as by its author; pick the audience deliberately.

### Joe Armstrong, Erlang
- **Claim.** HOPL III: studying concurrent logic languages (Parlog, Strand) for implementation ideas "actually turned out to be a mistake" because their concurrency is implicit and fine-grained while Erlang's is explicit and coarse; the industry's bet that "data must be shared for efficiency" was wrong and process isolation was right ("better a slow system that works"). Thesis (2003): reliability comes from isolation, "let it crash" and supervision rather than defensive code. "The Mess We're In" (2014) blames complexity and irreproducibility, not language features `[paraphrase; video]`.
- **Sources.** "A History of Erlang", HOPL III, 2007; PhD thesis, KTH, 2003.
- **Lesson.** Isolation plus supervision is a proven reliability model no mainstream language copied wholesale; its cost, no shared memory, was accepted knowingly.

### Dennis Ritchie, C
- **Claim.** HOPL II acknowledges C's declaration syntax, the precedence of bitwise operators relative to comparisons, and the array/pointer conflation as infelicities, the preprocessor and weak checking as products of history, and ends: "C is quirky, flawed, and an enormous success" `[paraphrase; all mirrors returned 503 during compilation]`. Bright's "C's Biggest Mistake" (2009) picks array decay as the root of buffer overflows.
- **Source.** "The Development of the C Language", HOPL II, 1993.
- **Lesson.** Historic conveniences became the industry's dominant vulnerability class; do not inherit C's syntax or array model out of familiarity.

## Objects, syntax and culture

### Brian Kernighan: "Why Pascal Is Not My Favorite Programming Language"
- **Claim.** Writing the Software Tools in Pascal convinced him the language was unusable for real programs: array sizes are part of the type (so no general string or array routines), there is no separate compilation or escape from the type system, no static initialisation, and I/O is inflexible; a language must provide for the programs people actually need to write, not only for teaching `[paraphrase; 1981 Bell Labs CSTR 100, not re-fetched]`. His later work (AWK, the Go and C books) consistently favours small languages with pragmatic escape hatches.
- **Lesson.** Purity that forbids common engineering needs (generic sizes, separate compilation, escape hatches) loses to a messier language that permits them; Sisu's safety story must include a sanctioned way out.

### Alan Kay, Smalltalk: what OOP meant
- **Claim.** "OOP to me means only messaging, local retention and protection and hiding of state-process, and extreme late-binding of all things" (email, 23 Jul 2003). He disliked Simula's inheritance and says later Smalltalks "backslid towards Simula". OOPSLA 1997: "I invented the term object-oriented, and I can tell you I did not have C++ in mind" `[widely quoted; video not re-fetched]`.
- **Lesson.** Class hierarchies and static dispatch are not what OOP's inventor meant; interfaces without inheritance (as in Go) are faithful to the idea.

### Larry Wall, Perl 6/Raku: Apocalypse 1
- **Claim.** Perl 5's "weird" relationship between `@foo` and `$foo[]` was a "botch"; 24 precedence levels could be 18; context sensitivity blocked improvements; objects were heavyweight; "It would be rather bad to suddenly give working code a brand new set of semantics." Perl 6 took 15 years and split the community.
- **Source.** "Apocalypse 1: The Ugly, the Bad, and the Good", 2001.
- **Lesson.** Sigils, context and operator tables are permanent; a ground-up redesign without a migration path can cost a language its momentum.

### Evan Czaplicki, Elm
- **Claim.** Elm's error messages ("Compiler Errors for Humans", 2015) were designed to show context, suggest fixes and avoid jargon, and became the model for Rust. On governance: "If you like what you see now, that's pretty much what Elm is going to be for a while"; the 0.19 delays caused burnout on "incremental improvements that are good ideas, but take a very long time and aren't very exciting"; "having more flexibility in planning is a competitive advantage". "What is Success?" (Elm Europe 2018) and "The Hard Parts of Open Source" (Strange Loop 2018) argue a small language should define success as serving users rather than growth and must resist open-source feature pressure `[paraphrase; videos]`.
- **Sources.** elm/compiler roadmap.md; elm-lang.org news (2015).
- **Lesson.** Diagnostics quality is a design value worth copying; a single-maintainer governance model caps adoption, whatever its merits.

### Dave Abrahams, Swift/Hylo: value semantics
- **Claim.** Hylo is "All in on Value Semantics and Generic Programming"; "Expressing double linked lists with value semantics is safe and faster than doing it with reference semantics"; structured concurrency "with no function coloring". His C++Now 2022 keynote argues C++ and Swift got value semantics partly right and that first-class references are the remaining problem `[paraphrase; video]`.
- **Sources.** hylo-lang.org; Racordon et al., JOT 2022; "A Future of Value Semantics and Generic Programming", C++Now 2022.
- **Lesson.** Mutable value semantics is the main live alternative to borrow checking for safe, fast mutation; it is unproven at scale.

### Jonathan Blow, Jai: the counter-position on safety
- **Claim.** Compile-time safety features tax iteration speed and attention for bugs that are, in game development, cheap to find and fix; a language for expert programmers should optimise for fast compilation, compile-time execution and control, and let the programmer decide ("Preventing the Collapse of Civilization", 2019; Jai design talks 2014-2024) `[paraphrase; videos; Jai is closed beta]`.
- **Lesson.** The strongest articulated dissent: the industry data in `memory-safety.md` answer it for security-exposed code, but his point about iteration speed is confirmed by every Rust survey.

## Fixes that shipped: regrets matched to the change that addressed them

The README ranks "fixes that shipped" as the second-strongest evidence class. This table pairs each regret above with
the concrete change, which both confirms the regret was real and shows how long the correction took.

| Regret | Who | Fix that shipped | Years from problem to fix |
|---|---|---|---|
| Null references | Hoare (1965) | Swift optionals (2014); Kotlin nullable types (1.0, 2016); C# 8 nullable reference types (2019); Dart 2.12 sound null safety (2021), enforced in Dart 3 (2023); TypeScript `strictNullChecks` (2.0, 2016) | ~50 |
| Checked exceptions | Hejlsberg (2003) | C# (2000) and Kotlin (2016) shipped without them; Java never removed them | n/a |
| No generics | Pike | Go 1.18 type parameters (2022) | 13 |
| No versioning | Cox | Go modules, Go 1.11 (2018), default in 1.13 (2019) | 9 |
| Error-handling verbosity | Go team | none; proposals closed (2025) | 13 and counting |
| Global interpreter lock | van Rossum (2007) | PEP 703 accepted (2023), experimental in 3.13 (2024); PEP 779 supported-optional in 3.14 (2025) | 16-18 |
| Python 2 to 3 migration | van Rossum | PEP 373 extended 2.7 support to 2020 | 12 |
| Optional, unsound types | Bracha | Dart 2 sound types (2018) | 7 |
| Unsound/erasable types' checker speed | Hejlsberg | TypeScript compiler ported to Go, ~10x (2025) | 13 |
| Implicits conveying mechanism | Odersky | Scala 3 `given`/`using` (2021) | ~10 |
| Strict concurrency too early | Swift team | SE-0461 caller isolation, per-module main-actor default (Swift 6.2, 2025) | 1 |
| Language-level async | Kelley | removed in 0.11 (2023); `std.Io` (0.15, 2025) | 4 |
| Safety opt-in | Bright | DIP 1028 "@safe by default" accepted then withdrawn (2020) `[not re-verified]`; `@safe` remains opt-in | unresolved |
| Exceptions by default | Bright | `nothrow`/`@nogc` attributes exist; defaults unchanged | unresolved |
| Records | Haskell committee | `RecordDotSyntax` (GHC 9.2, 2021) `[not re-verified]`; no full solution | ~30 |
| Untyped Elixir | Valim | set-theoretic types, Elixir 1.17 (2024) onwards | 13 |
| Perl 5 botches | Wall | Perl 6/Raku (2015), 15 years after Apocalypse 1 | 15 |
| Committee accretion | Stroustrup | none; profiles proposed (2024) | unresolved |

The median time from a designer's public regret to a shipped fix in this table is about a decade, and three of the
most consequential (Go error handling, D's safety default, C++ complexity) remain unfixed. That is the strongest
argument in this file for deciding these questions before Sisu has users.

## Synthesis for Sisu

1. **Nullability, error propagation and the concurrency/runtime model must be decided before 1.0; no language here
   retrofitted them cleanly** (high confidence): Hoare, Hejlsberg, Griesemer, van Rossum, Kelley and the Swift team
   all testify to it.
2. **A single design authority with a complexity budget beats committee accretion** (high): Stroustrup's Vasa and HOPL
   papers from the inside; Wirth, Pike, Czaplicki and Hickey from the outside; the alternative costs a decade per
   feature (concepts, Go generics).
3. **Ship spec, second implementation, formatter, package manager, test runner and doc tool together** (high): Pike's
   and Bright's "got right" lists are almost identical and are tooling, not type theory.
4. **Soundness wins over optional typing** (high): Dart reversed Bracha; TypeScript's unsoundness was a retrofit cost;
   Elixir and Python are adding types.
5. **Safety and `nothrow` should be defaults; escape hatches opt-in** (high): Bright's clearest regrets, echoed by
   Google's data and Rust's design.
6. **Compatibility is a feature; breaking changes need a new name** (high): Cox, Hickey, Wall and the Python 3
   experience agree from four directions.
7. **Progressive disclosure of safety machinery** (medium-high): Swift 6's retreat and Mojo's borrow-by-default show
   ownership-style systems can and must stay invisible for single-threaded, non-aliasing code.
8. **Syntax budget decisions are permanent** (medium): Breslav's `?:`, Wall's sigils, Ritchie's declarations; allocate
   symbols with the whole language in view.
9. **Choose the audience, because it will choose the trade-offs** (medium): Graydon's conclusion; Sisu's "best of
   both" goal needs an explicit primary audience or it will be pulled apart.

## Sources

1. T. Hoare, "Null References: The Billion Dollar Mistake", QCon London 2009 (infoq.com); A. Hejlsberg, "The Trouble
   with Checked Exceptions", artima.com, 2003; TypeScript Design Goals (GitHub wiki); "A 10x Faster TypeScript", 11
   Mar 2025; R. Griesemer, go.dev/blog/error-syntax, 3 Jun 2025.
2. B. Stroustrup, P0977R0 (2018) and HOPL IV (2020); H. Sutter, P3081R0 (2024); N. Wirth, IEEE Computer, Feb 1995; R.
   Pike, commandcenter.blogspot.com, Jan 2024 and Sep 2017; R. Cox, research.swtch.com/vgo-intro (2018) and
   go.dev/blog/compat (2023); R. Hickey, "Spec-ulation" transcript (github.com/matthiasn/talk-transcripts).
3. W. Bright, "D at 20: Hits and Misses" (2019) and "C's Biggest Mistake" (2009), digitalmars.com; A. Breslav, The
   Pragmatic Engineer podcast, 12 Feb 2026, and talks list at abreslav.com.
4. G. Bracha, "Pluggable Type Systems" (2004); dart.dev type-system and null-safety pages; M. Odersky, Scala 3
   reference (contextual motivation; capture checking); Hudak, Hughes, Peyton Jones, Wadler, HOPL III (2007); J.
   Valim, "Elixir v1.17.0 released" (2024); Castagna, Duboc, Valim, arXiv 2306.06391.
5. G. van Rossum, artima.com, 2007; PEP 373, 703, 779; Modular, "Mojo vs. Rust", 12 Feb 2024; Swift Evolution vision
   "Improving the approachability of data-race safety" (2025); Zig 0.10.0 and 0.15.1 release notes; L. Cro,
   kristoff.it, 13 Jul 2025; G. Hoare, graydon2.dreamwidth.org/307291.html (2023).
6. J. Armstrong, HOPL III (2007) and KTH thesis (2003); D. Ritchie, HOPL II (1993); A. Kay, email of 23 Jul 2003
   (archived by Stefan Ram) and OOPSLA 1997 keynote; L. Wall, "Apocalypse 1", perl.com, 2 Apr 2001; E. Czaplicki,
   elm/compiler roadmap.md and elm-lang.org news; D. Abrahams, C++Now 2022, hylo-lang.org; J. Blow, DevGAMM 2019.
