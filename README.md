# Sisu

*Sisu* (Finnish: grit, stubborn perseverance) is a proposal for a
general-purpose programming language that tries to keep only the upsides of
the languages that came before it. This repository currently contains the
**research** behind the proposal, the **proposal** itself, and the list of
**open questions** that the research could not settle.

There is no compiler yet; the first code is the stack harness in
`taskbench/`, which tests the riskiest runtime bet before the language
exists. The point of this phase is to make the design decisions on
evidence rather than taste, and to be honest about where the evidence runs
out.

## Where to start

| If you want to... | Read |
|---|---|
| Understand what Sisu is in 10 minutes | [`proposal/README.md`](proposal/README.md) |
| See code | [`proposal/01-tour.md`](proposal/01-tour.md) and [`proposal/examples/`](proposal/examples/) |
| Know what happens next | [`proposal/18-next-steps.md`](proposal/18-next-steps.md), then [`proposal/20-pre-registration.md`](proposal/20-pre-registration.md) |
| Know *why* a decision was made | [`proposal/15-decision-log.md`](proposal/15-decision-log.md), then the linked research |
| Argue about something | [`open-questions.md`](open-questions.md) |
| Check the evidence | [`research/evidence/`](research/evidence/) |
| Read what we learned from a specific language | [`research/languages/`](research/languages/) |
| Compare approaches to one problem (memory, errors, concurrency, ...) | [`research/topics/`](research/topics/) |
| See what block languages, spreadsheets, live coding, logic languages and the rest have to teach | [`research/topics/unconventional-ideas.md`](research/topics/unconventional-ideas.md), then [`research/unconventional/`](research/unconventional/) |
| Judge the features no language has | [`proposal/19-novel-features.md`](proposal/19-novel-features.md) |

## How the material is organised

```
research/
  README.md            methodology, evaluation dimensions, evidence standards
  languages/           one analysis per language, same template for all
  topics/              cross-cutting syntheses, each ending in a recommendation
  evidence/            surveys, studies, memory-safety data, designer retrospectives
  unconventional/      second round: block, live, spreadsheet, synchronous, logic, config,
                       capability, esoteric, game and holes families, with transferable ideas
  ideation/            candidate features that exist in no language, through three lenses
proposal/
  README.md            one-page summary and index
  00-...14-*.md        the design, one document per area
  15-decision-log.md   every decision, the alternatives, and the evidence
  16-validation-plan   what a prototype must test first
  17-testing.md        the testing design
  18-next-steps.md     the experiment plan
  19-novel-features.md the unconventional round's verdicts: 66 candidates, decisions, questions
  20-pre-registration  the experiment's rules of evidence (a draft until the Phase 0 freeze)
  21-recruitment.md    roles, eligibility and the call for participants
  examples/            Sisu source files illustrating the design
taskbench/             Phase 1 of the experiment: the bet-3 stack harness (Rust)
compiler/              the compiler: stage 0 (the Rust interpreter and front end, the message catalog, SIR,
                       the conformance suite), stage 1 (compiler/stage1, the front end written in Sisu) and
                       stage 2 (the native backend and runtime, Rust)
open-questions.md      what the research could not settle, with options and a current lean
```

The chain of reasoning is always: **evidence → language analysis → topic
synthesis → design decision → open question (if unresolved)**. A decision that
cannot be traced back along that chain is a mistake in this repository and
should be filed as an issue.

## Status

| Phase | Status |
|---|---|
| Research | Done (October 2026). 47 language analyses (~14,000 lines), 20 topic syntheses, 4 evidence dossiers; a second round of 14 studies of unconventional languages and environments and 3 ideation studies (~6,500 lines). |
| Proposal | Draft 3 (October 2026). Indentation-based, Python-flavoured syntax; a testing design; a 52-week experiment plan with phases, corpus, measurements and decision points; the unconventional round's assessment of 66 candidate features, 14 of them found in no language ([`proposal/19-novel-features.md`](proposal/19-novel-features.md)), folded into the design; a pre-registration and a recruitment brief for the experiment. |
| Open questions | 53 recorded; 50 decided by the project owner (October 2026), 3 open: runtime stacks (Q-6), governance details (Q-21), numerics (Q-23). |
| Prototype | Phase 1: the bet-3 stack harness in [`taskbench/`](taskbench/) (Rust, about 10,000 lines) builds, passes its tests and has results against Go, Java and tokio on virtual machines and a GitHub runner; the memory primary holds provisionally through pooled stacks with probes and decommit on park ([report](taskbench/docs/report-phase1.md)). Stage 0: the interpreter and front end in [`compiler/`](compiler/) (Rust, about 90,000 lines in fifteen crates, plus a standard library of 6,400 lines written in Sisu) has the message catalog, the lossless syntax tree, the serialised SIR, per-item incremental queries, the ownership checker with its dynamic monitor and counterexample mode, traits with coherence, generics, comptime with derive and reflection, a seeded task scheduler with scopes, cancellation, channels and select, capabilities with attenuation, lending, revocation and deterministic fakes, an HTTP/1.1 client and server in Sisu, contracts and refinement types, a capability log with replay tokens, a simulation profile, parallel tests with snapshots, transcripts and property tests, packages with a manifest and lockfile, the formatter, watch mode with hot replacement, a REPL, worksheets and a language server with completion, rename, code actions and inlay hints, f32 and exact-width integers, arrays with constant expressions as type arguments; a conformance suite of about 400 cases and 234 tests pass. Stage 1: the front end written in Sisu in [`compiler/stage1/`](compiler/stage1/) (52,000 lines in 72 modules: lexer, parser, AST, name resolution, type checker with the template engine, lowering to SIR, the ownership checker, the comptime evaluator, packages) gives stage 0's tokens, trees, AST, names, type diagnostics, serialised SIR and ownership diagnostics byte for byte on every program of the corpus and on every module of its own package (3,412 files identical, no discrepancy outstanding; the five it found were decided for the documents and stage 0 changed, D-079), checked in CI on every push. Stage 2: the native backend in [`compiler/crates/sisu-codegen`](compiler/crates/sisu-codegen) (Cranelift, the debug backend of 10) and the runtime in [`compiler/crates/sisu-rt`](compiler/crates/sisu-rt) compile the SIR the interpreter runs: every runnable conformance case passes natively from source and from serialised SIR, tasks run on the pooled stacks Phase 1 chose, and stage 1 compiles natively unchanged in about 4 s (0.08 s per thousand lines, within 10's target) and runs twenty to sixty times faster than on the interpreter. The backend also writes DWARF line tables and typed locals for debuggers, builds packages into build/debug or build/release, and has a release profile (Cranelift's speed level with overflow, bounds and contract checks kept). Next: the fixed point of D-078 (stage 1 built from its own SIR producing the same SIR again) and stage 1's parity with the newest stage-0 rules. |

## Contributing to the design

Disagree with a decision? The useful contribution is **evidence**, not
preference: a retrospective from a language designer, a survey figure, a
study, or a concrete program that the proposed design handles badly.
Add it to `research/`, then argue from it in `open-questions.md`.
[`CONTRIBUTING.md`](CONTRIBUTING.md) has the conventions; every push runs
the documentation checker and the harness's build, tests and smoke run
([`.github/workflows/ci.yml`](.github/workflows/ci.yml)). The repository
is dual-licensed: [Apache-2.0](LICENSE) or the [Unlicense](UNLICENSE),
at your option.
