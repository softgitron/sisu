# Phase 1 report: the bet-3 stack harness

*Template. Every cell is filled from the committed CSVs
(`results/<machine>/<date>/*.csv`), for both machines, for every configuration,
including the ones that fail. A configuration that fails is a result, not an
omission. Era label for every number: **harness** (Rust code on native stacks).*

## 0. Provenance

Policy of record: the owner's answers in the README's "Policy decisions"
section (machines may be virtual or GitHub-hosted runners; default
sysctls; Go after its garbage collection as the memory baseline; D(B) as
the default strategy; a 5 KiB probe reserve; the 1M-after-excursion cell
required only of page-releasing strategies; compiler-inserted yield
points with their cost in the probe budget).

| | Machine 1 (owner) | Machine 2 (reviewer's reproduction) |
|---|---|---|
| machine_id | | |
| CPU model, cores used (`--threads`) | | |
| memory, swap | | |
| kernel (`uname`) | | |
| `vm.max_map_count`, `kernel.threads-max`, `kernel.pid_max` | | |
| `vm.overcommit_memory`, THP `enabled`/`defrag` | | |
| `RLIMIT_NOFILE` soft/hard, `RLIMIT_NPROC` | | |
| rustc, taskbench version | | |
| Go version, `GOMAXPROCS` | | |
| JDK version, flags, warm-up | | |
| results directory and run ids | | |

Pre-registration commit: `__________` (thresholds below are copied from it;
any amendment is listed in section 10).

## 1. Memory per idle task (threshold: within 4x Go at 100k and 1M)

Bytes per task = (VmRSS + VmPTE after - before) / N, from `idle-memory`.
Ratio = taskbench / Go. For Go, "excursion" is read immediately after the
excursion and again after GC (Go shrinks stacks at GC). The pre-registration
fixes which Go figure is the baseline.

| Configuration | N | shallow M1 | shallow M2 | after 32 KiB excursion M1 | M2 | ratio to Go (shallow / excursion) | verdict |
|---|---|---|---|---|---|---|---|
| A, 128 KiB | 100k | | | | | | |
| A, 1 MiB | 100k | | | | | | |
| A, 8 MiB | 100k | | | | | | |
| B, 128 KiB | 100k | | | | | | |
| B, 1 MiB | 100k | | | | | | |
| B, 8 MiB | 100k | | | | | | |
| C, 4/8/16 KiB (+excursion) | 100k | | | | | | |
| D(A), 128 KiB / 1 MiB / 8 MiB | 100k | | | | | | |
| D(B), 128 KiB / 1 MiB / 8 MiB | 100k | | | | | | |
| E (stackless) | 100k | | | | | | |
| tokio | 100k | | | | | | |
| Go (no GC / after GC) | 100k | | | | | 1.0 | |
| Java virtual threads (RSS / heap used) | 100k | | | | | | |
| *(repeat every row at 1M)* | 1M | | | | | | |

Also report per row: page-table bytes per task (`pte_per_task_*`), VMAs, and
for D the decommit count and bytes (CSV note).

At 1M tasks, a design that keeps the excursion's pages needs about 32 GiB after
the excursion (A, B, C, and Go until its GC shrinks stacks). On a 16 GiB machine
the memory guard (`--mem-guard`) stops that phase, and the row is `failed` with
the number of tasks that made the excursion; report it as such, not as missing.

## 2. Spawn plus join (threshold: median within 3x Go with pooled stacks)

| Configuration | median M1 | p99 M1 | median M2 | p99 M2 | median / Go | verdict |
|---|---|---|---|---|---|---|
| A (no cache) | | | | | | |
| A (`--stack-cache 1024`) | | | | | | |
| **B** | | | | | | |
| C | | | | | | |
| D(A), D(B) | | | | | | |
| E | | | | | | |
| tokio | | | | | | |
| OS threads | | | | | | |
| B, batch 100 (per task) | | | | | | |
| B, scope per round | | | | | | |
| Go | | | | | 1.0 | |
| Java virtual threads | | | | | | |

## 3. Scale: 1M idle tasks on a 16 GiB machine with default sysctls

From `max-tasks --tasks 1000000` (and `idle-memory --tasks 1000000`).

| Configuration | tasks created M1 | limit M1 | created M2 | limit M2 | RSS+PTE at stop | verdict |
|---|---|---|---|---|---|---|
| A (guard page per task) | | | | | | expected: `vma_count` near 32k |
| B, C, D(B) | | | | | | |
| D(A) | | | | | | |
| E, tokio | | | | | | |
| OS threads | | | | | | |
| Go, Java | | | | | | |

If A fails: "guard-page-per-task is out; pooled stacks with software probes
become the design, with probe overhead on compute loops at most 3%" (section 7).

## 4. Echo (threshold: 10k-connection p99 within 2x Go) and idle connections

| Configuration | p50 | p99 | p99.9 | req/s | p99 / Go | idle RSS+PTE per connection (100k) |
|---|---|---|---|---|---|---|
| A, B, C, D, E (one row each, M1 and M2) | | | | | | |
| tokio | | | | | | |
| Go | | | | | 1.0 | |
| Java virtual threads | | | | | | |

State the I/O design next to any miss (taskbench: the workers poll one shared
epoll instance, busy workers every 31 scheduling decisions and idle ones before
they spin, and one sleeping worker blocks in it; Go: the integrated netpoller).
A miss is "runtime design review, not a language change".

## 5. Stack exhaustion (threshold: no segfault, ever)

| Configuration | 10k-deep parse: outcome, depth reached | 1M-deep recursion: outcome, depth | mechanism | destructors ran (`leaked_*` = 0) | probe reserve used | runtime healthy after |
|---|---|---|---|---|---|---|
| A 128 KiB / 1 MiB / 8 MiB | | | guard page | expected no | n/a | |
| B 128 KiB / 1 MiB / 8 MiB | | | software probe | | | |
| C 2/8/16 KiB | | | software probe | | | |
| D(A), D(B) | | | | | | |
| E (sync and async recursion) | | | worker-stack probe | | n/a | |
| tokio | | | worker-stack probe | | n/a | |

Count of `process_exit` rows (crashes) across all CSVs: **must be 0**.
The corpus-wide `--stats stack-depth` histogram (interpreter era) is reported
separately and decides whether the default reservation suffices.

## 6. Starvation (threshold: probe p99 no worse than Go and under 10 ms)

| Configuration | yield every | slice between yields | probe p50 / p99 / max, loaded | baseline p99 (alone) | vs Go | verdict |
|---|---|---|---|---|---|---|
| A-E, tokio | 10000 iterations | | | | | |
| B | 1000 | | | | | |
| B | none | | | | | expected: starves |
| OS threads | n/a (kernel preemption) | | | | | |
| Go (asynchronous preemption) | none | | | | 1.0 | |
| Go | 10000 (`Gosched`) | | | | | |
| Java (no preemption) | none / 10000 (`Thread.yield`) | | | | | |

## 7. Software probe overhead (threshold: at most 3% on compute loops)

| Workload | split (`cmp rsp, fs:[0x70]`) M1 | M2 | arg model | tls model | instructions (if counters available) |
|---|---|---|---|---|---|
| fib (call-dominated worst case) | | | | | |
| calls (loop calling a small leaf) | | | | | |
| tree (recursive walk with work) | | | | | |

Note whether the full-run machines had hardware counters; on a VM without a
PMU only wall-clock medians are available and the noise floor must be stated.

## 8. io_uring cancellation into stack buffers (amendment A1, D-058)

| Configuration | source | reads | data / cancelled | cancel: ok / -ENOENT / -EALREADY | completed after failed cancel | writes after return (must be 0) |
|---|---|---|---|---|---|---|
| A, B, C, D | pipe | | | | | |
| B | file (O_DIRECT) | | | | | |
| B | file-buffered (IOSQE_ASYNC) | | | | | |
| B, `--break-contract` | pipe | | | | | must be > 0 (negative control) |

## 9. Verdict and Q-6 recommendation

For each bet-3 row: **holds**, **fails** or **not shown**, with the numbers.

* Strategy: ______ ; default reservation: ______ ; probes or guard pages: ______
* Commit creep: is decommit-on-park (D) needed, and at what park frequency does
  its system call cost show in spawn-join and echo?
* `spawn` ambient or capability-gated: ______ (no supply-chain attack in the
  research needed to spawn)
* If memory or exhaustion fails: the stackless fallback is declared, its cost
  from `docs/stackless-fallback-estimate.md`, and the yield-point frame
  boundary question is flagged to Phase 3 before the checker design freezes.

## 10. Deviations, amendments, gaps

Every deviation from the pre-registration, every configuration that could not
be run (with the reason), and anything measured on one machine only.
