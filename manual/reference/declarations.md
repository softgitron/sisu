# Declarations

This page lists every declaration: the items that make up a module
(functions, constants, types, traits, impls, tests, imports and foreign
functions) and the bindings that a body declares with `let` and `var`.
For each it gives the form, what it declares, where it may stand and the
diagnostics that guard it. What the types mean is
[R2](types.md); how the modules of a package see each other's items is
[R13](modules.md).

**See also:** the tutorial's [chapter 2](../tutorial/02-values.md)
(`let`, `var` and `const`), [chapter 3](../tutorial/03-functions.md)
(functions), [chapter 7](../tutorial/07-structs-and-enums.md) (structs
and enums), [chapter 8](../tutorial/08-errors.md) (error sets),
[chapter 14](../tutorial/14-traits.md) (traits and impls),
[chapter 17](../tutorial/17-modules-and-packages.md) (imports and
visibility) and [chapter 19](../tutorial/19-testing.md) (tests); the
proposal's [02, "Declarations"](../../proposal/02-syntax.md#declarations)
and [03, "Traits"](../../proposal/03-types-and-generics.md#traits);
[R17, Diagnostics](diagnostics.md#names-and-declarations) for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains.

## Items and statements

A module is a sequence of items, each starting at the left margin:

| Item | Declares |
|---|---|
| `def`, `pure def`, `comptime def` | a function ([Functions](#functions)) |
| `const` | a constant ([Constants](#constants)) |
| `struct`, `view struct` | a struct ([Structs](#structs), [View structs](#view-structs)) |
| `enum` | an enum ([Enums](#enums)) |
| `error` | an error set ([Error sets](#error-sets)) |
| `trait` | a trait ([Traits](#traits)) |
| `impl` | methods for a type, or a trait's implementation ([Impls](#impls)) |
| `type` | an alias or a refinement ([Type declarations](#type-declarations)) |
| `test`, `suite`, `bench` | a test, a group of tests, a benchmark ([Tests, suites and benchmarks](#tests-suites-and-benchmarks)) |
| `import`, `from ... import` | names from other modules ([Imports](#imports)) |
| `extern` | foreign functions ([Foreign functions](#foreign-functions)) |

- Items may come in any order: a function may call one declared after
  it, and a type may name one declared later.
- An item may have attributes on the lines before it
  ([Attributes and doc comments](#attributes-and-doc-comments)) and
  `pub` before it ([Visibility](#visibility)).
- Two items of one module cannot share a name (E0202,
  [R17](diagnostics.md#e0202)).
- Statements, `let` and `var` among them, stand only in the body of a
  function or a test; one at the top level of a module is E0107
  ([R17](diagnostics.md#e0107)). A script is the exception: its
  top-level statements are its `main`
  ([R13](modules.md#scripts-and-inline-metadata)).

<!-- file: examples/reference-declarations/items.sisu -->
```sisu
# The items of a module may come in any order.
def main():
    print(double(LIMIT))
    print(Point(-3).norm())

def double(n: int) -> int:
    return n * 2

const LIMIT: int = 21

struct Point:
    x: int

impl Point:
    def norm(self) -> int:
        return abs(self.x)
```

<!-- output: examples/reference-declarations/items.out -->
```text
42
3
```

<!-- file: examples/reference-declarations/top_level_let.sisu -->
```sisu
# A binding is a statement, and statements belong in a function.
let greeting = "Hei"

def main():
    print(greeting)
```

<!-- output: examples/reference-declarations/top_level_let.console -->
```console
$ sisu run top_level_let.sisu
error[E0107]: statements cannot appear at the top level of a module; put them in a function such as `main`
  --> top_level_let.sisu:2:1
  |
2 | let greeting = "Hei"
  | ^^^

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** [02](../../proposal/02-syntax.md#closures) lets a
function hold a nested `def`, for a closure too long for a `do` block;
stage 0 reports one as E0900 (`stage0.unsupported`).

**Status.** In stage 0, except a nested `def`.

## Bindings

`let NAME = VALUE` binds a name that cannot be assigned again; `var
NAME = VALUE` binds one that can. A type annotation after the name,
`let limit: int = 10`, gives the type instead of inferring it
([R2](types.md#static-types-and-local-inference)).

- A tuple unpacks into as many names as it has elements, with or
  without parentheses: `let q, r = divide(17, 5)` and `let (low, high)
  = pair` (E0364 for a count that differs). The parentheses of a tuple
  may also be left out after `=` and `return`.
- The left of `=` may be any pattern that matches every value of its
  type; one that might not is E0329, and `let PATTERN = VALUE else:`
  handles the others ([R6](patterns.md#a-plain-let-matches-every-value)).
- An assignment to a `let` binding is E0312, whose fix declares it
  with `var` ([R17](diagnostics.md#e0312)).

<!-- file: examples/reference-declarations/bindings.sisu -->
```sisu
# `let` binds once, `var` may change, and a tuple unpacks into names.
def divide(a: int, b: int) -> (int, int):
    return a // b, a % b

def main():
    let name = "Aino"
    var visits = 0
    visits += 1
    let limit: int = 10
    let q, r = divide(17, 5)
    let (low, high) = (1, 9)
    let total = q + r
    let total = total * 2
    print("{name} {visits} {limit} {q} {r} {low} {high} {total}")
```

<!-- output: examples/reference-declarations/bindings.out -->
```text
Aino 1 10 3 2 1 9 10
```

<!-- file: examples/reference-declarations/immutable.sisu -->
```sisu
# A `let` binding cannot be assigned again.
def main():
    let visits = 0
    visits += 1
    print(visits)
```

<!-- output: examples/reference-declarations/immutable.console -->
```console
$ sisu run immutable.sisu
error[E0312]: `visits` is immutable; declare it with `var` to assign to it
  --> immutable.sisu:4:5
  |
4 |     visits += 1
  |     ^^^^^^
  |
3 |     let visits = 0
  |         ------ `visits` is defined here
  = fix: declare `visits` with `var`

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** Star unpacking, `let first, *rest = xs`, is
designed in [02](../../proposal/02-syntax.md#declarations); stage 0
reports it as E0900, whose message names the list pattern `[first,
*rest]`, which works in `match` and in `let ... else`
([R19](false-friends.md#first-rest--xs)).

**Status.** In stage 0, except star unpacking.

## A binding declared before its value

`let NAME: TYPE` without a value declares a binding that the compiler
checks is assigned exactly once on every path before it is used, as in
Swift and Kotlin. It is how a value comes out of an `if` or a `match`
whose branches need statements.

- The type is required (E0358).
- A use on a path where it has no value yet is E0366, and a second
  assignment to it E0367.

<!-- file: examples/reference-declarations/deferred.sisu -->
```sisu
# A `let` without a value is assigned once on every path before its use.
def describe(n: int) -> String:
    let label: str
    if n > 0:
        label = "positive"
    elif n < 0:
        label = "negative"
    else:
        label = "zero"
    return "{n} is {label}"

def main():
    print(describe(5))
    print(describe(0))
```

<!-- output: examples/reference-declarations/deferred.out -->
```text
5 is positive
0 is zero
```

**Status.** In stage 0.

## Rebinding and shadowing

A `let` or `var` may bind a name that the same block already binds; the
new binding hides the old one from then on, which is how a value is
narrowed step by step: `let total = total * 2`. A name bound again in an
inner block, hiding a binding of an enclosing one, is W0002, a warning
([R17](diagnostics.md#w0002)), since a reader of the inner block may
take it for the outer one (D-024).

<!-- file: examples/reference-declarations/shadowing.sisu -->
```sisu
# A name bound again in an inner block hides the outer one, and warns.
def main():
    let name = "Aino"
    for i in range(1):
        let name = "Eero"
        print("{i} {name}")
    print(name)
```

<!-- output: examples/reference-declarations/shadowing.console -->
```console
$ sisu run shadowing.sisu
warning[W0002]: `name` shadows a binding of an enclosing scope
  --> shadowing.sisu:5:13
  |
5 |         let name = "Eero"
  |             ^^^^

0 Eero
Aino
```

**Status.** In stage 0 (D-024).

## Constants

`const NAME: TYPE = VALUE` declares a constant, at the top level of a
module or in the body of a type, where it is read as `Type.NAME`.

- The type is required (E0219); the name is in `SCREAMING_CASE`
  ([R1](lexical-structure.md#naming-rules)).
- The value is computed at compile time, from literals, other
  constants, the operators and calls of `pure` functions, and of
  `comptime` functions inside `comptime`
  ([Compile-time and pure functions](#compile-time-and-pure-functions)).
- A constant is not a pattern: `case LIMIT:` is E0391, and the test is
  a guard, `case n if n == LIMIT:`.

<!-- file: examples/reference-declarations/constants.sisu -->
```sisu
# A constant has a type and a value known at compile time.
const SAUNA_DEGREES: int = 80
const GREETING: str = "Hei"
const LIMITS: (int, int) = (60, 2 * SAUNA_DEGREES)

struct Thermometer:
    reading: int
    const MAX: int = 120

def main():
    print("{GREETING} {SAUNA_DEGREES} {LIMITS} {Thermometer.MAX}")
```

<!-- output: examples/reference-declarations/constants.out -->
```text
Hei 80 (60, 160) 120
```

**Status.** In stage 0.

## Functions

`def NAME[TYPE PARAMETERS](PARAMETERS) -> RESULT:` and an indented
body declare a function. The type parameters and `-> RESULT` may be
left out.

- A function gives a value only with `return VALUE`; a block has no
  value of its own (D-056). A function with a result whose body can end
  without a `return` is E0315 ([R17](diagnostics.md#e0315)).
- A function without `-> RESULT` returns nothing, `()`: its `return`
  takes no value (E0316), and one with a result needs a value (E0317).
- `-> T ! E` is a result that may be an error from the set `E`, `-> T !`
  one whose error set is inferred, and `-> ! E` nothing or an error;
  each returns `Ok(...)` or `Err(...)`, so `-> ! E` ends with `return
  Ok(())` ([R2](types.md#results-and-error-sets)).
- A function's name is its own: there is no overloading by the types or
  the number of parameters (D-050), so a second `def` of one name is
  E0202. Default and keyword arguments do the work of overloads.
- `main` takes nothing or one `System` and returns nothing or `! E`
  (E0378); it is where a program starts.

<!-- file: examples/reference-declarations/functions.sisu -->
```sisu
# A function returns a value only with `return`.
error Closed:
    Shop

def area(width: int, height: int) -> int:
    return width * height

def greet(name: str):
    print("Hei, {name}!")

def open_at(hour: int) -> ! Closed:
    if hour < 8:
        return Err(Shop)
    print("open at {hour}")
    return Ok(())

def main():
    greet("Aino")
    print(area(3, 4))
    _ = open_at(9)
    print(open_at(6))
```

<!-- output: examples/reference-declarations/functions.out -->
```text
Hei, Aino!
12
open at 9
Err(Shop)
```

<!-- file: examples/reference-declarations/missing_return.sisu -->
```sisu
# Every path through a function with a result must return.
def sign(n: int) -> int:
    if n > 0:
        return 1
    elif n < 0:
        return -1

def main():
    print(sign(2))
```

<!-- output: examples/reference-declarations/missing_return.console -->
```console
$ sisu run missing_return.sisu
error[E0315]: `sign` returns `int`, but control can reach the end of its body without a `return`
  --> missing_return.sisu:2:5
  |
2 | def sign(n: int) -> int:
  |     ^^^^
  = note: return a value on that path too, or end the body with a `return`

error[D0006]: the program was not run: errors found (1)
```

<!-- file: examples/reference-declarations/overloading.sisu -->
```sisu
# Two functions cannot share a name, whatever their parameters.
def area(side: int) -> int:
    return side * side

def area(width: int, height: int) -> int:
    return width * height

def main():
    print(area(2))
```

<!-- output: examples/reference-declarations/overloading.console -->
```console
$ sisu run overloading.sisu
error[E0202]: `area` is defined more than once in this module
  --> overloading.sisu:5:5
  |
5 | def area(width: int, height: int) -> int:
  |     ^^^^
  |
2 | def area(side: int) -> int:
  |     ---- first defined here
  = note: rename one of them, or remove the one not wanted

error[D0006]: the program was not run: errors found (1)
```

A function that contains `yield` is a generator and returns `Iter[T]`
(E0436 otherwise); it ends when its body ends
([R5](statements.md#generators)).

**Status.** In stage 0 (D-050, D-056).

## Parameters

A parameter is `NAME: TYPE`, with the convention that says how it takes
its argument before it: none for a borrow, `mut` for a borrow that may
change the argument, `own` to take it ([R7](ownership.md#parameter-conventions)).

- `NAME: TYPE = DEFAULT` gives a default, used when a call leaves the
  argument out. The default is computed at each such call, not once
  where the function is declared, so a default list is a new list every
  time. A parameter without a default cannot follow one with a default
  (E0222).
- An argument is given by position or by keyword, `greet(word="Moi")`,
  and positional arguments come first (E0376). A keyword the function
  has no parameter for is E0306 ([R17](diagnostics.md#e0306)), an
  argument given twice E0307, and a parameter without a default and
  without an argument E0308 ([R17](diagnostics.md#e0308)).
- A bare `*` in the list makes the parameters after it keyword-only; a
  bare `/` makes those before it positional-only (PEP 3102, PEP 570).
- `*NAME: T` collects the remaining positional arguments into a
  `Slice[T]`. There is no `**kwargs`.
- A method's first parameter is `self`, `mut self` or `own self`
  ([Structs](#structs)); `self` after another parameter is E0218, and
  outside a type's body or impl E0215 ([R17](diagnostics.md#e0215)).

<!-- file: examples/reference-declarations/parameters.sisu -->
```sisu
# Defaults, keyword arguments, and `*` and `/` in a parameter list.
def greet(name: str, word: str = "Hei", *, mark: str = "!") -> String:
    return "{word}, {name}{mark}"

def clamp(value: int, /, low: int, high: int) -> int:
    return max(low, min(value, high))

def main():
    print(greet("Aino"))
    print(greet("Eero", word="Moi", mark="."))
    print(greet(word="Terve", name="Roni"))
    print("{clamp(15, low=0, high=10)} {clamp(-3, 0, 10)}")
```

<!-- output: examples/reference-declarations/parameters.out -->
```text
Hei, Aino!
Moi, Eero.
Terve, Roni!
10 0
```

<!-- file: examples/reference-declarations/defaults_per_call.sisu -->
```sisu
# A default is computed again at each call that leaves it out.
def add_item(item: str, own items: list[str] = []) -> list[str]:
    var kept = items
    kept.push(item)
    return kept

def main():
    print(add_item("coffee"))
    print(add_item("bun"))
    print(add_item("tea", ["milk"]))
```

<!-- output: examples/reference-declarations/defaults_per_call.out -->
```text
["coffee"]
["bun"]
["milk", "tea"]
```

**Stage 0 differs.** A variadic parameter, `*prices: int`, which
[02](../../proposal/02-syntax.md#declarations) gives and
[14](../../proposal/14-non-goals.md) rules out, is E0900, whose message
names that disagreement and a `Slice[T]` parameter as what takes a
list meanwhile ([R19](false-friends.md#args)).

**Status.** In stage 0, except variadic parameters.

## Compile-time and pure functions

`pure def` declares a function that is deterministic: it takes no
capability, changes nothing shared and observes nothing of the
schedule, so the same arguments give the same result on every run, and
it may run at compile time as well (D-074;
[R2](types.md#pure-function-types)). `comptime def` declares a function
that runs only at compile time, inside `comptime`, and may take and give
types (`T: type`); a call of one at run time is E0605. What compile-time
code may do is [R12](compile-time.md#what-compile-time-code-may-do).

<!-- file: examples/reference-declarations/compile_time.sisu -->
```sisu
# A `comptime def` runs at compile time only; a `pure def` at either.
comptime def square(n: int) -> int:
    return n * n

pure def cube(n: int) -> int:
    return n * n * n

const AREA: int = comptime square(5)
const VOLUME: int = comptime cube(3)

def main():
    print("{AREA} {VOLUME} {cube(4)}")
```

<!-- output: examples/reference-declarations/compile_time.out -->
```text
25 27 64
```

**Status.** In stage 0 (D-018, D-074).

## Structs

`struct NAME[TYPE PARAMETERS]:` and an indented body declare a struct.
The body holds, in any order:

- fields, `NAME: TYPE`, each with an optional default, `= VALUE`; a
  default is computed when the struct is built and cannot read another
  field (E0221);
- methods, `def` with `self`, `mut self` or `own self` first, called as
  `value.method()`; `mut self` may change the fields and `own self`
  consumes the value;
- associated functions, `def` without `self`, called as
  `Type.function()`, such as a named constructor;
- constants, `const NAME: TYPE = VALUE`, read as `Type.NAME`.

`Self` in the body is the struct's type. A struct is built by a call on
its type with an argument per field, by position in declaration order or
by keyword, where a field with a default may be left out (E0337 for one
without, [R17](diagnostics.md#e0337)). Two fields of one name are E0205,
two methods E0217; a method called on the type is E0361 and an
associated function called on a value E0362.

<!-- file: examples/reference-declarations/structs.sisu -->
```sisu
# A struct's fields, methods and associated functions.
struct Point:
    x: int
    y: int = 0
    label: String = "p"

    def origin() -> Point:
        return Point(0, 0)

    def moved(self, dx: int) -> Self:
        return Point(self.x + dx, self.y, self.label.copy())

    def shift(mut self, dx: int):
        self.x += dx

    def into_label(own self) -> String:
        return self.label

def main():
    var p = Point(1, label="a")
    p.shift(2)
    let q = Point(x=5)
    print("{p.x} {p.y} {p.label} {q:?} {Point.origin():?}")
    print("{p.moved(1).x} {q.into_label()}")
```

<!-- output: examples/reference-declarations/structs.out -->
```text
3 0 a Point(x=5, y=0, label="p") Point(x=0, y=0, label="p")
4 p
```

`impl Type:` adds methods and associated functions to a type outside
its body ([Impls](#impls)).

**Status.** In stage 0.

## View structs

`view struct NAME:` declares a struct whose fields may be references
and views, such as `str`; a value of it is a view and cannot outlive
what it borrows. A plain `struct` with such a field is E0411, whose fix
writes `view` ([R17](diagnostics.md#e0411)). The rules of views are
[R7](ownership.md#views), with an example.

**Status.** In stage 0 (D-004).

## Enums

`enum NAME[TYPE PARAMETERS]:` and an indented body declare an enum. Each
line of the body is a variant, a unit (`Dot`), a tuple (`Circle(f64)`)
or a record (`Rect(w: f64, h: f64)`), and the body may hold methods,
associated functions and constants as a struct's does. Two variants of
one name are E0206.

A variant is built like a struct, `Rect(w=2.0, h=3.0)`, named either
alone where the enum is the type expected or under its enum,
`Shape.Circle(1.0)`; a variant with fields named without them is E0386.
An enum may name itself in its variants
([R2](types.md#structs-and-enums)).

<!-- file: examples/reference-declarations/enums.sisu -->
```sisu
# An enum's variants, and a constant and a method in its body.
enum Shape:
    Dot
    Circle(f64)
    Rect(w: f64, h: f64)

    const KINDS: int = 3

    def area(self) -> f64:
        return match self:
            case Dot: 0.0
            case Circle(r): 3.0 * r * r
            case Rect(w, h): w * h

def main():
    let shapes = [Dot, Shape.Circle(1.0), Rect(w=2.0, h=3.0)]
    for shape in shapes:
        print("{shape:?} {shape.area()}")
    print("{Shape.KINDS} kinds")
```

<!-- output: examples/reference-declarations/enums.out -->
```text
Dot 0.0
Circle(1.0) 3.0
Rect(w=2.0, h=3.0) 6.0
3 kinds
```

A variant may give its discriminant, `Red = 1`, the number that
foreign code sees, which [03](../../proposal/03-types-and-generics.md#algebraic-data-types)
designs with `@repr` enums.

**Stage 0 differs.** An explicit discriminant is E0900 until the
layouts of `@repr` enums arrive ([R2](types.md#structs-and-enums)).

**Status.** In stage 0 (D-043), with the differences noted.

## Error sets

`error NAME:` and an indented body of variants declare an error set: an
enum whose variants are errors, which a function's result names after
`!` ([R2](types.md#results-and-error-sets)). A line `..` at the end of
the body makes the set open, so that other packages may add variants to
it; a `match` on an open set needs a `case _` arm (E0356). Errors, their
propagation and their traces are [R8](errors.md).

<!-- file: examples/reference-declarations/error_sets.sisu -->
```sisu
# An error set; `..` leaves it open to other packages' variants.
error LoadError:
    Missing(name: String)
    Corrupt
    ..

def load(name: str) -> String ! LoadError:
    if name == "":
        return Err(Corrupt)
    return Err(Missing(name.to_string()))

def main():
    match load("menu.txt"):
        case Ok(text): print(text)
        case Err(Missing(name)): print("missing {name}")
        case Err(_): print("something else")
```

<!-- output: examples/reference-declarations/error_sets.out -->
```text
missing menu.txt
```

**Status.** In stage 0.

## Traits

`trait NAME[TYPE PARAMETERS](SUPERTRAITS):` and an indented body declare
a trait. The body holds:

- required methods, a `def` line without a body, which every impl
  defines;
- provided methods, a `def` with a body, which an impl may replace;
- associated types, `type NAME`, with an optional default, `= TYPE`,
  which every impl binds;
- associated constants, `const NAME: TYPE`, which every impl gives, or
  `const NAME: TYPE = VALUE`, whose value an impl may replace.

The supertraits in parentheses, `trait Container(Named):`, are traits
that every implementer must implement too; a trait that is its own
supertrait, directly or through others, is E0247. In the body, `Self` is
the implementing type and an associated type is named by itself
(`Item`). A name after `impl` or in a bound that is not a trait is E0230.

<!-- file: examples/reference-declarations/traits.sisu -->
```sisu
# A trait with a supertrait, an associated type, and a provided method.
trait Named:
    def name(self) -> String

trait Container(Named):
    type Item
    def items(self) -> list[Item]

    def describe(self) -> String:
        return "{self.name()} with {len(self.items())} items"

struct Basket:
    fruit: list[String]

impl Named for Basket:
    def name(self) -> String:
        return "a basket"

impl Container for Basket:
    type Item = String

    def items(self) -> list[String]:
        return self.fruit.copy()

impl[T: Display] Named for list[T]:
    def name(self) -> String:
        return "a list of {len(self)}"

def main():
    print(Basket(["omena", "päärynä"]).describe())
    print([1, 2].name())
```

<!-- output: examples/reference-declarations/traits.out -->
```text
a basket with 2 items
a list of 2
```

An associated constant is read through a type: `Self.NAME` in the
trait's and its impls' methods and in a default's value, `T.NAME` for a
type parameter bounded by the trait, and `Square.NAME` for a type that
implements it, whose impl's value it is, or the trait's default. Read
through the trait, `Shape.SIDES`, it is E0449, since each impl gives its
own. A value is checked as the body of a `pure` function, and a trait
with a constant is no `dyn` type (E0241):

<!-- file: examples/reference-declarations/trait_consts.sisu -->
```sisu
# A trait's constants: one that every impl gives, one with a default.
trait Shape:
    const SIDES: int
    const CORNERS: int = Self.SIDES

    def describe(self) -> String:
        return "sides {Self.SIDES}, corners {Self.CORNERS}"

struct Square:
    side: f64

struct Disc:
    radius: f64

impl Shape for Square:
    const SIDES: int = 4

impl Shape for Disc:
    const SIDES: int = 1
    const CORNERS: int = 0

def sides[T: Shape](shape: T) -> int:
    return T.SIDES

def main():
    print(Square(2.0).describe())
    print(Disc(1.0).describe())
    print("{Square.CORNERS} {sides(Disc(1.0))}")
```

<!-- output: examples/reference-declarations/trait_consts.out -->
```text
sides 4, corners 4
sides 1, corners 0
4 1
```

A constant of a type, given by an impl or by the type's own body, is
known when the program compiles, so it can be a type's constant
argument, `Array[f64, Square.SIDES]`, wherever a type is written. A
trait's constant read through a type parameter cannot: `T.SIDES` is a
value of each instance, known only where the function runs. A function
whose type needs it takes a `const` parameter instead, which a caller
gives as `Square.SIDES`:

<!-- file: examples/reference-declarations/const_argument.sisu -->
```sisu
# A type's constant as an array's length, and a `const` parameter that
# carries one into a generic function.
trait Shape:
    const SIDES: int

struct Square:
    side: f64

impl Shape for Square:
    const SIDES: int = 4

def corners[T: Shape, const N: int](shape: T) -> Array[int, N]:
    return Array.filled(T.SIDES)

def main():
    let lengths: Array[f64, Square.SIDES] = Array.filled(2.0)
    print(lengths)
    print(corners[Square, Square.SIDES](Square(2.0)))
```

<!-- output: examples/reference-declarations/const_argument.out -->
```text
[2.0, 2.0, 2.0, 2.0]
[4, 4, 4, 4]
```

Written through the type parameter, the length is E0274
(`resolve.const-not-type`), which calls `SIDES` a constant of `Shape`
and whose note names the `const` parameter:

<!-- file: examples/reference-declarations/const_through_param.sisu -->
```sisu
# A trait's constant through a type parameter, as an array's length.
trait Shape:
    const SIDES: int

def corners[T: Shape](shape: T) -> Array[int, T.SIDES]:
    return Array.filled(0)
```

<!-- output: examples/reference-declarations/const_through_param.console -->
```console
$ sisu check const_through_param.sisu
error[E0274]: `SIDES` is a constant of `Shape`, which bounds `T`, not a type
  --> const_through_param.sisu:5:47
  |
5 | def corners[T: Shape](shape: T) -> Array[int, T.SIDES]:
  |                                               ^^^^^^^
  = note: `SIDES` is a trait's constant, which each impl gives: through a type parameter it is a value of the instance, known where the function runs, and a type's constant argument is known where the type is written; take it as a `const` parameter, `[T: Shape, const N: int]`, which a caller gives as `Square.SIDES`

```

**Status.** In stage 0 (03).

## Impls

`impl TRAIT for TYPE:` implements a trait for a type, `impl[T: BOUND]
TRAIT for TYPE[T]:` for every type that a pattern matches, and `impl
TYPE:` adds methods to a type without a trait.

- A trait's impl defines every required method with the signature the
  trait gives it: one missing is E0232, a method the trait does not have
  E0233 ([R17](diagnostics.md#e0232)), and one whose signature differs
  E0234. It binds every associated type without a default, `type Item =
  String` (E0237); a name the trait does not have is E0238, and an
  associated type outside a trait's impl E0248.
- It gives each constant that the trait declares without a value
  (E0232), as a constant with the trait's type (E0272, E0273), and no
  constant the trait does not declare (E0271).
- An impl of a trait needs the type to implement the trait's
  supertraits (E0242, [R17](diagnostics.md#e0242)).
- A method of an impl inherits `pure` from the trait's declaration of
  it.
- A type implements a trait at most once, and the impl stands in the
  package of the trait or of the type, or in the root package
  ([R2](types.md#one-impl-for-each-type-and-trait)).
- `impl TYPE:` adds methods to any type, one of the standard library or
  of another package included. The methods are visible in the module
  that declares them and wherever that module is imported, and a name
  the type already has is E0217 (Q-12).
- `impl Drop for TYPE:` is for a struct or enum of the same module, with
  exactly `def drop(mut self)` (E0213, E0216;
  [R7](ownership.md#drops-and-their-order)).

<!-- file: examples/reference-declarations/impl_missing.sisu -->
```sisu
# An impl defines every required method and nothing else.
trait Priced:
    def price(self) -> int
    def discount(self) -> int

struct Coffee:
    cups: int

impl Priced for Coffee:
    def price(self) -> int:
        return 350 * self.cups

    def tip(self) -> int:
        return 50

def main():
    print(Coffee(1).price())
```

<!-- output: examples/reference-declarations/impl_missing.console -->
```console
$ sisu run impl_missing.sisu
error[E0232]: the impl of `Priced` for `Coffee` is missing `discount`
  --> impl_missing.sisu:9:1
  |
9 | impl Priced for Coffee:
  | ^^^^^^^^^^^^^^^^^^^^^^
  = note: write `def discount(self) -> int` in the impl

error[E0233]: `tip` is not a method of `Priced`
  --> impl_missing.sisu:13:9
   |
13 |     def tip(self) -> int:
   |         ^^^

error[D0006]: the program was not run: errors found (2)
```

<!-- file: examples/reference-declarations/supertrait_missing.sisu -->
```sisu
# Implementing a trait needs its supertraits too.
trait Named:
    def name(self) -> String

trait Container(Named):
    def count(self) -> int

struct Basket:
    n: int

impl Container for Basket:
    def count(self) -> int:
        return self.n

def main():
    print(Basket(1).count())
```

<!-- output: examples/reference-declarations/supertrait_missing.console -->
```console
$ sisu run supertrait_missing.sisu
error[E0242]: `Basket` implements `Container`, which requires `Named`, but `Basket` does not implement `Named`
  --> supertrait_missing.sisu:11:1
   |
11 | impl Container for Basket:
   | ^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0 (D-013, Q-12).

## Type declarations

`type NAME = TYPE` declares an alias, another name for the same type,
and `type NAME = TYPE where PREDICATE` a refinement type, the base type
with a predicate on `self`. Both may take type parameters. Their meaning
is [R2](types.md#type-aliases-and-newtypes) and
[R2](types.md#refinement-types); an alias that refers to itself is
E0228.

**Status.** In stage 0.

## Tests, suites and benchmarks

`test "NAME":` and an indented body declare a test, which `sisu test`
runs and `sisu run` leaves out. A test may stand beside the code it
tests, at the top level of any module.

- `suite "NAME":` groups tests one level deep; a test's full name is
  `SUITE/NAME`.
- `test "NAME" for A, B in CASES:` is parametrised: one test for each
  element of `CASES`, reported with the element.
- `test "NAME"(sys: System):` takes the real capabilities, and `test
  "NAME"(fs: Fs, clock: Clock):` fresh fakes of the ones it names
  (E0702 for another signature).
- `bench "NAME"(b: Bench):` declares a benchmark, which `sisu bench`
  runs.
- A test returns nothing (E0381) and has no contract (E0807).

<!-- file: examples/reference-declarations/tests.sisu -->
```sisu
# Tests, a suite and a parametrised test beside the code they test.
def double(n: int) -> int:
    return n * 2

test "double doubles":
    assert double(2) == 4

suite "double":
    test "of zero":
        assert double(0) == 0

    test "of each case" for n, want in [(1, 2), (-3, -6)]:
        assert double(n) == want
```

<!-- output: examples/reference-declarations/tests.console -->
```console
$ sisu test tests.sisu
test "double doubles" ... ok
test "double/of zero" ... ok
test "double/of each case (1, 2)" ... ok
test "double/of each case (-3, -6)" ... ok
4 passed, 0 failed, 0 blocked
```

What a test may do, its outcomes and its tools are [R15](testing.md).

**Status.** In stage 0.

## Imports

`import PATH` binds the last name of a module's path, so `import
std.math` binds `math`; `import PATH as NAME` binds another name. `from
PATH import NAME, NAME as OTHER` binds items of the module. A path
names a module of the standard library (`std.NAME`), of the program's
own package by the package's name, or of a dependency by its name; one
that names nothing is E0220. There are no glob imports and no cycles
between packages. The rules are [R13](modules.md#imports).

<!-- file: examples/reference-declarations/imports.sisu -->
```sisu
# `import` binds a module, `from ... import` binds names from it.
import std.math
from std.num import gcd, lcm as least_multiple

def main():
    print("{math.sqrt(16.0)} {gcd(12, 18)} {least_multiple(4, 6)}")
```

<!-- output: examples/reference-declarations/imports.out -->
```text
4.0 6 12
```

**Status.** In stage 0.

## Foreign functions

`extern "C":` and an indented body of `def` lines without bodies
declare functions of C, whose signatures are a claim the compiler cannot
check; each call of one stands in an `unsafe` block or expression
(E0982), and a block without a `# safety:` comment above it is W0007.
`import c "HEADER" as NAME` will read the declarations from a C header.
A dependency may contain `extern` only when the root package trusts it
(E0703). The rules of unsafe code are
[R7](ownership.md#unsafe-code-and-raw-pointers).

**Stage 0 differs.** `extern` blocks are arriving: the checker takes
them, the interpreter emulates a few C functions (`strlen`, `memcpy`,
`malloc`, `free`, `getenv`, `write`) and panics at any other, and `sisu
run --native` calls them. A C header import is E0900.

**Status.** Designed ([12](../../proposal/12-interop-and-targets.md),
[22](../../proposal/22-unsafe.md); D-080, D-082), and arriving.

## Visibility

An item is private to its module unless it is declared `pub`, which
exports it to every module that imports it, or `pub(package)`, which
exports it to the modules of its own package only. A field and a
method are private to the module of their type in the same way. A use
of an item from where it is not visible is E0255, E0256 (for
`pub(package)`), E0258 for a field and E0259 for a method; a name that a
module imported is visible through it only when the import is `pub`
(E0257). Visibility between the modules of a package is
[R13](modules.md#visibility).

**Status.** In stage 0 (D-045).

## Attributes and doc comments

An attribute, `@NAME` or `@NAME(ARGUMENTS)`, stands on the line before
an item or a field and is compile-time data, never a call (D-046):
`@derive(Debug, Eq)`, `@must_consume`, `@cfg(test)`. Every attribute
and where it may stand is [R14](attributes.md).

A `##` comment on the lines before an item or a field documents it, and
`##!` documents the module; `sisu doc` renders them and `sisu test` runs
their examples ([R1](lexical-structure.md#comments-and-doc-comments)).

**Status.** In stage 0 (D-046).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0107](diagnostics.md#e0107), [E0202](diagnostics.md#e0202) | `parse.top-level-statement`, `resolve.duplicate-definition` | [Items and statements](#items-and-statements) |
| E0364, [E0329](diagnostics.md#e0329), [E0312](diagnostics.md#e0312) | `type.unpack-count`, `type.refutable-let`, `type.assign-immutable` | [Bindings](#bindings) |
| E0358, E0366, E0367 | `type.let-needs-type`, `type.uninitialized`, `type.reassign-let` | [A binding declared before its value](#a-binding-declared-before-its-value) |
| [W0002](diagnostics.md#w0002) | `resolve.shadow-outer` | [Rebinding and shadowing](#rebinding-and-shadowing) |
| E0219, E0391 | `resolve.const-type-required`, `type.const-pattern` | [Constants](#constants) |
| [E0315](diagnostics.md#e0315), E0316, E0317, E0378, E0436 | `type.missing-return`, `type.return-value-in-unit`, `type.return-needs-value`, `type.main-signature`, `type.generator-result` | [Functions](#functions) |
| E0222, E0376, [E0306](diagnostics.md#e0306), E0307, [E0308](diagnostics.md#e0308), E0218, [E0215](diagnostics.md#e0215) | `resolve.parameter-default-order`, `type.positional-after-keyword`, `type.unknown-keyword`, `type.duplicate-argument`, `type.missing-argument`, `resolve.self-not-first`, `resolve.self-outside-method` | [Parameters](#parameters) |
| [E0605](diagnostics.md#e0605) | `comptime.only` | [Compile-time and pure functions](#compile-time-and-pure-functions) |
| E0221, [E0337](diagnostics.md#e0337), E0205, [E0217](diagnostics.md#e0217), E0361, E0362 | `resolve.field-default-order`, `type.missing-field`, `resolve.duplicate-field`, `resolve.duplicate-method`, `type.method-needs-receiver`, `type.assoc-not-method` | [Structs](#structs) |
| [E0411](diagnostics.md#e0411) | `borrow.view-field-in-plain-struct` | [View structs](#view-structs) |
| E0206, E0386 | `resolve.duplicate-variant`, `type.variant-needs-fields` | [Enums](#enums) |
| E0356 | `type.open-set-needs-wildcard` | [Error sets](#error-sets) |
| E0230, E0247, [E0449](diagnostics.md#e0449), [E0274](diagnostics.md#e0274) | `resolve.not-a-trait`, `resolve.trait-cycle`, `type.trait-constant`, `resolve.const-not-type` | [Traits](#traits) |
| [E0232](diagnostics.md#e0232), [E0233](diagnostics.md#e0233), E0234, [E0237](diagnostics.md#e0237), E0238, E0248, [E0271](diagnostics.md#e0271), [E0272](diagnostics.md#e0272), [E0273](diagnostics.md#e0273), [E0242](diagnostics.md#e0242), E0213, E0216 | `resolve.missing-trait-method`, `resolve.not-a-trait-method`, `resolve.method-signature`, `resolve.missing-assoc-type`, `resolve.unknown-assoc-type`, `resolve.assoc-outside-trait-impl`, `resolve.not-a-trait-constant`, `resolve.constant-type`, `resolve.const-or-method`, `resolve.supertrait-missing`, `resolve.drop-target`, `resolve.drop-signature` | [Impls](#impls) |
| E0228 | `resolve.alias-cycle` | [Type declarations](#type-declarations) |
| E0702, E0381, [E0807](diagnostics.md#e0807) | `type.test-signature`, `type.return-in-test`, `contract.not-in-test` | [Tests, suites and benchmarks](#tests-suites-and-benchmarks) |
| [E0220](diagnostics.md#e0220) | `resolve.import-unknown` | [Imports](#imports) |
| E0982, [E0703](diagnostics.md#e0703), [W0007](diagnostics.md#w0007) | `unsafe.foreign-call`, `cap.untrusted`, `unsafe.no-safety-comment` | [Foreign functions](#foreign-functions) |
| [E0255](diagnostics.md#e0255), [E0256](diagnostics.md#e0256), E0257, [E0258](diagnostics.md#e0258), [E0259](diagnostics.md#e0259) | `resolve.private-item`, `resolve.package-item`, `resolve.not-reexported`, `resolve.private-field`, `resolve.private-method` | [Visibility](#visibility) |
