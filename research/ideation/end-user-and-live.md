# End users, learners and live work: candidate features

Ideation for the owner's request for features that exist in no current language, through one lens: people who are not professional programmers (students in a first course, teachers, scientists and analysts, designers, children) and the experience of seeing a program run while it is being written. Nothing here is decided. Each candidate gives a description and a sketch, the closest precedents, what it buys, what it costs, its risks and interactions with the decided design, a verdict with confidence, and the evidence that would settle it. P10 ("steal, do not invent") still applies, so a candidate without a precedent is at most "prototype", never "adopt in v1".

**Finding.** In this lens almost every good idea already exists, usually in a research system or a niche product (Hazel, Babylonian/S, Unison, Pluto, Aquascope, the Whyline, MakeCode, Sonic Pi). What those systems lacked and Sisu has is a static authority model (capabilities), values without aliasing (ownership with second-class references) and an interpreter that is the first implementation. Two candidates exist nowhere we looked, and both follow from those properties: ownership errors that run under the dynamic monitor instead of stopping the program (2), and a reactive notebook whose dependency graph includes in-place mutation (5). Two more are enforced today only in pure functional systems and would be new for an imperative language: evaluation that is always on because it provably cannot touch the world (1; Unison has it), and complete record and replay at the authority boundary (4; Elm enforces it, and Durable Functions asks developers to keep orchestrator code deterministic by convention). The rest are well-precedented ideas that Sisu can do unusually well, or should not do.

## Summary

| # | Candidate | New? | Verdict | Confidence |
|---|---|---|---|---|
| 1 | Always-on examples, probes and counterexamples | for imperative languages (Unison has the gate) | tooling, first language-server feature after 0.1 | medium-high |
| 2 | Run anyway: deferred errors, holes, the monitor as counterexample | yes, for ownership errors | prototype in the experiment as a team instrument; teaching tooling after | medium |
| 3 | The visible notional machine: value-tree stepper, permissions overlay | no | tooling, after the bet-1 verdict | medium-high |
| 4 | Replayable runs: the capability log, REPL undo, why-queries | for imperative languages (Elm enforces it; Durable Functions by convention) | log: prototype in the experiment; undo and why: tooling | medium-high / medium-low |
| 5 | Reactive notebook with mutation-exact dependencies | yes | tooling prototype after 0.1, needs a non-goal amended | low-medium |
| 6 | Typed tables (`std.x.table`) | no | standard library (std.x), after Q-23 | low-medium |
| 7 | Units of measure, and library-rendered types in diagnostics | no | units: std.x; rendering hook: adopt in v1; built-in units: reject | medium |
| 8 | Projections over canonical text: blocks and localised keywords | no | message catalog and lossless syntax tree: adopt in v1; projections: tooling later | medium-high / low |
| 9 | Monotone language levels: the teaching profile as a ladder | no | adopt in v1 as the shape of Q-41 | medium |
| 10 | Concrete-first authoring | no | tooling (language-server actions) | medium |
| 11 | A creative-coding kit on hot replacement | no | std.x and tooling after 0.1; output-directed editing: reject | medium |
| 12 | Moldable views: a `View` trait | no | standard library (std.x first); `{expr=}`: adopt in v1 | medium-high |
| 13 | Conversational explanations: error dialogues, "explain this program" | no | reject; keep consequence lines and a signature gloss | medium-high |

## 1. Always-on examples, probes and counterexamples

A few hundred milliseconds after typing stops, the language server reruns every `##` doc example and `test` block that reaches the function under the cursor, in the interpreter, each with fresh capability fakes and a fuel limit, and shows the outcome inline: a result per example, the actual values beside a failing `assert` (the sub-expression capture that already exists), and probes for the function under the cursor driven by the example the user selects (per-line values, one column per loop iteration, as Projection Boxes show them). Functions with contracts also get a background `check` with a small budget (100 cases, 200 ms) and the shrunk counterexample appears on the `ensures` line; in the sketch, once the learner adds `requires not xs.is_empty()`, the next counterexample is an overflow. The gate is static: the server runs code automatically only when every capability it can reach is a fake, so `test "x"(sys: System)`, calls into trusted `unsafe` packages, and doc examples that need real capabilities run on request only. No syntax is added; the inputs are the doc examples and tests that already exist.

```sisu
## ```
## assert average([2, 4, 9]) == 5        # live: 5, pass
## ```
def average(xs: list[int]) -> int:
    ensures min(xs) <= result <= max(xs)  # check: xs = [] panics (division by zero)
    var total = 0                         # probe for [2, 4, 9]: 0 │ 2 │ 6 │ 15
    for x in xs:
        total += x                        # next: [9223372036854775807, 1] overflows
    return total // len(xs)
```

- **Precedents.** Pyret's `where:` blocks attach examples to a function [7]; Babylonian-style programming (Squeak/Smalltalk and Lively4) attaches examples to methods and shows probes [1]; Projection Boxes [2]; Unison evaluates `>` watch expressions on every save, caches them by hash, and its types forbid I/O in them and require every other ability to be handled [5]; the Haskell Language Server's eval plugin runs `-- >>>` comments on request, `IO` included [6]; Go's runnable `Example` functions with `// Output:` comments; Light Table and Quokka.js evaluate everything as you type, with no gate; continuous testing [4]; Code Hunt shows learners Pex-generated counterexamples [8]. New only for an imperative language: Unison gets its gate from purity and handlers; we found no imperative language that proves auto-run code cannot touch the world while still running effectful functions, here against the fakes tests already receive.
- **Buys.** Saff and Ernst's students with continuous testing finished an assignment 78% of the time against 27% without (groups of 18 and 11) [4]; in a CS1 course (N=237), Projection Boxes left learning outcomes unchanged but made some code-tracing tasks faster, and students preferred them [3]; the CHI 2020 study found the always-on display not distracting and preferences widely varying [2]. The evidence is speed and satisfaction, not learning gains. For teachers the same gate allows grading in-process: student functions run against a reference (`check(gen, x -> student(x) == reference(x))`, Code Hunt's model) with fakes, fuel and a memory limit and no container, because an untrusted student package holds no capability the grader did not hand it.
- **Costs.** No language change. An interpreter instance in the language server; scheduling and cancellation of runs; result caching keyed by body hashes (Unison's trick); a run-time fuel limit beside comptime's; probe rendering. Every std fake must be deterministic, which the testing design already requires.
- **Risks and interactions.** Work on large packages (limit runs to the function under the cursor and the tests that reach it); a 100-case `check` gives false confidence unless labelled as such; Lerner's "loop-datavoid" problem in unfinished loops (his follow-up seeds a loop with a hypothetical value) [2]. Contracts and capability fakes are what make this cheap; explicit `return` gives probes an obvious place to show results; one statement per line gives every probe an unambiguous home; hot replacement is unaffected (a separate interpreter).
- **Verdict.** Tooling: the first language-server feature after 0.1-interp, not in the experiment (principle 6). Confidence medium-high for examples and counterexamples, medium for probes.
- **Settled by.** Playground logs with inline results on for half the sessions (time to first passing example, runs per fix); a classroom comparison inside the Q-41 trial.

## 2. Run anyway: deferred errors, holes and the monitor as counterexample

`sisu run --defer` (the default in the teaching profile and the live view, never in `build` or `test`) runs a program that does not compile. An ill-typed or unresolved expression lowers to a `Panic` carrying its diagnostic, so the program fails only if it reaches it (GHC's deferred type errors). `todo()` becomes a typed hole: the static diagnostic lists the expected type and the bindings in scope that fit it (GHC's valid hole fits), and reaching it stops the run showing the hole's environment (Hazel's hole closures). Ownership errors are deferred but not erased: the program runs under the dynamic ownership monitor the experiment builds anyway, so a rejected borrow either produces the concrete conflict with its values, or the run completes and the report says that this input reached no conflict and why the checker still rejects the program.

```sisu
var xs = [1, 2, 3]
let first = ref xs[0]       # deferred: `xs` is borrowed by `first` until line 4
xs.push(4)                  # monitor stops here: xs modified while first → xs[0] = 1 is live,
print("{first}")            #   because first is still used here
let n: int = todo()         # hole: expects int; in scope and fitting: len(xs)
```

- **Precedents.** GHC's `-fdefer-type-errors` since 7.6 [9]; Eclipse's Java compiler runs classes with errors, which throw "Unresolved compilation problem" when reached; Hazel evaluates around holes and localises and recovers from type errors [10]; Swift checks exclusive access at run time where it cannot prove it statically [11]; Miri checks Rust programs the compiler accepted. Not found anywhere: running a program the borrow checker *rejected*, with the rejection turned into a run-time check. Rust's 2018 migration mode downgraded some borrow errors to warnings with no check at all.
- **Buys.** [learnability.md](../topics/learnability.md) records that the explanation of borrow errors, not the rule, is what improved Rust's learning curve; a monitor counterexample is the most concrete explanation there is (this `push`, these values, that later use). Unfinished programs still run, which is Hazel's thesis. For the experiment, a team-side instrument for bet 1: rerun every rejected corpus program under the monitor across its tests; rejections that never produce a conflict are candidate false positives, which the risk table already requires to be logged as data.
- **Costs.** Lowering error nodes to `Panic` (the parser and checker are already error-tolerant and produce SIR for diagnostics); the monitor exists; message text. No language change.
- **Risks and interactions.** "It ran" read as "it is fine": one run proves nothing about other inputs, and the report must say so. Deferral is safe only because the interpreter checks every access, so it never reaches a native build. A missing `return` (D-056) defers to a panic at the end of the function. Shown to the bet-1 study authors it would change the friction they report, so the instrument stays with the team.
- **Verdict.** Prototype in the experiment as a team instrument for bet-1 false positives, by an amendment to the pre-registration (about a week in Phase 3); a teaching-profile and live-view feature after the verdict. Confidence medium.
- **Settled by.** The share of rejected corpus programs whose deferred runs reach a conflict; in a classroom, time to fix borrow errors with and without monitor counterexamples.

## 3. The visible notional machine: a value-tree stepper and a permissions overlay

Two views over the interpreter. The stepper (teaching profile) steps a run forwards and backwards and shows each frame's named values. The teaching levels exclude `ref`, `view` and `Shared` (candidate 9), so every value is a tree: no heap and no heap arrows, the usual source of confusion in reference-semantics visualisers. A borrowed parameter is drawn as a label naming the caller's variable, since second-class references point only into frames that outlive them, and a move is drawn as the value leaving one name for another, the old name greyed out as moved. Steps can go inside expressions, as in Thonny and DrRacket, so `?`, `??` and `else` show which branch they took. `sisu doc --worked` renders a doc example or a test as a static worked example: the step table, with the example's comments as subgoal labels. The permissions overlay (intermediate users) shows on each line the read, write and own permissions each binding gains and loses, as Aquascope does, from the borrow checker's loan sets, and marks the permission an erroneous operation needed and lacked.

```sisu
var team = [User(name="Aino", age=27)]    # team +R +W +O (var)
let roni = User(name="Roni", age=30)      # roni +R +O (let: no W)
add_user(mut team, roni)                  # needs W on team: held; roni −R −O: moved into team
print(roni.name)                          # needs R on roni: lost at line 3
```

- **Precedents.** Python Tutor (over 25 million users since 2010; heap diagrams with arrows) [13]; Thonny's expression stepping and DrRacket's algebraic stepper [14]; Sorva's UUhistle, which improved performance on one of four tasks in a 172-student study [3]; Aquascope and Brown's Rust Book experiment, whose permissions are R, W, O and F [12]; RustViz. Not new. Sisu-specific: value semantics make the first-course machine arrow-free, and there are no lifetimes to draw.
- **Buys.** Crichton, Gray and Krishnamurthi's conceptual model with permission diagrams raised Ownership Inventory scores by 9% (N=342, d=0.56) [12], the best evidence any aid in this file has, though it measured a rewritten chapter plus the tool, not the tool alone. Misconceptions about aliasing and references are a documented novice problem [15]; subgoal-labelled worked examples are among computing education's more replicated findings [16].
- **Costs.** Tooling only. Frames are already `{body, pc, locals}`, so traces are cheap; backward steps reuse candidate 4's replay; the overlay needs the checker to export per-point loans, which it computes. UI in the playground and the language server.
- **Risks and interactions.** Giving Sisu's bet-1 authors an overlay while the Rust controls have none confounds the friction comparison: give both arms Aquascope, or ship after the verdict. It extends the "explain this borrow" command already planned in [10](../../proposal/10-tooling.md) rather than adding a second one.
- **Verdict.** Tooling: the overlay in the language server after the bet-1 verdict; the stepper and `--worked` with the playground after 0.1. Confidence medium-high for the overlay, medium for the stepper.
- **Settled by.** Crichton's Ownership Inventory, which is published, given to Sisu learners with and without the overlay; the stepper inside the Q-41 trial.

## 4. Replayable runs: the capability log, REPL undo and why-queries

In the interpreter every source of non-determinism is a capability call, the seeded scheduler, or one of two runtime inputs, so a run is determined by the program plus what those returned. `sisu run --record` wraps each capability host object (attenuations are already wrappers) in a logger and writes a `.sisulog`: the scheduler seed, the per-process hash key (D-049), standard input (ambient under Q-5, so the runtime logs it itself), every capability result in order, and every hot-replacement swap. `sisu replay` re-executes with the log standing in for the world, and every panic or failing test prints its replay command. Two features sit on top. REPL undo: `:back 3` replays the session's earlier inputs against the log, with no heap snapshots, which is cheap at REPL scale and never touches the file system twice. Why-queries: `:why port`, or clicking a value in the stepper, replays with tracing and answers with the dynamic slice (the assignments, calls and logged capability results that produced the value), as the Whyline does.

```
sisu> let cfg = load_config(fs, "app.toml")?
sisu> let port = cfg.port + 1
sisu> :why port
port = 8081      ← cfg.port (8080) + 1                    repl:2
  cfg.port       ← parse_int("8080")                      config.sisu:14
    "8080"       ← fs.read_to_string("app.toml")          log #1
```

- **Precedents.** rr records system calls, and Pernosco adds "where did this value come from" [18]; WinDbg's Time Travel Debugging; OCaml's debugger has stepped backwards using checkpoints for decades; Elm 0.18's debugger (2016) imports and exports message histories, which works because a pure language routes every effect through its runtime [19]; Durable Functions replay orchestrations from a log of activity results and require developers to keep orchestrator code deterministic [20]; deterministic simulation testing (FoundationDB, TigerBeetle, Antithesis); the Whyline [17]. Not found: an imperative general-purpose language whose replay boundary is its authority model, enforced rather than by convention. Elm is the closest.
- **Buys.** Reproducible bug reports from learners to teachers ("send me the log"); corpus failures in the experiment replay exactly, beside the seed and SIR dump the plan already keeps; the REPL loses the hidden-state confusion notebooks are known for. The Whyline cut debugging time by nearly a factor of 8 and let users finish 40% more tasks, measured on six scenarios in Alice [17]; it stayed a research tool, and recording full traces of large programs is costly, while learner programs in an interpreter are small.
- **Costs.** Logging wrappers and a versioned log format; capability results must serialise (byte chunks for `Net` streams, which can be large); tracing for why-queries costs memory proportional to the run.
- **Risks and interactions.** A log holds file contents and environment values, so it is as sensitive as the data. Replay holds in the interpreter and the native `single` profile; the M:N `full` runtime would need interleavings recorded, as rr does. Trusted `unsafe` code can break determinism. Capabilities are what make the boundary exist; the ambient standard streams are the one hole, closed by the runtime.
- **Verdict.** Prototype the log and replay in the experiment (Phases 6 to 8 already have the deterministic scheduler and the wrappers, and triage needs it); REPL undo and why-queries are tooling after 0.1. Confidence medium-high for the log, medium-low for why-queries.
- **Settled by.** The share of corpus failures reproduced from a log alone (target: all); log size for the HTTP-server program under its load test; use of `:why` in the playground.

## 5. A reactive notebook whose dependencies include mutation

`sisu notebook` treats a script-mode `.sisu` file with `# %%` cell markers (Jupytext's percent format, so the file stays valid Sisu) as a reactive notebook on the interpreter: editing a cell reruns it and everything that depends on it, as Pluto and marimo do. Unlike them, the graph includes in-place mutation. A cell that passes a binding as `mut`, or calls a `mut self` method on it, is a writer of that binding, which the type checker resolves statically; a binding whose type has interior mutability (`Cell`, `Mutex`, `Atomic`) counts as written by any method call. Each binding has one defining cell (Pluto's rule); writers run in file order after it, readers after every writer, and a reader placed above a writer is an error. Editing a writer reruns the binding's definer first, so mutations never accumulate. A cell may borrow another cell's bindings but not move out of them (the fix-it is `.copy()`). Cells that use a real capability are effectful: they rerun on request, with results from candidate 4's log until refreshed. A cell's trailing expression statement is displayed through `View` (candidate 12); that is a display rule like the REPL's, and blocks still have no value (D-056).

```sisu
# %% load: effectful, reruns on request; the notebook binds `sys`
let fs = sys.fs.restrict_to("./data")
let raw = fs.read_to_string("weather.csv")?
# %% parse: defines rows
var rows = parse_rows(raw)
# %% clean: writes rows (retain takes mut self), so editing it reruns parse first
rows.retain(it.temp_c > -90.0)
# %% stats: reads rows, so it runs after every writer
summary(rows)
```

- **Precedents.** Observable (2018), whose `mutable` cells make reassignment across cells explicit while in-place mutation of objects goes untracked; Pluto.jl, which derives dependencies by static analysis [22]; marimo, whose documentation says it "does not track mutations" because "tracking mutations reliably is impossible in Python" [21]; NBSafety and ipyflow, which trace lineage at run time and flag some stale cells [24]; Livebook; Kotlin Notebook; evcxr (Rust in Jupyter, not reactive). Not found: a notebook that tracks in-place mutation statically. It needs mutation to be visible to the type checker, which Sisu's parameter conventions provide.
- **Buys.** Pimentel et al. found that about a quarter of 1.4 million GitHub notebooks ran without error and about 4% reproduced their stored results [23]; reactive notebooks fix the execution-order half of that, and Sisu would fix the mutation half. With candidates 6, 7 and 12 it gives scientists and analysts an exploration story.
- **Costs.** Tooling, but large: a front end or a Jupyter-kernel adapter, cell analysis on the existing queries, rerun scheduling, and snapshots of definer outputs (`.copy()`) so that reruns stay cheap.
- **Risks and interactions.** [00](../../proposal/00-vision-and-principles.md) says Sisu is not for notebook exploration "(yet)", on an AOT premise that D-052 has weakened; amending that is the owner's call. Without numerics (Q-23 is open) and tables there is little to explore, and an unoptimised interpreter meets data workloads at once (Julia's time-to-first-plot lesson). The borrow-only rule between cells is new friction.
- **Verdict.** Tooling prototype after 0.1, together with Q-23's outcome and only if the non-goal is amended. Confidence low-medium: high that the mechanism is sound and new, low on demand.
- **Settled by.** Notebooks written by outsiders, rerun top to bottom, against marimo on the same tasks (same output or not); fix-it churn from the borrow-only rule in a pilot.

## 6. Typed tables with a Python-like surface (`std.x.table`)

`Table[Row]` is columnar, with `Row` an ordinary struct. Comptime reflection lays the columns out as a struct of arrays, as Zig's `MultiArrayList` does, so `t.filter(it.temp_c > 25.0)` and `t.col.temp_c.mean()` are typed and a misspelt column is a compile error rather than a `KeyError` four hundred lines into a script. Loading validates each row at the boundary through `@derive(Decode)`, refinements included, and reports the first bad row by line number. For exploration without writing the struct, a row type can be built at compile time from a sample file, `type Reading = comptime csv.row_type(embed("data/sample.csv"))`, which is F#'s type-provider trick done with Q-27's type construction and `embed`. Q-26 rules out anonymous records, so schema-changing operations name their result type, and with no `**kwargs` aggregation is a trailing block. In the interpreter, columns are host objects with host-implemented kernels, as NumPy's are written in C.

```sisu
type Celsius = f64 where self > -273.15

@derive(Debug, Decode)
struct Reading:
    city: String
    temp_c: Celsius

struct CityStats:
    city: String
    n: int
    mean_temp: f64

let t = Table[Reading].read_csv(fs, "readings.csv")?
let stats = t.filter(it.temp_c > 25.0).group_by(it.city).map do g:
    return CityStats(city=g.key.copy(), n=len(g), mean_temp=g.col.temp_c.mean())
```

- **Precedents.** pandas; R's data frames and dplyr; data.table; Polars, which checks schemas when a lazy plan is built; DataFrames.jl; Kotlin DataFrame, which generates typed column properties after each notebook cell and through a compiler plugin [25]; F#'s CSV type provider, which infers a schema from a sample at compile time [26]; Haskell's Frames; Zig's `MultiArrayList` [27]. Not new; only the mechanism (comptime instead of a plugin or provider) is.
- **Buys.** The boundary validation std.json already does, extended to the format analysts use most, and typed columns. We found no study showing that typed data frames reduce errors, so that part is a hypothesis.
- **Costs.** A std.x package; comptime generation of accessor types (allowed by Q-27); fuel for schema inference on large samples; speed needs host kernels now and SIMD later (Q-23).
- **Risks and interactions.** Analysts value reshaping (pivots, joins, wide to long) that changes schemas constantly, and nominal types make that verbose; Kotlin needed a compiler plugin to follow schema changes. No macros means no pandas-style query strings. Analysts may stay with Python regardless.
- **Verdict.** Standard library (std.x), after 0.1 and after Q-23's research; the language needs nothing beyond Q-27. Confidence low-medium.
- **Settled by.** Porting five real pandas analyses, counting the declarations added and the column errors caught.

## 7. Units of measure, and library-rendered types in diagnostics

`std.x.units`: a comptime function constructs the quantity type for a number type and a dimension (exponents of the SI base dimensions; Q-27), its `*` and `/` compute the result dimension at compile time, `+` requires equal dimensions, and constants `m`, `s`, `km` and `h` make `9.81 * m / (s * s)` work through ordinary operator overloading with no new literal syntax. The language part is one attribute: `@diagnostic_name(f)` on a constructed or generic type names a comptime function the compiler calls when it prints that type in a diagnostic, falling back to the raw form if the function fails or runs out of fuel, with `sisu explain --raw-types` to see the truth. Without it the error shows an exponent vector, which is much of what makes C++ and Rust units libraries hard to use.

```sisu
from std.x.units import m, s, km, h

let g = 9.81 * m / (s * s)
let v = g * (3.0 * s)               # Quantity[f64, m/s]
let bad = v + 3.0 * s               # error: cannot add Quantity[f64, s] to Quantity[f64, m/s]
let shown = v.to(km / h)            # conversions are explicit and checked
```

- **Precedents.** F#'s units of measure, inferred by a dedicated solver and erased at run time [28]; GNAT's dimensionality checking for Ada; Boost.Units and mp-units in C++ (the latter proposed for the standard as P3045); Rust's `uom`; Unitful.jl; Pint (at run time); Frink, a language built around units; Swift's `Measurement`. For the hook, Lean 4's delaborators and unexpanders let libraries control how terms print in messages, and Rust's `#[diagnostic::on_unimplemented]` lets them write trait errors [31]. Not new.
- **Buys.** The Mars Climate Orbiter was lost to pound-force-seconds read as newton-seconds [30]; the hook improves the errors of every comptime-built type (tables, refinements), not only units. A 2021 survey found many units libraries and little use of them [29]; the reasons given are ergonomics and error messages, which is what the hook targets.
- **Costs.** A std.x library; for the hook, the diagnostic renderer calls comptime with fuel, which is small. Generic code over arbitrary dimensions needs definition-time reasoning about dimension arithmetic, which a library cannot give and F#'s solver can.
- **Risks and interactions.** A renderer that lies hides the real type. Every quantity is a struct in the interpreter, so units are slow until a native backend erases them. F#-style units in the type checker would be the domain-specific feature that [14](../../proposal/14-non-goals.md) excludes.
- **Verdict.** Units: std.x after 0.1, with Q-23. `@diagnostic_name`: adopt in v1 if Q-27's type construction ships. Built-in units in the type system: reject. Confidence medium.
- **Settled by.** Porting an engineering calculation suite and counting unit errors caught against annotations added; a message-comprehension test with and without the hook.

## 8. Projections over canonical text: blocks and localised keywords

The file on disk is always canonical Sisu as `sisu fmt` writes it, and editors may project it. Blocks: a MakeCode-style editor for the teaching profile whose blocks are the formatter's syntax tree, one block per statement, a C-shaped slot per indented suite, sockets shaped by type as Scratch shapes its booleans, and a palette chosen by the teaching level (candidate 9). Round trips are lossless because the formatter is canonical and preserves comments, and there are no macros or user-defined syntax to project; indentation maps onto C-shaped blocks directly. Localised keywords: the editor shows keywords and prelude names in the learner's language, with an inline dictionary on hover, while the file keeps English, which is how Scratch keeps projects language-neutral and Excel stores formulas. The cheapest part, and the part with the clearest demand, is diagnostics: message identifiers with parameters in a catalog, never English concatenated in code.

```
on disk (canonical)              projected for a Finnish learner
for x in xs:                     jokaiselle x joukossa xs:
    if x > 0:                        jos x > 0:
        print("{x}")                     tulosta("{x}")
```

- **Precedents.** MakeCode converts between blocks, Static TypeScript and Python; Pencil Code's Droplet (blocks with CoffeeScript or JavaScript); BlockPy; Snap!; App Inventor [32]; Scratch renders its blocks in dozens of languages over language-neutral projects; Excel shows localised function names over canonical storage; Hedy is offered in 47 languages and lets learners write keywords in their own [36]; Babylscript, a multilingual JavaScript (about 2010). Not new.
- **Buys.** In high-school classrooms blocks gave larger learning gains and more interest than text [33]. Among 15,015 Scratch users in five countries, novices coding with keywords and environment localised into their country's main language learned new concepts faster than compatriots using English (Dasgupta and Hill; observational) [34]. Guo's 840 respondents from 86 countries listed English keywords among their code-reading problems; 21% wanted inline dictionaries, and 7% preferred English to translations [35].
- **Costs.** A block editor is a product with a team behind it (MakeCode is Microsoft's); keyword tables per language and a display layer; a message catalog is cheap from the first diagnostic and expensive to retrofit. The v1 requirements are a lossless syntax-tree library (the formatter is already part of the parser) and the catalog.
- **Risks and interactions.** Localised keywords fragment help-seeking (a search for `jos x` finds nothing), as Excel's cross-locale formulas do, and Guo's 7% asked not to fragment the ecosystem; keeping the file English and the projection per viewer is the mitigation. The move from blocks to text is a known difficulty [33].
- **Verdict.** Adopt in v1: diagnostics through a message catalog, and the lossless syntax tree as a library (confidence medium-high). Block and keyword projections: tooling after 1.0 or by the community, keywords tried first in the playground (confidence low).
- **Settled by.** A playground comparison with Finnish- and Spanish-speaking learners, with and without keyword projection, measuring time to a first working program and later transfer to English text.

## 9. Monotone language levels: the teaching profile as a ladder

Q-41 decided on a compiler-enforced teaching profile. Make it a ladder: `profile = "teaching-3"` in the manifest or a script's header block, where each level is a named set of feature gates and a strict subset of the next level and of full Sisu, so a valid program stays valid, unchanged, as the learner climbs (Racket's HtDP languages, Ada's restriction identifiers). Hedy's levels are the counter-model: its early levels accept syntax that later levels reject, such as text without quotation marks. Using a construct from a later level gives a level-up diagnostic that names it and the level. Teachers define course ladders as data, a list of gates plus message overrides. Proposed levels: L1 script mode (top-level statements, `let`/`var`, `print`, `if`, `for` over `range` and lists, `int`, `f64`, `bool`, `str`); L2 `def` with signatures and `return`; L3 `struct`, `enum`, `match`; L4 `T?`, `T ! E`, `?`, `else`; L5 `test`, `assert`, contracts; L6 `mut` and `own` parameters and moves; L7 traits and `impl`; L8 `task.scope` and `spawn`, deterministic on the interpreter, for animations and simulations; full Sisu adds generics, `ref`, `view`, `Shared`, `comptime` and `unsafe`.

```toml
# course.toml: a teacher's ladder is data
[level.week3]
extends = "teaching-3"
deny = ["while"]
message.while = "This week loops are `for` over `range(...)`; `while` arrives in week 6."
```

- **Precedents.** HtDP's Beginning, Intermediate and Advanced Student languages [38]; Ada's `pragma Restrictions` and `Profile (Ravenscar)`; Hedy [36]; Java's on-ramp of compact source files and instance `main` (JEP 512, Java 25) [39]; Elm's lesson that absence teaches. Not new; monotonicity and teacher-defined ladders are design choices.
- **Buys.** The HtDP and Pyret evidence in [learnability.md](../topics/learnability.md); level-up messages target the "construct I have not met yet" class of novice error. For L8: children use concurrency freely in Scratch (sprites animate by running scripts at once), and Scratch avoids most races by switching threads only at the end of loops and at waits [37]; Sisu's interpreter yields on loop back-edges and calls, and `Send`/`Sync` rule out data races anyway.
- **Costs.** Gates are a list in the checker. Each level needs its slice of the wrong-program diagnostics corpus reviewed. L1's top-level statements are the one language change, and they exist anyway if scripts allow them.
- **Risks and interactions.** Script mode is a second way to write `main` (P1), so confine it to scripts and teaching levels. A fixed ladder will not match every course, hence ladders as data. Monotonicity forbids Hedy's quote-free first level; that loss is accepted.
- **Verdict.** Adopt in v1 as the shape of Q-41, shipped when the classroom trial allows ([18](../../proposal/18-next-steps.md) defers the profile until then); L8 is prototyped in the trial. Confidence medium, low-medium for L8.
- **Settled by.** The Q-41 trial with two arms, one teaching profile against the ladder, comparing error rates and task times; teacher uptake of custom ladders.

## 10. Concrete-first authoring

Three language-server actions that follow the HtDP design recipe's order, examples first, without changing the language. "Make a function from this": select statements written against concrete values, tick the literals that should become parameters, and the action writes a `def` with the original values kept as its first doc example (Edwards' example-centric programming; Extract Method plus Introduce Parameter). "Fill in the signature": in the teaching profile an unannotated `def` is an error whose fix-it writes the signature inferred from the body and from the examples that call it, so P2 holds (the file always has the signature) and the tool pays its cost, as GHC prints inferred signatures and MonkeyType infers Python annotations from observed calls. "Generalise this literal": a literal becomes a parameter with the literal as its default. In the teaching profile a generated example is written as `== ?`, so the learner states the expected value before seeing the actual one.

```sisu
# concrete first, in a script
let minutes = 135
print("{minutes // 60} h {minutes % 60} min")

# after "Make a function from this", with 135 ticked
## ```
## assert hours_and_minutes(135) == "2 h 15 min"
## ```
def hours_and_minutes(minutes: int) -> String:
    return "{minutes // 60} h {minutes % 60} min"
```

- **Precedents.** HtDP's design recipe [38]; Edwards' "Example centric programming" and Subtext [40]; IDE refactorings; GHC's `-Wmissing-signatures`; MonkeyType [40]; OCaml's expect tests and Jest's inline snapshots, which write observed values into the source. Not new.
- **Buys.** Explicit signatures cost novices more than experts, and this removes the cost without removing the signature; the design recipe has a long classroom record. We found no evaluation of the actions themselves.
- **Costs.** Code actions over the type checker. Inferring a signature from uses is not always unique (`x + 1` suggests `int`; ambiguous cases ask). No language change beyond the teaching profile's error-with-fix for an unannotated `def`.
- **Risks and interactions.** A characterising example turns a bug into the expected value, hence `== ?`. Generated signatures over-specialise, which is harmless where there are no generics. The explicit `return` is part of what the action writes.
- **Verdict.** Tooling, with the teaching profile. Confidence medium.
- **Settled by.** Think-aloud sessions in the Q-41 trial; how often students use the actions rather than writing signatures first.

## 11. A creative-coding kit on hot replacement

`std.x.sketch` for children, artists and designers: `main` hands a `Canvas` capability (and optionally audio) to a `draw` function called every frame. D-053 swaps function bodies at the next call, so editing `draw` while the sketch runs changes the next frame and keeps the state. In the editor, numeric literals in `draw` get scrubbers (drag to change), each drag a body swap with an unchanged signature, which is the case D-053 already supports. `live_loop` (Sonic Pi) is a task whose body is fetched again on every iteration and timed by a virtual clock like the test clock, so musical timing is exact. The `Canvas` fake records drawing commands, so `assert_snapshot(canvas.commands())` tests a picture, and the playground maps `Canvas` onto an HTML canvas. Output-directed editing (drag a drawn circle and the literal behind it changes, as in Sketch-n-Sketch) is rejected for now: it needs trace-based solving and is still research.

```sisu
from std.x.sketch import Canvas, Color, run, window

struct Ball:
    x: f64

def draw(mut b: Ball, c: Canvas):
    b.x = (b.x + 2.5) % c.width              # drag 2.5 to change the speed, live
    c.circle(b.x, 120.0, r=30.0, fill=Color.hex("#e4572e"))

def main(sys: System):
    run(window(sys.display, width=400.0, height=240.0), Ball(x=0.0), draw)
```

- **Precedents.** Processing and p5.js; Processing's Tweak Mode; Bret Victor's "Inventing on Principle" (2012); Flutter's hot reload; SwiftUI previews, where edits in the canvas update the code; Sonic Pi's `live_loop` and its temporal semantics [41]; TidalCycles; Sketch-n-Sketch [41]. Not new.
- **Buys.** Creative coding is an established on-ramp: Processing's and p5.js's education communities, Sonic Pi in schools (evidence of adoption, not trials). It is also the best demonstration of three decided features at once: a capability (the program draws only because `main` handed it the canvas), hot replacement, and the deterministic clock.
- **Costs.** A std.x package; a playground canvas host; a native window backend in a trusted package; a display capability, because `System` today holds `Fs`, `Net`, `Clock`, `Rand`, `Env` and `Proc` only (`sys.display` above is the proposal); scrubbers in the language server.
- **Risks and interactions.** Interpreter frame rates for non-trivial sketches, where p5.js runs on a JIT; scope creep towards a game engine; a changed `draw` signature restarts the sketch under D-053, and the message must say why.
- **Verdict.** Standard library (std.x) and tooling after 0.1, as the playground's flagship; output-directed editing rejected. Confidence medium for the kit, medium-high for the rejection.
- **Settled by.** Frame times of ported p5.js gallery examples on the interpreter (target: 60 fps for the first fifty); a workshop with children or designers.

## 12. Moldable views: a `View` trait

`View` in `std.x.view` has one method, `pure def view(self) -> ViewNode`, where `ViewNode` is a small pure document tree (text, record, table, list, tree, image bytes, a plot specification, a colour swatch) that every tool showing a value renders: the REPL and terminal (text fallback), hover and probes in the language server, notebook cells, the playground, doc examples, and assertion and snapshot diffs. `@derive(View)` renders a struct as a record and a list of structs as a table; types without `View` fall back to `Debug`. Because `view` is pure and takes no capabilities, any tool may call it at any time, which keeps candidate 1's gate sound. A related small adoption: Python 3.8's `{expr=}` in interpolation, so that `print("{total=}")` prints `total=42`.

```sisu
@derive(Debug, View)
struct Reading:
    city: String
    temp_c: f64

impl View for Histogram:
    pure def view(self) -> ViewNode:
        return ViewNode.bars([(b.label, b.count) for b in self.bins])
```

- **Precedents.** IPython's rich display (`_repr_html_` and its siblings); Julia's multimedia display system; Glamorous Toolkit's moldable inspector [42]; Clerk's viewers; Livebook's Kino; Pharo's inspectors; Python 3.8's `=` specifier. Not new.
- **Buys.** Jupyter's display protocol is why tables and plots appear without ceremony; the moldable-inspector work reports that custom views are small and cheap to write [42]; seeing a value as a picture is the centre of the live-programming experience.
- **Costs.** A trait, a document type and one renderer per tool; an API commitment, which is why it starts in std.x.
- **Risks and interactions.** P1: three rendering traits. The rule is `Display` for a program's users, `Debug` for programmers as text, `View` for tools as structure, with `View` derived from `Debug` unless written. Large values are truncated by default.
- **Verdict.** Standard library (std.x at 0.1, a candidate for core at 1.0) and tooling; `{expr=}`: adopt in v1. Confidence medium-high.
- **Settled by.** How many corpus and std types acquire custom views within a year; whether playground users open them.

## 13. Conversational explanations: error dialogues and "explain this program"

Two directions from the brief, examined and mostly rejected. Error dialogues: when a diagnostic has several plausible fixes, the compiler asks which outcome the learner wants and applies the answer. An "explain this program" view: the toolchain writes prose about what code does. What survives is narrow. Every diagnostic with several fix-its gives each a one-line consequence, which is a code-action list plus a text rule; and the teaching profile offers a deterministic gloss of dense signatures on hover, in the manner of cdecl: `def oldest(users: list[User]) -> ref User` reads "borrows `users` to read it and returns a reference to one of its elements, usable while `users` is".

```
error: `roni` is used after it was moved into `team` (line 3)
  fix 1  add_user(mut team, roni.copy())    `roni` stays usable; `team` gets its own copy
  fix 2  remove line 4                      `team` holds the only User("Roni")
```

- **Precedents.** Code actions in every editor; Elm's conversational message style; DrRacket's novice messages, which Marceau, Fisler and Krishnamurthi found students often misread despite careful design, largely over vocabulary [43]; Becker et al.'s mixed review [44]; cdecl; explainshell; language-model "explain this code" in every editor since 2023.
- **Buys.** Consequence lines answer the misreading Marceau et al. documented; the gloss is exact where a language model may be wrong about ownership.
- **Costs.** Dialogues need an interactive terminal mode and question text for every multi-fix diagnostic. Prose explanation needs a generator for every construct, which would rot and duplicates what models already do.
- **Risks and interactions.** Dialogues teach choosing an option without understanding it; a prose explainer competes with tools that do it better.
- **Verdict.** Reject both as features; adopt the consequence-line rule for diagnostics and the gloss for the teaching profile. Confidence medium-high.
- **Settled by.** The diagnostics corpus review; a comprehension test of messages with and without consequence lines.

## Also considered and rejected

- **Hedy-style levels that change syntax.** Programs and habits stop transferring between levels, which amounts to several dialects (P1); candidate 9 keeps the gradual idea without that.
- **Exact rationals by default in the teaching profile (Pyret).** The profile would become a different language in meaning, not a subset; `int / int` is already a type error with a fix-it, and `std.num` has `Ratio` and `Decimal`.
- **Sisu as a spreadsheet formula language** (Excel's `LAMBDA`, Python in Excel, Grist). Ownership and capabilities buy nothing inside a cell formula; candidate 5 covers reactivity.
- **An `example` clause in function headers (Pyret's `where:`).** A fifth header clause beside the contracts duplicates doc examples and `test` blocks (P1); candidate 1 uses those.
- **Translated standard-library names.** They fragment the ecosystem (Guo's 7%, Excel's cross-locale formulas); candidate 8 projects per viewer instead.
- **An image-based live environment.** Out of scope by Q-22 and [14](../../proposal/14-non-goals.md); hot replacement plus replay covers the live parts.

## Method and verification

Precedents were checked against primary sources until the web-search budget shared by this session's agents ran out; after that, known pages were fetched directly. Sources marked (checked) were verified during this work; the others are cited from the literature as known before it and should be re-checked before a decision relies on them. "Not found" means that we did not find the idea in the systems listed, not that the search was exhaustive.

## Sources

1. Rauch, Rein, Ramson, Lincke, Hirschfeld, "Babylonian-style Programming", *Programming* 3(3), 2019. https://programming-journal.org/2019/3/9/ (checked)
2. Lerner, "Projection Boxes: On-the-fly Reconfigurable Visualization for Live Programming", CHI 2020; follow-up on loop seeds. https://www.cs.cornell.edu/~lerner/pb (checked)
3. Huang, Ferdowsi, Selvaraj, Soosai Raj, Lerner, "Investigating the Impact of Using a Live Programming Environment in a CS1 Course", SIGCSE 2022 (also reports UUhistle). https://cseweb.ucsd.edu/~lerner/papers/proj-boxes-edu-sigcse2022.pdf (checked)
4. Saff, Ernst, "An experimental evaluation of continuous testing during development", ISSTA 2004. https://homes.cs.washington.edu/~mernst/pubs/ct-user-study-issta2004-abstract.html (checked)
5. Unison, "Interactive scratch files". https://www.unison-lang.org/docs/tour/_scratch-files (checked)
6. hls-eval-plugin. https://hackage.haskell.org/package/hls-eval-plugin (checked)
7. Pyret, "Testing" (`check:` and `where:` blocks). https://pyret.org/docs/latest/testing.html (checked)
8. Bishop, Horspool, Xie, Tillmann, de Halleux, "Code Hunt: Experience with Coding Contests at Scale", ICSE 2015. https://www.microsoft.com/en-us/research/project/code-hunt/ (checked)
9. Vytiniotis, Peyton Jones, Magalhães, "Equality proofs and deferred type errors", ICFP 2012; GHC wiki. https://gitlab.haskell.org/ghc/ghc/wikis/defer-errors-to-runtime (checked)
10. Omar, Voysey, Chugh, Hammer, "Live functional programming with typed holes", POPL 2019; Zhao et al., "Total type error localization and recovery with holes", POPL 2024. https://hazel.org
11. Swift Evolution SE-0176, "Enforce Exclusive Access to Memory". https://github.com/swiftlang/swift-evolution/blob/main/proposals/0176-enforce-exclusive-access-to-memory.md
12. Crichton, Gray, Krishnamurthi, "A Grounded Conceptual Model for Ownership Types in Rust", OOPSLA 2023, https://arxiv.org/abs/2309.04134 ; Aquascope, https://cel.cs.brown.edu/aquascope/ ; Rust Book experiment, ch. 4.2, https://rust-book.cs.brown.edu/ch04-02-references-and-borrowing.html (all checked)
13. Python Tutor. https://pythontutor.com (checked)
14. Annamaa, "Introducing Thonny, a Python IDE for Learning Programming", Koli Calling 2015; Clements, Flatt, Felleisen, "Modeling an Algebraic Stepper", ESOP 2001.
15. Sorva, "Notional Machines and Introductory Programming Education", ACM TOCE 13(2), 2013.
16. Margulieux, Guzdial, Catrambone, "Subgoal-labeled instructional material improves performance and transfer", ICER 2012; Morrison, Margulieux, Guzdial, "Subgoals, context, and worked examples in learning computing problem solving", ICER 2015.
17. Ko, Myers, "Designing the Whyline: A Debugging Interface for Asking Questions about Program Behavior", CHI 2004. https://faculty.washington.edu/ajko/papers/Ko2004Whyline.pdf (checked)
18. rr, https://rr-project.org ; Pernosco, https://pernos.co
19. Elm 0.18, "The Perfect Bug Report", 2016. https://elm-lang.org/news/the-perfect-bug-report
20. Microsoft, "Orchestrator function code constraints" (Durable Functions). https://learn.microsoft.com/en-us/azure/azure-functions/durable/durable-functions-code-constraints
21. marimo, "Reactivity". https://docs.marimo.io/guides/reactivity/ (checked)
22. Pluto.jl FAQ. https://plutojl.org/en/docs/faq/ (checked)
23. Pimentel, Murta, Braganholo, Freire, "A Large-scale Study about Quality and Reproducibility of Jupyter Notebooks", MSR 2019.
24. Macke et al., "Fine-Grained Lineage for Safer Notebook Interactions", PVLDB 14(6), 2021; ipyflow, https://github.com/ipyflow/ipyflow
25. Kotlin DataFrame, "Extension Properties API". https://kotlin.github.io/dataframe/extensionpropertiesapi.html (checked)
26. FSharp.Data, CSV type provider. https://fsprojects.github.io/FSharp.Data/library/CsvProvider.html
27. Zig standard library, `std.MultiArrayList`. https://ziglang.org/documentation/master/std/
28. F# units of measure, https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/units-of-measure ; GNAT User's Guide, dimensionality analysis; mp-units and P3045, https://mpusz.github.io/mp-units/ ; uom, https://docs.rs/uom ; Unitful.jl; Pint; Frink.
29. McKeever, Bennich-Björkman, Porter, "Unit of measurement libraries, their popularity and suitability", *Software: Practice and Experience*, 2021. https://doi.org/10.1002/spe.2926
30. NASA, Mars Climate Orbiter Mishap Investigation Board, Phase I Report, 1999.
31. *Metaprogramming in Lean 4* (delaborators, unexpanders), https://leanprover-community.github.io/lean4-metaprogramming-book/ ; Rust 1.78 release notes (`#[diagnostic::on_unimplemented]`), 2024.
32. MakeCode, https://makecode.com ; Droplet, https://github.com/PencilCode/droplet ; Bart et al., "BlockPy", 2017.
33. Weintrop, Wilensky, "Comparing Block-Based and Text-Based Programming in High School Computer Science Classrooms", ACM TOCE 18(1), 2017; Weintrop, Holbert, "From Blocks to Text and Back", SIGCSE 2017.
34. Dasgupta, Hill, "Learning to Code in Localized Programming Languages", L@S 2017. https://mako.cc/academic/dasgupta_hill-learning_code_localization-L@S2017.pdf (checked)
35. Guo, "Non-Native English Speakers Learning Computer Programming: Barriers, Desires, and Design Opportunities", CHI 2018. https://pg.ucsd.edu/publications/non-native-english-speakers-learning-programming_CHI-2018.pdf (checked)
36. Hedy, https://www.hedy.org (checked for the 47 languages); Hermans, "Hedy: A Gradual Language for Programming Education", ICER 2020.
37. Maloney, Resnick, Rusk, Silverman, Eastmond, "The Scratch Programming Language and Environment", ACM TOCE 10(4), 2010.
38. Felleisen, Findler, Flatt, Krishnamurthi, *How to Design Programs*, 2nd ed., https://htdp.org ; Ada Reference Manual 13.12 (`Restrictions`) and D.13 (Ravenscar).
39. JEP 512, "Compact Source Files and Instance Main Methods" (Java 25). https://openjdk.org/jeps/512
40. Edwards, "Example Centric Programming", OOPSLA Companion 2004; MonkeyType, https://github.com/Instagram/MonkeyType
41. Processing Tweak Mode (Sasson, 2013); Victor, "Inventing on Principle", 2012; Chugh, Hempel, Spradlin, Albers, "Programmatic and Direct Manipulation, Together at Last", PLDI 2016; Aaron, Orchard, Mycroft, "Temporal Semantics for a Live Coding Language", FARM 2014.
42. Chiş et al., "The Moldable Inspector", Onward! 2015; IPython display protocol; Julia manual, "Multimedia I/O"; Clerk viewers.
43. Marceau, Fisler, Krishnamurthi, "Measuring the Effectiveness of Error Messages Designed for Novice Programmers", SIGCSE 2011, and "Mind Your Language: On Novices' Interactions with Error Messages", Onward! 2011.
44. Becker et al., "Compiler Error Messages Considered Unhelpful", ITiCSE Working Group Reports 2019; see [studies.md](../evidence/studies.md).
