# The Sisu proposal

## One page

**Sisu** is a statically typed, ahead-of-time compiled, general-purpose
language. In one sentence per area:

| Area | Sisu | Closest relatives |
|---|---|---|
| Safety | Memory-safe and data-race-free in safe code; `unsafe` is a block and a package permission | Rust |
| Memory | Values own their data; parameters borrow by default; references are second-class with no named lifetimes; `Shared[T]` for shared ownership; deterministic destruction | Rust, Hylo, Swift, Mojo |
| Types | Sound static typing; local inference, explicit signatures; `enum` + exhaustive `match`; `T?` instead of null; traits with coherence; `[T]` generics checked at definition | Rust, Swift, Kotlin, Haskell |
| Errors | `T ! E` error sets with payloads, inferred where you want, structurally subtyped; `?` propagates; panics for bugs | Zig, Rust, Swift 6 |
| Concurrency | Structured, colourless tasks; `scope`/`spawn`; cancellation is structural; `Send`/`Sync` inferred | Go, Kotlin, Trio, Java Loom |
| Capabilities | No ambient authority: `Fs`, `Net`, `Clock`, ... are values handed down from `main`; dependencies can only do what they are given | Austral, Zig `Io`, Roc |
| Contracts | `requires`, `ensures`, `invariant`, refinement types; checked, documented, verifiable later | Ada/SPARK, D, Dafny |
| Metaprogramming | `comptime` evaluation and typed reflection; `derive` is a function; no macros | Zig |
| Syntax | Indentation blocks, Python idioms where they are not mistakes, `x -> e` and `do` closures, explicit `return`, one formatter | Python, Nim, Scala 3 |
| Packages | One tool; lockfile; owner-namespaced, content-addressed registry with transparency log; API-diff-enforced semver; no build scripts; `[trust]` gates unsafe | Cargo, Go modules, Elm |
| Tooling | Build, test, fmt, doc, LSP, REPL, migrate in one binary; compile-speed targets; diagnostics with fix-its | Go, Cargo, Elm |
| Evolution | Specification, compatibility promise, editions, six-month releases, preview features | Go, Rust, Java |

What it does not have: `null`, exceptions, class inheritance, `async`/`await`,
macros, header files, truthiness, implicit lossy conversions, undefined
behaviour in safe code, a garbage collector, build scripts, formatter
options.

## Documents

| | Document | What it decides |
|---|---|---|
| 00 | [Vision and principles](00-vision-and-principles.md) | Priorities and the ten principles used to break ties |
| 01 | [Tour](01-tour.md) | The language by example |
| 02 | [Syntax](02-syntax.md) | Lexical rules, literals, declarations, operators, control flow, closures |
| 03 | [Types and generics](03-types-and-generics.md) | Inference, ADTs, optionals, traits, coherence, numbers, strings |
| 04 | [Memory and ownership](04-memory-and-ownership.md) | Values, borrowing without lifetimes, views, `Shared`, `unsafe`, allocators |
| 05 | [Errors](05-errors.md) | Error sets, propagation, handling, traces, panics |
| 06 | [Concurrency and capabilities](06-concurrency-and-capabilities.md) | Tasks, scopes, cancellation, channels, `Send`/`Sync`, capabilities, enforcement |
| 07 | [Contracts](07-contracts.md) | Pre/postconditions, invariants, refinement types |
| 08 | [Metaprogramming](08-metaprogramming.md) | `comptime`, reflection, derive, conditional compilation |
| 09 | [Modules, packages and build](09-modules-packages-and-build.md) | Module tree, manifest, registry, build, workspaces |
| 10 | [Tooling](10-tooling.md) | The `sisu` binary, compile-speed targets, diagnostics, formatter, tests |
| 11 | [Standard library](11-stdlib.md) | `std` and `std.x`, contents and principles |
| 12 | [Interop and targets](12-interop-and-targets.md) | Targets, runtime profiles, C interop, ABI |
| 13 | [Evolution and governance](13-evolution-and-governance.md) | Compatibility, editions, specification, governance phases |
| 14 | [Non-goals](14-non-goals.md) | What Sisu will not try to be |
| 15 | [Decision log](15-decision-log.md) | Seventy-seven decisions with alternatives, evidence and status |
| 17 | [Testing](17-testing.md) | Tests, assertions, fixtures, property tests, fuzzing, snapshots, benchmarks |
| 16 | [Validation plan](16-validation-plan.md) | What the prototype must test first, and what would change the design |
| 18 | [Next steps](18-next-steps.md) | The experiment: phases, corpus, measurements, team, decision points |
| 19 | [Novel features](19-novel-features.md) | What unconventional languages teach and what exists in no language: 66 candidates assessed, 20 new decisions, 12 new questions |
| 20 | [Pre-registration](20-pre-registration.md) | The rules of evidence for the experiment, to be frozen at the end of Phase 0: hypotheses, thresholds, procedure, analysis plan, decision rules, amendments |
| 21 | [Recruitment brief](21-recruitment.md) | The outsiders the experiment needs: roles, eligibility, screening, consent, the call for participants |
| | [Examples](examples/) | Sisu source files |
| | [Open questions](../open-questions.md) | What is not settled, and what would settle it |

The Phase 1 harness that tests the third bet before the language exists
is in [`../taskbench/`](../taskbench/).

## How to read the decisions

Each decision in the log has a status. *Decided* means the evidence was
one-sided enough that the proposal commits, or that the project owner
decided it in October 2026 after reading the options (the open-questions
file records which). *Leaning* means the proposal commits provisionally
and an open question names what could change it; after the owner's
answers only the runtime's stack strategy (Q-6), governance details (Q-21)
and numerics (Q-23) remained open. The unconventional-languages round
([19-novel-features.md](19-novel-features.md)) then added decisions D-058
to D-077 and questions Q-42 to Q-53; the owner answered Q-42 to Q-51 in
October 2026 (reversing the randomised `dict` order and accepting six
amendments to the experiment), which decides D-061, D-069, D-074, D-076
and D-077 and the amendments half of D-070, while the rest stay leaning until the log is reviewed and
Q-52 and Q-53 are open. Draft 3 (October 2026) folds all of these into the
design documents 00 to 18, so each document reads as one text and marks a
leaning decision where it first relies on one.

## The three bets

Most of Sisu is a combination of things that have each been validated by a
shipped language. Three things are combinations that have not shipped
together, and they are where the design could be wrong:

1. **Ownership without named lifetimes is expressive enough.** Rust's
   lifetimes exist for real reasons; Sisu bets that second-class references
   plus single-lifetime views plus `Shared` cover what real programs need.
   Hylo and Mojo are betting similarly and neither has shipped at scale.
2. **Capabilities as ordinary parameters are tolerable at scale.** Every
   I/O function takes one more parameter. Zig is making the same bet with
   `Io`; the outcome is not yet known.
3. **Colourless tasks without a garbage collector are implementable with
   acceptable stack behaviour.** Go and Erlang relocate stacks because
   their GCs can, and Java's virtual threads copy frames into heap chunks
   when a thread unmounts; Sisu can do neither. The closest precedents are
   C and Rust stackful-coroutine libraries, and the experiment measures
   against them first ([18-next-steps.md](18-next-steps.md), Phase 1).

A prototype should attack these three before anything else.
