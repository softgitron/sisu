# 17. Modules and packages

In this chapter a program grows past one file. Each file of a package
is a module, each item is private to its module until it is `pub`, and
`import` reaches the items of other modules. A package can use another
as a library, choose its optional parts with features, and share a
workspace with it. Last, you document a library for its users and take
a first look at publishing one. Challenge III closes Part III. You need
about two hours.

## A module for each file

Chapter 9 made a package with one module, `src/main.sisu`. Every other
file under `src/` is a module too, named by its path, and a directory
groups modules. A package that works out the fuel for a trip has three:

<!-- file: examples/17-modules-and-packages/trip.pkg/src/main.sisu -->
```sisu
# The root module imports the other modules by the package's name.
import trip.distance
from trip.fuel.cost import trip_cost

def main():
    let km = distance.between("Turku", "Tampere")
    print("{km} km, {trip_cost(km, 6.5, 1.95):.2f} € of fuel")
```

<!-- file: examples/17-modules-and-packages/trip.pkg/src/distance.sisu -->
```sisu
# The module `trip.distance`, from the file `src/distance.sisu`.

## The road distance in kilometres between two towns, or 0 when it is
## not known.
pub def between(a: str, b: str) -> int:
    for x, y, km in table():
        if (x == a and y == b) or (x == b and y == a):
            return km
    return 0

## The known distances.
def table() -> list[(str, str, int)]:
    return [("Turku", "Tampere", 160), ("Tampere", "Oulu", 490)]
```

<!-- file: examples/17-modules-and-packages/trip.pkg/src/fuel/cost.sisu -->
```sisu
# The module `trip.fuel.cost`, from the file `src/fuel/cost.sisu`.

## The price of the fuel for `km` kilometres, for a car that uses
## `litres` litres for 100 kilometres, at `price` euros a litre.
pub def trip_cost(km: int, litres: f64, price: f64) -> f64:
    return km.to_f64() / 100.0 * litres * price
```

<!-- output: examples/17-modules-and-packages/trip.console -->
```console
$ sisu run
160 km, 20.28 € of fuel
```

- `src/main.sisu` is the root module, named by the package: `trip`.
  `src/distance.sisu` is the module `trip.distance`, and
  `src/fuel/cost.sisu`, in the directory `fuel`, is `trip.fuel.cost`.
  No declaration lists the modules: the files are the list (D-019).
- `import trip.distance` binds the name `distance`, and the code calls
  `distance.between(...)`. The module's name says where each function
  comes from.
- `from trip.fuel.cost import trip_cost` binds the function itself.
  `as` gives an imported name another one: `from trip.fuel.cost import
  trip_cost as cost`.

A package whose root module is `src/lib.sisu` instead is a library: it
has no `main`, and other packages use its items.

Only a package makes several files one program. A file run alone is a
module of its own, so `sauna.sisu`, which imports `stove` from the file
`stove.sisu` beside it, does not compile:

<!-- output: examples/17-modules-and-packages/beside.console -->
```console
$ sisu run sauna.sisu
error[E0220]: cannot import `stove`: it names no module of this package, of a dependency in `sisu.toml`, or of the standard library
  --> sauna.sisu:2:8
  |
2 | import stove
  |        ^^^^^
  = note: `stove` beside this file is a module of a package only: a package is a directory with a `sisu.toml` and the files in `src/`, which makes them one program (09)

error[D0006]: the program was not run: errors found (1)
```

E0220's note says why. Move both files into the `src/` of a package,
which `sisu new` makes, and import the module by the package's name,
as `import trip.distance` does. A module in a package is never alone:
`sisu test src/distance.sisu` compiles the whole package and runs the
tests of that one module.

## Private unless pub

An item is private to its module unless it is declared `pub` (D-045).
`between` is `pub`, and `table`, the list it searches, is not.

**Try it.** In `main`, add a line that prints how many distances
`distance.table()` holds.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/17-modules-and-packages/private.console -->
```console
$ sisu run
error[E0255]: `table` is private to the module `trip.distance`; declare it `pub` to use it from another module
  --> src/main.sisu:8:26
  |
8 |     print("{len(distance.table())} known distances")
  |                          ^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0255 (`resolve.private-item`) says that `table` is private, and that
`pub` would let other modules use it. Before you add it, decide whether
the list belongs to what the module offers, or is a detail that may
change. Everything `pub` is a promise to the module's users.

</details>

A struct's fields and methods are private in the same way: `pub` on a
field or a method makes it visible where the type is (E0258, E0259).
`pub(package)` is visible to every module of the same package and to
no other package (E0256), which suits a helper that two modules share
but a library's users should not see.

## What an import can name

An import names a module by its path: a module of the same package by
the package's name, a module of a library by the library's name, and a
module of the standard library by `std` and its name, as in `import
std.json` or `from std.collections import Deque`. A few names need no
import at all: the prelude's fixed list, which holds `print`, `len`,
`range`, `sorted`, the basic types and collections, `Option`, `Result`
and the core traits such as `Display` and `Iterator`
([R13](../reference/modules.md#names-every-module-sees) lists them).
The prelude declares helpers of its own too, for the standard library's
code, and a program that names one, such as `days_in_month`, is E0278
(`resolve.prelude-internal`). Anything else is
imported by name, so each file says where its names come from. An
import that the file does not use is no error, and `sisu fmt` keeps
it; `sisu lint` names it, W0027 (`lint.unused-import`), with a fix that
removes it ([R16](../reference/sisu-command.md#sisu-lint)). Imports
stand at the top of a file, before its declarations, and their names
are the whole module's: an import inside a function's body, which
Python allows, is E0277 (`resolve.import-in-body`), whose fix moves it
to the top.

**Try it.** In `src/main.sisu`, change `import trip.distance` to `from
trip.distance import *`, and call `between` without `distance.`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/17-modules-and-packages/glob.console -->
```console
$ sisu run
error[E0128]: Sisu has no glob imports: name what the module gives, as in `from std.collections import Deque, Heap`, or import the module and write `collections.Deque`
  --> src/main.sisu:2:27
  |
2 | from trip.distance import *
  |                           ^

error[E0200]: cannot find `between` in this scope
  --> src/main.sisu:6:14
  |
6 |     let km = between("Turku", "Tampere")
  |              ^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

E0128 (`parse.glob-import`) says that Sisu has no glob imports, so the
second error follows. Name each item you import, or import the module
and write `distance.between`.

</details>

> **From Python.** A module is a file, as in Python, and `import` and
> `from ... import` read the same. Sisu has no `from x import *`, no
> `__init__.py` (a directory is a module without one), and no import
> that runs code: importing a module only makes its names visible.

## A library and a program that uses it

A café's till adds up receipts with a library, `money`, which sits
beside it in a directory of its own. The library's manifest names it,
and its root module is `src/lib.sisu`:

<!-- file: examples/17-modules-and-packages/shop.pkg/money/sisu.toml -->
```toml
[package]
name = "money"
version = "0.1.0"
edition = "2027"

[dependencies]

[features]
default = []
vat = []
```

<!-- file: examples/17-modules-and-packages/shop.pkg/money/src/lib.sisu -->
```sisu
##! Amounts of money in euros and cents.

## An amount of money, in cents.
@derive(Copy, Eq, Ord)
pub struct Money:
    pub cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100},{self.cents % 100:02} €")

impl Add for Money:
    def add(self, other: Money) -> Money:
        return Money(self.cents + other.cents)

## `amount` with value added tax at `percent`, rounded down to a cent.
@cfg(feature="vat")
pub def with_vat(amount: Money, percent: int) -> Money:
    return Money(amount.cents * (100 + percent) // 100)
```

<!-- file: examples/17-modules-and-packages/shop.pkg/money/src/parse.sisu -->
```sisu
##! Reading amounts of money from text.

from money import Money

## Why a text is not an amount of money.
pub error MoneyError:
    NotAnAmount(text: String)

## The amount that `text` writes in euros and cents, such as "12,50".
pub def parse_money(text: str) -> Money ! MoneyError:
    let euros, cents = text.trim().split_once(",") else:
        return Err(NotAnAmount(text.to_string()))
    let e = euros.parse_int() else:
        return Err(NotAnAmount(text.to_string()))
    let c = cents.parse_int() else:
        return Err(NotAnAmount(text.to_string()))
    return Ok(Money(e * 100 + c))
```

The till's manifest lists `money` under `[dependencies]` with the path
to its directory, and its `main` imports from it:

<!-- file: examples/17-modules-and-packages/shop.pkg/till/sisu.toml -->
```toml
[package]
name = "till"
version = "0.1.0"
edition = "2027"

[dependencies]
money = { path = "../money", features = ["vat"] }
```

<!-- file: examples/17-modules-and-packages/shop.pkg/till/src/main.sisu -->
```sisu
# A till adds up a receipt with the library `money`.
from money import Money, with_vat
from money.parse import MoneyError, parse_money

def main() -> ! MoneyError:
    var total = Money(0)
    for line in ["2,80", "12,50", "3,05"]:
        total = total + parse_money(line)?
    print("total {total}, with tax {with_vat(total, 24)}")
    return Ok(())
```

`sisu run till` runs the package in the directory `till`, from the
directory that holds both:

<!-- output: examples/17-modules-and-packages/shop.console -->
```console
$ sisu run till
total 18,35 €, with tax 22,75 €
```

- `path = "../money"` makes `money` a path dependency: the library is
  the directory at that path, relative to the manifest, and its name
  in the code is the name its own manifest gives it. `features =
  ["vat"]` turns on an optional part of it, which the section
  "Features" explains.
- `from money import Money` reaches an item of the library's root
  module, and `from money.parse import ...` one of its module
  `money.parse`. The library's own modules import each other the same
  way: `parse.sisu` imports `Money` from the root module.
- `pub struct Money` and `pub cents` make the type and its field
  visible to the till, and `pub error MoneyError` the error set that
  `main` passes on.

The first command that builds the till writes a lockfile, `sisu.lock`,
beside its manifest. It records every package the build used, with
its version and where it came from:

<!-- output: examples/17-modules-and-packages/shop.lock -->
```toml
# Written by `sisu` from the manifests: the packages this build resolved.
# It is data; `sisu` rewrites it when a manifest changes.
version = 1

[[package]]
name = "money"
version = "0.1.0"
source = "path+../money"
dependencies = []

[[package]]
name = "till"
version = "0.1.0"
source = "path+."
dependencies = ["money"]
```

A lockfile is data that `sisu` writes and rewrites when a manifest
changes, so you keep it with the code and never edit it.
`sisu build --locked` refuses to change it, which a build machine
uses to make sure that it builds what the developer built. The
packages of a build form a graph without cycles: a library that
depended on its own user would get E1007 (`package.cycle`).

## A library obeys the orphan rule

Chapter 14 said that a library may not implement a trait it did not
declare for a type it did not declare. Here the library `money` tries
to give `f64` the order that chapter 14's program gave itself.

**Try it.** Add the impl `impl Ord for f64` of chapter 14's section "One
impl for each type and trait" to `money/src/lib.sisu`, and run the till.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/17-modules-and-packages/orphan.console -->
```console
$ sisu run till
error[E0236]: `Ord` for `f64` can be implemented only in the package of the trait or of the type, or in the root package
  --> ../money/src/lib.sisu:22:1
   |
22 | impl Ord for f64:
   | ^^^^^^^^^^^^^^^^
  = note: the impl is this package's when `f64` is a type of this package, `Ord` a trait of it, or one of the trait's type arguments a type of its own, as `Vector` in `impl Mul[Vector] for f64`

error[D0006]: the program was not run: errors found (1)
```

E0236 (`resolve.orphan-impl`) points into the library. The till is the
root package, so the same impl in `till/src/main.sisu` would compile:
nothing depends on the till, and no other package can choose a second
order for `f64` in the same build (D-013). The note names the third way
an impl is the library's own, through a type argument of the trait, as
chapter 14's `plane` gives `f64` an operator that takes a `Vector`.

</details>

## Features

A library can have optional parts, called features, which a user turns
on in the dependency's line. `money` declares one, `vat`, in its
manifest's `[features]` table, and `@cfg(feature="vat")` on `with_vat`
leaves the function out of every build that does not ask for it. The
till asks with `features = ["vat"]`.

**Try it.** Delete `, features = ["vat"]` from the till's manifest, and
run the till.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/17-modules-and-packages/no_vat.console -->
```console
$ sisu run till
error[E0204]: module `money` has no item `with_vat`
  --> src/main.sisu:2:26
  |
2 | from money import Money, with_vat
  |                          ^^^^^^^^
  = note: `with_vat` is left out of this build by its `@cfg(feature="vat")`, which does not hold

error[E0200]: cannot find `with_vat` in this scope
  --> src/main.sisu:9:37
  |
9 |     print("total {total}, with tax {with_vat(total, 24)}")
  |                                     ^^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

E0204 (`resolve.not-in-module`) says that `money` has no
`with_vat`: without the feature, the function is not in the build at
all. Its note names the condition that left it out,
`@cfg(feature="vat")`, so the fix is to turn the feature on.

</details>

`default` lists the features that are on unless a user writes
`default-features = false`. The root package's own features are turned
on with `sisu run --features NAMES`, and `@cfg(test)` marks an item
that exists only in `sisu test`.

## The manifest's tables

A manifest is data, never a program (D-063), and Sisu has no build
scripts (D-032): what a package needs is written in these tables.

- `[package]`: `name`, `version` (three numbers, `MAJOR.MINOR.PATCH`),
  `edition`, and the optional `description` and `license`.
- `[dependencies]`: one line for each library, with its `path`, the
  `features` it should have, and `when`, which limits the dependency to
  some targets. `[dev-dependencies]` lists the ones that only the tests
  use.
- `[features]`: the package's own features, as in `money`.
- `[trust]`: `unsafe = [...]`, the dependencies that may use `unsafe`
  and call C. A dependency not listed there can do only what the
  capabilities it is handed allow; chapter 27 is about `unsafe`.
- `[workspace]`: the packages of a workspace, which the section "A
  workspace" shows.

An unknown key gets the warning W0005 (`package.unknown-key`), and a
mistake an error at its line in the file, so a misspelt key does not
go unnoticed.

A dependency with `when` is not built on the other targets, so code
that builds everywhere imports it under the same condition, which an
import's `@cfg` may test, and uses it only in an `if comptime` branch
on the same fact. Here the program `sauna` has `winlog = { path =
"../winlog", when = { os = "windows" } }` among its dependencies:

<!-- file: examples/reference-diagnostics/e0275_excluded_cfg.files/sauna/src/main.sisu -->
```sisu
# The import tests the target, and so does the code that uses it.
@cfg(target.os == "windows")
import winlog

def main():
    if comptime target.os == "windows":
        winlog.record("the stove is on")
    else:
        print("the stove is on")
```

<!-- output: examples/reference-diagnostics/e0275_excluded_cfg.console -->
```console
$ sisu run sauna
the stove is on
```

On any other system the import binds nothing and the first branch is
left out of the build, so the program prints its own line. A use of `winlog`
outside such a branch is E0200, whose note names the `@cfg` and the
`if comptime` to write ([R13](../reference/modules.md#target-conditions)).
The runtime is a fact of the same kind: `when = { runtime = "native" }`
and `@cfg(target.runtime == "native")` hold in a build for the native
runtime, such as `sisu run --native`, so a library that only native
code needs is built and imported there alone
([R13](../reference/modules.md#target-conditions) shows one).

## Documentation for a library's users

A library's users read its documentation, which Sisu writes from the
comments in the code. `##!` at the top of a file documents the module,
and `##` on the lines before an item documents the item. An example in
a doc comment, between lines of three backticks, runs as a test:

<!-- file: examples/17-modules-and-packages/workspace.pkg/wordcount/src/text.sisu -->
```sisu
##! Cutting a text into words.

## The words of `text`: its runs of letters and digits, in lower case.
##
## ```
## assert words("Hei, hei!") == ["hei", "hei"]
## ```
pub pure def words(text: str) -> list[String]:
    var found: list[String] = []
    var word = String()
    for c in text.chars():
        if c.is_alphabetic() or c.is_digit():
            word.push_char(c)
        elif not word.is_empty():
            found.push(word.to_lowercase())
            word = String()
    if not word.is_empty():
        found.push(word.to_lowercase())
    return found

## Each word of `text` with the word after it, in order.
pub pure def pairs(text: str) -> list[(String, String)]:
    let all = words(text)
    return [(all[i].copy(), all[i + 1].copy()) for i in range(len(all) - 1)]
```

This is a module of a library, `wordcount`, that the next section puts
in a workspace. `sisu test` runs the example of `words` as a test named
`text/words (doc example)`, so the documentation cannot drift from what
the code does. `sisu doc` writes the documentation as a Markdown page
for each module, with the module's text, a list of its `pub` items, and
for each item its signature and its doc comment; `sisu doc --output
docs` puts the pages in the directory `docs`. Only `pub` items appear:
the documentation is the library's API, and what is private is left
out.

## A workspace

A library and the program that uses it are often developed together.
A workspace holds both under one directory, whose manifest lists them
as its members:

<!-- file: examples/17-modules-and-packages/workspace.pkg/sisu.toml -->
```toml
[workspace]
members = ["wordcount", "app"]
```

The library `wordcount` keeps the counting, and the package `app` the
command line. The library's root module re-exports the functions of
its module `text`, so that its users find them in one place:

<!-- file: examples/17-modules-and-packages/workspace.pkg/wordcount/src/lib.sisu -->
```sisu
##! Counting the words of a text, and anything else a dict can key.

pub from wordcount.text import words, pairs

## How often each item was added, for items that a dict can use as a
## key.
pub struct Tally[T: Hash]:
    counts: dict[T, int]

    ## A tally of nothing yet.
    pub pure def new() -> Tally[T]:
        return Tally({})

    ## Counts `item` once more.
    pub pure def add(mut self, own item: T):
        self.counts.entry(item).or_insert(0) += 1

    ## The `n` most common items with their counts, the most common
    ## first; items counted as often keep the order of their first add.
    pub pure def top(self, n: int) -> list[(T, int)]:
        var ranked = [(item.copy(), n) for item, n in self.counts]
        ranked.sort_by((a, b) -> b.1.cmp(a.1))
        return ranked[:n]

## The ten most common items, a line for each, as "   3  kissa".
impl[T: Hash + Display] Display for Tally[T]:
    def fmt(self, mut f: Formatter):
        let lines = ["{n:>4}  {item}" for item, n in self.top(10)]
        f.write(lines.join("\n"))

test "a tally of words":
    var tally = Tally[String].new()
    for word in own words("kissa koira kissa"):
        tally.add(word)
    assert tally.top(1) == [("kissa", 2)]
```

<!-- file: examples/17-modules-and-packages/workspace.pkg/app/src/main.sisu -->
```sisu
##! The word counter's command line, over the library `wordcount`.

import cli
import wordcount
from wordcount import Tally

## What the command line can say.
struct Options:
    paths: list[Path]

def main(sys: System) -> ! {IoError, cli.Error}:
    let options = cli.parse[Options](sys.env.args())?
    var tally = Tally[String].new()
    for path in options.paths:
        let text = sys.fs.read_to_string(path)?
        for word in own wordcount.words(text):
            tally.add(word)
    print(tally)
    return Ok(())
```

`sisu test` in the workspace's directory runs the tests of every
member, the library's first, each list under a line that names its
package; `sisu test wordcount/src/text.sisu` runs the tests of one
module, named with the module's name first; and `sisu run app` runs
the program:

<!-- output: examples/17-modules-and-packages/workspace.console -->
```console
$ sisu test
tests of `wordcount`:
test "a tally of words" ... ok
test "text/words (doc example)" ... ok
2 passed, 0 failed, 0 blocked
tests of `app`:
0 passed, 0 failed, 0 blocked
$ sisu test wordcount/src/text.sisu
test "text/words (doc example)" ... ok
1 passed, 0 failed, 0 blocked
$ sisu run app -- poem.txt
   4  the
   3  is
   2  sauna
   2  warm
   2  lake
   1  cold
   1  after
   1  too
```

- `pub from wordcount.text import words, pairs` imports the two
  functions into the root module and makes them part of it, so the
  program calls `wordcount.words(text)`. Without `pub`, the import is
  the root module's own, and a user's `wordcount.words` gets E0257
  (`resolve.not-reexported`). In stage 0, `from wordcount import words`
  does not find a re-exported function (E0204); `import wordcount` and
  the module's name do.
- The workspace has one lockfile, beside its manifest, for all its
  members, so they always build with the same versions.
- The app's manifest names the library by its path, `../wordcount`, as
  the till named `money`.

A program that grows into a workspace keeps its tests beside the code
they test, in each member.

## Publishing, a first look

A library that others may use is published to a registry, which the
compiler does not have yet: `sisu add`, which adds a dependency from
the registry, and `sisu publish` are designed in
[09](../../proposal/09-modules-packages-and-build.md#the-registry) and
wait for it. Three things are planned to happen when you publish
(D-031, D-066):

- `sisu publish` compares the library's `pub` items with the last
  version's. A change that breaks a user, such as a removed function
  or a new required parameter, needs a new major version, and the
  registry refuses it under a minor one.
- The registry shows which capabilities the library's API takes. A
  library whose functions take no `Fs`, `Net` or other capability, and
  that is not in anyone's `[trust]`, can only compute.
- A new library, or a new version of one, waits through a cooldown
  before builds that admit new packages take it, so that a bad release
  has time to be found.

Documentation is part of a library's API: every `pub` item with a doc
comment, and examples that run.

## Exercise 17.1: the sauna queue

Open
[`manual/exercises/17-modules-and-packages/sauna.sisu`](../exercises/17-modules-and-packages/sauna.sisu):

<!-- file: exercises/17-modules-and-packages/sauna.sisu -->
```sisu
##! Exercise 17.1, the sauna queue. Kind: fix. Level: 1.
##!
##! A sauna lets bathers in from a queue, and keeps how many times each
##! bather has been in, in alphabetical order. The types come from the
##! standard library's module `std.collections`, which the file does not
##! import the way Sisu imports. The file does not compile. Run
##! `sisu test sauna.sisu`, read the diagnostics, and fix the imports,
##! not the code or the tests, so that every test passes. The hints are in
##! chapter 17 of the tutorial, under "Exercise 17.1: the sauna queue".

from std.collections import *

## The bathers of `names`, let in `seats` at a time: each round goes
## in, and comes out to the back of the queue, until `rounds` are done.
## The result counts how many times each bather went in.
def bathe(names: list[str], seats: int, rounds: int) -> SortedDict[str, int]:
    var queue: Deque[str] = Deque.new()
    for name in names:
        queue.push_back(name)
    var visits: SortedDict[str, int] = SortedDict.new()
    for _ in range(rounds):
        var inside: list[str] = []
        for _ in range(seats):
            if let Some(name) = queue.pop_front():
                inside.push(name)
        for name in inside:
            _ = visits.insert(name, (visits.get(name) ?? 0) + 1)
            queue.push_back(name)
    return visits

test "two seats, three rounds":
    let visits = bathe(["Roni", "Aino", "Eero"], 2, 3)
    assert visits.keys() == ["Aino", "Eero", "Roni"]
    assert (visits.get("Roni") ?? 0) == 2

test "one seat, four rounds":
    let visits = bathe(["Roni", "Aino", "Eero"], 1, 4)
    assert (visits.get("Roni") ?? 0) == 2
    assert (visits.get("Eero") ?? 0) == 1
```

<!-- output: solutions/17-modules-and-packages/sauna.console -->
```console
$ sisu test sauna.sisu
error[E0128]: Sisu has no glob imports: name what the module gives, as in `from std.collections import Deque, Heap`, or import the module and write `collections.Deque`
  --> sauna.sisu:11:29
   |
11 | from std.collections import *
   |                             ^

error[E0201]: cannot find a type named `SortedDict`
  --> sauna.sisu:16:57
   |
16 | def bathe(names: list[str], seats: int, rounds: int) -> SortedDict[str, int]:
   |                                                         ^^^^^^^^^^^^^^^^^^^^

error[E0201]: cannot find a type named `Deque`
  --> sauna.sisu:17:16
   |
17 |     var queue: Deque[str] = Deque.new()
   |                ^^^^^^^^^^

error[E0200]: cannot find `Deque` in this scope
  --> sauna.sisu:17:29
   |
17 |     var queue: Deque[str] = Deque.new()
   |                             ^^^^^

error[E0201]: cannot find a type named `SortedDict`
  --> sauna.sisu:20:17
   |
20 |     var visits: SortedDict[str, int] = SortedDict.new()
   |                 ^^^^^^^^^^^^^^^^^^^^

error[E0200]: cannot find `SortedDict` in this scope
  --> sauna.sisu:20:40
   |
20 |     var visits: SortedDict[str, int] = SortedDict.new()
   |                                        ^^^^^^^^^^

error[D0006]: the program was not run: errors found (6)
```

<details>
<summary>Hint 1</summary>

The other errors follow from the first one: the names are not found
because nothing imported them.

</details>

<details>
<summary>Hint 2</summary>

The section "What an import can name" says how to import from a
module of the standard library, and what Sisu has instead of `*`.

</details>

When every test passes, compare your answer with
[`manual/solutions/17-modules-and-packages/sauna.sisu`](../solutions/17-modules-and-packages/sauna.sisu).

## Exercise 17.2: a documented function

Open
[`manual/exercises/17-modules-and-packages/initials.sisu`](../exercises/17-modules-and-packages/initials.sisu):

<!-- file: exercises/17-modules-and-packages/initials.sisu -->
```sisu
##! Exercise 17.2, a documented function. Kind: fix. Level: 1.
##!
##! A library documents each `pub` item, and the examples in its doc
##! comments run as tests. `initials` fails its doc example and a test.
##! Run `sisu test initials.sisu`, read the reports, and fix the body of
##! `initials`, not its doc comment or the tests, so that every test
##! passes. The hints are in chapter 17 of the tutorial, under
##! "Exercise 17.2: a documented function".

## The initials of `name`: the first letter of each of its words, in
## capitals, each followed by a dot.
##
## ```
## assert initials("aino maria virtanen") == "A.M.V."
## ```
pub def initials(name: str) -> String:
    var out = String()
    for word in name.split(" "):
        if let Some(first) = word.chars().next():
            out.push_char(first)
        out.push(".")
    return out

test "spaces around the words":
    assert initials("Eero  Virtanen") == "E.V."
    assert initials(" Roni ") == "R."
```

<!-- output: solutions/17-modules-and-packages/initials.console -->
```console
$ sisu test initials.sisu
test "spaces around the words" ... FAILED
test "initials (doc example)" ... FAILED

test "spaces around the words" ... FAILED
panic: assertion failed: initials("Eero  Virtanen") == "E.V."
  left:  "E..V."
  right: "E.V."
  at test "spaces around the words" (initials.sisu:25:5)
replay: sisu test --replay 1-588072136504aa03-0.0.1-t0-s0-r0 initials.sisu

test "initials (doc example)" ... FAILED
panic: assertion failed: initials("aino maria virtanen") == "A.M.V."
  left:  "a.m.v."
  right: "A.M.V."
  at test "initials (doc example)" (initials.sisu:14:4)
replay: sisu test --replay 1-588072136504aa03-0.0.1-t1-s0-r0 initials.sisu

0 passed, 2 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

Each report shows the two sides of the comparison that failed. The doc
example's says what the doc comment promises and the function does not
do; the test's says what extra spaces do to the result.

</details>

<details>
<summary>Hint 2</summary>

`split(" ")` gives an empty word between two spaces, and
`split_whitespace()` does not. A `char` has no method that makes it a
capital, but text has.

</details>

When every test passes, compare your answer with
[`manual/solutions/17-modules-and-packages/initials.sisu`](../solutions/17-modules-and-packages/initials.sisu).

## Challenge III: a word library

The challenge turns the word counter of chapter 9 into a library, with
documented `pub` items, in three parts with a suite of tests for each.
Open
[`manual/exercises/17-modules-and-packages/wordcount.sisu`](../exercises/17-modules-and-packages/wordcount.sisu):

<!-- file: exercises/17-modules-and-packages/wordcount.sisu -->
```sisu
##! Exercise 17.3, Challenge III: a word library. Kind: challenge. Level: 3.
##!
##! The word counter of chapter 9 becomes a library that any program can
##! use. A library's API is its `pub` items, each with a doc comment
##! that says what it does. This file holds the library and its tests;
##! when every test passes, the section "Challenge III" of chapter 17
##! shows how it moves into a workspace beside an application. Solve the
##! parts in order: each part's tests are blocked until it is done. Run
##! the tests with `sisu test wordcount.sisu`.
##!
##! Part 1. Write `words` and the methods of `Tally[T]`, which counts
##! items of any type that a dict can use as a key.
##!
##! Part 2. A tally counts more than words. Write `pairs`, which gives
##! each word of a text with the word after it, so that a tally of pairs
##! finds the words that most often come together.
##!
##! Part 3. Write the `Display` of a tally: its ten most common items,
##! a line for each, with the count right-aligned in four columns, two
##! spaces and the item, and no newline after the last line. `pure`
##! marks a function that only computes, as chapter 26 explains; `fmt`
##! is `pure`, so it can call only `pure` methods, such as `top`.
##!
##! The hints are in chapter 17 of the tutorial, under "Challenge III: a
##! word library".

## The words of `text`: its runs of letters and digits, in lower case.
pub pure def words(text: str) -> list[String]:
    return todo("words")

## Each word of `text` with the word after it, in order.
pub pure def pairs(text: str) -> list[(String, String)]:
    return todo("pairs")

## How often each item was added, for items that a dict can use as a
## key.
pub struct Tally[T: Hash]:
    counts: dict[T, int]

    ## A tally of nothing yet.
    pub pure def new() -> Tally[T]:
        return todo("new")

    ## Counts `item` once more.
    pub pure def add(mut self, own item: T):
        todo("add")

    ## How often `item` was added.
    pub pure def count(self, item: T) -> int:
        return todo("count")

    ## The `n` most common items with their counts, the most common
    ## first; items counted as often keep the order in which they were
    ## first added.
    pub pure def top(self, n: int) -> list[(T, int)]:
        return todo("top")

## The ten most common items, a line for each, as "   3  kissa".
impl[T: Hash + Display] Display for Tally[T]:
    def fmt(self, mut f: Formatter):
        todo("fmt")

suite "part 1, a tally of words":
    test "the words of a text":
        assert words("Hei, hei maailma!") == ["hei", "hei", "maailma"]
        assert words("a well-known walk") == ["a", "well", "known", "walk"]

    test "a tally of words":
        var tally = Tally[String].new()
        for word in own words("kissa koira kissa kala kissa koira"):
            tally.add(word)
        assert tally.count("kissa") == 3
        assert tally.count("lehmä") == 0
        assert tally.top(2) == [("kissa", 3), ("koira", 2)]

    test "a tally of numbers":
        var tally = Tally[int].new()
        for n in [3, 1, 3, 2, 1, 3]:
            tally.add(n)
        assert tally.top(5) == [(3, 3), (1, 2), (2, 1)]

suite "part 2, pairs of words":
    test "the pairs of a text":
        let found = pairs("yksi kaksi, kolme")
        assert found == [("yksi", "kaksi"), ("kaksi", "kolme")]
        assert pairs("yksi") == []

    test "the pair that comes most often":
        var tally = Tally[(String, String)].new()
        for pair in own pairs("on the lake, on the shore, in the sauna"):
            tally.add(pair)
        assert tally.top(1) == [(("on", "the"), 2)]
        assert tally.count(("the", "lake")) == 1

suite "part 3, a table":
    test "a short table":
        var tally = Tally[String].new()
        for word in own words("kissa koira kissa"):
            tally.add(word)
        assert "{tally}" == "   2  kissa\n   1  koira"

    test "ten lines at most":
        var tally = Tally[int].new()
        for n in range(20):
            tally.add(n % 15)
        assert "{tally}".lines().count() == 10
        assert "{tally}".starts_with("   2  0\n   2  1\n")
```

Before you change anything, every test is blocked:

<!-- output: solutions/17-modules-and-packages/wordcount.console -->
```console
$ sisu test wordcount.sisu
warning[W0001]: typed hole `words` of type `list[String]`
  --> wordcount.sisu:29:12
   |
29 |     return todo("words")
   |            ^^^^^^^^^^^^^

warning[W0001]: typed hole `pairs` of type `list[(String, String)]`
  --> wordcount.sisu:33:12
   |
33 |     return todo("pairs")
   |            ^^^^^^^^^^^^^

warning[W0001]: typed hole `new` of type `Tally[T]`
  --> wordcount.sisu:42:16
   |
42 |         return todo("new")
   |                ^^^^^^^^^^^

warning[W0001]: typed hole `add` of type `()`
  --> wordcount.sisu:46:9
   |
46 |         todo("add")
   |         ^^^^^^^^^^^

warning[W0001]: typed hole `count` of type `int`
  --> wordcount.sisu:50:16
   |
50 |         return todo("count")
   |                ^^^^^^^^^^^^^

warning[W0001]: typed hole `top` of type `list[(T, int)]`
  --> wordcount.sisu:56:16
   |
56 |         return todo("top")
   |                ^^^^^^^^^^^

warning[W0001]: typed hole `fmt` of type `()`
  --> wordcount.sisu:61:9
   |
61 |         todo("fmt")
   |         ^^^^^^^^^^^

test "part 1, a tally of words/the words of a text" ... blocked by todo("words")
test "part 1, a tally of words/a tally of words" ... blocked by todo("new")
test "part 1, a tally of words/a tally of numbers" ... blocked by todo("new")
test "part 2, pairs of words/the pairs of a text" ... blocked by todo("pairs")
test "part 2, pairs of words/the pair that comes most often" ... blocked by todo("new")
test "part 3, a table/a short table" ... blocked by todo("new")
test "part 3, a table/ten lines at most" ... blocked by todo("new")

test "part 1, a tally of words/the words of a text" ... blocked by todo("words")
what the test printed:
  in scope at the typed hole `todo("words")`:
    text = "Hei, hei maailma!"

test "part 2, pairs of words/the pairs of a text" ... blocked by todo("pairs")
what the test printed:
  in scope at the typed hole `todo("pairs")`:
    text = "yksi kaksi, kolme"

0 passed, 0 failed, 7 blocked
```

The exercise is one file, so that `sisu test` can judge it as it
judges the others. When every test passes, move it into a workspace as
the section "A workspace" does: `words` and `pairs` into
`wordcount/src/text.sisu`, `Tally` and its `Display` into
`wordcount/src/lib.sisu` with the re-export, the tests beside them, and
a program that uses the library into `app/src/main.sisu`. The
workspace of that section is the result, so you can compare your
files with it.

<details>
<summary>Hint for part 1</summary>

`words` is the `words_of` of Challenge I. A `Tally` keeps its counts in
a dict; `new` makes an empty one, and `count` gives 0 for an item that
was never added. `top` makes a list of pairs of an item and its count,
and sorts it with `sort_by` from the most to the fewest, which keeps
the dict's order of first adds among equal counts.

</details>

<details>
<summary>Hint for part 2</summary>

`pairs` takes the words of the text and pairs each word with the next
one. A tally of `(String, String)` needs nothing new: the tuple is a
key that a dict can use, which is why `Tally` is generic.

</details>

<details>
<summary>Hint for part 3</summary>

`fmt` makes a line for each of the ten most common items, `{n:>4}`
pads the count, and `join` puts newlines between the lines and none
after the last.

</details>

When every test passes, compare your answer with
[`manual/solutions/17-modules-and-packages/wordcount.sisu`](../solutions/17-modules-and-packages/wordcount.sisu).

## What you learned

- Each file under `src/` is a module named by its path; `src/main.sisu`
  or `src/lib.sisu` is the root, named by the package.
- An item is private to its module unless it is `pub` (E0255);
  `pub(package)` reaches the package's modules only (E0256).
- `import a.b` binds `b`, and `from a.b import x as y` binds an item;
  there are no glob imports, and the prelude's fixed list needs none.
  `pub from` re-exports (E0257).
- A manifest lists path dependencies, features, `[trust]` and
  workspace members; the lockfile records what a build used; packages
  have no cycles (E1007), and a library obeys the orphan rule (E0236).
- `##!` and `##` document modules and items, their examples run as
  tests, and `sisu doc` writes a page for each module.
- Publishing, with its API diff, capability summary and cooldown, waits
  for the registry.

That is the end of Part III. You can give your types abilities with
traits, write code once for many types, pass functions and pipelines
around, and split a program into modules and packages that document
themselves. Part IV, which starts with chapter 18, is about the world
outside the program: capabilities, testing in depth, concurrency and
the standard library.
