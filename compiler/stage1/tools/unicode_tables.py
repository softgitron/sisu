#!/usr/bin/env python3
"""Writes stage 1's `src/unicode_tables.sisu`: the tables that NFC names and
D-048's lints read (RULES.md, R136), from Python's `unicodedata`.

    python3 tools/unicode_tables.py            # rewrites src/unicode_tables.sisu
    python3 tools/unicode_tables.py --check    # exits 1 when the file is stale

Each table is a text of fixed-width records in hexadecimal, sorted by their
first field, each followed by one space or line break, as `text.unicode`'s
are, so that a lookup is a binary search over the text itself:

- `compositions()`: `BBBBBB-MMMMMM-CCCCCC`, the canonical composition of
  the letter or mark `B` and the mark `M` into the letter `C`, for every
  canonical decomposition of a letter into two code points that NFC
  composes again, sorted by `B` and then `M`;
- `decompositions()`: `CCCCCC-BBBBBB-MMMMMM-F`, every canonical
  decomposition of a letter or mark into one or two code points (`M` is 0
  for one), sorted by `C`, with `F` 1 where NFC composes it again and 0
  where it does not (a composition exclusion or a singleton), but for the
  compatibility ideographs, which a name keeps;
- `combining_classes()`: `LLLLLL-HHHHHH-CC`, the canonical combining class
  `CC` of the code points from `L` to `H`, for every class but 0;
- `scripts()`: `LLLLLL-HHHHHH-SS`, the script `SCRIPTS[SS]` of the letters
  from `L` to `H`, a letter's script being the first word of its name (the
  words of `COMMON_WORDS` make it no script's, those of `WIDTH_WORDS` are
  passed over, those of `ALIASES` name another script, and the scripts of
  `CJK_SCRIPTS` are one script, since a Japanese or a Korean name mixes
  them);
- `confusables()`: `LLLLLL-HHHHHH-PPPPPP`, the code points from `L` to `H`
  that read as the ASCII letters or digits from `P` on: the letters whose
  NFKC form is one ASCII letter or digit (the fullwidth forms, the
  mathematical alphabets, the small capitals that have one), and the
  Cyrillic and Greek letters whose glyphs are the Latin ones'
  (`GLYPHS`, below).
"""

import sys
import unicodedata
from pathlib import Path

PER_LINE = 6

# The words that begin the names of letters of no script of their own.
COMMON_WORDS = {
    "MATHEMATICAL", "MODIFIER", "SUPERSCRIPT", "SUBSCRIPT", "DOUBLE-STRUCK",
    "SCRIPT", "BLACK-LETTER", "CIRCLED", "PARENTHESIZED", "SQUARED",
    "NEGATIVE", "OUTLINED", "TAG", "MICRO", "CARON", "EULER", "PLANCK",
    "ALEF", "BET", "GIMEL", "DALET", "INFORMATION", "VERTICAL", "VEDIC",
}
# The words that begin the names of letters of a script they do not name.
ALIASES = {
    "FEMININE": "LATIN", "MASCULINE": "LATIN", "TURNED": "LATIN",
    "ROMAN": "LATIN", "KELVIN": "LATIN", "ANGSTROM": "LATIN", "OHM": "GREEK",
    "KATAKANA-HIRAGANA": "CJK", "MASU": "CJK",
}
# The words before a script's name that say only a width.
WIDTH_WORDS = {"FULLWIDTH", "HALFWIDTH"}
# The scripts a name of East Asia mixes, counted as one.
CJK_SCRIPTS = {"CJK", "HIRAGANA", "KATAKANA", "HANGUL", "BOPOMOFO", "IDEOGRAPHIC"}

# The Cyrillic and Greek letters that are written as Latin ones, and the
# Latin letter each reads as (the usual pairs of Unicode's confusables).
GLYPHS = {
    # Cyrillic small letters
    0x0430: "a", 0x0435: "e", 0x043E: "o", 0x0440: "p", 0x0441: "c",
    0x0443: "y", 0x0445: "x", 0x0455: "s", 0x0456: "i", 0x0458: "j",
    0x0501: "d", 0x051B: "q", 0x051D: "w", 0x04BB: "h", 0x0491: "r",
    # Cyrillic capital letters
    0x0410: "A", 0x0412: "B", 0x0415: "E", 0x041A: "K", 0x041C: "M",
    0x041D: "H", 0x041E: "O", 0x0420: "P", 0x0421: "C", 0x0422: "T",
    0x0425: "X", 0x0405: "S", 0x0406: "I", 0x0408: "J", 0x04AE: "Y",
    0x051A: "Q", 0x051C: "W",
    # Greek small letters
    0x03BF: "o", 0x03B1: "a", 0x03BD: "v", 0x03C1: "p", 0x03B9: "i",
    0x03BA: "k", 0x03C5: "u", 0x03C7: "x", 0x03B3: "y",
    # Greek capital letters
    0x0391: "A", 0x0392: "B", 0x0395: "E", 0x0396: "Z", 0x0397: "H",
    0x0399: "I", 0x039A: "K", 0x039C: "M", 0x039D: "N", 0x039F: "O",
    0x03A1: "P", 0x03A4: "T", 0x03A5: "Y", 0x03A7: "X",
    # Latin letters that read as others
    0x0131: "i", 0x0237: "j", 0x0251: "a", 0x0261: "g",
}


def is_letter(c):
    return unicodedata.category(chr(c)).startswith("L")


def is_letter_or_mark(c):
    return unicodedata.category(chr(c))[0] in "LM"


def decomposition(c):
    d = unicodedata.decomposition(chr(c))
    if not d or d.startswith("<"):
        return None
    return [int(x, 16) for x in d.split()]


def tables():
    comps, decomps = [], []
    for c in range(0x110000):
        if not is_letter_or_mark(c):
            continue
        parts = decomposition(c)
        if parts is None or len(parts) > 2:
            continue
        # The compatibility ideographs, each another ideograph in NFC, are
        # left out: a name of them keeps them.
        if 0xF900 <= c <= 0xFAFF or 0x2F800 <= c <= 0x2FA1F:
            continue
        again = unicodedata.normalize("NFC", chr(c)) == chr(c) and len(parts) == 2
        base, mark = parts[0], parts[1] if len(parts) == 2 else 0
        decomps.append((c, base, mark, 1 if again else 0))
        if again and is_letter(c):
            comps.append((base, mark, c))
    comps.sort()
    classes = []
    for c in range(0x110000):
        k = unicodedata.combining(chr(c))
        if k == 0:
            continue
        if classes and classes[-1][1] == c - 1 and classes[-1][2] == k:
            classes[-1][1] = c
        else:
            classes.append([c, c, k])
    names, scripts = [], []
    for c in range(0x80, 0x110000):
        if not is_letter(c):
            continue
        words = unicodedata.name(chr(c), "").split()
        while words and words[0] in WIDTH_WORDS:
            words = words[1:]
        if not words or words[0] in COMMON_WORDS:
            continue
        s = ALIASES.get(words[0], words[0])
        if s in CJK_SCRIPTS:
            s = "CJK"
        if s not in names:
            names.append(s)
        k = names.index(s)
        if scripts and scripts[-1][1] == c - 1 and scripts[-1][2] == k:
            scripts[-1][1] = c
        else:
            scripts.append([c, c, k])
    confusable = {}
    for c in range(0x80, 0x110000):
        if not is_letter(c):
            continue
        k = unicodedata.normalize("NFKC", chr(c))
        if len(k) == 1 and k.isascii() and k.isalnum():
            confusable[c] = ord(k)
    for c, p in GLYPHS.items():
        confusable[c] = ord(p)
    ranges = []
    for c in sorted(confusable):
        p = confusable[c]
        if ranges and ranges[-1][1] == c - 1 and ranges[-1][2] + (c - ranges[-1][0]) == p:
            ranges[-1][1] = c
        else:
            ranges.append([c, c, p])
    return comps, decomps, classes, names, scripts, ranges


def block(records):
    lines = []
    for i in range(0, len(records), PER_LINE):
        lines.append(" ".join(records[i:i + PER_LINE]))
    return "\n".join(lines) + "\n"


def text():
    comps, decomps, classes, names, scripts, ranges = tables()
    out = []
    out.append("##! Unicode's tables for names (RULES.md, R136), generated by\n")
    out.append("##! `tools/unicode_tables.py` from Unicode {} as Python's `unicodedata`\n".format(unicodedata.unidata_version))
    out.append("##! has it: do not edit it by hand, and `check.sh` fails when it is stale.\n")
    out.append("##! Each table is a text of fixed-width records in hexadecimal, sorted by\n")
    out.append("##! their first field, each followed by one space or line break, so that a\n")
    out.append("##! lookup is a binary search over the text itself (`stage1.text.names`).\n")
    out.append("\n")
    out.append("## The scripts of `scripts()`, by their index.\n")
    out.append("pub const SCRIPTS: list[str] = [\n")
    for n in names:
        out.append('    "{}",\n'.format(n))
    out.append("]\n")
    def fn(name, doc, records):
        out.append("\n")
        for d in doc:
            out.append("## {}\n".format(d) if d else "##\n")
        out.append("pub pure def {}() -> str:\n".format(name))
        out.append('    return """' + block(records) + '"""\n')
    fn("compositions", [
        "`BBBBBB-MMMMMM-CCCCCC`: the letter `C` that NFC composes of `B` and the",
        "mark `M` after it, sorted by `B`, then `M` ({} records).".format(len(comps)),
    ], ["{:06x}-{:06x}-{:06x}".format(*r) for r in comps])
    fn("decompositions", [
        "`CCCCCC-BBBBBB-MMMMMM-F`: the canonical decomposition of `C` into `B`",
        "and `M` (0 for none), sorted by `C`; `F` is 1 where NFC composes it again",
        "({} records).".format(len(decomps)),
    ], ["{:06x}-{:06x}-{:06x}-{:x}".format(*r) for r in decomps])
    fn("combining_classes", [
        "`LLLLLL-HHHHHH-CC`: the canonical combining class of the code points",
        "from `L` to `H`, for every class but 0 ({} records).".format(len(classes)),
    ], ["{:06x}-{:06x}-{:02x}".format(*r) for r in classes])
    fn("scripts", [
        "`LLLLLL-HHHHHH-SS`: the script `SCRIPTS[SS]` of the letters beyond ASCII",
        "from `L` to `H` ({} records); a letter of no script has none.".format(len(scripts)),
    ], ["{:06x}-{:06x}-{:02x}".format(*r) for r in scripts])
    fn("confusables", [
        "`LLLLLL-HHHHHH-PPPPPP`: the letters from `L` to `H` that read as the",
        "ASCII letters or digits from `P` on ({} records).".format(len(ranges)),
    ], ["{:06x}-{:06x}-{:06x}".format(*r) for r in ranges])
    return "".join(out)


# ---------------------------------------------------------------- std.string's

# The line after which `std/string.sisu` holds the tables this script
# writes (RULES.md, R162): the normalisation forms' and the graphemes'.
STD_MARK = "# ---------------------------------------------------------------- Unicode's tables, generated\n"


def is_hangul_syllable(c):
    return 0xAC00 <= c <= 0xD7A3


def std_tables():
    canonical, compat, pool, comps, classes, marks = [], [], [], [], [], []
    for c in range(0x110000):
        if is_hangul_syllable(c):
            continue
        d = unicodedata.decomposition(chr(c))
        if not d:
            continue
        if d.startswith("<"):
            parts = [int(x, 16) for x in d.split()[1:]]
            compat.append((c, len(pool), len(parts)))
            pool.extend(parts)
            continue
        parts = [int(x, 16) for x in d.split()]
        again = len(parts) == 2 and unicodedata.normalize("NFC", chr(c)) == chr(c)
        canonical.append((c, parts[0], parts[1] if len(parts) == 2 else 0, 1 if again else 0))
        if again:
            comps.append((parts[0], parts[1], c))
    comps.sort()
    for c in range(0x110000):
        k = unicodedata.combining(chr(c))
        if k == 0:
            continue
        if classes and classes[-1][1] == c - 1 and classes[-1][2] == k:
            classes[-1][1] = c
        else:
            classes.append([c, c, k])
    for c in range(0x110000):
        if unicodedata.category(chr(c)) not in ("Mn", "Me", "Mc"):
            continue
        if marks and marks[-1][1] == c - 1:
            marks[-1][1] = c
        else:
            marks.append([c, c])
    return canonical, compat, pool, comps, classes, marks


def std_text():
    canonical, compat, pool, comps, classes, marks = std_tables()
    out = [STD_MARK, "\n"]
    out.append("# Written by `stage1/tools/unicode_tables.py` from Unicode {} as Python's\n".format(unicodedata.unidata_version))
    out.append("# `unicodedata` has it (RULES.md, R162): do not edit them by hand. Each is a\n")
    out.append("# text of fixed-width records in hexadecimal, sorted by their first field\n")
    out.append("# and each followed by one space or line break, which `record_find`\n")
    out.append("# searches.\n")
    def fn(name, doc, records):
        out.append("\n")
        for d in doc:
            out.append("## {}\n".format(d))
        out.append("pure def {}() -> str:\n".format(name))
        out.append('    return """' + block(records) + '"""\n')
    fn("canonical_table", [
        "`CCCCCC-AAAAAA-BBBBBB-F`: the canonical decomposition of `C` into `A` and",
        "`B` (0 for none), but for the Hangul syllables, which are computed; `F` is",
        "1 where the composition forms compose it again ({} records).".format(len(canonical)),
    ], ["{:06x}-{:06x}-{:06x}-{:x}".format(*r) for r in canonical])
    fn("compositions_table", [
        "`AAAAAA-BBBBBB-CCCCCC`: the character `C` that `A` and `B` compose, sorted",
        "by `A`, then `B` ({} records).".format(len(comps)),
    ], ["{:06x}-{:06x}-{:06x}".format(*r) for r in comps])
    fn("compatibility_table", [
        "`CCCCCC-OOOO-LL`: the compatibility decomposition of `C`, the `L` code",
        "points of `compatibility_pool()` from the `O`th ({} records).".format(len(compat)),
    ], ["{:06x}-{:04x}-{:02x}".format(*r) for r in compat])
    out.append("\n")
    out.append("## The code points of the compatibility decompositions, six hexadecimal\n")
    out.append("## digits each, {} to a line ({} code points).\n".format(PER_LINE * 4, len(pool)))
    out.append("pure def compatibility_pool() -> str:\n")
    lines = []
    for i in range(0, len(pool), PER_LINE * 4):
        lines.append("".join("{:06x}".format(x) for x in pool[i:i + PER_LINE * 4]))
    out.append('    return """' + "\n".join(lines) + '\n"""\n')
    fn("classes_table", [
        "`LLLLLL-HHHHHH-CC`: the canonical combining class of the code points from",
        "`L` to `H`, for every class but 0 ({} records).".format(len(classes)),
    ], ["{:06x}-{:06x}-{:02x}".format(*r) for r in classes])
    fn("marks_table", [
        "`LLLLLL-HHHHHH`: the marks (`Mn`, `Mc` and `Me`) from `L` to `H`, which a",
        "grapheme keeps after its base ({} records).".format(len(marks)),
    ], ["{:06x}-{:06x}".format(*r) for r in marks])
    return "".join(out)


def std_path():
    return Path(__file__).resolve().parent.parent.parent / "std" / "string.sisu"


def std_made(current):
    head = current.split(STD_MARK)[0]
    return head + std_text()


def main():
    path = Path(__file__).resolve().parent.parent / "src" / "unicode_tables.sisu"
    made = text()
    std = std_path()
    std_now = std.read_text()
    if sys.argv[1:] == ["--check"]:
        if not path.exists() or path.read_text() != made:
            print("unicode_tables: {} is stale; run tools/unicode_tables.py".format(path), file=sys.stderr)
            sys.exit(1)
        if STD_MARK in std_now and std_now != std_made(std_now):
            print("unicode_tables: {} is stale; run tools/unicode_tables.py".format(std), file=sys.stderr)
            sys.exit(1)
        return
    path.write_text(made)
    if STD_MARK in std_now:
        std.write_text(std_made(std_now))


if __name__ == "__main__":
    main()
