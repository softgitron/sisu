#!/usr/bin/env python3
"""Rewrite the sessions (.console) of the given directories or files only,
as manual/tools/check.py --bless would, without touching other sessions.

    python3 manual/tools/diagnostics/bless_sessions.py --sisu PATH [--check] DIR_OR_FILE...

A session runs where check.py runs it (check.py's `session_sandbox`): beside
its files, against the exercise's start state for a session under
solutions/, or in the package's tree for a session beside NAME.pkg/. With
--check, nothing is written, and each session that differs is listed (exit
status 1). Read the diff of every session it rewrites before committing it.
"""
import importlib.util
import os
import shlex
import shutil
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("check", os.path.join(HERE, "..", "check.py"))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


def sessions(paths):
    found = []
    for p in paths:
        p = os.path.abspath(p)
        if os.path.isdir(p):
            found += [os.path.join(p, n) for n in sorted(os.listdir(p)) if n.endswith(".console")]
        elif p.endswith(".console"):
            found.append(p)
        else:
            sys.exit(f"{p} is neither a directory nor a session")
    return found


def main():
    args = sys.argv[1:]
    if "--sisu" not in args:
        sys.exit(__doc__)
    i = args.index("--sisu")
    sisu = os.path.abspath(args[i + 1])
    del args[i:i + 2]
    only_check = "--check" in args
    bad = 0
    for path in sessions(a for a in args if not a.startswith("--")):
        steps = check.parse_session(check.read(path))
        if not steps:
            print(f"NOT A SESSION {path}")
            bad += 1
            continue
        tmp = check.session_sandbox(path)
        try:
            text = ""
            for command, _ in steps:
                _, got = check.run(sisu, shlex.split(command)[1:], tmp)
                text += f"$ {command}\n{got}"
        finally:
            shutil.rmtree(tmp)
        if text == check.read(path):
            continue
        bad += 1
        if only_check:
            print(f"DIFFERS {check.rel(path)}")
        else:
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
            print(f"blessed {check.rel(path)}")
    return 1 if bad and only_check else 0


if __name__ == "__main__":
    sys.exit(main())
