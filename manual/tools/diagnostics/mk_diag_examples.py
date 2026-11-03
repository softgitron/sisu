import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from diag_entries import E
D = "/home/user/sisu/manual/examples/reference-diagnostics"
os.makedirs(D, exist_ok=True)
def w(name, text):
    with open(os.path.join(D, name), "w", encoding="utf-8") as f:
        f.write(text)
def touch(name):
    p = os.path.join(D, name)
    if not os.path.exists(p):
        open(p, "w").close()
for e in E:
    s = e["stem"]
    w(s + ".sisu", e["wrong"])
    w(s + "_fixed.sisu", e["fixed"])
    touch(s + "_fixed.out")
    w(s + ".console", f"$ sisu run {s}.sisu\n")
    if e["kind"] in ("compile", "warning"):
        touch(s + ".diag")
    if e["kind"] in ("warning", "runtime"):
        touch(s + ".out")
print(len(E), "entries")
