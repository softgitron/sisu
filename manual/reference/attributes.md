# Attributes

This page lists Sisu's attributes: how an attribute is written and where
it may stand, how compile-time code reads attributes as data, and, for
each attribute that the compiler or the toolchain reads, where it has a
meaning, what it does and the diagnostics that guard it. The
declarations that attributes stand on are [R3](declarations.md); the
compile-time code that reads them is [R12](compile-time.md#reflection).

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
  `@orm.table`, which the proposal does not describe.
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

@deprecated("read the field `heat`")
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
are data for a library or a tool to read
([Attributes are data](#attributes-are-data)). Since nothing in this
program reads them, the run warns about each, W0022, and about the call
of the deprecated `heat`, W0023, on standard error.

An attribute on the line of its declaration is E0131
(`parse.attribute-line`), whose fix breaks the line, and the
declaration is read as if it had been broken. One before a statement is
E0129 (`parse.attribute-on-statement`), which names it, and the
statement is read as one; `@tile`, `@unroll` and `@vectorize` there get
a note that they are directives planned for loops, not in the language
yet. One at the end of a file, where no declaration follows, is E0106
(`parse.expected-declaration`):

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
error[E0131]: an attribute stands on a line of its own, before its declaration
  --> placement.sisu:2:13
  |
2 | @derive(Eq) struct Point:
  |             ^^^^^^
  = fix: put the declaration on the next line

error[E0129]: `@unused` stands before a statement; an attribute stands on the line before a declaration (a function, a type, a constant, a test), and none applies to a statement
  --> placement.sisu:6:5
  |
6 |     @unused
  |     ^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

Each message says where an attribute stands. Since E0129 drops the
attribute, `@unused`, a name that Sisu does not read, gets no W0022
([Where an attribute may stand](#where-an-attribute-may-stand)). An
attribute on a parameter is E0100 ([R17](diagnostics.md#e0100)).

**Status.** In stage 0 (D-046).

## Where an attribute may stand

| Declaration | Attributes stand |
|---|---|
| an item: `def` (with `pure`, `comptime` or `unsafe`), `struct`, `view struct`, `enum`, `error`, `trait`, `impl`, `type`, `const`, `import`, `from ... import`, `test`, `suite`, `bench`, an `extern` block | before it, and before its `pub` |
| a member: a field, a method of a struct, an enum, an impl or a trait, a variant of an enum or an error set, a function of an `extern` block, an item of a `suite` | before it, inside the body |

Nothing else takes an attribute: not a statement, an expression, a
parameter, a `case` arm or a closure. Each attribute that Sisu reads has
a meaning on some of these declarations only, which its section names.

An attribute whose name Sisu does not read is data for compile-time
code ([Attributes are data](#attributes-are-data)). One whose name is
close to an attribute Sisu reads, such as `@deprecatd`, is W0024
(`resolve.misspelt-attribute`), with the fix that writes the one it
misspells; any other is W0022 (`resolve.unknown-attribute`) when no
compile-time code of the program reads attributes, since nothing can
read it then. A program whose compile-time code calls
`reflect.attributes_of` keeps its own attributes without a word.

An attribute that Sisu reads, standing where it has no meaning, is
W0026 (`resolve.misplaced-attribute`), which names the declarations it
means something on, since it would do nothing there:

<!-- file: examples/reference-attributes/misplaced.sisu -->
```sisu
# Attributes that Sisu reads, each where it means nothing.
@derive(Debug)
def reading() -> int:
    return 82

@export("sauna_heat")
struct Stove:
    heat: int

def main():
    print(reading() + Stove(5).heat)
```

<!-- output: examples/reference-attributes/misplaced.console -->
```console
$ sisu run misplaced.sisu
warning[W0026]: `@derive` means something on a struct, an enum or an error set, so on a function it does nothing
  --> misplaced.sisu:2:2
  |
2 | @derive(Debug)
  |  ^^^^^^

warning[W0026]: `@export` means something on a function, so on a struct it does nothing
  --> misplaced.sisu:6:2
  |
6 | @export("sauna_heat")
  |  ^^^^^^

87
```

`@cfg` and `@preview` mean something on any declaration, and the
attributes that only the standard library writes, such as `@intrinsic`,
are E0225 in a program.

**Status.** In stage 0.

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
does not read is accepted, and kept without a word in a program whose
compile-time code reads attributes. `reflect.attributes_of(T)` gives the
attributes on the declaration of the type `T`, in the order written,
each with its `name`, its `args` and its `keys`. `args` holds each
argument's value: a string literal's text without its quotes and
escapes, an integer literal's number, `True` or `False`, and the text
of anything else as written, such as the name `hour`. A keyword
argument gives the value after its `=`, and `keys` holds each
argument's keyword, empty for one given by position.

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
Booking @table [bookings] @index [hour] [True]
```

Each argument keeps its type, so compile-time code computes with it,
and an element at a fixed index, `a.args[1]`, is that value whatever
the types of the others:

<!-- file: examples/reference-attributes/values.sisu -->
```sisu
# An attribute's arguments are values, and `keys` holds their keywords.
@table("bookings", 3)
@index(hour, unique=True)
struct Booking:
    name: String
    hour: int

## The table that `T` is kept in, and how many copies of it.
def stored[T]() -> (str, int):
    var name = ""
    var copies = 0
    comptime for a in reflect.attributes_of(T):
        comptime if a.name == "table":
            name = a.args[0]
            copies = a.args[1]
    return (name, copies)

def main():
    let (name, copies) = stored[Booking]()
    print("{name} in {copies + 1} copies")
    comptime for a in reflect.attributes_of(Booking):
        print("@{a.name} keys {a.keys:?}")
```

<!-- output: examples/reference-attributes/values.out -->
```text
bookings in 4 copies
@table keys ["", ""]
@index keys ["", "unique"]
```

`stored` reads the table's name as a `str` and its number of copies as
an `int`, and `keys` is empty for each argument given by position.

A template may number the arguments with `range(len(a.args))`, and
read each argument's value and keyword at the same position
([R12](compile-time.md#templates) lists what else its compile-time
code may call):

<!-- file: examples/reference-attributes/positions.sisu -->
```sisu
# A template numbers an attribute's arguments with `range`.
@index(hour, name, unique=True)
struct Booking:
    name: String
    hour: int

## Each argument of `T`'s attributes, by its position or its keyword.
def arguments[T]() -> list[String]:
    var out: list[String] = []
    comptime for a in reflect.attributes_of(T):
        comptime for i in range(len(a.args)):
            comptime if a.keys[i] == "":
                out.push("@{a.name} {i}: {a.args[i]}")
            else:
                out.push("@{a.name} {a.keys[i]}: {a.args[i]}")
    return out

def main():
    for line in arguments[Booking]():
        print(line)
```

<!-- output: examples/reference-attributes/positions.out -->
```text
@index 0: hour
@index 1: name
@index unique: True
```

A function's attributes are data too: `reflect.functions_of(m)` lists
the functions of a module `m` that the code imports, and
`reflect.attributes_of(f)` of each gives its attributes in the same
form, so a framework collects its routed functions into a table at
compile time, as 08 has it do. [Chapter
25](../tutorial/25-compile-time.md#attributes-are-data) builds such a
table of routes, and [R12](compile-time.md#reflection) lists what a
function's descriptor holds.

A field's, a variant's and a method's attributes are data in the same
form: `f.attributes` of each field that `reflect.fields_of` gives,
`v.attributes` of each variant that `reflect.variants_of` gives, and
`reflect.attributes_of(m)` of each method that `reflect.methods_of`
gives. A variant's field has none. Here a field takes its column's
name from a `@column`, and a variant its text from a `@label`:

<!-- file: examples/reference-attributes/members.sisu -->
```sisu
# A field's and a variant's attributes are data too.
struct Booking:
    @column("guest")
    name: String
    hour: int

enum Heat:
    @label("cold")
    Cold
    @label("ready for löyly")
    Ready

def main():
    comptime for f in reflect.fields_of(Booking):
        var column = f.name
        comptime for a in f.attributes:
            column = a.args[0]
        print("{f.name} is stored as {column}")
    comptime for v in reflect.variants_of(Heat):
        comptime for a in v.attributes:
            print("{v.name}: {a.args[0]}")
```

<!-- output: examples/reference-attributes/members.console -->
```console
$ sisu run members.sisu
name is stored as guest
hour is stored as hour
Cold: cold
Ready: ready for löyly
```

The program reads the attributes and prints what they say, with no
W0022: reading `f.attributes` or `v.attributes` counts as code that
reads attributes, as a call of `reflect.attributes_of` does, so an
attribute of the program's own is data that code may read
([Attributes are data](#attributes-are-data)). A method's attributes route a
call to it, `s.[m]()`, as [chapter
25](../tutorial/25-compile-time.md#attributes-of-methods-and-fields)
does.

**Stage 0 differs.** `reflect.attributes_of` takes a struct, an enum,
an error set, a method that `reflect.methods_of` gives or a function
that `reflect.functions_of` gives, and gives an empty list for a type
of the standard library and its declarations.

**Status.** In stage 0 (D-046, D-018), with the differences noted.

## The attributes Sisu reads

| Attribute | Stands on | Does | Status |
|---|---|---|---|
| `@derive(TRAIT, ...)` | a struct, an enum, an error set | writes the traits' impls from the fields ([Deriving traits](#deriving-traits)) | in stage 0 |
| `@delegate(TRAIT, ..., to="FIELD")` | a struct | forwards the traits to a field ([Delegating traits to a field](#delegating-traits-to-a-field)) | in stage 0 |
| `@must_consume` | a struct, an enum | makes each value end in an `own` parameter ([Values that must be consumed](#values-that-must-be-consumed)) | in stage 0 |
| `@cfg(CONDITION)` | an item, an item of a suite, an `extern` block, a method of an impl or a trait | keeps the declaration only where the condition holds ([Conditional compilation](#conditional-compilation)) | in stage 0 |
| `@comptime_budget(steps=N)` | a function, a constant | raises its compile-time budget ([Compile-time budgets](#compile-time-budgets)) | in stage 0 |
| `@tag("NAME", ...)` | a test | names it for `--tag` and `--skip-tag` ([Tags on tests](#tags-on-tests)) | in stage 0 |
| `@refines(REFERENCE)` | a function | tests it against a reference ([Refinement tests](#refinement-tests)) | in stage 0; leaning |
| `@fixture`, `@fixture(scope="...")` | a function | makes a value for the tests that name it ([Fixtures](#fixtures)) | in stage 0, made for each test |
| `@repr(c)`, `@repr(u8)` | a struct, an enum | fixes the layout ([The layout of a type](#the-layout-of-a-type)) | partly in stage 0 |
| `@export("NAME")` | a function | makes it callable from C ([Exported functions](#exported-functions)) | in stage 0, natively, but for a struct by value and a pointer to an option of a raw pointer |
| `@blocking` | a function of an `extern` block | says that a call may block ([Foreign functions that block](#foreign-functions-that-block)) | designed; checked in stage 0 |
| `@no_alloc`, `@no_block`, `@stack(max=N)` | a function | limit what it may do ([Resource limits](#resource-limits)) | leaning; `@no_alloc` checked, the others accepted with W0030 and not checked in stage 0 |
| `@float(fast)` | a function | allows fast floating point inside it ([Fast floating point](#fast-floating-point)) | in stage 0, as a permission it does not use |
| `@deprecated("MESSAGE")`, `@fix_inline` | an item, a field, a variant, a method; a function | warn at each use, and migrate the uses ([Deprecation and migrations](#deprecation-and-migrations)) | `@deprecated` in stage 0; `@fix_inline` in stage 0 for a body of one `return` |
| `@preview("NAME")` | not designed yet; in stage 0 a function, a type, a constant | asks for a feature that is not stable; in stage 0 warns at each use of the item ([Preview features](#preview-features)) | leaning; the item's warning in stage 0 |
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
  prints, which every type has without asking, a `pub` one too, and
  which a type that hides its fields replaces with an `impl Debug` of
  its own ([10](../../proposal/10-tooling.md#debugging-and-profiling));
- `Eq`: equal when the variants are and every field is;
- `PartialOrd` and `Ord`: by variant in the order declared, then field
  by field, the first that differs deciding;
- `Hash`: the fields' hashes in order, after an enum's variant;
- `Default`, for a struct: each field's own default where it has one,
  and its type's `default()` otherwise;
- `Plain` of `std.mem`, for a `@repr(c)` struct whose fields are plain
  and leave no padding (E0279, E0280; [The layout of a
  type](#the-layout-of-a-type));
- any trait that declares a `comptime def derive`, the program's own
  included ([R12](compile-time.md#the-derive-protocol)).

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
    print("{old < new} {sorted([new, old])}")
    print("{Version.default():?}")
    print({Light.Red: "stop", Light.Amber(3): "wait"}[Light.Amber(3)])
    print(Missing("hat".to_string()) == Missing("hat".to_string()))
```

<!-- output: examples/reference-attributes/derive.out -->
```text
True [Version(major=1, minor=2), Version(major=2, minor=0)]
Version(major=0, minor=0)
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
@derive(Copy, Ord)
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
  --> derive_needs.sisu:2:15
  |
2 | @derive(Copy, Ord)
  |               ^^^

error[D0006]: the program was not run: errors found (1)
```

`@derive(Encode, Decode)` takes the traits of `std.json`, which the
file imports, `from std.json import Decode, Encode`; it gives a type
the JSON form of its shape ([Read and write JSON](../guides/json.md)).

Without the import, `@derive(Encode)` is E0243 (`resolve.not-derivable`),
whose note names the import. A diagnostic in a derived impl is placed at the
derive's argument that asked for the impl, with a note that quotes the
impl's line, and without the labels and fixes that would point into
code the program does not have; a label that points into the
program's own files stays: E0235 for an impl written beside
`@derive(Eq)` stands at the `Eq` of the derive and labels the written
impl, "the other impl is here". E0347's fix "derive
it", for a generic type that derives the trait and a type argument
that does not, `Pair[Plain]`, goes to the argument, `Plain`.

**Stage 0 differs.** `@derive(Into, From)`, which
[03](../../proposal/03-types-and-generics.md#type-aliases-and-newtypes)
gives newtypes, are E0243, since what they write is the owner's
question Q-80.

**Status.** In stage 0 (D-018), except `Into` and `From`.

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
  = note: `+` on a `Basket` is the method of the trait `Add`: an `impl Add[Basket] for Basket` gives it one

error[D0006]: the program was not run: errors found (2)
```

**Status.** In stage 0 (D-012).

## Values that must be consumed

`@must_consume` on a struct or an enum makes each value of the type end
in an `own` parameter, usually a method that takes `own self`, before it
goes out of scope: a value that goes out of scope unconsumed is E0417
([R17](diagnostics.md#e0417)), and one that an early `return`, `break`
or `?` leaves behind is E0418 (`borrow.must-consume-exit`). A
collection of such values is must-consume too. The rules are
[R7](ownership.md#values-that-must-be-consumed), and cleanup that can
fail is [R8](errors.md#cleanup-that-can-fail).

**Status.** In stage 0 (D-054; containers lean, D-064).

## Conditional compilation

`@cfg(CONDITION, ...)` keeps its declaration only in the builds where
every condition holds; elsewhere the declaration is left out before
names are resolved, as if it were not written. It stands on an item, an
item of a suite, an `extern` block, a method of an impl or a trait, or a
field, a variant or a method in a struct's or an enum's own body. A
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
(`resolve.unknown-feature`). An import is the one item whose `@cfg`
may test the target, `@cfg(target.os == "windows")`, for a dependency
that only some targets build ([R13](modules.md#target-conditions)).
Anywhere else a condition on the target is written in a body with `if
comptime target.os == "linux":`, whose branches are both checked
([R12](compile-time.md#the-facts-of-the-target)), and E0266's note
`resolve.cfg-target` names that form:

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
  = note: a body tests the target with `if comptime target.os == "linux":`, whose other branch is left out of the build

error[D0006]: the program was not run: errors found (1)
```

On a member, `@cfg(test)` gives a type a field, a variant or a method
that only its tests use. A build without its tests has none of them, so
the value that `sisu run` prints has no `resets`, while the test reads
it:

<!-- file: examples/reference-attributes/cfg_members.sisu -->
```sisu
# `@cfg` on a field, a variant and a method of a type's own body.
struct Counter:
    count: int
    @cfg(test)
    resets: int = 0

    @cfg(test)
    def reset(mut self):
        self.count = 0
        self.resets += 1

enum Mode:
    Quiet
    @cfg(test)
    Traced

def main():
    let counter = Counter(count=3)
    print("{counter.count} {counter:?} {Mode.Quiet:?}")

test "a test build has the members":
    var counter = Counter(count=3)
    counter.reset()
    assert counter.count == 0 and counter.resets == 1
    assert "{Mode.Traced:?}" == "Traced"
```

<!-- output: examples/reference-attributes/cfg_members.console -->
```console
$ sisu run cfg_members.sisu
3 Counter(count=3) Quiet
$ sisu test cfg_members.sisu
test "a test build has the members" ... ok
1 passed, 0 failed, 0 blocked
```

A field that a build leaves out takes its default nowhere, and code
that every build compiles, such as `main`, cannot name it: the build
without its tests reports E0303 there.

**Status.** In stage 0 (08).

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

The evaluation and its budget are [R12](compile-time.md#budgets).

**Status.** In stage 0 (D-063 leans).

## Tags on tests

`@tag("NAME", ...)` on a test, in a suite or not, or on a suite, gives
it tags: `sisu test --tag NAME` runs the tests that have the tag, and
`--skip-tag NAME` those that do not ([R15](testing.md#the-forms-of-a-test),
[R16](sisu-command.md#sisu-test)).

A suite's `@tag` tags each of its tests, before the test's own:

<!-- file: examples/reference-attributes/suite_tags.sisu -->
```sisu
# A suite's tags tag each of its tests.
def double(n: int) -> int:
    return n * 2

@tag("slow")
suite "large numbers":
    test "a large number":
        assert double(1 << 40) == 1 << 41

    test "a larger number":
        assert double(1 << 50) == 1 << 51

test "a small number":
    assert double(2) == 4
```

<!-- output: examples/reference-attributes/suite_tags.console -->
```console
$ sisu test --tag slow suite_tags.sisu
test "large numbers/a large number" ... ok
test "large numbers/a larger number" ... ok
2 passed, 0 failed, 0 blocked
1 not selected
```

**Status.** In stage 0 (D-057).

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

`@fixture(scope="module")` on a function offers its value to the tests:
a test with a parameter of the function's name receives the value, made
once for the scope, here once for the module
([17](../../proposal/17-testing.md#fixtures)).

A fixture is a free function that takes nothing; a `@fixture` with a
parameter, or on a method, is E0281 (`type.fixture-signature`,
[R17](diagnostics.md#e0281)). Without a scope, a test's parameter that names it gets
what a call of it returns, made before the test and dropped after it
([R15](testing.md#fixtures)).

With `scope="module"` the value is made once, before the first of the
module's tests that names it, lent to each of them, which run in turn,
and dropped after the last; what it returns is `Sync`, E0283
(`type.module-fixture-sync`, [R17](diagnostics.md#e0283)) otherwise.

**Stage 0 differs.** A native test run makes the value for each test
that names it, as a fixture without a scope is.

**Status.** In stage 0, with the difference noted; whether the
attribute is needed at all is open
([17](../../proposal/17-testing.md#open-questions)).

## The layout of a type

`@repr(c)` on a struct or an enum fixes its layout as C's; a struct's
fields are then in the order declared, each at an offset that is a
multiple of its alignment, with padding where needed
([12](../../proposal/12-interop-and-targets.md#c-interop)). The derive of
`std.mem`'s `Plain`, for types whose every bit pattern is a value, asks
for it on a struct of more than one field (E0608 otherwise) and refuses
one whose layout has padding, which 22 allows no plain type: a gap
between two fields is E0279 and one at the end E0280
([R17](diagnostics.md#e0279)). Foreign functions take such structs
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
without `@repr(c)`. `Plain`'s derive asks for `@repr(c)` itself, so
`@repr(python)` is refused as a struct without it (E0608). `@repr(u8)` on an enum is kept and changes nothing a safe
program can see, and a variant's value, `Red = 1`, is E0900
([R2](types.md#structs-and-enums)).

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

Each parameter and the result is also a type that C has a layout for:
a number of a width C has, `bool`, `char`, nothing, a raw pointer, or a
type declared `@repr(c)`; any other is E0991
(`unsafe.export-not-c`), so a parameter of a struct with a `String` in
it is refused ([R17](diagnostics.md#e0991)).

A native build makes the function the global symbol `NAME` of C's own
convention over plain values, whether or not `main` calls it: the
integers and floats at their width, a `bool` a byte, a `char` a 32-bit
code, a raw pointer an address and `()` no result. A C program linked
with it declares `int32_t sauna_heat(int32_t celsius, int32_t
minutes);` and calls it. A `@repr(c)` struct goes by a pointer to it,
`ptr[Reading]`. The program's SIR names each exported function on a
line `export "NAME" FUNCTION` after its entry.

A panic cannot unwind into C, so a panic that leaves an exported
function prints its report and ends the program with status 134
([R8](errors.md#aborts)). A program can call its own export through
an `extern` block, as C would, which `sisu run --native` shows: the
first call returns, and the second fails the function's `assert` and
ends the run:

<!-- file: examples/reference-attributes/export_c.files/export_c.sisu -->
```sisu
# The program calls its own exported function as C calls it.
@export("sauna_heat")
def heat(minutes: i32) -> i32:
    assert minutes <= 30, "nobody stays that long"
    return 60 + minutes * 2

# safety: `sauna_heat` is the export above, of C's convention.
extern "C":
    def sauna_heat(minutes: i32) -> i32

def main():
    # safety: the export takes an `i32` and returns one.
    let short = unsafe sauna_heat(10)
    print("{heat(10)} {short}")
    # safety: as above.
    let long = unsafe sauna_heat(45)
    print("{long}")
```

<!-- output: examples/reference-attributes/export_c.console -->
```console
$ sisu run --native export_c.sisu
80 80
panic: assertion failed: minutes <= 30
  message: nobody stays that long
    minutes <= 30
    │
    45
  at heat (export_c.sisu:4:5)
```

The interpreter calls no foreign function but the six it emulates
([R3](declarations.md#foreign-functions)), so this program runs
natively only.

A `@repr(c)` struct goes by a pointer natively too. A native build lays
a declared struct's fields out at C's offsets, each at the next offset
its alignment allows, a `u8` in one byte and an `i32` or an `f32` in
four, so `ptr.of(ref reading)` is the address of a place laid out as C
lays it out, and a program hands its own export the `ptr[Reading]` that
C passes:

<!-- file: examples/reference-attributes/export_pointer.sisu -->
```sisu
# An exported function takes a `@repr(c)` struct by a pointer to it.
from std.mem import Layout, ptr

@repr(c)
struct Reading:
    celsius: i32
    minutes: u8
    humidity: f32

@export("sauna_load")
def load(reading: ptr[Reading]) -> i32:
    # safety: valid, aligned: `reading` points to a live `Reading` that
    #   nothing changes while the reference lives.
    let r = unsafe reading.as_ref()
    return r.celsius * r.minutes

def main():
    let reading = Reading(celsius=80, minutes=10, humidity=0.2)
    print(load(ptr.of(ref reading)))
    let layout = Layout.of[Reading]()
    print("{layout.size} bytes, aligned to {layout.align}")
```

<!-- output: examples/reference-attributes/export_pointer.console -->
```console
$ sisu run --native export_pointer.sisu
800
12 bytes, aligned to 4
```

`Layout.of[Reading]()` gives the size and the alignment C gives the
struct: the `i32` at offset 0, the `u8` at 4, three bytes of padding,
and the `f32` at 8.

**Stage 0 differs.** A `@repr(c)` struct passed by value has no C
classification yet, so the native build refuses it with D0103
(`native.unsupported`, [R17](diagnostics.md#d0103)), which names the
struct by value as the program wrote it and the function it is in; the
interpreter runs the program:

<!-- file: examples/reference-attributes/export_struct.files/export_struct.sisu -->
```sisu
# A `@repr(c)` struct passed to an exported function by value.
@repr(c)
struct Reading:
    celsius: i32
    minutes: i32

@export("sauna_load")
def load(reading: Reading) -> i32:
    return reading.celsius * reading.minutes

def main():
    print(load(Reading(celsius=80, minutes=10)))
```

<!-- output: examples/reference-attributes/export_struct.console -->
```console
$ sisu run export_struct.sisu
800
$ sisu build --native export_struct.sisu
error[D0103]: the native backend does not compile a `Reading` by value (a parameter of the exported function `sauna_load`) yet (in `load`); `sisu run` without `--native` runs the program
```

Pass a `ptr[Reading]` instead, as the example before it does. A tuple
lays its parts out at C's offsets natively too, each at the next offset
its alignment allows, so a raw pointer to a tuple with a part narrower
than a word compiles and reads as C would:

<!-- file: examples/reference-attributes/export_tuple.files/export_tuple.sisu -->
```sisu
# A raw pointer to a tuple of narrow parts, laid out as C lays it.
from std.mem import ptr

def main():
    let pair: (int, u8) = (80, 10)
    let p = ptr.of(ref pair)
    # safety: valid, aligned: `pair` lives and nothing changes it while
    #   the reference lives.
    print(unsafe p.as_ref().1)
```

<!-- output: examples/reference-attributes/export_tuple.console -->
```console
$ sisu run export_tuple.sisu
10
$ sisu run --native export_tuple.sisu
10
```

An option of a raw pointer, `ptr[T]?` or `(ptr mut T)?`, is one
nullable word natively too, the address or 0 for `None`, as C holds an
optional pointer, so a pointer to one, or to a struct that holds one,
compiles and reads as C would:

<!-- file: examples/reference-attributes/export_option.files/export_option.sisu -->
```sisu
# A raw pointer to an optional raw pointer, one word in C.
from std.mem import ptr

def main():
    let maybe: ptr[u8]? = None
    let p = ptr.of(ref maybe)
    # safety: valid, aligned: `maybe` lives and nothing changes it while
    #   the reference lives.
    print(unsafe p.as_ref().is_none())
```

<!-- output: examples/reference-attributes/export_option.console -->
```console
$ sisu run export_option.sisu
True
$ sisu run --native export_option.sisu
error[D0103]: the native backend does not compile `mem.ptr.of` of a `Option[ptr[u8]]`, which raw memory holds otherwise than a value of it yet (in `main`); `sisu run` without `--native` runs the program
```

A foreign call's parameter or result of type `ptr[T]?` passes the same
word, so C's optional pointer and Sisu's are one value (decision 1240
of the [architecture notes](../../compiler/docs/architecture.md)).

**Status.** In stage 0 ([12](../../proposal/12-interop-and-targets.md),
[22](../../proposal/22-unsafe.md); D-080), with the difference noted.

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

**Stage 0 differs.** `@no_alloc` is checked on the program's SIR,
through its calls into their bodies, the standard library's included:
a statement that makes, grows, copies or boxes a value that owns
memory, or calls a function that may, is E0282 (`type.no-alloc`,
[R17](diagnostics.md#e0282)) at the statement, with a note that names
where the callee allocates. The paths that only panic are exempt, so an
`assert`'s message may be built:

<!-- file: examples/reference-attributes/no_alloc.sisu -->
```sisu
# `@no_alloc` is checked: a path that panics may allocate, a report not.

## The highest of `readings`, which must not be above 120.
@no_alloc
def highest(readings: Array[int, 3]) -> int:
    var top = readings[0]
    for i in range(3):
        top = max(top, readings[i])
    assert top <= 120, "too hot: {top} degrees"
    return top

## Writes the highest of `readings` to the log.
@no_alloc
def report(readings: Array[int, 3]):
    print("highest: {highest(readings)}")

def main():
    let readings: Array[int, 3] = Array.filled(80)
    report(readings)
```

<!-- output: examples/reference-attributes/no_alloc.console -->
```console
$ sisu run no_alloc.sisu
error[E0282]: `report` is `@no_alloc`, but `fmt.new` may allocate
  --> no_alloc.sisu:15:11
   |
15 |     print("highest: {highest(readings)}")
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

A generic function is checked as 04 checks it, at each instance the
program makes: what allocates whatever the type arguments are is the
error at the function, and a call of a trait's method on a type
parameter is checked through the impl of each instance's type. An
instance that allocates is the error named by its type arguments, with
the note `type.no-alloc-instance` saying where the program makes it,
and an instance that allocates nothing checks:

<!-- file: examples/reference-attributes/no_alloc_generic.sisu -->
```sisu
# `@no_alloc` on a generic function is checked at each instance.
trait Reading:
    def celsius(self) -> int

struct Plain:
    value: int

impl Reading for Plain:
    def celsius(self) -> int:
        return self.value

struct Logged:
    value: int

impl Reading for Logged:
    def celsius(self) -> int:
        print("read {self.value} degrees")
        return self.value

## The highest of `readings`.
@no_alloc
def highest[T: Reading](readings: list[T]) -> int:
    var top = 0
    for i in range(len(readings)):
        top = max(top, readings[i].celsius())
    return top

def main():
    print(highest([Plain(value=80), Plain(value=95)]))
    print(highest([Logged(value=80)]))
```

<!-- output: examples/reference-attributes/no_alloc_generic.console -->
```console
```

`highest[Plain]` checks; `highest[Logged]` prints, which formats a
string. The check still differs from 04's in two ways: a closure that
captures by value is taken to allocate, since whether its captures fit
in its value is the backend's to know; and no collection is free of
allocation by its allocator, since the allocators are missing
([R7](ownership.md#allocators)). `@no_block` and `@stack` are accepted
and checked by nothing, and stage 0 says so at each with W0030
(`resolve.unchecked-attribute`, [R17](diagnostics.md#w0030)), so that a
program that writes one does not take the promise for a check; on a
declaration other than a function each attribute is W0026, as any
misplaced attribute is. The call of
the deprecated `old_levels` warns (W0023) with the fix that
`@fix_inline` gives it ([Deprecation and
migrations](#deprecation-and-migrations)), and `@float(fast)` keeps the
arithmetic strict, which the permission allows:

<!-- file: examples/reference-attributes/designed.sisu -->
```sisu
# Designed attributes that stage 0 accepts and does not check.
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

<!-- output: examples/reference-attributes/designed.console -->
```console
$ sisu run designed.sisu
warning[W0030]: `@no_block` is accepted and checked by nothing yet, so nothing ensures that the function does not block
  --> designed.sisu:2:2
  |
2 | @no_block
  |  ^^^^^^^^

warning[W0030]: `@stack` is accepted and checked by nothing yet, so nothing ensures the function's bound on its stack
  --> designed.sisu:3:2
  |
3 | @stack(max=256)
  |  ^^^^^

warning[W0023]: `old_levels` is deprecated: use `levels`
  --> designed.sisu:17:13
   |
17 |     print("{old_levels(3)} {blend(1.0, 2.0)}")
   |             ^^^^^^^^^^^^^
  = fix: write what `old_levels` returns in the call's place: `levels(3)`

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

`@float` takes `fast` only; any other argument is E0448
(`resolve.float-attribute`, [R17](diagnostics.md#e0448)), since without
the attribute floating point is strict already.

**Stage 0 differs.** Stage 0 uses none of the permission: it keeps the
arithmetic of a `@float(fast)` function strict, which the permission
allows ([Resource limits](#resource-limits) shows one).

**Status.** In stage 0 (D-074), with the difference noted.

## Deprecation and migrations

`@deprecated("MESSAGE")` on an item makes each use of it a warning that
names the replacement, with a rewrite by `sisu fix` where one is
mechanical ([13](../../proposal/13-evolution-and-governance.md#deprecation)).
`@fix_inline` on a deprecated function whose body is written in terms of
the new API lets `sisu fix --deps` replace the calls in the packages
that depend on it with that body, moving borrows, `own` arguments and
`defer` into the caller, so their behaviour is kept by construction
(D-067).

A use of a deprecated item is W0023 (`type.deprecated`) with the
attribute's message, or W0025 (`type.deprecated-plain`) for an
attribute without one ([R17](diagnostics.md#w0023)): a call of a
function or a method, or its use as a value; a type or a trait named,
or a type built; a constant read; a variant built or matched; a field
read, changed, given or matched. The item's own body may use
it without a word, and so may a type's own declaration and its impls:

<!-- file: examples/reference-attributes/deprecated_items.sisu -->
```sisu
# A deprecated type, constant and variant warn where they are used.
@deprecated("use `Heater`")
struct Stove:
    heat: int

@deprecated("use `MAX_HEAT`")
const HOTTEST: int = 110

enum Fuel:
    Wood
    @deprecated("use `Electric`")
    Coal
    Electric

def main():
    let stove = Stove(80)
    let fuel = Fuel.Coal
    print("{stove.heat} of {HOTTEST} {fuel:?}")
```

<!-- output: examples/reference-attributes/deprecated_items.console -->
```console
$ sisu run deprecated_items.sisu
warning[W0023]: `Stove` is deprecated: use `Heater`
  --> deprecated_items.sisu:16:17
   |
16 |     let stove = Stove(80)
   |                 ^^^^^

warning[W0023]: `Fuel.Coal` is deprecated: use `Electric`
  --> deprecated_items.sisu:17:16
   |
17 |     let fuel = Fuel.Coal
   |                ^^^^^^^^^

warning[W0023]: `HOTTEST` is deprecated: use `MAX_HEAT`
  --> deprecated_items.sisu:18:29
   |
18 |     print("{stove.heat} of {HOTTEST} {fuel:?}")
   |                             ^^^^^^^

80 of 110 Coal
```

A trait and a field warn in the same way. A deprecated trait warns where
it is named: in a bound (`T: Measured`), an impl's header, a list of
supertraits or a `dyn` type, but not in its own declaration. A
deprecated field warns where it is read, changed, given a value (at its
keyword) or matched, but not in its struct's own methods and impls, so
the impl's `self.width` is silent here:

<!-- file: examples/reference-attributes/deprecated_members.sisu -->
```sisu
# A deprecated trait and field warn where they are used.
@deprecated("use `Rated`")
trait Measured:
    def size(self) -> int

struct Panel:
    @deprecated("use `width_cm`")
    width: int
    width_cm: int

impl Measured for Panel:
    def size(self) -> int:
        return self.width

def main():
    let panel = Panel(width=3, width_cm=30)
    print("{panel.width} {panel.size()}")
```

<!-- output: examples/reference-attributes/deprecated_members.console -->
```console
$ sisu run deprecated_members.sisu
warning[W0023]: `Measured` is deprecated: use `Rated`
  --> deprecated_members.sisu:11:6
   |
11 | impl Measured for Panel:
   |      ^^^^^^^^

warning[W0023]: `Panel.width` is deprecated: use `width_cm`
  --> deprecated_members.sisu:16:23
   |
16 |     let panel = Panel(width=3, width_cm=30)
   |                       ^^^^^

warning[W0023]: `Panel.width` is deprecated: use `width_cm`
  --> deprecated_members.sisu:17:19
   |
17 |     print("{panel.width} {panel.size()}")
   |                   ^^^^^

3 3
```

`@fix_inline` on a deprecated function makes each call's warning carry
a fix, `type.inline-call`, that writes what the function returns in the
call's place, each parameter replaced by the call's argument. An
argument, and the whole expression, are written in parentheses only
where precedence needs them: `twice(turns + 1)` becomes `scaled(turns +
1, 2)`, since the argument fills a place of the call whole, and
`warmer(sauna) * 2.0` of a body `sauna + 4.0` would become `(sauna +
4.0) * 2.0`. The body must be one `return` of an
expression, of a free function whose parameters take no `mut` or `own`;
another is W0033 (`type.fix-inline-body`, [R17](diagnostics.md#w0033))
at the attribute, and its calls warn without the fix. A call gets no
fix where the rewrite could mean something else: an argument with
effects used other than once, or before the expression's other calls,
a parameter's name bound again inside the expression, a default left
out, or, in another module, a name the expression takes from the
function's module. [sisu fix](sisu-command.md#sisu-fix) applies the
fixes:

<!-- file: examples/reference-attributes/inline_body.sisu -->
```sisu
# `@fix_inline` rewrites the calls of a function whose body is a return.
def scaled(n: int, by: int) -> int:
    return n * by

@deprecated("use `scaled`")
@fix_inline
def twice(n: int) -> int:
    return scaled(n, 2)

@deprecated("use `scaled`")
@fix_inline
def thrice(n: int) -> int:
    let by = 3
    return scaled(n, by)

def main():
    let turns = 4
    print("{twice(turns)} {thrice(turns)} {twice(turns + 1)}")
```

<!-- output: examples/reference-attributes/inline_body.console -->
```console
$ sisu check inline_body.sisu
warning[W0033]: `@fix_inline` rewrites a call of `thrice` with the expression it returns, so its body is one `return`, and its parameters take no `mut` or `own`: its calls warn without the rewrite
  --> inline_body.sisu:11:2
   |
11 | @fix_inline
   |  ^^^^^^^^^^

warning[W0023]: `twice` is deprecated: use `scaled`
  --> inline_body.sisu:18:13
   |
18 |     print("{twice(turns)} {thrice(turns)} {twice(turns + 1)}")
   |             ^^^^^^^^^^^^
  = fix: write what `twice` returns in the call's place: `scaled(turns, 2)`

warning[W0023]: `thrice` is deprecated: use `scaled`
  --> inline_body.sisu:18:28
   |
18 |     print("{twice(turns)} {thrice(turns)} {twice(turns + 1)}")
   |                            ^^^^^^^^^^^^^

warning[W0023]: `twice` is deprecated: use `scaled`
  --> inline_body.sisu:18:44
   |
18 |     print("{twice(turns)} {thrice(turns)} {twice(turns + 1)}")
   |                                            ^^^^^^^^^^^^^^^^
  = fix: write what `twice` returns in the call's place: `scaled(turns + 1, 2)`

$ sisu fix --dry-run inline_body.sisu
inline_body.sisu:18:13: W0023 fixed: write what `twice` returns in the call's place: `scaled(turns, 2)`
  18:13-18:25 "scaled(turns, 2)"
inline_body.sisu:18:44: W0023 fixed: write what `twice` returns in the call's place: `scaled(turns + 1, 2)`
  18:44-18:60 "scaled(turns + 1, 2)"
fixes that would be applied to `inline_body.sisu`: 2, and `--dry-run` leaves it as it is
```

**Stage 0 differs.** D-067's `@fix_inline` moves a body of several
statements into the caller, with its borrows, `own` arguments and
`defer`, and rewrites methods too, where stage 0 rewrites a call of a
free function whose body is one `return`; `sisu fix --deps`, which
would apply the fixes in the packages that depend on a library, is
designed ([R16](sisu-command.md#sisu-fix---edition-and---deps)).

**Status.** `@deprecated` in stage 0
([13](../../proposal/13-evolution-and-governance.md)); `@fix_inline`
in stage 0 with the differences noted, and leaning (D-067).

## Preview features

`@preview("NAME")` asks for a language feature that is not stable yet,
by its name. A feature stays a preview for at least one release, may
change or go while it is one, and is made stable or removed within four
releases, two years ([13](../../proposal/13-evolution-and-governance.md#release-cadence),
D-072). Where the attribute stands is not designed yet.

**Stage 0 differs.** Stage 0 has no previews of the language. The
attribute marks an item of a program or a library instead, a function,
a type or a constant, whose every use warns, as a use of a deprecated
item does: W0028 (`type.preview`) with the attribute's text, W0029
(`type.preview-plain`) without one. The item's own declaration uses it
without a word:

<!-- file: examples/reference-attributes/preview.sisu -->
```sisu
# Items marked as previews warn at each use, not where they are declared.

## The minutes of each sauna turn for `guests` guests.
@preview("the turns may come to depend on the stove")
def turns(guests: int) -> list[int]:
    return [20 for _ in range(guests)]

## The heat of the stove, which may move to another module.
@preview
const HEAT: int = 80

def main():
    print("{turns(2)} at {HEAT} degrees")
```

<!-- output: examples/reference-attributes/preview.console -->
```console
$ sisu run preview.sisu
warning[W0028]: `turns` is a preview, which may change or be removed before it is stable: the turns may come to depend on the stove
  --> preview.sisu:13:13
   |
13 |     print("{turns(2)} at {HEAT} degrees")
   |             ^^^^^^^^

warning[W0029]: `HEAT` is a preview, which may change or be removed before it is stable
  --> preview.sisu:13:27
   |
13 |     print("{turns(2)} at {HEAT} degrees")
   |                           ^^^^

[20, 20] at 80 degrees
```

**Status.** Leaning (D-072), with the differences noted.

## Names of types in diagnostics

`@diagnostic_name(f)` on a type that a library makes at compile time,
such as a quantity whose units are computed while compiling, names a
`comptime` function that the compiler calls to print the type in a
diagnostic, so that a mistake with units shows `m/s` rather than a
vector of exponents; `sisu explain --raw-types` shows the type itself
([10](../../proposal/10-tooling.md), Q-27).

**Stage 0 differs.** The attribute is E0900 until compile-time code
can make types (Q-27). The first step, a `comptime def` that returns a
`type` and a type position that calls it, is planned, and how that call
is written is the owner's question Q-81
([R12](compile-time.md#reflection)).

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
| [E0131](diagnostics.md#e0131), [E0129](diagnostics.md#e0129), [E0106](diagnostics.md#e0106), [E0100](diagnostics.md#e0100) | `parse.attribute-line`, `parse.attribute-on-statement`, `parse.expected-declaration`, `parse.expected` | [Writing an attribute](#writing-an-attribute) |
| [W0022](diagnostics.md#w0022), [W0024](diagnostics.md#w0024), [W0026](diagnostics.md#w0026) | `resolve.unknown-attribute`, `resolve.misspelt-attribute`, `resolve.misplaced-attribute` | [Where an attribute may stand](#where-an-attribute-may-stand) |
| [E0243](diagnostics.md#e0243), [E0244](diagnostics.md#e0244), [E0242](diagnostics.md#e0242), [E0235](diagnostics.md#e0235) | `resolve.not-derivable`, `resolve.derive-field`, `resolve.supertrait-missing`, `resolve.overlapping-impl` | [Deriving traits](#deriving-traits) |
| [E0608](diagnostics.md#e0608) | `comptime.derive-failed` | [Deriving traits](#deriving-traits), [The layout of a type](#the-layout-of-a-type) |
| [E0279](diagnostics.md#e0279), [E0280](diagnostics.md#e0280) | `resolve.plain-padding`, `resolve.plain-padding-end` | [Deriving traits](#deriving-traits), [The layout of a type](#the-layout-of-a-type) |
| E0250, E0251, [E0252](diagnostics.md#e0252), E0253 | `resolve.not-delegable`, `resolve.delegate-to`, `resolve.delegate-self`, `resolve.delegate-field` | [Delegating traits to a field](#delegating-traits-to-a-field) |
| [E0417](diagnostics.md#e0417), [E0418](diagnostics.md#e0418) | `borrow.must-consume`, `borrow.must-consume-exit` | [Values that must be consumed](#values-that-must-be-consumed) |
| [E0266](diagnostics.md#e0266), W0006, E0303 | `resolve.bad-cfg`, `resolve.unknown-feature`, `type.unknown-field` | [Conditional compilation](#conditional-compilation) |
| [E0603](diagnostics.md#e0603), [E0606](diagnostics.md#e0606) | `comptime.budget`, `comptime.budget-attribute` | [Compile-time budgets](#compile-time-budgets) |
| E0821 | `test.refines-mismatch` | [Refinement tests](#refinement-tests) |
| [E0281](diagnostics.md#e0281), [E0283](diagnostics.md#e0283), E0702 | `type.fixture-signature`, `type.module-fixture-sync`, `type.test-signature` | [Fixtures](#fixtures) |
| [E0989](diagnostics.md#e0989), [E0991](diagnostics.md#e0991), [D0103](diagnostics.md#d0103) | `unsafe.export-signature`, `unsafe.export-not-c`, `native.unsupported` | [Exported functions](#exported-functions) |
| [E0990](diagnostics.md#e0990) | `unsafe.blocking-not-foreign` | [Foreign functions that block](#foreign-functions-that-block) |
| [E0282](diagnostics.md#e0282), [W0030](diagnostics.md#w0030) | `type.no-alloc`, `resolve.unchecked-attribute` | [Resource limits](#resource-limits) |
| [E0448](diagnostics.md#e0448) | `resolve.float-attribute` | [Fast floating point](#fast-floating-point) |
| [W0023](diagnostics.md#w0023), [W0025](diagnostics.md#w0025), [W0033](diagnostics.md#w0033) | `type.deprecated`, `type.deprecated-plain`, `type.fix-inline-body` | [Deprecation and migrations](#deprecation-and-migrations) |
| [W0028](diagnostics.md#w0028), [W0029](diagnostics.md#w0029) | `type.preview`, `type.preview-plain` | [Preview features](#preview-features) |
| [E0900](diagnostics.md#e0900) | `stage0.unsupported` | [Names of types in diagnostics](#names-of-types-in-diagnostics) |
| [E0225](diagnostics.md#e0225), E0240, E0701 | `resolve.std-only-attribute`, `resolve.sealed-trait`, `cap.internal` | [Attributes of the standard library](#attributes-of-the-standard-library) |
