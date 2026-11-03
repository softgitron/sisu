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
manual/                the programming manual: tutorial, guides, reference and explanations, with checked
                       examples and exercises (manual/README.md is the plan)
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
| Proposal | Draft 3 (October 2026). Indentation-based, Python-flavoured syntax; a testing design; a 52-week experiment plan with phases, corpus, measurements and decision points; the unconventional round's assessment of 66 candidate features, 14 of them found in no language ([`proposal/19-novel-features.md`](proposal/19-novel-features.md)), folded into the design; a pre-registration and a recruitment brief for the experiment. November 2026: a chapter on `unsafe` ([`proposal/22-unsafe.md`](proposal/22-unsafe.md), D-080 to D-086) so that the runtime can be written in Sisu, a backend decision (a debug backend written in Sisu, LLVM through bitcode for release builds, D-087 and D-088) from [`research/topics/backends.md`](research/topics/backends.md), and the plan of the programming manual ([`manual/README.md`](manual/README.md), 96 pages in tutorial, guides, reference and explanations, every example and exercise checked in CI) from [`research/topics/manuals.md`](research/topics/manuals.md), with its first chapter written. |
| Open questions | 75 recorded; 50 decided by the project owner (October 2026); 25 open: runtime stacks (Q-6), governance details (Q-21), numerics (Q-23), and the twenty-two questions of the November rounds on `unsafe` (Q-54 to Q-57, Q-71, Q-72), the backend (Q-58 to Q-61) and the manual (Q-62 to Q-70, Q-73 to Q-75). |
| Prototype | Phase 1: the bet-3 stack harness in [`taskbench/`](taskbench/) (Rust, about 10,000 lines) builds, passes its tests and has results against Go, Java and tokio on virtual machines and two GitHub runners; the memory primary holds provisionally through pooled stacks with probes and decommit on park: at the 1 MiB default reservation, idle tasks, a million included, stay within 2.42 times Go's memory per task (the bar is 4) on both machines with 16 GiB ([report](taskbench/docs/report-phase1.md)). Stage 0: the interpreter and front end in [`compiler/`](compiler/) (Rust, about 98,000 lines in fifteen crates, plus a standard library of 11,500 lines written in Sisu) has the message catalog, the lossless syntax tree, the serialised SIR, per-item incremental queries, the ownership checker with its dynamic monitor and counterexample mode, traits with coherence, generics, comptime with derive and reflection in every place 08 allows, conditional compilation with features, a seeded task scheduler with scopes, cancellation, channels and select, Mutex, RwLock, Atomic, Once and Condvar, capabilities with attenuation, lending, revocation, deterministic fakes and an audit, an HTTP/1.1 client and server in Sisu, contracts and refinement types, a capability log with replay tokens, a simulation profile, parallel tests with snapshots, transcripts, property tests and a cache, packages with a manifest and lockfile, the formatter, `sisu doc`, watch mode with hot replacement, a REPL, worksheets and a language server, f32 and exact-width integers, arrays with constant expressions as type arguments, and the standard library's modules of 11; a conformance suite of 544 cases (including a wrong-program corpus of 93 and thirteen shapes from rustc's borrow-checker tests) and 275 tests pass; architecture decisions 1 to 388 are recorded in [`compiler/docs/architecture.md`](compiler/docs/architecture.md). Stage 1: the front end written in Sisu in [`compiler/stage1/`](compiler/stage1/) (66,000 lines: lexer, parser, AST, name resolution, type checker with the template engine, lowering to SIR, the ownership checker, the comptime evaluator, packages, the diagnostic renderer, the formatter and `sisu doc`, every part of the plan's stage-1 table) is the front end of `sisu` (D-089): it gives stage 0's output byte for byte on every program of the corpus, on the standard library and on every module of its own package except where its own rules decide otherwise (12,145 files checked, 11,416 identical, every one of the 729 that differ listed under a rule of its [RULES.md](compiler/stage1/RULES.md), R1 to R36, decisions 269 to 349), checked in CI on every push with 144 tests of its own. Stage 2: the backend is written in Sisu ([`compiler/stage2/`](compiler/stage2/)): a lowering of SIR to a backend-neutral low IR and an x86-64 Linux debug backend with DWARF line tables, the default for debug builds since it compiles itself and stage 1 to byte-identical executables (its own fixed point, [`compiler/stage2/bootstrap.sh`](compiler/stage2/bootstrap.sh)); the Rust Cranelift path stays as the oracle and for release builds, and every runnable conformance case passes natively through both; the interpreter runs unsafe code in a checked memory model, `sisu test` runs a program's tests from stage 1's SIR, unsafe code's included, and exits with 3 when none failed and one is blocked by a typed hole, and the runtime in Sisu ([`compiler/rt/`](compiler/rt/)) holds the structural operations on values, text building, integers' digits, text search, floats' text, `Display` and `Debug` by the type table and lists over raw memory, which the Sisu backend compiles in its fixed point too. Stage 3's fixed point holds for the front end (D-078): stage 1 built by stage 0 prints the SIR of its own package, the backend builds a second stage 1 from that SIR alone, and the second prints the same SIR again, byte for byte, with identical executables ([`compiler/stage1/bootstrap.sh`](compiler/stage1/bootstrap.sh)); the backend and the runtime stay in Rust, as the plan expects. The programming manual ([`manual/`](manual/)): the tutorial's 28 chapters, 28 of 32 guides, the 22 reference pages and all 14 explanations are written (four guides and one reference page wait for their features), with 119 exercises and about 1,300 checked source files, every example and solution a passing conformance case in CI, every link and anchor checked, and the pages' findings about the compiler kept as one list of 209 items for the next rounds. What remains is listed in [`compiler/docs/port-debt.md`](compiler/docs/port-debt.md) and in stage 1's README ("What is next"). |

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
