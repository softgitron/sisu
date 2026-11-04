# 25. Compile-time code

Some work can be done once, when the program is built, rather than
each time it runs: a table of numbers, a check on a constant, the code
that compares or prints a struct field by field. In Sisu that work is
ordinary code that the compiler runs. In this chapter you compute
values and check them while the program compiles, give a long
computation the steps it needs, and ask a type about its fields. Then
you write a function that depends on the shape of its type, write a
derive of your own, read attributes as data, make a file part of the
program, and choose code for a target or for the tests. You need about
two hours.

## Values computed while compiling

`comptime e` asks the compiler to compute `e`. The code is ordinary
code, run by the same interpreter as the program, with the same
arithmetic, the same overflow checks and the same order of a `dict`;
the program keeps the value it gave:

<!-- file: examples/25-compile-time/table.sisu -->
```sisu
# `comptime` runs ordinary code while the program compiles.

## The prime numbers below `n`, by the sieve of Eratosthenes.
def primes_below(n: int) -> list[int]:
    var composite: list[bool] = []
    for _ in range(n):
        composite.push(False)
    var primes: list[int] = []
    for k in range(2, n):
        if not composite[k]:
            primes.push(k)
            var multiple = k * k
            while multiple < n:
                composite[multiple] = True
                multiple += k
    return primes

const PRIMES: list[int] = comptime primes_below(100)

def main():
    print("{len(PRIMES)} primes, the last {PRIMES[^1]}")
    print(comptime "computed while compiling".to_uppercase())
    print(primes_below(20))
```

<!-- output: examples/25-compile-time/table.out -->
```text
25 primes, the last 97
COMPUTED WHILE COMPILING
[2, 3, 5, 7, 11, 13, 17, 19]
```

- `const PRIMES: list[int] = comptime primes_below(100)` runs the sieve
  while the program compiles. When the program runs, `PRIMES` is built
  from the 25 numbers the sieve found, as if they had been written as a
  literal.
- `primes_below` is an ordinary function, and `main` calls it again at
  run time. Nothing marks a function as one that can run at compile
  time: any function can, as long as it does not reach the world.
- `comptime` applies to the expression after it, here the call of
  `to_uppercase`. In `comptime f() + 1`, only `f()` is computed early.

> **From Python.** A table built at the top of a Python module is
> built each time the program starts. A `comptime` table is built once,
> when the program is compiled, and a program that starts a thousand
> times does not build it a thousand times.

## Checks while compiling

`comptime assert` checks a condition while the program compiles and
leaves nothing behind for run time. A `comptime def` is a function that
only compile-time code may call:

<!-- file: examples/25-compile-time/checks.sisu -->
```sisu
# `comptime assert` checks while compiling; a `comptime def` runs only then.

## Whether `n` is a power of two.
comptime def power_of_two(n: int) -> bool:
    return n > 0 and n & (n - 1) == 0

## How many turns the sauna's booking ring holds.
const SLOTS: int = 16

## The slot of the `n`th booking, counting round the ring.
def slot(n: int) -> int:
    comptime assert power_of_two(SLOTS), "{SLOTS} slots is not a power of two"
    return n & (SLOTS - 1)

def main():
    print([slot(n) for n in [3, 16, 21]])
```

<!-- output: examples/25-compile-time/checks.out -->
```text
[3, 0, 5]
```

`slot` keeps a booking's place in a ring of 16 slots with `& (SLOTS -
1)`, which is the remainder of a division by 16 only when 16 is a power
of two. The `comptime assert` makes that assumption a check, made once,
when the program is compiled, at no cost when it runs.

**Try it.** Make `SLOTS` 12. Then put it back, and print
`power_of_two(SLOTS)` in `main`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/25-compile-time/checks_twelve.console -->
```console
$ sisu run checks_twelve.sisu
error[E0602]: the compile-time evaluation of this expression panicked: assertion failed: power_of_two(SLOTS)
  message: 12 slots is not a power of two
  --> checks_twelve.sisu:12:5
   |
12 |     comptime assert power_of_two(SLOTS), "{SLOTS} slots is not a power of two"
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<!-- output: examples/25-compile-time/checks_run_time.console -->
```console
$ sisu run checks_run_time.sisu
error[E0605]: `power_of_two` is a `comptime def`: it can be called only at compile time, inside `comptime`
  --> checks_run_time.sisu:16:11
   |
16 |     print(power_of_two(SLOTS))
   |           ^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

A compile-time assertion that fails is a compile error, E0602
(`comptime.panicked`), with the assertion's message, so a ring of 12
slots never reaches a user. E0605 (`comptime.only`) says that a
`comptime def` cannot run when the program runs; write `comptime
power_of_two(SLOTS)` to compute it while compiling.

</details>

## What compile-time code cannot do

Compile-time code cannot reach the world. A build must give the same
program every time, on every machine, so nothing reads a file, the
clock or the environment, and nothing prints, while the program
compiles:

<!-- file: examples/25-compile-time/world.sisu -->
```sisu
# Nothing reaches the world at compile time.

## A greeting for `name`, and a note of it on standard output.
def greet(name: str) -> String:
    print("greeting {name}")
    return "Hei, {name}!"

const GREETING: String = comptime greet("Aino")

def main():
    print(GREETING)
```

<!-- output: examples/25-compile-time/world.console -->
```console
$ sisu run world.sisu
error[E0601]: the compile-time evaluation of this expression used the standard output; nothing reaches the world at compile time
  --> world.sisu:8:26
  |
8 | const GREETING: String = comptime greet("Aino")
  |                          ^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0601 (`comptime.io`) stops the evaluation where it used the standard
output, and names what it used. A function needs no `pure` to run at
compile time; one that tries to reach the world is stopped where it
tries. Two more rules hold:

- Compile-time code sees only what is known while compiling: a constant
  or a literal, not a local or a parameter of the function around it.
  `comptime name.to_uppercase()` with `name` a local is E0600
  (`comptime.not-constant`).
- The value must have a literal form for the program to keep: numbers,
  text, booleans, characters, and lists, sets, dicts, tuples, optional
  values, results, structs and enums of them. A closure or a reference
  made at compile time is E0604 (`comptime.not-kept`).

## A budget of steps

A computation at compile time could run for a long time, or forever.
Each `comptime` expression has a budget of a million steps of the
compiler's interpreter, and an item that needs more says so with
`@comptime_budget`:

<!-- file: examples/25-compile-time/budget.sisu -->
```sisu
# Compile-time code has a budget of steps, which an item can raise.

## The number of ways to climb `n` steps, one or two at a time.
def climbs(n: int) -> int:
    if n < 2:
        return 1
    return climbs(n - 1) + climbs(n - 2)

@comptime_budget(steps=2_000_000)
const STAIRS: int = comptime climbs(26)

def main():
    print("{STAIRS} ways up 26 steps")
```

<!-- output: examples/25-compile-time/budget.out -->
```text
196418 ways up 26 steps
```

`climbs(26)` calls itself hundreds of thousands of times, more than a
million steps in all. The budget belongs to the item that spends it,
here `STAIRS`, so a package that uses this one never has to raise a
budget for it. A computation this long makes every build of the
program slower, so keep compile-time work to what is worth doing
before the program ever runs.

**Try it.** Remove the `@comptime_budget` line.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/25-compile-time/budget_spent.console -->
```console
$ sisu run budget_spent.sisu
error[E0603]: the compile-time evaluation in `STAIRS` used up its budget of 1000000 steps; raise it with `@comptime_budget(steps=N)` on `STAIRS`
  --> budget_spent.sisu:9:21
  |
9 | const STAIRS: int = comptime climbs(26)
  |                     ^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0603 (`comptime.budget`) names the item and its budget, and the fix:
the attribute on `STAIRS`. The count of steps is the same on every
machine, so a build that fits its budget fits it everywhere.

</details>

## Asking a type about itself

The module `reflect` answers questions about types while the program
compiles: a type's name, whether it is a struct or an enum, its fields
and its variants. A `comptime def` can take a type as a parameter, `T:
type`, and a call passes the type:

<!-- file: examples/25-compile-time/reflect.sisu -->
```sisu
# `reflect` answers questions about a type while the program compiles.

## A turn in the sauna.
struct Booking:
    name: String
    hour: int
    minutes: int = 60

enum Heater:
    Wood
    Electric(kilowatts: int)

## The names of the fields of `T`, a struct.
comptime def columns(T: type) -> list[str]:
    var names: list[str] = []
    comptime for f in reflect.fields_of(T):
        names.push(f.name)
    return names

## A sentence about `T`.
comptime def describe(T: type) -> String:
    let name = reflect.name_of(T)
    if comptime reflect.is_struct(T):
        return "{name} has {len(reflect.fields_of(T))} fields"
    return "{name} has {len(reflect.variants_of(T))} variants"

def main():
    print(comptime columns(Booking))
    print(comptime describe(Booking))
    print(comptime describe(Heater))
```

<!-- output: examples/25-compile-time/reflect.out -->
```text
["name", "hour", "minutes"]
Booking has 3 fields
Heater has 2 variants
```

- `reflect.fields_of(T)` is the list of a struct's fields. Each has a
  `name`, an `index`, its type `ty`, and `default`, whether it has a
  default value.
- `reflect.variants_of(T)` is the list of an enum's variants, each with
  a `name`, an `index` and its `fields`.
- `comptime for` runs its loop while compiling. The list of fields
  exists only at compile time, so a loop over it is a `comptime for`.
- `if comptime` decides its condition while compiling, so a branch can
  ask about the type.

**Try it.** Remove `comptime` before the `for` in `columns`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/25-compile-time/reflect_loop.console -->
```console
$ sisu run reflect_loop.sisu
error[E0609]: making `columns[Booking]` stopped at compile time: a list is a compile-time value with no run-time form
  --> reflect_loop.sisu:28:20
   |
28 |     print(comptime columns(Booking))
   |                    ^^^^^^^^^^^^^^^^
   |
16 |     for f in reflect.fields_of(T):
   |              -------------------- the template, where it stopped
  = fix: unroll the loop at compile time with `comptime for`: `comptime `

error[D0006]: the program was not run: errors found (1)
```

E0609 (`comptime.instance-failed`) says that a list made by `reflect`
has no form at run time, and its label points at the loop, where the
compiler stopped. Its fix writes `comptime` before the `for` again.

</details>

## Code that depends on the shape of a type

A generic function whose body asks `reflect` about its type parameter
is made once for each type it is called with, and each of those copies
is checked on its own. Inside it, `value.[f]` is the field that `f`
names:

<!-- file: examples/25-compile-time/template.sisu -->
```sisu
# A generic function that reflects is made once for each type it is given.

## A turn in the sauna.
struct Booking:
    name: String
    hour: int

## A reading of the stove's thermometer.
struct Reading:
    minute: int
    celsius: f64

## `value` as a line of comma-separated values, a field at a time.
def csv_row[T](value: T) -> String:
    if comptime reflect.is_struct(T):
        var cells: list[String] = []
        comptime for f in reflect.fields_of(T):
            cells.push("{value.[f]}")
        return cells.join(",")
    else:
        return "{value}"

def main():
    print(csv_row(Booking(name="Aino", hour=18)))
    print(csv_row(Reading(minute=5, celsius=78.5)))
    print(csv_row(42))
```

<!-- output: examples/25-compile-time/template.out -->
```text
Aino,18
5,78.5
42
```

- `csv_row(Booking(...))` makes a copy of `csv_row` for `Booking`, in
  which the `comptime for` is unrolled into two lines, one for `name`
  and one for `hour`. The copy for `Reading` has its own two lines.
- `if comptime reflect.is_struct(T)` narrows `T`: inside the branch,
  `T` is known to be a struct, so asking for its fields makes sense.
  For `42`, the `else` branch is the whole copy.
- `"{value.[f]}"` interpolates the field, so each field's type must
  have a `Display`. That is checked in each copy, for the types that
  are really used.

**Try it.** Remove the `if comptime` and the `else` branch, so that the
loop runs for every type.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/25-compile-time/template_any.console -->
```console
$ sisu run template_any.sisu
Aino,18
5,78.5

```

The program compiles, and `csv_row(42)` gives an empty line: an `int`
has no fields, so the loop unrolls into nothing. The branch on the
shape of `T` is what makes the function right for other types.

</details>

## Writing a derive

`@derive(Eq, Debug)` asks each trait to write its impl for the type.
Any trait can be derived in the same way: its `comptime def derive(T:
type) -> impl Trait for T` returns the impl, written with reflection,
and `@derive(Trait)` runs it for the type:

<!-- file: examples/25-compile-time/derive.sisu -->
```sisu
# A trait's `derive` writes its impl for a type, with reflection.
trait Changes:
    ## The names of the fields in which `self` and `other` differ.
    def changes(self, other: Self) -> list[String]

    comptime def derive(T: type) -> impl Changes for T:
        return impl Changes for T:
            def changes(self, other: Self) -> list[String]:
                var names: list[String] = []
                comptime for f in reflect.fields_of(T):
                    if self.[f] != other.[f]:
                        names.push(f.name.to_string())
                return names

## A turn in the sauna.
@derive(Changes)
struct Booking:
    name: String
    hour: int
    minutes: int

def main():
    let before = Booking(name="Aino", hour=18, minutes=60)
    let after = Booking(name="Aino", hour=19, minutes=90)
    print(before.changes(after))
    print(before.changes(before))
```

<!-- output: examples/25-compile-time/derive.out -->
```text
["hour", "minutes"]
[]
```

- `return impl Changes for T:` with an indented body is the impl that
  the derive writes. The compile-time parts run while compiling: the
  `comptime for` unrolls into one comparison for each field.
- What is left is ordinary code, checked like an impl you wrote, so a
  derive cannot write code that does not type-check.
- A derive writes only an impl of its own trait, for the type it is
  given. Compile-time code cannot declare a new struct or a new
  function, which keeps it from becoming a second language beside
  Sisu, as macros are in other languages.

**Try it.** Add a field `heater: Heater` to `Booking`, of an enum
`Heater` with the variants `Wood` and `Electric`, and give both
bookings `heater=Wood`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/25-compile-time/derive_heater.console -->
```console
$ sisu run derive_heater.sisu
error[E0347]: `Heater` does not implement `Eq`
  --> derive_heater.sisu:21:9
   |
21 | @derive(Changes)
   |         ^^^^^^^
  = note: in the impl of `Changes` that the derive writes for `Booking`: `if self.heater != other.heater:`

error[D0006]: the program was not run: errors found (1)
```

E0347 (`type.bound-not-satisfied`) comes from the code that the derive
wrote: the comparison of the two heaters needs `Eq`, which `Heater`
does not have. Its place is the derive's argument that asked for the
impl, `Changes` in `@derive(Changes)`, and its note quotes the line of
the impl that the derive wrote for `Booking`, the unrolled code.
`@derive(Eq)` on `Heater` fixes it.

</details>

> **From Python.** Python's `@dataclass` writes `__eq__` and
> `__repr__` by looking at the class when the program runs. A derive
> does that work while the program compiles, and what it writes is
> checked like any other code.

## Attributes are data

An attribute such as `@table(bookings)` does nothing by itself. It is
data on the declaration that compile-time code can read:

<!-- file: examples/25-compile-time/attributes.sisu -->
```sisu
# Attributes are data that compile-time code can read.

## A turn in the sauna, kept in the table `bookings`.
@table(bookings)
struct Booking:
    name: String
    hour: int

## The query that reads every row of `T` from its table.
comptime def select_all(T: type) -> String:
    var table = reflect.name_of(T).to_lowercase()
    comptime for a in reflect.attributes_of(T):
        if a.name == "table":
            table = a.args[0].to_string()
    var columns = String.new("")
    var comma = ""
    comptime for f in reflect.fields_of(T):
        columns = "{columns}{comma}{f.name}"
        comma = ", "
    return "SELECT {columns} FROM {table}"

def main():
    print(comptime select_all(Booking))
```

<!-- output: examples/25-compile-time/attributes.out -->
```text
SELECT name, hour FROM bookings
```

`reflect.attributes_of(T)` gives each attribute on `T`'s declaration,
with its `name` and `args`, the text of each argument. `select_all`
reads the table's name from `@table`, or makes one from the type's
name, and writes the query once, while compiling. A web framework can
collect its routes the same way, from attributes on its handlers.

## A file in the program

`embed("path")` makes the text of a file part of the program while it
compiles. The path is a string literal, relative to the program's own
directory. Here is the file `glossary.txt`:

<!-- file: examples/25-compile-time/glossary.files/glossary.txt -->
```text
kiuas = the stove of a sauna
lauteet = the benches of a sauna
löyly = the steam from water thrown on the stones
vihta = a bundle of birch twigs
```

<!-- file: examples/25-compile-time/glossary.sisu -->
```sisu
# `embed` makes a file's text part of the program while it compiles.

const GLOSSARY: str = embed("glossary.txt")

## What `word` means, from the glossary.
def meaning(word: str) -> str?:
    for line in GLOSSARY.lines():
        if let Some((name, text)) = line.split_once(" = "), name == word:
            return Some(text)
    return None

def main():
    print("{len(GLOSSARY.lines().collect())} words")
    print(meaning("löyly"))
    print(meaning("sisu"))
```

<!-- output: examples/25-compile-time/glossary.out -->
```text
4 words
Some("the steam from water thrown on the stones")
None
```

The glossary is in the program, so the program needs no file system
and no `Fs` to read it. The file is one of the build's inputs: the
build that embeds it gives the same program every time the file is the
same. A path that does not name a file of the program's directory is
E0612 (`comptime.embed-failed`), and a path that is not a string
literal is E0613 (`comptime.embed-path`).

## Code for one target, or for the tests

`target` describes what the program is compiled for: `target.os`,
`target.arch` and `target.runtime`. With `if comptime` a function can
choose code for one system, and both branches are still checked, so a
branch for a system you do not use keeps compiling. `@cfg` leaves out a
declaration whose condition does not hold:

<!-- file: examples/25-compile-time/targets.sisu -->
```sisu
# `target` and `@cfg` choose code while the program compiles.

## The separator of the parts of a path on the target's system.
def separator() -> str:
    return if comptime target.os == "windows": "\\" else: "/"

@cfg(test)
def greeting() -> str:
    return "hei from a test"

@cfg(not test)
def greeting() -> str:
    return "hei"

def main():
    print("{greeting()}: paths are written home{separator()}aino")

test "a test has its own greeting":
    assert greeting() == "hei from a test"
```

<!-- output: examples/25-compile-time/targets.console -->
```console
$ sisu run targets.sisu
hei: paths are written home/aino
$ sisu test targets.sisu
test "a test has its own greeting" ... ok
1 passed, 0 failed, 0 blocked
```

- `@cfg(test)` keeps a declaration in the builds that run the tests,
  and `@cfg(not test)` in the others: `sisu run` builds the program
  without its tests, so it gets the second `greeting`.
- `all(...)` and `any(...)` combine conditions, and `feature="NAME"`
  holds when a feature of the package is on, as chapter 17 showed.
- An item that `@cfg` leaves out is not in the program at all, and
  neither is a field, a variant or a method of a type that it leaves
  out, so a use of it is an error of the build that leaves it out.

## What is still to come

Three parts of compile-time code are designed in
[08](../../proposal/08-metaprogramming.md) and
[10](../../proposal/10-tooling.md) and not in the compiler yet.
`sisu eval` will render a module of `const` items as JSON, TOML or YAML
for configuration files, checking their refinement types and
invariants on the way (D-063). `@diagnostic_name` will let a library
say how its types are written in diagnostics, once compile-time code
can make types; until then it is E0900. And a `comptime` function
that parses a string literal, a route or a query, will report a mistake
at its place inside the literal; today a failed `comptime` call points
at the call.

## Exercise 25.1: predict the compile-time values

Open
[`manual/exercises/25-compile-time/predict_comptime.sisu`](../exercises/25-compile-time/predict_comptime.sisu):

<!-- file: exercises/25-compile-time/predict_comptime.sisu -->
```sisu
##! Exercise 25.1, predict the compile-time values. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of what a compile-time expression
##! gives, written as `print` would write it. Write each as a literal,
##! then run `sisu test predict_comptime.sisu`. The hints are in chapter
##! 25 of the tutorial, under "Exercise 25.1: predict the compile-time
##! values".

## A sauna's stove.
struct Stove:
    celsius: int
    stones_kg: int = 60
    lit: bool = False

## What heats a stove.
enum Fuel:
    Wood
    Electric(kilowatts: int)
    Gas(litres: int, pressure: f64)

## How many fields `T` has.
comptime def field_count(T: type) -> int:
    return len(reflect.fields_of(T))

## The names of the fields of `T` that have a default.
comptime def defaults(T: type) -> list[str]:
    var names: list[str] = []
    comptime for f in reflect.fields_of(T):
        if f.default:
            names.push(f.name)
    return names

## How many fields each variant of `T` has.
comptime def variant_sizes(T: type) -> list[int]:
    var sizes: list[int] = []
    comptime for v in reflect.variants_of(T):
        sizes.push(len(v.fields))
    return sizes

## The sum of the numbers from 1 to `n`.
def triangle(n: int) -> int:
    var total = 0
    for k in range(1, n + 1):
        total += k
    return total

const FIELDS: str = todo("field_count(Stove)")
const DEFAULTS: str = todo("defaults(Stove)")
const SIZES: str = todo("variant_sizes(Fuel)")
const NAME: str = todo("reflect.name_of(Fuel)")
const TRIANGLE: str = todo("triangle(100)")

test "the fields of a struct":
    assert "{comptime field_count(Stove)}" == FIELDS

test "the fields with defaults":
    assert "{comptime defaults(Stove)}" == DEFAULTS

test "the fields of each variant":
    assert "{comptime variant_sizes(Fuel)}" == SIZES

test "a type's name":
    assert "{comptime reflect.name_of(Fuel)}" == NAME

test "ordinary code at compile time":
    assert "{comptime triangle(100)}" == TRIANGLE
```

<!-- output: solutions/25-compile-time/predict_comptime.console -->
```console
$ sisu test predict_comptime.sisu
warning[W0001]: typed hole `field_count(Stove)` of type `str`
  --> predict_comptime.sisu:47:21
   |
47 | const FIELDS: str = todo("field_count(Stove)")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `defaults(Stove)` of type `str`
  --> predict_comptime.sisu:48:23
   |
48 | const DEFAULTS: str = todo("defaults(Stove)")
   |                       ^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `variant_sizes(Fuel)` of type `str`
  --> predict_comptime.sisu:49:20
   |
49 | const SIZES: str = todo("variant_sizes(Fuel)")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `reflect.name_of(Fuel)` of type `str`
  --> predict_comptime.sisu:50:19
   |
50 | const NAME: str = todo("reflect.name_of(Fuel)")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `triangle(100)` of type `str`
  --> predict_comptime.sisu:51:23
   |
51 | const TRIANGLE: str = todo("triangle(100)")
   |                       ^^^^^^^^^^^^^^^^^^^^^

test "the fields of a struct" ... blocked by todo("field_count(Stove)")
test "the fields with defaults" ... blocked by todo("defaults(Stove)")
test "the fields of each variant" ... blocked by todo("variant_sizes(Fuel)")
test "a type's name" ... blocked by todo("reflect.name_of(Fuel)")
test "ordinary code at compile time" ... blocked by todo("triangle(100)")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

`reflect.fields_of` lists every field of a struct, and `f.default` says
whether one has a default value. A list of `str` is written as `print`
writes it, its items in quotes.

</details>

<details>
<summary>Hint 2</summary>

A variant's `fields` are the values it carries: none for a variant
without parentheses. Compile-time code computes what the same code
would at run time.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/25-compile-time/predict_comptime.sisu`](../solutions/25-compile-time/predict_comptime.sisu).

## Exercise 25.2: a constant over its budget

Open
[`manual/exercises/25-compile-time/journeys.sisu`](../exercises/25-compile-time/journeys.sisu):

<!-- file: exercises/25-compile-time/journeys.sisu -->
```sisu
##! Exercise 25.2, a constant over its budget. Kind: fix. Level: 1.
##!
##! `LONGEST` is worked out while the program compiles: the number below
##! a thousand whose journey to 1, by the steps of chapter 4's Collatz
##! exercise, is the longest. The file does not compile. Run
##! `sisu test journeys.sisu`, read the diagnostic, and fix the program,
##! not the tests, so that every test passes. The hints are in chapter 25
##! of the tutorial, under "Exercise 25.2: a constant over its budget".

## The number of steps from `n` to 1, halving an even number and taking
## an odd one to three times itself plus one.
def steps(n: int) -> int:
    var count = 0
    var k = n
    while k != 1:
        k = if k % 2 == 0: k // 2 else: 3 * k + 1
        count += 1
    return count

## The number below `limit` whose journey to 1 has the most steps.
def longest_below(limit: int) -> int:
    var best = 1
    for n in range(1, limit):
        if steps(n) > steps(best):
            best = n
    return best

const LONGEST: int = comptime longest_below(1000)

test "a few journeys":
    assert steps(1) == 0
    assert steps(6) == 8

test "the longest journey below a thousand":
    assert LONGEST == 871
    assert steps(LONGEST) == 178
```

<!-- output: solutions/25-compile-time/journeys.console -->
```console
$ sisu test journeys.sisu
error[E0603]: the compile-time evaluation in `LONGEST` used up its budget of 1000000 steps; raise it with `@comptime_budget(steps=N)` on `LONGEST`
  --> journeys.sisu:28:22
   |
28 | const LONGEST: int = comptime longest_below(1000)
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

E0603 names the item and the attribute that raises its budget. A
budget of a few million steps is enough here.

</details>

<details>
<summary>Hint 2</summary>

There is a second fix: `longest_below` counts the steps of `best` again
for every `n`. Keeping that count in a variable halves the work, and
the default budget is then enough. Raise the budget when the work is
needed, and do less work when it is not.

</details>

When every test passes, compare your answer with
[`manual/solutions/25-compile-time/journeys.sisu`](../solutions/25-compile-time/journeys.sisu).

## Exercise 25.3: a template that does not unfold

Open
[`manual/exercises/25-compile-time/csv.sisu`](../exercises/25-compile-time/csv.sisu):

<!-- file: exercises/25-compile-time/csv.sisu -->
```sisu
##! Exercise 25.3, a template that does not unfold. Kind: fix. Level: 2.
##!
##! `to_csv` writes a list of values as comma-separated values: a line of
##! column names, then a line for each value. A struct has a column for
##! each field; any other value has one column, `value`. The file does not
##! compile. Run `sisu test csv.sisu`, read the diagnostic, and fix
##! `to_csv`, not the tests, so that every test passes. The hints are in
##! chapter 25 of the tutorial, under "Exercise 25.3: a template that does
##! not unfold".

## A turn in the sauna.
struct Booking:
    name: String
    hour: int

## `rows` as comma-separated values, with a line of column names first.
def to_csv[T](rows: list[T]) -> String:
    var names: list[String] = []
    for f in reflect.fields_of(T):
        names.push(f.name.to_string())
    var lines = [names.join(",")]
    for row in rows:
        var cells: list[String] = []
        comptime for f in reflect.fields_of(T):
            cells.push("{row.[f]}")
        lines.push(cells.join(","))
    return lines.join("\n")

test "bookings":
    let rows = [Booking(name="Aino", hour=17), Booking(name="Eero", hour=18)]
    assert to_csv(rows) == "name,hour\nAino,17\nEero,18"

test "no bookings":
    let rows: list[Booking] = []
    assert to_csv(rows) == "name,hour"

test "numbers":
    assert to_csv([3, 1, 4]) == "value\n3\n1\n4"
```

<!-- output: solutions/25-compile-time/csv.console -->
```console
$ sisu test csv.sisu
error[E0609]: making `to_csv[Booking]` stopped at compile time: a list is a compile-time value with no run-time form
  --> csv.sisu:31:12
   |
31 |     assert to_csv(rows) == "name,hour\nAino,17\nEero,18"
   |            ^^^^^^^^^^^^
   |
19 |     for f in reflect.fields_of(T):
   |              -------------------- the template, where it stopped
  = fix: unroll the loop at compile time with `comptime for`: `comptime `

error[E0324]: the type of `f` cannot be inferred here; add a type annotation
  --> <to_csv[int]>/main.sisu:4:20
  |
4 |         names.push(f.name.to_string())
  |                    ^

error[D0006]: the program was not run: errors found (2)
```

<details>
<summary>Hint 1</summary>

The first diagnostic points at a loop over `reflect.fields_of(T)`,
which exists only while compiling, and its fix makes the loop one that
the compiler unrolls. The loop over the rows is the other kind: it runs
over the values, when the program runs.

</details>

<details>
<summary>Hint 2</summary>

The second diagnostic comes from the copy for `int`, which has no
fields. A list of numbers needs its own branch, with the one column
`value`, as in the section "Code that depends on the shape of a
type".

</details>

When every test passes, compare your answer with
[`manual/solutions/25-compile-time/csv.sisu`](../solutions/25-compile-time/csv.sisu).

## Exercise 25.4: a table built while compiling

Open
[`manual/exercises/25-compile-time/ones.sisu`](../exercises/25-compile-time/ones.sisu):

<!-- file: exercises/25-compile-time/ones.sisu -->
```sisu
##! Exercise 25.4, a table built while compiling. Kind: write. Level: 2.
##!
##! Counting the bits that are set in a file is quicker with a table that
##! gives the count for each of the 256 bytes. Write `ones_in`, which
##! counts the bits of one byte, make `ONES` the table of its counts for
##! the bytes 0 to 255, worked out while the program compiles, and write
##! `count_ones`, which looks each byte of its data up in the table. Run
##! the tests with `sisu test ones.sisu`. The hints are in chapter 25 of
##! the tutorial, under "Exercise 25.4: a table built while compiling".

## How many bits are set in `byte`, from 0 to 255, counted one at a time.
def ones_in(byte: int) -> int:
    return todo("ones_in")

## How many bits are set in each byte, from 0 to 255.
const ONES: list[int] = todo("a table worked out with `comptime`")

## How many bits are set in `data`, from `ONES`.
def count_ones(data: bytes) -> int:
    return todo("count_ones")

test "the bits of a byte":
    assert ones_in(0) == 0
    assert ones_in(0b1011) == 3
    assert ones_in(255) == 8

test "the table":
    assert len(ONES) == 256
    assert ONES[0] == 0 and ONES[0b1011] == 3 and ONES[255] == 8
    assert all([ONES[b] == ones_in(b) for b in range(256)])

test "counting the bits of data":
    assert count_ones(b"") == 0
    assert count_ones(b"\x00\x01\x03") == 3
    assert count_ones("sisu".encode()) == 19
```

<!-- output: solutions/25-compile-time/ones.console -->
```console
$ sisu test ones.sisu
warning[W0001]: typed hole `ones_in` of type `int`
  --> ones.sisu:13:12
   |
13 |     return todo("ones_in")
   |            ^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a table worked out with `comptime`` of type `list[int]`
  --> ones.sisu:16:25
   |
16 | const ONES: list[int] = todo("a table worked out with `comptime`")
   |                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `count_ones` of type `int`
  --> ones.sisu:20:12
   |
20 |     return todo("count_ones")
   |            ^^^^^^^^^^^^^^^^^^

test "the bits of a byte" ... blocked by todo("ones_in")
test "the table" ... blocked by todo("a table worked out with `comptime`")
test "counting the bits of data" ... blocked by todo("count_ones")

test "the bits of a byte" ... blocked by todo("ones_in")
what the test printed:
  in scope at the typed hole `todo("ones_in")`:
    byte = 0

test "counting the bits of data" ... blocked by todo("count_ones")
what the test printed:
  in scope at the typed hole `todo("count_ones")`:
    data = b""

0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

`ones_in` looks at the lowest bit with `& 1` and moves the next one
down with `>> 1`, as chapter 24's section "Working with bits" does,
until nothing is left.

</details>

<details>
<summary>Hint 2</summary>

The table is `ones_in` of each number from 0 to 255, in order, which
one comprehension can make. `comptime` before it makes the compiler
work it out.

</details>

<details>
<summary>Hint 3</summary>

A byte of `data` is a `u8`, and a place in the table is an `int`; a
`u8` widens to an `int`, or `to_int()` says so.

</details>

When every test passes, compare your answer with
[`manual/solutions/25-compile-time/ones.sisu`](../solutions/25-compile-time/ones.sisu).

## Exercise 25.5: a derive that lays out the fields

Open
[`manual/exercises/25-compile-time/form.sisu`](../exercises/25-compile-time/form.sisu):

<!-- file: exercises/25-compile-time/form.sisu -->
```sisu
##! Exercise 25.5, a derive that lays out the fields. Kind: write. Level: 3.
##!
##! `Form` shows a struct's fields as a form, a line for each field: its
##! name, padded with spaces to the length of the longest name of the
##! struct's fields, then " : " and its value, as `print` writes it. Write
##! the body of `form` in the impl that `derive` writes, so that every
##! struct that asks for `@derive(Form)` gets it and every test passes.
##! Run the tests with `sisu test form.sisu`. The hints are in chapter 25
##! of the tutorial, under "Exercise 25.5: a derive that lays out the
##! fields".

trait Form:
    ## `self` as a form, a line for each field.
    def form(self) -> String

    comptime def derive(T: type) -> impl Form for T:
        return impl Form for T:
            def form(self) -> String:
                return todo("form")

## A turn in the sauna.
@derive(Form)
struct Booking:
    name: String
    hour: int
    minutes: int

## A sauna's stove.
@derive(Form)
struct Stove:
    celsius: f64
    lit: bool

test "a booking":
    let b = Booking(name="Aino", hour=18, minutes=90)
    assert b.form() == "name    : Aino\nhour    : 18\nminutes : 90"

test "a stove":
    let s = Stove(celsius=78.5, lit=True)
    assert s.form() == "celsius : 78.5\nlit     : True"
```

<!-- output: solutions/25-compile-time/form.console -->
```console
$ sisu test form.sisu
warning[W0001]: typed hole `form` of type `String`
  --> form.sisu:22:9
   |
22 | @derive(Form)
   |         ^^^^
  = note: in the impl of `Form` that the derive writes for `Booking`: `return todo("form")`

warning[W0001]: typed hole `form` of type `String`
  --> form.sisu:29:9
   |
29 | @derive(Form)
   |         ^^^^
  = note: in the impl of `Form` that the derive writes for `Stove`: `return todo("form")`

test "a booking" ... blocked by todo("form")
test "a stove" ... blocked by todo("form")

test "a booking" ... blocked by todo("form")
what the test printed:
  in scope at the typed hole `todo("form")`:
    self = Booking(name="Aino", hour=18, minutes=90)

test "a stove" ... blocked by todo("form")
what the test printed:
  in scope at the typed hole `todo("form")`:
    self = Stove(celsius=78.5, lit=True)

0 passed, 0 failed, 2 blocked
```

The warnings stand at each `@derive(Form)`, once for each struct, and
their notes quote the line of the impl that the derive wrote: the hole
is in the template, so each derived impl has one. Each test stops at
the hole with `self` in scope, the value whose form it makes, which its
block after the list shows.

<details>
<summary>Hint 1</summary>

A form has a line for each field, in the order of the fields, joined
with a line break. The section "Writing a derive" builds a list the
same way.

</details>

<details>
<summary>Hint 2</summary>

The width is the longest of the fields' names, and `len(f.name)` is
known while compiling. A first `comptime for` can find the width, and
a second can write the lines.

</details>

<details>
<summary>Hint 3</summary>

Pad a name with as many spaces as it is shorter than the width:
`repeat` makes a run of spaces.

</details>

When every test passes, compare your answer with
[`manual/solutions/25-compile-time/form.sisu`](../solutions/25-compile-time/form.sisu).

## What you learned

- `comptime e` runs ordinary code while the program compiles, with the
  semantics of run time, and the program keeps the value;
  `const X = comptime f()` is a constant computed once.
- `comptime assert` checks while compiling (E0602 when it fails), and
  a `comptime def` runs only then (E0605).
- Compile-time code reaches nothing in the world (E0601), sees only
  what is known while compiling (E0600), and keeps only values with a
  literal form (E0604).
- Each `comptime` expression has a budget of a million steps;
  `@comptime_budget(steps=N)` raises an item's budget (E0603).
- `reflect` gives a type's name, fields and variants; `comptime for`
  unrolls a loop over them, `if comptime` decides a branch, and
  `value.[f]` reads a field. A generic function that reflects is made
  for each type it is called with (E0609 when one cannot be made).
- A trait's `comptime def derive` returns `impl Trait for T:`, which
  `@derive(Trait)` writes for a type and the compiler checks.
- Attributes are data that `reflect.attributes_of` reads; `embed`
  makes a file part of the program; `target`, `if comptime` and `@cfg`
  choose code for a target, for the tests or for a feature.

Chapter 26 is about `pure` functions and what they promise, and about
running loops in parallel with `std.par` while getting the same result
on one worker as on many.
