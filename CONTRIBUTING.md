# Contributing

Sisu is in its research and experiment phase: there is no compiler yet,
and the repository holds the research, the proposal, the open questions,
the experiment's pre-registration and the Phase 1 stack harness. The
contributions that help now are evidence and measurements, not
preferences.

## What to contribute

- **Evidence against a decision.** Every decision in
  [`proposal/15-decision-log.md`](proposal/15-decision-log.md) names the
  alternatives it rejected and the evidence it rests on. To reopen one,
  bring one of: a retrospective from a language designer, a survey figure,
  a study, or a concrete program the decided design handles badly and an
  alternative handles well. Add it under `research/`, then argue from it in
  [`open-questions.md`](open-questions.md).
- **A correction to a research file.** The analyses in `research/` cite
  their sources; a wrong number, a misattributed quotation or a missing
  precedent is a bug. Say which source you checked.
- **Harness work.** [`taskbench/`](taskbench/) is a Rust crate with its own
  README; a new benchmark or control must write the same CSV schema and
  record the machine specification. Measured runs that count for the
  experiment are governed by
  [`proposal/20-pre-registration.md`](proposal/20-pre-registration.md)
  once it is frozen; before that, every run is a smoke run.
- **Not now:** syntax proposals without evidence, a compiler, or a
  standard library. Principle 6 of the experiment plan keeps everything
  beyond the three bets waiting for the verdict.

## How changes are checked

Every push runs [`tools/check_docs.py`](tools/check_docs.py), which fails
on a broken relative link or a reference to a question or decision that
does not exist, and builds, tests and smoke-runs the harness
([`.github/workflows/ci.yml`](.github/workflows/ci.yml)). Run the checker
locally with:

```
python3 tools/check_docs.py
```

## Conventions

- Documents are Markdown, wrapped at about 72 columns, with relative
  links. Decisions are `D-nnn`, questions `Q-nn`, candidate features
  `N-nn`; a new one takes the next number and is never renumbered.
- Sisu code in documents follows
  [`proposal/02-syntax.md`](proposal/02-syntax.md).
- Commit messages say what changed and why in plain sentences.
- Code and documents do not name AI models or tools; the work is judged
  on its evidence.

## Licence

Contributions are accepted under the repository's licence
([Apache-2.0](LICENSE)); submitting a change means you agree to that.
