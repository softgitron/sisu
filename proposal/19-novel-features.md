# 19 - Features that exist in no language: an assessment

The owner asked two things after the proposal's second draft: look at the
languages the first round skipped (block and visual languages, spreadsheets,
synchronous and dataflow languages, logic and constraint systems, live
coding, game scripting, configuration languages, the holes-and-synthesis
tradition) and think about features that exist in no language at all, then
assess whether Sisu would be better with them. Fourteen family studies in
[`../research/unconventional/`](../research/unconventional/) and three
ideation files in [`../research/ideation/`](../research/ideation/) produced
about 130 candidates; [`unconventional-ideas.md`](../research/topics/unconventional-ideas.md)
synthesises what the families teach. This document merges the candidates,
gives each a verdict, and lists what the proposal changes as a result.

**How verdicts were reached.** Principle P10 ("steal, do not invent") still
holds: a candidate is *adopt* only when each of its parts has shipped in
some language, and a genuinely new combination is at most *prototype*, with
the measurement that would promote it. The verdict vocabulary is *adopt*
(into the v1 design, recorded as a leaning decision), *rule* (a design or
specification rule), *prototype* (in the experiment as a named amendment,
or after 0.1), *tooling* (no language change), *std* or *std.x* (a library
shape decided now because it is slow to add later), and *reject*. "New"
means the agents found it in no system they looked at, which is a weaker
claim than "exists nowhere"; the family files say what was searched.

## What is new

The surprise of the round is how little is new *and* valuable, and how
much of what is both follows from three decided properties: every effect
passes through a capability value, one runtime owns every scheduling
decision and yields only at known points, and every heap allocation is
visible in a type. The following candidates were found in no language, and
each is a known technique turned into a checked guarantee by those
properties:

1. **`pure` that is deterministic across runs, processes, thread counts
   and targets**, checked by the compiler (N-01, with N-02 for floating
   point). Haskell's purity, the `Par` monad and LVish give deterministic
   parallelism without covering hash order or float code generation;
   Deterministic Parallel Java stayed a research language; Starlark and
   Joe-E are hermetic within one evaluation but say nothing about threads
   or targets.
2. **A simulation the program provably cannot escape** (N-03). FoundationDB
   and TigerBeetle hold theirs by discipline; a `sim` build of Sisu fails to
   compile if reachable code could reach the world through an unmodelled
   `extern`, and faults are drawn only from each method's declared error
   set.
3. **Record and replay at a cut whose completeness the type system
   verifies** (N-04). R2 let developers choose the recorded interface but
   left completeness to them; Elm has the property through purity.
4. **Programs the borrow checker rejected still run**, under the dynamic
   ownership monitor, so the rejection comes back as a concrete conflict
   with values or as "no conflict on this input" (N-26). GHC defers type
   errors, and Rust's 2018 migration mode downgraded some borrow errors to
   warnings with no check at all; nothing defers borrow errors with a check.
5. **Evaluation that is always on because it provably cannot touch the
   world**, for an imperative language (N-25). Unison has the gate through
   purity; Power Fx confines effects to behaviour formulas and Python in
   Excel runs without network access, which are the same discipline by
   convention rather than by type.
6. **An I/O API that borrows the caller's buffer for completion-based I/O
   and DMA**, built on a rule that views, scopes and suspended frames are
   never leaked (N-07). The rule itself has shipped, in Rust's scoped
   threads and Swift's noncopyable types; no safe language builds its I/O
   API on it, because in Rust `mem::forget` is safe.
7. **A reactive notebook whose dependency graph includes in-place
   mutation**, tracked statically (N-34). marimo's documentation
   says tracking mutation "reliably is impossible in Python".
8. **Memory and CPU budgets on structured tasks in a shared heap without a
   garbage collector** (N-11). Erlang has per-process limits because it has
   per-process heaps.
9. **A review tool that classifies weakened promises** (removed `ensures`,
   deleted tests, narrowed generators; N-41). Nothing classifies weakened
   specifications in a diff. The authority delta beside it (N-42) has a Go
   precedent, capcheck's CI gate on capability drift; Sisu's addition is
   that reach follows from types, so the report is exact up to `unsafe`.
10. **Behavioural semver**: the previous version's black-box tests run at
    publish (N-44). NoRegrets and Dependabot work on the consumer side.
11. **A review ledger keyed by content hashes of the capability slice**
    (N-43). cargo-vet records audits per version and deltas between
    versions; crev's proofs can cover parts of code; neither is keyed by
    function-level content hashes.
12. **One `why` query over every inferred fact** (N-47). rustc, Argus, the
    Whyline and Scala's `-explain` each cover one.
13. **A hole-filling loop whose judge the filler cannot touch** (N-27):
    fills edit only the hole's span, contracts become properties, tests are
    read-only, and "unfillable" is a legal answer. Read-only tests and a
    flag-and-exit option are ImpossibleBench's measured mitigations; no
    shipped toolchain filters fills by the tests and contracts that reach
    the hole.
14. **Loudness relative to the training-data majority** as a design rule
    (N-49): where Sisu spells something as Python does, it means the same
    or fails to compile.

Everything else in this document is a transplant that Sisu can do unusually
well, a library shape, or a rejection.

## Gaps in the decided design found on the way

Three files flagged the first gap independently, from different angles
and with different fixes, and the determinism candidates cannot be
assessed until it is closed. These are
recorded as open questions for the owner because they touch decisions in
the log.

| Gap | Where | Question |
|---|---|---|
| **F1.** D-049 randomises `dict` iteration per process, so a `pure` function that lists a `dict`'s keys returns different lists in different processes. That contradicts [06](06-concurrency-and-capabilities.md) ("provably pure"), makes memoisation across processes unsound, breaks replay, and forces a choice between reproducible builds and [08](08-metaprogramming.md)'s promise that `comptime` runs "exactly what would run at run time". Python separated flooding resistance (a keyed SipHash) from iteration (insertion order) in 3.7. | [config 1](../research/unconventional/config-and-total-languages.md), [systems F1](../research/ideation/systems-and-determinism.md), [AI-era A11](../research/ideation/ai-era-and-evolution.md) | Q-42 |
| **F2.** Floating point is specified only as "IEEE 754" ([03](03-types-and-generics.md)); the comptime promise fails as soon as a backend fuses `a * b + c` or links a different `libm`. | [systems F2](../research/ideation/systems-and-determinism.md) | Q-44 |
| **F3.** A scope reports "the first failure" in time, so on the `full` runtime two runs of one program can report different errors. Right for effectful code; wrong inside `pure`. | [systems F3](../research/ideation/systems-and-determinism.md) | Q-43 |
| **F4.** Nothing says what a `Mutex` does when unwinding drops its guard; a sibling or a restarted worker can see half-updated state even when the protected type declares an `invariant`. Rust poisons and calls `UnwindSafe` "a speed bump"; its non-poisoning locks are on nightly. | [systems F4](../research/ideation/systems-and-determinism.md) | Q-45 |
| **F5.** Function types do not say whether a closure mutates its captures or whether it is `pure`; `par` bodies, bit-deterministic `pure` and `@no_block` on closure parameters all need it. | [parallel 1](../research/unconventional/parallel-languages.md), [systems 1](../research/ideation/systems-and-determinism.md) | Q-43 |
| **F6.** Capabilities are `Copy` ([06](06-concurrency-and-capabilities.md)), so authority can be narrowed but never taken back, and a callee can keep what it was lent. | [capabilities 3](../research/unconventional/capabilities-distribution-and-resources.md) | Q-51 |

## The candidates

| ID | Candidate | New? | Verdict | Confidence | Source |
|---|---|---|---|---|---|
| N-01 | Bit-deterministic `pure` | new as a checked guarantee | adopt (Q-42, Q-43) | medium-high | systems 1 (synchronous 4 is the alternative) |
| N-02 | Reproducible floating point by default | new across thread counts | adopt the strict default (Q-44); rest with Q-23 | medium | systems 4 |
| N-03 | `sim` profile: deterministic simulation, typed fault injection, model-checking explorer | new as a compiler guarantee | prototype in the experiment | high | systems 2, live 1, synchronous 1, logic 1 |
| N-04 | Record and replay at the capability boundary; replay tokens; checked cut; reverse debugging | new as a checked cut | log and tokens: prototype in the experiment; rest: tooling | medium-high / medium | end-user 4, systems 3, AI-era A8, live 2, 3, 7, incremental 3 |
| N-05 | `exhaustive[T]` small-scope generators for `check` | no | std.test | medium-high | logic 2 |
| N-06 | Capability interceptors; trace context inherited through scopes | no | std | medium-high | incremental 2; the trace context is this document's addition |
| N-07 | Guaranteed-drop views, scopes and suspended frames | new as an I/O guarantee | rule, adopt | high | systems 7 |
| N-08 | `@no_alloc`, `@no_block`, `@stack(max=N)` | no | adopt; `@stack` with the native backend | medium-high | systems 5, capabilities 5 |
| N-09 | `@no_panic` | no | tooling now; attribute after 0.1 | medium | systems 6 |
| N-10 | Regions as allocator-typed views | no (Rust 1.100, stabilisation merged) | adopt as a `std.collections` rule | medium | systems 8 |
| N-11 | Memory and CPU budgets on tasks | new for shared heaps without GC | prototype after 0.1 | low-medium | systems 9 |
| N-12 | Static lock ordering | no | tooling (lockdep), then std.x | medium | systems 10 |
| N-13 | Priority tasks with compile-time ceilings for `freestanding` | no | std.x | medium | systems 11 |
| N-14 | `Mutex` unwinding: re-check the invariant, poison only on failure | no (Rust poisons; non-poisoning locks on nightly) | Q-45 | medium | systems F4 |
| N-15 | `@must_consume` propagates through containers | no (Move) | adopt; settles Q-37's open half | high / medium | capabilities 1 |
| N-16 | Revocable and lent capabilities | no | std.cap (Q-51) | medium / low-medium | capabilities 3 |
| N-17 | Pure-package label; capability-gated plugin loading | no | label: tooling; loader: reject for now | medium / low | capabilities 4 |
| N-18 | `std.par`: `par_iter`, `par.join`, reducers with a fixed merge order; `par` keyword | no | std; keyword to Q-47 | medium-high | parallel 1, 2, 4, 5, systems 12 |
| N-19 | A non-blocking pool for `pure` bodies, apart from tasks | no | rule | high / medium | parallel 3 |
| N-20 | `SoaList[T]`: layout-polymorphic collections | no | std.collections, with Q-27 | medium-high | schedule 1, systems 13 |
| N-21 | `@refines(reference)`: generated equivalence tests | no | std.test | medium | schedule 2 |
| N-22 | Checked no-op directives (`@vectorize`, `@unroll`, `@tile`) | no (ISPC) | later, with the native backend and Q-23 | medium | schedule 3 |
| N-23 | Access-set scheduling library | no (Bluespec, Bevy) | std.x later | low-medium | schedule 4 |
| N-24 | `schedule` blocks | no (Halide) | reject | medium-high | schedule, systems 13 |
| N-25 | Always-on evaluation gated by static authority: examples, probes, counterexamples, worksheets, doc values | new for imperative languages | tooling after 0.1; `sisu doc` values: adopt | medium-high / medium | end-user 1, live 4, spreadsheets 1, 4, natural 1 |
| N-26 | Run anyway: deferred errors, holes, the monitor as counterexample | new for ownership errors | prototype in the experiment (team instrument); teaching after | medium | end-user 2, game 1 |
| N-27 | Typed hole `todo("name")`: report, release gate, `sisu fill`, judge isolation | new as a combination | adopt the hole (Q-46); fill engine: tooling | high / medium | AI-era A1, holes 1-5, visual 4 |
| N-28 | Notional-machine stepper and permissions overlay | no (Aquascope) | tooling: overlay after the bet-1 verdict, stepper after 0.1 | medium-high / medium | end-user 3 |
| N-29 | Monotone teaching ladder, teacher-defined ladders as data | no (HtDP) | adopt as the shape of Q-41 | medium | end-user 9, spreadsheets 2 |
| N-30 | Message catalog and lossless syntax tree now; block, frame and keyword projections later | no | adopt / tooling later | medium-high / low | end-user 8, visual 1, 5, 6 |
| N-31 | Structural editing commands; a palette that knows types and capabilities | no | tooling | medium-high / medium | visual 2, 3 |
| N-32 | Concrete-first authoring; approve a value to get a test | no | tooling | medium / low | end-user 10, spreadsheets 3, 5 |
| N-33 | `View` as a `pure` trait; `{expr=}` interpolation | no | std.x; `{expr=}`: adopt | medium-high | end-user 12, live 5 |
| N-34 | Worksheets; a reactive notebook with mutation-exact dependencies | notebook: new | worksheet: tooling after 0.1; notebook: prototype after Q-23 and Q-48 | medium / low-medium | end-user 5, spreadsheets 1, live 8 |
| N-35 | Epoch-consistent hot replacement; drift-free time; sketch kit; tick model; tracepoints | no | adopt (refines D-053); std; first-party package | high / medium | esoteric 1, 2, 4, game 2, 3, 4, end-user 11 |
| N-36 | Transcript tests | no (Inform, doctest) | std.test | medium | natural 2 |
| N-37 | Typed tables; units of measure; `@diagnostic_name` | no | std.x after Q-23; hook: adopt with Q-27 | low-medium / medium | end-user 6, 7 |
| N-38 | Consequence lines on multi-fix diagnostics; a signature gloss | no | adopt | medium-high | end-user 13 |
| N-39 | Deterministic cost metrics for exercises | no | tooling | medium-low | game 5 |
| N-40 | A checker that produces types for broken code | no (Luau) | adopt as checker architecture | high | game 1 |
| N-41 | Specification-weakening report and gate | new | tooling for v1 | medium-high | AI-era A2 |
| N-42 | Authority delta at review and admission; cooldowns | no (capcheck, pnpm); exactness from types is new | tooling for v1 | high | AI-era A3, capabilities 4 |
| N-43 | Review ledger keyed by capability-slice hashes | new | prototype with the registry | medium-low | AI-era A4 |
| N-44 | Behavioural semver at publish | new | tooling with the registry | medium | AI-era A5 |
| N-45 | `@fix_inline` library migrations | no (Go) | tooling for v1 | high | AI-era A6 |
| N-46 | Schema lock and required migrations for persisted types | no outside languages (Buf, schema registries, OTP `code_change`); none inside one | std.x and tooling after the experiment | medium | AI-era A7 |
| N-47 | `sisu why` over every inferred fact | new as one query | tooling for v1 | medium-high | AI-era A9, holes 8, end-user 4 |
| N-48 | Left-to-right binding types; a completion-engine API | no | measure in the experiment (Q-50) | medium-low | AI-era A10 |
| N-49 | Loud divergence from Python's and Rust's meanings | new as a principle | rule, adopt | medium-high | AI-era A11 |
| N-50 | Context-window digest; a model arm in the experiment | no (llms.txt) | prototype in the experiment | medium | AI-era A12 |
| N-51 | Verification re-timed; solver-aided `verify` and `solve`; holes in specifications | no | post-1.0 as decided; feasibility study after the verdict | medium / medium-low | AI-era A13, logic 4, holes 6, 9 |
| N-52 | `Model` capability and `derive(Schema)` | no | std.x | medium | holes 7 |
| N-53 | Chained fallible conditions | no (Swift, Rust 1.88) | adopt | high | logic 3 |
| N-54 | The manifest stays data; structured target conditions | no | adopt | high | config 2 |
| N-55 | Comptime budgets owned by the spending package | no (Rust 1.72's lesson) | adopt | medium-high | config 3 |
| N-56 | `sisu eval`: Sisu as its own configuration language | no (Pkl) | tooling | medium | config 4 |
| N-57 | A maximum lifetime for `@preview` features | no | adopt | medium | config 6 |
| N-58 | Conflict-checked configuration merging | no (CUE) | std.x | medium-low | config 5 |
| N-59 | Grammars as comptime values (`std.x.peg`) | no (LPeg) | std.x | medium | natural 3 |
| N-60 | `task.race`, search combinators, lockdep traces | no | std | medium-high / medium | synchronous 2, logic 5 |
| N-61 | Eventual references and promise pipelining | no (E, Cap'n Proto) | std.x later | medium / low | capabilities 2, systems |
| N-62 | Logical-time reactors (`std.x.reactor`) | no (Lingua Franca) | std.x | medium | synchronous 3 |
| N-63 | Incremental computation (`std.x.incr`) | no (Salsa) | std.x | medium | incremental 1 |
| N-64 | Differentiation and probabilistic programming as libraries | no | library; reject in the language | low-medium | incremental 4 |
| N-65 | One definitional interpreter for the runner, explorer and symbolic evaluator | no (K) | rule | medium-low | logic 6 |
| N-66 | Read-only diagrams generated from code | no | tooling | low | visual 7 |

## A. Determinism, simulation and replay

### N-01 Bit-deterministic `pure`

*What.* `pure` means "the same bits for the same arguments, on every run,
process, thread count and target". Three rules: `dict` and `set` iterate in
insertion order while the SipHash key stays per process (F1); inside `pure`
code the operations that observe the schedule are errors (`select`,
`Mutex`, `Atomic`, channels with more than one sender, `try_lock`,
`try_recv`, `try_send`, `Weak.upgrade` across tasks, address-derived
values), while `task.scope`,
`spawn` and `join` stay legal; a scope inside `pure` code reports the
lowest-index failure rather than the first in time (F3). Higher-order
`pure` code demands `pure` callbacks, so `pure` enters function types (F5).

```sisu
pure def apply(state: Ledger, batch: Slice[Transfer]) -> Ledger ! TaskError:
    let parts = task.scope do s:                  # fork-join of pure tasks stays legal
        let fees = s.spawn(() -> compute_fees(batch))
        let risk = s.spawn(() -> score_risk(state, batch))
        return Ok((fees.join()?, risk.join()?))   # join order, never completion order
    let fees, risk = parts?
    return Ok(state.posted(fees, risk))           # same bits on 1 or 64 workers, x86 or ARM
```

*Precedents.* Each rule has shipped: insertion order in Python 3.7, Ruby,
JavaScript's `Map` and Starlark; Rayon's `join` propagates the first
closure's panic by index; Haskell's purity, the `Par` monad and LVish;
Joe-E's exclusion of ambient nondeterminism, which Waterken relies on for
its ETags. The combined guarantee across thread counts and targets is
checked nowhere; Temporal polices it with a linter that "will not catch all
cases" and Durable Functions with a run-time detector that "won't catch all
violations".

*Why Sisu can.* Capabilities already mark every external effect, so the
purity checker that Phase 8 builds only needs the schedule-observing
primitives added to its list.

*Buys.* Replicated state machines, lockstep simulation, reproducible builds
and pipelines, sound memoisation across processes (N-63), and the
simulation and replay candidates below, which all assume it.

*Costs and risks.* An insertion-ordered `dict` costs an index array and
tombstones (Python 3.6's compact layout was still 20-25 % smaller than its
predecessor); a second failure policy for `pure` scopes, whose failures
propagate more slowly because earlier siblings run to completion;
reversing the iteration half of D-049, whose stated reason in Draft 2 of
[11](11-stdlib.md) ("lets code depend on it silently") becomes the point:
a specified order is safe to depend on.

*Verdict.* Adopt (D-074 pending Q-42 and Q-43). Confidence medium-high;
the `dict` change is the contestable part. *Settled by* the corpus's
`pure` functions run under `--explore 1000` and later natively with 1 and
16 workers, outputs diffed; more than one rejected `pure` function per 500
lines of pure code reopens the primitive list; an insertion-ordered `dict`
within 10 % of a plain SipHash map on the corpus.

### N-02 Reproducible floating point by default

*What.* One source gives the same bits on every target and optimisation
level: no implicit contraction (an FMA only through `a.mul_add(b, c)`), no
reassociation or fast-math, `std.math` correctly rounded, NaN payloads
canonicalised where observable, and `sum`, `product` and `dot` defined as a
fixed-shape pairwise reduction sequentially and in `par_iter`, with chunks
fixed by index. `@float(fast)` opts a function out and visibly forfeits
N-01's guarantee.

*Precedents.* Java made all floating point strict in Java 17 (JEP 306);
WebAssembly's scalar floats are deterministic except for NaN bits; CORE-MATH's
correctly rounded functions ship in glibc 2.42 and 2.43; ReproBLAS;
Box2D v3 reached cross-platform, cross-thread-count determinism by
disabling FMA, avoiding fast-math and fixing merge order. Clang contracts
within a statement by default, the kind of default that makes x86 and ARM
disagree.

*Costs.* Dense kernels lose peak throughput until they use `mul_add` or
`std.simd` explicitly; correctly rounded functions are slower on some
inputs; a fixed reduction shape constrains chunking. Deciding now costs
nothing because no code depends on the alternative; Java needed more than
twenty years and the end of x87.

*Verdict.* Adopt the strict default and the explicit-only FMA rule (D-074,
Q-44); reproducible reductions and `std.math` go with Q-23. Confidence
medium. *Settled by* a numerical corpus built for x86-64 and AArch64 at
both optimisation levels with bit equality required, and a strict-versus-fast
throughput gap above 25 % on the kernels that matter reopening the default.

### N-03 The `sim` profile: deterministic simulation with typed fault injection

*What.* `sisu test --sim` runs a test in a fourth runtime profile where
`System` is entirely simulated: a clock that advances only when every task
is blocked (the test clock of [17](17-testing.md)), an in-process `Net`
with latency, loss and partitions, an `Fs` with a pessimistic durability
model in which unsynced writes vanish at a simulated crash, a seeded `Rand`,
and several nodes with their own `System` and module statics in one
process. The scheduler is seeded and pluggable: random, PCT, bounded
exhaustive with state hashing and partial-order reduction. Faults come from
types: at each capability call the simulator may return any error in that
method's declared error set and nothing else. The language-level part is a
guarantee: a `sim` build fails to compile if reachable code could escape
the simulation, which is possible only through `extern` or `unsafe` in a
`[trust]`ed package that supplies no model (the ambient standard streams
are captured per node). Explicit `choose` and `always`
make it the P-style model checker of
[logic 1](../research/unconventional/logic-constraint-and-rewriting.md).

```sisu
test "replicas converge under partitions, torn writes and crashes":
    let sim = Sim(scheduler=Pct(depth=3), faults=Faults.storage() + Faults.network(drop=0.02))
    let nodes = [sim.node("r{i}", own sys -> replica.main(sys)) for i in range(3)]   # production main
    sim.every(Duration.minutes(5), own s -> s.crash(s.pick(s.nodes())))  # unsynced writes are lost
    sim.always(own s -> len(lease_holders(s.nodes())) <= 1)               # checked after every step
    sim.run_for(Duration.hours(6))                                     # seconds of real time
    assert linearizable(sim.history())
# error[E0971]: this `sim` build reaches `sqlite.exec`, an `extern` call with no simulation model
```

*Precedents.* FoundationDB ("unlikely that we would have been able to build
FoundationDB without this technology"), TigerBeetle's VOPR, Shuttle
(PCT, "not sound", scales beyond Loom), Coyote, Go's `synctest`, Antithesis
and Hermit from outside the process, P and S3's ShardStore (16 issues kept
out of production), SPIN's partial-order reduction; LFI derived faults from
library interfaces by binary analysis. No compiler guarantees that a program
cannot escape its simulation.

*Costs.* A simulated `Net` and a crash-consistent `Fs` model (Phase 6
already calls `Net.fake` "easy to underestimate"), PCT and a bounded
exhaustive mode in the scheduler, per-node module statics. About four
person-weeks added to Phases 6 and 8.

*Risks.* An `Fs` model kinder than Linux gives false confidence, so the
model must be pessimistic (fsync failures, rename and directory
durability); simulated time hides real-time performance bugs; the `full`
profile's weak-memory behaviour of `Atomic` needs a loom-style mode of its
own.

*Verdict.* Prototype in the experiment as an amendment to Phases 6 and 8
(Q-49), because it adds faults and crashes to the 1000-schedule run that
Phase 6's exit criteria already require and so strengthens the test of bet
3's cancellation and panic semantics. Confidence high. *Settled by* a
seeded-bug study: twenty known concurrency and crash-consistency bugs from
the ALICE and Jepsen catalogues planted in the corpus HTTP server and a
small replicated log; `--sim` should find at least fifteen within a fixed
CPU budget, against fewer for random schedules alone.

### N-04 Record and replay at the capability boundary

*What.* In the interpreter every source of nondeterminism is a capability
call, the seeded scheduler or standard input, so `sisu run --record` wraps
each root capability in a logger (the same interception point as test
fakes and N-06) and writes the scheduler seed, the per-process hash key,
standard input, every capability result in order and every hot-replacement
swap. `sisu replay` re-executes with the log standing in for the world.
Every failing test prints a replay token (content hash, toolchain version,
`Rand` and scheduler seeds, shrunk case), and `sisu test --replay <token>`
reproduces it anywhere. On top, as tooling after the verdict: a cut below
`System` (`--record-at Db`) whose completeness the tool checks because code
outside `Db`'s methods was never handed another route to the world; native
`full`-profile runs that record only the order of synchronisation events
and replay in the interpreter (RecPlay's method, whose precondition,
data-race freedom, Sisu guarantees statically); a flight recorder keeping
the tail of the log in release builds; `sisu test new --from-trace` turning
a recorded failure into a regression test with preloaded fakes; reverse
execution in the debugger from checkpoints; REPL undo; and "why" queries
that answer with the dynamic slice that produced a value.

*Precedents.* rr and Pernosco at the system-call boundary (rr: at most
1.2x on Firefox tests, under 2x on important workloads); WinDbg's TTD at
the instruction level (about 10-20x); R2's interface-level recording (SQLite's log
from 890 MB to 3 MB, slowdown from 126 % to 9.6 %), with completeness the
developer's job; Elm 0.18's importable message histories; Durable Functions
and Temporal; the Whyline (debugging time cut by nearly a factor of eight
in six scenarios); Dark's trace-driven development and its post-mortem on
trace storage. No imperative language has an enforced replay boundary.

*Costs.* Logging wrappers, a versioned log format, serialisable capability
results (comptime-derived encodings; byte chunks for `Net` streams),
checkpoints for reverse execution. A log holds file contents and
environment values, so it is as sensitive as the data and needs
per-capability redaction.

*Verdict.* The interpreter log, `sisu replay` and replay tokens: prototype
in the experiment (Q-49; about one person-week in Phase 8, where corpus
triage needs them). The checked cut, native recording, flight recorder,
trace-to-test, reverse execution and why-queries: tooling after the
verdict. Confidence medium-high for the log, medium for the rest.
*Settled by* the share of corpus failures reproduced from a token alone
(target: all that do not hold the real `System`), recording overhead on the
corpus HTTP server (R2's 10 % for Apache is the bar) and log bytes per
request.

### N-05 `exhaustive[T]` small-scope generators

*What.* Beside `arbitrary[T]`, a derived `exhaustive[T](max_len=4)` that
enumerates every value up to a small size, so `check` can run every history
of up to four operations against a reference model (ShardStore's method);
refinement types bound the domains. *Precedents.* Alloy's small-scope
hypothesis, SmallCheck, the Chord bugs. *Verdict.* std.test (D-070).
Confidence medium-high; cost low.

### N-06 Capability interceptors and inherited trace context

*What.* A generic `Wrap[T]` for any capability trait, built from the
reflection behind `Fake[T]`, intercepts every call to the outside world,
visibly in `main` and typed, with no pointcuts; tests, tracing, auditing and
N-04's recording share the one interception point. A `task.scope` inherits
a context (trace and span identifiers) that `log` attaches automatically,
as Kotlin's coroutine context and Erlang's logger metadata do; this half is
the assessment's own addition and comes from no family file.
*Verdict.* std (D-075 records the shape). Confidence medium-high.

## B. Resource guarantees and systems code

### N-07 Guaranteed-drop views, scopes and suspended frames

*What.* State as a rule what the design nearly gives: a suspended task
frame, a `task.scope` and any `view` value are never leaked; each ends only
by running its `Drop`, on scope exit or during unwinding (process abort
aside), and std offers no safe `forget` or `leak` for them. The runtime can then promise that a
blocking operation that lent a buffer to the kernel or a DMA engine
returns, even when cancelled, only after the device has released the
buffer (an io_uring cancellation that returns `-EALREADY` waits for the
original completion). Completion-based I/O and DMA can borrow `mut buf`
from the caller's frame instead of taking owned buffers.

```sisu
def fill(file: File, mut buf: Slice[u8], offset: int) -> int ! {IoError, TaskError}:
    return file.read_at(mut buf, offset)     # io_uring writes straight into the caller's buffer
# Cancelled mid-flight, read_at submits a kernel cancel, parks until the kernel has let go of
# `buf`, and only then returns Err(Cancelled).
```

*Precedents.* Rust cannot offer this safely ("any object can be leaked in
Rust, trivially and safely"), so tokio-uring takes buffers by value and
`embedded-dma` requires `'static` buffers; the enabling rule has shipped
twice, in Rust's closure-based `std::thread::scope` and in Swift's
noncopyable types, whose `deinit` always runs because SE-0390 rejected a
general `forget`.

*Costs and risks.* Next to nothing in the language; cancellation latency
becomes "until the device lets go"; the rule constrains std forever. Under
`panic = abort` ([05](05-errors.md)) and on a double panic no `Drop` runs,
so a DMA-lending API must be unavailable with `panic = abort` or the abort
hook must reset the device. It
also keeps bet 3's fallback honest: if Phase 1 sends Sisu to stackless task
bodies, the state machines must stay owned by their scope and never become
droppable future values, or Sisu inherits Rust's problem.

*Verdict.* Adopt as a specification rule plus the std API shape (D-058).
Confidence high. One addition to Phase 1's harness (Q-49): cancel a million
in-flight io_uring reads into stack buffers and check that no buffer is
written after its cancelled call has returned.

### N-08 `@no_alloc`, `@no_block` and `@stack(max=N)`

*What.* Attributes checked transitively over the call graph after
monomorphisation: no heap allocation (no construction or growth of `list`,
`dict`, `String`, `Box` or `Shared`, no allocating callee); no blocking (no
capability call, lock, channel operation, `join`, `select`, `@blocking`
extern or compiler-inserted yield point); a stack bound with recursion only
through a `decreases` measure, which gives that contextual keyword its
first semantics when `@stack` arrives with the native backend (Q-16 reserves
it in v1). They are part of function types, so `dyn`
methods and closure parameters can require them, and `sisu build --report
resources` prints the bound per task entry point and per `freestanding`
image.

*Precedents.* Clang 20's `[[clang::nonblocking]]` and `[[clang::nonallocating]]`
on function types with RealtimeSanitizer as the run-time complement;
Swift's experimental `@_noAllocation` and `@_noLocks`; Ada's `pragma
Restrictions`; GNATstack; Zig's open `@stack_size` proposal. The idea
exists; Sisu's version is cheap because allocation is visible in types
(rule 7 of [04](04-memory-and-ownership.md)) and every blocking point is a
capability or a synchronisation primitive.

*Costs and risks.* One SIR pass per attribute; checking after
monomorphisation is a deliberate exception to D-017's definition-site rule,
with errors at the instantiation as in Swift and Clang; drop glue for
recursive auto-boxed types must be iterative or any function that drops a
tree has an unbounded stack; a long `@no_block` loop on a task thread
starves its siblings and deserves a lint.

*Verdict.* Adopt `@no_alloc` and `@no_block` for v1 as SIR passes built
after the verdict; `@stack` with the native backend (D-059). Confidence
medium-high. *Settled by* three realtime-shaped ports (an audio callback,
an interrupt-driven UART driver, a game frame loop): fewer than one
rejection in ten a false positive the author cannot fix by restructuring.

### N-09 `@no_panic`

*What.* The compiler shows that no panic is reachable from a function: no
trapping arithmetic that can overflow, no out-of-bounds index, no `unwrap`
or `expect`, no failing contract, no allocation failure. Without the
verifier the proof is syntactic plus interval arithmetic over refinement
bounds and `requires` clauses; the post-1.0 verifier widens it. Before the
attribute, `sisu audit --panics f` lists every potential panic site
reachable from `f`. *Precedents.* SPARK's Silver level; Wuffs, in Chrome's
GIF decoder since 2021 with every check discharged at compile time; Prusti;
the `no-panic` crate's linker trick. *Risk.* The interval analysis must be
written into the specification or programs pass and fail by compiler
version, which D-036 cannot tolerate. *Verdict.* Tooling now; the
attribute with a specified analysis after 0.1 (D-059). Confidence medium.

### N-10 Regions as allocator-typed views

*What.* Collections take their allocator as a type parameter with a
default, `list[T, A: Allocator = Global]`, and a borrowed bump `Region` is a
valid allocator, so `list[T, ref Region]` is a view by the existing rule:
it cannot escape the frame that owns the region, be stored in `Shared` or
cross `spawn`, and resetting the region needs `mut region`, which the
checker grants only once every collection using it is dead. Per-request and
per-query regions (nginx pools, PostgreSQL memory contexts, protobuf
arenas) become safe with no new mechanism and no lifetime names.

```sisu
def handle(req: Request) -> Response ! HttpError:
    with Region.bump(capacity=65536) as region:       # one region per request, freed at the dedent
        var headers = list.with_allocator(ref region)  # list[Header, ref Region]: a view
        parse_headers(req.head, mut headers)?
        return Ok(render(headers))                     # the Response is built in the global heap
```

*Precedents.* Rust merged the stabilisation of `Vec<T, A>`, `Box<T, A>` and
`impl Allocator for &A` in September 2026, for Rust 1.100, ten years after
RFC 1398; Cyclone, MLKit,
RTSJ scoped memory; Go's arena experiment is on hold because explicit
arenas are infectious to APIs. *Costs.* An allocator parameter on every
collection, hidden by the default; std naming must separate a bump `Region`
from the generational `Arena[T]` of [04](04-memory-and-ownership.md); a
function that builds a `list` must be generic over `A` to work inside a
region, which the measurement below counts. *Verdict.* Adopt as a
`std.collections` rule (D-060), because the parameter is cheap before 1.0
and slow to add after. Confidence medium. *Settled by* the corpus HTTP
server and parser with per-request and per-file regions: a 20 % end-to-end
gain keeps it, and the count of functions that had to become
allocator-generic says what it costs.

### N-11 Memory and CPU budgets on tasks

*What.* `spawn_with(memory=, cpu=)`; the runtime charges each allocation to
the nearest budgeted ancestor and credits each free; an overrun fails
through Q-4's two paths (`try_` variants return `OutOfMemory`, everything
else unwinds the task, whose joiner sees `Err(BudgetExceeded)`). A
multi-tenant server contains a pathological request instead of being
OOM-killed with every other request. *Precedents.* Erlang's
`max_heap_size` (per-process heaps), ClickHouse's per-query tracker,
WebAssembly fuel, Cloudflare Workers; Sisu's own comptime limits. No
shared-heap language without a GC attaches budgets to structured tasks.
*Costs.* Per-allocation accounting (a budget pointer in the header or
per-task size-class pages); the cost must be zero when no budget is set.
*Verdict.* Prototype after 0.1 (about one person-week in the interpreter,
whose host allocator sees every allocation). Confidence low-medium.
*Settled by* a native allocator benchmark (under 5 % on allocation-heavy
code) and the corpus server with one runaway request leaving the others'
p99 unchanged.

### N-12 Static lock ordering

*What.* An optional lock level as a comptime value in a `Mutex`'s type
with a declared partial order; acquiring out of order is a compile error
within a function and, with inferred acquired-level sets, across a
package. Before any compiler work, the interpreter runs lockdep: it
records the acquisition-order graph across all test runs and reports cycles
even when no deadlock happened. *Precedents.* Fuchsia's netstack3 lock
levels as types; Clang's `ACQUIRED_BEFORE`; Linux lockdep. Deadlock is the
one concurrency bug class [06](06-concurrency-and-capabilities.md) admits
Sisu does not prevent. *Verdict.* Tooling first (lockdep is nearly free
once Phase 6 exists), then a std.x library in netstack3's token style,
compiler inference only if the corpus produces deadlocks the tooling
misses. Confidence medium.

### N-13 Priority tasks with compile-time ceilings for `freestanding`

*What.* `freestanding` has no scheduler, so `task.scope` does not compile
there. The Stack Resource Policy fills the gap: interrupt handlers are
tasks with static priorities declared by attributes, shared resources are
fields of one struct, and a comptime function computes each resource's
ceiling, the vector table and the single-stack bound. *Precedents.* RTIC
(through a procedural macro), Ada's Ravenscar profile, Baker's SRP.
*Verdict.* std.x after 0.1; outside the experiment. Confidence medium.
*Settled by* one port of an RTIC example with comparable code size and a
computed stack bound checked against a measured high-water mark.

### N-14 `Mutex` guards during unwinding

*What.* When unwinding drops a guard, re-check the protected type's
`invariant` if it has one and poison the mutex only if the check fails, so
later `lock()` calls panic; types without invariants unlock normally.
`lock()` stays infallible and contracts get a run-time job they suit.
*Alternatives.* Always poison (Rust), never poison (Rust's nightly
`nonpoison`), a crash-isolated scope type (`UnwindSafe`, rejected as "a
speed bump"). *Verdict.* Q-45, owner's call; lean towards the invariant
re-check. Confidence medium.

### N-15 `@must_consume` propagates through containers

*What.* A container of must-consume values is must-consume (Move's
conditional abilities: `Cup<T>` has `drop` only if `T` does), discharged by
consuming iteration; `?` out of such a loop is an error unless the
remainder is returned. This settles the half of Q-37 that its "what would settle it"
defers to practice, whether generic containers need special handling. *Verdict.*
Adopt (D-064). Confidence high for propagation (Move ships it), medium for
the `?` rule. *Settled by* `list[File]` and transaction batches in the
standard library.

### N-16 Revocable and lent capabilities

*What.* `std.cap`: a caretaker gives dynamic revocation
(`let plugin_fs, revoker = fs.revocable()`); a lent capability is a `view`,
so the callee cannot keep it and its authority ends with the call at zero
run-time cost. *Precedents.* E's caretakers, `Proxy.revocable` (ES2015).
*Conflict.* Capabilities are `Copy`, so APIs must accept both forms and
`Revoked` joins every I/O error set (F6, Q-51). *Verdict.* std.cap, shape
decided with Q-51. Confidence medium for revocation, low-medium for
lending.

### N-17 Pure-package labels and capability-gated plugins

*What.* `sisu audit --authority` labels a package *pure* when its API takes
no capability types and it is not in `[trust]`; the registry shows it and
pure packages need no capability review (part of N-42). The interpreter
could also load an untrusted plugin, checked with an empty trust set,
holding only what it is handed. *Verdict.* The label: tooling for v1. The
loader: reject for now; it conflicts with the "no `eval`" non-goal, needs
the interpreter as an embeddable library, and gives the first concrete case
for a `Tasks` capability (a plugin could spawn without bound), which Q-6
should note. Confidence medium / low.

## C. Parallelism and layout

### N-18 `std.par`, and whether `par` is a keyword

*What.* `par_iter` and `par_iter_mut` with work-stealing chunks and
`Send`/`Sync` checks; `par.join` for divide and conquer (two to eight
closures, Rayon's index-ordered panic rule); reducers (`par.Sum`,
`par.Collect`, user types implementing `par.Reducer`) whose per-worker
views merge in a fixed tree shape so results are identical across runs and
core counts; adaptive splitting with a `grain=` override and a `sisu bench`
warning when a `par` loop is slower than its serial elision. The guarantee
is serial elision made exact: removing the parallelism gives the same
result, and a failing loop reports the lowest-index error. A `par:` block
whose statements run concurrently is rejected: `task.scope` already is
occam's `PAR`, X10's `finish` and Chapel's `cobegin`, and a statement block
with task semantics has no clear meaning for `return` or `?`.

```sisu
let lo, hi = xs.split_at_mut(mid)
par.join(() -> sort(mut lo), () -> sort(mut hi))    # disjoint `mut` borrows, proven today
for p in particles.par_iter_mut():                  # work-stealing chunks, `Send` checked
    p.step(dt)
```

The open half is spelling. [parallel 1](../research/unconventional/parallel-languages.md)
argues for `for x in par mut xs` and `[f(x) for x in par xs]` as a
contextual keyword after `in`, on occam's symmetry and Chapel's `forall`,
with the compiler naming the crossing variable when an iteration assigns a
shared binding; [systems 12](../research/ideation/systems-and-determinism.md)
argues the library form is enough and the keyword tempts a task per
statement. Both agree on the check. *Verdict.* The library ships in v1
(D-069); the keyword is Q-47, decided by how often the corpus writes
`par_iter` in a plain loop. Confidence medium-high.

### N-19 A non-blocking pool for `pure` bodies

*What.* A `pure` body cannot do I/O, take a lock or wait on a channel, but
it may still spawn and join tasks (N-01), so the pool's rule is stricter:
`par` bodies and `par.join` closures are `pure` and `@no_block` (no
`task.scope`, `spawn` or `join`; nested fork-join goes through `par.join`),
or mutate only their own element, and they run to completion on a worker's
stack with no task stack and no parking, which keeps fine-grained
parallelism out of bet 3's stack budget. The runtime either shares OS threads between the scheduler
and the pool or keeps two pools (Q-6).

```sisu
for url in urls.par_iter():
    let page = http.get(net, url)?   # error: `http.get` uses the capability `net`; a `par` body
                                     #   cannot block. Use task.scope and s.spawn for concurrent I/O.
```

*Verdict.* Rule, within D-069. Confidence high for the distinction, medium
for the runtime design.

### N-20 `SoaList[T]`: layout-polymorphic collections

*What.* A comptime-generated collection storing each field of `T` in its
own array; code is written against the logical struct; element access
returns a `view struct` of field references, which the ownership model
already expresses. The explicit type keeps the layout visible, against a
`@layout(soa)` attribute on the struct that would change what
`list[Particle]` costs everywhere. *Precedents.* Odin `#soa`, Zig
`MultiArrayList`, ISPC, Taichi, Futhark (automatic). *Verdict.*
std.collections, with Q-27's type construction (D-069). Confidence
medium-high. *Settled by* whether `SoaList[T]` with view elements passes
the ownership corpus without `unsafe`.

### N-21 `@refines(reference)`

*What.* The clear algorithm stays in the source as the specification; the
optimised version names it, and `sisu test` generates property tests
comparing the two on `arbitrary` inputs, with an explicit tolerance where
reassociation is intended (`@refines(blur_ref, tolerance=Ulps(2))`). A
later verifier could prove such pairs for the pure, affine subset, as Exo
does. *Verdict.* std.test (D-070). Confidence medium.

### N-22 Checked no-op directives

*What.* `@vectorize(8)`, `@unroll(4)` and `@tile(64, 64)` on a loop nest
whose independence is proven either hold or fail to compile (ISPC's lesson:
vectorisation is a guarantee, not a hope), with the loop-carried dependence
named in the error. *Conflict.* Attributes sit on the line before an item
([02](02-syntax.md)), not on statements. *Verdict.* Later, with the native backend and
Q-23's SIMD design. Confidence medium.

### N-23 Access-set scheduling

*What.* Parameter conventions already declare access (a borrowed parameter
reads, `mut` writes), so a comptime planner can build the conflict graph of
a set of functions over shared state and run non-conflicting ones in
parallel with the sequential order's result, without Bevy's access-declaring
query types. *Verdict.* std.x later; games and simulation have the demand
and no evidence exists elsewhere. Confidence low-medium.

### N-24 `schedule` blocks

Rejected. Halide works over pure functions on integer domains and Exo over
affine loops, where every dependence is visible and a schedule provably cannot change the
result; outside such a sublanguage equivalence is undecidable, and
floating-point reassociation changes results even inside it. There is no
measured success in application code, and a schedule language is a second
language with its own legality checks and a polyhedral engine, all for one
domain while Q-23 is open. Kernel DSLs in the Halide mould stay libraries
over comptime staging, `std.simd` and C interop. Confidence medium-high.

## D. Learners, live work and end users

### N-25 Always-on evaluation gated by static authority

*What.* A few hundred milliseconds after typing stops, the language server
reruns every doc example and `test` block that reaches the function under
the cursor, in the interpreter, with fresh capability fakes and a fuel
limit, and shows the outcome inline: a result per example, the actual
values beside a failing `assert` (the sub-expression capture that already
exists), probes for the function under the cursor driven by the example
the user selects (per-line values, one column per loop iteration), and for
functions with contracts a background `check` with a small budget whose
shrunk counterexample appears on the `ensures` line. The gate is static:
code runs without being asked only when every capability it can reach is a
fake, so tests that take `System`, calls into trusted `unsafe` packages and
doc examples that need real capabilities run on request only. One form
needs no language server: `sisu doc` renders each doc example's expression
statements with their values (Unison and Go do this); a worksheet (N-34) is
the language-server form that re-runs a script after each edit.

```sisu
## ```
## assert average([2, 4, 9]) == 5        # live: 5, pass
## ```
def average(xs: list[int]) -> int:
    ensures min(xs) <= result <= max(xs)  # check: xs = [] panics (division by zero)
    var total = 0                         # probe for [2, 4, 9]: 0 │ 2 │ 6 │ 15
    for x in xs:
        total += x
    return total // len(xs)
```

*Precedents.* Unison's watch expressions, cached by hash and gated by its
abilities; Babylonian-style programming; Projection Boxes; Light Table and
Quokka without a gate; continuous testing (assignment completion 78 %
against 27 %); Code Hunt's counterexamples. New for an imperative language:
no one proves auto-run code cannot touch the world while still running
effectful functions against fakes.

*Buys and risks.* Speed and satisfaction, not measured learning gains (a
237-student course: faster tracing, same outcomes). "Loop datavoid" in
unfinished loops; a 100-case `check` gives false confidence unless
labelled; large packages need runs limited to the function under the
cursor. Spreadsheet evidence says liveness without validation makes users
more confident and no more correct, so every shown value offers "approve to
test" (N-32).

*Verdict.* Tooling: the first language-server feature after 0.1, not in the
experiment (principle 6); `sisu doc` with values: adopt (D-065). Confidence
medium-high for examples and counterexamples, medium for probes. *Settled
by* playground logs with inline results on for half the sessions (time to
first passing example, runs per fix) and a classroom comparison inside the
Q-41 trial.

### N-26 Run anyway: deferred errors, holes and the monitor as counterexample

*What.* `sisu run --defer` (default in the teaching profile and the live
view, never in `build` or `test`) runs a program that does not compile. An
ill-typed or unresolved expression lowers to a panic carrying its
diagnostic, so the program fails only if it reaches it (GHC's deferred type
errors; Luau's infallible checker, N-40, produces the typed tree). A hole
stops the run with its environment (N-27). Ownership errors are deferred
but not erased: the program runs under the dynamic ownership monitor the
experiment builds anyway, so a rejected borrow either produces the concrete
conflict with its values, or the run completes and the report says this
input reached no conflict and why the checker still rejects the program.

```sisu
var xs = [1, 2, 3]
let first = ref xs[0]       # deferred: `xs` is borrowed by `first` until line 4
xs.push(4)                  # monitor stops here: xs modified while first -> xs[0] = 1 is live,
print("{first}")            #   because first is still used here
```

*Precedents.* GHC's `-fdefer-type-errors`; Eclipse's Java compiler;
Hazel's evaluation around holes; Swift's run-time exclusivity checks where
static proof fails; Miri. Running a program the borrow checker rejected,
with the rejection turned into a run-time check, was found nowhere.

*Buys.* [learnability.md](../research/topics/learnability.md) records that
the explanation of borrow errors, not the rule, is what improved Rust's
learning curve, and a monitor counterexample is the most concrete
explanation there is. For the experiment it is a team instrument for bet 1:
rerun every rejected corpus program under the monitor across its tests;
rejections that never produce a conflict are candidate false positives,
which the risk table already requires to be logged.

*Risks.* "It ran" read as "it is fine"; the report must say one run proves
nothing about other inputs. Shown to the bet-1 study authors it would change
the friction they report, so the instrument stays with the team.

*Verdict.* Prototype in the experiment as a team instrument, by an amendment
to the pre-registration (about a week in Phase 3, Q-49); a teaching-profile
and live-view feature after the verdict. Confidence medium. *Settled by* the
share of rejected corpus programs whose deferred runs reach a conflict, and
in a classroom the time to fix borrow errors with and without monitor
counterexamples.

### N-27 The typed hole, its report and `sisu fill`

*What.* The prelude's `todo`, typed `todo[T](name: str = "") -> T`, is a
hole: the body type-checks around it, the interpreter runs until a test
reaches it and then stops with the hole's closure (the `Debug` rendering of
each binding in scope), and `sisu test` reports the test as blocked by the
hole. `sisu check --holes` reports, per hole, the expected type and the
definitions of the types it mentions, the bindings in scope with their
conventions and capabilities, the enclosing `requires` as facts and
`ensures` as obligations, the tests that reach it with the values seen, and
enumerated fits to depth two (GHC's valid hole fits), in human and JSON
form and as a language-server query. `sisu build --release` and `sisu
publish` reject holes; `sisu audit` lists them; `sisu doc` marks functions
containing one, as Lean flags `sorry`. Placeholders meant to ship are
`panic("...")`.

```sisu
## The middle element of `xs` in sorted order.
def median(xs: list[int]) -> int:
    requires len(xs) > 0
    ensures result in xs
    let s = sorted(xs)
    return todo("middle")

test "median of three":
    assert median([3, 1, 2]) == 2
```

```
warning[W0901]: hole "middle": int                                  stats.sisu:6:12
  in scope     xs: list[int]    s: list[int]
  facts        len(xs) > 0                                          requires, line 3
  obligation   result in xs                                         ensures, line 4
  reached by   test "median of three"    xs = [3, 1, 2]    s = [1, 2, 3]
  fits         s[0]   s[^1]   xs[0]   len(s)    (+9 of depth 2: sisu fill "middle")
```

`sisu fill` asks proposers (a built-in enumerator; any external process
speaking a JSON protocol; no model or vendor is built in, which keeps
D-029's one binary vendor-neutral) for
candidates, formats and deduplicates them by AST, type- and
ownership-checks them, then runs them in the interpreter against the tests
that reach the hole, the function's contracts as properties over
`arbitrary` inputs satisfying `requires`, and the property tests that call
it, all under capability fakes. A failing property's shrunk counterexample
goes back to the proposer; survivors are split by distinguishing inputs;
a person chooses by behaviour and reads the code. The judge is out of the
filler's reach: a fill changes only the hole's span, tests, contracts,
generators and snapshots are read-only during a fill, names must resolve in
the lockfile, a candidate comparing an input with a literal from a test is
flagged as special-casing, `sisu test --mutate` reports mutants of the fill
the tests do not kill, and `unfillable: <reason>` is a legal answer.
Accepted fills are plain source; an optional record names the proposer and
the judgements, and `--recheck` re-runs them rather than regenerating,
since model output is not reproducible even at temperature 0.

*Precedents.* GHC's typed holes and valid fits (on for twelve years), Idris
and Agda holes, Lean's `sorry` and `exact?`, Hazel's hole closures and its
measured ChatLSP context (type definitions had the largest effect, and the
gains were largest in the low-resource language), PropR (roughly two thirds to four
fifths of test-passing repairs overfitted in a manual sample), ImpossibleBench, S*'s distinguishing
inputs, FlashFill's disambiguation studies. rust-analyzer's term search was
little used, which argues for the report over enumeration. No shipped
toolchain filters fills by the tests and contracts reaching the hole.

*Spelling.* [holes 1](../research/unconventional/holes-synthesis-and-ai-era.md)
proposes `?name` in operand position (Idris) and would retire `todo`;
[AI-era A1](../research/ideation/ai-era-and-evolution.md) keeps `todo`
because `?` already means propagation, `T?`, `?.` and `??`, and P1 wants
one hole. This document keeps `todo`: the semantics are an intrinsic's, as
`assert`'s are, no lexical form is added, and `sisu fix` has nothing to
migrate. Q-46 records the alternative.

*Verdict.* The typed `todo`, its report and the release gate: adopt for v1
(D-061; the hover report fits Phase 8's language server). The fill engine:
tooling after 0.1. Confidence high / medium. *Settled by* a
fill-acceptance study once `sisu fill` exists, after 0.1.

### N-28 The visible notional machine

*What.* Two views over the interpreter. A stepper for the teaching profile
that steps forwards and backwards (N-04's replay) and shows each frame's
named values; because the teaching levels exclude `ref`, `view` and
`Shared`, every value is a tree, so there is no heap and no arrows, a
borrowed parameter is a label naming the caller's variable, and a move is a
value leaving one name for another. `sisu doc --worked` renders a doc
example as a static worked example with the example's comments as subgoal
labels. A permissions overlay shows on each line the read, write and own
permissions each binding gains and loses, from the checker's loan sets, and
marks the permission an erroneous operation needed and lacked.
*Precedents.* Python Tutor, Thonny, DrRacket's stepper, UUhistle, and
Aquascope, whose permission diagrams with a rewritten chapter raised
Ownership Inventory scores by 9 % (N=342, d=0.56), the best evidence of
any learning aid in the end-user file. *Verdict.* Tooling: the overlay after the bet-1 verdict
(giving Sisu's bet-1 authors an overlay while the Rust controls have none
confounds the comparison), the stepper with the playground after 0.1.
Confidence medium-high / medium.

### N-29 The teaching profile as a monotone ladder

*What.* `profile = "teaching-3"`: each level is a named set of feature
gates and a strict subset of the next and of full Sisu, so a valid program
stays valid, unchanged, as the learner climbs; using a construct from a
later level gives a level-up diagnostic naming it; teachers define course
ladders as data (a list of gates plus message overrides). Proposed levels:
script mode with `let`/`var`, `print`, `if`, `for` over `range` and lists;
`def` with signatures and `return`; `struct`, `enum`, `match`; `T?`,
`T ! E`, `?`, `else`; `test`, `assert`, contracts; `mut` and `own`
parameters and moves; traits; `task.scope` deterministic on the
interpreter for animations; full Sisu adds generics, `ref`, `view`,
`Shared`, `comptime` and `unsafe`. Hedy's levels, which change syntax
("code ran at the wrong level" was 18.4 % of its errors), are the
counter-model. *Precedents.* HtDP's student languages, Ada's restriction
identifiers, Java's on-ramp. *Verdict.* Adopt as the shape of Q-41 (D-073),
shipped when the classroom trial allows. Confidence medium. *Settled by* the
Q-41 trial with two arms and teacher uptake of custom ladders.

### N-30 Projections over canonical text

*What.* The file on disk is always canonical Sisu as `sisu fmt` writes it,
and editors may project it: a MakeCode-style block view for the teaching
profile whose blocks are the formatter's syntax tree, a Stride-style frame
editor (a logical line is a frame; a line ending in `:` opens a container),
and localised keywords with an inline dictionary while the file keeps
English, as Scratch and Excel do. The cheap parts, needed from the first
diagnostic and expensive to retrofit: message identifiers with parameters
in a catalog, never English concatenated in code, and the lossless syntax
tree as a library with node identities that survive edits. *Evidence.*
Blocks gave larger learning gains in high-school classrooms, and the gains
fade within a term; novices in localised Scratch learned faster (15,015
users, observational); Guo's 840 respondents listed English keywords among
their reading problems and 7 % asked not to fragment the ecosystem.
*Verdict.* Adopt the catalog and the tree library for v1 (D-065); block and
keyword projections are tooling after 1.0 or by the community, keywords
tried first in the playground. Confidence medium-high / low.

### N-31 Structural editing and a typed palette

*What.* Language-server commands that act on the tree: expand and shrink
selection by node, move a statement or block past its siblings with correct
re-indentation, wrap a selection in `if`, `for`, `with` or `task.scope`,
unwrap, turn an `if`/`elif` chain into `match`, extract a function that
takes the capability parameters the selection uses. A palette request
returns the vocabulary callable at the cursor grouped by module and trait,
each entry a snippet with holes, and shows what is *not* available and why
("Network: needs a `Net`; `save()` was not given one"). *Evidence.*
Paredit's 81 % among experienced Clojure users; 65.4 % of 217 text-to-block
switches by novices were made to add a command, two thirds of them a block
type not yet used in the program. *Verdict.* Tooling. Confidence
medium-high / medium.

### N-32 Concrete-first authoring and approve-to-test

*What.* "Make a function from this" turns statements written against
concrete values into a `def` with the original values as its first doc
example; "fill in the signature" writes the signature inferred from the
body and the examples that call it (in the teaching profile an unannotated
`def` is an error with this fix-it, so P2 holds); every value a live view
shows offers "this is right", which writes a test from the rendered value,
and "this is wrong", which writes a failing stub. *Risk.* A characterising
example turns a bug into the expected value, hence `== ?` in generated
examples, and ticks become rubber stamps when 5-23 % of end-user judgments
are wrong. *Verdict.* Tooling, with the teaching profile. Confidence medium
/ low.

### N-33 `View` as a `pure` trait, and `{expr=}`

*What.* `View` in `std.x.view` has one method, `pure def to_node(self) ->
ViewNode`, where `ViewNode` is a small document tree (text, record, table,
list, tree, image bytes, plot specification) that every tool showing a
value renders: REPL, hover, probes, notebook cells, doc examples, assertion
and snapshot diffs. `@derive(View)` renders a struct as a record and a list
of structs as a table; types without `View` fall back to `Debug`. Because
`to_node` is `pure`, any tool may call it at any time, which keeps N-25's gate
sound and turns IPython's convention into a checked guarantee. The rule
against a fourth rendering trait: `Display` for a program's users, `Debug`
for programmers as text, `View` for tools as structure. Python 3.8's
`{expr=}` makes `print("{total=}")` print `total=42`. *Verdict.* `View` in
std.x after 0.1 ([16](16-validation-plan.md) keeps `std.x` out of 0.1), a
candidate for core at 1.0; `{expr=}` adopt (D-071).
Confidence medium-high.

### N-34 Worksheets and the reactive notebook

*What.* A worksheet is a script-mode `.sisu` file the language server
re-runs top to bottom after each edit, showing each value as virtual text
that is never saved; it has no hidden state and no cells, real capabilities
run only on an explicit run, and it is where one computes before any
function exists. A reactive notebook (`# %%` cell markers in a plain file)
goes further: editing a cell reruns it and everything that depends on it,
and unlike Pluto and marimo the dependency graph includes in-place
mutation, which the type checker resolves statically from `mut` passing and
`mut self` calls; each binding has one defining cell, writers run after it
and readers after every writer, a cell may borrow another's bindings but
not move out of them. *Evidence.* Of 863,878 attempted executions of GitHub
notebooks, 24 % ran without error and 4 % reproduced their stored results; reactive
notebooks fix the ordering half and Sisu could fix the mutation half.
*Conflict.* [00](00-vision-and-principles.md) says Sisu is not for notebook
exploration "yet", on an AOT premise that D-052 weakened; worksheets need
only a sentence separating them from notebooks, and the notebook needs the
non-goal amended (Q-48), Q-23's numerics and typed tables (N-37) to have
anything to explore. *Verdict.* Worksheet: tooling after 0.1. Notebook:
prototype after 0.1, only with Q-23's outcome and Q-48. Confidence medium /
low-medium (high that the mechanism is sound and new, low on demand).

### N-35 Live work: epochs, time, the sketch kit and the tick model

*What.* Draft 2 of [18](18-next-steps.md) swapped a body at its next call
and let frames in flight finish on the old one, so one loop iteration could
mix an old `bar` with a new helper. Sonic Pi, Extempore, TidalCycles and Erlang converged on swapping
at a loop boundary; the epoch design below is this proposal's refinement
of that, with Flutter's `const` rule and Papyrus's persistence rule from
the game engines: a successful `--watch` check publishes every changed
body whose signature is unchanged as one *epoch*, and a changed signature
or layout restarts the program with a message; a task adopts the newest epoch at task start and at
`live.boundary()` (which a `Ticker.wait` calls), and resolves every call in
between in its adopted epoch; a save that fails to check publishes nothing;
a function used as a value holds its slot, not its body, in debug builds; a
changed `const` or comptime result re-swaps every body that uses it through
the query graph (Flutter's rule); nothing the runtime persists for the user
may contain frames, code addresses or epochs. `std.time` gains a deadline
`Ticker` with an explicit missed-tick policy and a `Timeline` with Sonic
Pi's virtual-time sleep, both over `Clock`, so the same code is real-time
in production and deterministic under the test clock. Observation-only
tracepoints (`--trace song.bar`) attach to a definition's slot and never
alter control flow. A first-party `sketch` package (canvas, input,
`update`/`draw` that hot-swap, a `Canvas` fake whose recorded commands make
pictures snapshot-testable, scrubbers on numeric literals in the editor)
is the playground's flagship and the best demonstration of three decided
features at once; a lockstep `tick` library gives the teaching profile
"many things at once" without races. *Evidence.* Godot's own post-mortem
says visual scripting failed for lack of packaged high-level components;
Processing, p5.js and Sonic Pi are established on-ramps (adoption, not
trials). *Verdict.* Epochs, slots and the const rule: adopt as a refinement
of D-053 (D-068). `Ticker` and `Timeline`: std. The sketch package and tick
model: first-party packages after 0.1; output-directed editing of general
programs rejected. Confidence high for the boundary semantics, medium for
the rest.

### N-36 Transcript tests

*What.* `sisu test` runs `tests/*.transcript` files: a `$` line runs the
package's `main` with those arguments and a fake `System` (in-memory `Fs`
from a fixture directory, frozen `Clock`, seeded `Rand`, no `Net`), `>`
lines are standard input, other lines are expected output; a mismatch fails
with a diff and `--update` re-blesses after showing it. With `sisu>`
prompts it tests REPL sessions in tutorials. *Precedents.* Inform's "Test me
with", doctest sessions, Go's `Example` output comments; the fakes remove
their fragility. *Verdict.* std.test (D-070), under the snapshot update
ceremony. Confidence medium.

### N-37 Typed tables, units of measure, and library-rendered types in diagnostics

*What.* `std.x.table`: `Table[Row]` columnar over an ordinary struct via
comptime reflection, boundary validation through `@derive(Decode)`, and a
row type inferred at compile time from a sample file (F#'s type-provider
trick done with Q-27 and `embed`). `std.x.units`: a comptime-constructed
quantity type whose `*` and `/` compute dimensions at compile time and
whose `+` requires equal dimensions, with constants `m`, `s`, `km`, `h`
through ordinary operator overloading. The language part is one attribute,
`@diagnostic_name(f)`, naming a comptime function the compiler calls when
it prints that type in a diagnostic, with `sisu explain --raw-types` to see
the truth; without it a units error shows an exponent vector, which is much
of what makes C++ and Rust units libraries hard to use. *Evidence.* Kotlin
DataFrame needed a compiler plugin to follow schema changes; a 2021 survey
found many units libraries and little use, blaming ergonomics and error
messages; Lean's delaborators and Rust's `#[diagnostic::on_unimplemented]`
are the hook's precedents. *Verdict.* Tables and units: std.x after 0.1 and
Q-23; `@diagnostic_name`: adopt if Q-27's type construction ships (D-065);
built-in units in the type system: reject. Confidence low-medium / medium.

### N-38 Consequence lines and a signature gloss

*What.* Every diagnostic with several fix-its gives each a one-line
consequence (`fix 1 add_user(mut team, roni.copy())  roni stays usable;
team gets its own copy`), which answers the misreading Marceau, Fisler and
Krishnamurthi documented in novices; the teaching profile offers a
deterministic gloss of dense signatures on hover, in the manner of cdecl.
Error dialogues that ask the learner which outcome they want, and a prose
"explain this program" view, are rejected: dialogues teach choosing without
understanding, and a prose generator rots and duplicates what models do.
*Verdict.* Adopt the two rules (D-065). Confidence medium-high.

### N-39 Deterministic cost metrics

*What.* `sisu test --cost` reports interpreter steps (the comptime fuel
counter, so it is reproducible), peak allocation and a PICO-8-style token
count per test, as JSON a course server can turn into histograms.
*Verdict.* Tooling, documented as a teaching measure. Confidence medium-low.

### N-40 A checker that produces types for broken code

*What.* The checker returns a typed tree for every input, flagging
ill-typed subterms instead of failing, so hover, completion and
go-to-definition work in broken code (Luau's first design goal), and
N-25's live views and N-26's deferred runs have something to run. It must
be designed into the first checker, since error recovery is hard to
retrofit, and it fits the rule that nothing downstream of the first error
in a chain is reported. *Verdict.* Adopt as checker architecture (D-065);
the language server already shares `sisu check`'s queries. Confidence high.

## E. The AI era, review and evolution

### N-41 The specification-weakening report and gate

*What.* `sisu review base..head` lists, beside the code diff, each change
that weakens a promise: a removed or edited `ensures`, `invariant` or
refinement; an added or edited `requires`; a deleted test, changed expected
value, new skip, narrowed generator or smaller case budget; an updated
snapshot; a dropped doc example, `pure` or `@must_consume`. Direction is
syntactic where possible and "unknown" for an edited clause until a
verifier proves implication; the normalised hash recognises moves and
renames. Under `[review] spec-weakening = "require-trailer"`, CI fails
unless the commit carries a human-written `Spec-Weakened-By:` trailer,
which agent harnesses can be set never to write. *Evidence.* On
ImpossibleBench, whose tests contradict their specifications, frontier
models cheat often and stronger ones more; METR saw o3 reward-hack in
30.4 % of RE-Bench runs. Tests and contracts are syntax in Sisu, so the
report is exact where other languages need heuristics. *Verdict.* Tooling
for v1, right after 0.1 (D-066). Confidence medium-high. *Settled by*
ImpossibleBench-style tasks in Sisu: cheats caught, false alarms per
hundred legitimate commits.

### N-42 Authority deltas and dependency admission

*What.* `sisu review --authority`, `sisu add` and `sisu update` report, per
public function and package, newly reachable capabilities (via parameters,
fields, closures, `dyn` values), new `unsafe` and `extern`, `[trust]`
changes, new dependencies and new panic sites; with no ambient authority
beyond standard streams, reach follows from types and the report is exact
up to `unsafe` in trusted packages. Admission adds a seven-day cooldown
applied before resolution (advisory fixes exempt), refuses packages younger
than that or without dependents unless `--accept-new-package` is given, and
in CI or agent runs never adds a package or widens `[trust]` against
policy. The pure-package label (N-17) is the same computation. *Evidence.*
Capslock computes capabilities for Go and deps.dev found fewer than 2 % of
version updates add one, so the signal gets read; models invent package
names (4.6-21.7 % across studies; 53 of the names five models all invent
were still registrable); eight of ten recent attacks had windows under a
week; the Nx compromise used `postinstall` to drive the victim's AI tools,
which Sisu's no-install-scripts rule and capability confinement each
block. *Verdict.* Tooling for v1; admission ships with the registry
(D-066). Confidence high.

### N-43 A review ledger keyed by capability-slice hashes

*What.* An audit record in `supply-chain/reviews.toml` holds a Merkle root
over the normalised-AST hashes of its scope, by default the *capability
slice* (the functions that can reach a capability, `unsafe` or `extern`);
an unchanged root means the next version inherits the review, otherwise
only functions whose hashes changed are listed. A pure parser has an empty
slice. *Precedents.* cargo-vet (audits per version and deltas), crev.
*Risk.* The hash becomes a specified artefact versioned with editions.
*Verdict.* Prototype with the registry. Confidence medium-low; slice size
and churn are unmeasured.

### N-44 Behavioural semver

*What.* `sisu publish` already diffs signatures and contracts; it would also
run the previous version's black-box tests and doc examples against the
candidate, and a failure needs a major version or a recorded, public
`--supersede "<test>" --reason "..."` shown in the changelog. *Evidence.*
NoRegrets classified at least 90 % of npm updates correctly from clients'
tests and found 26 breaking minor or patch releases; Dependabot's
compatibility score is missing for 83 % of updates; Debian gates migrations
on reverse-dependency tests. No package manager replays a package's own
previous tests at publish. *Risk.* Hyrum's law: a fix breaks a test that
encoded the bug, which is what the supersede record is for. *Verdict.*
Tooling with the registry. Confidence medium. *Settled by* a study runnable
now on crates.io: version N's integration tests against N+1 for patch
releases.

### N-45 `@fix_inline` migrations

*What.* A library deprecates an item by rewriting its body in terms of the
new API and marking it `@fix_inline`; `sisu fix --deps` replaces
dependents' calls with that body, so behaviour is preserved by
construction with no template language and no macros, and a major
release's API diff marks removals that had such a form as migratable.
*Precedents.* Go's `//go:fix inline`, Dart's data-driven fixes, Kotlin's
`ReplaceWith`. *Cost.* An ownership-aware inliner (borrows, `own` moves and
`defer` move into the caller's scope). *Verdict.* Tooling for v1 in Go's
form (D-067). Confidence high.

### N-46 Schema lock and required migrations for persisted types

*What.* A stored or transmitted type says so (`@persisted("config",
version=3)`); reflection gives a shape hash including field names, recorded
in a committed `schemas.lock`; a shape change needs a new version, and a new
version a compatible change (a field with a default) or a migration from
the frozen old shape, which `sisu schema freeze` writes as source; `sisu
test` derives the property that decoding old bytes equals migrating the old
value. It also answers hot replacement's state-migration question for
persisted state. *Precedents.* Buf's breaking-change categories, schema
registries, Django migrations, OTP's `code_change`; Java's unchecked
`serialVersionUID` is the warning. *Verdict.* std.x plus tooling after the
experiment, whose std has no encoder yet. Confidence medium.

### N-47 `sisu why`

*What.* One command, in human and JSON form, that explains an inferred fact:
`sisu why send cache.sisu:31` answers "Job is not Send: Job.cache:
Shared[Cache] -> Cache.hits: Cell[int], and Cell is not Sync; fixes: hits:
Atomic[int], or cache: Shared[Mutex[Cache]]". Version 1 covers error sets,
`Send`/`Sync`, `pure`, capability reach, impl selection and comptime fuel;
later, with N-04, why a value had the value it had. Tracing runs only in
explain mode by re-running the query, so compiles pay nothing. *Evidence.*
Argus (25 participants localised 2.2 times as many trait-inference faults,
3.3 times faster), the Whyline, rustc's "required because" notes, Scala's
`-explain`. No language has one query over all its inferred facts.
*Verdict.* Tooling for v1 (D-065); the `Send` and error-set explanations
fit Phase 8's language server. Confidence medium-high.

### N-48 Left-to-right binding types and a completion-engine API

*What.* A rule that a binding's type is fixed by the end of its declaring
statement, and a compiler-library `complete(prefix)` that says whether a
prefix can still become well typed, for constrained decoding in any model
runtime. Mündler et al. had to enforce exactly these conditions to
constrain TypeScript, and MoonBit ships a sampler under its checker. The
tour's `var results = list.with_capacity(len(handles))`, typed by a later
`push`, would need an annotation. *Verdict.* Measure in the experiment by
counting bindings typed by later statements (cheap instrumentation, Q-49);
adopt the rule if under about 3 % and authors report no friction (Q-50);
the API is post-experiment tooling. Confidence medium-low.

### N-49 Loud divergence from the training-data majority

*What.* A design rule: where Sisu spells something as Python does (or as
Rust does, for syntax shared with Rust), it means the same or fails to
compile; a run-time failure is acceptable only where no static check
exists; a silent difference is a design bug. The specification keeps a
false-friend table with a check per row: `xs[-1]` with a constant negative
index becomes a compile error with the fix-it `xs[^1]`; `7 / 2`, `if xs:`
and `a if c else b` already fail; `-7 % 3` is 2 as in Python and is flagged
in the Rust-to-Sisu corpus; and relying on `dict` insertion order is the
silent, flaky row that Q-42 would close. *Evidence.* Python 3's silent `/`
change is the cautionary tale; models default to Python and drift between
similar languages, and agents in unfamiliar languages prototype in Python
first. *Verdict.* Adopt as a rule with the table in the specification
(D-062), measured per row in the model arm. Confidence medium-high.

### N-50 A context-window digest and a model arm in the experiment

*What.* `sisu doc --digest` writes one plain-text file per package with
each `pub` signature, its inferred error set, capability parameters,
contracts and first doc example; the toolchain ships a pocket language
reference on a token budget (about 25k tokens; std about 50k). The
experiment gains a model arm: several frontier models solve the corpus
tasks with nothing, the reference, or reference plus digest, each with and
without compiler feedback, measuring compile and hidden-test pass rates,
false-friend bugs and tokens to a pass. It runs in the playground's sandbox
with no outsiders and no confound with the human results, and it carries
N-49's false-friend rows; N-27's fill measurements wait for the fill
engine after 0.1. *Risk.* Tuning the language to this year's
models; the arm measures and the design does not chase it. *Verdict.*
Prototype in the experiment as one amendment (Q-49); the digest is v1
tooling. Confidence medium. *Settled by* whether reference and digest bring
rates near those for the same tasks in Python; if not, the MultiPL-T corpus
is necessary.

### N-51 Verification, re-timed

*What.* Keep Q-16 as decided (verifier after 1.0; `ghost`, `decreases` and
loop `invariant` reserved) but move its feasibility study to right after
the verdict: translate the `pure` subset and its contracts to an auto-active
backend with model-proposed invariants, measures and lemmas (lemmas being
ordinary `pure` functions with `ensures`), run `pure` functions over
symbolic values in the SIR interpreter and hand path conditions to a solver
(Rosette's architecture; `verify` finds inputs that break `ensures` or trap,
`solve` finds inputs reaching a branch to seed `check`), and `sisu verify
--suggest` proposing `ensures` from doc prose for property tests to vet and
a person to accept; `ensures todo("post")` holes later. *Evidence.* Off-the-shelf
models solved 82 % of vericoding's Dafny specifications and pure Dafny
verification rose from 68 % to 96 % in a year, while the best model wrote sound and
complete specifications for 52.3 % of VERINA's Lean tasks and end-to-end
verified generation reached 1 of 161 CLEVER problems; proof burden, which kept SPARK and
Dafny niche, is the cost models cut fastest, and the checker stays the
trusted base. *Verdict.* Post-1.0 tooling as decided, study moved earlier.
Confidence medium / medium-low.

### N-52 A `Model` capability and `derive(Schema)`

*What.* `model.generate[Triage]("Classify this bug report: {report}")`
where `derive(Schema)` is a comptime function emitting a JSON Schema or
grammar for constrained output, refinements are re-checked on the parsed
value, and `Model` is a capability like `Net`, visible in signatures and
replaced by a scripted fake in tests. Model calls at build time stay
forbidden (non-deterministic builds, a breach of `comptime`'s no-I/O rule, a
supply-chain channel by construction). *Verdict.* std.x. Confidence medium.

## F. Configuration, syntax and library shapes

### N-53 Chained fallible conditions

*What.* A condition is a comma-separated sequence of `let` bindings and
boolean tests, left to right, every binding in scope in the branch, in
`if`, `elif` and `while`; nothing is rolled back (unlike Verse). A comma,
not `and`, because `and` is `bool`-only (D-025).

```sisu
if let Some(user) = users.get(id), let Some(team) = user.team, team.active:
    notify(team)
```

*Precedents.* Swift's condition lists; Rust 1.88's let chains. *Verdict.*
Adopt (D-071). Confidence high.

### N-54 The manifest stays data

*What.* No conditionals, templating or expressions in `sisu.toml`; target
conditions are structured keys (`winapi = { version = "0.3", when = { os =
"windows" } }`), not Cargo-style `cfg()` strings; anything computed lives
in `comptime` Sisu. *Evidence.* The Borg retrospective, the configuration
complexity clock, Declarative Gradle, CDKTF's archiving, and `sisu add`,
which can edit data reliably and expressions not at all. *Verdict.* Adopt
(D-063). Confidence high.

### N-55 Comptime budgets owned by the spending package

*What.* Fuel is counted in SIR steps; exhaustion raises a deny-by-default
diagnostic naming the expression and the package, with progress printed for
long evaluations; a package raises its own budget
(`@comptime_budget(steps=50_000_000)` on the item) so dependents never edit
their manifest for a dependency's internals. This replaces the
manifest-wide knob in [08](08-metaprogramming.md). *Evidence.* Rust 1.72
removed its const-evaluation cap because hitting it depended on library
internals; Dhall's and Starlark's designers concede totality does not bound
running time. *Verdict.* Adopt (D-063). Confidence medium-high.

### N-56 `sisu eval`: Sisu as its own configuration language

*What.* A module of `const` items and `pure` functions, evaluated by the
comptime interpreter and rendered as JSON, TOML or YAML through `Encode`,
with types, refinements and invariants as the schema: Pkl's model and the
Borg recommendation without a second language or a Starlark-style dialect,
since `comptime` is already hermetic and deterministic. *Verdict.* Tooling
(D-063). Confidence medium.

### N-57 A maximum lifetime for previews

*What.* A `@preview` feature is stabilised or removed within four releases
(two years); [13](13-evolution-and-governance.md) sets only a minimum.
*Evidence.* Nix flakes, experimental since November 2021. *Verdict.* Adopt
(D-072). Confidence medium.

### N-58 Conflict-checked configuration merging

*What.* CUE's rule as a library: equal values unify, conflicting scalars
are an error (`config.merge(base, overlay)?` returning `MergeConflict`),
overriding is explicit. *Verdict.* std.x. Confidence medium-low.

### N-59 Grammars as comptime values

*What.* `std.x.peg`: grammars are ordinary values built from combinators,
compiled at comptime into a parsing table with LPeg's predictable cost
model, and run the same at compile time and run time, so the checked
string DSLs [08](08-metaprogramming.md) already plans (and Tidal-style
notations) are uniform; a typo in a constant literal fails the build. They
produce values and cannot extend Sisu's syntax, so "no macros" holds.
*Verdict.* std.x. Confidence medium.

### N-60 Small library additions

`task.race` (Céu's `par/or`: the first branch to finish wins, the others get
`Cancelled` at their next blocking point and run their `Drop`), fair
interleaving, iterative deepening and bounded search over `Iter[T]` added
to the existing `std.search` (binary search and selection), and lockdep traces from the interpreter (N-12). *Verdict.*
std (D-075). Confidence medium-high for `task.race`, medium for the rest.

### N-61 Eventual references and promise pipelining

*What.* A `.later` view returning `Promise[T]` values that can be called
before they resolve, for remote objects and local actors, with E's
near/eventual split and `Drop` sending the release Workers RPC needs
`using` for. [systems](../research/ideation/systems-and-determinism.md)
rejects it as function colour under another name and prefers an explicit
batch API; [capabilities 2](../research/unconventional/capabilities-distribution-and-resources.md)
rates the model proven and its fit with colourless tasks untested.
*Verdict.* std.x later, after an RPC library exists to measure round trips
saved; no syntax. Confidence medium / low.

### N-62 Logical-time reactors

*What.* `std.x.reactor`: Lingua Franca's guarantee from parts Sisu has: a
`pure` reaction cannot block, read the clock or touch shared state, the
graph supplies the order within a tag, capabilities are the physical
inputs, `g.check()` rejects a cycle without a delay, and `g.replay(trace)`
replays in tests. No language mode: Esterel's causality problem and the
zero-delay limit do not transplant. *Verdict.* std.x; demand beyond
robotics and simulation is unproven. Confidence medium.

### N-63 Incremental computation

*What.* `std.x.incr`, Salsa-shaped: an explicit `Db` handle, inputs and
memoised `pure` queries with early cutoff; plain memoisation is a generic
function over `Hash` and `Eq`. Skip built this into a language and now ships as a framework with
TypeScript interfaces over a runtime written in Skiplang; Salsa,
Incremental and Bonsai succeed as libraries.
*Cost.* `Db`'s caches mutate, so `Db.read` and `Db.get` need a trusted
"observationally pure" declaration in a `[trust]` package, an audited hole
in `pure` to be recorded in [06](06-concurrency-and-capabilities.md);
attributes cannot wrap functions, so there is no `@memo` decorator.
*Verdict.* std.x. Confidence medium.

### N-64 Differentiation and probabilistic programming

Forward-mode AD is operator overloading on a `Dual` type over the numeric
traits; reverse mode is a tape library or Enzyme on the LLVM release
backend; a probabilistic model takes its inference handler as a trait
parameter. Swift for TensorFlow's differentiation is still feature-gated
five years after the project was archived, and every working probabilistic
system is a restricted DSL or a library. *Verdict.* Libraries; reject in
the language. Confidence low-medium (Sisu's numeric traits are untested,
Q-23).

### N-65 One definitional interpreter

Keep the SIR interpreter definitional so the runner, the schedule explorer
(N-03) and a future symbolic evaluator (N-51) share it, and it can anchor
Q-38's conformance suite, as K's one semantics drives three engines.
*Verdict.* Rule for the implementation ([18](18-next-steps.md)); the
conflict is with interpreter optimisation. Confidence medium-low.

### N-66 Read-only diagrams from code

`sisu doc` can draw what text hides and wires show: a module's
`task.scope` tree and channel wiring, the capability flow from `main`, the
transition graph of an `enum`-driven state machine. Generated and
read-only, they keep the visible dependencies Green and Petre grant
dataflow without its viscosity and merge problem. *Verdict.* Tooling.
Confidence low.

## Rejected

The consolidated table of dead ends, with the evidence, is in
[`unconventional-ideas.md`](../research/topics/unconventional-ideas.md).
Of the candidates proposed to the ideation agents or raised in the family
files, these are rejected as features of Sisu: `par:` blocks (N-18),
`schedule` blocks (N-24), a reactor or FRP mode (N-62), STM in the language,
an eventual-send operator (N-61), `@memo` (N-63), AD or probabilistic
keywords (N-64), logic variables, narrowing, junctions and goal-directed
evaluation, graph patterns in `match`, a separate specification language,
global rewrite rules, a solver in the language, totality by syntax, a
lattice type system, hash pins in imports, an executable manifest or a
configuration dialect, English-like or localised syntax, a source format
other than text, node-graph scripting and event sheets, an optional or
unsound typing mode, a scripting dialect, output-directed editing, error
dialogues and a prose explainer, Hedy-style levels, exact rationals in the
teaching profile, built-in units of measure, intent annotations,
provenance markers, a tokeniser-tuned vocabulary, a second canonical form,
a machine-oriented syntax, model calls at build time, a home-grown
semantic merge tool (contribute a Sisu definition to Mergiraf), WCET
budgets as effects, production hot upgrade, crash-isolated scope types,
`COME FROM` or pointcuts, 2D source, and killing a late thread.

## What changes in the proposal

**The owner's answers (October 2026).** Q-42 to Q-51 were answered as
leaned: insertion order for `dict` and `set`, the proposed deterministic
`pure` list, strict floating point, the invariant re-check for `Mutex`,
`todo("name")`, the library form of `par`, worksheets without notebooks,
all six experiment amendments, measuring binding types first, and
capabilities kept `Copy` with `std.cap`. Two things were added: a general
rule that the language core stays small and features go to the standard
library wherever reasonable, and a request for exact arithmetic as
separate `std.num` types with symbolic mathematics where achievable
(D-076, Q-53). Q-52 remains open.

- **Decisions D-058 to D-077** in the [decision log](15-decision-log.md),
  with status *leaning* except D-061, D-069, D-074, D-076 and D-077 and the
  amendments half of D-070, which the owner's answers to Q-42 to Q-51
  decided. The one reversal
  of a decided point (D-049, in D-074) is paired with Q-42; D-059, D-063,
  D-066, D-068 and D-072 amend decided text and say so.
- **Open questions Q-42 to Q-53** in [open-questions.md](../open-questions.md):
  `dict` and `set` order, the deterministic-`pure` primitive list, strict
  floats, `Mutex` unwinding, the hole's spelling, the `par` keyword,
  worksheets and the notebook non-goal, the experiment amendments,
  left-to-right binding types, whether capabilities stay `Copy`, whether
  resource attributes and capture kinds may enter function types (answered:
  no), and the scope and semantics of exact arithmetic (answered: an
  unboundedly exact `Decimal` whose division yields a `Ratio`).
- **Experiment amendments proposed** ([18](18-next-steps.md), Q-49): the
  io_uring cancellation test in Phase 1; the monitor as counterexample
  generator in Phase 3; the `sim` profile across Phases 6 and 8; the
  capability log and replay tokens in Phase 8; the binding-type count in
  Phase 5; the model arm in Phase 9. The owner accepted all six (Q-49);
  they cost about six person-weeks, the plan takes four more weeks rather
  than trimming Phase 8's polish, and [18](18-next-steps.md) now runs 52
  weeks with them in place.
- **Design documents** were first given a short "additions proposed in 19"
  paragraph each; Draft 3 (October 2026) folded those into the body text,
  marking leaning decisions where a document first relies on one. The
  documents touched: [02](02-syntax.md) (typed
  `todo`, `{expr=}`, chained conditions), [03](03-types-and-generics.md)
  (floating point), [04](04-memory-and-ownership.md) (guaranteed drop,
  regions, must-consume containers), [06](06-concurrency-and-capabilities.md)
  (deterministic `pure`, `std.par`, interceptors), [08](08-metaprogramming.md)
  (budgets), [09](09-modules-packages-and-build.md) (manifest as data,
  review and admission), [10](10-tooling.md) (catalog, `why`, `review`,
  `eval`, `fix --deps`, holes), [11](11-stdlib.md) (`dict` order, `std.par`,
  `SoaList`, time), [13](13-evolution-and-governance.md) (preview lifetime),
  [17](17-testing.md) (`exhaustive[T]`, transcripts, `@refines`, simulation,
  replay tokens) and [00](00-vision-and-principles.md) (worksheets).
