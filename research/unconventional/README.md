# Unconventional languages and environments

The first research round ([`../languages/`](../languages/)) covered the
languages people write software in. This round covers the ones they mostly
do not: visual and block languages, spreadsheets, synchronous and dataflow
languages, logic and constraint systems, live-coding and art languages,
game and narrative scripting, capability and resource languages, and the
synthesis and holes tradition. The purpose is different too. The first
round asked "what did this language get right or wrong"; this one asks
"what idea does this family have that no mainstream textual language has,
and would Sisu be better with it".

The evidence standards in [`../README.md`](../README.md) still apply.
Because these systems are rarely surveyed, the bar for "it works" is
adoption or measured outcomes in the niche where the system lives, and the
bar for "transferable" is a concrete sketch of what the idea looks like in
Sisu's syntax and what it would cost.

## Template for each file

```markdown
# <Family>

## Why this family matters for Sisu
Two or three sentences: what question about Sisu this family can answer.

## Systems covered
| System | Years | Status (Oct 2026) | Central idea |

## The ideas
One section per idea (not per system). For each:
- what it is, with a short example from the system that has it;
- what it demonstrably buys, with evidence (adoption in its niche,
  measured outcomes, designers' accounts);
- where it failed or stayed niche, with evidence;
- whether anyone has transplanted it into a textual general-purpose
  language, and how that went.

## Transferable ideas for Sisu
Ranked. For each: the idea; how it would look in Sisu (a sketch in Sisu's
indentation syntax or a tooling description); what it would cost; what
it conflicts with in the current proposal; confidence (high/medium/low).

## Dead ends
Ideas from this family not to pursue, with the reason.

## Sources
Numbered, with URLs and years.
```

## Files

| File | Family |
|---|---|
| `visual-and-structure-editing.md` | Block, node and projectional editors |
| `live-and-exploratory-environments.md` | Live programming, notebooks, trace-driven and moldable tools |
| `spreadsheets-and-end-user.md` | Spreadsheets, formula languages, end-user and gradual-learning languages |
| `natural-language-and-literate.md` | Natural-language-like and literate programming |
| `synchronous-and-dataflow.md` | Synchronous reactive languages and dataflow |
| `parallel-languages.md` | Structured parallelism, PGAS and data-parallel languages |
| `algorithm-schedule-separation.md` | Halide-style separation of algorithm from schedule |
| `logic-constraint-and-rewriting.md` | Logic, constraint, functional-logic and rewriting languages |
| `config-and-total-languages.md` | Total, hermetic and lattice-typed configuration languages |
| `capabilities-distribution-and-resources.md` | Object-capability, distributed and resource-typed languages |
| `incremental-reversible-and-exotic-paradigms.md` | Self-adjusting computation, reversible, probabilistic, differentiable, quantum |
| `esoteric-and-live-coding.md` | Esoteric, art and live-coding languages |
| `game-and-narrative-scripting.md` | Game engines' scripting and visual scripting, narrative languages |
| `holes-synthesis-and-ai-era.md` | Typed holes, program synthesis, programming by example, LLM-era tooling |

The synthesis is in [`../topics/unconventional-ideas.md`](../topics/unconventional-ideas.md)
and the assessment of features that exist in no language is in
[`../../proposal/19-novel-features.md`](../../proposal/19-novel-features.md).
