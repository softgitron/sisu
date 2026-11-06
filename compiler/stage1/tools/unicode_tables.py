#!/usr/bin/env python3
"""Writes stage 1's `src/unicode_tables.sisu`: the tables that NFC names and
D-048's lints read (RULES.md, R136, R205), from Python's `unicodedata` and
Unicode's own `Scripts.txt`, `confusables.txt` and
`DerivedCoreProperties.txt`; and the tables at the end of
`std/string.sisu`, below its line `STD_MARK`, which its normalisation forms
and graphemes read (RULES.md, R162, R205), the graphemes' from
`GraphemeBreakProperty.txt`, `DerivedCoreProperties.txt` (`InCB`) and
`emoji-data.txt` (`Extended_Pictographic`).

    python3 tools/unicode_tables.py            # rewrites both
    python3 tools/unicode_tables.py --check    # exits 1 when either is stale

Unicode's files are those of Unicode 15.1.0, the version of Python's
`unicodedata`, fetched once from their pinned URLs (`FILES`, below) into
`compiler/target/unicode/15.1.0/`, which the repository does not hold; each
is checked to be the version's by its first line.

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
  and marks from `L` to `H` beyond ASCII, as `Scripts.txt` gives it (its
  `Common`, `Inherited` and `Unknown` are no script's; the scripts of
  `CJK_SCRIPTS` are one script, since a Japanese or a Korean name mixes
  them), a range running on over the code points no character is
  assigned to;
- `confusables()`: `SSSSSS-OOOO-LL`, the character `S` that can stand in a
  name (`XID_Continue`) and reads as the `L` code points of
  `confusable_pool()` from the `O`th, every such record of
  `confusables.txt`, from which a skeleton is made as UTS #39 makes it.
"""

import sys
import unicodedata
import urllib.request
from pathlib import Path

PER_LINE = 6

VERSION = "15.1.0"
UCD = "https://www.unicode.org/Public/15.1.0/ucd/"
# Each file Unicode publishes, by its pinned URL: `confusables.txt` is the
# security data's (UTS #39), published beside the character database.
FILES = {
    "GraphemeBreakProperty.txt": UCD + "auxiliary/GraphemeBreakProperty.txt",
    "GraphemeBreakTest.txt": UCD + "auxiliary/GraphemeBreakTest.txt",
    "DerivedCoreProperties.txt": UCD + "DerivedCoreProperties.txt",
    "emoji-data.txt": UCD + "emoji/emoji-data.txt",
    "Scripts.txt": UCD + "Scripts.txt",
    "confusables.txt": "https://www.unicode.org/Public/security/15.1.0/confusables.txt",
}


def data_dir():
    return Path(__file__).resolve().parent.parent.parent / "target" / "unicode" / VERSION


def data(name):
    """The lines of Unicode's file `name`, fetched once into `data_dir()`."""
    path = data_dir() / name
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        try:
            with urllib.request.urlopen(FILES[name], timeout=60) as r:
                body = r.read()
        except OSError as e:
            sys.exit("unicode_tables: cannot fetch {} ({}); put it in {}".format(FILES[name], e, path.parent))
        path.write_bytes(body)
    text = path.read_text(encoding="utf-8")
    # The version in the file's header: `Scripts-15.1.0.txt`, `Version:
    # 15.1.0`, or the emoji data's `Emoji Version 15.1`.
    head = text.split("\n", 12)[:12]
    if not any(VERSION in l or "Version " + VERSION[:-2] + " " in l for l in head):
        sys.exit("unicode_tables: {} is not Unicode {}'s".format(path, VERSION))
    return text.split("\n")


def properties(name):
    """`(low, high, value)` for each record of a property file: the code
    points `low` to `high` and the value its fields after the first name."""
    out = []
    for line in data(name):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        fields = [f.strip() for f in line.split(";")]
        lo, _, hi = fields[0].partition("..")
        out.append((int(lo, 16), int(hi or lo, 16), fields[1:]))
    return out

# The scripts a name of East Asia mixes, counted as one.
CJK_SCRIPTS = {"Han", "Hiragana", "Katakana", "Hangul", "Bopomofo"}
# `Scripts.txt`'s values that are no script's.
NO_SCRIPT = {"Common", "Inherited", "Unknown"}


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
    script = {}
    for lo, hi, [value] in properties("Scripts.txt"):
        if value in NO_SCRIPT:
            continue
        for c in range(max(lo, 0x80), hi + 1):
            if is_letter_or_mark(c):
                script[c] = "CJK" if value in CJK_SCRIPTS else value.upper().replace("_", " ")
    names, scripts = [], []
    for c in sorted(script):
        s = script[c]
        if s not in names:
            names.append(s)
        k = names.index(s)
        # A range runs on over code points no character is assigned to,
        # which no name holds.
        if scripts and scripts[-1][2] == k and all(
            unicodedata.category(chr(x)) == "Cn" for x in range(scripts[-1][1] + 1, c)
        ):
            scripts[-1][1] = c
        else:
            scripts.append([c, c, k])
    in_names = set()
    for lo, hi, fields in properties("DerivedCoreProperties.txt"):
        if fields[0] == "XID_Continue":
            in_names.update(range(lo, hi + 1))
    ranges, pool = [], []
    for lo, hi, fields in properties("confusables.txt"):
        if lo not in in_names:
            continue
        target = [int(x, 16) for x in fields[0].split()]
        ranges.append((lo, len(pool), len(target)))
        pool.extend(target)
    ranges.sort()
    return comps, decomps, classes, names, scripts, ranges, pool


def block(records):
    lines = []
    for i in range(0, len(records), PER_LINE):
        lines.append(" ".join(records[i:i + PER_LINE]))
    return "\n".join(lines) + "\n"


def text():
    comps, decomps, classes, names, scripts, ranges, pool = tables()
    out = []
    out.append("##! Unicode's tables for names (RULES.md, R136), generated by\n")
    out.append("##! `tools/unicode_tables.py` from Unicode {} as Python's `unicodedata`\n".format(unicodedata.unidata_version))
    out.append("##! has it, with the scripts of `Scripts.txt` and the confusables of\n")
    out.append("##! `confusables.txt` (R205): do not edit it by hand, and `check.sh` fails\n")
    out.append("##! when it is stale.\n")
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
        "`LLLLLL-HHHHHH-SS`: the script `SCRIPTS[SS]` of the letters and marks",
        "beyond ASCII from `L` to `H`, as `Scripts.txt` gives it ({} records); a".format(len(scripts)),
        "character of no script has none.",
    ], ["{:06x}-{:06x}-{:02x}".format(*r) for r in scripts])
    fn("confusables", [
        "`SSSSSS-OOOO-LL`: the character `S` of a name that reads as the `L` code",
        "points of `confusable_pool()` from the `O`th, as `confusables.txt` gives",
        "it ({} records).".format(len(ranges)),
    ], ["{:06x}-{:04x}-{:02x}".format(*r) for r in ranges])
    out.append("\n")
    out.append("## The code points of `confusables()`, six hexadecimal digits each, {} to a\n".format(PER_LINE * 4))
    out.append("## line ({} code points).\n".format(len(pool)))
    out.append("pub pure def confusable_pool() -> str:\n")
    lines = []
    for i in range(0, len(pool), PER_LINE * 4):
        lines.append("".join("{:06x}".format(x) for x in pool[i:i + PER_LINE * 4]))
    out.append('    return """' + "\n".join(lines) + '\n"""\n')
    return "".join(out)


# ---------------------------------------------------------------- std.string's

# The line after which `std/string.sisu` holds the tables this script
# writes (RULES.md, R162): the normalisation forms' and the graphemes'.
STD_MARK = "# ---------------------------------------------------------------- Unicode's tables, generated\n"


def is_hangul_syllable(c):
    return 0xAC00 <= c <= 0xD7A3


# The values of `Grapheme_Cluster_Break` (`GraphemeBreakProperty.txt`) and
# of `Indic_Conjunct_Break` (`DerivedCoreProperties.txt`), as the grapheme
# table writes them; `std/string.sisu`'s constants `GCB_*` and `INCB_*`
# are the same numbers.
GCB = {
    "CR": 1, "LF": 2, "Control": 3, "Extend": 4, "ZWJ": 5,
    "Regional_Indicator": 6, "Prepend": 7, "SpacingMark": 8, "L": 9, "V": 10,
    "T": 11, "LV": 12, "LVT": 13,
}
INCB = {"Consonant": 1, "Extend": 2, "Linker": 3}
PICTOGRAPHIC = 0x10


def grapheme_records():
    """`[low, high, properties]` for every run of code points with the same
    grapheme properties, but for the Hangul syllables (their `LV` and
    `LVT` alternate, and `std.string` computes them)."""
    props = {}
    for lo, hi, fields in properties("GraphemeBreakProperty.txt"):
        for c in range(lo, hi + 1):
            if not is_hangul_syllable(c):
                props[c] = props.get(c, 0) | GCB[fields[0]]
    for lo, hi, fields in properties("emoji-data.txt"):
        if fields[0] == "Extended_Pictographic":
            for c in range(lo, hi + 1):
                props[c] = props.get(c, 0) | PICTOGRAPHIC
    for lo, hi, fields in properties("DerivedCoreProperties.txt"):
        if fields[0] == "InCB":
            for c in range(lo, hi + 1):
                props[c] = props.get(c, 0) | INCB[fields[1]] * 0x20
    out = []
    for c in sorted(props):
        if out and out[-1][1] == c - 1 and out[-1][2] == props[c]:
            out[-1][1] = c
        else:
            out.append([c, c, props[c]])
    return out


def std_tables():
    canonical, compat, pool, comps, classes = [], [], [], [], []
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
    graphemes = grapheme_records()
    return canonical, compat, pool, comps, classes, graphemes


def std_text():
    canonical, compat, pool, comps, classes, graphemes = std_tables()
    out = [STD_MARK, "\n"]
    out.append("# Written by `stage1/tools/unicode_tables.py` from Unicode {} as Python's\n".format(unicodedata.unidata_version))
    out.append("# `unicodedata` has it (RULES.md, R162), the graphemes' from Unicode's own\n")
    out.append("# `GraphemeBreakProperty.txt`, `emoji-data.txt` and\n")
    out.append("# `DerivedCoreProperties.txt` (R205): do not edit them by hand. Each is a\n")
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
    fn("grapheme_table", [
        "`LLLLLL-HHHHHH-PP`: the grapheme properties of the code points from `L`",
        "to `H` (UAX #29), `PP` their sum: the `Grapheme_Cluster_Break` value",
        "(`GCB_*`), 0x10 for `Extended_Pictographic`, and `Indic_Conjunct_Break`",
        "times 0x20 (`INCB_*`); a code point of none, and a Hangul syllable,",
        "whose class is computed, has no record ({} records).".format(len(graphemes)),
    ], ["{:06x}-{:06x}-{:02x}".format(*r) for r in graphemes])
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
