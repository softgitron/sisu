"""The Phase 1 report's tables, computed from the committed CSVs through
summarize.py's functions, so that no number in the report is typed by hand.
render.py calls one function per table; run alone, this prints every table.

Usage: python3 -I -B report/tables.py TASKBENCH_DIR

M1_DIR and M2_DIR name the two machines' results directories. When machine 2's
full-scale run is committed (results/github-runner/full-<tag>/), M2_DIR moves to
it, and the template's prose about machine 2 changes with it.
"""
import csv
import importlib.util
import os
import re
import sys

TB = sys.argv[1]
spec = importlib.util.spec_from_file_location("summarize", f"{TB}/summarize.py")
S = importlib.util.module_from_spec(spec)
spec.loader.exec_module(S)

M1_DIR = f"{TB}/results/vm/2026-10-13-phase1"
M2_DIR = f"{TB}/results/github-runner/2026-10-13-smoke-b5349dd405e8"
cells = S.cells_of(S.load([M1_DIR, M2_DIR]))
M1, M2 = "vm", "github-runner"


def c(m, bench, label):
    return cells.get((m, bench, label))


def cell_text(cell, metric, unit="", digits=None):
    return S.fmt_cell(cell, metric, unit, digits).replace(" (n=5)", "").replace(" (n=1)", "")


def compact(text):
    """Short forms of the harness's status notes for the report's tables."""
    m = re.search(r"failed: created (\d+) of (\d+): spawn failed: VMA limit reached", text)
    if m:
        return f"fails: VMA limit at {int(m.group(1)):,} tasks"
    if "skipped: not run (--skip-excursion)" in text:
        return "not run (policy 7)"
    m = re.search(r"failed: RLIMIT_NOFILE[= ].*?(\d+) .*?for (\d+) connections", text)
    if m:
        return f"not run: RLIMIT_NOFILE {m.group(1)} < {2 * int(m.group(2)) + 64}"
    if "RLIMIT_NOFILE" in text and "too low" in text:
        return "not run: RLIMIT_NOFILE too low"
    return text


def short(cell, metric, unit="", digits=None):
    """median [min-max] without the run count, or the status in short form."""
    if cell is None:
        return "not run"
    return compact(cell_text(cell, metric, unit, digits))


def ratio_text(cell, metric, ref, ref_metric=None):
    r = S.ratio(cell, metric, ref, ref_metric)
    if r is None:
        return ""
    lohi = r[2]
    if lohi and f"{lohi[0]:.2f}" != f"{lohi[1]:.2f}":
        return f"{r[0]:.2f} [{lohi[0]:.2f}-{lohi[1]:.2f}]"
    return f"{r[0]:.2f}"


def table(head, rows):
    print("| " + " | ".join(head) + " |")
    print("|" + "---|" * len(head))
    for r in rows:
        print("| " + " | ".join(x.replace("|", "/") for x in r) + " |")
    print()


def mem():
    b = "idle-memory"
    print("### T1 memory\n")
    configs = [("A/128K", "A, 128 KiB"), ("A/1M", "A, 1 MiB"), ("A/8M", "A, 8 MiB"),
               ("B/128K", "B, 128 KiB"), ("B/1M", "B, 1 MiB"), ("B/8M", "B, 8 MiB"),
               ("C/36K", "C, 4 KiB (+32 KiB: 36 KiB)"), ("C/40K", "C, 8 KiB (+32 KiB: 40 KiB)"),
               ("C/48K", "C, 16 KiB (+32 KiB: 48 KiB)"),
               ("D(A)/128K", "D(A), 128 KiB"), ("D(A)/1M", "D(A), 1 MiB"), ("D(A)/8M", "D(A), 8 MiB"),
               ("D(B)/128K", "D(B), 128 KiB"), ("D(B)/1M", "D(B), 1 MiB"), ("D(B)/8M", "D(B), 8 MiB"),
               ("E", "E (stackless)"), ("tokio", "tokio"), ("go", "Go"), ("java", "Java virtual threads")]
    for n in (100000, 1000000):
        rows = []
        go1 = c(M1, b, f"go N={n}")
        for lab, name in configs:
            x = c(M1, b, f"{lab} N={n}")
            y = c(M2, b, f"{lab} N=10000")
            exc_go = "total_per_task_excursion_after_gc"
            if lab == "go":
                exc1 = short(x, "total_per_task_excursion", "KiB") + "; after GC " + short(x, exc_go, "KiB")
                exc2 = short(y, "total_per_task_excursion", "KiB") + "; after GC " + short(y, exc_go, "KiB")
            else:
                exc1 = short(x, "total_per_task_excursion", "KiB")
                exc2 = short(y, "total_per_task_excursion", "KiB")
            if lab == "java":
                exc1 += "; heap " + short(x, "java_heap_used_per_task_excursion", "KiB")
            go_ref = go1
            r_sh = ratio_text(x, "total_per_task_shallow", go_ref)
            if n == 100000 or (go1 and not go1.bad(exc_go)):
                r_ex = ratio_text(x, "total_per_task_excursion", go_ref, exc_go)
            else:
                a = ratio_text(x, "total_per_task_excursion", c(M1, b, "go N=100000"), exc_go)
                bb = ratio_text(x, "total_per_task_excursion", go1, "total_per_task_shallow")
                r_ex = (f"{a} (vs Go after GC at 100k); {bb} (vs Go shallow at 1M)") if a else ""
            y_go = c(M2, b, "go N=10000")
            r2 = ""
            if y is not None and y_go is not None:
                r2 = (ratio_text(y, "total_per_task_shallow", y_go) + " / "
                      + ratio_text(y, "total_per_task_excursion", y_go, exc_go))
            row = [name, short(x, "total_per_task_shallow", "KiB"), exc1,
                   short(x, "pte_per_task_shallow", "KiB"), r_sh, r_ex]
            if n == 100000:
                row += [short(y, "total_per_task_shallow", "KiB"), exc2, r2]
            rows.append(row)
        head = ["Configuration", "M1 shallow", "M1 after 32 KiB excursion", "M1 page tables (shallow)",
                "M1 shallow / Go", "M1 excursion / Go after GC"]
        if n == 100000:
            head += ["M2 shallow (10k tasks, smoke)", "M2 after the excursion (10k, smoke)",
                     "M2 ratios to Go, shallow / excursion (10k)"]
        print(f"*{n:,} tasks (KiB per task)*\n")
        table(head, rows)



def spawn_join():
    b = "spawn-join"
    print("### T2 spawn-join (microseconds)\n")
    labels = [("A/1M", "A (no cache)"), ("A/1M stack_cache=1024", "A (`--stack-cache 1024`)"), ("B/1M", "**B**"),
              ("C/8K", "C (8 KiB)"), ("D(A)/1M", "D(A)"), ("D(A)/1M stack_cache=1024", "D(A) (`--stack-cache 1024`)"),
              ("D(B)/1M", "D(B)"), ("E", "E"), ("tokio", "tokio"), ("os-threads", "OS threads"),
              ("B/1M batch=100", "B, batch 100 (per task)"), ("B/1M scope_per_round=true", "B, scope per round"),
              ("go", "Go"), ("java", "Java virtual threads")]
    rows = []
    for lab, name in labels:
        x, y = c(M1, b, lab), c(M2, b, lab)
        g1, g2 = c(M1, b, "go"), c(M2, b, "go")
        rows.append([name, short(x, "spawn_join_p50", "us"), short(x, "spawn_join_p99", "us"),
                     short(y, "spawn_join_p50", "us"), short(y, "spawn_join_p99", "us"),
                     ratio_text(x, "spawn_join_p50", g1), ratio_text(y, "spawn_join_p50", g2)])
    table(["Configuration", "median M1", "p99 M1", "median M2 (smoke)", "p99 M2 (smoke)", "median / Go M1",
           "median / Go M2"], rows)


def max_tasks():
    b = "max-tasks"
    print("### T3 max-tasks\n")
    labels = [("A/1M", "A (guard page per task)"), ("B/1M", "B"), ("C/8K", "C (8 KiB)"), ("D(B)/1M", "D(B)"),
              ("D(A)/1M", "D(A)"), ("E", "E"), ("tokio", "tokio"), ("os-threads", "OS threads"), ("go", "Go"),
              ("java", "Java virtual threads")]
    rows = []
    for lab, name in labels:
        n1 = "N=30000" if lab == "os-threads" else "N=1000000"
        n2 = "N=5000" if lab == "os-threads" else "N=100000"
        x, y = c(M1, b, f"{lab} {n1}"), c(M2, b, f"{lab} {n2}")
        lim1 = ", ".join(sorted({n for st, n in x.status.get("limit", [])})) if x else ""
        lim2 = ", ".join(sorted({n for st, n in y.status.get("limit", [])})) if y else ""
        rows.append([name, short(x, "max_tasks", "", 0), lim1, short(x, "total_per_task_at_stop", "KiB"),
                     short(x, "parked_after_hold", "", 0), short(x, "hold_secs", "", 1),
                     short(y, "max_tasks", "", 0) + (f" ({n2[2:]} cap)" if y else ""), lim2])
    table(["Configuration", "tasks created M1 (cap 1M)", "limit M1", "RSS+PTE per task at stop M1 (KiB)",
           "still parked after 60 s M1", "hold (s)", "created M2 (smoke)", "limit M2"], rows)


def echo():
    print("### T4 echo (ms) and idle connections (KiB)\n")
    labels = [("A/1M", "A"), ("B/1M", "B"), ("C/8K", "C (8 KiB)"), ("D(A)/1M", "D(A)"), ("D(B)/1M", "D(B)"),
              ("E", "E"), ("tokio", "tokio"), ("go", "Go"), ("java", "Java virtual threads")]
    for conns in (10000, 1000):
        rows = []
        for lab, name in labels:
            x = c(M1, "echo", f"{lab} connections={conns}")
            y = c(M2, "echo", f"{lab} connections={conns}")
            g1 = c(M1, "echo", f"go connections={conns}")
            g2 = c(M2, "echo", f"go connections={conns}")
            if x is not None and x.bad("echo"):
                rows.append([name, short(x, "echo"), "", "", "", "", "", ""])
                continue
            rows.append([name, short(x, "echo_rtt_p50", "ms"), short(x, "echo_rtt_p99", "ms"),
                         short(x, "echo_rtt_p999", "ms"), short(x, "echo_throughput", "", 0),
                         ratio_text(x, "echo_rtt_p99", g1),
                         (short(y, "echo_rtt_p99", "ms") + " / " + short(y, "echo_throughput", "", 0)) if y else "",
                         ratio_text(y, "echo_rtt_p99", g2)])
        print(f"{conns} connections\n")
        table(["Configuration", "p50 M1", "p99 M1", "p99.9 M1", "req/s M1", "p99 / Go M1",
               "p99 / req/s M2 (smoke, 2 workers)", "p99 / Go M2"], rows)
    rows = []
    for lab, name in labels:
        x = next((v for k, v in cells.items() if k[0] == M1 and k[1] == "idle-connections"
                  and k[2].split(" ")[0] == lab), None)
        y = next((v for k, v in cells.items() if k[0] == M2 and k[1] == "idle-connections"
                  and k[2].split(" ")[0] == lab), None)
        g1 = next((v for k, v in cells.items() if k[0] == M1 and k[1] == "idle-connections" and k[2].startswith("go ")), None)
        g2 = next((v for k, v in cells.items() if k[0] == M2 and k[1] == "idle-connections" and k[2].startswith("go ")), None)
        rows.append([name, short(x, "total_per_connection", "KiB"), short(x, "rss_per_connection", "KiB"),
                     ratio_text(x, "total_per_connection", g1), short(y, "total_per_connection", "KiB"),
                     ratio_text(y, "total_per_connection", g2)])
    print("idle connections\n")
    table(["Configuration", "RSS+PTE per connection M1", "RSS only M1", "/ Go M1", "RSS+PTE M2 (smoke, 5000)",
           "/ Go M2"], rows)


def deep():
    b = "deep-recursion"
    print("### T5 deep recursion\n")
    labels = ["A/128K", "A/1M", "A/8M", "B/128K", "B/1M", "B/8M", "C/2K", "C/8K", "C/16K", "D(A)/128K", "D(A)/1M",
              "D(A)/8M", "D(B)/128K", "D(B)/1M", "D(B)/8M", "E", "tokio"]
    rows = []
    for lab in labels:
        x, y = c(M1, b, lab), c(M2, b, lab)

        def oc(cell, case):
            if cell is None or not cell.status.get(f"{case}_outcome"):
                return ""
            notes = sorted({n.split(";")[0] for st, n in cell.status[f"{case}_outcome"]})
            bad = sum(1 for st, n in cell.status[f"{case}_outcome"] if st != "ok")
            depth = S.fmt_num(cell.med(f"{case}_depth_reached"), "", 0)
            return "/".join(notes) + f" at {depth}" + (f" ({bad} unclean)" if bad else "")

        def mx(cell, metric):
            v = cell.values.get(metric) if cell else None
            return f"{max(v):.0f}" if v else ""

        leaked = []
        for case, metric in (("naive", "naive_leaked_drop_guards"), ("framed", "framed_leaked_drop_guards")):
            if x and x.values.get(metric):
                leaked.append(f"{case} {S.fmt_num(x.med(metric), '', 0)}")
        rows.append([lab, oc(x, "parse"), oc(x, "naive"), oc(x, "framed"), oc(x, "naive-async"),
                     ", ".join(leaked), " / ".join(filter(None, [mx(x, "parse_probe_reserve_used"),
                                                                  mx(x, "naive_probe_reserve_used"),
                                                                  mx(x, "framed_probe_reserve_used")])),
                     short(x, "runtime_healthy_after", "", 0),
                     " / ".join(filter(None, [mx(y, "parse_probe_reserve_used"), mx(y, "naive_probe_reserve_used")])),
                     oc(y, "parse") + "; " + oc(y, "naive") if y else ""])
    table(["Configuration", "10k parse M1", "1M naive M1", "framed M1", "naive async M1",
           "drop guards skipped M1", "reserve bytes used M1 (parse / naive / framed, max of 5)", "healthy after M1",
           "reserve used M2 (smoke, 0.1.0)", "M2 outcomes (smoke)"], rows)


def starvation():
    b = "starvation"
    print("### T6 starvation (microseconds)\n")
    rows = []
    go1 = c(M1, b, "go yield_every=0")
    go2 = c(M2, b, "go yield_every=0")
    order = sorted(k[2] for k in cells if k[0] == M1 and k[1] == b)
    for lab in order:
        x = c(M1, b, lab)
        y = c(M2, b, lab)
        rows.append([lab, short(x, "slice_between_yields", "us"), short(x, "probe_loaded_late_p50", "us"),
                     short(x, "probe_loaded_late_p99", "us"), short(x, "probe_loaded_late_max", "us"),
                     short(x, "probe_baseline_late_p99", "us"), short(x, "probe_loaded_late_samples", "", 0),
                     ratio_text(x, "probe_loaded_late_p99", go1),
                     (short(y, "probe_loaded_late_p99", "us") + "; / Go " + ratio_text(y, "probe_loaded_late_p99", go2))
                     if y else ""])
    table(["Configuration (K = yield_every; default 10000)", "slice between yields", "p50 loaded", "p99 loaded",
           "max loaded", "p99 alone", "wake-ups", "p99 / Go (default preemption)", "M2 p99 (smoke)"], rows)


def probe():
    b = "probe-overhead"
    print("### T7 probe overhead (percent)\n")
    rows = []
    for lab in sorted(k[2] for k in cells if k[0] == M1 and k[1] == b):
        x = c(M1, b, lab)
        y = c(M2, b, lab)
        if x.bad("probe_overhead"):
            continue
        rows.append([lab] + [short(x, f"{w}_split_probe_overhead", "", 1) for w in ("fib", "calls", "tree", "loop")]
                    + [short(x, f"{w}_arg_probe_overhead", "", 1) for w in ("fib", "calls", "tree")]
                    + [short(x, f"{w}_tls_probe_overhead", "", 1) for w in ("fib", "calls", "tree")]
                    + [" / ".join(short(y, f"{w}_split_probe_overhead", "", 1) for w in ("fib", "calls", "tree"))
                       if y else ""])
    table(["Configuration", "fib split", "calls split", "tree split", "loop split (back-edge)", "fib arg", "calls arg",
           "tree arg", "fib tls", "calls tls", "tree tls", "M2 split fib / calls / tree (smoke)"], rows)


def uring():
    b = "cancel-uring"
    print("### T8 io_uring\n")
    rows = []
    for lab in sorted(k[2] for k in cells if k[0] == M1 and k[1] == b):
        x = c(M1, b, lab)
        y = c(M2, b, lab)
        w = x.values.get("buffer_writes_after_return", [])
        wy = y.values.get("buffer_writes_after_return", []) if y else []
        rows.append([lab, short(x, "reads", "", 0), short(x, "cancels_submitted", "", 0),
                     short(x, "ended_ecanceled", "", 0), short(x, "cancel_ok", "", 0), short(x, "cancel_enoent", "", 0),
                     short(x, "cancel_ealready", "", 0), short(x, "completed_after_failed_cancel", "", 0),
                     short(x, "completed_with_late_data", "", 0),
                     f"{sum(w):.0f} (max {max(w):.0f} per run, {len(w)} runs)" if w else "",
                     short(x, "cancel_to_return_p50", "ms") + " / " + short(x, "cancel_to_return_p99", "ms"),
                     (f"{short(y, 'reads', '', 0)} reads, {sum(wy):.0f} bytes") if y else ""])
    table(["Configuration", "reads per run", "cancels submitted", "ended -ECANCELED", "cancel ok", "-ENOENT", "-EALREADY",
           "completed after failed cancel", "completed with the late data", "buffer bytes written after return (5 runs)",
           "cancel to return p50 / p99 (ms)", "M2 (smoke)"], rows)


def timing():
    print("### Timing on M1 (timing.csv)\n")
    groups = []

    def read(path, label):
        per = {}
        with open(path) as f:
            for r in csv.DictReader(f):
                per.setdefault(r["benchmark"], []).append((r["start_utc"], int(r["seconds"])))
        for b, v in per.items():
            groups.append((b, label, v))

    gated = set()
    if os.path.exists(f"{M1_DIR}/load.csv"):
        with open(f"{M1_DIR}/load.csv") as f:
            gated = {r["benchmark"] for r in csv.DictReader(f) if r["quiet_below"] != "off"}
    read(f"{M1_DIR}/timing.csv", "kept")
    groups[:] = [(b, "kept, gated" if b in gated else label, v) for b, label, v in groups]
    for sub, label in (("superseded-cancel-uring-before-check-fix", "superseded (A1's check, section 10.3)"),
                       ("superseded-before-quiet-gate", "superseded (no quiet gate, section 10.2)"),
                       ("aborted-gated-rerun", "aborted (load accounting, section 10.2)")):
        path = f"{M1_DIR}/{sub}/timing.csv"
        if os.path.exists(path):
            read(path, label)
    groups.sort(key=lambda g: min(st for st, _ in g[2]))
    rows = []
    for b, label, v in groups:
        secs = [x for _, x in v]
        rows.append([b, label, min(st for st, _ in v)[:16].replace("T", " "), str(len(v)),
                     f"{sum(secs) / 60:.1f}", f"{min(secs)}-{max(secs)}"])
    kept = sum(sum(x for _, x in v) for _, label, v in groups if label.startswith("kept"))
    every = sum(sum(x for _, x in v) for _, _, v in groups)
    rows.append(["total of the kept groups", "", "", "", f"{kept / 60:.1f}", ""])
    rows.append(["total, superseded and aborted runs included", "", "", "", f"{every / 60:.1f}", ""])
    table(["Benchmark (with its controls)", "runs", "first start (UTC)", "repetitions", "minutes",
           "seconds per repetition"], rows)


def load_summary():
    print("### Machine load (load.csv)\n")
    S.t_load([M1_DIR], True)


def rule():
    print("### Section 2.4 memory rule (M1)\n")
    S.t_memory_rule(cells, True)


def main():
    for f in (mem, spawn_join, max_tasks, echo, deep, starvation, probe, uring, rule, timing, load_summary):
        f()
    crashes = sum(1 for k, v in cells.items() for st in v.status.get("process_exit", []))
    print(f"process_exit rows over both directories: {crashes}")


if __name__ == "__main__":
    main()
