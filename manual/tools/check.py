#!/usr/bin/env python3
"""Check the manual: what a page shows is what the compiler does.

    python3 manual/tools/check.py [--sisu PATH] [--bless]

The conformance runner checks the examples and the solutions as cases
(.out, .err, .status, .diag, .test, .ws; see manual/README.md, "How the
manual is checked"). This script checks the rest:

1. Pages. Every fenced block in a page under tutorial/, guides/,
   reference/ or explanations/ has a marker on the line before it:
   <!-- file: PATH --> shows a source file whole, <!-- output: PATH -->
   an expectation file (for .ws, .repl, .watch and .defer files, the
   standard output, before the line "--- stderr"), and
   <!-- unchecked: REASON --> says why a block is not checked. PATH is
   relative to manual/. The block must equal the file.
2. Sessions. A NAME.console file is a terminal session: a line starting
   "$ sisu " is a command and the lines after it are what it printed,
   standard output and standard error together. Each runs in a fresh
   directory holding the files beside the session (for a session under
   solutions/, the exercise's start state instead) and the contents of
   NAME.files/, with an empty environment. --bless rewrites the output.
3. Exercises. The first line of exercises/CHAPTER/NAME.sisu reads
   "##! Exercise N.M, TITLE. Kind: KIND. Level: L."; the solution
   solutions/CHAPTER/NAME.sisu has the same `test` and `suite` blocks
   and passes them all; the exercise as it stands is not solved (a `fix`
   exercise does not compile or fails a test; any other compiles and
   passes no test).
4. Sources. Every .sisu file has lines of at most 80 columns and is as
   `sisu fmt` writes it, unless it does not parse or NAME.unformatted
   says why; every example is shown by some page (its source, an
   expectation or a session), so is every session, and every exercise
   is named by one.

Exit status 1 on any problem.
"""
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile

MANUAL = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(MANUAL)
PAGE_DIRS = ("tutorial", "guides", "reference", "explanations")
KINDS = ("predict", "fix", "complete", "write", "challenge")
SECTIONED = (".ws", ".repl", ".watch", ".defer")
MAX_COLUMNS = 80
MARKER = re.compile(r"^<!-- (file|output|unchecked): (.+?) -->$")
HEADER = re.compile(
    r"^##! Exercise (\d+)\.(\d+), (.+)\. Kind: (\w+)\. Level: ([123])\.$")
SUMMARY = re.compile(r"^(\d+) passed, (\d+) failed, (\d+) blocked$", re.M)

problems = []


def problem(where, what):
    problems.append(f"{where}: {what}")


def rel(path):
    return os.path.relpath(path, MANUAL)


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def walk(top, suffix):
    found = []
    for d, dirs, files in os.walk(os.path.join(MANUAL, top)):
        dirs[:] = sorted(x for x in dirs if not x.endswith(".files"))
        found += [os.path.join(d, f) for f in sorted(files) if f.endswith(suffix)]
    return found


def run(sisu, args, cwd):
    """Run sisu with ARGS in CWD; return (status, stdout and stderr)."""
    env = {"PATH": "/usr/bin:/bin"}
    p = subprocess.run([sisu] + args, cwd=cwd, env=env, timeout=120,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return p.returncode, p.stdout.decode("utf-8", "replace")


def sandbox(sources, extra):
    """A temporary directory holding the regular files of SOURCES and the
    contents of the directory EXTRA, if it exists."""
    tmp = tempfile.mkdtemp(prefix="sisu-manual-")
    for f in sorted(os.listdir(sources)):
        if os.path.isfile(os.path.join(sources, f)):
            shutil.copy(os.path.join(sources, f), tmp)
    if extra and os.path.isdir(extra):
        shutil.copytree(extra, tmp, dirs_exist_ok=True)
    return tmp


def check_pages(shown):
    """Rule 1. Records in SHOWN every path a marker names."""
    blocks = unchecked = 0
    pages = []
    for top in PAGE_DIRS:
        pages += walk(top, ".md")
    for page in pages:
        lines = read(page).split("\n")
        i = 0
        while i < len(lines):
            if not lines[i].startswith("```"):
                i += 1
                continue
            start = i
            i += 1
            while i < len(lines) and lines[i] != "```":
                i += 1
            body = "\n".join(lines[start + 1:i]) + "\n"
            i += 1
            blocks += 1
            where = f"{rel(page)}:{start + 1}"
            j = start - 1
            m = MARKER.match(lines[j]) if j >= 0 else None
            if not m:
                problem(where, "a fenced block needs a marker on the line before it")
                continue
            kind, arg = m.groups()
            if kind == "unchecked":
                unchecked += 1
                continue
            path = os.path.join(MANUAL, arg)
            shown.add(os.path.normpath(path))
            if not os.path.isfile(path):
                problem(where, f"`{arg}` does not exist")
                continue
            want = read(path)
            if kind == "output" and path.endswith(SECTIONED):
                want = want.split("\n--- stderr\n")[0] + "\n"
            if body != want:
                problem(where, f"the block differs from `{arg}`")
    return len(pages), blocks, unchecked


def parse_session(text):
    """[(command, expected output)] of a session file."""
    steps = []
    lines = text.split("\n")
    if text.endswith("\n"):
        lines.pop()
    for line in lines:
        if line.startswith("$ "):
            steps.append([line[2:], ""])
        elif steps:
            steps[-1][1] += line + "\n"
        else:
            return None
    return steps


def check_sessions(sisu, bless):
    """Rule 2."""
    sessions = walk("examples", ".console") + walk("solutions", ".console")
    for path in sessions:
        stem = os.path.basename(path)[: -len(".console")]
        here = os.path.dirname(path)
        steps = parse_session(read(path))
        if not steps:
            problem(rel(path), "a session starts with a line `$ sisu ...`")
            continue
        sources = here
        if rel(here).startswith("solutions" + os.sep):
            sources = os.path.join(MANUAL, "exercises", rel(here)[len("solutions") + 1:])
        tmp = sandbox(sources, os.path.join(here, stem + ".files"))
        try:
            text = ""
            for command, want in steps:
                args = shlex.split(command)
                if not args or args[0] != "sisu":
                    problem(rel(path), f"`{command}`: a session runs `sisu` commands only")
                    break
                _, got = run(sisu, args[1:], tmp)
                text += f"$ {command}\n{got}"
                if got != want and not bless:
                    problem(rel(path), f"`{command}` printed something else:\n{diff(want, got)}")
            else:
                if bless and text != read(path):
                    with open(path, "w", encoding="utf-8") as f:
                        f.write(text)
                    print(f"blessed {rel(path)}")
        finally:
            shutil.rmtree(tmp)
    return len(sessions)


def diff(want, got):
    import difflib
    lines = difflib.unified_diff(want.split("\n"), got.split("\n"),
                                 "expected", "printed", lineterm="", n=1)
    return "\n".join("    " + x for x in lines)


def test_blocks(text):
    """The top-level `test` and `suite` blocks of a file, as text."""
    blocks, current = [], None
    for line in text.split("\n"):
        if re.match(r"^(@\w|test |suite )", line):
            if current is None or not current[-1].startswith("@"):
                current = []
                blocks.append(current)
            current.append(line)
        elif current is not None and (line == "" or line.startswith(" ")):
            current.append(line)
        else:
            current = None
    return ["\n".join(b).rstrip("\n") for b in blocks]


def summary(output):
    m = list(SUMMARY.finditer(output))
    return tuple(int(x) for x in m[-1].groups()) if m else None


def check_exercises(sisu, named):
    """Rule 3. Returns the number of exercises."""
    exercises = walk("exercises", ".sisu")
    for path in exercises:
        where = rel(path)
        chapter = os.path.basename(os.path.dirname(path))
        name = os.path.basename(path)
        text = read(path)
        m = HEADER.match(text.split("\n")[0])
        if not m:
            problem(where, "the first line must read `##! Exercise N.M, TITLE. Kind: KIND. Level: L.`")
            continue
        kind = m.group(4)
        if kind not in KINDS:
            problem(where, f"`{kind}` is not a kind of exercise ({', '.join(KINDS)})")
        if os.path.normpath(path) not in named:
            problem(where, "no page names this exercise")
        solution = os.path.join(MANUAL, "solutions", chapter, name)
        if not os.path.isfile(solution):
            problem(where, f"there is no solution `{rel(solution)}`")
            continue
        tests = test_blocks(text)
        if not tests:
            problem(where, "an exercise needs at least one `test` block")
        if tests != test_blocks(read(solution)):
            problem(where, "the solution's `test` and `suite` blocks differ from the exercise's")
        stem = name[: -len(".sisu")]
        tmp = sandbox(os.path.dirname(path), os.path.join(os.path.dirname(path), stem + ".files"))
        try:
            checked, _ = run(sisu, ["check", name], tmp)
            status, out = run(sisu, ["test", name], tmp)
            counts = summary(out)
            if kind == "fix":
                if checked == 0 and status == 0 and counts and counts[1] == 0:
                    problem(where, "a `fix` exercise must not compile, or must fail a test, as it stands")
            elif checked != 0 or not counts or counts[0] != 0:
                problem(where, f"a `{kind}` exercise must compile and pass no test as it stands")
        finally:
            shutil.rmtree(tmp)
        tmp = sandbox(os.path.dirname(solution), os.path.join(os.path.dirname(solution), stem + ".files"))
        try:
            status, out = run(sisu, ["test", name], tmp)
            counts = summary(out)
            if status != 0 or not counts or counts[0] == 0 or counts[1:] != (0, 0):
                problem(rel(solution), f"the solution must pass every test:\n{out}")
        finally:
            shutil.rmtree(tmp)
    return len(exercises)


def check_sources(sisu, shown):
    """Rule 4."""
    sources = walk("examples", ".sisu") + walk("exercises", ".sisu") + walk("solutions", ".sisu")
    for path in sources:
        for n, line in enumerate(read(path).split("\n"), 1):
            if len(line) > MAX_COLUMNS:
                problem(f"{rel(path)}:{n}", f"{len(line)} columns; the limit is {MAX_COLUMNS}")
        stem = path[: -len(".sisu")]
        parses, _ = run(sisu, ["parse", "--shape", path], MANUAL)
        if parses == 0 and not os.path.exists(stem + ".unformatted"):
            status, out = run(sisu, ["fmt", "--check", path], MANUAL)
            if status != 0:
                problem(rel(path), f"not as `sisu fmt` writes it:\n{out}")
        top = rel(path).split(os.sep)[0]
        if top == "examples" and not any(s.startswith(os.path.normpath(stem) + ".")
                                         for s in shown):
            problem(rel(path), "no page shows this example, its output or its session")
    for path in walk("examples", ".console") + walk("solutions", ".console"):
        if os.path.normpath(path) not in shown:
            problem(rel(path), "no page shows this session")
    return len(sources)


def named_exercises():
    """The exercise files that some page names by their path."""
    named = set()
    for top in PAGE_DIRS:
        for page in walk(top, ".md"):
            for m in re.finditer(r"exercises/[\w-]+/[\w-]+\.sisu", read(page)):
                named.add(os.path.normpath(os.path.join(MANUAL, m.group(0))))
    return named


def main():
    args = sys.argv[1:]
    bless = "--bless" in args
    sisu = os.path.join(ROOT, "compiler", "target", "release", "sisu")
    if "--sisu" in args:
        sisu = os.path.abspath(args[args.index("--sisu") + 1])
    if not os.path.isfile(sisu):
        sisu = shutil.which("sisu") or sisu
    if not os.path.isfile(sisu):
        print(f"no sisu executable at {sisu}; build it or pass --sisu PATH")
        return 2
    shown = set()
    pages, blocks, unchecked = check_pages(shown)
    sessions = check_sessions(sisu, bless)
    exercises = check_exercises(sisu, named_exercises())
    sources = check_sources(sisu, shown)
    for p in problems:
        print(p)
    print(f"{pages} pages, {blocks} blocks ({unchecked} unchecked), {sessions} sessions, "
          f"{exercises} exercises, {sources} source files, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
