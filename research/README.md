# Research methodology

This directory holds the research that precedes the Sisu language proposal.
The goal is not to describe languages, but to extract **transferable
lessons**: which design decisions demonstrably worked, which demonstrably did
not, and where the evidence is genuinely mixed.

The proposal in [`../proposal/`](../proposal/) must be traceable back to this
research. Every design decision in the proposal's decision log cites the
language analyses and topic syntheses that justify it.

## Structure

| Directory | Contents |
|-----------|----------|
| `languages/` | One analysis per language (or small family). Primary research. |
| `topics/` | Cross-cutting syntheses: one per design dimension (type system, memory, errors, ...). Compares the approaches found in `languages/` and ends with a recommendation and open questions. |
| `evidence/` | Empirical material: developer surveys, academic studies, industry memory-safety data, designer retrospectives. Shared by all other documents. |
| `unconventional/` | Second round (October 2026): one file per family of unconventional languages and environments (block and projectional editors, live environments, spreadsheets, natural-language and literate programming, synchronous and dataflow, parallel, Halide-style schedules, logic and constraint, configuration, capability and resource, incremental and exotic paradigms, esoteric and live coding, game scripting, holes and synthesis). Each ends with ranked transferable ideas and dead ends; see its [README](unconventional/README.md). |
| `ideation/` | Three candidate-feature studies (learners and live work, systems and determinism, the AI era and evolution) that ask what Sisu could offer that no language does, each candidate with precedents, costs, a verdict and what would settle it. Assessed in [`../proposal/19-novel-features.md`](../proposal/19-novel-features.md). |

## Which languages and why

Languages were chosen to cover the design space, not popularity alone.
Each one is here because it is either (a) widely used, so its pain points are
well documented, (b) the origin of an idea that other languages later copied,
or (c) a recent design that attempts to fix a known problem of an older one.

| Group | Languages | Why |
|-------|-----------|-----|
| Systems, old guard | C, C++, Ada/SPARK, Fortran | Origin of most performance and safety debates; decades of post-mortems. |
| Systems, modern | Rust, Zig, Odin, Jai, C3 | Memory safety without GC; compile-time execution; reactions against C++. |
| Managed mainstream | Go, Java, C# | Garbage collection at scale; simplicity vs. expressiveness; evolution over 20+ years. |
| Platform-tied modern | Swift, Kotlin, Dart, Scala | Pragmatic modern type systems; null safety; structured concurrency; interop as adoption driver. |
| Dynamic / scripting | Python, Ruby, Perl, PHP, Lua, JavaScript, TypeScript | Learnability, ergonomics, gradual typing, and the cost of retrofitting rigor. |
| Typed functional | Haskell, OCaml, F#, Standard ML, Elm, PureScript, Idris, Agda, Lean | Type inference, ADTs, purity, effects, dependent types; what is practical and what is not. |
| Paradigm outliers | Erlang/Elixir, Gleam, Clojure, Common Lisp, Scheme/Racket, Smalltalk, Prolog, APL/BQN/K, Forth | Ideas mainstream languages lack: fault tolerance, image-based development, homoiconicity, array programming, logic programming. |
| New and experimental | Julia, Nim, D, Crystal, Mojo, Carbon, Hylo, Austral, Pony, Vale, Koka, Effekt, Unison, Roc, Flix | Recent attempts at "the best of both worlds"; what they bet on and how it is going. |

## Evaluation dimensions

Every language analysis covers the same dimensions so they can be compared.
If a dimension is irrelevant for a language, say so in one line rather than
skipping it.

1. **Syntax and readability** - consistency, noise, learnability, how it reads after a year.
2. **Type system** - static/dynamic, strength, inference, ADTs, generics, nullability, soundness.
3. **Memory management and safety** - GC, RC, ownership, manual; what classes of bugs are prevented.
4. **Mutability, aliasing and value semantics** - defaults, how shared mutable state is controlled.
5. **Error handling** - exceptions, result values, error unions, panics; visibility and ergonomics.
6. **Concurrency and parallelism** - threads, async, actors, structured concurrency, data-race freedom, function colouring.
7. **Polymorphism** - generics, traits/interfaces/typeclasses, inheritance, dynamic dispatch.
8. **Metaprogramming** - macros, compile-time execution, reflection, code generation.
9. **Modules, packages, build** - module system, dependency management, reproducibility, supply-chain story.
10. **Tooling** - compiler speed, LSP, formatter, test runner, debugger, documentation.
11. **Interop and platforms** - FFI, embedding, target platforms, ABI stability.
12. **Performance and compilation model** - AOT/JIT/interpreted, predictability, zero-cost claims.
13. **Standard library and ecosystem** - batteries included vs. minimal; quality of the core.
14. **Learnability** - time to productivity, common beginner traps, teaching experience.
15. **Evolution and governance** - how breaking changes are handled, who decides, trust.

## Evidence standards

Claims about strengths and weaknesses must be backed. Rank evidence roughly as:

1. **Designer retrospectives** - what the designers themselves say they got wrong or would change (strongest: they have no incentive to invent problems).
2. **Fixes that shipped or were proposed** - an RFC, PEP, JEP, or language version that changes a feature is an admission the original was a problem. Note the proposal's name or number.
3. **Empirical studies and industry data** - memory-safety statistics from Microsoft, Google, Mozilla; academic studies on defect rates; productivity studies.
4. **Developer surveys** - Stack Overflow, JetBrains, the language's own annual survey; note the question wording and year.
5. **Adoption and imitation** - a feature copied by several later languages is evidence that it works; a feature no later language copied is weak evidence it does not.
6. **Widely cited critiques and well-known post-mortems** - only when concrete (a specific bug class, a specific incident).

Anecdote and personal taste are allowed only when labelled as such.

Research was conducted in October 2026. Where a language's state changed
recently (for example a major release), say what version the observation
applies to.

## Template for `languages/<name>.md`

```markdown
# <Language>

## Snapshot
| | |
|---|---|
| First appeared | |
| Designed by | |
| Current version (Oct 2026) | |
| Governance | |
| Typing | |
| Memory | |
| Compilation | |
| Primary domains | |
| Notable users | |

## Stated design goals
What the designers said they were optimising for, in their own words where
possible, and what niche the language was built to fill.

## Strengths
One subsection per strength. For each: the concrete feature, why it is good,
and the evidence (see evidence standards). Prefer 5-12 well-argued items over
a long list.

## Weaknesses
One subsection per weakness. Same structure. Include the weaknesses the
designers themselves acknowledge, and the fixes attempted.

## Designer retrospectives
What the people who built the language say they would do differently.
Quote or paraphrase with a source.

## Influence
What other languages copied from this one (evidence the idea works), and
what this language copied from others.

## Lessons for Sisu
- **Adopt**: features to take as-is, with the reason.
- **Adapt**: ideas that are right in spirit but need changes, and what change.
- **Avoid**: mistakes not to repeat, and the reason.
- **Open questions raised**: where this language shows the trade-off is real and unresolved.

## Sources
Numbered list. Prefer primary sources: official docs, RFCs, talks and posts by
designers, survey reports, papers. Include the URL and the year.
```

Length guidance: 250-500 lines for a major language, 100-250 for a minor one.
Depth beats breadth: a weakness described with the specific feature, the
specific failure, and the specific fix attempted is worth ten vague ones.
