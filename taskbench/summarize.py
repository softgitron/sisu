#!/usr/bin/env python3
"""Prints the bet-3 tables from one or more results directories (results/<machine>/<date>).

Usage: python3 summarize.py [--markdown] RESULTS_DIR [RESULTS_DIR...]

Reads the CSVs written by run_all.sh (taskbench and the Go and Java controls,
one schema). Every cell is one configuration on one machine; when a cell ran
several times (run_all.sh --reps, five at full scale), the value shown is the
median over the runs with the range [min-max] and the number of runs, as the
harness protocol asks (proposal/20, section 6.5). Ratios to Go are ratios of
medians, against Go on the same machine with the baselines of the owner's
policies: memory after the excursion against Go after its garbage collection,
starvation against Go with its default asynchronous preemption (no
runtime.Gosched). Ratio intervals are 95% percentile bootstrap intervals from
10,000 resamples of the runs (section 7.2; reported, never used for a decision),
followed by the conservative range [min/max, max/min] that section 2.4 uses to
call a cell inconclusive when it crosses a threshold. The last table is the
machine's load during the runs (load.csv, written by load.sh): how many
processes were measured again because other processes were busy, and how many
CPUs the other processes used during the processes that were kept. Standard
library only.
"""
import csv
import glob
import math
import os
import random
import re
import statistics
import sys
from collections import defaultdict


def load(dirs):
    rows = []
    for d in dirs:
        for path in sorted(glob.glob(os.path.join(d, "*.csv"))):
            if os.path.basename(path) in ("machine-spec.csv", "timing.csv", "load.csv"):
                continue
            with open(path, newline="") as f:
                for r in csv.DictReader(f):
                    r["_dir"] = d
                    rows.append(r)
    return rows


CONTROLS = ("go", "java", "tokio", "os-threads")

# Parameters that change what a run measures, with the value that is left out
# of the label because it is the default (None: always shown when present).
VARIANT_DEFAULTS = {
    "batch": "1",
    "yield_every": "10000",
    "source": "pipe",
    "pipe_mix": "race",
    "break_contract": "false",
    "scope_per_round": "false",
    "stack_cache": None,
    "probe_reserve": None,
    "guard_bytes": None,
    "chunk_bytes": None,
    "canary": None,
}
# Parameters that are part of the label for some benchmarks only.
PER_BENCH = {"echo": ("connections",), "idle-connections": ("connections",)}


def human(b):
    for unit, size in (("G", 1 << 30), ("M", 1 << 20), ("K", 1 << 10)):
        if b >= size and b % size == 0:
            return f"{b // size}{unit}"
    return str(b)


def params_of(r):
    return dict(kv.split("=", 1) for kv in r["params"].split(";") if "=" in kv)


def config(r):
    """(label, variant): the label separates configurations of one implementation
    and strategy; the variant is the part a control can share (for example
    connections=1000), used to pick the Go cell a configuration is compared with."""
    if r["implementation"] in CONTROLS:
        s = r["implementation"]
    else:
        s = r["strategy"]
        if s.startswith(("A", "B", "D")) and r["reservation_bytes"]:
            s += "/" + human(int(r["reservation_bytes"]))
        if s == "C" and r["stack_bytes"]:
            s += "/" + human(int(r["stack_bytes"]))
    p = params_of(r)
    variant = ""
    for key, default in VARIANT_DEFAULTS.items():
        if key in p and p[key] != default:
            variant += f" {key}={p[key]}"
    for key in PER_BENCH.get(r["benchmark"], ()):
        if key in p:
            variant += f" {key}={p[key]}"
    if r["benchmark"] in ("idle-memory", "max-tasks") and r["tasks"]:
        variant += f" N={r['tasks']}"
    return s + variant, variant


class Cell:
    """One configuration on one machine: its metrics over every run."""

    def __init__(self, variant):
        self.variant = variant
        self.values = defaultdict(list)  # metric -> [value per run]
        self.status = defaultdict(list)  # metric -> [(status, note)] per row
        self.runs = set()

    def med(self, metric):
        v = self.values.get(metric)
        return statistics.median(v) if v else None

    def rng(self, metric):
        v = self.values.get(metric)
        return (min(v), max(v), len(v)) if v else None

    def bad(self, metric):
        """The first non-ok status of the metric, with its note, if any."""
        for st, note in self.status.get(metric, []):
            if st != "ok":
                return st, note
        return None


def cells_of(rows):
    cells = {}
    for r in rows:
        if r["implementation"] == "machine":
            continue
        label, var = config(r)
        key = (r["machine_id"], r["benchmark"], label)
        c = cells.setdefault(key, Cell(var))
        c.runs.add(r["run_id"])
        if r["value"] != "":
            c.values[r["metric"]].append(float(r["value"]))
        c.status[r["metric"]].append((r["status"], r["note"]))
    return cells


def bootstrap_ratio(a, b, n=10_000, seed=20261013):
    """95% percentile bootstrap interval of median(a) / median(b), resampling runs."""
    if len(a) < 2 or len(b) < 2:
        return None
    rnd = random.Random(seed)
    out = []
    for _ in range(n):
        ma = statistics.median(rnd.choices(a, k=len(a)))
        mb = statistics.median(rnd.choices(b, k=len(b)))
        if mb:
            out.append(ma / mb)
    if not out:
        return None
    out.sort()
    return out[int(0.025 * len(out))], out[min(len(out) - 1, int(0.975 * len(out)))]


def ratio(cell, metric, ref, ref_metric=None):
    """(median ratio, bootstrap interval, conservative range) or None."""
    ref_metric = ref_metric or metric
    a = cell.values.get(metric) if cell else None
    b = ref.values.get(ref_metric) if ref else None
    if not a or not b or not statistics.median(b) or cell.bad(metric) or ref.bad(ref_metric):
        return None
    r = statistics.median(a) / statistics.median(b)
    lo_hi = (min(a) / max(b), max(a) / min(b)) if min(b) else None
    return r, bootstrap_ratio(a, b), lo_hi


def fmt_ratio(x):
    if x is None:
        return ""
    r, ci, lohi = x
    g = "{:.3g}".format
    s = g(r)
    if ci:
        s += f" (CI {g(ci[0])}-{g(ci[1])}"
        if lohi:
            s += f"; range {g(lohi[0])}-{g(lohi[1])}"
        s += ")"
    return s


def scaled(v, unit):
    if unit == "KiB":
        return v / 1024
    if unit == "us":
        return v / 1e3
    if unit == "ms":
        return v / 1e6
    return v


def fmt_num(v, unit, digits=None):
    x = scaled(v, unit)
    if digits is None:
        digits = 0 if abs(x) >= 100 or float(x).is_integer() else (1 if abs(x) >= 10 else 2)
    s = f"{x:,.{digits}f}"
    # a value that rounds to zero is shown without a sign ("-0.0" reads as negative)
    return s[1:] if s.startswith("-") and float(s[1:].replace(",", "")) == 0 else s


def fmt_cell(cell, metric, unit="", digits=None):
    """median [min-max] (n), or the status and note of a cell that did not measure."""
    if cell is None:
        return "not run"
    bad = cell.bad(metric)
    rg = cell.rng(metric)
    if rg is None:
        if bad:
            return f"{bad[0]}: {bad[1][:110]}"
        return ""
    lo, hi, n = rg
    m = cell.med(metric)
    s = fmt_num(m, unit, digits)
    if n > 1:
        s += f" [{fmt_num(lo, unit, digits)}-{fmt_num(hi, unit, digits)}]"
    s += f" (n={n})"
    if bad:
        s += f" {bad[0]}: {bad[1][:90]}"
    return s


def print_table(title, head, lines, markdown):
    print(f"\n## {title}\n")
    if markdown:
        print("| " + " | ".join(head) + " |")
        print("|" + "---|" * len(head))
        for line in lines:
            print("| " + " | ".join(c.replace("|", "/") for c in line) + " |")
    else:
        print(" | ".join(head))
        for line in lines:
            print(" | ".join(line))


def machines(cells, bench):
    return sorted({k[0] for k in cells if k[1] == bench})


def labels(cells, machine, bench):
    return sorted(k[2] for k in cells if k[0] == machine and k[1] == bench)


def go_cell(cells, machine, bench, variant="", fallback=True):
    """The Go cell on `machine` with `variant`, else (fallback) the plainest Go cell."""
    c = cells.get((machine, bench, "go" + variant))
    if c is not None or not fallback:
        return c
    gos = sorted((len(k[2]), k[2]) for k in cells if k[0] == machine and k[1] == bench and k[2].split(" ")[0] == "go")
    return cells[(machine, bench, gos[0][1])] if gos else None


def t_idle_memory(cells, md):
    b = "idle-memory"
    for m in machines(cells, b):
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            n_var = " N=" + lab.split(" N=")[1] if " N=" in lab else ""
            go = go_cell(cells, m, b, n_var, fallback=False)
            exc_go = "total_per_task_excursion_after_gc"
            lines.append([
                lab,
                fmt_cell(c, "total_per_task_shallow", "KiB"),
                fmt_cell(c, "total_per_task_excursion", "KiB"),
                fmt_cell(c, exc_go, "KiB") if lab.startswith("go") else "",
                fmt_cell(c, "rss_per_task_shallow", "KiB"),
                fmt_cell(c, "pte_per_task_shallow", "KiB"),
                fmt_ratio(ratio(c, "total_per_task_shallow", go)),
                fmt_ratio(ratio(c, "total_per_task_excursion", go, exc_go)),
                fmt_ratio(ratio(c, "rss_per_task_shallow", go)),
                fmt_ratio(ratio(c, "rss_per_task_excursion", go, "rss_per_task_excursion_after_gc")),
            ])
        print_table(f"idle-memory on {m}: KiB per task (RSS + page tables), median [range] (runs)",
                    ["configuration", "shallow", "after 32 KiB excursion", "Go after GC", "RSS only, shallow",
                     "page tables, shallow", "shallow / Go", "excursion / Go after GC", "RSS-only shallow / Go",
                     "RSS-only excursion / Go after GC"], lines, md)


def brief(why):
    """A short form of a failure note: the limit and how far the run got."""
    m = re.search(r"created (\d+) of (\d+): (.*)", why)
    if m:
        created, wanted, rest = int(m.group(1)), int(m.group(2)), m.group(3)
        limit = "VMA limit" if "VMA limit" in rest else ("memory guard" if "headroom" in rest else rest[:40])
        return f"{limit} at {created:,} of {wanted:,} tasks"
    return why[:70]


def t_memory_rule(cells, md):
    """Section 2.4 of proposal/20: for each stackful configuration, its worst ratio
    w to Go over the four memory conditions (100k and 1M tasks, shallow and after
    the excursion). Baselines: Go shallow; after the excursion, Go after its
    collection (policy 4). The 1M cell after the excursion is required only of
    strategies that release pages on park (policy 7); Go's own cell there does
    not fit 16 GiB, so the substitute baselines are Go after GC at 100k (same
    condition, nearest N) and Go shallow at 1M (a lower bound for Go after GC at
    1M); the larger of the two ratios is used. A configuration passes at w <= 4;
    a cell whose conservative range crosses 4, or that the memory guard stopped
    because the machine is too small, is inconclusive."""
    b = "idle-memory"
    for m in machines(cells, b):
        def cell(label, n):
            return cells.get((m, b, f"{label} N={n}"))

        go = {n: cell("go", n) for n in (100000, 1000000)}
        configs = sorted({k[2].split(" N=")[0] for k in cells if k[0] == m and k[1] == b
                          and k[2].split("/")[0] in ("A", "B", "C", "D(A)", "D(B)")})
        if not any(go.values()):
            print(f"\n## memory rule (proposal/20, section 2.4) on {m}: no Go cell at 100k or 1M tasks; not computable")
            continue
        lines = []
        for lab in configs:
            conds, notes, worst, crosses = [], [], None, False
            releases = lab.startswith("D(")
            for n in (100000, 1000000):
                c = cell(lab, n)
                for phase in ("shallow", "excursion"):
                    metric = f"total_per_task_{phase}"
                    if n == 1000000 and phase == "excursion" and not releases:
                        conds.append("not required (policy 7)")
                        continue
                    if c is None:
                        conds.append("not run")
                        continue
                    bad = c.bad(metric)
                    if bad and bad[0] == "failed" or (c.bad("tasks_created") or ("", ""))[0] == "failed":
                        why = (c.bad("tasks_created") or bad)[1]
                        if "memory headroom below" in why:
                            # The machine is too small for the cell (the guard
                            # stopped it): no value, not a failure of the strategy.
                            conds.append("not computable: " + brief(why))
                            crosses = True
                            continue
                        conds.append("fails: " + brief(why))
                        worst = float("inf")
                        continue
                    if phase == "shallow":
                        refs = [("Go", go[n], metric)]
                    elif n == 100000 or (go[n] and not go[n].bad("total_per_task_excursion_after_gc")):
                        refs = [("Go after GC", go[n], "total_per_task_excursion_after_gc")]
                    else:
                        refs = [("Go after GC at 100k", go[100000], "total_per_task_excursion_after_gc"),
                                ("Go shallow at 1M", go[1000000], "total_per_task_shallow")]
                    parts = []
                    for name, ref, ref_metric in refs:
                        r = ratio(c, metric, ref, ref_metric)
                        if r is None:
                            parts.append(f"no {name}")
                            continue
                        parts.append(f"{fmt_ratio(r)} vs {name}")
                        worst = r[0] if worst is None else max(worst, r[0])
                        if r[2] and r[2][0] <= 4 < r[2][1]:
                            crosses = True
                    conds.append("; ".join(parts))
            if worst is None:
                verdict = "not computable"
            elif worst > 4:
                verdict = f"fails (w = {worst:.3g})" if worst != float("inf") else "fails"
            elif crosses:
                verdict = f"inconclusive (w = {worst:.3g}; a range crosses 4 or a cell did not fit)"
            else:
                verdict = f"passes (w = {worst:.3g})"
            lines.append([lab] + conds + [verdict])
        print_table(f"memory rule (proposal/20, section 2.4) on {m}: ratio to Go, worst over four conditions",
                    ["configuration", "100k shallow", "100k after excursion", "1M shallow", "1M after excursion",
                     "verdict on this machine"], lines, md)


def t_spawn_join(cells, md):
    b = "spawn-join"
    for m in machines(cells, b):
        go = go_cell(cells, m, b)
        lines = [[lab, fmt_cell(cells[(m, b, lab)], "spawn_join_p50", "us"),
                  fmt_cell(cells[(m, b, lab)], "spawn_join_p99", "us"),
                  fmt_ratio(ratio(cells[(m, b, lab)], "spawn_join_p50", go)),
                  fmt_ratio(ratio(cells[(m, b, lab)], "spawn_join_p99", go))]
                 for lab in labels(cells, m, b)]
        print_table(f"spawn-join on {m}: microseconds per spawn plus join",
                    ["configuration", "median", "p99", "median / Go", "p99 / Go"], lines, md)


def t_max_tasks(cells, md):
    b = "max-tasks"
    for m in machines(cells, b):
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            limit = sorted({note for st, note in c.status.get("limit", [])})
            lines.append([lab, fmt_cell(c, "max_tasks", "", 0), ", ".join(limit),
                          fmt_cell(c, "total_per_task_at_stop", "KiB"), fmt_cell(c, "vmas_at_stop", "", 0),
                          fmt_cell(c, "hold_secs", "", 1), fmt_cell(c, "parked_after_hold", "", 0),
                          fmt_cell(c, "total_per_task_after_hold", "KiB")])
        print_table(f"max-tasks on {m}",
                    ["configuration", "tasks created", "limit", "KiB per task at stop", "VMAs at stop", "held (s)",
                     "still parked after the hold", "KiB per task after the hold"], lines, md)


def t_echo(cells, md):
    b = "echo"
    for m in machines(cells, b):
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            go = go_cell(cells, m, b, c.variant, fallback=False)
            if c.bad("echo"):
                lines.append([lab, fmt_cell(c, "echo"), "", "", "", ""])
                continue
            lines.append([lab, fmt_cell(c, "echo_rtt_p50", "ms"), fmt_cell(c, "echo_rtt_p99", "ms"),
                          fmt_cell(c, "echo_rtt_p999", "ms"), fmt_cell(c, "echo_throughput", "", 0),
                          fmt_ratio(ratio(c, "echo_rtt_p99", go))])
        print_table(f"echo on {m}: round trip in ms, requests per second",
                    ["configuration", "p50", "p99", "p99.9", "req/s", "p99 / Go"], lines, md)


def t_idle_connections(cells, md):
    b = "idle-connections"
    for m in machines(cells, b):
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            go = go_cell(cells, m, b, c.variant, fallback=False)
            if c.bad("idle_connections"):
                lines.append([lab, fmt_cell(c, "idle_connections"), "", ""])
                continue
            lines.append([lab, fmt_cell(c, "total_per_connection", "KiB"), fmt_cell(c, "rss_per_connection", "KiB"),
                          fmt_ratio(ratio(c, "total_per_connection", go))])
        print_table(f"idle-connections on {m}: KiB per idle connection",
                    ["configuration", "RSS + page tables", "RSS", "total / Go"], lines, md)


def outcome(c, metric):
    notes = sorted({note.split(";")[0] for st, note in c.status.get(metric, [])})
    bad = [st for st, note in c.status.get(metric, []) if st != "ok"]
    return "/".join(notes) + (f" ({len(bad)} not clean)" if bad else "")


def t_deep_recursion(cells, md):
    b = "deep-recursion"
    for m in machines(cells, b):
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            reserve = {k.split("_")[0]: max(c.values[k])
                       for k in ("parse_probe_reserve_used", "naive_probe_reserve_used", "framed_probe_reserve_used")
                       if c.values.get(k)}
            framed = ""
            if c.status.get("framed_outcome"):
                framed = (outcome(c, "framed_outcome") + ", depth " + fmt_cell(c, "framed_depth_reached", "", 0)
                          + ", guards skipped " + fmt_cell(c, "framed_leaked_drop_guards", "", 0))
            lines.append([lab, outcome(c, "parse_outcome"), fmt_cell(c, "parse_depth_reached", "", 0),
                          outcome(c, "naive_outcome"), fmt_cell(c, "naive_depth_reached", "", 0),
                          outcome(c, "naive-async_outcome") if c.status.get("naive-async_outcome") else "",
                          fmt_cell(c, "naive_leaked_drop_guards", "", 0), framed,
                          " / ".join(f"{k} {v:.0f}" for k, v in reserve.items()),
                          fmt_cell(c, "runtime_healthy_after", "", 0)])
        print_table(f"deep-recursion on {m}",
                    ["configuration", "10k parse", "parse depth", "1M naive", "naive depth", "naive async (E, tokio)",
                     "drop guards skipped", "framed (256-byte frames, drop code)", "max reserve bytes used",
                     "healthy after"], lines, md)


def t_starvation(cells, md):
    b = "starvation"
    for m in machines(cells, b):
        # The baseline: Go with its default asynchronous preemption (yield_every=0).
        go = cells.get((m, b, "go yield_every=0")) or go_cell(cells, m, b)
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            lines.append([lab, fmt_cell(c, "slice_between_yields", "us"), fmt_cell(c, "probe_baseline_late_p99", "us"),
                          fmt_cell(c, "probe_loaded_late_p50", "us"), fmt_cell(c, "probe_loaded_late_p99", "us"),
                          fmt_cell(c, "probe_loaded_late_max", "us"), fmt_cell(c, "probe_loaded_late_samples", "", 0),
                          fmt_ratio(ratio(c, "probe_loaded_late_p99", go)),
                          fmt_cell(c, "compute_iterations_per_sec", "", 0)])
        print_table(f"starvation on {m}: probe lateness in microseconds (Go baseline: default preemption)",
                    ["configuration", "slice between yields", "p99 alone", "p50 loaded", "p99 loaded", "max loaded",
                     "wake-ups", "p99 / Go", "compute iterations/s"], lines, md)


def t_probe_overhead(cells, md):
    b = "probe-overhead"
    for m in machines(cells, b):
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            if c.bad("probe_overhead"):
                continue
            lines.append([lab] + [fmt_cell(c, f"{w}_{v}_probe_overhead", "", 1)
                                  for w in ("fib", "calls", "tree", "loop") for v in ("split",)]
                         + [fmt_cell(c, f"calls_{v}_probe_overhead", "", 1) for v in ("arg", "tls")])
        print_table(f"probe-overhead on {m}: percent over the same code without probes",
                    ["configuration", "fib split", "calls split", "tree split", "loop (back-edge check)",
                     "calls arg", "calls tls"], lines, md)


def t_cancel_uring(cells, md):
    b = "cancel-uring"
    for m in machines(cells, b):
        lines = []
        for lab in labels(cells, m, b):
            c = cells[(m, b, lab)]
            if c.bad("cancel_uring"):
                lines.append([lab, fmt_cell(c, "cancel_uring")] + [""] * 9)
                continue
            w = c.values.get("buffer_writes_after_return", [])
            lines.append([lab, fmt_cell(c, "reads", "", 0), fmt_cell(c, "cancels_submitted", "", 0),
                          fmt_cell(c, "ended_ecanceled", "", 0), fmt_cell(c, "cancel_ok", "", 0),
                          fmt_cell(c, "cancel_enoent", "", 0), fmt_cell(c, "cancel_ealready", "", 0),
                          fmt_cell(c, "completed_after_failed_cancel", "", 0),
                          fmt_cell(c, "completed_with_late_data", "", 0) if c.values.get("completed_with_late_data") else "",
                          f"{max(w):.0f} max, {sum(w):.0f} total" if w else "",
                          (fmt_cell(c, "cancel_to_return_p50", "ms") + " / " + fmt_cell(c, "cancel_to_return_p99", "ms"))
                          if c.values.get("cancel_to_return_p50") else ""])
        print_table(f"cancel-uring on {m} (per run; buffer writes after return summed over runs)",
                    ["configuration", "reads", "cancels submitted", "ended -ECANCELED", "cancel ok", "-ENOENT",
                     "-EALREADY", "completed after failed cancel", "completed with the late data",
                     "buffer bytes written after return", "cancel to return p50 / p99 (ms)"], lines, md)


def t_load(dirs, md):
    lines = []
    for d in dirs:
        path = os.path.join(d, "load.csv")
        if not os.path.exists(path):
            continue
        with open(path, newline="") as f:
            by = defaultdict(list)
            for r in csv.DictReader(f):
                by[r["benchmark"]].append(r)
        name = "/".join(os.path.normpath(d).split(os.sep)[-2:])
        for bench in sorted(by):
            rs = by[bench]
            kept = [r for r in rs if r["kept"] == "yes"]
            cpus = [float(r["others_cpus"]) for r in kept]
            busy = sum(1 for r in kept if r["quiet_below"] != "off" and float(r["others_cpu_s"]) >= 0.05
                       and float(r["others_cpus"]) >= float(r["quiet_below"]))
            waited = [int(r["gate_wait_s"]) for r in rs]
            steal = [float(r["steal_s"]) / float(r["wall_s"]) for r in kept if float(r["wall_s"]) > 0.05]
            lines.append([
                name, bench, ", ".join(sorted({r["quiet_below"] for r in rs})), str(len(kept)), str(len(rs) - len(kept)),
                str(sum(1 for r in rs if r["gate"] == "timeout")), str(busy),
                f"{statistics.median(cpus):.3f}" if cpus else "", f"{max(cpus):.3f}" if cpus else "",
                f"{statistics.median(steal):.3f}" if steal else "",
                f"{sum(waited)} / {max(waited)}" if waited else "",
            ])
    if lines:
        print_table("Machine load during the runs (load.csv)",
                    ["results", "benchmark", "quiet below (CPUs)", "processes kept", "attempts measured again",
                     "gate timeouts", "kept although busy", "others' CPUs while kept: median", "max",
                     "steal (CPUs): median", "gate wait (s): total / longest"], lines, md)


def main():
    args = sys.argv[1:]
    md = "--markdown" in args
    dirs = [a for a in args if a != "--markdown"]
    if not dirs:
        print(__doc__)
        sys.exit(2)
    rows = load(dirs)
    cells = cells_of(rows)
    t_idle_memory(cells, md)
    t_memory_rule(cells, md)
    t_spawn_join(cells, md)
    t_max_tasks(cells, md)
    t_echo(cells, md)
    t_idle_connections(cells, md)
    t_deep_recursion(cells, md)
    t_starvation(cells, md)
    t_probe_overhead(cells, md)
    t_cancel_uring(cells, md)
    t_load(dirs, md)
    crashes = [r for r in rows if r["metric"] == "process_exit"]
    print(f"\nprocess_exit rows (crashes): {len(crashes)}")
    for r in crashes:
        print("  ", r["machine_id"], r["benchmark"], r["strategy"], r["params"], r["note"])
    runs = {r["run_id"] for r in rows if r["implementation"] != "machine"}
    print(f"rows: {len(rows)}; processes (run ids): {len(runs)}; non-finite values: "
          f"{sum(1 for r in rows if r['value'] and not math.isfinite(float(r['value'])))}")


if __name__ == "__main__":
    main()
