# Match text with regular expressions

This guide compiles a pattern, finds matches and takes them apart by
their groups, replaces and splits text, and works around what the
pattern language leaves out to keep every match linear in time. It
assumes text and errors as values, from chapters 6 and 8 of the
tutorial.

**See also:** the tutorial's
[chapter 21](../tutorial/21-standard-library.md#regular-expressions)
(`std.regex`); the source of `std.regex`,
[`compiler/std/regex.sisu`](../../compiler/std/regex.sisu), whose first
comment gives the whole syntax; the proposal's
[11, Standard library](../../proposal/11-stdlib.md).

## 1. Compile the pattern once

`Regex.new(pattern)` compiles a pattern and gives `Regex !
RegexError`. Write the pattern as a raw literal, `r"..."`, so that `\d`
needs no second backslash and a brace is not interpolation. Compile it
once, outside the loop that uses it. A pattern that does not compile is
`Syntax(at, message)`, with the index in the pattern where the problem
is, so a pattern that a user typed can be reported. This program
compiles four patterns and prints what became of each:

<!-- file: examples/guide-regex/patterns.sisu -->
```sisu
# A pattern that does not compile is an error that says where.
from std.regex import Regex

def main():
    for pattern in [r"\d+:\d\d", r"(\d+:\d\d", r"(\w+) \1", r"(?i)sauna"]:
        match Regex.new(pattern):
            case Ok(re):
                print("{pattern}: {re.groups()} groups")
            case Err(Syntax(at, message)):
                print("{pattern}: at {at}, {message}")
```

<!-- output: examples/guide-regex/patterns.out -->
```text
\d+:\d\d: 0 groups
(\d+:\d\d: at 0, an unclosed group
(\w+) \1: at 6, the back-reference `\1`, which a match in linear time leaves out
(?i)sauna: at 2, a flag such as `(?i)`, which stage 0 leaves out: `(?:...)` is the one group flag
```

The last two are features that the engine leaves out, as step 4 shows,
and the message says so: a back-reference for ever, since no match in
linear time can have it, and a flag in stage 0, which has `(?:...)`
alone.

## 2. Find matches and read their groups

`find(text)` gives the leftmost match, `Match?`, and `find_all(text)`
every match, left to right, none overlapping. A match has `text`,
`start` and `end`, and `group(i)` gives the text of group `i`, counted
from 1 by its opening parenthesis, as a `String?`: `None` for a group
that took no part. This program finds each turn of a sauna's evening:

<!-- file: examples/guide-regex/turns.sisu -->
```sisu
# Finds every turn in a text, and takes each match apart by its groups.
from std.regex import Regex, RegexError

def main() -> ! RegexError:
    let turn = Regex.new(r"(\d{1,2}):(\d\d) (\w+)")?
    let text = "17:30 Aino, 18:05 Eero and 9:15 Roni"
    for m in turn.find_all(text):
        let hour = m.group(1) ?? ""
        let name = m.group(3) ?? ""
        print("{name} at {hour}, bytes {m.start} to {m.end}")
    if let Some(first) = turn.find(text):
        print("the first: {first.text}")
    return Ok(())
```

<!-- output: examples/guide-regex/turns.out -->
```text
Aino at 17, bytes 0 to 10
Eero at 18, bytes 12 to 22
Roni at 9, bytes 27 to 36
the first: 17:30 Aino
```

`start` and `end` are byte indices into the text, so they fit
`byte_slice` and not a count of characters. `is_match(text)` asks
whether there is a match at all.

## 3. Replace and split

`replace_all(text, replacement)` replaces every match; in the
replacement, `$0` is the match, `$1` to `$9` its groups and `$$` a
dollar sign. `split(text)` gives the parts between the matches. This
program rewrites times with a full stop and splits a list of names
whatever the spaces around its commas:

<!-- file: examples/guide-regex/replace.sisu -->
```sisu
# Rewrites each time in a text, and splits a list at its commas.
from std.regex import Regex, RegexError

def main() -> ! RegexError:
    let time = Regex.new(r"(\d{1,2}):(\d\d)")?
    print(time.replace_all("sauna 17:30, swim 18:05", "$1.$2"))
    let comma = Regex.new(r"\s*,\s*")?
    print(comma.split("Aino , Eero,Roni ,  Ella"))
    return Ok(())
```

<!-- output: examples/guide-regex/replace.out -->
```text
sauna 17.30, swim 18.05
["Aino", "Eero", "Roni", "Ella"]
```

## 4. Do in code what a pattern cannot say

The engine has no back-references (`\1`), no look-ahead or look-behind
(`(?=...)`, `(?<=...)`), no named groups and no flags such as `(?i)`.
Back-references cannot be matched in linear time, so they are left out
on purpose, and look-around with them, as in RE2. Match the parts with
a pattern and compare them in code instead. This program finds
doubled words, which Python would find with `(\w+) \1`:

<!-- file: examples/guide-regex/doubled.sisu -->
```sisu
# Finds doubled words in code, since a pattern has no back-references.
from std.regex import Regex, RegexError

def main() -> ! RegexError:
    let word = Regex.new(r"\w+")?
    let text = "the sauna is is warm, and the lake is cold cold"
    let words = word.find_all(text)
    for i in range(1, len(words)):
        if words[i].text == words[i - 1].text:
            print("{words[i].text} twice, from byte {words[i - 1].start}")
    return Ok(())
```

<!-- output: examples/guide-regex/doubled.out -->
```text
is twice, from byte 10
cold twice, from byte 38
```

For a match without regard to case, lower the text first with
`to_lowercase()` and write the pattern in lower case.

## 5. Match untrusted text and patterns safely

Every match takes time linear in the length of the text, whatever the
pattern, since the engine runs every alternative at once rather than
trying one and backing up. A nested repetition such as `^(a+)+$` makes
a backtracking engine, such as Python's `re`, take time that doubles
with each `a` of a text that does not match; here it does not:

<!-- file: examples/guide-regex/linear.sisu -->
```sisu
# A nested repetition, which makes a backtracking engine take time that
# doubles with each character, matches here in time linear in the text.
from std.regex import Regex, RegexError

def main() -> ! RegexError:
    let nested = Regex.new(r"^(a+)+$")?
    for n in [10, 100, 1000]:
        let text = "{"a".repeat(n)}!"
        print("{n} a's and a !: {nested.is_match(text)}")
    return Ok(())
```

<!-- output: examples/guide-regex/linear.out -->
```text
10 a's and a !: False
100 a's and a !: False
1000 a's and a !: False
```

So a server can match a pattern that its users write against text that
its users send, and neither can make it hang (ReDoS). A pattern's
length still costs: a counted repetition such as `\d{1,1000}` grows the
compiled program, so a count above 1000 is a `Syntax` error.

**Status.** In stage 0, as [11](../../proposal/11-stdlib.md) plans
it: a linear-time engine of RE2's kind. Of RE2's syntax it has the core
that its first comment lists, without named groups, flags or Unicode
classes such as `\p{L}`.
