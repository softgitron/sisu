# 20 - Pre-registration of the Sisu experiment

This document fixes the rules of evidence for the experiment in
[18-next-steps.md](18-next-steps.md) before any participant writes a line,
as 18's principle 4 requires. It states what each of the three bets in
[16-validation-plan.md](16-validation-plan.md) predicts, how every number is
produced and by whom, how each number is compared with its threshold, and
what the design does for each outcome. It is written to be judged on its
own: the thresholds and decision rules are copied from 18, not referenced,
so a reader can check the final report against this text alone. Once
frozen, its commit hash is cited in the final report, and nothing in it
changes except by a numbered amendment recorded before the data it affects
are collected.

Where 18 is silent on a detail that a pre-registration needs, this draft
proposes a concrete default and marks it **[default proposed; owner to
confirm]**. Things only the owner can supply are written `[owner: ...]`.
The document cannot be frozen while either kind of mark remains, except
the few that the materials register fills later (sections 1.3 and 4.7).
Section 13 lists every placeholder. The people the study
needs are recruited with [21-recruitment.md](21-recruitment.md).

## 1. Registration

### 1.1 Record

| Field | Value |
|---|---|
| Title | Pre-registration of the Sisu experiment |
| Version | 1.0 (draft for the owner's sign-off) |
| Date | October 2026 |
| Plan registered | [18-next-steps.md](18-next-steps.md): Phases 0-9 over 52 weeks, with the six accepted amendments A1-A6 in place (section 12) |
| Questions answered | Bet 1 (D-004, Q-1), bet 2 (D-015, Q-5), bet 3 (D-014, Q-6); the parallel questions Q-2, Q-3, Q-9, Q-11, Q-28, Q-34 and Q-50; the amendments' own questions (N-03, N-04, N-26, N-50, D-058, D-062) |
| Commit hash at freeze | `[to be filled at freeze]` |
| Early-protocol commit hash | `[to be filled at freeze]` (section 1.3, rule 3) |
| Freeze tag | `prereg-v1.0` [default proposed; owner to confirm] |
| Status | Draft. Not frozen. No participant has started. |
| Data location | `experiments/` in this repository; `experiments/PREREGISTRATION.md` holds only the freeze hash and a link to this document [default proposed; owner to confirm] |
| Funding | `[owner: funder or own funds, and the total budget]` |
| Institution and ethics review | `[owner: institution, review body and approval reference, or the reason none is required]` |

### 1.2 Roles

| Role | Name | Duties under this document |
|---|---|---|
| Project owner | `[owner]` | Signs the freeze and every amendment. Funds the study. Receives the escalation in section 8.6. Decides every item in section 13. |
| Principal investigator (PI) | `[owner]` | Writes and maintains this document. Recruits, screens and contracts participants (21). Writes the behavioural test suites, the instruments and the analysis scripts before the data they touch exist. Runs the studies and the model arm. Rates nothing. |
| Runtime engineer | `[owner]` | Builds and runs the Phase 1 harness, including the io_uring cancellation test (A1). Owns the interpreter loop, the scheduler, `std.cap` and `std.x.http`. Builds the monitor's counterexample mode (A2) in Phase 3. |
| Compiler engineer | `[owner]` | 18's lead: SIR, drop elaboration, the ownership checker and the dynamic monitor. Triages checker bugs within 24 hours during the studies and reruns every rejected corpus program in the counterexample mode (A2). Plants the seeded bugs of A4 (section 4.9). |
| Language engineer | `[owner]` | 18's type-system engineer: the type checker with the binding count (A3), traits, comptime, the test surface with the `sim` profile and replay (A4, A5), the language server, and the API digest the model arm reads (A6). |
| Go/Java runtime reviewer | `[owner]` | Configures and reviews the Go, JDK, tokio and coroutine-library controls of Phase 1, and reproduces every harness configuration on the second machine. |
| Independent referee | `[owner]` | Co-signs the freeze and every amendment. Writes the 20 adversarial programs of Phase 3. Runs every threshold comparison from the raw logs. Audits the triage labels, classifies the god-struct candidates and a sample of the desk audit. Writes the review that is published unedited with the report. |

The compiler and language engineer titles map to 18's lead and type-system
engineer. 18 gives the `sim` profile and replay to the type-system
engineer and the counterexample mode's construction to the runtime
engineer; who reruns corpus rejections and who plants the seeded bugs are
assigned here [default proposed; owner to confirm]. Participants are in
section 3.

### 1.3 Freeze and change rules

1. **Freeze.** The document is frozen when the owner and the referee each
   record their sign-off in the freeze commit, every "default proposed"
   mark has been resolved (kept, changed or deleted), and every `[owner:
   ...]` placeholder has been filled except the items that section 4.7
   lets the materials register fill later.
2. **Deadline.** The freeze happens by the end of week 4, Phase 0's gate
   in 18, and in any case before the first participant writes a line.
3. **Early protocols.** Two protocols collect data before week 4: the
   lifetime desk audit (section 4.6) and the harness measurement protocol
   (sections 4.4 and 6.5). They are frozen by the end of week 1 in a commit
   signed by the owner and the PI, whose hash is recorded in section 1.1 at
   the full freeze. The referee reviews them at the full freeze; a change
   the referee asks for is an amendment and applies only to data not yet
   collected. Harness runs made before the full freeze are development runs
   and are never reported as results [default proposed; owner to confirm].
4. **Hash.** A commit cannot contain its own hash, so the hash field in
   section 1.1 is filled by the commit that immediately follows the freeze
   commit, and that commit changes nothing else. The hash is also published
   outside the repository at `[owner: one external location]`, and added to
   the posted call for participants (21), which goes out in week 1, before
   the hash exists [default proposed; owner to confirm].
5. **Amendments.** After freeze, any change to this document is a
   numbered, dated amendment recorded in section 12 before the affected
   data are collected. An amendment needs the owner's and the referee's
   sign-off, names the data it affects, and states that none of them exist
   yet [default proposed; owner to confirm]. Amended text is changed in
   place with the marker "(amended by A*n*)", and the replaced text is kept
   in the amendment entry.
6. **No retroactive change.** A rule whose data already exist cannot be
   amended. If such a rule proves wrong, the report gives the analysis under
   the frozen rule as the result of record and may add the corrected
   analysis beside it, labelled as such; verdicts and design actions follow
   the frozen rule [default proposed; owner to confirm].
7. **Deviations.** Anything that departs from this document after the
   affected data exist (a missed diary, a failed machine, a late patch, a
   dropout) is a deviation, not an amendment. Deviations are logged with
   date, cause and affected data in `experiments/DEVIATIONS.md` and listed
   in the report. A deviation never changes a threshold or a decision rule
   [default proposed; owner to confirm].
8. **Precedence.** Before freeze, a disagreement between this document and
   18 is resolved by correcting this document. After freeze, this document
   governs the experiment; a later change to 18 that would alter a rule here
   takes effect only through an amendment.

## 2. Hypotheses

### 2.1 How verdicts are worded

Each bet ends in one of three verdicts, as 18 requires.

- **Holds.** The bet's primary threshold is met with the pre-specified
  effect, and none of its guard thresholds trips.
- **Fails.** The primary is missed by the margin this section names for
  that bet, or a guard threshold trips.
- **Not shown.** Anything else: an effect in the predicted direction that
  is smaller than the pre-specified one, replications that disagree (the
  two harness machines, the two ports), or too little data under section
  7.5. Following 18's principle 4, anything smaller than the pre-specified
  effect is reported as "not shown", never as "holds". No fallback is built
  on a "not shown" result, and the next data come from the playground's
  unsolicited corpus and from rerunning the corpus on the native backend
  after the experiment.

The three kinds of threshold (primary, guard, secondary) are defined in
section 7.8. A verdict is a statement about this study only: "holds" from
six authors is evidence for the design, not proof of it, and every verdict
sentence in the report carries its sample size and, for bet 3, its era
label (section 7.10).

The four tables below are copied verbatim from 18's
[Measurements and thresholds](18-next-steps.md#measurements-and-thresholds),
including the rows the amendments added (section 2.5). Section 6 gives each
row's operational definition; where a row leaves a choice open, section 6
makes it and marks it.

### 2.2 Bet 1: ownership without named lifetimes (D-004, Q-1)

**H1.** People who did not design the language can write the six corpus
programs of section 4.1 with second-class references, single-lifetime
views, `Shared` and `Arena`, such that:

- at most one of the six programs needs `unsafe` for an ordinary pattern,
  and P2 and P4 can be written without it (T1.1);
- escape-hatch density exceeds 1.5 times the same program's Rust control
  in at most two programs (T1.2);
- the fighting-the-checker rate is at most half the same author's Rust
  rate (T1.3, the primary).

H1 is falsified by more than one program needing `unsafe` for an ordinary
pattern, by P2 or P4 being unwritable, or by a fighting-the-checker rate
that is not materially below Rust's.

| What | How measured | Baseline | Threshold that changes the design |
|---|---|---|---|
| T1.1 `unsafe` for an ordinary pattern | per program, `sisu audit --ownership` plus the diary's "I would need unsafe here" | none | more than one of six programs: fallback branch; P2 or P4 unwritable: the stop condition in 16 |
| T1.2 escape-hatch density: `Shared` + `Weak` + arena handle + `Cell` per 100 final lines | audit script in CI | the same program's Rust control (`Rc`/`Arc`/`RefCell`/`Weak`/arena/index) | Sisu above 1.5x Rust in three or more programs |
| T1.3 fighting-the-checker rate: rater category (b) events per 100 final lines | diaries and JSONL logs, two blind raters, kappa at least 0.6 | the same author's Rust implementation | Sisu not at or below 0.5x Rust: friction branch; not materially below Rust: the stop condition in 16 |
| T1.4 exit questionnaire, "borrow checker rejecting valid code" is a big problem | Rust survey wording verbatim | 10.6% ([`surveys.md`](../research/evidence/surveys.md)) | reported, not pass/fail |
| T1.5 time to green tests | timed sessions, second-written language | the same author's Rust; the non-Rust author's home language | Sisu slower than Rust for the same author; non-Rust authors over 2x their home language |
| Desk-audit prior | share of explicit lifetimes with no Sisu mapping in application-tier crates | none | 20% or more pre-builds the generation-checked reference fallback behind a flag |
| View-struct splits per program; designer interventions that changed program structure | diaries, help-desk log | none | flag at more than 2 and more than 3 respectively; reported |
| Missing-feature entries ("could not be written without X") | diaries | none | named lifetimes or first-class references named independently by two authors: design review |
| Checker soundness: monitor violations on programs the static checker accepted | the corpus plus 24 hours of mutation fuzzing | 0 | any violation blocks 0.1; one involving views reopens the view and escape rules |
| Candidate false positives: rejections that reach no conflict under the monitor | every rejected corpus program rerun across its tests in the counterexample mode, by the team only | none | reported beside T1.3; they are the false-positive log the friction branch's rule refinements draw on |

**Roles of the rows.** Primary: T1.3, as the median of the six
within-author Sisu/Rust ratios (section 6.3). Guards: T1.1, T1.2, and the
soundness row's violations that involve views. Secondary: T1.4, T1.5, the
desk-audit prior, view-struct splits, designer interventions,
missing-feature entries, the soundness row's other violations, and the
candidate false positives (A2).

**"Materially below Rust"** is not quantified in 18 or 16. This document
fixes it as a T1.3 ratio at or below 0.8 [default proposed; owner to
confirm].

| Verdict | Condition |
|---|---|
| Holds | T1.3 ratio at or below 0.5; T1.1 and T1.2 pass; no open soundness violation involving views |
| Not shown | T1.3 ratio above 0.5 and at or below 0.8, with T1.1 passing; or T1.3 at or below 0.5 with T1.2 failing; or fewer complete pairs than section 7.5 requires |
| Fails | T1.1 fails (more than one program needs `unsafe` for an ordinary pattern, or P2 or P4 cannot be written without it); or T1.3 ratio above 0.8 |

A soundness violation involving views suspends the verdict until the view
and escape rules have been re-examined (section 8.1). T1.5 never changes
the verdict; it triggers its own branch in section 8.1. 18 names no branch
for T1.2 failing while T1.1 and T1.3 pass; this document records that case
as "not shown" and sends the programs over 1.5x to the Phase 9 synthesis
for review against Q-1's fourth option, without building anything
[default proposed; owner to confirm].

### 2.3 Bet 2: capability parameters at scale (D-015, Q-5)

**H2.** Capabilities passed as ordinary parameters stay tolerable on a real
service. Across the two ports of the pre-registered Go service (S1, S2),
both implementations of P3 and C1:

- the median I/O-doing function carries at most two capability parameters,
  and none carries more than four (T2.1, the primary);
- no struct bundles four or more distinct capability types outside
  `main`'s composition root (T2.3).

H2 is falsified by a pooled median above two, by any I/O-doing function
with more than four capability parameters, or by any god struct.

| What | How measured | Baseline | Threshold that changes the design |
|---|---|---|---|
| T2.1 capability parameters on I/O-doing functions | `sisu audit --caps` from SIR over S1, S2, P3, C1 | the Go original's `ctx` and injected-dependency parameters per I/O-doing function | median above 2 or maximum above 4: implement the reserved `with Fs, Clock` clause (16, Q-5) |
| T2.3 god structs: four or more distinct capability types bundled outside `main`'s composition root | `sisu audit --caps` | 0 | any: implement `with`; a struct taking two or three related capabilities at construction is the accepted pattern |
| Threading depth from `main` to each leaf use | `sisu audit --caps` | Go's `ctx` threading depth | reported |
| Plumbing share of porting time | diaries | none | flag above 20% |
| Time to write ten pre-specified tests with fakes | timed | the same ten against the Go original's mocks | slower: flag; more than 25% slower: redesign the `std.cap` fakes (recording fakes derived by reflection, [`testing.md`](../research/topics/testing.md)) |
| Supply-chain demonstration | a planted dependency fails to type-check without `Fs` and `Net` | none | must pass and be reproducible |

18's table has no T2.2; the identifier is not used for anything added
later.

**Roles of the rows.** Primary: T2.1. Its median clause is judged on the
pooled median of all I/O-doing functions; its maximum clause also counts
on an incomplete port, because more code cannot undo it. Guard: T2.3, for
the same reason. Secondary: threading depth, plumbing share, test-writing
time and the supply-chain demonstration.

| Verdict | Condition |
|---|---|
| Holds | pooled median at most 2 and each port's own median at most 2; no I/O-doing function above 4; no god struct; both ports pass at least 80% of their translated integration tests |
| Not shown | pooled median at most 2 but one port's own median above 2; or a port below 80% of its translated integration tests, with no fail condition met |
| Fails | pooled median above 2 with both ports at 80% or more; or any I/O-doing function with more than four capability parameters; or any god struct outside the composition root (the last two count even on an incomplete port) |

The per-port rule only makes "holds" harder; it never blocks the fail
actions of section 8.2 [default proposed; owner to confirm].

### 2.4 Bet 3: colourless tasks without a moving GC (D-014, Q-6)

**H3.** Stackful, colourless tasks on stacks that never move can be built
without a moving garbage collector with acceptable stack behaviour:

- at least one stackful strategy (A-D in section 4.4) keeps resident memory
  per idle task within four times Go's, at 100k and at 1M tasks, after
  shallow use and after a 32 KiB excursion, on both machines (the primary,
  harness era);
- ordinary corpus code does not exhaust a default stack reservation that
  passes the memory primary (the guard, interpreter era).

H3 is falsified by every stackful strategy exceeding four times Go's
memory per idle task, or by stack exhaustion in ordinary corpus code that
no admissible default reservation covers.

| What | How measured | Baseline | Threshold that changes the design |
|---|---|---|---|
| Memory per idle task at 100k and at 1M, after shallow use and after a 32 KiB excursion | harness, two machines (virtual machines or GitHub-hosted runners are enough in the harness era) | Go measured after its garbage collection, and JDK 21+, on the same machine; the 1M-after-excursion cell is required only of strategies that release pages on park | above 4x Go: stackless compilation of task bodies behind the same `scope`/`spawn` API (16) |
| Spawn-plus-join median and p99 | harness | Go | median above 3x Go with pooled stacks: runtime design review |
| Scale: 1M idle tasks on a 16 GiB machine with default sysctls | harness | none | fail: guard-page-per-task is out; pooled stacks with software probes become the design, with probe overhead on compute loops at most 3% |
| 10k-connection echo p99; 100k-connection idle RSS | harness | Go | p99 above 2x Go: runtime design review, not a language change |
| Stack exhaustion | deep-recursion tests; `--stats stack-depth` histogram over the whole corpus (interpreter era) | none | any segfault is a bug; any ordinary (bet-1 or bet-2) program whose depth exceeds the default reservation: a larger default or the stackless fallback; a default above 8 MiB re-evaluates memory at that size |
| Starvation (Q-34) | p99 latency of a probe task under N compute-bound tasks with compiler-inserted yield points, whose cost counts in the probe-overhead budget | Go with its default asynchronous preemption and no manual yields (harness); 10 ms (interpreter) | worse than Go, or p99 above 10 ms: revisit asynchronous preemption |
| Cancellation with a lent buffer (D-058) | harness: a million in-flight io_uring reads into stack buffers cancelled on each stackful strategy; buffers written after their cancelled call returned; cancel-to-return latency | 0 | any is a runtime bug; one that waiting for the original completion cannot remove reopens D-058 |
| Semantics: schedules explored per test, orphan tasks, unobserved failures, lock guards not dropped on unwind | interpreter, `--explore 1000`, then PCT and bounded exhaustive schedules, injected faults and simulated crash and restart under `--sim` | 0 | any non-zero count is a bug; a class that needs a language change reopens D-014 |
| `sim` seeded-bug study | interpreter: the twenty pre-registered ALICE and Jepsen bugs planted in copies of P3 and a small replicated log; bugs found within a fixed CPU budget | random `--explore` schedules alone, same budget | fewer than fifteen, or no more than random alone: the `sim` profile stays a prototype and is not carried into v1 (D-070) |

**Roles of the rows.** Primary: memory per idle task. Guard: stack
exhaustion in ordinary corpus code. Secondary: spawn-plus-join, scale with
its probe-overhead limit, echo latency and idle RSS, starvation,
cancellation with a lent buffer (A1), semantics, and the `sim` seeded-bug
study (A4), which decides whether the `sim` profile enters v1 and never
changes bet 3's verdict.

For each stackful strategy *s*, let *w(s, M)* be its worst ratio to Go over
the four memory conditions on machine *M*. Strategy *s* passes if *w* is at
or below 4 on both machines, fails if *w* is above 4 on both, and is
inconclusive otherwise; a machine whose five-run range for a cell crosses 4
makes that cell inconclusive [default proposed; owner to confirm].

| Verdict | Condition |
|---|---|
| Holds | at least one stackful strategy passes and is not ruled out by the scale row; the exhaustion guard has not tripped |
| Not shown | no stackful strategy passes, and at least one is inconclusive (the machines disagree) |
| Fails | every stackful strategy fails; or the exhaustion guard trips |

The guard trips when an ordinary program's estimated depth (section 6.5)
exceeds the default reservation and the larger default that would cover it
is above 8 MiB and fails the memory primary when memory is re-measured at
that size, which is how this document reads 18's exhaustion row ("a larger
default or the stackless fallback; a default above 8 MiB re-evaluates
memory at that size") [default proposed; owner to confirm]. The memory
primary is decided at week 8 in the harness era; the guard can only be
decided once the corpus's frame-depth histogram exists, at week 40.

### 2.5 Parallel questions, health and the amendments' rows

| What | How measured | Baseline | Threshold that changes the design |
|---|---|---|---|
| Q-2 atomic elision | fraction of `Shared(...)` construction sites proven task-local, validated by the monitor | none | below 80% flags the single-`Shared` design for a native benchmark; the "within 10%" benchmark in 16 needs native code and is deferred |
| Q-3 `.copy()` per 30 lines of application code | audit | 16 | more than one reopens implicit copying |
| Q-9, adapted to explicit `return` | the top-ten diagnostics list from all diaries | none | any top-ten diagnostic caused by a design rule rather than a bug gets a design review ("expected (), found T" is moot) |
| Q-11 newtype wrappers per 200 lines | audit | 16 | more than one reopens scoped instances |
| Q-28 | P4's diaries: `dyn` plus `@delegate` reported natural, or inheritance emulated with boilerplate | none | boilerplate reopens the question |
| Q-50 bindings whose type is fixed by a later statement | checker count over every corpus implementation | none | under about 3% with no friction reported adopts the left-to-right rule; otherwise D-003's local inference stays |
| Model arm (N-50): compile and hidden-test pass rates, false-friend bugs, tokens to a pass | frontier models on P1-P6's tasks with no context, the pocket reference, or reference plus digest, each with and without compiler feedback | the same tasks in Python | reference and digest far from Python's rates: the MultiPL-T corpus of [10-tooling.md](10-tooling.md) becomes necessary; a false friend that compiles is a design bug under D-062; nothing else in the design follows the models |
| Parser fuzzing: panics, diagnostic storms, trees that do not reprint their input, inputs the checker leaves untyped | CI | 0 | any is a bug |
| `sisu check` after a one-body edit on the std-plus-corpus tree; LSP response | CI | 300 ms; 100 ms | missing by 2x is a design review per [10-tooling.md](10-tooling.md) |
| Incremental rebuild of the Rust prototype | CI | 10 s | Roc's complaint; above it, the crate layout is revisited |
| Interpreter adequacy | the whole corpus's test suites and the port's integration tests on a laptop | 10 minutes each | missing schedules a compact bytecode encoding and cached impl lookup as optimisation work, not a redesign |
| Replay (N-04) | corpus failures reproduced from their token alone on a second machine; 100 recorded `--explore` runs replayed; recording overhead and log bytes per request on P3 | every failure that does not hold the real `System`; zero divergences; 10% overhead (R2's bar for Apache) | a token that does not reproduce, or a divergence, is a bug; overhead above 10% keeps the log a prototype (D-070) |
| Diagnostics quality | share of borrow and `Send`/`Sync` errors in W that name a fix; author ratings per message | 100% | a message rated unhelpful by two authors is rewritten before 0.1 |
| Process | pre-registration hash cited; amendments listed; eligibility verified; raw logs published; benchmarks reproduced on a second machine by an outsider; the referee's review published unedited | all | a report that fails any of these is not published as a Sisu evidence report |

**Rows from the amendments.** Seven rows in the four tables come from the
amendments of section 12 or were changed by them: cancellation with a lent
buffer (A1); candidate false positives (A2); Q-50 (A3); the semantics
row's `--sim` schedules and the seeded-bug study (A4); replay (A5); and the
model arm (A6). Where a row uses a word rather than a number, this document
fixes the number:

- Q-50, "under about 3%": strictly below 3.0% of the bindings counted in
  section 6.8; "no friction reported": no author answers "big problem" to
  the exit-questionnaire item of section 6.8 [default proposed; owner to
  confirm].
- Model arm, "far from Python's rates": the best Sisu condition's
  hidden-test pass rate is below 0.8 of Python's for the same models and
  tasks [default proposed; owner to confirm]. "A false friend that
  compiles is a design bug": any compiling attempt with a hidden-test
  failure attributed to a row of the false-friend table (section 6.9)
  makes that row a design bug under D-062.
- Seeded-bug study, "a fixed CPU budget": two CPU-hours on M1 per planted
  bug, the same for `--sim` and for random `--explore` [default proposed;
  owner to confirm].
- Replay, "100 recorded `--explore` runs": drawn by the registered script
  from the semantics suite T and P3's tests [default proposed; owner to
  confirm].

**Q-10.** 18's corpus table ties P5 to Q-10 but no row measures it. This
document adds one descriptive count with no threshold: the lines of P5
duplicated between its by-ref and by-value variants (section 6.7). It
changes nothing in the design, because Q-10's decision waits for the
standard library [default proposed; owner to confirm].

## 3. Design

### 3.1 Overview

The experiment is four studies and one benchmark, run in the order of 18's
principle 2 (cost times damage):

| Study | Phase | Design | Decides |
|---|---|---|---|
| Stack harness | 1 | benchmark of five stack strategies against four controls on two machines | bet 3 (harness era), Q-6 provisionally |
| Bet-1 corpus study | 5 | within-author comparison: each Rust-arm author writes two programs in Sisu and in Rust; each non-Rust author writes two programs in Sisu and in their home language; order counterbalanced | bet 1 |
| Port study | 7 | two independent ports of one Go service; the Go porter writes ten tests with Sisu fakes and the same ten with the original's mocks | bet 2, Q-5 |
| Semantics suite and seeded-bug study | 6, 8 | the team's suite T under random schedules and the `sim` profile; twenty pre-registered bugs planted in P3 and L, hunted by `--sim` and by random schedules | bet 3 (semantics, interpreter era); whether `sim` enters v1 |
| Model arm | 9 | frontier models solve P1-P6's tasks under six context and feedback conditions, against the same tasks in Python | A6's question only, never a bet |

Every human comparison that decides anything is within one person: a
Rust-arm author's Sisu program against the same author's Rust program, a
non-Rust author's Sisu program against the same author's home-language
program, the Go porter's tests with Sisu fakes against the same porter's
tests with Go mocks. This is the design's answer to Prechelt's finding that
the variability between programmers in one language exceeds the difference
between languages ([`studies.md`](../research/evidence/studies.md),
section 10). Team-written programs never count as evidence about people;
the harness and the semantics suite measure properties of a runtime, which
is why bet 3 may use them.

### 3.2 Participants

| Role | Number | Background | Writes | Control | Paid time | Weeks |
|---|---|---|---|---|---|---|
| Corpus author, Rust arm (the Rust control group) | 3 | two or more years of production Rust; one of the three is a Rust library author (P5) | two corpus programs in Sisu | the same two programs in Rust | about 4 weeks | 14-32 |
| Corpus author, non-Rust arm | 3 | Go, Kotlin or Swift application developers with no Rust; one Go backend engineer (P3) and one Swift or Kotlin application developer (P4) | two corpus programs in Sisu | the same two programs in their home language | about 4 weeks | 14-32 |
| Red-team author | 1 | a Rust expert briefed to break the model | R | the Rust version of each pattern | 3 weeks | 20-25 |
| Porter S1, the Go control | 1 | Go background | a port of the Go service; ten tests with Sisu fakes | the same ten tests against the Go original with its existing mocking approach | 5 weeks | 29-40 |
| Porter S2 | 1 | Rust background | a port of the Go service; ten tests with Sisu fakes | none | 5 weeks | 29-40 |
| C1 author | 1 | a student or junior developer | C1 | none | 3 weeks | 28-34 |
| Friction raters | 2 | not on the team | code every diary and log event | n/a | 1 week each | 23-32, plus one day at week 41 |
| Go/Java runtime reviewer | 1 | Go and JVM runtime tuning | configures the controls; reproduces the harness on the second machine | n/a | 2 weeks | 1-10 |
| Referee | 1 | a publicly sceptical Rust expert | the adversarial programs A; every threshold comparison; the review | n/a | 2 weeks | 1-52 |
| Desk-audit contractor | 1 | a Rust expert | the lifetime desk audit | n/a | 2 weeks | 1-4 |
| Reserve authors | 2 | one per arm | replace a dropout, or the re-runs of section 8.1 | as the slot they fill | on activation | 8-40 |

The numbers are 18's, which total about 48 contractor-weeks. The reserve
authors are not in 18: they exist because 18's re-run branches need "two
new authors" at short notice and because a six-author study cannot absorb a
dropout [default proposed; owner to confirm]. Where 18's corpus table says
P6 is written by "both non-Rust authors", this document follows 18's rule
that each of the twelve implementations pairs one Rust-arm author with one
non-Rust author, because only that rule gives each author two programs;
P6's non-Rust author writes its home-language control, which is Go when
that author's home language is Go [default proposed; owner to confirm].

### 3.3 Eligibility

All roles: 18 or older; able to contract with `[owner: contracting
entity]`; able to write diaries and reports in English; available for the
role's weeks; consents under section 3.7.

| Role | Required |
|---|---|
| Rust arm | two or more years of production Rust (code shipped to users or deployed, as a main language); for the P5 slot, maintains a published Rust library used by others |
| Non-Rust arm | two or more years of production work in Go, Kotlin or Swift as a main language; no Rust beyond tutorials or exercises, about 500 lines in total at most, self-reported and checked at interview [default proposed; owner to confirm]; for the P3 slot, production Go services; for the P4 slot, shipped Swift or Kotlin applications with a user interface |
| Red-team author | five or more years of Rust, and has written code that uses each pattern of section 4.1.1 or its close relative [default proposed; owner to confirm] |
| Porter S1 | two or more years of production Go services, including tests that use interfaces, mocks or fakes for I/O [default proposed; owner to confirm] |
| Porter S2 | two or more years of production Rust [default proposed; owner to confirm] |
| C1 author | a student, or under two years of professional programming [default proposed; owner to confirm] |
| Friction raters | reads Rust and at least one of Go, Kotlin or Swift fluently; experience of code review or qualitative coding [default proposed; owner to confirm] |
| Go/Java runtime reviewer | production experience tuning Go and JVM runtimes (garbage collectors, schedulers, virtual threads) and measuring performance on Linux [default proposed; owner to confirm] |
| Referee | Rust expertise at the level of a recognised library or compiler contributor, and a public record of scepticism that ownership without named lifetimes is expressive enough [default proposed; owner to confirm] |
| Desk-audit contractor | five or more years of Rust [default proposed; owner to confirm] |

**Experience bands.** Professional programming experience is recorded in
three bands: 3-5, 6-10, and 11 or more years. Each arm of corpus authors
has one author in each band, so that the arms are matched on experience
as well as on years in their main language (two or more in both)
[default proposed; owner to confirm]. If a band cannot be filled, the
matching rule is relaxed by amendment before any author starts.

### 3.4 Exclusion

A person is excluded from every participant role if they:

1. have committed to this repository, opened an issue or pull request on
   it, or taken part in its design discussions;
2. are, or were in the last two years, employed by, contracted to, or a
   close collaborator of the owner, the PI or an engineer [default
   proposed; owner to confirm];
3. are public advocates of ownership without named lifetimes: have argued
   publicly that such a model (Hylo, Mojo, Vale, Austral or similar) is
   sufficient or preferable, or have contributed to such a language's
   design (18; definition [default proposed; owner to confirm]). This does
   not apply to the referee, whose eligibility is the opposite;
4. for corpus authors, the red team, porters, the C1 author and raters:
   have written Sisu code other than the proposal's examples before
   onboarding;
5. would hold two participant roles, except that the desk-audit contractor
   may also be the red-team author [default proposed; owner to confirm];
6. for porters: have contributed to the chosen Go service.

Reading the proposal is not an exclusion; it is recorded at screening,
because 16 states some thresholds. Eligibility is recorded per participant
in `experiments/ELIGIBILITY.md` under pseudonyms, checked by the referee
before contracts are signed, and published with the report.

### 3.5 Assignment and order

- **Programs to corpus authors.** A recorded random draw under these
  constraints: every program has one Rust-arm and one non-Rust author;
  P3's non-Rust author is the Go backend engineer and P4's is the Swift or
  Kotlin developer; P5's Rust-arm author is the Rust library author; each
  author gets two programs; no author gets both P3 and P4, whose windows
  overlap (18 states the random draw; the constraints are [default
  proposed; owner to confirm]).
- **Order.** Each author writes one of their programs Sisu-first and the
  other control-first, so the order is counterbalanced within every author
  and across both arms; which program is which is part of the draw. 18
  requires counterbalancing for the Rust arm; extending it to the non-Rust
  arm is [default proposed; owner to confirm].
- **Mechanics.** The draw script is committed before freeze. The referee
  supplies its seed after the last author contract is signed and publishes
  the seed with the output [default proposed; owner to confirm].
- **Porters.** Both port the same service; nothing is assigned. The Go
  porter writes the ten tests in Sisu first and in Go second, the order
  that favours Go [default proposed; owner to confirm].
- **Model arm.** The order of conditions is randomised per task and model
  by the same kind of seeded script.

### 3.6 Blinding

| Who | Blind to | How |
|---|---|---|
| Corpus authors, red team, porters, C1 author | numeric thresholds, decision rules, interim results, A2's counterexamples | the consent form asks them not to read sections 2 and 6-8 of this document or 18's measurement tables until their last deliverable, and screening records what they had read; A2 runs on copies whose output never reaches them; interim values are sealed (section 7.9) |
| Friction raters | the language of each event where feasible; authors' identities; thresholds; A2's output; other raters' codes until reconciliation | the masking script (section 4.5); events from all languages interleaved in random order; pseudonyms |
| Engineers | diaries and session times during the studies | they see bug reports, help-desk questions and the diagnostic logs that triage and A2 need, nothing else [default proposed; owner to confirm] |
| Referee | nothing | the referee sees everything and runs every threshold comparison from the raw logs (section 7.7) |
| PI | nothing | the PI rates nothing and runs scripts written before the data |

The referee runs: every threshold comparison of section 2, the kappa
computation, the classification of the desk-audit sample, the god-struct
classification, the audit of triage labels, and the interim analysis.
Blinding is impossible in three places, and the report says so: authors
know which language they write; raters can often tell the language from
its syntax; the PI and the team know the hypotheses.

### 3.7 Data handling and consent

1. **Raw data are published.** Every diagnostic log, session log, diary,
   help-desk transcript, coding sheet, questionnaire answer, program
   version, harness CSV and model-arm transcript is committed under
   `experiments/data/` and published with the report.
2. **Personal data are minimised.** Participants appear in the data only by
   pseudonym: R1-R3 and N1-N3 for corpus authors, RT for the red team, S1
   and S2 for the porters (after their ports), C1 for its author, RA and RB
   for the raters, X1 and X2 for reserve authors. Session times are
   stored as offsets from each participant's first session, not clock
   times. The entry questionnaire keeps experience bands and languages,
   not employers, ages or locations. Contact and payment details are kept
   by the PI apart from the data, are never committed, and are deleted
   `[owner: retention period]` after the report [default proposed; owner
   to confirm].
3. **Consent.** Written informed consent is given before any screening data
   are kept, and separately for publication of diaries. Each participant
   reviews their own diaries before publication and may redact personal
   content but not technical content; every redaction is marked [default
   proposed; owner to confirm].
4. **Withdrawal.** A participant may withdraw at any time without a reason
   and is paid for the work done. They choose whether the data collected so
   far are kept and published or deleted. The report counts withdrawals
   either way, and section 7.5 applies [default proposed; owner to
   confirm].
5. **Identifiability.** With six authors whose backgrounds are described,
   a pseudonym does not stop people who know a participant from
   recognising them. The consent form says so. A participant who accepts
   authorship (section 11) is named without the link to a pseudonym.
6. **Payment does not depend on results.** Fees are fixed per role; a
   program delivered as blocked is paid as delivered (21).
7. **Licence and law.** Programs and diaries are published under `[owner:
   licence]`. The data controller and the applicable data-protection law
   are `[owner: data controller and law]`.

## 4. Materials

### 4.1 The corpus

The rows, programs and starting weeks are 18's corpus table. The size
bands are 18's where it gives one (300-1500 lines for the Phase 5
programs; 3-8 kLOC for the Go service) and proposed here otherwise.

| ID | Program | Size band | Written by | Language controls | Bets and questions | Counts as evidence | Starts |
|---|---|---|---|---|---|---|---|
| E | The ten files in [examples](examples/) other than `ffi.sisu`, kept in step with the syntax, a drop-order torture program, a 10k-connection echo server | as the examples | team | none | tooling | no | Phases 0-8 |
| W | Wrong-program corpora (40 core, 60 ownership) with expected codes and named fixes; a translated slice of rustc's borrow-checker UI tests | 100 files plus the slice | team | rustc's own expected errors for the slice | checker; the diagnostics-quality row | no (instrument) | Phases 2-3 |
| A | 20 adversarial programs that try to alias or dangle in safe code | 20 programs | the referee | none | checker soundness | gate only (all must be rejected) | Phase 3 |
| D | Dry-run drafts of P1-P6, used to debug the toolchain and then discarded | as P1-P6 | team | none | checker bugs | no; never shown to participants | Phase 3 |
| P1 | Tokenizer and recursive-descent parser whose AST borrows from the source buffer (view structs, lists of views, `from` returns) | 300-1500 lines | one Rust-arm author, one non-Rust author | Rust; the non-Rust author's home language | 1 | yes | week 16 |
| P2 | Graph with parent pointers and mutation during traversal (a dominator tree or Dijkstra with path reconstruction; add and remove nodes while iterating) | 300-1500 lines | same pattern | Rust; home language | 1, Q-1 | yes | week 16 |
| P3 | HTTP/1.1 server with a shared LRU cache and a background eviction task on `std.x.http` | 300-1500 lines | one Go backend engineer, one Rust-arm author | Rust; Go | 1, 2, 3 | yes | week 26 |
| P4 | Widget tree with parent navigation, focus and event callbacks stored in widgets (escaping closures, `dyn`, `@delegate`) | 300-1500 lines | one Swift or Kotlin application developer, one Rust-arm author | Rust; Swift or Kotlin | 1, Q-28 | yes | week 22 |
| P5 | Iterator adapter library (map, filter, zip, chunks, windows, peekable, by-ref and by-value variants, one lending-style case) | 300-1500 lines | one Rust library author, one non-Rust author | Rust; home language | 1, Q-10 | yes | week 16 |
| P6 | Application-shaped program chosen by the non-Rust authors (for example a CLI that streams JSON lines through a trait-based plugin registry), section 4.1.2 | 300-1500 lines | one Rust-arm author, one non-Rust author (section 3.2) | Rust; home language (Go where that is the author's) | 1 | yes | week 16 |
| R | Red-team program: the ten patterns of section 4.1.1, with the cost of each workaround | about 50-300 lines per pattern [default proposed; owner to confirm] | a Rust expert briefed to break the model | the Rust version of each pattern | 1 | yes, descriptive | week 20 |
| C1 | grep/du-style command-line tool with `Fs`, `Env`, `Proc`, grown from `wordcount.sisu` | 300-1500 lines [default proposed; owner to confirm] | an outside author, a student or junior developer | none | 2 | yes | week 28 |
| S1, S2 | Two independent ports of one pre-registered 3-8 kLOC Go service with its tests (section 4.1.3) | the original's 3-8 kLOC | one Go-background porter (S1), one Rust-background porter (S2) | the Go original: its parameter counts, and the ten tests with mocks written by S1 | 2, Q-5 | yes | week 30 |
| B | `taskbench` with strategies A-E and controls in Go, JDK 21+, tokio and a C or Rust coroutine library; the io_uring cancellation test | n/a | the runtime engineer; reviewed by the runtime reviewer | Go, JDK 21+ virtual threads, tokio, a C or Rust stackful coroutine library | 3, Q-6, Q-34, D-058 | yes, harness era | week 1 |
| T | Structured-concurrency semantics suite (supervisor tree restarting a panicking worker, deep recursion in a task, mixed compute and I/O starvation) under schedule exploration and the `sim` profile's faults, plus the frame-depth histogram over everything | n/a | team | none | 3 | yes for zero-count rows: a team suite can show a bug, not its absence elsewhere | Phase 6 |
| L | A small replicated log and copies of P3, with the twenty pre-registered seeded bugs, for the `sim` study (section 4.9) | the log about 500-1500 lines [default proposed; owner to confirm] | team | none (random `--explore` schedules under the same budget are the study's control) | 3; whether `sim` enters v1 | for the seeded-bug study only | Phase 8 |
| M | P1-P6's tasks solved by frontier models in six conditions, and in Python as the baseline (section 4.8) | as the tasks | several frontier models, run by the PI | the same tasks in Python | the model arm, D-062 | for the model arm only, never for a bet | Phase 9 |
| U | Unsolicited playground submissions after 0.1 | any | anyone | none | 1 | not analysed in this report [default proposed; owner to confirm] | Phase 9 |

Every row is 18's; rows L and M come from amendments A4 and A6.

#### 4.1.1 The red team's ten patterns

18 names five of "the ten patterns for which Rust needs lifetimes":
self-referential structs, multi-lifetime structs, stored references,
intrusive lists and cursor APIs. The other five are proposed here [default
proposed; owner to confirm]: lending iterators (an iterator that yields
references into itself, such as mutable windows); closures over borrowed
data with higher-ranked bounds (`for<'a> Fn(&'a T) -> &'a U`); arena graphs
whose nodes hold `&'arena` references to each other; a function returning a
reference that may come from either of two borrowed arguments
(`longest<'a>`); and disjoint mutable borrows of one collection
(`split_at_mut`, field-level splitting). Before starting, the red-team
author may replace up to two of the proposed five with patterns of their
own choosing, with the referee's agreement, recorded in the materials
register. For each pattern the report records: expressible directly;
expressible with a workaround (which one: `Shared`, arena handle, index,
`.copy()`, restructuring); or not expressible. The cost of a workaround is
the lines added relative to the Rust version and its run-time mechanism
(allocation, reference count, bounds or generation check).

#### 4.1.2 Choosing P6

At freeze the PI
fixes a shortlist of three application-shaped candidates, each specified
to the same level as P1-P5 and each in the 300-1500-line band; once the
non-Rust authors are contracted they rank the three without seeing any
Sisu result, the top-ranked candidate becomes P6, and the choice is
recorded in the materials register before week 16 [default proposed; owner
to confirm]. The shortlist is `[owner: three P6 candidates]`.

#### 4.1.3 The Go service

18's criteria: 3-8 kLOC; I/O in at least 30% of functions; a test suite;
interfaces or mocks for I/O. The PI shortlists at least three public
services that meet them before the Go AST tool is run on any, and the
service is chosen from the shortlist by a recorded random draw, so that no
one can choose a service by its parameter counts [default proposed; owner
to confirm]. The chosen service is `[owner: name, repository, commit]`,
recorded at freeze.

### 4.2 Specifications and behavioural test suites

For each of P1-P6, R and C1, the PI writes a behavioural specification and
a language-neutral test suite in advance, so that "done" is objective
(18). A suite is a set of input files and expected outputs run through a
thin driver per language (Sisu, Rust, Go, Kotlin, Swift; Python for the
model arm) [default proposed; owner to confirm]. The specifications are
frozen with this document, except P6's (section 4.1.2); the executable
suites and drivers are registered before the first author starts each
program. For S1 and S2, the PI translates the original's integration tests
to Sisu before week 30, and writes the ten pre-specified tests as
specifications, both registered before week 30.

**Libraries allowed in controls.** A control may use its language's
standard library, plus the equivalents of what Sisu's std gives the same
program (an arena or slotmap crate in Rust; a minimal HTTP/1.1 library for
P3), and nothing that solves the program's core. The allowed list per
program is `[owner: per-program allowlist, fixed at freeze]` [default
proposed; owner to confirm]. A Rust author may write P3 with threads or
with an async runtime, as they would normally; the choice is recorded.

### 4.3 Toolchain version policy

1. Each study runs on a tagged toolchain (`study-<phase>-<n>`). The tag
   current when a program starts is that program's frozen tag for its whole
   duration. Features a later program needs (Phase 4's `dyn` and
   `@delegate` for P4; Phase 6's tasks and `std.x.http` for P3) arrive as a
   new tag before that program starts, never during one.
2. Inside a tag only patch releases are allowed. A patch fixes a category
   (c) checker bug, a crash or internal compiler error, or a host bug. It
   never changes a rule, adds a feature, or rewords a diagnostic unless the
   diagnostic is factually wrong [default proposed; owner to confirm].
3. A rule implemented more conservatively than
   [04-memory-and-ownership.md](04-memory-and-ownership.md) specifies it
   (for example lexical borrow ends where 04 specifies non-lexical ones,
   the fallback in 18's risk table) is a checker bug: the rejections it
   causes are coded (c), and its fix is a patch [default proposed; owner to
   confirm].
4. Clarity improvements to diagnostics found during a study are queued
   until that study's verdict (week 32 for bet 1, week 40 for bet 2).
5. Every patch is logged with its date, the bug it fixes and the events it
   affects; every logged event carries the tag that produced it.
6. Control toolchains are fixed at freeze: Rust stable `[to be fixed at
   freeze]`, Go `[to be fixed at freeze]`, Kotlin `[to be fixed at
   freeze]`, Swift `[to be fixed at freeze]`, Python `[to be fixed at
   freeze]` for the model arm.
7. The model arm runs on one tag, the 0.1-interp release candidate, for all
   attempts [default proposed; owner to confirm].

### 4.4 The harness, its controls and its machines

- **Harness.** `taskbench/` (18, Phase 1): an M:N work-stealing scheduler
  over OS threads, an assembly context switch from an existing library,
  structured scope and join, a cancellation flag, and the stack strategies
  (A) per-task `mmap` reservation with a `PROT_NONE` guard page and lazy
  commit, (B) pooled stacks carved from large reservations with software
  stack-limit probes, (C) exact small stacks (2-16 KiB) for tasks whose call
  tree is bounded, (D) `madvise` decommit on park, and (E) a stackless
  state-machine variant as the lower bound of the fallback. A-D are the
  stackful strategies of H3; D is measured on top of A and of B; C counts
  only for tasks it applies to [default proposed; owner to confirm]. Its
  code may exist before freeze (Phase 1 starts in week 1); its measured
  runs may not (section 1.3, rule 3).
- **The io_uring cancellation test (A1).** A million in-flight reads into
  buffers on task stacks are cancelled, on each stackful strategy and on
  both machines (18); section 6.5 defines the procedure.
- **Controls.** Go, latest stable, `GOMAXPROCS` matched; JDK 21 or later
  with virtual threads, flags and warm-up recorded; tokio as the stackless
  reference; a C or Rust stackful coroutine library as the closest no-GC
  precedent. Exact versions, flags and the coroutine library are fixed by
  the reviewer in the early protocol: `[reviewer: versions and flags]`.
  Every runtime uses as many worker threads as the machine has physical
  cores [default proposed; owner to confirm].
- **Machines.** Two: M1, the runtime engineer's, and M2, the reviewer's,
  on which the reviewer reproduces every configuration. The owner decided
  (October 2026) that virtual machines and GitHub-hosted runners are enough
  in the harness era and that default sysctls apply; bare metal with a
  performance monitoring unit and a fixed frequency waits for the native
  era, and the 3 % probe-overhead bar is reported as measured with its
  noise. Requirements: Linux x86-64 (18's assumption); at least 16 GiB of
  RAM, with the scale test run in a 16 GiB memory cgroup so that both
  machines meet 18's "16 GiB machine"; a kernel with io_uring asynchronous
  cancellation, 6.1 or later, for A1; default sysctls (`vm.max_map_count`
  of 65530); the SMT state and the virtualisation host recorded; nothing
  else running. Specifications are recorded by `taskbench --machine-spec`
  at run time, and M1 and M2 differ in CPU vendor or generation when the
  available runners allow it [default proposed; owner to confirm].
- **Interpreter-era measurements** run on M1, except interpreter adequacy,
  which runs on a reference laptop `[owner: laptop specification]`.

### 4.5 Instruments

| ID | Instrument | Used by | Produces |
|---|---|---|---|
| I-1 | Entry questionnaire | every participant | experience band, languages and years, what of the proposal they had read, conflict declarations |
| I-2 | Session logger, in the provided environment | corpus authors, red team, porters, C1 author | session start and stop; every save, check, build, test run and help-desk message, as offsets |
| I-3 | Diagnostic logger | same | one JSONL record per diagnostic: offset, tool, toolchain tag, code, message, spans, file hash; a content-addressed source snapshot on every ownership rejection (for A2) |
| I-4 | Weekly diary | same | per-event answers and weekly questions (section 6.2) |
| I-5 | Help-desk transcript | same | every question and answer, verbatim |
| I-6 | Exit questionnaire | corpus authors, red team, porters, C1 author | T1.4 item, the Q-50 item, diagnostic ratings, the Q-28 item for P4 authors, open comments |
| I-7 | Codebook and coding sheet | raters | category, family and named feature per event |
| I-8 | Masking script | PI, before coding | masked events in random order |
| I-9 | Audit and count tools | team; rerun by the referee | `sisu audit --ownership`, `sisu audit --caps`, `--stats shared`, `--stats stack-depth`, the binding count (A3), the Rust escape-hatch counter, the Go AST tool, the line counter |
| I-10 | Analysis scripts | PI writes; referee runs | every number in the report |

`sisu check --diary` (18, Phase 3) is I-3 for Sisu. For the controls, I-3
is a wrapper around `cargo check`, `cargo build` and `cargo test`, `go
build`, `go vet` and `go test`, and the Kotlin and Swift build and test
commands, writing the same JSONL [default proposed; owner to confirm].
Everyone in one study works in the same container image holding the
toolchains, the loggers and the suites; editors are free, and an author's
use of an editor-integrated checker (such as rust-analyzer) is recorded.
Errors such a checker shows before a save are not logged, which can only
lower the control's counts, a bias against H1 that the report states
[default proposed; owner to confirm].

### 4.6 The desk-audit protocol (early-frozen)

- **Crates.** About ten real crates spanning the corpus domains (a regex
  engine, a parser-combinator library, a graph library, an immediate-mode
  GUI, a hyper-based service, a JSON library, slotmap users), named with
  exact versions: `[owner: crate list with versions]`.
- **Tier.** Each crate is labelled before classification starts:
  application-tier if its primary artefact is a binary or it is the
  application code of a service or GUI, library-tier otherwise [default
  proposed; owner to confirm].
- **Unit.** Each explicit lifetime in item signatures, type definitions,
  impl headers and where-clauses: named lifetime parameters and their uses,
  `'static` and `for<'a>` binders. `'_` and lifetimes in tests, examples,
  benchmarks and macro bodies are excluded [default proposed; owner to
  confirm].
- **Categories (18).** Expressible in Sisu: elided by the
  one-borrowed-parameter or `self` rule; a `from` clause; a
  single-lifetime `view struct`; a scoped task borrow. Not expressible: a
  multi-lifetime struct; references stored in long-lived containers;
  self-referential; `'static` closures with no natural `own` capture. Plus
  "unclassifiable", which counts in the denominator but not the numerator
  [default proposed; owner to confirm].
- **Share and threshold.** Not-expressible lifetimes divided by all
  lifetimes in application-tier crates; 20% or more pre-builds the
  generation-checked reference fallback (Q-1's fourth option,
  [`vale.md`](../research/languages/vale.md)) behind a flag in Phase 3.
  Library-tier shares are reported.
- **Classifiers.** The desk-audit contractor classifies everything. The
  referee classifies a 10% random sample without seeing the contractor's
  labels; Cohen's kappa is reported; the threshold is applied to the
  contractor's labels [default proposed; owner to confirm].

### 4.7 The materials register

Some materials can only be produced after freeze. They are produced
according to specifications frozen here and registered, with a date and a
content hash, in `experiments/REGISTER.md` before they are used.
Registration is not an amendment, because it changes no rule; a material
that cannot be produced as specified needs an amendment [default proposed;
owner to confirm]. Registered later: the executable suites and drivers;
each toolchain tag; the analysis scripts, after the referee has reviewed
them and before the data they analyse exist; the line counter and audit
tools; the masking script; P6's choice; the translated integration tests
and the ten test specifications; the red team's pattern substitutions; the
planted copies of P3 and L (section 4.9), registered by hash before the
hunt and published after it; and the model-arm models, prompts, pocket
reference and digest.

### 4.8 Model-arm materials (A6)

- **Tasks.** P1-P6 (18), with the specification text the human authors
  received, stripped of any reference to the test suites [default
  proposed; owner to confirm].
- **Hidden tests.** The same behavioural suites, never shown to a model.
- **Context conditions (N-50).** Nothing; the pocket language reference
  (about 25k tokens, std about 50k); the reference plus the API digest
  written by `sisu doc --digest` for std and `std.x.http`.
- **Feedback conditions (N-50).** Without compiler feedback: one round.
  With it: up to five rounds, each returning `sisu check`'s diagnostics
  for the previous round's program [default proposed; owner to confirm].
- **Python baseline.** The same tasks in Python with the same hidden tests
  through a Python driver, with and without feedback, so each task and
  model has eight cells: six Sisu conditions and two Python ones. Python's
  "compiler feedback" is the output of byte-compiling and importing the
  module, the nearest equivalent [default proposed; owner to confirm].
- **Models.** Three generally available frontier models from three
  different providers, chosen by the PI and the referee at the start of
  Phase 9 and registered with exact version identifiers before the first
  attempt [default proposed; owner to confirm]: `[owner: models]`.
- **Samples and budget.** Five attempts per task, model and cell, so 6
  tasks x 3 models x 8 cells x 5 = 720 attempts; each provider's default
  sampling settings, recorded; at most 200k tokens per attempt, so at most
  144M tokens in all [default proposed; owner to confirm]. The money is
  capped by the pre-registered API budget, `[owner: model-API budget]`
  (18); if the cap is reached, the remaining attempts are dropped in a
  pre-drawn random order of cells, and the report says which.
- **Sandbox.** The playground's sandbox: no network, an in-memory `Fs` and
  a frozen `Clock` (18, Phase 9). No retrieval beyond the condition's
  context; no participant program is ever in a model's context.

### 4.9 The seeded bugs (A4)

18 puts the twenty bugs that the `sim` study plants into the
pre-registration, chosen from the ALICE and Jepsen catalogues before the
profile exists, so that they cannot be chosen to suit it.

- **Source.** Crash-consistency bugs from the ALICE catalogue and
  concurrency or partition bugs from the Jepsen analyses, ten of each,
  each assigned to P3 (a server with a shared cache and an eviction task)
  or to L (a small replicated log) by where its mechanism fits [default
  proposed; owner to confirm the ten-and-ten split].
- **Selection rule.** Each bug has a public write-up, a known trigger (a
  crash point, an interleaving, a partition or a failed `fsync`), and a
  mechanism that Sisu code in P3 or L can reproduce without `unsafe`. The
  PI selects under this rule; the referee checks the list against it.
- **Sealing.** So that the engineer building the profile cannot tune it to
  the list, the list is sealed: the PI and the referee keep its text
  outside the repository, and its SHA-256 hash is recorded here at freeze:
  `[to be filled at freeze]`. The text is published with the study's
  results and must hash to the recorded value [default proposed; owner to
  confirm; the alternative is to list the twenty here in the open].
- **Planting.** The compiler engineer, who does not build the profile,
  plants each bug alone in its own copy of L or of one P3 implementation
  (drawn at random among those that pass their suite) in weeks 44-45, so
  that every failure belongs to one bug. The planted copies are registered
  by hash (section 4.7). The language engineer runs the hunt without
  knowing which bug is in which copy [default proposed; owner to
  confirm].
- **Hunt and count.** Each planted copy is hunted by `sisu test --sim` and,
  separately, by random `--explore` schedules alone, each under the budget
  of section 2.5. A bug counts as found when a run fails and the planter,
  comparing the failure with the sealed description, confirms that it is
  the planted bug. The count is recorded at week 46.

## 5. Procedure

### 5.1 Phases, with the amendments in place

The phases and weeks are 18's 52-week plan. Only the work the evidence
depends on is listed; 18 has the full deliverables.

| Phase | Weeks | Team work the evidence depends on | Participants | Data and gates |
|---|---|---|---|---|
| 0 | 1-4 | lexer, error-tolerant parser, diagnostics catalog, query skeleton, SIR v0, interpreter loop; the PI writes the specifications, instruments, consent forms and the draw scripts, shortlists and draws the Go service, posts the call (21) | desk-audit contractor classifies (weeks 1-4); reviewer fixes the control versions (week 1); referee reviews and signs (weeks 2-4) | early protocols frozen (end of week 1); desk-audit prior applied and full freeze (end of week 4) |
| 1 | 1-8 | the runtime engineer builds `taskbench`, runs strategies A-E and the controls after the full freeze, and runs the io_uring cancellation test (**A1**) on each stackful strategy | reviewer configures and reviews the controls and reproduces every configuration on M2 | week 8: bet-3 harness verdict and cancellation result, run by the referee; Q-6 provisional; any stackless flag goes to Phase 3 |
| 2 | 5-10 | core language, errors, Drop, the 40-file wrong-program corpus | none | |
| 3 | 9-16 | ownership checker, dynamic monitor, the counterexample mode (**A2**, team-only) run over the wrong-ownership corpus, `sisu check --diary`, `sisu audit --ownership`, dry-run drafts; the generation-checked flag if the desk audit tripped | referee writes the 20 adversarial programs (weeks 9-14) | week 16: Phase 3 exit (all adversarial programs rejected, zero monitor violations, 24 hours of mutation fuzzing); outsiders may start |
| 4 | 15-22 | traits, generics, `dyn`, `@delegate`, std in Sisu; the tag P4 needs by week 22 | none | |
| 5 | 16-32 | 24-hour triage; every ownership rejection rerun in the counterexample mode (A2); the binding count (**A3**) on final programs; 24 more hours of mutation fuzzing at week 32 | corpus authors (onboarding weeks 14-15; P1, P2, P5, P6 from week 16; P4 from 22; P3 from 26); red team (weeks 20-25); raters (weeks 23-26 and 31-32) | week 24: interim (section 7.9); week 32: bet-1 verdict and branch |
| 6 | 19-32 | tasks, scheduler, `std.cap`, fakes, `std.x.http`, `sisu audit --caps`; 1000 random schedules of T; the first half of the `sim` profile (**A4**: fault-injecting fakes, PCT and bounded exhaustive scheduling, weeks 31-32) and T under it | none | P3's tag by week 26; Q-34's interpreter number and T's counts by week 32 |
| 7 | 30-40 | std gaps, host bugs, the Go AST tool rerun on the original | porters (onboarding week 29, ports weeks 30-38, ten tests weeks 38-39); C1 author (weeks 28-34) | week 40: bet-2 verdict and Q-5; Q-50 count and the frame-depth histogram over every participant program |
| 8 | 33-46 | test surface; the rest of the `sim` profile (**A4**: virtual clock, several nodes, simulated crash and restart) and T under it; L written; the capability log, `sisu replay` and replay tokens (**A5**); the seeded bugs planted (weeks 44-45) and hunted | none | week 46: seeded-bug count; replay row measured |
| 9 | 44-52 | the report; the decisions; the playground, the pocket reference and `sisu doc --digest`; the model arm (**A6**, weeks 46-50, run by the PI); 0.1-interp | referee's review (weeks 48-51) | week 52: report with the pre-registration hash, verdicts, every branch with an owner and a date; release |

### 5.2 What each participant does

**Desk-audit contractor (weeks 1-4).** Classifies every lifetime in the
early-frozen crate list under section 4.6, records one row per lifetime,
and delivers the per-crate category counts by the end of week 4.

**Go/Java runtime reviewer (weeks 1-10).** Week 1: fixes control versions,
flags and the coroutine library for the early protocol. Weeks 2-6:
configures and reviews each control in `taskbench`. Weeks 6-8: reproduces
every configuration on M2 and writes a short review of the controls'
fairness, published unedited. Weeks 9-10: reruns, only if a section 8.3
branch reruns the harness.

**Referee (weeks 1-52).** Weeks 2-4: reviews this document and the early
protocols, classifies the desk-audit sample, co-signs the freeze. Weeks
9-14: writes the 20 adversarial programs. Weeks 8, 24, 32, 40, 46 and 50:
runs the registered comparisons from the raw logs and commits the outputs; at
weeks 32 and 40 also audits the triage labels and classifies god-struct
candidates. Whenever asked: co-signs or refuses amendments. Weeks 48-51:
writes the review. Week 52: confirms that the report cites the hash and
lists the amendments.

**Corpus authors (weeks 8-32).**

1. Sign the contract and the consent form by week 8.
2. Onboarding, weeks 14-15: the same written material for everyone (the
   tour in the indentation syntax and the toolchain guide) and a fixed
   set of warm-up exercises, about four hours, whose logs are not data
   [default proposed; owner to confirm].
3. For each of their two programs, in the drawn order: write the first
   implementation in the provided environment until its suite passes, the
   time box is reached, or they declare it blocked; then the second
   implementation the same way. Program windows: P1, P2, P5 and P6, weeks
   16-23; P4, weeks 22-27; P3, weeks 26-30 [default proposed; owner to
   confirm].
4. Each week: fill the diary for that week's events and answer the weekly
   questions (section 6.2).
5. Ask questions only through the help desk, whose transcript is
   published. No other channel to the team exists for the work.
6. Within a week of the last implementation: the exit questionnaire.
7. Review their own diaries before publication (section 3.7).

**Red-team author (weeks 20-25).** Implements each pattern of section 4.1.1
in Sisu, with its Rust version, keeps the same logs and diary, and writes a
per-pattern report under the headings of section 4.1.1.

**Friction raters (weeks 23-32, and one day at week 41).** Week 23:
calibrate on the dry-run drafts' logs until their agreement on a
calibration set reaches 0.6. Weeks 24-26: code batch 1 (P1, P2, P5, P6 and
R) independently, then reconcile. Weeks 31-32: code batch 2 (P3, P4).
Week 41: code the porters' and the C1 author's events whose diagnostic
codes are in the top ten of section 6.7, for Q-9 only [default proposed;
owner to confirm].

**C1 author (weeks 28-34).** Grows C1 from `wordcount.sisu` to its
specification with the same logs, diary and exit questionnaire.

**Porters (weeks 29-40).** Week 29: onboarding as for corpus authors.
Weeks 30-38: port the service and run its translated integration tests,
tagging time in the diary (section 6.2). Weeks 38-39: write the ten
pre-specified tests against the Sisu fakes; then porter S1 writes the same
ten against the Go original with its existing mocking approach. Week 40:
exit questionnaire.

**Reserve authors (weeks 8-40).** On retainer; activated under section 7.5
or for a re-run branch of section 8.1, they follow the corpus-author
procedure for the slot they fill.

### 5.3 Slips

A slip moves dates, never thresholds or rules. If a phase slips, every
date that depends on it moves by the same amount, the new dates are
recorded as a deviation, and no rule changes. 18 budgets a 50% slip on
Phase 3, which would move the outsiders' start and everything after it by
up to four weeks (52 weeks nominal, 56 with the slip, as 18 says).

### 5.4 Order of the decision points

1. End of week 1: early protocols frozen (section 1.3, rule 3).
2. End of week 4: the desk-audit prior is applied (20% or more pre-builds
   the generation-checked fallback behind a flag); full freeze.
3. Week 8: bet 3 in the harness era (memory primary, spawn cost, scale,
   echo, starvation, exhaustion tests) and the cancellation test (A1);
   Q-6 closes provisionally or the stackless fallback is flagged to Phase
   3 before the checker's design is frozen.
4. Weeks 9-10: the harness reruns, if a section 8.3 branch requires it.
5. Week 16: Phase 3's exit gates; outsiders start.
6. Week 24: the bet-1 interim (section 7.9).
7. Week 32: the bet-1 verdict and its branch; the interpreter-era
   starvation number and T's counts under random and `sim` schedules.
8. Week 40: the bet-2 verdict and Q-5; the exhaustion guard; the Q-50
   count.
9. Week 46: the seeded-bug study (A4) and the replay row (A5).
10. Week 50: the model arm's results (A6).
11. Week 52: the report, every branch with an owner and a date, the
    parallel questions in [`open-questions.md`](../open-questions.md).

## 6. Measures

Every measure below says what is counted, from which log or tool output,
and by whom. Where 18 fixes the definition it is restated; where 18 leaves
a choice open, the choice is marked.

### 6.1 Common definitions

| Term | Definition |
|---|---|
| Participant program | a program written by a participant of section 3.2; team-written programs are never participant programs |
| Implementation | one program written by one participant in one language |
| Final version | the last version committed before the author declares the implementation done or blocked, or reaches the time box |
| Final lines | non-blank, non-comment source lines of the final version, excluding test code (test blocks, test files, the shared suite and its driver), counted by the registered line counter with the same rules in every language [default proposed; owner to confirm] |
| Complete | the final version passes every test of its suite under its frozen tag |
| Blocked | stopped before passing, with a blocking construct named by the author |
| Incomplete | stopped before passing with no blocking construct named (the time box, a dropout) |
| Time box | 60 active hours per implementation, 1.5 times 18's budget of about one week per implementation [default proposed; owner to confirm] |
| Active time | the sum, over consecutive activity events in a session (save, check, build, test run, help-desk message), of the gap between them, each gap capped at 15 minutes [default proposed; owner to confirm] |
| Time to green | active time from the start of the implementation's first session to the first test run in which every test of its suite passes |
| Ownership family | diagnostics and diary events about moves, borrows, references, lifetimes, views, escapes, `Send`/`Sync` or drop order, in any language; assigned by the raters (section 6.2) |
| Ordinary pattern | any pattern needed to meet the program's specification. The interpreter has no FFI and no unsafe operations, so every `unsafe` in a participant program counts, unless triage records it as a workaround for a checker bug [default proposed; owner to confirm] |
| Ordinary program | a participant program of bet 1 or bet 2 (P1-P6, C1, S1, S2), excluding R and the deep-recursion tests (18: "ordinary (bet-1 or bet-2) program") |
| Capability type | `System`, `Fs`, `Net`, `Clock`, `Rand`, `Env`, `Proc`, their attenuations, and the `std.cap` traits that fakes implement |
| I/O-doing function (Sisu) | a function or method that calls a method of a capability type, or calls, directly or transitively within the program, a function that does [default proposed; owner to confirm] |
| Capability parameter (Sisu) | a parameter other than the receiver whose type is a capability type, or a struct or tuple containing one; such a struct counts as one parameter [default proposed; owner to confirm] |
| I/O-doing function and its parameters (Go baseline) | the same definitions over the Go original: I/O is a call into the standard library's network, file, process, environment, clock or randomness APIs, or a method of an interface whose implementations make one; a capability parameter is a non-receiver parameter of type `context.Context`, or of an interface or pointer type that gives access to I/O, from a type list fixed with the Go AST tool at freeze [default proposed; owner to confirm] |

### 6.2 Logs, diaries and the coding scheme

**Log events.** I-3 records every diagnostic of every check, build or test
run. An event is a diagnostic with one code in one enclosing function, from
the first run in which it appears to the first run in which it no longer
does; repeats in between are the same event. Warnings are logged but are
not events [default proposed; owner to confirm].

**Diary events.** A diary entry that describes an obstacle, a
restructuring or a missing feature and is not linked to a log event is an
event of its own. An entry linked to a log event is part of that event.

**The weekly diary (I-4).** For each of the week's log events the author
answers:

1. Did you understand the message? yes / partly / no.
2. What did you do? fixed a mistake / restructured the design / added a
   wrapper or a copy (`Shared`, `Cell`, an arena handle, `.copy()`, or the
   control language's equivalent) / asked the help desk / abandoned the
   approach / other, with a note.
3. Would you need `unsafe` here? yes / no.
4. About how many minutes did it take?
5. An optional note.

Each week the author also answers: What slowed you down most? Is there
anything you could not write without a feature the language lacks, and
which feature? Did you choose any design for performance reasons, and
which? (18's risk table asks diaries to record performance intent.)
Porters add their hours on capability plumbing (adding, passing or
attenuating capabilities, wiring fakes), on other porting, on tests and on
toolchain problems. P4 authors add: did you emulate inheritance anywhere,
and where? The questions and their order are fixed at freeze [default
proposed; owner to confirm].

**The codebook (I-7).** Two raters code every event of every participant
implementation, Sisu and controls alike, on three fields.

| Field | Values |
|---|---|
| Category | **(a)** diagnostic unclear: the time went into understanding the message, and the eventual fix kept the design. **(b)** a rule rejected a sound design and the author restructured: the rejected code would not have been a memory-safety or type-safety error, and the author changed the design, not only a line, to satisfy the checker. **(c)** checker bug: taken from the team's triage record; raters neither assign nor remove it. **(d)** missing library: the obstacle was a missing library function or module. **(e)** correct rejection: the code was wrong (a real use after move, an aliasing error, a type error, a typo) and the author fixed it without restructuring. **(f)** not codable: too little information. |
| Family | ownership / other (section 6.1) |
| Named feature | for "could not be written without X" entries: X, normalised to a fixed list (named lifetimes, first-class references, inheritance, and so on) that the raters extend only by agreement |

Categories (a)-(d) are 18's. (e) and (f) are added because every event
needs a code and most checker errors are correct rejections [default
proposed; owner to confirm].

**Coding procedure.**

1. **Masking.** I-8 replaces language and tool names, file extensions and
   error-code prefixes with neutral tokens, and interleaves events from
   all languages in random order.
2. **Calibration** (week 23) on the dry-run drafts' logs, which are not
   data, until kappa on a calibration set of at least 100 events reaches
   0.6 [default proposed; owner to confirm].
3. **Independent coding** of every event by both raters.
4. **Agreement.** Cohen's kappa on the category field over all events.
   Kappa of at least 0.6, or the rating is redone with a refined codebook
   (18). A refinement may clarify definitions and add examples; it may not
   add, remove or merge a category; both codebooks are published, and
   every event is recoded [default proposed; owner to confirm].
5. **Reconciliation.** The raters discuss each disagreement and record a
   consensus code; with no consensus the event is coded (f). The primary
   counts use consensus codes; counts from events both raters coded alike
   before reconciliation are reported as a sensitivity analysis [default
   proposed; owner to confirm].
6. **Triage audit.** A (c) label stands only if the triage record links a
   fix released inside the study; otherwise the event takes the raters'
   code. The referee audits the (c) list and may send any (c) label to the
   raters [default proposed; owner to confirm].
7. **After rating only**, the candidate false positives of A2 are set
   beside categories (b) and (c) (18), as a cross-tabulation.

### 6.3 Bet 1

| Row | Operational definition | Source | By |
|---|---|---|---|
| T1.1 | A program needs `unsafe` if any of its Sisu implementations has an `unsafe` block in its final version, or a diary "would need `unsafe`" answer on an event not coded (c). P2 or P4 is unwritable if no Sisu implementation of it is complete without an `unsafe` block; a blocked implementation names its blocking construct, and the referee confirms it is a language limitation rather than a checker bug or a missing library [default proposed; owner to confirm]. Value: how many of the six programs need `unsafe`, and a flag each for P2 and P4 | `sisu audit --ownership`, I-4, the triage record | audit tool; referee |
| T1.2 | Per implementation: occurrences in type positions and constructor calls of non-test code of `Shared`, `Weak`, `Cell` and arena handle types (Sisu), or of `Rc`, `Arc`, `RefCell`, `Weak`, `Cell`, arena and slotmap handle types and index newtypes used as handles (Rust), per 100 final lines; Rust's `Cell` is added to 18's list to mirror Sisu's [default proposed; owner to confirm]. Per program: the Rust-arm author's Sisu density over the same author's Rust density, each count plus 0.5. A program counts when its ratio is above 1.5; T1.2 fails at three or more | I-9; the Rust counter flags candidate index newtypes | audit tools; raters confirm index newtypes; referee |
| T1.3 | the fighting-the-checker rate, below | I-3, I-4, codes | raters; referee |
| T1.4 | the exit item of section 6.6, answered for Sisu by all six authors and for Rust as well by the Rust-arm authors; counts out of n beside 10.6% | I-6 | PI |
| T1.5 | Per pair: time to green in Sisu over time to green in the control. Value: the median of the six Rust-arm ratios (fails above 1.0) and the median of the six non-Rust ratios (fails above 2.0). Reported for first-written and second-written implementations separately, which is how this document reads "second-written language" in 18's row [default proposed; owner to confirm] | I-2 | referee |
| Desk-audit prior | section 4.6 | contractor's sheet | contractor; referee's sample |
| View-struct splits | per program, the places where an author split one view struct into two or more because it needed references of more than one origin; flagged above 2 | I-4, version history | raters |
| Designer interventions | per program, help-desk answers after which the author changed the program's design; flagged above 3 | I-5, version history | raters |
| Missing-feature entries | the named-feature field; "named independently by two authors" means named by two different participants in their own diaries [default proposed; owner to confirm] | I-4 | raters |
| Checker soundness | internal soundness errors raised by the monitor on programs the static checker accepted: every run of every participant program and suite during the studies, plus 24 hours of mutation fuzzing at Phase 3's exit and 24 more at week 32 [the second run: default proposed; owner to confirm]; each tagged "involves views" or not | interpreter logs | compiler engineer; referee |
| Candidate false positives (A2) | section 6.8 | | |

**The fighting-the-checker rate (T1.3).** For implementation *i*, let
*F(i)* be its number of events with consensus category (b) and family
"ownership", and *L(i)* its final lines; its rate is 100 F(i)/L(i). For
each Rust-arm pair (one author, one program) the ratio is

> ((F(Sisu) + 0.5) / L(Sisu)) / ((F(Rust) + 0.5) / L(Rust)),

where the 0.5 keeps a pair with no events defined [default proposed; owner
to confirm]. T1.3 is the median of the six pair ratios. It passes at or
below 0.5 and is "not materially below Rust" above 0.8. Also reported:
every pair's rates and ratio; the pooled ratio (all Sisu events over all
Sisu lines, against the same for Rust); the non-Rust authors' Sisu rates
against their home-language rates; and the (b) events outside the
ownership family.

### 6.4 Bet 2

| Row | Operational definition | Source | By |
|---|---|---|---|
| T2.1 | for every I/O-doing function in the final versions of S1, S2, both P3 implementations and C1, its number of capability parameters (section 6.1); pooled median and maximum, and each program's own; baseline: the Go AST tool over the original with the same definitions | `sisu audit --caps`; the Go AST tool | PI runs; referee reruns |
| T2.3 | every struct with four or more distinct capability types among its fields, directly or through nested structs, taken from `sisu audit --caps` (which lists those with three or more). The composition root is `main` and the functions `main` calls to construct long-lived values before the first request or main loop [default proposed; owner to confirm]. The referee decides for each listed struct whether a value of it is created or passed outside the root; any such struct trips T2.3 | `sisu audit --caps`; the code | referee |
| Threading depth | for each call of a capability method, the length of the shortest call path from `main` along which that capability, or an attenuation of it, is passed as an argument; median and maximum. Go: the same for `ctx` from the function that creates it | `sisu audit --caps`; the Go AST tool | PI; referee |
| Plumbing share | a porter's plumbing hours over all their hours on the port; flagged above 20% | I-4 time tags | PI |
| Test-writing time | active time from the first session on the ten tests to the run in which all ten pass. The ratio is porter S1's Sisu time over S1's Go time with mocks: above 1.0 flags, above 1.25 redesigns the fakes. S2's Sisu time is reported | I-2 | referee |
| Supply-chain demonstration | a registered script builds the planted dependency against each port; it must fail to type-check unless handed `Fs` and `Net`; the attenuated capabilities each port passes to each dependency are listed. Passes if the script reproduces on a clean checkout | script output | referee runs |

### 6.5 Bet 3

**Harness protocol.** The harness rows of the table below, with section
4.4, are the protocol frozen early under section 1.3. Each measurement cell
(runtime or strategy, task count, condition, machine) runs five times in
fresh processes; the cell's value is the median, and the range is reported
[default proposed; owner to confirm]. The runtime engineer runs M1, the
reviewer runs M2, and the referee applies the thresholds to the committed
CSVs. Interpreter-era rows are run by the team and judged by the referee.

| Row | Operational definition |
|---|---|
| Memory per idle task | After runtime start-up and a warm-up of 1,000 tasks, read the baseline resident set *B* from `/proc/self/smaps_rollup`. Spawn *N* tasks (10k, 100k and 1M; the threshold uses 100k and 1M). Each task either touches under 1 KiB of stack (shallow) or recurses until it has touched 32 KiB and returns (excursion), then parks on its own wait primitive (a channel receive, a virtual-thread park, a oneshot). When all *N* are parked, the controls run a full collection (`runtime.GC()`, `System.gc()`), and one second later the resident set *R* is read. Memory per idle task is (*R* - *B*)/*N*; the ratio is a strategy's value over Go's for the same *N*, condition and machine [default proposed; owner to confirm] |
| Spawn-plus-join | the latency of spawning a task that returns at once and joining it, timed per pair with a monotonic clock over 100k sequential pairs after 10k warm-up pairs; median and p99; the ratio uses pooled stacks (B) against Go |
| Scale | create 1M tasks that park, with default sysctls, inside a 16 GiB memory cgroup; pass if all park and the process survives 60 seconds; fail on an allocation or `mmap` failure or an out-of-memory kill; per strategy; the maximum creatable tasks is also reported |
| Probe overhead | three compute benchmarks (recursive Fibonacci, a call-heavy tree walk, a tight loop of small calls) with and without software stack-limit probes, ten runs each; overhead is the ratio of medians minus one; at most 3% [benchmark list: default proposed; owner to confirm] |
| Echo latency and throughput | a TCP echo server with 10k concurrent loopback connections; the load generator on cores disjoint from the server's sends 64-byte messages open-loop at a fixed total rate, half the slowest control's saturation throughput on that machine; 10 s warm-up, 60 s measured; latency from send to complete echo at the client; p50, p99, p99.9; throughput from a separate closed-loop saturation run; the ratio is a strategy's p99 over Go's [default proposed; owner to confirm] |
| Idle RSS at 100k connections | 100k connections opened and left idle for 30 s; resident set minus baseline, over 100k |
| Stack exhaustion, harness | a 10k-deep recursive-descent input and a 1M-deep naive recursion in a task; pass if the task fails with a clean "stack exhausted" error and the process survives; any segfault is a bug |
| Starvation, harness | four compute-bound tasks per worker that yield every K iterations, K being the yield-point budget the design will use (recorded in the early protocol), and also 10K and 100K; one probe task that sleeps 1 ms in a loop and records how late it wakes; p99 lateness over 10k wake-ups, compared with Go running the same loops [default proposed; owner to confirm] |
| Cancellation with a lent buffer (A1) | On each stackful strategy and machine, 1M cancelled reads in total, in batches of at most 10k in flight at once. In a batch, each task submits an io_uring read into a 4 KiB buffer on its own stack from one of a pool of pipes that hold no data, so the read stays in flight, and is then cancelled through its scope. The cancelled call submits an asynchronous cancel and, when that returns `-EALREADY`, waits for the original completion before it returns. The task then writes a canary over the buffer, whose memory stays quarantined (mapped, never reused) until its batch ends. Enough data are then written into every pipe that any read the kernel still holds completes, and every canary of the batch is checked. Value: the buffers whose canary changed, plus the calls whose original completion arrived after the call returned; the threshold is zero. The cancel-to-return latency (median, p99) is reported [procedure: default proposed; owner to confirm] |
| Stack exhaustion, interpreter | `--stats stack-depth` over every participant program's suite and every team suite; per program, the maximum frame depth and an estimate of native stack use: the sum, over the frames of the deepest stack, of 16 bytes per SIR local plus 64 bytes [default proposed; owner to confirm]; compared with the default reservation chosen at week 8 |
| Starvation, interpreter | `--stats`' p99 scheduling latency of an I/O task under a compute-bound sibling (Phase 6's Q-34 number), against 10 ms |
| Semantics | the suite T under `--explore 1000`, then under PCT and bounded exhaustive schedules with injected faults (Phase 6), then with simulated crash and restart (Phase 8): counts of orphan tasks, unobserved failures (a panic no join reports), cancelled subtrees not joined before their scope returns, and lock guards not dropped on an unwinding path, from the interpreter's own accounting under the monitor |
| Seeded-bug study (A4) | the number of the twenty bugs of section 4.9 found by `--sim`, and by random `--explore` alone, each within two CPU-hours per bug on M1 |

### 6.6 Questionnaires

**Entry questionnaire (I-1).** Role; main languages and years in each;
professional experience band; Rust experience in lines and years (non-Rust
arm); which parts of the Sisu proposal they have read; the conflict
declarations of section 3.4.

**Exit questionnaire (I-6)**, in this order:

1. **T1.4**, verbatim from the Rust survey's productivity-limiters question
   ([`surveys.md`](../research/evidence/surveys.md), 2024 report, n =
   6,862) with "Rust" replaced by "Sisu": "Which of the following aspects
   of Sisu present non-trivial problems to your programming productivity?
   Borrow checker rejecting valid code: big problem / could be improved
   but does not limit me / not an issue." The Rust-arm authors then answer
   the same item for Rust [default proposed; owner to confirm]. 18
   abbreviates the scale as "big problem / could be improved / not a
   problem"; the survey's own words are used.
2. **The Q-50 item** (section 6.8).
3. **Diagnostic ratings.** For the five Sisu diagnostic codes the author
   saw most often, each shown with one real example from their log:
   helpful / neutral / unhelpful [default proposed; owner to confirm].
4. **The Q-28 item**, P4 authors only: "For the widget tree, composition
   with `dyn` and `@delegate` felt: natural / workable with boilerplate /
   I emulated inheritance."
5. Open comments.

Porters and the C1 author answer items 2, 3 and 5.

### 6.7 Parallel questions and health

| Row | Operational definition |
|---|---|
| Q-2 | the share of static `Shared(...)` construction sites in participant programs that `--stats shared` tags task-local; validated by the monitor, for which a tagged site crossing a task is a soundness error |
| Q-3 | `.copy()` call sites in the non-test code of P1-P4, P6, C1, S1 and S2, per 30 final lines, pooled; P5 and R are excluded as library and adversarial code [default proposed; owner to confirm] |
| Q-9 | the ten diagnostic codes with the most events across all participant Sisu programs; for each, the share of its events coded (b); a code is "caused by a design rule rather than a bug" when most of its events are coded (b) [default proposed; owner to confirm] |
| Q-10 | lines of P5 duplicated between a by-ref adapter and its by-value variant, found by a token-level clone detector registered with the tools; reported only |
| Q-11 | single-field structs that carry `@delegate`, or that wrap a type from another package to implement a trait from another package, per 200 final lines, pooled over participant programs [default proposed; owner to confirm] |
| Q-28 | P4's exit item and the raters' coding of P4 diary events about inheritance; "boilerplate" means any P4 author answering "I emulated inheritance", or any P4 event the raters code as emulating inheritance [default proposed; owner to confirm] |
| Q-50 | section 6.8 |
| Model arm | section 6.9 |
| Parser fuzzing | counted by CI over the fuzz corpus: panics; diagnostic storms (more than ten diagnostics for an input with one injected syntax error); trees that do not reprint their input byte for byte; inputs the checker leaves untyped [storm definition: default proposed; owner to confirm] |
| `sisu check` and LSP | CI: the median over 20 edits of one randomly chosen body in the std-plus-corpus tree; the LSP's median hover and diagnostics latency on the corpus tree |
| Incremental rebuild | CI: the median of five rebuilds after touching one file in the type-checker crate |
| Interpreter adequacy | wall time of each program's full suite and of the port's integration tests on the reference laptop |
| Replay | section 6.8 |
| Diagnostics quality | the share of borrow and `Send`/`Sync` diagnostics in W whose message names a fix; a message is "rated unhelpful by two authors" when two different authors rate its code unhelpful in exit item 3 |
| Process | the checklist of section 11 |

### 6.8 The counterexample runs, the binding count and replay

**Candidate false positives (A2).** For every ownership-family rejection
in a participant's Sisu log, the compiler engineer reruns the source
snapshot that I-3 stored, in the counterexample mode, across the program's
suite. Each rerun ends in one of three outcomes: conflict reached (the
monitor stops at the concrete conflict with its values); no conflict
(every test runs and none reaches a conflict: a candidate false positive);
or inconclusive (the snapshot does not run far enough to reach the rejected
code, for unrelated reasons). Each candidate false positive is explained,
by the compiler engineer with the referee, as an untested path, or logged
as a false positive. Reported: the share of each outcome and, after
rating, the cross-tabulation with categories (b) and (c). Participants
never see any of it. There is no threshold; the log is what the friction
branch's rule refinements draw on (18).

**Bindings typed by a later statement (A3, Q-50).** For each `let` or
`var` declaration in the non-test code of the final version of every
participant Sisu implementation, the type checker records whether the
binding's type still holds an unresolved inference variable at the end of
its declaring statement that a later statement resolves. One count per
declared name; `for` variables and pattern bindings are not counted,
because their types come from what they bind [default proposed; owner to
confirm]. The value is the pooled share, computed at week 40. **The
friction item** (exit item 2): the author is shown up to three of their own
bindings that the rule would require annotating, drawn at random from the
count run on their final versions at exit, and asked
"Would writing a type annotation on these have been a problem? big problem
/ could be improved but does not limit me / not an issue." The item is
skipped for an author with none. "No friction reported" means no author
answers "big problem" [default proposed; owner to confirm].

**Replay (A5).** From the day the capability log ships (registered under
section 4.7) to week 46, every failing test in a participant program,
rerun by the team under the recording toolchain, yields a token, and each
token is replayed alone on a second machine, not the one that recorded it
(18). Participants keep their frozen tags, which
do not have the log. Reported: the share of tokens that reproduce their
failure, out of failures that do not hold the real `System`; divergences
in 100 recorded `--explore` runs replayed; and, for P3 serving 10,000
requests over 100 connections, the throughput overhead of `sisu run
--record` and the log bytes per request [load: default proposed; owner to
confirm].

### 6.9 The model arm (A6)

An **attempt** is one model, one task, one cell and one sample; its
prompt, every round's program, every diagnostic returned and its token
counts are logged and published.

| Measure | Definition |
|---|---|
| Compile rate | the share of attempts whose final program type-checks with no error (`sisu check`; for Python, byte-compiles and imports) |
| Hidden-test pass rate | the share of attempts whose final program passes every hidden test; the mean share of hidden tests passed is also reported |
| False-friend bug rate, per row | for each row of the table below, the share of compiling attempts with a hidden-test failure attributed to the row. A failure is attributed to a row when replacing every use of the row's construct by its source-language meaning (Python's, or Rust's for FF-6) makes that test pass, checked by a registered substitution script [default proposed; owner to confirm]. Also reported: how often each row's own diagnostic fired, which shows whether the loud rows work |
| Tokens to a pass | input plus output tokens from the first prompt to the end of the first round whose program passes every hidden test (the hidden tests run on every round's program but are never shown to the model); median over passing attempts; attempts that never pass are counted, not given a number |

**The false-friend table** is D-062's, as the specification holds it at
the model arm's frozen tag. At this draft its rows are:

| Row | Habit, with its source language's meaning | Sisu at this draft |
|---|---|---|
| FF-1 | `xs[-1]` for the last element (Python) | a constant negative index is a compile error with the fix-it `xs[^1]`; a computed negative index traps at run time |
| FF-2 | relying on `dict` or `set` iteration order (Python) | insertion order, as in Python (Q-42), so the row should be empty; measured to confirm |
| FF-3 | `7 / 2` on integers | a type error with a fix-it offering `//` or a float conversion |
| FF-4 | `if xs:` for a non-empty test | a type error; there is no truthiness |
| FF-5 | `a if c else b` | a parse error |
| FF-6 | `-7 % 3` read as Rust's -1 | 2, as in Python; silent for Rust-trained code |

Rows added to the specification's table before the model arm's tag is
fixed are measured too; no row is removed [default proposed; owner to
confirm]. Each Sisu cell is compared with the same model's Python cells on
the same tasks; "far from Python's rates" is section 2.5's 0.8. The arm
carries no inferential test, and every cell's numbers are published.

## 7. Analysis plan

### 7.1 The primary analysis is a threshold comparison

For each bet the referee computes the primary and guard values from the
raw logs with the registered scripts and compares them with the thresholds
of section 2. The verdict follows sections 2.2-2.4 mechanically, and the
branch of section 8 follows the verdict. No significance test drives any
decision. With six authors and two porters, a test has little power to
detect anything but the large effects the thresholds already demand, and a
p-value invites reading a small effect as a result. Comparisons use the
point values defined in section 6; a value exactly on a threshold falls on
the side the threshold's wording names ("at or below 0.5" passes).

### 7.2 Effect sizes and intervals

Every ratio, rate and share is reported with a 95% percentile bootstrap
interval from 10,000 resamples of its unit of independence: the author,
with all of that author's pairs, for human measures; the run for harness
cells; the task for the model arm [default proposed; owner to confirm]. Intervals are
reported, never used for a decision. With six pairs they are wide, and the
report says so in the same sentence as the value.

### 7.3 Variance between programmers

Prechelt found that the variability between programmers in one language
exceeds the difference between languages
([`studies.md`](../research/evidence/studies.md), section 10). The analysis
handles it in six ways:

1. Decisions use within-person comparisons only (section 3.1).
2. Every human measure is reported per participant, one row per author and
   program, before any summary.
3. Next to each language difference the report gives the range of the
   per-author Sisu values; when that range is wider than the difference
   between the Sisu and control medians, the verdict sentence says so
   [default proposed; owner to confirm].
4. Arms are never pooled into one ratio: Rust-arm ratios are against Rust,
   non-Rust ratios against home languages.
5. Order is counterbalanced, and each measure is also reported split by
   Sisu-first and control-first.
6. The bootstrap resamples authors, not events, because one author's
   events are not independent of each other.

### 7.4 Rater agreement

Section 6.2 gives the procedure. Kappa is reported with its interval; the
friction measures are computed only once kappa is at least 0.6, on the
first or the refined codebook.

### 7.5 Dropouts, incomplete tasks and minimum data

1. A **dropout** is a participant who stops before delivering an
   implementation; a withdrawal with deletion (section 3.7) is a dropout
   whose data do not exist.
2. A dropout before 25% of the implementation's time box is used is
   replaced by a reserve author of the same arm, who takes the same slot
   (program and order) and starts afresh; the dropout's partial data are
   published and excluded from the primary statistics [default proposed;
   owner to confirm].
3. A dropout after that point leaves the implementation **incomplete**. It
   is reported and excluded from the ratio statistics, and the slot is not
   refilled unless the program would otherwise have no complete Sisu
   implementation, in which case a reserve author writes it [default
   proposed; owner to confirm].
4. **Blocked** implementations count for T1.1's writability test;
   incomplete ones count only in the report.
5. **Minimum data** [default proposed; owner to confirm]:
   - T1.3 and T1.5's Rust clause need at least four complete Rust-arm
     pairs of the six; T1.5's other clause needs four complete non-Rust
     pairs; T1.2 needs complete Rust-arm pairs in at least five programs.
     Otherwise the measure is "not shown" for insufficient data.
   - Bet 2's median clause needs both ports at 80% or more of their
     translated integration tests; the maximum and god-struct clauses
     count on whatever exists (section 2.3).
   - A bet-3 strategy passes or fails only with every memory cell present
     on both machines; a missing cell makes it inconclusive.
   - A model-arm cell with fewer than three completed attempts on some
     task is reported but not compared.
6. Every dropout, replacement, incomplete and blocked implementation is
   listed in the report with its reason, if one was given.

### 7.6 No metric after freeze

No metric is added after freeze. A number the registered scripts do not
produce does not appear in the report's results, verdicts or decisions.
Observations made along the way may appear only in a separate section
headed "Unregistered observations", in prose and without verdicts, and
they cannot justify a design action, or an amendment that applies to data
already collected [default proposed; owner to confirm].

### 7.7 Who runs the analysis

The PI writes every analysis script before the data it reads exist; the
referee reviews it; it is registered (section 4.7). Scripts read only raw
files under `experiments/data/`, so a number computed from a summary does
not count. The referee runs the scripts at each decision point and commits
the outputs; the PI's runs are reproductions. Where the two differ, the
referee's numbers are the numbers of record and the difference is
explained in the report [default proposed; owner to confirm]. Every number
in the report is regenerated by a CI script from the public data (18).

### 7.8 The multiple-outcome rule

Each bet has one primary threshold. Its other thresholds are guards or
secondaries.

| Bet | Primary | Guards | Secondary |
|---|---|---|---|
| 1 | T1.3 | T1.1; T1.2; soundness violations involving views | T1.4, T1.5, the desk-audit prior, view-struct splits, designer interventions, missing-feature entries, other soundness violations, candidate false positives |
| 2 | T2.1 | T2.3; T2.1's maximum clause on an incomplete port | threading depth, plumbing share, test-writing time, the supply-chain demonstration |
| 3 | memory per idle task | stack exhaustion in ordinary corpus code | spawn-plus-join, scale with its probe overhead, echo latency and idle RSS, starvation, cancellation with a lent buffer, semantics, the seeded-bug study |

1. A bet can be reported as holding only through its primary, with no
   guard tripped.
2. A guard can make a bet fail or keep it from holding; it can never make
   it hold. The guards are the thresholds that 18's decision points name
   as fail conditions.
3. A secondary is reported with its value and triggers its own
   pre-registered action (section 8); it never changes a verdict.
4. After freeze, no secondary is promoted to a primary and no primary is
   replaced.
5. The parallel questions and the amendments' own questions are separate
   questions, each with its own threshold and action; none affects a
   bet's verdict.

### 7.9 The interim analysis at week 24

1. The referee computes T1.1 and P2's writability from P1, P2, P5 and P6,
   using the implementations complete or blocked by the end of week 23.
2. A hard fail on P2 pauses the bet-2 spend before the porters start
   (18): the porters' contracts are paused under their pause clause (21).
3. If T1.1 already fails on those four programs (two or more need
   `unsafe`), the hard-fail branch of section 8.1 starts at week 24, not
   week 32 [default proposed; owner to confirm].
4. Interim values of T1.2 and T1.5 are computed and sealed: the referee
   alone sees them until week 32. T1.3 is not computed before all coding
   is done [default proposed; owner to confirm].
5. No "holds" is declared at the interim, and no rule is adjusted because
   of it.

### 7.10 Era labels

Every bet-3 number carries one of 18's era labels: harness (Phase 1, Rust
code on native stacks), interpreter (Phases 6-9, heap frames) or native
(after the experiment). Numbers of different eras are never pooled. The
memory primary uses harness numbers only; the interpreter-era rows
(frame depth, interpreter starvation, semantics, the seeded-bug study) are
judged on their own. Native numbers arrive after the experiment; they close
Q-6 finally and do not change this report's verdicts.

### 7.11 Reporting template

Every result, positive or negative, is reported under one template (18):
the row and its threshold; the measured value with its interval and n;
the verdict or flag; the action taken, with its owner and date; and every
deviation that touches it.

## 8. Decision rules

The actions below are 18's
[Decision points](18-next-steps.md#decision-points), copied with
cross-references added. 18
asks that each branch be recorded with an owner and a time box before the
data arrive; where 18 gives a time box it is used, and otherwise one is
proposed. Words a branch leaves open are fixed after each table.

### 8.1 Bet 1 (D-004, Q-1), against section 1 of [16-validation-plan.md](16-validation-plan.md)

| Outcome | Action (18) | Owner | Time box |
|---|---|---|---|
| **Holds** (T1.1-T1.3 pass) | D-004 stands; the generation-checked reference flag is removed; the proposal's claim that the model removes most of Rust's learning curve is kept, with the measured numbers attached. | compiler engineer (the flag); PI (the text) | by week 34 [default proposed; owner to confirm] |
| **Hard fail** (more than one program needs `unsafe` for an ordinary pattern, or P2 or P4 cannot be written) | Run the pre-designed generation-checked reference tier on the failing programs with two new authors (four weeks); if still failing, opt-in named lifetimes on `view struct` only, which is 16's second fallback; if that fails, the remaining no-GC option is Swift-style implicit reference types (compiler-counted `Shared` for class-like types). Beyond that is 16's stop clause, the rewrite as "Go with sum types, capabilities and contracts" on a tracing GC. That clause contradicts the owner's no-GC decision (Q-1), so the pre-registration says what happens at that branch: the team escalates the data to the owner as evidence and does not act on it. Agreeing this in Phase 0 is what makes the study able to change the design (section 8.6). | compiler engineer; PI for the re-runs; owner at the stop clause | four weeks for the first step (18); four weeks for each later step [default proposed; owner to confirm] |
| **Friction fail only** (T1.3 or T1.5 fails with expressiveness holding) | If the raters attribute most events to diagnostics (category a), fix the diagnostics and re-run two programs with two new authors (four weeks); if to rules (category b), adopt the specific Rust refinements that fixed the same complaints (two-phase borrows, disjoint closure captures, field-level partial borrows) and re-run; if friction is still not materially below Rust's, the "removes most of the learning curve" claim in [04-memory-and-ownership.md](04-memory-and-ownership.md) is withdrawn, the design continues on the expressiveness result, and the stop clause's second condition is reported to the owner as above. | compiler engineer (diagnostics or rules); PI (the re-run) | four weeks per re-run (18) |
| **Soundness** (any monitor violation on an accepted program) | 0.1 is blocked until fixed; a violation involving views reopens the view and escape rules before any friction number is interpreted, because an unsound checker understates friction. | compiler engineer | before 0.1; for views, before the friction verdict |
| **Not shown** (section 2.2) | Reported as not shown; no fallback is built on it. A generation-checked flag already built stays in the code, switched off, until the native-era rerun [default proposed; owner to confirm]. The friction branch still runs when its trigger is met. | PI | week 32 |

Words fixed here [all default proposed; owner to confirm]:

- **"T1.3 fails"** as the branch trigger means a T1.3 ratio above 0.5, 18's
  table's "not at or below 0.5x Rust: friction branch". The verdict label
  comes from section 2.2; the branch runs whatever the label. **"T1.5
  fails"** means either of its clauses fails. **"Expressiveness holding"**
  means T1.1 passes.
- **"Most events"**: of the ownership-family events coded (a) or (b) in
  the Sisu implementations, whichever category has more; on a tie, both
  actions.
- **The friction re-run**: the two programs whose Sisu implementations had
  the most events of the attributed category; two new Rust-arm authors (the
  reserve, or new recruits meeting section 3.3), each writing one of the
  two programs in Sisu and in Rust in counterbalanced order; the re-run's
  T1.3 is the mean of its two pair ratios, judged on the same thresholds
  and labelled as a re-run with n = 2. "Materially below" is section 2.2's
  0.8.
- **The hard-fail re-run**: two new authors, one per arm, write the failing
  programs with the generation-checked flag on; "still failing" means any
  re-run program still needs `unsafe` or still cannot be written.

### 8.2 Bet 2 (D-015, Q-5), against section 2 of [16-validation-plan.md](16-validation-plan.md)

| Outcome | Action (18) | Owner | Time box |
|---|---|---|---|
| **Holds** | The `with Fs, Clock` clause stays reserved; the supply-chain demonstration and the attenuation counts go into [06-concurrency-and-capabilities.md](06-concurrency-and-capabilities.md). | runtime engineer; PI | by week 42 [default proposed; owner to confirm] |
| **Fails** (median above two capability parameters on I/O-doing functions, any I/O-doing function with more than four, or a god struct outside the composition root) | Implement the reserved `with` clause as 16 says, re-port the worst module (two weeks), recount; if bundling persists, downgrade the supply-chain claim to coarse-grained capabilities and document the composition-root pattern as the idiom. | language engineer (the clause); the porter of the worst module, on a contract extension (21) | two weeks for the re-port (18), weeks 41-44 |
| **Fakes slower than mocks by more than 25%** | Redesign the `std.cap` fakes (recording fakes derived by reflection) before 0.1; the testability claim is reported with the ratio either way. | runtime engineer | before 0.1 |
| **Not shown** (section 2.3) | `with` stays reserved and nothing is built; Q-5 records the numbers. | PI | week 40 |

Words fixed here [default proposed; owner to confirm]: the **worst
module** is the module of either port with the highest median capability
parameters per I/O-doing function; **recount** means the referee reruns
T2.1 and T2.3 on the re-ported module; **bundling persists** means that a
god struct remains after the re-port, or the module's median is still
above two. The `with` decision (Q-5) is taken by week 40 (18).

### 8.3 Bet 3 (D-014, Q-6), against section 3 of [16-validation-plan.md](16-validation-plan.md)

| Outcome | Action (18) | Owner | Time box |
|---|---|---|---|
| **Holds at week 8** | Q-6 is closed provisionally with the harness era label (strategy, default reservation, probes versus guard pages, `spawn` ambient), and finally when the native backend reruns the harness on Sisu frames. | runtime engineer | week 8 |
| **Fails on memory or exhaustion** (above 4x Go's per-task memory, or exhaustion in ordinary corpus code) | Stackless compilation of task bodies behind the same `scope`/`spawn` API, as 16 says. Because a suspended stackless frame is heap state that can hold `ref`s into itself, Phase 3 designs the escape rule to treat yield points as frame boundaries before the corpus study, and the hot-replacement design is re-costed from Phase 1's written estimate. This is why bet 3 runs first. Under D-058 the state machines stay owned by their scope and never become droppable future values (N-07). | runtime engineer; compiler engineer (the escape rule) | weeks 9-16, when decided at week 8 |
| **Fails on creation cost or VMA limits only** | Pooled stacks with software probes become the design; the harness is rerun on that strategy (two weeks). | runtime engineer; reviewer | weeks 9-10 |
| **Starvation** (p99 worse than Go in the harness, or above 10 ms in the interpreter) | Asynchronous preemption is revisited (Q-34). | runtime engineer | by the Phase 9 synthesis [default proposed; owner to confirm] |
| **Cancellation with a lent buffer** (any buffer written after its cancelled call returned, at week 8) | A runtime bug, fixed and the test rerun; if waiting for the original completion cannot prevent it, D-058 is reopened and completion I/O takes owned buffers, as tokio-uring does. | runtime engineer | the rerun by week 10 [default proposed; owner to confirm] |
| **Semantics** (a class of orphan, unobserved failure or cancellation bug, under random schedules or under the `sim` profile's faults and crashes, that needs a language change rather than an interpreter fix) | Reopens D-014. | runtime engineer; owner | recorded within two weeks of the finding [default proposed; owner to confirm] |
| **Simulation** (at week 46) | If `--sim` finds at least fifteen of the twenty seeded bugs within the budget and more than random schedules alone, the profile and its compile-time escape check go into the v1 design ([17-testing.md](17-testing.md), D-070); otherwise it stays a prototype and the 1000 random schedules remain the semantics test of record. | language engineer | week 46 |
| **Not shown** (section 2.4: the machines disagree) | No fallback is built; Q-6 stays open with both machines' numbers; Phase 3 checks on paper, without building anything, that the escape rule would survive yield points as frame boundaries; Q-6 closes on the native backend [default proposed; owner to confirm]. | runtime engineer; compiler engineer | Phase 3 |

**The exhaustion guard after Phase 3.** 18's memory-or-exhaustion branch
assumes the decision comes at week 8, before Phase 3 designs the escape
rule, but the exhaustion guard can trip only at week 40. If it does, the
first remedy is the larger default reservation that 18's exhaustion row
allows; if the guard still trips under section 2.4, the stackless fallback
and its escape-rule redesign are scheduled after the experiment, and 0.1
ships with the finding in its public list of what is not in it [default
proposed; owner to confirm].

### 8.4 Parallel questions and the amendments' own questions

18: Q-2, Q-3, Q-9 (adapted), Q-11, Q-28, Q-34 and Q-50 follow the
thresholds in the table; each gets an entry in
[`open-questions.md`](../open-questions.md) with the data at Phase 9. The
capability log and replay tokens go into v1 if they meet their row
(D-070). The model arm changes the language only where D-062 already counts
a silent false friend as a design bug. D-038's compile-speed targets are
measured on the interpreter-era front end only; the native targets wait for
the backend.

| Row | Action when its threshold is met (from the table in section 2.5) | Owner | Time box |
|---|---|---|---|
| Q-2 | below 80%: the single-`Shared` design is flagged for a native benchmark | runtime engineer | Phase 9 |
| Q-3 | more than one `.copy()` per 30 lines: implicit copying is reopened | PI (the entry) | Phase 9 |
| Q-9 | a top-ten diagnostic caused by a design rule: a design review | compiler engineer | Phase 9 |
| Q-11 | more than one newtype per 200 lines: scoped instances are reopened | PI (the entry) | Phase 9 |
| Q-28 | boilerplate: the question is reopened | PI (the entry) | Phase 9 |
| Q-50 | under 3% and no friction: the left-to-right rule is adopted; otherwise D-003's local inference stays | language engineer | Phase 9 |
| Model arm | far from Python's rates: the MultiPL-T corpus of [10-tooling.md](10-tooling.md) becomes necessary; a compiling false friend: that row is a design bug under D-062 and gets a check before 0.1 or a dated entry in the decision log; nothing else in the design follows the models | PI; language engineer (the checks) | week 52 |
| Replay | a token that does not reproduce, or a divergence: a bug; overhead above 10%: the log stays a prototype; otherwise the log and tokens go into v1 (D-070) | language engineer | week 46 |
| Candidate false positives (A2) | reported beside T1.3; they are the log the friction branch's rule refinements draw on | compiler engineer | week 32 |
| Diagnostics quality | a message rated unhelpful by two authors is rewritten before 0.1 | compiler engineer | before 0.1 |
| Parser fuzzing; `sisu check` and LSP; incremental rebuild; interpreter adequacy | a bug; a design review per [10-tooling.md](10-tooling.md); the crate layout revisited; optimisation work scheduled, as each row says | the owning engineer | when found |
| Process | a report that fails any item is not published as a Sisu evidence report | PI; referee | week 52 |

### 8.5 Recording a branch

When a branch is taken, the PI records in the report and in
[15-decision-log.md](15-decision-log.md) the branch, the values that
triggered it, its owner and its date, and later its outcome. No branch is
taken that is not in this section. A result that matches no branch is
reported and sent to the owner as evidence, as the stop clause is
[default proposed; owner to confirm].

### 8.6 Escalation: the stop clause

16's stop clause says the project should be abandoned, or its memory model
replaced with a tracing GC and the proposal rewritten as "Go with sum
types, capabilities and contracts", if the widget tree or the graph cannot
be written without `unsafe` and no second-tier reference design fixes it
without named lifetimes, or if friction is not materially below Rust's.
That contradicts the owner's no-GC decision (Q-1). This document therefore
registers what happens at that branch, as 18 requires: the team escalates
the data to the owner as evidence and does not act on it. The owner's
response is recorded in the report and is not part of any verdict. The
owner's sign-off on this document is the agreement in Phase 0 that 18 says
makes the study able to change the design.

## 9. What is not built or measured

### 9.1 Not built

18's list, copied verbatim. 18 has already removed from it what the
amendments build: the `sim` profile for tests, the capability log and
replay, the counterexample mode as a team instrument, the binding count,
the io_uring test, and `sisu doc --digest` for the model arm. The list is
part of this pre-registration, so an addition needs an amendment (18's
risk table).

- Any native backend (LLVM, Cranelift, the self-hosted debug backend), and
  therefore monomorphisation, instantiation sharing, the instantiation
  budget, DWARF and the debug ABI beyond what hot replacement needs in the
  interpreter. The one native artefact is the Phase 1 harness, which
  contains no Sisu.
- An M:N parallel scheduler inside the interpreter. The interpreter runs
  the `single` profile, and the `sim` profile for tests, deterministically;
  parallel throughput is a native-runtime question.
- The package registry, lockfile resolution, `sisu add/update/publish`,
  API-diff semver, the transparency log; `[trust]` beyond parsing and the
  one supply-chain demonstration. Path dependencies only.
- C interop: header import, `extern`, `@export`, raw pointers; `unsafe`
  parses and is counted so that the bet-1 measure exists, but there are no
  unsafe operations to perform in the interpreter.
- Syntax work of any kind beyond adopting Python's indentation rules: no
  sugar decisions, no formatter, no bikeshedding in review. If the owner's
  rewrite changes block structure materially after Phase 2, the parser
  (about two weeks of work) is redone and the corpus is mechanically
  reformatted.
- `sisu fmt`, `sisu lint`, `sisu doc` beyond the digest the model arm
  reads, `sisu fix` beyond the pre-1.0 migration rule, editions, the
  teaching profile (Q-41 waits for a classroom trial after the verdict),
  the Unicode confusable lints.
- A full LSP: diagnostics, hover and go-to-definition only; no rename, no
  fix-it application, no inlay hints.
- WebAssembly as a compile target for Sisu programs, `freestanding`,
  explicit allocators, `release-size`, cross-compilation, Windows as a
  measured tier.
- Perceus-style in-place reuse for `Shared` and the atomic-elision
  performance benchmark; only the escape analysis and its runtime
  validation are built.
- std beyond what the corpus needs: no TLS, DNS, UDP, Unix sockets, regex,
  JSON encoder (corpus authors may write one), time zones, `BigInt`,
  `Ratio`, `Decimal`, persistent collections, structured logging.
- Fix-and-resume from a chosen frame; hot replacement is body swap at
  epoch boundaries only.
- The contract verifier, loop invariants, `decreases`, ghost code, SMT-backed
  refinement types (reserved words only).
- A mechanised semantics (Q-38); the conformance suite with chapter
  citations is the specification for this phase, and the prose comes after
  the verdict.
- Interpreter optimisation beyond the adequacy thresholds (bytecode
  encoding, NaN-boxing, inline caches) until a measurement demands it.
- Governance apparatus (Q-21 is open and the owner's to decide), a legal
  entity, a conference, a foundation; the `with Fs, Clock` clause,
  generation-checked references outside arenas and stackless task bodies
  unless their thresholds trip.
- Numerics and SIMD (Q-23 needs research first); a numerics-heavy corpus
  program would be meaningless without it and is not included.
- The parts of the candidates in
  [19-novel-features.md](19-novel-features.md) that are not prototyped
  here: a recording cut below `System`, native recording, a flight
  recorder and reverse execution (N-04); deferred runs for authors,
  classrooms or the live view (N-26); the completion-engine API (N-48);
  and every other adopt or tooling verdict in 19 beyond principle 6's
  exception.

### 9.2 Not measured

These are not measured in this experiment, so the report makes no claim
about them:

- the "within 10%" benchmark for Q-2 in 16, the native compile-speed
  targets of D-038, the instantiation budget and monomorphisation cost,
  parallel throughput, and `Atomic`'s weak-memory behaviour: all need the
  native backend;
- the final close of Q-6, which needs the harness rerun on real Sisu
  frames after the experiment, and the corpus rerun on the native backend,
  which checks Q-3's copy count and the friction numbers under performance
  pressure;
- Q-7 and Q-13, which 16's parallel table lists and 18 does not measure;
  Q-7 was decided in October 2026 to be settled while the standard library
  is written, and Q-13 was decided: implicit lossless widening stays;
- Q-3's Swift-style implicit-copy variant and its "why is this slow"
  reports, from 16, which 18 does not build;
- the compile-time escape check of the `sim` profile, which has nothing to
  reject in the interpreter because it has no `extern` (18, Phase 6); it
  enters v1 with the profile or not at all, on the seeded-bug study;
- any latency or memory number from `--sim` (18's risk table);
- N-26's classroom measure (time to fix borrow errors with and without a
  counterexample), which needs the teaching profile;
- N-27's fill-acceptance measure, which N-50 defers ("N-27's fill
  measurements wait for the fill engine after 0.1", 19) and which Q-49's
  table does not list: it needs the fill engine, which is tooling after
  0.1, and a hole report that 18 does not build [default proposed; owner
  to confirm];
- Q-50's constrained-decoding pilot, which needs the completion-engine API
  of N-48;
- the unsolicited playground corpus U, which is collected from Phase 9 and
  is the next data for any "not shown" verdict, but is not analysed in this
  report;
- the harness on any platform but Linux x86-64.

## 10. Timeline

18's 52-week calendar, with the amendments and the participants' weeks.

| Weeks | Phase | Amendment | Team | Participants | Gate |
|---|---|---|---|---|---|
| 1-4 | 0 skeleton, pre-registration, desk audits | | compiler engineer, language engineer, PI | desk-audit contractor; reviewer (week 1); referee (weeks 2-4) | end of week 1: early protocols frozen; end of week 4: pre-registration frozen and signed |
| 1-8 | 1 stack harness, io_uring cancellation | A1 | runtime engineer | reviewer (to week 10) | week 8: bet 3 stacks and cancellation, Q-6 provisional |
| 5-10 | 2 core language, errors, Drop | | compiler engineer, language engineer; runtime engineer from week 9 | | |
| 9-16 | 3 ownership checker and monitor | A2 built | compiler engineer; language engineer from week 11; runtime engineer for three weeks | referee (adversarial programs, weeks 9-14) | week 16: Phase 3 exit; outsiders may start |
| 15-22 | 4 traits, generics, std | | compiler engineer, language engineer | | |
| 16-32 | 5 bet-1 corpus study | A2 in use; A3 | PI; compiler engineer on triage | six corpus authors (onboarding from week 14); red team (weeks 20-25); raters (weeks 23-32) | week 24: interim; week 32: bet 1 |
| 19-32 | 6 tasks, capabilities, HTTP in Sisu, `sim` faults and schedulers | A4, first half (weeks 31-32) | runtime engineer; language engineer from week 23 | | week 32: T under random and `sim` schedules; Q-34's interpreter number |
| 30-40 | 7 bet-2 port study | | PI; runtime engineer on support | porters (weeks 29-40); C1 author (weeks 28-34) | week 40: bet 2 and Q-5; the exhaustion guard; the Q-50 count |
| 33-46 | 8 test surface with `sim` and replay, comptime, hot replacement, REPL, LSP | A4, second half; A5 | compiler engineer, language engineer | raters (one day, week 41) | week 46: the seeded-bug study; the replay row |
| 44-52 | 9 report, decisions, model arm, 0.1-interp | A6 (weeks 46-50) | everyone | referee (review, weeks 48-51) | week 52: report and release |

The calendar is 52 weeks nominal and 56 with Phase 3's budgeted slip spent
(18); section 5.3 says what a slip may and may not move. Contractors total
about 48 contractor-weeks (18); the reserve authors are on top of that.

## 11. Publication commitments

18's process row is the checklist. A report that fails any item is not
published as a Sisu evidence report (18).

1. **Raw data committed.** Every harness CSV, diagnostic log, session log,
   diary (after its author's review), help-desk transcript, coding sheet
   with both raters' independent codes and the consensus, questionnaire
   answer, program version, model-arm transcript, triage record and the
   deviations log, under `experiments/data/`.
2. **The referee's review published unedited**, including disagreements.
3. **Benchmarks reproduced on a second machine by an outsider**: the
   reviewer reproduces every harness configuration on M2.
4. **The pre-registration hash cited** in the report, with every amendment
   listed and dated.
5. **Eligibility verified**: the referee checks the eligibility record,
   which is published.
6. **Raw logs published**, as in item 1.

This document adds four commitments that are not on 18's checklist
[default proposed; owner to confirm]:

7. Every number in the report is regenerated by a CI script from the
   public data, and negative results use the same template as positive
   ones (18's Phase 9 deliverables).
8. The report is published whatever the outcome: by week 52, or, after a
   slip, within four weeks of the last decision point.
9. The reviewer's review of the controls is published unedited too.
10. Participants who accept are named as authors of the report. The
    referee's review is published under the referee's own name as a
    separate document, and the referee may decline authorship of the report
    to stay independent.

## 12. Amendments

A1-A6 were proposed in [19-novel-features.md](19-novel-features.md) (N-07,
N-26, N-48, N-03, N-04, N-50), accepted by the owner in October 2026
(Q-49, recorded in D-070) and integrated into 18. They are numbered here so
that the report can list them. Because they precede freeze, they are part
of the frozen rules, not changes to them. Together they cost about six
person-weeks and four weeks of calendar; 18 takes the four weeks rather
than trimming Phase 8's polish.

### A1: the io_uring cancellation test

| Field | Value |
|---|---|
| Number and date | A1, October 2026 |
| Status | accepted by the owner before freeze (Q-49) |
| Phase | 1 |
| What it adds | the test of the guaranteed-drop rule (D-058, N-07): a million in-flight io_uring reads into buffers on task stacks are cancelled on each stackful strategy and both machines; every cancelled call must return only after the kernel has let go of its buffer; a canary exposes any later write; the cancel-to-return latency is recorded |
| Sections | 2.4, 4.4, 6.5, 8.3 |
| Rationale | D-058 lets completion I/O and DMA borrow the caller's buffer because views, scopes and suspended frames are never leaked; the test checks the runtime half of that promise before Q-6 closes, and strengthens bet 3's cancellation semantics |
| Cost | days: about three days of the runtime engineer (18) |

### A2: the monitor as a counterexample generator

| Field | Value |
|---|---|
| Number and date | A2, October 2026 |
| Status | accepted by the owner before freeze (Q-49) |
| Phase | 3 (built); 5 (used) |
| What it adds | a team-only mode in which a program the checker rejected runs anyway across its tests with its ownership errors deferred to the monitor (N-26); every rejected corpus program is rerun this way, and rejections that never reach a conflict are candidate false positives |
| Sections | 2.2 (its row), 3.6, 6.2 (step 7), 6.8, 8.4 |
| Rationale | bet 1's false-positive accounting, which 18's risk table already requires; kept from the authors because it would change the friction they report |
| Cost | about one week (18: one of the runtime engineer's three weeks in Phase 3) |

### A3: the count of bindings typed by a later statement

| Field | Value |
|---|---|
| Number and date | A3, October 2026 |
| Status | accepted by the owner before freeze (Q-49) |
| Phase | 5 |
| What it adds | a type-checker count of the bindings whose type is fixed by a later statement (N-48), with the exit-questionnaire friction item |
| Sections | 2.5, 6.6, 6.8, 8.4 |
| Rationale | Q-50's decision (October 2026): measure first, and adopt the left-to-right rule if fewer than about 3% of corpus bindings are typed by later statements and the authors report no friction |
| Cost | hours of checker instrumentation (18) |

### A4: the `sim` profile and the seeded-bug study

| Field | Value |
|---|---|
| Number and date | A4, October 2026 |
| Status | accepted by the owner before freeze (Q-49) |
| Phase | 6 (weeks 31-32) and 8 |
| What it adds | Phase 6: fault-injecting `Net` and `Fs` fakes with every fault drawn from the called method's declared error set, a pessimistic durability model, PCT and bounded exhaustive scheduling. Phase 8: a clock that advances only when every task is blocked, several nodes in one process, simulated crash and restart, and the seeded-bug study on P3 and L (N-03, D-070) |
| Sections | 2.4, 4.1 (row L), 4.9, 5.1, 6.5, 8.3 |
| Rationale | adds faults and crashes to the 1000-schedule run that Phase 6's exit criteria already require, so it strengthens the test of bet 3's cancellation and panic semantics; the seeded-bug study decides whether the profile enters v1 |
| Cost | about four person-weeks, about half in each phase (18) |

### A5: the capability log, `sisu replay` and replay tokens

| Field | Value |
|---|---|
| Number and date | A5, October 2026 |
| Status | accepted by the owner before freeze (Q-49) |
| Phase | 8 |
| What it adds | `sisu run --record`, `sisu replay`, and a replay token on every failing test (N-04), with per-capability redaction from the first version |
| Sections | 2.5 (its row), 6.8, 8.4 |
| Rationale | corpus triage and schedule exploration need failures that reproduce anywhere; the replay row decides whether the log enters v1 (D-070) |
| Cost | about one person-week (18) |

### A6: the model arm

| Field | Value |
|---|---|
| Number and date | A6, October 2026 |
| Status | accepted by the owner before freeze (Q-49) |
| Phase | 9 |
| What it adds | frontier models solve P1-P6's tasks with no context, the pocket reference, or the reference plus the API digest, each with and without compiler feedback, in the playground's sandbox, with the same tasks in Python as the baseline; it measures compile and hidden-test pass rates, false-friend bugs per row of D-062's table, and tokens to a pass (N-50) |
| Sections | 2.5, 3.1, 4.1 (row M), 4.3, 4.8, 6.9, 8.4 |
| Rationale | measures the new-language penalty for model-written code ([`studies.md`](../research/evidence/studies.md), section 11) from the start; it runs after the human studies with no outsiders, so it cannot confound them, and the design does not chase it |
| Cost | about two person-weeks of the PI's time and model-API money within the pre-registered budget (18) |

### Template for later amendments

| Field | Value |
|---|---|
| Number and date | A*n*, the date |
| Proposed by | |
| Status | proposed, accepted or refused, with the owner's and the referee's sign-off |
| Sections changed | the old text and the new text, in full |
| Rationale | |
| Data affected | the data the change applies to, and a statement that none of them exist yet |
| Cost and calendar | |

## 13. Open items for the owner

Placeholders, one line each. The owner deferred every budget, personnel,
institution and compensation item in October 2026 ("no need to worry
about those at this point"); they stay listed so the freeze checklist is
complete, marked (deferred).

1. The commit hash at freeze and the early-protocol commit hash (1.1).
2. Funding source and total budget (1.1) (deferred).
3. Institution, ethics review body and approval, or the reason none is
   needed (1.1) (deferred).
4. The external location where the freeze hash is published (1.3).
5. Names: owner, PI, runtime engineer, compiler engineer, language
   engineer, Go/Java runtime reviewer, referee (1.2) (deferred).
6. The contracting entity (3.3) (deferred).
7. The retention period for contact and payment details (3.7) (deferred).
8. The licence for programs and diaries (3.7) (deferred; the repository's
   own licence is Apache-2.0).
9. The data controller and the applicable data-protection law (3.7)
   (deferred).
10. Three P6 candidates (4.1.2).
11. The Go service: name, repository and commit (4.1.3).
12. The per-program allowlist of libraries for controls (4.2).
13. Control toolchain versions: Rust, Go, Kotlin, Swift, Python (4.3).
14. Control runtime versions and flags, and the coroutine library, from the
    reviewer (4.4).
15. Machines M1 and M2: virtual machines or GitHub-hosted runners, their
    specifications recorded at run time (4.4) (decided).
16. The reference laptop for interpreter adequacy (4.4).
17. The desk-audit crate list with versions (4.6).
18. The three models for the model arm (4.8).
19. The model-API budget (4.8).
20. The hash of the sealed seeded-bug list (4.9).
21. Compensation per role, the application channel, the closing date and
    the contact address ([21-recruitment.md](21-recruitment.md))
    (deferred).

Every "[default proposed; owner to confirm]" mark in sections 1-12 is also
an open item: each is kept, changed or deleted before freeze (section 1.3).
