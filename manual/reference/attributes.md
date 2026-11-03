# Attributes

This page lists Sisu's attributes: how an attribute is written and where
it may stand, how compile-time code reads attributes as data, and, for
each attribute that the compiler or the toolchain reads, where it has a
meaning, what it does and the diagnostics that guard it. The
declarations that attributes stand on are [R3](declarations.md); the
compile-time code that reads them is [R12](../README.md#the-reference),
when it is written.

**See also:** the tutorial's [chapter 7](../tutorial/07-structs-and-enums.md#comparing-and-copying-with-derive)
(`@derive`), [chapter 13](../tutorial/13-sharing.md#values-that-must-be-used-up)
(`@must_consume`), [chapter 14](../tutorial/14-traits.md#a-new-type-for-an-old-value)
(`@delegate`), [chapter 19](../tutorial/19-testing.md#suites-tags-and-choosing-tests)
(`@tag` and `@refines`), [chapter 25](../tutorial/25-compile-time.md#attributes-are-data)
(attributes as data, `@comptime_budget` and `@cfg`),
[chapter 27](../tutorial/27-systems.md#data-laid-out-as-c-lays-it-out)
(`@repr(c)`, `@export`, `@blocking` and the resource attributes) and
[chapter 28](../tutorial/28-code-that-lasts.md#keeping-an-old-name)
(`@deprecated`); the proposal's [02,
"Declarations"](../../proposal/02-syntax.md#declarations) and [08,
"What you cannot do"](../../proposal/08-metaprogramming.md#what-you-cannot-do-and-the-intended-workaround);
the compiler's [architecture notes](../../compiler/docs/architecture.md),
"Names and the standard library" and "Traits and generics";
[R17, Diagnostics](diagnostics.md#codes-and-their-ranges) for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. That attributes are written `@name(args)` and are
compile-time data, not calls, is decided (D-046).

## Writing an attribute

An attribute is `@NAME` or `@NAME(ARGUMENTS)` on a line of its own,
before the declaration it belongs to.

- Several attributes stand one to a line, in any order. A blank line
  between an attribute and its declaration is allowed, and the
  formatter removes it.
- `NAME` is a name. The parser also takes names joined by dots, as
  `@orm.table`, which nothing reads yet.
- `ARGUMENTS` are written as a call's: expressions separated by commas,
  and keyword arguments `NAME=VALUE`. They are never evaluated
  ([Attributes are data](#attributes-are-data)), so an argument may
  name what is not in scope, such as a column of a table.

<!-- file: examples/reference-attributes/forms.sisu -->
```sisu
# Attributes stand on their own lines before a declaration or a member.
@derive(Eq, Hash)
@table("saunas", key="name")
struct Sauna:
    name: String
    @unit("celsius")
    heat: int = 80

enum Stove:
    Wood
    @since("2026")
    Electric

@deprecated("use `heat_of`")
def heat(sauna: Sauna) -> int:
    return sauna.heat

def main():
    let home = Sauna("Kotiharju")
    print("{home.name} {heat(home)} {Stove.Electric:?} {home == home}")
```

<!-- output: examples/reference-attributes/forms.out -->
```text
Kotiharju 80 Electric True
```

`@derive` and `@deprecated` are Sisu's; `@table`, `@unit` and `@since`
are data for a library or a tool to read.

An attribute on the line of its declaration is E0117
([R17](diagnostics.md#e0117)), followed by E0106
(`parse.expected-declaration`) for the declaration's body, and one
before a statement, or at the end of a file, where no declaration
follows, is E0106:

<!-- file: examples/reference-attributes/placement.sisu -->
```sisu
# An attribute on the line of its item, and one before a statement.
@derive(Eq) struct Point:
    x: int

def main():
    @unused
    let origin = Point(0)
    print(origin == Point(0))
```

<!-- output: examples/reference-attributes/placement.console -->
```console
$ sisu run placement.sisu
error[E0117]: unexpected `struct` after the end of the statement
  --> placement.sisu:2:13
  |
2 | @derive(Eq) struct Point:
  |             ^^^^^^

error[E0106]: expected a declaration, found an indented block
  --> placement.sisu:3:1
  |
3 |     x: int
  | ^

error[E0106]: expected a declaration, found `let`
  --> placement.sisu:7:5
  |
7 |     let origin = Point(0)
  |     ^^^

error[D0006]: the program was not run: errors found (3)
```

An attribute on a parameter is E0100 ([R17](diagnostics.md#e0100)).

**Status.** In stage 0 (D-046).

## Where an attribute may stand

| Declaration | Attributes stand |
|---|---|
| an item: `def` (with `pure`, `comptime` or `unsafe`), `struct`, `view struct`, `enum`, `error`, `trait`, `impl`, `type`, `const`, `import`, `from ... import`, `test`, `suite`, `bench`, an `extern` block | before it, and before its `pub` |
| a member: a field, a method of a struct, an enum, an impl or a trait, a variant of an enum or an error set, a function of an `extern` block, an item of a `suite` | before it, inside the body |

Nothing else takes an attribute: not a statement, an expression, a
parameter, a `case` arm or a closure. Each attribute that Sisu reads has
a meaning on some of these declarations only, which its section names.

**Stage 0 differs.** Stage 0 accepts any attribute on any declaration of
the table without a word: a name that nothing reads, a misspelt one such
as `@deprecatd`, and one that Sisu reads where it has no meaning, such as
`@derive(Debug)` on a function or `@tag("slow")` on a suite, which tags
none of the suite's tests.

**Status.** In stage 0, with the difference noted.

## Attributes are data

An attribute does nothing by itself. It is data on its declaration,
which the compiler, the tools and compile-time code read (D-046): it
never runs, and it cannot wrap, replace or change the declaration, so
Python's decorators, which are calls, have no counterpart. A function
that adds behaviour to another is a function that calls it, and a
framework that needs to know its handlers reads their attributes at
compile time
([08](../../proposal/08-metaprogramming.md#what-you-cannot-do-and-the-intended-workaround)).

A library may define attributes of its own, since a name that Sisu
does not read is accepted. `reflect.attributes_of(T)` gives the
attributes on the declaration of the type `T`, in the order written,
each with its `name` and its `args`, the text of each argument as
written: a string's quotes are part of its text, and a keyword argument
is `NAME=VALUE`.

<!-- file: examples/reference-attributes/data.sisu -->
```sisu
# Attributes are data that compile-time code reads.
@table("bookings")
@index(hour, unique=True)
struct Booking:
    name: String
    hour: int

comptime def describe(T: type) -> String:
    var out = String.new(reflect.name_of(T))
    comptime for a in reflect.attributes_of(T):
        out = "{out} @{a.name}"
        comptime for arg in a.args:
            out = "{out} [{arg}]"
    return out

def main():
    print(comptime describe(Booking))
```

<!-- output: examples/reference-attributes/data.out -->
```text
Booking @table ["bookings"] @index [hour] [unique=True]
```

**Stage 0 differs.** `reflect.attributes_of` takes a struct, an enum or
an error set, and gives an empty list for a type of the standard
library. The
attributes of a function, a field, a method or a variant reach no
compile-time code, and no query lists the functions of a package, so the
framework of 08 that collects its routed functions at compile time
cannot be written yet.

**Status.** In stage 0 (D-046, D-018), with the differences noted.

## The attributes Sisu reads

| Attribute | Stands on | Does | Status |
|---|---|---|---|
| `@derive(TRAIT, ...)` | a struct, an enum, an error set | writes the traits' impls from the fields ([Deriving traits](#deriving-traits)) | in stage 0 |
| `@delegate(TRAIT, ..., to="FIELD")` | a struct | forwards the traits to a field ([Delegating traits to a field](#delegating-traits-to-a-field)) | in stage 0 |
| `@must_consume` | a struct, an enum | makes each value end in an `own` parameter ([Values that must be consumed](#values-that-must-be-consumed)) | in stage 0 |
| `@cfg(CONDITION)` | an item, an item of a suite, a method of an impl or a trait | keeps the declaration only where the condition holds ([Conditional compilation](#conditional-compilation)) | in stage 0 |
| `@comptime_budget(steps=N)` | a function, a constant | raises its compile-time budget ([Compile-time budgets](#compile-time-budgets)) | in stage 0 |
| `@tag("NAME", ...)` | a test | names it for `--tag` and `--skip-tag` ([Tags on tests](#tags-on-tests)) | in stage 0 |
| `@refines(REFERENCE)` | a function | tests it against a reference ([Refinement tests](#refinement-tests)) | in stage 0; leaning |
| `@fixture(scope="...")` | a function | makes a value for the tests that name it ([Fixtures](#fixtures)) | designed |
| `@repr(c)`, `@repr(u8)` | a struct, an enum | fixes the layout ([The layout of a type](#the-layout-of-a-type)) | partly in stage 0 |
| `@export("NAME")` | a function | makes it callable from C ([Exported functions](#exported-functions)) | designed; checked in stage 0 |
| `@blocking` | a function of an `extern` block | says that a call may block ([Foreign functions that block](#foreign-functions-that-block)) | designed; checked in stage 0 |
| `@no_alloc`, `@no_block`, `@stack(max=N)` | a function | limit what it may do ([Resource limits](#resource-limits)) | leaning |
| `@float(fast)` | a function | allows fast floating point inside it ([Fast floating point](#fast-floating-point)) | designed |
| `@deprecated("MESSAGE")`, `@fix_inline` | an item; a function | warn at each use, and migrate the uses ([Deprecation and migrations](#deprecation-and-migrations)) | designed |
| `@preview("NAME")` | not designed yet | asks for a feature that is not stable ([Preview features](#preview-features)) | leaning |
| `@diagnostic_name(f)` | a type made at compile time | says how diagnostics print it ([Names of types in diagnostics](#names-of-types-in-diagnostics)) | leaning |
| `@intrinsic`, `@builtin`, `@structural`, `@keeps`, `@sealed`, `@marker`, `@internal` | the standard library's declarations | ([Attributes of the standard library](#attributes-of-the-standard-library)) | in stage 0 |

**Status.** As each section says.

## Deriving traits

`@derive(TRAIT, ...)` on a struct, an enum or an error set gives the
type an impl of each trait, written from its fields by the trait's
derive at compile time (D-018). It takes:

- `Copy`, when every field is `Copy` (E0244 otherwise,
  [R17](diagnostics.md#e0244)); a type is `Copy` only through the
  derive ([R7](ownership.md#copy-types-and-explicit-copies));
- `Debug`: the name and the fields as they are written, the form `{x:?}`
  prints, which every type has without asking;
- `Eq`: equal when the variants are and every field is;
- `PartialOrd` and `Ord`: by variant in the order declared, then field
  by field, the first that differs deciding;
- `Hash`: the fields' hashes in order, after an enum's variant;
- `Default`, for a struct: each field's own default where it has one,
  and its type's `default()` otherwise;
- `Plain` of `std.mem`, for a `@repr(c)` struct whose fields are plain
  ([The layout of a type](#the-layout-of-a-type));
- any trait that declares a `comptime def derive`, the program's own
  included ([R12](../README.md#the-reference), when it is written).

The impl of a generic type bounds each type parameter by the trait, as
`impl[T: Eq] Eq for Pair[T]`.

<!-- file: examples/reference-attributes/derive.sisu -->
```sisu
# `@derive` writes impls from a type's fields.
@derive(Copy, Eq, PartialOrd, Ord, Hash, Default)
struct Version:
    major: int
    minor: int = 0

@derive(Eq, Hash)
enum Light:
    Red
    Amber(seconds: int)

@derive(Eq)
error Problem:
    Missing(name: String)

def main():
    let old = Version(1, 2)
    let new = Version(major=2)
    print("{old < new} {sorted([new, old])} {Version.default():?}")
    print({Light.Red: "stop", Light.Amber(3): "wait"}[Light.Amber(3)])
    print(Missing("hat".to_string()) == Missing("hat".to_string()))
```

<!-- output: examples/reference-attributes/derive.out -->
```text
True [Version(major=1, minor=2), Version(major=2, minor=0)] Version(major=0, minor=0)
wait
True
```

A trait that has no derive, such as `Display`, is E0243
([R17](diagnostics.md#e0243)); a field without the trait is E0244; a
derived trait whose supertraits the type does not have is E0242
([R17](diagnostics.md#e0242)); an impl written beside a derived one is
E0235 ([R17](diagnostics.md#e0235)); and a derive that stops while it
unfolds, such as `Default` for an enum, is E0608
(`comptime.derive-failed`), at the `@derive` with a label where the
trait's template stopped.

<!-- file: examples/reference-attributes/derive_needs.sisu -->
```sisu
# `Ord` needs `Eq` and `PartialOrd`, which the type does not derive.
@derive(Ord)
struct Version:
    major: int
    minor: int

def main():
    print(sorted([Version(2, 0), Version(1, 4)]))
```

<!-- output: examples/reference-attributes/derive_needs.console -->
```console
$ sisu run derive_needs.sisu
error[E0242]: `Version` implements `Ord`, which requires `Eq`, but `Version` does not implement `Eq`
  --> derive_needs.sisu:2:9
  |
2 | @derive(Ord)
  |         ^^^

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** `@derive(Into, From)`, which
[03](../../proposal/03-types-and-generics.md#type-aliases-and-newtypes)
gives newtypes, `@derive(Encode, Decode)` of `std.json` and
`@derive(Arbitrary)` of `std.test` are E0243, whose list of the traits
it takes leaves out `Default`. A diagnostic in a derived impl is placed
in `<derive>/main.sisu`, a module that the compiler writes, so E0235 for
an impl beside a derived one points there and at the written impl, not
at the `@derive`.

**Status.** In stage 0 (D-018), except `Into`, `From`, `Encode`,
`Decode` and `Arbitrary`.

## Delegating traits to a field

`@delegate(TRAIT, ..., to="FIELD")` on a struct writes an impl of each
trait that forwards to the field, so that a new type over a value keeps
the abilities it should keep and no others
([R2](types.md#type-aliases-and-newtypes), D-012). It takes the
operator traits `Add`, `Sub`, `Mul`, `Div`, `Rem` and `Neg`, and `Eq`,
`PartialOrd`, `Ord`, `Hash`, `Display`, `Debug` and `Len`.

<!-- file: examples/reference-attributes/delegate.sisu -->
```sisu
# `@delegate` forwards traits to a field.
@delegate(Add, Sub, PartialOrd, Display, to="value")
struct Celsius:
    value: f64

@delegate(Len, Display, to="items")
struct Basket:
    items: list[String]
    owner: String

def main():
    let warm = Celsius(20.5) + Celsius(1.5) - Celsius(2.0)
    print("{warm} {warm > Celsius(19.0)}")
    let basket = Basket(["leipä", "maito"], "Aino")
    print("{len(basket)} {basket} for {basket.owner}")
```

<!-- output: examples/reference-attributes/delegate.out -->
```text
20.0 True
2 ["leipä", "maito"] for Aino
```

- An operator trait returns `Self`, so it can be delegated only to a
  struct's one field (E0252, `resolve.delegate-self`).
- Another trait is E0250 (`resolve.not-delegable`); a `@delegate`
  without `to=`, or naming a field the struct does not have, is E0251
  (`resolve.delegate-to`); a field whose type lacks the trait is E0253
  (`resolve.delegate-field`).

<!-- file: examples/reference-attributes/delegate_self.sisu -->
```sisu
# An operator returns `Self`, so only a one-field struct delegates it.
@delegate(Add, Display, to="items")
struct Basket:
    items: list[String]
    owner: String

def main():
    print(Basket(["leipä"], "Aino") + Basket(["maito"], "Eero"))
```

<!-- output: examples/reference-attributes/delegate_self.console -->
```console
$ sisu run delegate_self.sisu
error[E0252]: `Add` returns a `Basket`, so `Basket` can delegate it only when `items` is its one field
  --> delegate_self.sisu:2:11
  |
2 | @delegate(Add, Display, to="items")
  |           ^^^

error[E0310]: `+` cannot be applied to `Basket` and `Basket`
  --> delegate_self.sisu:8:11
  |
8 |     print(Basket(["leipä"], "Aino") + Basket(["maito"], "Eero"))
  |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

**Status.** In stage 0 (D-012).

## Values that must be consumed

`@must_consume` on a struct or an enum makes each value of the type end
in an `own` parameter, usually a method that takes `own self`, before it
goes out of scope: a value that goes out of scope unconsumed is E0417
([R17](diagnostics.md#e0417)), and one that an early `return`, `break`
or `?` leaves behind is E0418 (`borrow.must-consume-exit`). A
collection of such values is must-consume too. The rules are [R7](ownership.md#values-that-must-be-consumed),
and cleanup that can fail is [R8](errors.md#cleanup-that-can-fail).

**Status.** In stage 0 (D-054; containers lean, D-064).

## Conditional compilation

`@cfg(CONDITION, ...)` keeps its declaration only in the builds where
every condition holds; elsewhere the declaration is left out before
names are resolved, as if it were not written. It stands on an item, an
item of a suite, an `extern` block, or a method of an impl or a trait. A
condition is one of:

| Condition | Holds when |
|---|---|
| `test` | the program is built with its tests: `sisu test`, `sisu bench`, `sisu check` and the tools that check, but not `sisu run`, `sisu build` or `sisu sir` |
| `feature="NAME"` | the package's feature `NAME` is on ([R13](modules.md#features)) |
| `not CONDITION` | the condition does not hold |
| `all(CONDITION, ...)`, `any(CONDITION, ...)` | every one, or at least one, holds |

A build without its tests leaves out the `test`, `suite` and `bench`
items and the doc examples too, so a test may use an `@cfg(test)`
helper. The same file prints one thing when it runs and passes its
test:

<!-- file: examples/reference-attributes/cfg.sisu -->
```sisu
# `@cfg` keeps a declaration only in the builds where it holds.
@cfg(test)
def sample() -> list[int]:
    return [3, 1, 2]

@cfg(not test)
def mode() -> str:
    return "a run"

@cfg(test)
def mode() -> str:
    return "the tests"

def main():
    print("built for {mode()}")

test "sorts the sample":
    assert sorted(sample()) == [1, 2, 3]
    assert mode() == "the tests"
```

<!-- output: examples/reference-attributes/cfg.console -->
```console
$ sisu run cfg.sisu
built for a run
$ sisu test cfg.sisu
test "sorts the sample" ... ok
1 passed, 0 failed, 0 blocked
```

Anything else is E0266 (`resolve.bad-cfg`), and its declaration is kept;
a feature that the package does not declare is W0006
(`resolve.unknown-feature`). A condition on the target is written in a
body with `if comptime target.os == "linux":`, whose branches are both
checked ([R12](../README.md#the-reference), when it is written):

<!-- file: examples/reference-attributes/cfg_condition.sisu -->
```sisu
# `@cfg` takes conditions on the build, not expressions.
@cfg(target.os == "linux")
def separator() -> str:
    return "/"

def main():
    print("usr{separator()}bin")
```

<!-- output: examples/reference-attributes/cfg_condition.console -->
```console
$ sisu run cfg_condition.sisu
error[E0266]: `@cfg` takes a condition: `test`, `feature="NAME"`, `not` and a condition, or `all(...)` or `any(...)` of conditions; this is none of them
  --> cfg_condition.sisu:2:6
  |
2 | @cfg(target.os == "linux")
  |      ^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** `@cfg` on a field, on a variant, or on a method in
the body of a struct or an enum is accepted and ignored: the declaration
is kept in every build. A method in an impl is left out as the rule
says.

**Status.** In stage 0 (08), with the difference noted.

## Compile-time budgets

`@comptime_budget(steps=N)` on a function or a constant sets the budget
of the compile-time evaluation in that item to `N` steps of the
compiler's interpreter, instead of 1,000,000. The budget belongs to the
item that spends it, so a package that uses this one never raises a
budget for it (D-063). An evaluation that spends its budget is E0603
(`comptime.budget`), whose message names the item; an attribute without
`steps=` and a positive number is E0606 (`comptime.budget-attribute`):

<!-- file: examples/reference-attributes/budget.sisu -->
```sisu
# `@comptime_budget` takes the number of steps by name.
def triangle(n: int) -> int:
    var total = 0
    for i in range(n + 1):
        total += i
    return total

@comptime_budget(2_000_000)
const BIG: int = comptime triangle(200_000)

def main():
    print(BIG)
```

<!-- output: examples/reference-attributes/budget.console -->
```console
$ sisu run budget.sisu
error[E0606]: `@comptime_budget` takes `steps=N`, a positive number of steps
  --> budget.sisu:8:1
  |
8 | @comptime_budget(2_000_000)
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

The evaluation and its budget are [R12](../README.md#the-reference),
when it is written.

**Status.** In stage 0 (D-063 leans).

## Tags on tests

`@tag("NAME", ...)` on a test, in a suite or not, gives it tags: `sisu
test --tag NAME` runs the tests that have the tag, and `--skip-tag NAME`
those that do not ([R15](testing.md#the-forms-of-a-test),
[R16](sisu-command.md#sisu-test)).

**Stage 0 differs.** `@tag` on a suite is accepted and tags none of the
suite's tests.

**Status.** In stage 0 (D-057), with the difference noted.

## Refinement tests

`@refines(REFERENCE)` on a function makes `sisu test` check, on many
cases of the parameters' types, that the function gives what the
function `REFERENCE` gives; `@refines(REFERENCE, tolerance=Ulps(N))`
lets an `f64` result differ by `N` units in the last place. Both are
functions of the module with the same parameters and result (E0821,
`test.refines-mismatch`, with the reason). The rule and an example are
[R15](testing.md#refinement-tests).

**Status.** In stage 0 (leaning, D-070).

## Fixtures

`@fixture(scope="module")` on a function makes a value that each test
naming the function's name as a parameter receives, made once for the
scope ([17](../../proposal/17-testing.md#fixtures)).

**Stage 0 differs.** `@fixture` is accepted and does nothing, and a test
with a parameter that is not a capability is E0702
(`type.test-signature`); a fixture is a value that a function makes and
each test calls ([R15](testing.md#fixtures)).

**Status.** Designed; whether it is needed at all is open
([17](../../proposal/17-testing.md#open-questions)).

## The layout of a type

`@repr(c)` on a struct or an enum fixes its layout as C's; a struct's
fields are then in the order declared, each at an offset that is a
multiple of its alignment, with padding where needed
([12](../../proposal/12-interop-and-targets.md#c-interop)). The derive of
`std.mem`'s `Plain`, for types whose every bit pattern is a value, asks
for it on a struct of more than one field (E0608 otherwise), and foreign
functions take such structs
([R7](ownership.md#unsafe-code-and-raw-pointers)):

<!-- file: examples/reference-attributes/repr.sisu -->
```sisu
# `@repr(c)` fixes a struct's layout, which `Plain` asks for.
from std.mem import Plain, read_plain, size_of

@repr(c)
@derive(Copy, Plain)
struct Pixel:
    red: u8
    green: u8
    blue: u8
    alpha: u8

def main():
    let pixel = read_plain[Pixel](b"\xff\x80\x00\xff", 0)
    print("{size_of[Pixel]()} bytes")
    print("{pixel.red} {pixel.green} {pixel.blue} {pixel.alpha}")
```

<!-- output: examples/reference-attributes/repr.out -->
```text
4 bytes
255 128 0 255
```

An enum may also be `@repr(u8)` for foreign code, and may declare its
variants' values, its discriminants; an enum with only unit variants is
a C enum ([03](../../proposal/03-types-and-generics.md#algebraic-data-types)).

**Stage 0 differs.** Stage 0 lays out every struct as C does, with or
without `@repr(c)`, and `Plain`'s derive accepts a struct with any
`@repr`, `@repr(python)` included. `@repr(u8)` changes nothing, and a
variant's value, `Red = 1`, is E0117 ([R2](types.md#structs-and-enums)).

**Status.** `@repr(c)` on a struct in stage 0 for `std.mem`'s `Plain`
(D-081; Q-71 open), with the differences noted; on an enum, and
discriminants, designed ([12](../../proposal/12-interop-and-targets.md)).

## Exported functions

`@export("NAME")` on a function makes it callable from C under the name
`NAME`, with the C ABI ([12](../../proposal/12-interop-and-targets.md#c-interop)).
Writing one is safe, because its parameters and result hold no
reference, view, `Shared`, `Weak`, closure or capability, which foreign
code could not keep alive (E0989, `unsafe.export-signature`, at each one
that does), and a panic that reaches the boundary aborts the process
instead of unwinding into foreign code
([22](../../proposal/22-unsafe.md#foreign-calls)):

<!-- file: examples/reference-attributes/export.sisu -->
```sisu
# An exported function takes nothing that foreign code cannot keep.
@export("sauna_heat")
def heat(celsius: i32, minutes: i32) -> i32:
    return celsius * minutes

@export("sauna_name_length")
def name_length(name: str) -> int:
    return len(name)

def main():
    print(heat(80, 10))
```

<!-- output: examples/reference-attributes/export.console -->
```console
$ sisu run export.sisu
error[E0989]: `name_length` is `@export`ed, and foreign code that calls it cannot keep `str` alive: an exported function's signature holds no reference, view, `Shared`, closure or capability
  --> export.sisu:7:17
  |
7 | def name_length(name: str) -> int:
  |                 ^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** The check of the signature is all: a native build
gives the function no symbol named `NAME`, and the rule of 12 that the
types at the boundary are `@repr(c)` is not checked, so a parameter of a
struct with a `String` in it passes.

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md),
[22](../../proposal/22-unsafe.md); D-080), with its check in stage 0.

## Foreign functions that block

`@blocking` on a function of an `extern` block says that a call of it
may block its thread for an unbounded time, so the scheduler moves the
call off the threads that run tasks; a foreign function without it is a
promise, in the block's safety comment, that its call returns promptly
([22](../../proposal/22-unsafe.md#foreign-calls)). On anything but a
foreign function it is E0990 (`unsafe.blocking-not-foreign`), since a
Sisu function's blocking points are its capability calls and
synchronisation, which the scheduler sees
([R3](declarations.md#foreign-functions)):

<!-- file: examples/reference-attributes/blocking.sisu -->
```sisu
# `@blocking` marks a foreign function, whose call may block its thread.
from std.ffi import c_uint

# safety: `sleep` is POSIX's: it takes seconds and returns those left.
extern "C":
    @blocking
    def sleep(seconds: c_uint) -> c_uint

@blocking
def wait_for_heat() -> int:
    return 80

def main():
    print(wait_for_heat())
```

<!-- output: examples/reference-attributes/blocking.console -->
```console
$ sisu run blocking.sisu
error[E0990]: `@blocking` marks a foreign function whose call may block its thread, and `wait_for_heat` is not a foreign function
  --> blocking.sisu:9:1
  |
9 | @blocking
  | ^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** `@blocking` changes nothing about how a call runs
yet.

**Status.** Designed ([06](../../proposal/06-concurrency-and-capabilities.md),
[22](../../proposal/22-unsafe.md); D-084), with its check in stage 0.

## Resource limits

Three attributes on a function ask the compiler to show that it stays
within a limit, which the realtime code of an audio callback or a
driver needs ([04](../../proposal/04-memory-and-ownership.md#resource-attributes)):

| Attribute | The function, and everything it calls, does not |
|---|---|
| `@no_alloc` | allocate on the heap: no construction or growth of a `list`, `dict`, `String`, `Box` or `Shared` |
| `@no_block` | block: no capability call, lock, channel operation, `join`, `select`, `@blocking` foreign call or yield point, so it runs to completion |
| `@stack(max=N)` | use more than `N` bytes of stack in its deepest chain of calls, recursion allowed only with a `decreases` measure |

- The checks run over the call graph after monomorphisation and report
  an error at the instantiation that breaks the limit; inside such a
  function a `dyn` call, or a call through a closure parameter that is
  not generic, is an error, since it must be assumed to allocate and
  block. Panic paths are exempt.
- The attributes never enter function types (Q-52): `pure` stays the
  one property a function type carries.
- `@stack` arrives with the native backend, which knows the sizes of
  frames.
- `@no_panic`, which asks that no panic be reachable, comes after
  version 0.1; until then `sisu audit --panics` lists the places that
  can panic ([R16](sisu-command.md#sisu-audits-other-reports)).

**Stage 0 differs.** Stage 0 accepts the three attributes and checks
nothing, as it does the attributes of the next sections:

<!-- file: examples/reference-attributes/designed.sisu -->
```sisu
# Designed attributes that stage 0 accepts and does not check.
@no_alloc
@no_block
@stack(max=256)
def levels(gain: int) -> list[int]:
    return [gain, gain * 2]

@deprecated("use `levels`")
@fix_inline
def old_levels(gain: int) -> list[int]:
    return levels(gain)

@float(fast)
def blend(a: f64, b: f64) -> f64:
    return a * 0.5 + b * 0.5

def main():
    print("{old_levels(3)} {blend(1.0, 2.0)}")
```

<!-- output: examples/reference-attributes/designed.out -->
```text
[3, 6] 1.5
```

**Status.** Leaning (D-059); their place outside function types is
decided (Q-52).

## Fast floating point

`@float(fast)` on a function allows the compiler to contract and
reassociate floating-point arithmetic inside it, fusing a multiplication
and an addition or reordering a sum, for speed. Everywhere else floating
point is strict, one rounding for each operation, so a `pure` function
gives the same bits on every target; inside the function that guarantee
is given up, visibly ([R2](types.md#floating-point), D-074).

**Stage 0 differs.** `@float(fast)` is accepted and changes nothing
([Resource limits](#resource-limits) shows it).

**Status.** Designed (D-074).

## Deprecation and migrations

`@deprecated("MESSAGE")` on an item makes each use of it a warning that
names the replacement, with a rewrite by `sisu fix` where one is
mechanical ([13](../../proposal/13-evolution-and-governance.md#deprecation)).
`@fix_inline` on a deprecated function whose body is written in terms of
the new API lets `sisu fix --deps` replace the calls in the packages
that depend on it with that body, moving borrows, `own` arguments and
`defer` into the caller, so their behaviour is kept by construction
(D-067).

**Stage 0 differs.** Both are accepted and do nothing: a call of a
deprecated function gets no warning ([Resource limits](#resource-limits)
shows it), and `sisu fix` is designed ([R16](sisu-command.md#sisu-fix)).

**Status.** Designed ([13](../../proposal/13-evolution-and-governance.md)); `@fix_inline` leans (D-067).

## Preview features

`@preview("NAME")` asks for a language feature that is not stable yet,
by its name. A feature stays a preview for at least one release, may
change or go while it is one, and is made stable or removed within four
releases, two years ([13](../../proposal/13-evolution-and-governance.md#release-cadence),
D-072). Where the attribute stands is not designed yet.

**Stage 0 differs.** Stage 0 has no previews, and accepts the attribute
without a word.

**Status.** Leaning (D-072).

## Names of types in diagnostics

`@diagnostic_name(f)` on a type that a library makes at compile time,
such as a quantity whose units are computed while compiling, names a
`comptime` function that the compiler calls to print the type in a
diagnostic, so that a mistake with units shows `m/s` rather than a
vector of exponents; `sisu explain --raw-types` shows the type itself
([10](../../proposal/10-tooling.md), Q-27).

**Stage 0 differs.** The attribute is accepted and nothing calls `f`.

**Status.** Leaning (D-065), with the types made at compile time of
Q-27.

## Attributes of the standard library

Seven attributes are for the standard library's own source. In a
program's module each is E0225 (`resolve.std-only-attribute`), since a
function with an intrinsic body would make an unsafe operation of the
host a safe call:

| Attribute | On | Means |
|---|---|---|
| `@intrinsic("NAME")` | a function without a body | the host's operation `NAME` is its body |
| `@builtin` | a type | the host represents its values |
| `@structural("NAME")` | a function with a body | the host's `NAME` computes the same result where every bound is met structurally |
| `@keeps(self, x)` | an intrinsic | the `own` and `mut` arguments whose values it keeps or returns |
| `@sealed` | a trait | only the standard library implements it (E0240 otherwise) |
| `@marker` | a trait | it has no methods and the language's rules implement it, as for `Copy` |
| `@internal` | a function | only the standard library calls it (E0701 otherwise) |

<!-- file: examples/reference-attributes/reserved.sisu -->
```sisu
# Attributes that only the standard library may write.
@sealed
trait Shape:
    def area(self) -> f64

@intrinsic("math.sqrt")
def root(x: f64) -> f64

def main():
    print(root(2.0))
```

<!-- output: examples/reference-attributes/reserved.console -->
```console
$ sisu run reserved.sisu
error[E0225]: `@sealed` is reserved for the standard library
  --> reserved.sisu:2:1
  |
2 | @sealed
  | ^^^^^^^

error[E0225]: `@intrinsic` is reserved for the standard library
  --> reserved.sisu:6:1
  |
6 | @intrinsic("math.sqrt")
  | ^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

**Stage 0 differs.** `sisu test`, whose front end is the Rust one,
reports E0225 on an item and not on a method; `sisu check` and `sisu
run` report both ([R16](sisu-command.md#choosing-the-front-end)).

**Status.** In stage 0.

## What attributes leave out

- No attribute runs code or wraps a declaration. There is no `@memo`
  decorator, because an attribute cannot wrap a function
  ([11](../../proposal/11-stdlib.md)), and no `@test` or `@route` with
  behaviour: tests are `test` blocks, and a framework reads its routes as
  data (D-046).
- No attribute stands on a statement. Checked directives on a loop nest,
  such as `@vectorize(8)`, `@unroll(4)` and `@tile(64, 64)`, are an idea
  for later, with the native backend, that would need one (N-22 of
  [19](../../proposal/19-novel-features.md)).
- No attribute changes the layout of every list of a type:
  `SoaList[T]` of `std.collections` makes a structure of arrays a type
  of its own, chosen where it is used (D-069).
- No attribute enters a function's type (Q-52).
- `@inline`, which [02](../../proposal/02-syntax.md#declarations) names
  among its examples of attributes, has no design yet; stage 0 accepts it
  and does nothing.

**Status.** Decided (D-046, D-069, Q-52); the loop directives are an
idea for later, and `@inline` has no design.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0117](diagnostics.md#e0117), E0106, [E0100](diagnostics.md#e0100) | `parse.trailing-tokens`, `parse.expected-declaration`, `parse.expected` | [Writing an attribute](#writing-an-attribute) |
| [E0243](diagnostics.md#e0243), [E0244](diagnostics.md#e0244), [E0242](diagnostics.md#e0242), [E0235](diagnostics.md#e0235) | `resolve.not-derivable`, `resolve.derive-field`, `resolve.supertrait-missing`, `resolve.overlapping-impl` | [Deriving traits](#deriving-traits) |
| E0608 | `comptime.derive-failed` | [Deriving traits](#deriving-traits), [The layout of a type](#the-layout-of-a-type) |
| E0250, E0251, E0252, E0253 | `resolve.not-delegable`, `resolve.delegate-to`, `resolve.delegate-self`, `resolve.delegate-field` | [Delegating traits to a field](#delegating-traits-to-a-field) |
| [E0417](diagnostics.md#e0417), E0418 | `borrow.must-consume`, `borrow.must-consume-exit` | [Values that must be consumed](#values-that-must-be-consumed) |
| E0266, W0006 | `resolve.bad-cfg`, `resolve.unknown-feature` | [Conditional compilation](#conditional-compilation) |
| E0603, E0606 | `comptime.budget`, `comptime.budget-attribute` | [Compile-time budgets](#compile-time-budgets) |
| E0821 | `test.refines-mismatch` | [Refinement tests](#refinement-tests) |
| E0702 | `type.test-signature` | [Fixtures](#fixtures) |
| E0989 | `unsafe.export-signature` | [Exported functions](#exported-functions) |
| E0990 | `unsafe.blocking-not-foreign` | [Foreign functions that block](#foreign-functions-that-block) |
| E0225, E0240, E0701 | `resolve.std-only-attribute`, `resolve.sealed-trait`, `cap.internal` | [Attributes of the standard library](#attributes-of-the-standard-library) |
