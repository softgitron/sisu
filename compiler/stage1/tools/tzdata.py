#!/usr/bin/env python3
"""Writes the tz database's history of the zones `Zone.named` knows at the
end of `std/time.sisu`, below its line `MARK` (RULES.md, R206), from the
system's compiled tz database, the TZif files under `/usr/share/zoneinfo`
(RFC 8536), whose version `tzdata.zi` names.

    python3 tools/tzdata.py            # rewrites the table
    python3 tools/tzdata.py --check    # exits 1 when it is stale

The zones are those of `std.time`'s `ZONES`, each with its rule as POSIX
writes a `TZ` variable, which must be the rule each TZif file ends with.
For each zone, `zone_history()` holds one line:

    NAME|ABBR/OFFSET ABBR/OFFSET ...|AT/K AT/K ...

the zone's local time types (the abbreviation and the offset east of UTC
in seconds), the first of them its time before its first change, and its
changes, from `AT`, seconds since the epoch, to the `K`th type, up to the
first change from which the zone's rule gives every later one, after which
`Zone` follows the rule. A zone without changes (`UTC`) has no line.

`--check` compares with the table only on a system whose database is the
table's version, since another version's history is another text.
"""

import re
import struct
import sys
import calendar
from pathlib import Path

ZONEINFO = Path("/usr/share/zoneinfo")
MARK = "# ---------------------------------------------------------------- the tz database, generated\n"


def time_path():
    return Path(__file__).resolve().parent.parent.parent / "std" / "time.sisu"


def version():
    first = (ZONEINFO / "tzdata.zi").read_text().split("\n", 1)[0]
    m = re.fullmatch(r"# version (\S+)", first)
    if not m:
        sys.exit("tzdata: {} names no version".format(ZONEINFO / "tzdata.zi"))
    return m.group(1)


def tzif(name):
    """The 64-bit data of the TZif file of `name`: its changes, the type
    each changes to, its types (offset, daylight, abbreviation) and the
    rule its footer writes."""
    b = (ZONEINFO / name).read_bytes()
    if b[:4] != b"TZif" or b[4] < ord("2"):
        sys.exit("tzdata: {} is not a TZif file of version 2 or later".format(name))

    def counts(at):
        return struct.unpack(">6l", b[at + 20:at + 44])

    isut, isstd, leap, times, types, chars = counts(0)
    at = 44 + times * 5 + types * 6 + chars + leap * 8 + isstd + isut
    isut, isstd, leap, times, types, chars = counts(at)
    p = at + 44
    changes = list(struct.unpack(">%dq" % times, b[p:p + 8 * times]))
    p += 8 * times
    kinds = list(b[p:p + times])
    p += times
    raw = [struct.unpack(">lBB", b[p + 6 * i:p + 6 * i + 6]) for i in range(types)]
    p += 6 * types
    names = b[p:p + chars]
    p += chars + leap * 12 + isstd + isut
    footer = b[p:].strip().decode()
    table = [(off, dst, names[a:names.index(b"\0", a)].decode()) for off, dst, a in raw]
    return changes, kinds, table, footer


def seconds(text):
    sign = -1 if text.startswith("-") else 1
    parts = [int(x) for x in text.lstrip("+-").split(":")] + [0, 0]
    return sign * (parts[0] * 3600 + parts[1] * 60 + parts[2])


def parse_rule(rule):
    """A POSIX `TZ` rule as `std.time`'s `parse_rule` reads it."""
    name = r"(<[^>]*>|[A-Za-z]+)"
    change = r"(M[\d.]+(?:/[-+]?[\d:]+)?)"
    m = re.fullmatch(name + r"([-+]?[\d:]+)(?:" + name + r"([-+]?[\d:]+)?," + change + "," + change + ")?", rule)
    if not m:
        sys.exit("tzdata: the rule {!r} is not one std.time reads".format(rule))
    std, std_off = m.group(1).strip("<>"), -seconds(m.group(2))
    if not m.group(3):
        return std, std_off, None

    def parsed(c):
        day, _, at = c.partition("/")
        month, week, weekday = [int(x) for x in day[1:].split(".")]
        return month, week, weekday, seconds(at) if at else 7200

    dst = m.group(3).strip("<>")
    dst_off = -seconds(m.group(4)) if m.group(4) else std_off + 3600
    return std, std_off, (dst, dst_off, parsed(m.group(5)), parsed(m.group(6)))


def change_seconds(year, c):
    month, week, weekday, at = c
    first = calendar.timegm((year, month, 1, 0, 0, 0)) // 86400
    first_weekday = (first % 7 + 4) % 7
    day = 1 + (weekday - first_weekday + 7) % 7 + (week - 1) * 7
    while day > calendar.monthrange(year, month)[1]:
        day -= 7
    return (first + day - 1) * 86400 + at


def rule_at(rule, t):
    """The offset and abbreviation `rule` gives at `t`, as `Zone` does."""
    std, std_off, daylight = rule
    if daylight is None:
        return std_off, std
    dst, dst_off, start, end = daylight
    year = (calendar.datetime.datetime(1970, 1, 1) + calendar.datetime.timedelta(seconds=t + std_off)).year
    on = change_seconds(year, start) - std_off
    off = change_seconds(year, end) - dst_off
    if (on <= t < off) if on < off else (t >= on or t < off):
        return dst_off, dst
    return std_off, std


def zones():
    """The zones of `ZONES` in `std/time.sisu`, with their rules."""
    text = time_path().read_text()
    start = text.index("const ZONES: list[(str, str)] = [")
    end = text.index("]\n", start)
    return re.findall(r'\("([^"]+)", "([^"]+)"\)', text[start:end])


def history(name, rule):
    """The types and the changes of `name` up to the one from which its rule
    gives every later change, or `None` for a zone without changes."""
    changes, kinds, table, footer = tzif(name)
    if footer != rule:
        sys.exit("tzdata: {}'s rule is {!r} in the database and {!r} in ZONES".format(name, footer, rule))
    if not changes:
        return None
    parsed = parse_rule(rule)

    def shown(k):
        return table[k][0], table[k][2]

    if rule_at(parsed, changes[-1]) != shown(kinds[-1]):
        sys.exit("tzdata: {}'s rule does not give its last change".format(name))
    # The first change `k` from which the rule gives every change and the
    # time between them.
    k = len(changes) - 1
    while k > 0:
        j = k - 1
        if rule_at(parsed, changes[j]) != shown(kinds[j]) or rule_at(parsed, changes[k] - 1) != shown(kinds[j]):
            break
        k = j
    return table, changes[:k + 1], kinds[:k + 1]


def table_text():
    lines, kept, changes_total = [], 0, 0
    for name, rule in zones():
        h = history(name, rule)
        if h is None:
            continue
        table, changes, kinds = h
        kept += 1
        changes_total += len(changes)
        types = " ".join("{}/{}".format(abbr, off) for off, _, abbr in table)
        shifts = " ".join("{}/{}".format(at, k) for at, k in zip(changes, kinds))
        lines.append("{}|{}|{}".format(name, types, shifts))
    out = [MARK, "\n"]
    out.append("# Written by `stage1/tools/tzdata.py` from the tz database {}, as the\n".format(version()))
    out.append("# system's TZif files under /usr/share/zoneinfo hold it (RULES.md, R206):\n")
    out.append("# do not edit it by hand.\n")
    out.append("\n")
    out.append("## The history of each zone of `ZONES` that has one, a line each:\n")
    out.append("## `NAME|ABBR/OFFSET ...|AT/K ...`, its local time types, the first its\n")
    out.append("## time before its first change, and its changes, from `AT` (seconds since\n")
    out.append("## the epoch) to the `K`th type, up to the one from which its rule gives\n")
    out.append("## every later change ({} zones, {} changes).\n".format(kept, changes_total))
    out.append("pure def zone_history() -> str:\n")
    out.append('    return """' + "\n".join(lines) + '\n"""\n')
    return "".join(out)


def made(current):
    return current.split(MARK)[0] + table_text()


def main():
    path = time_path()
    now = path.read_text()
    if sys.argv[1:] == ["--check"]:
        m = re.search(r"from the tz database (\S+), as the", now)
        if m and m.group(1) != version():
            print("tzdata: the table is of tz {}, the system's {}: not compared".format(m.group(1), version()), file=sys.stderr)
            return
        if MARK not in now or now != made(now):
            print("tzdata: {} is stale; run tools/tzdata.py".format(path), file=sys.stderr)
            sys.exit(1)
        return
    if MARK not in now:
        now = now.rstrip("\n") + "\n\n" + MARK
    path.write_text(made(now))


if __name__ == "__main__":
    main()
