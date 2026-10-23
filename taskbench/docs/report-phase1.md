# Phase 1 report: the bet-3 stack harness

*Era label for every number in this report: **harness** (Rust code on native
stacks, standing in for compiled Sisu). Filled from
[`report-template.md`](report-template.md). Every number comes from the files in
the two machines' full-scale results directories of section 0: the benchmark
CSVs, `machine-spec.csv`, `versions.txt`, `controls-versions.txt`, `timing.csv`
and `load.csv`. `summarize.py --markdown` on those directories prints the
tables' values (medians and ranges over runs, ratios, bootstrap intervals, and
section 2.4's rule on each machine and on both), and the few figures taken from
a row's `note` column (decommit counts, limits) say so where they are used. The
report itself is rendered from
[`report/report-phase1.md.in`](../report/report-phase1.md.in) by
`report/render.py`, which computes every figure in it from those files. Two
labelled exceptions: machine 2's earlier CI smoke run, whose own CSVs supply
three measurements and the runner image's settings before the workflow changes
them, and the cross-check of section 10.2, which quotes the log summary of a run
whose CSVs were lost. A cell that was not run says so, with the reason; a cell
that the memory guard stopped is a failed row and is reported as one. KiB = 1024
bytes; "M1" is machine 1, "M2" machine 2.*

**Status of these verdicts.** The pre-registration
([`proposal/20`](../../proposal/20-pre-registration.md)) is a draft (version
1.0, "not frozen"), and its early harness protocol has no frozen commit either.
Its section 1.3, rule 3, a proposed default, would make harness runs made before
the freeze development runs. The verdicts below apply the draft's thresholds and
rules, as they stand at commit `e93cd8a`, to both machines' full-scale runs.
They remain provisional until the pre-registration is frozen. M2 is also smaller
than section 4.4's machine (7.75 GiB rather than at least 16 GiB, section 10.2),
so some of its 1M-task cells could not be measured.

## Summary

| Bet-3 row (role in proposal/20, 7.8) | Threshold | M1: this VM, full scale, five runs per cell | M2: GitHub-hosted runner, full scale, five runs per cell | Verdict (both machines) |
|---|---|---|---|---|
| Memory per idle task (primary) | every one of the four conditions within 4x Go on both machines; after the excursion, Go after its GC (policy 4) | D(B) passes at all three reservations: worst ratio w = 1.75 (128 KiB), 2.41 (1 MiB), 3.16 (8 MiB). B and C fail after the excursion (6.86x and 7.08x Go after GC at 100k); A and D(A) cannot create 100k tasks (VMA limit at 32,730) | D(B) passes at 128 KiB: w = 1.75. At 1 MiB and 8 MiB every condition M2 measured is within 4x (worst 2.41 and 3.16, at 100k), but the 1M cells are failed rows: the memory guard stopped them at 950,272-962,560 and 712,704-733,184 of 1,000,000 tasks. B and C fail after the excursion (6.94x and 7.15x); A and D(A): VMA limit at 32,738 | **holds** (provisional): D(B) at 128 KiB passes on both machines (w = 1.75 on both machines), and no configuration passes on one machine and fails on the other. At the proposed 1 MiB default, section 2.4 is inconclusive on M2 (too little data at 1M), so that reservation's 1M cells rest on M1 alone |
| Stack exhaustion (guard) | no segfault; ordinary corpus depth within the default reservation | every exhaustion clean in 17 configurations x 5 runs; 0 crashes; at most 2,088 bytes of the 5 KiB reserve used | the same: every exhaustion clean in 17 configurations x 5 runs (0 unclean outcomes); 0 crashes; at most 2,088 bytes of the reserve | harness part **holds** on both machines; the guard itself is decided by the corpus histogram (interpreter era, week 40) |
| Spawn plus join (secondary) | median within 3x Go with pooled stacks | B 1.40x Go (0.64 [0.64-0.80] us against 0.46 [0.41-0.55] us) | B 1.84x Go (0.73 [0.52-0.73] us against 0.39 [0.39-0.41] us) | **holds** on both machines, which differ by 0.44 (conservative ranges 1.15-1.92 and 1.26-1.86, all below 3); no design review |
| Scale, 1M idle tasks on 16 GiB, default sysctls (secondary) | fail: guard-page-per-task is out | B, C, D(B), E, tokio, Go and Java park 1M tasks and keep them parked 60 s; A and D(A) stop at 32,730 (`vm.max_map_count` 65530) | the same: 1M tasks parked through the hold for B, C, D(B), E, tokio, Go and Java; A and D(A) stop at 32,738 (`vm.max_map_count` 65530), on a 7.75 GiB machine | **fails for guard pages per task** (A, D(A)) on both machines: pooled stacks with software probes are the design, as policy 5 already says; holds for B, C and D(B) |
| Probe overhead on compute loops (with the scale row) | at most 3 % | `split` model: fib 13.3 [11.9-16.3] %, calls 0.0 [-0.1-0.9] %, tree 4.5 [1.9-5.3] %, back-edge yield check -0.1 [-0.2-0.5] % (B) | fib 8.6 [8.2-8.7] %, calls 0.2 %, tree 2.2 [1.8-2.3] %, back-edge yield check 3.5 % (B) | **not decided** (policy 1: no PMU on either machine); taken at face value `fib` exceeds 3 % on both machines, `tree` on M1 only and the back-edge check on M2 only, a warning for the native era (section 7) |
| Echo p99 at 10k connections (secondary) | within 2x Go | not run: both ends in one process need 20,064 descriptors, the hard limit is 20,000. At 1000 connections D(B) 0.85x and B 0.88x Go | D(B) 0.91x and B 0.91x Go; every stackful strategy at most 1.07x. At 1000 connections D(B) 0.87x and B 0.79x | **not shown**: the 10k cell exists on M2 only, where it is within 2x; M1's descriptor limit cannot run it (policy 3) |
| Idle RSS at 100k connections | reported | at 9,900 connections: D(B) 10.6 KiB against Go 7.06 [7.02-7.16] KiB | at 100,000 connections: D(B) 10.6 KiB against Go 5.41 [5.33-5.47] KiB; A and D(A) failed (the client's `connect` timed out) | reported (no threshold); the registered N on M2 only |
| Starvation (secondary) | no worse than Go (default preemption) and p99 under 10 ms | yield points every 10k iterations: B p99 25.7 [25.0-30.0] us; every 100k: 229 [221-269] us; Go 39.3 [39.3-39.4] ms | every 10k: B 23.6 [23.3-23.7] us; every 100k: 316 [315-316] us; Go 39.2 ms | **holds** on both machines with compiler-inserted yield points; without them the probe starves (reported, not counted) |
| Cancellation with a lent buffer, A1 (secondary) | 0 buffers written after their cancelled read returned | 0 bytes over 25 runs of 1M reads each, every read cancelled in flight, on A, B, C, D(A) and D(B), and 0 in the race mix; the negative control sees 195,718,784 bytes | 0 bytes over 25 runs of 1M cancelled reads, and 0 in the race mix; the negative control sees 192,795,904 bytes | **holds** on both machines |

**Q-6, provisional (harness era).** Strategy D(B), pooled stacks with software
probes and decommit on park; default reservation 1 MiB, whose 1M-task cells only
M1 could hold (M2 confirms it at 100k, and 128 KiB is the reservation that
section 2.4 shows on both machines); probes, not guard pages; `spawn` ambient,
without a capability; the stackless fallback is not needed. Section 9 gives the
reasons and the conditions.

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
| CPU model, cores used (`--threads`) | Intel(R) Xeon(R) Processor @ 2.10GHz (family 6, model 207), a KVM guest (CPUID `KVMKVMKVM`), 4 vCPUs, 1 thread per core, no SMT; `--threads 4` | Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz, a Hyper-V guest (CPUID `Microsoft Hv`), 2 vCPUs that are one core's two hardware threads (`threads_per_core` 2, SMT `active=1 control=on`), governor `performance`; `--threads 2` |
| memory, swap | 16,874,930,176 bytes (15.7 GiB); memory cgroup limit 14,343,688,192 bytes (13.4 GiB); no swap | 8,322,932,736 bytes (7.75 GiB); no swap (the workflow turns it off); memory cgroup limit 12,884,901,888 bytes (12.0 GiB), the workflow's scope, above the machine's memory (see below); pids cgroup 9491 |
| kernel (`uname`) | Linux 6.18.44-fc-v114, Ubuntu 24.04.5 LTS userland | Linux 6.17.0-1022-azure, Ubuntu 24.04.5 LTS |
| `vm.max_map_count`, `kernel.threads-max`, `kernel.pid_max` | 65530 (kernel default), 128599, 32768 | 65530 (the kernel default, restored by the workflow; the image sets 262144), 63275, 4194304 |
| `vm.overcommit_memory`, THP `enabled`/`defrag` | 0, `madvise`/`madvise` | 0, `madvise`/`madvise` (the workflow sets `enabled` to `madvise`; the image's is `always`) |
| `RLIMIT_NOFILE` soft/hard, `RLIMIT_NPROC` | 20000/20000, 64299 | 1048576/1048576 inside the workflow's scope (the runner's own processes had 65536/65536 in the smoke run), 31637 |
| io_uring | enabled (`kernel.io_uring_disabled` 0) | enabled (`kernel.io_uring_disabled` 0) |
| hardware counters (PMU) | none: "hardware counters unavailable: No such file or directory (os error 2)" | none: "hardware counters unavailable: Permission denied (os error 13)" |
| rustc, taskbench version | rustc 1.97.0 (2d8144b78 2026-07-07); taskbench 0.2.0, sources sha256 `789d610d...`, `51e42157...` and `6a1fedf5...`, the last differing from the second only in the scripts (`versions.txt`, section 10.3) | rustc 1.99.0 (b940084d7 2026-09-28); taskbench 0.2.0, sources sha256 `6a1fedf5...`, the digest of M1's gated groups; binary `d402ccbd...` (another compiler, another binary) |
| Go version, `GOMAXPROCS` | go1.24.7, 4 | go1.24.13, 2 |
| JDK version, flags, warm-up | OpenJDK 21.0.12.1 (build 21.0.12.1+1-1-24.04.4-Ubuntu); `-Djdk.virtualThreadScheduler.parallelism=4 -Djdk.virtualThreadScheduler.maxPoolSize=4 -XX:+UseG1GC`; spawn-join Go 10000 rounds, Java 20000 rounds (JIT); echo 5 s unrecorded (`controls-versions.txt`) | OpenJDK 21.0.12.1 (Temurin-21.0.12.1+1, build 21.0.12.1+1-LTS), installed by the workflow; `-Djdk.virtualThreadScheduler.parallelism=2 -Djdk.virtualThreadScheduler.maxPoolSize=2 -XX:+UseG1GC`; spawn-join Go 10000 rounds, Java 20000 rounds (JIT); echo 5 s unrecorded |
| results directory and run ids | [`results/vm/2026-10-13-phase1/`](../results/vm/2026-10-13-phase1/): 730 processes (run ids in every row), five per cell; `timing.csv` has each repetition's start and duration, `load.csv` the machine's load during every process of the gated groups (section 11); superseded and aborted runs are in subdirectories, each with a note | [`results/github-runner/full-38518035636/`](../results/github-runner/full-38518035636/): the `harness-full` workflow's run 38518035636, after which the workflow names the directory; 730 processes (run ids in every row), five per cell; `timing.csv`, `load.csv` (every process, no quiet gate), `versions.txt` and `controls-versions.txt`. Unlike the smoke run's directory it has no `artifact-provenance.txt` with the artifact's id and digest |

Harness commit: the M1 runs used the working tree on top of commit `e93cd8a`,
with the changes listed in section 10.3; the coordinator commits them with these
results, and `versions.txt` carries the digest of the sources the binary and the
controls were built from: `789d610d...` for deep-recursion and idle-memory,
`51e42157...` (the same sources plus the fix of A1's check, section 10.3) for
max-tasks and cancel-uring, and `6a1fedf5...` (the same harness with the
machine-load record and the quiet gate of section 11, which change only the
scripts) for spawn-join, starvation, probe-overhead, echo and idle-connections.
M2's run records no commit (`versions.txt`: "not recorded (working tree; see
source_sha256)"), because the workflow's memory scope passes only `PATH`,
`HOME`, `GITHUB_RUN_ID`, `JAVA_HOME` and `GOROOT` to `run_all.sh`, not
`GITHUB_SHA`. Its source digest, `6a1fedf5...`, is the one M1's gated groups
were built from, so M2 ran the harness, the controls and the scripts of those
groups, which include the fix of A1's check.

**M2's memory scope.** M2's run executed inside `sudo systemd-run --scope -p
MemoryMax=12G -p MemorySwapMax=0` (`.github/workflows/harness-full.yml`). The
workflow's comment gives the reason: a hosted runner has no memory cgroup, and
the 1M-task cells of an earlier attempt without a cap, run 38513665561, starved
the runner agent, which lost the run; the 12 GiB scope was to leave the agent
its memory and let the harness's memory guard turn an over-large cell into a
failed row. On this runner the cap could not bind: `machine-spec.csv` records
the scope's limit, 12,884,901,888 bytes, and `MemTotal`, 8,322,932,736 bytes, so
the cap is 4.25 GiB above the machine's memory. The guard stops a cell when the
smaller of `MemAvailable` and the cgroup's headroom falls below its margin (1
GiB in `idle-memory`, 512 MiB in `max-tasks`), and with the cap above the
machine's memory the smaller is `MemAvailable`: the cells the guard stopped were
stopped by the size of the machine, not by the cap. They are failed rows,
reported as such, all in `idle-memory` at 1M tasks: B/1 MiB at 946,176-962,560
tasks and B/8 MiB at 675,840-733,184 (the shallow phase, the only one policy 7
requires of B at 1M), and D(B)/1 MiB at 950,272-962,560 and D(B)/8 MiB at
712,704-733,184 of 1,000,000 (the shallow phase stopped there, and nearly every
task then skipped its excursion; the ranges are over the five runs). No other
cell met the guard: every other `idle-memory` cell completed on M2, and
`max-tasks` parked 1M tasks for every pooled strategy. On this runner the cap
therefore added no protection beyond the guards' own margins, which are all that
the lost run 38513665561 had as well; this run completed. Two other limits of
the scope shape M2's results: its processes had an `RLIMIT_NOFILE` of
1048576/1048576, where the runner's own processes had 65536/65536 in the smoke
run (section 10.2), and its pids cgroup allowed 9491 tasks (the smoke run's,
outside any scope, 9490), which stopped OS threads at 9,489 (section 3).

**M2's other runs.** The CI smoke run of 2026-10-13
([`results/github-runner/2026-10-13-smoke-b5349dd405e8/`](../results/github-runner/2026-10-13-smoke-b5349dd405e8/):
harness 0.1.0 at commit `b5349dd405e8`, one run per cell at smoke scale, on an
AMD EPYC 7763 64-Core Processor with 2 vCPUs) no longer supplies any of M2's
columns or verdict figures. It is cited only for what the full run cannot show,
each labelled where it is used: the runner image's own settings, which the
workflow changes (`vm.max_map_count`, transparent huge pages, and the descriptor
and pids limits outside the scope; this section), Go under the image's
transparent huge pages `always` (section 1), the first unwind before the
unwinder fix (section 5), and C under the old 8 KiB reserve (section 1). Two
full-scale attempts before run 38518035636 were lost: run 38495423545 completed,
but its artifact was not uploaded (the artifact's name, taken from the branch,
contained a slash), and the summary captured from its job log,
[`full-38495423545-log-summary.md`](../results/github-runner/full-38495423545-log-summary.md),
is used only for the cross-check of section 10.2, not as data of record; run
38513665561, made without a memory cap, was lost as described above.

Pre-registration commit: none yet. Thresholds are copied from proposal/20 at
commit `e93cd8a` (draft 1.0, not frozen); there are no amendments beyond A1-A6,
which are in that text.

Every table can be regenerated with `python3 summarize.py --markdown
results/vm/2026-10-13-phase1 results/github-runner/full-38518035636` (medians
[min-max] over runs, ratios of medians, 95 % percentile bootstrap intervals over
runs, and section 2.4's rule on each machine and on both).

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
uses. Both machines ran every cell five times at 100k and at 1M tasks; where
M2's memory guard stopped a cell, the table shows the stop and the range of
tasks created over the runs.

*100,000 tasks (KiB per task)*

| Configuration | shallow M1 | shallow M2 | after 32 KiB excursion M1 | after 32 KiB excursion M2 | shallow / Go M1 | shallow / Go M2 | excursion / Go after GC M1 | excursion / Go after GC M2 |
|---|---|---|---|---|---|---|---|---|
| A, 128 KiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| A, 1 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| A, 8 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| B, 128 KiB | 4.68 [4.68-4.68] | 4.69 [4.69-4.69] | 32.7 [32.7-32.7] | 32.7 [32.7-32.7] | 1.73 [1.73-1.75] | 1.75 [1.74-1.75] | 6.51 [6.51-6.53] | 6.58 [6.52-6.63] |
| B, 1 MiB | 6.44 [6.44-6.44] | 6.44 [6.44-6.44] | 34.4 [34.4-34.4] | 34.4 [34.4-34.4] | 2.38 [2.38-2.40] | 2.41 [2.39-2.41] | 6.86 [6.86-6.88] | 6.94 [6.87-6.99] |
| B, 8 MiB | 8.45 [8.45-8.45] | 8.45 [8.45-8.46] | 36.5 [36.5-36.5] | 36.6 [36.6-36.6] | 3.13 [3.13-3.15] | 3.16 [3.14-3.16] | 7.28 [7.28-7.30] | 7.36 [7.29-7.42] |
| C, 4 KiB (+32 KiB: 36 KiB) | 5.50 [5.50-5.50] | 5.50 [5.50-5.50] | 35.5 [35.5-35.5] | 35.5 [35.5-35.5] | 2.04 [2.03-2.05] | 2.06 [2.04-2.06] | 7.08 [7.08-7.09] | 7.15 [7.08-7.20] |
| C, 8 KiB (+32 KiB: 40 KiB) | 5.50 [5.50-5.50] | 5.51 [5.51-5.51] | 35.5 [35.5-35.5] | 35.5 [35.5-35.5] | 2.04 [2.04-2.06] | 2.06 [2.05-2.06] | 7.08 [7.08-7.09] | 7.15 [7.08-7.21] |
| C, 16 KiB (+32 KiB: 48 KiB) | 5.52 [5.52-5.52] | 5.53 [5.53-5.53] | 35.5 [35.5-35.5] | 35.5 [35.5-35.5] | 2.04 [2.04-2.06] | 2.06 [2.05-2.07] | 7.08 [7.08-7.09] | 7.16 [7.09-7.21] |
| D(A), 128 KiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| D(A), 1 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| D(A), 8 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| D(B), 128 KiB | 4.68 [4.68-4.68] | 4.69 [4.69-4.69] | 4.69 [4.69-4.69] | 4.69 [4.69-4.69] | 1.73 [1.73-1.75] | 1.75 [1.74-1.75] | 0.93 [0.93-0.94] | 0.95 [0.94-0.95] |
| D(B), 1 MiB | 6.44 [6.44-6.44] | 6.44 [6.44-6.44] | 6.44 [6.44-6.44] | 6.45 [6.45-6.45] | 2.38 [2.38-2.40] | 2.41 [2.39-2.41] | 1.28 [1.28-1.29] | 1.30 [1.29-1.31] |
| D(B), 8 MiB | 8.45 [8.45-8.45] | 8.45 [8.45-8.46] | 8.45 [8.45-8.46] | 8.46 [8.46-8.46] | 3.13 [3.13-3.15] | 3.16 [3.14-3.16] | 1.68 [1.68-1.69] | 1.70 [1.69-1.72] |
| E (stackless) | 0.64 [0.64-0.64] | 0.64 [0.64-0.64] | 0.64 [0.64-0.65] | 0.65 [0.64-0.65] | 0.24 | 0.24 | 0.13 | 0.13 |
| tokio | 0.40 [0.40-0.40] | 0.40 [0.40-0.40] | 0.40 [0.40-0.41] | 0.40 [0.40-0.41] | 0.15 | 0.15 | 0.08 | 0.08 |
| Go | 2.70 [2.68-2.70] | 2.68 [2.68-2.69] | 34.7 [33.4-34.7]; after GC 5.02 [5.01-5.02] | 34.4 [23.2-35.9]; after GC 4.96 [4.93-5.01] | 1.00 [0.99-1.01] | 1.00 [0.99-1.01] | 6.92 [6.66-6.93] | 6.93 [4.62-7.28] |
| Java virtual threads | 1.93 [1.44-1.94] | 1.89 [1.79-2.65] | 3.16 [3.12-3.54]; heap 1.73 [1.16-1.82] | 3.23 [2.91-3.67]; heap 1.95 [1.72-2.45] | 0.72 [0.53-0.72] | 0.70 [0.67-0.99] | 0.63 [0.62-0.71] | 0.65 [0.58-0.74] |

*1,000,000 tasks (KiB per task)*

| Configuration | shallow M1 | shallow M2 | after 32 KiB excursion M1 | after 32 KiB excursion M2 | shallow / Go M1 | shallow / Go M2 | excursion / Go after GC M1 | excursion / Go after GC M2 |
|---|---|---|---|---|---|---|---|---|
| A, 128 KiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | not run (policy 7) | not run (policy 7) |  |  |  |  |
| A, 1 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | not run (policy 7) | not run (policy 7) |  |  |  |  |
| A, 8 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | not run (policy 7) | not run (policy 7) |  |  |  |  |
| B, 128 KiB | 4.68 [4.68-4.68] | 4.68 [4.68-4.68] | not run (policy 7) | not run (policy 7) | 1.75 [1.73-1.77] | 1.74 [1.73-1.74] |  |  |
| B, 1 MiB | 6.43 [6.43-6.43] | fails: memory guard at 946,176-962,560 of 1,000,000 tasks | not run (policy 7) | not run (policy 7) | 2.41 [2.38-2.43] |  |  |  |
| B, 8 MiB | 8.45 [8.45-8.45] | fails: memory guard at 675,840-733,184 of 1,000,000 tasks | not run (policy 7) | not run (policy 7) | 3.16 [3.13-3.19] |  |  |  |
| C, 4 KiB (+32 KiB: 36 KiB) | 5.50 [5.50-5.50] | 5.50 [5.50-5.50] | not run (policy 7) | not run (policy 7) | 2.06 [2.03-2.07] | 2.04 |  |  |
| C, 8 KiB (+32 KiB: 40 KiB) | 5.50 [5.50-5.50] | 5.50 [5.50-5.50] | not run (policy 7) | not run (policy 7) | 2.06 [2.04-2.08] | 2.04 |  |  |
| C, 16 KiB (+32 KiB: 48 KiB) | 5.52 [5.52-5.52] | 5.52 [5.52-5.52] | not run (policy 7) | not run (policy 7) | 2.07 [2.04-2.08] | 2.05 [2.04-2.05] |  |  |
| D(A), 128 KiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| D(A), 1 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| D(A), 8 MiB | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks | fails: VMA limit at 32,730 tasks | fails: VMA limit at 32,738 tasks |  |  |  |  |
| D(B), 128 KiB | 4.68 [4.68-4.68] | 4.68 [4.68-4.68] | 4.68 [4.68-4.68] | 4.69 [4.69-4.69] | 1.75 [1.73-1.77] | 1.74 [1.73-1.74] | 0.93 [0.93-0.94] (vs Go after GC at 100k); 1.75 [1.73-1.77] (vs Go shallow at 1M) | 0.94 [0.93-0.95] (vs Go after GC at 100k); 1.74 (vs Go shallow at 1M) |
| D(B), 1 MiB | 6.43 [6.43-6.43] | fails: memory guard at 950,272-962,560 of 1,000,000 tasks | 6.44 [6.44-6.44] | fails: memory guard at 950,272-962,560 of 1,000,000 tasks | 2.41 [2.38-2.43] |  | 1.28 [1.28-1.29] (vs Go after GC at 100k); 2.41 [2.38-2.43] (vs Go shallow at 1M) |  |
| D(B), 8 MiB | 8.45 [8.45-8.45] | fails: memory guard at 712,704-733,184 of 1,000,000 tasks | 8.45 [8.45-8.45] | fails: memory guard at 712,704-733,184 of 1,000,000 tasks | 3.16 [3.13-3.19] |  | 1.68 [1.68-1.69] (vs Go after GC at 100k); 3.16 [3.13-3.19] (vs Go shallow at 1M) |  |
| E (stackless) | 0.63 [0.63-0.63] | 0.63 [0.63-0.63] | 0.64 [0.64-0.64] | 0.64 [0.64-0.64] | 0.24 [0.23-0.24] | 0.23 [0.23-0.24] | 0.13 (vs Go after GC at 100k); 0.24 (vs Go shallow at 1M) | 0.13 (vs Go after GC at 100k); 0.24 (vs Go shallow at 1M) |
| tokio | 0.40 [0.40-0.40] | 0.40 [0.40-0.40] | 0.40 [0.40-0.40] | 0.40 [0.40-0.40] | 0.15 | 0.15 | 0.08 (vs Go after GC at 100k); 0.15 (vs Go shallow at 1M) | 0.08 (vs Go after GC at 100k); 0.15 (vs Go shallow at 1M) |
| Go | 2.67 [2.65-2.70] | 2.70 [2.70-2.70] | not run (policy 7) | not run (policy 7) | 1.00 [0.98-1.02] | 1.00 |  |  |
| Java virtual threads | 1.07 [1.00-1.14] | 1.06 [1.04-1.17] | 2.00 [1.81-2.01]; heap 0.92 [0.91-0.93] | 1.81 [1.76-1.92]; heap 0.93 [0.91-1.13] | 0.40 [0.37-0.43] | 0.39 [0.39-0.43] | 0.40 [0.36-0.40] (vs Go after GC at 100k); 0.75 [0.67-0.76] (vs Go shallow at 1M) | 0.37 [0.35-0.39] (vs Go after GC at 100k); 0.67 [0.65-0.71] (vs Go shallow at 1M) |

Section 2.4 of proposal/20, applied to each machine and then to both: each
stackful configuration's worst ratio w over the four conditions (at 1M after the
excursion, where Go's own cell fits neither machine, the larger of the ratios to
Go after GC at 100k and to Go shallow at 1M, a lower bound of Go after GC at
1M). A configuration passes if w is at or below 4 on both machines, fails if it
is above 4 on both, and is inconclusive otherwise; a cell whose conservative
range crosses 4, or that the memory guard stopped, makes it inconclusive on that
machine.

| Configuration | M1 | M2 | section 2.4 (passes or fails only if both machines agree) |
|---|---|---|---|
| A/128K | fails | fails | fails |
| A/1M | fails | fails | fails |
| A/8M | fails | fails | fails |
| B/128K | fails (w = 6.51) | fails (w = 6.58) | fails |
| B/1M | fails (w = 6.86) | fails (w = 6.94) | fails |
| B/8M | fails (w = 7.28) | fails (w = 7.36) | fails |
| C/36K | fails (w = 7.08) | fails (w = 7.15) | fails |
| C/40K | fails (w = 7.08) | fails (w = 7.15) | fails |
| C/48K | fails (w = 7.08) | fails (w = 7.16) | fails |
| D(A)/128K | fails | fails | fails |
| D(A)/1M | fails | fails | fails |
| D(A)/8M | fails | fails | fails |
| D(B)/128K | passes (w = 1.75) | passes (w = 1.75) | passes |
| D(B)/1M | passes (w = 2.41) | inconclusive (worst measured 2.41; the memory guard stopped a cell) | inconclusive |
| D(B)/8M | passes (w = 3.16) | inconclusive (worst measured 3.16; the memory guard stopped a cell) | inconclusive |

The four conditions on each machine (ratio to Go, its 95 % bootstrap interval
over the runs, and the conservative range):

**M1**

| Configuration | 100k shallow | 100k after excursion | 1M shallow | 1M after excursion | on this machine |
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

**M2**

| Configuration | 100k shallow | 100k after excursion | 1M shallow | 1M after excursion | on this machine |
|---|---|---|---|---|---|
| A/128K | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | not required (policy 7) | fails |
| A/1M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | not required (policy 7) | fails |
| A/8M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | not required (policy 7) | fails |
| B/128K | 1.75 (CI 1.74-1.75; range 1.74-1.75) vs Go | 6.58 (CI 6.52-6.63; range 6.52-6.63) vs Go after GC | 1.74 (CI 1.73-1.74; range 1.73-1.74) vs Go | not required (policy 7) | fails (w = 6.58) |
| B/1M | 2.41 (CI 2.39-2.41; range 2.39-2.41) vs Go | 6.94 (CI 6.87-6.99; range 6.87-6.99) vs Go after GC | not computable: memory guard at 946,176-962,560 of 1,000,000 tasks | not required (policy 7) | fails (w = 6.94) |
| B/8M | 3.16 (CI 3.14-3.16; range 3.14-3.16) vs Go | 7.36 (CI 7.29-7.42; range 7.29-7.42) vs Go after GC | not computable: memory guard at 675,840-733,184 of 1,000,000 tasks | not required (policy 7) | fails (w = 7.36) |
| C/36K | 2.06 (CI 2.04-2.06; range 2.04-2.06) vs Go | 7.15 (CI 7.08-7.2; range 7.08-7.2) vs Go after GC | 2.04 (CI 2.04-2.04; range 2.04-2.04) vs Go | not required (policy 7) | fails (w = 7.15) |
| C/40K | 2.06 (CI 2.05-2.06; range 2.05-2.06) vs Go | 7.15 (CI 7.08-7.21; range 7.08-7.21) vs Go after GC | 2.04 (CI 2.04-2.04; range 2.04-2.04) vs Go | not required (policy 7) | fails (w = 7.15) |
| C/48K | 2.06 (CI 2.05-2.07; range 2.05-2.07) vs Go | 7.16 (CI 7.09-7.21; range 7.09-7.21) vs Go after GC | 2.05 (CI 2.04-2.05; range 2.04-2.05) vs Go | not required (policy 7) | fails (w = 7.16) |
| D(A)/128K | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails |
| D(A)/1M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails |
| D(A)/8M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails |
| D(B)/128K | 1.75 (CI 1.74-1.75; range 1.74-1.75) vs Go | 0.945 (CI 0.936-0.952; range 0.936-0.952) vs Go after GC | 1.74 (CI 1.73-1.74; range 1.73-1.74) vs Go | 0.944 (CI 0.934-0.951; range 0.934-0.951) vs Go after GC at 100k; 1.74 (CI 1.74-1.74; range 1.74-1.74) vs Go shallow at 1M | passes (w = 1.75) |
| D(B)/1M | 2.41 (CI 2.39-2.41; range 2.39-2.41) vs Go | 1.3 (CI 1.29-1.31; range 1.29-1.31) vs Go after GC | not computable: memory guard at 950,272-962,560 of 1,000,000 tasks | not computable: memory guard at 950,272-962,560 of 1,000,000 tasks | inconclusive (worst measured 2.41; the memory guard stopped a cell) |
| D(B)/8M | 3.16 (CI 3.14-3.16; range 3.14-3.16) vs Go | 1.7 (CI 1.69-1.72; range 1.69-1.72) vs Go after GC | not computable: memory guard at 712,704-733,184 of 1,000,000 tasks | not computable: memory guard at 712,704-733,184 of 1,000,000 tasks | inconclusive (worst measured 3.16; the memory guard stopped a cell) |

* **The verdict.** D(B) is the only stackful strategy that passes. At 128 KiB it
  passes, with w = 1.75 on both machines, which is what section 2.4's primary
  asks of at least one stackful strategy (section 9.1). At 1 MiB and 8 MiB it
  passes on M1 (w = 2.41 and 3.16). On M2 both 100k conditions pass at those
  reservations (2.41x and 1.30x at 1 MiB, 3.16x and 1.70x at 8 MiB), but the 1M
  cells are failed rows, stopped by the memory guard at 950,272-962,560 and
  712,704-733,184 tasks, so section 2.4 calls these two configurations
  inconclusive on M2, not failed. Over the tasks they had created when the guard
  stopped them, the D(B)/1 MiB runs held 6.44 KiB per task, as at 100k (6.44
  KiB); that is not a measurement of the cell, and no verdict uses it.
* **The others fail on both machines.** B and C keep the excursion's pages
  ("commit creep": after the excursion B/1 MiB holds 34.4 KiB per task on both
  machines) and fail against Go after its collection (5.02 [5.01-5.02] and 4.96
  [4.93-5.01] KiB), although measured against Go before its collection (34.7
  [33.4-34.7] and 34.4 [23.2-35.9] KiB) B/1 MiB is at 0.99x and 1.00x. A and
  D(A) fail on scale first: each guard page splits a mapping, and the process
  stops at 32,730 tasks on M1 and 32,738 on M2 under `vm.max_map_count` 65530
  (their per-task figures over the tasks that were created match B's and
  D(B)'s).
* **The machines agree.** Over every B, C and D(B) configuration at 100k, the
  two machines' ratios to Go differ by at most 0.08 (D(B) alone: at most 0.03),
  and D(B)/128 KiB's worst ratio is 1.75 on both machines. The most variable
  cell is a control's: Go's after-excursion figure before its GC, whose five
  runs on M2 span 23.2-35.9 KiB; it is not a baseline of record.
* **What a task costs.** D(B)/1 MiB holds 4.41 KiB resident per task at 1M on M1
  (one stack page plus the task record) and 2.02 KiB of page tables; M2 measured
  4.42 and 2.02 KiB at 100k. A 4 KiB page-table page maps 2 MiB, which two 1 MiB
  slots share. The reservation costs page tables, not resident memory: at 1M on
  M1, 128 KiB slots need 0.27 KiB and 8 MiB slots 4.03 KiB (M2: 0.27 KiB at 128
  KiB and 1M, 4.03 KiB at 8 MiB and 100k). Measured on RSS alone, as section 6.5
  literally defines the resident set, the stackful strategies' ratios are
  smaller still (D(B)/1 MiB at 100k: 1.64x shallow on M1 and 1.66x on M2; the
  RSS-only columns of `summarize.py`), because Go's page tables are negligible
  and theirs are not.
* **Decommit on park.** D(B) made 1,000,001 `madvise(MADV_DONTNEED)` calls for
  1M tasks on M1, and 1,000,001 in M2's D(B)/128 KiB cell at 1M (the
  `decommits=` field of the rows' `note`): one per task, at the park after its
  excursion. The park after shallow use made none, because the probe watermark
  showed that the stack had not grown below its top page.
* **1M tasks.** Per-task memory does not change from 100k to 1M for any stackful
  strategy on M1, nor on M2 for the cells it completed. At 1M, A, B, C and Go
  measured the shallow phase only (policy 7): keeping the excursion's pages for
  1M tasks needs about 33 GiB (B/1 MiB's figure at 100k times 1M).
* **C and the 5 KiB reserve.** C's shallow figure is 5.50 KiB on both machines,
  against 4.59 KiB in M2's smoke run of harness 0.1.0 (10k tasks, one run),
  whose reserve was 8 KiB. A slot of 36 + 5 KiB is not a whole number of pages,
  so a quarter of the slots have their top frames straddle a page boundary and
  touch two pages. Rounding C's slots to pages (the reserve absorbing the
  rounding) would restore the earlier figure; C does not pass either way.
* **The controls.** Java's virtual threads copy a parked thread's frames into
  heap chunks, so Java's figures (1.07 [1.00-1.14] KiB shallow at 1M on M1, 1.06
  [1.04-1.17] KiB on M2) are a managed baseline, not a non-moving-stack
  precedent. E and tokio keep no stack per task (0.63 and 0.40 KiB on M1, 0.63
  and 0.40 KiB on M2). Go's before-GC figure after the excursion, 34.4
  [23.2-35.9] KiB on M2 with transparent huge pages on `madvise`, was 66.4 KiB
  in M2's smoke run under the image's `always` (10k tasks, one run), which is
  why the workflow sets `madvise`; after GC it was 5.04 KiB there and is 4.96
  [4.93-5.01] KiB here.

## 2. Spawn plus join (threshold: median within 3x Go with pooled stacks)

100k spawn-plus-join pairs after 10k unrecorded pairs (Java: 20k, for its JIT),
each timed with a monotonic clock; the trivial task returns at once.

| Configuration | median M1 | median M2 | p99 M1 | p99 M2 | median / Go M1 | median / Go M2 |
|---|---|---|---|---|---|---|
| A (no cache) | 7.41 [7.31-9.15] | 6.75 [6.54-9.02] | 22.1 [19.8-32.7] | 12.0 [11.8-15.1] | 16.11 [13.19-22.15] | 17.14 [15.96-23.14] |
| A (`--stack-cache 1024`) | 0.64 [0.63-0.66] | 0.51 [0.51-0.51] | 0.94 [0.74-1.01] | 0.54 [0.54-0.67] | 1.38 [1.14-1.59] | 1.29 [1.24-1.31] |
| **B** | 0.64 [0.64-0.80] | 0.73 [0.52-0.73] | 0.96 [0.81-0.97] | 0.82 [0.79-0.83] | 1.40 [1.15-1.92] | 1.84 [1.26-1.86] |
| C (8 KiB) | 0.64 [0.64-0.70] | 0.73 [0.72-0.73] | 0.92 [0.84-1.03] | 0.82 [0.80-0.85] | 1.40 [1.16-1.70] | 1.85 [1.76-1.87] |
| D(A) | 7.99 [7.74-8.09] | 7.47 [7.11-7.62] | 23.4 [21.1-24.8] | 12.9 [12.5-12.9] | 17.36 [13.98-19.60] | 18.97 [17.33-19.53] |
| D(A) (`--stack-cache 1024`) | 1.06 [1.05-1.53] | 0.97 [0.96-0.97] | 1.62 [1.39-4.28] | 1.02 [1.00-1.14] | 2.31 [1.90-3.69] | 2.45 [2.35-2.49] |
| D(B) | 0.67 [0.66-0.83] | 0.75 [0.75-0.76] | 1.04 [0.88-1.08] | 0.84 [0.83-0.87] | 1.45 [1.19-2.02] | 1.91 [1.82-1.94] |
| E | 0.52 [0.51-0.60] | 0.46 [0.46-0.46] | 0.72 [0.65-0.83] | 0.49 [0.48-0.60] | 1.12 [0.92-1.46] | 1.15 [1.11-1.18] |
| tokio | 0.52 [0.41-0.56] | 0.43 [0.42-0.43] | 11.9 [10.2-12.4] | 2.67 [2.65-2.71] | 1.12 [0.74-1.35] | 1.09 [1.04-1.11] |
| OS threads | 44.3 [43.1-46.5] | 33.2 [32.7-33.6] | 171 [161-183] | 91.3 [90.7-96.1] | 96.20 [77.78-112.57] | 84.14 [79.85-86.28] |
| B, batch 100 (per task) | 1.30 [1.27-1.37] | 0.46 [0.46-0.46] | 2.87 [2.17-3.37] | 0.52 [0.52-0.55] | 2.82 [2.30-3.31] | 1.16 [1.11-1.18] |
| B, scope per round | 0.82 [0.82-0.88] | 0.71 [0.69-0.85] | 2.83 [2.81-3.39] | 2.44 [2.42-2.45] | 1.79 [1.48-2.12] | 1.80 [1.67-2.17] |
| Go | 0.46 [0.41-0.55] | 0.39 [0.39-0.41] | 9.73 [9.12-11.2] | 1.20 [0.95-2.00] | 1.00 [0.75-1.34] | 1.00 [0.95-1.05] |
| Java virtual threads | 8.29 [8.26-10.1] | 1.21 [1.00-1.40] | 29.7 [26.0-34.9] | 7.71 [7.31-7.91] | 18.03 [14.92-24.44] | 3.08 [2.44-3.58] |

* **Verdict.** B's median is 1.40 times Go's on M1 (95% bootstrap interval
  1.16-1.73; range 1.15-1.92) and 1.84 times on M2 (95% bootstrap interval
  1.31-1.86; range 1.26-1.86): within 3x on both machines (**holds**). The
  machines differ by 0.44: B costs 0.64 [0.64-0.80] us on M1 and 0.73
  [0.52-0.73] us on M2, Go 0.46 [0.41-0.55] and 0.39 [0.39-0.41] us. D(B), the
  candidate, costs about what B costs on each machine (1.45x and 1.91x Go): a
  task that never grew below its top page is released without a system call.
* **The cost of mapping.** A maps, protects and unmaps a stack per task (7.41
  [7.31-9.15] us on M1, 6.75 [6.54-9.02] us on M2); with a cache of 1024 freed
  stacks it costs 0.64 [0.63-0.66] and 0.51 us. D(A) with the cache still pays
  one `madvise` per release (1.06 [1.05-1.53] and 0.97 [0.96-0.97] us), which
  D(B)'s watermark avoids.
* **Tails.** B's p99 is below Go's on both machines (0.96 [0.81-0.97] against
  9.73 [9.12-11.2] us on M1, 0.82 [0.79-0.83] against 1.20 [0.95-2.00] us on
  M2).
* **The load during M2's pooled cells.** During each of M2's B, C and D(B)
  processes (0.08-0.09 s of wall-clock time) `load.csv` counts other activity of
  between 0.45 and 0.84 CPUs, against between -0.33 and 0.14 during A's, E's,
  tokio's and Go's, and between -0.17 and 0.35 during the same cells' processes
  on M1. B's fastest run on M2 (0.52 us) is the one with the least of it (0.45
  CPUs). The run does not record what that activity was (on a kernel that
  accounts interrupt time separately, `load.sh` counts a process's own interrupt
  and softirq time as other processes', section 10.3), so whether it slowed
  these cells, and explains part of the spread between the machines, is not
  known; it does not bring B near the bar.
* **The cells that differ most between the machines** are controls and variants,
  not verdict inputs: Java's virtual threads cost 8.29 [8.26-10.1] us per spawn
  and join on M1 and 1.21 [1.00-1.40] us on M2, both after 20,000 unrecorded
  rounds for the JIT (18.03x and 3.08x Go), and B with batches of 100 costs 1.30
  [1.27-1.37] and 0.46 us per task.

## 3. Scale: 1M idle tasks on a 16 GiB machine with default sysctls

`max-tasks --tasks 1000000 --mem-guard 512M --hold-secs 60`: tasks that do
shallow work and park are spawned one at a time until the cap, a failed spawn,
or less than 512 MiB of memory headroom (`MemAvailable`, or the memory cgroup's
headroom when smaller); a run that reaches the cap then keeps every task parked
for 60 seconds, checks that all of them are still parked, and reads memory again
(section 6.5: "pass if all park and the process survives 60 seconds"). OS
threads have a cap of 30,000.

| Configuration | tasks created M1 (cap 1M) | tasks created M2 (cap 1M) | limit (M1 / M2) | RSS+PTE per task at stop M1 (KiB) | M2 (KiB) | still parked after 60 s M1 | M2 |
|---|---|---|---|---|---|---|---|
| A (guard page per task) | 32,730 [32,730-32,730] | 32,738 [32,738-32,738] | vma_count | 6.43 [6.43-6.44] | 6.44 [6.43-6.44] |  |  |
| B | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | cap | 6.43 [6.43-6.43] | 6.44 [6.44-6.44] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] |
| C (8 KiB) | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | cap | 5.44 [5.44-5.44] | 5.44 [5.44-5.44] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] |
| D(B) | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | cap | 6.43 [6.43-6.43] | 6.44 [6.44-6.44] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] |
| D(A) | 32,730 [32,730-32,730] | 32,738 [32,738-32,738] | vma_count | 6.44 [6.43-6.44] | 6.44 [6.43-6.44] |  |  |
| E | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | cap | 0.59 [0.59-0.59] | 0.59 [0.59-0.59] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] |
| tokio | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | cap | 0.39 [0.39-0.39] | 0.39 [0.39-0.39] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] |
| OS threads | 16,128 [16,128-16,128] | 9,489 [9,489-9,489] | vma_count / threads | 9.97 [9.97-9.98] | 10.0 [10.00-10.0] |  |  |
| Go | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | cap | 2.61 [2.61-2.61] | 2.63 [2.60-2.63] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] |
| Java virtual threads | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | cap | 1.01 [0.95-1.05] | 1.00 [1.00-1.04] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] |

* A and D(A) stop at 32,730 tasks on M1 and 32,738 on M2, the process then
  holding 65,531 mappings on both machines: every task's guard page splits its
  stack mapping, so the process meets `vm.max_map_count` (65530) at about 32k
  tasks. That is the scale row's fail condition, met on both machines:
  **guard-page-per-task is out** as a default, which policy 3 already concludes.
* B, C, D(B), E and tokio, and the Go and Java controls, park 1M tasks on both
  machines; after the 60-second hold every task is still parked, and per-task
  memory is unchanged (B/1 MiB 6.43 KiB at the stop and 6.43 KiB after the hold
  on M1, 6.44 and 6.44 KiB on M2). The pooled strategies' reservations do not
  approach the mapping limit: B/1 MiB has 69 mappings in all at 1M tasks on M1
  and 57 on M2, with about 0.96 TiB of address space reserved in 1 GiB chunks
  (`reserved_bytes` in the `idle-memory` rows' note on M1).
* OS threads stop at 16,128 on M1 (vma_count): each Rust thread maps its stack,
  its guard, and the signal stack that `std` maps with its own guard. On M2 they
  stop at 9,489 (threads), at the scope's pids cgroup of 9491 tasks.
* M2 has 7.75 GiB, not the row's 16 GiB. 1M pooled tasks fit in it (B/1 MiB:
  about 6.1 GiB, beside the 512 MiB margin), and the fail condition is a mapping
  limit, not memory, so the row's verdict does not depend on it.

## 4. Echo (threshold: 10k-connection p99 within 2x Go) and idle connections

`echo`: N closed-loop client tasks in the same process (64-byte messages, a
write and a read of the echo per round trip), 5 s unrecorded then 30 s recorded;
the server is one task per connection. `idle-connections`: N server tasks parked
in `read`, the client ends held as plain sockets, memory read after 30 s idle
(section 6.5). Both ends count against `RLIMIT_NOFILE`, so N connections need 2N
+ 64 descriptors.

10,000 connections

| Configuration | M1 | p50 M2 | p99 M2 | p99.9 M2 | req/s M2 | p99 / Go M2 |
|---|---|---|---|---|---|---|
| A | not run: RLIMIT_NOFILE 20000 < 20064 | 146 [142-147] | 152 [150-155] | 159 [154-161] | 68,211 [68,140-70,340] | 0.91 [0.76-0.96] |
| B | not run: RLIMIT_NOFILE 20000 < 20064 | 146 [143-147] | 153 [151-165] | 160 [159-215] | 68,260 [67,468-70,059] | 0.91 [0.77-1.02] |
| C (8 KiB) | not run: RLIMIT_NOFILE 20000 < 20064 | 145 [141-147] | 152 [148-153] | 158 [151-160] | 68,989 [68,066-70,973] | 0.91 [0.75-0.95] |
| D(A) | not run: RLIMIT_NOFILE 20000 < 20064 | 170 [168-174] | 180 [175-187] | 184 [181-200] | 58,534 [56,998-59,515] | 1.07 [0.89-1.16] |
| D(B) | not run: RLIMIT_NOFILE 20000 < 20064 | 146 [142-149] | 152 [148-160] | 154 [150-263] | 68,348 [66,851-70,411] | 0.91 [0.75-0.99] |
| E | not run: RLIMIT_NOFILE 20000 < 20064 | 144 [142-146] | 151 [149-157] | 157 [155-207] | 69,546 [68,276-70,342] | 0.90 [0.76-0.97] |
| tokio | not run: RLIMIT_NOFILE 20000 < 20064 | 3.18 [3.13-3.20] | 487 [483-497] | 515 [508-527] | 113,732 [113,387-115,762] | 2.91 [2.45-3.08] |
| Go | not run: RLIMIT_NOFILE 20000 < 20064 | 156 [153-160] | 167 [162-197] | 196 [166-230] | 63,740 [62,288-65,212] | 1.00 [0.82-1.22] |
| Java virtual threads | not run: RLIMIT_NOFILE 20000 < 20064 | 222 [220-228] | 330 [311-334] | 374 [355-391] | 45,083 [43,859-45,648] | 1.97 [1.58-2.06] |

1,000 connections

| Configuration | p50 M1 | p99 M1 | p99.9 M1 | req/s M1 | p99 / Go M1 | p50 M2 | p99 M2 | p99.9 M2 | req/s M2 | p99 / Go M2 |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 3.11 [3.07-3.21] | 4.51 [4.16-7.94] | 6.50 [5.72-13.0] | 314,121 [280,012-318,494] | 0.92 [0.52-1.64] | 7.05 [6.99-7.27] | 8.16 [7.56-11.7] | 10.3 [9.88-14.0] | 140,820 [128,511-142,661] | 0.80 [0.54-1.27] |
| B | 3.06 [3.03-3.10] | 4.32 [4.13-6.73] | 6.46 [5.75-9.75] | 319,259 [304,219-322,846] | 0.88 [0.52-1.39] | 7.01 [6.97-7.79] | 8.05 [7.47-10.8] | 10.3 [9.79-13.1] | 142,019 [124,156-143,043] | 0.79 [0.54-1.17] |
| C (8 KiB) | 3.04 [2.95-3.96] | 4.32 [3.97-7.91] | 6.42 [5.32-10.4] | 318,677 [244,297-333,410] | 0.88 [0.50-1.64] | 7.01 [6.94-7.06] | 8.02 [7.72-8.87] | 10.2 [9.85-11.0] | 141,645 [138,020-143,445] | 0.79 [0.55-0.96] |
| D(A) | 13.4 [13.2-13.5] | 18.4 [17.9-19.8] | 24.1 [24.0-26.4] | 73,266 [72,715-74,131] | 3.73 [2.24-4.11] | 9.45 [9.08-9.67] | 11.4 [9.91-12.1] | 12.9 [12.4-14.9] | 103,887 [102,417-109,909] | 1.12 [0.71-1.32] |
| D(B) | 3.07 [2.99-3.08] | 4.16 [4.03-4.39] | 5.81 [5.18-6.96] | 318,935 [316,259-328,027] | 0.85 [0.50-0.91] | 7.19 [6.96-8.00] | 8.85 [7.39-11.7] | 12.2 [9.62-15.8] | 137,090 [118,321-143,329] | 0.87 [0.53-1.27] |
| E | 2.97 [2.93-3.17] | 4.10 [4.00-5.80] | 5.80 [5.34-9.22] | 329,920 [295,210-333,751] | 0.83 [0.50-1.20] | 7.03 [6.89-7.18] | 8.98 [7.44-10.5] | 10.8 [9.54-12.6] | 139,584 [133,364-144,708] | 0.88 [0.53-1.14] |
| tokio | 2.32 [2.27-2.37] | 7.75 [7.62-11.2] | 11.3 [10.7-21.1] | 353,546 [328,134-361,553] | 1.57 [0.95-2.31] | 2.38 [2.36-2.44] | 42.0 [37.1-44.7] | 59.5 [52.0-63.8] | 148,399 [145,019-151,078] | 4.13 [2.67-4.86] |
| Go | 3.24 [3.18-3.41] | 4.92 [4.83-8.00] | 8.01 [7.51-13.2] | 303,681 [279,851-309,769] | 1.00 [0.60-1.66] | 7.94 [7.78-8.40] | 10.2 [9.19-13.9] | 12.6 [11.9-17.3] | 125,543 [113,525-128,763] | 1.00 [0.66-1.51] |
| Java virtual threads | 5.99 [5.91-6.32] | 12.0 [11.2-13.4] | 15.7 [15.2-17.7] | 161,280 [149,019-163,054] | 2.44 [1.40-2.78] | 15.7 [15.3-16.5] | 26.5 [22.4-33.9] | 52.9 [33.1-69.6] | 62,719 [60,146-65,227] | 2.61 [1.61-3.68] |

idle connections

| Configuration | RSS+PTE per connection M1 (9,900) | RSS only M1 | / Go M1 | RSS+PTE per connection M2 (100,000) | RSS only M2 | / Go M2 |
|---|---|---|---|---|---|---|
| A | 10.6 [10.6-10.6] | 8.54 [8.54-8.54] | 1.50 [1.47-1.50] | failed: connect: Connection timed out (os error 110) |  |  |
| B | 10.6 [10.6-10.6] | 8.54 [8.54-8.54] | 1.50 [1.48-1.51] | 10.6 [10.6-10.6] | 8.53 [8.53-8.53] | 1.95 [1.93-1.98] |
| C (8 KiB) | 8.57 [8.57-8.58] | 8.54 [8.54-8.54] | 1.21 [1.20-1.22] | 8.56 [8.56-8.56] | 8.53 [8.53-8.53] | 1.58 [1.56-1.61] |
| D(A) | 10.6 [10.6-10.6] | 8.54 [8.54-8.54] | 1.50 [1.47-1.50] | failed: connect: Connection timed out (os error 110) |  |  |
| D(B) | 10.6 [10.6-10.6] | 8.54 [8.54-8.55] | 1.50 [1.48-1.51] | 10.6 [10.6-10.6] | 8.53 [8.53-8.53] | 1.95 [1.93-1.98] |
| E | 4.97 [4.97-4.97] | 4.96 [4.96-4.96] | 0.70 [0.69-0.71] | 4.95 [4.95-4.95] | 4.94 [4.94-4.94] | 0.92 [0.90-0.93] |
| tokio | 4.81 [4.81-4.82] | 4.80 [4.80-4.81] | 0.68 [0.67-0.69] | 4.78 [4.78-4.78] | 4.77 [4.77-4.77] | 0.88 [0.87-0.90] |
| Go | 7.06 [7.02-7.16] | 7.05 [7.01-7.14] | 1.00 [0.98-1.02] | 5.41 [5.33-5.47] | 5.40 [5.32-5.46] | 1.00 [0.97-1.03] |
| Java virtual threads | 11.6 [10.9-13.7] | 11.5 [10.9-13.7] | 1.64 [1.53-1.95] | 9.18 [7.19-9.29] | 9.16 [7.18-9.27] | 1.70 [1.31-1.74] |

* **10k connections ran on M2 only.** On M1 they need 20,064 descriptors and the
  hard limit is 20,000 (policy 3: the cell runs only where the default limit
  allows); the harness and the controls record failed rows instead. Inside M2's
  scope the hard limit was 1,048,576 (section 10.2), and there every stackful
  strategy's p99 is within 2x Go's: B 0.91x (95% bootstrap interval 0.78-0.99;
  range 0.77-1.02), D(B) 0.91x (95% bootstrap interval 0.77-0.95; range
  0.75-0.99), and at most 1.07x (D(A)); Java's virtual threads reach 1.97x and
  tokio 2.91x. One machine cannot decide the row: it is **not shown** (section
  9.1).
* **At 1000 connections** B's p99 is 0.88 times Go's on M1 (95% bootstrap
  interval 0.54-1.37; range 0.52-1.39) and 0.79 on M2 (95% bootstrap interval
  0.58-1.15; range 0.54-1.17), and D(B)'s 0.85 and 0.87. D(A) is the cell that
  differs most between the machines: 3.73x Go on M1 (95% bootstrap interval
  2.30-4.03; range 2.24-4.11) and 1.12x on M2 (95% bootstrap interval 0.82-1.27;
  range 0.71-1.32), a spread of 2.61. Without a watermark it makes a `madvise`
  call at every park, and every request parks its server task and its client
  task; that costs D(A) 10.3 ms of median round trip over B on M1 and 2.4 ms on
  M2. tokio's p99 also differs (1.57x and 4.13x Go). I/O design, for the record:
  taskbench's workers poll one shared epoll instance (busy workers every 31
  scheduling decisions, idle ones before they spin) and one sleeping worker
  blocks in it; Go uses its integrated netpoller. A miss would be "runtime
  design review, not a language change".
* **Idle connections.** M1 ran them at 9,900 connections, the most its
  descriptor limit allows, and M2 at the registered 100,000. A pooled strategy's
  idle connection costs 10.6 KiB on both machines (B/1 MiB; for C/8 KiB, whose
  small slots need fewer page tables, 8.57 [8.57-8.58] and 8.56 KiB), against
  Go's 7.06 [7.02-7.16] and 5.41 [5.33-5.47] KiB, so the ratio is 1.50x on M1
  and 1.95x on M2: Go's per-connection cost is lower at M2's 100,000 connections
  than at M1's 9,900, and the pooled strategies' is the same at both, so the two
  ratios are not one cell measured twice. The server task holds a 4 KiB read
  buffer on its stack. A and D(A) failed at 100k on M2 in every run with
  "connect: Connection timed out (os error 110)". A server task per connection
  needs a guarded mapping each, so the acceptor's `spawn` meets the VMA limit
  near 32,738 tasks, the acceptor stops, and the client side's blocking
  `connect` times out on the full listen backlog. The CSV records only the
  timeout; the rest is the harness's code path (`src/bench/echo.rs`), not a
  recorded field.

## 5. Stack exhaustion (threshold: no segfault, ever)

`deep-recursion --framed` in every stackful configuration (E and tokio run the
parse and the naive recursion on the worker's 8 MiB stack, plus a recursion of
boxed futures): a 10k-deep recursive-descent parse (four frames per level), a
1M-deep naive recursion with a drop guard every 256 frames, and the framed
recursion (a 256-byte local and a drop guard in every frame), each in its own
task, then a task that checks the runtime still works. The reserve columns are
the bytes of the 5 KiB probe reserve the unwind overwrote (the reserve is
painted before the case and scanned after), the maximum over five runs.

| Configuration | 10k parse (M1 / M2) | 1M naive (M1 / M2) | framed (M1 / M2) | naive async (M1 / M2) | drop guards skipped (M1 / M2) | reserve bytes used M1 (parse / naive / framed, max of 5) | reserve bytes used M2 | healthy after (M1 / M2) |
|---|---|---|---|---|---|---|---|---|
| A/128K | stack_exhausted at 354 | stack_exhausted at 2,722 | stack_exhausted at 429 |  | naive 11, framed 430 / naive 11, framed 429 |  |  | 1 [1-1] |
| A/1M | stack_exhausted at 2,848 | stack_exhausted at 21,837 / 21,836 | stack_exhausted at 3,447 |  | naive 86, framed 3,448 |  |  | 1 [1-1] |
| A/8M | completed at 10,000 | stack_exhausted at 174,754 | stack_exhausted at 27,592 |  | naive 683, framed 27,592 |  |  | 1 [1-1] |
| B/128K | stack_exhausted at 363 | stack_exhausted at 2,786 / 2,785 | stack_exhausted at 439 |  | naive 0, framed 0 | 1880 / 1832 / 2072 | 1880 / 1800 / 2088 | 1 [1-1] |
| B/1M | stack_exhausted at 2,856 | stack_exhausted at 21,900 | stack_exhausted at 3,457 |  | naive 0, framed 0 | 1800 / 1800 / 2040 | 1800 / 1816 / 2056 | 1 [1-1] |
| B/8M | completed at 10,000 | stack_exhausted at 174,818 / 174,817 | stack_exhausted at 27,602 / 27,601 |  | naive 0, framed 0 | 0 / 1832 / 2088 | 0 / 1800 / 1800 | 1 [1-1] |
| C/2K | stack_exhausted at 4 | stack_exhausted at 34 / 33 | stack_exhausted at 4 |  | naive 0, framed 0 | 1864 / 1832 / 1928 | 1864 / 1800 / 1944 | 1 [1-1] |
| C/8K | stack_exhausted at 20 | stack_exhausted at 162 / 161 | stack_exhausted at 24 |  | naive 0, framed 0 | 1880 / 1832 / 1864 | 1880 / 1800 / 1880 | 1 [1-1] |
| C/16K | stack_exhausted at 43 | stack_exhausted at 332 | stack_exhausted at 51 |  | naive 0, framed 0 | 1880 / 1800 / 1880 | 1880 / 1816 / 1896 | 1 [1-1] |
| D(A)/128K | stack_exhausted at 354 | stack_exhausted at 2,722 | stack_exhausted at 429 |  | naive 11, framed 430 / naive 11, framed 429 |  |  | 1 [1-1] |
| D(A)/1M | stack_exhausted at 2,848 | stack_exhausted at 21,837 / 21,836 | stack_exhausted at 3,447 |  | naive 86, framed 3,448 |  |  | 1 [1-1] |
| D(A)/8M | completed at 10,000 | stack_exhausted at 174,754 | stack_exhausted at 27,592 |  | naive 683, framed 27,592 |  |  | 1 [1-1] |
| D(B)/128K | stack_exhausted at 363 | stack_exhausted at 2,786 / 2,785 | stack_exhausted at 439 |  | naive 0, framed 0 | 1880 / 1832 / 2072 | 1880 / 1800 / 2088 | 1 [1-1] |
| D(B)/1M | stack_exhausted at 2,856 | stack_exhausted at 21,900 | stack_exhausted at 3,457 |  | naive 0, framed 0 | 1800 / 1800 / 2040 | 1800 / 1816 / 2056 | 1 [1-1] |
| D(B)/8M | completed at 10,000 | stack_exhausted at 174,818 / 174,817 | stack_exhausted at 27,602 / 27,601 |  | naive 0, framed 0 | 0 / 1832 / 2088 | 0 / 1800 / 1800 | 1 [1-1] |
| E | completed at 10,000 | stack_exhausted at 173,278 / 173,279 |  | stack_exhausted at 86,638 | naive 0 |  |  | 1 [1-1] |
| tokio | completed at 10,000 | stack_exhausted at 173,273 |  | stack_exhausted at 86,634 | naive 0 |  |  | 1 [1-1] |

*Depths are medians over five runs; where one depth is given, both machines
reached it.*

* **Every exhaustion is clean** on both machines: 255 outcome rows on M1 and 255
  on M2, 0 unclean, and the follow-up task ran in every run. `process_exit` rows
  (crashes) across every CSV of both results directories: **0**.
* **Mechanism.** B, C and D(B) fail through the software probe and unwind, so
  every destructor runs (0 drop guards skipped). A and D(A) fail through the
  guard page; the trap handler ends the task without unwinding, so the frames'
  destructors are skipped (in the naive recursion at 1 MiB, 86 drop guards on
  both machines; in the framed one, 3,448 on both machines).
* **The probe reserve.** The largest unwind used 2,088 bytes of the 5,120-byte
  reserve on both machines. Before the fix of section 10.3, the first unwind of
  each process used about 5 KiB; M2's smoke run of harness 0.1.0, made before
  the fix, shows that pattern (3,416 bytes for the first unwind, 1,816 for the
  next), and M2's full run of 0.2.0 does not.
* **Depth per reservation.** The 10k-deep parse completes only at 8 MiB (and on
  E and tokio, whose worker stacks are 8 MiB); at 1 MiB it fails cleanly at
  level 2,856 on both machines. Wherever the table gives two depths, the
  machines' medians differ by 1 level at most. B's usable stack is a little over
  its reservation (the slot is rounded up to a page above the 5 KiB reserve), so
  B reaches a few more levels than A.
* The corpus-wide `--stats stack-depth` histogram (interpreter era) is reported
  separately and decides whether the default reservation suffices.

## 6. Starvation (threshold: probe p99 no worse than Go and under 10 ms)

Four compute tasks per worker (16 on M1's four workers, 8 on M2's two) run an
LCG loop and reach a yield point every K iterations, the stand-in for a
compiler-inserted yield check; a probe task sleeps 1 ms in a loop and records
how late each wake-up is, first alone (1 s) and then under the load (11 s). Go
runs the same loop with no yields at all and relies on its default asynchronous
preemption (policy 8); Java has no preemption of virtual threads and runs with
no yields and with `Thread.yield()` every K iterations. Labels: the default K is
10000; `yield_every=0` means no yield points. In an "M1 / M2" column, a single
figure means that both machines measured the same.

| Configuration (K = yield_every; default 10000) | slice between yields (M1 / M2) | p99 loaded M1 | p99 loaded M2 | max loaded (M1 / M2) | p99 alone (M1 / M2) | wake-ups (M1 / M2) | p99 / Go M1 (default preemption) | p99 / Go M2 |
|---|---|---|---|---|---|---|---|---|
| A/1M | 21.0 [20.3-32.5] / 18.3 [18.3-19.1] | 25.9 [24.8-28.3] | 23.7 [23.6-23.9] | 455 [90.0-3,955] / 201 [64.5-282] | 181 [170-219] / 70.9 [65.9-111] | 10,936 [10,909-10,937] / 10,938 [10,937-10,940] | 0.00066 [0.00063-0.00072] | 0.00060 [0.00060-0.00061] |
| A/1M yield_every=100000 | 212 [195-221] / 181 [180-184] | 233 [225-271] | 316 [316-316] | 833 [377-7,026] / 350 [325-1,245] | 185 [158-281] / 71.2 [68.0-103] | 10,559 [10,350-10,566] / 9,954 [9,891-10,004] | 0.0059 [0.0057-0.0069] | 0.0081 [0.0080-0.0081] |
| B/1M | 21.3 [20.3-22.0] / 18.1 [18.0-18.3] | 25.7 [25.0-30.0] | 23.6 [23.3-23.7] | 570 [89.6-3,463] / 63.8 [62.4-125] | 178 [172-215] / 72.7 [71.4-102] | 10,931 [10,920-10,937] / 10,941 [10,940-10,943] | 0.00065 [0.00064-0.00076] | 0.00060 [0.00059-0.00061] |
| B/1M yield_every=0 | 206 [181-218] / 182 [180-185] | 10,999,124 [10,994,330-11,001,644] | 10,999,888 [10,998,116-11,000,097] | 10,999,124 [10,994,330-11,001,644] / 10,999,888 [10,998,116-11,000,097] | 175 [162-210] / 73.6 [70.3-102] | 1 [1-2] | 279.61 [279.36-279.75] | 280.39 [280.34-280.41] |
| B/1M yield_every=1000 | 2.14 [2.00-2.81] / 1.82 [1.81-1.85] | 4.50 [4.10-5.39] | 2.88 [2.86-3.15] | 4,489 [2,776-19,707] / 114 [50.2-2,146] | 192 [181-421] / 71.6 [68.6-96.2] | 10,978 [10,952-10,984] / 10,988 [10,987-10,989] | 0.00011 [0.00010-0.00014] | 0.000073 [0.000073-0.000080] |
| B/1M yield_every=100000 | 202 [189-224] / 183 [181-184] | 229 [221-269] | 316 [315-316] | 1,217 [345-3,034] / 353 [328-547] | 180 [152-230] / 71.7 [69.9-81.6] | 10,555 [10,358-10,557] / 9,946 [9,920-10,033] | 0.0058 [0.0056-0.0068] | 0.0081 [0.0080-0.0081] |
| C/8K | 20.6 [18.5-21.8] / 18.2 [18.1-18.4] | 26.1 [25.9-30.0] | 23.5 [23.4-23.6] | 3,861 [118-7,930] / 91.6 [62.5-104] | 183 [159-255] / 74.7 [72.2-111] | 10,928 [10,914-10,939] / 10,940 [10,939-10,943] | 0.00066 [0.00066-0.00076] | 0.00060 |
| C/8K yield_every=100000 | 213 [199-273] / 183 [181-186] | 233 [230-276] | 316 [315-316] | 1,648 [352-4,269] / 381 [330-1,189] | 201 [160-229] / 70.3 [69.6-71.8] | 10,540 [10,347-10,558] / 9,955 [9,930-10,021] | 0.0059 [0.0059-0.0070] | 0.0080 [0.0080-0.0081] |
| D(A)/1M | 20.3 [19.9-21.4] / 18.3 [18.2-18.3] | 26.8 [24.8-29.3] | 24.1 [24.1-24.2] | 650 [166-9,486] / 65.4 [63.5-398] | 182 [166-395] / 68.7 [63.3-83.5] | 10,930 [10,866-10,936] / 10,935 [10,934-10,937] | 0.00068 [0.00063-0.00075] | 0.00061 [0.00061-0.00062] |
| D(A)/1M yield_every=100000 | 201 [189-274] / 183 [181-183] | 236 [226-268] | 316 [257-316] | 1,068 [436-3,137] / 343 [325-387] | 165 [160-280] / 70.3 [67.3-100] | 10,549 [10,356-10,563] / 9,917 [9,891-9,993] | 0.0060 [0.0057-0.0068] | 0.0081 [0.0066-0.0081] |
| D(B)/1M | 21.8 [20.7-22.2] / 18.3 [18.0-18.3] | 25.4 [25.0-27.2] | 23.6 [23.5-23.7] | 370 [86.4-562] / 3,680 [122-6,167] | 184 [152-288] / 72.8 [70.8-78.1] | 10,932 [10,924-10,939] / 10,936 [10,935-10,941] | 0.00065 [0.00064-0.00069] | 0.00060 |
| D(B)/1M yield_every=100000 | 205 [202-221] / 183 [180-186] | 234 [234-271] | 316 [316-316] | 674 [334-5,113] / 368 [348-717] | 193 [168-250] / 71.5 [70.0-89.3] | 10,535 [10,349-10,542] / 9,950 [9,920-10,011] | 0.0060 [0.0059-0.0069] | 0.0081 [0.0080-0.0081] |
| E | 20.7 [18.0-22.0] / 18.3 [18.1-18.3] | 17.8 [17.7-18.9] | 10.7 [8.56-16.2] | 376 [86.9-3,954] / 362 [74.9-578] | 222 [181-483] / 70.4 [62.8-120] | 10,940 [10,936-10,946] / 10,962 [10,956-10,965] | 0.00045 [0.00045-0.00048] | 0.00027 [0.00022-0.00041] |
| E yield_every=100000 | 207 [204-224] / 183 [182-185] | 137 [133-153] | 138 [138-144] | 277 [201-5,580] / 217 [193-255] | 190 [166-265] / 67.6 [62.7-88.7] | 10,582 [10,407-10,602] / 9,974 [9,891-10,048] | 0.0035 [0.0034-0.0039] | 0.0035 [0.0035-0.0037] |
| go yield_every=0 |  | 39,338 [39,327-39,356] | 39,230 [39,228-39,231] | 39,610 [39,424-42,139] / 39,303 [39,232-40,164] | 217 [175-263] / 104 [79.2-142] | 535 [533-536] / 538 [538-539] | 1.00 | 1.00 |
| java |  | 774 [585-2,196] | 1,036 [740-1,765] | 52,105 [29,067-69,541] / 11,092 [8,485-83,437] | 465 [377-871] / 322 [247-380] | 8,529 [7,995-9,599] / 9,438 [9,182-9,567] | 0.020 [0.015-0.056] | 0.026 [0.019-0.045] |
| java yield_every=0 |  | 11,003,284 [11,001,379-11,005,227] | 11,003,743 [11,001,587-11,005,361] | 11,003,284 [11,001,379-11,005,227] / 11,003,743 [11,001,587-11,005,361] | 454 [408-744] / 356 [266-517] | 1 [1-1] | 279.71 [279.54-279.84] | 280.49 [280.43-280.55] |
| java yield_every=100000 |  | 6,103 [4,037-211,502] | 1,279 [350-3,184] | 2,082,688 [605,333-2,810,069] / 306,537 [9,726-2,035,571] | 436 [362-1,406] / 306 [243-442] | 2,080 [593-2,582] / 7,668 [4,565-9,221] | 0.16 [0.10-5.38] | 0.033 [0.0089-0.081] |
| os-threads | 20.9 [18.2-23.6] / 18.2 [18.1-18.2] | 3,000 [3,000-3,033] | 1,889 [1,888-1,944] | 12,901 [11,293-27,099] / 4,153 [2,111-5,645] | 180 [172-298] / 73.2 [68.9-85.2] | 9,635 [9,553-9,701] / 10,170 [10,156-10,186] | 0.076 [0.076-0.077] | 0.048 [0.048-0.050] |
| tokio | 21.4 [19.0-24.7] / 18.2 [18.0-18.3] | 1,060 [1,058-1,075] | 1,067 [1,065-1,070] | 4,960 [1,848-12,625] / 1,392 [1,110-4,208] | 1,299 [1,199-1,561] / 1,094 [1,079-1,125] | 5,500 [5,475-5,502] / 5,501 [5,500-5,501] | 0.027 | 0.027 |
| tokio yield_every=100000 | 213 [194-224] / 183 [181-184] | 1,525 [1,502-1,583] | 1,640 [1,634-1,640] | 2,130 [2,012-2,684] / 1,913 [1,836-2,568] | 1,270 [1,235-2,021] / 1,097 [1,092-1,124] | 5,499 [5,496-5,502] / 5,495 [5,494-5,497] | 0.039 [0.038-0.040] | 0.042 |

* **With yield points the probe's p99 is far below Go's on both machines**: on
  M1 25.7 [25.0-30.0] us at K = 10k (slices of 21.3 [20.3-22.0] us) and 229
  [221-269] us at K = 100k, against 39.3 [39.3-39.4] ms for Go's preemption; on
  M2 23.6 [23.3-23.7] us (slices of 18.1 [18.0-18.3] us) and 316 [315-316] us,
  against 39.2 ms. The threshold holds for every strategy on both machines.
* **Without yield points** (`yield_every=0`) the probe cannot run until the
  compute tasks stop: 10,999 [10,994-11,002] ms on M1 and 11,000 [10,998-11,000]
  ms on M2. Policy 8 expects this and reports it without counting it.
* **What yield points cost.** Compute throughput with no yield points is
  1,887,211,880 [1,767,957,571-1,933,528,494] iterations per second on M1 and
  1,061,054,903 [1,060,559,620-1,061,217,148] on M2. Relative to that median, a
  yield point every 100k iterations changes the median throughput by +1.5 % on
  M1 and -0.2 % on M2, every 10k by -2.9 % and -0.9 %, and every 1k by -18.1 %
  and -7.3 %. On M1 the five-run ranges (the one above, and 1,832,735,066
  [1,740,280,698-1,895,116,740] at 10k) are wider than the first two
  differences, so at 10k and 100k iterations the cost is below what M1 resolves.
  M2, where the other processes used at most 0.045 CPUs during any starvation
  process, resolves them: its ranges at 10k (1,051,417,764
  [1,050,459,095-1,052,502,702]) and at 100k (1,059,239,242
  [1,059,115,521-1,059,687,241]) lie below the whole range without yield points.
  At 1k (slices of 2.14 [2.00-2.81] and 1.82 [1.81-1.85] us) the cost is clear
  on both. This loop yields between chunks of K iterations and checks nothing in
  between, so these are the costs of the switches alone; the per-iteration check
  a compiler would insert is the `loop` workload of section 7, which costs
  nothing measurable on M1 and 3.5 % on M2. Because policy 8 puts yield points
  in the 3 % budget, the yield interval and the check's placement are design
  parameters: slices of about 0.2 ms (K = 100k) keep the probe's p99 at 229
  [221-269] us on M1 and 316 [315-316] us on M2, 172 and 124 times lower than
  Go's.
* tokio's p99 of about 1 ms on both machines (1,060 [1,058-1,075] and 1,067
  [1,065-1,070] us) is its timer's granularity, not starvation. OS threads rely
  on the kernel's preemption.

## 7. Software probe overhead (threshold: at most 3 % on compute loops)

Each workload is compiled without probes and with three probe models, and the
four variants run alternately 31 times inside one task on the strategy's stack;
a cell's value is the ratio of medians minus one, and the table shows the median
and range over five processes on each machine. `split` is the model of record:
`cmp rsp, fs:[0x70]; jb`, the prologue check of GCC's `-fsplit-stack`, against a
slot the scheduler keeps equal to the running task's limit. `arg` passes the
limit as an argument; `tls` reads a Rust thread-local (not migration-safe).
`loop` is the call-free LCG loop of section 6 with the same check on every
back-edge: the cost model of a compiler-inserted yield check, which policy 8
counts in this budget (in the other workloads the entry check can double as the
yield point, as Go's prologue check did before asynchronous preemption).

| Configuration | fib M1 | fib M2 | calls M1 | calls M2 | tree M1 | tree M2 | loop (back-edge) M1 | loop (back-edge) M2 |
|---|---|---|---|---|---|---|---|---|
| A/1M | 14.0 [13.9-15.0] | 8.6 [8.4-8.7] | 0.0 [-0.9-0.5] | 0.2 [0.2-0.2] | 4.2 [2.5-6.8] | 2.1 [1.9-2.4] | -0.3 [-0.4-0.1] | 3.5 [3.5-3.5] |
| B/1M | 13.3 [11.9-16.3] | 8.6 [8.2-8.7] | 0.0 [-0.1-0.9] | 0.2 [0.2-0.2] | 4.5 [1.9-5.3] | 2.2 [1.8-2.3] | -0.1 [-0.2-0.5] | 3.5 [3.5-3.5] |
| C/8K | 14.3 [11.7-15.0] | 8.8 [8.3-8.9] | 0.1 [-0.2-1.1] | 0.2 [0.2-0.2] | 6.1 [4.9-11.0] | 1.4 [0.9-2.4] | -0.1 [-0.5-0.2] | 3.5 [3.5-3.5] |
| D(B)/1M | 14.6 [13.1-15.7] | 8.6 [8.2-8.6] | 0.2 [-0.3-0.6] | 0.2 [0.2-0.2] | 5.3 [2.1-7.3] | 2.2 [2.2-2.4] | 0.1 [-0.2-0.9] | 3.5 [3.5-3.5] |
| E | 14.0 [13.1-14.4] | 8.7 [8.5-8.8] | 0.2 [-0.3-0.5] | 0.1 [0.1-0.2] | 4.7 [2.7-6.0] | 1.9 [1.7-2.0] | 0.1 [0.0-0.2] | 3.5 [3.5-3.5] |

*The alternative cost models (M1 / M2)*

| Configuration | fib arg | calls arg | tree arg | fib tls | calls tls | tree tls |
|---|---|---|---|---|---|---|
| A/1M | 23.5 [22.9-24.3] / 21.3 [20.5-21.4] | 0.1 [-1.6-0.3] / 3.6 [3.6-3.6] | 3.3 [1.4-4.8] / 4.0 [3.7-4.5] | 13.8 [13.7-15.5] / 15.8 [15.8-15.9] | 1.3 [0.6-2.4] / 11.1 [10.7-11.3] | 2.6 [-0.5-6.5] / 4.4 [4.2-5.0] |
| B/1M | 23.5 [21.4-23.7] / 21.4 [21.1-21.5] | 0.2 [-0.5-0.9] / 3.6 [3.6-3.6] | 4.1 [0.7-6.6] / 3.9 [3.7-4.0] | 13.6 [11.9-14.2] / 15.8 [15.8-15.9] | 0.8 [0.2-2.3] / 10.8 [10.6-11.7] | 5.1 [2.8-6.5] / 4.6 [4.5-5.1] |
| C/8K | 23.9 [22.4-24.1] / 20.8 [20.6-21.2] | 0.0 [-0.3-0.7] / 3.6 [3.6-3.6] | 4.9 [3.9-6.7] / 3.8 [3.5-4.0] | 13.9 [12.7-14.9] / 17.5 [17.4-17.5] | 0.7 [0.6-1.3] / 11.1 [10.9-11.8] | 6.0 [4.9-7.4] / 3.8 [3.2-4.7] |
| D(B)/1M | 23.4 [22.9-24.6] / 21.1 [20.8-21.2] | 0.1 [-0.2-0.5] / 3.6 [3.6-3.6] | 3.3 [3.1-5.7] / 4.0 [3.7-4.4] | 13.9 [12.8-14.9] / 15.8 [15.7-15.9] | 0.8 [0.7-2.7] / 11.2 [10.7-11.7] | 4.8 [2.3-6.1] / 4.8 [4.4-5.0] |
| E | 24.6 [22.2-24.8] / 21.2 [20.7-21.4] | 0.1 [-0.2-1.0] / 3.6 [3.5-3.6] | 4.5 [3.3-5.7] / 3.8 [3.5-4.2] | 14.1 [13.7-15.5] / 15.8 [15.8-16.0] | 0.9 [0.6-3.3] / 10.9 [10.6-11.7] | 5.1 [2.9-8.1] / 4.3 [3.9-4.6] |

* **Taken at face value, two of the four workloads exceed 3 % on each machine,
  but not the same two.** On M1, `fib` does in every run on every strategy (B:
  13.3 [11.9-16.3] %; the lowest single run of any strategy, 11.7 %), and
  `tree`'s median does on every strategy (B: 4.5 [1.9-5.3] %), although single
  runs fall below it; `calls` (0.0 [-0.1-0.9] %) and the back-edge yield check
  (-0.1 [-0.2-0.5] %) are within it, the check because its compare and branch
  run in parallel with the loop's multiply chain. On M2, `fib` exceeds it as
  well (B: 8.6 [8.2-8.7] %; lowest run of any strategy 8.2 %), and so does the
  back-edge check, in every run of every strategy (B: 3.5 %; lowest run 3.5 %),
  while `tree` (2.2 [1.8-2.3] %) and `calls` (0.2 %) are within it. For B the
  machines differ by 4.7 points on `fib`, 2.3 on `tree` and 3.6 on the back-edge
  check.
* **Why this does not decide the bar.** `fib` is an upper bound by construction:
  LLVM turns one of its two recursive calls into a loop that keeps the probe
  written at the top of the source function, so the check runs about twice per
  machine-level call, which a compiler inserting it into the prologue after
  optimisation would not do. `calls`' callee is a small leaf that a compiler
  would exempt (Go's `NOSPLIT` rule); `tree` is a fair case. There are no
  hardware counters on either machine ("hardware counters unavailable: No such
  file or directory (os error 2)" on M1, "hardware counters unavailable:
  Permission denied (os error 13)" on M2), so only wall-clock medians exist:
  between the five processes of B's cell the without-probes time of `fib` moved
  by 4.24 % on M1 and 0.01 % on M2, and that of `tree` by 8.87 % and 3.51 %.
  M2's runs are steadier than M1's, but the two machines' figures for `fib` and
  for the back-edge check differ by more than the bar itself, so the overhead
  depends on the processor as much as on the probe. The owner's policy 1
  therefore reports the bar as measured and leaves the decision to the native
  era, on a machine with a PMU and a fixed frequency, with Sisu's own probe
  placement. The measurements are a warning for that decision, not a pass, and
  M2 adds one: a check on every back-edge can by itself cost more than the whole
  budget.
* **Not measured:** a vectorisable loop. A check inserted before vectorisation
  would prevent it, so a compiler must place yield checks after loop
  optimisation (per vector iteration, or every N iterations).

## 8. io_uring cancellation into stack buffers (amendment A1, D-058)

Two procedures per stackful strategy, five runs each on each machine, 1M reads
per run in batches of 8,192 in flight. `pipe_mix=cancel` is A1 as registered:
every read targets an empty pipe, so it stays in flight, and every one is
cancelled through its task; the cancelled call submits `IORING_OP_ASYNC_CANCEL`
and returns only on the read's own completion; afterwards late data is written
into every pipe, so any read the kernel still held would land, and every buffer
(4 KiB, on the task's stack, the frame kept alive until the batch ends) is
checked again. The default race mix (512-byte buffers) writes data for a quarter
of the reads, cancels a quarter, and does both, in either order, for half. B
also runs the file-backed variants: `O_DIRECT` reads cancelled while the device
holds them (`-ENOENT`, and a later completion), and buffered reads with
`IOSQE_ASYNC` (`-EALREADY`). `--break-contract` is the negative control: a
cancelled task stops waiting at once. In an "M1 / M2" column, a single figure
means that both machines measured the same.

| Configuration | reads per run | cancels submitted (M1 / M2) | cancel ok (M1 / M2) | -ENOENT (M1 / M2) | -EALREADY (M1 / M2) | completed after failed cancel (M1 / M2) | completed with the late data (M1 / M2) | buffer bytes written after return M1 | M2 | cancel to return p50 / p99, ms, M1 | M2 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A/1M | 1,000,000 [1,000,000-1,000,000] | 712,963 [696,220-728,737] / 724,041 [722,183-727,232] | 261,732 [254,227-275,448] / 293,302 [287,395-298,271] | 451,229 [420,770-474,510] / 430,726 [425,841-439,826] | 2 [0-4] / 11 [7-13] | 57,318 [46,567-68,436] / 58,916 [42,450-68,965] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 14.5 [13.0-15.0] / 31.8 [26.9-37.6] | 16.0 [15.3-16.9] / 27.5 [25.9-28.5] |
| A/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 20.9 [19.7-23.5] / 40.0 [33.2-41.7] | 25.1 [24.3-26.7] / 40.5 [38.9-43.5] |
| B/1M | 1,000,000 [1,000,000-1,000,000] | 723,567 [718,533-731,475] / 697,162 [686,395-699,760] | 256,689 [252,364-264,968] / 273,620 [271,497-278,031] | 464,686 [458,356-474,786] / 421,701 [414,535-425,505] | 1 [0-4] / 20 [19-28] | 51,685 [45,036-66,262] / 50,825 [36,948-66,166] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 12.5 [12.2-14.1] / 31.6 [26.2-32.9] | 13.6 [12.1-14.3] / 27.4 [25.0-29.9] |
| B/1M break_contract=true | 250,000 [250,000-250,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 195,718,784 (max 39,692,800 per run, 5 runs) | 192,795,904 (max 38,887,488 per run, 5 runs) | 0.11 [0.05-0.24] / 8.32 [4.48-9.00] | 2.15 [1.67-2.59] / 6.41 [5.82-16.4] |
| B/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [999,995-1,000,000] / 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-5] / 0 [0-0] | 0 [0-5] / 0 [0-0] | 0 [0-5] / 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 20.9 [19.1-29.5] / 36.5 [32.4-57.2] | 26.0 [25.4-28.4] / 42.9 [40.4-46.6] |
| B/1M source=file | 1,000,000 [1,000,000-1,000,000] | 411,736 [400,417-412,333] / 486,156 [484,624-486,242] | 0 [0-1] / 0 [0-0] | 411,736 [400,417-412,333] / 486,156 [484,624-486,242] | 0 [0-0] | 161,059 [157,692-168,755] / 399,436 [389,698-404,296] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 20.7 [20.2-21.6] / 48.1 [41.2-85.3] | 799 [797-812] / 855 [843-916] |
| B/1M source=file-buffered | 1,000,000 [1,000,000-1,000,000] | 325,368 [315,631-347,480] / 475,281 [472,451-479,049] | 5,601 [4,936-14,802] / 26,726 [15,457-39,664] | 314,929 [309,409-340,004] / 382,660 [374,365-391,753] | 593 [561-750] / 67,968 [57,159-72,028] | 117,827 [108,756-133,243] / 307,056 [279,514-319,707] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 86.5 [80.9-102] / 220 [215-249] | 606 [602-615] / 884 [806-915] |
| C/8K | 1,000,000 [1,000,000-1,000,000] | 724,601 [713,512-741,845] / 697,439 [691,343-707,825] | 255,661 [253,201-271,342] / 277,026 [273,676-280,433] | 468,939 [444,516-487,886] / 419,071 [413,329-430,784] | 1 [0-2] / 22 [15-22] | 63,914 [47,793-85,902] / 50,291 [38,885-62,251] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 13.5 [11.7-14.4] / 30.1 [26.7-38.0] | 13.1 [12.5-14.7] / 28.9 [26.5-29.6] |
| C/8K pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [999,997-1,000,000] / 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-3] / 0 [0-0] | 0 [0-3] / 0 [0-0] | 0 [0-3] / 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 20.8 [20.2-23.4] / 39.0 [36.5-46.7] | 25.6 [25.5-26.9] / 41.8 [41.3-43.6] |
| D(A)/1M | 1,000,000 [1,000,000-1,000,000] | 514,160 [494,354-524,949] / 672,830 [656,957-686,816] | 265,806 [262,533-273,929] / 279,846 [278,551-288,432] | 248,353 [229,563-254,640] / 390,024 [378,399-406,964] | 3 [1-5] / 7 [6-9] | 146,487 [135,449-175,474] / 168,749 [139,319-175,642] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 49.7 [39.2-53.0] / 92.5 [84.8-104] | 25.6 [25.2-27.3] / 43.3 [42.4-46.3] |
| D(A)/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 57.6 [55.2-65.9] / 125 [119-147] | 32.6 [32.2-33.3] / 55.8 [55.1-56.2] |
| D(B)/1M | 1,000,000 [1,000,000-1,000,000] | 732,197 [708,111-740,222] / 704,243 [678,513-714,456] | 261,803 [257,468-270,724] / 273,626 [270,766-281,985] | 471,566 [437,380-482,752] / 422,242 [406,737-438,237] | 2 [2-7] / 27 [16-39] | 42,273 [30,928-63,543] / 54,761 [48,643-69,952] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 13.7 [13.6-14.7] / 30.1 [28.5-45.0] | 14.0 [12.7-14.7] / 27.8 [26.0-43.2] |
| D(B)/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 1,000,000 [1,000,000-1,000,000] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 [0-0] | 0 (max 0 per run, 5 runs) | 0 (max 0 per run, 5 runs) | 21.1 [20.8-23.9] / 42.0 [36.6-51.0] | 25.5 [24.2-27.3] / 42.6 [38.7-44.6] |

* **Zero, as required, on both machines.** No byte of any buffer changed after
  its call returned, in any run of either procedure on A, B, C, D(A) and D(B),
  nor in the file variants: 0 bytes over 60 runs on M1 and 0 bytes over 60 on
  M2. The negative control writes 195,718,784 bytes over its five runs on M1 and
  192,795,904 on M2, so the check sees what it is meant to see.
* **A cancel that cannot stop the read.** A quarter of the reads carry
  `IOSQE_ASYNC`, which sends them to an io-wq worker thread, where a blocked
  pipe read can only be interrupted. A cancel that finds such a read already
  executing answers `-EALREADY` (58 times in the 50 pipe runs above on M1, 435
  in as many on M2), and the read usually ends at once. In M1's first full-scale
  group, 8 of 50,000,000 pipe reads did not: each stayed blocked until the
  harness wrote its late data at the round's 30-second deadline, and each call
  returned only after that completion, 29.992-29.995 s after its cancel (the
  slowest cancellation of each of the 4 runs concerned), with its buffer intact
  afterwards. In the runs above, 8 reads did on M1 and 0 on M2
  (`completed_with_late_data`). Waiting is what keeps a lent buffer safe, and
  D-058 requires it; the consequence is that a cancelled task waits as long as
  an uncancellable read does. That is a liveness question for the native runtime
  (re-issuing the cancel signals the worker again; pollable files can use
  poll-armed reads, which a cancel always stops), not a memory-safety one, and
  it reopens nothing. M1's first group's check counted these reads' late data as
  "written at return"; section 10.3 describes the fix and where the superseded
  rows are kept.
* **The case D-058 exists for** is the file-backed one: a cancel that finds the
  read already issued to the device (`-ENOENT`) while the read still completes
  later (161,059 [157,692-168,755] reads per run on M1, 399,436
  [389,698-404,296] on M2); the call waited for those completions, which on M2's
  disk take much longer: the cancel-to-return median of that variant is 799
  [797-812] ms on M2, against 20.7 [20.2-21.6] ms on M1.
* **Cancel-to-return latency** in the registered procedure is 20.9 [19.1-29.5]
  ms median and 36.5 [32.4-57.2] ms p99 for B on M1, and 26.0 [25.4-28.4] and
  42.9 [40.4-46.6] ms on M2: the harness cancels 8,192 reads in a burst, and
  each cancellation queues one request to the ring's thread, so the figure is
  dominated by the burst, not by one cancellation. M1's group ran before the
  quiet gate existed (section 10.2), so its latencies may include other work's
  CPU use; none of these latencies is a verdict input.

## 9. Verdict and Q-6 recommendation

### 9.1 Verdict per bet-3 row (harness era; provisional, see the status note)

Section 2.1 of proposal/20: **holds** means the threshold is met with the
pre-specified effect; **fails** means it is missed or a guard trips; anything
else, including too little data or machines that disagree, is **not shown**, and
no fallback is built on it. Section 2.4 states the two-machine rule for the
primary, and the plan asks for every result on two machines, so every row below
holds or fails only if both machines say so. Section 7.8: only the primary
(memory) decides the bet, the guard (exhaustion in ordinary code) can only stop
it, and each secondary triggers its own action.

| Row | Role | Verdict | M1 | M2 | Action (section 8.3) |
|---|---|---|---|---|---|
| Memory per idle task | primary | **holds** (provisional) | D(B) passes, w = 1.75 / 2.41 / 3.16 at 128 KiB / 1 MiB / 8 MiB, every five-run range far from 4; B (w = 7.28 at 8 MiB, 6.86 at 1 MiB) and C (7.08) fail after the excursion; A and D(A) fail at the VMA limit | D(B) passes at 128 KiB (w = 1.75); at 1 MiB and 8 MiB inconclusive: the 100k conditions pass (worst 2.41 and 3.16), the 1M cells are failed rows (memory guard at 950,272-962,560 and 712,704-733,184 tasks); B (w = 7.36 at 8 MiB, 6.94 at 1 MiB) and C (7.15) fail after the excursion; A and D(A) fail at the VMA limit | "holds at week 8": Q-6 is closed provisionally with the harness era label (section 9.2); the default reservation's 1M cells are shown on M1 only |
| Stack exhaustion | guard | harness part **holds**; the guard is not decided | every exhaustion on every strategy and reservation is a clean "stack exhausted", five runs each; the runtime works afterwards; 0 crashes; the reserve's need is at most 2,088 bytes of its 5,120 | the same; 0 crashes; at most 2,088 bytes | the corpus `--stats stack-depth` histogram decides at week 40 |
| Spawn plus join | secondary | **holds** | B 1.40x Go (95% bootstrap interval 1.16-1.73; range 1.15-1.92) | B 1.84x Go (95% bootstrap interval 1.31-1.86; range 1.26-1.86) | no design review |
| Scale: 1M idle tasks, 16 GiB, default sysctls | secondary | **fails for A and D(A)**; holds for B, C and D(B) | A and D(A) stop at 32,730 tasks; B, C, D(B), E and tokio, and Go and Java, park 1M tasks and keep all of them parked through the 60-second hold | the same, A and D(A) at 32,738 tasks, on 7.75 GiB | "fails on creation cost or VMA limits only": pooled stacks with software probes become the design. Policy 5 already makes D(B) the default, and these runs are the rerun on that strategy |
| Probe overhead (part of the scale row) | secondary | **not decided** (policy 1) | `fib` 13.3 [11.9-16.3] % (an upper bound), `tree` 4.5 [1.9-5.3] %, `calls` 0.0 [-0.1-0.9] %, back-edge yield check -0.1 [-0.2-0.5] % (B) | `fib` 8.6 [8.2-8.7] %, `tree` 2.2 [1.8-2.3] %, `calls` 0.2 %, back-edge yield check 3.5 % (B) | the native era, on a machine with a PMU and a fixed frequency, with Sisu's own probe placement |
| Echo p99 at 10k connections | secondary | **not shown** (one machine) | not run (descriptor limit); at 1000 connections B 0.88x, D(B) 0.85x and D(A) 3.73x Go | B 0.91x, D(B) 0.91x, D(A) 1.07x Go at 10k; at 1000, 0.79x, 0.87x and 1.12x | none; a second machine that runs the 10k cell (or a client in a second process on M1) would decide it |
| Idle RSS at 100k connections | reported | (no threshold) | at 9,900: D(B) 1.50x Go | at 100,000: D(B) 1.95x Go; A and D(A) failed | none |
| Starvation | secondary | **holds** with compiler-inserted yield points | B's p99 25.7 [25.0-30.0] us (K = 10k) and 229 [221-269] us (K = 100k) against Go's 39.3 [39.3-39.4] ms; without yield points the probe waits the whole run (10,999 [10,994-11,002] ms), reported and not counted (policy 8) | 23.6 [23.3-23.7] us and 316 [315-316] us against Go's 39.2 ms; without yield points 11,000 [10,998-11,000] ms | asynchronous preemption is not revisited |
| Cancellation with a lent buffer (A1) | secondary | **holds** | 0 bytes written after return in 50 runs on the five stackful strategies, 1M reads per run; the negative control detects its violations (195,718,784 bytes) | 0 bytes in 50 runs; the negative control 192,795,904 bytes | none; D-058 stands |
| Semantics, `sim` study | secondary | not run | interpreter era | interpreter era | Phases 6-9 |

**Bet 3 at week 8: holds, provisionally.** Through its primary: D(B), the
default strategy, passes section 2.4 on both machines at 128 KiB (w = 1.75 on
both machines) and on M1 at every reservation; the machines agree wherever both
measured, and M2 could not hold D(B)'s 1M cells at 1 MiB and 8 MiB. No guard has
tripped: the exhaustion guard is decided at week 40, against the default
reservation chosen here, and can still stop the bet. The verdict, like every
verdict in this report, stays provisional until the pre-registration is frozen.

### 9.2 Q-6 recommendation (provisional, harness era)

* **Strategy: D(B)**, pooled stacks carved from large reservations, with
  software probes and decommit on park, the probe doubling as the decommit
  watermark. It is the only stackful strategy that passes the memory primary: on
  both machines at 128 KiB, on M1 at every reservation, and on M2 in every
  condition it could measure. B and C fail it by commit creep (an excursion's
  pages stay resident) on both machines, and A and D(A) cannot create 100k tasks
  under default sysctls on either. Its cost over B is small: per spawn plus join
  0.67 [0.66-0.83] against 0.64 [0.64-0.80] us on M1 and 0.75 [0.75-0.76]
  against 0.73 [0.52-0.73] us on M2; echo p99 at 1000 connections 4.16
  [4.03-4.39] against 4.32 [4.13-6.73] ms on M1 and 8.85 [7.39-11.7] against
  8.05 [7.47-10.8] ms on M2. Decommit is needed, and its system call shows only
  where it is paid on every park: D(A) has no watermark and makes one `madvise`
  per park, and its echo throughput is 73,266 [72,715-74,131] requests per
  second against D(B)'s 318,935 [316,259-328,027] on M1, and 103,887
  [102,417-109,909] against 137,090 [118,321-143,329] on M2. In `idle-memory`,
  D(B) made one call per task, at the park after its excursion, and none at the
  park after shallow use.
* **Default reservation: 1 MiB, with one condition.** Under D(B) a reservation
  costs page tables, not resident memory: on M1, w = 1.75, 2.41 and 3.16 at 128
  KiB, 1 MiB and 8 MiB; on M2, w = 1.75 at 128 KiB, and the worst 100k
  conditions are 2.41 at 1 MiB and 3.16 at 8 MiB. 1 MiB keeps a margin under the
  bar for the runtime's growth, is the size of the JVM's default thread stack,
  and runs 21,900 levels of a small recursion and 2,856 levels of the four-frame
  parser before a clean failure, on both machines. The condition: section 2.4
  shows 1 MiB at 1M tasks on M1 alone, because M2 (7.75 GiB) could not hold 1M
  D(B) tasks at 1 MiB beside its 1 GiB guard. The reservation that passes on
  both machines is 128 KiB, which runs only 2,786 levels of the small recursion
  and 363 of the parser (M1; 2,785 and 363 on M2). So the 1 MiB default stands
  provisionally on M1's 1M cells and M2's 100k cells, and its 1M cells should be
  reproduced on a second machine with the 16 GiB that section 4.4 requires
  before the default is final. This does not change how the guard is decided:
  section 2.4 trips it only when an ordinary program needs a default above 8 MiB
  that then fails the memory primary. 8 MiB is admissible on memory on M1 (it
  completes the 10k-deep parse) and is the step to take if the corpus histogram
  finds ordinary programs deeper than 1 MiB; above 8 MiB, memory must be
  re-measured (the plan's exhaustion row).
* **Probes, not guard pages.** One guard page per task stops at 32,730 tasks on
  M1 and 32,738 on M2 under the default `vm.max_map_count`, and a guard-page
  trap discards the exhausted task's frames without running their destructors
  (86 drop guards skipped in the 1M-deep recursion at 1 MiB, on both machines;
  none under probes). With probes every exhaustion unwinds cleanly. The plan
  attaches a condition to this choice, probe overhead of at most 3 % on compute
  loops, and that condition is open: policy 1 leaves it to the native era, and
  at face value `fib` exceeds it on both machines, `tree` on M1 and the
  back-edge check on M2 (section 7). Two further conditions come with probes:
  the runtime must resolve the unwinder's lazily bound symbols before tasks run
  (or link them eagerly), which this phase found and fixed (section 10.3); and
  the native compiler's probe should compare the stack pointer minus the frame
  size before allocating the frame, so that the reserve covers only the unwinder
  and unprobed callees. With both, the policy's 5 KiB (5,120-byte) reserve
  leaves 3,032 bytes over the largest need measured on either machine.
* **`spawn` stays ambient**, without a capability. The research found no
  supply-chain attack that gating `spawn` would have stopped, and nothing in the
  harness argues otherwise: a task costs 0.67 [0.66-0.83] us (M1) and 0.75
  [0.75-0.76] us (M2) to spawn and join and 6.43 KiB at rest (M1 at 1M tasks;
  M2: 6.44 KiB at 100k), and the risk of code that spawns too many is a resource
  limit for the runtime to enforce (the scale row's guards), not an authority to
  grant.
* **Yield checks (Q-34)**, decided as compiler-inserted, need an interval and a
  placement. Yielding every 100k iterations of the test loop (slices of about
  202 [189-224] us on M1 and 183 [181-184] us on M2) held the probe's p99 at 229
  [221-269] and 316 [315-316] us, and every 10k at 25.7 [25.0-30.0] and 23.6
  [23.3-23.7] us. The switches cost little at those intervals: median throughput
  changes of +1.5 % and -2.9 % on M1, within its five runs' spread, and -0.2 %
  and -0.9 % on M2, which resolves them; every 1k iterations cost 18.1 % of the
  throughput on M1 and 7.3 % on M2. The check itself is not free everywhere: on
  every back-edge it cost nothing measurable on M1 and 3.5 % on M2 (section 7),
  more than the whole budget. A slice of a few hundred microseconds meets the
  starvation row by two orders of magnitude, so the compiler can afford to check
  far less often than every back-edge; how often, and where (after unrolling or
  vectorisation, or behind a counter), should be chosen to fit what the probes
  leave of the 3 % budget, which the native era measures.
* **The stackless fallback is not needed** on this evidence: the default
  strategy passes the memory primary on both machines with a wide margin (at 128
  KiB on both, at every reservation on M1), and every exhaustion is clean on
  both. Its cost stays on file
  ([`stackless-fallback-estimate.md`](stackless-fallback-estimate.md): 17-21
  engineer-weeks, plus 3 if frames become values), for the cases that would call
  for it: a 16 GiB second machine contradicting M1's 1M cells at the default
  reservation, or the corpus histogram (week 40) finding ordinary depth beyond 8
  MiB. The yield-point frame-boundary question is therefore not flagged to Phase
  3.

### 9.3 What the harness era cannot settle

* **The 3 % probe budget.** Neither machine has hardware counters, M1's
  run-to-run noise exceeds the bar, and the two machines' figures for `fib` and
  for the back-edge check differ by more than the bar (section 7). The decision
  waits for the native era on bare metal.
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
* **The registered echo and connection cells.** The 10k-connection echo and the
  100k idle connections ran on M2 only (M1's descriptor limit), and the
  open-loop load generator on disjoint cores of section 6.5 on neither machine.
* **A second machine with 16 GiB.** M2 has 7.75 GiB, so the 1M-task cells of B
  and D(B) at 1 MiB and 8 MiB are measured on M1 only.

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
| 1. Machines | two machines, the second the reviewer's, implicitly bare metal | M1 is a 4-vCPU KVM virtual machine without a PMU and with an unpinned frequency; M2 is a GitHub-hosted runner (2 vCPUs on one core, 7.75 GiB, a Hyper-V guest without a PMU), driven by a workflow rather than by a reviewer. The 3 % probe bar is reported with its noise and not decided (section 7) |
| 2. Context switch | "an assembly context switch from an existing library" | corosensei 0.3.x, unchanged since the harness was built |
| 3. Sysctls and limits | default sysctls; the earlier full-scale script needed `RLIMIT_NOFILE` of 200,064 | default sysctls on both machines; on M2 the workflow restores the kernel default `vm.max_map_count=65530`, which the runner image raises to 262144. The 10k echo and the 100k idle connections run only where the hard limit allows: on M1 (20,000) the 10k echo is a failed row and idle connections ran at 9,900; on M2 (1,048,576 inside the workflow's scope, section 10.2) both ran |
| 4. Go after the excursion | "within 4x Go", the Go figure unspecified | Go after four `runtime.GC()` and `FreeOSMemory` |
| 5. Default strategy | none named | D(B) |
| 6. Probe reserve | the harness defaults of 16 KiB (B) and 8 KiB (C) | 5 KiB for B, C and D(B), now the harness default (version 0.2.0) |
| 7. 1M tasks after the excursion | every strategy | only D(A) and D(B); A, B, C and Go measure the shallow phase at 1M (`--skip-excursion`) on both machines, and their cell reads "not run (policy 7)" |
| 8. Yield points | compute tasks "that yield every K iterations", Go with or without `Gosched` | compiler-inserted yield points every K = 10k and 100k iterations (and 1k); the baseline is Go with its default asynchronous preemption only; no yield points at all is reported, not counted |

### 10.2 Deviations from the harness protocol (proposal/20, section 6.5)

* **Machine 2's run, and the runs lost before it.** M2's results are the
  `harness-full` workflow's run 38518035636: one invocation of `run_all.sh
  --scale full` from 2026-10-14T05:42:47Z, five repetitions of the whole suite
  in rep-major order, without the quiet gate, inside the memory scope of section
  0. Two earlier full-scale attempts were lost (section 0): 38495423545, whose
  artifact was never uploaded, and 38513665561, which starved the runner agent.
  The first one's log summary is a cross-check, not data of record, and it
  agrees with this run where the two should agree: D(B)/1 MiB's 100k ratios were
  2.41 and 1.29 there (2.41 and 1.30 here), D(B)/128 KiB passed with w = 1.75,
  the memory guard stopped D(B)/1 MiB at 1M after 966,656 tasks, A stopped at
  32,738 tasks, and A1's runs on the five stackful strategies wrote 0 bytes
  after return. Its timing figures differ (B's spawn-join 1.14x Go there, 1.84x
  here; `fib`'s probe overhead 30.4 % there, 8.6 % here), and the summary does
  not record the machine, so it cannot say whether the runner's processor was
  the same.
* **M2 is smaller than the plan's machine.** Section 4.4 asks for at least 16
  GiB and the scale test in a 16 GiB memory cgroup; M2 has 8,322,932,736 bytes
  (7.75 GiB) and 2 vCPUs (`machine-spec.csv`), the size the workflow's comment
  gives for `ubuntu-latest` on a private repository. On it the memory guard
  stopped the 1M-task `idle-memory` cells of B and D(B) at 1 MiB and 8 MiB
  (section 0), so section 2.4 is inconclusive on M2 for D(B) at those
  reservations; `max-tasks`'s 1M pooled tasks fit (section 3). The workflow's 12
  GiB scope did not bind on this machine (section 0).
* **M2's descriptor limit.** Inside the scope the processes had an
  `RLIMIT_NOFILE` of 1048576/1048576; the runner's own processes had 65536/65536
  in the smoke run, and the lost run 38495423545, outside any scope, ran idle
  connections at 32,700, the most (in whole hundreds) that a hard limit of
  65,536 allows. The run does not record why the scope's limit was higher; it is
  most likely inherited through `sudo`, which starts the scope. The workflow did
  not raise the limit, but its benchmarks did not run under the runner's default
  either. Under that default the 10k echo would still have run (it needs
  20,064), and the 100k idle connections would have run at 32,700. Neither cell
  decides a verdict: the echo row is not shown, and idle RSS is reported only.
* **M2's kernel and other limits.** M2 runs Linux 6.17.0-1022-azure with
  io_uring enabled, which meets section 4.4's "6.1 or later"; the workflow
  restores `vm.max_map_count` to 65530 (the image's is 262144), so A and D(A)
  meet the same wall as on M1. The scope's pids cgroup (9491 tasks) stops OS
  threads at 9,489, below M1's 16,128 (where the mapping limit stops them first)
  and below the 10k OS threads of `idle-memory`, whose M2 cell is a failed row.
  The descriptor limit stopped no cell on M2: the A and D(A) idle-connection
  failures at 100k come, by the harness's code path, from the mapping limit
  (section 4). Hardware counters are unavailable there too ("hardware counters
  unavailable: Permission denied (os error 13)").
* **M2's huge pages and swap.** The workflow sets transparent huge pages to
  `madvise`, as on M1, and turns swap off. Under the image's `always`, Go's
  goroutines held 66.4 KiB each after the excursion in the smoke run (before its
  GC), which on M2's memory would push Go's 100k cell into the memory guard.
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
  6). M2 is a hosted runner that runs nothing but the job, measured without the
  gate: during its kept processes the other processes used a median of at most
  0.047 CPUs per benchmark (section 11). 32 of its 730 processes saw them use
  0.3 CPUs or more, none of them longer than 0.64 s: 20 in spawn-join (the
  pooled strategies' cells, section 2), 11 in deep-recursion and 1 in max-tasks,
  whose figures do not depend on CPU time.
* **Workers** are the VMs' vCPUs: 4 on M1, which the guest sees as four
  single-thread cores, and 2 on M2, which it sees as one core's two hardware
  threads. Section 4.4's default is one worker per physical core; on M2 that
  would be one.
* **Memory** is resident plus page tables (VmRSS + VmPTE from
  `/proc/self/status`), the harness's figure of record since its first version;
  section 6.5 says "resident set" from `/proc/self/smaps_rollup`. The RSS-only
  figures are in the CSVs and in `summarize.py`'s output, and every verdict is
  the same on them. The harness reads its baseline after runtime start-up
  without the 1,000-task warm-up of section 6.5; at most the first 1,000 tasks
  could find warm memory, so the effect is at most 1 % at 100k and conservative
  (it raises the per-task figure) for every implementation.
* **Echo** is a closed loop with the clients in the same process and on the same
  vCPUs, measured for 30 s after 5 s of warm-up; section 6.5 asks for an
  open-loop load generator on disjoint cores at a fixed rate, 60 s after 10 s of
  warm-up, at 10k connections. M2's 10k figures and both machines'
  1000-connection figures are the harness's comparison of the runtimes, not the
  registered measurement.
* **Starvation** runs four compute tasks per worker as section 6.5 asks (16 on
  M1, 8 on M2), for 11 s, which gives each taskbench run about 10k wake-ups;
  Go's probe wakes much later under load, so its runs have fewer wake-ups
  (section 6 lists them).
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
* **Controls.** Go is go1.24.7 on M1 and go1.24.13 on M2, not the latest stable
  release; the JDK is 21.0.12 on both (Ubuntu's build on M1, Temurin's on M2;
  JEP 491's unpinning is 24 and later). The "C or Rust stackful coroutine
  library" control is strategy A itself (corosensei's own default stack is a
  guarded mapping per coroutine), not a separate library.
* **Reviewer.** No Go/Java reviewer has configured or checked the controls, M2
  was run by a workflow rather than by the reviewer, and no referee has applied
  the thresholds; the verdicts are the runtime engineer's reading of the draft
  rules.

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
  the full extended register state on the current stack: about 3 KiB on M1's
  AVX-512 processor. The first stack exhaustion of every process therefore
  needed about 5 KiB of reserve and later ones about 1.8-2.1 KiB; the earlier "5
  KiB when landing pads run drop code" was this one-time cost, not drop code
  (with `LD_BIND_NOW=1` the first unwind also needs 1.8 KiB). M2's smoke run,
  made before the fix, shows the same pattern with a smaller register state:
  3,416 bytes for the first unwind and 1,816 for the next. With the 5 KiB
  default, the unit test `exhaustion_is_a_clean_task_failure_on_every_strategy`
  overran a B slot's reserve, and its canary caught it. The runtime now resolves
  those bindings once, on an ordinary stack, before it starts its workers
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
  `51e42157...` at 20:50 UTC for max-tasks and did not rebuild afterwards. M2
  built `6a1fedf5...` with its own compiler (section 0).
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
  bootstrap intervals, the policies' baselines and section 2.4's rule, on each
  machine and on both, and gives the range of tasks created over a cell's runs
  when the memory guard stops it; `report/tables.py` puts the two machines side
  by side in every table. The report's scripts change no measurement and are
  outside the sources' digest.

### 10.4 Not run, and why

| Cell | Machine | Why |
|---|---|---|
| Echo at 10k connections | M1 | both ends in one process need 2 x 10,000 + 64 = 20,064 descriptors; the hard `RLIMIT_NOFILE` is 20,000 and cannot be raised (policy 3). A client in a second process would fit, and is not implemented |
| Idle connections at 100k | M1 | the same limit; run at 9,900, the most it allows |
| 1M tasks after the excursion for A, B, C and Go | M1, M2 | policy 7; about 33 GiB |
| `idle-memory` at 1M tasks: B and D(B) at 1 MiB and 8 MiB | M2 | failed rows: the memory guard stopped them at 946,176-962,560 (B/1 MiB), 675,840-733,184 (B/8 MiB), 950,272-962,560 (D(B)/1 MiB) and 712,704-733,184 (D(B)/8 MiB) of 1,000,000 tasks, on a 7.75 GiB machine; the 12 GiB scope did not bind (section 0) |
| Idle connections at 100k for A and D(A) | M2 | failed rows: the client's `connect` timed out in every run; a guarded mapping per server task meets the VMA limit near 32,738 tasks (section 4) |
| OS threads beyond 9,489 (`idle-memory` at 10k, `max-tasks`) | M2 | the scope's pids cgroup of 9491 tasks: thread creation fails with `EAGAIN`, a failed row |
| Instruction counts for probe overhead | M1, M2 | no PMU on either machine |
| The semantics rows, the `sim` study, the corpus stack-depth histogram, interpreter starvation | both | interpreter era (Phases 6-9), not Phase 1 |

### 10.5 Measured on one machine only

On M1 only: the 1M-task `idle-memory` cells of B and D(B) at 1 MiB and 8 MiB,
which M2's memory guard stopped, and OS threads beyond M2's pids limit. On M2
only: the 10k-connection echo and idle connections at 100k (where A and D(A)
failed), which M1's descriptor limit excludes. Every other full-scale cell (100k
and 1M tasks, five runs, 1M cancelled reads, the 60-second hold, starvation with
four tasks per worker, the framed recursion, the back-edge yield check) was
measured on both machines, five runs each.

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

On M2, the workflow `.github/workflows/harness-full.yml` restored the kernel's
`vm.max_map_count`, turned swap off, set transparent huge pages to `madvise`,
installed JDK 21 and Go 1.24, built the harness, and ran one invocation for the
whole suite inside the memory scope of section 0:

```
./run_all.sh --scale full --machine-id github-runner --out results/github-runner/full-${GITHUB_RUN_ID}
```

The five repetitions run in rep-major order over the whole suite (every
benchmark and its controls once, then again), without the quiet gate, since a
hosted runner runs nothing but the job. Wall-clock time per group, from its
`timing.csv`:

| Benchmark (with its controls) | first start (UTC) | repetitions | minutes | seconds per repetition |
|---|---|---|---|---|
| spawn-join | 2026-10-14 05:43 | 5 | 0.4 | 5-5 |
| idle-memory | 2026-10-14 05:43 | 5 | 16.5 | 197-200 |
| max-tasks | 2026-10-14 05:46 | 5 | 38.8 | 465-466 |
| echo | 2026-10-14 05:54 | 5 | 53.5 | 641-642 |
| idle-connections | 2026-10-14 06:04 | 5 | 75.1 | 893-914 |
| deep-recursion | 2026-10-14 06:20 | 5 | 0.1 | 1-1 |
| starvation | 2026-10-14 06:20 | 5 | 21.1 | 253-253 |
| probe-overhead | 2026-10-14 06:24 | 5 | 5.4 | 64-65 |
| cancel-uring | 2026-10-14 06:25 | 5 | 32.9 | 392-400 |
| total |  |  | 243.8 |  |

The machine's load during every process of M1's gated groups and of every M2
group, from `load.csv` (the other processes' CPU time is all CPUs' busy time
from `/proc/stat` less the process's own; it can be a few ticks below zero):

**Machine load during the runs (load.csv)**

| results | benchmark | quiet below (CPUs) | processes kept | attempts measured again | gate timeouts | kept although busy | others' CPUs while kept: median | max | steal (CPUs): median | gate wait (s): total / longest |
|---|---|---|---|---|---|---|---|---|---|---|
| vm/2026-10-13-phase1 | echo | 0.3 | 80 | 7 | 0 | 2 | 0.015 | 0.707 | 0.040 | 504 / 88 |
| vm/2026-10-13-phase1 | idle-connections | 0.3 | 45 | 9 | 0 | 0 | 0.038 | 0.291 | 0.003 | 167 / 36 |
| vm/2026-10-13-phase1 | probe-overhead | 0.3 | 25 | 10 | 0 | 0 | 0.067 | 0.285 | 0.012 | 99 / 16 |
| vm/2026-10-13-phase1 | spawn-join | 0.3 | 70 | 4 | 0 | 0 | 0.034 | 0.349 | 0.009 | 150 / 4 |
| vm/2026-10-13-phase1 | starvation | 0.3 | 105 | 34 | 0 | 2 | 0.042 | 1.008 | 0.034 | 588 / 76 |
| github-runner/full-38518035636 | cancel-uring | off | 65 | 0 | 0 | 0 | 0.009 | 0.037 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | deep-recursion | off | 85 | 0 | 0 | 0 | 0.000 | 0.763 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | echo | off | 90 | 0 | 0 | 0 | 0.009 | 0.029 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | idle-connections | off | 45 | 0 | 0 | 0 | 0.003 | 0.024 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | idle-memory | off | 195 | 0 | 0 | 0 | 0.022 | 0.220 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | max-tasks | off | 50 | 0 | 0 | 0 | 0.007 | 0.483 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | probe-overhead | off | 25 | 0 | 0 | 0 | 0.008 | 0.031 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | spawn-join | off | 70 | 0 | 0 | 0 | 0.047 | 0.845 | 0.000 | 0 / 0 |
| github-runner/full-38518035636 | starvation | off | 105 | 0 | 0 | 0 | 0.006 | 0.045 | 0.000 | 0 / 0 |

The other work on M1 came in bursts: the gate measured a process again 64 times.
Four processes were kept on their third attempt although the other processes had
used more than 0.3 CPUs while they ran (the last attempt is kept whatever its
load): two starvation runs at K = 10k (C and D(A)) and two echo runs at 1000
connections (A and C). The two echo runs have the highest p99 of their cells,
and the two starvation runs lie within their cells' ranges, far under the
threshold; none of these cells decides a verdict. On M2 no process was measured
again (the gate was off); section 10.2 counts the processes during which the
other processes used 0.3 CPUs or more.
