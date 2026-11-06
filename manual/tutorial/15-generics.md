# 15. Generics

In this chapter you write code once for many types. A generic function
or type takes types as parameters, and a bound on a parameter names the
traits that its types must have. The compiler checks the bounds where
you write them, and makes the code for each type it is used with. When
the type is known only while the program runs, a `dyn` value carries it
along. Last come types that take numbers as parameters, as `Array`
does. You need about two hours.

## A function for every type with an ability

A function that counts how often a value appears in a list does the
same work for numbers, words and anything else that `==` can compare.
A generic function says so once:

<!-- file: examples/15-generics/matching.sisu -->
```sisu
# A generic function works for every type that has its bound.
def count_matching[T: Eq](items: list[T], wanted: T) -> int:
    var count = 0
    for item in items:
        if item == wanted:
            count += 1
    return count

def main():
    print(count_matching([3, 1, 3, 3], 3))
    print(count_matching(["kyllä", "ei", "kyllä"], "ei"))
    print(count_matching[bool]([True, False, True], False))
```

<!-- output: examples/15-generics/matching.out -->
```text
3
1
1
```

- `[T: Eq]` after the name declares a type parameter, `T`, with a
  bound: `T` stands for any type that implements `Eq`. The parameters
  then use `T` as a type: a list of `T` and one `T`.
- A call works out `T` from its arguments, `int` in the first call and
  `str` in the second. `count_matching[bool](...)` names it, which is
  rarely needed.
- `item == wanted` compiles because the bound promises `Eq`.

The bound is the whole of what the body may assume about a `T`: it may
compare two of them, and nothing else.

> **From Python.** A Python function takes values of any type, and a
> value that lacks what the body does with it fails when that line
> runs, perhaps months later. A generic function says in its signature
> what it needs from its types, and a call with any other type does not
> compile.

## Bounds are checked where they are written

**Try it.** Delete the bound `: Eq` from `count_matching`, and run the
program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/15-generics/no_bound.console -->
```console
$ sisu run no_bound.sisu
error[E0347]: `T` does not implement `Eq`
  --> no_bound.sisu:5:12
  |
5 |         if item == wanted:
  |            ^^^^^^^^^^^^^^
  = fix: bound `T` by `Eq`: `: Eq`

error[D0006]: the program was not run: errors found (1)
```

E0347 (`type.bound-not-satisfied`) points into the body of the
function, not at a call: without a bound, nothing says that a `T` can
be compared, whichever types the calls give. Its fix adds the bound.

</details>

The body is checked once, against its bounds, when it is written. A
call is checked against the bounds too, so a type that lacks one is
reported at the call:

<!-- file: examples/15-generics/bound_at_call.sisu -->
```sisu
# A call with a type that lacks the bound.
def count_matching[T: Eq](items: list[T], wanted: T) -> int:
    var count = 0
    for item in items:
        if item == wanted:
            count += 1
    return count

struct Guest:
    name: String

def main():
    let guests = [Guest("Aino"), Guest("Eero")]
    print(count_matching(guests, Guest("Aino")))
```

<!-- output: examples/15-generics/bound_at_call.console -->
```console
$ sisu run bound_at_call.sisu
error[E0347]: `Guest` does not implement `Eq`
  --> bound_at_call.sisu:14:11
   |
14 |     print(count_matching(guests, Guest("Aino")))
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: derive it: `@derive(Eq)`

error[D0006]: the program was not run: errors found (1)
```

`Guest` does not implement `Eq`, so this call cannot use
`count_matching`. The error is in the caller's code, where the fix is,
either `@derive(Eq)` on `Guest` or another function. A generic function
in a library therefore never fails inside the library because of a
type that a user gave it (D-017). A method that the bound does not
have gets E0304 (`type.unknown-method`) in the body, with a note that
names the traits that have it.

## Several bounds

A parameter can have several bounds, joined with `+`. When the bounds
make the signature long, a `where` clause after the result type holds
them instead:

<!-- file: examples/15-generics/tally.sisu -->
```sisu
# Bounds join with `+`, and a `where` clause can hold them instead.
def tally[T](items: list[T]) -> dict[T, int] where T: Hash + Copy:
    var counts: dict[T, int] = {}
    for item in items:
        counts.entry(item).or_insert(0) += 1
    return counts

def report[T: Display + Hash](counts: dict[T, int]) -> String:
    let parts = ["{item} x{n}" for item, n in counts.items()]
    return parts.join(", ")

def main():
    print(report(tally(["kyllä", "ei", "kyllä"])))
    print(report(tally([3, 1, 3, 3])))
```

<!-- output: examples/15-generics/tally.out -->
```text
kyllä x2, ei x1
3 x3, 1 x1
```

- `tally` needs `Hash`, to use each item as a dict's key, and `Copy`,
  to put a copy of an item that the loop borrows into the dict. The
  `where T: Hash + Copy` clause says so.
- `report` writes each item with `{item}`, which needs `Display`, and
  reads a dict whose keys are `T`, which needs `Hash`.

A `dict[T, int]` needs `Hash` of its key type wherever it appears, so a
function that only reads one names the bound too.

## Types with parameters

A struct or an enum can take type parameters, as `list[T]` and `T?`
do. Its fields and methods then use them:

<!-- file: examples/15-generics/labelled.sisu -->
```sisu
# A generic type takes the types of its fields as parameters.
struct Labelled[T]:
    label: String
    value: T

    def relabel(own self, label: str) -> Labelled[T]:
        return Labelled(label.to_string(), self.value)

impl[T: Display] Display for Labelled[T]:
    def fmt(self, mut f: Formatter):
        f.write("{self.label}: {self.value}")

def main():
    let age = Labelled("age", 34)
    let heat: Labelled[f64] = Labelled("sauna", 80.5)
    print(age)
    print(heat.relabel("löyly"))
    print(Labelled("guests", ["Aino", "Eero"]))
```

<!-- output: examples/15-generics/labelled.out -->
```text
age: 34
löyly: 80.5
guests: ["Aino", "Eero"]
```

- `struct Labelled[T]:` declares a type with a parameter. A
  `Labelled[int]` holds an `int` value and a `Labelled[f64]` an `f64`;
  `Labelled("age", 34)` works out `T` from its value, and
  `let heat: Labelled[f64]` names it.
- The methods in the body use `T` without declaring it again, and
  `relabel` returns a `Labelled[T]` of the same `T`.
- `impl[T: Display] Display for Labelled[T]:` is a generic impl: every
  `Labelled[T]` whose `T` has `Display` has `Display` too. The impl
  needs the bound, since `fmt` writes the value.

**Try it.** Add a struct `Guest` with a `name: String` field, and
print `Labelled("first", Guest("Aino"))`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/15-generics/labelled_guest.console -->
```console
$ sisu run labelled_guest.sisu
error[E0347]: `Labelled[Guest]` does not implement `Display`
  --> labelled_guest.sisu:22:5
   |
22 |     print(Labelled("first", Guest("Aino")))
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

A `Labelled[Guest]` is a type like any other, but the generic impl
gives `Display` only to a `Labelled[T]` whose `T` has `Display`, and
`Guest` does not.

</details>

A field of a generic struct can have a default, as any field can: with
`items: list[T] = []`, each `Shelf[int]()` starts with an empty list of
`int`s. An associated function that builds the empty value, as exercise
15.4 writes, does the same where the value needs more than defaults.

## Type parameters with defaults

A type parameter can have a default, written after it with `=`, and a
use that leaves the parameter out gets the default:

<!-- file: examples/15-generics/pair.sisu -->
```sisu
# A type parameter can have a default, which a use may leave out.
struct Pair[A, B = A]:
    first: A
    second: B

def total(pair: Pair[int]) -> int:
    return pair.first + pair.second

def main():
    let both: Pair[int] = Pair(2, 3)
    let mixed: Pair[str, int] = Pair("Aino", 34)
    print(total(both))
    print("{mixed.first} is {mixed.second}")
```

<!-- output: examples/15-generics/pair.out -->
```text
5
Aino is 34
```

`Pair[int]` is `Pair[int, int]`, since `B` defaults to `A`, and
`Pair[str, int]` gives both. A parameter with a default comes after the
ones without (E0245). You met one in chapter 14: the operator traits
are declared `Add[Rhs = Self]`, so `impl Add for Vector` adds two
vectors and `Mul[f64]` names another type for the right operand. The
standard collections are to take their allocator this way,
`list[T, A: Allocator = Global]`, so that ordinary code never names it
(D-060); that parameter is designed in
[03](../../proposal/03-types-and-generics.md#generic-syntax) and is not
in the compiler yet.

## Values that carry their type

A generic function takes a list of one type: all coffees, or all buns.
A basket holds both. `dyn Priced` is a value of some type that
implements `Priced`, which carries its type with it, so that a call of
`price` runs the impl of whatever the value is:

<!-- file: examples/15-generics/basket.sisu -->
```sisu
# A `dyn` value carries its type, and calls that type's impl.
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

def total(basket: list[Box[dyn Priced]]) -> int:
    var sum_so_far = 0
    for item in basket:
        sum_so_far += item.price()
    return sum_so_far

def show(item: ref dyn Priced) -> String:
    return "{item.price()} cents"

def main():
    let basket: list[Box[dyn Priced]] = [Box(Coffee(2)), Box(Bun("apple"))]
    print(total(basket))
    let bun = Bun("cardamom")
    print(show(ref bun))
```

<!-- output: examples/15-generics/basket.out -->
```text
980
280 cents
```

- `Box(Coffee(2))` becomes a `Box[dyn Priced]` where that type is
  expected, and so does a `Box` of a bun, so one list holds both.
- `item.price()` asks each item for its own impl's `price`, while the
  program runs.
- `show` takes a `ref dyn Priced`, a borrow of any value with the
  ability, and the call lends it with `ref bun`.

A `dyn` value's size depends on what it holds, so it always lives
behind something of a known size: a `Box`, which owns it, or a `ref`,
which borrows it.

**Try it.** Change the parameter of `show` to `item: dyn Priced`, with
no `ref`, and the call to `show(Bun("cardamom"))`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/15-generics/bare_dyn.console -->
```console
$ sisu run bare_dyn.sisu
error[E0395]: `dyn Priced` has no size of its own; hold it behind `ref` or in a `Box`
  --> bare_dyn.sisu:12:10
   |
12 | def show(item: dyn Priced) -> String:
   |          ^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0395 (`type.unsized-dyn`) asks for a `ref` or a `Box`, which chapter
14's E0231 writes in its fix, `ref dyn Priced`, for a parameter.

</details>

## What generics and dyn cost

The two ways to write "any type with the ability" are compiled
differently:

<!-- file: examples/15-generics/two_totals.sisu -->
```sisu
# The same total, generic and through `dyn`.
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

def total_of[T: Priced](items: list[T]) -> int:
    var sum_so_far = 0
    for item in items:
        sum_so_far += item.price()
    return sum_so_far

def total_dyn(items: list[Box[dyn Priced]]) -> int:
    var sum_so_far = 0
    for item in items:
        sum_so_far += item.price()
    return sum_so_far

def main():
    print(total_of([Coffee(1), Coffee(3)]))
    print(total_of([Bun("apple"), Bun("plum")]))
    print(total_dyn([Box(Coffee(1)), Box(Bun("apple"))]))
```

<!-- output: examples/15-generics/two_totals.out -->
```text
1400
560
630
```

- `total_of` is generic. The native compiler makes one copy of it for
  each type it is called with, here one for `Coffee` and one for `Bun`,
  and in each copy `item.price()` is a direct call to that type's
  method. This is called monomorphisation. The cost is compile time
  and program size: a generic function used with twenty types is
  twenty functions in the executable.
- `total_dyn` is compiled once. Each `item.price()` looks up the
  method in the value's type while the program runs, which costs a
  little on every call, and each item is a `Box` of its own.

Prefer generics: they cost nothing while the program runs, and the
compiler knows every type. Choose `dyn` when one collection holds
values of several types, or when a function used with very many types
makes the program too large (D-017). The compiler is to share the
copies whose types have the same layout, and to report the copies of
each package with `sisu build --explain-size`; both are designed in
[03](../../proposal/03-types-and-generics.md#dispatch) and are not in
the compiler yet, whose `--explain-size` reports the parts of a native
executable, the program's own code among them
([chapter 27](27-systems.md#what-is-still-to-come)). Stage 0's
interpreter runs one copy of every function and passes the types
along.

## Traits that cannot be dyn

A `dyn` value can call only the methods that make sense without knowing
its type. A method without `self`, such as one that makes a new value,
has no value to ask:

<!-- file: examples/15-generics/make.sisu -->
```sisu
# A trait with a method that a `dyn` value cannot call.
trait Shape:
    def unit() -> Self
    def area(self) -> f64

struct Square:
    side: f64

impl Shape for Square:
    def unit() -> Square:
        return Square(1.0)

    def area(self) -> f64:
        return self.side * self.side

def total(shapes: list[Box[dyn Shape]]) -> f64:
    var sum_so_far = 0.0
    for shape in shapes:
        sum_so_far += shape.area()
    return sum_so_far

def main():
    print(total([Box(Square(2.0)), Box(Square.unit())]))
```

<!-- output: examples/15-generics/make.console -->
```console
$ sisu run make.sisu
error[E0241]: `Shape` cannot be used as `dyn Shape`: its method `unit` cannot be called through it, because it does not take `self` or `mut self`
  --> make.sisu:16:28
   |
16 | def total(shapes: list[Box[dyn Shape]]) -> f64:
   |                            ^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0241 (`resolve.not-object-safe`) names the method and the reason.
`Shape.unit()` would need to know which type to make, and a
`dyn Shape` cannot say. A trait can be a `dyn` when each method takes
`self` or `mut self`, has no type parameters of its own, and mentions
`Self` only as its receiver.

## An enum or a trait

The basket can also be an enum, with a variant for each product:

<!-- file: examples/15-generics/products.sisu -->
```sisu
# A closed set of kinds is an enum, and a `match` covers every kind.
enum Product:
    Coffee(cups: int)
    Bun(filling: String)

def price(product: Product) -> int:
    return match product:
        case Coffee(cups): 350 * cups
        case Bun(_): 280

def main():
    let basket: list[Product] = [Coffee(2), Bun("apple")]
    var total = 0
    for product in basket:
        total += price(product)
    print(total)
```

<!-- output: examples/15-generics/products.out -->
```text
980
```

Both work, and the choice depends on who adds what:

- An enum is a closed set. Its variants are all in one place, a `match`
  must cover each, and adding a variant shows every `match` that needs
  a new arm. Adding an operation is one more function. Choose an enum
  when you know every kind, as the café knows its menu.
- A trait is an open set. Anyone can add a type with an impl, even in
  another package, without changing your code, but each new operation
  changes the trait and every impl. Choose a trait, generic or `dyn`,
  when others add kinds: shapes in a drawing library, or the products
  of every café that uses it.

## Numbers as type parameters

A type parameter can be a constant: `const N: int` takes a number, as
the `N` of `Array[T, N]` does. The number is part of the type, and an
expression of the parameters can be an argument:

<!-- file: examples/15-generics/grid.sisu -->
```sisu
# A type can take a number as a parameter, as `Array` does.
struct Grid[T, const W: int, const H: int]:
    cells: Array[T, W * H]

    def get(self, x: int, y: int) -> T where T: Copy:
        return self.cells[y * W + x]

    def set(mut self, x: int, y: int, own value: T):
        self.cells[y * W + x] = value

def show(board: Grid[char, 3, 2]):
    for y in range(2):
        var line: String = ""
        for x in range(3):
            line.push("{board.get(x, y)}")
        print(line)

def main():
    var board: Grid[char, 3, 2] = Grid(Array.filled('.'))
    board.set(1, 0, 'x')
    board.set(2, 1, 'o')
    show(board)
    print("{len(board.cells)} cells")
```

<!-- output: examples/15-generics/grid.out -->
```text
.x.
..o
6 cells
```

- `Grid[T, const W: int, const H: int]` takes a type and two numbers,
  and its field `cells` is an `Array[T, W * H]`: six cells for a grid
  three wide and two high. The size is computed when the type is
  written.
- In the methods, `W` is an `int` value, so `y * W + x` finds a cell.
- `Array.filled('.')` takes its size from the type it is expected to
  have, and fills every cell with a dot.
- `get` returns a copy of a cell, so it asks for `T: Copy` in a `where`
  clause of its own: only a grid of a type that is `Copy` has `get`.

**Try it.** Make the board in `main` three by three, and leave `show`
as it is.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/15-generics/grid_size.console -->
```console
$ sisu run grid_size.sisu
error[E0300]: expected `Grid[char, 3, 2]`, found `Grid[char, 3, 3]`
  --> grid_size.sisu:22:10
   |
22 |     show(board)
   |          ^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0300 (`type.mismatch`) says that a grid of three by three is not one
of three by two. The size is checked before the program runs, so a
function that takes a `Grid[char, 3, 2]` never sees a grid of another
size.

</details>

## Exercise 15.1: predict the generic calls

Open
[`manual/exercises/15-generics/predict_generics.sisu`](../exercises/15-generics/predict_generics.sisu):

<!-- file: exercises/15-generics/predict_generics.sisu -->
```sisu
##! Exercise 15.1, predict the generic calls. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_generics.sisu`. The hints are in chapter 15 of the
##! tutorial, under "Exercise 15.1: predict the generic calls".

trait Animal:
    def sound(self) -> String

    def call(self) -> String:
        return "{self.sound()} {self.sound()}"

struct Cat:
    name: String

struct Cow:
    name: String

impl Animal for Cat:
    def sound(self) -> String:
        return "miau"

impl Animal for Cow:
    def sound(self) -> String:
        return "ammuu"

    def call(self) -> String:
        return "{self.name} says {self.sound()}"

def chorus[T: Animal](animals: list[T]) -> String:
    let calls = [animal.call() for animal in animals]
    return calls.join(" / ")

def sounds(animals: list[Box[dyn Animal]]) -> String:
    let each = [animal.sound() for animal in animals]
    return each.join(" ")

struct Shelf[T, const N: int]:
    slots: Array[T, N]

const CATS: str = todo("two cats in a chorus")
const COWS: str = todo("one cow in a chorus")
const BARN: str = todo("the sounds of a cat and a cow")
const SLOTS: int = todo("the slots of a shelf")

test "a chorus of cats":
    assert chorus([Cat("Mirri"), Cat("Musti")]) == CATS

test "a chorus of cows":
    assert chorus([Cow("Kukka")]) == COWS

test "a barn":
    let barn: list[Box[dyn Animal]] = [Box(Cat("Mirri")), Box(Cow("Kukka"))]
    assert sounds(barn) == BARN

test "a shelf":
    let shelf: Shelf[int, 2 * 3] = Shelf(Array.filled(0))
    assert len(shelf.slots) == SLOTS
```

<!-- output: solutions/15-generics/predict_generics.console -->
```console
$ sisu test predict_generics.sisu
warning[W0001]: typed hole `two cats in a chorus` of type `str`
  --> predict_generics.sisu:42:19
   |
42 | const CATS: str = todo("two cats in a chorus")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `one cow in a chorus` of type `str`
  --> predict_generics.sisu:43:19
   |
43 | const COWS: str = todo("one cow in a chorus")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the sounds of a cat and a cow` of type `str`
  --> predict_generics.sisu:44:19
   |
44 | const BARN: str = todo("the sounds of a cat and a cow")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the slots of a shelf` of type `int`
  --> predict_generics.sisu:45:20
   |
45 | const SLOTS: int = todo("the slots of a shelf")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "a chorus of cats" ... blocked by todo("two cats in a chorus")
test "a chorus of cows" ... blocked by todo("one cow in a chorus")
test "a barn" ... blocked by todo("the sounds of a cat and a cow")
test "a shelf" ... blocked by todo("the slots of a shelf")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

A generic call runs the impl of the type it is called with, and so does
a call through `dyn`. A type's impl may replace a provided method.

</details>

<details>
<summary>Hint 2</summary>

`join` puts its text between the items, not after each. A shelf's
second parameter is a number, computed where the type is written.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/15-generics/predict_generics.sisu`](../solutions/15-generics/predict_generics.sisu).

## Exercise 15.2: the most common item

Open
[`manual/exercises/15-generics/most_common.sisu`](../exercises/15-generics/most_common.sisu):

<!-- file: exercises/15-generics/most_common.sisu -->
```sisu
##! Exercise 15.2, the most common item. Kind: fix. Level: 1.
##!
##! `most_common` finds the item that appears most often in a list, for
##! any type of item that can be compared and copied. The file does not
##! compile. Run `sisu test most_common.sisu`, read the diagnostics, and
##! fix `most_common`, not the tests, so that every test passes. The
##! hints are in chapter 15 of the tutorial, under "Exercise 15.2: the
##! most common item".

## The item that appears most often in `items`, the first of them when
## several appear as often; `None` for an empty list.
def most_common[T](items: list[T]) -> T?:
    var best: T? = None
    var best_count = 0
    for item in items:
        var count = 0
        for other in items:
            if other == item:
                count += 1
        if count > best_count:
            best = Some(item)
            best_count = count
    return best

@derive(Copy, Eq)
enum Weather:
    Sun
    Rain
    Snow

test "numbers":
    assert most_common([3, 1, 3, 2, 1, 3]) == Some(3)
    assert most_common([5, 7, 7, 5]) == Some(5)

test "words":
    assert most_common(["kyllä", "ei", "ei"]) == Some("ei")

test "a week of weather":
    let week = [Rain, Sun, Rain, Snow, Sun, Rain, Sun]
    assert most_common(week) == Some(Rain)

test "nothing at all":
    let empty: list[int] = []
    assert most_common(empty) == None
```

<!-- output: solutions/15-generics/most_common.console -->
```console
$ sisu test most_common.sisu
error[E0347]: `T` does not implement `Eq`
  --> most_common.sisu:18:16
   |
18 |             if other == item:
   |                ^^^^^^^^^^^^^
  = fix: bound `T` by `Eq`: `: Eq`

error[E0300]: expected `T`, found `ref T`
  --> most_common.sisu:21:25
   |
21 |             best = Some(item)
   |                         ^^^^
  = fix: copy it: `.copy()`

error[D0006]: the program was not run: errors found (2)
```

<details>
<summary>Hint 1</summary>

Each diagnostic offers a fix. The second says that `item` is a `ref
T`, a borrow of an item of the list, where `best` holds a `T`, and its
fix makes a copy.

</details>

<details>
<summary>Hint 2</summary>

Chapter 10 made a borrowed value into a value of its own by copying
it. The doc comment says which abilities the item type needs.

</details>

When every test passes, compare your answer with
[`manual/solutions/15-generics/most_common.sisu`](../solutions/15-generics/most_common.sisu).

## Exercise 15.3: the largest

Open
[`manual/exercises/15-generics/largest.sisu`](../exercises/15-generics/largest.sisu):

<!-- file: exercises/15-generics/largest.sisu -->
```sisu
##! Exercise 15.3, the largest. Kind: write. Level: 2.
##!
##! Write the body of `largest`, which works for a list of any type that
##! has an order, so that every test passes. Run the tests with
##! `sisu test largest.sisu`. The hints are in chapter 15 of the
##! tutorial, under "Exercise 15.3: the largest".

## The largest of `items`, the first of them when several are as large;
## `None` for an empty list.
def largest[T](items: list[T]) -> T?:
    return todo("largest")

## A version number, ordered by its major number first.
@derive(Copy, Eq, Ord)
struct Version:
    major: int
    minor: int

test "numbers":
    assert largest([3, 9, 4]) == Some(9)
    assert largest([-5, -2, -7]) == Some(-2)

test "words":
    assert largest(["kuusi", "koivu", "mänty"]) == Some("mänty")

test "versions":
    let versions = [Version(1, 10), Version(2, 0), Version(1, 9)]
    assert largest(versions) == Some(Version(2, 0))

test "an empty list":
    let empty: list[int] = []
    assert largest(empty) == None
```

<!-- output: solutions/15-generics/largest.console -->
```console
$ sisu test largest.sisu
warning[W0001]: typed hole `largest` of type `T?`
  --> largest.sisu:11:12
   |
11 |     return todo("largest")
   |            ^^^^^^^^^^^^^^^

test "numbers" ... blocked by todo("largest")
test "words" ... blocked by todo("largest")
test "versions" ... blocked by todo("largest")
test "an empty list" ... blocked by todo("largest")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Write the body as you would for a list of `int`, and run the tests.
The diagnostics say which bounds `T` needs.

</details>

<details>
<summary>Hint 2</summary>

Keep the largest so far in a `T?`, starting at `None`. Only an item
that is larger replaces it, which keeps the first of equal items.

</details>

When every test passes, compare your answer with
[`manual/solutions/15-generics/largest.sisu`](../solutions/15-generics/largest.sisu).

## Exercise 15.4: a stack

Open
[`manual/exercises/15-generics/stack.sisu`](../exercises/15-generics/stack.sisu):

<!-- file: exercises/15-generics/stack.sisu -->
```sisu
##! Exercise 15.4, a stack. Kind: write. Level: 2.
##!
##! A stack holds items of any one type: the last item pushed is the
##! first one taken off. Write the bodies of its methods and impls so
##! that every test passes. Run the tests with `sisu test stack.sisu`.
##! The hints are in chapter 15 of the tutorial, under "Exercise 15.4: a
##! stack".

## Items of type `T`, taken off in the reverse of the order they came.
struct Stack[T]:
    items: list[T]

    ## A stack with nothing on it.
    def empty() -> Stack[T]:
        return todo("empty")

    ## Puts `item` on top.
    def push(mut self, own item: T):
        todo("push")

    ## Takes the top item off, or gives `None` when there is none.
    def pop(mut self) -> T?:
        return todo("pop")

    ## The top item, left where it is.
    def peek(self) -> ref T?:
        return todo("peek")

impl[T] Len for Stack[T]:
    def len(self) -> int:
        return todo("len")

## A stack written from the bottom to the top, as in "[1, 2, 3>".
impl[T: Display] Display for Stack[T]:
    def fmt(self, mut f: Formatter):
        todo("fmt")

test "the last in is the first out":
    var stack = Stack[int].empty()
    stack.push(1)
    stack.push(2)
    stack.push(3)
    assert stack.pop() == Some(3)
    assert stack.pop() == Some(2)
    assert len(stack) == 1

test "an empty stack":
    var stack: Stack[String] = Stack.empty()
    assert stack.pop() == None
    assert stack.peek() == None
    assert len(stack) == 0

test "a look at the top":
    var plates: Stack[String] = Stack.empty()
    plates.push("blue")
    plates.push("green")
    assert plates.peek().is_some()
    if let Some(top) = plates.peek():
        assert top == "green"
    assert len(plates) == 2

test "a stack as text":
    var stack = Stack[int].empty()
    assert "{stack}" == "[>"
    for n in [4, 5, 6]:
        stack.push(n)
    assert "{stack}" == "[4, 5, 6>"
```

<!-- output: solutions/15-generics/stack.console -->
```console
$ sisu test stack.sisu
warning[W0001]: typed hole `empty` of type `Stack[T]`
  --> stack.sisu:15:16
   |
15 |         return todo("empty")
   |                ^^^^^^^^^^^^^

warning[W0001]: typed hole `push` of type `()`
  --> stack.sisu:19:9
   |
19 |         todo("push")
   |         ^^^^^^^^^^^^

warning[W0001]: typed hole `pop` of type `T?`
  --> stack.sisu:23:16
   |
23 |         return todo("pop")
   |                ^^^^^^^^^^^

warning[W0001]: typed hole `peek` of type `(ref T)?`
  --> stack.sisu:27:16
   |
27 |         return todo("peek")
   |                ^^^^^^^^^^^^

warning[W0001]: typed hole `len` of type `int`
  --> stack.sisu:31:16
   |
31 |         return todo("len")
   |                ^^^^^^^^^^^

warning[W0001]: typed hole `fmt` of type `()`
  --> stack.sisu:36:9
   |
36 |         todo("fmt")
   |         ^^^^^^^^^^^

test "the last in is the first out" ... blocked by todo("empty")
test "an empty stack" ... blocked by todo("empty")
test "a look at the top" ... blocked by todo("empty")
test "a stack as text" ... blocked by todo("empty")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

The stack keeps its items in a list, with the top at the end. Each
method is a line or two over that list's own methods.

</details>

<details>
<summary>Hint 2</summary>

`fmt` writes each item with `{item}`, which the impl's bound allows. A
comprehension makes the texts, and `join` puts the commas between them.

</details>

When every test passes, compare your answer with
[`manual/solutions/15-generics/stack.sisu`](../solutions/15-generics/stack.sisu).

## Exercise 15.5: the last few

Open
[`manual/exercises/15-generics/recent.sisu`](../exercises/15-generics/recent.sisu):

<!-- file: exercises/15-generics/recent.sisu -->
```sisu
##! Exercise 15.5, the last few. Kind: write. Level: 3.
##!
##! A program that watches a sensor keeps only its last `N` readings, in
##! `N` slots that never grow: a ring buffer. A new reading goes into the
##! slot after the newest one, round to the first slot after the last,
##! and replaces the oldest once every slot is full. Write `Recent`, and
##! add the fields it needs besides `slots`, so that every test passes.
##! Run the tests with `sisu test recent.sisu`. The hints are in chapter
##! 15 of the tutorial, under "Exercise 15.5: the last few".

## The last `N` items pushed, kept in `N` slots.
struct Recent[T, const N: int]:
    slots: Array[T?, N]

    ## A buffer that holds nothing yet.
    def empty() -> Recent[T, N]:
        return todo("empty")

    ## Keeps `item`, in place of the oldest item once the buffer is full.
    def push(mut self, own item: T):
        todo("push")

    ## The items, from the oldest to the newest.
    def items(self) -> list[T] where T: Copy:
        return todo("items")

    ## The newest item, or `None` when there is none.
    def newest(self) -> T? where T: Copy:
        return todo("newest")

test "fewer items than slots":
    var recent = Recent[int, 3].empty()
    recent.push(7)
    recent.push(8)
    assert recent.items() == [7, 8]
    assert recent.newest() == Some(8)

test "the oldest items go":
    var recent = Recent[int, 3].empty()
    for n in range(1, 6):
        recent.push(n)
    assert recent.items() == [3, 4, 5]
    assert recent.newest() == Some(5)

test "round and round":
    var recent = Recent[str, 2].empty()
    for word in ["yksi", "kaksi", "kolme", "neljä", "viisi"]:
        recent.push(word)
    assert recent.items() == ["neljä", "viisi"]

test "nothing yet":
    let recent = Recent[f64, 4].empty()
    assert recent.items() == []
    assert recent.newest() == None
```

<!-- output: solutions/15-generics/recent.console -->
```console
$ sisu test recent.sisu
warning[W0001]: typed hole `empty` of type `Recent[T, N]`
  --> recent.sisu:17:16
   |
17 |         return todo("empty")
   |                ^^^^^^^^^^^^^

warning[W0001]: typed hole `push` of type `()`
  --> recent.sisu:21:9
   |
21 |         todo("push")
   |         ^^^^^^^^^^^^

warning[W0001]: typed hole `items` of type `list[T]`
  --> recent.sisu:25:16
   |
25 |         return todo("items")
   |                ^^^^^^^^^^^^^

warning[W0001]: typed hole `newest` of type `T?`
  --> recent.sisu:29:16
   |
29 |         return todo("newest")
   |                ^^^^^^^^^^^^^^

test "fewer items than slots" ... blocked by todo("empty")
test "the oldest items go" ... blocked by todo("empty")
test "round and round" ... blocked by todo("empty")
test "nothing yet" ... blocked by todo("empty")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

`slots` alone does not say where the oldest item is or how many slots
are in use. Two `int` fields can, and `empty()` starts both at 0.

</details>

<details>
<summary>Hint 2</summary>

The slot after slot `i`, going round, is `(i + 1) % N`. When the
buffer is full, a new item goes where the oldest is, and the next
oldest becomes the oldest.

</details>

<details>
<summary>Hint 3</summary>

`Array.filled(None)` makes the empty slots, and the section "Numbers as
type parameters" shows that `N` is a value inside the methods.

</details>

When every test passes, compare your answer with
[`manual/solutions/15-generics/recent.sisu`](../solutions/15-generics/recent.sisu).

## What you learned

- A generic function or type takes type parameters, `[T: Bound]`, and a
  call works out the types from its arguments. Several bounds join with
  `+`, and a `where` clause can hold them.
- A body is checked against its bounds where it is written (E0347,
  E0304), and a call against the bounds where it is made, so no error
  appears inside a generic function at a use.
- A generic impl, `impl[T: Display] Display for Labelled[T]`, gives the
  ability to every type it covers. A type parameter can have a default.
- `dyn Trait` is a value that carries its type, behind a `Box` or a
  `ref` (E0395), for a trait whose methods can be called through it
  (E0241).
- Generic code is copied for each type and costs nothing while it
  runs; `dyn` code is compiled once and looks its methods up. Prefer
  generics, and an enum for a closed set of kinds.
- `const N: int` makes a number part of a type, as in `Array[T, N]`,
  and an argument can be an expression of such numbers.

Chapter 16 is about functions as values: closures, the `Iterator`
trait and the adapters that make loops into pipelines, and generators.
