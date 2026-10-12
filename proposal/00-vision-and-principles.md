# 00 - Vision and principles

## The one-paragraph pitch

Sisu is a statically typed, ahead-of-time compiled, general-purpose language
for writing software that has to keep working: services, command-line tools,
desktop and mobile applications, embedded firmware, compilers, games. It is
memory-safe and data-race-free without a garbage collector, it has no
`null`, no exceptions and no hidden control flow, and its concurrency is
structured and colourless. Every program's access to the outside world flows
through explicit capabilities, so a dependency can only do what you hand it.
The compiler is fast, the error messages teach, the formatter has no options,
and the package manager, test runner, documentation generator and language
server ship in the same binary as the compiler.

## Why another language

The research in [`../research/`](../research/) shows a consistent pattern:
the languages that dominate today each got a few things right at great cost
elsewhere.

- Rust proved that memory safety and data-race freedom without a garbage
  collector are achievable in production, but compile times and `async`
  ergonomics are the top complaints of its users, its learning curve is the
  top reason given by the people who have not adopted it or have left, and
  its original designer has written that he would have made different
  choices on most of the points people struggle with.
- Go proved that fast builds, one formatter, a batteries-included standard
  library and colourless concurrency drive adoption, but it shipped without
  generics, sum types or null safety, and its own team has now given up on
  fixing error-handling verbosity.
- Swift and Kotlin proved that optionals, value semantics and pragmatic type
  inference can be mainstream, but both are tied to a platform owner, and
  Swift's attempt to retrofit strict concurrency checking had to be partly
  walked back in 6.2.
- Zig proved that compile-time execution can replace macros and templates,
  and that explicit allocators and inferred error sets are ergonomic, but it
  offers no safety guarantees in release builds and no way to check a generic
  function before it is instantiated.
- Python and TypeScript proved that readability and gradual adoption win
  users, and also that retrofitting types, packaging and parallelism onto a
  language that did not plan for them takes decades.
- Erlang proved that isolation and supervision beat defensive programming for
  reliability. Hylo, Austral, Koka and Pony each proved one sharp idea
  (mutable value semantics, capabilities, algebraic effects, reference
  capabilities) at small scale.

No existing language combines these. The ones that tried to combine
everything (C++, Scala, D) show the failure mode: feature accretion without a
governing principle. Sisu's claim is not that the ideas are new, but that
picking them with evidence, from the start, with a strict rule for resolving
conflicts, yields a language that none of the above had the chance to be.

## Priorities, in order

When two goals conflict, the higher one wins. This ordering is itself a
design decision; see [`15-decision-log.md`](15-decision-log.md), D-001.

1. **Safety.** Safe Sisu code cannot have memory errors, data races,
   null dereferences or unhandled error paths. There is an `unsafe` escape
   hatch, and it is loud.
2. **Readability.** Code is read far more often than written. Explicit
   signatures, one way to do each thing, no clever syntax. If a feature makes
   code shorter to write but harder to read a year later, it loses.
3. **Predictable performance.** No hidden allocation, no hidden control flow,
   no hidden reference counting in code that does not ask for it. Sisu does
   not promise to be the fastest language; it promises that the cost of code
   is visible in the code.
4. **Fast feedback.** Compile times, error-message quality, and tooling are
   language design constraints, not afterthoughts. A feature that makes
   incremental builds slow or error messages worse has to pay for itself.
5. **Expressiveness.** Generics, traits, pattern matching, closures,
   compile-time evaluation: enough power to write libraries without
   boilerplate, not so much that libraries become languages of their own.
6. **Interoperability.** C ABI in both directions, WebAssembly, and a
   runtime that can be reduced to nothing for embedded targets.

Note what is *not* on the list: brevity, familiarity to users of any one
language, and novelty. Brevity is a consequence of good design, not a goal.
Familiarity is traded away whenever the evidence says the familiar thing is
a mistake. Novelty is actively avoided: every feature should have a
predecessor that validated it.

## Principles

These are the rules of thumb used throughout the proposal. Each links to the
research that motivates it.

### P1. One obvious way

Where two features overlap, keep one. The research on C++, Scala, D and Perl
(see [`../research/languages/cpp.md`](../research/languages/cpp.md),
[`scala.md`](../research/languages/scala.md), [`d.md`](../research/languages/d.md),
[`perl-php.md`](../research/languages/perl-php.md)) shows that feature overlap
compounds: every pair of overlapping features is a style debate, a learning
cost, and an interaction to specify. Python's "one obvious way" and Go's
deliberate minimalism are the counter-evidence.

### P2. Explicit at boundaries, inferred inside

Function signatures, public types and module interfaces are written out in
full. Inside a function body, types are inferred. This is where Go, Rust,
Swift, Kotlin, TypeScript and Carbon all independently landed, and the
reasons are the same each time: readable APIs, errors reported where the
mistake is, and parallel, incremental compilation (a function body can be
type-checked knowing only signatures). Global inference (OCaml, Haskell,
Crystal) saves keystrokes at the cost of all three.

### P3. Errors are values, bugs are panics

A function that can fail says so in its type and returns the failure as a
value. A violated invariant (index out of bounds, integer overflow, failed
assertion) is a bug and panics, unwinding to the nearest task boundary where
a supervisor decides what to do. Exceptions are rejected because they make
control flow invisible (see
[`../research/topics/error-handling.md`](../research/topics/error-handling.md));
Java's checked exceptions are the specific failed experiment that shows how
*not* to make errors visible, and Zig's inferred error sets show how to do
it without boilerplate.

### P4. No ambient authority

Code gets access to the file system, network, clock, randomness and
environment only through capability values handed to it. `main` receives
them; everything else is given what it needs. This is Austral's and Zig's
`Io` design, and it is the only known answer to the dependency supply-chain
problem that is enforced by the compiler rather than by policy (see
[`../research/topics/capabilities-and-effects.md`](../research/topics/capabilities-and-effects.md)).
It also makes code testable without mocking frameworks.

### P5. Values by default, sharing by declaration

A variable owns its value. Passing a value to a function borrows it for the
duration of the call unless the parameter asks to take ownership. Shared
ownership and interior mutability exist, but they are types you choose
(`Shared[T]`, `Cell[T]`), never the default. This is the core of Rust, Swift,
Hylo and Mojo, and it is what makes the absence of a garbage collector
tolerable.

### P6. The tooling is the language

`sisu` is one binary: compiler, package manager, formatter, test runner,
documentation generator, language server, migration tool. Go, Rust, Zig,
Deno, Elixir and `uv` for Python show that integrated tooling is the single
largest determinant of day-one productivity, and the Python, JavaScript, C++
and Haskell packaging histories show the cost of leaving it to the ecosystem.

### P7. Designed to compile fast

No cyclic package dependencies, no header files, explicit signatures, bounded
compile-time evaluation, monomorphisation budgets, a query-based incremental
compiler and a non-LLVM debug backend. Go, Zig and the TypeScript-to-Go port
show that compile speed is a property you design for, and Rust, Swift, Scala
and C++ show what happens when you do not.

### P8. Errors messages are a feature

Elm demonstrated that compiler messages can be designed, tested and iterated
like any user interface, and Rust showed that doing so at scale changes how a
hard language is perceived. Every diagnostic in Sisu has a stable code, a
one-line summary, the source span, and, where possible, a suggested fix.

### P9. Evolve without breaking

A written specification from the first release. A compatibility promise.
Editions that let a package opt in to incompatible changes while linking
against packages on older editions. A migration tool that rewrites code for
you. Python 3, Swift 1-3, Scala 2-3 and Perl 6 show the cost of getting this
wrong; Go's promise and Rust's editions show the value of getting it right.

### P10. Steal, do not invent

Every feature in this proposal has a predecessor in a shipped language, and
the decision log names it. Where no predecessor exists, the feature is an
open question, not a decision. The one place Sisu combines known ideas in a
way no shipped language has (ownership with second-class references plus
capabilities plus structured colourless concurrency) is flagged as the
largest risk in [`../open-questions.md`](../open-questions.md).

## Who Sisu is for

- Teams writing long-lived services and tools who want Rust's guarantees with
  Go's build times and learning curve.
- Embedded and systems programmers who want safety without a runtime.
- Application developers who want value semantics, optionals and structured
  concurrency without being tied to a platform vendor.
- People learning their first compiled language, because the error messages
  and the small core are designed for them.

## Who Sisu is not for (yet)

- Interactive numerical exploration in a notebook. Julia and Python own this
  and Sisu's static, AOT design does not compete; see
  [`14-non-goals.md`](14-non-goals.md).
- Code that must interoperate with C++ templates and classes directly. Carbon
  exists for that.
- Anyone who needs it today. There is no compiler.

## Additions proposed in October 2026

The unconventional-languages round ([19-novel-features.md](19-novel-features.md))
asks two things of this document. The notebook non-goal above should
distinguish a *worksheet* (a script-mode file the language server re-runs
top to bottom with values shown inline and never saved; no cells, no
hidden state), which Sisu can offer as tooling after 0.1, from a
*notebook*, which stays a non-goal unless Q-48 amends it. And one
principle is added as a design rule under P1 and P10: where Sisu spells
something as Python or Rust does, it means the same or fails to compile
(D-062), because the people and models who read Sisu will read it with
those languages' meanings first.
