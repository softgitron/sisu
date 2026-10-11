# 15 - Decision log

Every design decision, the alternatives that were considered, the evidence
that decided it, and its status. Evidence links point into
[`../research/`](../research/). Status is **decided** (the proposal commits
to it), **leaning** (the proposal commits to it but names an open question
that could reverse it), or **open** (no commitment; see
[`../open-questions.md`](../open-questions.md)).

Format: *Decision* / *Alternatives* / *Evidence* / *Consequences* / *Status*.

---

### D-001 Priority order: safety > readability > predictable performance > fast feedback > expressiveness > interop

*Alternatives:* performance first (C++, Zig, Jai); expressiveness first (Scala, Haskell); familiarity first (Kotlin, TypeScript).
*Evidence:* The memory-safety data ([evidence/memory-safety.md](../research/evidence/memory-safety.md)) makes safety non-negotiable for a new language in 2026. Readability over expressiveness follows from the C++/Scala/D feature-accretion retrospectives ([cpp.md](../research/languages/cpp.md), [scala.md](../research/languages/scala.md), [d.md](../research/languages/d.md)) and Python's dominance in teaching. Performance third rather than first because Go's success shows "fast enough and predictable" beats "fastest" for most software, while "predictable" is kept above expressiveness because hidden costs are the top complaint about Swift ARC and Haskell laziness.
*Consequences:* Every other decision is resolved by this order when its evidence is mixed.
*Status:* decided.

### D-002 Static, sound typing with no `any`

*Alternatives:* dynamic (Python, Clojure, Elixir); optional/gradual (Dart 1, TypeScript, Python typing); unsound static (TypeScript, Java arrays).
*Evidence:* Dart's reversal to sound types in 2.0, and its measured codegen benefit of soundness (a getter from 47 instructions to 3) ([dart.md](../research/languages/dart.md)); Gao, Bird & Barr 2017 (types catch ~15% of shipped JS bugs) and the Elixir/Python/Ruby retrofit experiences ([studies.md](../research/evidence/studies.md), [python.md](../research/languages/python.md), [erlang-elixir.md](../research/languages/erlang-elixir.md)); TypeScript's `any` as the leading source of runtime type errors in typed codebases ([typescript.md](../research/languages/typescript.md)).
*Consequences:* No REPL-style redefinition of types at runtime; exploratory work is served by tooling (D-029).
*Status:* decided.

### D-003 Local bidirectional inference; explicit signatures on all items

*Alternatives:* global Hindley-Milner (OCaml, Haskell, Crystal); no inference (Java pre-10, C).
*Evidence:* Error locality and separate compilation ([ocaml.md](../research/languages/ocaml.md), [haskell.md](../research/languages/haskell.md), [crystal.md](../research/languages/crystal.md)); Swift's solver timeouts from over-eager bidirectional inference with literal polymorphism ([swift.md](../research/languages/swift.md)); Go/Rust/Kotlin/Carbon convergence.
*Consequences:* Enables D-038 (fast incremental compilation) and the API-diff tool in D-031.
*Status:* decided.

### D-004 Ownership with value semantics and second-class references; no named lifetimes

*Alternatives:* tracing GC (Go, Java, OCaml); reference counting everywhere (Swift, Nim); Rust's first-class references with lifetimes; Hylo's pure mutable value semantics; Vale's generational references; Austral's linear types.
*Evidence:* Memory-safety data rules out unsafe defaults. Rust's surveys show the learning curve concentrated in non-adopters and ex-users, with lifetimes and borrowing the named obstacle ([evidence/surveys.md](../research/evidence/surveys.md)); Graydon Hoare's retrospective and Hylo's papers identify first-class references as the source of annotation burden ([rust.md](../research/languages/rust.md), [hylo.md](../research/languages/hylo.md)); Swift's `~Escapable` and Mojo's inferred origins are moves in the same direction ([swift.md](../research/languages/swift.md), [mojo.md](../research/languages/mojo.md)). GC rejected as the *default* because of priority 3 and the embedded/WebAssembly targets; its absence is the largest open risk.
*Consequences:* No `Pin`, no `'a`; view types; `Shared` for graphs; arena handles idiomatic. See [04](04-memory-and-ownership.md).
*Status:* decided (Q-1 closed: no GC tier; the prototype corpus still validates expressiveness per [16](16-validation-plan.md)).

### D-005 Shared ownership is `Shared[T]`: atomic RC with Perceus-style reuse

*Alternatives:* `Rc`/`Arc` split (Rust); tracing GC for the shared tier (Nim's ORC cycle collector; a GC'd region as in Verona); no shared ownership (pure MVS).
*Evidence:* Koka/Lean/Roc show RC with reuse analysis competitive with tracing for functional workloads ([koka-effekt.md](../research/languages/koka-effekt.md), [roc.md](../research/languages/roc.md)); Swift shows the cost of RC on pointer-heavy code; Rust's `Rc`/`Arc` split is a frequent source of "wrong one" refactors.
*Consequences:* Cycles need `Weak`. The compiler chooses atomic or non-atomic counting by escape analysis; the developer never chooses (Q-2 closed).
*Status:* decided.

### D-006 Parameter conventions `x` (borrow), `mut x` (in-place), `own x` (consume); `mut` visible at call sites

*Alternatives:* owned by default (Rust); convention in the type (`&T`, `&mut T`); Hylo's `let`/`inout`/`sink`/`set`; Swift's compiler-chosen with `borrowing`/`consuming` overrides.
*Evidence:* Swift and Hylo (borrow default is what most functions want; [swift.md](../research/languages/swift.md), [hylo.md](../research/languages/hylo.md)); Rust's "should this take `&T` or `T`" API-design burden ([rust.md](../research/languages/rust.md)); Swift's and Hylo's `&x` call-site marker for in-place mutation praised for visibility.
*Consequences:* `ref T` appears only in return types, locals and views.
*Status:* decided (Q-18: `own`, `mut`, call-site `mut`).

### D-007 Explicit `.copy()`; `Copy` is opt-in for trivially copyable types

*Alternatives:* implicit copy with copy-on-write (Swift); implicit copy for all `Copyable` (Hylo); `.clone()` only (Rust).
*Evidence:* Priority 3 (visible cost); Swift's COW makes allocation invisible and is the cause of its "unexpected copy" performance issues ([swift.md](../research/languages/swift.md)); Rust's model is well-understood.
*Status:* decided (Q-3).

### D-008 Deterministic destruction at scope end, reverse declaration order

*Alternatives:* ASAP destruction at last use (Mojo); finalisers (GC languages); `defer` only (Go, Zig).
*Evidence:* RAII is the most-copied C++ feature ([cpp.md](../research/languages/cpp.md)); scope-end is predictable for guards; Go's `defer` function-scoping is a documented trap ([go.md](../research/languages/go.md)).
*Consequences:* `Drop` trait; block-scoped `defer` kept as convenience (D-048).
*Status:* decided.

### D-009 Integer overflow traps in all build modes

*Alternatives:* wrap in release (Rust); UB in release (Zig `ReleaseFast`, C); wrap always (Go, Java); arbitrary precision (Python).
*Evidence:* Integer-overflow CVE classes ([memory-safety.md](../research/evidence/memory-safety.md)); Swift traps always with low measured cost ([swift.md](../research/languages/swift.md)); Rust's wrap-in-release is a recurring RFC topic.
*Status:* decided.

### D-010 No null; `Option[T]` with `T?` sugar, `?.`, `??`, `if let`, `let else`

*Alternatives:* nullable references with flow typing (Kotlin, TypeScript, C# 8); `Optional` as a library type only (Java, Rust).
*Evidence:* Hoare's retrospective; Java `Optional`'s failure to displace `null` ([java.md](../research/languages/java.md)); Kotlin/Swift ergonomics ([kotlin.md](../research/languages/kotlin.md), [swift.md](../research/languages/swift.md)); Rust's `Option` showing an ADT suffices when the sugar is good.
*Status:* decided.

### D-011 No exceptions; errors are values in error sets with `?` propagation; panics for bugs

*Alternatives:* unchecked exceptions (C#, Python, C++); checked exceptions (Java); untyped `throws` (Swift <6); `(T, error)` tuples (Go); condition system (Common Lisp).
*Evidence:* [05-errors.md](05-errors.md) and [topics/error-handling.md](../research/topics/error-handling.md); Go's 2025 decision to stop pursuing error syntax ([go.md](../research/languages/go.md)); Kotlin adding a typed error-union channel fifteen years after choosing unchecked exceptions (KEEP-0462, [kotlin.md](../research/languages/kotlin.md)); Zig's error sets ([zig.md](../research/languages/zig.md)); Swift's typed throws SE-0413 ([swift.md](../research/languages/swift.md)); exception-handling bug studies ([studies.md](../research/evidence/studies.md)).
*Status:* decided; syntax details Q-7; condition system Q-36.

### D-012 No class inheritance; traits with default methods, composition, `@delegate`

*Alternatives:* single inheritance (Java, C#, Swift classes); multiple (C++); mixins (Ruby, Scala traits with state).
*Evidence:* "Composition over inheritance" (GoF 1994 onward); Kotlin's final-by-default and Go's and Rust's success without it; Swift's protocol-oriented programming ([swift.md](../research/languages/swift.md)); GUI frameworks (the strongest case for inheritance) being written without it in Rust/Go/Elm.
*Status:* decided; whether delegation suffices for OOP-heavy domains is Q-28.

### D-013 Explicit `impl Trait for Type`; global coherence, with orphan implementations allowed in the root package

*Alternatives:* implicit structural satisfaction (Go, TypeScript); scoped instances (Scala `given`; OCaml's modular implicits, proposed 2014, never shipped, with a reduced "modular explicits" arriving in OCaml 5.5 in 2026); no coherence (Haskell orphans allowed with warnings).
*Evidence:* Go's accidental satisfaction and inability to attach defaults ([go.md](../research/languages/go.md)); Rust's coherence making collections behave uniformly; Scala implicits' documented confusion and Scala 3 redesign ([scala.md](../research/languages/scala.md)); the Rust orphan rule as a problem for 15.8% of survey respondents ([rust.md](../research/languages/rust.md)).
*Consequences:* Libraries keep the orphan rule. The root package of a build may implement any trait for any type: there is exactly one root and nothing depends on it, so coherence holds without newtypes in application code. A library that later adds the same impl produces a compile error in the root, fixed by deleting the root's copy.
*Status:* decided (Q-11).

### D-014 Colourless structured concurrency: tasks on a runtime scheduler, `scope`/`spawn`, no `async`/`await`

*Alternatives:* `async`/`await` (C#, JS, Python, Rust, Swift, Kotlin `suspend`); raw threads (Java pre-21, C++); actors in the language (Erlang, Pony, Swift actors); explicit `Io` with `io.async` (Zig 0.16).
*Evidence:* [06](06-concurrency-and-capabilities.md); Nystrom's function-colouring argument; Rust async survey pain ([rust.md](../research/languages/rust.md)); Java Loom and Go ([java.md](../research/languages/java.md), [go.md](../research/languages/go.md)); Go leak studies (Tu et al. 2019); Trio/Kotlin/Swift structured concurrency convergence; Zig's removal and redesign of async ([zig.md](../research/languages/zig.md)).
*Status:* decided; runtime/stack details Q-6; preemption Q-34.

### D-015 Capabilities: no ambient authority; `System` handed to `main`; `[trust]` gates `unsafe`/`extern`

*Alternatives:* ambient I/O (every mainstream language); runtime permission flags (Deno); effect types (Koka, Flix); implicit context (Odin, Scala `using`).
*Evidence:* Supply-chain incident history ([javascript.md](../research/languages/javascript.md), [studies.md](../research/evidence/studies.md)); Austral's rationale ([austral.md](../research/languages/austral.md)); Zig's `Io` and Roc's platforms as proof the pattern is usable ([zig.md](../research/languages/zig.md), [roc.md](../research/languages/roc.md)); testability without mocks.
*Consequences:* I/O functions take a parameter; stdio exception; no build scripts (D-032).
*Status:* decided; stdio and implicit context Q-5; effects Q-17.

### D-016 Numeric conversions: implicit lossless widening only; everything else explicit

*Alternatives:* none implicit (Rust); C's implicit everything; Zig/Swift middle.
*Evidence:* Rust's `as usize` paper-cut ([rust.md](../research/languages/rust.md)); C's conversion bugs ([c.md](../research/languages/c.md)).
*Status:* decided (Q-13).

### D-017 Generics: `[T]` syntax, trait bounds checked at definition, monomorphised with instantiation sharing, explicit `dyn`

*Alternatives:* `<T>` (C++, Java, Rust, Swift); duck-typed templates (C++) or comptime (Zig); erasure (Java); dictionary passing (Haskell, Swift unspecialised); GC-shape stenciling (Go).
*Evidence:* Turbofish and parsing ([rust.md](../research/languages/rust.md)); Go's `[]` choice and rationale ([go.md](../research/languages/go.md)); C++ concept diagnostics and Zig instantiation errors ([cpp.md](../research/languages/cpp.md), [zig.md](../research/languages/zig.md)); Carbon's checked generics ([carbon.md](../research/languages/carbon.md)); Graydon on monomorphisation cost.
*Status:* decided; HKT Q-10; comptime generics Q-27.

### D-018 No macros in v1; `comptime` evaluation plus compile-time reflection; `derive` is a comptime function

*Alternatives:* proc macros (Rust), AST macros (Lisp, Elixir, Nim, Julia), template metaprogramming (C++), CTFE + string mixins (D).
*Evidence:* [08](08-metaprogramming.md); Zig ([zig.md](../research/languages/zig.md)); Rust compile-time complaints; D's string-mixin regret ([d.md](../research/languages/d.md)); Dart's cancelled macros ([dart.md](../research/languages/dart.md)); the Lisp curse ([lisp-scheme-racket.md](../research/languages/lisp-scheme-racket.md)).
*Status:* decided (Q-15: no macros).

### D-019 File = module, directory = module, no headers, no `mod` declarations, no package cycles

*Alternatives:* `mod` declarations (Rust); header/interface files (C, OCaml); cyclic imports allowed (Python, JS).
*Evidence:* Go's directory-is-package and acyclic rule as the enabler of parallel builds ([go.md](../research/languages/go.md)); Rust `mod` confusion; C++ modules' decade-long road ([cpp.md](../research/languages/cpp.md)).
*Status:* decided.

### D-020 No semicolons; newline terminates statements

*Alternatives:* mandatory semicolons (C, Rust, Java); ASI (JavaScript).
*Evidence:* Go's and Swift's rules work without JavaScript's ASI pitfalls because the grammar is designed for it ([go.md](../research/languages/go.md), [javascript.md](../research/languages/javascript.md)).
*Status:* decided.

### D-021 No `++`/`--`, no ternary, no `goto`, no `switch` fallthrough, no `do-while`, no C-style `for`

*Evidence:* Go removed `++` as an expression and the ternary; Swift and Rust have no `++`; fallthrough is a known bug source (Duff aside); `if` as expression covers the ternary. Each removal is one fewer way to write a thing (P1).
*Status:* decided.

### D-022 Fixed operator set overloadable via traits; no user-defined operators or precedence

*Alternatives:* user-defined operators (Haskell, Scala, Swift, F#); no overloading (Go, Java).
*Evidence:* Scala/Haskell symbol soup and the resulting unreadability ([scala.md](../research/languages/scala.md), [haskell.md](../research/languages/haskell.md)); Swift's custom operators as a type-checker cost ([swift.md](../research/languages/swift.md)); Go's lack of overloading making numeric libraries unpleasant.
*Status:* decided.

### D-023 No properties; fields and methods only

*Alternatives:* properties (C#, Kotlin, Swift, Python).
*Evidence:* Priority 3: `x.y` that runs code is hidden control flow; the research on C# and Swift notes computed properties with side effects as a bug class. Rust and Go do without.
*Status:* decided.

### D-024 Rebinding in the same scope allowed (`let x = x?`); shadowing an outer scope warns

*Alternatives:* free shadowing (Rust); no shadowing (Go warns in vet; Zig errors).
*Evidence:* Rust's shadowing is widely liked for `let x = parse(x)?` chains; Zig's ban is cited as friction; cross-scope shadowing bugs appear in the Go vet rationale.
*Status:* decided (Q-35).

### D-025 `and`, `or`, `not` as words; `!` reserved for error types

*Alternatives:* `&& || !` (C family).
*Evidence:* Stefik & Siebert 2013 (word operators measurably help novices; [studies.md](../research/evidence/studies.md)); Python, Carbon, Ada; frees `!` for `T ! E`.
*Status:* decided.

### D-026 Closures are `{ params -> body }`, inline and trailing; `own { }` to capture by move; no implicit `it`

*Alternatives:* `|x| body` (Rust, Ruby inline); `{ x in }` (Swift); `x => body` (JS, C#, Scala); `lambda` (Python).
*Evidence:* Trailing-closure ergonomics (Kotlin, Swift, Ruby; [kotlin.md](../research/languages/kotlin.md), [ruby.md](../research/languages/ruby.md)); single form for inline and trailing; `->` consistency with `match` and return types.
*Status:* decided; implicit `it` Q-25.

### D-027 Braces, not significant indentation

*Alternatives:* indentation (Python, Haskell, F#, Nim).
*Evidence:* Mixed. Indentation is loved in Python and measurably readable; braces are robust to copy-paste, code generation, and multi-line lambdas, and are the majority in compiled languages. Tooling robustness (formatter, generation, trailing closures) decided it.
*Status:* leaning; Q-8.

### D-028 Contracts (`requires`, `ensures`, `invariant`, refinement types) in the language

*Alternatives:* assertions only (most languages); library contracts (C++ pre-26, Java JML); full dependent types (Idris, Agda).
*Evidence:* [07](07-contracts.md); Ada/SPARK ([ada-spark.md](../research/languages/ada-spark.md)); D; C++26 contracts history ([cpp.md](../research/languages/cpp.md)); refinement types ([idris-agda-lean.md](../research/languages/idris-agda-lean.md)).
*Status:* decided; verifier and ghost code Q-16.

### D-029 One binary for build, test, fmt, doc, package, LSP, REPL

*Evidence:* [10](10-tooling.md); every survey's tooling questions ([surveys.md](../research/evidence/surveys.md)); Go, Cargo, Zig, mix, `uv`.
*Status:* decided.

### D-030 Formatter with no options

*Evidence:* gofmt, rustfmt, Black, Prettier, Elixir formatter: adoption and the end of style review ([go.md](../research/languages/go.md), [python.md](../research/languages/python.md)).
*Status:* decided.

### D-031 Package manager: semver + lockfile, owner-namespaced content-addressed registry with a transparency log, API-diff-enforced semver, provenance

*Alternatives:* MVS without a lockfile (Go); flat namespace (crates.io, PyPI); mutable packages (npm historically).
*Evidence:* [09](09-modules-packages-and-build.md); Elm's enforced semver ([elm-purescript.md](../research/languages/elm-purescript.md)); Go sumdb ([go.md](../research/languages/go.md)); left-pad and the npm incident history ([javascript.md](../research/languages/javascript.md)); Unison's content addressing ([unison.md](../research/languages/unison.md)).
*Status:* decided (Q-19: central registry plus private registries; Q-20: lockfile with newest-satisfying resolution).

### D-032 No build scripts; `[trust]` lists packages allowed `unsafe`/`extern`

*Evidence:* `build.rs` and `postinstall` as the primary supply-chain vectors ([rust.md](../research/languages/rust.md), [javascript.md](../research/languages/javascript.md)); comptime sandbox covers code generation ([zig.md](../research/languages/zig.md)).
*Status:* decided.

### D-033 Small compatibility-promised `std` plus independently versioned blessed `std.x`

*Evidence:* [11](11-stdlib.md); Go's stdlib praise and `golang.org/x`; Python's dead batteries (PEP 594); Rust's async-runtime fragmentation ([python.md](../research/languages/python.md), [rust.md](../research/languages/rust.md)).
*Status:* decided.

### D-034 Strings are UTF-8; `str` view and `String` owned; no integer indexing; scalar-value `char`; graphemes via iterator

*Alternatives:* grapheme-cluster strings (Swift); UTF-16 (Java, JS, C#); byte strings with no validity (C, Go partially); `[Char]` (Haskell).
*Evidence:* [swift.md](../research/languages/swift.md) (correct but slow and awkward); Rust/Go/Zig convergence on UTF-8; Haskell's `String` regret ([haskell.md](../research/languages/haskell.md)).
*Status:* decided; grapheme default Q-14.

### D-035 `int` is 64-bit everywhere and is the index type; sized types for layout

*Alternatives:* `usize` indices (Rust, Zig); platform `int` (Go, C); arbitrary precision default (Python).
*Evidence:* Rust `as usize` friction; Swift `Int` success ([swift.md](../research/languages/swift.md)).
*Status:* decided.

### D-036 Specification before 1.0; compatibility promise; three-yearly editions; `sisu fix` migrations; six-month releases with previews

*Evidence:* [13](13-evolution-and-governance.md); Go promise ([go.md](../research/languages/go.md)); Rust editions ([rust.md](../research/languages/rust.md)); Python 2/3, Swift 1-3, Scala 2/3, Perl 6 ([python.md](../research/languages/python.md), [swift.md](../research/languages/swift.md), [scala.md](../research/languages/scala.md), [perl-php.md](../research/languages/perl-php.md)); Java's cadence ([java.md](../research/languages/java.md)); Dart's migration tool ([dart.md](../research/languages/dart.md)).
*Status:* decided.

### D-037 Governance: small team with a lead until 1.0, then foundation + language team with written boundaries and a 90-day proposal time box

*Evidence:* [designer-retrospectives.md](../research/evidence/designer-retrospectives.md): Elm's stall, Python's PEP 572, Rust's governance crises, C++'s committee, Zig's BDFL; Carbon's written process ([carbon.md](../research/languages/carbon.md)).
*Status:* leaning; Q-21.

### D-038 Compile speed is a design constraint with numeric targets; non-LLVM debug backend; query-based incremental front end

*Evidence:* [10](10-tooling.md); Rust/Swift/Scala/C++ compile-time complaints in every survey ([surveys.md](../research/evidence/surveys.md)); Zig's self-hosted backend ([zig.md](../research/languages/zig.md)); Go's design for build speed; TypeScript's Go port ([typescript.md](../research/languages/typescript.md)).
*Status:* decided.

### D-039 Unstable Sisu ABI; C ABI for stable boundaries

*Evidence:* Swift's library-evolution cost ([swift.md](../research/languages/swift.md)); Rust's choice ([rust.md](../research/languages/rust.md)).
*Status:* decided for now (Q-31).

### D-040 Panics unwind to the enclosing task scope; no `catch`; scope observes `TaskPanicked`

*Alternatives:* `catch_unwind` (Rust), `recover` (Go), abort always (Zig default).
*Evidence:* Erlang supervision ([erlang-elixir.md](../research/languages/erlang-elixir.md)); Go `recover` misuse as exceptions; structured concurrency gives a natural supervisor.
*Status:* decided.

### D-041 Tests, doc tests, property tests, benchmarks, snapshots in the language and standard library

*Evidence:* Zig/D/Rust inline tests; Python doctest; property-testing adoption when zero-dependency ([studies.md](../research/evidence/studies.md)).
*Status:* decided.

### D-042 Postfix `?` for propagation

*Alternatives:* prefix `try` (Zig, Swift); `!`/`?` bang (Gleam `use`, Roc `!` suffix).
*Evidence:* Chaining; Rust's `?` consistently rated among its best features ([rust.md](../research/languages/rust.md)).
*Status:* decided; Q-7 covers residual syntax.

### D-043 Exhaustive `match`; `_` on same-package enums warns; recursive enums auto-boxed

*Evidence:* ML lineage; Rust's "recursive type has infinite size" as a top beginner error.
*Status:* decided.

### D-044 Literals: `[a, b]` list (array when expected), `[k: v]` map, `(a, b)` tuple

*Evidence:* Swift's dictionary literal syntax avoids the `{}` block/closure ambiguity; Go's composite literals and Rust's `vec!` as counter-examples.
*Status:* decided.

### D-045 `pub` / `pub(package)` visibility; private by default

*Alternatives:* capitalisation (Go); `export` lists (JS, Haskell); public by default (Python convention).
*Evidence:* Go's coupling of names to visibility ([go.md](../research/languages/go.md)); Rust's `pub(crate)` usefulness.
*Status:* decided; Q-29 on package-wide default.

### D-046 Attributes are `@name(args)`

*Evidence:* Kotlin/Swift/Python/Java majority; Rust `#[]` noise.
*Status:* decided.

### D-047 Block-scoped `defer`, kept alongside `Drop`

*Evidence:* Zig's `defer`/`errdefer` ergonomics; Rust's `scopeguard` crate as the demand signal; Go's function scoping as the trap to avoid.
*Status:* decided.

### D-048 Non-ASCII identifiers allowed, with confusable and mixed-script lints on by default

*Alternatives:* ASCII only (the first draft's lean).
*Evidence:* Python, Go, Rust, Swift and Julia allow them; Rust's experience is that the risk (CVE-2021-42574 "Trojan Source" and homoglyph identifiers) is handled by NFC normalisation plus `uncommon_codepoints`, `confusable_idents` and `mixed_script_confusables` lints; the teaching and non-English-ecosystem argument decided it.
*Status:* decided (Q-33 reversed the lean).

### D-049 `Map` iteration randomised; SipHash keyed per process

*Evidence:* Hash-flooding DoS history (Python, Java, PHP, Ruby all patched); Go's deliberate randomisation.
*Status:* decided.

### D-050 No function overloading by type; named and default arguments instead

*Alternatives:* overloading (C++, Java, C#, Swift); none (Go, Rust, Zig).
*Evidence:* Swift's overload resolution as its type-checker cost centre ([swift.md](../research/languages/swift.md)); Rust and Go do without; named arguments cover the readability case (Python, Kotlin).
*Status:* decided.

### D-052 Interpreter mode is a feature and the first implementation

*Alternatives:* AOT only with a fast debug backend (the first draft's lean).
*Evidence:* Python's and Julia's exploratory workflow as an adoption driver ([python.md](../research/languages/python.md), [julia.md](../research/languages/julia.md)); the `comptime` evaluator already requires an interpreter over the typed IR; building the interpreter first gives a runnable 0.1 in the shortest time, which the research on Carbon, Hylo, Roc and Pony identifies as the main survival factor ([16](16-validation-plan.md)).
*Consequences:* `sisu run`, the REPL and scripts use the interpreter; the debug native backend comes second; semantics are defined once over the typed IR and shared by both.
*Status:* decided (Q-30).

### D-053 Hot function replacement in debug builds

*Evidence:* Smalltalk, Lisp and Clojure live development; Dart's hot reload as a measured adoption driver ([dart.md](../research/languages/dart.md), [smalltalk.md](../research/languages/smalltalk.md), [clojure.md](../research/languages/clojure.md)).
*Consequences:* The debug ABI keeps frames reifiable; every top-level definition has an indirection point in debug builds that release builds remove; image-based deployment stays out of scope.
*Status:* decided (Q-22).

### D-054 `@must_consume` types

*Alternatives:* lint only; full linear types (Austral).
*Evidence:* Austral's finding that strict linearity is hostile to loops and early return, and its author's own recommendation of opt-in linearity per resource type ([austral.md](../research/languages/austral.md)); Idris 2's multiplicities as the proven form ([idris-agda-lean.md](../research/languages/idris-agda-lean.md)); Vale's "Higher RAII" ([vale.md](../research/languages/vale.md)).
*Consequences:* A value of a `@must_consume` type that goes out of scope without being passed to an `own` parameter is a compile error; unwinding on panic is exempt.
*Status:* decided (Q-37).

### D-051 No implicit zero values; initialisation is mandatory; `Default` is a trait

*Alternatives:* zero-is-initialisation (Go, Odin, Jai); uninitialised memory (C); `late` (Dart).
*Evidence:* Go's typed-nil and zero-value-connection-handle bugs ([go.md](../research/languages/go.md)); the Odin/Jai argument that types should be designed so the zero value is valid ([odin-jai-c3.md](../research/languages/odin-jai-c3.md)) is accepted as *library guidance* (`Default` impls) but not as a language rule, because a zero `Mutex` is fine and a zero file handle is not; Dart's `late` as an invisible runtime check ([dart.md](../research/languages/dart.md)).
*Status:* decided.
