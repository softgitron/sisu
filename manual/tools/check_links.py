#!/usr/bin/env python3
"""Check the manual's links: every relative link reaches a file, and every
#anchor a heading of the page it names.

    python3 manual/tools/check_links.py [-v]

Reads every Markdown file under manual/ (the pages, the plan, the style
guide, SUMMARY.md and the tools' notes) and each link `[text](target)`
outside code: a target that starts with a scheme (https:, mailto:) is not
checked; any other is a path relative to the file, which must exist, and
an `#anchor` after it (or alone, for the file itself) must be the anchor
that GitHub gives one of the headings of that Markdown file, wherever in
the repository it is. GitHub makes an anchor from a heading's text by
lowering its case, dropping every character but letters, digits, spaces,
`-` and `_`, and writing each space as `-`; a heading whose anchor is
taken already gets `-1`, `-2` and so on. Headings inside fenced blocks
are not headings. `tools/check_docs.py` at the repository root checks the
paths of every Markdown file's links, not their anchors.

Exit status 1 on any problem.
"""
import os
import re
import sys

MANUAL = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(MANUAL)
LINK = re.compile(r"\[((?:[^\[\]]|\[[^\[\]]*\])*)\]\(\s*<?([^()\s<>]+(?:\([^()\s]*\))?)>?(?:\s+\"[^\"]*\")?\s*\)")
HEADING = re.compile(r"^ {0,3}(#{1,6})[ \t]+(.*?)(?:[ \t]+#+)?[ \t]*$")
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def mask_code(text):
    """TEXT with fenced blocks and code spans blanked out (newlines kept),
    so that offsets and line numbers stay the same."""
    out = list(text)
    lines = text.split("\n")
    pos = 0
    fence = None
    for line in lines:
        m = FENCE.match(line)
        if fence is None and m:
            fence = m.group(1)[0] * len(m.group(1))
            blank = True
        elif fence is not None:
            blank = True
            if m and m.group(1).startswith(fence) and line.strip() == m.group(1):
                fence = None
        else:
            blank = False
        if blank:
            for i in range(pos, pos + len(line)):
                out[i] = " "
        pos += len(line) + 1
    text = "".join(out)
    # Code spans: a run of N backticks up to the next run of exactly N.
    out = list(text)
    i = 0
    while i < len(text):
        if text[i] != "`":
            i += 1
            continue
        j = i
        while j < len(text) and text[j] == "`":
            j += 1
        run = j - i
        k = j
        close = -1
        while k < len(text):
            if text[k] == "`":
                e = k
                while e < len(text) and text[e] == "`":
                    e += 1
                if e - k == run:
                    close = k
                    break
                k = e
            elif text.startswith("\n\n", k):
                break
            else:
                k += 1
        if close < 0:
            i = j
            continue
        for x in range(i, close + run):
            if out[x] != "\n":
                out[x] = " "
        i = close + run
    return "".join(out)


def heading_text(raw):
    """The text GitHub renders for a heading's source RAW."""
    text = re.sub(r"!?\[((?:[^\[\]]|\[[^\[\]]*\])*)\]\([^)]*\)", r"\1", raw)
    text = re.sub(r"<[^>]+>", "", text)
    text = text.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
    return text.strip()


def slug(text):
    text = text.lower()
    kept = "".join(c for c in text if c.isalnum() or c in " -_")
    return kept.replace(" ", "-")


_anchors = {}


def anchors(path):
    """The anchors of the headings of the Markdown file PATH."""
    if path not in _anchors:
        found, seen = set(), {}
        fence = None
        for line in read(path).split("\n"):
            m = FENCE.match(line)
            if fence is None and m:
                fence = m.group(1)[0] * len(m.group(1))
                continue
            if fence is not None:
                if m and m.group(1).startswith(fence) and line.strip() == m.group(1):
                    fence = None
                continue
            h = HEADING.match(line)
            if not h:
                continue
            base = slug(heading_text(h.group(2)))
            name = base
            while name in seen:
                seen[base] += 1
                name = f"{base}-{seen[base]}"
            seen[name] = 0
            found.add(name)
        _anchors[path] = found
    return _anchors[path]


def markdown_files():
    found = []
    for d, dirs, files in os.walk(MANUAL):
        dirs[:] = sorted(x for x in dirs
                         if not x.startswith(".") and x not in ("__pycache__", "tmpl"))
        found += [os.path.join(d, f) for f in sorted(files) if f.endswith(".md")]
    return found


def main():
    verbose = "-v" in sys.argv[1:]
    problems = []
    files = markdown_files()
    links = with_anchor = 0
    for page in files:
        text = read(page)
        plain = mask_code(text)
        where_page = os.path.relpath(page, MANUAL)
        for m in LINK.finditer(plain):
            target = m.group(2)
            if SCHEME.match(target):
                continue
            links += 1
            line = plain.count("\n", 0, m.start()) + 1
            where = f"{where_page}:{line}"
            path, _, anchor = target.partition("#")
            full = page if not path else os.path.normpath(
                os.path.join(os.path.dirname(page), path))
            if not os.path.exists(full):
                problems.append(f"{where}: `{target}`: no file `{os.path.relpath(full, ROOT)}`")
                continue
            if not anchor:
                continue
            with_anchor += 1
            if os.path.isdir(full):
                full = os.path.join(full, "README.md")
                if not os.path.isfile(full):
                    problems.append(f"{where}: `{target}`: an anchor into a directory with no README.md")
                    continue
            if not full.endswith(".md"):
                problems.append(f"{where}: `{target}`: an anchor into a file that is not Markdown")
                continue
            if anchor not in anchors(full):
                problems.append(f"{where}: `{target}`: `{os.path.relpath(full, ROOT)}` has no heading "
                                f"with the anchor `#{anchor}`")
            elif verbose:
                print(f"{where}: `{target}` ok")
    for p in problems:
        print(p)
    print(f"{len(files)} files, {links} relative links ({with_anchor} with an anchor), "
          f"{len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
