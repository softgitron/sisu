"""Renders the Phase 1 report from its template: every {{ expression }} field is
evaluated against the committed CSVs (through summarize.py and tables.py), so
that no number in the report is typed by hand, and the text is rewrapped at 80
columns. A field that cannot be evaluated is an error (the report is not
written), unless OUTPUT ends in ".draft".

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


def w(m, label):
    """Section 2.4's worst ratio over the four memory conditions (the 1M cell
    after the excursion against the larger of its two substitute baselines)."""
    rs = []
    for n in (100000, 1000000):
        rs.append(float(imr(m, label, n)))
    rs.append(float(imr(m, label, 100000, "excursion")))
    rs.append(float(imr(m, label, 1000000, "excursion", 100000)))
    rs.append(float(rat(m, "idle-memory", f"{label} N=1000000", "total_per_task_excursion", "go N=1000000",
                        "total_per_task_shallow")))
    return f"{max(rs):.2f}"


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
env["TB"] = TB
env["cells"] = cells
env.update(M1=RT.M1, M2=RT.M2, IM="idle-memory", SJ="spawn-join", MT="max-tasks", EC="echo",
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
