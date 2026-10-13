#!/usr/bin/env python3
"""Check every Markdown file in the repository: relative links resolve, and
every Q-nn and D-nnn reference names a question in open-questions.md or a
decision in proposal/15-decision-log.md. Exit status 1 on any problem."""
import re, os, sys
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
md = []
for d, _, fs in os.walk(root):
    if '.git' in d or 'target' in d.split(os.sep): continue
    for f in fs:
        if f.endswith('.md'): md.append(os.path.join(d, f))
qids = set(re.findall(r'^### (Q-\d+)\.', open(f'{root}/open-questions.md').read(), re.M))
dids = set(re.findall(r'^### (D-\d+) ', open(f'{root}/proposal/15-decision-log.md').read(), re.M))
bad = 0
for p in sorted(md):
    txt = open(p, encoding='utf-8').read()
    # strip fenced code blocks for link checking
    plain = re.sub(r'```.*?```', '', txt, flags=re.S)
    plain = re.sub(r'`[^`\n]*`', '', plain)
    for m in re.finditer(r'\]\(([^)\s]+)\)', plain):
        link = m.group(1)
        if link.startswith(('http://', 'https://', 'mailto:', '#')): continue
        tgt = link.split('#')[0]
        if not tgt: continue
        full = os.path.normpath(os.path.join(os.path.dirname(p), tgt))
        if not os.path.exists(full):
            print(f'BROKEN LINK {os.path.relpath(p, root)}: {link}'); bad += 1
    for q in set(re.findall(r'\bQ-\d+\b', plain)):
        if q not in qids: print(f'UNKNOWN {q} in {os.path.relpath(p, root)}'); bad += 1
    for d in set(re.findall(r'\bD-\d{3}\b', plain)):
        if d not in dids: print(f'UNKNOWN {d} in {os.path.relpath(p, root)}'); bad += 1
print(f'{len(md)} files, {len(qids)} questions, {len(dids)} decisions, {bad} problems')
sys.exit(1 if bad else 0)
