# 24. Numbers in depth

Chapter 2 introduced `int` and `f64`, overflow that stops the program,
and the operators that wrap or saturate. This chapter goes further. You
use integers of each size and their bits, choose between arithmetic that
traps, wraps, saturates or asks first, and convert between types where
information can be lost. Then you see why floating point gives the same
bits on every machine, what NaN does to comparisons, and how the exact
types `BigInt`, `Ratio` and `Decimal` count without rounding. You need
about an hour and a half.

## Integers of every size

Beside `int`, Sisu has integers of fixed sizes: `u8`, `u16`, `u32`,
`u64` and `u128` hold 0 to 2 to the power of their bits, less one, and
`i8`, `i16`, `i32`, `i64` and `i128` hold as many values on each side
of zero. `i64` is another name for `int`, and `uint` another name for
`u64`:

<!-- file: examples/24-numbers/sizes.sisu -->
```sisu
# The sized integers, for data whose size is fixed by a format.
def main():
    let level: u8 = 255
    let sample: i16 = -32768
    let colour = 0xFF8800u32
    let file_size: u64 = 18_446_744_073_709_551_615
    print("{level} {sample} {colour} {file_size}")
    let wider: int = sample
    let total = level + colour
    print("{wider} {total}")
```

<!-- output: examples/24-numbers/sizes.out -->
```text
255 -32768 16746496 18446744073709551615
-32768 16746751
```

- A literal takes the sized type that is expected, as for `level`, or
  the one its suffix names, as `0xFF8800u32` does. It must fit: `256`
  where a `u8` is expected is E0374 (`type.literal-out-of-range`).
- A narrower integer widens by itself where nothing is lost: the `i16`
  becomes an `int` for `wider`, and `level + colour` adds a `u8` and a
  `u32` as two `u32`s.
- Use `int` for counts, lengths and indices, and a sized type where a
  file format, a protocol or a foreign function fixes the size. `uint`
  is for bits and for sizes that come from foreign code.

> **From Python.** Python has one integer type, and meets fixed sizes
> only in modules such as `struct`, `array` and NumPy's types. In Sisu
> each size is an ordinary type, with the same operators as `int` and
> the same check for overflow.

**Try it.** Add `sample` to `colour` in place of `level`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/24-numbers/sizes_mixed.console -->
```console
$ sisu run sizes_mixed.sisu
error[E0310]: `+` cannot be applied to `i16` and `u32`
  --> sizes_mixed.sisu:9:17
  |
9 |     let total = sample + colour
  |                 ^^^^^^^^^^^^^^^
  = fix: make the `i16` an `int`, which holds the values of both: `.to_int()`

error[D0006]: the program was not run: errors found (1)
```

E0310 (`type.binary-operator`) refuses the sum. An `i16` can be
negative and a `u32` cannot, so neither holds all of the other's
values, and Sisu does not pick one. An `int` holds the values of both,
so the fix makes the `i16` an `int` with `.to_int()`, and the `u32`
widens beside it. To keep a sized result, convert one of them, as the
section "Conversions that can lose information" shows.

</details>

## Working with bits

The integer types take the bitwise operators: `&` (and), `|` (or), `^`
(exclusive or), `~` (not), and the shifts `<<` and `>>`. Here they
keep the permissions of a file in the bits of a `u8`, and take a colour
apart:

<!-- file: examples/24-numbers/bits.sisu -->
```sisu
# Bits: and, or, exclusive or, not, and the shifts.
const READ: u8 = 0b100
const WRITE: u8 = 0b010
const RUN: u8 = 0b001

def main():
    let mode = READ | WRITE
    print("{mode:03b} {(mode & WRITE) != 0} {(mode & RUN) != 0}")
    print("{mode ^ WRITE:03b} {~mode:08b}")
    let colour: u32 = 0xFF8800
    let byte = 8
    print("{colour >> 16} {(colour >> byte) & 0xFF} {colour & 0xFF}")
```

<!-- output: examples/24-numbers/bits.out -->
```text
110 True False
100 11111001
255 136 0
```

- `READ | WRITE` sets two bits, and `mode & WRITE` keeps only the bit
  of `WRITE`, which is not zero when it is set.
- `^` flips the bits that are set in its right side, and `~` flips them
  all. `{mode:03b}` prints a number in binary with three digits.
- `colour >> 16` moves the red byte down to the lowest bits, and `&
  0xFF` keeps the lowest eight.
- A shift keeps the type of the value it shifts, whatever integer type
  the amount has: `byte` is an `int`, and `colour >> byte` is still a
  `u32`.
- Bits shifted past the top are lost. A shift by the type's width or
  more, such as a `u8` shifted by 8, panics, since no bit would be
  left.

Sisu asks for parentheses where a bitwise operator meets arithmetic or
a shift: `mode & WRITE + 1` is E0110 (`parse.bitwise-arithmetic`),
since few readers know which binds first. A comparison binds less
tightly than `&`, as in Python, so the parentheses of `(mode & WRITE)
!= 0` are there for the reader.

Every integer type also has methods that count and turn its bits:

<!-- file: examples/24-numbers/bit_methods.sisu -->
```sisu
# The bit methods count, find and turn the bits of an integer.
def main():
    let mode: u8 = 0b0110_1000
    print("{mode.count_ones()} set, {mode.count_zeros()} clear")
    print("{mode.leading_zeros()} above, {mode.trailing_zeros()} below")
    print("{mode.rotate_left(4):08b} {mode.rotate_right(1):08b}")
    let port: u16 = 0x1F90
    print("{port:04x} {port.swap_bytes():04x} {port.to_be():04x}")
    let reading: i8 = -3
    print("{reading.to_bits():08b} {i8.from_bits(0b1111_1101u8)}")
    print("{reading.count_ones()} {(0).leading_zeros()}")
```

<!-- output: examples/24-numbers/bit_methods.out -->
```text
3 set, 5 clear
1 above, 3 below
10000110 00110100
1f90 901f 901f
11111101 -3
7 64
```

- `count_ones()` and `count_zeros()` count the bits that are set and
  those that are not; `leading_zeros()` and `trailing_zeros()` count
  the zeros above the highest bit that is set and below the lowest. Each
  gives an `int`, and the width of the type for 0, so 64 for an `int`.
- `rotate_left(n)` and `rotate_right(n)` turn the bits: those that
  leave one end come back at the other, where a shift loses them. The
  amount is any `int`, taken modulo the width.
- `swap_bytes()` reverses the order of the bytes, and `to_be()` and
  `to_le()` give the bytes in big-endian and little-endian order, for a
  format that fixes it. Every target of stage 0 is little-endian, so
  `to_be()` swaps and `to_le()` changes nothing.
- A signed integer's `to_bits()` gives its bits, its two's complement,
  as the unsigned type of the same width: -3 as an `i8` is 253, the
  `u8` `0b1111_1101`. `i8.from_bits(...)` turns them back. The bit
  methods of a signed type count those bits, so -3 has 7 ones.

## Arithmetic that traps, wraps, saturates or asks

Every integer type has four kinds of arithmetic, and you write the one
you mean. `+`, `-` and `*` trap: a result that does not fit stops the
program. `+%`, `-%` and `*%` wrap around, `+|`, `-|` and `*|` stop at
the type's limits, and the methods `checked_add`, `checked_sub` and
`checked_mul` give `None` where the operator would trap:

<!-- file: examples/24-numbers/kinds.sisu -->
```sisu
# Arithmetic that traps, wraps, saturates or asks first.
def main():
    let volume: u8 = 250
    print("{volume +| 10} {volume -| 251} {volume *| 2}")
    let counter: u16 = 65535
    print("{counter +% 1} {counter *% 2}")
    print(9_223_372_036_854_775_807.checked_add(1))
    print(3_037_000_499.checked_mul(3_037_000_499))
    print(volume + 10)
```

<!-- output: examples/24-numbers/kinds.console -->
```console
$ sisu run kinds.sisu
255 0 255
0 65534
None
Some(9223372030926249001)
panic: integer overflow: 250 + 10 as u8
  at main (kinds.sisu:9:11)
```

- A volume control saturates: 250 and 10 is 255, and the volume never
  goes below 0 or wraps to a whisper.
- A `u16` counter that wraps goes from 65535 to 0, as a checksum or a
  sequence number of a protocol does.
- `checked_mul` tells the caller that 3037000499 squared fits and
  `checked_add` that the largest `int` plus one does not, without a
  panic. The methods are `int`'s; `std.num` has them as functions too,
  `checked_add(a, b)`, with `checked_pow`. The sized types' methods
  that [03](../../proposal/03-types-and-generics.md#numbers) gives are
  not in the compiler yet.
- The panic names the type, `as u8`: overflow traps for every size, in
  every build. So does a division by zero, and the smallest `int`
  divided by -1, whose result is one more than the largest.

## Conversions that can lose information

A conversion that can lose information is a method, and its name says
what happens to a value that does not fit:

<!-- file: examples/24-numbers/lossy.sisu -->
```sisu
# A conversion that can lose information says how it loses it.
def main():
    let reading = -5
    print("{reading.to_u8()} {reading.truncate_to[u8]()}")
    let largest = u64.MAX
    print("{largest.to_int()} {largest.truncate_to[u32]()}")
    print("{largest.truncate_to[i128]()} {u128.MAX.truncate_to[i128]()}")
    let odd = 9_007_199_254_740_993
    print(odd.to_f64())
    print("{2.9.truncate_to_int()} {(-2.9).truncate_to_int()}")
    print("{2.5.round_to[int]()} {(-2.5).round_to[int]()}")
    print(200.4.round_to[u8]())
    print(1e30.truncate_to_int())
```

<!-- output: examples/24-numbers/lossy.console -->
```console
$ sisu run lossy.sisu
None 251
None 4294967295
18446744073709551615 -1
9007199254740992.0
2 -2
3 -3
200
panic: cannot convert 1e30 to `int`
  at main (lossy.sisu:13:11)
```

- `to_u8()` and its siblings give `None` for a value that does not fit,
  here a negative reading. `truncate_to[u8]()` keeps the lowest bits,
  as `+%` does: -5 becomes 251.
- Every integer type has its limits as constants, `u64.MAX` and
  `i8.MIN` among them. `to_int()` of a `u64`, a `u128` or an `i128`
  gives an optional `int`, since their largest values do not fit; of a
  smaller type it gives an `int`. Every integer type has
  `truncate_to`, so `u64.MAX.truncate_to[u32]()` keeps the low 32
  bits, `u32.MAX`. Taken to a type of 128 bits, a value keeps its bits
  read as that type's, so `u64.MAX.truncate_to[i128]()` is `u64.MAX`
  itself and `u128.MAX.truncate_to[i128]()` is -1.
- `to_f64()` gives the nearest `f64`, which is exact up to 2 to the
  power 53: 9007199254740993 is one more, and becomes its even
  neighbour.
- `truncate_to_int()` drops a float's fraction, toward zero, and panics
  for a NaN or a number beyond `int`, such as 10 to the power 30.
- `round_to[int]()` gives the nearest integer, a half away from zero,
  so 2.5 becomes 3 and -2.5 becomes -3. `round_to[u8]()` makes the
  rounded value a `u8` as `truncate_to[u8]()` does, keeping its lowest
  bits.

> **From Python.** Python's `int(x)` of a float grows to any size, and
> `x & 0xFF` is how a Python program keeps a byte. In Sisu a float
> beyond `int` cannot become one, and the name of each method says what
> it keeps.

## Floating point that gives the same bits everywhere

An `f64` holds a binary fraction, so most decimal fractions are
rounded, as in Python. What Sisu adds is that the rounding is the same
on every machine, at every optimisation level: each operation rounds
once, and nothing is reordered or fused behind your back (D-074):

<!-- file: examples/24-numbers/strict.sisu -->
```sisu
# Each float operation rounds once, in the same way on every machine.
def main():
    print(0.1 + 0.2)
    let tenths = [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1]
    var folded = 0.0
    for x in tenths:
        folded += x
    print("{folded} {sum(tenths)}")
    let a = 0.1
    print("{a * 10.0 - 1.0} {a.mul_add(10.0, -1.0)}")
```

<!-- output: examples/24-numbers/strict.out -->
```text
0.30000000000000004
0.9999999999999999 1.0
0.0 5.551115123125783e-17
```

- `0.1 + 0.2` is not `0.3`, since neither tenth is exact in binary.
  Compare floats with a tolerance, or count in `Decimal`.
- Adding ten tenths one by one rounds ten times and ends below 1.
  `sum` adds them pairwise, in a tree of a fixed shape, which rounds
  less and gives 1.0. The shape does not depend on the machine or on
  the number of workers that chapter 26's parallel loops use.
- `a * 10.0 - 1.0` rounds twice: the product rounds to exactly 1.0
  first. `a.mul_add(10.0, -1.0)` is a fused multiply-add, which rounds
  once and keeps the tiny difference. Sisu never fuses on its own, so
  you write it where you want it.

An `f32`, a float of 32 bits for graphics and audio, follows the same
rules: each operation rounds once to `f32`, and an `f32` widens to an
`f64` where one is expected. `@float(fast)` on a function allows
reordering and fusing inside it, giving up the guarantee there
([03](../../proposal/03-types-and-generics.md#numbers)); stage 0
accepts it and keeps the arithmetic strict, which the permission
allows, and `@float` with any other argument is E0448.

## NaN and the order of floats

A float can be NaN, "not a number", the result of `0.0 / 0.0` or a
failed measurement. NaN is equal to nothing, itself included, and is
neither less nor greater than any number:

<!-- file: examples/24-numbers/nan.sisu -->
```sisu
# NaN is equal to nothing, itself included, and has no order.
def main():
    let nan = 0.0 / 0.0
    print("{nan == nan} {nan < 1.0} {nan >= 1.0} {nan.is_nan()}")
    print("{1.0 / 0.0} {-1.0 / 0.0}")
    var temps = [21.5, 19.0, 23.25]
    temps.sort_by((a, b) -> a.cmp(b))
    print(temps)
    temps.push(nan)
    temps.sort_by((a, b) -> a.cmp(b))
    print(temps)
```

<!-- output: examples/24-numbers/nan.console -->
```console
$ sisu run nan.sisu
False False False True
inf -inf
[19.0, 21.5, 23.25]
panic: NaN has no order
  at closure in main (nan.sisu:10:29)
  at main (nan.sisu:10:5)
```

- Every comparison with NaN is `False`, so `x == x` is `False` for a
  NaN, and `is_nan()` is the way to ask.
- Dividing a number other than zero by zero gives an infinity, which
  is ordered: `inf` is greater than every number.
- Because NaN has no place in an order, `f64` is not `Ord`, and
  `sorted(temps)` or `temps.sort()` gets E0347
  (`type.bound-not-satisfied`), whose note names `total_order()`.
  `sort_by` with `a.cmp(b)` sorts floats, and `cmp` panics on a NaN
  rather than leave the list in an order that depends on where the NaN
  was.

Remove NaNs, or replace them, before you sort, or sort in an order
that has a place for them. `x.total_order()` wraps a float in a
`TotalF64`, which is `Ord`: the numbers in order, `-0.0` before `0.0`,
and NaN after every number. Its field `value` gives the float back.
`max_by` and `min_by` take a key that is only partially ordered, such
as a float, and panic on two keys that are not ordered:

<!-- file: examples/24-numbers/total.sisu -->
```sisu
# `total_order()` gives floats an order with a place for NaN.
def main():
    let temps = [21.5, 0.0 / 0.0, 19.0, 23.25]
    let ordered = sorted(temps.iter().map(t -> t.total_order()))
    print([t.value for t in ordered])
    let dry = [21.5, 19.0, 23.25]
    print("{dry.iter().min_by(it).copied()} {dry.iter().max_by(it).copied()}")
    print(temps.iter().max_by(it))
```

<!-- output: examples/24-numbers/total.console -->
```console
$ sisu run total.sisu
[19.0, 21.5, 23.25, NaN]
Some(19.0) Some(23.25)
panic: max_by or min_by compared two keys that are not ordered, such as a NaN
  at main (total.sisu:8:11)
```

- `sorted(temps.iter().map(t -> t.total_order()))` sorts the wrapped
  values, and the NaN comes last, wherever it was. This is the spelling
  that E0347's note names for `sorted(temps)`.
- `min_by(it)` and `max_by(it)` use each reading as its own key, and
  `.copied()` turns the optional borrow into an optional `f64`.
- With the NaN among the keys, `max_by` panics, as `cmp` does, rather
  than give an answer that depends on where the NaN was.

## Integers of any size

A `BigInt` of `std.num` is an integer that grows as large as it needs,
as every Python `int` does:

<!-- file: examples/24-numbers/bigint.sisu -->
```sisu
# A `BigInt` is an integer of any size.
from std.num import BigInt

## `n` factorial: 1 times 2 times 3, and so on up to `n`.
def factorial(n: int) -> BigInt:
    var product: BigInt = 1
    for k in range(2, n + 1):
        product = product * BigInt.of(k)
    return product

def main():
    print(factorial(25))
    print(BigInt.of(2).pow(64) - 1)
    let n = BigInt.parse("123456789012345678901234567890") ?? BigInt.of(0)
    print(n * n)
```

<!-- output: examples/24-numbers/bigint.out -->
```text
15511210043330985984000000
18446744073709551615
15241578753238836750495351562536198787501905199875019052100
```

- `var product: BigInt = 1` takes a literal with no suffix: where a
  `BigInt` is expected, an integer literal is one, and so is a literal
  beside a `BigInt` in arithmetic, as the `1` of `pow(64) - 1`.
- `BigInt.of(k)` makes one from an `int` value, which never becomes a
  `BigInt` by itself, and `BigInt.parse` from text
  of any length, as an optional value. The operators are exact, and
  `pow` raises to a power.
- A `BigInt` allocates as it grows, which is why `int` stays fixed:
  arithmetic on `int` never allocates.

**Try it.** Write the number as a literal: `let n: BigInt =
123456789012345678901234567890`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/24-numbers/bigint_literal.console -->
```console
$ sisu run bigint_literal.sisu
error[E0374]: `123456789012345678901234567890` does not fit in `int`
  --> bigint_literal.sisu:14:21
   |
14 |     let n: BigInt = 123456789012345678901234567890
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: `int` holds -9223372036854775808 to 9223372036854775807
  = note: a larger integer is a `BigInt` (`from std.num import BigInt`): `BigInt.parse("123456789012345678901234567890")`, or the literal beside a `BigInt`, which it becomes

error[D0006]: the program was not run: errors found (1)
```

The proposal checks such a literal at compile time and makes a
`BigInt` of it. The compiler reads every integer literal as an `int`
first, so a literal beyond `int` gets E0374, whose first note gives
`int`'s range. Its second note says what to write until the compiler
does it: `BigInt.parse` reads the number from text, and a literal
beside a `BigInt` becomes one, as this section showed.

</details>

## Exact fractions

A `Ratio` is a fraction of two `int`s, kept in lowest terms, with
operators that never round:

<!-- file: examples/24-numbers/ratio.sisu -->
```sisu
# A `Ratio` is an exact fraction, kept in lowest terms.
from std.num import Ratio

def main():
    let third = Ratio.of(1, 3)
    print("{third + third + third} {Ratio.of(2, 6)} {Ratio.of(3, -4)}")
    let shares = [Ratio.of(1, 2), Ratio.of(1, 3), Ratio.of(1, 6)]
    var total = Ratio.whole(0)
    for share in shares:
        total = total + share
    print("{total} {total == Ratio.whole(1)}")
    print(Ratio.of(1, 10) + Ratio.of(2, 10) == Ratio.of(3, 10))
```

<!-- output: examples/24-numbers/ratio.out -->
```text
1 1/3 -3/4
1 True
True
```

`Ratio.of(1, 3)` is one third, and three of them make exactly 1.
`Ratio.of(2, 6)` is kept as `1/3`, so equal fractions are equal values,
and the sign goes on top. A tenth plus two tenths is three tenths,
which no `f64` sum gives. `Ratio.whole(n)` is a whole number as a
fraction.

## Decimal numbers

Money is counted in tenths and hundredths, which `f64` rounds. A
`Decimal` holds a base-10 number exactly, adds, subtracts and
multiplies without rounding, and rounds only when you say how:

<!-- file: examples/24-numbers/decimal.sisu -->
```sisu
# A `Decimal` holds a base-10 number exactly, and rounds when told to.
from std.num import Decimal, Rounding

def main():
    let price: Decimal = 0.1
    print(price + price + price)
    let bill: Decimal = 100
    let share = bill.div(Decimal.of(3), 2, Rounding.HalfEven)
    print("{share} each, {bill - share * 3} left over")
    let tip: Decimal = 2.345
    for mode in [Rounding.HalfEven, Rounding.HalfUp, Rounding.Down]:
        print("{mode:?}: {tip.round(2, mode)}")
```

<!-- output: examples/24-numbers/decimal.out -->
```text
0.3
33.33 each, 0.01 left over
HalfEven: 2.34
HalfUp: 2.35
Down: 2.34
```

- `let price: Decimal = 0.1` is one tenth exactly: a literal reaches a
  `Decimal` as its digits, not as the nearest binary fraction. Three of
  them make `0.3`.
- `div(other, places, mode)` divides to the places asked for, rounding
  by a named mode. A hundred split three ways is 33.33 each, with a cent
  left over that the program can see and give to someone. The `3` of
  `share * 3` is a `Decimal`, as a literal beside one is.
- `round(places, mode)` rounds. `Rounding` has seven modes:
  `HalfEven` (a tie goes to the even neighbour, the banker's rounding),
  `HalfUp` (a tie goes away from zero), `HalfDown`, `Down` (toward
  zero), `Up`, `Floor` and `Ceiling`. Python's `decimal` has an eighth,
  `ROUND_05UP`, which `Rounding` leaves out.

`/` between two decimals, which 03 designs to give a `Ratio`, is not
in the compiler yet: it is E0310, whose note names `div`, which says
where to round, and `to_ratio`, which gives the exact fraction. A
literal keeps the places it is written with, so `3.50` prints as
`3.50`, as `Decimal.parse("3.50")` does.

> **From Python.** Python's `decimal` rounds every result to the
> precision of a context, 28 digits unless changed, and the context is
> state that any code can change. A `Decimal` has no context: `+`, `-`
> and `*` are exact, and each rounding says its places and its mode
> where it happens.

## Exercise 24.1: predict the numbers

Open
[`manual/exercises/24-numbers/predict_numbers.sisu`](../exercises/24-numbers/predict_numbers.sisu):

<!-- file: exercises/24-numbers/predict_numbers.sisu -->
```sisu
##! Exercise 24.1, predict the numbers. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of what an expression gives, written
##! as `print` would write it. Write each as a literal, then run
##! `sisu test predict_numbers.sisu`. The hints are in chapter 24 of the
##! tutorial, under "Exercise 24.1: predict the numbers".

from std.num import Ratio

const WRAPPED: str = todo("200u8 +% 100")
const SATURATED: str = todo("-100i8 -| 100")
const TRUNCATED: str = todo("(-1).truncate_to[u16]()")
const CONVERTED: str = todo("(-1).to_u16()")
const HALVES: str = todo("0.5 + 0.25 == 0.75")
const TENTHS: str = todo("0.1 + 0.2 == 0.3")
const RATIO: str = todo("Ratio.of(6, -4)")

test "wrapping":
    let level: u8 = 200
    assert "{level +% 100}" == WRAPPED

test "saturating":
    let sample: i8 = -100
    assert "{sample -| 100}" == SATURATED

test "truncating":
    assert "{(-1).truncate_to[u16]()}" == TRUNCATED

test "converting":
    assert "{(-1).to_u16()}" == CONVERTED

test "binary fractions":
    assert "{0.5 + 0.25 == 0.75}" == HALVES

test "decimal fractions":
    assert "{0.1 + 0.2 == 0.3}" == TENTHS

test "an exact fraction":
    assert "{Ratio.of(6, -4)}" == RATIO
```

<!-- output: solutions/24-numbers/predict_numbers.console -->
```console
$ sisu test predict_numbers.sisu
warning[W0001]: typed hole `200u8 +% 100` of type `str`
  --> predict_numbers.sisu:10:22
   |
10 | const WRAPPED: str = todo("200u8 +% 100")
   |                      ^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `-100i8 -| 100` of type `str`
  --> predict_numbers.sisu:11:24
   |
11 | const SATURATED: str = todo("-100i8 -| 100")
   |                        ^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `(-1).truncate_to[u16]()` of type `str`
  --> predict_numbers.sisu:12:24
   |
12 | const TRUNCATED: str = todo("(-1).truncate_to[u16]()")
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `(-1).to_u16()` of type `str`
  --> predict_numbers.sisu:13:24
   |
13 | const CONVERTED: str = todo("(-1).to_u16()")
   |                        ^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `0.5 + 0.25 == 0.75` of type `str`
  --> predict_numbers.sisu:14:21
   |
14 | const HALVES: str = todo("0.5 + 0.25 == 0.75")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `0.1 + 0.2 == 0.3` of type `str`
  --> predict_numbers.sisu:15:21
   |
15 | const TENTHS: str = todo("0.1 + 0.2 == 0.3")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `Ratio.of(6, -4)` of type `str`
  --> predict_numbers.sisu:16:20
   |
16 | const RATIO: str = todo("Ratio.of(6, -4)")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^

test "wrapping" ... blocked by todo("200u8 +% 100")
test "saturating" ... blocked by todo("-100i8 -| 100")
test "truncating" ... blocked by todo("(-1).truncate_to[u16]()")
test "converting" ... blocked by todo("(-1).to_u16()")
test "binary fractions" ... blocked by todo("0.5 + 0.25 == 0.75")
test "decimal fractions" ... blocked by todo("0.1 + 0.2 == 0.3")
test "an exact fraction" ... blocked by todo("Ratio.of(6, -4)")
0 passed, 0 failed, 7 blocked
```

<details>
<summary>Hint 1</summary>

Work out the exact result first, then what the operator does with a
result that does not fit: wrapping counts on from the other end, and
saturating stops at the end. A `u8` holds 0 to 255 and an `i8` -128 to
127.

</details>

<details>
<summary>Hint 2</summary>

-1 is all ones in every width. A fraction is exact in binary when its
denominator is a power of two.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/24-numbers/predict_numbers.sisu`](../solutions/24-numbers/predict_numbers.sisu).

## Exercise 24.2: a checksum that overflows

Open
[`manual/exercises/24-numbers/checksum.sisu`](../exercises/24-numbers/checksum.sisu):

<!-- file: exercises/24-numbers/checksum.sisu -->
```sisu
##! Exercise 24.2, a checksum that overflows. Kind: fix. Level: 1.
##!
##! `djb2` is a quick checksum of bytes: start at 5381, and for each byte
##! multiply by 33 and add the byte, keeping the lowest 32 bits. Two tests
##! pass and two stop the program. Run `sisu test checksum.sisu`, read what
##! stops it, and fix `djb2`, not the tests, so that every test passes.
##! The hints are in chapter 24 of the tutorial, under "Exercise 24.2: a
##! checksum that overflows".

## The djb2 checksum of `data`.
def djb2(data: bytes) -> u32:
    var hash: u32 = 5381
    for byte in data:
        hash = hash * 33 + byte
    return hash

test "nothing":
    assert djb2(b"") == 5381

test "one letter":
    assert djb2(b"a") == 177670

test "a word":
    assert djb2(b"sauna") == 274150173

test "letters beyond ASCII":
    assert djb2("löyly".encode()) == 301641992
```

<!-- output: solutions/24-numbers/checksum.console -->
```console
$ sisu test checksum.sisu
test "nothing" ... ok
test "one letter" ... ok
test "a word" ... FAILED
test "letters beyond ASCII" ... FAILED

test "a word" ... FAILED
panic: integer overflow: 193505550 * 33 as u32
  at djb2 (checksum.sisu:14:16)
  at test "a word" (checksum.sisu:24:12)
replay: sisu test --replay 1-f8aa413be5b4d3d3-0.0.1-t2-s0-r0 checksum.sisu

test "letters beyond ASCII" ... FAILED
panic: integer overflow: 193501226 * 33 as u32
  at djb2 (checksum.sisu:14:16)
  at test "letters beyond ASCII" (checksum.sisu:27:12)
replay: sisu test --replay 1-f8aa413be5b4d3d3-0.0.1-t3-s0-r0 checksum.sisu

2 passed, 2 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

The panic says which operation does not fit in a `u32`. A checksum
keeps the lowest 32 bits of its arithmetic and throws the rest away on
purpose.

</details>

<details>
<summary>Hint 2</summary>

One of the four kinds of arithmetic of the section "Arithmetic that
traps, wraps, saturates or asks" keeps the lowest bits. Both operations
of the line need it.

</details>

When every test passes, compare your answer with
[`manual/solutions/24-numbers/checksum.sisu`](../solutions/24-numbers/checksum.sisu).

## Exercise 24.3: a colour in 32 bits

Open
[`manual/exercises/24-numbers/colour.sisu`](../exercises/24-numbers/colour.sisu):

<!-- file: exercises/24-numbers/colour.sisu -->
```sisu
##! Exercise 24.3, a colour in 32 bits. Kind: write. Level: 2.
##!
##! A colour is three bytes, red, green and blue, which a screen keeps in
##! one `u32` as 0xRRGGBB: red in the bits from 16 to 23, green in those
##! from 8 to 15 and blue in those from 0 to 7. Write the bodies of `pack`,
##! `unpack` and `brighten` so that every test passes. Run the tests with
##! `sisu test colour.sisu`. The hints are in chapter 24 of the tutorial,
##! under "Exercise 24.3: a colour in 32 bits".

## A colour of three bytes.
@derive(Eq, Debug)
struct Colour:
    red: u8
    green: u8
    blue: u8

## `colour` as one number, 0xRRGGBB.
def pack(colour: Colour) -> u32:
    return todo("pack")

## The colour that the lowest 24 bits of `packed` hold.
def unpack(packed: u32) -> Colour:
    return todo("unpack")

## `colour` with each of its parts made brighter by `amount`, to 255 at
## most.
def brighten(colour: Colour, amount: u8) -> Colour:
    return todo("brighten")

test "packing":
    assert pack(Colour(red=0xFF, green=0x88, blue=0x00)) == 0xFF8800
    assert pack(Colour(red=1, green=2, blue=3)) == 0x010203

test "unpacking":
    assert unpack(0xFF8800) == Colour(red=255, green=136, blue=0)
    assert unpack(0x12345678) == Colour(red=0x34, green=0x56, blue=0x78)

test "unpacking undoes packing":
    check(arbitrary[Colour]) do colour:
        return unpack(pack(colour)) == colour

test "brightening stops at 255":
    let orange = Colour(red=250, green=100, blue=0)
    assert brighten(orange, 10) == Colour(red=255, green=110, blue=10)
    assert brighten(orange, 0) == orange
```

<!-- output: solutions/24-numbers/colour.console -->
```console
$ sisu test colour.sisu
warning[W0001]: typed hole `pack` of type `u32`
  --> colour.sisu:19:12
   |
19 |     return todo("pack")
   |            ^^^^^^^^^^^^

warning[W0001]: typed hole `unpack` of type `Colour`
  --> colour.sisu:23:12
   |
23 |     return todo("unpack")
   |            ^^^^^^^^^^^^^^

warning[W0001]: typed hole `brighten` of type `Colour`
  --> colour.sisu:28:12
   |
28 |     return todo("brighten")
   |            ^^^^^^^^^^^^^^^^

test "packing" ... blocked by todo("pack")
test "unpacking" ... blocked by todo("unpack")
test "unpacking undoes packing" ... blocked by todo("pack")
test "brightening stops at 255" ... blocked by todo("brighten")

test "packing" ... blocked by todo("pack")
what the test printed:
  in scope at the typed hole `todo("pack")`:
    colour = Colour(red=255, green=136, blue=0)

test "unpacking" ... blocked by todo("unpack")
what the test printed:
  in scope at the typed hole `todo("unpack")`:
    packed = 16746496

test "unpacking undoes packing" ... blocked by todo("pack")
what the test printed:
  in scope at the typed hole `todo("pack")`:
    colour = Colour(red=175, green=244, blue=79)

test "brightening stops at 255" ... blocked by todo("brighten")
what the test printed:
  in scope at the typed hole `todo("brighten")`:
    colour = Colour(red=250, green=100, blue=0)
    amount = 10

0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

To pack, shift each byte to its place and combine the three with `|`.
A `u8` shifted by 16 panics, since a `u8` has only 8 bits: make each
byte a `u32` first, which a `let` with a type does by widening.

</details>

<details>
<summary>Hint 2</summary>

To unpack, shift the part you want down to the lowest bits. A `u32`
becomes a `u8` through the conversion that keeps the lowest bits, which
also drops everything above the byte.

</details>

<details>
<summary>Hint 3</summary>

Brightening is the arithmetic that stops at the limit.

</details>

When every test passes, compare your answer with
[`manual/solutions/24-numbers/colour.sisu`](../solutions/24-numbers/colour.sisu).

## Exercise 24.4: the warmest reading

Open
[`manual/exercises/24-numbers/warmest.sisu`](../exercises/24-numbers/warmest.sisu):

<!-- file: exercises/24-numbers/warmest.sisu -->
```sisu
##! Exercise 24.4, the warmest reading. Kind: fix. Level: 2.
##!
##! A thermometer that fails reports NaN in place of a temperature.
##! `warmest` finds the warmest reading of a day and must leave out the
##! failed ones, but two of its tests fail. Run `sisu test warmest.sisu`,
##! read the values in the reports, and fix `warmest`, not the tests, so
##! that every test passes. The hints are in chapter 24 of the tutorial,
##! under "Exercise 24.4: the warmest reading".

## The warmest of `readings`, leaving out the failed ones, or `None` when
## there is no reading that worked.
def warmest(readings: Slice[f64]) -> f64?:
    if len(readings) == 0:
        return None
    var best = readings[0]
    for r in readings:
        if r > best:
            best = r
    return Some(best)

## What a thermometer that failed reports.
def failed() -> f64:
    return 0.0 / 0.0

test "every reading worked":
    assert warmest([21.5, 23.0, 19.5]) == Some(23.0)
    assert warmest([-4.0]) == Some(-4.0)

test "a reading in the middle failed":
    assert warmest([21.5, failed(), 23.0]) == Some(23.0)

test "the first reading failed":
    assert warmest([failed(), 21.5, 19.0]) == Some(21.5)

test "no reading worked":
    assert warmest([failed(), failed()]) == None
    assert warmest([]) == None
```

<!-- output: solutions/24-numbers/warmest.console -->
```console
$ sisu test warmest.sisu
test "every reading worked" ... ok
test "a reading in the middle failed" ... ok
test "the first reading failed" ... FAILED
test "no reading worked" ... FAILED

test "the first reading failed" ... FAILED
panic: assertion failed: warmest([failed(), 21.5, 19.0]) == Some(21.5)
  left:  Some(NaN)
  right: Some(21.5)
  at test "the first reading failed" (warmest.sisu:33:5)
replay: sisu test --replay 1-b02019ab2ff4b3df-0.0.1-t2-s0-r0 warmest.sisu

test "no reading worked" ... FAILED
panic: assertion failed: warmest([failed(), failed()]) == None
  left:  Some(NaN)
  right: None
  at test "no reading worked" (warmest.sisu:36:5)
replay: sisu test --replay 1-b02019ab2ff4b3df-0.0.1-t3-s0-r0 warmest.sisu

2 passed, 2 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

Both failing tests start with a failed reading. Follow the loop by
hand when `best` starts as NaN: what does `r > best` give?

</details>

<details>
<summary>Hint 2</summary>

A NaN must never become `best`. Start with no reading at all, an
optional value, and skip each NaN before you compare.

</details>

When every test passes, compare your answer with
[`manual/solutions/24-numbers/warmest.sisu`](../solutions/24-numbers/warmest.sisu).

## Exercise 24.5: a mortgage table

Open
[`manual/exercises/24-numbers/mortgage.sisu`](../exercises/24-numbers/mortgage.sisu):

<!-- file: exercises/24-numbers/mortgage.sisu -->
```sisu
##! Exercise 24.5, a mortgage table. Kind: write. Level: 3.
##!
##! A loan is paid back monthly. Each month costs interest: the balance
##! times the yearly rate over twelve, rounded to the cent, with a tie
##! going to the even cent. The payment pays that interest first, and the
##! rest pays off the balance. The last payment is what is left with its
##! interest, so that the balance ends at exactly zero. Write the body of
##! `schedule` so that every test passes; in `Decimal`, no cent is lost
##! to binary fractions, and `Rounding` is imported from `std.num` beside
##! it. Run the tests with `sisu test mortgage.sisu`. The hints are in
##! chapter 24 of the tutorial, under "Exercise 24.5: a mortgage table".

from std.num import Decimal

## One month of a loan: what it cost in interest, how much of the loan
## it paid off, and what was still owed after it.
@derive(Eq, Debug)
struct Month:
    interest: Decimal
    principal: Decimal
    balance: Decimal

## The months of a loan of `amount` at the yearly `rate` (0.06 is 6 %),
## paid back with `payment` each month. The payment must be more than the
## first month's interest, or the loan is never paid back.
def schedule(amount: Decimal, rate: Decimal, payment: Decimal) -> list[Month]:
    return todo("schedule")

## The interest of all of `months` together.
def total_interest(months: list[Month]) -> Decimal:
    var total = Decimal.of(0)
    for month in months:
        total = total + month.interest
    return total

test "the first month":
    let months = schedule(1000.00, 0.06, 200.00)
    assert months[0] == Month(interest=5.00, principal=195.00, balance=805.00)

test "a tie goes to the even cent":
    let months = schedule(1000.00, 0.06, 200.00)
    assert months[1] == Month(interest=4.02, principal=195.98, balance=609.02)

test "the last payment pays off what is left":
    let months = schedule(1000.00, 0.06, 200.00)
    assert len(months) == 6
    assert months[5] == Month(interest=0.08, principal=15.20, balance=0)
    assert total_interest(months) == 15.28

test "a long loan loses no cent":
    let months = schedule(250000.00, 0.045, 1500.00)
    assert len(months) == 263
    assert months[262].balance == 0
    assert total_interest(months) == 143066.83
```

<!-- output: solutions/24-numbers/mortgage.console -->
```console
$ sisu test mortgage.sisu
warning[W0001]: typed hole `schedule` of type `list[Month]`
  --> mortgage.sisu:27:12
   |
27 |     return todo("schedule")
   |            ^^^^^^^^^^^^^^^^

test "the first month" ... blocked by todo("schedule")
test "a tie goes to the even cent" ... blocked by todo("schedule")
test "the last payment pays off what is left" ... blocked by todo("schedule")
test "a long loan loses no cent" ... blocked by todo("schedule")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Check the first two months by hand. 1,000.00 at 6 % a year costs 5.00
in the first month; in the second, 805.00 costs 4.025, which rounds to
the even cent, 4.02.

</details>

<details>
<summary>Hint 2</summary>

Loop while something is owed. A month's interest is the balance times
the rate, divided by 12 to two places with `div` and the rounding that
sends a tie to the even cent. The last payment is the smaller of the
payment and what is owed with its interest.

</details>

<details>
<summary>Hint 3</summary>

A `Decimal` is not `Copy`. A parameter you want to change needs a copy
of its own in a `var`, and the balance you keep in a month must be a
copy too, since the loop goes on changing it.

</details>

When every test passes, compare your answer with
[`manual/solutions/24-numbers/mortgage.sisu`](../solutions/24-numbers/mortgage.sisu).

## Exercise 24.6: a hash that turns its bits

Open
[`manual/exercises/24-numbers/rotating.sisu`](../exercises/24-numbers/rotating.sisu):

<!-- file: exercises/24-numbers/rotating.sisu -->
```sisu
##! Exercise 24.6, a hash that turns its bits. Kind: write. Level: 2.
##!
##! A rotating hash mixes each byte of its data into a `u32`: it turns the
##! hash's bits 5 places to the left, the bits that leave the top coming
##! back in at the bottom, and then flips the bits that the byte has set.
##! A parity bit says whether data has an odd number of bits set, so that
##! a reader of the data can tell that one bit changed. Write the bodies
##! of `rotating` and `odd_parity` so that every test passes. Run the
##! tests with `sisu test rotating.sisu`. The hints are in chapter 24 of
##! the tutorial, under "Exercise 24.6: a hash that turns its bits".

## The rotating hash of `data`, starting from 0.
def rotating(data: bytes) -> u32:
    return todo("rotating")

## Whether `data` has an odd number of bits set.
def odd_parity(data: bytes) -> bool:
    return todo("odd_parity")

test "the hash of a short text":
    assert rotating(b"") == 0
    assert rotating(b"a") == 0x61
    assert rotating(b"ab") == 0xC42

test "the bits that leave the top come back at the bottom":
    assert rotating(b"sauna and sisu") == 0x610C29E6
    assert rotating(b"\xff\xff\xff\xff\xff\xff\xff") == 0x318C6321

test "parity":
    assert not odd_parity(b"")
    assert odd_parity(b"\x07")
    assert not odd_parity(b"\x03\x03")
    assert odd_parity("sisu".encode())
```

<!-- output: solutions/24-numbers/rotating.console -->
```console
$ sisu test rotating.sisu
warning[W0001]: typed hole `rotating` of type `u32`
  --> rotating.sisu:14:12
   |
14 |     return todo("rotating")
   |            ^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `odd_parity` of type `bool`
  --> rotating.sisu:18:12
   |
18 |     return todo("odd_parity")
   |            ^^^^^^^^^^^^^^^^^^

test "the hash of a short text" ... blocked by todo("rotating")
test "the bits that leave the top come back at the bottom" ... blocked by todo("rotating")
test "parity" ... blocked by todo("odd_parity")

test "the hash of a short text" ... blocked by todo("rotating")
what the test printed:
  in scope at the typed hole `todo("rotating")`:
    data = b""

test "the bits that leave the top come back at the bottom" ... blocked by todo("rotating")
what the test printed:
  in scope at the typed hole `todo("rotating")`:
    data = b"sauna and sisu"

test "parity" ... blocked by todo("odd_parity")
what the test printed:
  in scope at the typed hole `todo("odd_parity")`:
    data = b""

0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

Check the second byte by hand: `0x61` turned 5 places to the left is
`0xC20`, and flipping the bits of `0x62` in it gives `0xC42`. The bits
that leave the top of the `u32` are what a shift would lose.

</details>

<details>
<summary>Hint 2</summary>

A byte of `data` is a `u8`, and the hash a `u32`: a `let` with the type
`u32` widens the byte. The section "Working with bits" has a method
that turns the bits and one that counts them.

</details>

<details>
<summary>Hint 3</summary>

The parity is the count of the bits that are set in every byte, added
up, and whether it is odd.

</details>

When every test passes, compare your answer with
[`manual/solutions/24-numbers/rotating.sisu`](../solutions/24-numbers/rotating.sisu).

## What you learned

- The sized integers `u8` to `u128` and `i8` to `i128` are for data
  whose size is fixed; `int` is `i64` and `uint` is `u64`. A narrower
  integer widens where nothing is lost, and signed and unsigned types
  do not mix (E0310).
- `&`, `|`, `^`, `~`, `<<` and `>>` work on the bits; a shift keeps the
  type of the value shifted, and a shift by the width panics.
- `count_ones`, `leading_zeros`, `rotate_left`, `swap_bytes` and the
  other bit methods count and turn the bits; a signed type's
  `to_bits()` gives its two's complement as an unsigned type.
- `+` traps, `+%` wraps, `+|` saturates, and `std.num`'s
  `checked_add` gives `None` where `+` would trap.
- Conversions that lose information are methods: `to_u8()` gives
  `None`, `truncate_to[u8]()` keeps the lowest bits, `to_f64()` rounds
  to the nearest and `truncate_to_int()` panics beyond `int`.
- Floating point is strict (D-074): one rounding per operation, no
  fusing but `mul_add`, and `sum` in a fixed pairwise shape.
- NaN compares `False` with everything; `f64` is not `Ord`, and
  `cmp` panics on a NaN.
- `BigInt`, `Ratio` and `Decimal` of `std.num` are exact; a `Decimal`
  rounds only in `div` and `round`, with a named `Rounding`.

Chapter 25 is about code that runs while the program compiles:
`comptime` values and functions, reflection over a type's fields,
derives written in Sisu, files embedded at compile time, and code for
one target only.
