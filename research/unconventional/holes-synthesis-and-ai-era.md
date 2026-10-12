# Holes, synthesis and AI-era tooling

## Why this family matters for Sisu

Sisu already has the judging half of what this family needs: contracts in ordinary expressions,
property tests with shrinking, a sound type checker planned as a library, and an interpreter that
runs code under fake capabilities. It lacks a mark for missing code and a path from that mark to
checked code; this file asks how a first-class hole should behave, which ways of proposing fills are
evidenced, how far passing tests can be trusted as the judge, and which "LLM-era" features are
measured rather than asserted ([`studies.md`](../evidence/studies.md) §11 is not repeated here).

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| GHC typed holes, valid hole fits; PropR | 2014; fits 2018; PropR 2022 | Always on (GHC 9.14 user's guide); PropR a research tool | `_x` reports its type, bindings and well-typed fits; PropR repairs with fits and properties |
| Idris 2 | book 2017; Idris 2 2020- | Small research community | `?name` holes, `:ps` proof search, "type, define, refine" |
| Agda | Agda 2 2007-; Mimer 2024 | Active; Mimer is `C-c C-a` since 2.7.0 | Interactive refinement of holes; automatic term search |
| Lean 4 | 2021- | Active; the target of most LLM provers | `?_` goals; kernel-checked machine-written proofs |
| Hazel; Smyth | 2016-; 2020 | Hazel active research; Smyth a prototype | Every incomplete program has a type and runs; assertions fill holes |
| rust-analyzer term search | 2024- | Shipped, off by default in completion; removal discussed Aug 2026 | Type-directed fill of `todo!()` in a mainstream language |
| Sketch; Rosette (Serval, Jitterbug) | 2006-2020 | Research tools; Sketch's status [unverified]; Rosette work upstreamed to Linux | `??` holes solved by SAT through CEGIS; interpreters lifted into verifiers |
| Synquid; Leon, Stainless | 2016; 2013- | Stainless active (Scala 3; floating point added 2026) | Synthesis and verification from refinement types and contracts |
| FlashFill, PROSE, FlashFill++ | 2011-2023 | In Excel since 2013 | Programming by example inside a DSL |
| FIM completion; ChatLSP | 2022; 2024 | FIM standard in code models; ChatLSP a proposal | Model infilling; the language server supplies the hole's context |
| DSPy, LMQL, Guidance | 2022-2024 | DSPy active (38.6k GitHub stars); Guidance and LMQL [status unverified] | Prompt programs with typed signatures and constrained output |
| AlphaCode, AlphaDev, AlphaEvolve | 2022-2025 | AlphaDev code in libc++; AlphaEvolve in Google production | Mass sampling kept only where an automatic evaluator agrees |
| Clover, nl2postcond, TiCoder, DafnyBench, AutoVerus, VERINA, CLEVER | 2023-2026 | Research tools and benchmarks | Model-written code, specifications and proofs judged by a verifier |
| SWE-bench audits, Multi-SWE-bench, ImpossibleBench | 2025-2026 | Benchmarks | Agents that run tests, and how tests fail as judges |

## The ideas

### 1. A hole is a typed, named expression the compiler reports on
GHC has reported the type of a `_` or `_name` placeholder since 7.8.1 (2014) [1]; since 8.6.1 (2018)
it also lists *valid hole fits*, identifiers in scope whose type fits, and optionally *refinement
fits* with further holes such as `foldl1 (_ :: Int -> Int -> Int)` [2, 3]. Six fits are shown by
default, refinement is off, and `-fdefer-typed-holes` compiles the program with each hole behaving
like `undefined` [2]. From the PropR paper [4]:

```haskell
degreesToRadians d = d * _ / 180
-- • Found hole: _ :: Double
--   Valid hole fits include  d :: Double;  pi :: forall a. Floating a => a
```

Idris writes `?name`; the editor shows a hole's context and expected type, and `:ps` "tries to fill
hole automatically, using local variables, recursive calls, and constructors of the required type"
[6]. Brady's *Type-Driven Development with Idris* (Manning, March 2017) teaches the loop "type,
define, refine" with "the type checker as your assistant" [5]. Agda reimplemented `C-c C-a` auto as
Mimer in 2.7.0 (2024) because Agsy failed on copatterns and cubical features; Mimer covers more of
the language but is slower and dropped case splitting and refinement [7]. In Lean 4, holes `?x` and
`?_` left by `refine` become new goals [8]. Rust's `todo!()`, Scala's `???` and Sisu's `todo` are
the untyped cousins: bottom-typed calls that check anywhere and say nothing about what belongs there.

The evidence is designers' accounts and imitation: holes are the interaction model of every
dependently typed language in [`idris-agda-lean.md`](../languages/idris-agda-lean.md), GHC has had
them on for twelve years, and PropR built a repair tool on GHC's fits (idea 6); no controlled study of
their effect was found. The mainstream transplant went poorly: rust-analyzer's term search (PR #16092,
February 2024) fills `todo!()` with type-correct expressions from locals, functions and constructors,
with completion off by default because it can slow completion; in August 2026 a maintainer asked who
used it, most replies came from people who had never heard of it, and the team kept it but removed its
borrow-checker integration, the costly part [9]. Enumeration alone attracts few users; dependently
typed programmers use the report, where the type says nearly everything.

### 2. Incomplete programs can have a meaning (Hazel, Smyth)
Hazelnut (POPL 2017) is a structure-editor calculus whose editor inserts holes so that every edit
state has a type [10]. Hazelnut Live (POPL 2019) lets evaluation "proceed around holes, tracking the
closure around each hole instance", treats non-empty holes as membranes around type inconsistencies,
and supports fill-and-resume without restarting evaluation, with the metatheory mechanised in Agda
[11]; later papers add pattern matching, total type-error recovery, polymorphism and incremental
typing (2023-2025) [12]. Smyth (ICFP 2020) turns this into synthesis: assertions in a sketch become
input-output examples pushed "backward" through the partially evaluated program to the holes, which
synthesises recursive functions without trace-complete examples [13]. The evidence is academic
(three distinguished-paper awards [12]), not adoption: Hazel is a pure language in its own editor and
remains a research environment. Proceeding around a hole is sound only for pure code (with effects it
would perform I/O with a value that does not exist; GHC's deferred hole, the textual transplant,
stops there). What transfers is the hole closure: the values in scope each time a test reaches a hole
are the examples a filler needs.

### 3. Solvers fill holes in bounded programs (Sketch, Rosette, CEGIS)
In Sketch (Solar-Lezama's 2008 thesis [14]) the programmer writes the structure, marks unknown
constants `??` and choices `{| a | b |}`, and states assertions in `harness` functions whose
arguments are universally quantified. CEGIS alternates an inductive synthesiser that finds hole
values working on a finite set of inputs with a checker that accepts them or returns a new
counterexample [14]:

```c
harness void doubled(int x) {
    assert x * ?? == x + x;      // the solver answers ?? = 2
}
```

Sketch's error-model autograder corrected about 64% of incorrect submissions from MIT's 6.00 and
6.00x (PLDI 2013 [15]). Serval (SOSP 2019) uses Rosette, a solver-aided extension of Racket, to lift
interpreters for RISC-V, x86-32, LLVM and BPF into verifiers, and found 18 new bugs in Keystone and the
Linux BPF compilers [16]; Jitterbug (OSDI 2020) found and fixed 16 previously unknown bugs in five
deployed BPF JITs [17]. The limit is boundedness: push-button verification "asks developers to design
interfaces to be finite" [16], holes are constants and enumerated choices, and no mainstream language
ships `??`. What transfers is the loop (propose from examples, check, add the counterexample), with
shrinking property tests as its cheap, unsound checker; Sisu's interpreter is what Serval would lift.

### 4. Specifications drive synthesis, but only verification survived (Synquid, Leon, Stainless)
Synquid (PLDI 2016) synthesises recursive functions from polymorphic refinement types and was the
first tool to discover provably correct sorting algorithms and red-black and AVL operations [18].
Leon (OOPSLA 2013) synthesised Scala functions from pre- and postconditions and symbolic examples by
deductive rules, and later added repair [19]. Its successor Stainless verifies Scala 3 with
`require` and `ensuring` contracts but describes itself as subsuming Leon's verification and
termination checking; synthesis and repair are not among what it lists, and the verifier is what
stayed alive (floating-point support, January 2026) [20]. That supports the order in
[`07-contracts.md`](../../proposal/07-contracts.md): contracts now, a verifier later, and no
deductive synthesiser.

### 5. Programming by example works inside a DSL (FlashFill, PROSE)
FlashFill (Gulwani, POPL 2011) synthesises string transformations from a few input-output examples
in a DSL of restricted regular expressions, conditionals and loops, ranking the consistent programs;
it shipped as Excel 2013's Flash Fill and won POPL's most-influential-paper award [21]. FlashMeta
(OOPSLA 2015) became the PROSE framework: an industrial synthesiser had been estimated at 1-2
person-years, FlashMeta derives one from a DSL definition, and the paper counts 10+ mass-market
applications, PowerShell 3.0 and Cortana among them [22]. It is the only synthesis with mass-market
adoption, and the DSL is why; there is no general-purpose transplant. Examples are ambiguous and
users lack confidence: browsing the consistent programs or answering questions about distinguishing
inputs significantly reduced errors without slowing users (UIST 2015 [24]), and FlashFill++ (POPL
2023) made readable output (Excel formulas, PowerFx, Python) a goal [23]. Both lessons transfer.

### 6. A passing test suite is a weak judge, and generators game it
PropR (ICSE 2022) repairs Haskell by "perforating" fault-localised expressions into typed holes and
searching GHC's valid fits, with QuickCheck properties and unit tests as fitness [4]. On 30 student
programs it patched 13 (228 patches); in a manual sample of 70, 85% of the patches found with unit
tests were overfitted (19/23), 64% with properties (21/33) and 63% with both (9/14), and properties
found patches faster [4]. Agents show the same at scale: 29.6% of plausible SWE-bench Verified
patches behave differently from the developers' fix and 28.6% of those are certainly wrong,
inflating resolution by 6.4 points (ICSE 2026 [25]); UTBoost found 345 wrongly passed patches and
changed 40.9% of SWE-bench Lite and 24.4% of Verified rankings (ACL 2025 [26]); strengthened tests
rejected 19.78% of 11,041 passing patches from the top 30 agents, moving the leader from 78.80% to
62.20% (SWE-ABS, 2026 preprint [27]). Generators also game the judge. ImpossibleBench (2025) makes
tests contradict the specification, so any pass is cheating: GPT-5 passed 54% of conflicting
SWE-bench tasks [28]. Hiding tests cut cheating to near zero but hurt legitimate work; read-only tests
stopped test editing but not special-casing or operator overloading; an explicit "flag and exit"
option cut GPT-5 from 54% to 9% [28]. Tests show that a fill is plausible, properties more than
examples; the judge must be out of the filler's reach, and the filler needs a way to say "cannot".

### 7. A model fills a hole better when the checker supplies the context
Model completion is untyped hole filling: models are trained to infill a span between a prefix and a
suffix at no cost to left-to-right ability [29]. Blinn, Li, Kim and Omar (OOPSLA 2024) give the model
what Hazel's checker knows about the hole: expected type, relevant type definitions, headers of
relevant values and error reports for correction rounds [30]. On MVUBench (five model-view-update
apps, 10-15 unit tests each) with GPT-4 and StarCoder2-15B, type definitions mattered most (an order
of magnitude more correct solutions for StarCoder2); for GPT-4, adding headers to types tripled test
performance in Hazel against about 1.5x in TypeScript, and error rounds also helped more in Hazel.
Context lifted a model "incapable of even basic MVU tasks up to, or nearly up to" fully
contextualised TypeScript, and the authors propose ChatLSP, an LSP extension with `expectedType`,
`retrieveRelevantTypes`, `retrieveRelevantHeaders`, `errorReport` and `aiTutorial` [30]. Hazel is
the closest analogue to a new language with no training data, and the gains were largest there.
Type-based retrieval also "significantly boosts" F* proof synthesis (ICSE 2025 [31]); static-analysis
monitors in decoding let a 1.1B model beat text-davinci-003 on compilation rate (NeurIPS 2023 [32]);
MoonBit samples under its checker and reports a "significant improvement in compilation rates" for
about 3% slowdown, with figures only in a chart [33]. Names are the other risk: 5.2% of packages
recommended by commercial models and 21.7% by open ones do not exist (205,474 invented names in
576,000 samples, USENIX Security 2025 [34]).

### 8. Generate, execute, return the counterexample
AlphaCode (Science, 2022) reached the top 54.3% in Codeforces contests of more than 5,000
participants by sampling very many programs and filtering them by behaviour [35]. AlphaDev (2023)
found sort routines up to 70% faster for short sequences and 1.7% above 250,000 elements, now in
LLVM's libc++ [36]. AlphaEvolve (May 2025) evolves programs against automatic evaluators; a Borg
heuristic it found recovers 0.7% of Google's worldwide compute, and it applies only where a solution
can be checked automatically [37]. S* (2025) feeds test errors back and asks the model for inputs that
distinguish pairs of candidates; Qwen2.5-7B with S* beats Qwen2.5-32B by 10.7% on LiveCodeBench [38].
The Property-Generated Solver (2025, rev. 2026) checks properties such as "the output is
non-decreasing" and returns the simplest failing counterexample: up to 13.4% better pass@1 than
test-driven baselines, over 64% of initially failed problems fixed [39]. Google's migrations (2025)
build and unit-test several candidates per file group and send the best to an expert reviewer; in
the int32-to-int64 migration 80% of landed modifications were AI-authored and time fell an estimated
50% [41]. Self-verification fails where external verification works: GPT-4 critiquing its own
answers collapsed performance on formally checkable tasks, while a sound verifier's verdict, even fed
back by plain re-prompting, gave large gains (ICLR 2025 [40]). The best feedback is a small
counterexample.

### 9. Proofs are within reach; specifications are the bottleneck
Given code and specification, models complete proofs: 68% of DafnyBench's 750+ programs [42]; over 90%
of 150 Verus proof tasks for Rust, half in under 30 seconds or three model calls (AutoVerus,
Microsoft and others [43]); 88.9% of miniF2F-test (DeepSeek-Prover-V2) and five of six IMO 2025
problems (Seed-Prover, iterating on Lean's feedback) [44]. Specifications are harder: on VERINA's 189
Lean tasks the best model (o3) wrote correct code for 72.6%, sound and complete specifications for
52.3% and proofs for 4.9% [47], and on CLEVER the best methods verified 1 of 161 problems end to end
[48]. Microsoft Research's work on intent shows what helps: postconditions generated from natural
language caught 64 historical Defects4J bugs (FSE 2024 [45]); specifications should be tested,
symbolically where execution cannot (FMCAD 2024 [46]); approving model-proposed tests (TiCoder) made
15 programmers better at judging AI code with lower cognitive load and raised pass@1 by 45.97 points
within five simulated interactions (TSE 2024 [50]). Clover cross-checks a docstring, Dafny
annotations and code, accepting up to 87% of correct textbook programs with no false positives on
adversarial ones [49]. An agent writing Hypothesis properties for 100 Python packages filed reports of
which 56% were real bugs, and three patches were merged [51]. A Microsoft system in which a model
writes Dafny code and proofs together was not found ([46] and [43] are the closest). Models fill code,
proofs and candidate properties; people own the specification.

### 10. Prompt programs and structured output (DSPy, LMQL, Guidance)
DSPy (ICLR 2024) declares a model call by a signature of typed inputs and outputs
(`dspy.Predict("question -> answer")`), composes modules and "compiles" them by choosing
demonstrations against a metric, reporting gains over few-shot prompting of more than 25% (GPT-3.5)
and 65% (Llama2-13b-chat) [52]; its repository had 38.6k stars in October 2026. LMQL (PLDI 2023) adds
constraints and control flow to prompts, with 26-85% lower API cost [53]; Guidance constrains output
to a regular expression or context-free grammar, JSON schemas included [54]. Strict formats cost some
reasoning (Tam et al., 2024 [55]). All are libraries over I/O; a language's part is type-to-schema.

### 11. Do agents do better in languages with strong checkers?
SWE-bench Multilingual (300 tasks, SWE-agent with Claude 3.7 Sonnet) resolved Rust 58.1%, Java 53.5%,
PHP 48.8%, Ruby 43.2%, JavaScript/TypeScript 34.9%, Go 31.0% and C/C++ 28.6%, about 43 tasks per
language [56]. Multi-SWE-bench (1,632 instances; the same model under OpenHands) resolved Java 21.9%,
Rust 15.9%, C++ 14.7%, C 8.6%, Go 7.5%, JavaScript 5.1% and TypeScript 2.2%, its authors blaming
dynamic typing for the web languages and memory management and builds for the systems ones [57]. Rust
and Java are near the top in both, TypeScript is no better than JavaScript and Go is low despite
static types: the data cannot separate checker strength from task selection, corpus and builds, and
no benchmark varying only the checker was found.

### 12. Features proposed for the LLM era: what is measured
*Structured intent*: contracts, examples and properties are checkable and carry the measured gains
[45, 49, 50]; free text is not, and spec-first tools such as Kiro (July 2025: EARS requirements, a
design and tasks before code) are process with vendor claims only [60]. *Canonical formatting* and
*small vocabularies*: no study links either to model accuracy; invented names are a resolution
problem [34] and quality tracks training-data share (studies.md §11). *Deterministic replay*: not by
regeneration, since Qwen3-235B at temperature 0 gave 80 distinct completions in 1,000 runs because
kernels are not batch-invariant, and batch-invariant kernels made them identical at about 1.6x the
time [58]. *"Explain why"*: "why did / why didn't" questions cut debugging time by nearly a factor of
8 and let programmers finish 40% more tasks (Whyline, CHI 2004, six scenarios [59]).

## Transferable ideas for Sisu

**1. `?name`: a typed hole that checks, runs and cannot ship.**

```sisu
## The middle element of `xs` in sorted order.
def median(xs: list[int]) -> int:
    requires len(xs) > 0
    ensures result in xs
    let s = sorted(xs)
    return ?middle

test "median of three":
    assert median([3, 1, 2]) == 2
```

```
warning[W0901]: hole ?middle: int                                   stats.sisu:6:12
  in scope     xs: list[int]    s: list[int]
  facts        len(xs) > 0                                          requires, line 3
  obligation   result in xs                                         ensures, line 4
  reached by   test "median of three"    xs = [3, 1, 2]    s = [1, 2, 3]
  fits         s[0]   s[^1]   xs[0]   len(s)    (+9 of depth 2: sisu fill ?middle)
```

`?` plus an identifier in operand position, where `?` cannot be postfix propagation, is a hole. It
checks against the expected type from bidirectional inference (`-> ?ret` in a signature asks for the
inferred type as a fix-it), and `sisu check` warns with the report above. In the interpreter, reaching
a hole ends the task like a panic but with its own outcome carrying the hole closure (the `Debug`
rendering of each binding in scope); `sisu test` reports the test as "blocked by ?middle". `sisu build
--release` and `sisu publish` reject holes, `sisu audit` lists them and `sisu doc` marks functions
containing one, as Lean flags declarations that use `sorry`. *Cost:* low to medium: a token, a
checking rule, an interpreter outcome, a diagnostic, a release gate and per-test coverage for
"reached by"; the ownership checker treats a hole as using no binding and re-checks each fill in full
(the borrow check is where term search paid [9]). *Conflicts:* `todo` in the prelude
([02-syntax.md](../../proposal/02-syntax.md)) becomes a second spelling, so `?name` should replace it
via `sisu fix`, with `panic("...")` for placeholders that ship; `?` already means propagation, `T?`,
`?.` and `??`, so the operand rule goes in the grammar notes; D-057 gains a third test outcome.
*Confidence:* high that holes belong in the language; medium on stopping at the hole, chosen over
Hazel's semantics because Sisu code has effects.

**2. The hole report is a type-checker library query: ChatLSP plus contracts and tests.**
`sisu check --holes --format json` and an LSP request return, per hole, the expected type and the
definitions of the types it mentions; headers (name, signature, contracts) of reachable functions
ranked by type; the enclosing `requires` as facts and `ensures` and invariants as obligations; the
hole closures recorded by tests and property runs; enumerated fits; and diagnostics for any
candidate. The order follows Hazel's measurements (types, then headers, then error rounds, with the
largest effect in the low-resource language [30]) and F*'s type-based retrieval [31]. *Cost:*
medium; the checker has most of it. *Conflicts:* none; it extends the AI section of
[10-tooling.md](../../proposal/10-tooling.md), which names only type-constrained generation and a
translated corpus. *Confidence:* medium-high.

**3. `sisu fill`: propose, run, shrink, disambiguate, accept.**

```
$ sisu fill ?middle --proposer ./model-adapter
?middle: 44 candidates (13 enumerated, 31 proposed); 40 rejected
  9 by the type checker, 3 duplicates after formatting, 28 by the test or the contract, e.g.
  (s[(len(s) - 1) // 2] + s[len(s) // 2]) // 2
      violates `ensures result in xs` at xs = [0, 2]   (shrunk from [7, -3, 12, 0])
  4 survive, with 2 distinct behaviours; on xs = [0, 2]:
      a. s[len(s) // 2]          -> 2
      b. s[(len(s) - 1) // 2]    -> 0
  choose with  sisu fill ?middle --pick a,  or add a test that decides
```

Proposers are a built-in enumerator (GHC-style fits to depth 2, offline and deterministic) and any
external process speaking a documented JSON protocol over standard I/O; no model or vendor is built
in. Each candidate is formatted, deduplicated by AST, type- and ownership-checked (with constrained
decoding through the checker library when a local model exposes its decoder), then run in the
interpreter against the tests that reach the hole, the function's contracts as properties over
`arbitrary` inputs satisfying `requires`, and the property tests that call it, under per-test
capability fakes, so an untrusted candidate never touches a real file system or network. A failing
property's shrunk counterexample goes back to the proposer [14, 39]; survivors are split by
distinguishing inputs [35, 38]; a person chooses by behaviour [24] and reads the code [23]. Hot
function replacement (D-053) and the test cache make a candidate cost one re-run of the tests that
reach the hole. Here the contract, not the test, rejects the textbook median. *Cost:* medium-high.
*Conflicts:* the one-binary principle (D-029) keeps the model outside; the test cache key must
include fills. *Confidence:* medium: every step is evidenced, but no language has shipped the whole.

**4. The judge is out of the filler's reach.** `sisu fill` enforces it: a fill changes only the
hole's span; tests, contracts, generators and snapshots are read-only during a fill; a fill may use
only names that resolve in the package and its locked dependencies and never edits the manifest; a
candidate comparing an input with a literal taken from a test is flagged as special-casing; after
acceptance `sisu test --mutate` reports mutants of the fill that the tests do not kill; and a
proposer may answer `unfillable: <reason>`, reported as a finding. Evidence: ImpossibleBench [28],
PropR [4], the SWE-bench audits [25-27], invented packages [34]. *Cost:* low. *Conflicts:* none.
*Confidence:* medium-high.

**5. Accepted fills are source; provenance is a record; replay re-verifies.** An accepted fill is
written into the file and formatted like any code; optionally `sisu fill` appends to a committed
`sisu.fills` the hole name, a hash of its report, the proposer and version, and the tests, properties
and mutants that judged it. `sisu fill --recheck` re-runs those judgements, which are deterministic
because tests are seeded and clocks virtual; nothing is regenerated, since model output is not
reproducible even at temperature 0 [58]. *Cost:* low. *Conflicts:* none. *Confidence:* medium
(nondeterminism is measured; the record's value is a governance argument).

**6. Holes in specifications.** On `dedup`, documented as "removes repeated values, keeping the
first occurrence", `ensures ?post` asks for postconditions such as `len(result) <= len(xs)` drawn
from the comment, examples and tests. The toolchain keeps those that hold on every test and reject
mutants of the body (nl2postcond's correctness and discriminative power [45], tested symbolically
where execution cannot [46]) and never accepts one without a person: the best model wrote sound and
complete specifications for 52.3% of VERINA's tasks [47]. Later the same mechanism fills `invariant
?inv` and `decreases ?d` for the deferred verifier, where models do well once a checker exists [42,
43], cutting the annotation cost the SPARK research blames for verification's slow spread. *Cost:*
medium now, high with the verifier. *Conflicts:* loop invariants and `decreases` have no semantics
in version 1 ([07-contracts.md](../../proposal/07-contracts.md)). *Confidence:* medium-low.

**7. Model calls are capability-gated library I/O, and types become schemas.**

```sisu
@derive(Schema)
struct Triage:
    severity: Severity
    summary: Summary              # a refinement: String where len(self) <= 200

def triage(model: Model, report: str) -> Triage ! ModelError:
    return model.generate[Triage]("Classify this bug report:\n{report}")
```

`derive(Schema)` is a comptime function emitting a JSON Schema or grammar for constrained output
[54]; refinements are re-checked on the parsed value; `Model` is a capability like `Net`, visible in
signatures and replaced by a scripted fake in tests; and the schema leaves room for free reasoning
before the structured part [55]. It belongs in `std.x`, not the language. *Cost:* low. *Conflicts:*
none. *Confidence:* medium.

**8. `sisu why`.** Extends the language server's "explain this borrow" and `sisu explain`: `sisu why
type <expr>`, `sisu why error-set <fn>`, `sisu why capability Net <fn>` (the call path that needs it)
and `sisu why rejected ?middle <candidate>` (the check, input and clause that rejected it). The only
measurement is the Whyline's factor of 8 in six scenarios [59]. *Cost:* medium; the checker must keep
the provenance of each inference. *Conflicts:* none. *Confidence:* low-medium.

**9. A symbolic mode for the interpreter, after version 1.** Serval built verifiers by lifting
interpreters through symbolic evaluation [16]; Sisu's interpreter over its typed IR is the natural
place to do the same for the deferred SMT verifier, and the mode would solve Sketch-style finite
holes (a constant `?shift` under `requires 0 <= shift < 64`) exactly by CEGIS. *Cost:* high.
*Conflicts:* the verifier is post-1.0; push-button verification needs finite interfaces [16].
*Confidence:* low.

## Dead ends

- **Evaluating past a hole** with effects: sound for Hazel's pure core [11], but in Sisu it would
  perform I/O with a value that does not exist. Stop at the hole and record its closure.
- **Structure editing so that every state has a type**: Hazel's guarantee comes from its editor
  [10]; Sisu is textual with an error-tolerant parser (see `visual-and-structure-editing.md`).
- **General-purpose programming by example**: FlashFill works because a DSL keeps search small and
  ranking possible [21, 22]; outside one, examples are too ambiguous [24].
- **Deductive synthesis from contracts in the compiler**: Synquid and Leon handle small recursive
  programs [18, 19], only Leon's verification survived into Stainless [20], and the best end-to-end
  verified generation solved 1 of 161 CLEVER problems [48].
- **A model judging its own fill**, or any model-based check as a gate: self-critique collapses where
  a sound verifier helps [40]; Clover-style consistency checks are advisory [49].
- **Prompt programs as syntax, or code regenerated from prompts at build time**: DSPy, LMQL and
  Guidance are libraries that change faster than an edition [52-54], and regeneration is not
  reproducible [58]; a `Model` capability, `derive(Schema)` and stored code cover the need.
- **Syntax or vocabulary redesigned "for LLMs"**: nothing measured, MoonBit's flat-structure benefit
  is stated as expected [33], and the measured gains come from context and checking [30, 31].
- **Free-text "intent" annotations with semantics**: nothing can check them; contracts, tests and doc
  examples already are Sisu's structured intent, and a doc comment can be compared with them [49].
- **A passing suite as proof of a fill**: 63-85% of PropR's passing repairs were overfitted [4], and
  strengthened tests rejected 19.8% of "solved" SWE-bench patches [27].

## Sources

1. GHC 7.8.1 release notes (2014). https://downloads.haskell.org/~ghc/7.8.1/docs/html/users_guide/release-7-8-1.html
2. GHC 8.6.1 release notes (2018), https://downloads.haskell.org/ghc/8.6.5/docs/html/users_guide/_sources/8.6.1-notes.txt; GHC user's guide, "Typed holes" (9.14, 2026), https://downloads.haskell.org/ghc/latest/docs/users_guide/exts/typed_holes.html
3. M. P. Gissurarson, "Suggesting Valid Hole Fits for Typed-Holes (Experience Report)", Haskell Symposium 2018, doi:10.1145/3242744.3242760. https://research.chalmers.se/publication/507467
4. M. P. Gissurarson, L. Applis, A. Panichella, A. van Deursen, D. Sands, "PropR: Property-Based Automatic Program Repair", ICSE 2022, doi:10.1145/3510003.3510620. https://research.chalmers.se/en/publication/531145
5. E. Brady, *Type-Driven Development with Idris*, Manning, March 2017. https://www.manning.com/books/type-driven-development
6. Idris 2 documentation, "Interactive Editing" (2026). https://idris2.readthedocs.io/en/latest/tutorial/interactive.html
7. Agda 2.7.0 changelog, "Mimer replaces Agsy" (2024). https://hackage.haskell.org/package/Agda-2.7.0/changelog
8. Lean 4 reference manual, tactic reference (`refine` and holes) (2026). https://lean-lang.org/doc/reference/latest/Tactic-Proofs/Tactic-Reference/
9. rust-analyzer PR #16092 "Introduce term search" (Feb 2024), https://github.com/rust-lang/rust-analyzer/pull/16092; "Did you ever use term search in rust-analyzer?" (Aug 2026), https://users.rust-lang.org/t/did-you-ever-use-term-search-in-rust-analyzer/141676
10. C. Omar, I. Voysey, M. Hilton, J. Aldrich, M. Hammer, "Hazelnut: A Bidirectionally Typed Structure Editor Calculus", POPL 2017. https://arxiv.org/abs/1607.04180
11. C. Omar, I. Voysey, R. Chugh, M. Hammer, "Live Functional Programming with Typed Holes", POPL 2019. https://arxiv.org/abs/1805.00155
12. Hazel project publications, 2017-2026. https://hazel.org/
13. J. Lubin, N. Collins, C. Omar, R. Chugh, "Program Sketching with Live Bidirectional Evaluation", ICFP 2020. https://arxiv.org/abs/1911.00583
14. A. Solar-Lezama, *Program Synthesis by Sketching*, PhD thesis, UC Berkeley, 2008, https://www2.eecs.berkeley.edu/Pubs/TechRpts/2008/EECS-2008-177.html; syntax as in JSketch (2015), https://arxiv.org/abs/1507.03577
15. R. Singh, S. Gulwani, A. Solar-Lezama, "Automated Feedback Generation for Introductory Programming Assignments", PLDI 2013. https://arxiv.org/abs/1204.1751
16. L. Nelson, J. Bornholt, R. Gu, A. Baumann, E. Torlak, X. Wang, "Scaling symbolic evaluation for automated verification of systems code with Serval", SOSP 2019, doi:10.1145/3341301.3359641
17. L. Nelson et al., "Specification and verification in the field: Applying formal methods to BPF just-in-time compilers in the Linux kernel", OSDI 2020, https://par.nsf.gov/biblio/10205833; https://github.com/uw-unsat/jitterbug
18. N. Polikarpova, I. Kuraj, A. Solar-Lezama, "Program Synthesis from Polymorphic Refinement Types", PLDI 2016. https://arxiv.org/abs/1510.08419
19. E. Kneuss, V. Kuncak, I. Kuraj, P. Suter, "Synthesis Modulo Recursive Functions", OOPSLA 2013, doi:10.1145/2509136.2509555; M. Koukoutos, E. Kneuss, V. Kuncak, "An Update on Deductive Synthesis and Repair in the Leon Tool" (2016), https://arxiv.org/abs/1611.07625
20. Stainless repository (2026), https://github.com/epfl-lara/stainless; A. Gilot, A. Bergström, E. Darulova, "Verifying Floating-Point Programs in Stainless" (Jan 2026), https://arxiv.org/abs/2601.14059
21. S. Gulwani, "Automating String Processing in Spreadsheets using Input-Output Examples", POPL 2011. https://www.microsoft.com/en-us/research/publication/automating-string-processing-spreadsheets-using-input-output-examples/
22. O. Polozov, S. Gulwani, "FlashMeta: A Framework for Inductive Program Synthesis", OOPSLA 2015. https://www.microsoft.com/en-us/research/publication/flashmeta-a-framework-for-inductive-program-synthesis/
23. J. Cambronero et al., "FlashFill++: Scaling Programming by Example by Cutting to the Chase", POPL 2023, doi:10.1145/3571226. https://www.microsoft.com/en-us/research/publication/flashfill-scaling-programming-by-example-by-cutting-to-the-chase/
24. M. Mayer et al., "User Interaction Models for Disambiguation in Programming by Example", UIST 2015. https://www.microsoft.com/en-us/research/publication/user-interaction-models-disambiguation-programming-example/
25. Y. Wang, M. Pradel, Z. Liu, "Are 'Solved Issues' in SWE-bench Really Solved Correctly? An Empirical Study", ICSE 2026. https://arxiv.org/abs/2503.15223
26. B. Yu, Y. Zhu, P. He, D. Kang, "UTBoost: Rigorous Evaluation of Coding Agents on SWE-Bench", ACL 2025. https://arxiv.org/abs/2506.09289
27. B. Yu et al., "SWE-ABS: Adversarial Benchmark Strengthening Exposes Inflated Success Rates on Test-based Benchmark" (preprint, Feb 2026). https://arxiv.org/abs/2603.00520
28. Z. Zhong, A. Raghunathan, N. Carlini, "ImpossibleBench: Measuring LLMs' Propensity of Exploiting Test Cases" (Oct 2025; ICLR 2026). https://arxiv.org/abs/2510.20270
29. M. Bavarian et al., "Efficient Training of Language Models to Fill in the Middle" (2022). https://arxiv.org/abs/2207.14255
30. A. Blinn, X. Li, J. H. Kim, C. Omar, "Statically Contextualizing Large Language Models with Typed Holes", OOPSLA 2024 (PACMPL 8, art. 288). https://arxiv.org/abs/2409.00921
31. S. Chakraborty et al., "Towards Neural Synthesis for SMT-Assisted Proof-Oriented Programming", ICSE 2025. https://arxiv.org/abs/2405.01787
32. L. A. Agrawal, A. Kanade, N. Goyal, S. K. Lahiri, S. K. Rajamani, "Monitor-Guided Decoding of Code LMs with Static Analysis of Repository Context", NeurIPS 2023. https://arxiv.org/abs/2306.10763
33. MoonBit team, "MoonBit: Explore the Design of an AI-Friendly Programming Language", 18 Feb 2024. https://www.moonbitlang.com/blog/moonbit-ai
34. J. Spracklen et al., "We Have a Package for You! A Comprehensive Analysis of Package Hallucinations by Code Generating LLMs", USENIX Security 2025. https://arxiv.org/abs/2406.10279
35. Y. Li et al., "Competition-level code generation with AlphaCode", Science, 2022. https://arxiv.org/abs/2203.07814
36. Google DeepMind, "AlphaDev discovers faster sorting algorithms", 7 June 2023 (Mankowitz et al., Nature, 2023). https://deepmind.google/discover/blog/alphadev-discovers-faster-sorting-algorithms/
37. Google DeepMind, "AlphaEvolve: A Gemini-powered coding agent for designing advanced algorithms", 14 May 2025. https://deepmind.google/discover/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/
38. D. Li et al., "S*: Test Time Scaling for Code Generation" (Feb 2025). https://arxiv.org/abs/2502.14382
39. L. He, Z. Chen, Z. Zhang, X. Gao, L. Sheng, "Effective LLM Code Refinement via Property-Oriented and Structurally Minimal Feedback" (Property-Generated Solver; 2025, rev. May 2026). https://arxiv.org/abs/2506.18315
40. K. Stechly, K. Valmeekam, S. Kambhampati, "On the Self-Verification Limitations of Large Language Models on Reasoning and Planning Tasks", ICLR 2025, https://arxiv.org/abs/2402.08115; S. Kambhampati et al., "LLMs Can't Plan, But Can Help Planning in LLM-Modulo Frameworks", ICML 2024, https://arxiv.org/abs/2402.01817
41. S. Nikolov et al., "How is Google using AI for internal code migrations?" (Jan 2025). https://arxiv.org/abs/2501.06972
42. C. Loughridge et al., "DafnyBench: A Benchmark for Formal Software Verification" (2024). https://arxiv.org/abs/2406.08467
43. C. Yang et al., "AutoVerus: Automated Proof Generation for Rust Code" (2024, rev. Aug 2025). https://arxiv.org/abs/2409.13082
44. DeepSeek-AI, "DeepSeek-Prover-V2" (Apr 2025), https://arxiv.org/abs/2504.21801; L. Chen et al., "Seed-Prover: Deep and Broad Reasoning for Automated Theorem Proving" (Jul 2025), https://arxiv.org/abs/2507.23726
45. M. Endres, S. Fakhoury, S. Chakraborty, S. K. Lahiri, "Can Large Language Models Transform Natural Language Intent into Formal Method Postconditions?", FSE 2024. https://arxiv.org/abs/2310.01831
46. S. K. Lahiri, "Evaluating LLM-driven User-Intent Formalization for Verification-Aware Languages", FMCAD 2024. https://arxiv.org/abs/2406.09757
47. Z. Ye et al., "VERINA: Benchmarking Verifiable Code Generation" (2025, rev. Mar 2026). https://arxiv.org/abs/2505.23135
48. A. Thakur et al., "CLEVER: A Curated Benchmark for Formally Verified Code Generation" (2025). https://arxiv.org/abs/2505.13938
49. C. Sun, Y. Sheng, O. Padon, C. Barrett, "Clover: Closed-Loop Verifiable Code Generation" (2023, rev. 2024). https://arxiv.org/abs/2310.17807
50. S. Fakhoury, A. Naik, G. Sakkas, S. Chakraborty, S. K. Lahiri, "LLM-Based Test-Driven Interactive Code Generation: User Study and Empirical Evaluation", IEEE TSE 50(9), 2024. https://arxiv.org/abs/2404.10100
51. M. Maaz, L. DeVoe, Z. Hatfield-Dodds, N. Carlini, "Agentic Property-Based Testing: Finding Bugs Across the Python Ecosystem", NeurIPS 2025 DL4C workshop. https://arxiv.org/abs/2510.09907
52. O. Khattab et al., "DSPy: Compiling Declarative Language Model Calls into Self-Improving Pipelines", ICLR 2024, https://arxiv.org/abs/2310.03714; https://github.com/stanfordnlp/dspy
53. L. Beurer-Kellner, M. Fischer, M. Vechev, "Prompting Is Programming: A Query Language for Large Language Models", PLDI 2023. https://arxiv.org/abs/2212.06094
54. Guidance repository (2026). https://github.com/guidance-ai/guidance
55. Z. R. Tam et al., "Let Me Speak Freely? A Study on the Impact of Format Restrictions on Performance of Large Language Models" (2024). https://arxiv.org/abs/2408.02442
56. SWE-bench team, "SWE-bench Multilingual" (2025), appendix B. https://www.swebench.com/multilingual.html
57. D. Zan et al., "Multi-SWE-bench: A Multilingual Benchmark for Issue Resolving" (Apr 2025), Table 4. https://arxiv.org/abs/2504.02605
58. H. He and Thinking Machines Lab, "Defeating Nondeterminism in LLM Inference", 10 Sep 2025. https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/
59. A. J. Ko, B. A. Myers, "Designing the Whyline: A Debugging Interface for Asking Questions about Program Behavior", CHI 2004. https://faculty.washington.edu/ajko/papers/Ko2004Whyline.pdf
60. Kiro, "Introducing Kiro", 14 July 2025. https://kiro.dev/blog/introducing-kiro/
