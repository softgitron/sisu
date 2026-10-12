#!/usr/bin/env python3
"""Prints the key bet-3 figures from a results directory (results/<machine>/<date>).

Usage: python3 summarize.py RESULTS_DIR [RESULTS_DIR...]

Reads the CSVs written by run_all.sh (taskbench and the Go and Java controls,
one schema) and prints one table per bet-3 row, with ratios to Go where the
threshold is stated against Go. Standard library only.
"""
import csv
import glob
import os
import sys
from collections import defaultdict


def load(dirs):
    rows = []
    for d in dirs:
        for path in sorted(glob.glob(os.path.join(d, "*.csv"))):
            with open(path, newline="") as f:
                for r in csv.DictReader(f):
                    r["_dir"] = d
                    rows.append(r)
    return rows


CONTROLS = ("go", "java", "tokio", "os-threads")

# Parameters that change what a run measures, with the value that is left out
# of the label because it is the default.
VARIANT_DEFAULTS = {
    "batch": "1",
    "yield_every": "10000",
    "source": "pipe",
    "break_contract": "false",
    "scope_per_round": "false",
    "stack_cache": None,
    "probe_reserve": None,
    "guard_bytes": None,
    "chunk_bytes": None,
    "canary": None,
}


def config(r):
    """(label, variant): the label separates configurations of one implementation
    and strategy; the variant is the part a control can share (for example
    yield_every=0), used to pick the Go row a configuration is compared with."""
    if r["implementation"] in CONTROLS:
        s = r["implementation"]
    else:
        s = r["strategy"]
        if s.startswith(("A", "B", "D")) and r["reservation_bytes"]:
            s += "/" + human(int(r["reservation_bytes"]))
        if s == "C" and r["stack_bytes"]:
            s += "/" + human(int(r["stack_bytes"]))
    p = dict(kv.split("=", 1) for kv in r["params"].split(";") if "=" in kv)
    variant = ""
    for key, default in VARIANT_DEFAULTS.items():
        if key in p and p[key] != default:
            variant += f" {key}={p[key]}"
    if r["benchmark"] in ("idle-memory", "max-tasks") and r["tasks"]:
        variant += f" N={r['tasks']}"
    return s + variant, variant


def human(b):
    for unit, size in (("G", 1 << 30), ("M", 1 << 20), ("K", 1 << 10)):
        if b >= size and b % size == 0:
            return f"{b // size}{unit}"
    return str(b)


def table(rows, bench, metrics, go_metric=None, fmt="{:.0f}"):
    sel = [r for r in rows if r["benchmark"] == bench]
    if not sel:
        return
    by = defaultdict(dict)
    status = {}
    variant = {}
    for r in sel:
        label, var = config(r)
        key = (r["machine_id"], label)
        variant[key] = var
        if r["value"]:
            by[key][r["metric"]] = float(r["value"])
        if r["status"] != "ok":
            status[key] = r["status"] + ": " + r["note"][:60]

    def go_reference(key):
        """The Go run on the same machine with the same variant, else the plainest Go run."""
        machine = key[0]
        same = by.get((machine, "go" + variant[key]))
        if same is not None:
            return same
        gos = sorted((len(k[1]), k[1]) for k in by if k[0] == machine and k[1].split(" ")[0] == "go")
        return by[(machine, gos[0][1])] if gos else {}

    print(f"\n## {bench}")
    head = ["machine", "configuration"] + metrics + ([f"{go_metric}/go"] if go_metric else []) + ["status"]
    print(" | ".join(head))
    for key in sorted(by):
        vals = by[key]
        cells = [key[0], key[1]] + [fmt.format(vals[m]) if m in vals else "" for m in metrics]
        if go_metric:
            g = go_reference(key).get(go_metric)
            cells.append(f"{vals[go_metric] / g:.2f}" if g and go_metric in vals else "")
        cells.append(status.get(key, "ok"))
        print(" | ".join(cells))


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    rows = load(sys.argv[1:])
    table(rows, "spawn-join", ["spawn_join_p50", "spawn_join_p99"], "spawn_join_p50")
    table(rows, "idle-memory", ["total_per_task_shallow", "total_per_task_excursion", "total_per_task_excursion_after_gc",
                                "pte_per_task_shallow"], "total_per_task_shallow")
    table(rows, "max-tasks", ["max_tasks", "total_per_task_at_stop", "vmas_at_stop"])
    table(rows, "echo", ["echo_rtt_p50", "echo_rtt_p99", "echo_rtt_p999", "echo_throughput"], "echo_rtt_p99")
    table(rows, "idle-connections", ["total_per_connection"], "total_per_connection")
    table(rows, "deep-recursion", ["parse_depth_reached", "naive_depth_reached", "naive_leaked_drop_guards",
                                   "parse_probe_reserve_used", "runtime_healthy_after"])
    table(rows, "starvation", ["probe_baseline_late_p99", "probe_loaded_late_p50", "probe_loaded_late_p99",
                               "probe_loaded_late_max"], "probe_loaded_late_p99")
    table(rows, "probe-overhead", ["fib_split_probe_overhead", "calls_split_probe_overhead", "tree_split_probe_overhead",
                                   "calls_arg_probe_overhead", "calls_tls_probe_overhead"], fmt="{:.1f}")
    table(rows, "cancel-uring", ["reads", "cancel_ok", "cancel_enoent", "cancel_ealready", "completed_after_failed_cancel",
                                 "buffer_writes_after_return"])
    crashes = [r for r in rows if r["metric"] == "process_exit"]
    print(f"\nprocess_exit rows (crashes): {len(crashes)}")
    for r in crashes:
        print("  ", r["benchmark"], r["strategy"], r["params"], r["note"])


if __name__ == "__main__":
    main()
