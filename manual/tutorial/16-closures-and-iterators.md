# 16. Closures and iterators

In this chapter you pass functions around as values, write them where
they are used as closures, and give them blocks with `do`. Then you
meet the `Iterator` trait, whose adapters turn loops into pipelines
that do no work until something asks for their items, and generators,
which give their items one at a time with `yield`. Last comes what `?`
does inside a closure. You need about two hours.

## Functions are values

A function can be passed to another function, kept in a binding and
called through it. Its type is written with the types of its
parameters and of its result:

<!-- file: examples/16-closures-and-iterators/values.sisu -->
```sisu
# A function is a value, and its type says what it takes and gives.
def apply_to_all(numbers: list[int], f: (int) -> int) -> list[int]:
    return [f(n) for n in numbers]

def double(n: int) -> int:
    return n * 2

def negate(n: int) -> int:
    return -n

def main():
    let numbers = [1, 2, 3]
    print(apply_to_all(numbers, double))
    print(apply_to_all(numbers, negate))
    let chosen: (int) -> int = if len(numbers) > 2: double else: negate
    print(chosen(21))
```

<!-- output: examples/16-closures-and-iterators/values.out -->
```text
[2, 4, 6]
[-1, -2, -3]
42
```

- `f: (int) -> int` is a parameter whose type is a function that takes
  one `int` and returns one. `f(n)` calls it.
- `apply_to_all(numbers, double)` passes `double` itself, without
  calling it: no parentheses after the name.
- `chosen` holds one function or the other, picked while the program
  runs.

A function type without parameters is `() -> int`, and one that
returns nothing is `(int) -> ()`.

## Closures

A closure is a function written as an expression, where it is used:
the parameters, an arrow and an expression for the result:

<!-- file: examples/16-closures-and-iterators/closures.sisu -->
```sisu
# A closure is a function written where it is used.
def apply_to_all(numbers: list[int], f: (int) -> int) -> list[int]:
    return [f(n) for n in numbers]

def main():
    let numbers = [1, 2, 3]
    print(apply_to_all(numbers, n -> n * n))
    print(apply_to_all(numbers, it + 100))
    let add = (a: int, b: int) -> a + b
    let answer = () -> 42
    print("{add(2, 3)} {answer()}")
```

<!-- output: examples/16-closures-and-iterators/closures.out -->
```text
[1, 4, 9]
[101, 102, 103]
5 42
```

- `n -> n * n` takes `n` and returns its square. Its parameter's type
  comes from the function type that `apply_to_all` expects.
- `it + 100` is a closure too. Where a function of one parameter is
  expected, an expression that mentions `it` is a closure whose
  parameter is `it` (D-026). In calls inside calls, `it` belongs to
  the innermost one.
- `(a: int, b: int) -> a + b` takes two parameters, and `() -> 42`
  none. A closure kept in a `let` has no expected type to learn from,
  so its parameters name their types.

> **From Python.** A closure is Python's `lambda`, with an arrow
> instead of the keyword: `n -> n * n` is `lambda n: n * n`. As with
> `lambda`, its body is one expression; the next section shows the
> form with a block.

## A closure with a block

A closure that needs statements is written as a block after the call,
with `do`, the parameters and a colon. The block is the call's last
argument:

<!-- file: examples/16-closures-and-iterators/retry.sisu -->
```sisu
# A trailing `do` block is a closure whose body is a block.
def retry(times: int, attempt: (int) -> bool) -> bool:
    for n in range(1, times + 1):
        if attempt(n):
            return True
    return False

def main():
    let ok = retry(3) do n:
        print("attempt {n}")
        return n == 2
    print("succeeded: {ok}")
    var total = 0
    let found = retry(5) do n:
        total += n
        return total > 100
    print("{found} after adding up to {total}")
```

<!-- output: examples/16-closures-and-iterators/retry.out -->
```text
attempt 1
attempt 2
succeeded: True
False after adding up to 15
```

- `retry(3) do n:` calls `retry` with `3` and a closure of one
  parameter, `n`, whose body is the indented block.
- `return` inside the block returns from the closure, not from `main`:
  each attempt gives `retry` its `bool`.
- `do:` without parameters binds `it`.

A `do` block can only end a line, so a block never sits in the middle
of an expression. A call that needs two closures with blocks is
clearer with a nested `def` for one of them.

## What a closure keeps

A closure uses the variables around it by borrowing them, as chapter 11
showed, and it can change a `var` it borrows. A closure that must
outlive the function that makes it is an `own` closure, which keeps a
copy of what it uses:

<!-- file: examples/16-closures-and-iterators/captures.sisu -->
```sisu
# A closure borrows what it uses; an `own` closure keeps a copy.
def each_word(text: str, action: (str) -> ()):
    for word in text.split_whitespace():
        action(word)

def scaler(factor: f64) -> (f64) -> f64:
    return own x -> x * factor

def main():
    var longest = ""
    each_word("hei kaunis maailma") do word:
        if len(word) > len(longest):
            longest = word
    print(longest)
    let to_feet = scaler(3.28)
    print(to_feet(2.0))
```

<!-- output: examples/16-closures-and-iterators/captures.out -->
```text
maailma
6.56
```

- The `do` block borrows `longest` with `mut` and changes it, so after
  the call `longest` holds the longest word.
- `scaler` returns `own x -> x * factor`. The closure keeps its own
  copy of `factor`, so it still works after `scaler` has returned.

**Try it.** Write a function `greeter(own greeting: String)` that
returns `own name -> "{greeting}, {name}"`, and call the closure it
returns.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/16-closures-and-iterators/greeter.console -->
```console
$ sisu run greeter.sisu
error[E0423]: an `own` closure keeps a copy of `greeting`, and `String` is not `Copy`; stage 0 copies captures (a capture by move arrives with tasks)
  --> greeter.sisu:3:12
  |
3 |     return own name -> "{greeting}, {name}"
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: copy it into a local first (`let c = name.copy()`) and capture that, or use a plain closure, which borrows it

error[D0006]: the program was not run: errors found (1)
```

E0423 (`type.own-closure-capture`) says that stage 0 copies what an
`own` closure keeps, and a `String` is not `Copy`. A closure that
moves a value into itself is designed (D-026), and the message says
that the compiler will have it with tasks. Until then an `own` closure
keeps `Copy` values, such as numbers. The note's first suggestion, a
copy in a local, gets the same error, since the copy is a `String`
too.

</details>

## Pure function types

Chapter 26 is about `pure` functions, which use no capability and
give the same result for the same arguments. A `pure` function that
calls a function it is given can stay pure only if that function is
pure too, so a function type can say `pure`:

<!-- file: examples/16-closures-and-iterators/pure_twice.sisu -->
```sisu
# A `pure` function type admits only functions that are pure.
pure def apply_twice(f: pure (int) -> int, x: int) -> int:
    return f(f(x))

pure def square(n: int) -> int:
    return n * n

def main():
    print(apply_twice(n -> n * 3, 2))
    print(apply_twice(square, 3))
```

<!-- output: examples/16-closures-and-iterators/pure_twice.out -->
```text
18
81
```

`f: pure (int) -> int` admits a closure that only computes, such as
`n -> n * 3`, which the compiler sees is pure, and a `pure def` such as
`square`. A plain `(int) -> int` admits any function.

**Try it.** Add a function `noisy_square` that prints before it
squares, and pass it to `apply_twice`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/16-closures-and-iterators/impure_twice.console -->
```console
$ sisu run impure_twice.sisu
error[E0335]: this function is passed where a pure function is required, but it calls `noisy_square`, which is not pure
  --> impure_twice.sisu:15:23
   |
15 |     print(apply_twice(noisy_square, 3))
   |                       ^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0335 (`type.impure-argument`) refuses the function, since printing is
not pure. The standard library's `sort_by` takes a `pure` comparison,
so that sorting gives the same order every time.

</details>

## The Iterator trait

A `for` loop works on anything that implements `Iterator`, a trait
with an associated type, `Item`, and one required method, `next`,
which gives the next item or `None` at the end. Implement it, and a
type of yours works in a `for` loop:

<!-- file: examples/16-closures-and-iterators/countdown.sisu -->
```sisu
# A type with `next` is an iterator, and gets every adapter with it.
struct Countdown:
    left: int

impl Iterator for Countdown:
    type Item = int

    def next(mut self) -> int?:
        if self.left == 0:
            return None
        self.left -= 1
        return Some(self.left + 1)

def main():
    for n in Countdown(3):
        print(n)
    print(Countdown(10).filter(it % 3 == 0).collect())
    print(sum(Countdown(4)))
```

<!-- output: examples/16-closures-and-iterators/countdown.out -->
```text
3
2
1
[9, 6, 3]
10
```

`next` takes `mut self`, since each call moves the countdown on. The
trait's provided methods, such as `filter` and `collect`, work on any
iterator, so `Countdown` gets them without writing them, and so does
`sum`, which takes anything a `for` loop can go through.

## Adapters and consumers

The provided methods of `Iterator` come in two kinds. An adapter, such
as `map`, `filter`, `take`, `skip` and `enumerate`, wraps an iterator
in another. A consumer, such as `collect`, `fold`, `count`, `find`,
`any`, `all`, `max_by` and `join`, runs an iterator to its end, or to
the item it looks for, and gives a value. A list gives an iterator over
its elements with `iter()`:

<!-- file: examples/16-closures-and-iterators/pipeline.sisu -->
```sisu
# Adapters make an iterator from an iterator; consumers drive it.
def main():
    let trees = ["kuusi", "mänty", "koivu", "leppä", "tammi", "pihlaja"]
    let k_trees = trees.iter().filter(it.starts_with("k"))
    print(k_trees.map(it.to_uppercase()).collect())
    print(trees.iter().map(len(it)).fold(0, (total, n) -> total + n))
    print(trees.iter().skip(2).take(3).join(" "))
    print(trees.iter().find(len(it) > 5))
    let has_oak = trees.iter().any(it == "tammi")
    print("{has_oak} {trees.iter().all(len(it) > 4)}")
    print(trees.iter().max_by(len(it)))
```

<!-- output: examples/16-closures-and-iterators/pipeline.out -->
```text
["KUUSI", "KOIVU"]
32
koivu leppä tammi
Some("pihlaja")
True True
Some("pihlaja")
```

*Kuusi*, *mänty*, *koivu*, *leppä*, *tammi* and *pihlaja* are Finnish
for spruce, pine, birch, alder, oak and rowan.

- `filter(it.starts_with("k"))` keeps the items for which the closure
  is `True`, and `map(it.to_uppercase())` passes each through its
  closure. `collect()` puts the items into a list.
- `fold(0, (total, n) -> total + n)` starts from 0 and combines it with
  each item in turn: here, the total length.
- `skip(2).take(3)` leaves out two items and keeps the next three, and
  `join(" ")` writes them with spaces between.
- `find`, `any`, `all` and `max_by` take closures that judge an item;
  `find` and `max_by` give an optional value, since there may be none.

`iter()` borrows each element, so the closures see `ref str` items, and
the list stays as it is. In stage 0, `collect()` always makes a list: a
type argument such as `collect[set[int]]()` compiles but still gives a
list. A set comes from a set comprehension, as in chapter 5.

> **From Python.** Python's `map(f, xs)` and `filter(f, xs)` are
> functions that take a list. In Sisu they are methods of an iterator,
> so a list needs `iter()` first, and the steps read from left to
> right: `xs.iter().filter(...).map(...)`. A comprehension, as in
> chapter 5, is often the clearest way to make one list from another.

## Nothing runs until it is asked for

An adapter does no work when it is made. It runs its closure for an
item only when something asks for that item:

<!-- file: examples/16-closures-and-iterators/lazy.sisu -->
```sisu
# An adapter does nothing until something asks for its items.
def loud_square(n: int) -> int:
    print("  squaring {n}")
    return n * n

def main():
    let squares = range(1, 1_000_000).map(loud_square)
    print("nothing has run yet")
    for square in squares.take(2):
        print("got {square}")
```

**Predict.** `range(1, 1_000_000)` has nearly a million numbers. How
many does the program square? Then compare:

<!-- output: examples/16-closures-and-iterators/lazy.out -->
```text
nothing has run yet
  squaring 1
got 1
  squaring 2
got 4
```

`map(loud_square)` squares nothing when it is made, which is why the
first line comes before any squaring. The `for` loop asks `take(2)` for
items, `take` asks `map`, and `map` asks the range, one item at a time.
After two items `take` stops asking, so two numbers are squared, not a
million. Such iterators are called lazy, and they are why a pipeline
over a large input costs no more memory than its items in flight.

## Generators

A generator is a function that gives its items one at a time with
`yield`, and returns `Iter[T]`, an iterator of `T`. Each time the code
that uses it asks for an item, the generator runs to its next `yield`
and waits there:

<!-- file: examples/16-closures-and-iterators/fibonacci.sisu -->
```sisu
# A generator gives its items with `yield`, one each time it is asked.
def fibonacci() -> Iter[int]:
    var a = 0
    var b = 1
    while True:
        yield a
        a, b = b, a + b

def main():
    print(fibonacci().take(10).collect())
    print(fibonacci().filter(it % 2 == 0).take(4).collect())
    for f in fibonacci():
        if f > 1000:
            print("the first above 1000 is {f}")
            break
```

<!-- output: examples/16-closures-and-iterators/fibonacci.out -->
```text
[0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
[0, 2, 8, 34]
the first above 1000 is 1597
```

`fibonacci` loops without end, which is safe because it is lazy: it
makes only the numbers that are asked for. `take(10)` asks for ten,
`filter` with `take(4)` for as many as it takes to find four even ones,
and the `for` loop stops it with `break`. A generator's result type
must be `Iter[T]`, and a generator ends when its body ends.

**Try it.** Python's generator expression, `(p * p for p in primes)`,
makes a lazy iterator in one line. Write one in Sisu.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/16-closures-and-iterators/squares_expr.console -->
```console
$ sisu run squares_expr.sisu
error[E0900]: a generator expression is not implemented in stage 0 yet; it arrives with generators (Phase 8)
  --> squares_expr.sisu:4:19
  |
4 |     let squares = (p * p for p in primes)
  |                   ^^^^^^^^^^^^^^^^^^^^^^^

error[E0304]: `list[int]` has no method `collect`
  --> squares_expr.sisu:5:19
  |
5 |     print(squares.collect())
  |                   ^^^^^^^
  = note: `collect` is a method of `Iterator`
  = fix: iterate it first: `.iter()`

error[D0006]: the program was not run: errors found (2)
```

E0900 (`stage0.unsupported`) says that stage 0 does not have
generator expressions yet; they are designed in
[01](../../proposal/01-tour.md#collections-and-iteration). The second
error follows from the first. Until they arrive, write
`primes.iter().map(it * it)`, which is lazy too.

</details>

## Errors inside a pipeline

`?` inside a closure leaves the closure, not the function around it,
so a closure with `?` returns a result of its own:

<!-- file: examples/16-closures-and-iterators/parse_all.sisu -->
```sisu
# `?` inside a closure leaves the closure, not the function around it.
error ParseError:
    NotANumber(text: String)

def number(text: str) -> int ! ParseError:
    let n = text.trim().parse_int() else:
        return Err(NotANumber(text.to_string()))
    return Ok(n)

def all_doubled(lines: list[str]) -> list[int] ! ParseError:
    var numbers: list[int] = []
    for line in lines:
        numbers.push(number(line)? * 2)
    return Ok(numbers)

def main():
    let lines = ["12", " 7", "x"]
    print(lines.iter().map(line -> Ok(number(line)? * 2)).collect())
    print(all_doubled(lines))
    print(all_doubled(lines[:2]))
```

<!-- output: examples/16-closures-and-iterators/parse_all.out -->
```text
[Ok(24), Ok(14), Err(NotANumber(text="x"))]
Err(NotANumber(text="x"))
Ok([24, 14])
```

- In `main`, the closure `line -> Ok(number(line)? * 2)` returns a
  result for each line, and `map` collects one result per line, an
  error among them. The pipeline goes on after the error.
- `all_doubled` wants every number or the first error. A loop with `?`
  gives that: the first `Err` returns from `all_doubled` at once.

`try_map` and `try_collect`, which stop a pipeline at its first error
the way the loop does, are designed in
[05](../../proposal/05-errors.md#propagation) and are not in the
compiler yet, so a parse that can fail is a loop.

## Exercise 16.1: predict the pipelines

Open
[`manual/exercises/16-closures-and-iterators/predict_pipelines.sisu`](../exercises/16-closures-and-iterators/predict_pipelines.sisu):

<!-- file: exercises/16-closures-and-iterators/predict_pipelines.sisu -->
```sisu
##! Exercise 16.1, predict the pipelines. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_pipelines.sisu`. The hints are in chapter 16 of
##! the tutorial, under "Exercise 16.1: predict the pipelines".

## 0, 1, 2 and so on, without end.
def count_up() -> Iter[int]:
    var n = 0
    while True:
        yield n
        n += 1

## How many squares a pipeline works out before it is done.
def squares_worked_out() -> int:
    var calls = 0
    let squares = range(1, 100).map() do n:
        calls += 1
        return n * n
    let first_three = squares.take(3).collect()
    return calls

const MULTIPLES: list[int] = todo("multiples of 3 below 10, times 10")
const FOLDED: int = todo("10 minus each number in turn")
const TAKEN: list[int] = todo("a slice of the counting")
const JOINED: str = todo("letters in capitals, joined")
const CALLS: int = todo("squares worked out")

test "multiples":
    let tens = range(1, 10).filter(it % 3 == 0).map(it * 10)
    assert tens.collect() == MULTIPLES

test "a fold":
    assert [1, 2, 3, 4].iter().fold(10, (acc, x) -> acc - x) == FOLDED

test "counting without end":
    assert count_up().skip(2).take(3).collect() == TAKEN

test "a join":
    let letters = ["a", "b", "c"]
    assert letters.iter().map(it.to_uppercase()).join("-") == JOINED

test "laziness":
    assert squares_worked_out() == CALLS
```

<!-- output: solutions/16-closures-and-iterators/predict_pipelines.console -->
```console
$ sisu test predict_pipelines.sisu
test "multiples" ... blocked by todo("multiples of 3 below 10, times 10")
test "a fold" ... blocked by todo("10 minus each number in turn")
test "counting without end" ... blocked by todo("a slice of the counting")
test "a join" ... blocked by todo("letters in capitals, joined")
test "laziness" ... blocked by todo("squares worked out")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

Work each pipeline from left to right, writing down the items after
each step. `range(1, 10)` stops before 10, and a fold starts from the
value it is given.

</details>

<details>
<summary>Hint 2</summary>

`squares_worked_out` counts the calls of its closure. The section
"Nothing runs until it is asked for" says which items a `map` works
out.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/16-closures-and-iterators/predict_pipelines.sisu`](../solutions/16-closures-and-iterators/predict_pipelines.sisu).

## Exercise 16.2: the adults

Open
[`manual/exercises/16-closures-and-iterators/adults.sisu`](../exercises/16-closures-and-iterators/adults.sisu):

<!-- file: exercises/16-closures-and-iterators/adults.sisu -->
```sisu
##! Exercise 16.2, the adults. Kind: fix. Level: 1.
##!
##! A guest list is filtered and mapped the way a Python programmer
##! would write it. The file does not compile. Run
##! `sisu test adults.sisu`, read the diagnostics, and fix the two
##! functions, not the tests, so that every test passes. The hints are
##! in chapter 16 of the tutorial, under "Exercise 16.2: the adults".

struct Guest:
    name: String
    age: int

## The names of the guests who are `age` or older, in capitals.
def adults(guests: list[Guest], age: int) -> list[String]:
    let old_enough = guests.filter(it.age >= age)
    return old_enough.map(it.name.to_uppercase()).collect()

## The total age of the guests.
def total_age(guests: list[Guest]) -> int:
    return guests.map(it.age).fold(0, (total, n) -> total + n)

## The guests of the party.
def party() -> list[Guest]:
    return [Guest("Aino", 34), Guest("Ella", 12), Guest("Eero", 29)]

test "the adults":
    assert adults(party(), 18) == ["AINO", "EERO"]
    assert adults(party(), 30) == ["AINO"]

test "the total age":
    assert total_age(party()) == 75
    assert total_age([]) == 0
```

<!-- output: solutions/16-closures-and-iterators/adults.console -->
```console
$ sisu test adults.sisu
error[E0304]: `list[Guest]` has no method `filter`
  --> adults.sisu:15:29
   |
15 |     let old_enough = guests.filter(it.age >= age)
   |                             ^^^^^^
  = note: `filter` is a method of `Iterator`
  = fix: iterate it first: `.iter()`

error[E0304]: `list[Guest]` has no method `map`
  --> adults.sisu:20:19
   |
20 |     return guests.map(it.age).fold(0, (total, n) -> total + n)
   |                   ^^^
  = note: `map` is a method of `Iterator`
  = fix: iterate it first: `.iter()`

error[E0324]: the type of `total` cannot be inferred here; add a type annotation
  --> adults.sisu:20:53
   |
20 |     return guests.map(it.age).fold(0, (total, n) -> total + n)
   |                                                     ^^^^^
  = note: a closure's parameters take their types from where the closure is passed; one kept in a `let` names them, as in `(a: int, b: int) -> a + b`

```

<details>
<summary>Hint 1</summary>

Read the first error of each function: the others follow from it.
E0324 (`type.cannot-infer`) appears because the closure given to `fold`
has nothing to take the type of `total` from once `map` is missing.

</details>

<details>
<summary>Hint 2</summary>

The note says whose methods `filter` and `map` are. The section
"Adapters and consumers" says how a list gives one.

</details>

When every test passes, compare your answer with
[`manual/solutions/16-closures-and-iterators/adults.sisu`](../solutions/16-closures-and-iterators/adults.sisu).

## Exercise 16.3: a pipeline over sales

Open
[`manual/exercises/16-closures-and-iterators/sales.sisu`](../exercises/16-closures-and-iterators/sales.sisu):

<!-- file: exercises/16-closures-and-iterators/sales.sisu -->
```sisu
##! Exercise 16.3, a pipeline over sales. Kind: write. Level: 2.
##!
##! A bakery keeps a record of each sale. Write the bodies of the three
##! functions so that every test passes, each as one pipeline of
##! adapters if you can. Run the tests with `sisu test sales.sisu`. The
##! hints are in chapter 16 of the tutorial, under "Exercise 16.3: a
##! pipeline over sales".

## One sale: a product, the town of the shop, and the price in cents.
struct Sale:
    product: String
    town: String
    cents: int

## The total in cents of the sales in `town`.
def total_in(sales: list[Sale], town: str) -> int:
    return todo("total_in")

## The products of the sales of at least `cents`, in capitals, in the
## order of the sales.
def dear(sales: list[Sale], cents: int) -> list[String]:
    return todo("dear")

## The product of the largest sale, the first of them on a tie.
def best_seller(sales: list[Sale]) -> String?:
    return todo("best_seller")

## A morning's sales.
def morning() -> list[Sale]:
    return [
        Sale("pulla", "Turku", 280),
        Sale("kakku", "Tampere", 2400),
        Sale("leipä", "Turku", 590),
        Sale("pulla", "Tampere", 280),
        Sale("piirakka", "Turku", 2400),
    ]

test "a town's total":
    assert total_in(morning(), "Turku") == 3270
    assert total_in(morning(), "Oulu") == 0

test "the dear products":
    assert dear(morning(), 500) == ["KAKKU", "LEIPÄ", "PIIRAKKA"]
    assert dear(morning(), 3000) == []

test "the best seller":
    assert best_seller(morning()) == Some("kakku")
    assert best_seller([]) == None
```

<!-- output: solutions/16-closures-and-iterators/sales.console -->
```console
$ sisu test sales.sisu
in scope at the typed hole `todo("total_in")`:
  sales = [Sale(product="pulla", town="Turku", cents=280), Sale(product="kakku", town="Tampere", cents=2400), Sale(product="leipä", town="Turku", cents=590), Sale(product="pulla", town="Tampere", cents=280), Sale(product="piirakka", town="Turku", cents=2400)]
  town = "Turku"
in scope at the typed hole `todo("dear")`:
  sales = [Sale(product="pulla", town="Turku", cents=280), Sale(product="kakku", town="Tampere", cents=2400), Sale(product="leipä", town="Turku", cents=590), Sale(product="pulla", town="Tampere", cents=280), Sale(product="piirakka", town="Turku", cents=2400)]
  cents = 500
in scope at the typed hole `todo("best_seller")`:
  sales = [Sale(product="pulla", town="Turku", cents=280), Sale(product="kakku", town="Tampere", cents=2400), Sale(product="leipä", town="Turku", cents=590), Sale(product="pulla", town="Tampere", cents=280), Sale(product="piirakka", town="Turku", cents=2400)]
test "a town's total" ... blocked by todo("total_in")
test "the dear products" ... blocked by todo("dear")
test "the best seller" ... blocked by todo("best_seller")
0 passed, 0 failed, 3 blocked
```

*Pulla*, *kakku*, *leipä* and *piirakka* are a sweet bun, a cake, a
loaf and a pie.

<details>
<summary>Hint 1</summary>

Each function keeps some sales and looks at one field of each. Say it
in words first, "the cents of the sales in the town, added up", then
find the adapter or consumer for each step.

</details>

<details>
<summary>Hint 2</summary>

`sum` adds up any iterator of numbers. `max_by` gives the first of the
largest, as an optional borrow of a sale, so `?` can take it out in a
function that returns an optional value.

</details>

When every test passes, compare your answer with
[`manual/solutions/16-closures-and-iterators/sales.sisu`](../solutions/16-closures-and-iterators/sales.sisu).

## Exercise 16.4: primes without end

Open
[`manual/exercises/16-closures-and-iterators/primes.sisu`](../exercises/16-closures-and-iterators/primes.sisu):

<!-- file: exercises/16-closures-and-iterators/primes.sisu -->
```sisu
##! Exercise 16.4, primes without end. Kind: write. Level: 2.
##!
##! A prime is a whole number above 1 that only 1 and itself divide.
##! Write `primes`, a generator that gives every prime in order, without
##! end, so that every test passes. The tests take only as many as they
##! need. Run the tests with `sisu test primes.sisu`. The hints are in
##! chapter 16 of the tutorial, under "Exercise 16.4: primes without
##! end".

## Every prime, from 2 up.
def primes() -> Iter[int]:
    return todo("primes")

test "the first primes":
    let first = primes().take(10).collect()
    assert first == [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]

test "the first prime above a thousand":
    assert primes().find(it > 1000) == Some(1009)

test "the hundredth prime":
    assert primes().nth(99) == Some(541)

test "primes that end in 3":
    let threes = primes().filter(it % 10 == 3).take(4).collect()
    assert threes == [3, 13, 23, 43]
```

<!-- output: solutions/16-closures-and-iterators/primes.console -->
```console
$ sisu test primes.sisu
test "the first primes" ... blocked by todo("primes")
test "the first prime above a thousand" ... blocked by todo("primes")
test "the hundredth prime" ... blocked by todo("primes")
test "primes that end in 3" ... blocked by todo("primes")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

The generator `fibonacci` of the section "Generators" has the shape you
need: a loop without end that yields. A generator has no `return` with
a value, so the line with the hole goes.

</details>

<details>
<summary>Hint 2</summary>

A small function that says whether one number is prime keeps the
generator short. A divisor larger than the square root of `n` has a
partner smaller than it, so the search can stop there.

</details>

When every test passes, compare your answer with
[`manual/solutions/16-closures-and-iterators/primes.sisu`](../solutions/16-closures-and-iterators/primes.sisu).

## Exercise 16.5: a quiz scoreboard

Open
[`manual/exercises/16-closures-and-iterators/scoreboard.sisu`](../exercises/16-closures-and-iterators/scoreboard.sisu):

<!-- file: exercises/16-closures-and-iterators/scoreboard.sisu -->
```sisu
##! Exercise 16.5, a quiz scoreboard. Kind: write. Level: 3.
##!
##! A quiz keeps its scores as text, one line for each team: a name, a
##! colon and the points, such as "Kettu: 12". Blank lines and lines
##! that start with `#` are left out. Write `parse_board`, which reads
##! every line or gives the error of the first bad one, and `ranking`,
##! so that every test passes. Run the tests with
##! `sisu test scoreboard.sisu`. The hints are in chapter 16 of the
##! tutorial, under "Exercise 16.5: a quiz scoreboard".

error BoardError:
    NoColon(line: int)
    BadPoints(line: int, text: String)

## A team and its points.
@derive(Debug, Eq)
struct Score:
    team: String
    points: int

## The scores on the lines of `text`. An error names the first bad
## line, counting every line from 1.
def parse_board(text: str) -> list[Score] ! BoardError:
    return todo("parse_board")

## The teams of `scores` from the most points to the fewest, written as
## "1. Karhu (15)". Teams with as many points keep their order.
def ranking(scores: list[Score]) -> list[String]:
    return todo("ranking")

## The scores of an evening, with a comment and a blank line.
def evening() -> str:
    return "# round 3\nKettu: 12\nKarhu: 15\n\nIlves:  12\n"

test "a board":
    let scores = [
        Score("Kettu", 12),
        Score("Karhu", 15),
        Score("Ilves", 12),
    ]
    assert parse_board(evening()) == Ok(scores)
    assert parse_board("") == Ok([])

test "the first bad line":
    assert parse_board("Kettu: 12\nKarhu 15\nIlves: x") == Err(NoColon(2))
    assert parse_board("Kettu: 12\n\nIlves: x") == Err(BadPoints(3, "x"))

test "a ranking":
    let lines = ranking(parse_board(evening()).or([]))
    assert lines == ["1. Karhu (15)", "2. Kettu (12)", "3. Ilves (12)"]
```

<!-- output: solutions/16-closures-and-iterators/scoreboard.console -->
```console
$ sisu test scoreboard.sisu
in scope at the typed hole `todo("parse_board")`:
  text = "# round 3\nKettu: 12\nKarhu: 15\n\nIlves:  12\n"
in scope at the typed hole `todo("parse_board")`:
  text = "Kettu: 12\nKarhu 15\nIlves: x"
in scope at the typed hole `todo("parse_board")`:
  text = "# round 3\nKettu: 12\nKarhu: 15\n\nIlves:  12\n"
test "a board" ... blocked by todo("parse_board")
test "the first bad line" ... blocked by todo("parse_board")
test "a ranking" ... blocked by todo("parse_board")
0 passed, 0 failed, 3 blocked
```

*Kettu*, *karhu* and *ilves* are Finnish for fox, bear and lynx.

<details>
<summary>Hint 1</summary>

Write a function that reads one line, given its number, and returns a
`Score` or an error. `split_once(":")` cuts the line at its colon.

</details>

<details>
<summary>Hint 2</summary>

`parse_board` goes through the lines with their numbers and leaves out
the blank ones and the comments. The section "Errors inside a pipeline"
says why a loop with `?` gives the first error and a `map` does not.

</details>

<details>
<summary>Hint 3</summary>

`sort_by` is stable, so teams with as many points keep their order.
Sorting the positions of the scores, rather than the scores, leaves the
borrowed list as it is.

</details>

When every test passes, compare your answer with
[`manual/solutions/16-closures-and-iterators/scoreboard.sisu`](../solutions/16-closures-and-iterators/scoreboard.sisu).

## What you learned

- A function is a value of a function type, `(int) -> int`; a closure,
  `x -> expr` or an expression with `it`, is one written where it is
  used, and a `do` block gives a closure statements.
- A closure borrows what it uses; an `own` closure keeps a copy, of a
  `Copy` value in stage 0 (E0423). A `pure` function type admits only
  pure functions (E0335).
- A type with `next` implements `Iterator`, and gets the adapters
  (`map`, `filter`, `take`, `skip`, `enumerate`) and the consumers
  (`collect`, `fold`, `count`, `find`, `any`, `all`, `max_by`, `join`).
  A list gives an iterator with `iter()`.
- Iterators are lazy: an adapter works on an item only when it is
  asked for it, so an endless one is safe to take from.
- A generator yields its items and returns `Iter[T]`.
- `?` in a closure leaves the closure; a loop with `?` stops at the
  first error.

Chapter 17 is about modules and packages: how a program grows into
files, how packages depend on each other, and how a library documents
what it makes public.
