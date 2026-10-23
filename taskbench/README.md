# taskbench: the bet-3 stack harness

Phase 1 of the Sisu experiment ([proposal/18](../proposal/18-next-steps.md),
"Phase 1: the bet-3 stack harness, no Sisu involved"). The question:

> Can stackful, colourless tasks on stacks that cannot move match Go and Java
> virtual threads within the pre-registered factors?

`taskbench` is a standalone Rust crate with no dependency on the Sisu compiler.
It contains an M:N work-stealing runtime with structured scopes and
cancellation, five interchangeable stack strategies (A-E), a benchmark binary
whose subcommands measure every bet-3 row, Go and Java controls that run the
same workloads into the same CSV schema, a tokio reference, and scripts that
run everything. Every number it produces carries the era label **harness**:
Rust code on native stacks, standing in for compiled Sisu.

## Status

| Deliverable | State |
|---|---|
| Runtime: scheduler, scopes, cancellation, parking, timers, network I/O, io_uring driver | done, unit-tested (`cargo test`) |
| Stack strategies A (guard page), B (pooled + probes), C (exact small), D (decommit on park), E (stackless) | done, unit-tested |
| Benchmarks `spawn-join`, `idle-memory`, `max-tasks`, `echo`, `idle-connections`, `deep-recursion`, `starvation`, `probe-overhead`, `cancel-uring` | done, smoke-tested (`tests/smoke.rs`), run at full scale on both machines |
| Controls: Go, Java virtual threads, tokio, OS threads | done, run at full scale on both machines |
| `run_all.sh`, `scale.sh`, `load.sh`, `controls/run_controls.sh`, `summarize.py` | done, run here; `.github/workflows/harness-full.yml` runs them on a GitHub-hosted runner |
| `docs/stackless-fallback-estimate.md`, `docs/report-template.md` | done |
| Phase 1 report, [`docs/report-phase1.md`](docs/report-phase1.md) | done for both machines: every configuration side by side, the verdict per threshold (provisional until the pre-registration is frozen) and the Q-6 recommendation; rendered from the CSVs by `report/render.py` |
| Full-scale runs | machine 1 (this VM): every benchmark, five runs per cell, 2026-10-13, in [`results/vm/2026-10-13-phase1/`](results/vm/2026-10-13-phase1/), the groups that measure time behind the quiet gate; the 10k-connection echo is not run and idle connections ran at 9,900 (`RLIMIT_NOFILE`). Machine 2 (a GitHub-hosted runner, 2 vCPUs, 7.75 GiB): every benchmark, five runs per cell, 2026-10-14, the `harness-full` workflow's run 38518035636 inside a 12 GiB memory scope, in [`results/github-runner/full-38518035636/`](results/github-runner/full-38518035636/); the memory guard stopped the 1M-task `idle-memory` cells of B and D(B) at 1 MiB and 8 MiB (failed rows). Its earlier CI smoke run is in [`results/github-runner/2026-10-13-smoke-b5349dd405e8/`](results/github-runner/2026-10-13-smoke-b5349dd405e8/), and the log summary of a lost first full run in [`results/github-runner/full-38495423545-log-summary.md`](results/github-runner/full-38495423545-log-summary.md) (a cross-check only) |

## Layout

```
Cargo.toml            single crate, MSRV 1.97, edition 2024
src/main.rs           the `taskbench` binary (calls cli::main)
src/cli.rs            command-line parsing and help
src/csv.rs            the shared CSV schema and writer
src/machine.rs        machine specification (uname, CPU, memory, sysctls, rlimits)
src/mem.rs            VmRSS/VmPTE/Pss readings, VMA count, mincore residency
src/perf.rs           optional hardware counters (instructions, cycles)
src/stats.rs          nearest-rank percentiles
src/stack/            StackStrategy and A-E, probes, guard-page fault handling
src/rt/               the runtime: scheduler, tasks, scopes, API, timer, net, io_uring
src/bench/            one module per benchmark, plus shared workload bodies
src/tokio_ref.rs      the same workloads on tokio
tests/smoke.rs        integration smoke tests: every benchmark through the binary
controls/go/          Go control (spawn-join, idle-memory, max-tasks, echo, idle-connections, starvation); `go test`
controls/java/        Java virtual-thread control (same workloads)
controls/run_controls.sh
scale.sh              the smoke and full scale parameters, shared by run_all.sh and the controls
load.sh               the machine-load record (load.csv) and the quiet gate (--quiet-below), shared likewise
run_all.sh            every benchmark x every configuration x controls x repetitions -> results/<machine>/<date>/
summarize.py          per-threshold tables (median [range] over repetitions, ratios to Go) from results directories
docs/                 stackless-fallback cost estimate; the Phase 1 report template; the Phase 1 report
report/               the Phase 1 report's source (report-phase1.md.in) and its renderer (render.py, tables.py),
                      which computes every figure from the results CSVs
results/              vm/2026-10-13-phase1: the Phase 1 runs (full scale, five repetitions, machine 1;
                      superseded and aborted runs in subdirectories, each with a note);
                      github-runner/full-38518035636: the Phase 1 runs on a runner (full scale, five
                      repetitions, machine 2); github-runner/2026-10-13-smoke-b5349dd405e8: the CI smoke
                      run on a runner; github-runner/full-38495423545-log-summary.md: the job log's
                      summary of a full run whose CSVs were lost;
                      vm/2026-10-12 and vm/2026-10-12-large: development runs of harness 0.1.0
```

## Build and test

```
cargo build --release          # target/release/taskbench
cargo test                     # 56 unit tests, 11 integration smoke tests, 1 doc test
cargo clippy --all-targets     # clean
(cd controls/go && go test)    # the Go control's memory-guard test
./run_all.sh                   # smoke scale, one repetition, about 3 minutes on 4 vCPUs
./run_all.sh --scale full      # five repetitions of every cell, about 3 hours on 4 vCPUs, 16 GiB
./run_all.sh --scale full --only idle-memory,max-tasks --reps 1   # a subset
./run_all.sh --scale full --quiet-below 0.3   # on a shared machine: wait for quiet, measure again when busy
python3 summarize.py [--markdown] results/<machine>/<date> [more directories]
```

The GitHub workflow `.github/workflows/harness-full.yml` runs `--scale full` on
a GitHub-hosted runner, inside a systemd scope with a 12 GiB memory cap, when a
tag `harness-run-*` is pushed or a push to the working branch carries
`[harness-full]` in its head commit message, and uploads the results as the
artifact `taskbench-full-<run id>`.

## How the harness maps to the bet-3 thresholds

Thresholds are those of [16](../proposal/16-validation-plan.md) section 3 and the
bet-3 table of [18](../proposal/18-next-steps.md) ("Measurements and thresholds").

| Bet-3 row (threshold) | Benchmark | Metrics (CSV `metric`) | How to read |
|---|---|---|---|
| Memory per idle task at 100k and 1M, after shallow use and after a 32 KiB excursion (within 4x Go) | `idle-memory --tasks N` | `total_per_task_shallow`, `total_per_task_excursion` (RSS + page tables), also `rss_*`, `pte_*`, `pss_*` | divide by Go's same metric; after the excursion by Go's `*_excursion_after_gc` (policy 4: Go shrinks stacks at GC); `summarize.py` applies section 2.4's rule |
| Spawn-plus-join median and p99 (median within 3x Go with pooled stacks) | `spawn-join` | `spawn_join_p50`, `spawn_join_p99` | strategy B against `go` |
| 1M idle tasks on a 16 GiB machine with default sysctls (fail: guard-page-per-task is out) | `max-tasks --tasks 1000000 --hold-secs 60` | `max_tasks`, `limit` (`vma_count`, `memory`, `threads`, `mem_guard`, `cap`), `parked_after_hold` | A stops at `vma_count` near 32k with `vm.max_map_count=65530` |
| 10k-connection echo p99 (within 2x Go); 100k-connection idle RSS | `echo --connections 10000`, `idle-connections --connections 100000` | `echo_rtt_p50/p99/p999`, `echo_throughput`; `total_per_connection` | p99 against `go` |
| Stack exhaustion (any segfault is a bug) | `deep-recursion` | `parse_outcome`, `naive_outcome` (note: completed / stack_exhausted and mechanism), `*_depth_reached`, `*_leaked_*`, `*_probe_reserve_used`, `runtime_healthy_after` | every outcome must be clean; a crash leaves a `process_exit` row |
| Starvation (no worse than Go, p99 under 10 ms) | `starvation --tasks 4*threads --yield-every K` | `probe_loaded_late_p99` (and `_baseline_`) | against Go with its default asynchronous preemption and no `Gosched` (policy 8) |
| Probe overhead on compute loops (at most 3 % if B becomes the design) | `probe-overhead` | `<workload>_split_probe_overhead` (also `arg`, `tls`; instruction counts where a PMU exists) | the `split` model is a compiler-inserted prologue check; `loop` is the yield check on a back-edge, which policy 8 counts in the same budget |
| Amendment A1 (D-058): no buffer written after its cancelled read returned | `cancel-uring --pipe-mix cancel --buf-bytes 4096` (and the race mix) | `buffer_writes_after_return` (must be 0), cancel outcomes `cancel_ok/enoent/ealready`, `cancel_to_return_*` | `--break-contract` must report violations |

## The runtime under test

**Scheduler** (`src/rt/worker.rs`). N worker OS threads (`--threads`, default the
CPU count). Each has a crossbeam FIFO deque and a one-task `next` slot (Go's
`runnext`); there is a global injector, and idle workers steal batches from a
random victim. A task spawned or woken by the running task takes the `next`
slot, so spawn-then-join and wake-then-block stay on one worker. A yielding
task goes to the back of the global queue, which every worker checks first
once every 61 scheduling decisions. Network-woken tasks are queued FIFO, never
into `next`, as Go does with netpoll results; that one rule halved the echo
p99. Idle workers poll the network, spin briefly, then sleep. Pushing stealable
work wakes a sleeper only when nobody is spinning. All task state transitions
are SeqCst; correctness over cleverness.

**Tasks** (`src/rt/task.rs`, `src/rt/api.rs`). A stackful task is a corosensei
coroutine on a stack from the strategy; corosensei supplies the context switch
(no hand-written assembly). A stackless task (E) is a boxed future. Both use one
state machine (SCHEDULED, RUNNING, PARKED, DONE) and a wake-up token, so the
same `Waker` serves timers, I/O, joins and gates. `park()`, `yield_now()` and
`current_waker()` are the primitives. The task body catches its own panics, so
no panic crosses the context switch. Panics are reported on the worker's own
stack (`src/rt/panic_hook.rs`), because the default hook's backtrace needs tens
of KiB, more than a B or C stack has spare.

**Structured scopes and cancellation** (`src/rt/scope.rs`). The rules of
[06](../proposal/06-concurrency-and-capabilities.md):

- `rt::scope(|s| ...)` cannot return before every child has finished, even if
  its body panics, so children may borrow from the frame (as with
  `std::thread::scope`).
- A child that panics or exhausts its stack cancels its siblings. Its failure
  goes to its joiner, or becomes the scope's result if nobody joined it.
- `s.cancel()` cancels the subtree: every blocked task is woken and sees
  `Err(Cancelled)` at its next blocking point (`yield_now`, `sleep`, `join`,
  gate waits, network operations, io_uring reads).
- `AsyncScope` is the stackless counterpart. Its children must be `'static`,
  because a Rust future can be leaked (N-07).

**Stack strategies** (`src/stack/`), all behind `trait StackStrategy`:

| | Strategy | Defaults | Exhaustion |
|---|---|---|---|
| A | one `mmap` per task (`MAP_NORESERVE`, `MADV_NOHUGEPAGE`) with a `PROT_NONE` guard below; lazy commit; optional cache of freed stacks (`--stack-cache`) | 1 MiB reservation, 4 KiB guard | the SIGSEGV handler on a per-worker alternate stack recognises the guard of the running task and makes its coroutine return; the frames are discarded **without running destructors** (unwinding from a fault would need non-call exception tables, which Rust lacks) |
| B | slots of reservation + probe reserve carved from 1 GiB reservations (69 mappings in all at 1M tasks), LIFO free list, no guard | 1 MiB + 5 KiB reserve (the slot is rounded up to a page: 1 MiB + 3 KiB usable) | software probe at function entry; `resume_unwind` of a `StackExhausted` payload, so destructors run |
| C | exact small slots (2-16 KiB + reserve, 16-byte aligned) from the same pools, sized for the whole task body | 8 KiB + 5 KiB reserve | software probe |
| D | A or B (`--base`) plus `MADV_DONTNEED` of everything below the parked stack pointer minus a 2 KiB switch margin (measured need: 488 bytes); D(B) also decommits a slot on release. D(B) uses the probe as a **watermark**: after a decommit the task's probe limit is raised to the decommit point, a probe that fires above the real limit only marks the task dirty, and parks or releases of clean tasks skip the system call | as base | as base |
| E | no stack: Rust `async` tasks on the same scheduler, polled on the worker's stack | worker stack 8 MiB | the probe compares against the worker thread's stack |

**Probes** (`src/stack/probe.rs`). Task bodies call `SplitStackProbe`, which is
`cmp rsp, fs:[0x70]; jb slow`, the prologue of GCC's `-fsplit-stack`, using the
TCB slot glibc reserves for split stacks. The scheduler writes the running
task's limit there on every resume, so the check is migration-safe; it is zero
(never fires) on guard-page strategies. `LimitProbe` (limit passed as an
argument) and `TlsProbe` (a Rust thread-local) exist only as alternative cost
models in `probe-overhead`. The thread-local model is not migration-safe: LLVM
keeps the thread-local's address in a callee-saved register across calls.
Without guard pages, memory safety on B and C rests on the probe in every
probed function plus a reserve large enough for unprobed callees and the
unwinder. The reserve is 5 KiB by default (the owner's policy; until harness
0.2.0 the defaults were 16 KiB for B and 8 KiB for C). `deep-recursion` measures
the need by painting the reserve: 1.8-2.1 KiB per unwind, with or without drop
code in the deepest frames and with 256-byte frames (`--framed`). The earlier
figure of "5 KiB with drop code" was the first unwind of the process: `libgcc_s`
binds its calls into the C library lazily, and the dynamic linker's resolver
saves the full extended register state on the stack it runs on, a task's
reserve, which is about 3 KiB on an AVX-512 machine. With the 5 KiB default
that overran a slot's reserve in a unit test (the canary caught it), so the
runtime now resolves those bindings once at start-up on an ordinary stack
(`stack::probe::warm_unwinder`). `--canary` checks for a miss after the fact.

**I/O and timers** (`src/rt/net.rs`, `src/rt/timer.rs`, `src/rt/uring.rs`). There
are no helper threads in the scheduling path; the workers do the work, as in Go:

- *Network.* Non-blocking sockets share one epoll instance, and the workers poll
  it. A busy worker polls without blocking every 31 decisions, and an idle
  worker polls before it spins. One sleeping worker blocks in `epoll_pwait2`,
  and other workers wake it through an eventfd.
- *Timers.* Every scheduling decision checks whether a timer is due: one atomic
  load and one clock read. A due timer fires on the worker that noticed it, and
  its task takes that worker's `next` slot. One sleeping worker owns the earliest
  deadline and sleeps only until then.

Two measurements drove this design. With a dedicated poller thread, the echo
p99 was about 2x Go's; with a dedicated timer thread, the starvation probe's
p99 under load was about 0.5 ms (now about 30 us). Both threads competed with
the workers for CPUs.

io_uring for the cancellation test has a ring-owner thread. Its `read` returns
only after the kernel's original completion, even when cancelled, which is the
D-058 rule; the unsafe `start_read` is the negative-control escape hatch.

## Running the benchmarks

Common options: `--strategy A|B|C|D|E|tokio|os-threads` (default B), `--threads N`,
`--tasks N`, `--csv PATH` (append), `--machine-spec` (also record the machine),
`--machine-id ID`, `--reservation 128K|1M|8M` (A, B, D), `--stack-size 2K..16K` (C),
`--probe-reserve`, `--guard`, `--chunk`, `--base A|B` (D), `--stack-cache N` (A),
`--canary`, `--worker-stack`, `--quiet`. Every option is validated, so a typo is an
error. Defaults run on a development machine in seconds; full-scale values are in
`run_all.sh --scale full`. `taskbench help` lists everything.

| Command | Default scale | What it does |
|---|---|---|
| `spawn-join [--rounds 20000 --warmup 2000 --batch 1 --scope-per-round]` | 20k rounds | spawns `batch` trivial tasks in a long-lived scope (or a fresh scope per round) and joins them; per-task latency of each round |
| `idle-memory [--tasks 10000 --excursion 32K --settle-ms 200 --mem-guard 1G --skip-excursion]` | 10k tasks | N tasks do shallow work and park on a gate; memory read; each makes one stack excursion (4000-byte frames, every page touched) and parks again; memory read. Strategy C's exact size grows by the excursion (the compiler would size it for the whole body). When the memory headroom falls below the guard, spawning stops, and tasks that have not made their excursion skip it; the phase is then a `failed` row (the Go and Java controls do the same). `--skip-excursion` keeps the configuration but measures the shallow phase only and reports the excursion cell as `skipped` (policy 7: at 1M tasks the cell is required only of strategies that release pages) |
| `max-tasks [--tasks CAP --mem-guard 1G --hold-secs 0]` | cap 100k (OS threads 10k) | spawn parked tasks until a spawn fails, the cap, or the memory headroom (`MemAvailable`, or the memory cgroup's when smaller) below the guard; reports the limit hit. `--hold-secs 60`: a run that reached the cap keeps every task parked 60 s more, then checks that all are still parked (the pre-registered scale row) |
| `echo [--connections 1000 --msg-bytes 64 --duration 3 --warmup-secs 1]` | 1000 connections | in-process closed-loop clients; connections opened by an OS thread with blocking `connect`; several listening ports above 20k connections |
| `idle-connections [--connections 5000]` | 5000 | server tasks parked in `read`, client ends held as plain sockets |
| `deep-recursion [--parse-depth 10000 --naive-depth 1000000 --framed]` | 10k / 1M | a recursive-descent parse of `((((1+2)+2)...` (4 frames per level, iterative `Drop`) and a naive recursion with a drop guard every 256 frames, each in its own task; `--framed` adds a recursion with a 256-byte local and a drop guard in every frame (the worst frame shape for the reserve); checks the outcome, the mechanism, destructors, reserve use and that the runtime still works |
| `starvation [--tasks 2*threads --yield-every 10000 --duration 2 --probe-interval-us 1000]` | 8 tasks on 4 workers | compute tasks yield every K LCG iterations (about 19 us here; `0` = no yield points); a probe task sleeps 1 ms in a loop and records lateness, alone and under load; stopped by an outside thread |
| `probe-overhead [--reps 11 --workload all --fib-n 32 --calls 20000000 --tree-depth 15 --loop-iters 20000000]` | about 15 s | `fib`, `calls`, `tree`, and `loop` (a call-free loop with the check on every back-edge: the cost model of a compiler-inserted yield check, which policy 8 counts in this budget), each compiled without probes and with the three probe models, run alternately in one task |
| `cancel-uring [--tasks 4096 --concurrency 512 --source pipe|file|file-buffered --pipe-mix race|cancel --buf-bytes 512 --async-percent 25 --break-contract]` | 4096 reads | see `src/bench/cancel_uring.rs`; `pipe` races data against cancels, `--pipe-mix cancel` is amendment A1's registered procedure (no data, every read cancelled in flight; use `--buf-bytes 4096`), `file` cancels O_DIRECT reads in the device (`-ENOENT` with a later completion), `file-buffered` with `IOSQE_ASYNC` provokes `-EALREADY`; reports the cancel-to-return latency |

Examples:

```
taskbench idle-memory --strategy B --reservation 128K --tasks 100000 --csv out.csv --machine-spec
taskbench deep-recursion --strategy C --stack-size 16K
taskbench cancel-uring --strategy B --source file-buffered --async-percent 100
taskbench echo --strategy tokio --connections 1000
```

## Controls

* **Go** (`controls/go`): goroutines and channels, `GOMAXPROCS` = `--threads`. Joins
  are channel receives inside a goroutine. Echo uses `net` with a goroutine per
  connection, a 4 KiB buffer and `TCP_NODELAY` (Go's default). Starvation runs
  with no yields, relying on asynchronous preemption, which is the bet's
  baseline (policy 8; `run_controls.sh` runs only that); `-yield-every K` would
  add `runtime.Gosched`. idle-memory also reports after forced GC, because Go
  shrinks stacks at GC; `-skip-excursion` and `-hold-secs` match the harness's
  options. Its memory guard is
  checked by every 256th goroutine before its excursion, not by a watcher: the
  goroutine that closes the gate is preempted while it readies a million
  goroutines and waits behind all of them in the global run queue, so a watcher
  on it ran only after every excursion (or the OOM kill, as happened here before
  the change).
* **Java** (`controls/java`): JDK 21+ virtual threads, with
  `jdk.virtualThreadScheduler.parallelism` and `maxPoolSize` = `--threads`.
  Waits use `java.util.concurrent`, never `synchronized`, which pins the carrier
  on JDK 21-23. The spawn-join loop runs inside a virtual thread; on a platform
  thread every join is an OS-level wake (38 us measured instead of 1.5 us).
  Warm-up: 20k unrecorded spawn-join rounds and the echo warm-up window. The JVM
  flags are recorded in the `params` column. The memory guard also counts the
  Java heap's headroom (the default maximum heap is a quarter of RAM). Virtual
  threads copy frames into heap stack chunks when they unmount, so they are a
  managed baseline, not a non-moving-stack precedent.
* **tokio** (`--strategy tokio`): the stackless reference, on the same workload
  bodies (the gate and the probes are executor-agnostic).
* **OS threads** (`--strategy os-threads`): one thread per task, for
  `spawn-join`, `idle-memory`, `max-tasks` and `starvation`.
* `controls/run_controls.sh OUT_DIR [--threads N] [--machine-id ID] [--scale smoke|full] [--only BENCH,...] [--build-only|--no-build]`
  builds both controls, writes `controls-versions.txt` (versions, flags, warm-up)
  and appends to `OUT_DIR/<benchmark>.csv`; a control that dies becomes a
  `process_exit` row, and a connection count above the RLIMIT_NOFILE hard limit
  becomes a `failed` row written by the script. `run_all.sh` builds the controls
  once and then runs each benchmark's controls right after the harness's own
  configurations, repetition by repetition. Both scripts take their parameters
  from `scale.sh`.

The Phase 1 plan also lists "a C or Rust stackful coroutine library" as a
control. Strategy A *is* such a library's design (corosensei's `DefaultStack` is
a guarded mmap per coroutine), so A on this scheduler stands in for it; a
separate `may`- or Boost.Context-based control is not implemented.

## CSV schema

One row per measured metric (long format), the same in all three languages:

```
schema,run_id,timestamp_utc,machine_id,implementation,benchmark,strategy,reservation_bytes,stack_bytes,threads,tasks,params,metric,value,unit,status,note
```

| Column | Meaning |
|---|---|
| `schema` | schema version (1) |
| `run_id` | one process run: `<UTC timestamp>-<pid>` |
| `timestamp_utc` | ISO 8601 |
| `machine_id` | `--machine-id`, default the host name |
| `implementation` | `taskbench`, `tokio`, `os-threads`, `go`, `java` (`machine` for specification rows) |
| `benchmark` | the subcommand (`machine-spec` for specification rows) |
| `strategy` | `A`, `B`, `C`, `D(A)`, `D(B)`, `E`, `tokio`, `os-threads`, `go`, `java` |
| `reservation_bytes` | A, B, D: usable stack per task |
| `stack_bytes` | per-task stack size (the exact size for C) |
| `threads`, `tasks` | workers; the scale (tasks, connections, rounds x batch, reads) |
| `params` | the remaining parameters, `key=value;...` (runtime versions and JVM flags for controls) |
| `metric`, `value`, `unit` | the measurement; latency summaries are `<prefix>_{p50,p90,p99,p999,max,mean,samples}` in ns, nearest-rank |
| `status` | `ok`; `failed` (the configuration failed, which is a result, e.g. A at the VMA limit); `skipped` (not runnable here, with the reason) |
| `note` | free text: the limit hit, the exhaustion mechanism, counters |

A process that dies is recorded by `run_all.sh` as a `process_exit` row with
status `failed`; for `deep-recursion` that would be a harness bug.

## Machine specification

`taskbench machine-spec --csv PATH` (or `--machine-spec` on any benchmark) records
uname, OS, CPU model and counts, threads per core and the SMT state, the
hypervisor (CPUID vendor), governor, memory and swap, page size,
`vm.max_map_count`, `vm.overcommit_memory`/`ratio`, `kernel.threads-max`,
`kernel.pid_max`, `kernel.io_uring_disabled`, `fs.nr_open`, `fs.file-max`,
`net.core.somaxconn`, `net.ipv4.ip_local_port_range`, `tcp_max_syn_backlog`,
transparent huge page settings, the Spectre v2 mitigation state, rlimits
(NOFILE, NPROC, STACK, AS, MEMLOCK), cgroup memory and pids limits, and the
harness version. `run_all.sh` also appends to `versions.txt` (rustc, cargo, uname,
the arguments, SHA-256 digests of the harness and control sources and of the
binary, the quiet-gate setting, and the commit on CI) and to `timing.csv` (each
repetition's start and duration), once per invocation; the controls append to
`controls-versions.txt`. Both scripts write a row per benchmark process to
`load.csv` (`load.sh`): its wall-clock and CPU time and the CPU time all other
processes used meanwhile (from `/proc/stat`), so a run on a shared machine shows
what it shared. With `--quiet-below CPUS`, every process first waits until the
other processes use fewer than CPUS CPUs, and is measured again (at most three
attempts, the last kept) when they used more while it ran; `summarize.py` prints
the load per benchmark.

## Results

### Phase 1 (October 2026)

The Phase 1 report is [`docs/report-phase1.md`](docs/report-phase1.md): every
configuration at full scale on both machines, five runs per cell, side by side
(machine 1 in [`results/vm/2026-10-13-phase1/`](results/vm/2026-10-13-phase1/),
machine 2 in
[`results/github-runner/full-38518035636/`](results/github-runner/full-38518035636/)),
the verdict against each pre-registered threshold, and the Q-6 recommendation.
Machine 1 is a 4-vCPU KVM guest (Intel Xeon @ 2.10 GHz, no PMU), 15.7 GiB with a
13.4 GiB memory cgroup, no swap, Linux 6.18, default sysctls, `RLIMIT_NOFILE`
20000; Rust 1.97.0, Go 1.24.7, OpenJDK 21.0.12. It is shared with other
development work, so the groups that measure time ran behind the quiet gate
(`--quiet-below 0.3`; the load is in `load.csv`). Machine 2 is a GitHub-hosted
runner: an Intel Xeon Platinum 8370C Hyper-V guest whose 2 vCPUs are one core's
two hardware threads, 7.75 GiB (`machine-spec.csv`), no swap, Linux
6.17.0-1022-azure, `vm.max_map_count` restored to 65530; Rust 1.99.0, Go
1.24.13, Temurin 21.0.12.1; run by the `harness-full` workflow (run
38518035636) inside a `systemd-run` scope with `MemoryMax=12G` and
`MemorySwapMax=0`, whose `RLIMIT_NOFILE` was 1048576. The 12 GiB cap is above
the machine's memory, so it never bound; the memory guard stopped the 1M-task
`idle-memory` cells of B and D(B) at 1 MiB and 8 MiB, which are failed rows.
Medians of five runs; KiB per task is resident memory plus page tables. The
verdicts are provisional until the pre-registration is frozen.

| Bet-3 row | Machine 1 | Machine 2 | Verdict (provisional) |
|---|---|---|---|
| Memory per idle task within 4x Go (after the excursion: Go after GC), on both machines | D(B) passes at every reservation: worst ratio 1.75 / 2.41 / 3.16 at 128 KiB / 1 MiB / 8 MiB (D(B)/1 MiB: 6.43 KiB shallow and 6.44 KiB after the excursion at 1M tasks, against Go's 2.67 [2.65-2.70] and 5.02 [5.01-5.02] after GC at 100k). B and C fail after the excursion (6.86x, 7.08x); A and D(A) stop at 32,730 tasks | D(B) passes at 128 KiB (worst ratio 1.75); at 1 MiB and 8 MiB the 100k conditions pass (worst 2.41, 3.16) but the memory guard stopped the 1M cells at 950,272-962,560 and 712,704-733,184 tasks, so they are inconclusive. B and C fail after the excursion (6.94x, 7.15x); A and D(A) stop at 32,738 tasks | holds, through D(B) at 128 KiB on both machines; at the 1 MiB default, inconclusive on machine 2 (its 1M cells rest on machine 1) |
| Spawn plus join within 3x Go (pooled stacks) | B 0.64 [0.64-0.80] us, 1.40x Go | B 0.73 [0.52-0.73] us, 1.84x Go | holds |
| 1M idle tasks, default sysctls | B, C, D(B), E, tokio, Go, Java: 1M, still parked after 60 s; A and D(A): 32,730 (`vm.max_map_count`) | the same; A and D(A): 32,738 | fails for guard pages per task, holds for pooled stacks |
| Echo p99 at 10k connections within 2x Go | not run (descriptor limit); at 1000 connections D(B) 0.85x Go | D(B) 0.91x and B 0.91x Go, every stackful strategy at most 1.07x; at 1000 connections D(B) 0.87x | not shown (one machine) |
| Clean "stack exhausted", never a segfault | every case clean, 0 crashes; at most 2,088 bytes of the 5 KiB reserve used | the same | holds (harness part) |
| Starvation: no worse than Go, under 10 ms | B 25.7 [25.0-30.0] us with yield points every 10k iterations; Go 39.3 [39.3-39.4] ms | B 23.6 [23.3-23.7] us; Go 39.2 ms | holds |
| Probe overhead at most 3 % | `split`: fib 13.3 %, calls 0.0 %, tree 4.5 %, back-edge yield check -0.1 % | fib 8.6 %, calls 0.2 %, tree 2.2 %, back-edge yield check 3.5 % | not decided (no PMU; policy 1); at face value `fib` exceeds 3 % on both, `tree` on machine 1 and the back-edge check on machine 2 |
| A1: no buffer written after its cancelled read returned | 0 bytes over 1M cancelled reads per run, 25 runs in all on the five stackful strategies | the same, 25 runs | holds |

Q-6, provisionally: D(B) with a 1 MiB default reservation, software probes
rather than guard pages, the 5 KiB reserve, `spawn` ambient, and no stackless
fallback. The 1 MiB default's 1M-task cells are measured on machine 1 only
(machine 2's 7.75 GiB could not hold them); they should be reproduced on a
second machine with 16 GiB before the default is final.

### Earlier development runs (harness 0.1.0, October 12)

The figures below are the development runs made before the owner's policies,
with harness 0.1.0 (probe reserves of 16 KiB for B and 8 KiB for C, no unwinder
warm-up), on another instance of the development VM (Intel Xeon @ 2.80 GHz). The
Phase 1 report supersedes them; they are kept because earlier documents cite
them.

The tables show one run of each configuration (for the smoke suite, the last of
three runs that day), on the development VM, not on the pre-registered machines: 4 vCPUs (Intel Xeon @ 2.80 GHz, a KVM guest without a PMU), 16 GiB of
RAM (memory cgroup limit 13.4 GiB), no swap, Linux 6.18, default sysctls
(`vm.max_map_count` 65530), `RLIMIT_NOFILE` 20000. Rust 1.97.0, Go 1.24.7,
OpenJDK 21.0.12; 4 workers, `GOMAXPROCS=4`, virtual-thread parallelism 4.
Memory is resident memory plus page tables, per task; KiB = 1024 bytes.

#### Smoke scale

`./run_all.sh` (2 minutes 15 seconds); CSVs in `results/vm/2026-10-12/`.

| | A/1M | B/1M | C/8K | D(A)/1M | D(B)/1M | E | tokio | Go | Java | OS threads |
|---|---|---|---|---|---|---|---|---|---|---|
| spawn+join median, us (1) | 7.52 (0.53 cached) | 0.53 | 0.53 | 7.83 (0.94 cached) | 0.55 | 0.46 | 0.29 | 0.43 | 0.77 | 65 |
| idle task, shallow, KiB (10k tasks; OS threads 2k) | 6.4 | 6.5 | 4.5 | 6.4 | 6.5 | 0.7 | 0.4 | 2.7 | 3.1 | 10.0 |
| same, after a 32 KiB excursion | 34.5 | 34.5 | 32.5 | 6.5 | 6.5 | 0.7 | 0.4 | 34.8 (5.1 after GC) | 4.4 (4.5 after GC) | 42.0 |
| max tasks (cap 100k; OS threads 5k) | 32,730 (vma_count) | 100,000 | 100,000 | 32,730 (vma_count) | 100,000 | 100,000 | 100,000 | 100,000 | 100,000 | 5,000 |
| echo, 1000 connections: p99 ms / requests per s | 7.9 / 190k | 6.9 / 195k | 7.7 / 207k | 34.2 / 41k | 6.2 / 216k | 6.6 / 205k | 11.2 / 242k | 14.6 / 139k | 24.2 / 64k |  |
| idle connection, KiB (5000) | 10.6 | 10.6 | 8.6 | 10.6 | 10.6 | 5.0 | 4.9 | 7.1 | 10.1 |  |
| 1M-deep recursion: depth at the clean failure / drop guards skipped (2) | 21,837 / 86 | 21,836 / 0 | 162 / 0 | 21,837 / 86 | 21,836 / 0 | 173,278 / 0 | 173,273 / 0 |  |  |  |
| starvation probe p99 under load (3) | 27 us | 30 us (2.0 s no yields) | 29 us | 29 us | 28 us | 29 us | 1.0 ms | 4.0 ms (27.2 ms no yields) | 722 us (2.0 s no yields) | 3.0 ms |
| probe overhead, %: fib / calls / tree | 47 / -6 / 3 | 38 / -10 / 2 | 34 / -8 / 3 |  |  | 35 / -6 / 2 |  |  |  |  |
| io_uring: writes into a buffer after its cancelled read returned | 0 | 0 (163840 with --break-contract) | 0 | 0 | 0 |  |  |  |  |  |

(1) A and D(A) map and unmap a stack per task; "cached" is `--stack-cache 1024`.
(2) 1 MiB stacks (C: 8 KiB; E and tokio: the 8 MiB worker stack). A and D(A)
discard a trapped task's frames without running their destructors.
(3) The compute tasks yield every 10k iterations, about 19 us (Go with
`runtime.Gosched`, Java with `Thread.yield`); "no yields" has none, so Go relies
on asynchronous preemption.

Reading notes:

* Run to run (three runs of the suite on the same day), spawn-plus-join medians
  moved by under 5 %, except Java (0.77-12 us) and C (0.53-0.72 us). The echo p99
  ranged 5.7-11.2 ms for B and 7.7-14.6 ms for Go. Go's starvation p99 ranged
  0.06-4.0 ms with `Gosched` and 27-39 ms without yields; taskbench's stayed at
  22-32 us. Probe overhead: `fib` 21-58 %, `calls` -10 to +3 %, `tree` 0-7 %.
* D(A) decommits with `madvise` at every park, so every echo request pays a
  system call and refaults (p99 2.3-3.9x Go's). D(B)'s watermark skips both, so
  D(B) costs what B costs.
* tokio's 1 ms starvation p99 is its timer granularity, not starvation.
* Java's idle memory is low because a parked virtual thread's frames are copied
  into a heap object sized to the frames, not to the stack it had.

#### idle-memory and max-tasks at full scale

`./run_all.sh --scale full --only idle-memory,max-tasks` (6.5 minutes, one run);
CSVs in `results/vm/2026-10-12-large/`. This VM has the 16 GiB of the scale row.

| | A/1M | B/1M | C/8K | D(B)/1M | E | tokio | Go | Java |
|---|---|---|---|---|---|---|---|---|
| idle task, 100k tasks, shallow (KiB) | VMA limit at 32,730 tasks | 6.5 | 4.5 | 6.5 | 0.6 | 0.4 | 2.7 | 1.7 |
| same, after a 32 KiB excursion | VMA limit | 34.5 | 32.5 | 6.5 | 0.6 | 0.4 | 34.7 (5.0 after GC) | 3.6 (3.5 after GC) |
| idle task, 1M tasks, shallow (KiB) | VMA limit | 6.5 | 4.5 | 6.5 | 0.6 | 0.4 | 2.7 | 1.0 |
| same, after a 32 KiB excursion | VMA limit | guard stop after 21 % of the excursions | guard stop after 28 % | 6.5 | 0.6 | 0.4 | guard stop after 30 % | 2.1 (2.1 after GC) |
| `max-tasks`, cap 1M | 32,730 (`vma_count`) | 1,000,000 | 1,000,000 | 1,000,000 | 1,000,000 | 1,000,000 | 1,000,000 | 1,000,000 |
| KiB per task at the stop | 6.4 | 6.5 | 4.4 | 6.5 | 0.6 | 0.4 | 2.6 | 1.0 |

* Per-task memory does not change from 10k to 1M tasks for the stackful
  strategies, so the smoke-scale ratios to Go carry over.
* "Guard stop" is the memory guard at work (1 GiB of headroom left): B/8M stopped
  after 14 % of the excursions, B/128K after 27 %, C after 28 % at every size. D(A)
  stops at 32,730 tasks like A; OS threads stop at 16,128 (four mappings per Rust
  thread: stack, guard, and the signal stack std maps with its own guard).
* A run of B at 1M tasks with 8 MiB reservations reserves 8 TiB of address space
  and still needs only 8.4 KiB per task: page tables, not reservations, are the
  cost of a large reservation (4 KiB per task at 8 MiB, 2 KiB at 1 MiB).

#### Against the thresholds, on this VM (development runs)

| Bet-3 row (threshold) | Measured here | Verdict here |
|---|---|---|
| Idle memory after shallow use, within 4x Go | at 10k, 100k and 1M tasks: B/1M 2.4x Go, B/8M 3.2x, B/128K 1.8x, C 1.7x, D(B)/1M 2.4x, E 0.24x | holds for B, C and D(B) at every reservation; A fails on scale first |
| Idle memory after a 32 KiB excursion, within 4x Go | 10k and 100k: A, B, C 0.94-1.05x Go measured right after the excursion, 6.4-7.3x Go after its GC; D(B)/1M 1.3x Go after GC (D(B)/8M 1.7x). 1M: D(B) 6.5 KiB per task; B, C and Go stopped by the memory guard | against Go after GC, only decommit (D) holds; at 1M on 16 GiB only D completes the cell (question 7) |
| Spawn+join median within 3x Go, pooled stacks | over three runs: B 1.15-1.22x Go (527 ns against 431 ns in the last), C 1.15-1.65x, D(B) 1.20-1.27x, E 1.01-1.07x | holds |
| 1M idle tasks on 16 GiB with default sysctls | B, C, D(B), E, tokio, Go and Java reach the 1M cap; A and D(A) stop at 32,730 (`vm.max_map_count`) | holds for B, C and D(B); fails for A, as predicted: guard pages per task are out |
| 10k-connection echo p99 within 2x Go | 1000 connections only: B 0.47-1.35x Go over three runs, D(B) 0.42-0.94x, D(A) 2.3-3.9x | holds at 1000 connections except D(A); 10k not run |
| Clean "stack exhausted", never a segfault | the 1M-deep recursion fails cleanly at the end of every stack (and so does the 10k-deep parse wherever the stack is too small); the runtime stays healthy after; no crash in the final runs | holds; A and D(A) skip the destructors of the discarded frames |
| Probe p99 no worse than Go, under 10 ms | 22-32 us with a yield point every 10k iterations, against Go's 0.06-4.0 ms with `Gosched` and 27-39 ms with preemption only; with no yield points, 2 s | holds if the compiler inserts yield points; without them it fails (question 8) |
| Probe overhead at most 3 % | `calls` -10 to +3 %, `tree` 0-7 %, `fib` 21-58 % (an upper bound) | not shown: the noise exceeds the threshold and there is no PMU |
| A1: no write into a buffer after its cancelled read returned | 0 on A, B, C, D(A) and D(B), for pipe, O_DIRECT and buffered-file reads (32 `-EALREADY` races); the negative control sees 163,840 | holds |

## Not tested here, and why

* **Machine 2's 1M-task cells at 1 MiB and 8 MiB.** Machine 2 is the 2-vCPU,
  8 GB `ubuntu-latest` runner of a private repository (7.75 GiB in
  `machine-spec.csv`), not the 16 GB machine of proposal/20, section 4.4. Its
  memory guard stopped the 1M-task `idle-memory` cells of B and D(B) at 1 MiB
  and 8 MiB (failed rows), so those cells, D(B)'s at the recommended default
  among them, are measured on machine 1 only. The workflow's 12 GiB memory scope
  is above that runner's memory and never bound. A 16 GB runner (`ubuntu-latest`
  of a public repository, or a larger runner) would hold them.
* **10k-connection echo and 100k idle connections on machine 1.** Both ends live
  in one process, which needs `RLIMIT_NOFILE >= 2N + 64`. The hard limit here is
  20000 and cannot be raised (policy 3), so the 10k echo is a `failed` row and
  idle connections run at 9,900, the most the limit allows. Machine 2's
  processes had a hard limit of 1048576 inside the workflow's scope (the
  runner's own default is 65536) and ran both cells, except idle connections
  for A and D(A), whose server tasks meet the VMA limit. The echo is a closed
  loop with in-process clients, not the open-loop generator on disjoint cores
  of proposal/20, section 6.5.
* **1M tasks after the 32 KiB excursion, for designs that keep the pages.** A, B, C
  and Go would need about 33 GiB; policy 7 requires the cell only of strategies
  that release pages, so the others run `--skip-excursion` at 1M and the cell is
  `skipped` with the reason.
* **Probe overhead within 3 %.** Neither machine exposes a hardware PMU, and
  wall-clock medians move by more than the threshold between processes and
  between builds (`fib` measured 21-58 % in the development builds; the Phase 1
  figures are in section 7 of the report; `fib` is an upper bound, see
  `src/bench/probe_overhead.rs`). Policy 1 defers the decision to the native era
  on a machine with pinned frequency and `perf_event` hardware counters.
* **Go "latest stable".** The machines have Go 1.24 (1.24.7 on machine 1,
  1.24.13 on machine 2); the plan asks for the latest stable release. JDK is
  21.0.12 on both (virtual threads final; JEP 491 unpinning is 24+).

## Not implemented

* A separate C or Rust stackful-coroutine library control (see Controls).
* Fault-path unwinding for guard-page strategies: A and D(A) fail an exhausted
  task cleanly but skip the destructors of its discarded frames. A trapped task
  whose open scopes still have children keeps its stack quarantined until they
  finish, so no child can see freed memory.
* `deep-recursion` for OS threads (Rust aborts the process on a thread stack
  overflow by design).
* Echo with the client in a separate process (which would allow 10k connections
  here).
* Production concerns: no `@blocking` FFI thread hand-off, no stack shrinking for
  B beyond D, no NUMA awareness.

## Unsafe code

`unsafe` is confined to `src/stack` (mmap, mprotect, madvise; the `corosensei::Stack`
implementation; the SIGSEGV handler; the probe's `asm!`), `src/rt` (lifetime
erasure of scoped task closures, guarded by the scope's join; raw pointers from
tasks to the runtime, guarded by the runtime outliving its tasks; the per-task
fields owned by whoever holds the RUNNING state; epoll and io_uring system calls),
the benchmark code that lends stack buffers to the kernel (`cancel_uring.rs`) or
paints stack reserves (`deep_recursion.rs`), and `perf.rs`.
Each block carries a `SAFETY:` comment with its invariant (`clippy::undocumented_unsafe_blocks`
is on). Edition 2024's `unsafe_op_in_unsafe_fn` is denied.

## Policy decisions (owner, October 2026)

The questions the first version of this harness raised were answered by
the project owner. They bind every later run and the Phase 1 report.

1. **Machines.** Virtual machines and GitHub-hosted runners are enough in
   the harness era; bare metal with hardware performance counters and a
   pinned frequency waits for the native era. The second machine of the
   plan may be a runner. Continuous integration runs the smoke suite on
   every push ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml));
   no scheduled runs are configured.
2. **Context-switch crate.** corosensei 0.3.x is the standard; a
   replacement must keep a trap-return path for guard pages and a
   parent-stack escape for runtime code.
3. **Sysctls and limits.** Default sysctls only. A deployment requirement
   to raise `vm.max_map_count` is not acceptable, so strategy A (one
   reservation per task) is out as a default once `max-tasks` shows the
   VMA limit; the same holds for `RLIMIT_NOFILE`, so the 10k-connection
   echo and the 100k idle-connection cells run only where the default hard
   limit allows and are otherwise reported as not run.
4. **Go baseline after the excursion.** The tighter figure: Go measured
   after its garbage collection has shrunk the stacks. On this machine
   only decommit-on-park (D) holds the 4x bar against it.
5. **Decommit policy.** Pooled stacks with software probes and decommit on
   park, D(B), is the default strategy; runtime code called from tasks runs
   on the worker's stack, as Go's `systemstack` does, so unprobed code never
   crosses the watermark or the reserve.
6. **Probe reserve.** 5 KiB, the measured need with drop code; it is also
   the floor for strategy C's slot size.
7. **The 1M-task cell after the excursion.** The cell is required only of
   strategies that release pages on park; for the others it is reported at
   100k and marked infeasible at 1M on a 16 GiB machine, together with Go,
   which shrinks stacks only at garbage collection.
8. **Yield points.** The compiler inserts yield checks (Q-34, decided), and
   their cost counts in the probe-overhead budget; the starvation baseline
   is Go with its default asynchronous preemption and no manual `Gosched`.
   A Sisu task without yield points is expected to starve the probe, and
   that case is reported, not counted.

The pre-registration ([`../proposal/20-pre-registration.md`](../proposal/20-pre-registration.md),
section 4.4 and section 6.5) carries the same decisions.
