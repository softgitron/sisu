"""Renders the Phase 1 report from its template: every {{ expression }} field is
evaluated against the committed CSVs (through summarize.py and tables.py) and
the results directories' machine-spec.csv, versions files, timing.csv and
load.csv, so that no number in the report is typed by hand, and the text is
rewrapped at 80 columns. M1 and M2 ran every benchmark; M3, the 16 GiB runner,
ran idle-memory only. The only other source is the log summary of machine
2's lost first run (lost()), which the report quotes as a cross-check. A field
that cannot be evaluated is an error (the report is not written), unless
OUTPUT ends in ".draft".

Usage, from taskbench/:
    python3 -I -B report/render.py . report/report-phase1.md.in docs/report-phase1.md
"""
import contextlib
import importlib.util
import io
import os
import re
import sys

TB, TEMPLATE, OUTPUT = sys.argv[1:4]
spec = importlib.util.spec_from_file_location(
    "tables", os.path.join(os.path.dirname(os.path.abspath(__file__)), "tables.py"))
RT = importlib.util.module_from_spec(spec)
sys.argv = [sys.argv[0], TB]
spec.loader.exec_module(RT)
S, cells = RT.S, RT.cells


def cell(m, bench, label):
    c = cells.get((m, bench, label))
    if c is None:
        raise KeyError(f"no cell {(m, bench, label)}")
    return c


def med(m, bench, label, metric, unit="", digits=None):
    c = cell(m, bench, label)
    v = c.med(metric)
    if v is None:
        raise KeyError(f"no value {metric} in {(m, bench, label)}")
    return S.fmt_num(v, unit, digits)


def rng(m, bench, label, metric, unit="", digits=None):
    c = cell(m, bench, label)
    lo, hi, n = c.rng(metric)
    s = S.fmt_num(c.med(metric), unit, digits)
    if n > 1 and S.fmt_num(lo, unit, digits) != S.fmt_num(hi, unit, digits):
        s += f" [{S.fmt_num(lo, unit, digits)}-{S.fmt_num(hi, unit, digits)}]"
    return s


def vmin(m, bench, label, metric, unit="", digits=None):
    return S.fmt_num(cell(m, bench, label).rng(metric)[0], unit, digits)


def vmax(m, bench, label, metric, unit="", digits=None):
    return S.fmt_num(cell(m, bench, label).rng(metric)[1], unit, digits)


def nruns(m, bench, label, metric):
    return str(cell(m, bench, label).rng(metric)[2])


def rat(m, bench, label, metric, ref_label, ref_metric=None, digits=2):
    r = S.ratio(cell(m, bench, label), metric, cell(m, bench, ref_label), ref_metric)
    if r is None:
        raise KeyError(f"no ratio {label} {metric} / {ref_label}")
    return f"{r[0]:.{digits}f}"


def ratci(m, bench, label, metric, ref_label, ref_metric=None, digits=2):
    r = S.ratio(cell(m, bench, label), metric, cell(m, bench, ref_label), ref_metric)
    if r is None:
        raise KeyError(f"no ratio {label} {metric} / {ref_label}")
    s = f"{r[0]:.{digits}f}"
    if r[1]:
        s += f" (95% bootstrap interval {r[1][0]:.{digits}f}-{r[1][1]:.{digits}f}"
        if r[2]:
            s += f"; range {r[2][0]:.{digits}f}-{r[2][1]:.{digits}f}"
        s += ")"
    return s


def ci(m, bench, label, metric, ref_label, ref_metric=None, digits=2):
    """The interval part of ratci: '95% bootstrap interval a-b; range c-d'."""
    r = S.ratio(cell(m, bench, label), metric, cell(m, bench, ref_label), ref_metric)
    if r is None or not r[1]:
        raise KeyError(f"no interval {label} {metric} / {ref_label}")
    s = f"95% bootstrap interval {r[1][0]:.{digits}f}-{r[1][1]:.{digits}f}"
    if r[2]:
        s += f"; range {r[2][0]:.{digits}f}-{r[2][1]:.{digits}f}"
    return s


def total(m, bench, label, metric):
    return S.fmt_num(sum(cell(m, bench, label).values[metric]), "", 0)


def note(m, bench, label, metric):
    return cell(m, bench, label).status[metric][0][1]


def notefield(m, bench, label, metric, key):
    """A key=value field of the first row's note, e.g. decommits=..."""
    for st, n in cell(m, bench, label).status[metric]:
        mo = re.search(rf"\b{key}=([^ ;)]+)", n)
        if mo:
            return mo.group(1)
    raise KeyError(f"no {key} in the notes of {(m, bench, label, metric)}")


def gib(n_bytes, digits=1):
    return f"{int(n_bytes) / (1 << 30):.{digits}f}"


def num(x, digits=0):
    return f"{float(x):,.{digits}f}"


def table(name):
    """A generated table, without the generator's own headings: '###' lines are
    dropped, '##' lines become bold captions, a 'not computable' caption stays
    as a sentence."""
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        getattr(RT, name)()
    out = []
    for line in buf.getvalue().splitlines():
        if line.startswith("### "):
            continue
        if line.startswith("## "):
            line = "**" + line[3:].strip() + "**"
        out.append(line)
    text = "\n".join(out)
    return re.sub(r"\n{3,}", "\n\n", text).strip()


def im(m, label, n, phase="shallow", unit="KiB"):
    """KiB per task (RSS + page tables), median [range], of an idle-memory cell."""
    metric = "total_per_task_" + phase
    return rng(m, "idle-memory", f"{label} N={n}", metric, unit)


def imr(m, label, n, phase="shallow", ref_n=None, ref_phase=None):
    """Ratio of an idle-memory cell to Go's (after GC for the excursion)."""
    ref_n = ref_n or n
    ref_phase = ref_phase or ("excursion_after_gc" if phase == "excursion" else phase)
    return rat(m, "idle-memory", f"{label} N={n}", "total_per_task_" + phase, f"go N={ref_n}",
               "total_per_task_" + ref_phase)


def rule_entry(m, label):
    """summarize.memory_rule's row for a configuration on machine m:
    (worst ratio, verdict, verdict text, the four conditions' texts)."""
    for lab, conds, worst, verdict, text in S.memory_rule(cells, m) or []:
        if lab == label:
            return worst, verdict, text, conds
    raise KeyError(f"no section 2.4 row for {label} on {m}")


def w(m, label):
    """Section 2.4's worst ratio over the four memory conditions (the 1M cell
    after the excursion against the larger of its two substitute baselines), as
    summarize.memory_rule computes it. An error unless all four were measured."""
    worst, verdict, text, _ = rule_entry(m, label)
    if verdict not in ("passes", "fails") or worst in (None, float("inf")):
        raise KeyError(f"w of {label} on {m} is not computable: {text}")
    return f"{worst:.2f}"


def w100(m, label):
    """The worse of the two 100k conditions' ratios (shallow; after the
    excursion against Go after GC)."""
    return f"{max(float(imr(m, label, 100000)), float(imr(m, label, 100000, 'excursion'))):.2f}"


def v24(m, label):
    """Section 2.4's verdict for a configuration on machine m: passes, fails,
    inconclusive or not computable."""
    return rule_entry(m, label)[1]


def v24both(label):
    """Section 2.4's verdict for a configuration on both machines."""
    return S.rule_on_both(v24(RT.M1, label), v24(RT.M2, label))


def v24pair(ma, mb, label):
    """Section 2.4's verdict for a configuration on a pair of machines (M1 with
    one runner): passes or fails only if both machines of the pair agree."""
    return S.rule_on_both(v24(ma, label), v24(mb, label))


def created(m, label, n, bench="idle-memory"):
    """The tasks created over the runs of a cell, 'a-b' (or 'a' when equal)."""
    v = cell(m, bench, f"{label} N={n}").values["tasks_created"]
    lo, hi = min(v), max(v)
    return f"{lo:,.0f}" if lo == hi else f"{lo:,.0f}-{hi:,.0f}"


def spread(a, b, digits=2):
    """The absolute difference of two figures given as text (e.g. two machines'
    ratios to Go)."""
    return f"{abs(float(str(b).replace(',', '')) - float(str(a).replace(',', ''))):.{digits}f}"


def mdir(m):
    """A machine's results directory; "smoke" is machine 2's CI smoke run."""
    return RT.SMOKE_DIR if m == "smoke" else RT.DIRS[m]


def spec(m, key):
    """A machine-spec.csv field of machine m (its first row)."""
    import csv

    with open(os.path.join(mdir(m), "machine-spec.csv")) as f:
        for r in csv.DictReader(f):
            if r["metric"] == key:
                return r["note"]
    raise KeyError(f"no {key} in machine-spec.csv of {m}")


def bracket(text):
    """The selected value of a sysfs choice such as 'always [madvise] never'."""
    mo = re.search(r"\[([^\]]+)\]", text)
    if not mo:
        raise KeyError(f"no selected value in {text!r}")
    return mo.group(1)


def threads(m, bench="spawn-join"):
    """The worker threads (--threads) of machine m's runs, from the CSV's
    threads column."""
    import csv

    with open(os.path.join(mdir(m), f"{bench}.csv")) as f:
        vals = {r["threads"] for r in csv.DictReader(f)}
    if len(vals) != 1:
        raise KeyError(f"several thread counts in {bench}.csv of {m}: {vals}")
    return vals.pop()


def gover(m):
    """The Go release of machine m's controls, e.g. go1.24.7."""
    return ver(m, "go", "controls-versions.txt").split()[2]


def jdk(m):
    """The JDK of machine m's controls: its version, and the runtime's vendor and
    build, e.g. 21.0.12.1 (Temurin-21.0.12.1+1, build 21.0.12.1+1-LTS)."""
    v = ver(m, "java", "controls-versions.txt")
    mo = re.search(r'version "([^"]+)".*?Runtime Environment (?:(\S+) )?\(build ([^,)]+)\)', v)
    if not mo:
        raise KeyError(f"no JDK version in {v!r}")
    vendor = f"{mo.group(2)}, " if mo.group(2) else ""
    return f"{mo.group(1)} ({vendor}build {mo.group(3)})"


def pair(a, b, noun=""):
    """Two machines' figures: 'a noun on both machines' when equal, else
    'a noun on M1 and b on M2'."""
    n = f" {noun}" if noun else ""
    return f"{a}{n} on both machines" if a == b else f"{a}{n} on M1 and {b} on M2"


def trio(a, b, c, noun=""):
    """The three machines' figures: 'a noun on all three machines' when equal,
    else 'a noun on M1, b on M2 and c on M3' (two equal figures are named
    once)."""
    n = f" {noun}" if noun else ""
    if a == b == c:
        return f"{a}{n} on all three machines"
    if a == b:
        return f"{a}{n} on M1 and M2 and {c} on M3"
    if a == c:
        return f"{a}{n} on M1 and M3 and {b} on M2"
    if b == c:
        return f"{a}{n} on M1 and {b} on M2 and M3"
    return f"{a}{n} on M1, {b} on M2 and {c} on M3"


def loss(m, bench, label, ref_label, metric, digits=1):
    """How much lower a cell's median is than another's, in percent (positive
    when lower)."""
    a, b = cell(m, bench, label).med(metric), cell(m, bench, ref_label).med(metric)
    return f"{(1 - a / b) * 100:.{digits}f}"


def load_rows(m, bench):
    """The kept processes of a benchmark in machine m's load.csv, in order."""
    import csv

    with open(os.path.join(mdir(m), "load.csv")) as f:
        return [r for r in csv.DictReader(f) if r["benchmark"] == bench and r["kept"] == "yes"]


def others(m, bench, procs):
    """'between a and b': the range of load.csv's others_cpus (the CPUs the rest
    of the machine used meanwhile) over the kept processes whose
    (implementation, args) is in procs."""
    v = [float(r["others_cpus"]) for r in load_rows(m, bench) if (r["implementation"], r["args"]) in procs]
    if not v:
        raise KeyError(f"no load.csv rows for {procs} on {m}")
    return f"between {min(v):.2f} and {max(v):.2f}"


def walls(m, bench, procs):
    """'a-b s': the wall-clock range of the kept processes whose (implementation,
    args) is in procs."""
    v = [float(r["wall_s"]) for r in load_rows(m, bench) if (r["implementation"], r["args"]) in procs]
    if not v:
        raise KeyError(f"no load.csv rows for {procs} on {m}")
    return f"{min(v):.2f}-{max(v):.2f} s" if min(v) != max(v) else f"{min(v):.2f} s"


def load_all(m):
    """Every kept process of machine m's load.csv."""
    import csv

    with open(os.path.join(mdir(m), "load.csv")) as f:
        return [r for r in csv.DictReader(f) if r["kept"] == "yes"]


def load_median_max(m):
    """The largest, over benchmarks, of the median CPUs the other processes used
    during a benchmark's kept processes."""
    import statistics

    by = {}
    for r in load_all(m):
        by.setdefault(r["benchmark"], []).append(float(r["others_cpus"]))
    return f"{max(statistics.median(v) for v in by.values()):.3f}"


def load_max(m, bench):
    """The most CPUs the other processes used during one kept process of a benchmark."""
    return f"{max(float(r['others_cpus']) for r in load_rows(m, bench)):.3f}"


def busy(m, q=0.3, bench=None):
    """The kept processes (of one benchmark, or all) during which the other
    processes used q CPUs or more."""
    rows = load_rows(m, bench) if bench else load_all(m)
    return [r for r in rows if float(r["others_cpus"]) >= q]


# The spawn-join processes of the pooled strategies and of the others (load.csv's args).
SJ_POOLED = [("taskbench", "--strategy B --rounds 100000 --warmup 10000"),
             ("taskbench", "--strategy C --rounds 100000 --warmup 10000"),
             ("taskbench", "--strategy D --base B --rounds 100000 --warmup 10000")]
SJ_UNPOOLED = [("taskbench", "--strategy A --rounds 100000 --warmup 10000"),
               ("taskbench", "--strategy A --stack-cache 1024 --rounds 100000 --warmup 10000"),
               ("taskbench", "--strategy E --rounds 100000 --warmup 10000"),
               ("taskbench", "--strategy tokio --rounds 100000 --warmup 10000"),
               ("go", "--rounds 100000 --warmup 10000")]


def least_loaded(m, bench, proc, label, metric, unit="", digits=None):
    """(others_cpus, metric value) of the run of a cell whose process had the
    least other activity; runs and load.csv rows are matched in order."""
    rows = [r for r in load_rows(m, bench) if (r["implementation"], r["args"]) == proc]
    vals = cell(m, bench, label).values[metric]
    if len(rows) != len(vals):
        raise KeyError(f"{len(rows)} load rows for {len(vals)} runs of {label} on {m}")
    o, v = min(zip((float(r["others_cpus"]) for r in rows), vals))
    return f"{o:.2f}", S.fmt_num(v, unit, digits)


def ver(m, key, filename="versions.txt"):
    """The value of the first 'key: value' line of versions.txt (or
    controls-versions.txt) of machine m."""
    with open(os.path.join(mdir(m), filename)) as f:
        for line in f:
            if line.startswith(key + ":"):
                return line.split(":", 1)[1].strip()
    raise KeyError(f"no {key} in {filename} of {m}")


CASES = ("parse_outcome", "naive_outcome", "framed_outcome", "naive-async_outcome")


def outcomes(m, clean=True):
    """deep-recursion outcome rows on machine m, clean ones or the others."""
    return sum(1 for k, c in cells.items() if k[0] == m and k[1] == "deep-recursion" for metric in CASES
               for st, n in c.status.get(metric, []) if (st == "ok") == clean)


def crashes(m=None):
    """process_exit rows (crashes) on machine m, or on both."""
    return sum(1 for k, c in cells.items() if m is None or k[0] == m for st in c.status.get("process_exit", []))


def nconfigs(m, bench):
    return sum(1 for k in cells if k[0] == m and k[1] == bench)


def nprocs(m):
    """Benchmark processes (run ids) of machine m's results directory."""
    return len({r for k, c in cells.items() if k[0] == m for r in c.runs})


def depth_spread():
    """The largest difference between the machines' median depths reached, over
    every deep-recursion configuration and case."""
    out = 0.0
    for k in cells:
        if k[0] != RT.M1 or k[1] != "deep-recursion":
            continue
        for case in ("parse", "naive", "framed", "naive-async"):
            a = cell(RT.M1, "deep-recursion", k[2]).med(f"{case}_depth_reached")
            b = cell(RT.M2, "deep-recursion", k[2]).med(f"{case}_depth_reached")
            if a is not None and b is not None:
                out = max(out, abs(a - b))
    return f"{out:,.0f}"


def maxrat(m, bench, labels, metric, ref_label, ref_metric=None):
    """The largest of several cells' ratios to the same reference."""
    return f"{max(float(rat(m, bench, lab, metric, ref_label, ref_metric)) for lab in labels):.2f}"


PROBED = ["B/128K", "B/1M", "B/8M", "C/2K", "C/8K", "C/16K", "D(B)/128K", "D(B)/1M", "D(B)/8M"]
RESERVE_METRICS = ["parse_probe_reserve_used", "naive_probe_reserve_used", "framed_probe_reserve_used"]


def reserve_max(m):
    """The most bytes of the probe reserve any unwind used on machine m (the
    probed configurations, every case, every run)."""
    return max(int(max(cell(m, "deep-recursion", lab).values[k])) for lab in PROBED for k in RESERVE_METRICS
               if cell(m, "deep-recursion", lab).values.get(k))


def lohi(m, bench, label, metric, ref_label, ref_metric=None, digits=2):
    """The conservative range [min/max, max/min] of a ratio, 'a-b'."""
    r = S.ratio(cell(m, bench, label), metric, cell(m, bench, ref_label), ref_metric)
    if r is None or not r[2]:
        raise KeyError(f"no range {label} {metric} / {ref_label}")
    return f"{r[2][0]:.{digits}f}-{r[2][1]:.{digits}f}"


def chg(m, bench, label, ref_label, metric, digits=1):
    """The change of a cell's median relative to another's, in percent, signed."""
    a, b = cell(m, bench, label).med(metric), cell(m, bench, ref_label).med(metric)
    return f"{(a / b - 1) * 100:+.{digits}f}"


def mem_spread(labels, n=100000, machines=None, phases=("shallow", "excursion")):
    """The largest difference between two machines' ratios to Go (shallow, and
    after the excursion against Go after GC) over idle-memory configurations;
    by default M1 and M2."""
    import itertools

    machines = machines or (RT.M1, RT.M2)
    out = 0.0
    for lab in labels:
        for phase in phases:
            for ma, mb in itertools.combinations(machines, 2):
                a, b = float(imr(ma, lab, n, phase)), float(imr(mb, lab, n, phase))
                out = max(out, abs(b - a))
    return f"{out:.2f}"


LOST_SUMMARY = os.path.join(TB, "results/github-runner/full-38495423545-log-summary.md")


def lost(table, label, col, pattern=r"-?\d[\d,]*(?:\.\d+)?"):
    """A figure from the log summary of machine 2's lost first full run
    (38495423545; a cross-check, not data of record): the first match of
    `pattern` (its group 1 if it has one) in column `col` (0 is the label) of the
    row `label` in the table whose '## ' title starts with `table`."""
    current = None
    with open(LOST_SUMMARY) as f:
        for line in f:
            if line.startswith("## "):
                current = line[3:].strip()
                continue
            if current and current.startswith(table) and line.startswith(f"| {label} |"):
                cols = [x.strip() for x in line.strip().strip("|").split("|")]
                mo = re.search(pattern, cols[col])
                if mo:
                    return mo.group(1) if mo.groups() else mo.group(0)
    raise KeyError(f"no {label!r} (column {col}) in the table {table!r} of the lost run's summary")


def lost_labels(table):
    """The row labels of a table in the lost run's log summary."""
    current, out = None, []
    with open(LOST_SUMMARY) as f:
        for line in f:
            if line.startswith("## "):
                current = line[3:].strip()
                continue
            if current and current.startswith(table) and line.startswith("| ") and not line.startswith("| configuration"):
                lab = line.split("|")[1].strip()
                if not set(lab) <= set("-"):
                    out.append(lab)
    return out


def smoke_cell(bench, label):
    """A cell of machine 2's CI smoke run (harness 0.1.0, one run), which the
    report cites only for what the full run cannot show."""
    c = RT.SMOKE.get((RT.M2, bench, label))
    if c is None:
        raise KeyError(f"no smoke cell {(bench, label)}")
    return c


def smoke_im(label, n, phase="shallow", unit="KiB"):
    return S.fmt_num(smoke_cell("idle-memory", f"{label} N={n}").med("total_per_task_" + phase), unit)


def smoke_vmax(bench, label, metric, unit="", digits=None):
    return S.fmt_num(smoke_cell(bench, label).rng(metric)[1], unit, digits)


def sumcells(m, bench, labels, metric):
    return S.fmt_num(sum(sum(cell(m, bench, lab).values[metric]) for lab in labels), "", 0)


def runs_of(m, bench, labels, metric):
    return str(sum(len(cell(m, bench, lab).values[metric]) for lab in labels))


STACKFUL_1M = ["A/1M", "B/1M", "C/8K", "D(A)/1M", "D(B)/1M"]

# The superseded first runs of the groups that measure time (section 10.2), for
# the comparison with their gated reruns.
SUPERSEDED = S.cells_of(S.load([f"{RT.M1_DIR}/superseded-before-quiet-gate"]))


def scell(bench, label):
    c = SUPERSEDED.get((RT.M1, bench, label))
    if c is None:
        raise KeyError(f"no superseded cell {(bench, label)}")
    return c


def smed(bench, label, metric, unit="", digits=None):
    return S.fmt_num(scell(bench, label).med(metric), unit, digits)


def srat(bench, label, metric, ref_label, ref_metric=None, digits=2):
    r = S.ratio(scell(bench, label), metric, scell(bench, ref_label), ref_metric)
    if r is None:
        raise KeyError(f"no superseded ratio {label} {metric} / {ref_label}")
    return f"{r[0]:.{digits}f}"


def superseded_rows():
    import csv

    path = f"{RT.M1_DIR}/superseded-cancel-uring-before-check-fix/cancel-uring.csv"
    with open(path) as f:
        return list(csv.DictReader(f))


def superseded_stuck():
    """Reads flagged by the old check (tasks_with_violations of failed rows, not
    the negative control), and the slowest cancel-to-return of those runs (ms)."""
    rows = superseded_rows()
    failed = [r for r in rows if r["metric"] == "buffer_writes_after_return" and r["status"] == "failed"]
    n = sum(int(re.search(r"tasks_with_violations=(\d+)", r["note"]).group(1)) for r in failed)
    ids = {r["run_id"] for r in failed}
    slow = [float(r["value"]) / 1e6 for r in rows if r["run_id"] in ids and r["metric"] == "cancel_to_return_max"]
    reads = sum(int(float(r["value"])) for r in rows if r["metric"] == "reads" and "break_contract=false" in r["params"]
                and "source=pipe" in r["params"])
    return n, min(slow), max(slow), len(failed), reads


env = {k: v for k, v in globals().items() if callable(v)}
env["STACKFUL_1M"] = STACKFUL_1M
env["PROBED"] = PROBED
env["TB"] = TB
env["cells"] = cells
env["SMOKE"] = RT.SMOKE
env["SJ_POOLED"] = SJ_POOLED
env["SJ_UNPOOLED"] = SJ_UNPOOLED
env.update(M1=RT.M1, M2=RT.M2, M3=RT.M3, IM="idle-memory", SJ="spawn-join", MT="max-tasks", EC="echo",
           IC="idle-connections", DR="deep-recursion", ST="starvation", PO="probe-overhead", CU="cancel-uring")
text = open(TEMPLATE).read()
errors = []


def evaluate(mo):
    expr = mo.group(1).strip()
    try:
        return str(eval(expr, env))
    except Exception as e:  # noqa: BLE001
        errors.append(f"{expr}: {e!r}")
        return "ERROR"


out = re.sub(r"\{\{(.+?)\}\}", evaluate, text, flags=re.S)


def reflow(md, width=80):
    """Rewraps paragraphs and bullet items; leaves tables, headings, code and
    italic or bold one-line captions as they are."""
    import textwrap

    blocks, cur = [], []
    for line in md.split("\n"):
        if line.strip() == "":
            if cur:
                blocks.append(cur)
                cur = []
            blocks.append(None)
        else:
            cur.append(line)
    if cur:
        blocks.append(cur)
    res = []
    for b in blocks:
        if b is None:
            res.append("")
            continue
        first = b[0]
        if first.startswith(("|", "#", "```", "    ")) or (len(b) == 1 and first.startswith("**")):
            res.extend(b)
            continue
        items, item = [], []
        for line in b:
            if line.startswith(("* ", "- ")) and item:
                items.append(item)
                item = []
            item.append(line.strip())
        items.append(item)
        for it in items:
            joined = " ".join(it)
            if joined.startswith(("* ", "- ")):
                res.extend(textwrap.wrap(joined[2:], width, initial_indent=joined[:2], subsequent_indent="  ",
                                         break_on_hyphens=False, break_long_words=False))
            else:
                res.extend(textwrap.wrap(joined, width, break_on_hyphens=False, break_long_words=False))
    return "\n".join(res)


out = reflow(out)
out = re.sub(r"\n{3,}", "\n\n", out).strip() + "\n"
DRAFT = OUTPUT.endswith(".draft")
if errors:
    print(f"{len(errors)} errors", file=sys.stderr)
    if not DRAFT:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
open(OUTPUT, "w").write(out)
print(f"wrote {OUTPUT}: {len(out.splitlines())} lines")
