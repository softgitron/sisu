#!/usr/bin/env python3
"""Rewrite the sessions (.console) under the given directories only, as
manual/tools/check.py --bless would, without touching other pages' sessions.

    python3 bless_sessions.py --sisu PATH [--check] DIR...
"""
import importlib.util, os, sys, shlex, shutil
spec = importlib.util.spec_from_file_location("check", "/home/user/sisu/manual/tools/check.py")
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)
args = sys.argv[1:]
sisu = os.path.abspath(args[args.index("--sisu") + 1]); del args[args.index("--sisu"):args.index("--sisu") + 2]
only_check = "--check" in args
dirs = [a for a in args if not a.startswith("--")]
bad = 0
for d in dirs:
    d = os.path.abspath(d)
    for name in sorted(os.listdir(d)):
        if not name.endswith(".console"):
            continue
        path = os.path.join(d, name)
        stem = name[:-len(".console")]
        steps = check.parse_session(check.read(path))
        tmp = check.sandbox(d, os.path.join(d, stem + ".files"), os.path.join(d, stem + ".pkg"))
        try:
            text = ""
            for command, want in steps:
                a = shlex.split(command)
                _, got = check.run(sisu, a[1:], tmp)
                text += f"$ {command}\n{got}"
            if text != check.read(path):
                if only_check:
                    print(f"DIFFERS {path}"); bad += 1
                else:
                    with open(path, "w", encoding="utf-8") as f:
                        f.write(text)
                    print(f"blessed {path}")
        finally:
            shutil.rmtree(tmp)
sys.exit(1 if bad else 0)
