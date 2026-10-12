# 18 - Next steps: the experiment

[16-validation-plan.md](16-validation-plan.md) says what the prototype must
answer. This document says how, in what order, by whom, and what each
result does to the proposal. The plan is the interpreter-first architecture
the owner decided (D-052, D-053), sequenced and judged like a pre-registered
experiment: the one runtime question that needs no Sisu code (bet 3's
stacks, Q-6) is measured in a standalone harness from week 1; the ownership
checker, with a dynamic monitor that re-checks it on every run, is the first
Sisu artefact, so that paid outsiders are writing the bet-1 corpus against
it by week 16; tasks and capabilities follow on the same interpreter so that
bet 2's port study and bet 3's semantics run in Sisu code; and the verdicts,
the decision tree and a 0.1 release on the interpreter land at about week 48
with three engineers. Everything built runs over the typed IR that the
native backend will later consume, so nothing in the experiment is thrown
away. The syntax rewrite, the testing design, governance (Q-21) and numerics
(Q-23) are owned elsewhere and are not on this plan's path.

## Principles for the experiment

1. **The interpreter is the first implementation and a feature, so it is
   built to keep (D-052).** Roc spent seven years without a 0.1 and then
   rewrote a compiler whose parser was not error-tolerant and whose panics
   were unfixable ([`roc.md`](../research/languages/roc.md)); every pass
   here runs over one typed IR that the checkers, the comptime evaluator,
   the interpreter and later the native backend share.
2. **Test the bets in order of cost times damage.** Bet 3's stack question
   is a property of a runtime, not of Sisu, and a Rust harness can answer
   it in eight weeks; bet 1 would reverse the memory model, so it gets a
   cheap desk prior before the checker exists and the first outsider
   programs; bet 2 needs both and goes last.
3. **Outsiders are the measurement.** Inter-programmer variance exceeds
   inter-language variance (Prechelt 2000,
   [`studies.md`](../research/evidence/studies.md)), so programs written by
   the designers count for nothing; the corpus is written by paid people
   who did not design the language, with controls, and the team's own
   programs are used only to find bugs before outsiders do.
4. **Pre-register the rules of evidence, and demand large effects.**
   Thresholds, programs, analysis and the decision for each outcome are
   committed before any participant writes a line, because a study whose
   rules can move after the data cannot change the design; with six
   authors, only halving Rust's fighting-the-checker rate or staying within
   4x of Go's per-task memory is a result, and anything smaller is reported
   as "not shown", not as "holds".
5. **The checker checks itself.** A static ownership checker with a
   soundness hole would accept programs it should reject and silently
   understate friction, so the interpreter carries a Miri-style dynamic
   ownership monitor (Miri: [`rust.md`](../research/languages/rust.md))
   and every corpus run is double-entry bookkeeping between the two.
6. **Nothing beyond the bets before the verdict.** No syntax work (the
   owner's rewrite is adopted as it lands), no native backend, no registry,
   no governance apparatus, no formatter, no specification prose; the
   interpreter-era features the owner has decided (hot replacement, REPL,
   scripts) are built because they are cheap on this architecture, not
   because they are polish.

## Phases

The phases overlap; the calendar is in [Team and timeline](#team-and-timeline).
Effort is in person-weeks (pw). "Engineer" means one of three full-time
engineers (a lead, a type-system engineer, a runtime engineer), "PI" the
half-time research lead who runs the studies and rates nothing, and
"contractor" a paid outsider.

### Phase 0: rules of evidence and a skeleton that cannot crash (weeks 1-4)

**Goal.** Fix the rules of evidence before code exists, get the cheapest
signal on each bet from programs that already exist, and run a
straight-line subset of Sisu end to end on the compiler architecture every
later phase depends on, so that nothing has to be rewritten later to become
error-tolerant, panic-free or incremental.

**Deliverables, evidence (PI and one Rust contractor).**

- `experiments/PREREGISTRATION.md`: the corpus programs, the Go service to
  port, participant criteria, every metric and threshold in
  [Measurements and thresholds](#measurements-and-thresholds), the analysis
  procedure, and the decision for each outcome in
  [Decision points](#decision-points). Signed off by the owner and by one
  external Rust sceptic (the referee). Its commit hash is cited in the
  final report; later changes are amendments and are listed.
- Lifetime desk audit: a Rust contractor classifies every explicit lifetime
  (`'a`, `'static`, `for<'a>`) in about ten real crates spanning the corpus
  domains (a regex engine, a parser-combinator library, a graph library, an
  immediate-mode GUI, a hyper-based service, a JSON library, slotmap users)
  as expressible in Sisu (elided by the one-borrowed-parameter or `self`
  rule; `from` clause; single-lifetime `view struct`; scoped task borrow)
  or not (multi-lifetime struct; references stored in long-lived
  containers; self-referential; `'static` closures with no natural `own`
  capture).
- Capability-parameter prior: a Go AST tool over the shortlisted services
  counting `ctx.Context` and injected-dependency parameters per function.
  Go's `ctx` is a million-user natural experiment in threading one
  parameter through everything ([`go.md`](../research/languages/go.md)).
- Bet-3 feasibility note: the `vm.max_map_count` arithmetic (the Linux
  default of 65530 means a guard page per task splits VMAs and caps
  guard-paged tasks near 32k unless stacks are pooled); the page-table cost
  of 1 MiB reservations; and a correction to Q-6's premise: Java's virtual
  threads copy frames into heap stack-chunk objects when a thread unmounts,
  so Loom is not a non-moving-stack precedent
  ([`java.md`](../research/languages/java.md) records the JEP, not the
  mechanism). The honest precedents are C and Rust stackful coroutine
  libraries, which become controls in Phase 1.
- The Go service for bet 2 shortlisted and chosen (3-8 kLOC, I/O in at
  least 30% of functions, a test suite, interfaces or mocks for I/O),
  recorded in the pre-registration.
- Recruitment opened for the contractors in [The corpus](#the-corpus).

**Deliverables, compiler (two engineers).**

- Lexer with Python's indentation algorithm as
  [`indentation-syntax.md`](../research/topics/indentation-syntax.md)
  recommends (INDENT/DEDENT from a width stack, layout only outside
  brackets, spaces only, no parser feedback) and the proposal's keyword set.
  No other syntax decision is made in this plan.
- Hand-written recursive-descent parser that never fails: every production
  can yield an Error node, recovery synchronises at NEWLINE and DEDENT,
  comments stay in the token stream for the future formatter.
- Diagnostics infrastructure: stable codes, multi-span labels, a fix-it
  slot, poisoned types so only the first error in a dependency chain is
  reported.
- Panic policy: deny-level lints on `unwrap`, `expect` and `panic` in every
  front-end crate; an internal error becomes an ICE diagnostic that dumps
  the IR; a fuzz target for the parser whose invariant is "never panics,
  always returns an AST, never emits a diagnostic storm".
- Query skeleton (`source -> tokens -> ast -> resolved module -> typed body
  -> SIR body`) so `sisu check` is incremental from the first commit and
  the LSP can share it, as [10-tooling.md](10-tooling.md) requires.
- SIR v0 (a typed control-flow graph: typed locals, basic blocks,
  Assign/Call/Goto/Switch/Return) and the trampolined interpreter loop: an
  explicit per-task frame stack, no host recursion for Sisu calls. Integers,
  floats, booleans, strings, `let`/`var`, arithmetic with overflow and
  divide-by-zero traps, `if`/`while`, functions, explicit `return`, ambient
  `print`/`print_err`.
- Conformance runner for `.sisu` files with expected-diagnostic
  annotations and expected output; CI runs it plus a front-end-time and an
  interpreter-time benchmark on every commit.

**Exit criteria.**

- Pre-registration merged with the referee's sign-off recorded; the
  lifetime audit reports per-crate category counts; the Go audit reports
  median and maximum dependency parameters per I/O-doing function;
  contracts are in progress with start dates inside Phases 5 and 7.
- If 20% or more of the lifetime annotations in application-tier crates
  are "not expressible", the generation-checked reference fallback (Q-1's
  fourth option, [`vale.md`](../research/languages/vale.md)) is
  pre-designed in Phase 3 behind a flag so the corpus study can run both
  conditions.
- `hello.sisu` and a collatz program run through the full pipeline; an
  overflow traps with a message and a location.
- A file with three syntax errors reports all three with sensible spans;
  the parser fuzz target has run 24 hours without a panic (a CI job from
  here on, not a gate).
- CI publishes front-end milliseconds per kLOC and interpreter nanoseconds
  per SIR statement.

**Effort.** About 12 pw engineering (two engineers for four weeks, plus the
feasibility note), 3 pw PI, 2 pw Rust contractor. Assumes engineers who have
built a compiler or interpreter in Rust before, and that the owner's syntax
rewrite is not waited for.

**Bets tested.** 1 and 2 as priors only; 3 as feasibility only. The rest is
the Roc checklist that every later phase depends on.

### Phase 1: the bet-3 stack harness, no Sisu involved (weeks 1-8)

**Goal.** Decide Q-6 and the runtime half of bet 3 with native code before
the language exists: can stackful, colourless tasks on stacks that cannot
move match Go and Java virtual threads within the pre-registered factors?

**Deliverables** (the runtime engineer, alone, from a one-page brief).

- `taskbench/`, a Rust crate with no dependency on the compiler: an M:N
  work-stealing scheduler over OS threads, an assembly context switch from
  an existing library, structured scope and join, a cancellation flag, and
  pluggable stack strategies: (A) per-task `mmap` reservation with a
  `PROT_NONE` guard page and lazy commit; (B) pooled stacks carved from
  large reservations with compiler-style software stack-limit probes
  instead of guard pages; (C) exact small stacks (2-16 KiB) for tasks whose
  call tree is bounded, simulating compiler-computed bounds; (D) `madvise`
  decommit on park; (E) a stackless state-machine variant as the lower
  bound the fallback design would reach.
- Controls configured and reviewed by the Go/Java runtime reviewer: Go
  (latest stable, `GOMAXPROCS` matched), JDK 21+ virtual threads (flags
  and warm-up recorded), tokio as the stackless reference, and a C or Rust
  stackful coroutine library as the closest no-GC precedent. Versions,
  flags and machine specifications recorded.
- Benchmarks: spawn-plus-join cost; resident memory per idle task at 10k,
  100k and 1M parked tasks, after shallow use and after one 32 KiB
  excursion to expose commit creep; maximum creatable tasks under default
  sysctls; a 10k-connection TCP echo server (p50, p99, p99.9 latency and
  throughput) and 100k-connection idle RSS; deep recursion in a task (a
  10k-deep recursive-descent input and a 1M-deep naive recursion) checking
  for a clean "stack exhausted" failure rather than a segfault; a
  starvation test with N compute-bound tasks that yield every K iterations
  plus one latency-probe task; the overhead of software stack probes on
  compute loops.
- A written cost estimate of the stackless alternative (C#-style state
  machines over SIR), including what it does to `ref`-into-frame semantics
  and to hot replacement, so the fallback is costed before it is needed.
- A report with every configuration, not the best one, reproduced on a
  second machine by the reviewer; raw CSVs committed.

**Exit criteria.**

- Every bet-3 metric in the table measured for strategies A-E and all
  controls on two machines.
- A verdict against the thresholds and a Q-6 recommendation (strategy,
  default reservation, probes versus guard pages, and whether `spawn` needs
  a capability; the research found no supply-chain attack that
  spawn-gating would have stopped), or the stackless fallback declared and
  its interaction with bet 1 (suspended frames that hold `ref`s into
  themselves) flagged to Phase 3 before the checker's design is frozen.

**Effort.** 8 pw runtime engineer, 1 pw reviewer, 1 pw reproduction.
Assumes Linux x86-64 only, an existing context-switch library, and no
attempt at a production scheduler.

**Bets tested.** 3 (stacks, memory, creation cost, latency, exhaustion,
starvation). The semantics half of bet 3 runs in Sisu in Phase 6.

### Phase 2: core language, errors, Drop (weeks 5-10)

**Goal.** The non-generic, non-concurrent core: enough to run
`wordcount.sisu` against a host-provided file system, with deterministic
destruction and panic unwinding working exactly as
[04-memory-and-ownership.md](04-memory-and-ownership.md) and
[05-errors.md](05-errors.md) specify.

**Deliverables.**

- Structs; enums with payloads; exhaustive `match`; tuples; `T?` with
  `?.`, `??`, `if let`, `let ... else`; `list`, `dict`, `set`,
  `String`/`str`, `Array` as host-backed builtins behind the std API; `for`
  over ranges and collections; non-escaping closures with `it`; explicit
  `return` only (blocks have no value).
- Error sets with payloads, union subtyping, inferred `T !`, postfix `?`,
  `else as e:`, must-use as an error, error return traces recorded at
  each `?`.
- Panics (bounds, overflow, divide by zero, shifts, `assert`, `expect`,
  `unreachable`, `panic`); `Never`.
- Drop elaboration in SIR: Drop statements at scope exits in reverse
  declaration order, fields after the value, drop flags for conditionally
  moved locals, unwind edges per block; user `impl Drop`; unwinding runs
  Drops frame by frame; a panic inside a Drop during unwinding aborts.
- Modules (file and directory), `import`, `pub`/`pub(package)`, a package with
  `sisu.toml` and path dependencies only; `sisu run` on a file or a
  package.
- The driver constructs `System` only when `main(sys: System)` is
  declared; `Fs`, `Env`, `Clock` as opaque host objects with intrinsic
  methods. The interpreter has no ambient file or socket intrinsic at all,
  only the standard streams.
- Team programs 1-4 from the [examples](examples/) and a 40-file
  wrong-program corpus with expected codes.

**Exit criteria.**

- `wordcount.sisu` and `errors.sisu` run unmodified except for syntax.
- A drop-order torture program (nested scopes, early return, `?`, a panic
  inside a loop, conditional moves, each value with a logging Drop) prints
  exactly the order 04 specifies, on the normal and the unwinding path.
- The wrong-program corpus produces the expected codes with no cascaded
  errors.
- Parse-to-SIR under 50 ms per kLOC.

**Effort.** About 14 pw (two engineers for six weeks; the runtime engineer
joins from week 9 to write the host builtins). Assumes the Phase 0 skeleton
held.

**Bets tested.** None directly; a prerequisite for all three.

### Phase 3: the ownership checker and the dynamic monitor (weeks 9-16)

**Goal.** Implement the memory model statically on SIR and re-check it
dynamically in the interpreter, so that the bet-1 corpus runs against a
checker whose own soundness is being tested at the same time.

**Deliverables.**

- Parameter conventions (`x`, `mut x`, `own x`) with call-site `mut`
  required; moves with use-after-move diagnostics; `@derive(Copy)`;
  explicit `.copy()`.
- `ref`/`ref mut` locals and returns; `from` clauses and the three elision
  rules.
- The borrow checker on SIR: loans created by Borrow statements, live while
  the reference is live (liveness dataflow, non-lexical), the aliasing
  rule. No named lifetimes, because every loan's origin is a local, a
  parameter or a view field.
- The escape rule: no `ref` or view in a non-view struct, in `Shared`, in
  a collection of non-view element type, in an escaping closure, or in a
  return not covered by `from`; `view struct` with one implicit lifetime
  and upward infection.
- Closures with capture modes and `own` closures; `for x in xs` / `in mut xs` /
  `in own xs`; methods returning `ref` into `self`.
- `Shared[T]` (host reference count for now), `Weak`, `Cell`, `Mutex`
  (single-task until Phase 6), `Box`, `Arena[T]` with generational handles
  and stale-handle detection.
- If Phase 0 triggered it: the generation-checked reference type behind a
  flag, so the corpus study can run both conditions.
- Borrow diagnostics that name the fix, in the form
  [10-tooling.md](10-tooling.md) commits to; a worse diagnostic would
  confound bet 1.
- The dynamic ownership monitor: moved-from locals become a `Moved` poison
  value; every referent carries shared and exclusive borrow counts;
  use-after-move, aliasing violations, drop-while-borrowed and a view
  escaping its frame are internal soundness errors that fail the run with
  the program, the seed and the SIR dump.
- `sisu audit --ownership` counts `Shared`, `Weak`, arena handles,
  `.copy()` and `unsafe` per file; `sisu check --diary` appends every
  diagnostic with a timestamp and code to a per-author JSONL log for the
  friction measurement.
- Conformance additions: a translated slice of rustc's borrow-checker UI
  tests; 20 adversarial programs written by the referee that try to alias
  or dangle in safe code; `ownership.sisu` and a 60-file wrong-ownership
  corpus with expected codes and named fixes.
- Designer-written dry-run drafts of all six corpus programs, used only to
  find checker bugs, then discarded and never shown to participants.

**Exit criteria.**

- `ownership.sisu` runs, including the tokenizer-of-views example and the
  arena graph with a cycle.
- The 60 wrong-ownership programs and the translated rustc tests produce
  the expected codes, each with a named fix; the referee's 20 adversarial
  programs are all rejected.
- Zero monitor violations across every program so far and across 24 hours
  of mutation fuzzing (random statement reorderings that the checker
  accepts must not trip the monitor).
- The six dry-run drafts type-check and run their tests.
- The first outside authors can start (Phase 5).

**Effort.** About 16 pw (the lead for eight weeks, the type-system engineer
for six, the runtime engineer for two), 1 pw referee. The view and escape
checker is the one piece with no reference implementation to copy: Hylo and
Mojo both report iterators and stored references as the unsolved residue
([`hylo.md`](../research/languages/hylo.md),
[`memory-management.md`](../research/topics/memory-management.md)). Budget a
50% slip here before anywhere else.

**Bets tested.** 1 (the instrument; the measurement is Phase 5).

### Phase 4: traits, generics, std in Sisu (weeks 15-22)

**Goal.** Enough polymorphism to write `std.core` and the iterator library
in Sisu, which is both a prerequisite for the corpus and the first bet-1
data point, because iterators are where lifetime-free designs struggle. Cut
to what the corpus needs.

**Deliverables.**

- Traits with provided methods, associated types and constants; `impl
  Trait for Type`; coherence with the orphan rule and the root-package
  exception (Q-11); missing-impl fix-its that list candidates.
- Generic functions and types checked once at the definition against
  bounds; `where`; const generics for `Array[T, N]`.
- Polymorphic interpretation: type arguments are runtime values, impl
  lookup by concrete type at call time, no monomorphisation.
- `dyn Trait` behind `Box`/`ref` with object-safety diagnostics; operator
  traits on the fixed set; `@delegate` on newtypes.
- Auto traits `Copy`, `Send`, `Sync`, `Escapable` computed structurally
  (consulted at spawn boundaries from Phase 6).
- The comptime evaluator as the same interpreter with fuel, a memory limit
  and no `System`, as [08-metaprogramming.md](08-metaprogramming.md)
  requires; used first for the built-in `@derive(Eq, Ord, Hash, Debug,
  Default, Copy)`. User-defined derive, reflection beyond derive and
  refinement types wait for Phase 8.
- `std.core` (Option and Result combinators, `Iterator` with map, filter,
  take, enumerate, collect), `std.collections` shells, `std.string`,
  `std.fmt`, written in Sisu over host intrinsics; doctests run through
  the conformance runner.
- `traits.sisu`, team-written.

**Exit criteria.**

- `traits.sisu` runs; `std.core` compiles and its doctests pass.
- A generic body that violates a bound is reported at the definition,
  never at a use site.
- `sisu check` re-checks one edited body in under 300 ms on the
  std-plus-corpus tree (the [10-tooling.md](10-tooling.md) target scaled
  down); a miss is logged, not a gate.

**Effort.** About 14 pw (two engineers for seven weeks).

**Bets tested.** 1 (P5, the iterator library, is the first outsider program
that depends on this phase).

### Phase 5: the bet-1 corpus study (weeks 16-32)

**Goal.** Find out whether second-class references, single-lifetime views,
`Shared` and `Arena` let people who did not design the language write the
programs where Rust programmers meet lifetimes, with materially less
friction than Rust.

**Deliverables.**

- Six programs: the five in [16-validation-plan.md](16-validation-plan.md)
  (P1 tokenizer and parser with a borrowing AST, P2 graph with parent
  pointers, P3 HTTP server with a shared cache, P4 widget tree with event
  callbacks, P5 iterator adapter library) plus P6, an application-shaped
  program chosen by the non-Rust authors in Phase 0 to counter the
  Rust-shaped bias of the other five. P1, P2, P5 and P6 start at week 16;
  P4 at week 22 (it needs `dyn` and `@delegate`); P3 at week 26 (it needs
  Phase 6).
- A shared, language-neutral behavioural test suite per program, written
  by the PI in advance, so that "done" is objective.
- Twelve Sisu implementations: each program by one Rust-experienced and
  one non-Rust (Go, Kotlin or Swift) author, assigned by a recorded random
  draw.
- Controls: the Rust-experienced authors also write their programs in
  Rust, with Sisu-first and Rust-first order counterbalanced across
  authors; the non-Rust authors write theirs in their home language as the
  friction floor.
- A red-team program by a Rust expert briefed to break the model with the
  ten patterns for which Rust needs lifetimes (self-referential structs,
  multi-lifetime structs, stored references, intrusive lists, cursor APIs),
  recording what Sisu cannot express and what each workaround costs.
- Instrumentation: per-author `sisu check --diary` logs, timed sessions,
  weekly diaries in which the author tags each checker error, a help-desk
  transcript (designers may answer questions; every answer is logged and
  counted), and an exit questionnaire reusing the Rust survey wording
  verbatim ("borrow checker rejecting valid code": big problem / could be
  improved / not a problem), so the numbers compare with the 10.6%
  big-problem baseline in [`surveys.md`](../research/evidence/surveys.md).
- Friction coding: two raters, not on the team, classify every diary and
  log event as (a) diagnostic unclear, (b) a rule rejected a sound design
  and the author restructured, (c) checker bug (triaged within 24 hours,
  excluded from friction, published), or (d) missing library; language
  tokens masked where feasible; Cohen's kappa reported.
- Hot function replacement exercised during the study (a handler body
  swapped in the running P3 server) with the monitor on, because
  replacement interacts with drop behaviour.
- Analysis against the pre-registered thresholds, a written verdict, and
  every program, log and rating published.

**Exit criteria.**

- All twelve implementations and the red-team report either pass their
  test suites or are documented as blocked, with the blocking construct
  named.
- T1.1-T1.5 and the descriptive flags in the measurement table have
  measured values; kappa of at least 0.6, or the rating is redone with a
  refined codebook.
- An interim verdict from P1, P2, P5 and P6 at week 24 (a hard fail on P2
  pauses the bet-2 spend before the porters start); the full verdict
  recorded (holds / fails / not shown) and the matching branch of
  [Decision points](#decision-points) executed by week 32.

**Effort.** 24 contractor-weeks (six authors at about four weeks each), 3
contractor-weeks red team, 2 pw raters, 10 pw PI, 5 pw lead (bug triage
only). Assumes programs of 300-1500 lines and that checker bugs are fixed
within 24 hours.

**Bets tested.** 1 (the definitive expressiveness and friction test); the
Q-1 fallback, Q-3, Q-11 and Q-28 counts.

### Phase 6: tasks, capabilities, an HTTP server in Sisu (weeks 19-30)

**Goal.** Structured colourless concurrency and the capability model
running in the interpreter on a deterministic scheduler, so that bet 2's
corpus can be written and bet 3's semantics (scopes, cancellation, panics
to scope, `Send`/`Sync`) are tested in Sisu code under thousands of
schedules.

**Deliverables.**

- Tasks as heap-allocated frame stacks on the already-trampolined
  interpreter; a single-OS-thread scheduler with a run queue, parked set,
  timer wheel and nonblocking I/O poller (the `single` profile of
  [12-interop-and-targets.md](12-interop-and-targets.md), deliberately; an
  M:N interpreter is not built).
- Seedable deterministic scheduling: `--schedule-seed N` and `--explore N`
  rerun a test under N seeds, a schedule explorer for free because the
  interpreter owns every yield point.
- `task.scope` as a view bound to its frame, `spawn` returning
  `Handle[T]`, `join`, scope end joins all, first failure cancels
  siblings; the cancellation flag at every blocking point, `task.shield`,
  `spawn_with(deadline:)`; bounded `channel`, `select` with timer and
  cancellation arms; `Mutex[T]`, `RwLock`, `Atomic`, `Condvar`.
- `Send`/`Sync` checked only where a value crosses `spawn`, with the
  message that names the field and the wrapper.
- Panics unwind the task's frames to its scope, which reports
  `TaskPanicked` on join and cancels siblings; the supervisor loop in
  [06-concurrency-and-capabilities.md](06-concurrency-and-capabilities.md)
  works as written.
- Yield points at loop back-edges and calls with a reduction budget;
  `--stats` reports scheduling latency so Q-34 has a number.
- The `Shared` escape analysis that tags each `Shared.new` site task-local
  or may-cross (Q-2); the monitor treats a task-local-tagged `Shared`
  crossing a task, a non-`Send` value crossing a task and a task outliving
  its scope as soundness errors; `--stats shared` counts tagged sites.
- `std.cap`: `System`, `Fs`, `Net` (TCP only), `Clock`, `Rand`, `Env`,
  `Proc` as host objects with attenuation (`restrict_to`, `read_only`,
  `only_hosts`, `frozen_at`, `advance`); fakes (`Fs.in_memory`,
  `Clock.frozen_at`, `Net.fake` with an in-process stream pair and request
  counts) written in Sisu where the language allows, so the fakes are
  themselves corpus.
- `std.x.http`: a minimal HTTP/1.1 server and client written in Sisu over
  `Net`, not host-provided, so the server is itself a bet-2 program.
- `sisu audit --caps`: per I/O-doing function, the number of capability
  parameters; for each leaf use, the threading depth to `main`; structs
  bundling three or more capabilities. Computed from SIR, where the checker
  already knows which types are capabilities.
- `concurrency.sisu`, `capabilities.sisu` and a 10k-connection echo
  server, team-written.

**Exit criteria.**

- `concurrency.sisu` and `capabilities.sisu` run; the echo server holds
  10k idle connections in the interpreter (functional, not fast).
- 1000 random schedules of the structured-concurrency suite: zero orphan
  tasks, zero unobserved panics, every cancelled subtree joined before its
  scope returns, lock guards dropped on every unwinding path.
- `--stats` reports the p99 scheduling latency of an I/O task under a
  compute-bound sibling (the Q-34 number).
- P3 is running on `std.x.http` with its diary and `audit --caps` output
  recorded.

**Effort.** About 20 pw (the runtime engineer for twelve weeks owns the
scheduler and `std.cap`; the type-system engineer joins from week 23 for
`std.x.http` and the schedule explorer). `Net.fake` is easy to
underestimate and is budgeted here.

**Bets tested.** 2 (the instrument) and 3 (semantics; stacks were Phase 1).

### Phase 7: the bet-2 port study (weeks 30-40)

**Goal.** Measure the capability-parameter burden on a real service rather
than on toy programs, against Go's own `ctx` and dependency-injection
burden as the control, and check whether authors bundle capabilities into
an ambient-authority struct to escape it.

**Deliverables.**

- Two independent ports of the pre-registered Go service to Sisu on the
  interpreter, by one Go-background porter and one Rust-background porter,
  with the original's integration tests translated and shared.
- `sisu audit --caps` over both ports; the Go AST tool's output for the
  original.
- Test-writing comparison: ten pre-specified tests written by each porter
  against the `Fs`, `Clock` and `Net` fakes, timed; the Go porter writes
  the same ten against the original with its existing mocking approach,
  timed (the fakes-versus-mocks question in
  [`testing.md`](../research/topics/testing.md)).
- Supply-chain demonstration: a planted dependency that tries to read a
  file and open a socket, shown to fail type-checking unless handed `Fs`
  and `Net`, with the attenuated capabilities the ports actually pass to
  each dependency listed. A sceptic will ask for this.
- Secondary bet-2 data: C1, a grep/du-style command-line tool grown from
  `wordcount.sisu` by an outside author (arguments, `Fs`, `Env`, `Proc`),
  and P3 from Phase 5.
- Diaries with time attributed to capability plumbing versus other porting
  work; the verdict against the thresholds.

**Exit criteria.**

- Both ports pass at least 80% of the translated integration tests, the
  rest documented.
- T2.1 and T2.3 measured; the reported figures (threading depth versus
  Go's `ctx` depth, plumbing share of port time, fake-versus-mock test
  time, attenuation calls used) published.
- The supply-chain demonstration is reproducible from the repository.
- Verdict recorded and the `with` decision (Q-5) taken by week 40.

**Effort.** 10 contractor-weeks (two porters at five weeks), 3
contractor-weeks for C1, 6 pw PI, 4 pw engineers (std gaps, host bugs, the
Go tool). Assumes the interpreter's single-thread tasks are adequate at
test load.

**Bets tested.** 2 (the definitive test); Q-5.

### Phase 8: the rest of the interpreter-era surface (weeks 31-42)

**Goal.** The pieces the owner has decided that an interpreter makes almost
free, and the test surface the corpus needs, without anything that is
ahead of the verdict.

**Deliverables.**

- The test surface following the separately written testing design: `test`
  blocks, doctests, `check` property tests with shrinking,
  `assert_snapshot`, `@bench`; each test gets fresh capability fakes; tests
  run in parallel on the task scheduler.
- `const` and `comptime` with fuel and memory limits named in the manifest;
  `if comptime`; `std.reflect` sufficient for user-defined `derive`; the
  checked string DSL example from `comptime.sisu`.
- Contracts checked at run time in the interpreter (`requires`, `ensures`
  with `result` and `old`, struct `invariant`); `pure` enforced and used
  for comptime eligibility. Refinement types, the verifier and ghost code
  stay reserved words.
- `sisu repl` on the comptime interpreter with a loaded package.
- `sisu run --watch` with hot function replacement: the function table of
  reference-counted bodies is the per-definition indirection point of
  D-053; on a file change the queries re-run and every function whose
  signature hash is unchanged has its body swapped; frames in flight keep
  the old body until they return; a changed signature or layout restarts
  with a message. Fix-and-resume from a chosen frame is demonstrated, not
  shipped.
- `sisu lsp` with diagnostics, hover (inferred error set, capability
  parameters, parameter conventions at call sites) and go-to-definition,
  on the same queries as `sisu check`.
- Every conformance file cites the proposal chapter it tests, so the suite
  is the skeleton of the specification draft; the prose comes after the
  verdict.

**Exit criteria.**

- Every file in [examples](examples/) except `ffi.sisu` runs after syntax
  conversion.
- Hot-swapping a handler body in the running HTTP server takes effect on
  the next request without dropping an open connection.
- A seeded bug in `binary_search` is found by `check` and shrunk to a list
  of at most three elements.
- The LSP answers hover and diagnostics in under 100 ms on the corpus tree.

**Effort.** About 22 pw (the lead and the type-system engineer for twelve
weeks, less triage time). Assumes the testing design exists by week 30.

**Bets tested.** 2 (test-writing time with fakes); supports 1 and 3
through the corpus's tests.

### Phase 9: synthesis, decisions, 0.1-interp release (weeks 40-48)

**Goal.** Turn the three verdicts into design decisions through the
pre-registered decision tree, publish everything a sceptic needs to check
the work, and release the interpreter so that it gets used rather than
demonstrated (the Zig, Gleam and Elixir pattern in
[16-validation-plan.md](16-validation-plan.md)).

**Deliverables.**

- The final report: per-bet verdict, every threshold with its measured
  value and its era label (harness, interpreter, native), all raw data,
  the corpus programs and diaries, the referee's written review including
  disagreements, and every amendment to the pre-registration listed.
- Every number in the report regenerated by a CI script from public data;
  negative results published under the same template as positive ones.
- Updates to [04](04-memory-and-ownership.md),
  [06](06-concurrency-and-capabilities.md), [16](16-validation-plan.md),
  [15-decision-log.md](15-decision-log.md) and
  [`open-questions.md`](../open-questions.md) (Q-2, Q-3, Q-5, Q-6, Q-9,
  Q-11, Q-28, Q-34) with the data.
- The 0.1-interp release: Linux and macOS tarballs; `sisu
  new/run/check/test/repl/lsp/explain`; the tour rewritten in the
  indentation syntax; a public list of what is not in it (no native code,
  no registry, no C interop). Stability tiers (stable / preview /
  experimental) with the rule that every pre-1.0 break ships with
  migration text and, where mechanical, a `sisu fix` rewrite, so that the
  syntax rewrite can keep landing without the keyword churn of Mojo's 0.x
  line ([`mojo.md`](../research/languages/mojo.md)); the syntax is
  labelled preview.
- A browser playground: the interpreter compiled to `wasm32` (possible
  because its frames are on the heap, so no stack switching is needed,
  [`interop-and-platforms.md`](../research/topics/interop-and-platforms.md)),
  permalinks, in-memory `Fs` and frozen `Clock` fakes, and an opt-in
  "submit as evidence" button that files a program and its diagnostics
  publicly. This is the cheapest channel for unsolicited outsider programs
  and the first public demonstration of capability sandboxing.

**Exit criteria.**

- Report merged with the pre-registration hash cited; every bet has a
  recorded verdict and the matching branch has an owner and a date.
- Every threshold in [16-validation-plan.md](16-validation-plan.md) has a
  measured number and a recorded go/no-go on the memory model, including
  the stop condition.
- 0.1-interp tagged and downloadable; the conformance suite passes on both
  platforms.

**Effort.** About 18 pw (three engineers for six weeks, including release
engineering and the reproduction scripts), 3 pw PI, 1 pw referee.

**Bets tested.** 1, 2 and 3, as the final verdict against the
pre-registered thresholds.

## The corpus

Programs are of two kinds. Team-written programs exist to find bugs and to
calibrate instruments; they are never counted as evidence. Outsider-written
programs are the evidence.

| ID | Program | Bets | Written by | Starts |
|---|---|---|---|---|
| E | The ten files in [examples](examples/) converted to the indentation syntax, a drop-order torture program, a 10k-connection echo server | tooling | team | Phases 0-8 |
| W | Wrong-program corpora (40 core, 60 ownership) with expected codes and named fixes; a translated slice of rustc's borrow-checker UI tests | checker | team | Phases 2-3 |
| A | 20 adversarial programs that try to alias or dangle in safe code | checker soundness | the referee | Phase 3 |
| D | Dry-run drafts of P1-P6, used to debug the toolchain and then discarded | checker bugs | team | Phase 3 |
| P1 | Tokenizer and recursive-descent parser whose AST borrows from the source buffer (view structs, lists of views, `from` returns) | 1 | one Rust-experienced author, one non-Rust author; Rust control | week 16 |
| P2 | Graph with parent pointers and mutation during traversal (a dominator tree or Dijkstra with path reconstruction; add and remove nodes while iterating) | 1, Q-1 | same pattern | week 16 |
| P3 | HTTP/1.1 server with a shared LRU cache and a background eviction task on `std.x.http` | 1, 2, 3 | one Go backend engineer, one Rust-experienced author | week 26 |
| P4 | Widget tree with parent navigation, focus and event callbacks stored in widgets (escaping closures, `dyn`, `@delegate`) | 1, Q-28 | one Swift or Kotlin application developer, one Rust-experienced author | week 22 |
| P5 | Iterator adapter library (map, filter, zip, chunks, windows, peekable, by-ref and by-value variants, one lending-style case) | 1, Q-10 | one Rust library author, one non-Rust author | week 16 |
| P6 | Application-shaped program chosen by the non-Rust authors in Phase 0 (for example a CLI that streams JSON lines through a trait-based plugin registry) | 1 | both non-Rust authors; Go control | week 16 |
| R | Red-team program: the ten patterns for which Rust needs lifetimes, with the cost of each workaround | 1 | a Rust expert briefed to break the model | week 20 |
| C1 | grep/du-style command-line tool with `Fs`, `Env`, `Proc` | 2 | an outside author, a student or junior developer | week 28 |
| S1, S2 | Two independent ports of one pre-registered 3-8 kLOC Go service with its tests | 2, Q-5 | one Go-background porter, one Rust-background porter | week 30 |
| B | `taskbench` with strategies A-E and controls in Go, JDK 21+, tokio and a C or Rust coroutine library | 3, Q-6, Q-34 | the runtime engineer; reviewed by the runtime reviewer | week 1 |
| T | Structured-concurrency semantics suite (supervisor tree restarting a panicking worker, deep recursion in a task, mixed compute and I/O starvation) under schedule exploration, plus the frame-depth histogram over everything | 3 | team | Phase 6 |
| U | Unsolicited playground submissions after 0.1 | 1 | anyone | Phase 9 |

**Who writes, and how they are found.** Twelve paid contractors: six corpus
authors (three with two or more years of production Rust, three Go, Kotlin
or Swift application developers with no Rust) at about four weeks each; one
red-team Rust expert at three weeks; two porters at five weeks; one C1
author at three weeks; two friction raters at one week each; one Go/Java
runtime reviewer at two weeks; one referee, a publicly sceptical Rust
expert, at two weeks across the whole plan. Volunteers for an unreleased
language are not a plan, so every role is paid. Recruitment starts in week
1 with a public call carrying the brief, the selection criteria and the
stipend, posted where Rust, Go and Swift/Kotlin developers read, and
contracts are signed by week 8 for authors starting at week 16. Selection
is by background diversity under published criteria. No participant is a
contributor to the repository or a public advocate of lifetime-free
ownership; eligibility is recorded. Each author gets a help desk whose
transcript is published, 24-hour bug triage, and the shared test suite for
their program. The friction diary is a deliverable, not a courtesy: `sisu
check --diary` produces the raw log automatically, the author annotates it
weekly, and the raters code it afterwards.

## Measurements and thresholds

Every bet-3 number carries an era label: harness (Phase 1, Rust code on
native stacks), interpreter (Phases 6-9, heap frames) or native (after the
experiment). Thresholds come from [16-validation-plan.md](16-validation-plan.md)
where it names one; the others are pre-registered here.

### Bet 1: ownership without named lifetimes

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

### Bet 2: capability parameters at scale

| What | How measured | Baseline | Threshold that changes the design |
|---|---|---|---|
| T2.1 capability parameters on I/O-doing functions | `sisu audit --caps` from SIR over S1, S2, P3, C1 | the Go original's `ctx` and injected-dependency parameters per I/O-doing function | median above 2 or maximum above 4: implement the reserved `with Fs, Clock` clause (16, Q-5) |
| T2.3 god structs: four or more distinct capability types bundled outside `main`'s composition root | `sisu audit --caps` | 0 | any: implement `with`; a struct taking two or three related capabilities at construction is the accepted pattern |
| Threading depth from `main` to each leaf use | `sisu audit --caps` | Go's `ctx` threading depth | reported |
| Plumbing share of porting time | diaries | none | flag above 20% |
| Time to write ten pre-specified tests with fakes | timed | the same ten against the Go original's mocks | slower: flag; more than 25% slower: redesign the `std.cap` fakes (recording fakes derived by reflection, [`testing.md`](../research/topics/testing.md)) |
| Supply-chain demonstration | a planted dependency fails to type-check without `Fs` and `Net` | none | must pass and be reproducible |

### Bet 3: colourless tasks without a moving GC

| What | How measured | Baseline | Threshold that changes the design |
|---|---|---|---|
| Memory per idle task at 100k and at 1M, after shallow use and after a 32 KiB excursion | harness, two machines | Go and JDK 21+ on the same machine | above 4x Go: stackless compilation of task bodies behind the same `scope`/`spawn` API (16) |
| Spawn-plus-join median and p99 | harness | Go | median above 3x Go with pooled stacks: runtime design review |
| Scale: 1M idle tasks on a 16 GiB machine with default sysctls | harness | none | fail: guard-page-per-task is out; pooled stacks with software probes become the design, with probe overhead on compute loops at most 3% |
| 10k-connection echo p99; 100k-connection idle RSS | harness | Go | p99 above 2x Go: runtime design review, not a language change |
| Stack exhaustion | deep-recursion tests; `--stats stack-depth` histogram over the whole corpus (interpreter era) | none | any segfault is a bug; any ordinary (bet-1 or bet-2) program whose depth exceeds the default reservation: a larger default or the stackless fallback; a default above 8 MiB re-evaluates memory at that size |
| Starvation (Q-34) | p99 latency of a probe task under N compute-bound tasks with yield points | Go's asynchronous preemption (harness); 10 ms (interpreter) | worse than Go, or p99 above 10 ms: revisit asynchronous preemption |
| Semantics: schedules explored per test, orphan tasks, unobserved failures, lock guards not dropped on unwind | interpreter, `--explore 1000` | 0 | any non-zero count is a bug; a class that needs a language change reopens D-014 |

### Parallel questions and health

| What | How measured | Baseline | Threshold that changes the design |
|---|---|---|---|
| Q-2 atomic elision | fraction of `Shared.new` sites proven task-local, validated by the monitor | none | below 80% flags the single-`Shared` design for a native benchmark; the "within 10%" benchmark in 16 needs native code and is deferred |
| Q-3 `.copy()` per 30 lines of application code | audit | 16 | more than one reopens implicit copying |
| Q-9, adapted to explicit `return` | the top-ten diagnostics list from all diaries | none | any top-ten diagnostic caused by a design rule rather than a bug gets a design review ("expected (), found T" is moot) |
| Q-11 newtype wrappers per 200 lines | audit | 16 | more than one reopens scoped instances |
| Q-28 | P4's diaries: `dyn` plus `@delegate` reported natural, or inheritance emulated with boilerplate | none | boilerplate reopens the question |
| Parser fuzzing panics and diagnostic storms | CI | 0 | any is a bug |
| `sisu check` after a one-body edit on the std-plus-corpus tree; LSP response | CI | 300 ms; 100 ms | missing by 2x is a design review per [10-tooling.md](10-tooling.md) |
| Incremental rebuild of the Rust prototype | CI | 10 s | Roc's complaint; above it, the crate layout is revisited |
| Interpreter adequacy | the whole corpus's test suites and the port's integration tests on a laptop | 10 minutes each | missing schedules a compact bytecode encoding and cached impl lookup as optimisation work, not a redesign |
| Diagnostics quality | share of borrow and `Send`/`Sync` errors in W that name a fix; author ratings per message | 100% | a message rated unhelpful by two authors is rewritten before 0.1 |
| Process | pre-registration hash cited; amendments listed; eligibility verified; raw logs published; benchmarks reproduced on a second machine by an outsider; the referee's review published unedited | all | a report that fails any of these is not published as a Sisu evidence report |

## Implementation language and architecture

**Language: Rust**, as [10-tooling.md](10-tooling.md) already says, with
Roc's objections answered rather than ignored. The reasons: Rust's
ownership, enums, pattern matching and traits are the closest existing
semantics to Sisu's, so the checker's data structures mirror a working
model and the interpreter stays honest (a Sisu move is a Rust move, a
`Shared` is an `Rc` or `Arc`, a `Drop` is a `Drop`); the interpreter borrows
its runtime from the host, which is the trick that let Elixir ship a public
release within months of starting by sitting on the BEAM
([`erlang-elixir.md`](../research/languages/erlang-elixir.md)), so the heap,
UTF-8 strings, hash maps, sockets, timers and `mmap` come from Rust and
`wordcount` runs in week 10; the bet-3 harness needs Rust or C regardless;
salsa gives the query-based incremental front end that 10 requires from the
first commit; and one implementation language keeps three people on one
codebase. Roc's two complaints ([`roc.md`](../research/languages/roc.md))
are handled by construction: compile time by a crate-per-stage workspace
(lexer, parser, resolve, typeck, sir, borrowck, interp, driver) with the
incremental rebuild time tracked in CI against a 10 s target, and panics by
the deny-level lint policy, the ICE-with-SIR-dump rule and fuzzing from
month one. Alternatives considered: Zig (Roc's choice; fastest rebuilds, but
every release breaks every program, there is no ownership model to learn
from, and the runtime pieces are thinner) and OCaml (the best language for
writing checkers, the weakest for the host-runtime role). The compiler in
Sisu stays a post-0.1 goal.

**Pipeline**, in the order data flows:

1. The lexer with Python's indentation algorithm.
2. An error-tolerant recursive-descent parser producing an AST with Error
   nodes, synchronising at NEWLINE and DEDENT.
3. Name resolution and desugaring to HIR (`T?`, `T ! E`, `?`, `else`,
   `for`, `it`, interpolation, method calls resolved to trait or inherent
   functions).
4. A bidirectional local type checker per body with explicit signatures,
   error-set subtyping, trait resolution under coherence and structural
   auto traits, producing a fully typed tree where every call is a function
   id or a (trait, method, type arguments) triple.
5. Lowering to SIR: a per-function typed control-flow graph with locals,
   basic blocks, statements (Assign, Move, Copy, Borrow{shared|mut}, Call,
   Drop, StorageDead, Yield, ContractCheck, Panic) and terminators (Goto,
   Switch, Return, Unwind). Drop elaboration inserts Drop at scope exits in
   reverse declaration order with drop flags and unwind edges; closures
   become a struct plus a function; `?` becomes a Switch; yield points go
   on loop back-edges and calls.
6. Checkers on SIR: initialisation and move dataflow; the borrow checker
   (loans from Borrow statements, live while the reference is live, checked
   against conflicting accesses; no named lifetimes because every loan's
   origin is a local, a parameter or a view field); the escape and view
   check; `Send`/`Sync` at spawn boundaries; purity for `pure`, contracts
   and comptime; the `Shared` escape analysis that tags each `Shared.new`
   site task-local or may-cross (Q-2).
7. The interpreter.

Every pass is a pure function from input to diagnostics plus output with
no internal panic on bad input, and SIR is the stable boundary a native
backend will consume. Nothing above SIR knows the interpreter exists.

**The interpreter** is a direct interpreter over SIR, which is already flat,
so no separate bytecode encoding is built until a measurement says it is
too slow. A frame is `{ body, pc, locals }`; a task is a vector of frames in
a slab so frame indices are stable; the dispatch loop is trampolined, so a
Sisu call pushes a frame and never recurses on the host stack. `Value` is a
tagged enum (integers, floats, booleans, characters, unit, string views,
strings, structs, enums, references as `{task, frame, local, projection}`,
`Shared`, host objects, closures, `Moved`). Heap objects (`list`, `dict`,
`String`, `Box`, `Shared`) are host objects whose Rust ownership mirrors
Sisu's. Generic bodies run polymorphically with type arguments as runtime
values; there is no monomorphisation, so the instantiation budget and
layout sharing are native-backend questions and are not measured here. A
Move transfers the value and leaves `Moved` behind; a Borrow produces a
reference and bumps the referent's borrow count; StorageDead and Drop
decrement it; the monitor treats use of `Moved`, conflicting borrows,
drop-while-borrowed, a view outliving its frame, a non-`Send` value
crossing a task and a task-local-tagged `Shared` crossing a task as
internal soundness errors that fail the run with the program, the seed and
the SIR dump. Drop order runs the user `Drop` then the fields in reverse
declaration order on both the normal and the unwind edges. A Panic
terminator puts the task in an unwinding state; the loop pops frames along
unwind edges running Drops; a panic inside a Drop while unwinding aborts;
when the root frame is popped the task finishes as panicked and its scope
sees `TaskPanicked` on join.

**Tasks** are green threads inside the interpreter on one OS thread, with a
run queue, a parked set, a timer wheel and a nonblocking I/O poller. A
`scope` is a view value tied to its frame; `spawn` creates a task with a
fresh frame stack that may hold references into the scope's frame, which
is legal because the scope joins before that frame is popped. Every
blocking intrinsic parks the task and checks the cancellation flag; yield
points decrement a reduction budget and requeue at zero. Scheduling is
deterministic and seedable, so tests replay under many interleavings. An
M:N parallel interpreter is not built; parallel throughput is a
native-runtime question. The harness in Phase 1 is a separate crate
because stack strategy is a property of the runtime, not of the language,
and must be settled before a backend is designed.

**Capabilities.** `System` is built by the driver only when `main(sys:
System)` is declared; `Fs`, `Net`, `Clock`, `Rand`, `Env` and `Proc` are
opaque host objects whose methods are intrinsics; attenuations are wrapper
host objects enforcing the restriction on every call; fakes are written in
Sisu over the `std.cap` traits; the interpreter exposes no I/O intrinsic
outside these objects except the standard streams.

**Comptime** is the same interpreter with a fuel counter, a memory limit
and a host with no `System` (only `embed`), invoked by the type checker for
`const`, array lengths, `derive` and `comptime` blocks; this is what
[08-metaprogramming.md](08-metaprogramming.md) and D-052 already require.

**Hot replacement.** Calls go through a function table of reference-counted
bodies, the per-definition indirection point of D-053; `--watch` re-runs
the queries and swaps bodies whose signature hash is unchanged, while frames
in flight keep their old body. Replacement is restricted to unchanged
signatures and takes effect at the next call, because a swapped body with
different drop behaviour under a live frame would be unsound; frame-level
fix-and-resume is demonstrated, not shipped.

**Incrementality.** Queries at file, module and body granularity are shared
by `sisu check`, `sisu run` and `sisu lsp`, so the three never disagree.

**After the verdict**, the same SIR feeds a Cranelift debug backend and the
`full` runtime profile, which rerun the Phase 1 harness on real Sisu frames
and close Q-6 finally; the interpreter keeps serving `sisu run`, the REPL,
scripts, the playground and the `single` profile.

## Team and timeline

**Roles.** A lead who owns SIR, drop elaboration, the ownership checker and
the monitor; a type-system engineer who owns the type checker, traits,
comptime, the test surface and the LSP; a runtime engineer who owns the
harness, the interpreter loop, the scheduler, `std.cap`, std in Sisu and
`std.x.http`; the owner or a delegate as PI at half time, who writes the
pre-registration, recruits, runs the studies, rates nothing and analyses.
The syntax rewrite and the testing design stay with the owner on their own
timelines.

| Weeks | Phase | Who | Gate |
|---|---|---|---|
| 1-4 | 0 skeleton, pre-registration, desk audits | lead, type-system engineer, PI, audit contractor | pre-registration signed |
| 1-8 | 1 stack harness | runtime engineer, reviewer | week 8: bet 3 stacks, Q-6 provisional |
| 5-10 | 2 core language, errors, Drop | lead, type-system engineer; runtime engineer from week 9 | |
| 9-16 | 3 ownership checker and monitor | lead; type-system engineer from week 11; referee | outsiders may start |
| 15-22 | 4 traits, generics, std | lead, type-system engineer | |
| 16-32 | 5 bet-1 corpus study | PI, six authors, red team, raters; lead on triage | week 24 interim; week 32: bet 1 |
| 19-30 | 6 tasks, capabilities, HTTP in Sisu | runtime engineer; type-system engineer from week 23 | |
| 30-40 | 7 bet-2 port study | PI, two porters, C1 author; runtime engineer on support | week 40: bet 2 |
| 31-42 | 8 test surface, comptime, hot replacement, REPL, LSP | lead, type-system engineer | |
| 40-48 | 9 report, decisions, 0.1-interp | everyone, referee | week 48: release |

Never more than three phases run at once, and no week needs more than
three engineers. Core engineering is about 133 pw against 144 available,
which leaves about 8% slack; the 50% slip budgeted on Phase 3 consumes it
and a little more, so the honest calendar is 48 weeks nominal and 52 with
the slip spent. PI effort is about 26 pw at half time. Contractors total
about 48 weeks.

**Scenario A: one or two people, part-time.** Two people at half time are
one FTE. The cheap and decisive pieces come first: Phase 1's harness and
Phase 0's desk audits (about 11 pw) settle Q-6 and give a bet-1 prior in
the first quarter. Then Phases 0, 2 and 3 at one FTE (about 42 pw) put the
ownership checker and the monitor in a public repository at about month
13, and a reduced bet-1 study (three outside authors on P1, P2 and P4, the
two stop-condition programs plus the parser, with the red team and the
authors' own diaries as the only control) gives the stop-condition answer
at about month 18-20. There is no Phase 6-8, no bet-2 study and no 0.1
until funding arrives; what exists is a checker plus interpreter that
outsiders can run. One person at part time should do Phase 1 and the desk
audits only (about six months at half time), publish the results and use
them to raise Scenario B; a one-person compiler at half time reaches the
ownership checker in about two years and the corpus study never, which is
the Carbon, Hylo, Roc, Vale and Austral calendar that
[16-validation-plan.md](16-validation-plan.md) says loses the audience
([`carbon.md`](../research/languages/carbon.md): four years in, nothing to
evaluate).

**Scenario B: a funded team of three to five.** Three engineers plus the
half-time PI is the table above: a bet-3 answer at week 8, bet 1 at week
32, bet 2 at week 40, the report and 0.1-interp at week 48. A fourth
engineer takes std, the host builtins, the test surface and the playground,
which removes the Phase 6/8 overlap and buys about four weeks of calendar
and real slack. A fifth starts the Cranelift debug backend from SIR at week
30 so that bet 3's numbers on real Sisu frames follow within a quarter of
0.1. The calendar does not drop below about 40 weeks with any team size,
because the studies are serial: outsiders need the instrument, the port
needs the tasks, and the verdict needs the raters.

**Calibration from the research.** Elixir had a public release in 2012 and
1.0 in 2014 by borrowing the BEAM
([`erlang-elixir.md`](../research/languages/erlang-elixir.md)); Zig's first
tagged release, 0.1.1, came about two years in
([`zig.md`](../research/languages/zig.md)); Gleam went from first public
release in 2019 to 1.0 in 2024 on one person's time
([`gleam.md`](../research/languages/gleam.md)); Roc has had no 0.1 in seven
years and rewrote its compiler ([`roc.md`](../research/languages/roc.md)).
With two full-time engineers the plan stretches to about 14 months; with
one, do not start the compiler.

## What is deliberately not built

- Any native backend (LLVM, Cranelift, the self-hosted debug backend), and
  therefore monomorphisation, instantiation sharing, the instantiation
  budget, DWARF and the debug ABI beyond what hot replacement needs in the
  interpreter. The one native artefact is the Phase 1 harness, which
  contains no Sisu.
- An M:N parallel scheduler inside the interpreter. The interpreter runs
  the `single` profile deterministically; parallel throughput is a
  native-runtime question.
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
- `sisu fmt`, `sisu lint`, `sisu doc`, `sisu fix` beyond the pre-1.0
  migration rule, editions, the teaching profile (Q-41 waits for a
  classroom trial after the verdict), the Unicode confusable lints.
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
  `Decimal`, persistent collections, structured logging.
- Fix-and-resume from a chosen frame; hot replacement is body swap at the
  next call only.
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

## Decision points

The pre-registration records every branch below before data arrives, with
an owner and a time box, so that a result changes the design rather than
starting an argument. Each bet's verdict is one of holds, fails or not
shown. "Not shown" means the measured effect is inside the noise of a
six-author study; it is reported as such, no fallback is built on it, and
the next data comes from the playground's unsolicited corpus and from
rerunning the corpus on the native backend.

### Bet 1 (D-004, Q-1), against section 1 of [16-validation-plan.md](16-validation-plan.md)

- **Holds** (T1.1-T1.3 pass): D-004 stands; the generation-checked
  reference flag is removed; the proposal's claim that the model removes
  most of Rust's learning curve is kept, with the measured numbers
  attached.
- **Hard fail** (more than one program needs `unsafe` for an ordinary
  pattern, or P2 or P4 cannot be written): run the pre-designed
  generation-checked reference tier on the failing programs with two new
  authors (four weeks); if still failing, opt-in named lifetimes on `view
  struct` only, which is 16's second fallback; if that fails, the remaining
  no-GC option is Swift-style implicit reference types (compiler-counted
  `Shared` for class-like types). Beyond that is 16's stop clause, the
  rewrite as "Go with sum types, capabilities and contracts" on a tracing
  GC. That clause contradicts the owner's no-GC decision (Q-1), so the
  pre-registration says what happens at that branch: the team escalates
  the data to the owner as evidence and does not act on it. Agreeing this
  in Phase 0 is what makes the study able to change the design.
- **Friction fail only** (T1.3 or T1.5 fails with expressiveness holding):
  if the raters attribute most events to diagnostics (category a), fix the
  diagnostics and re-run two programs with two new authors (four weeks); if
  to rules (category b), adopt the specific Rust refinements that fixed the
  same complaints (two-phase borrows, disjoint closure captures, field-level
  partial borrows) and re-run; if friction is still not materially below
  Rust's, the "removes most of the learning curve" claim in
  [04-memory-and-ownership.md](04-memory-and-ownership.md) is withdrawn,
  the design continues on the expressiveness result, and the stop clause's
  second condition is reported to the owner as above.
- **Soundness** (any monitor violation on an accepted program): 0.1 is
  blocked until fixed; a violation involving views reopens the view and
  escape rules before any friction number is interpreted, because an
  unsound checker understates friction.

### Bet 2 (D-015, Q-5), against section 2 of [16-validation-plan.md](16-validation-plan.md)

- **Holds**: the `with Fs, Clock` clause stays reserved; the supply-chain
  demonstration and the attenuation counts go into
  [06-concurrency-and-capabilities.md](06-concurrency-and-capabilities.md).
- **Fails** (median above two capability parameters on I/O-doing
  functions, or a god struct outside the composition root): implement the
  reserved `with` clause as 16 says, re-port the worst module (two weeks),
  recount; if bundling persists, downgrade the supply-chain claim to
  coarse-grained capabilities and document the composition-root pattern as
  the idiom.
- **Fakes slower than mocks by more than 25%**: redesign the `std.cap`
  fakes (recording fakes derived by reflection) before 0.1; the testability
  claim is reported with the ratio either way.

### Bet 3 (D-014, Q-6), against section 3 of [16-validation-plan.md](16-validation-plan.md)

- **Holds at week 8**: Q-6 is closed provisionally with the harness era
  label (strategy, default reservation, probes versus guard pages, `spawn`
  ambient), and finally when the native backend reruns the harness on Sisu
  frames.
- **Fails on memory or exhaustion** (above 4x Go's per-task memory, or
  exhaustion in ordinary corpus code): stackless compilation of task bodies
  behind the same `scope`/`spawn` API, as 16 says. Because a suspended
  stackless frame is heap state that can hold `ref`s into itself, Phase 3
  designs the escape rule to treat yield points as frame boundaries before
  the corpus study, and the hot-replacement design is re-costed from Phase
  1's written estimate. This is why bet 3 runs first.
- **Fails on creation cost or VMA limits only**: pooled stacks with
  software probes become the design; the harness is rerun on that strategy
  (two weeks).
- **Starvation** (p99 worse than Go in the harness, or above 10 ms in the
  interpreter): asynchronous preemption is revisited (Q-34).
- **Semantics**: a class of orphan, unobserved failure or cancellation bug
  that needs a language change rather than an interpreter fix reopens
  D-014.

### Parallel questions

Q-2, Q-3, Q-9 (adapted), Q-11, Q-28 and Q-34 follow the thresholds in the
table; each gets an entry in [`open-questions.md`](../open-questions.md)
with the data at Phase 9. D-038's compile-speed targets are measured on the
interpreter-era front end only; the native targets wait for the backend.

## Risks

| Risk | Mitigation |
|---|---|
| The view and escape checker is novel and Phase 3 slips by 50% or more | The conservative lexical rule ships first; liveness-based borrow ends are added after the first outsider program hits a false positive, with every false positive logged as data. The slip is budgeted and the slack is spent here first. |
| Checker bugs and weak diagnostics read as friction and confound bet 1 | 24-hour triage, bugs excluded from friction but published, diagnostics in the name-the-fix form before any outsider starts, raters separating "diagnostic unclear" from "rule rejected a sound design", dry-run drafts that find bugs before outsiders do. |
| Small n: six authors and two porters cannot show small effects, and individual variance dominates (Prechelt, [`studies.md`](../research/evidence/studies.md)) | Within-subject controls, counterbalanced order, thresholds that demand large effects, and "not shown" as a legal verdict. |
| Recruiting genuine outsiders who are competent and not fans; a corpus written by sympathisers convinces nobody | Paid contracts, published eligibility criteria, an external sceptic as referee, a public help-desk log; recruitment starts week 1 with contracts by week 8, because three weeks is not enough. |
| The interpreter cannot answer the stack half of bet 3; interpreter-era bet-3 numbers are the stackless design in disguise | The harness answers it in Phase 1 with native code; every bet-3 number carries an era label; the frame-depth histogram supplies what the harness cannot; Q-6 closes finally on the native backend. |
| Q-6's premise is partly wrong (Loom copies frames to heap chunks; guard pages hit `vm.max_map_count`) | The feasibility note corrects the record before measurement; C and Rust coroutine libraries are controls; pooled stacks with software probes are a first-class condition. |
| A stackless fallback reintroduces self-referential suspended frames that bet 1 claims to have removed | Bet 3 runs first so Phase 3 can design for it; Phase 1 writes the cost estimate before it is needed. |
| Benchmark unfairness: mis-tuned Go or JVM controls flatter Sisu | Controls configured and reviewed by the runtime reviewer, every configuration reported, second-machine reproduction by an outsider. |
| The pre-registered stop clause contradicts the owner's no-GC decision | The branch is agreed in Phase 0: escalate as evidence, do not act. Without that agreement the study cannot change the design and is not worth running. |
| The indentation grammar is still being rewritten and the parser keeps moving | Python's tokenizer rules and the proposal's keywords are adopted verbatim and frozen for the experiment; the corpus is written against them and mechanically migrated; stability tiers and `sisu fix` rewrites cover later changes. |
| Rust prototype compile times slow the team (Roc's central complaint) | Crate-per-stage layout from Phase 0 and a 10 s incremental target tracked in CI. |
| The interpreter is too slow for P3, the echo server or the port at test load | Adequacy thresholds are tied to the corpus's test suites, not to a CPython comparison; a miss schedules the bytecode encoding and cached impl lookup as optimisation work after the semantics are right. |
| Only a single-threaded interpreter runs Sisu code, so `Send`/`Sync` and the Q-2 tagging never meet real parallelism | The monitor validates both at task boundaries under schedule exploration; a class of concurrency unsoundness that only parallel execution reveals is accepted as an interpreter-era gap and is the first test on the native `full` runtime. |
| The dynamic monitor finds unsoundness only on programs that exist | Mutation fuzzing of accepted programs and the referee's adversarial programs reduce, but do not remove, this; the mechanised core stays a later research collaboration (Q-38). |
| Hot replacement interacts with ownership: a swapped body with different drop behaviour under a live frame is unsound | Replacement is restricted to unchanged signatures and swaps at the next call; it is exercised under the monitor during the corpus study. |
| Interpreter-era authors feel no performance pressure, so Q-3's copy count and the friction numbers are flattered | Diaries record performance intent; the corpus is rerun on the native backend after the experiment and the counts compared. |
| Scope creep from the interpreter being a language feature and from the testing design | The not-built list is part of the pre-registration; additions need an amendment. |
| The implementation becomes the specification | Every conformance file cites a proposal chapter from Phase 0; anything only the interpreter decides (evaluation order, overflow in comptime) is listed for the specification draft. |
| `Net.fake` with an in-process stream pair is easy to underestimate | Budgeted inside Phase 6 as a named deliverable. |
| Q-21 (governance) and Q-23 (numerics) are untouched | On purpose; a one-page phase-1 note saying who decides is all the release needs, and the owner decides that. |

## Proposed amendments (October 2026)

The unconventional-languages round ([19-novel-features.md](19-novel-features.md))
proposes six amendments to this plan, most argued to strengthen a bet's
measurement rather than add polish (the binding count and the model arm
measure questions of their own), and each needing the pre-registration
amended before any participant writes a line. They are the owner's call
(Q-49); none is adopted here.

| Amendment | Phase | Cost | Strengthens |
|---|---|---|---|
| One io_uring cancellation test into stack buffers: cancel a million in-flight reads and check that no buffer is written after its cancelled call has returned (D-058) | 1 | days | bet 3's cancellation semantics |
| The dynamic ownership monitor as a counterexample generator: rerun every rejected corpus program under the monitor across its tests; rejections that never reach a conflict are candidate false positives (N-26), team-only | 3 | about 1 week | bet 1's false-positive accounting |
| Count the bindings whose type is fixed by a later statement (N-48) | 5 | hours | Q-50 |
| The `sim` profile: fault-injecting `Net` and crash-consistent `Fs` fakes, PCT and bounded exhaustive scheduling, simulated crash and restart, on top of the 1000-schedule run Phase 6 already requires (N-03) | 6, 8 | about 4 weeks | bet 3's cancellation and panic semantics under faults |
| The capability log, `sisu replay` and a replay token on every failing test (N-04) | 8 | about 1 week | corpus triage; schedule exploration |
| A model arm: frontier models solve the corpus tasks with nothing, a pocket reference, or reference plus API digest, with and without compiler feedback, in the playground sandbox (N-50) | 9 | model-API money; a PI's time | the AI-assistance penalty measured on day one |

About six person-weeks in total, to be found by trimming Phase 8's polish
or extending the plan by a month. Principle 6 ("nothing beyond the bets
before the verdict") is kept: everything else in 19 that is adopt or
tooling waits for the verdict, with one exception that costs design
discipline rather than weeks: message identifiers with parameters, the
lossless syntax tree and a checker that produces types for broken code
(D-065, N-30, N-40) belong in Phase 0's diagnostics infrastructure and the
Phase 2 checker from the start, because none of the three can be
retrofitted.
