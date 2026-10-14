# Phase 1 report: the bet-3 stack harness

*Era label for every number in this report: **harness** (Rust code on native
stacks, standing in for compiled Sisu). Filled from
[`report-template.md`](report-template.md). Every number comes from the CSVs in
the two results directories of section 0; `summarize.py --markdown` on those
directories prints the tables' values (medians and ranges over runs, ratios,
bootstrap intervals and section 2.4's rule), and the few figures taken from a
row's `note` column (decommit counts, limits) say so where they are used. The
report itself is rendered from
[`report/report-phase1.md.in`](../report/report-phase1.md.in) by
`report/render.py`, which computes every figure in it from those CSVs. A cell
that was not run says so, with the reason. KiB = 1024 bytes; "M1" is machine 1,
"M2" machine 2.*

**Status of these verdicts.** The pre-registration
([`proposal/20`](../../proposal/20-pre-registration.md)) is a draft (version
1.0, "not frozen"), and its early harness protocol has no frozen commit either.
Its section 1.3, rule 3, a proposed default, would make harness runs made before
the freeze development runs. The verdicts below apply the draft's thresholds and
rules as they stand at commit `e93cd8a`. They are provisional until the freeze
and until machine 2 has run at full scale.

## Summary

| Bet-3 row (role in proposal/20, 7.8) | Threshold | M1: this VM, full scale, five runs per cell | M2: GitHub-hosted runner | Verdict |
|---|---|---|---|---|
| Memory per idle task (primary) | every one of the four conditions within 4x Go; after the excursion, Go after its GC (policy 4) | D(B) passes at all three reservations: worst ratio w = 1.75 (128 KiB), 2.41 (1 MiB), 3.16 (8 MiB). B and C fail after the excursion (6.86x and 7.08x Go after GC at 100k); A and D(A) cannot create 100k tasks (VMA limit at 32,730) | smoke scale only (10k tasks, harness 0.1.0, one run): D(B)/1 MiB 2.41x shallow, 1.31x after the excursion; no cell at 100k or 1M | **not shown**: section 2.4 needs both machines, and M2 has no cell at the threshold's N. On M1 alone D(B) passes with a large margin |
| Stack exhaustion (guard) | no segfault; ordinary corpus depth within the default reservation | every exhaustion clean in 17 configurations x 5 runs; 0 crashes | 0 unclean outcomes, 0 crashes (smoke) | harness part **holds**; the guard itself is decided by the corpus histogram (interpreter era, week 40) |
| Spawn plus join (secondary) | median within 3x Go with pooled stacks | B 1.40x Go (0.64 [0.64-0.80] us against 0.46 [0.41-0.55] us) | B 1.19x (smoke, 20k pairs) | **holds** on M1, and M2's smoke run agrees; no design review |
| Scale, 1M idle tasks on 16 GiB, default sysctls (secondary) | fail: guard-page-per-task is out | B, C, D(B), E, tokio, Go and Java park 1M tasks and keep them parked 60 s; A and D(A) stop at 32,730 (`vm.max_map_count` 65530) | 100k cap only (smoke), under the image's `vm.max_map_count` 262144 | **fails for guard pages per task** (A, D(A)): pooled stacks with software probes are the design, as policy 5 already says; holds for B, C and D(B) |
| Probe overhead on compute loops (with the scale row) | at most 3 % | `split` model: fib 13.3 [11.9-16.3] %, calls 0.0 [-0.1-0.9] %, tree 4.5 [1.9-5.3] %, back-edge yield check -0.1 [-0.2-0.5] % (B, five runs) | fib 30.4 %, calls 0.4 %, tree 25.0 % (smoke) | **not decided** here (policy 1: no PMU, wall-clock noise); taken at face value `fib` and `tree` exceed 3 %, a warning for the native era (section 7) |
| Echo p99 at 10k connections (secondary) | within 2x Go | not run: both ends in one process need 20,064 descriptors, the hard limit is 20000. At 1000 connections D(B) 0.85x and B 0.88x Go | 1000 connections (smoke, 2 workers): D(B) 0.84x | **not shown** (not run at 10k on either machine yet) |
| Idle RSS at 100k connections | reported | at 9,900 connections: D(B) 10.6 KiB against Go 7.06 [7.02-7.16] KiB | 5000 connections (smoke) | reported (no threshold) |
| Starvation (secondary) | no worse than Go (default preemption) and p99 under 10 ms | yield points every 10k iterations: B p99 25.7 [25.0-30.0] us; every 100k: 229 [221-269] us; Go 39.3 [39.3-39.4] ms | B 25.8 us, Go 39.3 ms (smoke) | **holds** with compiler-inserted yield points; without them the probe starves (reported, not counted) |
| Cancellation with a lent buffer, A1 (secondary) | 0 buffers written after their cancelled read returned | 0 bytes over 25 runs of 1M reads each, every read cancelled in flight, on A, B, C, D(A) and D(B), and 0 in the race mix; the negative control sees 195,718,784 bytes | 0 bytes (smoke: 4096 reads per strategy, the race mix) | **holds** on M1; M2 at 1M pending |

**Q-6, provisional (harness era).** Strategy D(B), pooled stacks with software
probes and decommit on park; default reservation 1 MiB; probes, not guard pages;
`spawn` ambient, without a capability; the stackless fallback is not needed.
Section 9 gives the reasons and the conditions.

## 0. Provenance

Policy of record: the owner's answers in the README's "Policy decisions" section
(machines may be virtual or GitHub-hosted runners; default sysctls; Go after its
garbage collection as the memory baseline; D(B) as the default strategy; a 5 KiB
probe reserve; the 1M-after-excursion cell required only of page-releasing
strategies; compiler-inserted yield points with their cost in the probe budget).
Section 10.1 lists what they changed in the plan.

| | Machine 1 (owner) | Machine 2 (reproduction) |
|---|---|---|
| machine_id | `vm` | `github-runner` (host `runnervmmprz5`) |
| CPU model, cores used (`--threads`) | Intel Xeon Processor @ 2.10 GHz (family 6, model 207), KVM guest (CPUID `KVMKVMKVM`), 4 vCPUs, 1 thread per core, no SMT; `--threads 4` | AMD EPYC 7763, 2 vCPUs (an Azure VM; SMT and hypervisor fields not recorded by harness 0.1.0); `--threads 2` |
| memory, swap | 16,874,930,176 bytes (15.7 GiB); memory cgroup limit 14,343,688,192 bytes (13.4 GiB); no swap | 8,323,969,024 bytes (7.75 GiB); no memory cgroup limit; 3 GiB swap; pids cgroup 9490 |
| kernel (`uname`) | Linux 6.18.44-fc-v114, Ubuntu 24.04.5 userland | Linux 6.17.0-1022-azure, Ubuntu 24.04.5 |
| `vm.max_map_count`, `kernel.threads-max`, `kernel.pid_max` | 65530 (kernel default), 128599, 32768 | 262144 (set by the runner image), 63267, 4194304 |
| `vm.overcommit_memory`, THP `enabled`/`defrag` | 0, `madvise`/`madvise` | 0, `always`/`madvise` |
| `RLIMIT_NOFILE` soft/hard, `RLIMIT_NPROC` | 20000/20000, 64299 | 65536/65536, 31633 |
| io_uring | enabled (`kernel.io_uring_disabled` 0) | enabled |
| hardware counters (PMU) | none: "hardware counters unavailable: No such file or directory (os error 2)" | none: "hardware counters unavailable: Permission denied (os error 13)" |
| rustc, taskbench version | rustc 1.97.0; taskbench 0.2.0, sources sha256 `789d610d...`, `51e42157...` and `6a1fedf5...`, the last differing from the second only in the scripts (`versions.txt`, section 10.3) | rustc 1.99.0; taskbench 0.1.0 at commit `b5349dd405e8` |
| Go version, `GOMAXPROCS` | go1.24.7, 4 | go1.24.13, 2 |
| JDK version, flags, warm-up | OpenJDK 21.0.12.1 (Ubuntu build); `-Djdk.virtualThreadScheduler.parallelism=4 -Djdk.virtualThreadScheduler.maxPoolSize=4 -XX:+UseG1GC`; spawn-join 20,000 unrecorded rounds, echo 5 s unrecorded (`controls-versions.txt`) | JDK 17.0.20.1 on the runner image, which has no virtual threads: the Java control did not compile and M2 has no Java rows |
| results directory and run ids | [`results/vm/2026-10-13-phase1/`](../results/vm/2026-10-13-phase1/): 730 processes (run ids in every row), five per cell; `timing.csv` has each repetition's start and duration, `load.csv` the machine's load during every process of the gated groups (section 11); superseded and aborted runs are in subdirectories, each with a note | [`results/github-runner/2026-10-13-smoke-b5349dd405e8/`](../results/github-runner/2026-10-13-smoke-b5349dd405e8/): the `ci` workflow's artifact of run 38416339473, unmodified (98 processes, one per cell); `artifact-provenance.txt` has the artifact id and digest |

Harness commit: the runs used the working tree on top of commit `e93cd8a`, with
the changes listed in section 10.3; the coordinator commits them with these
results, and `versions.txt` carries the digest of the sources the binary and the
controls were built from: `789d610d...` for deep-recursion and idle-memory,
`51e42157...` (the same sources plus the fix of A1's check, section 10.3) for
max-tasks and cancel-uring, and `6a1fedf5...` (the same harness with the
machine-load record and the quiet gate of section 11, which change only the
scripts) for spawn-join, starvation, probe-overhead, echo and idle-connections.
M2's smoke run is from commit `b5349dd405e8`, harness 0.1.0, whose probe
reserves were 16 KiB (B) and 8 KiB (C) and which did not yet resolve the
unwinder's bindings at start-up (section 5).

Pre-registration commit: none yet. Thresholds are copied from proposal/20 at
commit `e93cd8a` (draft 1.0, not frozen); there are no amendments beyond A1-A6,
which are in that text.

Every table can be regenerated with `python3 summarize.py --markdown
results/vm/2026-10-13-phase1
results/github-runner/2026-10-13-smoke-b5349dd405e8` (medians [min-max] over
runs, ratios of medians, 95 % percentile bootstrap intervals over runs, and
section 2.4's rule).

## 1. Memory per idle task (threshold: within 4x Go at 100k and 1M)

Bytes per task = (VmRSS + VmPTE after - before) / N from `/proc/self/status`,
read one second after all N tasks have parked (`--settle-ms 1000`). Shallow use
is about 1 KiB of frames; the excursion is a 32 KiB recursion of 4000-byte
frames that touches every page, after which the task parks again. For Go the
after-excursion figure of record is read after four `runtime.GC()` calls and
`debug.FreeOSMemory()` (policy 4); Java's after `System.gc()`. C's exact size is
its nominal size plus the excursion, as a compiler bounding the whole task body
would choose (4 KiB + 32 KiB = 36 KiB, and so on). Ratio = median over Go's
median on the same machine; ranges in brackets are the five runs' minimum and
maximum, and for ratios the conservative [min/max, max/min] that section 2.4
uses. M2's columns are its smoke run (10k tasks, one run, harness 0.1.0), which
is not at the threshold's N.

*100,000 tasks (KiB per task)*

| Configuration | M1 shallow | M1 after 32 KiB excursion | M1 page tables (shallow) | M1 shallow / Go | M1 excursion / Go after GC | M2 shallow (10k tasks, smoke) | M2 after the excursion (10k, smoke) | M2 ratios to Go, shallow / excursion (10k) |
|---|---|---|---|---|---|---|---|---|
| A, 128 KiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  | 4.68 | 32.7 | 1.72 / 6.49 |
| A, 1 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  | 6.43 | 34.5 | 2.36 / 6.83 |
| A, 8 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  | 8.46 | 36.5 | 3.10 / 7.25 |
| B, 128 KiB | 4.68 [4.68-4.68] | 32.7 [32.7-32.7] | 0.27 [0.27-0.27] | 1.73 [1.73-1.75] | 6.51 [6.51-6.53] | 4.79 | 32.8 | 1.76 / 6.50 |
| B, 1 MiB | 6.44 [6.44-6.44] | 34.4 [34.4-34.4] | 2.02 [2.02-2.02] | 2.38 [2.38-2.40] | 6.86 [6.86-6.88] | 6.55 | 34.6 | 2.41 / 6.85 |
| B, 8 MiB | 8.45 [8.45-8.45] | 36.5 [36.5-36.5] | 4.03 [4.03-4.03] | 3.13 [3.13-3.15] | 7.28 [7.28-7.30] | 8.55 | 36.6 | 3.14 / 7.26 |
| C, 4 KiB (+32 KiB: 36 KiB) | 5.50 [5.50-5.50] | 35.5 [35.5-35.5] | 0.08 [0.08-0.08] | 2.04 [2.03-2.05] | 7.08 [7.08-7.09] | 4.59 | 32.6 | 1.68 / 6.46 |
| C, 8 KiB (+32 KiB: 40 KiB) | 5.50 [5.50-5.50] | 35.5 [35.5-35.5] | 0.09 [0.09-0.09] | 2.04 [2.04-2.06] | 7.08 [7.08-7.09] | 4.60 | 32.6 | 1.69 / 6.47 |
| C, 16 KiB (+32 KiB: 48 KiB) | 5.52 [5.52-5.52] | 35.5 [35.5-35.5] | 0.10 [0.10-0.10] | 2.04 [2.04-2.06] | 7.08 [7.08-7.09] | 4.63 | 32.6 | 1.70 / 6.47 |
| D(A), 128 KiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  | 4.68 | 4.71 | 1.72 / 0.93 |
| D(A), 1 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  | 6.44 | 6.46 | 2.36 / 1.28 |
| D(A), 8 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  | 8.45 | 8.48 | 3.10 / 1.68 |
| D(B), 128 KiB | 4.68 [4.68-4.68] | 4.69 [4.69-4.69] | 0.27 [0.27-0.27] | 1.73 [1.73-1.75] | 0.93 [0.93-0.94] | 4.80 | 4.82 | 1.76 / 0.95 |
| D(B), 1 MiB | 6.44 [6.44-6.44] | 6.44 [6.44-6.44] | 2.02 [2.02-2.02] | 2.38 [2.38-2.40] | 1.28 [1.28-1.29] | 6.56 | 6.59 | 2.41 / 1.31 |
| D(B), 8 MiB | 8.45 [8.45-8.45] | 8.45 [8.45-8.46] | 4.03 [4.03-4.03] | 3.13 [3.13-3.15] | 1.68 [1.68-1.69] | 8.53 | 8.55 | 3.13 / 1.70 |
| E (stackless) | 0.64 [0.64-0.64] | 0.64 [0.64-0.65] | 0.00 [0.00-0.00] | 0.24 | 0.13 | 0.66 | 0.67 | 0.24 / 0.13 |
| tokio | 0.40 [0.40-0.40] | 0.40 [0.40-0.41] | 0.00 [0.00-0.00] | 0.15 | 0.08 | 0.43 | 0.44 | 0.16 / 0.09 |
| Go | 2.70 [2.68-2.70] | 34.7 [33.4-34.7]; after GC 5.02 [5.01-5.02] | 0.01 [0.00-0.01] | 1.00 [0.99-1.01] | 6.92 [6.66-6.93] | 2.72 | 66.4; after GC 5.04 | 1.00 / 13.16 |
| Java virtual threads | 1.93 [1.44-1.94] | 3.16 [3.12-3.54]; heap 1.73 [1.16-1.82] | 0.00 [0.00-0.00] | 0.72 [0.53-0.72] | 0.63 [0.62-0.71] | not run | not run |  |

*1,000,000 tasks (KiB per task)*

| Configuration | M1 shallow | M1 after 32 KiB excursion | M1 page tables (shallow) | M1 shallow / Go | M1 excursion / Go after GC |
|---|---|---|---|---|---|
| A, 128 KiB | fails: VMA limit at 32,730 tasks | not run (policy 7) | fails: VMA limit at 32,730 tasks |  |  |
| A, 1 MiB | fails: VMA limit at 32,730 tasks | not run (policy 7) | fails: VMA limit at 32,730 tasks |  |  |
| A, 8 MiB | fails: VMA limit at 32,730 tasks | not run (policy 7) | fails: VMA limit at 32,730 tasks |  |  |
| B, 128 KiB | 4.68 [4.68-4.68] | not run (policy 7) | 0.27 [0.27-0.27] | 1.75 [1.73-1.77] |  |
| B, 1 MiB | 6.43 [6.43-6.43] | not run (policy 7) | 2.02 [2.02-2.02] | 2.41 [2.38-2.43] |  |
| B, 8 MiB | 8.45 [8.45-8.45] | not run (policy 7) | 4.03 [4.03-4.03] | 3.16 [3.13-3.19] |  |
| C, 4 KiB (+32 KiB: 36 KiB) | 5.50 [5.50-5.50] | not run (policy 7) | 0.08 [0.08-0.08] | 2.06 [2.03-2.07] |  |
| C, 8 KiB (+32 KiB: 40 KiB) | 5.50 [5.50-5.50] | not run (policy 7) | 0.09 [0.09-0.09] | 2.06 [2.04-2.08] |  |
| C, 16 KiB (+32 KiB: 48 KiB) | 5.52 [5.52-5.52] | not run (policy 7) | 0.10 [0.10-0.10] | 2.07 [2.04-2.08] |  |
| D(A), 128 KiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  |
| D(A), 1 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  |
| D(A), 8 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,730 tasks |  |  |
| D(B), 128 KiB | 4.68 [4.68-4.68] | 4.68 [4.68-4.68] | 0.27 [0.27-0.27] | 1.75 [1.73-1.77] | 0.93 [0.93-0.94] (vs Go after GC at 100k); 1.75 [1.73-1.77] (vs Go shallow at 1M) |
| D(B), 1 MiB | 6.43 [6.43-6.43] | 6.44 [6.44-6.44] | 2.02 [2.02-2.02] | 2.41 [2.38-2.43] | 1.28 [1.28-1.29] (vs Go after GC at 100k); 2.41 [2.38-2.43] (vs Go shallow at 1M) |
| D(B), 8 MiB | 8.45 [8.45-8.45] | 8.45 [8.45-8.45] | 4.03 [4.03-4.03] | 3.16 [3.13-3.19] | 1.68 [1.68-1.69] (vs Go after GC at 100k); 3.16 [3.13-3.19] (vs Go shallow at 1M) |
| E (stackless) | 0.63 [0.63-0.63] | 0.64 [0.64-0.64] | 0.00 [0.00-0.00] | 0.24 [0.23-0.24] | 0.13 (vs Go after GC at 100k); 0.24 (vs Go shallow at 1M) |
| tokio | 0.40 [0.40-0.40] | 0.40 [0.40-0.40] | 0.00 [0.00-0.00] | 0.15 | 0.08 (vs Go after GC at 100k); 0.15 (vs Go shallow at 1M) |
| Go | 2.67 [2.65-2.70] | not run (policy 7); after GC not run (policy 7) | 0.00 [0.00-0.00] | 1.00 [0.98-1.02] |  |
| Java virtual threads | 1.07 [1.00-1.14] | 2.00 [1.81-2.01]; heap 0.92 [0.91-0.93] | 0.00 [0.00-0.00] | 0.40 [0.37-0.43] | 0.40 [0.36-0.40] (vs Go after GC at 100k); 0.75 [0.67-0.76] (vs Go shallow at 1M) |

Section 2.4 of proposal/20 applied to M1 (each stackful configuration's worst
ratio over the four conditions; at 1M after the excursion, where Go's own cell
cannot fit 16 GiB, the larger of the ratios to Go after GC at 100k and to Go
shallow at 1M, a lower bound of Go after GC at 1M):

**memory rule (proposal/20, section 2.4) on github-runner: no Go cell at 100k or 1M tasks; not computable**

**memory rule (proposal/20, section 2.4) on vm: ratio to Go, worst over four conditions**

| configuration | 100k shallow | 100k after excursion | 1M shallow | 1M after excursion | verdict on this machine |
|---|---|---|---|---|---|
| A/128K | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | not required (policy 7) | fails |
| A/1M | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | not required (policy 7) | fails |
| A/8M | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | not required (policy 7) | fails |
| B/128K | 1.73 (CI 1.73-1.75; range 1.73-1.75) vs Go | 6.51 (CI 6.51-6.52; range 6.51-6.53) vs Go after GC | 1.75 (CI 1.73-1.77; range 1.73-1.77) vs Go | not required (policy 7) | fails (w = 6.51) |
| B/1M | 2.38 (CI 2.38-2.4; range 2.38-2.4) vs Go | 6.86 (CI 6.86-6.88; range 6.86-6.88) vs Go after GC | 2.41 (CI 2.38-2.43; range 2.38-2.43) vs Go | not required (policy 7) | fails (w = 6.86) |
| B/8M | 3.13 (CI 3.13-3.15; range 3.13-3.15) vs Go | 7.28 (CI 7.28-7.3; range 7.28-7.3) vs Go after GC | 3.16 (CI 3.13-3.19; range 3.13-3.19) vs Go | not required (policy 7) | fails (w = 7.28) |
| C/36K | 2.04 (CI 2.03-2.05; range 2.03-2.05) vs Go | 7.08 (CI 7.08-7.09; range 7.08-7.09) vs Go after GC | 2.06 (CI 2.03-2.07; range 2.03-2.07) vs Go | not required (policy 7) | fails (w = 7.08) |
| C/40K | 2.04 (CI 2.04-2.06; range 2.04-2.06) vs Go | 7.08 (CI 7.08-7.09; range 7.08-7.09) vs Go after GC | 2.06 (CI 2.04-2.08; range 2.04-2.08) vs Go | not required (policy 7) | fails (w = 7.08) |
| C/48K | 2.04 (CI 2.04-2.06; range 2.04-2.06) vs Go | 7.08 (CI 7.08-7.09; range 7.08-7.09) vs Go after GC | 2.07 (CI 2.04-2.08; range 2.04-2.08) vs Go | not required (policy 7) | fails (w = 7.08) |
| D(A)/128K | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | fails |
| D(A)/1M | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | fails |
| D(A)/8M | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 100,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | fails: VMA limit at 32,730 of 1,000,000 tasks | fails |
| D(B)/128K | 1.73 (CI 1.73-1.75; range 1.73-1.75) vs Go | 0.934 (CI 0.934-0.936; range 0.934-0.936) vs Go after GC | 1.75 (CI 1.73-1.77; range 1.73-1.77) vs Go | 0.934 (CI 0.934-0.935; range 0.934-0.935) vs Go after GC at 100k; 1.75 (CI 1.73-1.77; range 1.73-1.77) vs Go shallow at 1M | passes (w = 1.75) |
| D(B)/1M | 2.38 (CI 2.38-2.4; range 2.38-2.4) vs Go | 1.28 (CI 1.28-1.29; range 1.28-1.29) vs Go after GC | 2.41 (CI 2.38-2.43; range 2.38-2.43) vs Go | 1.28 (CI 1.28-1.29; range 1.28-1.29) vs Go after GC at 100k; 2.41 (CI 2.38-2.43; range 2.38-2.43) vs Go shallow at 1M | passes (w = 2.41) |
| D(B)/8M | 3.13 (CI 3.13-3.15; range 3.13-3.15) vs Go | 1.68 (CI 1.68-1.69; range 1.68-1.69) vs Go after GC | 3.16 (CI 3.13-3.19; range 3.13-3.19) vs Go | 1.68 (CI 1.68-1.69; range 1.68-1.69) vs Go after GC at 100k; 3.16 (CI 3.13-3.19; range 3.13-3.19) vs Go shallow at 1M | passes (w = 3.16) |

* **The verdict on M1.** D(B) is the only stackful strategy that passes, at
  every reservation, with w = 2.41 at 1 MiB. B and C keep the excursion's pages
  ("commit creep", 34.4 KiB per task for B/1 MiB) and fail against Go after its
  collection (5.02 [5.01-5.02] KiB), although measured against Go before its
  collection (34.7 [33.4-34.7] KiB) they are at 0.99x (B/1 MiB) and 1.02x (C/4
  KiB). A and D(A) fail on scale first: each guard page splits a mapping, and
  the process stops at 32,730 tasks under `vm.max_map_count` 65530 (their
  per-task figures over the tasks that were created match B's and D(B)'s).
* **What a task costs.** D(B)/1 MiB holds 4.41 KiB resident per task at 1M (one
  stack page plus the task record) and 2.02 KiB of page tables: a 4 KiB
  page-table page maps 2 MiB, which two 1 MiB slots share. The reservation costs
  page tables, not resident memory: 128 KiB slots need 0.27 KiB, 8 MiB slots
  4.03 KiB. Measured on RSS alone, as section 6.5 literally defines the resident
  set, the stackful strategies' ratios are smaller still (D(B)/1 MiB at 100k:
  1.64x shallow; the RSS-only columns of `summarize.py`), because Go's page
  tables are negligible and theirs are not.
* **Decommit on park.** D(B) made 1,000,001 `madvise(MADV_DONTNEED)` calls for
  1M tasks (the `decommits=` field of the rows' `note`): one per task, at the
  park after its excursion. The park after shallow use made none, because the
  probe watermark showed that the stack had not grown below its top page.
* **1M tasks.** Per-task memory does not change from 100k to 1M for any stackful
  strategy. At 1M, A, B, C and Go measured the shallow phase only (policy 7):
  keeping the excursion's pages for 1M tasks needs about 33 GiB (B/1 MiB's
  figure at 100k times 1M).
* **C and the 5 KiB reserve.** C's shallow figure is 5.50 KiB, against 4.59 KiB
  in M2's smoke run with the earlier 8 KiB reserve. A slot of 36 + 5 KiB is not
  a whole number of pages, so a quarter of the slots have their top frames
  straddle a page boundary and touch two pages. Rounding C's slots to pages (the
  reserve absorbing the rounding) would restore the earlier figure; C does not
  pass either way.
* **The controls.** Java's virtual threads copy a parked thread's frames into
  heap chunks, so Java's figures (1.07 [1.00-1.14] KiB shallow at 1M) are a
  managed baseline, not a non-moving-stack precedent. E and tokio keep no stack
  per task (0.63 and 0.40 KiB). On M2, Go's before-GC figure after the excursion
  is 66.4 KiB (transparent huge pages are `always` there); after GC it is 5.04
  KiB, as on M1.
* **M2.** The smoke run's 10k-task ratios agree with M1's to within a few
  hundredths for A, B, D(A) and D(B) (D(B)/1 MiB 2.41x and 1.31x against M1's
  2.38x and 1.28x at 100k); C's differ because its slot geometry changed with
  the reserve (above). 10k is not a threshold N, so they are supporting evidence
  only.

## 2. Spawn plus join (threshold: median within 3x Go with pooled stacks)

100k spawn-plus-join pairs after 10k unrecorded pairs (Java: 20k, for its JIT),
each timed with a monotonic clock; the trivial task returns at once.

| Configuration | median M1 | p99 M1 | median M2 (smoke) | p99 M2 (smoke) | median / Go M1 | median / Go M2 |
|---|---|---|---|---|---|---|
| A (no cache) | 7.41 [7.31-9.15] | 22.1 [19.8-32.7] | 15.1 | 30.5 | 16.11 [13.19-22.15] | 41.77 |
| A (`--stack-cache 1024`) | 0.64 [0.63-0.66] | 0.94 [0.74-1.01] | 0.29 | 0.42 | 1.38 [1.14-1.59] | 0.81 |
| **B** | 0.64 [0.64-0.80] | 0.96 [0.81-0.97] | 0.43 | 0.65 | 1.40 [1.15-1.92] | 1.19 |
| C (8 KiB) | 0.64 [0.64-0.70] | 0.92 [0.84-1.03] | 0.41 | 0.67 | 1.40 [1.16-1.70] | 1.14 |
| D(A) | 7.99 [7.74-8.09] | 23.4 [21.1-24.8] | 16.8 | 43.3 | 17.36 [13.98-19.60] | 46.46 |
| D(A) (`--stack-cache 1024`) | 1.06 [1.05-1.53] | 1.62 [1.39-4.28] | 1.15 | 1.29 | 2.31 [1.90-3.69] | 3.19 |
| D(B) | 0.67 [0.66-0.83] | 1.04 [0.88-1.08] | 0.46 | 0.73 | 1.45 [1.19-2.02] | 1.28 |
| E | 0.52 [0.51-0.60] | 0.72 [0.65-0.83] | 0.32 | 0.54 | 1.12 [0.92-1.46] | 0.89 |
| tokio | 0.52 [0.41-0.56] | 11.9 [10.2-12.4] | 0.33 | 4.64 | 1.12 [0.74-1.35] | 0.92 |
| OS threads | 44.3 [43.1-46.5] | 171 [161-183] | 59.9 | 138 | 96.20 [77.78-112.57] | 165.88 |
| B, batch 100 (per task) | 1.30 [1.27-1.37] | 2.87 [2.17-3.37] | 0.32 | 0.46 | 2.82 [2.30-3.31] | 0.88 |
| B, scope per round | 0.82 [0.82-0.88] | 2.83 [2.81-3.39] | 0.61 | 2.65 | 1.79 [1.48-2.12] | 1.69 |
| Go | 0.46 [0.41-0.55] | 9.73 [9.12-11.2] | 0.36 | 1.36 | 1.00 [0.75-1.34] | 1.00 |
| Java virtual threads | 8.29 [8.26-10.1] | 29.7 [26.0-34.9] | not run | not run | 18.03 [14.92-24.44] |  |

* **Verdict.** B's median is 1.40 times Go's on M1 (95% bootstrap interval
  1.16-1.73; range 1.15-1.92) and 1.19 times on M2's smoke run: within 3x on
  both machines (**holds**). D(B), the candidate, costs about what B costs
  (1.45x): a task that never grew below its top page is released without a
  system call.
* **The cost of mapping.** A maps, protects and unmaps a stack per task (7.41
  [7.31-9.15] us); with a cache of 1024 freed stacks it costs what B costs (0.64
  [0.63-0.66] us). D(A) with the cache still pays one `madvise` per release
  (1.06 [1.05-1.53] us), which D(B)'s watermark avoids.
* **Tails.** B's p99 (0.96 [0.81-0.97] us) is below Go's (9.73 [9.12-11.2] us)
  on M1. Java's virtual threads cost 8.29 [8.26-10.1] us per spawn and join
  after 20,000 unrecorded rounds for the JIT.

## 3. Scale: 1M idle tasks on a 16 GiB machine with default sysctls

`max-tasks --tasks 1000000 --mem-guard 512M --hold-secs 60`: tasks that do
shallow work and park are spawned one at a time until the cap, a failed spawn,
or less than 512 MiB of memory headroom (`MemAvailable`, or the memory cgroup's
headroom when smaller); a run that reaches the cap then keeps every task parked
for 60 seconds, checks that all of them are still parked, and reads memory again
(section 6.5: "pass if all park and the process survives 60 seconds"). OS
threads have a cap of 30,000.

| Configuration | tasks created M1 (cap 1M) | limit M1 | RSS+PTE per task at stop M1 (KiB) | still parked after 60 s M1 | hold (s) | created M2 (smoke) | limit M2 |
|---|---|---|---|---|---|---|---|
| A (guard page per task) | 32,730 [32,730-32,730] | vma_count | 6.43 [6.43-6.44] |  |  | 100,000 (100000 cap) | cap |
| B | 1,000,000 [1,000,000-1,000,000] | cap | 6.43 [6.43-6.43] | 1,000,000 [1,000,000-1,000,000] | 60.0 [60.0-60.0] | 100,000 (100000 cap) | cap |
| C (8 KiB) | 1,000,000 [1,000,000-1,000,000] | cap | 5.44 [5.44-5.44] | 1,000,000 [1,000,000-1,000,000] | 60.0 [60.0-60.0] | 100,000 (100000 cap) | cap |
| D(B) | 1,000,000 [1,000,000-1,000,000] | cap | 6.43 [6.43-6.43] | 1,000,000 [1,000,000-1,000,000] | 60.0 [60.0-60.0] | 100,000 (100000 cap) | cap |
| D(A) | 32,730 [32,730-32,730] | vma_count | 6.44 [6.43-6.44] |  |  | 100,000 (100000 cap) | cap |
| E | 1,000,000 [1,000,000-1,000,000] | cap | 0.59 [0.59-0.59] | 1,000,000 [1,000,000-1,000,000] | 60.0 [60.0-60.0] | 100,000 (100000 cap) | cap |
| tokio | 1,000,000 [1,000,000-1,000,000] | cap | 0.39 [0.39-0.39] | 1,000,000 [1,000,000-1,000,000] | 60.0 [60.0-60.0] | 100,000 (100000 cap) | cap |
| OS threads | 16,128 [16,128-16,128] | vma_count | 9.97 [9.97-9.98] |  |  | 5,000 (5000 cap) | cap |
| Go | 1,000,000 [1,000,000-1,000,000] | cap | 2.61 [2.61-2.61] | 1,000,000 [1,000,000-1,000,000] | 60.1 [60.0-60.1] | 100,000 (100000 cap) | cap |
| Java virtual threads | 1,000,000 [1,000,000-1,000,000] | cap | 1.01 [0.95-1.05] | 1,000,000 [1,000,000-1,000,000] | 60.0 [60.0-60.0] | not run |  |

* A and D(A) stop at 32,730 tasks with 65,531 mappings: every task's guard page
  splits its stack mapping, so the process meets `vm.max_map_count` (65530) at
  about 32k tasks. That is the scale row's fail condition: **guard-page-per-task
  is out** as a default, which policy 3 already concludes.
* B, C, D(B), E and tokio, and the Go and Java controls, all park 1M tasks;
  after the 60-second hold every task is still parked, and per-task memory is
  unchanged (B/1 MiB 6.43 KiB at the stop and 6.43 KiB after the hold). The
  pooled strategies' reservations do not approach the mapping limit: B/1 MiB has
  69 mappings in all at 1M tasks, with about 0.96 TiB of address space reserved
  in 1 GiB chunks (`reserved_bytes` in the `idle-memory` rows' note).
* OS threads stop at 16,128 (vma_count): each Rust thread maps its stack, its
  guard, and the signal stack that `std` maps with its own guard.
* M2's smoke run had a cap of 100k and the runner image's `vm.max_map_count` of
  262144, so A reached its cap there; under the kernel default the full-scale
  workflow restores, A meets the same wall.

## 4. Echo (threshold: 10k-connection p99 within 2x Go) and idle connections

`echo`: N closed-loop client tasks in the same process (64-byte messages, a
write and a read of the echo per round trip), 5 s unrecorded then 30 s recorded;
the server is one task per connection. `idle-connections`: N server tasks parked
in `read`, the client ends held as plain sockets, memory read after 30 s idle
(section 6.5). Both ends count against `RLIMIT_NOFILE`, so N connections need 2N
+ 64 descriptors.

10000 connections

| Configuration | p50 M1 | p99 M1 | p99.9 M1 | req/s M1 | p99 / Go M1 | p99 / req/s M2 (smoke, 2 workers) | p99 / Go M2 |
|---|---|---|---|---|---|---|---|
| A | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| B | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| C (8 KiB) | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| D(A) | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| D(B) | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| E | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| tokio | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| Go | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |
| Java virtual threads | not run: RLIMIT_NOFILE 20000 < 20064 |  |  |  |  |  |  |

1000 connections

| Configuration | p50 M1 | p99 M1 | p99.9 M1 | req/s M1 | p99 / Go M1 | p99 / req/s M2 (smoke, 2 workers) | p99 / Go M2 |
|---|---|---|---|---|---|---|---|
| A | 3.11 [3.07-3.21] | 4.51 [4.16-7.94] | 6.50 [5.72-13.0] | 314,121 [280,012-318,494] | 0.92 [0.52-1.64] | 13.6 / 84,816 | 0.92 |
| B | 3.06 [3.03-3.10] | 4.32 [4.13-6.73] | 6.46 [5.75-9.75] | 319,259 [304,219-322,846] | 0.88 [0.52-1.39] | 12.6 / 85,051 | 0.85 |
| C (8 KiB) | 3.04 [2.95-3.96] | 4.32 [3.97-7.91] | 6.42 [5.32-10.4] | 318,677 [244,297-333,410] | 0.88 [0.50-1.64] | 12.9 / 85,728 | 0.87 |
| D(A) | 13.4 [13.2-13.5] | 18.4 [17.9-19.8] | 24.1 [24.0-26.4] | 73,266 [72,715-74,131] | 3.73 [2.24-4.11] | 17.8 / 61,480 | 1.20 |
| D(B) | 3.07 [2.99-3.08] | 4.16 [4.03-4.39] | 5.81 [5.18-6.96] | 318,935 [316,259-328,027] | 0.85 [0.50-0.91] | 12.4 / 85,108 | 0.84 |
| E | 2.97 [2.93-3.17] | 4.10 [4.00-5.80] | 5.80 [5.34-9.22] | 329,920 [295,210-333,751] | 0.83 [0.50-1.20] | 13.3 / 85,409 | 0.89 |
| tokio | 2.32 [2.27-2.37] | 7.75 [7.62-11.2] | 11.3 [10.7-21.1] | 353,546 [328,134-361,553] | 1.57 [0.95-2.31] | 57.8 / 93,359 | 3.89 |
| Go | 3.24 [3.18-3.41] | 4.92 [4.83-8.00] | 8.01 [7.51-13.2] | 303,681 [279,851-309,769] | 1.00 [0.60-1.66] | 14.8 / 79,373 | 1.00 |
| Java virtual threads | 5.99 [5.91-6.32] | 12.0 [11.2-13.4] | 15.7 [15.2-17.7] | 161,280 [149,019-163,054] | 2.44 [1.40-2.78] |  |  |

idle connections

| Configuration | RSS+PTE per connection M1 | RSS only M1 | / Go M1 | RSS+PTE M2 (smoke, 5000) | / Go M2 |
|---|---|---|---|---|---|
| A | 10.6 [10.6-10.6] | 8.54 [8.54-8.54] | 1.50 [1.47-1.50] | 10.6 | 1.27 |
| B | 10.6 [10.6-10.6] | 8.54 [8.54-8.54] | 1.50 [1.48-1.51] | 10.8 | 1.29 |
| C (8 KiB) | 8.57 [8.57-8.58] | 8.54 [8.54-8.54] | 1.21 [1.20-1.22] | 8.76 | 1.05 |
| D(A) | 10.6 [10.6-10.6] | 8.54 [8.54-8.54] | 1.50 [1.47-1.50] | 10.6 | 1.27 |
| D(B) | 10.6 [10.6-10.6] | 8.54 [8.54-8.55] | 1.50 [1.48-1.51] | 10.8 | 1.29 |
| E | 4.97 [4.97-4.97] | 4.96 [4.96-4.96] | 0.70 [0.69-0.71] | 5.00 | 0.60 |
| tokio | 4.81 [4.81-4.82] | 4.80 [4.80-4.81] | 0.68 [0.67-0.69] | 4.86 | 0.58 |
| Go | 7.06 [7.02-7.16] | 7.05 [7.01-7.14] | 1.00 [0.98-1.02] | 8.33 | 1.00 |
| Java virtual threads | 11.6 [10.9-13.7] | 11.5 [10.9-13.7] | 1.64 [1.53-1.95] | not run |  |

* **10k connections were not run on M1.** They need 20,064 descriptors and the
  hard limit is 20000 (policy 3: the cell runs only where the default limit
  allows). The harness and the controls record failed rows instead. M2's full
  run can run them (its hard limit is 65536).
* **At 1000 connections** B's p99 is 0.88 times Go's (95% bootstrap interval
  0.54-1.37; range 0.52-1.39) and D(B)'s 0.85 (95% bootstrap interval 0.52-0.91;
  range 0.50-0.91); D(A) is the exception at 3.73 (95% bootstrap interval
  2.30-4.03; range 2.24-4.11): without a watermark it makes a `madvise` call at
  every park, and every request parks its server task and its client task. I/O
  design, for the record: taskbench's workers poll one shared epoll instance
  (busy workers every 31 scheduling decisions, idle ones before they spin) and
  one sleeping worker blocks in it; Go uses its integrated netpoller. A miss
  would be "runtime design review, not a language change".
* **Idle connections.** At 9,900 connections a pooled strategy's idle connection
  costs 10.6 KiB (B/1 MiB; 8.57 [8.57-8.58] KiB for C/8 KiB, whose small slots
  need fewer page tables), against Go's 7.06 [7.02-7.16] KiB; the server task
  holds a 4 KiB read buffer on its stack.

## 5. Stack exhaustion (threshold: no segfault, ever)

`deep-recursion --framed` in every stackful configuration (E and tokio run the
parse and the naive recursion on the worker's 8 MiB stack, plus a recursion of
boxed futures): a 10k-deep recursive-descent parse (four frames per level), a
1M-deep naive recursion with a drop guard every 256 frames, and the framed
recursion (a 256-byte local and a drop guard in every frame), each in its own
task, then a task that checks the runtime still works. The reserve columns are
the bytes of the 5 KiB probe reserve the unwind overwrote (the reserve is
painted before the case and scanned after), the maximum over five runs.

| Configuration | 10k parse M1 | 1M naive M1 | framed M1 | naive async M1 | drop guards skipped M1 | reserve bytes used M1 (parse / naive / framed, max of 5) | healthy after M1 | reserve used M2 (smoke, 0.1.0) | M2 outcomes (smoke) |
|---|---|---|---|---|---|---|---|---|---|
| A/128K | stack_exhausted at 354 | stack_exhausted at 2,722 | stack_exhausted at 429 |  | naive 11, framed 430 |  | 1 [1-1] |  | stack_exhausted at 354; stack_exhausted at 2,722 |
| A/1M | stack_exhausted at 2,848 | stack_exhausted at 21,837 | stack_exhausted at 3,447 |  | naive 86, framed 3,448 |  | 1 [1-1] |  | stack_exhausted at 2,848; stack_exhausted at 21,836 |
| A/8M | completed at 10,000 | stack_exhausted at 174,754 | stack_exhausted at 27,592 |  | naive 683, framed 27,592 |  | 1 [1-1] |  | completed at 10,000; stack_exhausted at 174,754 |
| B/128K | stack_exhausted at 363 | stack_exhausted at 2,786 | stack_exhausted at 439 |  | naive 0, framed 0 | 1880 / 1832 / 2072 | 1 [1-1] | 3416 / 1800 | stack_exhausted at 354; stack_exhausted at 2,721 |
| B/1M | stack_exhausted at 2,856 | stack_exhausted at 21,900 | stack_exhausted at 3,457 |  | naive 0, framed 0 | 1800 / 1800 / 2040 | 1 [1-1] | 3416 / 1816 | stack_exhausted at 2,847; stack_exhausted at 21,836 |
| B/8M | completed at 10,000 | stack_exhausted at 174,818 | stack_exhausted at 27,602 |  | naive 0, framed 0 | 0 / 1832 / 2088 | 1 [1-1] | 0 / 3416 | completed at 10,000; stack_exhausted at 174,753 |
| C/2K | stack_exhausted at 4 | stack_exhausted at 34 | stack_exhausted at 4 |  | naive 0, framed 0 | 1864 / 1832 / 1928 | 1 [1-1] | 3480 / 1800 | stack_exhausted at 4; stack_exhausted at 33 |
| C/8K | stack_exhausted at 20 | stack_exhausted at 162 | stack_exhausted at 24 |  | naive 0, framed 0 | 1880 / 1832 / 1864 | 1 [1-1] | 3480 / 1800 | stack_exhausted at 20; stack_exhausted at 161 |
| C/16K | stack_exhausted at 43 | stack_exhausted at 332 | stack_exhausted at 51 |  | naive 0, framed 0 | 1880 / 1800 / 1880 | 1 [1-1] | 3480 / 1816 | stack_exhausted at 43; stack_exhausted at 332 |
| D(A)/128K | stack_exhausted at 354 | stack_exhausted at 2,722 | stack_exhausted at 429 |  | naive 11, framed 430 |  | 1 [1-1] |  | stack_exhausted at 354; stack_exhausted at 2,722 |
| D(A)/1M | stack_exhausted at 2,848 | stack_exhausted at 21,837 | stack_exhausted at 3,447 |  | naive 86, framed 3,448 |  | 1 [1-1] |  | stack_exhausted at 2,848; stack_exhausted at 21,836 |
| D(A)/8M | completed at 10,000 | stack_exhausted at 174,754 | stack_exhausted at 27,592 |  | naive 683, framed 27,592 |  | 1 [1-1] |  | completed at 10,000; stack_exhausted at 174,754 |
| D(B)/128K | stack_exhausted at 363 | stack_exhausted at 2,786 | stack_exhausted at 439 |  | naive 0, framed 0 | 1880 / 1832 / 2072 | 1 [1-1] | 3416 / 1800 | stack_exhausted at 354; stack_exhausted at 2,721 |
| D(B)/1M | stack_exhausted at 2,856 | stack_exhausted at 21,900 | stack_exhausted at 3,457 |  | naive 0, framed 0 | 1800 / 1800 / 2040 | 1 [1-1] | 3416 / 1816 | stack_exhausted at 2,847; stack_exhausted at 21,836 |
| D(B)/8M | completed at 10,000 | stack_exhausted at 174,818 | stack_exhausted at 27,602 |  | naive 0, framed 0 | 0 / 1832 / 2088 | 1 [1-1] | 0 / 3416 | completed at 10,000; stack_exhausted at 174,753 |
| E | completed at 10,000 | stack_exhausted at 173,278 |  | stack_exhausted at 86,638 | naive 0 |  | 1 [1-1] |  | completed at 10,000; stack_exhausted at 173,279 |
| tokio | completed at 10,000 | stack_exhausted at 173,273 |  | stack_exhausted at 86,634 | naive 0 |  | 1 [1-1] |  | completed at 10,000; stack_exhausted at 173,273 |

* **Every exhaustion is clean**: 255 outcome rows on M1, none unclean, and the
  follow-up task ran in every run. `process_exit` rows (crashes) across every
  CSV of both results directories: **0**.
* **Mechanism.** B, C and D(B) fail through the software probe and unwind, so
  every destructor runs (0 drop guards skipped). A and D(A) fail through the
  guard page; the trap handler ends the task without unwinding, so the frames'
  destructors are skipped (86 drop guards in the naive recursion at 1 MiB, 3,448
  in the framed one).
* **The probe reserve.** The largest unwind on M1 used 2,088 bytes of the
  5,120-byte reserve. Before the fix of section 10.3, the first unwind of each
  process used about 5 KiB, and M2's smoke run shows that pattern (3,416 bytes
  for the first, 1,816 for the next).
* **Depth per reservation.** The 10k-deep parse completes only at 8 MiB (and on
  E and tokio, whose worker stacks are 8 MiB); at 1 MiB it fails cleanly at
  level 2,856. B's usable stack is a little over its reservation (the slot is
  rounded up to a page above the 5 KiB reserve), so B reaches a few more levels
  than A.
* The corpus-wide `--stats stack-depth` histogram (interpreter era) is reported
  separately and decides whether the default reservation suffices.

## 6. Starvation (threshold: probe p99 no worse than Go and under 10 ms)

Four compute tasks per worker (16 on M1's four workers) run an LCG loop and
reach a yield point every K iterations, the stand-in for a compiler-inserted
yield check; a probe task sleeps 1 ms in a loop and records how late each
wake-up is, first alone (1 s) and then under the load (11 s). Go runs the same
loop with no yields at all and relies on its default asynchronous preemption
(policy 8); Java has no preemption of virtual threads and runs with no yields
and with `Thread.yield()` every K iterations. Labels: the default K is 10000;
`yield_every=0` means no yield points.

| Configuration (K = yield_every; default 10000) | slice between yields | p50 loaded | p99 loaded | max loaded | p99 alone | wake-ups | p99 / Go (default preemption) | M2 p99 (smoke) |
|---|---|---|---|---|---|---|---|---|
| A/1M | 21.0 [20.3-32.5] | 4.87 [4.62-5.95] | 25.9 [24.8-28.3] | 455 [90.0-3,955] | 181 [170-219] | 10,936 [10,909-10,937] | 0.00 | 26.8; / Go 0.00 |
| A/1M yield_every=100000 | 212 [195-221] | 32.0 [29.0-58.1] | 233 [225-271] | 833 [377-7,026] | 185 [158-281] | 10,559 [10,350-10,566] | 0.01 |  |
| B/1M | 21.3 [20.3-22.0] | 5.08 [4.85-5.64] | 25.7 [25.0-30.0] | 570 [89.6-3,463] | 178 [172-215] | 10,931 [10,920-10,937] | 0.00 | 25.8; / Go 0.00 |
| B/1M yield_every=0 | 206 [181-218] | 10,999,124 [1,123-11,001,644] | 10,999,124 [10,994,330-11,001,644] | 10,999,124 [10,994,330-11,001,644] | 175 [162-210] | 1 [1-2] | 279.61 [279.36-279.75] | 1,999,154; / Go 50.90 |
| B/1M yield_every=1000 | 2.14 [2.00-2.81] | 1.47 [1.41-1.80] | 4.50 [4.10-5.39] | 4,489 [2,776-19,707] | 192 [181-421] | 10,978 [10,952-10,984] | 0.00 | 2.91; / Go 0.00 |
| B/1M yield_every=100000 | 202 [189-224] | 32.4 [29.7-58.6] | 229 [221-269] | 1,217 [345-3,034] | 180 [152-230] | 10,555 [10,358-10,557] | 0.01 |  |
| C/8K | 20.6 [18.5-21.8] | 4.92 [4.76-5.95] | 26.1 [25.9-30.0] | 3,861 [118-7,930] | 183 [159-255] | 10,928 [10,914-10,939] | 0.00 | 27.7; / Go 0.00 |
| C/8K yield_every=100000 | 213 [199-273] | 33.7 [30.9-57.6] | 233 [230-276] | 1,648 [352-4,269] | 201 [160-229] | 10,540 [10,347-10,558] | 0.01 |  |
| D(A)/1M | 20.3 [19.9-21.4] | 5.11 [4.85-5.91] | 26.8 [24.8-29.3] | 650 [166-9,486] | 182 [166-395] | 10,930 [10,866-10,936] | 0.00 | 25.0; / Go 0.00 |
| D(A)/1M yield_every=100000 | 201 [189-274] | 32.0 [29.4-58.4] | 236 [226-268] | 1,068 [436-3,137] | 165 [160-280] | 10,549 [10,356-10,563] | 0.01 |  |
| D(B)/1M | 21.8 [20.7-22.2] | 4.91 [4.89-5.79] | 25.4 [25.0-27.2] | 370 [86.4-562] | 184 [152-288] | 10,932 [10,924-10,939] | 0.00 | 28.7; / Go 0.00 |
| D(B)/1M yield_every=100000 | 205 [202-221] | 35.7 [32.6-59.1] | 234 [234-271] | 674 [334-5,113] | 193 [168-250] | 10,535 [10,349-10,542] | 0.01 |  |
| E | 20.7 [18.0-22.0] | 4.58 [4.39-4.86] | 17.8 [17.7-18.9] | 376 [86.9-3,954] | 222 [181-483] | 10,940 [10,936-10,946] | 0.00 | 18.8; / Go 0.00 |
| E yield_every=100000 | 207 [204-224] | 31.8 [29.4-56.0] | 137 [133-153] | 277 [201-5,580] | 190 [166-265] | 10,582 [10,407-10,602] | 0.00 |  |
| go yield_every=0 |  | 19,179 [19,174-19,187] | 39,338 [39,327-39,356] | 39,610 [39,424-42,139] | 217 [175-263] | 535 [533-536] | 1.00 | 39,274; / Go 1.00 |
| java |  | 333 [88.4-337] | 774 [585-2,196] | 52,105 [29,067-69,541] | 465 [377-871] | 8,529 [7,995-9,599] | 0.02 [0.01-0.06] |  |
| java yield_every=0 |  | 11,003,284 [11,001,379-11,005,227] | 11,003,284 [11,001,379-11,005,227] | 11,003,284 [11,001,379-11,005,227] | 454 [408-744] | 1 [1-1] | 279.71 [279.54-279.84] |  |
| java yield_every=100000 |  | 2,731 [2,680-2,773] | 6,103 [4,037-211,502] | 2,082,688 [605,333-2,810,069] | 436 [362-1,406] | 2,080 [593-2,582] | 0.16 [0.10-5.38] |  |
| os-threads | 20.9 [18.2-23.6] | 68.4 [67.5-69.1] | 3,000 [3,000-3,033] | 12,901 [11,293-27,099] | 180 [172-298] | 9,635 [9,553-9,701] | 0.08 | 1,577; / Go 0.04 |
| tokio | 21.4 [19.0-24.7] | 1,000 [1,000-1,000] | 1,060 [1,058-1,075] | 4,960 [1,848-12,625] | 1,299 [1,199-1,561] | 5,500 [5,475-5,502] | 0.03 | 1,053; / Go 0.03 |
| tokio yield_every=100000 | 213 [194-224] | 1,003 [999-1,003] | 1,525 [1,502-1,583] | 2,130 [2,012-2,684] | 1,270 [1,235-2,021] | 5,499 [5,496-5,502] | 0.04 |  |

* **With yield points the probe's p99 is far below Go's**: 25.7 [25.0-30.0] us
  at K = 10k (slices of 21.3 [20.3-22.0] us) and 229 [221-269] us at K = 100k,
  against 39.3 [39.3-39.4] ms for Go's preemption; the threshold holds for every
  strategy. M2's smoke run agrees (B 25.8 us, Go 39.3 ms, with 4 compute tasks).
* **Without yield points** (`yield_every=0`) the probe cannot run until the
  compute tasks stop: 10,999 [10,994-11,002] ms. Policy 8 expects this and
  reports it without counting it.
* **What yield points cost.** Compute throughput with no yield points is
  1,887,211,880 [1,767,957,571-1,933,528,494] iterations per second. Relative to
  that median, the median throughput with a yield point every 100k iterations is
  +1.5 %, every 10k -2.9 %, and every 1k -18.1 %. The five-run ranges (the one
  above, and 1,832,735,066 [1,740,280,698-1,895,116,740] at 10k) are wider than
  the first two differences, so at 10k and 100k iterations the cost is below
  what this machine resolves; at 1k (slices of 2.14 [2.00-2.81] us) it is clear.
  The cost is the switches themselves: the check on the loop's back-edge is the
  `loop` workload of section 7 and costs almost nothing. Because policy 8 puts
  yield points in the 3 % budget, the yield interval is a design parameter: on
  this machine, slices of about 0.2 ms (K = 100k) cost nothing measurable and
  keep the probe's p99 at 229 [221-269] us, 172 times lower than Go's.
* tokio's p99 of about 1 ms is its timer's granularity, not starvation. OS
  threads rely on the kernel's preemption.

## 7. Software probe overhead (threshold: at most 3 % on compute loops)

Each workload is compiled without probes and with three probe models, and the
four variants run alternately 31 times inside one task on the strategy's stack;
a cell's value is the ratio of medians minus one, and the table shows the median
and range over five processes. `split` is the model of record: `cmp rsp,
fs:[0x70]; jb`, the prologue check of GCC's `-fsplit-stack`, against a slot the
scheduler keeps equal to the running task's limit. `arg` passes the limit as an
argument; `tls` reads a Rust thread-local (not migration-safe). `loop` is the
call-free LCG loop of section 6 with the same check on every back-edge: the cost
model of a compiler-inserted yield check, which policy 8 counts in this budget
(in the other workloads the entry check can double as the yield point, as Go's
prologue check did before asynchronous preemption).

| Configuration | fib split | calls split | tree split | loop split (back-edge) | fib arg | calls arg | tree arg | fib tls | calls tls | tree tls | M2 split fib / calls / tree (smoke) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A/1M | 14.0 [13.9-15.0] | 0.0 [-0.9-0.5] | 4.2 [2.5-6.8] | -0.3 [-0.4-0.1] | 23.5 [22.9-24.3] | 0.1 [-1.6-0.3] | 3.3 [1.4-4.8] | 13.8 [13.7-15.5] | 1.3 [0.6-2.4] | 2.6 [-0.5-6.5] | 30.5 / 0.1 / 25.6 |
| B/1M | 13.3 [11.9-16.3] | 0.0 [-0.1-0.9] | 4.5 [1.9-5.3] | -0.1 [-0.2-0.5] | 23.5 [21.4-23.7] | 0.2 [-0.5-0.9] | 4.1 [0.7-6.6] | 13.6 [11.9-14.2] | 0.8 [0.2-2.3] | 5.1 [2.8-6.5] | 30.4 / 0.4 / 25.0 |
| C/8K | 14.3 [11.7-15.0] | 0.1 [-0.2-1.1] | 6.1 [4.9-11.0] | -0.1 [-0.5-0.2] | 23.9 [22.4-24.1] | 0.0 [-0.3-0.7] | 4.9 [3.9-6.7] | 13.9 [12.7-14.9] | 0.7 [0.6-1.3] | 6.0 [4.9-7.4] | 30.5 / 0.1 / 32.0 |
| D(B)/1M | 14.6 [13.1-15.7] | 0.2 [-0.3-0.6] | 5.3 [2.1-7.3] | 0.1 [-0.2-0.9] | 23.4 [22.9-24.6] | 0.1 [-0.2-0.5] | 3.3 [3.1-5.7] | 13.9 [12.8-14.9] | 0.8 [0.7-2.7] | 4.8 [2.3-6.1] |  |
| E | 14.0 [13.1-14.4] | 0.2 [-0.3-0.5] | 4.7 [2.7-6.0] | 0.1 [0.0-0.2] | 24.6 [22.2-24.8] | 0.1 [-0.2-1.0] | 4.5 [3.3-5.7] | 14.1 [13.7-15.5] | 0.9 [0.6-3.3] | 5.1 [2.9-8.1] | 30.3 / -0.1 / 21.0 |

* **Taken at face value, two of the three registered workloads exceed 3 %.**
  `fib` does in every run on every strategy (B: 13.3 [11.9-16.3] %; the lowest
  single run of any strategy, 11.7 %), and `tree`'s median does on every
  strategy (B: 4.5 [1.9-5.3] %), although single runs of `tree` fall below it;
  `calls` is within it (0.0 [-0.1-0.9] %), and so is the back-edge yield check
  (-0.1 [-0.2-0.5] %, because the compare and branch run in parallel with the
  loop's multiply chain). M2's smoke run (harness 0.1.0 built with rustc 1.99,
  on an AMD EPYC 7763) measured `fib` 30.4 % and `tree` 25.0 %.
* **Why this does not decide the bar.** `fib` is an upper bound by construction:
  LLVM turns one of its two recursive calls into a loop that keeps the probe
  written at the top of the source function, so the check runs about twice per
  machine-level call, which a compiler inserting it into the prologue after
  optimisation would not do. `calls`' callee is a small leaf that a compiler
  would exempt (Go's `NOSPLIT` rule); `tree` is a fair case. And there are no
  hardware counters ("hardware counters unavailable: No such file or directory
  (os error 2)" on M1), so only wall-clock medians exist: between the five
  processes of B's cell the without-probes time of `fib` moved by 4.2 % and that
  of `tree` by 8.9 %. The owner's policy 1 therefore reports the bar as measured
  and leaves the decision to the native era, on a machine with a PMU and a fixed
  frequency, with Sisu's own probe placement. The measurements are a warning for
  that decision, not a pass.
* **Not measured:** a vectorisable loop. A check inserted before vectorisation
  would prevent it, so a compiler must place yield checks after loop
  optimisation (per vector iteration, or every N iterations).

## 8. io_uring cancellation into stack buffers (amendment A1, D-058)

Two procedures per stackful strategy, five runs each, 1M reads per run in
batches of 8,192 in flight. `pipe_mix=cancel` is A1 as registered: every read
targets an empty pipe, so it stays in flight, and every one is cancelled through
its task; the cancelled call submits `IORING_OP_ASYNC_CANCEL` and returns only
on the read's own completion; afterwards late data is written into every pipe,
so any read the kernel still held would land, and every buffer (4 KiB, on the
task's stack, the frame kept alive until the batch ends) is checked again. The
default race mix (512-byte buffers) writes data for a quarter of the reads,
cancels a quarter, and does both, in either order, for half. B also runs the
file-backed variants: `O_DIRECT` reads cancelled while the device holds them
(`-ENOENT`, and a later completion), and buffered reads with `IOSQE_ASYNC`
(`-EALREADY`). `--break-contract` is the negative control: a cancelled task
stops waiting at once.

| Configuration | reads per run | cancels submitted | ended -ECANCELED | cancel ok | -ENOENT | -EALREADY | completed after failed cancel | completed with the late data | buffer bytes written after return (5 runs) | cancel to return p50 / p99 (ms) | M2 (smoke) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A/1M | 1,000,000 [1,000,000-1,000,000] | 712,963 [696,220-728,737] | 261,734 [254,227-275,450] | 261,732 [254,227-275,448] | 451,229 [420,770-474,510] | 2 [0-4] | 57,318 [46,567-68,436] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 14.5 [13.0-15.0] / 31.8 [26.9-37.6] | 4,096 reads, 0 bytes |
| A/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 20.9 [19.7-23.5] / 40.0 [33.2-41.7] |  |
| B/1M | 1,000,000 [1,000,000-1,000,000] | 723,567 [718,533-731,475] | 256,689 [252,367-264,972] | 256,689 [252,364-264,968] | 464,686 [458,356-474,786] | 1 [0-4] | 51,685 [45,036-66,262] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 12.5 [12.2-14.1] / 31.6 [26.2-32.9] | 4,096 reads, 0 bytes |
| B/1M break_contract=true | 250,000 [250,000-250,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 195718784 (max 39692800 per run, 5 runs) | 0.11 [0.05-0.24] / 8.32 [4.48-9.00] | 1,024 reads, 161280 bytes |
| B/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [999,995-1,000,000] | 1,000,000 [999,995-1,000,000] | 0 [0-0] | 0 [0-5] | 0 [0-5] | 0 [0-5] | 0 (max 0 per run, 5 runs) | 20.9 [19.1-29.5] / 36.5 [32.4-57.2] |  |
| B/1M source=file | 1,000,000 [1,000,000-1,000,000] | 411,736 [400,417-412,333] | 0 [0-1] | 0 [0-1] | 411,736 [400,417-412,333] | 0 [0-0] | 161,059 [157,692-168,755] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 20.7 [20.2-21.6] / 48.1 [41.2-85.3] | 4,096 reads, 0 bytes |
| B/1M source=file-buffered | 1,000,000 [1,000,000-1,000,000] | 325,368 [315,631-347,480] | 5,616 [4,949-14,805] | 5,601 [4,936-14,802] | 314,929 [309,409-340,004] | 593 [561-750] | 117,827 [108,756-133,243] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 86.5 [80.9-102] / 220 [215-249] | 4,096 reads, 0 bytes |
| C/8K | 1,000,000 [1,000,000-1,000,000] | 724,601 [713,512-741,845] | 255,662 [253,201-271,342] | 255,661 [253,201-271,342] | 468,939 [444,516-487,886] | 1 [0-2] | 63,914 [47,793-85,902] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 13.5 [11.7-14.4] / 30.1 [26.7-38.0] | 4,096 reads, 0 bytes |
| C/8K pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [999,997-1,000,000] | 1,000,000 [999,997-1,000,000] | 0 [0-0] | 0 [0-3] | 0 [0-3] | 0 [0-3] | 0 (max 0 per run, 5 runs) | 20.8 [20.2-23.4] / 39.0 [36.5-46.7] |  |
| D(A)/1M | 1,000,000 [1,000,000-1,000,000] | 514,160 [494,354-524,949] | 265,807 [262,536-273,934] | 265,806 [262,533-273,929] | 248,353 [229,563-254,640] | 3 [1-5] | 146,487 [135,449-175,474] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 49.7 [39.2-53.0] / 92.5 [84.8-104] | 4,096 reads, 0 bytes |
| D(A)/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 57.6 [55.2-65.9] / 125 [119-147] |  |
| D(B)/1M | 1,000,000 [1,000,000-1,000,000] | 732,197 [708,111-740,222] | 261,807 [257,470-270,730] | 261,803 [257,468-270,724] | 471,566 [437,380-482,752] | 2 [2-7] | 42,273 [30,928-63,543] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 13.7 [13.6-14.7] / 30.1 [28.5-45.0] | 4,096 reads, 0 bytes |
| D(B)/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 21.1 [20.8-23.9] / 42.0 [36.6-51.0] |  |

* **Zero, as required.** No byte of any buffer changed after its call returned,
  in any run of either procedure on A, B, C, D(A) and D(B), nor in the file
  variants: 0 bytes over 60 runs. The negative control writes 195,718,784 bytes
  over its five runs, so the check sees what it is meant to see.
* **A cancel that cannot stop the read.** A quarter of the reads carry
  `IOSQE_ASYNC`, which sends them to an io-wq worker thread, where a blocked
  pipe read can only be interrupted. A cancel that finds such a read already
  executing answers `-EALREADY` (58 times in the 50 pipe runs above), and the
  read usually ends at once. In the first full-scale group, 8 of 50,000,000 pipe
  reads did not: each stayed blocked until the harness wrote its late data at
  the round's 30-second deadline, and each call returned only after that
  completion, 29.992-29.995 s after its cancel (the slowest cancellation of each
  of the 4 runs concerned), with its buffer intact afterwards. In the rerun
  above, 8 reads did (`completed_with_late_data`). Waiting is what keeps a lent
  buffer safe, and D-058 requires it; the consequence is that a cancelled task
  waits as long as an uncancellable read does. That is a liveness question for
  the native runtime (re-issuing the cancel signals the worker again; pollable
  files can use poll-armed reads, which a cancel always stops), not a
  memory-safety one, and it reopens nothing. The first group's check counted
  these reads' late data as "written at return"; section 10.3 describes the fix
  and where the superseded rows are kept.
* **The case D-058 exists for** is the file-backed one: a cancel that finds the
  read already issued to the device (`-ENOENT`) while the read still completes
  later (161,059 [157,692-168,755] reads per run); the call waited for those
  completions.
* **Cancel-to-return latency** in the registered procedure is 20.9 [19.1-29.5]
  ms median and 36.5 [32.4-57.2] ms p99 for B: the harness cancels 8,192 reads
  in a burst, and each cancellation queues one request to the ring's thread, so
  the figure is dominated by the burst, not by one cancellation. This group ran
  before the quiet gate existed (section 10.2), so these latencies may include
  other work's CPU use; they are not a verdict input.

## 9. Verdict and Q-6 recommendation

### 9.1 Verdict per bet-3 row (harness era; provisional, see the status note)

Section 2.1 of proposal/20: **holds** means the threshold is met with the
pre-specified effect; **fails** means it is missed or a guard trips; anything
else, including too little data or machines that disagree, is **not shown**, and
no fallback is built on it. Section 7.8: only the primary (memory) decides the
bet, the guard (exhaustion in ordinary code) can only stop it, and each
secondary triggers its own action.

| Row | Role | Verdict | The numbers | Action (section 8.3) |
|---|---|---|---|---|
| Memory per idle task | primary | **not shown** | M1: D(B) passes, w = 1.75 / 2.41 / 3.16 at 128 KiB / 1 MiB / 8 MiB, every five-run range far from 4; B (w = 7.28 at 8 MiB, 6.86 at 1 MiB) and C (7.08) fail after the excursion; A and D(A) fail at the VMA limit. M2: no cell at 100k or 1M (its 10k smoke ratios agree with M1's) | none yet. If M2's full run also has D(B) at or below 4, the row holds and the "holds at week 8" branch applies; if the machines disagree, the "not shown" branch (no fallback; Q-6 stays open) |
| Stack exhaustion | guard | harness part **holds**; the guard is not decided | every exhaustion on every strategy and reservation is a clean "stack exhausted", five runs each; the runtime works afterwards; 0 crashes; the probe reserve's need is at most 2,088 bytes of its 5,120 | the corpus `--stats stack-depth` histogram decides at week 40 |
| Spawn plus join | secondary | **holds** | B 1.40x Go on M1 (95% bootstrap interval 1.16-1.73; range 1.15-1.92), 1.19x on M2 (smoke) | no design review |
| Scale: 1M idle tasks, 16 GiB, default sysctls | secondary | **fails for A and D(A)**; holds for B, C and D(B) | A and D(A) stop at 32,730 tasks; B, C, D(B), E and tokio, and Go and Java, park 1M tasks and keep all of them parked through the 60-second hold | "fails on creation cost or VMA limits only": pooled stacks with software probes become the design. Policy 5 already makes D(B) the default, and this run is the rerun on that strategy |
| Probe overhead (part of the scale row) | secondary | **not decided** (policy 1) | `fib` 13.3 [11.9-16.3] % (an upper bound), `tree` 4.5 [1.9-5.3] %, `calls` 0.0 [-0.1-0.9] %, back-edge yield check -0.1 [-0.2-0.5] % (B); at face value above 3 % for `fib` and `tree` | the native era, on a machine with a PMU and a fixed frequency, with Sisu's own probe placement |
| Echo p99 at 10k connections | secondary | **not shown** | not run on M1 (descriptor limit); at 1000 connections B 0.88x, D(B) 0.85x and D(A) 3.73x Go | none; M2's full run includes the 10k echo (its hard limit is 65536) |
| Idle RSS at 100k connections | reported | (no threshold) | at 9,900: D(B) 1.50x Go | none |
| Starvation | secondary | **holds** with compiler-inserted yield points | B's p99 25.7 [25.0-30.0] us (K = 10k) and 229 [221-269] us (K = 100k) against Go's 39.3 [39.3-39.4] ms, all under 10 ms; without yield points the probe waits the whole run (10,999 [10,994-11,002] ms), reported and not counted (policy 8) | asynchronous preemption is not revisited |
| Cancellation with a lent buffer (A1) | secondary | **holds** on M1 | 0 bytes written after return in 50 runs on the five stackful strategies, 1M reads per run; the negative control detects its violations (195,718,784 bytes) | none; D-058 stands |
| Semantics, `sim` study | secondary | not run | interpreter era | Phases 6-9 |

**Bet 3 at week 8: not shown, pending M2.** The primary has one machine's
evidence, and on it the default candidate's worst ratio is 2.41 at 1 MiB against
a bar of 4; none of the bet's fail branches is triggered, so no fallback is
built. The guard is decided at week 40.

### 9.2 Q-6 recommendation (provisional, harness era)

* **Strategy: D(B)**, pooled stacks carved from large reservations, with
  software probes and decommit on park, the probe doubling as the decommit
  watermark. It is the only stackful strategy that passes the memory primary on
  M1; B and C fail it by commit creep (an excursion's pages stay resident), and
  A and D(A) cannot create 100k tasks under default sysctls. Its cost over B is
  within the noise: 0.67 [0.66-0.83] against 0.64 [0.64-0.80] us per spawn plus
  join, and echo p99 4.16 [4.03-4.39] against 4.32 [4.13-6.73] ms. Decommit is
  needed, and its system call shows only where it is paid on every park: D(A)
  has no watermark and makes one `madvise` per park, and its echo throughput is
  73,266 [72,715-74,131] requests per second against D(B)'s 318,935
  [316,259-328,027]. In `idle-memory`, D(B) made one call per task, at the park
  after its excursion, and none at the park after shallow use.
* **Default reservation: 1 MiB.** Under D(B) a reservation costs page tables,
  not resident memory: 1.75x, 2.41x and 3.16x Go (worst condition) at 128 KiB, 1
  MiB and 8 MiB. 1 MiB keeps a margin under the bar for the runtime's growth, is
  the size of the JVM's default thread stack, and runs 21,900 levels of a small
  recursion and 2,856 levels of the four-frame parser before a clean failure. 8
  MiB is admissible on memory (it completes the 10k-deep parse) and is the step
  to take if the corpus histogram finds ordinary programs deeper than 1 MiB;
  above 8 MiB, memory must be re-measured (the plan's exhaustion row).
* **Probes, not guard pages.** One guard page per task stops at 32,730 tasks
  under the default `vm.max_map_count`, and a guard-page trap discards the
  exhausted task's frames without running their destructors (86 drop guards
  skipped in the 1M-deep recursion at 1 MiB, none under probes). With probes
  every exhaustion unwinds cleanly. The plan attaches a condition to this
  choice, probe overhead of at most 3 % on compute loops, and that condition is
  open: policy 1 leaves it to the native era, and at face value `fib` and `tree`
  exceed it here (section 7). Two further conditions come with probes: the
  runtime must resolve the unwinder's lazily bound symbols before tasks run (or
  link them eagerly), which this phase found and fixed (section 10.3); and the
  native compiler's probe should compare the stack pointer minus the frame size
  before allocating the frame, so that the reserve covers only the unwinder and
  unprobed callees. With both, the policy's 5 KiB (5,120-byte) reserve leaves
  3,032 bytes over the largest need measured on M1.
* **`spawn` stays ambient**, without a capability. The research found no
  supply-chain attack that gating `spawn` would have stopped, and nothing in the
  harness argues otherwise: a task costs 0.67 [0.66-0.83] us to spawn and join
  and 6.43 KiB at rest, and the risk of code that spawns too many is a resource
  limit for the runtime to enforce (the scale row's guards), not an authority to
  grant.
* **Yield checks (Q-34)**, decided as compiler-inserted, need an interval:
  yielding every 100k iterations of the test loop (slices of about 202 [189-224]
  us) held the probe's p99 at 229 [221-269] us, and every 10k (about 21.3
  [20.3-22.0] us) at 25.7 [25.0-30.0] us, both at a throughput within the five
  runs' spread of the loop without yield points (median changes of +1.5 % and
  -2.9 %); every 1k iterations cost 18.1 %. The check on the back-edge is nearly
  free (section 7); the switches are the cost, and it falls with the slice. A
  slice of a few hundred microseconds meets the starvation row by two orders of
  magnitude, and the interval should be chosen to fit what the probes leave of
  the 3 % budget, which the native era measures.
* **The stackless fallback is not needed** on this evidence: the default
  candidate passes the memory primary on M1 with a wide margin and every
  exhaustion is clean. Its cost stays on file
  ([`stackless-fallback-estimate.md`](stackless-fallback-estimate.md): 17-21
  engineer-weeks, plus 3 if frames become values), for the two cases that would
  call for it: M2's full run contradicting M1, or the corpus histogram (week 40)
  finding ordinary depth beyond 8 MiB. The yield-point frame-boundary question
  is therefore not flagged to Phase 3.

### 9.3 What the harness era cannot settle

* **The 3 % probe budget.** Neither machine has hardware counters, and run to
  run the wall-clock overhead moves by more than the bar (section 7). The
  decision waits for the native era on bare metal.
* **The default reservation against real programs.** The harness shows what each
  size costs and how deep it goes; whether ordinary bet-1 and bet-2 programs fit
  1 MiB is the interpreter era's histogram.
* **Sisu's own frames.** Every number here is Rust code on native stacks. Frame
  sizes, where the compiler places probes and yield checks (before allocating a
  frame; on which back-edges; before or after vectorisation), how the unwinder
  or the runtime's own unwinding behaves, and what a Sisu task's record costs
  change these figures. The native era reruns this harness on Sisu frames, and
  only then is Q-6 closed finally (18: "closed provisionally with the harness
  era label ..., and finally when the native backend reruns the harness on Sisu
  frames").
* **How long a cancelled call may wait.** When a cancel cannot stop a read
  (`-EALREADY`), D-058 makes the call wait for the read's own completion, which
  keeps the buffer safe and can take as long as the I/O (section 8). Bounding
  that wait (re-issuing the cancel, poll-armed reads for pollable files) is a
  design question for the native runtime that the harness only observed.
* **The registered echo and connection cells**: 10k connections, 100k idle
  connections, an open-loop load generator on disjoint cores. None was run here.
* **The second machine at full scale**, and a second machine with 16 GB.

## 10. Deviations, amendments, gaps

### 10.1 What the owner's policies changed relative to the plan

The plan (18, Phase 1) asked for "a report with every configuration, not the
best one, reproduced on a second machine by the reviewer", and its threshold row
for probe overhead presupposes machines with hardware counters and a fixed
frequency; the pre-registration's section 4.4 asks for two machines with at
least 16 GiB, default sysctls, the scale test in a 16 GiB memory cgroup, and M2
run by the reviewer. The owner's decisions of October 2026 (README, "Policy
decisions") changed that, and this report follows them:

| Policy | The plan's original wording | What this report does instead |
|---|---|---|
| 1. Machines | two machines, the second the reviewer's, implicitly bare metal | M1 is a 4-vCPU KVM virtual machine without a PMU and with an unpinned frequency; M2 is a GitHub-hosted runner, driven by a workflow rather than by a reviewer. The 3 % probe bar is reported with its noise and not decided (section 7) |
| 2. Context switch | "an assembly context switch from an existing library" | corosensei 0.3.x, unchanged since the harness was built |
| 3. Sysctls and limits | default sysctls; the earlier full-scale script needed `RLIMIT_NOFILE` of 200,064 | default sysctls on M1. On M2 the new workflow restores the kernel default `vm.max_map_count=65530`, which the runner image raises to 262144 (the smoke run used here ran under 262144, so A reached its 100k cap there). The 10k echo and the 100k idle connections run only where the default hard limit allows; on M1 (hard limit 20000) the 10k echo is a failed row and idle connections ran at 9,900 |
| 4. Go after the excursion | "within 4x Go", the Go figure unspecified | Go after four `runtime.GC()` and `FreeOSMemory` |
| 5. Default strategy | none named | D(B) |
| 6. Probe reserve | the harness defaults of 16 KiB (B) and 8 KiB (C) | 5 KiB for B, C and D(B), now the harness default (version 0.2.0) |
| 7. 1M tasks after the excursion | every strategy | only D(A) and D(B); A, B, C and Go measure the shallow phase at 1M (`--skip-excursion`), and their cell reads "not run (policy 7)" |
| 8. Yield points | compute tasks "that yield every K iterations", Go with or without `Gosched` | compiler-inserted yield points every K = 10k and 100k iterations (and 1k); the baseline is Go with its default asynchronous preemption only; no yield points at all is reported, not counted |

### 10.2 Deviations from the harness protocol (proposal/20, section 6.5)

* **Machine 2 has no full-scale data yet.** M2's numbers are the CI smoke run of
  2026-10-13: 2 workers, one run per cell, harness 0.1.0 (the 16 KiB and 8 KiB
  reserves, no unwinder warm-up), `vm.max_map_count` 262144, 3 GiB of swap, and
  no Java (the runner's default JDK is 17). The workflow
  `.github/workflows/harness-full.yml` produces the full-scale M2 run when a
  `harness-run-*` tag is pushed; it installs JDK 21 and Go 1.24, restores the
  kernel's `vm.max_map_count`, turns swap off, sets transparent huge pages to
  `madvise` as on M1 (under the image's `always`, Go's goroutines held 66.4 KiB
  each after the excursion, which on an 8 GB runner would push Go's 100k cell
  into the memory guard), and uploads `results/github-runner/` as
  `taskbench-full-<tag>`.
* **M2 is smaller than the plan's machine.** This repository is private, and
  GitHub's `ubuntu-latest` runner for private repositories has 2 vCPUs and 8 GB
  (the smoke run's `machine-spec.csv`: 8,323,969,024 bytes); the 4-vCPU, 16 GB
  runner is for public repositories. On it the 1M-task cells stop at the memory
  guard and are recorded as failed rows, so M2 can reproduce every cell at 100k
  and below, but not the scale row or the 1M memory cells, unless the workflow
  runs on a 16 GB runner.
* **M1's memory limit** is the sandbox's memory cgroup of 13.4 GiB on a 15.7 GiB
  VM, not a 16 GiB cgroup on a machine of at least 16 GiB.
* **M1 is shared.** Other development work (compilations and tests elsewhere in
  the repository) ran on the VM during the runs, at times that were not
  recorded; section 6.5 assumes a machine that runs nothing else. The first runs
  of the four groups that measure time (spawn-join, starvation, probe-overhead,
  echo) were exposed to it: a listing of the compiler workspace's build
  directory made during the echo group showed files written in every ten-minute
  interval from 21:30 to 22:09 and from 22:20 to 22:43 UTC, which covers parts
  of the starvation, probe-overhead and echo groups (nothing is known about
  spawn-join's half minute at 19:54), and in the starvation group the first of
  B's five runs at K = 10k reached 1,357,540,204 iterations per second against a
  median of 1,794,087,833. Those four groups were run again with the quiet gate
  of `load.sh` (`--quiet-below 0.3`, section 11), and only the gated runs are
  reported; the first runs are kept in `superseded-before-quiet-gate/`. The
  memory, scale and correctness groups (deep-recursion, idle-memory, max-tasks,
  cancel-uring) were not run again: what they report does not depend on other
  processes' CPU use, apart from incidental timings (cancellation latencies,
  spawn rates) that are not verdict inputs. A first gated run of spawn-join was
  stopped after two repetitions because `load.sh` then left interrupt and
  softirq time out of the machine's busy time, which this kernel's tick
  accounting charges to the interrupted process, so a competing process would
  have been invisible during the echo benchmark; its rows are in
  `aborted-gated-rerun/`. The gated runs changed no verdict: B's spawn-join
  median is 1.40 times Go's (first runs: 1.48), B's starvation p99 at K = 10k
  25.7 us (first runs: 27.3), B's `fib` and `tree` probe overheads 13.3 % and
  4.5 % (first runs: 14.7 and 5.7), and D(B)'s echo p99 at 1000 connections 0.85
  times Go's (first runs: 0.87). What moved most is the throughput cost of yield
  points every 10k iterations, 2.9 % against 6.0 % in the first runs (section
  6).
* **Workers** are the VM's vCPUs (4 on M1, 2 on M2); section 4.4's default is
  one per physical core, which a VM does not expose.
* **Memory** is resident plus page tables (VmRSS + VmPTE from
  `/proc/self/status`), the harness's figure of record since its first version;
  section 6.5 says "resident set" from `/proc/self/smaps_rollup`. The RSS-only
  figures are in the CSVs and in `summarize.py`'s output, and every verdict is
  the same on them. The harness reads its baseline after runtime start-up
  without the 1,000-task warm-up of section 6.5; at most the first 1,000 tasks
  could find warm memory, so the effect is at most 1 % at 100k and conservative
  (it raises the per-task figure) for every implementation.
* **Echo** is a closed loop with the clients in the same process and on the same
  four vCPUs, measured for 30 s after 5 s of warm-up; section 6.5 asks for an
  open-loop load generator on disjoint cores at a fixed rate, 60 s after 10 s of
  warm-up, at 10k connections. The 1000-connection figures are the harness's
  comparison of the runtimes, not the registered measurement.
* **Starvation** runs four compute tasks per worker as section 6.5 asks, for 11
  s, which gives each taskbench run about 10k wake-ups; Go's probe wakes much
  later under load, so its runs have fewer wake-ups (section 6 lists them).
* **Probe overhead** is 31 alternating repetitions of every variant inside one
  process, five processes per cell; section 6.5 asks for ten runs of each. The
  `loop` workload (the yield check on a back-edge) is new and is not in section
  6.5's list.
* **A1** reads into a 4 KiB buffer on the task's stack from a pool of pipes that
  hold no data, cancels every read through its task handle, and checks the
  buffer twice: when the call returns, and again after enough "late" data has
  been written into every pipe for any read the kernel still held to complete,
  with the frame kept alive (and so never reused) until its batch of 8,192 ends.
  Section 6.5 writes a canary after the return and quarantines the memory;
  keeping the frame alive is the same check. "Completions that arrived after the
  call returned" cannot happen by construction in the safe API (`read` returns
  only on the completion) and are counted only in the negative control's
  `abandoned_reads`.
* **Controls.** Go is 1.24.7, not the latest stable release; the JDK is 21.0.12
  (JEP 491's unpinning is 24 and later). The "C or Rust stackful coroutine
  library" control is strategy A itself (corosensei's own default stack is a
  guarded mapping per coroutine), not a separate library.
* **Reviewer.** No Go/Java reviewer has configured or checked the controls, and
  no referee has applied the thresholds; the verdicts are the runtime engineer's
  reading of the draft rules.

### 10.3 Changes to the harness in this phase (version 0.2.0)

* **Policy defaults.** The probe reserve is 5 KiB for B, C and D(B)
  (`stack::DEFAULT_PROBE_RESERVE`); version 0.2.0 marks every CSV's
  `machine-spec.csv` (`harness_version`).
* **Bug: the io_uring ring at full scale.** `--concurrency 8192` asked the
  kernel for a completion queue of 131,072 entries, above its maximum of 65,536,
  so every full-scale `cancel-uring` run would have been a "skipped: io_uring
  unavailable" row. The ring sizes are now clamped to the kernel's limits
  (`rt::uring::ring_sizes`, with unit tests).
* **Bug: the first unwind ran the dynamic linker inside the probe reserve.**
  `libgcc_s` binds its calls into the C library lazily, and the resolver saves
  the full extended register state on the current stack: about 3 KiB on this
  AVX-512 machine. The first stack exhaustion of every process therefore needed
  about 5 KiB of reserve and later ones about 1.8-2.1 KiB; the earlier "5 KiB
  when landing pads run drop code" was this one-time cost, not drop code (with
  `LD_BIND_NOW=1` the first unwind also needs 1.8 KiB). M2's smoke run, made
  before the fix, shows the same pattern with a smaller register state: 3,416
  bytes for the first unwind and 1,816 for the next. With the 5 KiB default, the
  unit test `exhaustion_is_a_clean_task_failure_on_every_strategy` overran a B
  slot's reserve, and its canary caught it. The runtime now resolves those
  bindings once, on an ordinary stack, before it starts its workers
  (`stack::probe::warm_unwinder`).
* **Bug: A1's at-return check.** The check expected a completed read to hold the
  round's data byte, but a read that a cancel could not stop (`-EALREADY`)
  legitimately completes with the late data the harness writes at the round's
  deadline, and returns after it. The first full-scale `cancel-uring` group
  (five repetitions) therefore reported 29,184 bytes "after return" in 4 runs,
  all at return and none afterwards (`bad_late=0`), each in a run whose slowest
  cancellation took about 30 s. Late bytes within the length the read itself
  returned now count as data it delivered (`completed_with_late_data`), and the
  group was rerun. The superseded rows are kept in
  `results/vm/2026-10-13-phase1/superseded-cancel-uring-before-check-fix/`,
  outside `summarize.py`'s reach. The fix changed only
  `src/bench/cancel_uring.rs`: `versions.txt` shows the sources' digest
  `789d610d...` for deep-recursion and idle-memory (and the superseded first
  runs of spawn-join and cancel-uring), `51e42157...` for max-tasks and the
  cancel-uring rerun (and the superseded first runs of starvation,
  probe-overhead and echo), and `6a1fedf5...` for the gated groups, whose
  sources differ from `51e42157...` only in the scripts (`load.sh`,
  `run_all.sh`, `controls/run_controls.sh`): their `versions.txt` entries record
  the binary's digest, `5893a273...`, which is the binary cargo built from
  `51e42157...` at 20:50 UTC for max-tasks and did not rebuild afterwards.
* **New measurements for the registered protocol.** `idle-memory
  --skip-excursion` (policy 7); `max-tasks --hold-secs 60` (section 6.5's
  60-second survival); `cancel-uring --pipe-mix cancel` with 4 KiB buffers and
  the cancel-to-return latency (A1 as registered); `deep-recursion --framed`
  (256-byte frames with drop code in every frame); the `loop` workload of
  `probe-overhead` (policy 8); the SMT state, threads per core and hypervisor in
  `machine-spec`. The Go and Java controls take `--skip-excursion` and
  `--hold-secs` too.
* **The machine's load** (`load.sh`, sourced by `run_all.sh` and the controls):
  every benchmark process gets a row in `load.csv` with its wall-clock and CPU
  time and the CPU time the other processes used meanwhile (`/proc/stat`'s busy
  time of all CPUs, interrupts and softirqs included, less the process's own),
  and `--quiet-below CPUS` makes each process wait for a quiet machine and
  measures it again when the others were busy while it ran (the last of three
  attempts is kept whatever its load; discarded attempts keep their `load.csv`
  row and lose their CSV rows). `summarize.py` prints the load table of section
  11. The gate is off by default, and off in the M2 workflow.
* **Scripts.** `scale.sh` holds both scales' parameters for `run_all.sh` and the
  controls; `run_all.sh --reps N` (five at full scale) runs repetitions in
  rep-major order with each benchmark's controls right after it, and writes
  `timing.csv` and a record of every invocation in `versions.txt` (with a digest
  of the sources and of the binary); the controls run Go's starvation with its
  default preemption only, and record a connection count beyond the hard limit
  as a failed row. `summarize.py` reports medians and ranges over runs,
  bootstrap intervals, the policies' baselines and section 2.4's rule.

### 10.4 Not run, and why

| Cell | Machine | Why |
|---|---|---|
| Echo at 10k connections | M1 | both ends in one process need 2 x 10,000 + 64 = 20,064 descriptors; the hard `RLIMIT_NOFILE` is 20000 and cannot be raised (policy 3). A client in a second process would fit, and is not implemented |
| Idle connections at 100k | M1 | the same limit; run at 9,900, the most it allows |
| 1M tasks after the excursion for A, B, C and Go | M1 | policy 7; about 33 GiB |
| Everything at full scale | M2 | pending the `harness-run-*` tag; see 10.2 for what a 2-vCPU, 8 GB runner can reproduce |
| Java | M2 (smoke) | JDK 17 on the runner image; the full-scale workflow installs JDK 21 |
| Instruction counts for probe overhead | M1, M2 | no PMU on either machine |
| The semantics rows, the `sim` study, the corpus stack-depth histogram, interpreter starvation | both | interpreter era (Phases 6-9), not Phase 1 |

### 10.5 Measured on one machine only

Every full-scale cell (100k and 1M tasks, five runs, 1M cancelled reads, the
60-second hold, idle connections at 9,900, starvation with four tasks per
worker, the framed recursion, the back-edge yield check) is from M1 alone.

## 11. How the runs were made

On M1, each benchmark group was one invocation, all into the same directory:

```
./run_all.sh --scale full --reps 5 --machine-id vm --out results/vm/2026-10-13-phase1 --only <benchmark> [--quiet-below 0.3]
```

M1 is a development VM, and other work (compilations and tests elsewhere in the
repository) ran on it at times that were not recorded. No two benchmark
processes ran at once. The groups ran in this order: deep-recursion, spawn-join,
idle-memory, cancel-uring, max-tasks, cancel-uring again (after the fix of A1's
check), starvation, probe-overhead and echo; then, because the first runs of the
four groups that measure time had shared the VM with that work (section 10.2),
spawn-join, probe-overhead, starvation and echo again, and idle-connections, all
five with `--quiet-below 0.3`: before every process the script waited until the
other processes had used fewer than 0.3 CPUs over 2 s, and it measured a process
again (up to three attempts) when they had used 0.3 CPUs or more on average
while it ran, and at least 0.05 CPU seconds in all (five clock ticks, so that a
process of a few milliseconds is not measured again for one stray tick;
`load.sh`). Within a group the repetitions run in rep-major order (every
configuration and its controls once, then again). The idle-connections group's
runner was stopped by a two-hour limit on background jobs in the environment
that ran it, during the fifth repetition and after the harness's seven
configurations; that repetition's Go and Java runs were then made by hand with
the same gate (`controls/run_controls.sh ... --only idle-connections --no-build
--quiet-below 0.3`, described in `aborted-gated-rerun/why-aborted.txt`), so
`timing.csv` has four repetitions of that group and every cell has five runs.
Wall-clock time per group, from the `timing.csv` files of the results directory
and its subdirectories:

| Benchmark (with its controls) | runs | first start (UTC) | repetitions | minutes | seconds per repetition |
|---|---|---|---|---|---|
| deep-recursion | kept | 2026-10-13 19:54 | 5 | 0.1 | 0-1 |
| spawn-join | superseded (no quiet gate, section 10.2) | 2026-10-13 19:54 | 5 | 0.6 | 6-7 |
| idle-memory | kept | 2026-10-13 19:54 | 5 | 24.0 | 270-313 |
| cancel-uring | superseded (A1's check, section 10.3) | 2026-10-13 20:18 | 5 | 30.8 | 335-388 |
| max-tasks | kept | 2026-10-13 20:50 | 5 | 40.6 | 484-494 |
| cancel-uring | kept | 2026-10-13 21:30 | 5 | 28.6 | 308-388 |
| starvation | superseded (no quiet gate, section 10.2) | 2026-10-13 21:59 | 5 | 21.1 | 252-253 |
| probe-overhead | superseded (no quiet gate, section 10.2) | 2026-10-13 22:20 | 5 | 6.2 | 74-75 |
| echo | superseded (no quiet gate, section 10.2) | 2026-10-13 22:26 | 5 | 26.8 | 321-322 |
| spawn-join | aborted (load accounting, section 10.2) | 2026-10-13 23:08 | 2 | 1.3 | 36-43 |
| spawn-join | kept, gated | 2026-10-13 23:12 | 5 | 3.1 | 35-44 |
| probe-overhead | kept, gated | 2026-10-13 23:16 | 5 | 10.2 | 99-196 |
| starvation | kept, gated | 2026-10-13 23:26 | 5 | 37.8 | 384-513 |
| echo | kept, gated | 2026-10-14 00:04 | 5 | 39.4 | 354-754 |
| idle-connections | kept, gated | 2026-10-14 00:43 | 4 | 22.3 | 325-359 |
| total of the kept groups |  |  |  | 206.1 |  |
| total, superseded and aborted runs included |  |  |  | 292.8 |  |

The machine's load during every process of the gated groups, from `load.csv`
(the other processes' CPU time is all CPUs' busy time from `/proc/stat` less the
process's own; it can be a few ticks below zero):

**Machine load during the runs (load.csv)**

| results | benchmark | quiet below (CPUs) | processes kept | attempts measured again | gate timeouts | kept although busy | others' CPUs while kept: median | max | steal (CPUs): median | gate wait (s): total / longest |
|---|---|---|---|---|---|---|---|---|---|---|
| vm/2026-10-13-phase1 | echo | 0.3 | 80 | 7 | 0 | 2 | 0.015 | 0.707 | 0.040 | 504 / 88 |
| vm/2026-10-13-phase1 | idle-connections | 0.3 | 45 | 9 | 0 | 0 | 0.038 | 0.291 | 0.003 | 167 / 36 |
| vm/2026-10-13-phase1 | probe-overhead | 0.3 | 25 | 10 | 0 | 0 | 0.067 | 0.285 | 0.012 | 99 / 16 |
| vm/2026-10-13-phase1 | spawn-join | 0.3 | 70 | 4 | 0 | 0 | 0.034 | 0.349 | 0.009 | 150 / 4 |
| vm/2026-10-13-phase1 | starvation | 0.3 | 105 | 34 | 0 | 2 | 0.042 | 1.008 | 0.034 | 588 / 76 |

The other work came in bursts: the gate measured a process again 64 times. Four
processes were kept on their third attempt although the other processes had used
more than 0.3 CPUs while they ran (the last attempt is kept whatever its load):
two starvation runs at K = 10k (C and D(A)) and two echo runs at 1000
connections (A and C). The two echo runs have the highest p99 of their cells,
and the two starvation runs lie within their cells' ranges, far under the
threshold; none of these cells decides a verdict.

M2's full run is `.github/workflows/harness-full.yml`, triggered by pushing a
tag `harness-run-*`; it runs `./run_all.sh --scale full --machine-id
github-runner --out results/github-runner/full-<tag>` without the gate (a hosted
runner runs nothing else) and uploads `taskbench/results/github-runner/` as the
artifact `taskbench-full-<tag>`.
