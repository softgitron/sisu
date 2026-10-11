# Open questions

**Status (October 2026):** the project owner has answered every question
below; 38 are decided and 3 (Q-6, Q-21, Q-23) remain open. Each question
keeps its original options and evidence, followed by the decision, so the
reasoning stays auditable. The proposal documents and the decision log
have been updated to match.

What the research could not settle. Each question states the options, the
evidence for each, the proposal's current lean (what the design documents
assume), and what would settle it. Questions are grouped by area and
numbered stably so that the proposal and decision log can refer to them.

Anyone proposing to close a question should bring one of: a designer
retrospective, a survey figure, a study, or a concrete program that the
current lean handles badly and an alternative handles well.

---

## Memory model

### Q-1. Should there be an optional tracing-GC tier for application code?

**Question.** Sisu has ownership and `Shared` (reference counting) and no
tracing garbage collector. Application code in Go, Java, C# and OCaml never
thinks about ownership. Is "slightly more than Swift, much less than Rust"
the right amount of thinking for application code, or should there be a
`Gc[T]` region or a GC'd runtime profile?

**Options.**
- *No GC anywhere* (current lean). Simplest runtime, one memory model, all
  targets. Risk: application developers find ownership friction too high and
  stay with Go/Kotlin. Evidence: Swift ships ARC-only and is mainstream;
  Rust's survey shows the friction is real but that most users stay.
- *Optional `Gc[T]` type* (Nim's ORC-like cycle collection, or a true
  tracing region as in Verona/Rust's `gc-arena`). Graph-heavy code opts in.
  Risk: two memory models in one ecosystem (D's regret; Nim's history of
  switching defaults), `Send`/`Sync` interaction, runtime size.
- *GC'd runtime profile* where `Shared` is backed by tracing. Same source,
  different trade. Risk: libraries written for one profile behave
  differently (finalisation timing) on the other.

**Current lean.** No GC. `Shared` with Perceus-style reuse and `Weak` for
cycles; `Arena` with generational handles for graphs. A fourth option,
Vale-style generation-checked references outside arenas (5-11% runtime on
the one published benchmark, elidable under static ownership), is the
fallback if the corpus in [proposal/16](proposal/16-validation-plan.md)
shows arenas are not enough; note that Vale itself is archived and its
successor Valen abandoned the mechanism for "group borrowing"
([research/languages/vale.md](research/languages/vale.md)).

**What would settle it.** A prototype plus a corpus of three application-
style programs (a web service with a cache, a GUI with a widget tree, a
compiler with a graph IR) written by people who are not the language's
designers, measuring the count of `Shared`/`Weak`/arena uses and the time
spent on ownership errors.

**Decision (October 2026).** No tracing GC. Ownership with deterministic destruction is the single memory model, and the design effort goes into making it easy: borrow-by-default conventions, no named lifetimes, view types, arenas, and diagnostics that name the fix. Closes D-004's open status.

### Q-2. `Shared[T]`: atomic only, or also a non-atomic single-task variant? Cycle collector or not?

**Options.** Rust's `Rc`/`Arc` split (fast when you pick right, refactor
pain when you change your mind); Swift's always-atomic (measured 10-30%
overhead on pointer-heavy code; Swift later added non-atomic opt-ins);
Nim's ORC (RC plus a cycle collector; cycles never leak; some overhead and
non-determinism in when cycles die); Koka/Lean (atomic only, with reuse
analysis recovering most of the cost).

**Current lean.** One `Shared[T]`, atomic, with the compiler eliding
atomics when it can prove the value never leaves the task (escape
analysis on `Send`-ness). No cycle collector; `Weak` is the tool. Revisit
if the escape analysis proves weak.

**What would settle it.** Benchmarks on the prototype comparing the elision
strategy against a manual `Rc`/`Arc` split on allocation-heavy code.

**Decision (October 2026).** One `Shared[T]`. The compiler decides atomic versus non-atomic reference counting by escape analysis (atomic only when the value can reach another task); the developer never chooses and is never unsafe. No cycle collector; `Weak` for back-references.

### Q-3. Implicit copying: never, for `Copy` types only, or always with copy-on-write?

**Options.** Rust (explicit `.clone()`, `Copy` opt-in): costs visible, some
noise. Swift (implicit copies, COW collections): zero noise, hidden
allocation, the "unexpected copy" performance bug class. Hylo (implicit for
`Copyable`, explicit `sink`): in between.

**Current lean.** Rust's model: `Copy` opt-in, `.copy()` otherwise.

**What would settle it.** The same corpus as Q-1, counting `.copy()` calls
and the number of "why is this slow" incidents attributable to implicit
copies in a Swift-style variant.

**Decision (October 2026).** Rust's model: `Copy` opt-in for trivially copyable types, explicit `.copy()` otherwise.

### Q-4. Allocation failure: panic, or `! OutOfMemory` everywhere?

**Options.** Rust/Go/Swift (abort or panic; infallible APIs); Zig (every
allocation returns an error; verbose but kernels and embedded need it).

**Current lean.** Panic by default; `try_` variants on every allocating
operation return `! OutOfMemory`; the `freestanding` profile lints
against the infallible forms.

**What would settle it.** Feedback from embedded users of the prototype on
whether the `try_` variants are sufficient or whether they would want a
package-wide switch.

**Decision (October 2026).** Allocation failure panics. `try_` variants return `! OutOfMemory` for code that must handle it.

### Q-18. Naming of the parameter conventions

`own` versus `sink` (Hylo), `consuming` (Swift), `take`, `move`. And
whether `mut` at call sites should be `mut v`, `&v` (Swift, Hylo) or
`inout v`. Pure bikeshed with a real ergonomic component; to be settled by
a usability test on the prototype, not by argument.

**Decision (October 2026).** `own` and `mut` on parameters; `mut` repeated at the call site.

### Q-37. Linear (must-consume) types as a type rule rather than a lint

**Question.** A file that must be closed, a transaction that must be
committed or rolled back, a token that must be used exactly once. Austral
makes "used exactly once" the type rule; Rust uses `#[must_use]` plus `Drop`;
Sisu's lean is a lint on types with an explicit `close(own self)`.

**Options.** Lint (zero complexity; can be ignored); `@must_consume`
attribute making non-consumption a compile error (small complexity; a value
in a panicking task is a question); full linearity (Austral; affects every
generic container).

**Current lean.** Attribute making it an error, with panics exempt.

**What would settle it.** Whether generic containers (`list[File]`) need
special handling in practice; try it in the standard library first.

**Decision (October 2026).** `@must_consume` types: not consuming one is a compile error; panics are exempt.


## Concurrency and runtime

### Q-5. How much ambient authority? Standard streams, implicit or tracked context

**Question.** `print` without a capability is convenient and almost
harmless; it is also the thin end of a wedge. Implicit context passing
(Odin `context`, Scala `using`, Kotlin context parameters) would remove
the "thread `Fs` through every call" cost at the price of invisible
dependencies.

**Options.**
- Stdio ambient, everything else explicit (current lean).
- Nothing ambient (Austral). Hello world takes a parameter.
- Implicit context carrying capabilities, declared per function (`def f()
  with Fs, Clock`). Visible in the signature, not at the call site. This is
  close to an effect system (Q-17) and to Scala's capture checking.

**Current lean.** Stdio ambient; everything else explicit; no implicit
context in v1. The `with` clause form is the most likely future addition
because it keeps the signature honest; the Zig research
([research/languages/zig.md](research/languages/zig.md)) recommends exactly
this "tracked capability" shape over both Zig's explicit parameter and
Odin's untracked implicit context.

**What would settle it.** The capability-parameter count in the Q-1
corpus. If median I/O-doing functions carry more than two capability
parameters, add `with`.

**Decision (October 2026).** Standard streams are ambient; everything else is an explicit capability. The `with Fs, Clock` tracked-capability clause stays reserved for after the prototype measures the parameter burden.

### Q-6. Task runtime details: stacks, and should spawning require a capability?

**Question.** Without a moving GC, task stacks cannot be relocated. Options
are lazily committed virtual-memory stacks (costs address space, page
faults, and on Linux the `vm.max_map_count` limit when each task has a
guard page; note that Java's virtual threads are *not* a precedent here,
since Loom copies frames into heap stack-chunk objects on unmount), segmented stacks (Go pre-1.3; the hot-split
problem), compiler-computed stack bounds (works only for non-recursive
tasks), or a hybrid. Separately, Zig's design makes concurrency itself go
through the `Io` value, so a library cannot spawn without being given the
means; Sisu's lean makes `task.scope` ambient.

**Current lean.** Lazily committed stacks with a per-task maximum and
compiler-computed bounds where possible; `task.scope` ambient.

**What would settle it.** The standalone stack harness in
[proposal/18](proposal/18-next-steps.md) (Phase 1, weeks 1-8): task
creation cost and memory per task for five stack strategies against Go,
Java virtual threads, tokio and a C or Rust stackful-coroutine library; and whether any real
supply-chain attack would be stopped by gating spawn (none found in the
research: attacks exfiltrate, they do not need to spawn).

**Decision (October 2026).** Open. To be settled by the prototype's runtime experiment ([proposal/16](proposal/16-validation-plan.md), bet 3).

### Q-34. Preemption

Cooperative scheduling with compiler-inserted yield points (current lean)
versus signal-based asynchronous preemption (Go 1.14). The former is
cheaper and deterministic; the latter handles tight loops in FFI and
`unsafe` code that the compiler cannot instrument. Settled by whether
the prototype's yield-point insertion leaves measurable starvation in
benchmarks that mix compute and I/O tasks.

**Decision (October 2026).** As leaned: cooperative scheduling with compiler-inserted yield points; revisit if the prototype measures starvation.

### Q-17. Should Sisu have an effect system?

**Question.** Capabilities say what code *can* do. An effect system says
what it *does* and lets callers handle it (Koka, Effekt, Flix, OCaml 5,
Unison abilities). Effects would unify generators, cancellation, and
user-defined control flow; they would also add a second dimension to every
higher-order function signature.

**Evidence.** [research/languages/koka-effekt.md](research/languages/koka-effekt.md),
[flix.md](research/languages/flix.md), [unison.md](research/languages/unison.md),
[ocaml.md](research/languages/ocaml.md) (untyped effects shipped; typed
effects still planned), [scala.md](research/languages/scala.md) (capture
checking as a lighter alternative). No production deployment of typed
effects at scale in 2026; OCaml chose untyped to ship.

**Current lean.** No effect system. `pure` as the one tracked property.
Reserve syntax so that a `with` clause on functions could later carry
effects as well as capabilities.

**What would settle it.** Flix or Effekt reaching a production user base
with published ergonomic data, or Scala's capture checking stabilising and
being adopted.

**Decision (October 2026).** No effect system. `pure` is the one tracked property; the `with` clause is reserved for tracked capabilities.


## Errors

### Q-7. Residual error-syntax questions

Inferred error sets (`T !`) allowed on `pub` functions (Zig: yes; lean:
lint, not error)? `else as e:` versus a `catch` keyword for the
handler? Should `?` on an `Option` inside a function returning `T ! E`
convert `None` to a standard `NoneError` (Rust: no, needs `.ok_or`)? To be
settled by writing the standard library and counting the friction.

**Decision (October 2026).** Resolve while writing the standard library, following the current lean: inferred error sets on `pub` functions are a lint; `else as e:` is the handler form; `?` on an `Option` inside a function returning `T ! E` requires an explicit conversion.

### Q-36. A restart/condition system for the application tier

Common Lisp's conditions let the *signaller's* frame offer recovery
strategies that a *handler* higher up chooses, without unwinding. It is
strictly more powerful than exceptions or error values for interactive
and long-running systems (retry with a different file, skip the record,
use a default). It needs dynamic scope. Lean: not in v1; a library can
approximate it with a capability carrying a handler table. Settled by
whether that library approximation is used.

**Decision (October 2026).** No condition system.


## Types

### Q-10. Higher-kinded types

Haskell and Scala have them; Rust does not and its ecosystem has worked
around it (GATs cover some cases) at the cost of some duplication
(`Iterator` vs `Stream`, `Option`/`Result` combinators repeated). Lean: not
in v1, syntax reserved. Settled by whether the standard library's
abstraction duplication becomes a maintenance cost, which will be visible
within the first year of the prototype.

**Decision (October 2026).** Reserved syntax; to be considered after the prototype shows whether the standard library duplicates abstractions.

### Q-11. Coherence: orphan rule versus scoped instances

Global coherence (Rust, Haskell) makes `impl` resolution predictable and
`dict[K, V]` safe, at the cost of newtype boilerplate. Scoped instances
(Scala 3 `given`, OCaml's unshipped modular implicits) remove the
boilerplate and reintroduce "which instance is in scope" confusion.
Lean: orphan rule plus `@delegate`. Settled by counting newtype wrappers in
the corpus.

**Decision (October 2026).** Avoid newtype boilerplate. Coherence is kept, but the *root package of a build* (the application) may implement any trait for any type, because there is exactly one root and nothing depends on it, so no two packages can conflict; libraries keep the orphan rule and use `@delegate`. Scoped instances are rejected.

### Q-13. Implicit lossless widening versus no implicit conversions at all

Convenience versus inference complexity. Lean: widening allowed. Settled by
the prototype's type checker: if widening introduces ambiguity in overload-
free code it stays; if it forces a defaulting rule that surprises people,
it goes.

**Decision (October 2026).** Implicit lossless widening stays.

### Q-14. Grapheme clusters versus scalar values as the default `str` unit

Swift's grapheme default is correct for user-visible text and slow and
awkward for everything else; Rust's scalar default is the reverse. Lean:
scalar, with `graphemes()` one call away and the linter flagging `chars()`
on strings that flow to the UI. Settled by whether the linter heuristic is
good enough.

**Decision (October 2026).** Scalar values are the default string unit; `graphemes()` one call away; a linter heuristic flags `chars()` on strings that flow to user-visible output.

### Q-16. Verification beyond contracts: ghost code, loop invariants, a verifier

Lean: reserve `decreases`/`invariant` on loops and a `ghost` modifier;
ship a verifier as `std.x.verify` after v1 if SPARK-style discharge of
bounds and overflow proves tractable on Sisu's IR. Settled by a feasibility
study on the prototype.

**Decision (October 2026).** As leaned: `decreases`, loop `invariant` and `ghost` reserved; the verifier is post-1.0 tooling.

### Q-28. Is composition plus delegation enough for OOP-heavy domains?

**Question.** GUI toolkits, game entity hierarchies and ORM-style
frameworks are the domains where class inheritance is most used. Sisu has
traits, composition and `@delegate`. Rust (egui, Bevy's ECS), Go, Elm (The
Elm Architecture) and Swift's protocol-oriented style show these domains
can be served without inheritance; Swift and Kotlin kept classes anyway
for framework interop.

**Current lean.** No inheritance; `@delegate` for forwarding; ECS and
architecture patterns documented in the standard library guides.

**What would settle it.** The widget-tree program in the validation corpus
([proposal/16](proposal/16-validation-plan.md)): if its author reaches for
`dyn` trait objects plus delegation and reports it as natural, the
question closes; if they emulate inheritance with boilerplate, it reopens.

**Decision (October 2026).** No inheritance. Composition, traits and `@delegate`.

### Q-26. Anonymous structs and row polymorphism

OCaml's objects, PureScript's and Roc's records, and TypeScript's object
types allow ad-hoc structural data. Sisu has only nominal structs and
tuples. Lean: no; `@derive` and tuples cover the ad-hoc cases, and nominal
types give better errors and documentation. Settled by whether JSON-shaped
configuration handling becomes noticeably verbose in practice.

**Decision (October 2026).** No anonymous or row-polymorphic records. Nominal structs and tuples only, for simplicity.

### Q-27. Comptime-parameterised types alongside trait generics

Zig's `fn List(T: type) type` style is expressive for things traits
express badly (layout-dependent specialisation, SoA transforms). Lean: allow
`comptime` functions returning types for *type construction*, but all
generic *bounds* go through traits. Settled by whether the two mechanisms
are confusable in the standard library.

**Decision (October 2026).** As leaned: `comptime` functions may construct types; all generic bounds go through traits.


## Metaprogramming

### Q-15. Hygienic AST macros after version 1?

**Question.** Version 1 has `comptime` evaluation and reflection but no
way to define new syntax. Rust's proc macros, Elixir's macros, Julia's
`@macro`s, Lean 4's elaborator and Lisp's `defmacro` are the strongest
tools in their ecosystems and the largest sources of compile time,
tooling opacity and fragmentation.

**Options.**
- *Never* (Gleam, Go, Zig). Checked string DSLs and trailing closures cover
  most demand; the Zig standard library shows serialisation needs none.
- *Hygienic, declarative macros* (Scheme `syntax-rules`, Rust
  `macro_rules!`) with expansion visible in the IDE, no procedural tier.
- *Typed AST macros as a stable API* (Nim's direction with Nimony, Lean 4),
  with the cost model decided first: Dart cancelled type-aware macros
  because they could not be fast enough for hot reload; Swift's syntax-only
  macros are slow to build ([research/languages/dart.md](research/languages/dart.md),
  [research/languages/swift.md](research/languages/swift.md)).

**Current lean.** Never, until the corpus in
[proposal/16](proposal/16-validation-plan.md) produces a concrete need that
comptime cannot meet. If added: hygienic and declarative by default,
procedural as a separately marked tier, expansion exposed to tooling, and
a strongly curated standard library to counter the Lisp curse
([research/languages/lisp-scheme-racket.md](research/languages/lisp-scheme-racket.md)).

**What would settle it.** A library that cannot be written acceptably with
comptime plus reflection plus trailing closures; the first candidates are
an HTML templating library and an embedded query language.

**Decision (October 2026).** No macros. Revisit only when the validation corpus produces a need that `comptime` plus reflection cannot meet.


## Syntax

### Q-8. Braces versus indentation

Indentation (Python, Nim, F#) is measurably readable and loved by its
users; braces are robust for tooling, generation and trailing closures.
Lean: braces. Settled by nothing short of a controlled study, which nobody
has run on experienced programmers; absent that, the tooling argument
stands.

**Decision (October 2026).** Indentation-based syntax, following Python idioms wherever a Python idiom is not itself a documented mistake. The rules (layout in the lexer, spaces only, `do` trailing blocks, no brace dialect) come from [research/topics/indentation-syntax.md](research/topics/indentation-syntax.md); the idiom-by-idiom verdicts from [research/topics/python-idioms.md](research/topics/python-idioms.md); every deviation from Python is recorded with its reason in [proposal/02](proposal/02-syntax.md). Decisions D-027 (reversed), D-055.

### Q-9. Last-expression block values versus explicit `return` only

Rust's rule confuses newcomers; Kotlin requires `return` in block bodies.
Lean: last-expression values, because `if`/`match` as expressions need
them anyway. Settled by the error-message corpus: if "expected (), found
int" is a top-ten diagnostic in the prototype, reconsider.

**Decision (October 2026).** Explicit `return` only; blocks never have a value. `if` and `match` have expression forms only when every branch is a single expression (Swift SE-0380's rule). The MATLAB-style alternative the owner asked about, a result variable named in the signature, was assessed in [research/topics/function-results.md](research/topics/function-results.md): its documentation and contract benefits are real, but every shipped version (Go named results, Nim `result`, Pascal `Result`, Eiffel) carries the default-initialised-result and shadowing bug class, and every Go style guide restricts the naked return it enables. Sisu keeps the benefit without the variable: a function may declare *labelled* results, `def divmod(a: int, b: int) -> (q: int, r: int)`, which document the outputs, let `ensures` name them and let callers write `res.q`, while the body still writes `return (q, r)`; an unlabelled result is `result` inside `ensures` only. Decision D-056.

### Q-12. Uniform function call syntax

Nim and D let `f(x, y)` be written `x.f(y)`. It makes extension methods
free and chains readable; it makes "where is this method defined" harder
and collides with trait methods. Lean: no; extension traits (`impl MyExt
for str`) cover the use case. Settled by counting extension traits in the
corpus.

**Decision (October 2026).** No uniform function call syntax. Extension is explicit: an `impl Type:` block in another module adds methods, visible where that module is imported.

### Q-25. Implicit closure parameter (`it`, `$0`)

The first draft leaned no (`x -> x * 2` is a few characters longer than
`it * 2` and unambiguous); the owner decided yes.

**Decision (October 2026).** Yes: an argument expression that mentions `it` where a one-parameter function is expected is a closure (`xs.map(it * 2)`), and a `do:` block without parameters binds `it`. In nested calls `it` belongs to the innermost such argument. Decision D-026.

### Q-33. Non-ASCII identifiers

Rust and Go allow them; the Trojan Source class of attacks and the
practical reality that ecosystems are English-identifier-only argue
against. Lean: ASCII only. Settled by demand from a non-English-dominant
user community, which is a real consideration for teaching.

**Decision (October 2026).** Non-ASCII identifiers are allowed (NFC-normalised), with the Unicode confusable and mixed-script lints Rust added after allowing them, on by default.

### Q-35. Cross-scope shadowing: warn or allow

Lean: warn. Settled by lint noise measurements in the corpus.

**Decision (October 2026).** As leaned: rebinding in the same scope is allowed; shadowing an outer scope warns.


## Packages and tooling

### Q-19. Registry: central, federated, or both

Lean: one central registry run by the foundation, with `[registries]` in
the manifest for private mirrors, as Cargo does. Federation (Go's URL-based
module paths) avoids a central point of failure and control at the cost of
link rot and the proxy infrastructure Go had to build anyway. Settled by
governance (Q-21).

**Decision (October 2026).** The Cargo model: one central registry run by the foundation, plus `[registries]` in the manifest for private mirrors.

### Q-20. Lockfile with max-satisfying resolution versus minimal version selection

Cargo and npm pick the newest matching version and record it in a lockfile;
Go picks the oldest that satisfies everyone, deterministically, without a
lockfile. MVS is simpler and has no SAT solver; it also means upgrades are
manual. Lean: lockfile with max-satisfying. Settled by whether the API-diff
semver enforcement (which makes "newest compatible" actually compatible)
holds up; if it does, max-satisfying is safe.

**Decision (October 2026).** Lockfile with newest-satisfying resolution.

### Q-29. Default visibility within a package

Private-to-module by default (Rust) means a lot of `pub(package)`. Lean:
private to module. Settled by counting `pub(package)` in the corpus.

**Decision (October 2026).** Private to the module by default.

### Q-30. A real interpreter for scripts

`sisu run file.sisu` compiles then runs. For 50-line scripts, Python's
start-instantly workflow is the competitor. Lean: the debug backend's speed
plus a compilation cache makes this fast enough; add an interpreter mode
only if measured startup exceeds 200 ms for small scripts.

**Decision (October 2026).** An interpreter mode is a language feature (`sisu run` on the interpreter, the REPL, scripts), and the interpreter is also the first implementation of the language in the experiment: it is how the design is tested before any native backend exists.

### Q-32. Unused imports and variables: warning or error

Go errors (hated during development, clean in CI); Rust warns. Lean: warn,
auto-fix in `fmt`, `--deny-warnings` in CI. Settled by the Go survey's
evidence, which is already fairly strong for "warn".

**Decision (October 2026).** Warnings, auto-fixed by `sisu fmt` where mechanical, with `--deny-warnings` in CI.

### Q-22. Live programming

Smalltalk, Lisp and Clojure users report a workflow (redefine a running
function, inspect live state, fix-and-continue in the debugger) that no
compiled language fully offers; Dart's hot reload and Jupyter are partial
versions that measurably drive adoption. Lean: a compile-and-swap hot
reload for debug builds is feasible given deterministic layout and could
be a post-v1 tooling feature; full image-based development is out of
scope. Settled by demand and by whether the debug backend can support
function replacement cheaply.

**Decision (October 2026).** Hot function replacement in debug builds is a committed feature: the debug ABI keeps frames reifiable and each top-level definition has an indirection point that release builds remove. Full image-based development remains out of scope.


## Interop and platform

### Q-24. Direct C++ interop

Lean: no (Carbon's domain). Would be reopened only if a major C++ codebase
wants to adopt Sisu incrementally and the C-ABI wrapper route proves too
costly.

**Decision (October 2026).** No direct C++ interop. C ABI wrappers only.

### Q-31. A stable Sisu ABI for dynamic libraries

Lean: unstable; C ABI at boundaries. Reopened if an operating-system vendor
wants to ship Sisu system libraries.

**Decision (October 2026).** Unstable Sisu ABI for the time being; C ABI at boundaries.

### Q-23. Numerics: SIMD types, rank-polymorphic arrays, broadcasting

APL/Julia/NumPy-style array programming is a genuine upside the mainstream
lacks. Lean: `std.x.ndarray` with operator overloading and comptime-checked
shapes; portable SIMD vector types in `std.simd`; no language-level
broadcasting. Fortran's lesson that rank, shape and declared non-aliasing
in the type are what let a compiler vectorise applies: Sisu's exclusivity
rule gives the backend `noalias` with enforcement, but mutable sub-views of
one array (which Fortran's trust model allows and Rust's rules make
awkward) are the case to test ([research/languages/fortran.md](research/languages/fortran.md)).
Settled by whether the library can express rank polymorphism without HKTs
(Q-10) and whether numerical kernels with overlapping mutable views are
writable without `unsafe`.

**Decision (October 2026).** Open; further research needed before `std.simd` and `std.x.ndarray` are designed.


## Governance

### Q-21. Governance details

Who the initial lead is, how the language team is selected, what counts as
consensus, how the foundation is funded. The proposal fixes the shape
(phase 1 lead, phase 2 foundation plus team, 90-day time box, no sponsor
majority); the people and the money are not design questions and are
left to whoever starts the project.

**Decision (October 2026).** Open. Not a design question; left to whoever starts the project.

### Q-38. Mechanised core semantics

Austral and WebAssembly have formal semantics; Standard ML has a formal
definition; Rust has a retroactive specification. Lean: prose
specification plus a conformance test suite for 1.0; a mechanised core
(ownership and `Send`/`Sync` soundness) as a research collaboration after.
Settled by whether a research group wants to do it.

**Decision (October 2026).** As leaned: prose specification plus a conformance test suite for 1.0; a mechanised core as a later research collaboration.


## Open questions about the open questions

### Q-41. A compiler-enforced teaching profile

Racket's HtDP language ladder and Pyret show that restricting the language
for novices measurably helps, and that conventional syntax helps more than
S-expressions ([research/languages/lisp-scheme-racket.md](research/languages/lisp-scheme-racket.md)).
Ada's restriction profiles show a compiler-checked subset is a shipped
mechanism. Lean: a `profile = "teaching"` manifest setting that disables
generics, `unsafe`, `comptime`, concurrency and `ref`, with diagnostics
written for first-year students. Settled by a classroom trial once the
prototype exists.

**Decision (October 2026).** A compiler-enforced teaching profile is planned.

### Q-39. Have we studied the right languages?

The research covers about 45 languages. Notable omissions: Objective-C,
Visual Basic, COBOL, MATLAB/R (array/statistics), Verilog/VHDL (hardware),
Solidity (smart contracts), shell languages, SQL as a language, and the
proof assistants beyond Lean/Agda/Idris. Each could contribute a lesson
(R's data frames, SQL's declarative optimisation, shell's composition).
None is expected to change a core decision; any one could add a library
design.

**Decision (October 2026).** The coverage is sufficient.

### Q-40. Is "only the upsides" even coherent?

Several upsides are mutually exclusive: GC convenience and deterministic
destruction; global inference and local error messages; macros and
tooling simplicity; purity and ergonomic I/O. The proposal resolves each by
the priority order in D-001. A reader who rejects that order will reject
some decisions; the decision log is written so that they can see exactly
which ones.

**Decision (October 2026).** Accepted as a framing: "only upsides" means the best available assumptions under the priority order in D-001, revisited as evidence arrives.

