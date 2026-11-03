# Embed a file at compile time

This guide makes a file's text part of a program while it compiles, so
that the program needs no file system to read it, and says what the
build records about the file. It assumes packages and constants, from
chapters 2 and 17 of the tutorial.

**See also:** the tutorial's
[chapter 25](../tutorial/25-compile-time.md#a-file-in-the-program) (a
first `embed`); [R16](../reference/sisu-command.md) for the commands;
the proposal's [08, Metaprogramming](../../proposal/08-metaprogramming.md),
whose rule is that compile-time code does no I/O but `embed`.

## 1. Put the file beside the module, and embed it

`embed("path")` is the text of a file, read when the program is
compiled, and a `const` keeps it under a name. The path is relative to
the directory of the module that embeds it, so in a package the file
lives under `src/`. This package keeps a sauna's rules in
`src/rules.txt`, one on each line:

<!-- file: examples/guide-embed/rules.pkg/src/rules.txt -->
```text
Shower before the sauna.
Sit on a towel.
Ask before you throw water on the stones.
Cool off in the lake.
```

<!-- file: examples/guide-embed/rules.pkg/src/main.sisu -->
```sisu
##! The sauna's rules, kept in a file of their own and made part of the
##! program while it compiles.

## The rules, one on each line, from the file beside this module.
const RULES: str = embed("rules.txt")

## Rule number `n`, counting from 1, if there is one.
def rule(n: int) -> str?:
    var i = 0
    for line in RULES.lines():
        i += 1
        if i == n:
            return Some(line)
    return None

def main(sys: System):
    for arg in sys.env.args():
        let n = arg.parse_int() ?? 0
        print("{n}: {rule(n) ?? "there is no such rule"}")

test "the first rule, and one past the last":
    assert rule(1) == Some("Shower before the sauna.")
    assert rule(5) == None
```

## 2. Use the text as any constant

`RULES` is a `str` like any other, and the program reads it with no
`Fs` and no error to handle, since the text was there when it compiled.
Its tests read the same text, with no fixture:

<!-- output: examples/guide-embed/rules.console -->
```console
$ sisu run -- 1 3 9
1: Shower before the sauna.
3: Ask before you throw water on the stones.
9: there is no such rule
$ sisu test
test "the first rule, and one past the last" ... ok
1 passed, 0 failed, 0 blocked
```

Embed what belongs to the program, such as help texts, templates and
tables, and read through `Fs` what belongs to its user, such as their
settings and data: an embedded file changes only when the program is
built again.

## 3. Keep within what embed reads

The path is a string literal, so that the compiler knows it without
running anything, and it must name a UTF-8 text file inside the
program's directory. Anything else is refused while compiling:

<!-- file: examples/guide-embed/paths.sisu -->
```sisu
# The paths that `embed` refuses: outside the program, missing, computed.

def main():
    print(embed("../notes.txt"))
    print(embed("missing.txt"))
    let path = "rules.txt"
    print(embed(path))
```

<!-- output: examples/guide-embed/paths.console -->
```console
$ sisu check paths.sisu
error[E0612]: cannot embed `../notes.txt`: the path must stay inside the program's directory
  --> paths.sisu:4:11
  |
4 |     print(embed("../notes.txt"))
  |           ^^^^^^^^^^^^^^^^^^^^^

error[E0612]: cannot embed `missing.txt`: there is no such file
  --> paths.sisu:5:11
  |
5 |     print(embed("missing.txt"))
  |           ^^^^^^^^^^^^^^^^^^^^

error[E0613]: `embed` takes the path of a file as a string literal
  --> paths.sisu:7:11
  |
7 |     print(embed(path))
  |           ^^^^^^^^^^^

```

E0612 (`comptime.embed-failed`) gives the reason, which is the same on
every machine, and E0613 (`comptime.embed-path`) asks for a literal.

## 4. Know what the build records

Compile-time code does no I/O, and `embed` is the one input a build
reads besides the source. The compiler reads each embedded file when it
checks the program and hashes it into the program's interface, so the
same source and the same files give the same program, and a change of
the file is a change of the program: it is built again, and its tests
run again. The text travels inside the program, into a native
executable and into the log of `sisu run --record`, so neither needs
the file where it runs.

Keep embedded files under version control with the source: they are
source.

**Status.** In stage 0, with three differences from
[08](../../proposal/08-metaprogramming.md): the path is relative to the
module's directory and must stay inside it, where 08 resolves it from
the package's root; a replay token's content hash covers the source and
not the files it embeds, so a token made before a file changed is still
accepted after; and stage 1's compile-time evaluator cannot yet run the
string methods or the `dict` that would parse the text into a table
while compiling, so a program reads the text when it runs, as `rule`
does.
