# Types

This page lists Sisu's types and the rules that relate them: what each
builtin type holds, which conversions happen without a word and which
take a method, how literals take the type expected, how generic code,
traits and trait objects are typed, the auto traits, refinements and
aliases, and what the type system leaves out on purpose. The
declarations that make new types (`struct`, `enum`, `error`, `trait`,
`impl`, `type`) are [R3](declarations.md); this page says what the
types mean.

**See also:** the tutorial's [chapter 2](../tutorial/02-values.md)
(values and types), [chapter 5](../tutorial/05-collections.md)
(collections), [chapter 6](../tutorial/06-text.md) (text),
[chapter 7](../tutorial/07-structs-and-enums.md) (structs and enums),
[chapter 8](../tutorial/08-errors.md) (absent values and errors),
[chapter 14](../tutorial/14-traits.md) (traits),
[chapter 15](../tutorial/15-generics.md) (generics),
[chapter 16](../tutorial/16-closures-and-iterators.md) (function
types), [chapter 23](../tutorial/23-contracts.md) (refinement types)
and [chapter 24](../tutorial/24-numbers.md) (numbers in depth); the
proposal's [03, Types and
generics](../../proposal/03-types-and-generics.md); the compiler's
[architecture notes](../../compiler/docs/architecture.md), "Type
checking", "Traits and generics" and "Numbers, bytes and slices";
[R7](ownership.md) for references, views and the pointer types;
[R17, Diagnostics](diagnostics.md#types) for the codes. The methods of
each type are in the standard library's reference, which `sisu doc
--std` generates ([R16](sisu-command.md#the-standard-librarys-reference)).

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. The type system is decided: static and sound, with local
inference and explicit signatures (D-002, D-003).

## Static types and local inference

Every expression has a type that the compiler knows before the program
runs, and a program whose types do not fit is not run. There is no
`any` and no dynamic type (D-002).

- The parameters and results of functions, the fields of structs, the
  members of traits, constants and everything `pub` have their types
  written out.
- Inside a body, the type of a `let` or `var` binding, of a closure's
  parameters and of a generic call's type arguments is inferred from
  the value and from the type expected where the value is used, so
  information flows both ways (D-003). Inference never crosses a
  function's boundary.
- An integer literal is an `int` and a float literal an `f64`, unless
  the type expected is another number type
  ([Integers](#integers)).
- A value whose type is not the one expected is E0300
  ([R17](diagnostics.md#e0300)).

<!-- file: examples/reference-types/inference.sisu -->
```sisu
# Types inside a body are inferred, from a value or from a later use.
def mean(values: list[f64]) -> f64:
    return sum(values) / len(values).to_f64()

def main():
    let count = 3
    let ratio = 0.5
    var names = []
    names.push("Aino")
    let small: u8 = 7
    let empty: dict[str, int] = {}
    print("{count} {ratio} {names} {small} {len(empty)}")
    print(mean([1.0, 2.5]))
```

<!-- output: examples/reference-types/inference.out -->
```text
3 0.5 ["Aino"] 7 0
1.75
```

`var names = []` has no element type until `names.push("Aino")` gives it
one. A binding whose type nothing fixes is E0324, whose note asks for a
type annotation; a `let` without a value needs one too (E0358):

<!-- file: examples/reference-types/cannot_infer.sisu -->
```sisu
# Nothing tells the compiler what the list holds.
def main():
    let names = []
    print(len(names))
```

<!-- output: examples/reference-types/cannot_infer.console -->
```console
$ sisu run cannot_infer.sisu
error[E0324]: the type of `names` cannot be inferred here; add a type annotation
  --> cannot_infer.sisu:3:9
  |
3 |     let names = []
  |         ^^^^^

error[D0006]: the program was not run: errors found (1)
```

Whether a later statement may fix a binding's type, as the `push` does,
is open (Q-50): if measurements show that few programs rely on it, a
binding's type will have to be fixed by the end of the statement that
declares it.

**Status.** In stage 0 (D-002, D-003).

## The builtin types

Every program can name these types without an import. Their names
are the prelude's, not keywords ([R1](lexical-structure.md#keywords)).

| Kind | Types | Section |
|---|---|---|
| Integers | `int`, `uint`, `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64`, `u128` | [Integers](#integers) |
| Floating point | `f64`, `f32` | [Floating point](#floating-point) |
| Truth values and characters | `bool`, `char` | [Truth values and characters](#truth-values-and-characters) |
| Text and bytes | `str`, `String`, `bytes` | [Text](#text), [Bytes](#bytes) |
| Collections | `list[T]`, `dict[K, V]`, `set[T]`, `Array[T, N]`, `Slice[T]`, `Iter[T]` | [Collections](#collections) |
| Tuples, unit and never | `(A, B)`, `()`, `Never` | [Tuples](#tuples), [Unit and Never](#unit-and-never) |
| Optional values and results | `T?`, `T ! E` | [Optional values](#optional-values), [Results and error sets](#results-and-error-sets) |
| Functions | `(A) -> B`, `pure (A) -> B` | [Function types](#function-types) |
| References and views | `ref T`, `ref mut T`, `str`, `Slice[T]`, a `view struct` | [References and views](#references-and-views) |
| Pointers of the library | `Box[T]`, `Shared[T]`, `Weak[T]`, `Cell[T]`, `Mutex[T]`, `Atomic[T]`, `Arena[T]`, `Key[T]` | [The library's pointer types](#the-librarys-pointer-types) |
| Trait objects | `dyn Trait` | [Trait objects](#trait-objects) |
| Capabilities | `System`, `Fs`, `Net`, `Clock`, `Rand`, `Env`, `Proc`, `Stdin` | [R10](capabilities.md#the-root-capabilities) |

The prelude has more types, such as `Path`, `Duration` and
`DateTime`, and the modules of the standard library more again, such as
the exact numbers of `std.num` and `std.time`'s `Instant` and `Zone`;
the library's reference documents them all.

**Status.** In stage 0.

## Integers

| Type | Width | Use |
|---|---|---|
| `int` | 64 bits, signed | the default: indices, lengths, counts |
| `uint` | 64 bits, unsigned | bit manipulation, sizes from foreign code |
| `i8`, `i16`, `i32`, `i64`, `i128` | as named, signed | layouts, protocols, foreign code |
| `u8`, `u16`, `u32`, `u64`, `u128` | as named, unsigned | layouts, protocols, foreign code |

- `int` is 64 bits on every target, also on a 32-bit one, and it is
  the type of every index and length (D-035). `i64` is another name
  for `int`, and `uint` for `u64`.
- In memory each sized type takes its width: a `u8` and an `i8` one
  byte, a `u16` and an `i16` two, a `u32`, an `i32` and an `f32` four,
  in the interpreter and in a native build alike. So a `list[u8]`, a
  `Slice[u8]` and a `bytes` hold a byte to an element, and a
  `@repr(c)` struct of them is laid out as C lays it out
  ([R14](attributes.md#exported-functions)):

<!-- file: examples/reference-types/widths.sisu -->
```sisu
# Each sized number takes its own width in memory, as in C.
from std.mem import size_of

def main():
    print("{size_of[u8]()} {size_of[i16]()} {size_of[u32]()}")
    print("{size_of[f32]()} {size_of[int]()} {size_of[f64]()}")
```

<!-- output: examples/reference-types/widths.console -->
```console
$ sisu run widths.sisu
1 2 4
4 8 8
$ sisu run --native widths.sisu
1 2 4
4 8 8
```

- An integer literal takes the integer type expected where it stands,
  or the one its suffix names (`0xFFu16`); without either it is an
  `int` ([R1](lexical-structure.md#integer-literals)).
- A literal must fit in its type: one that does not is E0374
  ([R17](diagnostics.md#e0374)), whose note gives the type's range.
- Every integer type has the bit methods `count_ones()`,
  `count_zeros()`, `leading_zeros()` and `trailing_zeros()`, each an
  `int` and the type's width for 0; `rotate_left(n)` and
  `rotate_right(n)`, by any `int` amount, taken modulo the width, a
  negative one turning the other way; and `swap_bytes()`, `to_be()` and
  `to_le()`, the bytes in reverse, big-endian and little-endian order.
  A signed type's `to_bits()` gives its two's complement as the unsigned
  type of its width, `(-1i8).to_bits()` being `255u8`, and
  `i8.from_bits(255u8)` turns it back; the bit methods of a signed type
  are those of its two's complement
  ([chapter 24](../tutorial/24-numbers.md#counting-and-turning-bits)).

<!-- file: examples/reference-types/integers.sisu -->
```sisu
# The integer types, and literals that take the type expected.
def main():
    let count = 42
    let small: u8 = 200
    let flags = 0xFFu16
    let wide: i128 = 170_141_183_460_469_231_731_687_303_715_884_105_727
    let unsigned: uint = 18_000_000_000_000_000_000
    let same: i64 = count
    print("{count} {small} {flags} {same}")
    print("{wide}")
    print("{unsigned}")
```

<!-- output: examples/reference-types/integers.out -->
```text
42 200 255 42
170141183460469231731687303715884105727
18000000000000000000
```

<!-- file: examples/reference-types/too_big.sisu -->
```sisu
# A literal that does not fit in the type expected.
def main():
    let small: u8 = 300
    print(small)
```

<!-- output: examples/reference-types/too_big.console -->
```console
$ sisu run too_big.sisu
error[E0374]: `300` does not fit in `u8`
  --> too_big.sisu:3:21
  |
3 |     let small: u8 = 300
  |                     ^^^
  = note: `u8` holds 0 to 255

error[D0006]: the program was not run: errors found (1)
```

`ptrsize` and `uptrsize` are the integers as wide as an address, for
foreign code only: the prelude's names of `i64` and `u64` on every
target Sisu builds for now, so a value of one is a value of the other
([03](../../proposal/03-types-and-generics.md#numbers)). `std.ffi`'s C
types, such as `c_size`, name C's own.

<!-- file: examples/reference-types/ptrsize.sisu -->
```sisu
# `ptrsize` and `uptrsize` are as wide as an address, for foreign code.
def main():
    let offset: ptrsize = -16
    let length: uptrsize = 4096
    let end: i64 = offset + 4096
    print("{offset} {length} {end} {uptrsize.MAX == u64.MAX}")
```

<!-- output: examples/reference-types/ptrsize.out -->
```text
-16 4096 4080 True
```

**Stage 0 differs.** 03 names no bit methods, and stage 0's take
Rust's names. Every integer type has its limits, `u8.MAX` and
`i128.MIN`, and `truncate_to[T]()`, which keeps the bits that fit,
read as `T`'s, also for a type of 128 bits
(`u64.MAX.truncate_to[i128]()` is 18446744073709551615,
`u128.MAX.truncate_to[i128]()` is -1). `to_u8()` and its
siblings are `int`'s: a sized integer narrows with `truncate_to`
alone, 01's one spelling, which stage 1's twenty-second round kept
(its R194), and a checked narrowing compares with the target's bounds
first, `if x.to_int() <= u8.MAX.to_int(): x.truncate_to[u8]()`, or goes
through `to_int()` and `int`'s `to_u8()`.

**Status.** In stage 0 (D-035).

## Overflow traps

Arithmetic whose result does not fit in its type stops the program with
a panic, in every build (D-009); so does the least value of a signed
type divided by -1. A division or a remainder by zero panics too
(R0004), as do a negative exponent of an integer (R0011) and a shift by
the type's width or more (R0005, R0093); a shift does not check the bits
it moves out. Where wrapping or saturating is wanted, the operator says
so:

| Operators | On overflow |
|---|---|
| `+`, `-`, `*`, `//`, `%`, `**` | panic ([R0003](diagnostics.md#r0003)) |
| `+%`, `-%`, `*%` | wrap around, as two's complement does |
| `+\|`, `-\|`, `*\|` | stay at the type's least or greatest value |
| `a.checked_add(b)`, `checked_sub`, `checked_mul`, `checked_pow(exp)` | give `None` |

<!-- file: examples/reference-types/wrapping.sisu -->
```sisu
# Wrapping and saturating operators, and checked additions.
from std.num import checked_add

def main():
    let level: u8 = 250
    print("{level +% 10} {level +| 10} {level -| 255} {level *% 2}")
    let largest = 9_223_372_036_854_775_807
    print("{largest +% 1}")
    print("{checked_add(largest, 1)} {level.checked_add(5)}")
```

<!-- output: examples/reference-types/wrapping.out -->
```text
4 255 0 244
-9223372036854775808
None Some(255)
```

The panic names the operation and its operands, and its exit status is
101 ([R17](diagnostics.md#r0003)):

<!-- file: examples/reference-types/overflow.sisu -->
```sisu
# Arithmetic that overflows its type stops the program.
def main():
    let level: u8 = 250
    print(level + 3)
    print(level + 10)
```

<!-- output: examples/reference-types/overflow.console -->
```console
$ sisu run overflow.sisu
253
panic: integer overflow: 250 + 10 as u8
  at main (overflow.sisu:5:11)
```

The checked methods are every integer type's, `level.checked_add(10)`
of a `u8` included ([03](../../proposal/03-types-and-generics.md#numbers)),
and `checked_pow(exp)` takes an `int` exponent of 0 or more; `std.num`
has them as functions too, `checked_add(a, b)`.

**Status.** In stage 0 (D-009).

## Widening and conversions

A number becomes a value of a wider type without a word where that type
is expected and holds every value of the narrower one (D-016, Q-13):

- `u8` to `u16`, `u32`, `u64`, `u128` and to any wider signed type;
  `i8` to `i16`, `i32`, `int`, `i128`; and so on up each family;
- `int` and `u64` to `i128`, and `u64` to `u128`;
- `f32` to `f64`.

A signed type never widens to an unsigned one, and no integer becomes a
float or a float an integer without a method. Two operands of different
integer types widen the narrower when that loses nothing, in arithmetic
and in a comparison alike, and otherwise they do not mix (E0373,
[R17](diagnostics.md#e0373)). A shift is not such an operation: its
amount is a count of bits, so `x << k` and `x >> k` have the type of
`x` whatever integer type `k` has, `int` among them, and a literal on
the left, `1 << k`, is an `int`.

<!-- file: examples/reference-types/widening.sisu -->
```sisu
# A narrower number widens where a wider type is expected.
def total(prices: list[int]) -> int:
    return sum(prices)

def main():
    let a: u8 = 200
    let b: i32 = 7
    let c: int = a
    print("{a + b} {c} {total([a, b])} {b < a}")
    let half: f32 = 0.5
    let precise: f64 = half
    let huge: u64 = 18_000_000_000_000_000_000
    let wider: i128 = huge
    print("{precise} {wider}")
```

<!-- output: examples/reference-types/widening.out -->
```text
207 200 207 True
0.5 18000000000000000000
```

Every other conversion is a method, whose name says what it may lose:

| Method | From | Gives |
|---|---|---|
| `x.into()` | an integer | the wider integer type expected; loses nothing |
| `n.to_u8()`, `n.to_i32()`, ... | an `int` | the narrower type, or `None` when the value does not fit |
| `x.to_int()` | a sized integer | an `int`; for a `u64`, `u128` or `i128`, an `int?` that is `None` beyond `int`'s range |
| `n.truncate_to[T]()` | an integer | the low bits of `n` in `T`, as two's complement wraps |
| `n.to_f64()`, `n.to_f32()` | an `int` | the nearest float |
| `x.truncate_to_int()` | a float | the whole part, toward zero; a panic (R0023) for NaN or a value beyond `int` |
| `x.round()`, `x.floor()`, `x.ceil()` | a float | a float with no fraction |
| `x.round_to[T]()` | a float | the nearest value of the integer type `T`, a half away from zero, made a `T` as `truncate_to` makes it; a panic for NaN or a value beyond `int` |
| `x.to_f32()` | an `f64` | the nearest `f32`, or an infinity beyond its range |

<!-- file: examples/reference-types/conversions.sisu -->
```sisu
# Conversions that can lose information are methods.
def main():
    let big = 300
    let small: u8 = 200
    let wide: u32 = small.into()
    print("{big.to_u8()} {small.to_int()} {wide}")
    print("{big.truncate_to[u8]()} {(-1).truncate_to[u8]()}")
    let price = 2.75
    print("{price.truncate_to_int()} {price.round()} {big.to_f64()}")
```

<!-- output: examples/reference-types/conversions.out -->
```text
None 200 200
44 255
2 3.0 300.0
```

An `int` and an `f64` in one operation is E0373, whose fix converts the
`int`:

<!-- file: examples/reference-types/mixed.sisu -->
```sisu
# An `int` and an `f64` do not mix without a conversion.
def main():
    let count = 3
    let price = 2.5
    print(count * price)
```

<!-- output: examples/reference-types/mixed.console -->
```console
$ sisu run mixed.sisu
error[E0373]: `*` cannot mix `int` and `f64`; convert one of them explicitly
  --> mixed.sisu:5:11
  |
5 |     print(count * price)
  |           ^^^^^^^^^^^^^
  = fix: convert the `int`: `.to_f64()`

error[D0006]: the program was not run: errors found (1)
```

A narrower type where a wider one is expected widens, but a wider type
where a narrower one is expected is E0300 with no fix: the conversion is
one of the methods in the table, chosen for what it does with a value
that does not fit.

**Status.** In stage 0 (D-016).

## Floating point

`f64` is the default float and `f32` the narrow one, both IEEE 754.
Floating point is strict (D-074): one source gives the same bits on
every target, at every optimisation level and with any number of
workers.

- Each operation rounds once. Nothing fuses `a * b + c` into one
  rounding and nothing reassociates a sum; `a.mul_add(b, c)` is the
  fused form, written where it is wanted.
- `sum` adds a list pairwise, in a fixed tree, which is also more
  accurate than adding from the left; a parallel sum uses the same tree.
- `/` divides floats and `//` integers: `/` on integers is E0301
  ([R17](diagnostics.md#e0301)) and `//` on floats E0372.
- Dividing by zero gives an infinity or NaN, not a panic. NaN equals
  nothing, itself included; `x.is_nan()` tests for it.
- A float has a partial order: `f64` and `f32` implement `PartialOrd`
  but not `Ord` or `Hash`, so `sorted` of floats and a float as a key of
  a `dict` or `set` are E0347, whose note for `Ord` names
  `total_order()`. `x.partial_cmp(y)` gives `None` for NaN, and
  `x.cmp(y)` panics on it (R0017), so `sort_by` with either sorts
  floats, with what NaN does written out.
- `x.total_order()` is a `TotalF64`, which is `Eq` and `Ord`: the
  numbers in order, `-0.0` before `0.0`, and NaN after every number,
  every NaN equal to every other; its field `value` is the float. So
  `sorted(xs.iter().map(x -> x.total_order()))` sorts floats
  ([chapter 24](../tutorial/24-numbers.md#nan-and-the-order-of-floats)).
- `max_by` and `min_by` take a key that is only partially ordered, a
  float among them, and panic on two keys that are not ordered, a NaN
  and another.
- An `f32` is computed as an `f32`, rounded once from the exact result,
  and printed as the shortest text that reads back as the same `f32`.

<!-- file: examples/reference-types/floats.sisu -->
```sisu
# Floating point is strict: each operation rounds once.
def main():
    let a = 0.1
    print(a + 0.2)
    print("{a * 10.0 + 0.2} {a.mul_add(10.0, 0.2)}")
    print("{(0.1 + 0.2) + 0.3} {sum([0.1, 0.2, 0.3])}")
    let third: f32 = 1.0 / 3.0
    print("{third} {third.to_f64()}")
    let nan = 0.0 / 0.0
    print("{nan == nan} {nan.is_nan()} {1.0 / 0.0}")
    var prices = [3.5, 1.0, 2.25]
    prices.sort_by((x, y) -> x.partial_cmp(y) ?? Equal)
    print(prices)
```

<!-- output: examples/reference-types/floats.out -->
```text
0.30000000000000004
1.2 1.2000000000000002
0.6000000000000001 0.6
0.33333334 0.3333333432674408
False True inf
[1.0, 2.25, 3.5]
```

<!-- file: examples/reference-types/float_keys.sisu -->
```sisu
# An `f64` cannot be the key of a `dict`.
def main():
    let names = {1.5: "bun", 2.0: "coffee"}
    print(names)
```

<!-- output: examples/reference-types/float_keys.console -->
```console
$ sisu run float_keys.sisu
error[E0347]: `f64` does not implement `Hash`
  --> float_keys.sisu:3:17
  |
3 |     let names = {1.5: "bun", 2.0: "coffee"}
  |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

`@float(fast)` on a function permits contraction and reassociation
inside it, which strict floating point satisfies, so it may be no
faster ([R14](attributes.md#fast-floating-point)).

**Stage 0 differs.** Stage 0 keeps a `@float(fast)` function strict,
which the permission allows. `TotalF64` is `Ord` but not `Hash`, so it
sorts floats and is not yet the key of a `dict` that
[03](../../proposal/03-types-and-generics.md#numbers) makes it.

**Status.** In stage 0 (D-074), with the differences noted.

## Exact numbers

`std.num` has four types for arithmetic that must not round (D-076):

| Type | Holds | Arithmetic |
|---|---|---|
| `BigInt` | an integer of any size | `+`, `-`, `*`, `%`, unary `-`; `floor_div` and `divmod` |
| `Ratio` | an exact fraction of two `int`s, in lowest terms | `+`, `-`, `*`, `/`, unary `-` |
| `BigRatio` | an exact fraction of two `BigInt`s, in lowest terms | `+`, `-`, `*`, `/`, unary `-`; `floor`, `to_ratio`, `to_f64` |
| `Decimal` | a base-10 number, held as an integer and a count of places | `+`, `-`, `*` exactly; `/` exactly, giving a `BigRatio`; `div` and `round` with a named `Rounding` |

They take ordinary literals with no suffix, through `FromLiteral`
([Literals that take the type expected](#literals-that-take-the-type-expected)):
a literal reaches a `Decimal` as written, so `0.1` is exactly one tenth
and `3.50` keeps its two places, and a literal beside one of them in
arithmetic takes its type, as `3` and `0.03` do in `price * 3 + 0.03`.
Nothing rounds without being told how: `Rounding` names seven modes,
`HalfEven`, `HalfUp`, `HalfDown`, `Down`, `Up`, `Floor` and `Ceiling`,
and there is no hidden precision (Q-53). `int` stays 64 bits wide and
traps; these types are where arithmetic grows instead.

<!-- file: examples/reference-types/exact.sisu -->
```sisu
# The exact types of `std.num` take ordinary literals.
from std.num import BigInt, Decimal, Ratio, Rounding

def main():
    let n: BigInt = 9_000_000_000
    print(n * n * n)
    let tenth: Decimal = 0.1
    print("{tenth + tenth + tenth} {0.1 + 0.1 + 0.1}")
    let price: Decimal = 19.99
    print(price * 3 + 0.03)
    print(price.div(Decimal.of(3), 2, Rounding.HalfEven))
    print(price / 3)
    print(Ratio.of(1, 3) + Ratio.of(1, 6))
```

<!-- output: examples/reference-types/exact.out -->
```text
729000000000000000000000000000
0.3 0.30000000000000004
60.00
6.66
1999/300
1/2
```

**Stage 0 differs.** A `BigInt` literal must fit in an `int` (E0374);
[03](../../proposal/03-types-and-generics.md#numbers) takes a literal of
any length and checks it at compile time, and stage 0 reads a longer one
with `BigInt.parse`, which E0374's note names. 03 makes a `Decimal`'s
quotient a `Ratio`; stage 0's `Ratio` holds two `int`s, so `a / b` of
`Decimal`s gives a `BigRatio`, which holds any quotient exactly (`1.5 /
0.4` is `15/4`), and `a.to_big_ratio()` makes the exact fraction of
one. Whether `Ratio` should be one type of any size instead is Q-82.

**Status.** In stage 0 (D-076), with the differences noted.

## Truth values and characters

`bool` has the two values `True` and `False`, and only a `bool` is a
condition: Sisu has no truthiness, so `if xs:` is E0302
([R17](diagnostics.md#e0302)) and the test is written out, as `not
xs.is_empty()`. `and`, `or` and `not` take and give `bool`s (D-025).

`char` is one Unicode scalar value: a code point that is not a
surrogate. `c.code()` gives its number and `char.from_code(n)` a `char?`
back; the character classes (`is_alphabetic`, `is_digit`,
`is_whitespace`, ...) follow Unicode's tables. A `char` is not a short
string: `'a'` and `"a"` have different types, and a single-quoted text
of more than one character is E0016
([R17](diagnostics.md#e0016)).

**Status.** In stage 0.

## Text

Text is UTF-8, always valid, in two types (D-034):

- `str` is a view of text that lives elsewhere: a literal, a part of a
  `String`, a piece of a file's contents. It is `Copy`, and as a view it
  cannot outlive what it views ([R7](ownership.md#views)).
- `String` owns its text and can grow (`push`, `push_char`). A `String`
  is accepted wherever a `str` is expected, as a view of itself; a `str`
  becomes a `String` by `.to_string()` or `String.new(s)`, which copy
  the text.

Text has no index by position, since a position in characters costs a
walk from the start and a position in bytes can fall inside a
character. These say which they count:

| Expression | Gives |
|---|---|
| `len(s)` | the number of characters (scalar values) |
| `s.byte_len()` | the number of bytes of the encoding |
| `s.chars()` | an iterator over the characters |
| `s.bytes()` | an iterator over the bytes, as `u8`s |
| `s.byte_slice(a, b)` | the text from byte `a` to byte `b`, a view; a panic ([R0115](diagnostics.md#r0115)) when either is inside a character |

An interpolation makes a `String`, and so do the methods that build new
text (`to_uppercase`, `replace`, `repeat`); the methods that cut text
(`split`, `trim`, `lines`, `strip_prefix`) give views.

<!-- file: examples/reference-types/text.sisu -->
```sisu
# `str` views text, `String` owns it, and `char` is one scalar value.
def shout(text: str) -> String:
    return text.to_uppercase()

def main():
    let word: str = "sisu"
    var owned: String = word.to_string()
    owned.push("!")
    owned.push_char('?')
    print(shout(owned))
    let greeting = "hyvää"
    print("{len(greeting)} {greeting.byte_len()}")
    print(greeting.byte_slice(0, 3))
    for c in "äö".chars():
        print("{c} {c.code()}")
```

<!-- output: examples/reference-types/text.out -->
```text
SISU!?
5 7
hyv
ä 228
ö 246
```

An index into text is E0345, whose message names the three ways in:

<!-- file: examples/reference-types/text_index.sisu -->
```sisu
# Text has no index by position.
def main():
    let word = "hyvää"
    print(word[3])
```

<!-- output: examples/reference-types/text_index.console -->
```console
$ sisu run text_index.sisu
error[E0345]: `str` cannot be indexed by position; use `.chars()`, `.bytes()` or `.byte_slice(a, b)`
  --> text_index.sisu:4:11
  |
4 |     print(word[3])
  |           ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

`s.graphemes()` goes through the characters as a reader sees them,
Unicode's extended graphemes (UAX #29, every rule from GB3 to GB999),
each a `str` that views `s`: a base with the marks after it, a
spacing mark with its base, a `Prepend` character with what follows
it, `\r\n`, a Hangul syllable written in jamo, a flag of two regional
indicators (paired from the start of their run), an Indic conjunct
joined across its virama, `"\u{915}\u{94d}\u{937}"`, and pictographs
joined by U+200D are one grapheme each; U+200D after a letter joins
nothing, so `"a\u{200d}b"` is two, and a control character is one of
its own. The properties come from Unicode 15.1.0's own data files, and
`graphemes()` splits all 1,187 lines of Unicode's
`GraphemeBreakTest.txt` as the file says. The
normal forms of UAX #15 are `s.nfc()`, `s.nfd()`, `s.nfkc()` and
`s.nfkd()`, each a new `String`: the canonical or the compatibility
decomposition, its marks in canonical order, composed again for `nfc`
and `nfkc`. Both are methods of traits of `std.string`, `Graphemes`
and `Normalize`, implemented for `str` and called after `from
std.string import Graphemes` (or `Normalize`); their tables come from
Unicode 15.1.0 ([chapter 6](../tutorial/06-text.md#characters-as-a-reader-sees-them)
shows both).

**Stage 0 differs.** [03](../../proposal/03-types-and-generics.md#strings-and-text)
has `s.graphemes()` one method away from `chars()`, with no import.

**Status.** In stage 0 (D-034), with the differences noted.

## Bytes

`bytes` is an immutable owned sequence of bytes: the type of a byte
literal `b"..."`, of what a connection reads, and of what
`s.encode()` gives for a text.

- `b[i]` is a `u8`, `b[i:j]` a new `bytes` holding a copy of the run,
  `len(b)` the number of bytes, `a + b` the two joined, and `for x in b`
  iterates over the bytes.
- A `bytes` is accepted where a `Slice[u8]` is expected, as a view of
  itself.
- `b.decode()` gives the text the bytes encode, or the error
  `InvalidUtf8` with the position of the first byte that is not UTF-8.

<!-- file: examples/reference-types/bytes.sisu -->
```sisu
# `bytes` holds any bytes; `decode` checks that they are UTF-8.
def main():
    let raw: bytes = b"hei\xff"
    print("{raw[0]} {len(raw)} {raw[0:3]}")
    print(raw.decode().is_err())
    let text = "hei".encode()
    print(text.decode())
```

<!-- output: examples/reference-types/bytes.out -->
```text
104 4 b"hei"
True
Ok("hei")
```

**Status.** In stage 0.

## Collections

| Type | Holds | Literal |
|---|---|---|
| `list[T]` | a growable sequence | `[a, b]`, `[]` |
| `dict[K, V]` | a map from keys to values, iterated in insertion order | `{k: v}`, `{}` with a type expected |
| `set[T]` | values without repeats, iterated in insertion order | `{a, b}` |
| `Array[T, N]` | exactly `N` values; `N` is part of the type | `[a, b, c]` of `N` elements, with an `Array` expected; `Array.filled(value)` |
| `Slice[T]` | a read-only view of a run of values | a `list[T]` or a slice of one, where a `Slice` is expected |
| `Iter[T]` | an iterator that yields `T`s | `xs.iter()`, a generator, an adapter |

- A `dict` and a `set` keep the order in which their keys were first
  inserted, so iterating one gives the same order on every run and
  every target (D-074, Q-42).
- A key of a `dict` and an element of a `set` implement `Hash` and `Eq`
  (E0347 otherwise).
- An `Array[T, N]` is read and written by index with the same bounds
  check as a list (R0002), and nothing adds or removes an element.
  `Array[int, 3]` and `Array[int, 4]` are different types, and a list
  literal of another length than `N`, or `[]`, is no `Array[T, N]`
  (E0300, expected `Array[int, 3]`, found `list[int]`).
- `Slice[T]` is a view ([R7](ownership.md#views)): a function that
  takes one reads a list, a part of a list or a `bytes` alike.
- `Iterable[T]`, as the type of a parameter, accepts a `list`, `set`,
  `dict` or `Iter` whose items are `T`.

<!-- file: examples/reference-types/collections.sisu -->
```sisu
# The builtin collections and a `Slice` that views a list.
def total(values: Slice[int]) -> int:
    var sum = 0
    for v in values:
        sum += v
    return sum

def main():
    let scores: list[int] = [7, 9, 4]
    let ages: dict[String, int] = {"Eero": 9, "Aino": 7}
    let seen: set[char] = {'b', 'a'}
    var counts: Array[int, 3] = [0, 0, 0]
    counts[^1] = 5
    print("{scores} {ages} {seen} {counts}")
    print("{total(scores)} {total(scores[1:])}")
```

<!-- output: examples/reference-types/collections.out -->
```text
[7, 9, 4] {"Eero": 9, "Aino": 7} {'b', 'a'} [0, 0, 5]
20 13
```

**Stage 0 differs.** A key type with an `Eq` or `Hash` impl of its own, rather
than a derived one, is E0398 in stage 0, which hashes keys by their
structure. Each collection takes its allocator as a last parameter,
D-060's `list[T, A: Allocator = Global]`, `dict[K, V, A]`, `set[T, A]`
and `String[A]`, which a program leaves out: `list[int]` is `list[int,
Global]`, and a collection on another allocator has only
`with_allocator` and the `try_` growths until the allocators' later
steps ([R7](ownership.md#allocators)).

**Status.** In stage 0, with the differences noted.

## Literals that take the type expected

A literal whose expected type implements `FromLiteral[L]` becomes a
value of that type, made at the literal by the type's `from_literal`.
`L` is `int` for an integer literal, `f64` for a float literal and `str`
for a string literal without interpolation. A type is
expected of a literal where one is given, as a binding's, a parameter's
or a `return`'s, and beside a value of a type that is not a number in an
arithmetic operator, `f * 2` with `f` a `BigInt`, when the operator has
no impl for the literal as it is (an `impl Mul[int] for Vector` keeps
the `int`). This is typing, not conversion: it applies to literals
only, never to values already made.

| Type | Takes |
|---|---|
| `String` | a string literal |
| `Path` | a string literal: `let home: Path = "/home/aino"` |
| `Url` (`std.net`) | a string literal, read by `Url.parse`; one that is no URL stops the program with a panic that names the form |
| `BigInt` | an integer literal |
| `Decimal` | an integer or a float literal, read from its digits |
| a type of your own | whatever `impl FromLiteral[L]` it has |

<!-- file: examples/reference-types/literal_types.sisu -->
```sisu
# A literal becomes a value of any type that implements `FromLiteral`.
from std.net import Url

struct Celsius:
    degrees: f64

impl FromLiteral[f64] for Celsius:
    def from_literal(literal: f64) -> Celsius:
        return Celsius(literal)

def main():
    let home: Path = "/home/aino"
    let name: String = "Aino"
    let warm: Celsius = 21.5
    let site: Url = "https://example.com/sauna"
    print("{home} {name} {warm.degrees} {site.host}")
```

<!-- output: examples/reference-types/literal_types.out -->
```text
/home/aino Aino 21.5 example.com
```

A list literal is an `Array` where one is expected
([Collections](#collections)), without `FromLiteral`: the compiler
fills it.

**Status.** In stage 0.

## Tuples

`(A, B)` is a tuple of an `A` and a `B`, written `(a, b)`; its elements
are `t.0`, `t.1` and so on (E0363 for one it does not have). A tuple of
`Copy` values is `Copy`. A function's result may be a tuple with
labelled elements, `-> (quotient: int, remainder: int)`, whose elements
are read by label or by position. A tuple unpacks into names in a `let`,
with as many names as elements (E0364).

<!-- file: examples/reference-types/tuples.sisu -->
```sisu
# Tuples, labelled results, unit and `Never`.
def divide(a: int, b: int) -> (quotient: int, remainder: int):
    return (a // b, a % b)

def fail(reason: str) -> Never:
    panic(reason)

def main():
    let pair = (3, "kolme")
    print("{pair.0} {pair.1} {pair}")
    let result = divide(17, 5)
    print("{result.quotient} {result.remainder} {result.0}")
    let q, r = divide(9, 4)
    let n: int = if q > 0: q else: fail("no quotient")
    print("{q} {r} {n} {()}")
```

<!-- output: examples/reference-types/tuples.out -->
```text
3 kolme (3, "kolme")
3 2 3
2 1 2 ()
```

**Status.** In stage 0.

## Unit and Never

`()` is the unit type, whose one value is also written `()`: the result
type of a function without `->`. `Never` is the type of an expression
that never gives a value: `panic(...)`, `return`, `break`, `continue`, a
call of a function that returns `Never`, and `while True:` with no
`break`. A `Never` is accepted wherever any type is expected, so `if q >
0: q else: fail("...")` in [Tuples](#tuples) is an `int`.

**Status.** In stage 0.

## Optional values

`T?` is `Option[T]`, an ordinary enum with the variants `Some(T)` and
`None`; the formatter writes it `T?`. It is how Sisu says that a value
may be absent, since no type has a `null`. A value becomes an optional
one only in `Some`: `let n: int? = 5` is E0300. `?.`, `??`, `if let` and
`let ... else` take an optional value apart
([R4](expressions.md#optional-values-and-results)).

<!-- file: examples/reference-types/optional.sisu -->
```sisu
# `T?` is `Option[T]`: `Some` with a value, or `None`.
def find(names: list[str], wanted: str) -> int?:
    for i, name in enumerate(names):
        if name == wanted:
            return Some(i)
    return None

def main():
    let names = ["Aino", "Eero"]
    let missing = find(names, "Roni")
    print("{find(names, "Eero")} {missing} {missing ?? -1}")
```

<!-- output: examples/reference-types/optional.out -->
```text
Some(1) None -1
```

The proposal makes `Option[ref T]`, `Option[Box[T]]` and an option of
an enum with spare discriminants the same size as their payload; no
program can see the difference except through the size of a type.

**Status.** In stage 0; the layout is designed
([03](../../proposal/03-types-and-generics.md#option)).

## Results and error sets

`T ! E` is `Result[T, E]`, an enum with the variants `Ok(T)` and
`Err(E)`: the result of a function that can fail, read "T or error E".
`E` is an error set:

- An `error` declaration makes a named error set, whose variants may
  carry fields. `!` followed by something else is E0212.
- `{A, B}` is the union of two sets, and stands only after `!` (E0227).
- A set is a subtype of every set that holds all its variants: an
  `IoError` is accepted where `{IoError, ParseError}` is expected,
  with no conversion. This is one of the few subtypes of the language
  ([Subtypes and coercions](#subtypes-and-coercions)).
- `T !` with no set infers the set from the body: the union of every
  error it returns or passes on with `?`. It stands only as a function's
  result type (E0226).

<!-- file: examples/reference-types/error_sets.sisu -->
```sisu
# An error set is a subtype of every union that holds it.
error ParseError:
    Empty
    NotANumber(text: String)

error RangeError:
    TooLarge(value: int)

def parse(text: str) -> int ! ParseError:
    if text == "":
        return Err(Empty)
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def check(n: int) -> int ! RangeError:
    return if n > 100: Err(TooLarge(n)) else: Ok(n)

def read(text: str) -> int ! {ParseError, RangeError}:
    return check(parse(text)?)

def main():
    for text in ["42", "", "x", "500"]:
        print(read(text))
```

<!-- output: examples/reference-types/error_sets.out -->
```text
Ok(42)
Err(Empty)
Err(NotANumber(text="x"))
Err(TooLarge(value=500))
```

`check(...)` gives an `int ! RangeError`, and `read` returns it as an
`int ! {ParseError, RangeError}`. How errors propagate, are handled and
reach `main` is [R8](errors.md).

**Status.** In stage 0.

## Structs and enums

Types are nominal: two structs with the same fields are two types, and
a type implements a trait only by an `impl`. A `struct` is a product of
named fields; an `enum` is a sum of variants, each a unit, a tuple or a
record. A variant is named under its enum (`Expr.Add`) and may be named
alone wherever the type expected is the enum, as in a `match` on it
(E0211 where two enums share the name). There is no inheritance and no
subtyping between structs.

An enum may contain itself: the compiler puts the variants that would
otherwise be infinitely large behind a pointer, so a recursive enum
needs no `Box` (D-043).

<!-- file: examples/reference-types/recursive.sisu -->
```sisu
# A recursive enum needs no `Box`: the compiler adds the indirection.
enum Expr:
    Number(int)
    Add(Expr, Expr)
    Neg(Expr)

def eval(e: Expr) -> int:
    return match e:
        case Number(n): n
        case Add(a, b): eval(a) + eval(b)
        case Neg(inner): -eval(inner)

def main():
    print(eval(Expr.Add(Number(2), Expr.Neg(Number(5)))))
```

<!-- output: examples/reference-types/recursive.out -->
```text
-3
```

An enum may be `@repr(u8)`, for foreign code, and may declare explicit
discriminants; an enum with only unit variants is a C enum.

Stage 0 reads an explicit discriminant and refuses it until the layouts
of `@repr` enums arrive:

<!-- file: examples/reference-types/discriminant.sisu -->
```sisu
# An explicit discriminant gives a variant its number, for foreign code.
@repr(u8)
enum Light:
    Red = 1
    Green = 2

def main():
    print("{Light.Red:?}")
```

<!-- output: examples/reference-types/discriminant.console -->
```console
$ sisu run discriminant.sisu
error[E0900]: an explicit discriminant of an enum's variant is not implemented in stage 0 yet; it arrives with the layouts of `@repr` enums (03, 12)
  --> discriminant.sisu:4:9
  |
4 |     Red = 1
  |         ^^^

error[E0900]: an explicit discriminant of an enum's variant is not implemented in stage 0 yet; it arrives with the layouts of `@repr` enums (03, 12)
  --> discriminant.sisu:5:11
  |
5 |     Green = 2
  |           ^^^

error[D0006]: the program was not run: errors found (2)
```

**Stage 0 differs.** Stage 0 keeps `@repr(u8)` on an enum, which names
the layout of its tag; nothing a safe program does can observe that
layout until the layouts of `@repr` enums are lowered. An explicit
discriminant, `Red = 1`, is E0900
([R17](diagnostics.md#e0900)), the code of what stage 0 does not
implement yet.

**Status.** In stage 0 (D-043); `@repr` and discriminants are designed
([12](../../proposal/12-interop-and-targets.md)).

## Function types

`(A, B) -> C` is the type of a function or closure that takes an `A`
and a `B` and returns a `C`; a fallible one returns `C ! E`, and one
that returns nothing has the type `(A) -> ()`. A function named without
a call is a value of its function type, and so is a closure
([R4](expressions.md#closures)).

- How a closure captures, by borrowing or by copy, is not part of its
  type; the checker infers it for each closure ([R7](ownership.md#closures-and-their-captures)).
- A function that returns a borrow of its parameter cannot be a value
  (E0352), since a function value takes its arguments by value.
- A value that is not a function cannot be called (E0309), and a
  closure takes the number of parameters its type says (E0353).

<!-- file: examples/reference-types/function_types.sisu -->
```sisu
# Functions and closures are values of function types.
error Overflow:
    TooBig

def twice(f: (int) -> int, x: int) -> int:
    return f(f(x))

def triple(n: int) -> int:
    return n * 3

def main():
    let step: (int) -> int = n -> n + 1
    let by_name = triple
    print("{twice(step, 5)} {twice(by_name, 2)} {twice(it * it, 3)}")
    let add: (int, int) -> int ! Overflow = (a, b) -> Ok(a + b)
    print(add(1, 2))
```

<!-- output: examples/reference-types/function_types.out -->
```text
7 18 81
Ok(3)
```

**Status.** In stage 0.

## Pure function types

`pure def` declares a function that is deterministic: it uses no
capability, no shared mutation and nothing that observes the schedule,
and calls only pure functions, so it gives the same result for the same
arguments on every run (D-074). Inside an `unsafe` block, a call of a
function that is not pure is its author's assertion that it behaves as
a pure one (D-085). `pure` is also part of function types:

- `(A) -> B` admits any function, and `pure (A) -> B` only one that the
  checker has proved pure. A `pure (A) -> B` is accepted where an
  `(A) -> B` is expected.
- A closure is pure when its body is; that is inferred.
- A `pure` function that takes a function parameter types it `pure`,
  since a call of a plain function value from a pure one is E0392.
- A `pure` function that calls one that is not is E0333; one that uses
  a capability is E0334; a function passed where a `pure` one is
  required but that is not pure is E0335.

<!-- file: examples/reference-types/pure_types.sisu -->
```sisu
# A `pure` function type admits only functions proved pure.
pure def apply_twice(f: pure (int) -> int, x: int) -> int:
    return f(f(x))

pure def double(n: int) -> int:
    return n * 2

def apply(f: (int) -> int, x: int) -> int:
    return f(x)

def main():
    print(apply_twice(n -> n * 3, 2))
    print(apply_twice(double, 5))
    print(apply(double, 4))
```

<!-- output: examples/reference-types/pure_types.out -->
```text
18
20
8
```

<!-- file: examples/reference-types/impure_argument.sisu -->
```sisu
# A function that prints is not pure.
pure def apply_twice(f: pure (int) -> int, x: int) -> int:
    return f(f(x))

def noisy(n: int) -> int:
    print("called with {n}")
    return n * 3

def main():
    print(apply_twice(noisy, 2))
```

<!-- output: examples/reference-types/impure_argument.console -->
```console
$ sisu run impure_argument.sisu
error[E0335]: this function is passed where a pure function is required, but it calls `print`, which is not pure
  --> impure_argument.sisu:10:23
   |
10 |     print(apply_twice(noisy, 2))
   |                       ^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0335 names the operation responsible, here `print`, as
[03](../../proposal/03-types-and-generics.md#function-types) asks,
following the program's own functions to the first call that is not
pure; when the way is longer than one call, a note gives it.

**Status.** In stage 0 (D-074).

## References and views

`ref T` is a shared borrow of a `T` and `ref mut T` a borrow that may
change it. References are second-class: a local, a parameter and a
result may be one, a plain struct, an enum, a collection or a closure
that escapes may not, and no reference has a lifetime name. A result
that borrows comes from a parameter, named in a `from` clause where more
than one could lend it (`-> str from a, b`). `ref str` is `str`, since a
shared borrow of a view is the view.

A view is a value that holds a borrow: `str`, `Slice[T]`, `Lent[T]`, an
iterator over a borrowed collection, a `dyn Trait` behind `ref`, and a
`view struct`, whose fields may be references and views. A collection,
tuple or option that holds a view is a view too. The rules of both are
[R7](ownership.md#references) and [R7](ownership.md#views).

**Status.** In stage 0 (D-004).

## The library's pointer types

The standard library's types for ownership that one owner and one frame
cannot express, each visible in the program's types:

| Type | For |
|---|---|
| `Box[T]` | a value in an allocation of its own |
| `Shared[T]`, `Weak[T]` | several owners of one value, and a reference that does not keep it alive |
| `Cell[T]` | a value that changes behind shared owners, in one task |
| `Mutex[T]`, `RwLock[T]`, `Atomic[T]` | a value that changes behind owners in several tasks |
| `Arena[T]`, `Key[T]` | the nodes of a graph, named by checked keys |

`x.f` and a method call look through a `Box`, a `Shared` and a mutex
guard to the value inside. Their rules and costs are
[R7](ownership.md#sharing-cells-and-graphs). The raw pointers of
`std.mem`, `ptr[T]` and `ptr mut T`, are for `unsafe` code
([R7](ownership.md#unsafe-code-and-raw-pointers)).

**Status.** In stage 0 (D-005); the raw pointers are arriving.

## Traits are implemented by name

A type implements a trait only where an `impl Trait for Type` says so,
never because it happens to have the trait's methods. A type that has
every method of a trait but no `impl` gets E0347 with a fix that writes
the `impl`:

<!-- file: examples/reference-types/nominal.sisu -->
```sisu
# A type implements a trait only through an `impl`.
trait Priced:
    def price(self) -> int

struct Coffee:
    cups: int

    def price(self) -> int:
        return 350 * self.cups

def total[T: Priced](items: list[T]) -> int:
    var sum = 0
    for item in items:
        sum += item.price()
    return sum

def main():
    print(total([Coffee(1), Coffee(2)]))
```

<!-- output: examples/reference-types/nominal.console -->
```console
$ sisu run nominal.sisu
error[E0347]: `Coffee` does not implement `Priced`
  --> nominal.sisu:18:11
   |
18 |     print(total([Coffee(1), Coffee(2)]))
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: `Coffee` has every method of `Priced`; add `impl Priced for Coffee`

error[D0006]: the program was not run: errors found (1)
```

What a trait and an impl may declare is [R3](declarations.md#traits);
the traits of the prelude (`Display`, `Debug`, `Eq`,
`PartialOrd`, `Ord`, `Hash`, `Default`, `Len`, `Iterator`, the operator
traits, `FromLiteral`) are in the library's reference, and which of
them `@derive` writes is [R14](attributes.md#deriving-traits).

**Status.** In stage 0 (D-013).

## One impl for each type and trait

A type implements a trait at most once in the whole program, so
`dict[K, V]` hashes a key the same way everywhere (D-013).

- Two impls of one trait whose types could be the same overlap, which
  is E0235 ([R17](diagnostics.md#e0235)) at the second. A `where`
  clause never chooses between impls.
- The orphan rule: an impl stands in the package that declares the
  trait, in the package that declares the type, or in a package that
  declares one of the trait's type arguments, E0236 otherwise, whose
  note says which type would make the impl the package's. So the
  package that declares `Vector` may write `impl Mul[Vector] for f64`,
  which gives its users `2.0 * v` ([chapter 14](../tutorial/14-traits.md#one-impl-for-each-type-and-trait)),
  and no other package can write it. A type parameter of the impl may
  not stand bare before that type: `impl[T] Mul[Vector] for T` is
  E0236, since another package's impl of `Mul[Vector]` for its own type
  would overlap it. The root package of a build, which nothing depends
  on, may implement any trait for any type (Q-11).
- A trait that only the standard library implements, such as `Num`, is
  E0240 for any other type.
- To give a trait to another package's type, wrap the type in a struct
  of your own ([Type aliases and newtypes](#type-aliases-and-newtypes)).

<!-- file: examples/reference-types/overlap.sisu -->
```sisu
# A type implements a trait at most once.
struct Money:
    cents: int

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents} cents")

impl Display for Money:
    def fmt(self, mut f: Formatter):
        f.write("{self.cents // 100} euros")

def main():
    print(Money(250))
```

<!-- output: examples/reference-types/overlap.console -->
```console
$ sisu run overlap.sisu
error[E0235]: `Display` is implemented for `Money` more than once; a type implements a trait at most once
  --> overlap.sisu:9:1
  |
9 | impl Display for Money:
  | ^^^^^^^^^^^^^^^^^^^^^^
  |
5 | impl Display for Money:
  | ---------------------- the other impl is here

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0 (D-013).

## Generic types and functions

Functions, structs, enums, traits and impls take type parameters in
square brackets, each with optional bounds:

- `def largest[T: Ord + Copy](xs: list[T]) -> T` bounds `T` by two
  traits; a `where` clause after the result says the same, as in
  `where T: Display`, and may bound an associated type
  (`I.Item: Display`).
- A generic body is checked once, where it is written, against its
  bounds: an operation or a method that no bound provides is an error in
  the body, never at a use. A type argument that lacks a bound is E0347
  ([R17](diagnostics.md#e0347)) at the use.
- A call infers its type arguments from its arguments and from the
  type expected; `Grid[3, 2](...)` and `f[int](...)` give them
  explicitly. The wrong number of type arguments is E0208, type
  arguments for something that takes none E0349, and a generic type
  written in a signature without them, as `xs: list`, E0375.

<!-- file: examples/reference-types/generics.sisu -->
```sisu
# Type parameters with bounds, a default and a constant.
def largest[T: Ord + Copy](xs: list[T]) -> T:
    var best = xs[0]
    for x in xs:
        if x > best:
            best = x
    return best

def show_all[T](xs: list[T]) -> String where T: Display:
    return xs.iter().map(x -> "{x}").join(", ")

struct Labelled[T, L = str]:
    value: T
    label: L

struct Grid[const W: int, const H: int]:
    cells: Array[int, W * H]

def main():
    print("{largest([3, 9, 4])} {largest(['k', 'a'])}")
    print(show_all(["a", "b"]))
    let seven: Labelled[int] = Labelled(7, "seven")
    let grid = Grid[3, 2](Array.filled(0))
    print("{seven.value} {seven.label} {len(grid.cells)}")
```

<!-- output: examples/reference-types/generics.out -->
```text
9 k
a, b
7 seven 6
```

The comparison in this body needs an order that nothing gives `T`, and
the fix bounds `T`:

<!-- file: examples/reference-types/unbounded.sisu -->
```sisu
# A generic body may use only what the bounds of its parameters give.
def bigger[T](a: T, b: T) -> bool:
    return a > b

def main():
    print(bigger(3, 2))
```

<!-- output: examples/reference-types/unbounded.console -->
```console
$ sisu run unbounded.sisu
error[E0368]: `T` has no order, so it cannot be compared with `>` or sorted
  --> unbounded.sisu:3:12
  |
3 |     return a > b
  |            ^^^^^
  = fix: bound `T` by `PartialOrd`: `: PartialOrd`

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0 (D-017).

## Defaulted type parameters

A type's or a trait's type parameter may have a default, written after
its bound: `struct Labelled[T, L = str]` in
[Generic types and functions](#generic-types-and-functions). A use that
leaves the parameter out gets the default, so `Labelled[int]` is
`Labelled[int, str]`, in signatures and `impl` headers alike.

- A parameter without a default cannot follow one with a default
  (E0245).
- The type parameters of a function or an `impl` take no defaults
  (E0246).

The standard collections take their allocator this way, D-060's
`list[T, A: Allocator = Global]`, so that a list in a borrowed region
is `list[T, ref Region]` and every other list leaves the parameter out.

**Stage 0 differs.** A collection's type at `Global` is the one it had
before the parameter, in the SIR too. `list.with_allocator(ref
region)` builds a `list[int, ref Region]`, and `try_push` grows it, but
the collections' other impls are for `Global` only, so it has no `Len`
(E0347), and the runtime does not run it yet, until the allocators'
later steps ([R7](ownership.md#allocators)).

**Status.** In stage 0 (D-060 leaning), with the differences noted.

## Constant parameters

A parameter `const N: int` of a struct, enum, impl or function takes a
constant instead of a type, and the constant is part of the type:
`Array[int, 3]` and `Array[int, 4]` differ. The argument is an integer
literal, another `const` parameter, a constant of the program, or an
expression of them with the operators `+ - * // % ** << >> & | ^` and
unary `-` and `~`, computed where the type is written, as `Array[int, W
* H]` in [Generic types and functions](#generic-types-and-functions).
In the body, `N` is an `int`.

- A type where a constant goes is E0262, and a constant where a type
  goes E0263.
- A call, a local or a `comptime` expression as the argument is E0264,
  and a value that is not an `int` E0265; an operation that has no
  value, such as an overflow, is the E0602 its run would panic with.
- `W * H` and `H * W` are different expressions, so an `Array[T, W *
  H]` is not an `Array[T, H * W]` (E0300).

**Status.** In stage 0 (D-017).

## Monomorphisation

A generic function is compiled once for each list of type arguments a
program gives it, so a call through a bound costs what a call of the
concrete function costs (D-017). The interpreter runs generic code once,
with the type arguments as values, and the native backend compiles an
instance for each list of them ([R16](sisu-command.md#compiling-and-running-natively---native-and---release)).

**Stage 0 differs.** [03](../../proposal/03-types-and-generics.md#dispatch)
shares the instances whose types have the same layout and budgets them
per package, reported by `sisu build --explain-size`; stage 0 has
neither the sharing nor that report, and its `--explain-size` gives the
parts of a native executable instead, in which the standard library's
instances are counted with the program's own code
([R16](sisu-command.md#sisu-build)).

**Status.** In stage 0 (D-017), except sharing and the report of
instances.

## Trait objects

`dyn Trait` is a value of some type that implements `Trait`, which
carries its type, so a call of a method through it is dispatched while
the program runs. Its size is not known, so it lives behind `ref` or in
a `Box` (E0395 for a bare one, [R17](diagnostics.md#e0395)), and a `ref
T` or a `Box[T]` becomes one where it is expected.

- A trait can be `dyn` when every method takes `self` or `mut self`,
  has no type parameters of its own and mentions `Self` nowhere but its
  receiver, and when it has no constant. Using another trait as `dyn`
  is E0241, which names the member, a method or a constant, and the
  reason (D-017).
- A `dyn Trait` implements the trait's supertraits.
- A `dyn` owns its value, so a view cannot become one (E0396).
- A trait used as a type without `dyn` is E0231
  ([R17](diagnostics.md#e0231)).

<!-- file: examples/reference-types/trait_objects.sisu -->
```sisu
# `dyn Shape` holds a value of any type that implements `Shape`.
trait Shape:
    def area(self) -> f64

struct Square:
    side: f64

struct Circle:
    radius: f64

impl Shape for Square:
    def area(self) -> f64:
        return self.side * self.side

impl Shape for Circle:
    def area(self) -> f64:
        return 3.0 * self.radius * self.radius

def describe(shape: ref dyn Shape) -> String:
    return "an area of {shape.area()}"

def main():
    let shapes: list[Box[dyn Shape]] = [
        Box(Square(2.0)),
        Box(Circle(1.0)),
    ]
    for shape in shapes:
        print(shape.area())
    let square = Square(3.0)
    print(describe(ref square))
```

<!-- output: examples/reference-types/trait_objects.out -->
```text
4.0
3.0
an area of 9.0
```

<!-- file: examples/reference-types/not_object_safe.sisu -->
```sisu
# A method that names `Self` cannot be called through `dyn`.
trait Shape:
    def area(self) -> f64
    def larger(self, other: Self) -> bool

def show(shape: ref dyn Shape) -> f64:
    return shape.area()

def main():
    print("shapes")
```

<!-- output: examples/reference-types/not_object_safe.console -->
```console
$ sisu run not_object_safe.sisu
error[E0241]: `Shape` cannot be used as `dyn Shape`: its member `larger` cannot be used through it, because its signature mentions `Self` besides `self`
  --> not_object_safe.sisu:6:21
  |
6 | def show(shape: ref dyn Shape) -> f64:
  |                     ^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

A closed set of variants is an enum and an open set of behaviours a
trait; [chapter 15](../tutorial/15-generics.md#an-enum-or-a-trait)
compares the two.

**Status.** In stage 0 (D-017).

## The auto traits

`Send`, `Sync`, `Escapable` and `Sized` are implemented by the
compiler from a type's parts: a struct is `Send` when all its fields are. A program
does not implement them with a plain `impl`.

| Trait | A value of the type |
|---|---|
| `Send` | may move to another task |
| `Sync` | may be shared with another task by reference |
| `Escapable` | may leave the frame that made it: it holds no reference or view |
| `Sized` | has a size of its own: every type but a `dyn` one |

- Numbers, text, the capability handles and most library types are
  `Send` and `Sync`. A `Cell` is never `Sync`; a `Shared[T]` is `Send`
  and `Sync` when `T` is `Sync`; a function value is neither.
- A bound such as `[T: Send]` asks for them, and a task's captures are
  checked against them where it is spawned
  ([R9](concurrency.md#values-that-cross-to-a-task)).
- `Sized`, a marker trait of the prelude, is every type but a `dyn`
  one, whose size is not its own (`dyn Display` stands behind a `ref`
  or a `Box`); since a `dyn` type stands nowhere else (E0395), a bound
  `[T: Sized]` is met by every type argument.
- `unsafe impl Send` and `unsafe impl Sync` assert them for a type
  whose parts do not ([R7](ownership.md#unsafe-code-and-raw-pointers));
  an `impl` without `unsafe` is E0984, and `Escapable` and `Sized` are
  never asserted (E0986).

<!-- file: examples/reference-types/auto_traits.sisu -->
```sisu
# The auto traits follow from a type's parts.
def hand_over[T: Send](own value: T) -> String:
    return "it can move to another task"

def main():
    let counter = Cell(0)
    let scores = Shared([7, 9])
    print(hand_over(counter))
    print(hand_over(scores))
```

<!-- output: examples/reference-types/auto_traits.out -->
```text
it can move to another task
it can move to another task
```

<!-- file: examples/reference-types/not_send.sisu -->
```sisu
# A `Shared` value that holds a `Cell` cannot move to another task.
def hand_over[T: Send](own value: T) -> String:
    return "it can move to another task"

def main():
    let counter = Shared(Cell(0))
    print(hand_over(counter))
```

<!-- output: examples/reference-types/not_send.console -->
```console
$ sisu run not_send.sisu
error[E0347]: `Shared[Cell[int]]` does not implement `Send`
  --> not_send.sisu:7:11
  |
7 |     print(hand_over(counter))
  |           ^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

`Copy` is the exception that a type opts into: the compiler computes
whether a type may be `Copy`, and a struct or enum is `Copy` only when
it says `@derive(Copy)` and all its fields are `Copy` (E0244,
[R17](diagnostics.md#e0244); D-007). Which types are `Copy` is
[R7](ownership.md#copy-types-and-explicit-copies).

**Status.** In stage 0 (D-007).

## Refinement types

`type Port = int where 1 <= self <= 65535` declares a refinement type:
the base type `int` with a predicate on `self`. A `Port` is a subtype
of `int`, usable wherever an `int` is, and an `int` becomes a `Port`
only where the predicate is checked:

- a constant is checked at compile time (E0810 when the predicate is
  false for it);
- any other value is checked by `Port.try_from(n)`, which gives the
  `Port` or the error `Unsatisfied` of `RefinementError`;
- a base value where the refinement is expected is E0811, whose fix
  calls `try_from`, and a change through the base that could break the
  predicate is E0812.

<!-- file: examples/reference-types/refinements.sisu -->
```sisu
# A refinement type is its base with a predicate.
type Port = int where 1 <= self <= 65535

def connect(port: Port) -> String:
    return "port {port}, then {port + 1}"

def main():
    let web: Port = 443
    print(connect(web))
    for n in [8080, 70000]:
        match Port.try_from(n):
            case Ok(port): print(connect(port))
            case Err(e): print("{e:?}")
```

<!-- output: examples/reference-types/refinements.out -->
```text
port 443, then 444
port 8080, then 8081
Unsatisfied(ty="Port", predicate="1 <= self <= 65535")
```

The predicate, its checks and its place in contracts are
[R11](contracts.md#refinement-types).

**Status.** In stage 0
([07](../../proposal/07-contracts.md)).

## Type aliases and newtypes

`type Name = T` is a transparent alias: `Name` and `T` are the same
type, and either is accepted where the other is expected. An alias may
take type parameters, `type Pair[T] = (T, T)`, and may not refer to
itself (E0228).

<!-- file: examples/reference-types/aliases.sisu -->
```sisu
# A type alias is another name for the same type.
type Scores = dict[String, int]
type Pair[T] = (T, T)

def total(scores: Scores) -> int:
    return sum(scores.values())

def main():
    let scores: Scores = {"Aino": 7, "Eero": 9}
    let plain: dict[String, int] = scores
    let corners: Pair[int] = (0, 10)
    print("{total(plain)} {corners.1}")
```

<!-- output: examples/reference-types/aliases.out -->
```text
16 10
```

A new type that must not mix with its base is a struct with one field,
which costs nothing at run time. `@delegate(Trait, ..., to="field")`
forwards traits to the field, so a newtype of `f64` can add and print
without adding to the wrong unit:

<!-- file: examples/reference-types/newtypes.sisu -->
```sisu
# A one-field struct is a new type; an alias is the same type.
type Kilometres = f64

@delegate(Add, Display, to="value")
struct Metres:
    value: f64

struct Seconds:
    value: f64

def main():
    let run = Metres(400.0) + Metres(800.0)
    let lap = Seconds(95.0)
    let walk: Kilometres = 2.5
    print("{run} in {lap.value} s, then {walk + 1.0} km")
```

<!-- output: examples/reference-types/newtypes.out -->
```text
1200.0 in 95.0 s, then 3.5 km
```

<!-- file: examples/reference-types/newtype_mix.sisu -->
```sisu
# Metres and seconds do not add.
@delegate(Add, to="value")
struct Metres:
    value: f64

struct Seconds:
    value: f64

def main():
    let total = Metres(400.0) + Seconds(95.0)
    print(total.value)
```

<!-- output: examples/reference-types/newtype_mix.console -->
```console
$ sisu run newtype_mix.sisu
error[E0310]: `+` cannot be applied to `Metres` and `Seconds`
  --> newtype_mix.sisu:10:17
   |
10 |     let total = Metres(400.0) + Seconds(95.0)
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: `+` on a `Metres` is the method of the trait `Add`: an `impl Add[Seconds] for Metres` gives it one

error[D0006]: the program was not run: errors found (1)
```

**Stage 0 differs.** `@derive(Into, From)`, which 03 gives newtypes for
converting to and from the field, is not in stage 0 (E0243): the
prelude has no `From`, and what the two derives write waits for the
owner's question Q-80. A newtype's constructor and its field convert
both ways meanwhile, and `@delegate` forwards a trait to the field
([R14](attributes.md#delegating-traits-to-a-field)).

**Status.** In stage 0, except `@derive(Into, From)`.

## Subtypes and coercions

Where a type is expected, a value of one of these types is accepted and
converted without a word. There are no others:

| A value of | Where this is expected |
|---|---|
| a narrower integer, or an `f32` | a wider integer, or an `f64` ([Widening and conversions](#widening-and-conversions)) |
| `String`, `list[T]`, `bytes` | `str`, `Slice[T]`, `Slice[u8]`, as a view of itself |
| an error set | a set that holds all its variants |
| a refinement type | its base |
| a `pure` function type | the same function type without `pure` |
| `Never` | any type |
| `ref T`, `Box[T]` | `ref dyn Trait`, `Box[dyn Trait]`, when `T` implements `Trait` |
| a type that implements a capability's operations | the capability ([R10](capabilities.md#capabilities-in-generic-code)) |

A literal that takes the type expected is typing, not a coercion
([Literals that take the type expected](#literals-that-take-the-type-expected)).

<!-- file: examples/reference-types/coercions.sisu -->
```sisu
# Where a type is expected, a few others convert to it implicitly.
type Percent = int where 0 <= self <= 100

pure def double(n: int) -> int:
    return n * 2

def apply(f: (int) -> int, x: int) -> int:
    return f(x)

def half(n: int) -> int:
    return n // 2

def initial(text: str) -> str:
    return text.byte_slice(0, 1)

def main():
    let share: Percent = 40
    let name = "Aino".to_string()
    let small: u8 = 9
    print("{half(share)} {apply(double, small)} {initial(name)}")
```

<!-- output: examples/reference-types/coercions.out -->
```text
20 18 A
```

**Status.** In stage 0.

## What the type system leaves out

These are left out on purpose ([03](../../proposal/03-types-and-generics.md#what-is-not-in-the-type-system),
[14](../../proposal/14-non-goals.md)):

- implicit conversions other than those in
  [Subtypes and coercions](#subtypes-and-coercions);
- `null`: absence is `T?`;
- inheritance, subtyping between structs, and variance annotations;
  the variance that exists, of references, error sets, refinements and
  `pure` function types, the compiler handles;
- overloading of functions by their parameters' types (D-050);
- an effect system: `pure` is the one property a function type
  carries (Q-17);
- higher-kinded types; the syntax `F[_]` is reserved for them (Q-10),
  and stage 0 reports it as E0100 with the note `parse.higher-kinded`,
  which says that the syntax is reserved and that a generic parameter
  is a type, `F`;
- dependent types: refinements and contracts cover the common cases,
  and a verifier comes after 1.0 (Q-16);
- computation on types beyond constant parameters and `comptime`
  functions that make types ([R12](compile-time.md#reflection)).

<!-- file: examples/reference-types/higher_kinded.sisu -->
```sisu
# `F[_]`, the syntax that 03 reserves for higher-kinded types.
def count[F[_]](items: F[int]) -> int:
    return 0

def main():
    print(count([1, 2]))
```

<!-- output: examples/reference-types/higher_kinded.console -->
```console
$ sisu run higher_kinded.sisu
error[E0100]: expected `]`, found `[`
  --> higher_kinded.sisu:2:12
  |
2 | def count[F[_]](items: F[int]) -> int:
  |            ^
  = note: `F[_]` is reserved for higher-kinded types, which Sisu does not have in its first version (03): a generic parameter is a type, `F`

error[D0006]: the program was not run: errors found (1)
```

**Status.** Decided (D-002, D-017).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0300](diagnostics.md#e0300) | `type.mismatch` | [Static types and local inference](#static-types-and-local-inference) |
| [E0324](diagnostics.md#e0324), E0358 | `type.cannot-infer`, `type.let-needs-type` | [Static types and local inference](#static-types-and-local-inference) |
| [E0374](diagnostics.md#e0374) | `type.literal-out-of-range` | [Integers](#integers) |
| [R0003](diagnostics.md#r0003), [R0004](diagnostics.md#r0004), R0005, R0011, R0093 | `panic.overflow`, `panic.divide-by-zero`, `panic.shift-out-of-range`, `panic.negative-exponent`, `panic.shift-out-of-range-sized` | [Overflow traps](#overflow-traps) |
| [E0373](diagnostics.md#e0373), R0023 | `type.mixed-numbers`, `panic.conversion` | [Widening and conversions](#widening-and-conversions) |
| [E0301](diagnostics.md#e0301), E0372, R0017 | `type.integer-division`, `type.float-floor-division`, `panic.unordered` | [Floating point](#floating-point) |
| [E0347](diagnostics.md#e0347) | `type.bound-not-satisfied` | [Floating point](#floating-point), [Collections](#collections), [Traits are implemented by name](#traits-are-implemented-by-name), [Generic types and functions](#generic-types-and-functions), [The auto traits](#the-auto-traits) |
| [E0302](diagnostics.md#e0302), [E0016](diagnostics.md#e0016) | `type.not-bool`, `lex.bad-char-literal` | [Truth values and characters](#truth-values-and-characters) |
| [E0345](diagnostics.md#e0345), [R0115](diagnostics.md#r0115) | `type.str-not-indexable`, `panic.char-boundary` | [Text](#text) |
| [E0398](diagnostics.md#e0398) | `type.structural-only` | [Collections](#collections) |
| E0363, E0364 | `type.tuple-index`, `type.unpack-count` | [Tuples](#tuples) |
| E0212, E0226, E0227 | `resolve.not-an-error-set`, `resolve.inferred-set-position`, `resolve.union-outside-result` | [Results and error sets](#results-and-error-sets) |
| [E0211](diagnostics.md#e0211) | `resolve.ambiguous-variant` | [Structs and enums](#structs-and-enums) |
| E0309, E0352, E0353 | `type.not-callable`, `type.value-returns-borrow`, `type.closure-parameter-count` | [Function types](#function-types) |
| [E0333](diagnostics.md#e0333), [E0334](diagnostics.md#e0334), [E0335](diagnostics.md#e0335), [E0392](diagnostics.md#e0392) | `type.impure-call`, `type.impure-capability`, `type.impure-argument`, `type.impure-value-call` | [Pure function types](#pure-function-types) |
| [E0235](diagnostics.md#e0235), [E0236](diagnostics.md#e0236), E0240 | `resolve.overlapping-impl`, `resolve.orphan-impl`, `resolve.sealed-trait` | [One impl for each type and trait](#one-impl-for-each-type-and-trait) |
| [E0368](diagnostics.md#e0368), E0208, E0349, E0375 | `type.not-ordered`, `resolve.type-argument-count`, `type.not-generic`, `type.type-arguments-needed` | [Generic types and functions](#generic-types-and-functions) |
| [E0245](diagnostics.md#e0245), E0246 | `resolve.generic-default-order`, `resolve.impl-default` | [Defaulted type parameters](#defaulted-type-parameters) |
| E0262, E0263, E0264, E0265, [E0602](diagnostics.md#e0602) | `resolve.const-argument`, `resolve.type-argument`, `resolve.const-expression`, `resolve.const-not-int`, `comptime.panicked` | [Constant parameters](#constant-parameters) |
| [E0395](diagnostics.md#e0395), [E0241](diagnostics.md#e0241), E0396, [E0231](diagnostics.md#e0231) | `type.unsized-dyn`, `resolve.not-object-safe`, `type.view-to-dyn`, `resolve.trait-as-type` | [Trait objects](#trait-objects) |
| [E0244](diagnostics.md#e0244), E0984, E0986 | `resolve.derive-field`, `unsafe.impl-needed`, `unsafe.never-asserted` | [The auto traits](#the-auto-traits) |
| [E0810](diagnostics.md#e0810), [E0811](diagnostics.md#e0811), [E0812](diagnostics.md#e0812) | `refine.not-satisfied`, `refine.needs-try-from`, `refine.change-through-base` | [Refinement types](#refinement-types) |
| E0228, [E0310](diagnostics.md#e0310) | `resolve.alias-cycle`, `type.binary-operator` | [Type aliases and newtypes](#type-aliases-and-newtypes) |
