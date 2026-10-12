# Sisu

*Sisu* (Finnish: grit, stubborn perseverance) is a proposal for a
general-purpose programming language that tries to keep only the upsides of
the languages that came before it. This repository currently contains the
**research** behind the proposal, the **proposal** itself, and the list of
**open questions** that the research could not settle.

There is no compiler yet. The point of this phase is to make the design
decisions on evidence rather than taste, and to be honest about where the
evidence runs out.

## Where to start

| If you want to... | Read |
|---|---|
| Understand what Sisu is in 10 minutes | [`proposal/README.md`](proposal/README.md) |
| See code | [`proposal/01-tour.md`](proposal/01-tour.md) and [`proposal/examples/`](proposal/examples/) |
| Know what happens next | [`proposal/18-next-steps.md`](proposal/18-next-steps.md) |
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
  examples/            Sisu source files illustrating the design
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
| Proposal | Draft 2 (October 2026). Indentation-based, Python-flavoured syntax; a testing design; an experiment plan with phases, corpus, measurements and decision points; an assessment of 66 candidate features from the unconventional round, 14 of them found in no language ([`proposal/19-novel-features.md`](proposal/19-novel-features.md)). |
| Open questions | 52 recorded; 38 decided by the project owner (October 2026), 3 open from the first round (runtime stacks, governance details, numerics), 11 from the unconventional round awaiting the owner. |
| Prototype | Not started. |

## Contributing to the design

Disagree with a decision? The useful contribution is **evidence**, not
preference: a retrospective from a language designer, a survey figure, a
study, or a concrete program that the proposed design handles badly.
Add it to `research/`, then argue from it in `open-questions.md`.
