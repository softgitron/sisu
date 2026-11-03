import os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from diag_entries import E
MANUAL = "/home/user/sisu/manual"
CAT = "/home/user/sisu/compiler/catalog/messages.catalog"

# The catalog's codes.
codes = []
for line in open(CAT, encoding="utf-8"):
    m = re.match(r"^([EWRD]\d{4})\s+(\w+)\s+(\S+)\s+(.*)$", line.rstrip("\n"))
    if m:
        codes.append(m.groups())
by_code = {c: (k, i, t) for c, k, i, t in codes}

def message(code):
    k, i, t = by_code[code]
    if code == "D0002":
        return "the list of commands that `sisu help` prints"
    t = re.sub(r"\\n\s*", " ", t)
    t = t.replace("{{", "{").replace("}}", "}")
    return t.replace("|", "\\|")

# The chapter of the tutorial that first shows a code.
chapters = sorted(f for f in os.listdir(f"{MANUAL}/tutorial") if f.endswith(".md"))
def taught(code):
    for f in chapters:
        if re.search(rf"\b{code}\b", open(f"{MANUAL}/tutorial/{f}", encoding="utf-8").read()):
            n = int(f[:2])
            return f"[chapter {n}](../tutorial/{f})"
    return None

entries = {e["code"]: e for e in E}

# Driver codes with an entry: the example is a session of R16.
DRIVER = [
    ("D0003", "usage-errors", "#commands-options-and-arguments",
     "The first word of the command line is not a command of `sisu`.",
     "Spell the command as `sisu help` lists it; a file to run is named after `run`, or first when it ends in `.sisu`."),
    ("D0004", "usage-errors", "#commands-options-and-arguments",
     "The command needs an argument that the command line does not give: a file for `sisu ws`, a log for `sisu replay`, a code for `sisu explain`, a name for `sisu new`.",
     "Give the argument. The message says \"a file\" for every command, also where a code or a name is needed."),
    ("D0006", None, "#e0200",
     "The program had errors, so the command did not run it (or build or document it). It follows the diagnostics and counts the errors; warnings do not stop a run.",
     "Fix the errors above it. Every session on this page that runs a wrong program ends with it."),
    ("D0009", "usage-errors", "#commands-options-and-arguments",
     "An option that the command does not take, or an argument after the file where the command takes none.",
     "Check the command's options in [R16](sisu-command.md); options come before the file."),
    ("D0010", None, "#sisu-explain",
     "`sisu explain` was given a code or an identifier that the catalog does not have.",
     "Copy the code from the diagnostic, as `E0200`, or the identifier, as `resolve.undefined-name`."),
    ("D0021", None, "#e0400",
     "The ownership checker found errors, and nothing else did, so the program did not run. It names `--defer-borrow`, which runs the program under the ownership monitor to see whether the conflict happens.",
     "Fix the ownership errors above it; `sisu run --defer-borrow` shows a run that reaches the conflict."),
    ("D0023", "usage-errors", "#commands-options-and-arguments",
     "An option that takes a number was given something else.",
     "Give a whole number, as in `--explore 20`."),
    ("D0037", None, "#recording-a-run---record-and---redact",
     "A replay reached a capability call that the log does not hold the result of: the log was redacted, or the program does something other than what was recorded.",
     "Record without `--redact` for the capabilities the replay needs."),
    ("D0052", None, "#sisu-test",
     "`--faults` sets the fault rate of the `sim` profile, and the command line has no `--sim`.",
     "Add `--sim`."),
    ("D0072", None, "#sisu-new",
     "A package name that is not lowercase letters, digits and `_` starting with a letter, or that is the name of a standard-library module.",
     "Choose a name such as `word_count`."),
    ("D0073", None, "#sisu-new",
     "`sisu new` does not write into a directory that exists.",
     "Choose another name, or remove the directory first."),
    ("D0111", None, "#compiling-and-running-natively---native-and---release",
     "`--release` chooses the native backend's release profile, and the command line has no `--native`; the interpreter has no profiles.",
     "Add `--native`, or leave `--release` out."),
    ("D0113", None, "#sisu-doc",
     "`sisu doc --std` was given a path that names no module of the standard library.",
     "Name the module as an import names it, as `std.time`."),
    ("D0115", None, "#choosing-the-front-end",
     "`--front-end` takes `stage1` or `rust`.",
     "Use one of the two, or leave the option out for stage 1."),
]
DRIVER += [
    ("R0108", None, "#sisu-test",
     "A snapshot test ran `assert_snapshot` for a value that has no stored snapshot yet. The test fails and shows the value it would store.",
     "Read the value, and if it is right, run `sisu test --update`, which stores it in `NAME.snap` beside the program `NAME.sisu`."),
]
driver = {c: (s, a, r, f) for c, s, a, r, f in DRIVER}

RANGES = [
    ("E00", "Lexing", "E0001 to E0018", "The lexer reads the text into tokens and the indentation into blocks. Its errors are about characters: tabs, unclosed strings and brackets, escapes, number literals, and what Sisu does not spell (`;`, a backslash at the end of a line)."),
    ("E01", "Parsing", "E0100 to E0126", "The parser builds the syntax tree. Its errors name what it expected and what it found, and several name a habit from another language and the Sisu way to write it."),
    ("E02", "Names and declarations", "E0200 to E0264", "Name resolution finds what each name means and checks declarations: undefined and duplicate names, imports and visibility, derives, traits and impls."),
    ("E03", "Types", "E0300 to E0399", "The type checker checks every expression's type: mismatches, calls, operators, `match`, mutability, indexing, traits and generics."),
    ("E04", "Ownership", "E0400 to E0444", "The ownership checker checks moves, borrows, views, tasks and must-consume values ([R7](../README.md#the-reference), when it is written)."),
    ("E05", "Lowering", "E0500 to E0501", "Lowering turns the checked tree into SIR."),
    ("E06", "Compile-time evaluation", "E0600 to E0614", "`comptime` expressions, budgets and derives ([R12](../README.md#the-reference), when it is written)."),
    ("E07", "Capabilities", "E0700 to E0703", "The rules of capabilities and of `[trust]`."),
    ("E08", "Contracts", "E0800 to E0821", "`requires`, `ensures`, invariants, refinement types and property tests."),
    ("E09", "Stage 0's limits", "E0900 to E0902", "What stage 0 parses and does not run yet, and its internal errors."),
    ("E10", "Packages and manifests", "E1000 to E1017", "`sisu.toml`, `sisu.lock` and the module tree."),
    ("W0", "Warnings", "W0001 to W0014", "A warning does not stop a program: `sisu run` prints it and runs."),
    ("R0", "Run time", "R0001 to R0199", "Messages of a running program: panics, errors that `main` returns, and the failures that `sisu test` reports. They are printed without their code, as `panic: ...` or `error: ...`, followed by the trace."),
    ("R1", "Run time, continued", "", ""),
    ("D0", "The driver", "D0001 to D0119", "The messages of the `sisu` command itself ([R16](sisu-command.md)): usage errors, the lines of `sisu test` and `sisu bench`, replays, packages and the native backend."),
    ("D1", "The driver, continued", "", ""),
]

def group(code):
    return code[:3] if code[0] == "E" else code[:2]

out = []
import textwrap
def p(s=""):
    out.append(s)
def para(s):
    out.append(textwrap.fill(s, width=72, break_long_words=False, break_on_hyphens=False))

def entry_block(e):
    c = e["code"]
    k, ident, t = by_code[c]
    p(f"### {c}")
    p()
    para(f"`{ident}`: {message(c)}" + ("" if message(c).endswith(".") else "."))
    p()
    para(e["rule"])
    p()
    p(f"@@file {e['stem']}.sisu@@")
    p()
    p(f"@@output {e['stem']}.console@@")
    p()
    para(e["fix"])
    p()
    p(f"@@file {e['stem']}_fixed.sisu@@")
    p()
    p(f"@@output {e['stem']}_fixed.out@@")
    p()
    t = taught(c)
    if t:
        p(f"Shown in {t}.")
        p()

def driver_block(c):
    s, anchor, rule, fix = driver[c]
    k, ident, t = by_code[c]
    p(f"### {c}")
    p()
    para(f"`{ident}`: {message(c)}" + ("" if message(c).endswith(".") else "."))
    p()
    para(rule)
    p()
    para(fix)
    p()
    where = f"[R16](sisu-command.md{anchor})" if not anchor.startswith("#e0") else f"[{anchor[1:].upper()}]({anchor})"
    p(f"Example: {where}.")
    p()

head = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "tmpl", "diagnostics-head.md"), encoding="utf-8").read()
p(head.rstrip("\n"))
p()
done = set()
for g, title, span, intro in RANGES:
    members = [c for c, *_ in codes if group(c) == g]
    have = [c for c in members if c in entries or c in driver]
    if g in ("R1", "D1"):
        continue
    if g == "R0":
        members = [c for c, *_ in codes if c[0] == "R"]
        have = [c for c in members if c in entries or c in driver]
    if g == "D0":
        members = [c for c, *_ in codes if c[0] == "D"]
        have = [c for c in members if c in entries or c in driver]
    if not have:
        continue
    span = f"{min(members)} to {max(members)}"
    p(f"## {title}")
    p()
    para(f"Codes {span}. " + intro)
    p()
    for c in have:
        if c in entries:
            entry_block(entries[c])
        else:
            driver_block(c)
        done.add(c)

p("## Every code")
p()
p("The tables list every code of the catalog in this version, by range, with")
p("its identifier and its message; `{name}` stands for a part of the message")
p("that the diagnostic fills in. A code with an entry on this page links to")
p("it. `sisu explain CODE` prints the same line.")
p()
TABLES = [("E00", "Lexing"), ("E01", "Parsing"), ("E02", "Names and declarations"),
          ("E03", "Types"), ("E04", "Ownership"), ("E05", "Lowering"),
          ("E06", "Compile-time evaluation"), ("E07", "Capabilities"),
          ("E08", "Contracts"), ("E09", "Stage 0's limits"),
          ("E10", "Packages and manifests"), ("W", "Warnings"), ("R", "Run time"),
          ("D", "The driver")]
for prefix, title in TABLES:
    rows = [c for c, *_ in codes if c.startswith(prefix)]
    if prefix == "E0":
        continue
    p(f"### Every code: {title.lower() if title[0] != 'E' else title}" if False else f"### {title}, every code")
    p()
    p("| Code | Identifier | Message |")
    p("|---|---|---|")
    for c in sorted(rows):
        k, ident, t = by_code[c]
        cell = f"[{c}](#{c.lower()})" if c in done else c
        p(f"| {cell} | `{ident}` | {message(c)} |")
    p()
open(sys.argv[1], "w", encoding="utf-8").write("\n".join(out) + "\n")
print("entries", len(done), "codes", len(codes))
