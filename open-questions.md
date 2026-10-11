# Open questions

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
cycles; `Arena` with generational handles for graphs.

**What would settle it.** A prototype plus a corpus of three application-
style programs (a web service with a cache, a GUI with a widget tree, a
compiler with a graph IR) written by people who are not the language's
designers, measuring the count of `Shared`/`Weak`/arena uses and the time
spent on ownership errors.

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

### Q-3. Implicit copying: never, for `Copy` types only, or always with copy-on-write?

**Options.** Rust (explicit `.clone()`, `Copy` opt-in): costs visible, some
noise. Swift (implicit copies, COW collections): zero noise, hidden
allocation, the "unexpected copy" performance bug class. Hylo (implicit for
`Copyable`, explicit `sink`): in between.

**Current lean.** Rust's model: `Copy` opt-in, `.copy()` otherwise.

**What would settle it.** The same corpus as Q-1, counting `.copy()` calls
and the number of "why is this slow" incidents attributable to implicit
copies in a Swift-style variant.

### Q-4. Allocation failure: panic, or `! OutOfMemory` everywhere?

**Options.** Rust/Go/Swift (abort or panic; infallible APIs); Zig (every
allocation returns an error; verbose but kernels and embedded need it).

**Current lean.** Panic by default; `try_` variants on every allocating
operation return `! OutOfMemory`; the `freestanding` profile lints
against the infallible forms.

**What would settle it.** Feedback from embedded users of the prototype on
whether the `try_` variants are sufficient or whether they would want a
package-wide switch.

### Q-18. Naming of the parameter conventions

`own` versus `sink` (Hylo), `consuming` (Swift), `take`, `move`. And
whether `mut` at call sites should be `mut v`, `&v` (Swift, Hylo) or
`inout v`. Pure bikeshed with a real ergonomic component; to be settled by
a usability test on the prototype, not by argument.

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

**What would settle it.** Whether generic containers (`List[File]`) need
special handling in practice; try it in the standard library first.

## Concurrency and runtime

### Q-5. How much ambient authority? Standard streams, implicit context

**Question.** `println` without a capability is convenient and almost
harmless; it is also the thin end of a wedge. Implicit context passing
(Odin `context`, Scala `using`, Kotlin context parameters) would remove
the "thread `Fs` through every call" cost at the price of invisible
dependencies.

**Options.**
- Stdio ambient, everything else explicit (current lean).
- Nothing ambient (Austral). Hello world takes a parameter.
- Implicit context carrying capabilities, declared per function (`fn f()
  with Fs, Clock`). Visible in the signature, not at the call site. This is
  close to an effect system (Q-17) and to Scala's capture checking.

**Current lean.** Stdio ambient; everything else explicit; no implicit
context in v1. The `with` clause form is the most likely future addition
because it keeps the signature honest.

**What would settle it.** The capability-parameter count in the Q-1
corpus. If median I/O-doing functions carry more than two capability
parameters, add `with`.

### Q-6. Task runtime details: stacks, and should spawning require a capability?

**Question.** Without a moving GC, task stacks cannot be relocated. Options
are lazily committed virtual-memory stacks (Java Loom on 64-bit; costs
address space and page faults), segmented stacks (Go pre-1.3; the hot-split
problem), compiler-computed stack bounds (works only for non-recursive
tasks), or a hybrid. Separately, Zig's design makes concurrency itself go
through the `Io` value, so a library cannot spawn without being given the
means; Sisu's lean makes `task.scope` ambient.

**Current lean.** Lazily committed stacks with a per-task maximum and
compiler-computed bounds where possible; `task.scope` ambient.

**What would settle it.** Prototype measurements of task creation cost and
memory per task against Go and Java virtual threads; and whether any real
supply-chain attack would be stopped by gating spawn (none found in the
research: attacks exfiltrate, they do not need to spawn).

### Q-34. Preemption

Cooperative scheduling with compiler-inserted yield points (current lean)
versus signal-based asynchronous preemption (Go 1.14). The former is
cheaper and deterministic; the latter handles tight loops in FFI and
`unsafe` code that the compiler cannot instrument. Settled by whether
the prototype's yield-point insertion leaves measurable starvation in
benchmarks that mix compute and I/O tasks.

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

## Errors

### Q-7. Residual error-syntax questions

Inferred error sets (`T !`) allowed on `pub` functions (Zig: yes; lean:
lint, not error)? `else { e -> ... }` versus a `catch` keyword for the
handler? Should `?` on an `Option` inside a function returning `T ! E`
convert `None` to a standard `NoneError` (Rust: no, needs `.ok_or`)? To be
settled by writing the standard library and counting the friction.

### Q-36. A restart/condition system for the application tier

Common Lisp's conditions let the *signaller's* frame offer recovery
strategies that a *handler* higher up chooses, without unwinding. It is
strictly more powerful than exceptions or error values for interactive
and long-running systems (retry with a different file, skip the record,
use a default). It needs dynamic scope. Lean: not in v1; a library can
approximate it with a capability carrying a handler table. Settled by
whether that library approximation is used.

## Types

### Q-10. Higher-kinded types

Haskell and Scala have them; Rust does not and its ecosystem has worked
around it (GATs cover some cases) at the cost of some duplication
(`Iterator` vs `Stream`, `Option`/`Result` combinators repeated). Lean: not
in v1, syntax reserved. Settled by whether the standard library's
abstraction duplication becomes a maintenance cost, which will be visible
within the first year of the prototype.

### Q-11. Coherence: orphan rule versus scoped instances

Global coherence (Rust, Haskell) makes `impl` resolution predictable and
`Map[K, V]` safe, at the cost of newtype boilerplate. Scoped instances
(Scala 3 `given`, OCaml's unshipped modular implicits) remove the
boilerplate and reintroduce "which instance is in scope" confusion.
Lean: orphan rule plus `@delegate`. Settled by counting newtype wrappers in
the corpus.

### Q-13. Implicit lossless widening versus no implicit conversions at all

Convenience versus inference complexity. Lean: widening allowed. Settled by
the prototype's type checker: if widening introduces ambiguity in overload-
free code it stays; if it forces a defaulting rule that surprises people,
it goes.

### Q-14. Grapheme clusters versus scalar values as the default `str` unit

Swift's grapheme default is correct for user-visible text and slow and
awkward for everything else; Rust's scalar default is the reverse. Lean:
scalar, with `graphemes()` one call away and the linter flagging `chars()`
on strings that flow to the UI. Settled by whether the linter heuristic is
good enough.

### Q-16. Verification beyond contracts: ghost code, loop invariants, a verifier

Lean: reserve `decreases`/`invariant` on loops and a `ghost` modifier;
ship a verifier as `std.x.verify` after v1 if SPARK-style discharge of
bounds and overflow proves tractable on Sisu's IR. Settled by a feasibility
study on the prototype.

### Q-26. Anonymous structs and row polymorphism

OCaml's objects, PureScript's and Roc's records, and TypeScript's object
types allow ad-hoc structural data. Sisu has only nominal structs and
tuples. Lean: no; `@derive` and tuples cover the ad-hoc cases, and nominal
types give better errors and documentation. Settled by whether JSON-shaped
configuration handling becomes noticeably verbose in practice.

### Q-27. Comptime-parameterised types alongside trait generics

Zig's `fn List(T: type) type` style is expressive for things traits
express badly (layout-dependent specialisation, SoA transforms). Lean: allow
`comptime` functions returning types for *type construction*, but all
generic *bounds* go through traits. Settled by whether the two mechanisms
are confusable in the standard library.

## Syntax

### Q-8. Braces versus indentation

Indentation (Python, Nim, F#) is measurably readable and loved by its
users; braces are robust for tooling, generation and trailing closures.
Lean: braces. Settled by nothing short of a controlled study, which nobody
has run on experienced programmers; absent that, the tooling argument
stands.

### Q-9. Last-expression block values versus explicit `return` only

Rust's rule confuses newcomers; Kotlin requires `return` in block bodies.
Lean: last-expression values, because `if`/`match` as expressions need
them anyway. Settled by the error-message corpus: if "expected (), found
int" is a top-ten diagnostic in the prototype, reconsider.

### Q-12. Uniform function call syntax

Nim and D let `f(x, y)` be written `x.f(y)`. It makes extension methods
free and chains readable; it makes "where is this method defined" harder
and collides with trait methods. Lean: no; extension traits (`impl MyExt
for str`) cover the use case. Settled by counting extension traits in the
corpus.

### Q-25. Implicit closure parameter (`it`, `$0`)

Lean: no; `{ x -> x * 2 }` is five characters longer than `{ it * 2 }` and
unambiguous. Settled by usability testing.

### Q-33. Non-ASCII identifiers

Rust and Go allow them; the Trojan Source class of attacks and the
practical reality that ecosystems are English-identifier-only argue
against. Lean: ASCII only. Settled by demand from a non-English-dominant
user community, which is a real consideration for teaching.

### Q-35. Cross-scope shadowing: warn or allow

Lean: warn. Settled by lint noise measurements in the corpus.

## Packages and tooling

### Q-19. Registry: central, federated, or both

Lean: one central registry run by the foundation, with `[registries]` in
the manifest for private mirrors, as Cargo does. Federation (Go's URL-based
module paths) avoids a central point of failure and control at the cost of
link rot and the proxy infrastructure Go had to build anyway. Settled by
governance (Q-21).

### Q-20. Lockfile with max-satisfying resolution versus minimal version selection

Cargo and npm pick the newest matching version and record it in a lockfile;
Go picks the oldest that satisfies everyone, deterministically, without a
lockfile. MVS is simpler and has no SAT solver; it also means upgrades are
manual. Lean: lockfile with max-satisfying. Settled by whether the API-diff
semver enforcement (which makes "newest compatible" actually compatible)
holds up; if it does, max-satisfying is safe.

### Q-29. Default visibility within a package

Private-to-module by default (Rust) means a lot of `pub(package)`. Lean:
private to module. Settled by counting `pub(package)` in the corpus.

### Q-30. A real interpreter for scripts

`sisu run file.sisu` compiles then runs. For 50-line scripts, Python's
start-instantly workflow is the competitor. Lean: the debug backend's speed
plus a compilation cache makes this fast enough; add an interpreter mode
only if measured startup exceeds 200 ms for small scripts.

### Q-32. Unused imports and variables: warning or error

Go errors (hated during development, clean in CI); Rust warns. Lean: warn,
auto-fix in `fmt`, `--deny-warnings` in CI. Settled by the Go survey's
evidence, which is already fairly strong for "warn".

### Q-22. Live programming

Smalltalk, Lisp and Clojure users report a workflow (redefine a running
function, inspect live state, fix-and-continue in the debugger) that no
compiled language fully offers; Dart's hot reload and Jupyter are partial
versions that measurably drive adoption. Lean: a compile-and-swap hot
reload for debug builds is feasible given deterministic layout and could
be a post-v1 tooling feature; full image-based development is out of
scope. Settled by demand and by whether the debug backend can support
function replacement cheaply.

## Interop and platform

### Q-24. Direct C++ interop

Lean: no (Carbon's domain). Would be reopened only if a major C++ codebase
wants to adopt Sisu incrementally and the C-ABI wrapper route proves too
costly.

### Q-31. A stable Sisu ABI for dynamic libraries

Lean: unstable; C ABI at boundaries. Reopened if an operating-system vendor
wants to ship Sisu system libraries.

### Q-23. Numerics: SIMD types, rank-polymorphic arrays, broadcasting

APL/Julia/NumPy-style array programming is a genuine upside the mainstream
lacks. Lean: `std.x.ndarray` with operator overloading and comptime-checked
shapes; portable SIMD vector types in `std.simd`; no language-level
broadcasting. Settled by whether the library can express rank polymorphism
without HKTs (Q-10).

## Governance

### Q-21. Governance details

Who the initial lead is, how the language team is selected, what counts as
consensus, how the foundation is funded. The proposal fixes the shape
(phase 1 lead, phase 2 foundation plus team, 90-day time box, no sponsor
majority); the people and the money are not design questions and are
left to whoever starts the project.

### Q-38. Mechanised core semantics

Austral and WebAssembly have formal semantics; Standard ML has a formal
definition; Rust has a retroactive specification. Lean: prose
specification plus a conformance test suite for 1.0; a mechanised core
(ownership and `Send`/`Sync` soundness) as a research collaboration after.
Settled by whether a research group wants to do it.

## Open questions about the open questions

### Q-39. Have we studied the right languages?

The research covers about 45 languages. Notable omissions: Objective-C,
Visual Basic, COBOL, MATLAB/R (array/statistics), Verilog/VHDL (hardware),
Solidity (smart contracts), shell languages, SQL as a language, and the
proof assistants beyond Lean/Agda/Idris. Each could contribute a lesson
(R's data frames, SQL's declarative optimisation, shell's composition).
None is expected to change a core decision; any one could add a library
design.

### Q-40. Is "only the upsides" even coherent?

Several upsides are mutually exclusive: GC convenience and deterministic
destruction; global inference and local error messages; macros and
tooling simplicity; purity and ergonomic I/O. The proposal resolves each by
the priority order in D-001. A reader who rejects that order will reject
some decisions; the decision log is written so that they can see exactly
which ones.
