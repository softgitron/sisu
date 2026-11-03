# Write a script with dependencies

This guide writes a one-file program that runs as a command, without a
package around it, and that uses a library by naming it in the file
itself. It assumes the packages of
[chapter 17](../tutorial/17-modules-and-packages.md) of the tutorial.

**See also:** [R13, Modules](../reference/modules.md#scripts-and-inline-metadata)
for scripts and their inline metadata;
[R16](../reference/sisu-command.md#running-a-file-as-a-script) for
running one; the tutorial's [chapter 1](../tutorial/01-hello.md) (a
first script); the proposal's
[10, Tooling](../../proposal/10-tooling.md), "REPL and scripts", and
D-052.

## 1. Start the file with a shebang line

A file whose first line is `#!/usr/bin/env sisu` is a script: its
top-level statements are its `main`, run in order, with `sys`, the
program's `System`, in scope. It may declare functions and types, but
not a `main` of its own (E0261). This script says how long a stove
takes to heat each sauna that `saunas.txt` lists, for a stove of the
kilowatts it is given:

<!-- file: examples/guide-script/heat.sisu -->
```sisu
#!/usr/bin/env sisu
# /// script
# [dependencies]
# stove = { path = "stove" }
# ///
# How long the stove takes to heat each sauna in `saunas.txt`.
import stove

## The size in cubic metres that a line `NAME SIZE` gives, or 0.
def size(line: str) -> int:
    let (_, cubic) = line.split_once(" ") ?? ("", "0")
    return cubic.parse_int() ?? 0

let kilowatts = (sys.env.args().first() ?? "9").parse_int() ?? 9
let text = sys.fs.read_to_string("saunas.txt")?
for line in text.lines():
    let minutes = stove.heating_minutes(kilowatts, size(line))
    print("{line}: {minutes} minutes")
```

- `?` works at the top level as in a `main` that returns an error: a
  missing `saunas.txt` ends the script with the error and status 1.
- `sys.env.args()` holds the words after the script's name.

## 2. Name the dependencies in the file

A comment block after the `#!` line, from `# /// script` to `# ///`, is
the script's manifest: its lines without their `# ` are what a package's
`sisu.toml` would say, here a `[dependencies]` table. A dependency is a
path to a package, relative to the script. The library `stove` lives in
the directory `stove` beside it, with a manifest and a `src/lib.sisu`:

<!-- file: examples/guide-script/heat.files/stove/sisu.toml -->
```toml
[package]
name = "stove"
version = "0.1.0"
edition = "2027"

[dependencies]
```

<!-- file: examples/guide-script/heat.files/stove/src/lib.sisu -->
```sisu
##! A sauna stove's arithmetic.

## The minutes a stove of `kilowatts` takes to heat a sauna of
## `cubic_metres` from 20 to 80 degrees.
pub def heating_minutes(kilowatts: int, cubic_metres: int) -> int:
    return 60 * cubic_metres // kilowatts
```

A block that is not closed with `# ///` is E1014. The script's imports
and the library's `pub` items work as in a package.

## 3. Run it

`sisu FILE ARGS` runs a script, as `sisu run FILE ARGS` does; once the
file is executable (`chmod +x heat.sisu`), `./heat.sisu ARGS` runs it
too, through the shebang line, wherever `sisu` is on the `PATH`. With
two saunas in `saunas.txt`:

<!-- output: examples/guide-script/heat.console -->
```console
$ sisu heat.sisu 9
Kota 10: 66 minutes
Ranta 14: 93 minutes
$ sisu run heat.sisu 12
Kota 10: 50 minutes
Ranta 14: 70 minutes
```

## 4. Know where the dependencies can come from

Stage 0 has no registry, so a dependency is a path. A version, as a
published package would be named, is E1010:

<!-- output: examples/guide-script/registry.console -->
```console
$ sisu registry.sisu
error[E0220]: cannot import `stove`: it names no module of this package, of a dependency in `sisu.toml`, or of the standard library
  --> registry.sisu:7:8
  |
7 | import stove
  |        ^^^^^

error[E1010]: the dependency `stove` is not a `path` dependency; stage 0 has no registry, so a dependency is `stove = { path = "../stove" }`
  --> registry.sisu:4:11
  |
4 |   stove = "1.2"
  |           ^^^^^

error[D0006]: the program was not run: errors found (2)
```

E1010 (`package.not-a-path`) says what to write instead. E0220, which
comes first, is the import that the missing dependency left without a
module; it names `sisu.toml`, where a script's dependencies are in its
inline metadata.

A script that grows past one file, or that others will depend on,
becomes a package: `sisu new NAME` makes one, and the inline table moves
to its `sisu.toml`.

**Status.** Scripts and inline metadata with path dependencies are in
stage 0 (D-052). Dependencies by version, from the registry of D-031,
are designed and wait for the registry.
