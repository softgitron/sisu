# 14. Traits

In this chapter you give your types abilities. A trait names an
ability, such as having a price or being written as text, and an `impl`
gives it to a type. You write the traits that `print`, `{x:?}`, `==`,
`+` and `in` use for your own types, let the compiler write the common
ones, and wrap a value in a type of its own so that it cannot be mixed
up with another. Part III starts here: chapter 15 builds on traits to
write code that works for many types. You need about two hours.

## A trait names an ability

A café sells coffee and buns, which are different types with one thing
in common: each has a price. A trait says so. It lists the methods that
a type with the ability has, and an `impl` gives the ability to one
type:

<!-- file: examples/14-traits/priced.sisu -->
```sisu
# A trait names an ability, and an `impl` gives it to a type.
trait Priced:
    def price(self) -> int

struct Coffee:
    cups: int

struct Bun:
    filling: String

impl Priced for Coffee:
    def price(self) -> int:
        return 350 * self.cups

impl Priced for Bun:
    def price(self) -> int:
        return 280

def main():
    let coffee = Coffee(2)
    let bun = Bun("cardamom")
    print("coffee {coffee.price()}, bun {bun.price()}")
    print("together {coffee.price() + bun.price()} cents")
```

<!-- output: examples/14-traits/priced.out -->
```text
coffee 700, bun 280
together 980 cents
```

- `trait Priced:` declares the trait. `def price(self) -> int` has no
  body: it is a required method, which every type with the ability
  must have.
- `impl Priced for Coffee:` gives `Coffee` the ability. Its body holds
  the method with the same signature and a body of its own.
- `coffee.price()` calls the method of `Coffee`'s impl, and
  `bun.price()` the one of `Bun`'s.

A trait is not a type, so a function cannot take "any `Priced`" by
naming the trait where a type goes.

**Try it.** Add a function `def together(a: Priced, b: Priced) -> int`
that returns the sum of the two prices, and use it in the last `print`
of `main`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/14-traits/not_a_type.console -->
```console
$ sisu run not_a_type.sisu
error[E0231]: `Priced` is a trait, not a type; a value of a type that implements it is `dyn Priced`
  --> not_a_type.sisu:19:17
   |
19 | def together(a: Priced, b: Priced) -> int:
   |                 ^^^^^^
  = fix: write `dyn Priced`: `dyn `

error[E0231]: `Priced` is a trait, not a type; a value of a type that implements it is `dyn Priced`
  --> not_a_type.sisu:19:28
   |
19 | def together(a: Priced, b: Priced) -> int:
   |                            ^^^^^^
  = fix: write `dyn Priced`: `dyn `

error[D0006]: the program was not run: errors found (2)
```

E0231 (`resolve.trait-as-type`) says that `Priced` names an ability,
not a type, and its fix offers `dyn Priced`, a value of some type that
has the ability. Chapter 15 shows it, and the other way to write such
a function, a generic one, which is usually the better choice.

</details>

> **From Python.** Python calls a method on any object that has it,
> whatever its class: an object that has a `price` method can be priced.
> In Sisu a type has a trait's ability only where an `impl` says so, so a
> method that happens to have the right name does not make a type
> `Priced` by accident, and the trait is the place to document what the
> method means.

## Methods that a trait provides

A trait can also give a method a body. Such a provided method is
written once, in the trait, in terms of the required ones, and every
type with the ability has it:

<!-- file: examples/14-traits/labels.sisu -->
```sisu
# A provided method has a body, which an impl may replace.
trait Priced:
    def price(self) -> int

    def label(self) -> String:
        let cents = self.price()
        return "{cents // 100}.{cents % 100:02} €"

struct Coffee:
    cups: int

struct Bun:
    filling: String

impl Priced for Coffee:
    def price(self) -> int:
        return 350 * self.cups

impl Priced for Bun:
    def price(self) -> int:
        return 280

    def label(self) -> String:
        return "{self.filling} bun, {self.price()} cents"

def main():
    print(Coffee(2).label())
    print(Bun("cardamom").label())
```

<!-- output: examples/14-traits/labels.out -->
```text
7.00 €
cardamom bun, 280 cents
```

- `label` is provided: its body calls `self.price()`, which every type
  with the ability has, so `Coffee` gets a label without writing one.
- `Bun`'s impl writes its own `label`, which replaces the provided one
  for buns.

A provided method is the nearest thing Sisu has to a method that a
class inherits. Sisu has no inheritance: a type never takes fields from
another type, only methods from a trait it implements.

## What an impl must hold

The compiler checks an impl against its trait. Here `Priced` has a
second required method, and the impl for `Bun` leaves it out:

<!-- file: examples/14-traits/missing_method.sisu -->
```sisu
# An impl that leaves out a required method.
trait Priced:
    def price(self) -> int
    def name(self) -> String

struct Bun:
    filling: String

impl Priced for Bun:
    def price(self) -> int:
        return 280

def main():
    print(Bun("cardamom").price())
```

<!-- output: examples/14-traits/missing_method.console -->
```console
$ sisu run missing_method.sisu
error[E0232]: the impl of `Priced` for `Bun` is missing `name`
  --> missing_method.sisu:9:1
  |
9 | impl Priced for Bun:
  | ^^^^^^^^^^^^^^^^^^^
  = note: write `def name(self) -> String` in the impl

error[D0006]: the program was not run: errors found (1)
```

E0232 (`resolve.missing-trait-method`) names the missing method, and
its note gives the line to write, with the signature the trait gives
it. A method in the impl that the trait does not have gets E0233
(`resolve.not-a-trait-method`): an impl holds the trait's methods only,
and a type's other methods go in its own body, as in chapter 7.

## Text for users

`print(x)` and `"{x}"` write a value for the program's users, and they
need the trait `Display`, which says how. Its one method, `fmt`,
writes the value's text into a `Formatter`:

<!-- file: examples/14-traits/money.sisu -->
```sisu
# `Display` says how a value is written for the program's users.
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100},{self.cents % 100:02} €")

def main():
    let price = Money(1250)
    print(price)
    print("Total: {price}, or [{price:>10}]")
    print("{price:?}")
    print([Money(250), Money(5)])
```

<!-- output: examples/14-traits/money.out -->
```text
12,50 €
Total: 12,50 €, or [   12,50 €]
Money(cents=1250)
[Money(cents=250), Money(cents=5)]
```

- `def fmt(self, mut f: Formatter)` is the method that `Display`
  requires. `f.write(text)` appends text to what the value is written
  as, and can be called as many times as you need.
- A format spec still works: `{price:>10}` pads the text that `fmt`
  wrote to ten characters, aligned right, as chapter 6 showed for
  numbers.
- `{price:?}` writes the `Debug` form, which every type has, and which
  the next section is about.
- A list writes its elements in their `Debug` form, whatever their
  `Display` says, so that a list of texts shows where each text starts
  and ends.

**Try it.** Delete the line `impl Display for Money:`, and move `fmt`
into the body of `Money`, under its field, so that `Money` has the
method and no impl.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/14-traits/no_impl.console -->
```console
$ sisu run no_impl.sisu
error[E0347]: `Money` does not implement `Display`
  --> no_impl.sisu:10:5
   |
10 |     print(price)
   |     ^^^^^^^^^^^^
  = fix: `Money` has every method of `Display`; add `impl Display for Money`

error[D0006]: the program was not run: errors found (1)
```

E0347 (`type.bound-not-satisfied`) says that `Money` does not have the
ability, though it has a method with the right name and signature. The
fix sees the method, and offers to write the `impl` line. A method
named `fmt` is only a method until an impl says what it means.

</details>

> **From Python.** `Display` does what `__str__` does, and `Debug` what
> `__repr__` does. As in Python, a list shows its elements in the form
> for programmers.

## Text for programmers

`Debug` writes a value the way it is written in code, for logs, test
failures and `{x:?}`. Every type has it: a struct or an enum without an
impl of its own gets the derived one, which writes the name and each
field. An impl of `Debug`, whose method is `fmt_debug`, replaces the
derived one, here to keep a password out of the logs:

<!-- file: examples/14-traits/password.sisu -->
```sisu
# An impl of `Debug` replaces the derived one, wherever it is used.
struct Password:
    text: String

impl Debug for Password:
    def fmt_debug(self, mut f: Formatter):
        f.write("Password(<{len(self.text)} characters>)")

struct Login:
    user: String
    password: Password

def main():
    let login = Login("aino", Password("kissa123"))
    print("{login:?}")
    print("{login.password:?}")
```

<!-- output: examples/14-traits/password.out -->
```text
Login(user="aino", password=Password(<8 characters>))
Password(<8 characters>)
```

`Login` has no impl of its own, so it gets the derived `Debug`, which
writes each field in that field's `Debug` form. For `password`, that is
the one `Password`'s impl writes, so the password is hidden wherever a
`Login` is logged too.

## Abilities that the compiler writes

Chapter 7 met `@derive`, which asks the compiler to write an impl from
the type's fields. It derives `Copy`, `Debug`, `Eq`, `PartialOrd`,
`Ord`, `Hash` and `Default`:

<!-- file: examples/14-traits/settings.sisu -->
```sisu
# `@derive` writes the impls of common traits from the fields.
@derive(Debug, Default, Eq, Hash)
struct Settings:
    volume: int = 5
    muted: bool
    theme: String

def main():
    let start = Settings.default()
    print("{start:?}")
    print(start == Settings(volume=5, muted=False, theme=""))
    let quiet = Settings(volume=1, muted=False, theme="dark")
    print(len({start, quiet, Settings.default()}))
    print("{int.default()} {String.default():?} {bool.default()}")
```

<!-- output: examples/14-traits/settings.out -->
```text
Settings(volume=5, muted=False, theme="")
True
2
0 "" False
```

- `Default` gives a type a value to start from, `Settings.default()`:
  each field's own default where it has one, here `volume`, and its
  type's `default()` otherwise. A number's is 0, a text's is empty, and
  a `bool`'s is `False`.
- `Eq` compares field by field, and `Hash`, with `Eq`, lets a value go
  into a set or be a dict's key. The set holds two settings, since the
  third equals the first.
- `Debug` is the derived impl that every type has anyway. Naming it
  says, to a reader, that the type is meant to be shown.

A derived impl needs every field to have the trait, since it works
field by field; chapter 7 showed the error, E0244, for `Copy` and a
`String` field.

**Try it.** Derive `Display` for a type, as in `@derive(Debug, Display)`
on a `Money` struct, and print a value.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/14-traits/derive_display.console -->
```console
$ sisu run derive_display.sisu
error[E0243]: `Display` cannot be derived; `@derive` takes `Copy` and the traits that have a `comptime def derive`: `Debug`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and any trait that declares one
  --> derive_display.sisu:2:16
  |
2 | @derive(Debug, Display)
  |                ^^^^^^^

error[E0347]: `Money` does not implement `Display`
  --> derive_display.sisu:7:5
  |
7 |     print(Money(1250))
  |     ^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (2)
```

E0243 (`resolve.not-derivable`) lists the traits that can be derived
(its list leaves out `Default`, which can). `Display` is not among
them on purpose: text for users is a decision about how a value should
read, and no rule over the fields can make it. A trait of your own can
have a derive too, written in Sisu, which chapter 25 shows.

</details>

`@derive(Into, From)`, which converts a one-field struct to and from
the value it wraps, is designed in
[03](../../proposal/03-types-and-generics.md#type-aliases-and-newtypes)
and is not in the compiler yet.

## Operators are traits

The arithmetic operators on your own types call traits: `a + b` calls
`Add`, and `-`, `*`, `/` and `%` call `Sub`, `Mul`, `Div` and `Rem`.
Unary `-a` calls `Neg`. Here is a vector in the plane:

<!-- file: examples/14-traits/vectors.sisu -->
```sisu
# An operator on a type of your own calls the trait that names it.
@derive(Copy, Debug, Eq)
struct Vector:
    x: f64
    y: f64

impl Add for Vector:
    def add(self, other: Vector) -> Vector:
        return Vector(self.x + other.x, self.y + other.y)

impl Neg for Vector:
    def neg(self) -> Vector:
        return Vector(-self.x, -self.y)

impl Mul[f64] for Vector:
    def mul(self, k: f64) -> Vector:
        return Vector(self.x * k, self.y * k)

def main():
    let step = Vector(1.0, 2.0)
    let wind = Vector(0.5, -1.0)
    print("{step + wind:?} {-step:?} {step * 3.0:?}")
    print(step + wind == Vector(1.5, 1.0))
    var position = Vector(0.0, 0.0)
    for _ in range(3):
        position += step
    print("{position:?}")
```

<!-- output: examples/14-traits/vectors.out -->
```text
Vector(x=1.5, y=1.0) Vector(x=-1.0, y=-2.0) Vector(x=3.0, y=6.0)
True
Vector(x=3.0, y=6.0)
```

- `impl Add for Vector:` gives vectors `+`. Its method takes the right
  operand as `other` and returns the sum.
- `Mul[f64]` is `Mul` with a parameter: the type of the right operand,
  here a number, so `step * 3.0` scales a vector. `Add` without a
  parameter takes a right operand of the same type, a `Vector`.
- `position += step` works once `+` does.
- `==` comes from the derived `Eq`. The comparisons `<`, `<=`, `>` and
  `>=` come from `PartialOrd` and `Ord`, which the section after next
  writes by hand.

The left operand decides which impl runs, so `3.0 * step` is an
operation on a number. On numbers the operators are the language's own
operations, and stage 0 rejects `3.0 * step` with E0310
(`type.binary-operator`); write the vector first.

The set of operators is fixed: a type can give a meaning to `+`, but a
program cannot make up a new operator or change the precedence of one
(D-022). `in` and the prelude function `len` work the same way:
`x in xs` calls the trait `Contains` of `xs`, and `len(xs)` calls
`Len`:

<!-- file: examples/14-traits/opening.sisu -->
```sisu
# `in` calls `Contains`, and `len` calls `Len`.
struct Opening:
    first: int
    last: int

impl Contains[int] for Opening:
    def contains(self, hour: int) -> bool:
        return self.first <= hour <= self.last

impl Len for Opening:
    def len(self) -> int:
        return self.last - self.first + 1

def main():
    let sauna = Opening(17, 21)
    print("{18 in sauna} {9 in sauna}")
    print("open for {len(sauna)} hours")
```

<!-- output: examples/14-traits/opening.out -->
```text
True False
open for 5 hours
```

> **From Python.** Python's special methods, `__add__`, `__neg__`,
> `__contains__` and `__len__`, become `Add`, `Neg`, `Contains` and
> `Len`. There is no `__radd__`: the impl is chosen by the left
> operand's type.

## Traits that need other traits

A trait can require another: a type with it must have the other too,
which is called a supertrait. `Ord`, the total order that `sort` and
`max` use, requires `Eq` and `PartialOrd`. A type that orders itself
another way than field by field writes all three, or derives `Eq` and
writes the orders:

<!-- file: examples/14-traits/leaderboard.sisu -->
```sisu
# An order of your own: the highest score comes first.
@derive(Eq)
struct Player:
    name: String
    score: int

impl PartialOrd for Player:
    def partial_cmp(self, other: Player) -> Ordering?:
        return Some(self.cmp(other))

impl Ord for Player:
    def cmp(self, other: Player) -> Ordering:
        return other.score.cmp(self.score)

def main():
    var board = [Player("Aino", 7), Player("Eero", 12), Player("Roni", 9)]
    board.sort()
    for player in board:
        print("{player.score:>3} {player.name}")
```

<!-- output: examples/14-traits/leaderboard.out -->
```text
 12 Eero
  9 Roni
  7 Aino
```

- `cmp` returns an `Ordering`: `Less`, `Equal` or `Greater`.
  `other.score.cmp(self.score)` compares the scores the other way
  round, so the highest score sorts first.
- `partial_cmp` is the order that `<` and `>` use. Some types have a
  partial order only: no `f64` is less or more than NaN, so `f64` has
  `PartialOrd` and not `Ord`. A type with a total order gives
  `Some(self.cmp(other))`.
- `board.sort()` sorts the list in place by that order. Stage 0 has a
  bug here: `sorted(board)` stops with an internal error when the
  elements are not `Copy` and their order is written by hand, so the
  example sorts in place.

Your own trait names its supertraits in parentheses after its name,
and its provided methods can use what the supertraits give:

<!-- file: examples/14-traits/labelled.sisu -->
```sisu
# A trait can require another, and its methods can use it.
trait Labelled(Display):
    def label(self) -> String:
        return "[ {self} ]"

struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100},{self.cents % 100:02} €")

impl Labelled for Money:
    pass

def main():
    print(Money(1250).label())
```

<!-- output: examples/14-traits/labelled.out -->
```text
[ 12,50 € ]
```

`label` writes `{self}`, which needs `Display`. Every type with
`Labelled` has `Display`, so the trait can rely on it, and an impl for
a type without it does not compile:

<!-- file: examples/14-traits/no_display.sisu -->
```sisu
# An impl of a trait whose supertrait the type does not have.
trait Labelled(Display):
    def label(self) -> String:
        return "[ {self} ]"

struct Weight:
    grams: int

impl Labelled for Weight:
    pass

def main():
    print(Weight(90).label())
```

<!-- output: examples/14-traits/no_display.console -->
```console
$ sisu run no_display.sisu
error[E0242]: `Weight` implements `Labelled`, which requires `Display`, but `Weight` does not implement `Display`
  --> no_display.sisu:9:1
  |
9 | impl Labelled for Weight:
  | ^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0242 (`resolve.supertrait-missing`) names the trait the type lacks.
The fix is an impl of `Display` for `Weight`, or a trait that does not
require it.

## Types that each impl chooses

A trait can leave a type open for each impl to choose, as it leaves a
method's body. Such an associated type is declared with `type` in the
trait and given with `type ... =` in each impl. Two sensors read values
of different types:

<!-- file: examples/14-traits/sensors.sisu -->
```sisu
# An associated type is a type that each impl of the trait chooses.
trait Sensor:
    type Reading
    def read(mut self) -> Reading

struct Thermometer:
    celsius: f64

struct Door:
    opened: int

impl Sensor for Thermometer:
    type Reading = f64

    def read(mut self) -> f64:
        self.celsius += 0.5
        return self.celsius

impl Sensor for Door:
    type Reading = bool

    def read(mut self) -> bool:
        self.opened += 1
        return self.opened % 2 == 1

def main():
    var sauna = Thermometer(78.0)
    var door = Door(0)
    for _ in range(2):
        print("{sauna.read()} °C, door open: {door.read()}")
```

<!-- output: examples/14-traits/sensors.out -->
```text
78.5 °C, door open: True
79.0 °C, door open: False
```

`type Reading` says that each sensor reads some type, and `read`
returns it. The thermometer's impl says it is an `f64`, and the door's a
`bool`. An impl that does not say gets E0237
(`resolve.missing-assoc-type`).

The operator traits have one too, `Output`, the type of the result,
which is the type itself unless the impl says otherwise. A dot product
multiplies two vectors into a number:

<!-- file: examples/14-traits/dot.sisu -->
```sisu
# The operator traits have an associated type too: `Output`.
@derive(Copy, Debug)
struct Vector:
    x: f64
    y: f64

impl Mul for Vector:
    type Output = f64

    def mul(self, other: Vector) -> f64:
        return self.x * other.x + self.y * other.y

impl Mul[f64] for Vector:
    def mul(self, k: f64) -> Vector:
        return Vector(self.x * k, self.y * k)

def main():
    let a = Vector(1.0, 2.0)
    let b = Vector(3.0, -1.0)
    print(a * b)
    print("{a * 2.0:?}")
```

<!-- output: examples/14-traits/dot.out -->
```text
1.0
Vector(x=2.0, y=4.0)
```

`Vector` implements `Mul` twice, once with each type of right operand,
and the operand decides which impl `*` calls. A type can have
constants of its own, declared with `const` in its body; constants
that a trait declares for each impl to give are designed in
[03](../../proposal/03-types-and-generics.md#traits) and are not in the
compiler yet. The `Iterator` trait of chapter 16 has an associated
type, `Item`, the type of what it gives.

## One impl for each type and trait

A type implements a trait at most once, in the whole program and every
package it uses. Two impls of `Display` for `Money` would leave `print`
two ways to write money:

<!-- file: examples/14-traits/two_displays.sisu -->
```sisu
# Two impls of one trait for one type.
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100},{self.cents % 100:02} €")

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("€{self.cents // 100}.{self.cents % 100:02}")

def main():
    print(Money(1250))
```

<!-- output: examples/14-traits/two_displays.console -->
```console
$ sisu run two_displays.sisu
error[E0235]: `Display` is implemented for `Money` more than once; a type implements a trait at most once
  --> two_displays.sisu:9:1
  |
9 | impl Display for Money:
  | ^^^^^^^^^^^^^^^^^^^^^^
  |
5 | impl Display for Money:
  | ---------------------- the other impl is here

error[D0006]: the program was not run: errors found (1)
```

E0235 (`resolve.overlapping-impl`) points at both. The rule is called
coherence, and it is what lets a `dict` or `sorted` behave the same in
every package: a type's `Hash` or `Ord` is the same wherever it is
used. The same holds for the standard library's own impls: `Ordering`
has a `Display`, so a program that writes another gets E0235 too.

Who may write an impl is part of the rule, the orphan rule: an impl
lives in the package of the trait or in the package of the type. Your
package may implement its own traits for any type and any trait for
its own types, but a library may not implement the standard library's
`Ord` for the standard library's `f64`, since two libraries could
choose two orders. The package you build, the root package, is the
exception: nothing depends on it, so it may implement any trait for any
type (D-013). A program can therefore decide that its `f64` values have
a total order:

<!-- file: examples/14-traits/float_order.sisu -->
```sisu
# The root package may give a standard type a standard trait.
impl Ord for f64:
    def cmp(self, other: f64) -> Ordering:
        return self.cmp(other)

def main():
    let times = [12.5, 9.75, 11.0]
    print(sorted(times))
    print(sorted(times)[0] < times[0])
```

<!-- output: examples/14-traits/float_order.out -->
```text
[9.75, 11.0, 12.5]
True
```

`sorted` needs `Ord`, which `f64` lacks, and without the impl it gets
E0347, whose fix offers this impl. Inside it, `self.cmp(other)`
calls the method that every `f64` has, which panics on a NaN, so a NaN
in `times` is a bug that stops the program. That is a choice this
program makes for itself. A library cannot make it for every program
that uses it, and gets E0236 (`resolve.orphan-impl`) for the same impl;
chapter 17 shows a library. If the standard library adds the impl one
day, the program gets E0235, and the fix is to delete its own.

## A new type for an old value

A distance and a time are both `f64` values, so nothing stops a program
from adding one to the other. A newtype is a struct with one field,
which wraps a value in a type of its own at no cost: in memory a
`Metres` is its `f64`. `@delegate` gives it the abilities of the field
it wraps, without impls written by hand:

<!-- file: examples/14-traits/units.sisu -->
```sisu
# A newtype wraps one value in a type of its own, at no cost.
@delegate(Add, Sub, Eq, PartialOrd, Display, to="value")
@derive(Copy, Debug)
struct Metres:
    value: f64

@delegate(Add, Eq, PartialOrd, Display, to="value")
@derive(Copy, Debug)
struct Seconds:
    value: f64

def speed(distance: Metres, time: Seconds) -> f64:
    return distance.value / time.value

def main():
    let run = Metres(400.0) + Metres(800.0)
    let time = Seconds(150.0) + Seconds(90.0)
    print("{run} m in {time} s, {speed(run, time)} m/s")
    print("{run > Metres(1000.0)} {run:?}")
```

<!-- output: examples/14-traits/units.out -->
```text
1200.0 m in 240.0 s, 5.0 m/s
True Metres(value=1200.0)
```

`@delegate(Add, Sub, Eq, PartialOrd, Display, to="value")` writes impls
that call the traits of the field `value`: `Metres + Metres` adds the
two values and wraps the sum, and a `Metres` is written as its number.
`Seconds` delegates fewer traits, since subtracting times is not
needed here. `@delegate` takes the operator traits, `Eq`, the orders,
`Hash`, `Display`, `Debug` and `Len`.

**Predict.** Swap the arguments of `speed` in `main`, and add a line
`print("all together: {run + time}")`. Before you run it, decide what
each change does.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/14-traits/mixed_units.console -->
```console
$ sisu run mixed_units.sisu
error[E0300]: expected `Metres`, found `Seconds`
  --> mixed_units.sisu:18:40
   |
18 |     print("{run} m in {time} s, {speed(time, run)} m/s")
   |                                        ^^^^

error[E0300]: expected `Seconds`, found `Metres`
  --> mixed_units.sisu:18:46
   |
18 |     print("{run} m in {time} s, {speed(time, run)} m/s")
   |                                              ^^^

error[E0310]: `+` cannot be applied to `Metres` and `Seconds`
  --> mixed_units.sisu:20:27
   |
20 |     print("all together: {run + time}")
   |                           ^^^^^^^^^^

error[D0006]: the program was not run: errors found (3)
```

The arguments in the wrong order get E0300 (`type.mismatch`), and the
sum of a distance and a time gets E0310: the types have no `Add`
between them. With two `f64` values both mistakes would run and print
a wrong number.

</details>

## Methods for a type you did not write

`impl Type:`, without a trait, adds methods to a type, including a
type of the standard library:

<!-- file: examples/14-traits/shout.sisu -->
```sisu
# `impl Type:` adds methods to a type that you did not declare.
impl str:
    def shout(self) -> String:
        return "{self.to_uppercase()}!"

impl int:
    def is_even(self) -> bool:
        return self % 2 == 0

def main():
    print("hei maailma".shout())
    print("{4.is_even()} {7.is_even()}")
```

<!-- output: examples/14-traits/shout.out -->
```text
HEI MAAILMA!
True False
```

The methods are visible in the module that declares them, and in a
package wherever that module is imported, which chapter 17 shows
(Q-12). They are ordinary methods: they reach the value through its
public methods and fields, and a name that the type has already gets
E0217 (`resolve.duplicate-method`).

## Exercise 14.1: predict the methods

Open
[`manual/exercises/14-traits/predict_methods.sisu`](../exercises/14-traits/predict_methods.sisu):

<!-- file: exercises/14-traits/predict_methods.sisu -->
```sisu
##! Exercise 14.1, predict the methods. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_methods.sisu`. The hints are in chapter 14 of the
##! tutorial, under "Exercise 14.1: predict the methods".

trait Greeter:
    def name(self) -> String

    def greet(self) -> String:
        return "Hei, {self.name()}"

struct Guest:
    first: String

struct Host:
    first: String

impl Greeter for Guest:
    def name(self) -> String:
        return self.first.copy()

impl Greeter for Host:
    def name(self) -> String:
        return "host {self.first}"

    def greet(self) -> String:
        return "Tervetuloa, I am the {self.name()}"

struct Room:
    number: int

impl Display for Room:
    def fmt(self, mut f: Formatter):
        f.write("room {self.number}")

const GUEST: str = todo("what a guest says")
const HOST: str = todo("what a host says")
const SHOWN: str = todo("a room written for users")
const DEBUGGED: str = todo("a room written for programmers")
const PADDED: str = todo("a room padded on the left")

test "a guest":
    assert Guest("Aino").greet() == GUEST

test "a host":
    assert Host("Eero").greet() == HOST

test "a room for users":
    assert "{Room(12)}" == SHOWN

test "a room for programmers":
    assert "{Room(12):?}" == DEBUGGED

test "a padded room":
    assert "[{Room(7):>8}]" == PADDED
```

<!-- output: solutions/14-traits/predict_methods.console -->
```console
$ sisu test predict_methods.sisu
warning[W0001]: typed hole `what a guest says` of type `str`
  --> predict_methods.sisu:38:20
   |
38 | const GUEST: str = todo("what a guest says")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `what a host says` of type `str`
  --> predict_methods.sisu:39:19
   |
39 | const HOST: str = todo("what a host says")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a room written for users` of type `str`
  --> predict_methods.sisu:40:20
   |
40 | const SHOWN: str = todo("a room written for users")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a room written for programmers` of type `str`
  --> predict_methods.sisu:41:23
   |
41 | const DEBUGGED: str = todo("a room written for programmers")
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a room padded on the left` of type `str`
  --> predict_methods.sisu:42:21
   |
42 | const PADDED: str = todo("a room padded on the left")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "a guest" ... blocked by todo("what a guest says")
test "a host" ... blocked by todo("what a host says")
test "a room for users" ... blocked by todo("a room written for users")
test "a room for programmers" ... blocked by todo("a room written for programmers")
test "a padded room" ... blocked by todo("a room padded on the left")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

A type's impl may replace a provided method. Check, for each type,
which `greet` runs, and then which `name` that `greet` calls.

</details>

<details>
<summary>Hint 2</summary>

`Room` has an impl of `Display` and none of `Debug`, so `{x:?}` uses
the derived form, as for `Money` in the section "Text for users". A
spec pads the text that `fmt` wrote.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/14-traits/predict_methods.sisu`](../solutions/14-traits/predict_methods.sisu).

## Exercise 14.2: a receipt

Open
[`manual/exercises/14-traits/receipt.sisu`](../exercises/14-traits/receipt.sisu):

<!-- file: exercises/14-traits/receipt.sisu -->
```sisu
##! Exercise 14.2, a receipt. Kind: fix. Level: 1.
##!
##! A shop prints a line of its receipt for each item it sells. The file
##! does not compile. Run `sisu test receipt.sisu`, read the diagnostics,
##! and fix the impls, not the trait or the tests, so that every test
##! passes. The hints are in chapter 14 of the tutorial, under
##! "Exercise 14.2: a receipt".

## Something the shop sells: it is written as its name, and has a price
## in cents.
trait Item(Display):
    def price(self) -> int

    ## The item's line on a receipt.
    def line(self) -> String:
        return "{self}: {self.price()} c"

struct Ticket:
    concert: String

struct Coffee:
    cups: int

impl Display for Coffee:
    def fmt(self, mut f: Formatter):
        f.write("coffee x{self.cups}")

impl Item for Ticket:
    def price(self) -> int:
        return 4500

impl Item for Coffee:
    def cost(self) -> int:
        return 350 * self.cups

test "a coffee":
    assert Coffee(2).line() == "coffee x2: 700 c"
    assert Coffee(1).line() == "coffee x1: 350 c"

test "a ticket":
    assert Ticket("Sibelius").line() == "ticket to Sibelius: 4500 c"
```

<!-- output: solutions/14-traits/receipt.console -->
```console
$ sisu test receipt.sisu
error[E0242]: `Ticket` implements `Item`, which requires `Display`, but `Ticket` does not implement `Display`
  --> receipt.sisu:28:1
   |
28 | impl Item for Ticket:
   | ^^^^^^^^^^^^^^^^^^^^

error[E0232]: the impl of `Item` for `Coffee` is missing `price`
  --> receipt.sisu:32:1
   |
32 | impl Item for Coffee:
   | ^^^^^^^^^^^^^^^^^^^^
  = note: write `def price(self) -> int` in the impl

error[E0233]: `cost` is not a method of `Item`
  --> receipt.sisu:33:9
   |
33 |     def cost(self) -> int:
   |         ^^^^

error[D0006]: the program was not run: errors found (3)
```

<details>
<summary>Hint 1</summary>

There are three diagnostics, and each names what one impl lacks or has
too much of. The tests say how a ticket is written.

</details>

<details>
<summary>Hint 2</summary>

`Item` requires `Display`, and only the coffee has an impl of it. A
method of an impl must have the name the trait gives it.

</details>

When every test passes, compare your answer with
[`manual/solutions/14-traits/receipt.sisu`](../solutions/14-traits/receipt.sisu).

## Exercise 14.3: vectors on a grid

Open
[`manual/exercises/14-traits/grid_vectors.sisu`](../exercises/14-traits/grid_vectors.sisu):

<!-- file: exercises/14-traits/grid_vectors.sisu -->
```sisu
##! Exercise 14.3, vectors on a grid. Kind: complete. Level: 1.
##!
##! A robot moves on a grid by vectors of whole steps. Replace the holes
##! in the impls so that `+`, `-`, unary `-` and `*` work on vectors and
##! every test passes. Run the tests with `sisu test grid_vectors.sisu`.
##! The hints are in chapter 14 of the tutorial, under "Exercise 14.3:
##! vectors on a grid".

## A move on the grid: `x` steps east and `y` steps north.
@derive(Copy, Debug, Eq)
struct Vector:
    x: int
    y: int

impl Add for Vector:
    def add(self, other: Vector) -> Vector:
        return todo("the sum")

impl Sub for Vector:
    def sub(self, other: Vector) -> Vector:
        return todo("the difference")

impl Neg for Vector:
    def neg(self) -> Vector:
        return todo("the opposite move")

impl Mul[int] for Vector:
    def mul(self, times: int) -> Vector:
        return todo("the move made `times` times")

## Where the robot ends after it makes each of `moves` from `start`.
def end_of(start: Vector, moves: list[Vector]) -> Vector:
    var here = start
    for step in moves:
        here += step
    return here

test "adding and taking away":
    assert Vector(1, 2) + Vector(3, -4) == Vector(4, -2)
    assert Vector(5, 5) - Vector(2, 7) == Vector(3, -2)

test "the opposite move":
    assert -Vector(2, -3) == Vector(-2, 3)
    assert Vector(4, 1) + -Vector(4, 1) == Vector(0, 0)

test "a move made several times":
    assert Vector(1, -2) * 3 == Vector(3, -6)
    assert Vector(7, 7) * 0 == Vector(0, 0)

test "a walk":
    let east = Vector(1, 0)
    let north = Vector(0, 1)
    assert end_of(Vector(0, 0), [east, north, east * 2]) == Vector(3, 1)
    assert end_of(Vector(5, 5), [-north, -east]) == Vector(4, 4)
```

<!-- output: solutions/14-traits/grid_vectors.console -->
```console
$ sisu test grid_vectors.sisu
warning[W0001]: typed hole `the sum` of type `Vector`
  --> grid_vectors.sisu:17:16
   |
17 |         return todo("the sum")
   |                ^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the difference` of type `Vector`
  --> grid_vectors.sisu:21:16
   |
21 |         return todo("the difference")
   |                ^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the opposite move` of type `Vector`
  --> grid_vectors.sisu:25:16
   |
25 |         return todo("the opposite move")
   |                ^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the move made `times` times` of type `Vector`
  --> grid_vectors.sisu:29:16
   |
29 |         return todo("the move made `times` times")
   |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "adding and taking away" ... blocked by todo("the sum")
test "the opposite move" ... blocked by todo("the opposite move")
test "a move made several times" ... blocked by todo("the move made `times` times")
test "a walk" ... blocked by todo("the move made `times` times")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Each method makes a new `Vector` from the fields of `self`, and of
`other` or `times`, one field at a time.

</details>

<details>
<summary>Hint 2</summary>

`end_of` already uses `+=`, which needs nothing but `Add`. The section
"Operators are traits" has the impls for a vector of floats.

</details>

When every test passes, compare your answer with
[`manual/solutions/14-traits/grid_vectors.sisu`](../solutions/14-traits/grid_vectors.sisu).

## Exercise 14.4: money that prints

Open
[`manual/exercises/14-traits/money.sisu`](../exercises/14-traits/money.sisu):

<!-- file: exercises/14-traits/money.sisu -->
```sisu
##! Exercise 14.4, money that prints. Kind: write. Level: 2.
##!
##! An amount of money is kept in cents, and written for people the way
##! Finland writes it: the euros, a comma, two digits of cents, a space
##! and the euro sign, with a minus sign first for an amount below zero.
##! Write the body of `fmt` so that every test passes. Run the tests with
##! `sisu test money.sisu`. The hints are in chapter 14 of the tutorial,
##! under "Exercise 14.4: money that prints".

## An amount of money, in cents.
@derive(Copy, Eq)
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        todo("fmt")

test "whole euros":
    assert "{Money(1200)}" == "12,00 €"
    assert "{Money(0)}" == "0,00 €"

test "euros and cents":
    assert "{Money(1250)}" == "12,50 €"
    assert "{Money(7)}" == "0,07 €"

test "amounts below zero":
    assert "{Money(-1250)}" == "-12,50 €"
    assert "{Money(-5)}" == "-0,05 €"

test "a column of amounts":
    let lines = ["[{Money(m):>9}]" for m in [250, -99999]]
    assert lines == ["[   2,50 €]", "[-999,99 €]"]
```

<!-- output: solutions/14-traits/money.console -->
```console
$ sisu test money.sisu
warning[W0001]: typed hole `fmt` of type `_`
  --> money.sisu:17:9
   |
17 |         todo("fmt")
   |         ^^^^^^^^^^^

test "whole euros" ... blocked by todo("fmt")
test "euros and cents" ... blocked by todo("fmt")
test "amounts below zero" ... blocked by todo("fmt")
test "a column of amounts" ... blocked by todo("fmt")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Start from the `Money` of the section "Text for users", and run the
tests: which ones fail, and what do they print?

</details>

<details>
<summary>Hint 2</summary>

`//` and `%` round towards minus infinity, as chapter 2 showed, so
`-5 // 100` is -1 and `-5 % 100` is 95. Work out the sign first, and
then the euros and cents of the amount without its sign.

</details>

<details>
<summary>Hint 3</summary>

The last test pads the text that `fmt` writes. Nothing in `fmt` needs
to know about the spec.

</details>

When every test passes, compare your answer with
[`manual/solutions/14-traits/money.sisu`](../solutions/14-traits/money.sisu).

## Exercise 14.5: metres and seconds

Open
[`manual/exercises/14-traits/training.sisu`](../exercises/14-traits/training.sisu):

<!-- file: exercises/14-traits/training.sisu -->
```sisu
##! Exercise 14.5, metres and seconds. Kind: fix. Level: 3.
##!
##! A training log keeps the distance and the time of each run. Both were
##! plain `f64` values until a time was added to a distance by mistake,
##! so now each has a type of its own. The file does not compile. Run
##! `sisu test training.sisu`, read the diagnostics, and fix the types
##! and the functions, not the tests, so that every test passes. The
##! hints are in chapter 14 of the tutorial, under "Exercise 14.5: metres
##! and seconds".

## A distance.
struct Metres:
    value: f64

## A time.
struct Seconds:
    value: f64

## One run of the log.
struct Run:
    distance: Metres
    time: Seconds

## The total distance of `runs`.
def total_distance(runs: list[Run]) -> Metres:
    var total = Metres(0.0)
    for run in runs:
        total += run.distance
    return total

## The total time of `runs`.
def total_time(runs: list[Run]) -> Seconds:
    var total = Seconds(0.0)
    for run in runs:
        total += run.distance
    return total

## The distance of the longest of `runs`.
def longest(runs: list[Run]) -> Metres:
    var best = Metres(0.0)
    for run in runs:
        if run.distance > best:
            best = run.distance
    return best

## The log in a line, such as "2 runs, 5000.0 m in 1500.0 s".
def summary(runs: list[Run]) -> String:
    let distance = total_distance(runs)
    return "{len(runs)} runs, {distance} m in {total_time(runs)} s"

## A week of runs.
def week() -> list[Run]:
    return [
        Run(Metres(4000.0), Seconds(1200.0)),
        Run(Metres(5000.0), Seconds(1500.0)),
        Run(Metres(3000.0), Seconds(900.0)),
    ]

test "totals":
    assert total_distance(week()) == Metres(12000.0)
    assert total_time(week()) == Seconds(3600.0)
    assert total_time([]) == Seconds(0.0)

test "the longest run":
    assert longest(week()) == Metres(5000.0)
    assert longest(week()[:1]) == Metres(4000.0)

test "a summary":
    assert summary(week()) == "3 runs, 12000.0 m in 3600.0 s"
```

<!-- output: solutions/14-traits/training.console -->
```console
$ sisu test training.sisu
error[E0310]: `+` cannot be applied to `Metres` and `Metres`
  --> training.sisu:28:9
   |
28 |         total += run.distance
   |         ^^^^^^^^^^^^^^^^^^^^^

error[E0310]: `+` cannot be applied to `Seconds` and `Metres`
  --> training.sisu:35:9
   |
35 |         total += run.distance
   |         ^^^^^^^^^^^^^^^^^^^^^

error[E0368]: `Metres` has no order, so it cannot be compared with `>` or sorted
  --> training.sisu:42:12
   |
42 |         if run.distance > best:
   |            ^^^^^^^^^^^^^^^^^^^

error[E0338]: cannot move `run.distance` out of a borrowed value; copy it with `.copy()`
  --> training.sisu:43:20
   |
43 |             best = run.distance
   |                    ^^^^^^^^^^^^
  = fix: copy it with `.copy()`: `.copy()`

error[E0347]: `Metres` does not implement `Display`
  --> training.sisu:49:32
   |
49 |     return "{len(runs)} runs, {distance} m in {total_time(runs)} s"
   |                                ^^^^^^^^
  = fix: show its `Debug` form: `:?`

error[E0347]: `Seconds` does not implement `Display`
  --> training.sisu:49:48
   |
49 |     return "{len(runs)} runs, {distance} m in {total_time(runs)} s"
   |                                                ^^^^^^^^^^^^^^^^
  = fix: show its `Debug` form: `:?`

error[E0347]: `Metres` does not implement `Eq`
  --> training.sisu:60:12
   |
60 |     assert total_distance(week()) == Metres(12000.0)
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0347]: `Seconds` does not implement `Eq`
  --> training.sisu:61:12
   |
61 |     assert total_time(week()) == Seconds(3600.0)
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0347]: `Metres` does not implement `Eq`
  --> training.sisu:65:12
   |
65 |     assert longest(week()) == Metres(5000.0)
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (9)
```

<details>
<summary>Hint 1</summary>

Most of the diagnostics say that a type lacks an ability that the code
uses: `+`, `>`, `==` or being written in text. The section "A new type
for an old value" gives a type those abilities in one line.

</details>

<details>
<summary>Hint 2</summary>

E0338 is chapter 10's error: a run is borrowed, and its distance would
move out of it. A distance is a number underneath, so it can be copied
as a number is.

</details>

<details>
<summary>Hint 3</summary>

Once the types have their abilities, one diagnostic is left. It is the
mistake that the types were made to catch.

</details>

When every test passes, compare your answer with
[`manual/solutions/14-traits/training.sisu`](../solutions/14-traits/training.sisu).

## What you learned

- A trait names an ability: required methods without a body, provided
  methods with one. `impl Trait for Type:` gives a type the ability,
  and the compiler checks the impl against the trait (E0232, E0233).
- `Display` writes a value for users and `Debug` for programmers; every
  type has `Debug`. `@derive` writes `Copy`, `Debug`, `Eq`,
  `PartialOrd`, `Ord`, `Hash` and `Default` from the fields, and never
  `Display`.
- The operators call `Add`, `Sub`, `Mul`, `Div`, `Rem` and `Neg`, `in`
  calls `Contains` and `len` calls `Len`. The set of operators is fixed.
- A trait can require supertraits (E0242) and leave associated types
  for each impl to choose (E0237).
- A type implements a trait at most once (E0235). An impl lives in the
  package of the trait or of the type, except in the root package,
  which may implement anything.
- A newtype wraps a value in a type of its own, and `@delegate` gives
  it the field's abilities. `impl Type:` adds methods to any type.

Chapter 15 writes functions and types that work for every type with an
ability: generics, bounded by traits, and `dyn` values that choose their
impl while the program runs.
