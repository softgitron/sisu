# 7. Structs and enums

In this chapter you define types of your own: structs, which hold
several named values, and enums, whose values are one of a few
variants. You give them methods, let the compiler derive comparisons
and copies, and take their values apart with `match`, which checks that
you have covered every case. You need about an hour and a half.

## Grouping values in a struct

A struct is a type made of named fields, each with a type of its own.
`struct` declares one, and calling the struct's name builds a value:

<!-- file: examples/07-structs-and-enums/point.sisu -->
```sisu
# A struct groups named fields into one value.
struct Point:
    x: f64
    y: f64

def main():
    let home = Point(x=3.0, y=4.0)
    var boat = Point(1.0, 1.0)
    boat.x = 2.5
    print("home is at {home.x}, {home.y}")
    print("the boat is at {boat.x}, {boat.y}")
    print("{boat:?}")
```

`sisu run point.sisu` prints:

<!-- output: examples/07-structs-and-enums/point.out -->
```text
home is at 3.0, 4.0
the boat is at 2.5, 1.0
Point(x=2.5, y=1.0)
```

- `struct Point:` declares the type, and the indented lines declare
  its fields, `x` and `y`, both `f64`. A type's name is `CamelCase`, as
  chapter 2's naming rules say.
- `Point(x=3.0, y=4.0)` builds a point by naming the fields, and
  `Point(1.0, 1.0)` fills them by position, in the order of their
  declaration. Every field gets a value.
- `home.x` reads a field. A field of a `var` binding can be changed,
  as `boat.x = 2.5` does; the fields of a `let` binding cannot.
- `{boat:?}` shows the value's `Debug` form, which chapter 6 met: the
  type's name and each field. Every type you declare has one. A plain
  `{boat}` does not compile, because a struct has no form for people to
  read until you write one, which chapter 14 shows.

> **From Python.** A struct is close to a Python dataclass: the fields
> are declared with their types, and the type comes with a way to build
> it, so there is no `__init__` to write. There is no inheritance
> either (D-012); chapter 14 shows what Sisu has instead.

## Fields with default values

A field can have a default value, given after its type. Building the
struct can then leave that field out:

<!-- file: examples/07-structs-and-enums/orders.sisu -->
```sisu
# A field with a default can be left out when the struct is built.
struct Order:
    item: String
    quantity: int = 1
    express: bool = False

def main():
    let bread = Order("rye bread")
    let coffee = Order("coffee", 3)
    let berries = Order(item="cloudberries", express=True)
    print("{bread:?}")
    print("{coffee:?}")
    print("{berries:?}")
```

<!-- output: examples/07-structs-and-enums/orders.out -->
```text
Order(item="rye bread", quantity=1, express=False)
Order(item="coffee", quantity=3, express=False)
Order(item="cloudberries", quantity=1, express=True)
```

`Order("rye bread")` gives only the item, so the other two fields
take their defaults. `Order("coffee", 3)` fills the first two fields by
position. `Order(item="cloudberries", express=True)` names the fields
it sets, in any order, and skips the one between them. A field without
a default must always be given; leaving one out gets E0337
(`type.missing-field`), whose fix puts a hole in its place.

The item is a `String`, and the literal `"rye bread"` becomes one
because a `String` is expected there, as chapter 6 showed for `Path`.

**Try it.** Change the type of `item` from `String` to `str` and run
the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/07-structs-and-enums/borrowed_field.console -->
```console
$ sisu run borrowed_field.sisu
error[E0411]: `Order` holds `item: str`, which borrows; only a `view struct` may hold a borrow
  --> borrowed_field.sisu:3:5
  |
3 |     item: str
  |     ^^^^^^^^^
  |
2 | struct Order:
  |        ----- `Order` is declared here
  = fix: own the text instead: `String`
  = fix: declare it `view struct Order`: `view `

error[D0006]: the program was not run: errors found (1)
```

A `str` is a view of text that lives somewhere else, and a struct owns
all of its fields. A field that borrowed text could outlive the text it
points into, so E0411 (`borrow.view-field-in-plain-struct`) refuses it.
Its fix turns the type into a `view struct`, a struct that may hold
borrows, which chapter 12 explains. That is not the fix you want here:
give the field the type `String`, and the struct owns its text.

</details>

## Methods

A method is a function declared in the body of a struct. Its first
parameter is `self`, the value it is called on:

<!-- file: examples/07-structs-and-enums/methods.sisu -->
```sisu
# Methods are functions in the struct's body that take `self`.
struct Point:
    x: f64
    y: f64

    def distance(self, other: Point) -> f64:
        let dx = self.x - other.x
        let dy = self.y - other.y
        return (dx * dx + dy * dy).sqrt()

    def shift(mut self, dx: f64, dy: f64):
        self.x += dx
        self.y += dy

def main():
    let home = Point(x=0.0, y=0.0)
    var boat = Point(x=0.0, y=0.0)
    boat.shift(3.0, 4.0)
    print("the boat is {boat.distance(home)} from home")
```

<!-- output: examples/07-structs-and-enums/methods.out -->
```text
the boat is 5.0 from home
```

- `boat.distance(home)` calls `distance` with `self` standing for
  `boat` and `other` for `home`. A plain `self` is a read-only borrow,
  like any plain parameter: the method can read the fields, not change
  them.
- `mut self` lets a method change the value it is called on, as
  `shift` changes `x` and `y`. It can also replace the whole value,
  with `self = ...`.
- A `mut self` method needs a value that can change, so `boat` is a
  `var`. Calling `shift` on `home`, a `let`, gets E0341
  (`type.not-mutable`), whose fix declares `home` with `var`.

**Try it.** Delete `mut` from `def shift(mut self, dx: f64, dy: f64):`
and run the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/07-structs-and-enums/no_mut_self.console -->
```console
$ sisu run no_mut_self.sisu
error[E0341]: cannot modify `self.x`; it is not declared with `var`
  --> no_mut_self.sisu:12:9
   |
12 |         self.x += dx
   |         ^^^^^^
  = fix: take `self` with `mut`: `mut `

error[E0341]: cannot modify `self.y`; it is not declared with `var`
  --> no_mut_self.sisu:13:9
   |
13 |         self.y += dy
   |         ^^^^^^
  = fix: take `self` with `mut`: `mut `

error[D0006]: the program was not run: errors found (2)
```

E0341 reports each change to a field of a plain `self`. Its message
speaks of `var`, and its note of a `ref mut` borrow, which chapter 11
explains; neither is the fix here. A method that changes its value says
so in its first parameter, so the fix is to put `mut` back.

</details>

## Functions that belong to a type

A function in a struct's body without `self` is an associated function.
It belongs to the type rather than to one value, and you call it on the
type's name:

<!-- file: examples/07-structs-and-enums/temperature.sisu -->
```sisu
# A function in the body without `self` is called on the type.
struct Temperature:
    celsius: f64

    def from_fahrenheit(degrees: f64) -> Temperature:
        return Temperature((degrees - 32.0) * 5.0 / 9.0)

    def freezing() -> Temperature:
        return Temperature(0.0)

    def is_below(self, other: Temperature) -> bool:
        return self.celsius < other.celsius

def main():
    let outside = Temperature.from_fahrenheit(14.0)
    let freezing = Temperature.freezing()
    print("{outside.celsius} °C")
    print("below freezing: {outside.is_below(freezing)}")
```

<!-- output: examples/07-structs-and-enums/temperature.out -->
```text
-10.0 °C
below freezing: True
```

`Temperature.from_fahrenheit(14.0)` and `Temperature.freezing()` both
build a temperature. Associated functions that build a value are named
constructors, and the standard library has many, such as
`String.new()` and `list.with_capacity(n)`. Since a function name in
Sisu has one meaning (D-050), each way of building a value gets a name
that says what it starts from.

> **From Python.** An associated function plays the part of a Python
> `@classmethod` or `@staticmethod`, without a decorator: leaving out
> `self` is enough.

## Comparing and copying with derive

A struct you declare can be shown with `{x:?}`, but it cannot be
compared, sorted or copied until you ask. `@derive` on the line before
the struct asks the compiler to write those abilities for you:

<!-- file: examples/07-structs-and-enums/versions.sisu -->
```sisu
# `@derive` gives a struct comparisons and copies, field by field.
@derive(Copy, Eq, Ord, Hash)
struct Version:
    major: int
    minor: int

def main():
    let installed = Version(1, 4)
    let wanted = installed
    print("{installed == wanted} {installed < Version(2, 0)}")
    print("{sorted([Version(1, 10), Version(0, 9), Version(1, 2)])}")
    print("{len({installed, wanted})} different versions")
```

**Predict.** Before you run it, decide in which order the three
versions on the sorted line come out. Then compare:

<!-- output: examples/07-structs-and-enums/versions.out -->
```text
True True
[Version(major=0, minor=9), Version(major=1, minor=2), Version(major=1, minor=10)]
1 different versions
```

Each name in `@derive(...)` is a trait, an ability a type can have,
which chapter 14 covers in full. The four here are the common ones:

- `Eq` gives `==` and `!=`.
- `Ord` gives `<`, `<=`, `>`, `>=`, and with them `sorted`, `max` and
  `min`. A derived order compares the first field, then the second if
  the first ones are equal, and so on in the order the fields are
  declared. So version 1.10 comes after 1.2, where the text "1.10"
  would come before "1.2".
- `Hash` lets the value go into a set or be a dict's key. The set of
  `installed` and `wanted` holds one version, because they are equal.
- `Copy` makes `let wanted = installed` copy the version, as it would
  copy a number, so both names can be used. Without it the version
  would move, as chapter 5's list did. Only a struct whose fields are
  all `Copy` can derive it. Copying a `String` takes new memory, which
  Sisu does only when you write `.copy()`, so a struct with a `String`
  field gets E0244 (`resolve.derive-field`) for `@derive(Copy)`.

**Try it.** Delete the line `@derive(Copy, Eq, Ord, Hash)` and run the
program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/07-structs-and-enums/no_derive.console -->
```console
$ sisu run no_derive.sisu
error[E0347]: `Version` does not implement `Eq`
  --> no_derive.sisu:9:13
  |
9 |     print("{installed == wanted} {installed < Version(2, 0)}")
  |             ^^^^^^^^^^^^^^^^^^^
  = fix: derive it: `@derive(Eq)`

error[E0368]: `Version` has no order, so it cannot be compared with `<` or sorted
  --> no_derive.sisu:9:35
  |
9 |     print("{installed == wanted} {installed < Version(2, 0)}")
  |                                   ^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: derive it: `@derive(PartialOrd)`

error[E0347]: `Version` does not implement `Ord`
  --> no_derive.sisu:10:13
   |
10 |     print("{sorted([Version(1, 10), Version(0, 9), Version(1, 2)])}")
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: derive it: `@derive(Eq, PartialOrd, Ord)`

error[E0438]: `list[ref Version]` would hold references, and a collection holds values it owns or views, never references (04, rule 3)
  --> no_derive.sisu:10:13
   |
10 |     print("{sorted([Version(1, 10), Version(0, 9), Version(1, 2)])}")
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: `sorted` gives references to items that are not `Copy`, which a list does not hold: sort a list of its own in place instead, `xs.sort()`

error[E0347]: `Version` does not implement `Hash`
  --> no_derive.sisu:11:17
   |
11 |     print("{len({installed, wanted})} different versions")
   |                 ^^^^^^^^^^^^^^^^^^^
  = fix: derive it: `@derive(Eq, Hash)`

error[D0006]: the program was not run: errors found (5)
```

Each line that needs an ability now names the trait it lacks: E0347
(`type.bound-not-satisfied`) for `Eq`, `Ord` and `Hash`, and E0368
(`type.not-ordered`) for `<`, each with the fix that writes the
`@derive` line. E0438 comes from `sorted` without `Copy`, which hands
out references to the items rather than the items, as chapter 11
explains; its note says that a list sorts itself in place with
`xs.sort()`. Without `Copy`, the ownership check would report the move of
`installed` as well, once these errors were fixed.

</details>

An attribute such as `@derive` is data that the compiler reads, not a
function that runs (D-046).

## Enums: one of several variants

An enum is a type whose value is one of a fixed set of variants.
`enum` declares one, with a variant on each line:

<!-- file: examples/07-structs-and-enums/suits.sisu -->
```sisu
# An enum's value is one of a fixed set of variants.
enum Suit:
    Clubs
    Diamonds
    Hearts
    Spades

    def is_red(self) -> bool:
        return match self:
            case Diamonds | Hearts: True
            case Clubs | Spades: False

def main():
    let trump = Suit.Hearts
    print("{trump:?} is red: {trump.is_red()}")
    for suit in [Clubs, Spades]:
        print("{suit:?} is red: {suit.is_red()}")
```

<!-- output: examples/07-structs-and-enums/suits.out -->
```text
Hearts is red: True
Clubs is red: False
Spades is red: False
```

- `Suit.Hearts` is the variant's full name. Where only one enum has a
  variant of that name, the variant's own name is enough, as in the
  list `[Clubs, Spades]` and in the `case` lines.
- An enum can have methods in its body, as a struct can, and
  `is_red` is one.
- `match self` chooses by variant, and `Diamonds | Hearts` matches
  either of two, as `|` did for values in chapter 4.

When two enums have a variant of the same name, the name alone is
ambiguous:

<!-- file: examples/07-structs-and-enums/two_reds.sisu -->
```sisu
# Two enums with a variant of the same name.
enum Light:
    Red
    Amber
    Green

enum Wine:
    Red
    White

def main():
    let order = Red
    print("{order:?}")
```

<!-- output: examples/07-structs-and-enums/two_reds.console -->
```console
$ sisu run two_reds.sisu
error[E0211]: `Red` is a variant of more than one type (Light, Wine); qualify it, as in `Light.Red`
  --> two_reds.sisu:12:17
   |
12 |     let order = Red
   |                 ^^^

error[D0006]: the program was not run: errors found (1)
```

E0211 (`resolve.ambiguous-variant`) names both enums and shows a full
name. Write `Wine.Red`, or give the binding its type, `let order: Wine
= Red`, so that the name has one meaning.

## Variants that carry values

A variant can carry values, written after its name like a struct's
fields, by position or by name. Each variant has its own:

<!-- file: examples/07-structs-and-enums/weather.sisu -->
```sisu
# A variant can carry values, by position or by name.
enum Weather:
    Sunny
    Rain(f64)
    Fog(metres: int)
    Snow(cm: int, wet: bool)

def report(weather: Weather) -> String:
    return match weather:
        case Sunny: "sunny"
        case Rain(mm): "rain, {mm} mm"
        case Fog(metres): "fog, {metres} m to see"
        case Snow(cm, wet): "snow, {cm} cm, wet: {wet}"

def main():
    let week = [Sunny, Rain(2.5), Fog(metres=200), Snow(cm=10, wet=False)]
    for day in week:
        print(report(day))
```

<!-- output: examples/07-structs-and-enums/weather.out -->
```text
sunny
rain, 2.5 mm
fog, 200 m to see
snow, 10 cm, wet: False
```

- `Sunny` carries nothing. `Rain(f64)` carries one value by position.
  `Fog(metres: int)` and `Snow(cm: int, wet: bool)` carry named values,
  which can be given by keyword, as in `Snow(cm=10, wet=False)`, or by
  position.
- In a `case`, a variant with names in its parentheses takes its values
  out: `Rain(mm)` matches any rain and binds what it carries to `mm`.
  The names in the pattern are yours to choose, and they bind by
  position.

A struct has one shape; an enum has several, each with its own fields.
A value of `Weather` is always exactly one of them, and `match` is how
you find out which.

> **From Python.** Python would give each kind of weather a class of
> its own and tell them apart with `isinstance`, or use `Enum`, whose
> members are fixed constants that cannot carry a day's amount of rain.
> A Sisu enum does both jobs: one type, whose values differ by variant
> and carry what each variant declares.

## Patterns inside patterns

A pattern can look inside the values it matches, and the parts of a
pattern are patterns too:

<!-- file: examples/07-structs-and-enums/shapes.sisu -->
```sisu
# Patterns nest, name fields, skip fields and test with `if`.
struct Point:
    x: int
    y: int

enum Shape:
    Circle(center: Point, radius: int)
    Square(corner: Point, side: int)

def describe(shape: Shape) -> String:
    return match shape:
        case Circle(center=Point(x=0, y=0), ..): "a circle at the origin"
        case Circle(radius=r, ..) if r > 10: "a big circle"
        case Circle(Point(x, y), r): "a circle of radius {r} at {x}, {y}"
        case Square(side=s, ..): "a square with sides of {s}"

def main():
    print(describe(Circle(Point(0, 0), 5)))
    print(describe(Circle(Point(2, 3), 50)))
    print(describe(Circle(Point(2, 3), 5)))
    print(describe(Square(Point(1, 9), 2)))
```

<!-- output: examples/07-structs-and-enums/shapes.out -->
```text
a circle at the origin
a big circle
a circle of radius 5 at 2, 3
a square with sides of 2
```

- `Circle(center=Point(x=0, y=0), ..)` nests a struct's pattern inside
  a variant's, and the literals `0` match only that value. `center=`
  names the field the inner pattern is for, as Python's keyword
  patterns do.
- `..` stands for the fields the pattern does not mention.
- `if r > 10` is a guard: the arm matches only when it is `True`.
- `Circle(Point(x, y), r)` takes everything apart by position.

The arms are tried in the order they are written, and the first that
matches is chosen, which is why the big circle at 2, 3 never reaches
the third arm.

In a pattern, a name that starts with a capital letter is a variant or
a constant to compare with, and a name in lowercase makes a binding.
That rule, which chapter 2 met in `let`, has a trap in `match`:

<!-- file: examples/07-structs-and-enums/lowercase_case.sisu -->
```sisu
# A lowercase name in a pattern binds whatever value is there.
enum Light:
    Red
    Amber
    Green

def advice(light: Light) -> str:
    return match light:
        case green: "go"
        case Amber: "wait"
        case Red: "stop"

def main():
    print(advice(Red))
```

<!-- output: examples/07-structs-and-enums/lowercase_case.console -->
```console
$ sisu run lowercase_case.sisu
warning[W0003]: this `case` arm is unreachable; the arms above it cover every value it matches
  --> lowercase_case.sisu:10:14
   |
10 |         case Amber: "wait"
   |              ^^^^^

warning[W0003]: this `case` arm is unreachable; the arms above it cover every value it matches
  --> lowercase_case.sisu:11:14
   |
11 |         case Red: "stop"
   |              ^^^

go
```

`case green:` does not compare with `Green`: it binds any light to a
new name, `green`, so the first arm matches every value, and a red
light gets "go". W0003 (`type.unreachable-arm`) warns that the other
arms can never run. Take the warning as an error: the fix is `case
Green:`.

> **From Python.** Python's `match` has the same rule, and it refuses
> a capture like `case green:` before other arms. Sisu warns and runs
> the program, so read the warnings.

## Every case, checked

A `match` must cover every value it can be given. Leave out a variant,
and the program does not compile. This is `weather.sisu` with the arm
for fog forgotten:

<!-- file: examples/07-structs-and-enums/forgotten_case.sisu -->
```sisu
# A variant can carry values, by position or by name.
enum Weather:
    Sunny
    Rain(f64)
    Fog(metres: int)
    Snow(cm: int, wet: bool)

def report(weather: Weather) -> String:
    return match weather:
        case Sunny: "sunny"
        case Rain(mm): "rain, {mm} mm"
        case Snow(cm, wet): "snow, {cm} cm, wet: {wet}"

def main():
    let week = [Sunny, Rain(2.5), Fog(metres=200), Snow(cm=10, wet=False)]
    for day in week:
        print(report(day))
```

<!-- output: examples/07-structs-and-enums/forgotten_case.console -->
```console
$ sisu run forgotten_case.sisu
error[E0314]: this `match` does not cover `Fog(_)`
  --> forgotten_case.sisu:9:12
  |
9 |     return match weather:
  |            ^^^^^^^^^^^^^
  = note: add an arm for `Fog(_)`, or a `case _` arm

error[D0006]: the program was not run: errors found (1)
```

E0314 (`type.non-exhaustive`) names a case that no arm covers,
`Fog(_)`, in which `_` stands for any value. When several are missing,
it names one; add it and run again. The note offers two fixes:

- An arm for the variant, here `case Fog(metres): ...`. This is the fix
  to choose when the variant needs its own answer.
- A `case _` arm, which matches whatever the arms before it did not.
  It also matches any variant you add to the enum later, without a
  word from the compiler, so prefer one arm per variant for your own
  enums (D-043).

The check is why a new variant is safe to add: every `match` that does
not handle it stops compiling, and the diagnostics list the places to
change.

## Enums that contain themselves

A variant can carry a value of its own enum, which makes structures of
any size out of one type. Here a route is either over, or a stop and
the rest of the route:

<!-- file: examples/07-structs-and-enums/route.sisu -->
```sisu
# An enum can hold a value of its own type.
enum Route:
    Arrived
    Via(stop: String, rest: Route)

def stops(route: Route) -> int:
    return match route:
        case Arrived: 0
        case Via(rest=rest, ..): 1 + stops(rest)

def itinerary(route: Route) -> String:
    return match route:
        case Arrived: "arrived"
        case Via(stop, rest): "{stop}, then {itinerary(rest)}"

def main():
    let route = Via("Oulu", Via("Tampere", Via("Helsinki", Arrived)))
    print("{stops(route)} stops: {itinerary(route)}")
```

<!-- output: examples/07-structs-and-enums/route.out -->
```text
3 stops: Oulu, then Tampere, then Helsinki, then arrived
```

`stops` and `itinerary` follow the shape of the type: an arm per
variant, and a call on the part that is itself a `Route`, as the
recursive functions of chapter 3 did. A `Route` cannot hold another
`Route` directly, since that one would hold another, without end. The
compiler keeps each `rest` in memory of its own and stores a pointer to
it, which is called boxing; you write the type as it reads (D-043), and
chapter 13 shows `Box`, which does the same by hand.

## Another name for a type

`type` gives an existing type a second name, an alias:

<!-- file: examples/07-structs-and-enums/aliases.sisu -->
```sisu
# `type` gives an existing type a second name.
type Row = list[int]
type Grid = list[Row]
type Metres = f64
type Seconds = f64

def total(grid: Grid) -> int:
    var so_far = 0
    for row in grid:
        so_far += sum(row)
    return so_far

def main():
    let grid: Grid = [[1, 2, 3], [4, 5, 6]]
    print("the grid adds up to {total(grid)}")
    let distance: Metres = 100.0
    let time: Seconds = 9.58
    print("metres plus seconds: {distance + time}")
```

<!-- output: examples/07-structs-and-enums/aliases.out -->
```text
the grid adds up to 21
metres plus seconds: 109.58
```

An alias shortens a long type and says what it is for, as `Grid` does
for a list of lists. It is the same type as the one it names, so a
`Grid` is accepted wherever a `list[list[int]]` is. That is also its
limit: `Metres` and `Seconds` are both `f64`, and adding them compiles.
For a type that must not be mixed with another, declare a struct with
one field; chapter 14 shows how to make such a type as handy as the
number inside it.

## Exercise 7.1: predict the matches

Open
[`manual/exercises/07-structs-and-enums/predict_matches.sisu`](../exercises/07-structs-and-enums/predict_matches.sisu):

<!-- file: exercises/07-structs-and-enums/predict_matches.sisu -->
```sisu
##! Exercise 7.1, predict the matches. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_matches.sisu`. The hints are in chapter 7 of the
##! tutorial, under "Exercise 7.1: predict the matches".

@derive(Copy, Eq, Ord)
enum Size:
    Small
    Medium
    Large

@derive(Copy, Eq, Ord)
struct Parcel:
    size: Size
    kilos: int = 1

## The postage of `parcel`, in euros.
def postage(parcel: Parcel) -> int:
    return match parcel:
        case Parcel(size=Small, ..): 3
        case Parcel(kilos=k, ..) if k > 10: 20
        case Parcel(Medium, _): 7
        case _: 12

const SMALL_AND_HEAVY: int = todo("a heavy small parcel")
const MEDIUM_AND_HEAVY: int = todo("a heavy medium parcel")
const LARGE: int = todo("a large parcel of the default weight")
const SIZES: bool = todo("two sizes compared")
const PARCELS: bool = todo("two parcels compared")

test "a heavy small parcel":
    assert postage(Parcel(Small, 30)) == SMALL_AND_HEAVY

test "a heavy medium parcel":
    assert postage(Parcel(Medium, 15)) == MEDIUM_AND_HEAVY

test "a large parcel of the default weight":
    assert postage(Parcel(Large)) == LARGE

test "two sizes compared":
    assert (Large > Small) == SIZES

test "two parcels compared":
    assert (Parcel(Medium, 2) < Parcel(Small, 9)) == PARCELS
```

<!-- output: solutions/07-structs-and-enums/predict_matches.console -->
```console
$ sisu test predict_matches.sisu
warning[W0001]: typed hole `a heavy small parcel` of type `int`
  --> predict_matches.sisu:27:30
   |
27 | const SMALL_AND_HEAVY: int = todo("a heavy small parcel")
   |                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a heavy medium parcel` of type `int`
  --> predict_matches.sisu:28:31
   |
28 | const MEDIUM_AND_HEAVY: int = todo("a heavy medium parcel")
   |                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a large parcel of the default weight` of type `int`
  --> predict_matches.sisu:29:20
   |
29 | const LARGE: int = todo("a large parcel of the default weight")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `two sizes compared` of type `bool`
  --> predict_matches.sisu:30:21
   |
30 | const SIZES: bool = todo("two sizes compared")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `two parcels compared` of type `bool`
  --> predict_matches.sisu:31:23
   |
31 | const PARCELS: bool = todo("two parcels compared")
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "a heavy small parcel" ... blocked by todo("a heavy small parcel")
test "a heavy medium parcel" ... blocked by todo("a heavy medium parcel")
test "a large parcel of the default weight" ... blocked by todo("a large parcel of the default weight")
test "two sizes compared" ... blocked by todo("two sizes compared")
test "two parcels compared" ... blocked by todo("two parcels compared")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

The arms of a `match` are tried in the order they are written, and
the first one that matches wins, even when a later arm would match
too.

</details>

<details>
<summary>Hint 2</summary>

`Parcel(Large)` gives only the size; the section "Fields with default
values" says what the weight is. A derived order looks at the fields
in the order they are declared, and at the variants in the order they
are listed.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/07-structs-and-enums/predict_matches.sisu`](../solutions/07-structs-and-enums/predict_matches.sisu).

## Exercise 7.2: the area of a shape

Open
[`manual/exercises/07-structs-and-enums/shape_area.sisu`](../exercises/07-structs-and-enums/shape_area.sisu):

<!-- file: exercises/07-structs-and-enums/shape_area.sisu -->
```sisu
##! Exercise 7.2, the area of a shape. Kind: fix. Level: 1.
##!
##! `area` was written before `Shape` had triangles, and the file does
##! not compile. Run `sisu test shape_area.sisu`, read the diagnostic,
##! and fix `area` so that every test passes. The hints are in chapter
##! 7 of the tutorial, under "Exercise 7.2: the area of a shape".

from std.math import PI

enum Shape:
    Circle(radius: f64)
    Rectangle(width: f64, height: f64)
    Triangle(base: f64, height: f64)

## The area of `shape`.
def area(shape: Shape) -> f64:
    return match shape:
        case Circle(radius): PI * radius * radius
        case Rectangle(width, height): width * height

test "circles":
    assert area(Circle(radius=1.0)) == PI
    assert area(Circle(radius=2.0)) == 4.0 * PI

test "rectangles":
    assert area(Rectangle(width=3.0, height=2.0)) == 6.0
    assert area(Rectangle(width=0.5, height=8.0)) == 4.0

test "triangles":
    assert area(Triangle(base=4.0, height=3.0)) == 6.0
    assert area(Triangle(base=5.0, height=2.0)) == 5.0
```

<!-- output: solutions/07-structs-and-enums/shape_area.console -->
```console
$ sisu test shape_area.sisu
error[E0314]: this `match` does not cover `Triangle(_, _)`
  --> shape_area.sisu:17:12
   |
17 |     return match shape:
   |            ^^^^^^^^^^^
  = note: add an arm for `Triangle(_, _)`, or a `case _` arm

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

The diagnostic names the case that `area` does not cover. The area of
a triangle is half its base times its height.

</details>

<details>
<summary>Hint 2</summary>

Of the two fixes that the note offers, one makes the file compile and
still fails the tests for triangles. The section "Every case, checked"
says which to choose.

</details>

When every test passes, compare your answer with
[`manual/solutions/07-structs-and-enums/shape_area.sisu`](../solutions/07-structs-and-enums/shape_area.sisu).

## Exercise 7.3: a hand of cards

Open
[`manual/exercises/07-structs-and-enums/cards.sisu`](../exercises/07-structs-and-enums/cards.sisu):

<!-- file: exercises/07-structs-and-enums/cards.sisu -->
```sisu
##! Exercise 7.3, a hand of cards. Kind: fix. Level: 2.
##!
##! A card game shows each hand sorted by suit, in the order clubs,
##! diamonds, hearts and spades, and by rank within a suit. The file
##! does not compile, because cards cannot be compared yet. Run
##! `sisu test cards.sisu`, read each diagnostic, and change the two
##! types, not `sort_hand` or the tests, until every test passes. The
##! hints are in chapter 7 of the tutorial, under "Exercise 7.3: a hand
##! of cards".

@derive(Copy)
enum Suit:
    Clubs
    Diamonds
    Hearts
    Spades

## A playing card. `rank` is 2 to 10 for the number cards, and 11, 12,
## 13 and 14 for the jack, queen, king and ace.
@derive(Copy)
struct Card:
    rank: int
    suit: Suit

## `hand` sorted by suit, and by rank within a suit.
def sort_hand(hand: list[Card]) -> list[Card]:
    return sorted(hand)

test "two cards of different suits":
    let hand = [Card(rank=2, suit=Spades), Card(rank=14, suit=Clubs)]
    let expected = [Card(rank=14, suit=Clubs), Card(rank=2, suit=Spades)]
    assert sort_hand(hand) == expected

test "a hand of four":
    let hand = [
        Card(rank=12, suit=Hearts),
        Card(rank=3, suit=Spades),
        Card(rank=9, suit=Diamonds),
        Card(rank=2, suit=Hearts),
    ]
    let expected = [
        Card(rank=9, suit=Diamonds),
        Card(rank=2, suit=Hearts),
        Card(rank=12, suit=Hearts),
        Card(rank=3, suit=Spades),
    ]
    assert sort_hand(hand) == expected
```

<!-- output: solutions/07-structs-and-enums/cards.console -->
```console
$ sisu test cards.sisu
error[E0347]: `Card` does not implement `Ord`
  --> cards.sisu:27:12
   |
27 |     return sorted(hand)
   |            ^^^^^^^^^^^^

error[E0347]: `list[Card]` does not implement `Eq`
  --> cards.sisu:32:12
   |
32 |     assert sort_hand(hand) == expected
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0347]: `list[Card]` does not implement `Eq`
  --> cards.sisu:47:12
   |
47 |     assert sort_hand(hand) == expected
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (3)
```

<details>
<summary>Hint 1</summary>

The diagnostics name the abilities that `sorted` and `==` need. The
section "Comparing and copying with derive" shows how a type gets them.

</details>

<details>
<summary>Hint 2</summary>

A struct can derive a comparison only when the types of its fields
have it too; a new diagnostic says so once `Card` asks for it.

</details>

<details>
<summary>Hint 3</summary>

When the file compiles, a test fails: the cards are sorted, in another
order than the game wants. A derived order decides by the first field
that differs.

</details>

When every test passes, compare your answer with
[`manual/solutions/07-structs-and-enums/cards.sisu`](../solutions/07-structs-and-enums/cards.sisu).

## Exercise 7.4: fractions

Open
[`manual/exercises/07-structs-and-enums/fractions.sisu`](../exercises/07-structs-and-enums/fractions.sisu):

<!-- file: exercises/07-structs-and-enums/fractions.sisu -->
```sisu
##! Exercise 7.4, fractions. Kind: write. Level: 2.
##!
##! A `Fraction` is kept in lowest terms with a bottom above 0, so that
##! two equal fractions have equal fields. Write the bodies of `new`,
##! `plus`, `flip` and `show` so that every test passes. Run the tests
##! with `sisu test fractions.sisu`. The hints are in chapter 7 of the
##! tutorial, under "Exercise 7.4: fractions".

## A fraction `top / bottom` in lowest terms, with `bottom` above 0.
@derive(Copy, Eq)
struct Fraction:
    top: int
    bottom: int

    ## The fraction `top / bottom` in lowest terms; `bottom` is not 0.
    def new(top: int, bottom: int) -> Fraction:
        return todo("new")

    ## The sum of this fraction and `other`.
    def plus(self, other: Fraction) -> Fraction:
        return todo("plus")

    ## Turns the fraction upside down; its `top` is not 0.
    def flip(mut self):
        self = todo("flip")

    ## The fraction as text, such as "3/4", or "2" for a whole number.
    def show(self) -> String:
        return todo("show")

## The greatest common divisor of `a` and `b`, which are not both 0.
def gcd(a: int, b: int) -> int:
    if b == 0:
        return abs(a)
    return gcd(b, a % b)

test "lowest terms":
    assert Fraction.new(2, 4) == Fraction(1, 2)
    assert Fraction.new(6, 3) == Fraction(2, 1)

test "a negative bottom":
    assert Fraction.new(1, -2) == Fraction(-1, 2)
    assert Fraction.new(-3, -9) == Fraction(1, 3)

test "adding":
    let half = Fraction.new(1, 2)
    assert half.plus(Fraction.new(1, 3)) == Fraction(5, 6)
    assert Fraction.new(1, 4).plus(Fraction.new(3, 4)) == Fraction(1, 1)

test "upside down":
    var f = Fraction.new(2, 3)
    f.flip()
    assert f == Fraction(3, 2)
    var g = Fraction.new(-1, 4)
    g.flip()
    assert g == Fraction(-4, 1)

test "as text":
    assert Fraction.new(3, 4).show() == "3/4"
    assert Fraction.new(4, 2).show() == "2"
    assert Fraction.new(1, -5).show() == "-1/5"
```

<!-- output: solutions/07-structs-and-enums/fractions.console -->
```console
$ sisu test fractions.sisu
warning[W0001]: typed hole `new` of type `Fraction`
  --> fractions.sisu:17:16
   |
17 |         return todo("new")
   |                ^^^^^^^^^^^

warning[W0001]: typed hole `plus` of type `Fraction`
  --> fractions.sisu:21:16
   |
21 |         return todo("plus")
   |                ^^^^^^^^^^^^

warning[W0001]: typed hole `flip` of type `Fraction`
  --> fractions.sisu:25:16
   |
25 |         self = todo("flip")
   |                ^^^^^^^^^^^^

warning[W0001]: typed hole `show` of type `String`
  --> fractions.sisu:29:16
   |
29 |         return todo("show")
   |                ^^^^^^^^^^^^

test "lowest terms" ... blocked by todo("new")
test "a negative bottom" ... blocked by todo("new")
test "adding" ... blocked by todo("new")
test "upside down" ... blocked by todo("new")
test "as text" ... blocked by todo("new")

test "lowest terms" ... blocked by todo("new")
what the test printed:
  in scope at the typed hole `todo("new")`:
    top = 2
    bottom = 4

test "a negative bottom" ... blocked by todo("new")
what the test printed:
  in scope at the typed hole `todo("new")`:
    top = 1
    bottom = -2

test "adding" ... blocked by todo("new")
what the test printed:
  in scope at the typed hole `todo("new")`:
    top = 1
    bottom = 2

test "upside down" ... blocked by todo("new")
what the test printed:
  in scope at the typed hole `todo("new")`:
    top = 2
    bottom = 3

test "as text" ... blocked by todo("new")
what the test printed:
  in scope at the typed hole `todo("new")`:
    top = 3
    bottom = 4

0 passed, 0 failed, 5 blocked
```

`gcd` is chapter 3's greatest common divisor, ready to use.

<details>
<summary>Hint 1</summary>

Start with `new`, since every test builds its fractions with it. Divide
both numbers by their greatest common divisor, and if the bottom is
negative, change the sign of both.

</details>

<details>
<summary>Hint 2</summary>

`a/b + c/d` is `(a*d + c*b) / (b*d)`, which is not always in lowest
terms. `new` already knows how to fix that, and so can `flip`.

</details>

<details>
<summary>Hint 3</summary>

The idea is to keep one function in charge of the rule: anything that
makes a fraction from two numbers goes through `new`.

</details>

When every test passes, compare your answer with
[`manual/solutions/07-structs-and-enums/fractions.sisu`](../solutions/07-structs-and-enums/fractions.sisu).

## Exercise 7.5: an expression evaluator

Open
[`manual/exercises/07-structs-and-enums/expressions.sisu`](../exercises/07-structs-and-enums/expressions.sisu):

<!-- file: exercises/07-structs-and-enums/expressions.sisu -->
```sisu
##! Exercise 7.5, an expression evaluator. Kind: write. Level: 3.
##!
##! `Expr` is an arithmetic expression as a tree: a number, a negation,
##! a sum or a product. Write the bodies of `evaluate` and `show` so
##! that every test passes. Run the tests with
##! `sisu test expressions.sisu`. The hints are in chapter 7 of the
##! tutorial, under "Exercise 7.5: an expression evaluator".

enum Expr:
    Number(int)
    Negate(Expr)
    Plus(Expr, Expr)
    Times(Expr, Expr)

## The value of `e`.
def evaluate(e: Expr) -> int:
    return todo("evaluate")

## `e` as text, written as a person would write it: a sum inside a
## product or a negation is in parentheses, and nothing else is.
def show(e: Expr) -> String:
    return todo("show")

test "a number":
    assert evaluate(Number(7)) == 7
    assert show(Number(7)) == "7"

test "sums and products":
    let e = Plus(Number(2), Times(Number(3), Number(4)))
    assert evaluate(e) == 14
    assert show(e) == "2 + 3 * 4"

test "a sum inside a product":
    let e = Times(Plus(Number(1), Number(2)), Number(3))
    assert evaluate(e) == 9
    assert show(e) == "(1 + 2) * 3"

test "negations":
    let e = Times(Number(2), Negate(Number(3)))
    assert evaluate(e) == -6
    assert show(e) == "2 * -3"
    let f = Negate(Plus(Number(1), Negate(Number(4))))
    assert evaluate(f) == 3
    assert show(f) == "-(1 + -4)"
```

<!-- output: solutions/07-structs-and-enums/expressions.console -->
```console
$ sisu test expressions.sisu
warning[W0001]: typed hole `evaluate` of type `int`
  --> expressions.sisu:17:12
   |
17 |     return todo("evaluate")
   |            ^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `show` of type `String`
  --> expressions.sisu:22:12
   |
22 |     return todo("show")
   |            ^^^^^^^^^^^^

test "a number" ... blocked by todo("evaluate")
test "sums and products" ... blocked by todo("evaluate")
test "a sum inside a product" ... blocked by todo("evaluate")
test "negations" ... blocked by todo("evaluate")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Write `evaluate` first. Its `match` has an arm for each variant, and
an arm calls `evaluate` on each part that is an expression, as
`stops` did in the section "Enums that contain themselves".

</details>

<details>
<summary>Hint 2</summary>

In `show`, whether a sum needs parentheses depends on where it stands,
not on the sum itself. The parts of a product and of a negation are
the places that need them.

</details>

<details>
<summary>Hint 3</summary>

A second, small function for "a part of a product or a negation" keeps
the rule in one place.

</details>

When every test passes, compare your answer with
[`manual/solutions/07-structs-and-enums/expressions.sisu`](../solutions/07-structs-and-enums/expressions.sisu).

## What you learned

- `struct` declares a type of named fields, with defaults if you want
  them. A value is built by position or by keyword, and its fields are
  read with `.` and changed through a `var`.
- Methods take `self`, or `mut self` to change the value. Associated
  functions take no `self` and are called on the type, often to build
  a value.
- `@derive(Copy, Eq, Ord, Hash)` gives a type copies, `==`, an order
  field by field, and a place in sets and as dict keys. Every type has
  a `Debug` form for `{x:?}`.
- `enum` declares a type whose value is one of its variants, each of
  which can carry values. A variant's own name is enough where it is
  not ambiguous.
- `match` takes values apart with patterns that nest, name fields,
  skip them with `..` and test with guards. A lowercase name in a
  pattern binds. A `match` must cover every case, and E0314 names a
  case it misses.
- An enum can contain itself, and `type` gives a type a second name.

Chapter 8 is about values that may be absent and operations that can
fail: optional values, errors as values, and the operators that make
both short to handle.
