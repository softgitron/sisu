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

### D-020 No semicolons; one statement per logical line; continuation inside brackets or on an operator-led line

*Alternatives:* mandatory semicolons (C, Rust, Java); ASI (JavaScript); backslash continuation (Python).
*Evidence:* Go's and Swift's rules work without JavaScript's ASI pitfalls because the grammar is designed for it ([go.md](../research/languages/go.md), [javascript.md](../research/languages/javascript.md)); Guido lists the backslash among his regrets and PEP 8 and Black break before binary operators ([indentation-syntax.md](../research/topics/indentation-syntax.md)).
*Status:* decided.

### D-021 No `++`/`--`, no ternary operator, no `goto`, no `switch` fallthrough, no `do-while`, no C-style `for`, no loop labels, no `else` on loops

*Evidence:* Go removed `++` as an expression and the ternary; Swift and Rust have no `++`; fallthrough is a known bug source (Duff aside); `if` as an expression covers the ternary; Python's loop `else` is its own tutorial's "look closely" case ([python-idioms.md](../research/topics/python-idioms.md)). Each removal is one fewer way to write a thing (P1). Same-direction chained comparisons (`a < b <= c`) are kept from Python; mixed-direction chains are an error.
*Status:* decided; revised with Q-8.

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

### D-025 `and`, `or`, `not` as words on `bool` only; `!` reserved for error types; no truthiness

*Alternatives:* `&& || !` (C family); Python's operand-returning `and`/`or` and truthiness of empty containers.
*Evidence:* Stefik & Siebert 2013 (word operators measurably help novices; [studies.md](../research/evidence/studies.md)); Python, Carbon, Ada; frees `!` for `T ! E`. Truthiness was refused by Nim and Codon, is accepted by Mojo only for `Boolable` types with a documented warning, and is refused by Swift, Rust and Go; Python's own `datetime.time` midnight bug ([python-idioms.md](../research/topics/python-idioms.md)).
*Status:* decided.

### D-026 Closures: `params -> expr` inline, implicit `it` for one parameter, `do params:` trailing blocks for bodies with statements; `own` to capture by move

*Alternatives:* `{ params -> body }` braces (Kotlin; the first draft); `|x| body` (Rust, Ruby inline); `{ x in }` (Swift); `lambda x: e` (Python, one expression only); nested `def` only (Python's answer to multi-line closures).
*Evidence:* Guido's rule that no indented block may sit inside an expression, and Python's twenty-year refusal of multi-line lambdas ([indentation-syntax.md](../research/topics/indentation-syntax.md)); Nim's `do` and Scala 3's trailing-lambda colon as the two shipped indentation-friendly trailing-block forms; Kotlin's `it` and Scala's `_` for the one-parameter case; `->` consistency with function types. A trailing block ends a logical line, so the rule holds; `return` inside a closure returns from the closure ([function-results.md](../research/topics/function-results.md)).
*Status:* decided (Q-8, Q-25).

### D-027 Indentation-based blocks: `:` opens a block, INDENT/DEDENT decided in the lexer, spaces only, no brace dialect

*Alternatives:* braces (the first draft's lean; Go, Rust, Kotlin, Swift, Carbon's P0623); layout rule inserting braces with a brace fallback (Haskell, Koka, F# verbose syntax); optional braces on a brace language (Scala 3).
*Evidence:* The project owner's decision (Q-8), and the research on how to do it well ([indentation-syntax.md](../research/topics/indentation-syntax.md)): decide layout in the lexer with no parser feedback (Python, Nim, Koka; Haskell's `parse-error(t)` rule as the counter-example); spaces only (every layout language that allowed tabs later restricted them); mandatory call parentheses, explicit `return` and no truthiness remove the CoffeeScript and Nim command-syntax ambiguities; no second dialect (Scala 3's split). Empirical comparisons of indentation and braces for working programmers do not exist; the tooling objections (paste, generation, multi-line closures) are answered by the reindent-on-paste LSP action, AST-based code generation, and `do` blocks.
*Consequences:* The formatter never changes block membership; the parser is error-tolerant with Python 3.10-style messages; comments are `#`; the tour, examples and every snippet in the proposal are in this form.
*Status:* decided (Q-8 reversed the lean).

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

### D-044 Literals: `[a, b]` list (array when expected), `{k: v}` map, `{a, b}` set, `(a, b)` tuple, comprehensions

*Alternatives:* `[k: v]` maps (Swift; the first draft, chosen to avoid the brace ambiguity that no longer exists without brace blocks).
*Evidence:* Python's literal forms are the ones the largest programmer population knows; comprehensions were judged clearer than `map`/`filter` by Python's designer and copied by Julia, Nim and Mojo ([python-idioms.md](../research/topics/python-idioms.md)). The overlap between comprehensions and iterator adaptors is an accepted violation of P1: comprehensions for the eager shape, adaptors for lazy pipelines and library composition.
*Status:* decided (Q-8).

### D-045 `pub` / `pub(package)` visibility; private by default

*Alternatives:* capitalisation (Go); `export` lists (JS, Haskell); public by default (Python convention).
*Evidence:* Go's coupling of names to visibility ([go.md](../research/languages/go.md)); Rust's `pub(crate)` usefulness.
*Status:* decided; Q-29 on package-wide default.

### D-046 Attributes are `@name(args)` and are compile-time data, not calls

*Evidence:* Kotlin/Swift/Python/Java majority; Rust `#[]` noise; Python decorators are run-time calls wearing declaration syntax, which type checkers needed `dataclass_transform` to see through, and Mojo made them compiler built-ins ([python-idioms.md](../research/topics/python-idioms.md)).
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

### D-055 Python idioms adopted, adapted and dropped

*Decision:* Sisu follows Python's surface idioms wherever the idiom is not a documented mistake. Adopted: `def` with `name: Type` and `->`, indentation and `:`, `elif`, `pass`, `and`/`or`/`not`, `in`, same-direction chained comparisons, keyword arguments with `*` and `/` separators, half-open slicing, `range`, `enumerate`/`zip` (strict), interpolation with Python's format specs, `with` as a scoped binding, `yield`, `match`/`case` with guards, or-patterns and struct patterns, `//` and `**`, `def f[T]` and `type` statements, explicit `self`, tuple and star unpacking, comprehensions, `True`/`False`. Adapted: `None` to `T?`; `lambda` to `x -> e`/`it`/`do`; decorators to compile-time attributes; docstrings to `##` comments; `import` binding the last path component; `assert` always on. Dropped: `class` and inheritance, truthiness, `is`, mutable defaults and late-binding closures, declaration by assignment with `global`/`nonlocal`, `**kwargs`, negative indices, `a if c else b`, `try`/`except`/`finally`, `:=`, loop `else`, underscore privacy, dunder protocols, single-quoted strings, tabs.
*Evidence:* Each item's PEP, style-guide position, measured bug class or the choice made by typed Python-syntax languages (Mojo, Nim, Codon, Cython) is recorded in [python-idioms.md](../research/topics/python-idioms.md); the departures table in [02-syntax.md](02-syntax.md) states the reason for each.
*Status:* decided (Q-8).

### D-056 Functions return only with `return`; `if`/`match` are expressions when every branch is one expression; labelled tuple results instead of result variables

*Alternatives:* implicit last-expression value (Rust, Ruby, Scala, Kotlin expression bodies, Gleam, Roc); named result variables (MATLAB output arguments, Go named results with naked `return`, Nim `result`, Pascal `Result`, Mojo `out`, Carbon `returned var`); Java-style `yield` for block-valued arms.
*Evidence:* [function-results.md](../research/topics/function-results.md). The cost of last-expression values is the statement/expression boundary (Rust's "expected `i32`, found `()`" and the clippy lint for each side of the style war); every explicit-return language later added single-expression sugar and `if`/`switch` expressions in the "single expression per branch" form (Swift SE-0255 and SE-0380, Java JEP 361, C# 8), which is the form adopted. Named result variables carry a documented footgun (default-initialised results: Nim RFC 378 and Nim 2's `strictDefs`, Go's zero values and shadowing restriction, Eiffel 16.05) and every Go style guide restricts naked returns; the documentation and contract benefits are kept as labels on tuple results (Swift, Dafny, Whiley) and the contextual `result` name inside `ensures` (SPARK, D, C++26). In-place construction is the compiler's job (Zig result locations), not syntax.
*Consequences:* A non-unit function whose end is reachable without `return` is a compile error; `return` in a closure returns from the closure; `?` targets the innermost function; generators are marked by their `Iter[T]` return type and forbid a valued `return`.
*Status:* decided (Q-9).

### D-057 Testing: `test`/`suite`/`bench` blocks, compiler-intrinsic `assert` with sub-expression capture, fixtures as values, parametrised tests, property tests and fuzzing in `std.test`, snapshots, per-test capability fakes with a virtual clock, parallel and random by default, no mocking framework

*Alternatives:* xUnit classes (JUnit, XCTest, unittest); naming-convention discovery (pytest `def test_*`, Go `TestXxx`); assertion libraries; mocking frameworks; coverage gates.
*Evidence:* [testing.md](../research/topics/testing.md): built-in beats third party (D's and Zig's inline tests, Go's `go test` needing no documentation, pytest displacing `unittest`); assertion capture copied by every modern design and always needing the compiler; fixtures by injection beating inheritance; flakiness causes (Luo et al. 2014: async wait 45%, concurrency 20%, order dependency 12%) each removed by per-test capability fakes and a virtual clock (Go `synctest`); property testing used an order of magnitude more when zero-dependency; Go's unification of fuzzing with tests; Jest's own snapshot misuse guidance; coverage not correlated with effectiveness (Inozemtseva & Holmes 2014); Google's incremental mutation testing; fakes over mocks (Google, Fowler, `gomock` archived).
*Status:* decided. See [17-testing.md](17-testing.md).

---

## Decisions from the unconventional-languages round (October 2026)

The round in [`../research/unconventional/`](../research/unconventional/)
and [`../research/ideation/`](../research/ideation/), assessed in
[19-novel-features.md](19-novel-features.md), produced the decisions below.
All have status **leaning**: the project owner has not reviewed them, and
each that touches an earlier decision is paired with an open question.

### D-058 Views, scopes and suspended task frames are never leaked; completion I/O and DMA borrow the caller's buffer

*Alternatives:* a safe `forget`/`leak` as in Rust (so owned buffers must be handed to the kernel and returned); `'static` buffers for DMA; buffer pools in `unsafe`.
*Evidence:* Rust's own account that "any object can be leaked in Rust, trivially and safely" is why tokio-uring takes buffers by value and `embedded-dma` requires `'static` ([systems-and-determinism.md](../research/ideation/systems-and-determinism.md), candidate 7); the enabling rule has shipped twice, in Rust's closure-based `std::thread::scope` and in Swift's noncopyable types (SE-0390 rejects a general `forget`); the io_uring cancellation contract (`-EALREADY` waits for the original completion).
*Consequences:* No safe `forget` or `leak` for views, scopes or suspended frames, ever; a cancelled I/O call returns only after the device has released the buffer; a stackless fallback for bet 3 must never expose droppable future values; one io_uring cancellation test joins the Phase 1 harness (Q-49). See N-07 in [19](19-novel-features.md).
*Status:* leaning.

### D-059 `@no_alloc` and `@no_block` in v1, `@stack(max=N)` with the native backend, `@no_panic` after 0.1 with a specified interval analysis

*Alternatives:* lints (Rust's `no_std`, clippy); run-time sanitisers only (RealtimeSanitizer); program-wide restrictions (Ada); nothing.
*Evidence:* Clang 20 checks `nonblocking` and `nonallocating` on function types; Swift's experimental `@_noAllocation` met friction with generic metadata; Ada's `pragma Restrictions` and GNATstack; SPARK's Silver level and Wuffs (every check discharged at compile time, in Chrome since 2021). Cheap in Sisu because allocation is visible in types (rule 7 of [04](04-memory-and-ownership.md)) and every blocking point is a capability call or a synchronisation primitive ([systems-and-determinism.md](../research/ideation/systems-and-determinism.md), candidates 5 and 6).
*Consequences:* The attributes are part of function types; checking happens after monomorphisation, an exception to D-017 with errors at the instantiation; drop glue for auto-boxed recursive types must be iterative; `sisu audit --panics` precedes the `@no_panic` attribute; the interval analysis must be written into the specification (D-036). See N-08 and N-09.
*Status:* leaning.

### D-060 Collections take an allocator type parameter with a default; a borrowed region makes the collection a view

*Alternatives:* a global allocator only; allocators as values passed to every call (Zig); arenas as a separate collection family; regions with named lifetimes (Rust's `Vec<T, &'a Bump>`).
*Evidence:* Rust stabilised `Vec<T, A>` and `impl Allocator for &A` for 1.100 in September 2026, ten years after RFC 1398; Cyclone, MLKit and RTSJ scoped memory; Go's arena experiment on hold because explicit arenas infect APIs ([systems-and-determinism.md](../research/ideation/systems-and-determinism.md), candidate 8).
*Consequences:* `list[T, A: Allocator = Global]`; `list[T, ref Region]` cannot escape the region's frame, enter `Shared` or cross `spawn`; resetting a region needs `mut region`; std must separate a bump `Region` from the generational `Arena[T]`; the corpus counts how many functions must become allocator-generic. See N-10.
*Status:* leaning.

### D-061 `todo("name")` is a typed hole: `sisu check` reports it, the interpreter stops at it, release builds and `publish` reject it; a fill engine is later tooling

*Alternatives:* `?name` in operand position, replacing `todo` (Idris, Agda, Lean; proposed in [holes-synthesis-and-ai-era.md](../research/unconventional/holes-synthesis-and-ai-era.md)); an untyped `todo` that panics (Rust `todo!()`, today's prelude); `_` (GHC; taken by patterns).
*Evidence:* Holes are the interaction model of every dependently typed language and have been on in GHC for twelve years; Hazel's measured ChatLSP context (type definitions first) lifts a model furthest in a low-resource language; rust-analyzer's term search was little used, which favours the report over enumeration; fills judged by tests alone overfit (PropR 63-85 %) and models edit tests when they can (ImpossibleBench), so the judge stays out of the filler's reach ([holes-synthesis-and-ai-era.md](../research/unconventional/holes-synthesis-and-ai-era.md), [ai-era-and-evolution.md](../research/ideation/ai-era-and-evolution.md) A1).
*Consequences:* `todo[T](name: str = "") -> T` is a compiler intrinsic like `assert`; a third test outcome, "blocked by a hole" (D-057); `panic("...")` for placeholders meant to ship; no new lexical form; `sisu fill` edits only the hole's span with tests, contracts, generators and snapshots read-only, and `unfillable` is a legal answer. The spelling is Q-46. See N-27.
*Status:* leaning.

### D-062 Loud divergence: where Sisu spells something as Python or Rust does, it means the same or fails to compile; the specification keeps a false-friend table

*Alternatives:* silent semantic differences documented in a migration guide (Python 3's `/`); run-time traps only.
*Evidence:* PEP 238's silent change is the cautionary tale; models default to Python's meaning for Python's spelling and drift between similar languages, and agents in unfamiliar languages prototype in Python first ([ai-era-and-evolution.md](../research/ideation/ai-era-and-evolution.md) A11); Rust's editions ship migration lints for changed meaning.
*Consequences:* `xs[-1]` with a constant negative index is a compile error with the fix-it `xs[^1]`; the table is checked per row in the experiment's model arm; the `dict` row is Q-42; post-1.0 rows go through editions. See N-49.
*Status:* leaning.

### D-063 The manifest stays data with structured target conditions; comptime budgets are owned by the package that spends them; `sisu eval` renders `pure` modules as configuration

*Alternatives:* Cargo-style `cfg()` strings; an executable manifest (`build.zig`, `Package.swift`, Gradle scripts); a manifest-wide fuel knob (the current text of [08](08-metaprogramming.md)); a separate configuration dialect (Starlark).
*Evidence:* The Borg retrospective, the configuration complexity clock, Declarative Gradle and CDKTF's archiving; Rust 1.72 removed its const-evaluation cap because hitting it depended on library internals; Dhall's and Starlark's designers concede totality does not bound running time; Pkl's model ([config-and-total-languages.md](../research/unconventional/config-and-total-languages.md), ideas 2 to 4).
*Consequences:* `winapi = { version = "0.3", when = { os = "windows" } }`; `@comptime_budget(steps=N)` on the item that spends it, deny-by-default diagnostics naming the expression and package; `sisu eval deploy/prod.sisu --to yaml` with types, refinements and invariants as the schema. See N-54 to N-56.
*Status:* leaning.

### D-064 `@must_consume` propagates through containers; `?` out of a loop over must-consume values is an error unless the remainder is returned

*Alternatives:* containers exempt (today's text); full linearity (Austral); a lint.
*Evidence:* Move's conditional abilities (`Cup<T>` has `drop` only if `T` does) ship the propagation rule; Move's transactions roll back on error where Sisu's `?` returns normally, so the early-return case needs its own rule ([capabilities-distribution-and-resources.md](../research/unconventional/capabilities-distribution-and-resources.md), idea 1).
*Consequences:* Settles the half of Q-37 the owner left to practice; consuming iteration discharges a container; std gains consuming container APIs. See N-15.
*Status:* leaning.

### D-065 Diagnostics and explanations: a message catalog, consequence lines on multi-fix diagnostics, a signature gloss in the teaching profile, `@diagnostic_name`, the lossless syntax tree as a library, a checker that produces types for broken code, `sisu why`, and doc examples rendered with their values

*Alternatives:* English concatenated in code; one fix-it per diagnostic without consequences; a prose "explain this program" generator; error dialogues; a checker that stops at the first error; per-feature explanation flags.
*Evidence:* Marceau, Fisler and Krishnamurthi's novices misread carefully designed messages, largely over vocabulary; Guo's non-native speakers; Scratch's and Excel's language-neutral storage; Luau's infallible checker as its first design goal; Argus (2.2 times as many faults localised, 3.3 times faster) and the Whyline (debugging time cut by nearly a factor of eight); Lean's delaborators and Rust's `#[diagnostic::on_unimplemented]`; Unison's and Go's rendered examples ([end-user-and-live.md](../research/ideation/end-user-and-live.md) 7, 8, 13; [game-and-narrative-scripting.md](../research/unconventional/game-and-narrative-scripting.md) 1; [ai-era-and-evolution.md](../research/ideation/ai-era-and-evolution.md) A9).
*Consequences:* Message identifiers with parameters from the first diagnostic; `sisu why` in human and JSON form over error sets, `Send`/`Sync`, `pure`, capability reach, impl selection and comptime fuel, tracing only in explain mode; `@diagnostic_name` once Q-27's type construction ships. See N-25, N-30, N-37, N-38, N-40, N-47.
*Status:* leaning.

### D-066 `sisu review` reports weakened specifications and authority deltas; dependency admission has a cooldown and refuses new packages in agent runs; packages are labelled pure or resource-holding

*Alternatives:* CODEOWNERS on test directories; Capslock-style capability baselines as a separate tool; no cooldown (npm's default); crowd compatibility scores.
*Evidence:* ImpossibleBench and METR's reward-hacking measurements; Capslock and deps.dev (fewer than 2 % of updates add a capability, so the signal is read); invented package names in 4.6-21.7 % of model suggestions; eight of ten recent attacks with windows under a week; pnpm's `minimumReleaseAge` and uv's `exclude-newer` ([ai-era-and-evolution.md](../research/ideation/ai-era-and-evolution.md) A2, A3; [capabilities-distribution-and-resources.md](../research/unconventional/capabilities-distribution-and-resources.md) idea 4).
*Consequences:* `[review] spec-weakening = "require-trailer"` fails CI without a human-written trailer; the authority delta reuses the reach computation the registry's capability summary already needs; a seven-day cooldown applied before resolution, advisory fixes exempt. See N-17, N-41, N-42.
*Status:* leaning.

### D-067 Libraries ship migrations as `@fix_inline` functions applied by `sisu fix --deps`

*Alternatives:* template-based rewrites (OpenRewrite); macros; deprecation messages only.
*Evidence:* Go's `//go:fix inline` applied by `go fix -inline`; Dart's data-driven fixes verified against goldens; Kotlin's `ReplaceWith` ([ai-era-and-evolution.md](../research/ideation/ai-era-and-evolution.md) A6).
*Consequences:* An ownership-aware inliner (borrows, `own` moves and `defer` move into the caller's scope); a major release's API diff marks migratable removals. See N-45.
*Status:* leaning.

### D-068 Hot replacement publishes epochs adopted at task start and at `live.boundary()`; a failed check publishes nothing; function values hold slots; a changed `const` re-swaps its dependents

*Alternatives:* D-053's next-call rule alone (one iteration can mix versions); image persistence.
*Evidence:* Sonic Pi, Extempore, TidalCycles and Erlang converged on version-consistent replacement at loop boundaries; Flutter's `const` rule; Papyrus's lesson that saved state must hold no frames or code identity ([esoteric-and-live-coding.md](../research/unconventional/esoteric-and-live-coding.md) idea 1; [game-and-narrative-scripting.md](../research/unconventional/game-and-narrative-scripting.md) idea 2).
*Consequences:* An epoch word per task and a versioned function table in debug builds, nothing in release builds; refines D-053. See N-35.
*Status:* leaning.

### D-069 `std.par`: `par_iter`, `par_iter_mut`, `par.join` and reducers with a fixed merge order on a non-blocking pool for `pure` bodies; `SoaList[T]` in `std.collections`; no `par:` block; the `par` keyword deferred

*Alternatives:* a `par:` statement block (occam `PAR`, DPJ `cobegin`); `for x in par xs` as a contextual keyword (Chapel `forall`); a `@layout(soa)` attribute on structs; parallel-by-default loops (Fortress).
*Evidence:* occam's compile-time usage rules and Rust's closure rules are what Sisu's exclusivity and `Send`/`Sync` already give; OpenMP's and `do concurrent`'s trust-based annotations produce silent wrong answers; Fortress was cancelled; Rayon's index-ordered panic rule and adaptive splitting; Cilk hyperobjects; Odin `#soa` and Zig `MultiArrayList` ([parallel-languages.md](../research/unconventional/parallel-languages.md), [algorithm-schedule-separation.md](../research/unconventional/algorithm-schedule-separation.md), [systems-and-determinism.md](../research/ideation/systems-and-determinism.md) 12 and 13).
*Consequences:* Serial elision made exact (removing the parallelism gives the same result; a failing loop reports the lowest-index error); `pure` bodies run to completion on a worker's stack, outside bet 3's budget; reductions follow D-074's fixed shape; the keyword is Q-47. See N-18 to N-20.
*Status:* leaning.

### D-070 `std.test` gains `exhaustive[T]` generators, transcript tests and `@refines`; the `sim` profile and replay tokens are prototyped in the experiment

*Alternatives:* property tests only; snapshot files only; equivalence by hand; simulation and replay as third-party libraries (madsim, turmoil, Shuttle).
*Evidence:* Alloy's small-scope hypothesis and SmallCheck; Inform's "Test me with" and doctest sessions; Exo's reference-equivalence; FoundationDB, TigerBeetle, Shuttle, Coyote, Go's `synctest`; S3's ShardStore; PCT's probabilistic guarantee; R2's interface-level recording; Elm's importable histories ([logic-constraint-and-rewriting.md](../research/unconventional/logic-constraint-and-rewriting.md) 1 and 2, [natural-language-and-literate.md](../research/unconventional/natural-language-and-literate.md) 2, [algorithm-schedule-separation.md](../research/unconventional/algorithm-schedule-separation.md) 2, [systems-and-determinism.md](../research/ideation/systems-and-determinism.md) 2 and 3, [end-user-and-live.md](../research/ideation/end-user-and-live.md) 4).
*Consequences:* A `sim` build fails to compile if reachable code could escape the simulation; faults come only from declared error sets; every failing test prints a replay token; the amendments are Q-49. See N-03 to N-05, N-21, N-36.
*Status:* leaning.

### D-071 Syntax additions: `{expr=}` in interpolation and chained fallible conditions; `View` as a `pure` trait in `std.x`

*Alternatives:* `{"expr"} {expr}` by hand; nested `if let`; a walrus operator; a fourth rendering trait in core.
*Evidence:* Python 3.8's `=` specifier; Swift's condition lists and Rust 1.88's let chains; Tratt's Converge retrospective on keeping failure in `if` without goal-directed evaluation; IPython's display protocol and the moldable inspector ([logic-constraint-and-rewriting.md](../research/unconventional/logic-constraint-and-rewriting.md) 3, [end-user-and-live.md](../research/ideation/end-user-and-live.md) 12, [live-and-exploratory-environments.md](../research/unconventional/live-and-exploratory-environments.md) 5).
*Consequences:* `if let Some(u) = users.get(id), let Some(t) = u.team, t.active:`; a comma, not `and` (D-025); `Display` for users, `Debug` for programmers, `View` for tools, with `View` derived from `Debug` unless written. See N-33, N-53.
*Status:* leaning.

### D-072 A `@preview` feature is stabilised or removed within four releases

*Alternatives:* a minimum only (today's [13](13-evolution-and-governance.md)); indefinite previews.
*Evidence:* Nix flakes, experimental since November 2021 and central to the ecosystem's split ([config-and-total-languages.md](../research/unconventional/config-and-total-languages.md) idea 6); Java's preview features (JEP 12), which are expected to be finalised or dropped within a release or two.
*Status:* leaning.

### D-073 The teaching profile is a monotone ladder of levels, each a strict subset of the next, with teacher-defined ladders as data

*Alternatives:* one profile (Q-41 as decided); Hedy-style levels that change syntax; no profile.
*Evidence:* HtDP's student languages and Pyret; Ada's restriction identifiers; Hedy, where "code ran at the wrong level" was 18.4 % of errors; Scratch's concurrency used freely by children ([end-user-and-live.md](../research/ideation/end-user-and-live.md) 9, [spreadsheets-and-end-user.md](../research/unconventional/spreadsheets-and-end-user.md) 2).
*Consequences:* `profile = "teaching-3"`; level-up diagnostics naming the construct and the level; script mode's top-level statements confined to scripts and teaching levels; the levels themselves are settled by the Q-41 trial. See N-29.
*Status:* leaning.

### D-074 `pure` is deterministic across runs, processes, thread counts and targets; floating point is strict by default

*Alternatives:* `pure` as "no capabilities" only (today's [06](06-concurrency-and-capabilities.md)); determinism by convention with a linter (Temporal, Durable Functions); implicit contraction and fast-math as compiler defaults (Clang).
*Evidence:* Three files found that D-049's per-process `dict` order contradicts [08](08-metaprogramming.md)'s comptime promise and [06](06-concurrency-and-capabilities.md)'s "provably pure"; Python 3.7 separated flooding resistance from iteration order; Rayon's index-ordered failure rule; Java 17's strict floating point (JEP 306), WebAssembly's deterministic scalars, CORE-MATH in glibc, Box2D v3's cross-platform determinism ([systems-and-determinism.md](../research/ideation/systems-and-determinism.md) 1 and 4, [config-and-total-languages.md](../research/unconventional/config-and-total-languages.md) 1, [ai-era-and-evolution.md](../research/ideation/ai-era-and-evolution.md) A11).
*Consequences:* `dict` and `set` iterate in insertion order with a per-process SipHash key (reversing the iteration half of D-049, Q-42); schedule-observing operations are errors in `pure` code and a `pure` scope reports the lowest-index failure (Q-43); `pure` enters function types; no implicit FMA, `a.mul_add(b, c)` for one rounding, `@float(fast)` opts out per function, reductions in a fixed shape (Q-44); correctly rounded `std.math` goes with Q-23. See N-01, N-02.
*Status:* leaning, pending Q-42 to Q-44.

### D-075 Library shapes: capability interceptors and inherited trace context, `task.race`, drift-free `Ticker` and `Timeline` over `Clock`, `std.search`

*Alternatives:* aspect-oriented pointcuts; manual context propagation (Go `context`); `sleep`-based timing; ad-hoc search loops.
*Evidence:* The AOP critique (obliviousness, fragile pointcuts); Kotlin's coroutine context and Erlang's logger metadata; Céu's `par/or` with `finalize`; tokio's missed-tick policies and Sonic Pi's virtual-time semantics; LogicT ([incremental-reversible-and-exotic-paradigms.md](../research/unconventional/incremental-reversible-and-exotic-paradigms.md) 3, [synchronous-and-dataflow.md](../research/unconventional/synchronous-and-dataflow.md) 2, [esoteric-and-live-coding.md](../research/unconventional/esoteric-and-live-coding.md) 2, [logic-constraint-and-rewriting.md](../research/unconventional/logic-constraint-and-rewriting.md) 5).
*Consequences:* One interception point shared by fakes, tracing, auditing and recording; `log` attaches the scope's span automatically; the same timing code is real-time in production and deterministic under the test clock. See N-06, N-35, N-60.
*Status:* leaning.
