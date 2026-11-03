"""The Phase 1 report's tables, computed from the committed CSVs through
summarize.py's functions, so that no number in the report is typed by hand.
render.py calls one function per table; run alone, this prints every table.

Usage: python3 -I -B report/tables.py TASKBENCH_DIR

M1_DIR and M2_DIR name the two machines' full-scale results directories (five
runs per cell on each); every table puts the two machines side by side, cell by
cell. M3_DIR is machine 3, the 16 GiB runner, which ran `idle-memory` alone
(five runs per cell) to measure the million-task cells machine 2 could not
hold: the idle-memory tables show the three machines, and section 2.4's rule
is applied to M1 with each runner. SMOKE_DIR is machine 2's earlier CI smoke
run (harness 0.1.0, one run per cell). It is loaded on its own, so that its cells never mix with M2_DIR's
(both have the machine id github-runner), and the report cites it only for what
the full run cannot show: the runner image's own settings (before the workflow
changes them), Go's memory under transparent huge pages "always", the first
unwind before the unwinder fix, and C under the old 8 KiB reserve.
"""
import csv
import importlib.util
import math
import os
import re
import sys

TB = sys.argv[1]
spec = importlib.util.spec_from_file_location("summarize", f"{TB}/summarize.py")
S = importlib.util.module_from_spec(spec)
spec.loader.exec_module(S)

M1_DIR = f"{TB}/results/vm/2026-10-13-phase1"
M2_DIR = f"{TB}/results/github-runner/full-38518035636"
M3_DIR = f"{TB}/results/github-runner/cell-41548553538"
SMOKE_DIR = f"{TB}/results/github-runner/2026-10-13-smoke-b5349dd405e8"
cells = S.cells_of(S.load([M1_DIR, M2_DIR, M3_DIR]))
SMOKE = S.cells_of(S.load([SMOKE_DIR]))
M1, M2, M3 = "vm", "github-runner", "github-runner-16g"
# Every benchmark ran on M1 and M2; M3 ran idle-memory only.
MACHINES = (M1, M2)
MEM_MACHINES = (M1, M2, M3)
TAGS = {M1: "M1", M2: "M2", M3: "M3"}
DIRS = {M1: M1_DIR, M2: M2_DIR, M3: M3_DIR}


def c(m, bench, label):
    return cells.get((m, bench, label))


def cell_text(cell, metric, unit="", digits=None):
    # Five runs per cell is the protocol; any other count stays visible.
    return S.fmt_cell(cell, metric, unit, digits).replace(" (n=5)", "")


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


def guard_stop(cell):
    """'memory guard at a-b of N tasks' (the range over the runs) for a cell that
    the memory guard stopped, else None."""
    bad = cell.bad("tasks_created") if cell is not None else None
    if not bad or "memory headroom below" not in bad[1]:
        return None
    return S.brief(bad[1], cell.values.get("tasks_created"))


def short(cell, metric, unit="", digits=None):
    """median [min-max] without the run count, or the status in short form. A
    cell the memory guard stopped is a failed row: it shows the stop, not the
    figures over the tasks it created."""
    if cell is None:
        return "not run"
    bad = cell.bad(metric)
    if not (bad and bad[0] == "skipped"):
        gs = guard_stop(cell)
        if gs:
            return f"fails: {gs}"
    text = compact(cell_text(cell, metric, unit, digits))
    created = cell.values.get("tasks_created")
    if text.startswith("fails: VMA limit at ") and created:
        # The tasks created over the runs, not the first run's count.
        lo, hi = min(created), max(created)
        text = f"fails: VMA limit at {lo:,.0f} tasks" if lo == hi else f"fails: VMA limit at {lo:,.0f}-{hi:,.0f} tasks"
    return text


def ratio_text(cell, metric, ref, ref_metric=None, small=False):
    """median ratio [conservative range]; with `small`, ratios below 0.1 keep two
    significant digits instead of rounding to 0.00."""
    r = S.ratio(cell, metric, ref, ref_metric)
    if r is None:
        return ""

    def f(v):
        if small and 0 < abs(v) < 0.1:
            # two significant digits, without an exponent
            return f"{v:.{1 - math.floor(math.log10(abs(v)))}f}"
        return f"{v:.2f}"

    lohi = r[2]
    if lohi and f(lohi[0]) != f(lohi[1]):
        return f"{f(r[0])} [{f(lohi[0])}-{f(lohi[1])}]"
    return f(r[0])


def both(f):
    """'M1 / M2' from f(machine), or one text when both are the same."""
    a, b = f(M1), f(M2)
    return a if a == b else f"{a} / {b}"


def table(head, rows, note=None):
    print("| " + " | ".join(head) + " |")
    print("|" + "---|" * len(head))
    for r in rows:
        print("| " + " | ".join(x.replace("|", "/") for x in r) + " |")
    print()
    if note:
        print(note + "\n")


MEM_CONFIGS = [("A/128K", "A, 128 KiB"), ("A/1M", "A, 1 MiB"), ("A/8M", "A, 8 MiB"),
               ("B/128K", "B, 128 KiB"), ("B/1M", "B, 1 MiB"), ("B/8M", "B, 8 MiB"),
               ("C/36K", "C, 4 KiB (+32 KiB: 36 KiB)"), ("C/40K", "C, 8 KiB (+32 KiB: 40 KiB)"),
               ("C/48K", "C, 16 KiB (+32 KiB: 48 KiB)"),
               ("D(A)/128K", "D(A), 128 KiB"), ("D(A)/1M", "D(A), 1 MiB"), ("D(A)/8M", "D(A), 8 MiB"),
               ("D(B)/128K", "D(B), 128 KiB"), ("D(B)/1M", "D(B), 1 MiB"), ("D(B)/8M", "D(B), 8 MiB"),
               ("E", "E (stackless)"), ("tokio", "tokio"), ("go", "Go"), ("java", "Java virtual threads")]
EXC_GO = "total_per_task_excursion_after_gc"


def exc_ratio(m, x, n):
    """The after-excursion ratio of record: against Go after its GC at the same N;
    at 1M, where no machine holds Go's cell, against Go after GC at 100k and Go
    shallow at 1M (section 2.4's substitutes; summarize.memory_rule)."""
    b = "idle-memory"
    go = c(m, b, f"go N={n}")
    if n == 100000 or (go and not go.bad(EXC_GO)):
        return ratio_text(x, "total_per_task_excursion", go, EXC_GO)
    a = ratio_text(x, "total_per_task_excursion", c(m, b, "go N=100000"), EXC_GO)
    bb = ratio_text(x, "total_per_task_excursion", go, "total_per_task_shallow")
    return f"{a} (vs Go after GC at 100k); {bb} (vs Go shallow at 1M)" if a else ""


def mem():
    b = "idle-memory"
    print("### T1 memory\n")
    for n in (100000, 1000000):
        rows = []
        for lab, name in MEM_CONFIGS:
            xs = [c(m, b, f"{lab} N={n}") for m in MEM_MACHINES]
            row = [name] + [short(x, "total_per_task_shallow", "KiB") for x in xs]
            for x in xs:
                t = short(x, "total_per_task_excursion", "KiB")
                if lab == "go":
                    g = short(x, EXC_GO, "KiB")
                    t = t if g == t else t + "; after GC " + g
                if lab == "java":
                    t += "; heap " + short(x, "java_heap_used_per_task_excursion", "KiB")
                row.append(t)
            row += [ratio_text(x, "total_per_task_shallow", c(m, b, f"go N={n}")) for m, x in zip(MEM_MACHINES, xs)]
            row += [exc_ratio(m, x, n) for m, x in zip(MEM_MACHINES, xs)]
            rows.append(row)
        print(f"*{n:,} tasks (KiB per task)*\n")
        table(["Configuration"] + [f"shallow {TAGS[m]}" for m in MEM_MACHINES]
              + [f"after 32 KiB excursion {TAGS[m]}" for m in MEM_MACHINES]
              + [f"shallow / Go {TAGS[m]}" for m in MEM_MACHINES]
              + [f"excursion / Go after GC {TAGS[m]}" for m in MEM_MACHINES], rows)


def rule_rows(m):
    return {lab: (w, v, text, conds) for lab, conds, w, v, text in S.memory_rule(cells, m) or []}


def rule_pairs():
    """Section 2.4 per configuration: the verdict on each machine, then on M1
    with each runner (the rule names two machines; it passes or fails a
    configuration only if both of the pair agree)."""
    print("### Section 2.4 memory rule, each machine and M1 with each runner\n")
    r = {m: rule_rows(m) for m in MEM_MACHINES}
    none = (None, "not run", "not run", None)
    rows = []
    for lab in sorted(set().union(*r.values())):
        v = {m: r[m].get(lab, none) for m in MEM_MACHINES}
        rows.append([lab] + [v[m][2] for m in MEM_MACHINES]
                    + [S.rule_on_both(v[M1][1], v[m][1]) for m in (M2, M3)])
    table(["Configuration"] + [TAGS[m] for m in MEM_MACHINES]
          + ["section 2.4 on M1 and M2", "section 2.4 on M1 and M3"], rows)


def rule_detail(m):
    rows = [[lab] + conds + [text] for lab, (w, v, text, conds) in sorted(rule_rows(m).items())]
    table(["Configuration", "100k shallow", "100k after excursion", "1M shallow", "1M after excursion",
           "on this machine"], rows)


def rule_m1():
    print("### Section 2.4 memory rule, M1\n")
    rule_detail(M1)


def rule_m2():
    print("### Section 2.4 memory rule, M2\n")
    rule_detail(M2)


def rule_m3():
    print("### Section 2.4 memory rule, M3\n")
    rule_detail(M3)


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
        xs = [c(m, b, lab) for m in MACHINES]
        rows.append([name] + [short(x, "spawn_join_p50", "us") for x in xs]
                    + [short(x, "spawn_join_p99", "us") for x in xs]
                    + [ratio_text(x, "spawn_join_p50", c(m, b, "go")) for m, x in zip(MACHINES, xs)])
    table(["Configuration", "median M1", "median M2", "p99 M1", "p99 M2", "median / Go M1", "median / Go M2"], rows)


def max_tasks():
    b = "max-tasks"
    print("### T3 max-tasks\n")
    labels = [("A/1M", "A (guard page per task)"), ("B/1M", "B"), ("C/8K", "C (8 KiB)"), ("D(B)/1M", "D(B)"),
              ("D(A)/1M", "D(A)"), ("E", "E"), ("tokio", "tokio"), ("os-threads", "OS threads"), ("go", "Go"),
              ("java", "Java virtual threads")]
    rows = []
    for lab, name in labels:
        n = "N=30000" if lab == "os-threads" else "N=1000000"
        xs = [c(m, b, f"{lab} {n}") for m in MACHINES]
        lims = [", ".join(sorted({note for st, note in x.status.get("limit", [])})) if x else "" for x in xs]
        rows.append([name] + [short(x, "max_tasks", "", 0) for x in xs] + [lims[0] if lims[0] == lims[1]
                                                                          else " / ".join(lims)]
                    + [short(x, "total_per_task_at_stop", "KiB") for x in xs]
                    + [short(x, "parked_after_hold", "", 0) for x in xs])
    table(["Configuration", "tasks created M1 (cap 1M)", "tasks created M2 (cap 1M)", "limit (M1 / M2)",
           "RSS+PTE per task at stop M1 (KiB)", "M2 (KiB)", "still parked after 60 s M1", "M2"], rows)


ECHO_LABELS = [("A/1M", "A"), ("B/1M", "B"), ("C/8K", "C (8 KiB)"), ("D(A)/1M", "D(A)"), ("D(B)/1M", "D(B)"),
               ("E", "E"), ("tokio", "tokio"), ("go", "Go"), ("java", "Java virtual threads")]


def echo():
    print("### T4 echo (ms) and idle connections (KiB)\n")
    for conns in (10000, 1000):
        per = {m: [c(m, "echo", f"{lab} connections={conns}") for lab, _ in ECHO_LABELS] for m in MACHINES}
        # A machine on which no configuration ran at this count gets one column, its status.
        ran = {m: any(x is not None and not x.bad("echo") for x in per[m]) for m in MACHINES}
        head, rows = ["Configuration"], [[name] for _, name in ECHO_LABELS]
        for m, tag in ((M1, "M1"), (M2, "M2")):
            go = c(m, "echo", f"go connections={conns}")
            if not ran[m]:
                head.append(tag)
                for row, x in zip(rows, per[m]):
                    row.append(short(x, "echo"))
                continue
            head += [f"p50 {tag}", f"p99 {tag}", f"p99.9 {tag}", f"req/s {tag}", f"p99 / Go {tag}"]
            for row, x in zip(rows, per[m]):
                if x is not None and x.bad("echo"):
                    row += [short(x, "echo"), "", "", "", ""]
                    continue
                row += [short(x, "echo_rtt_p50", "ms"), short(x, "echo_rtt_p99", "ms"),
                        short(x, "echo_rtt_p999", "ms"), short(x, "echo_throughput", "", 0),
                        ratio_text(x, "echo_rtt_p99", go)]
        print(f"{conns:,} connections\n")
        table(head, rows)
    b = "idle-connections"

    def find(m, lab):
        return next((v for k, v in sorted(cells.items()) if k[0] == m and k[1] == b and k[2].split(" ")[0] == lab),
                    None)

    def conns_of(m):
        g = find(m, "go")
        return f"{int(g.variant.split('connections=')[1]):,}" if g else "?"

    rows = []
    for lab, name in ECHO_LABELS:
        row = [name]
        for m in MACHINES:
            x, g = find(m, lab), find(m, "go")
            if x is not None and x.bad("idle_connections"):
                row += [short(x, "idle_connections"), "", ""]
                continue
            row += [short(x, "total_per_connection", "KiB"), short(x, "rss_per_connection", "KiB"),
                    ratio_text(x, "total_per_connection", g)]
        rows.append(row)
    print("idle connections\n")
    table(["Configuration", f"RSS+PTE per connection M1 ({conns_of(M1)})", "RSS only M1", "/ Go M1",
           f"RSS+PTE per connection M2 ({conns_of(M2)})", "RSS only M2", "/ Go M2"], rows)


def outcome_at(cell, case):
    """'stack_exhausted at 2,856' (the outcomes over the runs and the median depth)."""
    if cell is None or not cell.status.get(f"{case}_outcome"):
        return ""
    notes = sorted({n.split(";")[0] for st, n in cell.status[f"{case}_outcome"]})
    bad = sum(1 for st, n in cell.status[f"{case}_outcome"] if st != "ok")
    depth = S.fmt_num(cell.med(f"{case}_depth_reached"), "", 0)
    return "/".join(notes) + f" at {depth}" + (f" ({bad} unclean)" if bad else "")


def outcome_pair(x, y, case):
    a, b = outcome_at(x, case), outcome_at(y, case)
    if not a and not b:
        return ""
    if a.split(" at ")[0] == b.split(" at ")[0] and "unclean" not in a + b:
        da, db = a.split(" at ")[1], b.split(" at ")[1]
        return a if da == db else f"{a.split(' at ')[0]} at {da} / {db}"
    return f"{a or 'not run'} / {b or 'not run'}"


DR_LABELS = ["A/128K", "A/1M", "A/8M", "B/128K", "B/1M", "B/8M", "C/2K", "C/8K", "C/16K", "D(A)/128K", "D(A)/1M",
             "D(A)/8M", "D(B)/128K", "D(B)/1M", "D(B)/8M", "E", "tokio"]
RESERVE = ("parse_probe_reserve_used", "naive_probe_reserve_used", "framed_probe_reserve_used")


def deep():
    b = "deep-recursion"
    print("### T5 deep recursion\n")
    rows = []
    for lab in DR_LABELS:
        x, y = c(M1, b, lab), c(M2, b, lab)

        def mx(cell, metric):
            v = cell.values.get(metric) if cell else None
            return f"{max(v):.0f}" if v else ""

        def leaked(cell):
            out = []
            for case in ("naive", "framed"):
                metric = f"{case}_leaked_drop_guards"
                if cell and cell.values.get(metric):
                    out.append(f"{case} {S.fmt_num(cell.med(metric), '', 0)}")
            return ", ".join(out)

        def reserve(cell):
            return " / ".join(filter(None, [mx(cell, m) for m in RESERVE]))

        la, lb = leaked(x), leaked(y)
        rows.append([lab, outcome_pair(x, y, "parse"), outcome_pair(x, y, "naive"), outcome_pair(x, y, "framed"),
                     outcome_pair(x, y, "naive-async"), la if la == lb else f"{la} / {lb}", reserve(x), reserve(y),
                     both(lambda m: short(c(m, b, lab), "runtime_healthy_after", "", 0))])
    table(["Configuration", "10k parse (M1 / M2)", "1M naive (M1 / M2)", "framed (M1 / M2)", "naive async (M1 / M2)",
           "drop guards skipped (M1 / M2)", "reserve bytes used M1 (parse / naive / framed, max of 5)",
           "reserve bytes used M2", "healthy after (M1 / M2)"], rows,
          "*Depths are medians over five runs; where one depth is given, both machines reached it.*")


def starvation():
    b = "starvation"
    print("### T6 starvation (microseconds)\n")
    rows = []
    gos = {m: c(m, b, "go yield_every=0") for m in MACHINES}
    for lab in sorted(k[2] for k in cells if k[0] == M1 and k[1] == b):
        xs = {m: c(m, b, lab) for m in MACHINES}
        rows.append([lab, both(lambda m: short(xs[m], "slice_between_yields", "us")),
                     short(xs[M1], "probe_loaded_late_p99", "us"), short(xs[M2], "probe_loaded_late_p99", "us"),
                     both(lambda m: short(xs[m], "probe_loaded_late_max", "us")),
                     both(lambda m: short(xs[m], "probe_baseline_late_p99", "us")),
                     both(lambda m: short(xs[m], "probe_loaded_late_samples", "", 0)),
                     ratio_text(xs[M1], "probe_loaded_late_p99", gos[M1], small=True),
                     ratio_text(xs[M2], "probe_loaded_late_p99", gos[M2], small=True)])
    table(["Configuration (K = yield_every; default 10000)", "slice between yields (M1 / M2)", "p99 loaded M1",
           "p99 loaded M2", "max loaded (M1 / M2)", "p99 alone (M1 / M2)", "wake-ups (M1 / M2)",
           "p99 / Go M1 (default preemption)", "p99 / Go M2"], rows)


def probe():
    b = "probe-overhead"
    print("### T7 probe overhead (percent)\n")
    labs = [lab for lab in sorted(k[2] for k in cells if k[0] == M1 and k[1] == b) if not c(M1, b, lab).bad("probe_overhead")]
    rows = []
    for lab in labs:
        xs = [c(m, b, lab) for m in MACHINES]
        row = [lab]
        for w in ("fib", "calls", "tree", "loop"):
            row += [short(x, f"{w}_split_probe_overhead", "", 1) for x in xs]
        rows.append(row)
    table(["Configuration", "fib M1", "fib M2", "calls M1", "calls M2", "tree M1", "tree M2", "loop (back-edge) M1",
           "loop (back-edge) M2"], rows)
    rows = []
    for lab in labs:
        rows.append([lab] + [both(lambda m: short(c(m, b, lab), f"{w}_{model}_probe_overhead", "", 1))
                             for model in ("arg", "tls") for w in ("fib", "calls", "tree")])
    print("*The alternative cost models (M1 / M2)*\n")
    table(["Configuration", "fib arg", "calls arg", "tree arg", "fib tls", "calls tls", "tree tls"], rows)


def uring():
    b = "cancel-uring"
    print("### T8 io_uring\n")
    rows = []
    for lab in sorted(k[2] for k in cells if k[0] == M1 and k[1] == b):
        xs = {m: c(m, b, lab) for m in MACHINES}

        def written(m):
            w = xs[m].values.get("buffer_writes_after_return", []) if xs[m] else []
            return f"{sum(w):,.0f} (max {max(w):,.0f} per run, {len(w)} runs)" if w else "not run"

        rows.append([lab] + [both(lambda m: short(xs[m], metric, "", 0))
                             for metric in ("reads", "cancels_submitted", "cancel_ok", "cancel_enoent", "cancel_ealready",
                                            "completed_after_failed_cancel", "completed_with_late_data")]
                    + [written(M1), written(M2)]
                    + [short(xs[m], "cancel_to_return_p50", "ms") + " / " + short(xs[m], "cancel_to_return_p99", "ms")
                       for m in MACHINES])
    table(["Configuration", "reads per run", "cancels submitted (M1 / M2)", "cancel ok (M1 / M2)", "-ENOENT (M1 / M2)",
           "-EALREADY (M1 / M2)", "completed after failed cancel (M1 / M2)", "completed with the late data (M1 / M2)",
           "buffer bytes written after return M1", "M2", "cancel to return p50 / p99, ms, M1", "M2"], rows)


def read_timing(path, label, groups):
    per = {}
    with open(path) as f:
        for r in csv.DictReader(f):
            per.setdefault(r["benchmark"], []).append((r["start_utc"], int(r["seconds"])))
    for bench, v in per.items():
        groups.append((bench, label, v))


def timing_rows(groups):
    groups.sort(key=lambda g: min(st for st, _ in g[2]))
    rows = []
    for bench, label, v in groups:
        secs = [x for _, x in v]
        rows.append([bench, label, min(st for st, _ in v)[:16].replace("T", " "), str(len(v)),
                     f"{sum(secs) / 60:.1f}", f"{min(secs)}-{max(secs)}"])
    return rows


def timing():
    print("### Timing on M1 (timing.csv)\n")
    groups = []
    gated = set()
    if os.path.exists(f"{M1_DIR}/load.csv"):
        with open(f"{M1_DIR}/load.csv") as f:
            gated = {r["benchmark"] for r in csv.DictReader(f) if r["quiet_below"] != "off"}
    read_timing(f"{M1_DIR}/timing.csv", "kept", groups)
    groups[:] = [(bench, "kept, gated" if bench in gated else label, v) for bench, label, v in groups]
    for sub, label in (("superseded-cancel-uring-before-check-fix", "superseded (A1's check, section 10.3)"),
                       ("superseded-before-quiet-gate", "superseded (no quiet gate, section 10.2)"),
                       ("aborted-gated-rerun", "aborted (load accounting, section 10.2)")):
        path = f"{M1_DIR}/{sub}/timing.csv"
        if os.path.exists(path):
            read_timing(path, label, groups)
    rows = timing_rows(groups)
    kept = sum(sum(x for _, x in v) for _, label, v in groups if label.startswith("kept"))
    every = sum(sum(x for _, x in v) for _, _, v in groups)
    rows.append(["total of the kept groups", "", "", "", f"{kept / 60:.1f}", ""])
    rows.append(["total, superseded and aborted runs included", "", "", "", f"{every / 60:.1f}", ""])
    table(["Benchmark (with its controls)", "runs", "first start (UTC)", "repetitions", "minutes",
           "seconds per repetition"], rows)


def timing_runner(path, total_row=True):
    groups = []
    read_timing(path, "kept", groups)
    total = sum(sum(x for _, x in v) for _, _, v in groups)
    rows = [[r[0], r[2], r[3], r[4], r[5]] for r in timing_rows(groups)]
    if total_row:
        rows.append(["total", "", "", f"{total / 60:.1f}", ""])
    table(["Benchmark (with its controls)", "first start (UTC)", "repetitions", "minutes", "seconds per repetition"],
          rows)


def timing_m2():
    print("### Timing on M2 (timing.csv)\n")
    timing_runner(f"{M2_DIR}/timing.csv")


def timing_m3():
    print("### Timing on M3 (timing.csv)\n")
    timing_runner(f"{M3_DIR}/timing.csv", total_row=False)


def load_summary():
    print("### Machine load (load.csv)\n")
    S.t_load([M1_DIR, M2_DIR, M3_DIR], True)


def main():
    for f in (mem, rule_pairs, rule_m1, rule_m2, rule_m3, spawn_join, max_tasks, echo, deep, starvation, probe, uring,
              timing, timing_m2, timing_m3, load_summary):
        f()
    crashes = sum(1 for k, v in cells.items() for st in v.status.get("process_exit", []))
    print(f"process_exit rows over the three directories: {crashes}")


if __name__ == "__main__":
    main()
