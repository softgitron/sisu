# What unconventional languages teach

## The question

The first research round compared the languages people write software in
and produced a design that is, by its own account, a recombination of
shipped ideas. This round asked two further questions. Which ideas live
only in block editors, spreadsheets, synchronous and dataflow languages,
logic and constraint systems, configuration languages, capability and
resource languages, live-coding and art languages, game scripting, and the
holes-and-synthesis tradition, and would Sisu be better with them? And is
there anything, given what Sisu has already decided, that no existing
language offers and Sisu could? The fourteen family files are in
[`../unconventional/`](../unconventional/) and the three ideation files in
[`../ideation/`](../ideation/); this document is the synthesis, and
[19-novel-features.md](../../proposal/19-novel-features.md) is the
assessment of each candidate with its verdict.

## The families and their central lesson

| Family | File | Central lesson for Sisu | Taken |
|---|---|---|---|
| Block, node and projectional editors | [visual-and-structure-editing.md](../unconventional/visual-and-structure-editing.md) | The measured benefits of blocks (discoverable vocabulary, no syntax errors, faster early learning) come from the palette, holes and a canonical printer, not from blocks; every non-text source format lost diff, merge and review | Text stays the only source; palette, structural edits and holes in the language server; a frame or block view only for teaching |
| Live programming, notebooks, trace-driven and moldable tools | [live-and-exploratory-environments.md](../unconventional/live-and-exploratory-environments.md) | rr, TTD and Elm spent their effort building the nondeterminism boundary that capabilities give by construction; bespoke live editors and hidden-state kernels failed | Deterministic simulation, capability-level record and replay, always-on evaluation of capability-free code, inspectors as a `pure` trait |
| Spreadsheets, formula languages, end-user and gradual languages | [spreadsheets-and-end-user.md](../unconventional/spreadsheets-and-end-user.md) | Liveness without validation makes users more confident and no more correct (3.9 % cell error rate); silent errors and helpful conversions are the opposite of Sisu's decisions; levels must only add | Worksheet files, approve-a-value-to-test, monotone teaching levels |
| Natural-language-like and literate programming | [natural-language-and-literate.md](../unconventional/natural-language-and-literate.md) | "Easy to read AppleScript, but quite hard to write it" is its designer's own verdict; what survives is examples that show their values and transcripts as tests | Doc examples rendered with values, transcript tests, grammars as comptime values; English-like syntax rejected |
| Synchronous reactive and dataflow languages | [synchronous-and-dataflow.md](../unconventional/synchronous-and-dataflow.md) | Determinism became a certified language guarantee by making time logical and communication explicit; the causality problem and clock calculi did not transplant | Deterministic simulation, `task.race`, a reactor library, a checked deterministic scope; no language mode |
| Structured parallelism, PGAS and data-parallel languages | [parallel-languages.md](../unconventional/parallel-languages.md) | Checked independence (occam) beats asserted independence (OpenMP) and parallel-by-default (Fortress); `task.scope` already is `PAR`, `finish` and `cobegin` | `par` iteration with serial-elision semantics, `par.join`, reducers with a fixed merge order; a non-blocking pool for `pure` bodies |
| Halide-style algorithm and schedule separation | [algorithm-schedule-separation.md](../unconventional/algorithm-schedule-separation.md) | Schedules pay only over a sublanguage whose dependences the compiler sees, and only where a few kernels dominate; layout polymorphism has broader evidence | `SoaList[T]`, `@refines` equivalence tests, checked no-op directives; no `schedule` block |
| Logic, constraint, functional-logic and rewriting | [logic-constraint-and-rewriting.md](../unconventional/logic-constraint-and-rewriting.md) | Model checking of the code itself (P, ShardStore) and small-scope search pay off; choice as an expression form, logic variables and in-language solvers do not | The schedule explorer as a model checker, `exhaustive[T]` inputs, chained fallible conditions |
| Total, hermetic and lattice-typed configuration | [config-and-total-languages.md](../unconventional/config-and-total-languages.md) | Totality by syntax does not bound running time (its designers concede it); executable manifests are retreated from; two decided documents contradict each other on `dict` order | Fuel budgets owned by the spending package, manifest as data, Sisu as its own configuration language |
| Object-capability, distributed and resource languages | [capabilities-distribution-and-resources.md](../unconventional/capabilities-distribution-and-resources.md) | Capability-safe subsets of other languages died of taming; the near/eventual split and revocation by caretaker are the parts that survived | Conditional must-consume, revocable and lent capabilities, pure-package labels; pipelining as a library |
| Self-adjusting, reversible, probabilistic, differentiable, quantum | [incremental-reversible-and-exotic-paradigms.md](../unconventional/incremental-reversible-and-exotic-paradigms.md) | Every paradigm that went into a language (Skip, Swift for TensorFlow) lost to the same idea as a library over a checked `pure`; reversible languages lost to record and replay | Record and replay, `std.x.incr`, capability interceptors instead of aspects |
| Esoteric, art and live-coding languages | [esoteric-and-live-coding.md](../unconventional/esoteric-and-live-coding.md) | Sonic Pi, Extempore, TidalCycles and Erlang converged on replacement at loop boundaries; time as a value the program advances; no 2D source has a use | Epoch-consistent hot replacement, drift-free `Ticker` and `Timeline`, notations as comptime-checked strings |
| Game scripting, visual scripting and narrative languages | [game-and-narrative-scripting.md](../unconventional/game-and-narrative-scripting.md) | Godot removed its node graphs at 0.5 % use and Epic is reported to be moving Unreal to Verse; infallible type checking and tick-boundary reload are what the largest novice populations kept | A tolerant checker, engine reload rules, a packaged first-program library, a lockstep tick model for teaching |
| Typed holes, synthesis, programming by example, LLM-era tooling | [holes-synthesis-and-ai-era.md](../unconventional/holes-synthesis-and-ai-era.md) | Holes are the interaction model of every dependently typed language; fills must be judged by something the filler cannot touch (most test-passing repairs in PropR's manual sample were overfitted) | A typed hole with a report, a fill engine with the judge out of reach, a `Model` capability |

## What the evidence says

1. **Sisu's decided design owns the three choke points these families
   spent their effort building.** Every effect goes through a capability
   value; every scheduling decision goes through one runtime whose
   interpreter is seedable and yields only at known points; every heap
   allocation shows in a type. rr reconstructs the first from system calls
   at under 2x, and instruction-level recording (WinDbg's TTD) costs about
   10-20x; Elm got it from purity; Temporal and Durable Functions ask
   developers to keep it by convention, with a linter that "will not catch
   all cases" and a run-time detector that "won't catch all violations"
   ([live](../unconventional/live-and-exploratory-environments.md),
   [systems](../ideation/systems-and-determinism.md)). FoundationDB and
   TigerBeetle hold their simulations by discipline; in Sisu the compiler
   can refuse a `sim` build that reaches an unmodelled `extern`
   ([systems 2](../ideation/systems-and-determinism.md)). The strongest
   candidates in this round are the ones that cash this in: deterministic
   simulation with typed fault injection, record and replay at the
   capability boundary, always-on evaluation of code that provably cannot
   touch the world, and the schedule explorer as a model checker. Each is a
   known technique made cheap, which is the kind of novelty P10 allows.

2. **Three files flagged the same decision from different angles.** D-049
   randomises `dict` iteration per process; [08](../../proposal/08-metaprogramming.md)
   promises that `comptime` evaluates "exactly what would run at run time";
   [06](../../proposal/06-concurrency-and-capabilities.md) calls a function
   with no capabilities "provably pure". A `pure` function that lists a
   `dict`'s keys returns different lists in different processes, so
   memoisation across processes, replay and the comptime promise all fail
   ([systems F1](../ideation/systems-and-determinism.md) states the full
   contradiction; [config 1](../unconventional/config-and-total-languages.md)
   sets D-049 against 08's reproducibility and proposes a comptime-only
   exception; [AI-era A11](../ideation/ai-era-and-evolution.md) names
   `dict` order as a silent false friend for Python-trained readers).
   Python separated the two concerns in 3.7: a keyed SipHash against
   flooding, insertion order for iteration. The systems file also found that
   floating point is specified only as "IEEE 754", so the comptime promise
   fails as soon as a backend fuses a multiply-add; that a scope's "first
   failure" depends on the schedule; and that nothing says what a `Mutex`
   does when unwinding drops its guard.
   These are gaps in the decided design, not features, and
   [19](../../proposal/19-novel-features.md) records them first.

3. **Text stays the only source of truth, and the measured benefits of
   blocks transfer through tooling.** Every non-text format (MPS models,
   LabVIEW VIs, Unison's database, Enso's metadata block) forfeited diff,
   merge, grep and review; Godot removed its visual scripting at 0.5 % use,
   Unity's and Unreal's graphs remain, and Epic is reported to be moving
   Unreal to Verse
   ([visual](../unconventional/visual-and-structure-editing.md),
   [game](../unconventional/game-and-narrative-scripting.md)). The block
   benefits that were measured in classrooms are recognition over recall
   (65.4 % of 217 text-to-block switches were made to add a command, two
   thirds of them a block type not yet used), freedom
   from syntax errors, and an early learning gain that fades within a term;
   the first two transfer to a typed palette and to holes, the third only to
   a teaching tool. A frame editor fits Sisu's syntax unusually well, since
   a logical line is a frame and a line ending in `:` opens a container. The
   v1 requirements are cheap: a message catalog for diagnostics and the
   formatter's lossless syntax tree as a library
   ([end-user 8](../ideation/end-user-and-live.md)).

4. **Liveness needs validation, and validation needs the authority
   boundary.** Spreadsheet users with live values and no correctness
   feedback became more confident, not more correct (Ko et al.), and the
   solid error figure is a 3.9 % cell error rate, close to inspection rates
   for industrial code ([spreadsheets](../unconventional/spreadsheets-and-end-user.md)).
   Continuous testing raised assignment completion from 27 % to 78 % (Saff
   and Ernst), and Projection Boxes in a 237-student course made tracing
   faster without changing learning outcomes
   ([end-user 1](../ideation/end-user-and-live.md)). The design consequence
   is a gate: the language server runs code without being asked only when
   every capability it can reach is a test fake, which Unison gets from
   purity and no imperative language has; and every shown value offers
   "approve to test", so liveness feeds the test suite rather than
   replacing it.

5. **Holes belong in the language; the judge must be out of the filler's
   reach.** Typed holes are how Idris, Agda, Lean and GHC users work (the
   interaction model of the dependently typed languages studied), and
   Hazel's measurements show the hole's static context (type definitions
   first, then headers, then error rounds) lifts a model furthest in a
   low-resource language, which is Sisu's position   ([holes](../unconventional/holes-synthesis-and-ai-era.md), ideas 7 and
   transferable 2). Fills
   judged by tests alone overfit (PropR: roughly two thirds to four fifths
   of passing repairs in a manual sample;
   strengthened SWE-bench tests rejected 19.8 % of "solved" patches), models
   edit tests when they can (ImpossibleBench), and a model judging its own
   output collapses where a sound verifier helps. So the fill engine edits
   only the hole's span, tests and contracts are read-only, contracts become
   properties, mutation testing checks the suite, and "unfillable" is a
   legal answer ([holes](../unconventional/holes-synthesis-and-ai-era.md), transferable 3-5,
   [AI-era A1, A2](../ideation/ai-era-and-evolution.md)). The two files
   disagree on spelling: `?name` (Idris) or the existing `todo("name")`;
   [19](../../proposal/19-novel-features.md) keeps `todo`.

6. **Second concurrency and evaluation models are libraries.** Lingua
   Franca's reactors, FRP signals (removed from Elm in 0.17 as unnecessary),
   Rx streams, STM (Duffy's retrospective: side effects are "the elephant in
   the room"), promise pipelining (function colour under another name),
   Skip's reactive memoisation (now a framework with TypeScript interfaces), AD in the
   language (Swift for TensorFlow, archived) and probabilistic keywords all
   point the same way: the guarantee each wanted comes from a checked `pure`
   plus capabilities, and the mechanism is a library over them
   ([synchronous](../unconventional/synchronous-and-dataflow.md),
   [incremental](../unconventional/incremental-reversible-and-exotic-paradigms.md),
   [capabilities](../unconventional/capabilities-distribution-and-resources.md),
   [systems](../ideation/systems-and-determinism.md)). The one thing taken
   into the language is a stronger `pure`: deterministic across runs,
   processes, thread counts and targets, which no language checks.

7. **Parallelism: check independence, keep the syntax.** occam's usage
   rules and Rust's closure rules are what Sisu's exclusivity and
   `Send`/`Sync` already give; OpenMP's shared-by-default and Fortran's `do
   concurrent` buy silent wrong answers; Fortress's parallel-by-default was
   cancelled. `task.scope` is already `PAR`, `finish` and `cobegin`, so a
   `par:` block adds nothing but a keyword with unclear `return` semantics
   ([systems 12](../ideation/systems-and-determinism.md)). What is missing is
   Chapel's `forall`: potential parallelism with serial-elision semantics,
   reducers with a fixed merge order (Cilk hyperobjects), and a pool for
   bodies that cannot block, which keeps fine-grained parallelism out of bet
   3's stack budget ([parallel 1-4](../unconventional/parallel-languages.md)).
   Whether that is spelled `for x in par xs` or `xs.par_iter()` is an open
   question; the library form ships first.

8. **Schedules do not generalise; layouts do.** Halide works over pure
   functions on integer domains and Exo over affine loops, where the compiler sees every
   dependence and a schedule provably cannot change the result; outside such
   a sublanguage equivalence is undecidable and floating-point reassociation
   changes answers anyway. There is no measured success in application code.
   Struct-of-arrays layout (Odin, Zig, ISPC) has broader evidence and needs
   only comptime type construction (Q-27); a reference algorithm with
   generated equivalence tests reuses the existing `check`
   ([schedule](../unconventional/algorithm-schedule-separation.md)).

9. **Totality by syntax fails; budgets must be owned by the spender.**
   Dhall's and Starlark's designers concede totality does not bound running
   time; Buck2 added recursion back; Rust 1.72 removed its const-evaluation
   cap because hitting it depended on library internals. The honest bound is
   fuel counted in interpreter steps, raised by the package whose code
   spends it, never by a knob in the root manifest. Executable manifests
   (Gradle, `build.zig`, `Package.swift`) are in retreat, CDKTF was archived,
   the Borg retrospective recommends a data-only format changed
   programmatically from a real language, and Dagger's lesson is that people
   configure in the language they already know; so `sisu.toml` stays data
   with structured target conditions, and `pure` Sisu modules evaluated by
   the comptime interpreter are the configuration language ([config](../unconventional/config-and-total-languages.md)).

10. **Model checking the code itself has the strongest systems evidence of
    anything in this round.** P at AWS, S3's ShardStore (16 issues kept out
    of production), FoundationDB ("unlikely that we would have been able to
    build FoundationDB without this technology"), TigerBeetle's VOPR, and
    PCT's probabilistic guarantee all rest on one property: every source of
    nondeterminism is a choice the harness controls. Sisu's capability fakes
    and seeded scheduler are those choices, so the explorer that Phase 6
    already builds becomes a model checker by adding explicit `choose`,
    `always` invariants, a bounded exhaustive mode and typed fault injection
    drawn from each method's declared error set
    ([logic 1](../unconventional/logic-constraint-and-rewriting.md),
    [systems 2](../ideation/systems-and-determinism.md),
    [synchronous 1](../unconventional/synchronous-and-dataflow.md)). Alloy's
    small-scope hypothesis gives the cheap complement: exhaustive inputs up
    to a small size, derived like `arbitrary`.

11. **Resource guarantees are unusually cheap in Sisu.** Clang 20 checks
    `nonblocking` and `nonallocating` on function types, Swift's
    `@_noAllocation` is experimental, Ada's restrictions are program-wide,
    SPARK's Silver level proves absence of run-time errors, and Wuffs ships
    in Chrome with every bounds check discharged at compile time. In Sisu
    allocation is visible in types and every blocking point is a capability
    call or a synchronisation primitive, so `@no_alloc` and `@no_block` are
    one SIR pass each, and `@no_panic` is interval arithmetic over
    refinement bounds and `requires` clauses until the verifier exists
    ([systems 5, 6](../ideation/systems-and-determinism.md),
    [capabilities 5](../unconventional/capabilities-distribution-and-resources.md)).
    One rule the design nearly has should be stated: views, scopes and
    suspended frames are never leaked, which is what lets completion-based
    I/O and DMA borrow the caller's buffer, something Rust cannot offer
    safely because `mem::forget` is safe
    ([systems 7](../ideation/systems-and-determinism.md)).

12. **Live coding and games agree on where hot replacement must happen.**
    Sonic Pi, Extempore, TidalCycles and Erlang each swap at a loop
    boundary, so one iteration runs one code version; Extempore's callbacks
    point at a function's slot rather than its body; Flutter re-swaps
    dependents of a changed constant; Papyrus's lesson is that saved state
    must hold no frames or code identity. The next-call rule of the experiment plan lets one
    iteration mix versions; the refinement proposed from these is an epoch
    word per task, with a failed check publishing nothing ([esoteric 1](../unconventional/esoteric-and-live-coding.md),
    [game 2](../unconventional/game-and-narrative-scripting.md)). Time should
    be a value the program advances over the `Clock` capability (drift-free
    tickers, Sonic Pi's virtual timeline), which makes the same code
    real-time in production and deterministic in tests.

13. **In the AI era, structured intent carries the measured gains and
    everything else is folklore.** Contracts, examples and properties are
    checkable and show results (postconditions from prose caught 64 Defects4J
    bugs; approving model-proposed tests raised pass@1 by 46 points);
    tokeniser-tuned vocabularies, canonical forms beyond the formatter,
    intent annotations and machine syntaxes have no measured accuracy gain
    behind them (SimPy cut tokens by 10-13 % at readability's cost)
    ([holes](../unconventional/holes-synthesis-and-ai-era.md), ideas 9 and
    12; [AI-era R1-R7](../ideation/ai-era-and-evolution.md)). Where models fail
    is predictable and Sisu's design answers it: they invent package names
    (4.6-21.7 %, so dependency admission needs a cooldown and an authority
    delta, which Capslock shows is a rare and therefore readable signal);
    they weaken tests and contracts when the scorer is visible (so a review
    tool classifies weakened promises); they default to Python's meaning for
    Python's spelling (so a design rule makes every false friend fail to
    compile). A new language can measure its own model penalty in the
    experiment, with a pocket reference and an API digest, instead of
    waiting for a training cycle.

14. **Natural-language surfaces and literate sources are settled dead
    ends.** AppleScript's designer calls the experiment unsuccessful and its
    dialects were dropped in 1998; Inform 7 works because its domain is prose
    and the author declares the vocabulary; SQL's pipe-syntax paper calls the
    English keywords "an anachronism" and clause order the real defect;
    notebooks as source reproduce 4 % of the time
    ([natural](../unconventional/natural-language-and-literate.md),
    [live](../unconventional/live-and-exploratory-environments.md)). What
    survives is small: doc examples rendered with their values and
    transcript tests of whole programs against a fake `System`.

## Recommendation

The proposal takes the following from this round; the per-candidate
assessment, costs and what would settle each are in
[19-novel-features.md](../../proposal/19-novel-features.md), the new
decisions are D-058 to D-075 in the [decision log](../../proposal/15-decision-log.md),
and the questions that need the owner are Q-42 to Q-52 in
[open-questions.md](../../open-questions.md).

1. **Close the determinism gaps first** (finding 2): insertion-ordered
   `dict` with a per-process SipHash key (Q-42, reversing half of D-049),
   strict floating point with explicit-only fused multiply-add (Q-44),
   lowest-index failure in `pure` scopes, and a specified `Mutex` unwinding
   rule (Q-45). Then make `pure` bit-deterministic across runs, processes,
   thread counts and targets as a checked guarantee (Q-43), which no
   language offers and which the simulation, replay and memoisation
   candidates all rest on.
2. **State the guaranteed-drop rule for views, scopes and suspended frames**
   and build the I/O API on it (D-058); add `@no_alloc` and `@no_block` in
   v1 and `@stack` with the native backend (D-059); regions as
   allocator-typed views (D-060); must-consume propagation through
   containers, settling Q-37's open half (D-064).
3. **Put the capability dividend into the experiment** as named amendments
   to the pre-registration (Q-49): a `sim` profile with typed fault
   injection and PCT scheduling, the interpreter's capability log with
   replay tokens for every failure, the dynamic ownership monitor as a
   counterexample generator for rejected programs (a team instrument for
   bet 1), one io_uring cancellation test in the Phase 1 harness, the Phase
   5 count of bindings typed by later statements, and a model arm with a
   pocket reference and an API digest.
4. **Holes and their report in the language** (D-061): `todo("name")` is a
   typed hole whose expected type, in-scope bindings, contracts and reaching
   tests `sisu check` reports; release builds and `publish` reject holes;
   the fill engine with the judge out of the filler's reach is tooling after
   0.1. Q-46 records the `?name` alternative.
5. **Tooling commitments that cost little now and much to retrofit**
   (D-065 to D-067): diagnostics through a message catalog with consequence
   lines on every multi-fix diagnostic, the lossless syntax tree as a
   library, `sisu why` over every inferred fact, `sisu review` with
   specification-weakening and authority deltas, dependency admission with a
   cooldown, `@fix_inline` migrations, `sisu eval` for configuration, the
   manifest kept as data with structured target conditions and comptime
   budgets owned by the spending package (D-063), a two-year maximum for
   preview features (D-072), and the loud-divergence rule with a
   false-friend table in the specification (D-062).
6. **Standard-library shapes decided now because they are slow to add
   later**: `std.par` with `par_iter`, `par.join` and fixed-order reducers
   on a non-blocking pool and `SoaList[T]` (D-069); `exhaustive[T]` generators,
   transcript tests and `@refines` in `std.test` (D-070); `View` in `std.x`,
   `{expr=}` interpolation and chained fallible conditions (D-071);
   capability interceptors, `task.race` and drift-free time (D-075).
7. **Hot replacement refined to epochs at boundaries** (D-068) and the
   teaching profile shaped as a monotone ladder with teacher-defined ladders
   as data (D-073), with the worksheet mode and the always-on evaluation
   gate as the first language-server features after 0.1.
8. **Rejected**: `par:` blocks, `schedule` blocks, a reactor or FRP mode,
   STM in the language, promise-pipelining syntax, `@memo`, logic variables,
   junctions, English-like syntax, localised source, a non-text source
   format, node-graph scripting, event sheets, an optional typing mode, a
   second machine-oriented syntax, model calls at build time, intent
   annotations, totality by syntax, lattice types, an executable manifest,
   and a home-grown merge tool.

## Dead ends, consolidated

| Idea | Where it was tried | Why not | File |
|---|---|---|---|
| A source format other than formatted text | MPS, LabVIEW, Lamdu, Enso, Unison | Loses diff, merge, grep, review; the AI corpus needs text | visual |
| Visual dataflow as general notation | LabVIEW, Max, Blueprints, Godot VisualScript | 8:1 viscosity, Deutsch limit, 0.5 % use and removal | visual, game |
| Editors that forbid invalid text | 1980s syntax-directed editors, mbeddr | Rejected by programmers; Hazel itself went gradual | visual |
| Output-directed editing | Sketch-n-Sketch | Needs a literal-heavy domain; still research | visual, end-user |
| Bespoke live editor as the product | Light Table, Eve, Dark | None survived as a product, and Lamdu is a technology preview after fifteen years; ship through LSP and DAP | live |
| Imperative kernel with hidden state | Jupyter | 36 % out-of-order, 4 % reproducible | live |
| Deployless editing of production | Dark | Bypasses review and provenance | live |
| Grid and positional references | Spreadsheets | Pointing errors; Excel's fix was names | spreadsheets |
| Silent errors, blanks, helpful conversions | Excel, Power Fx | Gene names became dates; opposite of `T?` and `T ! E` | spreadsheets |
| Levels that change syntax | Hedy | "Wrong level" was 18.4 % of errors; a second dialect | spreadsheets, end-user |
| English-like syntax; English as the program | AppleScript, Inform 7, COBOL | "Easy to read ... but quite hard to write", its designer's verdict | natural |
| Localised source dialects | AppleScript, Hedy | Dropped in 1998; fragments help-seeking | natural, end-user |
| Rulebooks with specificity dispatch | Inform 7 | Suits a world model; Sisu has visible `match` order | natural |
| Extensible grammars, unevaluated blocks | Rebol, Raku | Macros by another name | natural |
| Literate files or notebooks as source | WEB, Jupyter | Reproduction and tooling | natural, live |
| Constructive causality in the core | Esterel | The designers' own causality hunt | synchronous |
| Clock calculi in a general type system | Lustre, Lucid Synchrone | Tied to a qualified toolchain; the transplant stayed in research | synchronous |
| First-class FRP signals | Elm 0.x | Removed in 0.17 as unnecessary | synchronous |
| Rx streams as the concurrency model | ReactiveX | `merge` is nondeterministic; JEP 444's critique | synchronous |
| Parallel-by-default loops | Fortress, Sisal, Id | Cancelled; the lesson was about associative data | parallel |
| Unchecked parallel annotations | OpenMP, `do concurrent` | Silent wrong answers | parallel |
| A `PAR` or `cobegin` keyword | occam, X10, Chapel | `task.scope` already is one | parallel, systems |
| Flattening, PGAS, places in the core | NESL, DPH, X10, Chapel | Experimental or niche; work stealing handles nesting on CPUs | parallel |
| A `schedule` block; autoscheduling; a tensor sublanguage | Halide, TVM, Tiramisu | No evidence outside dense kernels; vendor forks and hardware lag | schedule |
| Goal-directed evaluation by default | Icon, Converge | "Dangerous for the majority" of procedures | logic |
| Transactional rollback in conditions | Verse | Needs effects in every signature and an undo log | logic |
| Logic variables, narrowing, modes | Oz, Curry, Mercury | Subtle sharing semantics; Oz's own verdict | logic |
| Junctions | Raku | A type outside the hierarchy; no imitators | logic |
| A separate specification language | TLA+, Alloy, Quint | AWS moved towards models in the programming language | logic |
| Global rewrite rules; a solver in the language | Wolfram, Maude, Oz | Against local reasoning; solvers improve faster than languages | logic, natural |
| Totality by syntax | Dhall, Starlark | Does not bound running time; its designers say so | config |
| A lattice type system | CUE | Still 0.x; its best-known adopter left | config |
| Hash pins in imports; laziness; inheritance variants | Dhall, Nix, Jsonnet | Duplicate the lockfile; errors surface late; untraceable layers | config |
| An executable manifest; a config dialect | Gradle, `build.zig`, Starlark | Conflicts with D-032; `comptime` is already hermetic | config |
| Capability-safe subsets of other languages | Joe-E, Caja | Taming tracks every host release; Caja died with holes | capabilities |
| Network transparency | Oz, Erlang defaults | Partial failure cannot be hidden | capabilities |
| An eventual-send operator | E, TC39 proposal | Still at Stage 1 in October 2026; libraries ship method syntax | capabilities |
| Reactive memoisation in the language | Skip | Now a framework with TypeScript interfaces over a Skiplang runtime | incremental |
| Transparent memoisation of every `pure` function | Research | Overhead and memory against the priorities | incremental |
| Reversible languages; quantum types | Janus, Q# | No hardware, no adoption; outside the domain | incremental |
| AD or probabilistic keywords in the language | Swift for TensorFlow, Church | Archived; every working system is a DSL or library | incremental |
| Pointcuts and advice | AspectJ | Obliviousness and fragile pointcuts; capability interceptors instead | incremental, esoteric |
| 2D, image or self-modifying source | Befunge, Piet, Orca | Defeats compilation, tooling and replacement | esoteric |
| Killing a late thread | Sonic Pi | Report lateness instead | esoteric |
| Node-graph scripting; event sheets as syntax | Blueprints, Construct | Removed where measured; `for` plus a tick library | game |
| An optional or unsound typing mode; a scripting dialect | Luau, UnrealScript, Boo | No untyped legacy; the two-language tax | game |
| Evaluating past a hole in effectful code | Hazel | Performs I/O with a value that does not exist | holes |
| General programming by example; deductive synthesis in the compiler | FlashFill outside its DSL, Leon | Ambiguity; 1 of 161 CLEVER problems | holes |
| A model as its own judge; prompt programs as syntax; LLM-tuned syntax | Various | Self-critique collapses; not reproducible; nothing measured | holes, AI-era |
| Intent annotations; provenance markers; a machine syntax | Spec Kit, Agent Trace, SimPy | Unchecked; stale; two spellings of every program | AI-era |
| Production hot upgrade with state migration | Erlang, Kitsune | D-053 limits replacement to debug builds on purpose | systems |
| Crash-isolated scopes by type | Rust `UnwindSafe` | "A speed bump" by its own documentation | systems |
| WCET budgets as effects | aiT | Needs a hardware model | systems |

## Sources

The evidence is in the family files in [`../unconventional/`](../unconventional/)
and the ideation files in [`../ideation/`](../ideation/); each cites its
primary sources with years and URLs. Several agents report that the shared
web-search budget ran out partway through their work and that sources not
marked as checked were cited from prior knowledge; a decision that rests on
one of those should re-check it first.
