#!/usr/bin/env python3
"""Expand a page template into a manual page.

    python3 expand.py TEMPLATE PAGE EXAMPLES_DIR_NAME

In the template, a line `@@file NAME@@` becomes the marker and the fenced
block of examples/DIR/NAME, and `@@output NAME@@` the marker and the block
of an expectation or a session (for .ws, .repl, .watch, .defer and .build,
the standard-output part).
"""
import os, re, sys
MANUAL = "/home/user/sisu/manual"
tmpl, page, d = sys.argv[1:4]
SECTIONED = (".ws", ".repl", ".watch", ".defer", ".build")
LANG = {".sisu": "sisu", ".console": "console", ".toml": "toml"}
out = []
for line in open(tmpl, encoding="utf-8").read().split("\n"):
    m = re.match(r"^@@(file|output) (\S+)@@$", line)
    if not m:
        out.append(line)
        continue
    kind, name = m.groups()
    rel = name if name.startswith(("examples/", "exercises/", "solutions/")) else f"examples/{d}/{name}"
    path = os.path.join(MANUAL, rel)
    text = open(path, encoding="utf-8").read()
    if kind == "output" and path.endswith(SECTIONED):
        text = text.split("\n--- stderr\n")[0] + "\n"
    if not text.endswith("\n"):
        sys.exit(f"{rel} does not end with a newline")
    ext = os.path.splitext(path)[1]
    lang = LANG.get(ext, "text")
    out.append(f"<!-- {kind}: {rel} -->")
    out.append(f"```{lang}")
    out.extend(text[:-1].split("\n"))
    out.append("```")
open(page, "w", encoding="utf-8").write("\n".join(out))
print(f"wrote {page}")
