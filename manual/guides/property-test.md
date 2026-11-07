# Write a property test

This guide writes a property that must hold for every case, lets
`check` find a case where it does not, reads the shrunk counterexample,
replays it, keeps it as a test once the bug is fixed, and draws a type
its own way. It assumes tests and refinement types, from chapters 19
and 23 of the tutorial.

**See also:** the tutorial's
[chapter 19](../tutorial/19-testing.md#properties-instead-of-examples)
(properties, `exhaustive` and `@refines`); the proposal's
[17, "Property-based testing and fuzzing"](../../proposal/17-testing.md#property-based-testing-and-fuzzing);
the source of `check` and `arbitrary`, in the prelude,
[`compiler/std/prelude.sisu`](../../compiler/std/prelude.sisu).

## 1. Say what must hold, and check it

A property is a function of one case that returns whether the case is
right. `check(gen) do case:` runs it on the cases a generator makes,
and `arbitrary[T]` makes a hundred random cases of a type `T`. This
program splits a bill in cents among the people who share it, and its
property says that the shares add up to the bill:

<!-- file: examples/guide-property-test/split.sisu -->
```sisu
# A property: the shares of a bill add up to the bill.

## A sum in cents, of at most a thousand euros.
type Cents = int where 0 <= self and self <= 100_000

## How many people share a bill.
type People = int where 1 <= self and self <= 12

## A bill and the people who share it.
struct Bill:
    total: Cents
    people: People

## Each person's share of `bill`, in cents.
def split(bill: Bill) -> list[int]:
    return [bill.total // bill.people for _ in range(bill.people)]

test "the shares add up to the bill":
    check(arbitrary[Bill]) do bill:
        return sum(split(bill)) == bill.total
```

The best properties say what is true of every answer without working
the answer out: a total kept, an order kept, a round trip that gives
back what went in, or agreement with a slower function that is clearly
right.

## 2. Read the shrunk counterexample

`sisu test` finds a case for which the property returns `False`, then
shrinks it, making it smaller while it still fails, and reports the
smallest it found:

<!-- output: examples/guide-property-test/split.console -->
```console
$ sisu test split.sisu
test "the shares add up to the bill" ... FAILED

test "the shares add up to the bill" ... FAILED
panic: the property failed for Bill(total=1, people=2) (case 2, shrunk 16 times)
  the property returned False
  at test "the shares add up to the bill" (split.sisu:19:5)
replay: sisu test --replay 1-30b0f46550161bee-0.0.1-t0-s0-r0-c1.1 split.sisu

0 passed, 1 failed, 0 blocked
$ sisu test --replay 1-30b0f46550161bee-0.0.1-t0-s0-r0-c1.1 split.sisu
test "the shares add up to the bill" ... FAILED

test "the shares add up to the bill" ... FAILED
panic: the property failed for Bill(total=1, people=2) (case 1, shrunk 0 times)
  the property returned False
  at test "the shares add up to the bill" (split.sisu:19:5)
replay: sisu test --replay 1-30b0f46550161bee-0.0.1-t0-s0-r0-c1.1 split.sisu

0 passed, 1 failed, 0 blocked
```

- `Bill(total=1, people=2)` is the counterexample: one cent between
  two people gives two shares of nothing. Each part of it is as small
  as a failing case can be, so the bug is in what is left: a remainder
  that `//` drops.
- `case 2, shrunk 16 times` says that the second random case failed
  and that shrinking took 16 steps.
- The `replay:` line's token holds the shrunk case, so the second
  command fails at once on the same case, as its first. Use the token
  while you fix the bug, and give it in a bug report.

## 3. Fix the code, and keep the case

Once the property passes, keep the counterexample as an ordinary test,
since it is the case that found the bug. This version gives the cents
left over one each to the first people, adds a second property, and
keeps the case:

<!-- file: examples/guide-property-test/split_fixed.sisu -->
```sisu
# The shares add up to the bill, and differ by a cent at most.

## A sum in cents, of at most a thousand euros.
type Cents = int where 0 <= self and self <= 100_000

## How many people share a bill.
type People = int where 1 <= self and self <= 12

## A bill and the people who share it.
struct Bill:
    total: Cents
    people: People

## Each person's share of `bill`, in cents: the cents left over go one
## each to the first people.
def split(bill: Bill) -> list[int]:
    let share = bill.total // bill.people
    let left = bill.total % bill.people
    return [share + (if i < left: 1 else: 0) for i in range(bill.people)]

test "the shares add up to the bill":
    check(arbitrary[Bill]) do bill:
        return sum(split(bill)) == bill.total

test "no share is more than a cent above another":
    check(arbitrary[Bill]) do bill:
        let shares = sorted(split(bill))
        return shares[^1] - shares[0] <= 1

test "a cent between two":
    assert split(Bill(total=1, people=2)) == [1, 0]
```

<!-- output: examples/guide-property-test/split_fixed.test -->
```text
test "the shares add up to the bill" ... ok
test "no share is more than a cent above another" ... ok
test "a cent between two" ... ok
3 passed, 0 failed, 0 blocked
```

A test of the counterexample runs the same case on every run, where
the random cases differ from one change of the file to the next.

## 4. Bound the cases with the types

`arbitrary[T]` writes the generator of a struct or an enum from its
fields and variants, with no code of yours. A refinement type bounds
it: in step 1, `Cents` and `People` keep every case a bill of zero to a
thousand euros among one to twelve people, and a struct's `invariant`
bounds a whole case in the same way. Without them, the fixed `split`
fails on a case that no bill can be:

<!-- file: examples/guide-property-test/unbounded.sisu -->
```sisu
# The same property, over cases that no bill can be.

## A bill and the people who share it.
struct Bill:
    total: int
    people: int

## Each person's share of `bill`, in cents: the cents left over go one
## each to the first people.
def split(bill: Bill) -> list[int]:
    let share = bill.total // bill.people
    let left = bill.total % bill.people
    return [share + (if i < left: 1 else: 0) for i in range(bill.people)]

test "the shares add up to the bill":
    check(arbitrary[Bill]) do bill:
        return sum(split(bill)) == bill.total
```

<!-- output: examples/guide-property-test/unbounded.console -->
```console
$ sisu test unbounded.sisu
test "the shares add up to the bill" ... FAILED

test "the shares add up to the bill" ... FAILED
panic: the property failed for Bill(total=0, people=0) (case 1, shrunk once)
  it panicked: division by zero
  at test "the shares add up to the bill" (unbounded.sisu:16:5)
replay: sisu test --replay 1-5cfe5c74ed0aadb4-0.0.1-t0-s0-r0-c0.0.0.0 unbounded.sisu

0 passed, 1 failed, 0 blocked
```

A property that panics fails as one that returns `False` does, and its
case is shrunk too. Bound the cases where the program's own types say
what is possible, so that a counterexample is a case the program can
meet; where a case is possible, such as a bill for no one, handle it in
the code instead and keep the property.

## 5. Draw a type its own way

`arbitrary[T]` draws a struct field by field, each from its type. When
the cases a program meets have a shape that no type says, write the
generator: an impl of `Arbitrary`, whose `arbitrary` makes a value from
the draws, is what `arbitrary[T]` uses for the type, and for every type
that holds one. Cash in Finland is paid to five cents, so a till's sums
are drawn in steps of five:

<!-- file: examples/guide-property-test/cash.sisu -->
```sisu
# Cash sums drawn their own way, in steps of five cents.

## A sum paid in cash, in cents: Finnish shops round cash to 5 cents.
@derive(Debug)
struct Cash:
    cents: int

impl Arbitrary for Cash:
    def arbitrary(d: Draws) -> Cash:
        return Cash(Draws.gen_in(d, 0, 20_000) * 5)

## A bill, and the cash that the customer hands over for it.
@derive(Arbitrary, Debug)
struct Payment:
    due: Cash
    given: Cash

## `cents` rounded to the nearest five cents, a tie upwards.
def to_cash(cents: int) -> int:
    return (cents + 2) // 5 * 5

## Two cases of any type that can be drawn.
def two[T: Arbitrary]() -> Gen[(T, T)]:
    return Gen.of(d -> (T.arbitrary(d), T.arbitrary(d)))

test "the change needs no rounding":
    check(arbitrary[Payment]) do p:
        let change = p.given.cents - p.due.cents
        return to_cash(change) == change

test "two bills together need no rounding":
    check(two[Payment]()) do pair:
        let a, b = pair
        let total = a.due.cents + b.due.cents
        return to_cash(total) == total
```

<!-- output: examples/guide-property-test/cash.test -->
```text
test "the change needs no rounding" ... ok
test "two bills together need no rounding" ... ok
2 passed, 0 failed, 0 blocked
```

- `arbitrary[Payment]` draws both of a payment's sums through
  `Cash`'s impl, so the change is always a sum that cash can pay.
- `@derive(Arbitrary)` gives `Payment` the impl that draws it field by
  field, which code generic over `T: Arbitrary` needs to draw a `T`, as
  `two` does with `T.arbitrary(d)`. A type with a part that has no
  generator cannot derive it (E0820).

In such generic code, `arbitrary[T]` of the type parameter is the
generator its bound gives, the same as `Gen.of(d -> T.arbitrary(d))`:

<!-- file: examples/guide-property-test/bounded.sisu -->
```sisu
# `arbitrary[T]` of a type parameter draws through its bound.

## A sauna's heat, in whole degrees from 60 to 110.
@derive(Debug)
struct Heat:
    celsius: int

impl Arbitrary for Heat:
    def arbitrary(d: Draws) -> Heat:
        return Heat(Draws.gen_in(d, 60, 110))

## The cases of any type that can be drawn, through its `Arbitrary`.
def cases[T: Arbitrary]() -> Gen[T]:
    return arbitrary[T]

test "a heat is drawn in its range":
    check(cases[Heat]()) do h:
        return 60 <= h.celsius <= 110
```

<!-- output: examples/guide-property-test/bounded.test -->
```text
test "a heat is drawn in its range" ... ok
1 passed, 0 failed, 0 blocked
```

Without the bound `T: Arbitrary`, `arbitrary[T]` is E0820
(`test.no-generator`), whose message names the bound to add.

**Status.** In stage 0, with a part of
[17](../../proposal/17-testing.md#property-based-testing-and-fuzzing)
designed and missing: fuzzing with `--fuzz`. The failure database of 17
is in stage 0: `sisu test` keeps each failing case in `FILE.failures`
beside the program and tries it first on the next run, so a property
that still fails reports case 1 without shrinking
([R15](../reference/testing.md#property-tests)); a test that keeps the
case, as in step 3, holds it in the code for good.
