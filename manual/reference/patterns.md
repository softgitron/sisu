# Patterns

A pattern describes the shape of a value: it tests the value and binds
names to its parts. This page lists every form of pattern, the places a
pattern stands, how a name binds, and the rules that a `match` covers
every value and that every arm can be reached.

**See also:** the tutorial's [chapter 4](../tutorial/04-control-flow.md)
(`match` on values) and [chapter 7](../tutorial/07-structs-and-enums.md)
(destructuring, exhaustiveness); the proposal's [02, "Control
flow"](../../proposal/02-syntax.md) and [03, "Sum
types"](../../proposal/03-types-and-generics.md); the decision D-043;
[R17, Diagnostics](diagnostics.md) for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains.

## Where patterns stand

A pattern stands in each of these places:

| Place | Form | When the pattern does not match |
|---|---|---|
| A `match` arm | `case PATTERN:` or `case PATTERN if GUARD:` | the next arm is tried |
| `if let` | `if let PATTERN = VALUE:` | the `else` branch runs |
| `while let` | `while let PATTERN = VALUE:` | the loop ends |
| A chained condition | `if let P = A, let Q = B, TEST:` | the chain fails at that part |
| `let ... else` | `let PATTERN = VALUE else: ...` | the `else` block leaves the scope |
| A plain `let` or `var` | `let (a, b) = pair` | the pattern must match every value (E0329) |
| A `for` loop | `for (i, word) in enumerate(words):` | the pattern must match every element |

<!-- file: examples/reference-patterns/other_places.sisu -->
```sisu
# Patterns in `let`, `if let`, `while let`, `let ... else` and `for`.
def main():
    let (name, age) = ("Aino", 7)
    if let Some(n) = "42".parse_int():
        print("{name} is {age}; parsed {n}")
    var stack = [1, 2, 3]
    while let Some(top) = stack.pop():
        print("popped {top}")
    let Some(first) = "1 2".split_whitespace().next() else:
        return
    for (i, word) in enumerate(["a", "b"]):
        print("{first} {i} {word}")
```

<!-- output: examples/reference-patterns/other_places.out -->
```text
Aino is 7; parsed 42
popped 3
popped 2
popped 1
1 0 a
1 1 b
```

The parts of a chained condition run left to right; each binding is in
scope for the rest of the chain and in the branch it guards (leaning,
D-071).

**Status.** In stage 0.

## Literal patterns

An integer, a negative integer, a character, a string or a boolean
literal, and `None`, match a value equal to them. `_` matches any value
and binds nothing. `|` between patterns matches when any one does.

<!-- file: examples/reference-patterns/literals.sisu -->
```sisu
# Literal patterns, alternatives with `|`, and `_` for any value.
def describe(n: int) -> str:
    return match n:
        case 0: "none"
        case 1 | 2 | 3: "a few"
        case -1: "one owed"
        case _: "many"

def kind(c: char) -> str:
    return match c:
        case 'a' | 'e' | 'i' | 'o' | 'u': "a vowel"
        case ' ': "a space"
        case _: "another character"

def main():
    print("{describe(0)}, {describe(2)}, {describe(-1)}, {describe(9)}")
    print("{kind('e')}, {kind(' ')}, {kind('k')}")
    let answer = match "kyllä":
        case "kyllä" | "joo": True
        case _: False
    print(answer)
```

<!-- output: examples/reference-patterns/literals.out -->
```text
none, a few, one owed, many
a vowel, a space, another character
True
```

**Status.** In stage 0.

## Names that bind and names that name

A lowercase name in a pattern binds the value it stands for, and so
matches anything; a capitalised name is a variant or a constant, and
matches only that value. The naming rules make the two cases tell
themselves apart ([R1](lexical-structure.md#naming-rules)), where Python's
`case` cannot.

<!-- file: examples/reference-patterns/bindings.sisu -->
```sisu
# A lowercase name binds; a capitalised one names a variant.
enum Light:
    Red
    Amber
    Green

def action(light: Light) -> String:
    return match light:
        case Red: "stop"
        case other: "not red: {other:?}"

def main():
    print(action(Red))
    print(action(Green))
```

<!-- output: examples/reference-patterns/bindings.out -->
```text
stop
not red: Green
```

A capitalised name that the subject's type has no variant of is E0327
([R17](diagnostics.md#e0327)). A variant's own name is enough where it
is not ambiguous; where two enums share it, qualify it as `Light.Red`
(E0211).

**Status.** In stage 0.

## Variant patterns

A variant pattern names a variant and gives a pattern for each of its
fields: `Dot` for a unit variant, `Circle(r)` for a tuple variant, and
`Rect(w, h)` or `Rect(w=w, h=h)` for a record variant, by position or by
keyword. `Some(x)`, `None`, `Ok(v)` and `Err(e)` are variant patterns of
the prelude's enums.

<!-- file: examples/reference-patterns/variants.sisu -->
```sisu
# Variant patterns take apart unit, tuple and record variants.
enum Shape:
    Dot
    Circle(f64)
    Rect(w: f64, h: f64)

def area(shape: Shape) -> f64:
    return match shape:
        case Dot: 0.0
        case Circle(r): 3.0 * r * r
        case Rect(w, h): w * h

def main():
    for shape in [Dot, Circle(1.0), Rect(w=2.0, h=3.0)]:
        print(area(shape))
```

<!-- output: examples/reference-patterns/variants.out -->
```text
0.0
3.0
6.0
```

**Status.** In stage 0.

## Struct patterns

A struct pattern is the struct's name with a pattern for its fields, by
position in declaration order or by keyword, as in Python. `..` at the
end stands for the fields not named; without it, a pattern names every
field, and one that leaves a field out is E0328
([R17](diagnostics.md#e0328)).

<!-- file: examples/reference-patterns/structs.sisu -->
```sisu
# Struct patterns by position and keyword, with `..` for the rest.
struct Point:
    x: int
    y: int
    z: int = 0

def place(p: Point) -> String:
    return match p:
        case Point(0, 0, 0): "the origin"
        case Point(x=0, ..): "on the y-z plane"
        case Point(x, y, z=0): "flat at {x}, {y}"
        case _: "somewhere else"

def main():
    print(place(Point(0, 0)))
    print(place(Point(0, 5, 1)))
    print(place(Point(2, 3)))
    print(place(Point(1, 1, 1)))
```

<!-- output: examples/reference-patterns/structs.out -->
```text
the origin
on the y-z plane
flat at 2, 3
somewhere else
```

**Status.** In stage 0.

## Tuple and list patterns

A tuple pattern, `(x, y)`, has a pattern per element. A list pattern,
`[first, second]`, matches a list of exactly that length, and `*rest` in
it takes the elements that the other patterns leave, as a list, so
`[first, *rest]` matches any list with at least one element.

<!-- file: examples/reference-patterns/sequences.sisu -->
```sisu
# Tuple and list patterns; `*rest` takes the rest of a list.
def first_two(xs: list[int]) -> String:
    return match xs:
        case []: "empty"
        case [only]: "just {only}"
        case [first, second, *rest]: "{first}, {second} and {len(rest)} more"

def main():
    print(first_two([]))
    print(first_two([7]))
    print(first_two([1, 2, 3, 4]))
    let point = (3, -1)
    let quadrant = match point:
        case (0, 0): "origin"
        case (x, y) if x > 0 and y > 0: "first"
        case (_, y) if y < 0: "below the axis"
        case _: "elsewhere"
    print(quadrant)
```

<!-- output: examples/reference-patterns/sequences.out -->
```text
empty
just 7
1, 2 and 2 more
below the axis
```

**Stage 0 differs.** `let first, *rest = xs`, star unpacking in a plain
`let`, is designed in [02](../../proposal/02-syntax.md) and stage 0 does
not have it; the list pattern `[first, *rest]` works in `match` and in
`let ... else`.

**Status.** In stage 0, except star unpacking in a plain `let`.

## Nested patterns and guards

Every part of a pattern is a pattern, so patterns nest: `Some(Rect(w,
h))`. A guard, `if CONDITION` after the pattern, adds a test that runs
after the pattern matched, with the pattern's names in scope; an arm
whose guard is `False` does not match, and the next arm is tried.

<!-- file: examples/reference-patterns/nested.sisu -->
```sisu
# Patterns nest, and a guard adds a condition after `if`.
enum Shape:
    Circle(f64)
    Rect(w: f64, h: f64)

def describe(shape: Shape?) -> String:
    return match shape:
        case None: "nothing"
        case Some(Rect(w, h)) if w == h: "a square of {w}"
        case Some(Rect(w, h)): "a {w} by {h} rectangle"
        case Some(Circle(r)): "a circle of radius {r}"

def main():
    print(describe(Some(Rect(w=2.0, h=2.0))))
    print(describe(Some(Rect(w=2.0, h=3.0))))
    print(describe(Some(Circle(1.0))))
    print(describe(None))
```

<!-- output: examples/reference-patterns/nested.out -->
```text
a square of 2.0
a 2.0 by 3.0 rectangle
a circle of radius 1.0
nothing
```

**Status.** In stage 0.

## Alternatives that bind

The alternatives of `|` may bind names, as long as every alternative
binds the same names with the same types; the arm then runs with
whichever alternative matched.

<!-- file: examples/reference-patterns/or_bindings.sisu -->
```sisu
# Alternatives that bind must bind the same names.
enum Reading:
    Celsius(f64)
    Kelvin(f64)
    Missing

def value(r: Reading) -> f64:
    return match r:
        case Celsius(t) | Kelvin(t): t
        case Missing: 0.0

def main():
    print("{value(Celsius(21.5))} {value(Kelvin(290.0))} {value(Missing)}")
```

<!-- output: examples/reference-patterns/or_bindings.out -->
```text
21.5 290.0 0.0
```

An alternative that leaves out a name another binds is E0385:

<!-- file: examples/reference-patterns/or_unbound.sisu -->
```sisu
# An alternative that binds a name the other does not.
enum Reading:
    Celsius(f64)
    Missing

def value(r: Reading) -> f64:
    return match r:
        case Celsius(t) | Missing: t

def main():
    print(value(Missing))
```

<!-- output: examples/reference-patterns/or_unbound.console -->
```console
$ sisu run or_unbound.sisu
error[E0385]: `t` is bound in some alternatives of `|` but not in all; every alternative must bind the same names
  --> or_unbound.sisu:8:27
  |
8 |         case Celsius(t) | Missing: t
  |                           ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0.

## A match covers every value

A `match` covers every value of its subject's type, so that a new
variant shows each `match` that must handle it (D-043). One that does
not is E0314, which names every case it misses and suggests an arm for
each or a `case _` arm.

<!-- file: examples/reference-patterns/missing_case.sisu -->
```sisu
# A `match` that does not cover every value.
enum Light:
    Red
    Amber
    Green

def action(light: Light) -> str:
    return match light:
        case Red: "stop"

def main():
    print(action(Green))
```

<!-- output: examples/reference-patterns/missing_case.console -->
```console
$ sisu run missing_case.sisu
error[E0314]: this `match` does not cover `Amber`, `Green`
  --> missing_case.sisu:8:12
  |
8 |     return match light:
  |            ^^^^^^^^^^^
  = note: add an arm for each of `Amber`, `Green`, or a `case _` arm

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** The linter of
[03](../../proposal/03-types-and-generics.md) warns about `case _` on an
enum of the same package, since a new variant should then be a compile
error; stage 0 has no such warning.

**Status.** In stage 0 (D-043).

## Every arm can be reached

Arms are tried from the top, and the first that matches runs. An arm
whose values the arms above it all cover is W0003, a warning, and the
program runs:

<!-- file: examples/reference-patterns/unreachable.sisu -->
```sisu
# An arm that the arms above it cover.
def describe(n: int) -> str:
    return match n:
        case _: "a number"
        case 0: "zero"

def main():
    print(describe(0))
```

<!-- output: examples/reference-patterns/unreachable.console -->
```console
$ sisu run unreachable.sisu
warning[W0003]: this `case` arm is unreachable; the arms above it cover every value it matches
  --> unreachable.sisu:5:14
  |
5 |         case 0: "zero"
  |              ^

a number
```

**Status.** In stage 0.

## A plain let matches every value

The pattern of a plain `let`, `var` or `for` must match every value
of its type, as a tuple pattern or a struct pattern does. One that might
not, such as `Some(n)`, is E0329: add `else:` with a block that leaves
the scope, or use `if let` or `match`.

<!-- file: examples/reference-patterns/refutable.sisu -->
```sisu
# A plain `let` whose pattern might not match.
def main():
    let maybe: int? = None
    let Some(n) = maybe
    print(n)
```

<!-- output: examples/reference-patterns/refutable.console -->
```console
$ sisu run refutable.sisu
error[E0329]: this pattern might not match; add `else:` with a block that leaves the scope
  --> refutable.sisu:4:9
  |
4 |     let Some(n) = maybe
  |         ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0105](diagnostics.md#e0105) | `parse.expected-pattern` | [Literal patterns](#literal-patterns) |
| [E0211](diagnostics.md#e0211) | `resolve.ambiguous-variant` | [Names that bind and names that name](#names-that-bind-and-names-that-name) |
| [E0314](diagnostics.md#e0314) | `type.non-exhaustive` | [A match covers every value](#a-match-covers-every-value) |
| [E0328](diagnostics.md#e0328) | `type.pattern-field-count` | [Struct patterns](#struct-patterns) |
| [E0327](diagnostics.md#e0327) | `type.unknown-variant` | [Names that bind and names that name](#names-that-bind-and-names-that-name) |
| [E0329](diagnostics.md#e0329) | `type.refutable-let` | [A plain let matches every value](#a-plain-let-matches-every-value) |
| [E0385](diagnostics.md#e0385) | `type.bindings-in-or-pattern` | [Alternatives that bind](#alternatives-that-bind) |
| [W0003](diagnostics.md#w0003) | `type.unreachable-arm` | [Every arm can be reached](#every-arm-can-be-reached) |
