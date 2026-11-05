# Do exact arithmetic with money

This guide keeps sums of money exact: it holds them as `Decimal`,
rounds them once and by a named rule, splits a bill so that the shares
add up, and keeps fractions exact with `Ratio` until they become money.
It assumes numbers and structs, from chapters 2, 7 and 24 of the
tutorial.

**See also:** the tutorial's
[chapter 24](../tutorial/24-numbers.md) (the exact types); [R2,
Types](../reference/types.md) for `Decimal`, `Ratio` and their
literals; the source of `std.num`,
[`compiler/std/num.sisu`](../../compiler/std/num.sisu); D-076 and
Q-53.

## 1. Hold money as Decimal

An `f64` holds a binary fraction, so most sums of cents are not exact
in it, and the error shows in a total. A `Decimal`, of `std.num`, holds
a number of base 10 exactly, and `+`, `-` and `*` on it are exact
without bound:

<!-- file: examples/guide-money/money.sisu -->
```sisu
# Money in `Decimal`: sums that are exact, unlike those of `f64`.
from std.num import Decimal

def main():
    print(0.1 + 0.2)
    let dime: Decimal = 0.1
    let fifth: Decimal = 0.2
    print(dime + fifth)
    print(Decimal.parse("3.50") ?? Decimal.of(0))
    let written: Decimal = 3.50
    print(written)
```

<!-- output: examples/guide-money/money.out -->
```text
0.30000000000000004
0.3
3.50
3.50
```

A literal becomes a `Decimal` where one is expected, from its text, so
`3.50` keeps the two places it is written with, as `Decimal.parse("3.50")`
does; the places matter only for how it prints. Amounts that come from
text, such as a file or a form, go through `Decimal.parse`, which gives
`None` for text that is not a number.

## 2. Round once, by a named rule

A `Decimal` is never rounded behind your back. `round(places, mode)`
rounds to a number of places, and the mode names how a value between
two cents goes: `HalfEven` (to the even cent, banker's rounding),
`HalfUp` (away from zero, as shops do), `HalfDown`, `Down`, `Up`,
`Floor` or `Ceiling`. Compute with full precision and round where the
amount becomes a figure on paper:

<!-- file: examples/guide-money/invoice.sisu -->
```sisu
# An invoice: exact products and sums, rounded once, by a named rule.
from std.num import Decimal, Rounding

## A line of an invoice: what, how many, and the price of one.
struct Line:
    what: String
    count: int
    price: Decimal

def main():
    let lines = [Line("sauna", 2, 12.50), Line("towel", 3, 1.95)]
    var total = Decimal.of(0)
    for line in lines:
        let amount = line.price * Decimal.of(line.count)
        print("{line.what:<12}{amount.round(2, Rounding.HalfEven):>8}")
        total = total + amount
    let rate: Decimal = 0.14
    let tax = (total * rate).round(2, Rounding.HalfUp)
    let to_pay = (total + tax).round(2, Rounding.HalfEven)
    print("{"tax at 14%":<12}{tax:>8}")
    print("{"to pay":<12}{to_pay:>8}")
```

<!-- output: examples/guide-money/invoice.out -->
```text
sauna          25.00
towel           5.85
tax at 14%      4.32
to pay         35.17
```

The tax, 4.319 before rounding, is rounded up by the rule that the tax
uses; the other figures are rounded to cents for printing only.

## 3. Divide to the cent, and keep what is left

A division of money gives a remainder that someone must pay.
`div(other, places, mode)` divides to the places asked for; give the
cents left over to some of the shares, so that the shares add up to the
total:

<!-- file: examples/guide-money/split.sisu -->
```sisu
# A bill split to the cent, with shares that add up to the bill.
from std.num import Decimal, Rounding

## `total` split among `people`, to the cent: the cents left over go one
## each to the first shares.
def split(total: Decimal, people: int) -> list[Decimal]:
    let share = total.div(Decimal.of(people), 2, Rounding.Down)
    let cent: Decimal = 0.01
    var left = total - share * Decimal.of(people)
    var shares: list[Decimal] = []
    for _ in range(people):
        if left > Decimal.of(0):
            shares.push(share + cent)
            left = left - cent
        else:
            shares.push(share.copy())
    return shares

test "a hundred among three":
    let shares = split(Decimal.of(100), 3)
    assert "{shares}" == "[33.34, 33.33, 33.33]"
    assert shares[0] + shares[1] + shares[2] == Decimal.of(100)

test "every split adds up":
    for people in range(1, 13):
        let total: Decimal = 84.95
        var sum = Decimal.of(0)
        for share in split(total, people):
            sum = sum + share
        assert sum == total
```

<!-- output: examples/guide-money/split.test -->
```text
test "a hundred among three" ... ok
test "every split adds up" ... ok
2 passed, 0 failed, 0 blocked
```

A `Decimal` is not `Copy`, since it holds a number of any size, so the
loop pushes `share.copy()`. Whether a fixed-size decimal type that is
`Copy` should stand beside it is a question for the language's owner,
[Q-79](../../open-questions.md#q-79-is-decimal-a-copy-type).

## 4. Keep a fraction exact with Ratio

A `Ratio` is a fraction of two integers, kept in lowest terms, with
exact `+`, `-`, `*` and `/`. Keep a share that is a fraction, such as
a third, as a `Ratio`, and turn it into money once, at the end, with
one rounding:

<!-- file: examples/guide-money/shares.sisu -->
```sisu
# Shares kept exact as `Ratio`s, and money rounded once, at the end.
from std.num import Decimal, Ratio, Rounding

## The amount that `r` stands for, to the cent, rounded by `mode`.
def to_cents(r: Ratio, mode: Rounding) -> Decimal:
    return Decimal.of(r.num).div(Decimal.of(r.den), 2, mode)

def main():
    let aino = Ratio.of(1, 2)
    let eero = Ratio.of(1, 3)
    let roni = Ratio.of(1, 6)
    print(aino + eero + roni == Ratio.whole(1))
    let bill: Decimal = 249.99
    let exact = bill.to_ratio() ?? Ratio.whole(0)
    for name, share in [("Aino", aino), ("Eero", eero), ("Roni", roni)]:
        let amount = to_cents(exact * share, Rounding.HalfEven)
        print("{name}: {share} of the bill, {amount}")
```

<!-- output: examples/guide-money/shares.out -->
```text
True
Aino: 1/2 of the bill, 125.00
Eero: 1/3 of the bill, 83.33
Roni: 1/6 of the bill, 41.66
```

`bill.to_ratio()` gives the decimal as a fraction, or `None` where its
numerator or denominator is beyond `int`. Each share is rounded once,
`HalfEven`, so a tie such as Roni's 41.665 goes to the even cent, and
here the three amounts add up to the bill.

**Status.** `Decimal`, `Ratio` and the named roundings are in stage 0
(D-076). D-076 makes `/` between two decimals a `Ratio`; stage 0 does
not have it, and its E0310 says in a note what to write instead, `div`
or `to_ratio`:

<!-- file: examples/guide-money/divide.sisu -->
```sisu
# `/` between two decimals, which D-076 makes a `Ratio`.
from std.num import Decimal

def main():
    let bill: Decimal = 249.99
    print(bill / Decimal.of(3))
```

<!-- output: examples/guide-money/divide.console -->
```console
$ sisu check divide.sisu
error[E0310]: `/` cannot be applied to `num.Decimal` and `num.Decimal`
  --> divide.sisu:6:11
  |
6 |     print(bill / Decimal.of(3))
  |           ^^^^^^^^^^^^^^^^^^^^
  = note: a quotient of `Decimal`s has as many digits as it is given: `a.div(b, places, mode)` rounds it to `places` digits after the point by `mode` (`Rounding.HalfEven`, ...), and `a.to_ratio()` makes the exact fraction

```
