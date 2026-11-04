# Modules, packages and the manifest

This page states how a program is put together: the files that are its
modules, how one module imports another and what it may see of it, the
names every module sees without an import, the package and its manifest
`sisu.toml` key by key, the lockfile, workspaces, scripts and their
inline metadata, and editions. What the `sisu` commands do with a
package is [R16](sisu-command.md#package-options).

**See also:** the tutorial's [chapter 9](../tutorial/09-word-counter.md)
(a first package) and [chapter 17](../tutorial/17-modules-and-packages.md)
(modules, libraries, features, workspaces); the proposal's [09, Modules,
packages and build](../../proposal/09-modules-packages-and-build.md) and
[13, "Editions"](../../proposal/13-evolution-and-governance.md#editions);
the compiler's [architecture notes](../../compiler/docs/architecture.md),
"Modules and packages"; [R17, Diagnostics](diagnostics.md#codes-and-their-ranges)
for the codes (E02xx for imports, E10xx for manifests).

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Modules from files and package cycles as errors are decided
(D-019), and so are visibility (D-045) and the manifest without build
scripts (D-032); the manifest as data leans (D-063).

## What a program is made of

A program is one of these:

- a single file, `sisu run main.sisu`: one module, which imports only
  the standard library;
- a script, a file whose first line is `#!/usr/bin/env sisu`
  ([Scripts and inline metadata](#scripts-and-inline-metadata));
- a package: a directory with a manifest, `sisu.toml`, and a `src/`
  tree of modules;
- a member of a workspace, a package that builds together with the
  others of its workspace ([Workspaces](#workspaces)).

Without a file, `sisu check`, `run`, `test` and `build` work on the
package of the current directory, or of the nearest directory above it
that has a `sisu.toml` ([R16](sisu-command.md#what-a-command-works-on)).

**Status.** In stage 0.

## The module tree

A package's modules mirror its `src/` tree, with no declarations that
list them (D-019):

- The root module is `src/main.sisu` for an application, which `sisu
  run` runs, or `src/lib.sisu` for a library; a package has one of the
  two (E1008 with neither, E1009 with both). The root module's name is
  the package's name.
- Every other file is a module named by its path:
  `src/geo/point.sisu` is `atlas.geo.point` in the package `atlas`.
- A directory is a module whose children are its files and
  directories. A file beside the directory with the same name,
  `src/geo.sisu` beside `src/geo/`, gives that module items of its own.
- A module's name is lowercase letters, digits and `_`, so a file
  named otherwise is E1012.

<!-- file: examples/reference-modules/atlas.pkg/sisu.toml -->
```toml
[package]
name = "atlas"
version = "0.1.0"
edition = "2027"
```

<!-- file: examples/reference-modules/atlas.pkg/src/main.sisu -->
```sisu
# The root module, `atlas`, from `src/main.sisu`.
import atlas.geo
from atlas.geo.point import Point

def main():
    let oulu = Point(65.0, 25.5)
    print("{geo.describe(oulu)}, {geo.EARTH_KM} km from the centre")
```

<!-- file: examples/reference-modules/atlas.pkg/src/geo.sisu -->
```sisu
# The module `atlas.geo`, from the file beside the directory `src/geo/`.
from atlas.geo.point import Point

pub const EARTH_KM: int = 6371

pub def describe(p: Point) -> String:
    return "{p.lat} N, {p.lon} E"
```

<!-- file: examples/reference-modules/atlas.pkg/src/geo/point.sisu -->
```sisu
# The module `atlas.geo.point`, from `src/geo/point.sisu`.
pub struct Point:
    pub lat: f64
    pub lon: f64
```

`sisu run` in the package's directory prints:

<!-- output: examples/reference-modules/atlas.out -->
```text
65.0 N, 25.5 E, 6371 km from the centre
```

Modules of one package may import each other in any pattern, cycles
included; packages may not ([Dependencies](#dependencies)).

**Status.** In stage 0 (D-019).

## Imports

An import binds names in the module that writes it:

| Import | Binds |
|---|---|
| `import atlas.geo` | `geo`, the last name of the path |
| `import atlas.geo as g` | `g` |
| `from atlas.geo.point import Point` | `Point`, an item of the module |
| `from atlas.geo import describe as show, EARTH_KM` | `show` and `EARTH_KM` |

- A path is package-absolute: it starts with the name of the program's
  own package, of a dependency, or with `std` for the standard library
  (`import std.json`). There is no `super` and no relative path.
- `import a.b` binds `b`, not `a`, unlike Python. An item is then
  written after the module's name, its type arguments included:
  `collections.Deque[int].new()`.
- There are no glob imports: `from m import *` is E0128.
- An import that names nothing is E0220 for its first name, E0203 for
  a module further along the path, and E0204 for an item a module does
  not have; one of a package that is not a dependency is E0260.
- `import` names a module and `from ... import` an item: an item named
  as a module, `import atlas.geo.point.Point`, is E0223
  (`resolve.not-a-module`), whose fix writes `from atlas.geo.point import
  Point`.
- Imports stand at the top level of a module, and the formatter sorts
  them and removes the ones nothing uses. An import inside a function
  is E0900 in stage 0.

<!-- file: examples/reference-modules/imports.files/src/main.sisu -->
```sisu
# Imports that name nothing.
import atlas.geo
import atlas.geo.point.Point
import atlas.roads
from atlas.geo import Route
from std.collections import *
import maps

def main():
    print(geo.EARTH_KM)
```

<!-- output: examples/reference-modules/imports.console -->
```console
$ sisu check
error[E0223]: `atlas.geo.point.Point` is not a module
  --> src/main.sisu:3:8
  |
3 | import atlas.geo.point.Point
  |        ^^^^^^^^^^^^^^^^^^^^^
  = fix: import the item from its module: `from atlas.geo.point import Point`

error[E0203]: there is no module `atlas.roads`
  --> src/main.sisu:4:8
  |
4 | import atlas.roads
  |        ^^^^^^^^^^^

error[E0204]: module `atlas.geo` has no item `Route`
  --> src/main.sisu:5:23
  |
5 | from atlas.geo import Route
  |                       ^^^^^

error[E0128]: Sisu has no glob imports: name what the module gives, as in `from std.collections import Deque, Heap`, or import the module and write `collections.Deque`
  --> src/main.sisu:6:29
  |
6 | from std.collections import *
  |                             ^

error[E0220]: cannot import `maps`: it names no module of this package, of a dependency in `sisu.toml`, or of the standard library
  --> src/main.sisu:7:8
  |
7 | import maps
  |        ^^^^

```

**Stage 0 differs.** 09 warns about an import nothing uses; stage 0
says nothing, and `sisu fmt` removes it.

**Status.** In stage 0 (D-019).

## Names every module sees

Every module sees, besides its own items and what it imports, the
prelude: the same fixed set of names in every module, which no import
brings
([09](../../proposal/09-modules-packages-and-build.md#modules)).

| Kind | Names |
|---|---|
| Builtin types | `int`, `uint`, the sized integers, `f64`, `f32`, `bool`, `char`, `str`, `String`, `bytes`, `list`, `dict`, `set`, `Array`, `Slice`, `Iter` ([R2](types.md#the-builtin-types)) |
| Values | `True`, `False`, `None`, `Some`, `Ok`, `Err` |
| Library types | `Option`, `Result`, `Ordering`, `Box`, `Shared`, `Weak`, `Cell`, `Mutex`, `Atomic`, `Arena`, `Key`, `Path`, `Duration`, `DateTime` |
| Capabilities | `System`, `Fs`, `Net`, `Clock`, `Rand`, `Env`, `Proc`, `Stdin` |
| Traits | `Display`, `Debug`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Default`, `Len`, `Contains`, `Copy`, `Send`, `Sync`, `Escapable`, `Num`, `Drop`, `Iterator`, `Into`, `FromLiteral`, `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg` |
| Error sets | `IoError`, `NetError`, `HttpError`, `ProcError`, `TaskError`, `ChannelError`, `RefinementError` |
| Functions | `print`, `print_err`, `len`, `range`, `enumerate`, `zip`, `zip_shortest`, `reversed`, `sorted`, `min`, `max`, `sum`, `any`, `all`, `abs`, `panic`, `unreachable`, `todo`, `channel` |
| Modules | `task`, `log`, `cap` and `live`, by name, as if imported |

A module may declare an item with a prelude name, which then hides the
prelude's in that module; an error set that does is W0016, since the two
sets differ. The standard library's modules other than the four in the
last row are imported, as `import std.math`; in stage 0 some also have a
short name, `import json` for `import std.json`.

**Stage 0 differs.** Stage 0's prelude is wider than the list of 09: it
also holds the library's helpers and types that its own code uses, such
as `Formatter`, `Hasher` and `days_in_month`, which a program can name
too, while those marked internal to the library are E0701.

**Status.** In stage 0, with the difference noted.

## Visibility

An item, a field and a method are private to the module that declares
them, unless they say otherwise (D-045):

| Declared | Seen by |
|---|---|
| (nothing) | the module that declares it, and no other, its own child modules included |
| `pub(package)` | every module of the same package |
| `pub` | every module that imports it, in any package |

- A use from where an item is not seen is E0255, and of a
  `pub(package)` item from another package E0256.
- No other name stands between the parentheses: Rust's `pub(crate)` is
  E0130, whose fix writes `pub(package)`
  ([R19](false-friends.md#pubcrate)).
- A field is E0258 and a method E0259 in the same way. A struct with a
  private field can be built only where that field is seen, since its
  construction names every field.
- A module itself is always reachable by its path; its items are what
  visibility controls.

<!-- file: examples/reference-modules/private.files/src/geo/point.sisu -->
```sisu
# A struct with a private field and a private method.
pub struct Point:
    pub lat: f64
    lon: f64

    def secret(self) -> int:
        return 42
```

<!-- file: examples/reference-modules/private.files/src/main.sisu -->
```sisu
# Private items of another module.
from atlas.geo.point import Point

def main():
    let p = Point(65.0, 25.5)
    print("{p.lat} {p.secret()}")
```

<!-- output: examples/reference-modules/private.console -->
```console
$ sisu run
error[E0258]: the field `lon` of `Point` is private to the module `atlas.geo.point`; declare it `pub` to use it from another module
  --> src/main.sisu:5:13
  |
5 |     let p = Point(65.0, 25.5)
  |             ^^^^^^^^^^^^^^^^^

error[E0259]: the method `secret` of `atlas.geo.point.Point` is private to the module `atlas.geo.point`; declare it `pub` to call it from another module
  --> src/main.sisu:6:23
  |
6 |     print("{p.lat} {p.secret()}")
  |                       ^^^^^^

error[D0006]: the program was not run: errors found (2)
```

**Status.** In stage 0 (D-045).

## Re-exports

A name that a module imported is visible through it only when the
import is `pub`: `pub from sauna.comfort import feel` in the root module
of `sauna` lets its users write `sauna.feel`. Reaching a name that a
module imported without `pub`, as `geo.Point`, is E0257, whose message
names the module that declares it and the `pub from` that would
re-export it. A package's public API is what its root module declares
and re-exports.

**Stage 0 differs.** `from sauna import feel`, naming a re-exported
item in a `from` import, is E0204 in both front ends; `import sauna`
with `sauna.feel(...)` works.

**Status.** In stage 0, with the difference noted.

## The manifest

`sisu.toml` is TOML, and it is data: no conditions, templates or
expressions (D-063). A mistake is a diagnostic at its place in the file,
and an unknown key is W0005, a warning, and is ignored. The tables:

### `[package]`

| Key | Takes | Notes |
|---|---|---|
| `name` | the package's name, in lowercase letters, digits and `_` | required; E1004 otherwise, [E1017](diagnostics.md#e1017) for a standard-library module's name |
| `version` | `"MAJOR.MINOR.PATCH"` | required |
| `edition` | a year, as `"2027"` | [Editions](#editions) |
| `description`, `license` | text | recorded for the registry |
| `unsafe` | `"memory"` or `"extern"`: the unsafe code the package holds | D-082; [`[trust]`](#trust) |
| `authors`, `repository`, `readme`, `keywords` | data for the registry | accepted and kept for later |
| `profile` | the teaching profile | [The teaching profile](#the-teaching-profile) |

### `[dependencies]` and `[dev-dependencies]`

Each key is a dependency's name, and its value a table:

| Key | Takes |
|---|---|
| `path` | the directory of the dependency's package, relative to this manifest |
| `features` | the dependency's features to turn on ([Features](#features)) |
| `default-features` | `false` to leave out the dependency's `default` features |
| `when` | a condition on the build target ([Target conditions](#target-conditions)) |
| `version`, `git`, `rev`, `branch`, `tag`, `optional` | accepted and ignored until there is a registry |

A dependency is named by its own `[package] name` (E1006 for another
key), its directory has a manifest (E1005), and a dev-dependency is seen
only by the package's tests. `NAME = "1.2"`, a version range alone, is
E1010 in stage 0 ([The registry](#the-registry)).

### `[features]`

Each key is a feature, and its value the list of the features it turns
on, its own or a dependency's as `"DEP/FEATURE"`; `default` lists those
on unless a build says otherwise ([Features](#features)).

### `[trust]`

`unsafe = ["NAME", ...]` lists the dependencies that may contain
`unsafe` and `extern` code; every name must be a dependency (E1015). A
dependency with such code that the root package does not list is E0703,
or E1021 when it declares its level; a package that holds unsafe code
declares `unsafe` in its `[package]` (E1019, E1020), and a level that
rises from the one `sisu.lock` records needs `--accept-unsafe` (E1022),
which the commands that build a package take and which records the new
level ([R16](sisu-command.md#package-options)).
This is where a dependency gains more than the capabilities it is
handed (D-032, D-082).

### `[workspace]`

`members = ["DIR", ...]` lists the packages of a workspace
([Workspaces](#workspaces)).

### `[profile.release]`

`contracts = "off"`, `"preconditions"` or `"all"` sets which contracts a
release build checks ([R11](contracts.md#check-levels));
`[profile.debug]` takes the key too.

### Mistakes in the manifest

A manifest that is not TOML is E1000, and a key given twice E1001; a
missing key is E1002, a value of the wrong kind E1003. This manifest
holds several mistakes, which `sisu check` reports together:

<!-- file: examples/reference-modules/bad_manifest.files/sisu.toml -->
```toml
[package]
name = "stove"
version = "1.0"
edition = "2027"
colour = "black"

[dependencies]
json = "2.1"
stones = "0.4"

[trust]
unsafe = ["zlib"]
```

<!-- output: examples/reference-modules/bad_manifest.console -->
```console
$ sisu check
error[E1003]: `version` takes a version `MAJOR.MINOR.PATCH`, as in `"0.3.1"`, found `"1.0"`
  --> sisu.toml:3:11
  |
3 | version = "1.0"
  |           ^^^^^

warning[W0005]: `colour` is not a key of `[package]` that stage 0 knows; it is ignored
  --> sisu.toml:5:1
  |
5 | colour = "black"
  | ^^^^^^

error[E1017]: `json` is the name of a standard-library module, which `import json` names; a package of the program needs another name
  --> sisu.toml:8:1
  |
8 | json = "2.1"
  | ^^^^

error[E1010]: the dependency `json` is not a `path` dependency; stage 0 has no registry, so a dependency is `json = { path = "../json" }`
  --> sisu.toml:8:8
  |
8 | json = "2.1"
  |        ^^^^^

error[E1010]: the dependency `stones` is not a `path` dependency; stage 0 has no registry, so a dependency is `stones = { path = "../stones" }`
  --> sisu.toml:9:10
  |
9 | stones = "0.4"
  |          ^^^^^

error[E1015]: `[trust] unsafe` names `zlib`, which is not a dependency of this package
  --> sisu.toml:12:11
   |
12 | unsafe = ["zlib"]
   |           ^^^^^^

```

`json` is the name of a standard-library module, so no package of the
program may take it, and neither dependency is a `path`.

**Stage 0 differs.** A dev-dependency can be imported by the package's
own code, not only by its tests. The `[review]` table of
[10](../../proposal/10-tooling.md) is accepted and ignored.

**Status.** In stage 0 (D-063), with the differences noted.

## Target conditions

A dependency that a build needs on some targets only says so with
`when`, a table over the build target's facts: `os`, `arch` and
`runtime`, each a string or a list of strings, as in `when = { os =
"windows" }` (E1011 for anything else). It is data that the build
compares with the target, not an expression (D-063). A dependency whose
condition does not hold is not part of the build, and is still recorded
in the lockfile, which is the same on every target.

**Stage 0 differs.** An import of a dependency whose condition does not
hold on this target is E0260, whose message asks to add the dependency
to `[dependencies]`, where it already is.

**Status.** In stage 0 (D-063), with the difference noted.

## Features

A feature is a named, additive part of a package: it can add items,
never take any away. `@cfg(feature="NAME")` before an item keeps the
item only in a build with the feature on.

- A package's `[features]` declares them, and `default` lists the ones
  on by default.
- A dependent turns on a dependency's features with `features = [...]`
  in its dependency's table, and leaves out the defaults with
  `default-features = false`.
- `sisu run --features NAMES` and `--no-default-features` choose for a
  build ([R16](sisu-command.md#package-options)).
- `@cfg(feature="NAME")` with a name that no `[features]` key declares
  is W0006, and turning on a feature that a dependency does not have is
  E1018.

The workspace in [Workspaces](#workspaces) turns on the feature
`fahrenheit` of its library.

**Stage 0 differs.** An item that a feature left out is E0204 at its
use, with no word about the feature.

**Status.** In stage 0.

## The lockfile

`sisu.lock`, beside the root package's manifest or its workspace's,
records every package the build resolved, sorted by name: its version,
its source and its dependencies. It is data that `sisu` writes and
rewrites whenever the manifests imply another; `--locked` refuses to
rewrite it and stops instead (D0065, [R16](sisu-command.md#package-options)).
It lists every dependency, active on this target or not, so it is the
same on every target, and it records the unsafe level each package
declares. An application commits its lockfile; a library need not. The
lockfile of the workspace in [Workspaces](#workspaces):

<!-- output: examples/reference-modules/sauna.lock -->
```toml
# Written by `sisu` from the manifests: the packages this build resolved.
# It is data; `sisu` rewrites it when a manifest changes.
version = 1

[[package]]
name = "booking"
version = "0.1.0"
source = "path+booking"
dependencies = ["sauna"]

[[package]]
name = "sauna"
version = "0.2.0"
source = "path+sauna"
dependencies = []
```

**Status.** In stage 0.

## Dependencies

- A dependency is a package that this one names in `[dependencies]`;
  only those may be imported (E0260), and a module imports one by its
  name.
- Packages build in dependency order, so the dependencies of a program
  form no cycle (E1007), while the modules of one package may import
  each other freely.
- A build has one version of each package.
- A package has no build script: what one would compute is
  compile-time code or a tool the developer runs and commits the output
  of (D-032).

<!-- output: examples/reference-modules/cycle.console -->
```console
$ sisu check stove
error[E1007]: the packages depend on each other in a cycle: `stove` -> `benches` -> `stove`; packages build in dependency order, so their graph has no cycle
  --> ../benches/sisu.toml:6:1
  |
6 | stove = { path = "../stove" }
  | ^^^^^
  ::: sisu.toml:6:1
  |
6 | benches = { path = "../benches" }
  | ------- `stove` depends on `benches` here

```

**Stage 0 differs.** Every dependency is a `path` in stage 0; versions,
`git` sources and a registry wait ([The registry](#the-registry)), and
so does linking two major versions of one package.

**Status.** In stage 0 (D-019, D-032), with paths only.

## Workspaces

A directory whose `sisu.toml` has `[workspace]` with `members` builds
its member packages together, with one lockfile beside it. Each member
is a directory with its own manifest (E1013 otherwise), and the
members' dependencies on each other are ordinary `path` dependencies. A
manifest with `[workspace]` and no `[package]` is not a package, so it
cannot be a dependency (E1016). `sisu run DIR` and `sisu test DIR`, in
the workspace's directory, work on one member.

<!-- file: examples/reference-modules/sauna.pkg/sisu.toml -->
```toml
[workspace]
members = ["sauna", "booking"]
```

<!-- file: examples/reference-modules/sauna.pkg/sauna/sisu.toml -->
```toml
[package]
name = "sauna"
version = "0.2.0"
edition = "2027"
description = "How hot a sauna is"
license = "MIT"

[features]
default = []
fahrenheit = []
```

<!-- file: examples/reference-modules/sauna.pkg/sauna/src/lib.sisu -->
```sisu
##! How hot a sauna is.
pub from sauna.comfort import feel

## The temperature of a good sauna, in degrees Celsius.
pub const GOOD: int = 80

pub(package) def round_to_ten(c: int) -> int:
    return c // 10 * 10

## `c` degrees Celsius in Fahrenheit, rounded down.
@cfg(feature="fahrenheit")
pub def to_fahrenheit(c: int) -> int:
    return c * 9 // 5 + 32
```

<!-- file: examples/reference-modules/sauna.pkg/sauna/src/comfort.sisu -->
```sisu
# The module `sauna.comfort`.

## How a sauna at `celsius` degrees feels.
pub def feel(celsius: int) -> str:
    return if celsius >= 70: "hot" else: "warm"
```

<!-- file: examples/reference-modules/sauna.pkg/booking/sisu.toml -->
```toml
[package]
name = "booking"
version = "0.1.0"
edition = "2027"

[dependencies]
sauna = { path = "../sauna", features = ["fahrenheit"] }
```

<!-- file: examples/reference-modules/sauna.pkg/booking/src/main.sisu -->
```sisu
# An application that uses the library `sauna` of its workspace.
import sauna
from sauna.comfort import feel

def main():
    let c = sauna.GOOD
    print("{c} °C is {sauna.feel(c)}, 60 °C is {feel(60)}")
    print("{c} °C is {sauna.to_fahrenheit(c)} °F")
```

`sisu run` in `booking` prints:

<!-- output: examples/reference-modules/sauna.out -->
```text
80 °C is hot, 60 °C is warm
80 °C is 176 °F
```

The library's `pub(package)` function is seen by its own modules only:

<!-- output: examples/reference-modules/package_only.console -->
```console
$ sisu run booking
error[E0256]: `round_to_ten` is `pub(package)` in `sauna`, so only the modules of the package `sauna` can use it
  --> src/main.sisu:5:17
  |
5 |     print(sauna.round_to_ten(84))
  |                 ^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0.

## Scripts and inline metadata

A file whose first line is `#!/usr/bin/env sisu` is a script. Its
top-level statements are its `main`, run in order with `sys`, the
program's `System`, in scope, so it may not declare `main` as well
(E0261). It runs as `sisu FILE.sisu ARGS`, or as `./FILE.sisu` once it
is executable (D-052; [R16](sisu-command.md#running-a-file-as-a-script)).

A script names its dependencies in a comment block after the `#!` line,
between `# /// script` and `# ///`, whose lines without their `# ` are
a manifest (Python's PEP 723 shape); a block that never ends is E1014.

<!-- file: examples/reference-modules/heat.sisu -->
```sisu
#!/usr/bin/env sisu
# /// script
# [dependencies]
# sauna = { path = "sauna" }
# ///
# A script that names its dependency in its inline metadata.
import sauna

for arg in sys.env.args():
    let celsius = arg.parse_int() ?? 0
    print("{celsius} °C is {sauna.feel(celsius)}")
```

<!-- output: examples/reference-modules/heat.console -->
```console
$ sisu heat.sisu 80 60
80 °C is hot
60 °C is warm
```

The library it names lives in the directory `sauna` beside it:

<!-- file: examples/reference-modules/heat.files/sauna/src/lib.sisu -->
```sisu
##! How hot a sauna is.

## How a sauna at `celsius` degrees feels.
pub def feel(celsius: int) -> str:
    return if celsius >= 70: "hot" else: "warm"
```

**Status.** In stage 0 (D-052).

## Editions

A package declares its edition, a year, in `[package] edition`. Every
three years a new edition may change syntax, defaults, lints and the
prelude in ways that are not compatible, and packages of different
editions link together, because the compiler understands every edition
and they share one type system and ABI; an edition never changes the
meaning of a type or a trait that other editions see (D-036). `sisu
fix --edition` migrates a package to the next one. A false friend found
after 1.0 joins the table of [R19](false-friends.md) through an
edition, since the check that makes it loud can stop code that compiled.
Between editions, a feature that is not stable yet is behind `@preview`,
and each preview is stabilised or removed within four releases (D-072).

**Stage 0 differs.** Stage 0 knows one edition: it checks that
`edition` is a year and gives every year the same language. `sisu fix`
and `@preview` are designed ([R16](sisu-command.md#sisu-fix)).

**Status.** Designed (D-036; D-072 leans).

## The registry

A dependency named by a version range, `http = "1.2"`, comes from the
registry of D-031: content-addressed and immutable, with a public
transparency log, namespaced by owner (`roni/weather`), and with
semver checked against the public API's diff at `sisu publish`, and
against the previous version's black-box tests. Each version shows its
provenance and the capabilities its API takes, and a package that takes
none and is not in `[trust]` is labelled pure (D-066). Adding or
updating a dependency reports the authority it gains, and admission
waits out a cooldown and refuses new packages in unattended runs.

**Stage 0 differs.** There is no registry, so a dependency is a `path`
(E1010), and `sisu add`, `update` and `publish` are designed
([R16](sisu-command.md#commands-that-are-designed)).

**Status.** Designed (D-031; D-066 leans).

## The teaching profile

`profile = "teaching-3"` in `[package]` confines a package to a level
of the teaching profile: a ladder of levels, each a strict subset of the
next and of the whole language, so a program stays valid as a learner
climbs, and a construct from a later level gets a diagnostic naming its
level ([10](../../proposal/10-tooling.md); Q-41). Teachers define their
own ladders as data.

**Stage 0 differs.** Stage 0 accepts the key and ignores it.

**Status.** Leaning (D-073).

## C headers and sources

`import c "zlib.h" as zlib with link="z"` reads a C header's
declarations into a module, and `extern "C":` declares C functions by
hand ([R3](declarations.md#foreign-functions)); both are allowed only
in a package that the root package trusts (E0703). There is no direct
C++ interop (Q-24).

**Stage 0 differs.** A C header import is E0900 (`stage0.unsupported`).

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md),
[22](../../proposal/22-unsafe.md)).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| E1008, E1009, E1012 | `package.no-root-module`, `package.two-root-modules`, `package.module-name` | [The module tree](#the-module-tree) |
| [E0100](diagnostics.md#e0100), [E0203](diagnostics.md#e0203), [E0204](diagnostics.md#e0204), [E0220](diagnostics.md#e0220), [E0223](diagnostics.md#e0223), E0260 | `parse.expected`, `resolve.unknown-module`, `resolve.not-in-module`, `resolve.import-unknown`, `resolve.not-a-module`, `resolve.not-a-dependency` | [Imports](#imports) |
| W0016, E0701 | `resolve.shadow-prelude-set`, `cap.internal` | [Names every module sees](#names-every-module-sees) |
| [E0255](diagnostics.md#e0255), [E0256](diagnostics.md#e0256), [E0258](diagnostics.md#e0258), [E0259](diagnostics.md#e0259), [E0130](diagnostics.md#e0130) | `resolve.private-item`, `resolve.package-item`, `resolve.private-field`, `resolve.private-method`, `parse.visibility-scope` | [Visibility](#visibility) |
| E0257 | `resolve.not-reexported` | [Re-exports](#re-exports) |
| E1000, E1001, [E1002](diagnostics.md#e1002), [E1003](diagnostics.md#e1003), E1004, E1005, E1006, [E1010](diagnostics.md#e1010), [E1017](diagnostics.md#e1017), [W0005](diagnostics.md#w0005) | `package.toml-syntax`, `package.toml-duplicate`, `package.missing-key`, `package.bad-value`, `package.bad-name`, `package.no-manifest`, `package.renamed-dependency`, `package.not-a-path`, `package.reserved-name`, `package.unknown-key` | [The manifest](#the-manifest) |
| [E1015](diagnostics.md#e1015), [E0703](diagnostics.md#e0703), [E1019](diagnostics.md#e1019), E1020, [E1021](diagnostics.md#e1021), E1022 | `package.trust-unknown`, `cap.untrusted`, `package.unsafe-undeclared`, `package.unsafe-level`, `package.unsafe-untrusted`, `package.unsafe-raised` | [`[trust]`](#trust) |
| E1011 | `package.bad-when` | [Target conditions](#target-conditions) |
| W0006, E1018 | `resolve.unknown-feature`, `package.unknown-feature` | [Features](#features) |
| [E1007](diagnostics.md#e1007) | `package.cycle` | [Dependencies](#dependencies) |
| E1013, E1016 | `package.no-member`, `package.no-package` | [Workspaces](#workspaces) |
| E0261, E1014 | `resolve.script-main`, `package.script-metadata` | [Scripts and inline metadata](#scripts-and-inline-metadata) |
| D0065 | `cli.locked` | [The lockfile](#the-lockfile) |
