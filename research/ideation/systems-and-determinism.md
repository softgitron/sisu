# Systems, performance and determinism: candidate features

Ideation for the owner's request for features that exist in no current language, through one lens: kernels, firmware, games, databases, network services and simulations, and the people who write them. Nothing here is decided. Each candidate gives a description and a sketch, the closest precedents, what it buys, what it costs, its risks and interactions with the decided design ([04](../../proposal/04-memory-and-ownership.md), [06](../../proposal/06-concurrency-and-capabilities.md), [12](../../proposal/12-interop-and-targets.md), [16](../../proposal/16-validation-plan.md), [18](../../proposal/18-next-steps.md), [open questions](../../open-questions.md)), a verdict with confidence, and the evidence that would settle it. P10 ("steal, do not invent") applies: a candidate is "adopt in v1" only if each of its parts has a shipped predecessor; a genuinely new combination is at most "prototype".

**Finding.** Sisu already owns the three choke points that determinism and resource control need. Every external effect passes through a capability value; every scheduling decision passes through one runtime, seedable in the interpreter, with cooperative switches only at known SIR points (Q-34); and every heap allocation is visible in a type (04, rule 7). Most candidates below cash that in, and the strongest are cheap because of it. Five are new as far as I could find, each as a checked guarantee built from precedented parts: bit-determinism of `pure` across thread counts and targets (1), a simulation the program provably cannot escape (2), a recording cut whose completeness is checked (3), float reproducibility across thread counts (4) and per-task resource budgets in a shared heap without a GC (9). One idea that looks new is better described as a rule Sisu nearly has already and should state (7). Two of the thirteen are rejected as proposed, and eleven further ideas are rejected at the end.

**Method.** Precedents with a link were checked against that page on 2026-10-12, fetched directly or read through search results. The web-search budget shared by this session's agents ran out partway through; after that, known pages were fetched directly, and unlinked precedents are cited from the literature without a fresh check and should be re-checked before a decision relies on them. "I found none" means not found in the systems named, not that the search was exhaustive. The interpreter's capability log and replay are covered from the learner's side by [end-user-and-live.md](end-user-and-live.md), candidate 4; candidate 3 here is the systems extension on top of it, and the verdicts agree.

## Summary

| # | Candidate | New? | Verdict | Confidence |
|---|---|---|---|---|
| 1 | Bit-deterministic `pure` | as a checked guarantee across threads and targets (parts: Python, Haskell, Rayon) | adopt in v1 | medium-high |
| 2 | A `sim` profile with typed fault injection | as a compiler guarantee (FoundationDB, TigerBeetle, Shuttle, Coyote hold it by discipline) | prototype in the experiment | high |
| 3 | Record and replay at a checked capability cut | the checked cut (R2 had a hand-annotated one) | tooling after the verdict; the interpreter log as in end-user-and-live 4 | medium |
| 4 | Reproducible floating point by default | across thread counts (scalar: Java 17, Wasm) | strict default: adopt in v1; reductions and `std.math`: std with Q-23 | medium |
| 5 | `@no_alloc`, `@no_block`, `@stack(max=N)` | no (Clang 20, Swift experimental, Ada restrictions) | first two: adopt in v1; `@stack`: with the native backend | medium-high |
| 6 | `@no_panic` | no (SPARK Silver, Wuffs) | tooling now; attribute after 0.1 | medium |
| 7 | Guaranteed-drop views: borrowed buffers for io_uring and DMA | the I/O guarantee; the rule exists (Swift SE-0390, Rust scoped threads) | adopt in v1 (specification rule) | high |
| 8 | Regions as allocator-typed views | no (Rust `Vec<T, &Bump>`, stabilised September 2026 for Rust 1.100) | adopt in v1 (`std.collections` rule) | medium |
| 9 | Memory and CPU budgets on tasks | for shared-heap languages (Erlang has per-process limits) | prototype late in the experiment if slack allows | low-medium |
| 10 | Static lock ordering | no (Fuchsia netstack3, Clang, lockdep) | tooling, then std.x | medium |
| 11 | Priority tasks with compile-time ceilings for `freestanding` | no (RTIC, Ravenscar) | std.x after 0.1 | medium |
| 12 | `par:` blocks | no (occam, ParaSail, Rayon) | reject the syntax; `task.join` and `par_iter` in std | high |
| 13 | `schedule` blocks and layout transforms | no (Halide, Exo; Odin `#soa`, Zig) | reject `schedule`; `Soa[T]` in std | medium-high |

Top five by value times confidence: 7, 1, 2, 5, 8; candidate 4 is sixth and is what makes 1 hold across targets.

## Four findings that are not features

- **F1. `dict` order leaks into `pure`.** D-049 and [11](../../proposal/11-stdlib.md) randomise `dict` iteration per process, so `pure def keys(d: dict[str, int]) -> list[str]: return list(d.keys())` returns different lists in different processes. That contradicts 06's description of such functions as "provably pure modulo non-termination and panics", makes memoisation across processes unsound, breaks replay and simulation, and forces a choice between reproducible builds and [08](../../proposal/08-metaprogramming.md)'s promise that comptime runs "exactly what would run at run time": comptime must fix the key and run time does not. Python separated the two concerns in 3.7 (keyed SipHash against flooding, insertion order for iteration). Candidate 1 adopts that.
- **F2. Floating point is specified only as "IEEE 754"** ([03](../../proposal/03-types-and-generics.md)). 08's comptime promise names integer widths, overflow traps and `Drop` order; it fails for floats as soon as a backend fuses `a * b + c` into an FMA or links a different libm. Candidate 4.
- **F3. "First failure" depends on the schedule.** A scope reports "the first such failure" (06), first in time, so on the `full` runtime two runs of one program can report different errors. That is right for effectful code, where fast cancellation matters; candidate 1 changes it only inside `pure` code.
- **F4. `Mutex` behaviour on unwinding is unspecified.** A task that panics while holding a guard unwinds to its scope (D-040), the guard drops, and a sibling or the worker restarted by 06's supervisor sees half-updated state, even when the protected type declares an `invariant`. Rust poisons (its docs call `UnwindSafe` only a "speed bump" and count `&Mutex` as unwind-safe because of poisoning, [docs](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html)) and is now adding non-poisoning locks ([`std::sync::nonpoison`](https://doc.rust-lang.org/nightly/std/sync/nonpoison/index.html), nightly). Suggestion: when unwinding drops a guard, re-check the protected type's `invariant` if it has one and poison the mutex only if it fails, so that later `lock()` calls panic; types without invariants unlock normally. `lock()` stays infallible in its signature and contracts get a run-time job they suit.

## 1. Bit-deterministic `pure`

Make `pure` mean "the same bits for the same arguments, on every run, process, thread count and target", enforced by the compiler. Three rules do most of the work. `dict` iterates in insertion order while the SipHash key stays per process, so flooding resistance is unchanged and only iteration stops depending on the key (F1). Inside `pure` code the operations that observe the schedule are errors (`select`, `Mutex`, `Atomic`, channels with more than one sender, `try_lock`, `try_recv` and `try_send`, `Weak.upgrade` across tasks, anything derived from an address), while `task.scope`, `spawn` and `join` stay legal, so fork-join parallelism remains available. A scope inside `pure` code runs every earlier-spawned sibling to completion, cancels only later-spawned ones, and reports the lowest-index failure, so its result does not depend on which task failed first in time (F3). Floating point is candidate 4.

```sisu
pure def apply(state: Ledger, batch: Slice[Transfer]) -> Ledger ! TaskError:
    let parts = task.scope do s:                  # fork-join of pure tasks stays legal
        let fees = s.spawn(() -> compute_fees(batch))
        let risk = s.spawn(() -> score_risk(state, batch))
        return Ok((fees.join()?, risk.join()?))   # join order, never completion order
    let fees, risk = parts?
    return Ok(state.posted(fees, risk))           # same bits on 1 or 64 workers, x86 or ARM
# error[E0907]: `select` observes the schedule, so `apply` would not be deterministic
```

- **Precedents.** Haskell's purity, the `Par` monad (Marlow et al., Haskell Symposium 2011) and [LVish](https://hackage.haskell.org/package/lvish) give deterministic parallelism by construction, but hash-map order and float code generation sit outside the guarantee; Deterministic Parallel Java was "deterministic by default" through an effect system and stayed a research language ([Bocchino, 2010](https://sigplan.org/Awards/Dissertation/2010_bocchino.pdf)); Temporal and Durable Functions require deterministic workflow code and police it with a linter whose README says it "will not catch all cases of non-determinism" ([workflowcheck](https://github.com/temporalio/sdk-go/tree/master/contrib/tools/workflowcheck)) and a run-time detector that "won't catch all violations" ([Microsoft](https://learn.microsoft.com/en-us/azure/azure-functions/durable/durable-functions-code-constraints)); Python 3.7, Ruby and JavaScript's `Map` guarantee insertion order; Rayon's `join` always runs both closures and, if both panic, propagates "the panic value from the first closure" ([docs](https://docs.rs/rayon/latest/rayon/fn.join.html)), which is the index-ordered failure rule. I found no language that checks bit-determinism across thread counts and targets.
- **Buys.** Replicated state machines (a Raft apply function, Calvin-style deterministic databases), lockstep game simulation, reproducible builds and data pipelines, and sound memoisation across processes all need exactly this property, and today each project enforces it by review. It makes 08's comptime promise true and gives candidates 2 and 3 a deterministic core to simulate and replay.
- **Costs.** An insertion-ordered `dict` (Python's compact layout, Rust's `indexmap`) costs an indirection on lookup and tombstones on deletion; one more rule in the purity checker that Phase 8 already builds; a second failure policy for scopes in `pure` code, whose failures propagate more slowly because earlier siblings run to completion.
- **Risks and interactions.** Reverses the iteration half of D-049, whose stated reason (insertion order "lets code depend on it silently") becomes the point: a specified order is safe to depend on; `SortedDict` stays. An earlier sibling that never terminates now delays the failure report indefinitely (`spawn_with(deadline=...)` bounds it). Higher-order `pure` code must demand `pure` callbacks, so `pure` becomes part of function types. Comptime eligibility (06, 08) is unchanged.
- **Verdict.** Adopt in v1: all three rules have shipped predecessors, and the combined guarantee is documented only once the test below passes. Confidence medium-high; the `dict` change is the contestable part.
- **Settled by.** The corpus's `pure` functions run under `--explore 1000` and later on the native runtime with 1 and 16 workers, outputs diffed; the number of `pure` functions the new rule rejects (more than one per 500 lines of pure code reopens the primitive list); the insertion-ordered `dict` within 10% of a SipHash hash map on the corpus.

## 2. A `sim` profile with typed fault injection

A fourth runtime profile, used by `sisu test --sim`, in which `System` is entirely simulated: a clock that advances only when every task is blocked (the test clock of [17](../../proposal/17-testing.md)), an in-process `Net` with latency, loss and partitions, an `Fs` with a pessimistic durability model in which unsynced writes vanish at a simulated crash, and a seeded `Rand`. The scheduler is seeded and pluggable (random, PCT, bounded exhaustive), and several nodes, each with its own `System` and its own instances of module statics, run in one process. Faults come from types: at each capability call the simulator may return any error in that method's declared error set and nothing else. The language-level part is a guarantee: a `sim` build fails to compile if reachable code could escape the simulation, which is possible only through `extern` or `unsafe` in a `[trust]`ed package that supplies no model (the ambient standard streams are captured per node).

```sisu
test "replicas converge under partitions, torn writes and crashes":
    let sim = Sim.new(scheduler=Pct(depth=3), faults=Faults.storage() + Faults.network(drop=0.02))
    let nodes = [sim.node("r{i}", sys -> replica.main(sys)) for i in range(3)]   # production main
    sim.every(Duration.minutes(5), () -> sim.crash(sim.pick(nodes)))   # unsynced writes are lost
    sim.run_for(Duration.hours(6))                # simulated hours, seconds of real time
    assert linearizable(sim.history())
# error[E0971]: this `sim` build reaches `sqlite.exec`, an `extern` call with no simulation model
```

- **Precedents.** FoundationDB simulates whole clusters in one deterministic thread ("roughly one trillion CPU-hours of simulation"; "unlikely that we would have been able to build FoundationDB without this technology", [FDB](https://apple.github.io/foundationdb/testing.html)), and TigerBeetle's VOPR claims "one minute of VOPR time is equivalent to days of real-world testing" ([VOPR](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vopr.md)); both hold by discipline, since nothing stops their code calling the OS. Rust has the pattern as libraries (madsim, turmoil, and [Shuttle](https://github.com/awslabs/shuttle), which implements PCT, "is not sound" and "scales to much larger test cases than Loom"); .NET has [Coyote](https://microsoft.github.io/coyote/), whose Azure users report bugs found "in a matter of minutes that would have taken days with stress testing"; Go's [`synctest`](https://go.dev/blog/synctest) virtualises time inside a bubble; Antithesis and [Hermit](https://github.com/facebookexperimental/hermit) determinise unmodified binaries from outside. Deriving faults from an interface was LFI's idea (Marinescu and Candea, DSN 2009), done there by binary analysis of libraries. I found no language whose compiler guarantees that a program cannot escape its simulation.
- **Buys.** The strongest reliability results in the systems literature come from this technique (FoundationDB, TigerBeetle; S3's ShardStore checks "prevented 16 issues from reaching production", [SOSP 2021](https://www.amazon.science/publications/using-lightweight-formal-methods-to-validate-a-key-value-storage-node-in-amazon-s3)), and PCT finds a bug of depth d among n threads and k steps with probability at least 1/(n * k^(d-1)) per run ([Musuvathi et al.](https://www.microsoft.com/en-us/research/publication/a-randomized-scheduler-with-probabilistic-guarantees-of-finding-bugs/)). In Sisu the interposition costs nothing, because capabilities are the only route out and the interpreter owns every yield point; typed faults stop the simulator injecting failures the code could not have handled, the main source of noise in fault injection.
- **Costs.** A simulated `Net` and a crash-consistent `Fs` model are real work (Phase 6 already calls `Net.fake` "easy to underestimate"); PCT and a bounded exhaustive mode in the scheduler; per-node instances of module statics.
- **Risks and interactions.** An `Fs` model kinder than Linux (fsync failures, rename and directory durability) gives false confidence, so the model must be the pessimistic one; simulated time hides real-time performance bugs; `full`-profile parallelism and the weak-memory behaviour of `Atomic` are not simulated and need a loom-style mode of their own; D-057's virtual clock is already the test default, so this extends a decided design rather than adding one.
- **Verdict.** Prototype in the experiment, as an amendment to Phases 6 and 8 of about 4 pw (fault-injecting fakes, PCT, crash and restart), because it adds faults and crashes to the 1000-schedule run of the structured-concurrency suite that Phase 6's exit criteria already require, and so strengthens the test of bet 3's cancellation and panic semantics. Confidence high.
- **Settled by.** A seeded-bug study: plant 20 known concurrency and crash-consistency bugs (from the ALICE and Jepsen catalogues) in the corpus HTTP server and a small replicated log; `--sim` should find at least 15 within a fixed CPU budget, against fewer for random `--explore` schedules alone.

## 3. Record and replay at a checked capability cut

Beyond the interpreter log of end-user-and-live candidate 4, systems users need three things. The cut need not be `System`: `--record-at Db` logs only the results of `Db`'s methods and does not re-execute `Db`'s own file traffic, and the tool checks that the cut is complete, because code outside `Db`'s methods was never handed another route to the world. Native `full`-profile runs record the order of every synchronisation event (lock acquisitions, channel deliveries, `select` choices, joins, cancellation delivery, the task that released the last `Shared`) and replay in the interpreter. A release build can keep the tail of the log as a flight recorder, and a recorded failure becomes a regression test.

```toml
[profile.release]
record = { ring = "64MiB", at = ["Db", "Net"] }   # flight recorder: the last 64 MiB of Db and Net results
```
```sisu
test "regression: the crash recorded on 2026-10-12":
    let trace = Trace.parse(embed("traces/crash-2026-10-12.srec")).expect("valid trace")
    let run = trace.replay(sys -> server.main(sys))      # same results and schedule, in the interpreter
    assert run.panic() == None                           # fails until the bug is fixed
```

- **Precedents.** R2 (OSDI 2008) let developers choose the recorded interface by annotating it: recording SQLite at its own API instead of Win32 cut the log from 890 MB to 3 MB and the slowdown from 126% to 9.6%, and Apache recorded at about 10%, but completeness of the cut was the developer's responsibility ([R2](http://usenix.org/events/osdi08/tech/full_papers/guo/guo_html/index.html)); rr records Linux processes by serialising their threads ([rr](https://arxiv.org/abs/1705.05937)), and Microsoft's TTD and Pernosco are debuggers built on such recordings; RecPlay (Ronsse and De Bosschere, 1999) replays a data-race-free program exactly from its synchronisation order alone; Temporal and Durable Functions replay workflows from a history of activity results. I found no language that verifies the completeness of a recording cut.
- **Buys.** Production failures reproduced exactly and kept as tests. Recording cost scales with I/O volume, not instruction count, because everything between capability calls is re-executed. Sisu's static data-race freedom is precisely RecPlay's precondition, and Q-34's cooperative scheduling puts every context switch at a known SIR point, which rr has to recover with hardware performance counters.
- **Costs.** An encoding for every capability result type (comptime-derived); a synchronisation-order log on the native `full` runtime, one entry per contended lock or channel operation; a replayer that runs native recordings in the interpreter, which needs the two to agree exactly (D-052 promises it; candidate 4 makes it true for floats).
- **Risks and interactions.** User `Atomic` operations and trusted `unsafe` code cannot be recorded cheaply, so programs that reach them record in a serialised mode (rr's trade), and the compiler can tell which programs those are; logs contain every byte the program read, so traces are sensitive artefacts; hot replacement (D-053) composes only if a body swap is itself a logged event.
- **Verdict.** The interpreter log: prototype in the experiment, as end-user-and-live 4 says. The checked cut, native recording and the flight recorder: tooling after the verdict. Confidence medium.
- **Settled by.** On the corpus HTTP server, recording overhead (R2's 10% for Apache is the bar) and log bytes per request; 100 recorded `--explore` runs replayed with zero divergences.

## 4. Reproducible floating point by default

Specify floating point so that one source gives the same bits on every target, at every optimisation level and with any number of workers: no implicit contraction (an FMA only through `a.mul_add(b, c)`), no reassociation or fast-math, a `std.math` whose elementary functions are correctly rounded (so `sin(x)` depends on `x`, not on the libm), NaN payloads canonicalised where they become observable, and `sum`, `product` and `dot` defined as a fixed-shape pairwise reduction both sequentially and in `par_iter`, with chunks fixed by index rather than by worker count. Speed is opt-in per function with `@float(fast)`, which visibly forfeits candidate 1's guarantee for that function.

```sisu
let e = a * b + c                          # two roundings, on every target and at every -O level
let f = a.mul_add(b, c)                    # one rounding, only when asked for
let t = xs.par_iter().map(it * w).sum()    # fixed pairwise tree: same bits for 1 or 64 workers
let y = math.sin(x)                        # correctly rounded: the same bits from every platform

@float(fast)                               # contraction and reassociation allowed; not deterministic
def blend(mut out: Slice[f32], a: Slice[f32], b: Slice[f32]): ...
```

- **Precedents.** Java made all floating point strict in Java 17, removing a non-strict default that existed because of x87 ([JEP 306](https://openjdk.org/jeps/306)), and `StrictMath` fixes the fdlibm algorithms; WebAssembly's scalar floats are deterministic except for NaN bit patterns ([Nondeterminism.md](https://github.com/WebAssembly/design/blob/master/Nondeterminism.md)); Clang contracts within a statement by default (`-ffp-contract=on`, [manual](https://clang.llvm.org/docs/UsersManual.html)), the kind of default that makes x86 and ARM disagree; CORE-MATH's correctly rounded functions ship in glibc 2.42 and 2.43 ([CORE-MATH](https://core-math.gitlabpages.inria.fr/)); ReproBLAS makes sums order-independent at 4x on one core and under 1.2x on 512 cores ([ReproBLAS](https://bebop.cs.berkeley.edu/reproblas/)); Box2D v3 reached cross-platform, cross-thread-count determinism by disabling FMA, avoiding fast-math, replacing `atan2f` and fixing merge order ([Box2D](https://box2d.org/posts/2024/08/determinism/)); lockstep RTS games have depended on bit-identical simulation since Age of Empires (Bettner and Terrano, GDC 2001). I found no language that guarantees reproducibility across thread counts.
- **Buys.** Lockstep multiplayer, deterministic physics, reproducible scientific and ML runs, float constants that agree between comptime and run time (F2), and candidate 1 across targets. Pairwise summation is also more accurate than a left fold, which is why NumPy uses it.
- **Costs.** Without implicit FMA, dense kernels lose peak throughput until they use `mul_add` or `std.simd` explicitly; correctly rounded functions are slower than the fastest libms on some inputs (glibc adopting them suggests the cost is acceptable); a fixed reduction shape constrains how `par_iter` chunks work.
- **Risks and interactions.** Numerics is Q-23 and still open, so this is an input to that research, not a substitute for it; relaxed SIMD and GPU code cannot meet the guarantee and must be `@float(fast)`; `f16` and `bf16` behave less uniformly across hardware. Deciding now costs nothing, because no code depends on the alternative; Java needed more than twenty years and the end of x87 to make the same move.
- **Verdict.** Adopt the strict default and the explicit-only FMA rule in v1 (Java 17 is the predecessor); reproducible reductions and a correctly rounded `std.math` go to std together with Q-23. Confidence medium.
- **Settled by.** A numerical corpus (a physics step, an n-body integrator, a dot-product kernel) built for x86-64 and AArch64 at both optimisation levels: bit equality everywhere, and a strict-versus-`@float(fast)` throughput gap above 25% on the kernels that matter reopens the default.

## 5. `@no_alloc`, `@no_block` and `@stack(max=N)`

Three attributes the compiler checks transitively over the call graph, after monomorphisation. `@no_alloc`: no heap allocation, which in Sisu means no construction or growth of `list`, `dict`, `String`, `Box` or `Shared` and no allocating callee. `@no_block`: no capability call, lock, channel operation, `join`, `select` or `@blocking` extern, and no compiler-inserted yield point, so the function runs to completion. `@stack(max=N)`: the deepest frame chain fits in N bytes, with recursion allowed only through a `decreases` measure on a refinement-bounded parameter. The attributes are part of function types, so `dyn` methods and closure parameters can require them. `sisu build --report resources` prints the stack bound per task entry point and per `freestanding` image, which is the number firmware reviewers ask for.

```sisu
@no_alloc
@no_block
@stack(max=4096)
def render(mut out: Slice[f32], mut synth: Synth):     # the audio callback, every 2.7 ms
    for i in range(len(out)):
        out[i] = synth.next_sample()
# error[E0951]: `render` is @no_alloc, but `Synth.next_sample` may allocate:
#   it calls `self.voices.push(v)` (synth.sisu:88), and `list.push` can grow the list
```

- **Precedents.** Clang checks `[[clang::nonblocking]]` and `[[clang::nonallocating]]` at compile time, on function types ([function effect analysis](https://clang.llvm.org/docs/FunctionEffectAnalysis.html)), with [RealtimeSanitizer](https://clang.llvm.org/docs/RealtimeSanitizer.html) as the run-time complement; Swift has experimental `@_noAllocation` and `@_noLocks`, whose compiler "eagerly specializes generic functions" called from annotated code and which met friction with generic metadata ([forum](https://forums.swift.org/t/performance-annotations/54441)); Ada's `pragma Restrictions` (`No_Recursion`, `No_Implicit_Heap_Allocations`, no standard allocators after elaboration) is the program-wide form ([Ada RM D.7](https://docs.adacore.com/live/wave/arm05/html/arm05/RM-D-7.html)); GNATstack bounds Ada stacks; Zig's `@stack_size` proposal has been open since 2016 ([#157](https://github.com/ziglang/zig/issues/157)). The idea exists; Sisu's version is unusually cheap, not new.
- **Buys.** Realtime audio, interrupt handlers, kernel paths, game frame loops, and `@export` callbacks that C invokes on its own threads, since `@no_block` code never needs a scheduler context. The checks are cheap because allocation is visible in types (04, rule 7) and every blocking point is a capability, a synchronisation primitive or an `@blocking` extern.
- **Costs.** One SIR pass per attribute; checking after monomorphisation is a deliberate exception to D-017's definition-site checking, with errors at the instantiation as in Swift and Clang; `@stack` needs the native backend's frame sizes and a whole-program call graph; drop glue for recursive, auto-boxed types (D-043) must be iterative, or any function that drops a tree has an unbounded stack.
- **Risks and interactions.** Panic paths are exempt, as for `@must_consume` (D-054); `@no_block` suppresses yield points, so a long `@no_block` loop on a task thread starves its siblings (Q-34) and deserves a lint; the stack analysis is the one 06 already plans for exact task stacks (Q-6), so the work is shared.
- **Verdict.** Adopt `@no_alloc` and `@no_block` in v1, built after the verdict as SIR passes; `@stack` arrives with the native backend. Confidence medium-high.
- **Settled by.** Three realtime-shaped ports (an audio callback, an interrupt-driven UART driver, a game frame loop): the attributes stay if fewer than one rejection in ten is a false positive the author cannot fix by restructuring.

## 6. `@no_panic`: SPARK's Silver level as an attribute

`@no_panic` asks the compiler to show that no panic is reachable from a function: no trapping arithmetic that can overflow (D-009 makes every `+` a potential panic), no out-of-bounds index, no `unwrap` or `expect`, no failing contract, no allocation failure. Without the verifier the proof is syntactic plus interval arithmetic over refinement bounds and `requires` clauses: `x: int where 0 <= self < 256` makes `x * 4` safe, `xs.indices()` yields indices valid for `xs`, and wrapping or checked operators cover the rest; the post-1.0 verifier (Q-16) widens what can be discharged. Before the attribute exists, `sisu audit --panics f` lists every potential panic site reachable from `f`.

```sisu
@no_panic
pure def checksum(data: Slice[u8]) -> u32:           # from examples/capabilities.sisu: accepted
    return data.iter().fold(0u32, (acc, b) -> acc *% 31 +% b.into())

@no_panic
def header_len(buf: Slice[u8]) -> int:
    let words: int = buf[0]                           # u8 widens to int implicitly (D-016)
    return words * 4                                  # interval analysis: 0..1020, no overflow
# error[E0981]: `buf[0]` may panic because `buf` may be empty
#   help: add `requires len(buf) >= 1`, or use `buf.first()` and handle `None`
```

- **Precedents.** SPARK's Silver level is "absence of run-time errors", after which the executable can be built without run-time checks ([AdaCore](https://docs.adacore.com/spark2014-docs/html/ug/en/usage_scenarios.html)); Wuffs does all bounds and overflow checks "at compile time, not at run time", cannot allocate, has shipped in Chrome's GIF decoder since M93 (June 2021) and claims 2-6x faster GIF decoding than giflib ([Wuffs](https://github.com/google/wuffs)); Prusti proves absence of panics in Rust by default; the `no-panic` crate emulates the attribute with a linker error that depends on the optimiser ([no-panic](https://github.com/dtolnay/no-panic)). Exists, as language subsets and research tools.
- **Buys.** Kernels and firmware where a panic is an abort (`freestanding` has `panic = abort` or a user hook), parsers of untrusted input, interrupt handlers; and the inventory makes the cost of D-009's trap-everywhere rule visible and auditable.
- **Costs.** The interval analysis is small (bounds of the form `lo <= self <= hi` propagated through `+`, `-`, `*` and comparisons) but must be written into the specification, or programs will pass or fail by compiler version, which D-036's compatibility promise cannot tolerate; code under `@no_panic` is written in a narrower style, as Wuffs code is.
- **Risks and interactions.** Until the verifier exists the accepted subset is narrow and people may give up; contracts (07) supply exactly the facts the analysis needs, so callees' `requires` clauses become obligations at call sites, which is the gradual path SPARK took.
- **Verdict.** Tooling now (`sisu audit --panics`); the attribute with a specified interval analysis after 0.1; the verifier widens it later. Confidence medium.
- **Settled by.** A PNG chunk parser, a ring buffer and a UART driver written under `@no_panic`: worth specifying if none needs `unsafe` and they need fewer than one wrapping-operator rewrite per ten lines.

## 7. Guaranteed-drop views: borrowed buffers for io_uring and DMA

State as a rule what the design almost gives already: a suspended task frame, a `task.scope` and any `view` value are never leaked, so each ends only by running its `Drop`, on scope exit or during unwinding (process abort aside), and std offers no safe `forget` or `leak` for them. The runtime can then promise that a blocking operation that lent a buffer to the kernel or to a DMA engine returns, even when cancelled, only after the device has released the buffer; an io_uring cancellation that comes back `-EALREADY` waits for the original completion, as the [man page](https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html) requires. Completion-based I/O and DMA can then borrow `mut buf` from the caller's frame instead of taking owned buffers.

```sisu
def fill(file: File, mut buf: Slice[u8], offset: int) -> int ! {IoError, TaskError}:
    return file.read_at(mut buf, offset)     # io_uring writes straight into the caller's buffer
# Cancelled mid-flight, read_at submits a kernel cancel, parks until the kernel has let go of
# `buf` (cancelled, or -EALREADY and then completed), and only then returns Err(Cancelled).

view struct Transfer:                        # freestanding: a DMA transfer in flight
    rx: ref mut Array[u8, 512]
    def wait(own self) -> ! DmaError: ...    # Drop waits too, and a view's Drop always runs
```

- **Precedents.** Rust cannot offer this safely because "any object can be leaked in Rust, trivially and safely", so tokio-uring takes buffers by value and hands them back ([withoutboats, 2020](https://without.boats/blog/io-uring/)), and the scoped-task trilemma allows a Rust API only two of concurrency, parallelism and borrowing ([2023](https://without.boats/blog/the-scoped-task-trilemma/)); `embedded-dma` usually requires `'static` buffers "given the intrinsics of `mem::forget`" ([docs](https://docs.rs/embedded-dma/latest/embedded_dma/)). The enabling rule has shipped twice: Rust's closure-based `std::thread::scope`, whose join cannot be skipped, and Swift's noncopyable types, whose `deinit` always runs because SE-0390 deliberately rejects a general `forget` ([SE-0390](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0390-noncopyable-structs-and-enums.md)). Goroutines and Java virtual threads cannot be dropped while suspended either, but nothing stops a Go or Java program writing to a buffer the kernel is filling. I found no safe language that builds its I/O API on such a guarantee.
- **Buys.** Zero-copy completion I/O (io_uring, IOCP) and DMA with stack, arena or field buffers, with no buffer pools and no `(result, buf)` round trip: the API storage engines and network services want, where today they copy or use `unsafe` pools.
- **Costs.** Next to nothing in the language, since views are already second-class and scopes already join; in the runtime, cancellation latency becomes "until the device lets go", bounded by device latency (a regular-file read in progress may not be cancellable); the rule constrains std forever (no safe `forget` for views).
- **Risks and interactions.** This rule keeps bet 3's fallback honest: if Phase 1 sends Sisu to stackless task bodies (16, bet 3), the state machines must stay owned by their scope and never become droppable future values, or Sisu inherits Rust's problem. `Shared` cycles can still leak non-view values, which is harmless because those cannot hold borrows; `task.shield` composes.
- **Verdict.** Adopt in v1 as a specification rule plus the std API shape. Confidence high. One cheap addition to Phase 1's harness: cancel a million in-flight io_uring reads into stack buffers and check that no buffer is written after its cancelled call has returned.
- **Settled by.** That harness test, and a storage-engine microbenchmark of borrowed-buffer reads against an owned-buffer pool: a measurable saving in copies or memory keeps the API.

## 8. Regions as allocator-typed views

Collections take their allocator as a type parameter with a default, `list[T, A: Allocator = Global]`, and a borrowed region is a valid allocator, giving `list[T, ref Region]`. Because the collection then holds a `ref`, it is a view by the existing rule (04): it cannot escape the frame that owns the region, cannot be stored in `Shared`, and cannot cross `spawn` (a bump `Region` uses interior mutability, so it is not `Sync`). Resetting the region needs `mut region`, which the borrow checker grants only once every collection using it is dead. Per-request and per-query regions (nginx and APR pools, PostgreSQL memory contexts, protobuf arenas) become safe with no new mechanism and no lifetime names.

```sisu
def handle(req: Request) -> Response ! HttpError:
    with Region.bump(capacity=65536) as region:       # one region per request, freed at the dedent
        var headers = list.with_allocator(ref region)  # list[Header, ref Region]: a view
        parse_headers(req.head, mut headers)?
        return Ok(render(headers))                     # the Response is built in the global heap
        # return Ok(headers)  # error: `headers` borrows `region` and cannot leave this block
```

- **Precedents.** Rust merged the stabilisation of `Vec<T, A>`, `Box<T, A>` and `impl Allocator for &A` in September 2026, for Rust 1.100 and ten years after RFC 1398 ([PR 156882](https://github.com/rust-lang/rust/pull/156882), [#32838](https://github.com/rust-lang/rust/issues/32838)); `Vec<T, &'a Bump>` is this design with a named lifetime. Cyclone's regions, MLKit's region inference and the RTSJ's scoped memory (checked at run time) are the research and specification precedents; Go's arena experiment is "on hold indefinitely due to serious API concerns", chiefly that explicit arenas are infectious to APIs ([#51317](https://github.com/golang/go/issues/51317)). Exists; Sisu removes the lifetime parameter.
- **Buys.** Allocation-heavy services and compilers get bump-allocation speed and wholesale freeing with compile-time escape checking, which answers part of the application-tier pressure behind Q-1 without a GC.
- **Costs.** An allocator type parameter on every collection, hidden by the default in most code; std naming must separate a bump `Region` from today's generational `Arena[T]` with `Key[T]` (04 uses "arena" for both).
- **Risks and interactions.** Go's objection applies: a function that builds a `list` must be generic over `A` to work inside a region, so the measurement below counts how many must be; element `Drop` still runs, so deterministic destruction (D-008) holds and only the memory release is batched; a region-backed list cannot be returned, which is the point but will surprise people.
- **Verdict.** Adopt in v1 as a `std.collections` rule: the parameter is cheap before 1.0 and slow to add after (Rust took ten years). Confidence medium.
- **Settled by.** The corpus HTTP server and parser rewritten with per-request and per-file regions: allocation time and peak memory against the global allocator (a 20% end-to-end gain on the server keeps it), and the number of functions that had to become allocator-generic.

## 9. Memory and CPU budgets on tasks

`spawn_with` accepts budgets: `memory=` (bytes allocated by the task and its descendants and not yet freed) and `cpu=` (reductions or CPU time). The runtime charges each allocation to the nearest budgeted task among the allocating task and its ancestors, and credits each free to the budget that was charged. An allocation over budget fails through Q-4's two paths: `try_` variants return `OutOfMemory`, and everything else unwinds the task (D-040), whose joiner sees `Err(BudgetExceeded)`, a new `TaskError` variant beside `TaskPanicked`; a CPU overrun cancels the task's subtree and reports the same error. Budgets nest like cgroups, inside one process, and memory a finished task hands back (its result) is re-charged to the joiner.

```sisu
def guarded(net: Net, req: Request) -> Response:
    let outcome = task.scope do s:
        let h = s.spawn_with(memory=Bytes.mib(16), cpu=Duration.millis(200)) do:
            return handle(net, req)           # every allocation in this subtree is charged here
        return h.join()                       # joined here, so its failure cancels no one else
    match outcome:
        case Ok(resp):
            return resp
        case Err(BudgetExceeded(..)):         # its 17th MiB or its 201st ms; no OOM killer involved
            return Response.status(503)
        case Err(_):
            return Response.status(500)
```

- **Precedents.** Erlang's per-process `max_heap_size` (OTP 19) kills a process whose heap passes a limit, which is easy with per-process heaps; ClickHouse enforces per-query `max_memory_usage` with a tracker inside its allocator; WebAssembly runtimes meter fuel and memory per instance; Cloudflare Workers limit CPU time per request; Sisu's own comptime has fuel and memory limits (08). I found no shared-heap language without a GC that attaches budgets to structured tasks.
- **Buys.** Multi-tenant servers and databases contain a pathological request instead of being OOM-killed together with every other request; the task tree is already the supervision tree, so the budget tree comes with it.
- **Costs.** Per-allocation accounting (a pointer to the charged budget in the allocation header or in per-task size-class pages, plus an atomic add when a budget is shared across threads); CPU accounting reuses the reduction counter Phase 6 builds for yield points.
- **Risks and interactions.** Memory moved from a budgeted task to another task stays charged to its origin until freed, which can surprise; `Shared` values freed by another task must credit the origin, which is why the header is needed; `@no_alloc` code is unaffected; the cost must be zero when no budget is set, or the feature cannot exist in the default runtime.
- **Verdict.** Prototype late in the experiment only if Phase 8 has slack (about 1 pw in the interpreter, whose host allocator sees every allocation), otherwise right after 0.1; decide for native after measuring the header cost. Budgets on a shared heap have no shipped language predecessor, so P10 caps this at prototype. Confidence low-medium.
- **Settled by.** A native allocator benchmark, budgeted against unbudgeted (under 5% on allocation-heavy code), and a run of the corpus server with one runaway request: the other requests' p99 latency unchanged.

## 10. Static lock ordering

Give a `Mutex` an optional lock level, a comptime value in its type (`Mutex[Db, DB]`), and declare a partial order between levels. Acquiring a lock whose level is not ordered after every level the task holds is a compile error. Guards are already scope-bound locals, so within a function the held set is known; across calls, each function's acquired levels are inferred within a package and become part of its signature, and of the API diff, at package boundaries. Before any compiler work, the interpreter can run lockdep: record the acquisition-order graph across all test runs and report cycles even when no deadlock happened.

```sisu
const DB = lock.level("db")
const CACHE = lock.level("cache", after=DB)       # db is always taken before cache

def refresh(db: Mutex[Db, DB], cache: Mutex[Cache, CACHE]):
    let c = cache.lock()
    let d = db.lock()
# error[E0961]: lock level `db` acquired while holding `cache`, which is ordered after `db`
#   help: lock `db` first, or end `c` before this line
```

- **Precedents.** Fuchsia's netstack3 enforces lock order at compile time in Rust, with lock levels as types and `Locked` context tokens whose methods "prevent out-of-order locking" ([lock-order](https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/src/connectivity/network/netstack3/core/lock-order/src/lib.rs)); Clang's thread-safety analysis has `ACQUIRED_BEFORE` and `ACQUIRED_AFTER` ([Clang](https://clang.llvm.org/docs/ThreadSafetyAnalysis.html)); Linux's lockdep finds inversions at run time; Boyapati, Lee and Rinard's ownership types (OOPSLA 2002) prevented deadlocks with lock levels. Exists.
- **Buys.** Deadlock is the one concurrency bug class 06 admits Sisu does not prevent; in services and kernels it is rare, catastrophic and hard to reproduce.
- **Costs.** Inferring acquired-level sets across a package is an interprocedural analysis with the same effect-polymorphism problem as candidate 5 for closures and `dyn`; comptime-valued type parameters (Q-27's mechanism).
- **Risks and interactions.** Most programs hold one lock at a time and would pay annotation cost for nothing, so levels stay optional and unlevelled locks are unordered; a level belongs to a type position, not an instance, which is netstack3's model too.
- **Verdict.** Tooling first (lockdep in the interpreter, nearly free once Phase 6 exists), then a std.x library in netstack3's token style; compiler inference only if the corpus produces deadlocks the tooling misses. Confidence medium.
- **Settled by.** Lockdep over the corpus and the port study; if real lock-order cycles are rare, stop at the tooling.

## 11. Priority tasks with compile-time ceilings for `freestanding`

`freestanding` has no scheduler, so `task.scope` does not compile there (12) and firmware has no concurrency story. Fill the gap with the Stack Resource Policy: interrupt handlers are tasks with static priorities declared by attributes, and shared resources are fields of one struct. A comptime function reads the attributes (08: reflection exposes attributes as values), computes each resource's ceiling (the highest priority of any task that uses it) and produces the vector table and lock masks as `const` values, so no macro is needed. Tasks run to completion and must be `@no_block` (candidate 5), so all of them share one stack, bounded by the sum over priority levels of each level's largest `@stack`: the RAM figure for the whole image.

```sisu
struct Res:                                   # resources shared across interrupt priorities
    rx: RingBuffer[u8, 64]
    stats: Stats

@task(interrupt="USART1", priority=3, uses=["rx"])
def on_uart(mut cx: Ctx[Res]):
    cx.lock("rx") do rx:                      # raises BASEPRI to the ceiling computed at comptime
        rx.push(cx.uart.read_byte())

const APP = comptime srp.app(Res, [on_uart, on_tick])   # ceilings, vector table, stack bound
```

- **Precedents.** RTIC builds on SRP with ceilings computed at compile time and promises "race-free resource access and deadlock-free execution on a single-shared stack", through a Rust procedural macro ([RTIC](https://rtic.rs/2/book/en/)); Ada's Ravenscar profile uses ceiling-locked protected objects with ceilings declared by hand; Baker's SRP (1991) is the theory. Exists; the Sisu version needs no macro.
- **Buys.** Race-free, deadlock-free interrupt concurrency on Cortex-M and RISC-V 32 with a computed RAM bound, for 12's tier-2 bare-metal targets, which are exactly where colourless stackful tasks cannot go.
- **Costs.** A `std.x.srp` library, comptime reflection over a list of functions and their attributes, and a board-support interface for interrupt masking.
- **Risks and interactions.** Resource names in attributes are checked only at comptime, so errors must point at the attribute; two concurrency models in one language (tasks in `full` and `single`, SRP tasks in `freestanding`) must not leak into libraries, which stay concurrency-agnostic by taking capabilities.
- **Verdict.** Standard library (std.x) after 0.1; `freestanding` is outside the experiment. Confidence medium.
- **Settled by.** One embedded port of an RTIC example application: comparable code size, and the computed stack bound checked against a measured high-water mark.

## 12. `par:` blocks and parallel iteration

As proposed: a `par:` block whose statements run concurrently and join at the dedent, with the ownership checker proving that they touch disjoint state, and `for x in par mut xs` for data-parallel loops.

```sisu
let lo, hi = xs.split_at_mut(mid)
par:                                          # proposed sugar: each statement is a task
    sort(mut lo)
    sort(mut hi)
# what already works, and is kept:
task.join(() -> sort(mut lo), () -> sort(mut hi))    # disjoint `mut` borrows, proven today
for p in particles.par_iter_mut():                   # work-stealing chunks, `Send` checked
    p.step(dt)
```

- **Precedents.** occam's `PAR` with compile-time usage checking; ParaSail, where "every ParaSail expression is defined to have parallel evaluation semantics" and the rules "prevent data races" ([ParaSail](https://adacore.github.io/ParaSail/)); Cilk's `spawn` and `sync`; Rayon's `join` and `par_iter_mut`, checked by Rust's borrow checker; DPJ's `cobegin`. All exist.
- **Buys.** Brevity, and nothing else: `task.scope` already lets spawned closures borrow the frame and already checks disjointness and `Send`/`Sync`.
- **Costs.** A keyword, and a block whose statements have task semantics (what would `return` or `?` inside it mean?); a temptation to spawn a stackful task per statement where a leaf loop wanted a work-stealing chunk.
- **Risks and interactions.** Data-parallel loops want a work-stealing pool and candidate 4's fixed chunking, not one green thread per element, which is a library decision.
- **Verdict.** Reject the syntax; adopt `task.join` (two to eight closures, Rayon's index-ordered panic rule) and `par_iter`/`par_iter_mut` in `std.task`, with reductions defined by candidate 4. Confidence high.
- **Settled by.** Revisit sugar only if the corpus shows more than one `task.scope` per 300 lines used purely as a two-way fork.

## 13. `schedule` blocks and layout transforms

As proposed: Halide-style `schedule` blocks attached to `pure` functions (tile, vectorise, parallelise, fuse, store-at), and layout transforms such as struct-of-arrays applied to an existing type without changing the algorithm.

```sisu
let ps = Soa[Particle]()                   # kept: comptime builds one list per field
ps.push(Particle(pos=p, vel=v, mass=1.0))
for p in ps.iter_mut():                    # p is a view struct of `ref mut` fields
    p.pos += p.vel * dt
# rejected: a second language for one domain
# schedule blur: tile(x, y, 64, 64).vectorize(x, 8).parallel(y)
```

- **Precedents.** Halide (PLDI 2013), built on "decoupling algorithms from schedules" ([Halide](https://halide-lang.org/)), and Exo (PLDI 2022), which moves optimisation choices to the engineer and claims parity with MKL, OpenBLAS and BLIS ([Exo](https://exo-lang.dev/)), are DSLs embedded in C++ and Python, as are TVM and Tiramisu. Struct-of-arrays is built into Odin (`#soa[N]T`, `#soa[dynamic]T`, [Odin](https://odin-lang.org/docs/overview/)) and ISPC, is a comptime library in Zig (`std.MultiArrayList`), and is automatic in Futhark. All exist.
- **Buys.** Schedules give large speedups on stencils and tensor kernels, but only for a restricted algorithm language (pure functions over integer domains) that Sisu is not; SoA gives cache-friendly iteration for games, simulations and columnar data.
- **Costs.** For schedules: a second sublanguage with its own semantics, checks of schedule legality, and a rewrite or polyhedral engine in the compiler, all for one domain while Q-23 is open. For SoA: a comptime type constructor and its view element type.
- **Risks and interactions.** SoA fits as is: Q-27 lets comptime construct types, and the element "reference" is a view struct of `ref` fields, which the ownership model already handles. A built-in `@layout(soa)` on the struct would instead change what `list[Particle]` costs everywhere, invisibly, against the rule that costs show in types (04, rule 7); the explicit `Soa[T]` keeps the layout in the type.
- **Verdict.** Reject `schedule` in the language (revisit as a comptime-driven std.x kernel library inside Q-23's research); adopt `Soa[T]` in `std.collections`. Confidence medium-high.
- **Settled by.** Q-23's research for schedules; for SoA, whether `Soa[T]` with view elements passes the ownership corpus without `unsafe`.

## Also considered and rejected

- **Transactional `atomic:` blocks (STM).** Haskell's STM works because the type system keeps I/O out of transactions; Duffy's [retrospective on STM.NET](http://joeduffyblog.com/2010/01/03/a-brief-retrospective-on-transactional-memory/) calls side effects "the 'elephant in the room'" and records weak-atomicity and privatisation problems. Sisu could check "no capabilities, no shared mutation except transactional variables" the way it checks `pure`, so a `std.x.stm` over `pure` closures would be sound, but contended systems code rarely wants it and `Mutex[T]` plus bounded channels cover the cases. Not a language feature; low priority for std.x.
- **Logical-time reactors (Lingua Franca).** LF programs are deterministic unless they use explicitly nondeterministic constructs, and LF's own documentation says deadlines make a program nondeterministic ([LF](https://www.lf-lang.org/docs/), [deadlines](https://www.lf-lang.org/docs/writing-reactors/deadlines)). Candidate 2's virtual clock already gives Sisu logical time for testing; a reactor library on the `single` profile is a std.x option, and a second concurrency model in the language is not worth it.
- **Promise pipelining (E, Cap'n Proto).** Pipelining needs calls that return a promise without blocking ([Cap'n Proto](https://capnproto.org/rpc.html)), which is function colour under another name; an explicit batch API in an RPC library gets the round-trip saving.
- **`@memo` incremental computation (Skip, Salsa, Adapton).** [Skip](http://skiplang.com/) built memoisation with reactive invalidation into a language and stayed niche; with candidate 1, memoising `pure` functions is sound as a library, and the memory policy is application-specific.
- **Session types and typestate protocols.** `@must_consume` (D-054) plus `own self` transitions already express typestate the way embedded Rust does; session-typed channels (Singularity's channel contracts, Rust's `session-types`) are a library; no language change.
- **Time budgets and WCET as effects.** A worst-case execution time needs a model of the hardware (AbsInt's aiT analyses binaries for that reason), and run-time deadlines already exist as `spawn_with(deadline=...)`. Reject.
- **Production hot upgrade with state migration (Erlang, Kitsune).** D-053 limits replacement to debug builds on purpose, and services use rolling restarts and socket hand-off. Reject.
- **Crash-isolated scopes**, whose captures must be `own` or deeply immutable so that a panic cannot corrupt outside state. This is Rust's `UnwindSafe`, which its own documentation calls a "speed bump"; reject the type check and fix `Mutex` unwinding instead (F4).
- **Durability as typestate.** `write` returning a `@must_consume` `Unsynced` token, `sync` turning it into `Durable`, and acknowledgements requiring `Durable`: ALICE (OSDI 2014) and the fsync-failure study (USENIX ATC 2020) document the bug class. Worth an API experiment in `std.fs` under candidate 2's crash simulation; not a language feature.
- **Peripherals as capabilities with comptime register maps.** svd2rust-style singletons generated at comptime from `embed("device.svd")` instead of a build script; an ecosystem pattern for `freestanding`, and a test of comptime speed on multi-megabyte inputs.
- **Constant-time secret types (FaCT, Jasmin, CT-Wasm).** They need back-end guarantees against introduced branches; a question for the security lens, not this one.
