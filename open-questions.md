# Open questions

**Status (October 2026):** the project owner has answered the first 41
questions; 38 are decided and 3 (Q-6, Q-21, Q-23) remain open. Each
question keeps its original options and evidence, followed by the
decision, so the reasoning stays auditable. The proposal documents and the
decision log have been updated to match. The unconventional-languages
round ([`research/unconventional/`](research/unconventional/),
[`proposal/19-novel-features.md`](proposal/19-novel-features.md)) added
Q-42 to Q-53, all answered by the owner in October 2026; Q-6, Q-21 and
Q-23 were then the only open questions. The backend research (November
2026, [`research/topics/backends.md`](research/topics/backends.md)) adds
Q-58 to Q-61, which are open.

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

---

## Questions from the unconventional-languages round (October 2026)

Each question below comes from [`proposal/19-novel-features.md`](proposal/19-novel-features.md)
and names the candidate (N-xx) and the leaning decision (D-xx) it belongs
to. The owner answered Q-42 to Q-51 in October 2026 and Q-52 (added by the
consistency review) and Q-53 (raised by the owner's answer to Q-44) the
day after.

### Q-42. Should `dict` and `set` iterate in insertion order?

**Question.** D-049 randomised `dict` iteration per process so that, as
the Draft 2 text of [`proposal/11-stdlib.md`](proposal/11-stdlib.md) put
it, code could not depend on it silently. The systems analysis found that this contradicts
two other decided documents: a `pure` function that lists a `dict`'s keys
returns different lists in different processes, which breaks the comptime
promise in [`proposal/08-metaprogramming.md`](proposal/08-metaprogramming.md)
("exactly what would run at run time"), makes memoisation across processes
unsound, and breaks replay and simulation
([systems](research/ideation/systems-and-determinism.md) F1). The config
analysis reached D-049 from the comptime promise alone and proposed a
comptime-only exception ([config](research/unconventional/config-and-total-languages.md)
idea 1); the AI-era analysis reached it as a silent false friend for
Python-trained readers ([AI-era](research/ideation/ai-era-and-evolution.md) A11).

**Options.**
- *Insertion order, per-process SipHash key kept* (lean). Python 3.7's
  split: flooding resistance comes from the key, iteration from insertion
  order. Costs an index array and tombstones (Python's compact dict was
  still 20-25 % smaller than its predecessor); the reason Draft 2 of [`proposal/11-stdlib.md`](proposal/11-stdlib.md)
  gave ("lets code depend on it silently") becomes the point, since a specified order is
  safe to depend on; it also closes the silent false-friend row for
  Python-trained readers (D-062).
- *Keep randomisation, exempt `comptime`* (the config file's minimal fix).
  `comptime` uses insertion order and a fixed key; run time stays random;
  `sisu build --check` builds twice with different keys. Keeps the comptime
  promise but not `pure` determinism, replay or memoisation.
- *Keep randomisation everywhere.* Rust's and Go's position; `pure` stays
  "no capabilities" rather than "deterministic".

**Current lean.** Insertion order (D-074, N-01).

**What would settle it.** The insertion-ordered `dict` within 10 % of a
plain SipHash map on the corpus; the count of corpus programs that read
`dict` order.

**Decision (October 2026).** Insertion order for `dict` and `set`, with the per-process SipHash key kept. The iteration half of D-049 is reversed; D-074 is decided.

### Q-43. How far does deterministic `pure` go?

**Question.** D-074 makes `pure` mean "the same bits on every run,
process, thread count and target". Which operations are errors inside
`pure` code, and does `pure` enter function types?

**Options.**
- *The proposed list*: `select`, `Mutex`, `Atomic`, channels with more than
  one sender, `try_lock`, `try_recv`, `try_send`, `Weak.upgrade` across
  tasks and address-derived values are errors; `task.scope`, `spawn` and
  `join` stay legal; a `pure` scope runs earlier siblings to completion,
  cancels later ones and reports the lowest-index failure (F3). Higher-order
  `pure` code demands `pure` callbacks, so `pure` is part of function types
  (F5), as mutating-versus-non-mutating capture kinds must be for `par`
  bodies.
- *A smaller guarantee*: `pure` forbids only capabilities and shared
  mutation (today's text); determinism is a convention with a lint, as in
  Temporal.
- *A separate `@deterministic` property* beside `pure`
  ([synchronous](research/unconventional/synchronous-and-dataflow.md) idea
  4), which Q-17's one-tracked-property rule argues against, as it does
  D-059's attributes (Q-52).

**Current lean.** The proposed list, with `pure` in function types.

**What would settle it.** The corpus's `pure` functions run under
`--explore 1000` and later natively with 1 and 16 workers with outputs
diffed; more than one rejected `pure` function per 500 lines of pure code
reopens the list.

**Decision (October 2026).** The proposed list: schedule-observing operations are errors in `pure` code, a `pure` scope reports the lowest-index failure, and `pure` is part of function types.

### Q-44. Strict floating point by default?

**Question.** Floating point is specified only as "IEEE 754". D-074
proposes no implicit fused multiply-add (`a.mul_add(b, c)` for one
rounding), no reassociation or fast-math, canonical NaNs where observable,
fixed-shape reductions, and `@float(fast)` to opt out per function; a
correctly rounded `std.math` goes with Q-23.

**Options.** Strict by default with explicit opt-out (Java 17, lean);
contraction within a statement by default (Clang's `-ffp-contract=on`,
which makes x86 and ARM disagree); per-target behaviour documented and
accepted.

**Current lean.** Strict by default; deciding now costs nothing because no
code depends on the alternative, and Java needed twenty years and the end
of x87 to make the same move.

**What would settle it.** A numerical corpus (a physics step, an n-body
integrator, a dot-product kernel) built for x86-64 and AArch64 at both
optimisation levels: bit equality everywhere, and a strict-versus-fast
throughput gap above 25 % on the kernels that matter reopens the default.
This is an input to Q-23's research, not a substitute for it.

**Decision (October 2026).** Strict floating point by default, with `mul_add` for a fused operation and `@float(fast)` to opt out. The owner also asks for exact arithmetic as separate standard-library types: arbitrary-precision integers, decimals that represent base-10 values exactly, and exact symbolic mathematics where that is achievable (D-076; the scope is Q-53).

### Q-45. What does a `Mutex` do when unwinding drops its guard?

**Question.** A task that panics while holding a guard unwinds to its
scope (D-040), the guard drops, and a sibling or a restarted worker can see
half-updated state even when the protected type declares an `invariant`.
The proposal does not say what happens ([systems](research/ideation/systems-and-determinism.md) F4).

**Options.**
- *Re-check the invariant, poison only on failure* (lean). When unwinding
  drops a guard, the protected type's `invariant`, if any, is re-checked;
  if it fails the mutex is poisoned and later `lock()` calls panic; types
  without invariants unlock normally. `lock()` stays infallible in its
  signature.
- *Always poison* (Rust's `std::sync::Mutex`), which makes `lock()`
  fallible or panicking for everyone.
- *Never poison* (Rust's nightly `nonpoison` locks), which leaves the
  half-updated state visible.
- *Crash-isolated scope types* (`UnwindSafe`), which Rust's documentation
  calls "a speed bump" and [19](proposal/19-novel-features.md) rejects.

**Current lean.** Re-check the invariant.

**What would settle it.** The corpus's `Mutex` uses: how many protected
types carry an invariant, and whether any panic-while-locked case in the
structured-concurrency suite is misreported under the lean.

**Decision (October 2026).** Re-check the protected type's invariant when unwinding drops a guard, and poison the mutex only if the check fails (D-077).

### Q-46. Is the typed hole spelled `todo("name")` or `?name`?

**Question.** D-061 makes the prelude's `todo` a typed hole with a report
and a release gate. [holes](research/unconventional/holes-synthesis-and-ai-era.md)
proposes `?name` in operand position instead (Idris, Agda, Lean) and
retiring `todo`; [AI-era](research/ideation/ai-era-and-evolution.md) A1
keeps `todo` because `?` already means propagation, `T?`, `?.` and `??`,
and P1 wants one hole.

**Options.** `todo("name")` as an intrinsic, no new lexical form, nothing
to migrate (lean); `?name`, terser and visually distinct, with `todo`
removed by `sisu fix`; both (rejected by P1).

**Current lean.** `todo("name")`.

**What would settle it.** Fill acceptance with each spelling, measurable
once `sisu fill` exists, is unlikely to differ; this is a taste call for
the owner.

**Decision (October 2026).** `todo("name")`; no `?name` form.

### Q-47. Is `par` a contextual keyword or only a library?

**Question.** D-069 ships `par_iter`, `par_iter_mut`, `par.join` and
reducers in `std.par` with checked independence and serial-elision
semantics. [parallel](research/unconventional/parallel-languages.md) idea
1 argues for `for x in par mut xs:` and `[f(x) for x in par xs]` as a
contextual keyword after `in`, on occam's symmetry between sequential and
parallel forms and Chapel's `forall`, with the compiler naming the crossing
variable when an iteration assigns a shared binding;
[systems](research/ideation/systems-and-determinism.md) 12 argues the
library form is enough and a keyword tempts a task per statement.

**Options.** Library only (Rayon; lean for v1); the contextual keyword
desugaring to `std.par` with the same checks (Chapel); a `par:` statement
block (rejected: `task.scope` already is one).

**Current lean.** Library first; the keyword if the corpus shows more than
one `par_iter` in a plain loop per 300 lines.

**What would settle it.** That count, and whether comprehension bodies
want it.

**Decision (October 2026).** Library first. The owner adds a general rule for every later choice of this kind: keep the language core small and put features in the standard library wherever that is reasonable (recorded in [`proposal/00-vision-and-principles.md`](proposal/00-vision-and-principles.md)).

### Q-48. Worksheets, and the notebook non-goal

**Question.** [`proposal/00-vision-and-principles.md`](proposal/00-vision-and-principles.md)
says Sisu is not for notebook exploration "yet", on an AOT premise that
D-052 weakened. Two candidates touch it: a worksheet (a script-mode file
the language server re-runs top to bottom with values shown inline and
never saved; no cells, no hidden state) and a reactive notebook whose
dependency graph includes in-place mutation tracked statically, which no
notebook has ([end-user](research/ideation/end-user-and-live.md) 5,
[spreadsheets](research/unconventional/spreadsheets-and-end-user.md) 1).

**Options.** Worksheets yes, notebooks still a non-goal (lean: the
non-goal gains a sentence separating the two); amend the non-goal and
prototype the reactive notebook after Q-23 and typed tables exist; keep the
non-goal as written.

**Current lean.** Worksheets as tooling after 0.1; the notebook only with
Q-23's outcome and an amended non-goal.

**What would settle it.** Notebooks written by outsiders, rerun top to
bottom, against marimo on the same tasks; fix-it churn from the
borrow-only rule between cells in a pilot.

**Decision (October 2026).** As leaned: worksheets as tooling after 0.1; notebooks stay a non-goal, with the sentence separating the two added to [`proposal/00-vision-and-principles.md`](proposal/00-vision-and-principles.md); the reactive notebook waits for Q-23 and an amended non-goal.

### Q-49. Which amendments to the experiment's pre-registration are accepted?

**Question.** Six candidates ask for a place in the experiment of
[`proposal/18-next-steps.md`](proposal/18-next-steps.md), whose principle 6
is "nothing beyond the bets before the verdict". Most are argued to
strengthen a bet's measurement rather than add polish; two (the binding
count and the model arm) measure questions of their own:

| Amendment | Phase | Cost | Strengthens |
|---|---|---|---|
| One io_uring cancellation test into stack buffers (N-07) | 1 | days | bet 3's cancellation semantics; the guaranteed-drop rule |
| The dynamic ownership monitor as a counterexample generator for rejected programs, team-only (N-26) | 3 | about 1 week | bet 1's false-positive accounting |
| The count of bindings typed by later statements (N-48) | 5 | hours | Q-50 |
| The `sim` profile with typed fault injection and PCT scheduling (N-03) | 6, 8 | about 4 weeks | bet 3's cancellation and panic semantics under faults and crashes |
| The capability log, `sisu replay` and replay tokens (N-04) | 8 | about 1 week | corpus triage; bet 3's schedule exploration |
| A model arm with a pocket reference and an API digest (N-50) | 9 | model-API money and a PI's time | the AI-assistance penalty measured on day one; carries D-062's false-friend rows |

**Options.** All six, found by trimming Phase 8's polish or extending the
plan by a month (lean); the cheap four without the `sim` profile and the
model arm; none, keeping principle 6 strict.

**Current lean.** All six; the pre-registration is amended before any
participant writes a line, and the `sim` profile is justified by Phase 6's
existing 1000-schedule exit criterion.

**What would settle it.** The owner's budget; the Phase 0 pre-registration
review.

**Decision (October 2026).** All six amendments are accepted into the pre-registration.

### Q-50. Should a binding's type be fixed by the end of its declaring statement?

**Question.** Mündler et al. had to enforce initialised-or-annotated
variables and no use before declaration to constrain TypeScript decoding,
and MoonBit counts its top-level signature rule as model-friendly
([AI-era](research/ideation/ai-era-and-evolution.md) A10). Sisu's local
inference lets a later `push` fix an empty collection's element type.

**Options.** Keep D-003's local inference as is (lean until measured); a
left-to-right rule with annotations on empty collections, which also puts a
binding's type where it is declared for readers; the rule only for `pub`
items (already the case).

**Current lean.** Measure first (Q-49): adopt the rule if fewer than about
3 % of corpus bindings are typed by later statements and authors report no
friction.

**What would settle it.** That count, and a constrained-decoding pilot
with and without the rule.

**Decision (October 2026).** Measure first; adopt the left-to-right rule if fewer than about 3 % of corpus bindings are typed by later statements and the authors report no friction.

### Q-51. Should capabilities stay `Copy`?

**Question.** Capabilities are `Copy` ([`proposal/06-concurrency-and-capabilities.md`](proposal/06-concurrency-and-capabilities.md)),
so authority can be narrowed by attenuation but never taken back, and a
callee can keep what it was handed. [capabilities](research/unconventional/capabilities-distribution-and-resources.md)
idea 3 proposes `std.cap` with caretakers (dynamic revocation, `Revoked`
joining every I/O error set) and lent capabilities typed as views (zero
cost, the callee cannot store them).

**Options.** Keep `Copy`; add `Lent[T]` views and caretakers in `std.cap`
and let APIs accept both (lean); make capabilities non-`Copy` so every
hand-off is a move or an explicit `.copy()` (Austral's linear
capabilities; costs every call site); capabilities as views by default with
an explicit `.retain()` (the lending rule inverted).

**Current lean.** Keep `Copy` with `std.cap`.

**What would settle it.** The bet-2 port study: how often a callee stores
a capability it should not have kept, and whether `Lent[T]` is asked for.

**Decision (October 2026).** Capabilities stay `Copy`; `std.cap` ships the lent and revocable forms.

### Q-52. May resource attributes and closure capture kinds enter function types?

**Question.** Q-17 decided "no effect system; `pure` is the one tracked
property". D-059 makes `@no_alloc`, `@no_block` and `@stack(max=N)` part
of function types so that `dyn` methods and closure parameters can require
them, and `par` bodies (D-069) and bit-deterministic `pure` (Q-43) need
function types to say whether a closure mutates its captures. Each is a
tracked property in all but name.

**Options.**
- *Yes, as attributes on function types* (D-059's lean): `(x: int) -> int
  @no_alloc`; a closure parameter declared `@no_block` accepts only
  closures the checker proved non-blocking; capture kinds (`own`, `mut`,
  borrowing) shown in the type. Costs the signature noise Q-17 refused,
  limited to the functions that opt in.
- *Checked per function over the call graph, never in function types*:
  `dyn` calls and closure parameters are assumed to allocate and block, so
  an `@no_alloc` function cannot take a closure or call through `dyn`. No
  type-level change; realtime code gives up higher-order style, as Wuffs
  does.
- *Neither*: drop D-059's attributes, keep `sisu audit` reports only.

**Current lean.** The first, limited to the three attributes and capture
kinds, with the signature cost measured on the realtime ports of N-08.

**What would settle it.** The three realtime ports of N-08: how many
closure parameters and `dyn` calls their code needs, and whether the
call-graph-only option would have rejected them.

**Decision (October 2026).** Option two. The attributes are checked per function over the monomorphised call graph and never appear in function types; inside an attributed function a `dyn` call or a call through a non-generic closure parameter is an error, because the checker must assume it allocates and blocks, while a generic closure parameter is checked once its concrete closure is known. Closure capture kinds are likewise inferred per closure and checked at the use site, never written in a type. The owner's rule: avoid clutter and let the compiler deal with the details.

### Q-53. How far does exact arithmetic go, and what are its semantics?

**Question.** Answering Q-44, the owner asked for "perfect math" as
separate standard-library types: integers of unbounded size, decimals that
represent base-10 values exactly, and exact symbolic mathematics where
achievable. [`proposal/11-stdlib.md`](proposal/11-stdlib.md) already lists
`BigInt`, `Decimal` and `Ratio` in `std.num`; D-076 commits to them with
literal support. Two things are not settled: the semantics of `Decimal`,
and what "symbolic" should cover.

**Options.**
- *`Decimal` semantics.* Unbounded precision under a context that sets
  precision and rounding (Python's `decimal`, IEEE 754-2008's general
  decimal arithmetic; exact for addition and multiplication, rounded for
  division); or a fixed decimal128 (exact to 34 digits, a `Copy` value,
  SIMD-friendly); or a scaled integer (`BigInt` plus an exponent, exact
  for everything but division, which returns `Ratio`). Lean: the scaled
  `BigInt` form as `Decimal`, with division producing `Ratio` unless a
  rounding context is given, because it is exact by construction and needs
  no hidden context.
- *Symbolic mathematics.* Exact rationals and algebraic numbers only, in
  `std.num` (lean for v1); a computer algebra system (expression trees,
  simplification, differentiation, equation solving) in `std.x.symbolic`
  after 1.0, which is a project the size of SymPy; or a community package
  from the start.
- *Literals.* `FromLiteral` lets `let n: BigInt = 123456789012345678901234567890`
  and `let d: Decimal = 0.1` check at compile time with no suffix (lean);
  or suffixed literals (`0.1d`, Java's `123n`).

**Current lean.** `BigInt`, `Ratio` and a scaled `Decimal` in `std.num` for
v1, implemented in Sisu with `comptime`-eligible `pure` code and host
kernels in the interpreter; algebraic simplification and a CAS in `std.x`
after 1.0, scoped by Q-23's numerics research.

**What would settle it.** The numerics research of Q-23 (which decides
the numeric traits every exact type must implement), a port of a
bookkeeping program and of an exact-geometry kernel under each `Decimal`
option, and whether a CAS maintainer wants to build on Sisu.

**Decision (October 2026).** `Decimal` is exact without bound: a scaled `BigInt` (an integer mantissa and a decimal exponent), so addition, subtraction and multiplication never round, and division yields a `Ratio`; rounding is an explicit conversion with a named mode, and there is no hidden precision context. `BigInt`, `Ratio` and `Decimal` ship in `std.num` with suffix-free literals, kept simple in the first phase. Symbolic mathematics comes later, in the standard library, after 1.0.


## Questions from the backend research (November 2026)

Each question below comes from [`research/topics/backends.md`](research/topics/backends.md)
and decisions D-085 and D-086, which choose a debug backend written in Sisu
and LLVM, reached through bitcode, for release builds. All four are open.

### Q-58. What seeds the toolchain once no Rust remains, and does Sisu write WebAssembly itself?

**Question.** D-078 keeps the Rust front end until the backend and the
runtime are Sisu too, and D-085 retires the Rust backend and runtime when
the Sisu ones reach their fixed point. A toolchain written only in Sisu then
needs a Sisu compiler to build it from source. Go and Rust build each release
with the previous one; Zig commits a WebAssembly build of its compiler (637
KB compressed) and a 4,000-line C translator, so that a fresh machine needs
only a C compiler; Nim ships generated C. A Wasm seed needs Sisu to emit Wasm
itself, since LLVM is optional (D-086), and that emitter would also be the
debug backend for `wasm32-wasi`, where Sisu's tasks need stack switching
(phase 3), JS Promise Integration (browsers only) or a transformation such
as Binaryen's Asyncify (binaries about 50% larger).

**Options.**
- *The previous release* (Go, Rust): no new backend; a new platform is
  cross-compiled from a supported one; trust rests on reproducible builds
  along a chain of binaries.
- *A committed WebAssembly seed* (Zig): a Wasm emitter in the Sisu backend,
  the runtime's `single` profile on Wasm, and a small C program that
  translates or interprets the seed, regenerated only when the compiler's
  own needs change.
- *A C seed* (Nim): a C backend kept for this one use.
- *The frozen stage 0*: the Rust front end, backend and runtime kept
  buildable at a tagged commit as the from-source path; Rust stays in the
  repository's history, not in the toolchain.

**Current lean.** The previous release as the routine path, with the frozen
stage 0 kept buildable until a Wasm seed exists; Wasm emission as the Sisu
backend's third target after x86-64 and AArch64, with tasks on it by a
transformation until stack switching is standardised.

**What would settle it.** The cost of the Wasm emitter and of the `single`
profile on it, against keeping stage 0 buildable; whether the distributions
that package Sisu require a bootstrap from source (Zig's packaging troubles,
issue 16270); the stack-switching proposal's progress.

### Q-59. How does LLVM ship, which versions does the release backend support, and what does `--release` do without it?

**Question.** D-086 keeps LLVM out of the toolchain binary and runs it as a
separate program on bitcode that Sisu writes. Something must provide that
program, at a version that reads the bitcode and has the target.

**Options.**
- *Bundled*: a pinned LLVM (`clang`, or `opt` and `llc`, with `lld`) in
  every toolchain download, as Zig ships one today; 100 to 170 MB more per
  host (libLLVM alone is 118 to 137 MiB on the research machine).
- *An optional component*: `sisu` fetches a pinned LLVM on the first
  `--release` build or on request, content-addressed and cached like a
  package ([09](proposal/09-modules-packages-and-build.md)).
- *The system's*: any installed Clang inside a supported range, as GHC
  accepts LLVM 13 up to 21 and Odin 17 to 22; the bitcode is written for the
  oldest version in the range, which newer ones read (LLVM loads any bitcode
  since 3.0).
- *Without LLVM*, `--release` either fails and names the command that fetches
  it, or builds with the Sisu backend's code under the release profile's
  checks and contracts and says so.

**Current lean.** The optional, pinned, content-addressed component, with a
system Clang accepted inside a range of the last four LLVM releases;
`--release` without LLVM fails and names the fix, because a silently
unoptimised release build breaks priority 3 (predictable performance).

**What would settle it.** Download and disk sizes per host; the bitcode
features Sisu needs and the oldest LLVM that has them; the CI time to test
each version in the range; whether `release-size` changes the answer.

### Q-60. How fast must generated code be: what does LLVM buy on Sisu's own programs, and must the debug backend keep scalars in registers?

**Question.** On stage 1, Cranelift's optimising level gains little because
the time is in the runtime's intrinsic dispatch and its walks of the type
table, while arithmetic loops gain 1.3 to 2.9 times (architecture decisions
232, 233 and 246). Whether LLVM's pipeline changes that depends on its
inlining the runtime's fast paths, which it can do only once the runtime is
Sisu compiled with the program. On the debug side, a single-pass backend
with every local in memory makes code like LLVM `-O0`, up to 1.64 times
slower than Cranelift's in TPDE's comparison, and debug code is what `sisu
test --native` and a developer's own runs execute.

**Options.**
- *Measure first*: LIR printed as LLVM textual IR (a few thousand lines of
  Sisu), run on stage 1 and the corpus with the Rust runtime and again with
  the Sisu runtime, before the bitcode writer is built.
- *Build the bitcode writer at once* and measure with it.
- *Debug builds*: every local in memory (today's debug profile and the
  simplest debug information); scalars in registers by linear scan on SSA
  with DWARF location lists (Wimmer and Franz; Go's allocator); or two debug
  levels.

**Current lean.** Measure first; keep every local in memory in debug builds
until the measurement of tests and of stage 1's debug runs says otherwise.

**What would settle it.** Stage 1's time to print its own SIR and to check
the corpus under Cranelift's two levels, the textual-IR LLVM path at `-O0`
and `-O2`, and the Sisu backend; the same for `sisu test --native` on the
conformance corpus; each path's build time against 10's 10 s and 60 s.

### Q-61. Which linker does the toolchain use?

**Question.** Stage 2 links with the system's `cc`, using `ld.lld` when it
is installed (87 against 224 ms for a small program). A toolchain written in
Sisu still needs a linker for each object format, Windows debuggers need PDB
files built from CodeView, and 10's one-second rebuild and relink wants
incremental linking or patching. Zig wrote its own linkers, and they are
what still holds its own backends back on Windows and AArch64 (its 0.17
roadmap).

**Options.**
- *The system linker through `cc`* (today): no work; differences per
  platform, and Windows needs MSVC's linker or `lld-link`.
- *`lld`, bundled or fetched with LLVM* (Q-59): ELF, Mach-O, COFF and Wasm in
  one tool, PDB from CodeView, Apache-2.0 with LLVM exceptions.
- *A linker in Sisu* (Go, Zig): incremental relinking under the compiler's
  control and no C++ anywhere; the largest cost of the three, per format.

**Current lean.** `lld` as an external program, with the system linker as
the fallback, through 0.x; a linker in Sisu only for ELF, and only if hot
replacement (D-053) does not already meet 10's rebuild target.

**What would settle it.** The relink time of a 100 kLOC program after a
one-function edit with `lld`; whether `sisu run --watch` makes the relink
target moot for the edit loop; the Windows story (`lld-link`'s PDB files
against MSVC's in the debuggers people use).

## Questions from the unsafe round (November 2026)

Each question below comes from [`research/topics/unsafe.md`](research/topics/unsafe.md)
and the design in [`proposal/22-unsafe.md`](proposal/22-unsafe.md), and
names the leaning decision it belongs to (D-080 to D-084, D-087, D-088).
All six are open.

### Q-54. Are three machine intrinsics enough, or does Sisu need inline assembly?

**Question.** D-088 writes the runtime in Sisu with no inline assembly:
`rt.switch`, `rt.stack_pointer` and `rt.thread_slot` are `unsafe def`s
whose bodies the backend lowers. The Rust runtime gets its context switch
from corosensei's inline assembly, and the Phase 1 harness reads the stack
pointer and a thread-local stack limit with `asm!`. A `freestanding`
kernel or driver also needs interrupt entry and return, instructions such
as `wfi`, `hlt` and `cpuid`, control-register access and cache
maintenance.

**Options.**
- *A closed set of intrinsics, grown on demand* (lean). Each new need is a
  named `unsafe def` in `std.rt` or a per-target module that the backend
  lowers, as Rust's `core::arch` does for SIMD; the toolchain stays one
  language and the interpreter's checked model can describe every
  intrinsic. Costs a backend change for every new instruction.
- *Inline assembly inside unsafe blocks* (Rust's `asm!`, Zig's `asm`).
  General, but every target's assembler syntax and register constraints
  become part of the language and the specification, and the interpreter
  cannot run it.
- *Assembly files assembled by the toolchain* (Go's `.s` files with its
  own assembler). Keeps assembly out of the language, at the price of an
  assembler per target and an ABI between Sisu and assembly functions.

**Current lean.** Intrinsics only for version 1.

**What would settle it.** The runtime ported to Sisu on x86-64 and
AArch64 (switch, system stack, probes, decommit), and one `freestanding`
port, the interrupt-driven UART driver that is already one of D-059's
realtime ports: if either needs more than about a dozen intrinsics, or one
that cannot be specified without naming registers, the question reopens.

### Q-55. How does the interpreter run unsafe code and foreign calls, and what may checking cost?

**Question.** D-087 makes the interpreter check raw memory as Miri checks
Rust's MIR, so that `sisu test` checks unsafe code on every run and
comptime can evaluate std's collections written over raw memory. Miri runs
3,000 to 7,000 times slower than unoptimised native code and cannot run
the tests of the 21.7% of crates that call into C. The interpreter has no
raw memory and no foreign calls today; its host objects stand in for
std's collections.

**Options.**
- *A checked model for user code, host kernels for std, models for
  foreign calls* (lean). Unsafe code in user packages runs in the checked
  model under `sisu test`; std's collections keep host kernels in the
  interpreter, as D-076's exact types do, checked against their Sisu
  implementation by the conformance suite; a foreign call runs in the
  interpreter only through a model written in Sisu (the `sim` profile's
  mechanism), so a binding package's other tests run natively.
- *The checked model everywhere, std included.* One implementation of
  `list` in every mode, and std's own tests become the main check of std's
  unsafe code; every interpreted program pays the slowdown.
- *A foreign-function bridge in the interpreter* (`dlopen` and a
  libffi-style call), unchecked across the boundary, so bindings can be
  tested in the interpreter at the price of trusting C there too.

**Current lean.** The first.

**What would settle it.** The slowdown of the checked model on std's own
tests and on the ported runtime's (the lean holds if `sisu test` of an
unsafe-heavy module stays within about 50 times its unchecked run), and the
share of corpus packages with foreign calls whose tests cannot run without
a bridge.

### Q-56. Which aliasing rules do raw pointers follow?

**Question.** The contract of [`proposal/22-unsafe.md`](proposal/22-unsafe.md)
says that nothing writes behind a live `ref` except through a `Cell`,
that nothing touches what a live `ref mut` points to except through it,
and that a raw pointer made from a borrow carries that borrow's
permission. How raw accesses interact with references made before and
after them (when a raw write ends a reference; whether a raw pointer
survives a reborrow of its parent) is what Rust spent Stacked Borrows and
Tree Borrows on, and it decides which optimisations the release backend
may make, such as `noalias` on `mut` parameters.

**Options.**
- *The monitor's loan tree extended to raw pointers* (lean). A raw
  pointer carries the tag of the loan it came from and is a child of it,
  and every access ends the loans it conflicts with, as the monitor
  already does for references. This is close to Tree Borrows, which
  rejects 54% fewer test cases than Stacked Borrows on 30,000 crates and
  keeps most optimisations.
- *Raw pointers outside the model.* Only references made from raw
  pointers are checked, and raw accesses never end a loan: simpler, fewer
  false reports, fewer optimisations.
- *No aliasing guarantees beyond types* (C's and Zig's position): no
  `noalias`, and nothing to check.

**Current lean.** The loan tree extended, written into the specification
from the monitor's rules.

**What would settle it.** The ported runtime and std run under the
extended monitor: the count of reports that are not bugs (more than one
per thousand lines of unsafe code reopens the choice), and the release
profile's measured gain from `noalias` on `mut` parameters.

### Q-57. Should `std.rt`'s machine layer be public for `freestanding` programs?

**Question.** D-088 keeps `std.rt` (the machine intrinsics, the stack
pool, the scheduler beneath `std.task`, the C library bindings behind
`System`) internal to `std`, so that only the runtime can switch stacks or
reach the thread slot. A `freestanding` program supplies its own `System`
([`proposal/12-interop-and-targets.md`](proposal/12-interop-and-targets.md))
and may want its own scheduler, interrupt-driven tasks or stacks in memory
it manages. Go's `linkname` history shows that runtime internals get used
once they are reachable.

**Options.**
- *Internal in version 1* (lean): a `freestanding` program uses std's
  scheduler over a platform trait, and a public `std.x.rt` follows if a
  port needs more.
- *Public in std from the start*, under the compatibility promise.
- *Never public*: kernels in Sisu use std's scheduler or none.

**Current lean.** Internal in version 1.

**What would settle it.** A `freestanding` port with tasks (a small kernel
or an embedded scheduler with interrupts): whether it can live with std's
scheduler over a platform trait, and which of `std.rt`'s operations it
asked for.

### Q-71. Does safe reinterpretation of plain data ship in version 1?

**Question.** D-081 proposes `@derive(Plain)` for `Copy`, `@repr(c)` types
without padding, invariants or refinements whose fields are plain, with a
safe `bit_cast`, `as_bytes`, `read_plain` and `view_plain`. Reinterpreting
bytes as structs is a leading reason application packages use `unsafe`
(Go's conversion patterns, Rust's zerocopy and bytemuck crates), but it
adds a trait, a derive and layout rules to version 1.

**Options.**
- *Version 1, an opt-in derive checked by the compiler* (lean), as `Copy`
  is opt-in.
- *An auto trait inferred structurally*, like `Send`: no annotation, but a
  type's plainness changes with a private field, and a type whose
  invariant matters is plain by accident unless invariants exclude it.
- *After the corpus*, if its parsers need `unsafe`.
- *Never*: reinterpretation stays `mem.transmute` in unsafe code.

**Current lean.** Version 1, opt-in.

**What would settle it.** The count of `unsafe` blocks in the corpus's
parsers and its protocol and file-format code that `Plain` would remove;
more than one per program keeps it in version 1.

### Q-72. Should safety obligations be checked names, and should fields carry them?

**Question.** D-083 asks every `unsafe def` to list named obligations in a
`Safety:` section and every block's `# safety:` comment to name the
obligations it discharges, which a lint checks. Rust's RFC 3458 goes
further with unsafe fields: writing a field that carries a safety
invariant, such as a buffer's length, becomes an unsafe operation, because
otherwise any safe code in the module can break what the blocks rely on.

**Options.**
- *Named obligations checked by a lint, no unsafe fields* (lean): the
  module is the unit of review (D-083), and privacy keeps the fields that
  carry invariants inside it.
- *Free-text comments only*, as Rust and the Linux kernel have them.
- *Named obligations and unsafe fields.*
- *A safety-property language in attributes* (the Rust pre-RFC's
  `#[safety]` tags), checked against call sites.

**Current lean.** Named obligations checked by a lint, no unsafe fields.

**What would settle it.** The runtime port: how many of its soundness
bugs come from safe code in the same module breaking an invariant a block
relied on (unsafe fields would have caught them), and how many
undischarged obligations the name check finds.
