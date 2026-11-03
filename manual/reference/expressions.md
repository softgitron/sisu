# Expressions and operators

This page states what each expression means: the operators and their
precedence, calls, indexing and slicing, comprehensions, closures, the
expression forms of `if` and `match`, the operators and fallbacks of
optional values and results, interpolation and its format specs, and
`comptime` and `unsafe` expressions. The tokens themselves are
[R1](lexical-structure.md#operators-and-punctuation); the statements
that hold expressions are [R5](statements.md).

**See also:** the tutorial's [chapter 2](../tutorial/02-values.md)
(arithmetic, comparisons, logic), [chapter 3](../tutorial/03-functions.md)
(calls), [chapter 5](../tutorial/05-collections.md) (indexing,
slicing, comprehensions), [chapter 6](../tutorial/06-text.md) (format
specs), [chapter 8](../tutorial/08-errors.md) (`?`, `?.`, `??`,
`else`) and [chapter 16](../tutorial/16-closures-and-iterators.md)
(closures); the proposal's [02, "Expressions and
statements"](../../proposal/02-syntax.md#expressions-and-statements);
[R17, Diagnostics](diagnostics.md#parsing) for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Blocks without a value (D-056), the fixed operator set
(D-022) and `and`, `or`, `not` without truthiness (D-025) are decided.

## Precedence

From the operators that bind tightest to the loosest:

| Level | Operators | Notes |
|---|---|---|
| Postfix | `x.f`, `x[i]`, `x[i:j]`, `f(x)`, `x?`, `x?.f` | left to right |
| Power | `**` | right to left; `-x ** y` is [E0112](diagnostics.md#e0112) |
| Unary | `-x`, `~x`, `ref x`, `ref mut x` | |
| Multiplicative | `*`, `/`, `//`, `%`, `*%`, `*\|` | |
| Additive | `+`, `-`, `+%`, `-%`, `+\|`, `-\|` | |
| Shift | `<<`, `>>` | not mixed with arithmetic without parentheses (E0111) |
| Bitwise | `&`, `^`, `\|` | not mixed with each other (E0109) or with arithmetic and shifts ([E0110](diagnostics.md#e0110)) without parentheses |
| Comparison | `==`, `!=`, `<`, `<=`, `>`, `>=`, `in`, `not in` | chained in one direction ([Comparisons](#comparisons)) |
| `not` | `not x` | |
| `and` | `a and b` | not mixed with `or` without parentheses ([E0108](diagnostics.md#e0108)) |
| `or` | `a or b` | |
| `??` | `a ?? b` | the fallback of an optional value |
| Closure | `x -> e` | |

Assignment and augmented assignment are statements, not operators, so
`if x = y:` is E0114 and there is no walrus (E0123).

<!-- file: examples/reference-expressions/mixing.sisu -->
```sisu
# Mixtures that need parentheses.
def main():
    let ready = True
    let warm = False
    let n = 5
    print(ready and warm or ready)
    print(n & 1 + 2)
    print(-n ** 2)
    print(1 < n > 3)
```

<!-- output: examples/reference-expressions/mixing.console -->
```console
$ sisu check mixing.sisu
error[E0108]: `and` and `or` cannot be mixed without parentheses
  --> mixing.sisu:6:26
  |
6 |     print(ready and warm or ready)
  |                          ^^

error[E0110]: `&` cannot be mixed with arithmetic or shifts without parentheses
  --> mixing.sisu:7:20
  |
7 |     print(n & 1 + 2)
  |                    ^
  = fix: add parentheses: `(`, `)`

error[E0112]: `-x ** y` is ambiguous; write `-(x ** y)` or `(-x) ** y`
  --> mixing.sisu:8:18
  |
8 |     print(-n ** 2)
  |                  ^
  = fix: add parentheses: `(`, `)`

error[E0113]: a chained comparison must point one way; `<` and `>` do not
  --> mixing.sisu:9:17
  |
9 |     print(1 < n > 3)
  |                 ^

```

**Status.** In stage 0 (D-022).

## Arithmetic

| Operator | On | Gives |
|---|---|---|
| `a + b`, `a - b`, `a * b` | numbers of one type | that type; an overflow panics ([R2](types.md#overflow-traps)) |
| `a / b` | floats | a float; on integers [E0301](diagnostics.md#e0301) |
| `a // b` | integers | the quotient rounded down, toward negative infinity; on floats E0372 |
| `a % b` | integers or floats | the remainder, with the sign of `b`, so `-7 % 3` is 2 |
| `a ** b` | numbers | the power; a negative integer exponent panics |
| `+%`, `-%`, `*%` and `+\|`, `-\|`, `*\|` | integers | the wrapping and the saturating forms |
| `&`, `\|`, `^`, `~a`, `<<`, `>>` | integers | the bits, and shifts by 0 to the width less one |
| `-a` | numbers | the negation |

An `int` and an `f64` do not mix (E0373), and an integer widens to a
wider one where that loses nothing ([R2](types.md#widening-and-conversions)).
On other types the operators call the operator traits, `Add`, `Sub`,
`Mul`, `Div`, `Rem` and `Neg`, which a type implements to take them
(E0310 otherwise, [R17](diagnostics.md#e0310); D-022). `String`
implements `Add` for joining text.

<!-- file: examples/reference-expressions/arithmetic.sisu -->
```sisu
# The arithmetic and bitwise operators on integers and floats.
def main():
    print("{7 // 2} {-7 // 2} {-7 % 3} {7 % -3} {2 ** 10}")
    print("{7.0 / 2.0} {2.0 ** 0.5} {-(2 ** 2)} {(-2) ** 2}")
    print("{6 & 3} {6 | 3} {6 ^ 3} {~6} {1 << 4} {-16 >> 2}")
    let level: u8 = 250
    print("{level +% 10} {level +| 10} {level -| 251}")
```

<!-- output: examples/reference-expressions/arithmetic.out -->
```text
3 -4 2 -2 1024
3.5 1.4142135623730951 -4 4
2 7 5 -7 16 -4
4 255 0
```

**Status.** In stage 0 (D-009, D-022).

## Comparisons

`==` and `!=` compare by value through `Eq`, and `<`, `<=`, `>` and `>=`
by order through `PartialOrd`: a list, a tuple and a text compare
element by element. There is no `is`: identity is not a question about
values.

- A chain, `a < b <= c`, means `a < b and b <= c`, with `b` computed
  once, and points one way: `a < b > c` is E0113 ([R17](diagnostics.md#e0113)).
- `!=`, `in` and `not in` do not chain (E0119).
- `x in xs` is `xs.contains(x)` through the `Contains` trait: an element
  of a list or a set, a key of a `dict`, a part of a text.
- Values of two types that cannot be compared are E0369, and a type
  without an order E0368 ([R17](diagnostics.md#e0368)).

<!-- file: examples/reference-expressions/comparisons.sisu -->
```sisu
# Comparisons chain in one direction; `in` asks a collection.
def main():
    let n = 5
    let xs = [1, 2, 3]
    print("{1 < n <= 5} {10 > n >= 5} {n == 5}")
    let ages = {"Aino": 7}
    print("{2 in xs} {4 not in xs} {"au" in "sauna"} {"Aino" in ages}")
    print("{[1, 2] == [1, 2]} {(1, "a") < (1, "b")} {"aino" < "eero"}")
```

<!-- output: examples/reference-expressions/comparisons.out -->
```text
True True True
True True True True
True True True
```

**Stage 0 differs.** `==` between two `Shared` values gives `False`
whatever they hold, where 02 compares what they hold and keeps
`Shared.same(a, b)` for identity ([R7](ownership.md#sharing-cells-and-graphs)).

**Status.** In stage 0 (D-025).

## Logic

`and`, `or` and `not` take `bool`s and give a `bool`; `and` and `or`
evaluate their right side only when the left does not decide. Nothing
else is a condition: a number, a text or a collection as one is E0302,
whose note names the explicit test ([R2](types.md#truth-values-and-characters)).
`&&`, `||` and `!` are not operators ([R19](false-friends.md)).

**Status.** In stage 0 (D-025).

## Calls

`f(args)` calls a function, `x.m(args)` a method of `x`'s type or of a
trait it implements, and `Type.f(args)` an associated function. A
struct, an enum's variant or an error is built by a call on its name
([R3](declarations.md#structs)).

- Arguments are given by position, then by keyword, `f(1, step=2)`
  (E0376 for a positional one after a keyword); a keyword names a
  parameter (E0306), once (E0307), and every parameter without a
  default gets an argument (E0308). The wrong number of positional
  arguments is E0305.
- A `mut` parameter's argument says `mut`, `f(mut x)` (E0339, E0340;
  [R7](ownership.md#parameter-conventions)).
- A function or a closure is a value that can be passed and called; a
  method is not one (E0350), and a closure that calls it is.
- `x.m()` looks through references, `Box`, `Shared` and mutex guards to
  the value, and a method that two traits of the type give is E0394,
  called on the trait as `Trait.m(x)`.

<!-- file: examples/reference-expressions/calls.sisu -->
```sisu
# Calls by position and keyword; methods and associated functions.
struct Booking:
    name: String
    hour: int = 18

    def at(name: str, hour: int) -> Booking:
        return Booking(name.to_string(), hour)

    def label(self, prefix: str = "") -> String:
        return "{prefix}{self.name} at {self.hour}"

def main():
    let a = Booking("Aino")
    let b = Booking(hour=17, name="Eero")
    let c = Booking.at("Roni", 19)
    print("{a.label()}, {b.label(prefix="sauna: ")}, {c.label()}")
    let labels = [a, b].iter().map(x -> x.label()).collect()
    print(labels)
```

<!-- output: examples/reference-expressions/calls.out -->
```text
Aino at 18, sauna: Eero at 17, Roni at 19
["Aino at 18", "Eero at 17"]
```

**Status.** In stage 0 (D-050).

## Indexing and slicing

`xs[i]` is the element at `i`, counting from 0, and `xs[^i]` the `i`th
from the end, so `xs[^1]` is the last. `d[k]` is the value of a `dict`
at the key `k`. `xs[i:j]` is a new list of the elements from `i` up to
`j`, and either bound may be left out.

- An index is checked: one past the end panics (R0002), and so does a
  key a `dict` does not have (R0010), where `d.get(k)` gives a `V?`.
- A constant negative index, Python's `xs[-1]`, does not compile
  (E0344, with the fix `^1`), since it would mean the other end in
  Python; a negative index computed while the program runs panics
  (R0015) (D-062).
- An index, a key or a field is a place that an assignment can change:
  `grid[1][0] = 5`.
- A text has no index by position (E0345;
  [R2](types.md#text)).

<!-- file: examples/reference-expressions/indexing.sisu -->
```sisu
# An index counts from the start, `^` from the end; a slice is a copy.
def main():
    let xs = [10, 20, 30, 40]
    print("{xs[0]} {xs[^1]} {xs[^2]}")
    print("{xs[1:3]} {xs[2:]} {xs[:2]} {xs[:]}")
    let ages = {"Aino": 7}
    print("{ages["Aino"]} {ages.get("Eero")}")
    var grid = [[0, 0], [0, 0]]
    grid[1][0] = 5
    print(grid)
```

<!-- output: examples/reference-expressions/indexing.out -->
```text
10 40 30
[20, 30] [30, 40] [10, 20] [10, 20, 30, 40]
7 None
[[0, 0], [5, 0]]
```

<!-- file: examples/reference-expressions/negative_index.sisu -->
```sisu
# Python's negative index does not compile.
def main():
    let xs = [10, 20, 30]
    print(xs[-1])
```

<!-- output: examples/reference-expressions/negative_index.console -->
```console
$ sisu run negative_index.sisu
error[E0344]: a constant negative index does not compile; count from the end with `^`: `xs[^1]`
  --> negative_index.sisu:4:14
  |
4 |     print(xs[-1])
  |              ^^
  = fix: count from the end with `^`: `^1`

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** `^` in a slice bound, `xs[:^1]` for all but the
last element, which 02 gives, is E0383 in both front ends; `^` works in
an index only.

**Status.** In stage 0 (D-062), except `^` in a slice.

## Comprehensions

`[e for x in xs]`, `{k: v for x in xs}` and `{e for x in xs}` build a
list, a `dict` and a `set` from a loop over `xs`, keeping the elements
for which an `if` at the end holds. The loop's pattern may unpack, as
in `for i, x in enumerate(xs)`.

<!-- file: examples/reference-expressions/comprehensions.sisu -->
```sisu
# List, dict and set comprehensions build a collection from a loop.
def main():
    let xs = [3, 1, 2, 4]
    let squares = [x * x for x in xs if x > 1]
    let lengths = {name: len(name) for name in ["Aino", "Eero"]}
    let parities = {x % 2 for x in xs}
    let pairs = [(i, x) for i, x in enumerate(xs) if i < 2]
    print("{squares} {lengths} {parities} {pairs}")
```

<!-- output: examples/reference-expressions/comprehensions.out -->
```text
[9, 4, 16] {"Aino": 4, "Eero": 4} {1, 0} [(0, 3), (1, 1)]
```

**Stage 0 differs.** A generator expression, `(e for x in xs)`, a lazy
`Iter`, is E0900.

**Status.** In stage 0, except generator expressions.

## Closures

A closure is a function written where it is used:

- `x -> e`, `(a, b) -> e` and `() -> e` take parameters and give the
  value of one expression; a parameter may be typed, `(n: int) -> e`.
- `it` stands for the one parameter of a closure where a function of one
  parameter is expected: `map(it * 2)` is `map(x -> x * 2)` (E0342
  outside such a place).
- `call(args) do x:` with an indented block passes a closure whose body
  is the block to the call's last function-typed parameter; `return` in
  it returns from the closure. A `do` block stands only at the end of a
  line, so no block sits inside an expression.
- A closure borrows what it captures; `own x -> e` and `do own x:` keep
  copies instead ([R7](ownership.md#closures-and-their-captures)).
- The number of parameters is what its type says (E0353).

<!-- file: examples/reference-expressions/closures.sisu -->
```sisu
# Closures: `x -> e`, `it`, and a trailing `do` block for statements.
def twice(f: (int) -> int, x: int) -> int:
    return f(f(x))

def each(xs: list[int], f: (int) -> ()):
    for x in xs:
        f(x)

def main():
    let step = 3
    print("{twice(x -> x + step, 1)} {twice(it * 10, 2)}")
    print(twice((n) -> n - 1, 0))
    each([1, 2]) do x:
        let doubled = x * 2
        print("{x} doubled is {doubled}")
    let total = [1, 2, 3].iter().fold(0) do acc, x:
        return acc + x
    print(total)
```

<!-- output: examples/reference-expressions/closures.out -->
```text
7 200
-2
1 doubled is 2
2 doubled is 4
6
```

**Status.** In stage 0 (D-026).

## If and match as expressions

A block has no value: a function gives one only with `return` (D-056).
`if` and `match` are expressions when every branch is one expression:
`if c: a elif d: b else: e`, and a `match` whose arms are each `case P:
e`. An `if` expression has an `else` (E0116, [R17](diagnostics.md#e0116)),
a `match` expression covers every value ([R6](patterns.md#a-match-covers-every-value)),
and an arm with statements makes the whole a statement (E0384). The
branches have one type (E0365). Python's `a if c else b` is E0115
([R17](diagnostics.md#e0115)).

<!-- file: examples/reference-expressions/conditional.sisu -->
```sisu
# `if` and `match` are expressions when each branch is one expression.
def describe(n: int) -> str:
    return if n > 9: "many" elif n > 0: "a few" else: "none"

def main():
    let size = match len("sisu"):
        case 0: "empty"
        case 1 | 2 | 3: "short"
        case _: "long"
    print("{describe(12)} {describe(2)} {describe(0)} {size}")
```

<!-- output: examples/reference-expressions/conditional.out -->
```text
many a few none long
```

**Status.** In stage 0 (D-056).

## Optional values and results

| Expression | On | Means |
|---|---|---|
| `x?` | `T ! E` | the value, or return `Err(e)` from the function, the set widened to the function's |
| `x?` | `T?` | the value, or return `None` from a function that returns an optional value |
| `x?.f`, `x?.m()` | `T?` | the field or method of the value, or `None` |
| `a ?? b` | `T?` | the value, or `b` |
| `let v = x else: b` | `T ! E` or `T?` | the value, or the fallback `b`, an expression or a statement that leaves the scope |
| `let v = x else as e: b` | `T ! E` | the same, with the error bound to `e` in `b` |
| `x.or(b)`, `x.expect("why")` | `T ! E` or `T?` | the value or `b`; the value or a panic naming why |

- `?` in a function whose result cannot carry the error is E0320, one
  whose set does not hold it E0321, and on a value that is neither E0319
  ([R17](diagnostics.md#e0320)).
- `?.` and `??` need an optional value (E0370, E0371), so a result is
  handled with `?` or `else` instead.
- A result or an optional value that is not used is E0318; `_ = x`
  discards one on purpose ([R17](diagnostics.md#e0318)).

<!-- file: examples/reference-expressions/fallbacks.sisu -->
```sisu
# `?` returns early; `?.`, `??` and `else` give a value instead.
error ParseError:
    Bad(text: String)

def parse(text: str) -> int ! ParseError:
    let n = text.parse_int() else:
        return Err(Bad(text.to_string()))
    return Ok(n)

def doubled(text: str) -> int ! ParseError:
    return Ok(parse(text)? * 2)

def main():
    let a = parse("x") else: 0
    let b = parse("x") else as e: if e == Bad("x"): -1 else: -2
    let c = parse("7").or(0)
    let d = "5".parse_int() ?? 0
    let name: String? = None
    let shout = name?.to_uppercase() ?? "nobody"
    print("{a} {b} {c} {d} {shout} {doubled("21")} {doubled("x")}")
```

<!-- output: examples/reference-expressions/fallbacks.out -->
```text
0 -1 7 5 nobody Ok(42) Err(Bad(text="x"))
```

**Stage 0 differs.** `.or_else(f)`, which computes the fallback only
when it is needed, is E0304, and so are `try_map` and `try_collect`.
`?.` is one token, so Rust's `x?.lines()` on a result is E0370; it is
written `(x?).lines()`.

**Status.** In stage 0 (D-042), with the differences noted.

## Interpolation and format specs

`{e}` in a string literal writes `e`'s `Display` form into the text, and
makes the literal a `String`. After a colon, a format spec in Python's
mini-language shapes it: `[[fill]align][sign][#][0][width][,|_][.precision][type]`.

| Spec | Gives |
|---|---|
| `{x:>8}`, `{x:<8}`, `{x:^8}`, `{x:*^8}` | right, left or centre in 8 columns, with an optional fill |
| `{x:08}`, `{x:+}` | zeros before the number; a sign always |
| `{x:.2f}`, `{x:e}`, `{x:%}` | two decimals; scientific; a percentage |
| `{n:x}`, `{n:X}`, `{n:o}`, `{n:b}`, `{n:#b}` | hexadecimal, octal, binary, with `#` for the prefix |
| `{n:,}`, `{n:_}` | thousands separated |
| `{x:?}` | the `Debug` form, which every type has |
| `{x=}`, `{x=:spec}` | the expression's text, `=`, and its value |

`{{` and `}}` are literal braces. A spec that is not one is E0399, and a
type without the rendering a spec asks for E0380.

<!-- file: examples/reference-expressions/formats.sisu -->
```sisu
# Interpolation, with format specs, `Debug` forms and literal braces.
struct Point:
    x: int
    y: int

def main():
    let t = 7.5
    let n = 255
    let p = Point(1, 2)
    print("[{t:>8.2f}] [{t:<6}] [{"x":^5}] [{n:08}]")
    print("{n:x} {n:#b} {1234567:,} {0.25:%} {1e6:e}")
    print("{p:?} {t=} {n + 1=} {{literal braces}}")
```

<!-- output: examples/reference-expressions/formats.out -->
```text
[    7.50] [7.5   ] [  x  ] [00000255]
ff 0b11111111 1,234,567 25.000000% 1.000000e+06
Point(x=1, y=2) t=7.5 n + 1=256 {literal braces}
```

<!-- file: examples/reference-expressions/bad_spec.sisu -->
```sisu
# A format spec that is not one.
def main():
    let t = 7.5
    print("{t:>8.2z}")
```

<!-- output: examples/reference-expressions/bad_spec.console -->
```console
$ sisu run bad_spec.sisu
error[E0399]: `>8.2z` is not a format spec; it is `[[fill]align][sign][#][0][width][,|_][.precision][type]`, as in `>8.2f`, with the type one of `d x X o b e E f F % s ?`
  --> bad_spec.sisu:4:13
  |
4 |     print("{t:>8.2z}")
  |             ^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0 (leaning, D-071, for `{x=}`).

## Compile-time and unsafe expressions

`comptime e` computes `e` while the program is compiled and puts the
value in its place; `e` may use constants and call `pure` and
`comptime` functions, and nothing in it reaches the world (E0600 for a
value known only at run time;
[R12](compile-time.md#what-compile-time-code-may-do)). `unsafe e` marks
one expression whose unsafe operation the compiler cannot check
([R7](ownership.md#unsafe-code-and-raw-pointers));
one that holds no such operation is W0009.

<!-- file: examples/reference-expressions/compile_time.sisu -->
```sisu
# `comptime e` computes `e` while the program is compiled.
pure def fib(n: int) -> int:
    return if n < 2: n else: fib(n - 1) + fib(n - 2)

def main():
    let answer = comptime 6 * 7
    let big = comptime fib(20)
    print("{answer} {big}")
```

<!-- output: examples/reference-expressions/compile_time.out -->
```text
42 6765
```

**Status.** In stage 0 (D-018); `unsafe` arriving (D-080).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0108](diagnostics.md#e0108), E0109, [E0110](diagnostics.md#e0110), E0111, [E0112](diagnostics.md#e0112), E0114, [E0123](diagnostics.md#e0123) | `parse.mixed-and-or`, `parse.mixed-bitwise`, `parse.bitwise-arithmetic`, `parse.shift-arithmetic`, `parse.negative-power`, `parse.assignment-in-expression`, `parse.walrus` | [Precedence](#precedence) |
| [E0301](diagnostics.md#e0301), E0372, [E0373](diagnostics.md#e0373), [E0310](diagnostics.md#e0310) | `type.integer-division`, `type.float-floor-division`, `type.mixed-numbers`, `type.binary-operator` | [Arithmetic](#arithmetic) |
| [E0113](diagnostics.md#e0113), E0119, E0369, [E0368](diagnostics.md#e0368) | `parse.mixed-chain`, `parse.chain-operator`, `type.comparison-types`, `type.not-ordered` | [Comparisons](#comparisons) |
| [E0302](diagnostics.md#e0302) | `type.not-bool` | [Logic](#logic) |
| [E0305](diagnostics.md#e0305), [E0306](diagnostics.md#e0306), E0307, [E0308](diagnostics.md#e0308), E0376, E0350, E0394 | `type.argument-count`, `type.unknown-keyword`, `type.duplicate-argument`, `type.missing-argument`, `type.positional-after-keyword`, `type.not-a-value`, `type.ambiguous-method` | [Calls](#calls) |
| [E0344](diagnostics.md#e0344), E0383, [R0002](diagnostics.md#r0002), [R0010](diagnostics.md#r0010), R0015 | `type.constant-negative-index`, `type.from-end-outside-index`, `panic.index-out-of-bounds`, `panic.missing-key`, `panic.negative-index` | [Indexing and slicing](#indexing-and-slicing) |
| [E0342](diagnostics.md#e0342), E0353 | `type.it-outside-closure`, `type.closure-parameter-count` | [Closures](#closures) |
| [E0116](diagnostics.md#e0116), [E0115](diagnostics.md#e0115), E0384, E0365 | `parse.if-expression-else`, `parse.python-conditional`, `type.arm-needs-expression`, `type.if-branches` | [If and match as expressions](#if-and-match-as-expressions) |
| E0319, [E0320](diagnostics.md#e0320), E0321, E0370, E0371, [E0318](diagnostics.md#e0318) | `type.try-needs-fallible`, `type.try-context`, `type.error-not-in-set`, `type.optional-chain`, `type.coalesce`, `type.unused-result` | [Optional values and results](#optional-values-and-results) |
| [E0399](diagnostics.md#e0399), E0380 | `type.bad-format-spec`, `type.no-display` | [Interpolation and format specs](#interpolation-and-format-specs) |
| E0600, W0009 | `comptime.not-constant`, `unsafe.unused` | [Compile-time and unsafe expressions](#compile-time-and-unsafe-expressions) |
