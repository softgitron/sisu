# 5. Collections

In this chapter you keep many values together: in lists, which keep
them in order, in dicts, which find a value by its key, in sets, which
keep each value once, and in tuples, which group a few values of
different types. You build collections with comprehensions, use the
prelude's functions on them, and meet Sisu's first rule of ownership:
a list has one owner. You need about an hour and a half.

## Lists

A list holds values of one type, in order. Square brackets make one:

<!-- file: examples/05-collections/shopping.sisu -->
```sisu
# A list holds values of one type, in order.
def main():
    var shopping = ["milk", "rye bread", "cloudberries"]
    shopping.push("coffee")
    print("{len(shopping)} things: {shopping}")
    print("first {shopping[0]}, last {shopping[^1]}")
    shopping[1] = "oat bread"
    print("now the second is {shopping[1]}")
```

<!-- output: examples/05-collections/shopping.out -->
```text
4 things: ["milk", "rye bread", "cloudberries", "coffee"]
first milk, last coffee
now the second is oat bread
```

- `["milk", "rye bread", "cloudberries"]` is a `list[str]`: a list
  whose items are `str` values. Sisu works out the item type from the
  items, and every item must have it.
- The list changes, so its binding is a `var`. `push` adds an item at
  the end.
- `len(shopping)` is the number of items.
- `shopping[0]` is the first item: indices count from 0.
- `shopping[^1]` is the last item, read aloud as "the last element of
  shopping". `^2` is the one before it, and so on.
- `shopping[1] = "oat bread"` replaces the item at index 1.

An index outside the list stops the program, with the length and the
index in the report:

<!-- file: examples/05-collections/out_of_range.sisu -->
```sisu
# An index past the end stops the program.
def main():
    let shopping = ["milk", "rye bread", "cloudberries"]
    let wanted = 3
    print("number {wanted} is {shopping[wanted]}")
```

<!-- output: examples/05-collections/out_of_range.console -->
```console
$ sisu run out_of_range.sisu
panic: index out of bounds: the length is 3 but the index is 3
  at main (out_of_range.sisu:5:32)
```

Index 3 is the fourth item, and the list has three. Chapter 8 shows
`get`, which answers "no such item" without stopping the program.

## Counting from the end

Python counts from the end with negative indices: `xs[-1]` is the last
item. In Sisu that spelling does not compile, because a negative index
that was meant to count from the start, such as `xs[i - 1]` when `i` is
0, would quietly read the other end of the list. The spelling for
counting from the end is `^`.

**Try it.** Write `shopping[-1]` in place of `shopping[^1]` in
`shopping.sisu` and run it.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/05-collections/minus_one.console -->
```console
$ sisu run minus_one.sisu
error[E0344]: a constant negative index does not compile; count from the end with `^`: `shopping[^1]`
  --> minus_one.sisu:6:48
  |
6 |     print("first {shopping[0]}, last {shopping[-1]}")
  |                                                ^^
  = fix: count from the end with `^`: `^1`

error[D0006]: the program was not run: errors found (1)
```

E0344 (`type.constant-negative-index`) rejects a negative number
written in an index, and its fix writes the `^` form. An index that is
computed while the program runs and turns out negative stops the
program, as an index past the end does.

</details>

> **From Python.** `xs[-1]` is the last item in Python and does not
> compile in Sisu; write `xs[^1]`. Sisu keeps a familiar spelling only
> when it keeps the familiar meaning (D-062), so a Python habit either
> works the same or fails to compile.

## Slices

`xs[i:j]` is a new list of the items from index `i` up to `j`, without
`j`. Either end can be left out:

<!-- file: examples/05-collections/slices.sisu -->
```sisu
#!/usr/bin/env sisu
# A slice copies a run of a list's elements.
let week = [-3, -1, 2, 5, 4, 1, -2]
week[0:5]
week[5:]
week[:2]
len(week[2:4])
```

<!-- output: examples/05-collections/slices.ws -->
```text
#!/usr/bin/env sisu
# A slice copies a run of a list's elements.
let week = [-3, -1, 2, 5, 4, 1, -2]  # => [-3, -1, 2, 5, 4, 1, -2]
week[0:5]  # => [-3, -1, 2, 5, 4]
week[5:]  # => [1, -2]
week[:2]  # => [-3, -1]
len(week[2:4])  # => 2
```

The list holds a week of temperatures. `week[0:5]` is the five
weekdays, `week[5:]` runs from index 5 to the end, and `week[:2]` from
the start up to index 2. Counting from the end in a slice, as in
`week[:^1]`, is designed in
[02](../../proposal/02-syntax.md#slicing-and-indexing) and is not in
the compiler yet.

## Dicts

A dict maps keys to values. Each key appears once, and looking a key up
finds its value:

<!-- file: examples/05-collections/ages.sisu -->
```sisu
# A dict maps keys to values.
def main():
    var ages = {"Aino": 34, "Eero": 29}
    ages["Roni"] = 41
    print("{ages}")
    print("Eero is {ages["Eero"]}")
    print("{ages.get("Eero")} {ages.get("Liisa")}")
    for name, age in ages:
        print("{name} is {age}")
```

<!-- output: examples/05-collections/ages.out -->
```text
{"Aino": 34, "Eero": 29, "Roni": 41}
Eero is 29
Some(29) None
Aino is 34
Eero is 29
Roni is 41
```

- `{"Aino": 34, "Eero": 29}` is a `dict[str, int]`: its keys are `str`
  values and its values are `int` values.
- `ages["Roni"] = 41` adds a key with its value, or replaces the value
  if the key is there already.
- `ages["Eero"]` is the value for the key `"Eero"`.
- `ages.get("Eero")` is an optional value: `Some(29)` when the key is
  there, and `None` when it is not. Chapter 8 shows how to use one.
- `for name, age in ages` goes through the keys and values in the order
  the keys were added.

Looking up a key that is not there with square brackets stops the
program:

<!-- file: examples/05-collections/missing_key.sisu -->
```sisu
# Looking up a key that is not there stops the program.
def main():
    let ages = {"Aino": 34, "Eero": 29}
    print("Liisa is {ages["Liisa"]}")
```

<!-- output: examples/05-collections/missing_key.console -->
```console
$ sisu run missing_key.sisu
panic: the key "Liisa" is not in the dict
  at main (missing_key.sisu:4:22)
```

Use `in` to ask first, or `get` when the key may be missing.

> **From Python.** A Python dict also keeps its keys in the order they
> were added. A missing key raises `KeyError` in Python and panics in
> Sisu; `get` is the way to look up a key that may be missing.

## Counting with a dict

Counting how often each value appears is a common job for a dict. One
line does it:

<!-- file: examples/05-collections/counting.sisu -->
```sisu
# `entry(key).or_insert(0) += 1` counts how often each key appears.
def main():
    let pets = ["kissa", "koira", "kissa", "kala", "kissa"]
    var counts: dict[str, int] = {}
    for pet in pets:
        counts.entry(pet).or_insert(0) += 1
    print("{counts}")
```

<!-- output: examples/05-collections/counting.out -->
```text
{"kissa": 3, "koira": 1, "kala": 1}
```

*Kissa*, *koira* and *kala* are Finnish for cat, dog and fish.
`counts.entry(pet)` finds the place in the dict for the key `pet`.
`.or_insert(0)` puts 0 there if the key is new, and gives the place
either way, and `+= 1` adds one to what is in it. An empty dict has no
items to work its types out from, so `counts` has a type annotation.

## Sets

A set holds each value once. Braces without colons make one:

<!-- file: examples/05-collections/cities.sisu -->
```sisu
# A set holds each value once.
def main():
    var visited = {"Oulu", "Turku"}
    visited.add("Oulu")
    visited.add("Tampere")
    print("{visited}, {len(visited)} cities")
    let none_yet: set[str] = set()
    print("{none_yet}, {len(none_yet)} cities")
```

<!-- output: examples/05-collections/cities.out -->
```text
{"Oulu", "Turku", "Tampere"}, 3 cities
set(), 0 cities
```

Adding `"Oulu"` a second time changes nothing, so the set has three
cities. A set keeps its values in the order they were first added. `{}`
is an empty dict, so an empty set is written `set()`, as in Python.

## Tuples

A tuple groups a fixed number of values, which can have different
types. Parentheses make one:

<!-- file: examples/05-collections/tuples.sisu -->
```sisu
# A tuple groups a fixed number of values of any types.
def main():
    let city = ("Oulu", 214_000)
    print("{city.0} has {city.1} people")
    let name, people = city
    print("{name}: {people}")
    match [3, 1, 4, 1]:
        case [first, *rest]:
            print("{first}, then {rest}")
        case []:
            print("nothing")
```

<!-- output: examples/05-collections/tuples.out -->
```text
Oulu has 214000 people
Oulu: 214000
3, then [1, 4, 1]
```

- `("Oulu", 214_000)` is a `(str, int)`. `city.0` and `city.1` are its
  values, by position.
- `let name, people = city` unpacks the tuple into two bindings, as
  `for` did with the pairs of `enumerate` in chapter 4. Chapter 3 showed
  tuples with labels, which a function can return.
- In a `match`, the list pattern `[first, *rest]` takes a list apart:
  `first` is its first item and `rest` a list of the others. The
  pattern `[]` matches an empty list.

Python's star unpacking in a binding, `let first, *rest = xs`, is
designed in [02](../../proposal/02-syntax.md#declarations) and is not
in the compiler yet; the list pattern in a `match` does the same work.

## Asking with in

`in` asks whether a collection holds a value, and `not in` asks the
opposite:

<!-- file: examples/05-collections/membership.sisu -->
```sisu
#!/usr/bin/env sisu
# `in` asks whether a collection holds a value.
let primes = [2, 3, 5, 7]
5 in primes
4 in primes
4 not in primes
let ages = {"Aino": 34, "Eero": 29}
"Aino" in ages
"in" in "Aino"
```

<!-- output: examples/05-collections/membership.ws -->
```text
#!/usr/bin/env sisu
# `in` asks whether a collection holds a value.
let primes = [2, 3, 5, 7]  # => [2, 3, 5, 7]
5 in primes  # => True
4 in primes  # => False
4 not in primes  # => True
let ages = {"Aino": 34, "Eero": 29}  # => {"Aino": 34, "Eero": 29}
"Aino" in ages  # => True
"in" in "Aino"  # => True
```

On a list or a set, `in` looks at the items. On a dict it looks at the
keys, so `"Aino" in ages` is `True`, and asking for an `int` among keys
that are `str` values does not compile. On text, `in` looks for a run of
characters: `"in"` is part of `"Aino"`.

## Comprehensions

A comprehension builds a collection from a loop, in one expression:

<!-- file: examples/05-collections/comprehensions.sisu -->
```sisu
#!/usr/bin/env sisu
# Comprehensions build a list, a set or a dict from a loop.
let scores = [7, 9, 4, 9]
[s * 10 for s in scores]
[s for s in scores if s > 5]
{s for s in scores}
{name: len(name) for name in ["Aino", "Liisa"]}
```

<!-- output: examples/05-collections/comprehensions.ws -->
```text
#!/usr/bin/env sisu
# Comprehensions build a list, a set or a dict from a loop.
let scores = [7, 9, 4, 9]  # => [7, 9, 4, 9]
[s * 10 for s in scores]  # => [70, 90, 40, 90]
[s for s in scores if s > 5]  # => [7, 9, 9]
{s for s in scores}  # => {7, 9, 4}
{name: len(name) for name in ["Aino", "Liisa"]}  # => {"Aino": 4, "Liisa": 5}
```

- `[s * 10 for s in scores]` is a list of one value for each item.
- `[s for s in scores if s > 5]` keeps only the items for which the
  condition holds.
- Braces make a set, `{s for s in scores}`, or a dict when the value has
  a key before a colon, `{name: len(name) for name in ...}`.

> **From Python.** Comprehensions are written as in Python. A
> comprehension in parentheses, Python's generator expression, is
> designed in [02](../../proposal/02-syntax.md#literals) and is not in
> the compiler yet.

## Functions for collections

The prelude has functions that work on any collection:

<!-- file: examples/05-collections/collection_functions.sisu -->
```sisu
#!/usr/bin/env sisu
# The prelude's functions for collections.
let scores = [7, 9, 4, 9]
len(scores)
sorted(scores)
sum(scores)
max(4, 9)
min(4, 9)
any([s > 8 for s in scores])
all([s > 5 for s in scores])
```

<!-- output: examples/05-collections/collection_functions.ws -->
```text
#!/usr/bin/env sisu
# The prelude's functions for collections.
let scores = [7, 9, 4, 9]  # => [7, 9, 4, 9]
len(scores)  # => 4
sorted(scores)  # => [4, 7, 9, 9]
sum(scores)  # => 29
max(4, 9)  # => 9
min(4, 9)  # => 4
any([s > 8 for s in scores])  # => True
all([s > 5 for s in scores])  # => False
```

- `sorted` gives a new list in ascending order, and leaves the original
  as it was.
- `sum` adds the items up.
- `any` is `True` when at least one item is `True`, and `all` when every
  item is. Each is given a list of `bool` values, here built by a
  comprehension.
- `max` and `min` give the larger or the smaller of two values, and,
  given one collection, its largest or smallest item, the first of
  equal ones.

A list with no items has no largest item, so `max` of an empty list
stops the program:

<!-- file: examples/05-collections/max_of_list.sisu -->
```sisu
# `max` and `min` of a list, and of a list with no items.
def main():
    let scores = [7, 9, 4]
    print("best {max(scores)}, worst {min(scores)}")
    let nobody: list[int] = []
    print("best of nobody {max(nobody)}")
```

<!-- output: examples/05-collections/max_of_list.console -->
```console
$ sisu run max_of_list.sisu
best 9, worst 4
panic: `max` of no items
  at main (max_of_list.sisu:6:28)
```

The panic names the call, and the program ends there. Where a list may
be empty, test it first, with `if not nobody.is_empty():`.

> **From Python.** `max` and `min` work as Python's do, on two values or
> on a collection, and an empty collection is an error in both: Python
> raises `ValueError`, and Sisu panics, since a program that asks for
> the largest of nothing has a bug. Python's `key=` and `default=` are
> not parameters: `max_by` and `min_by` of chapter 16 take a key.

Three more functions go through collections for a `for` loop:

<!-- file: examples/05-collections/pairs.sisu -->
```sisu
# `zip` pairs two lists; `reversed` goes backwards.
def main():
    let names = ["Aino", "Eero", "Roni"]
    let ages = [34, 29, 41]
    for name, age in zip(names, ages):
        print("{name} is {age}")
    for name, age in zip_shortest(names, [34, 29]):
        print("{name} is {age}")
    for name in reversed(names):
        print(name)
```

<!-- output: examples/05-collections/pairs.out -->
```text
Aino is 34
Eero is 29
Roni is 41
Aino is 34
Eero is 29
Roni
Eero
Aino
```

`zip` pairs the items of two lists, and `zip_shortest` does the same
until the shorter list ends. `reversed` goes through the items from the
last to the first.

**Try it.** Give `zip` two lists of different lengths.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/05-collections/zip_unequal.console -->
```console
$ sisu run zip_unequal.sisu
Aino is 34
Eero is 29
panic: zip: the arguments have different lengths
  at main (zip_unequal.sisu:5:5)
```

`zip` pairs items until one list runs out, and then stops the program,
because two lists that should pair up but do not are usually a bug. Use
`zip_shortest` when different lengths are expected.

</details>

## Arrays of fixed length

A list grows and shrinks. An `Array` has a fixed number of items, and
that number is part of its type:

<!-- file: examples/05-collections/arrays.sisu -->
```sisu
# An `Array` has a length that is part of its type.
def main():
    var rgb: Array[int, 3] = Array.filled(0)
    rgb[0] = 255
    rgb[2] = 64
    print("{rgb}, {len(rgb)} values")
```

<!-- output: examples/05-collections/arrays.out -->
```text
[255, 0, 64], 3 values
```

`Array[int, 3]` is three `int` values, no more and no fewer.
`Array.filled(0)` makes one with every item 0, and an index assignment
changes an item. Use an array when the number of items is part of what
the data is, such as the three parts of a colour, and a list otherwise.
A list literal that becomes an array, `let rgb: Array[int, 3] = [255,
0, 64]`, is designed in
[03](../../proposal/03-types-and-generics.md#type-inference) and is not
in the compiler yet.

## A list has one owner

Every value in Sisu has one owner, which is the binding that holds it.
For a number, a `bool` or a `str`, giving the value to another binding
gives it a copy. A list can be long, so Sisu does not copy it unless
you ask. Giving it to another binding moves it, and the old binding can
no longer be used:

<!-- file: examples/05-collections/moved.sisu -->
```sisu
# Assigning a list to another name moves it.
def main():
    let scores = [7, 9, 4]
    var ranked = scores
    ranked.sort()
    print("ranked: {ranked}")
    print("as entered: {scores}")
```

<!-- output: examples/05-collections/moved.console -->
```console
$ sisu run moved.sisu
error[E0400]: `scores` was moved on line 4 and cannot be used afterwards
  --> moved.sisu:7:25
  |
7 |     print("as entered: {scores}")
  |                         ^^^^^^
  |
4 |     var ranked = scores
  |                  ------ `scores` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0400 (`borrow.use-after-move`) points at the use and, on line 4, at
the move. Its note gives the fixes and what each costs. `.copy()` makes
a second list, so that each binding owns one:

<!-- file: examples/05-collections/copied.sisu -->
```sisu
# `.copy()` makes a second list, so both names can be used.
def main():
    let scores = [7, 9, 4]
    var ranked = scores.copy()
    ranked.sort()
    print("ranked: {ranked}")
    print("as entered: {scores}")
```

<!-- output: examples/05-collections/copied.out -->
```text
ranked: [4, 7, 9]
as entered: [7, 9, 4]
```

A copy costs time and memory in proportion to the list's length, which
is why Sisu asks you to write it. The other fix, "move it later", means
using `scores` before the line that moves it.

Passing a list to a function does not move it. The function borrows the
list for the call, and the caller keeps it:

<!-- file: examples/05-collections/borrowed.sisu -->
```sisu
# Passing a list to a function lends it; the caller keeps it.
def total(scores: list[int]) -> int:
    var sum_so_far = 0
    for score in scores:
        sum_so_far += score
    return sum_so_far

def main():
    let scores = [7, 9, 4]
    print("total: {total(scores)}")
    print("scores: {scores}")
```

<!-- output: examples/05-collections/borrowed.out -->
```text
total: 20
scores: [7, 9, 4]
```

Moves and borrows are the heart of Part II of this tutorial, which
starts with chapter 10.

> **From Python.** In Python, `ranked = scores` makes two names for one
> list, and sorting through one name sorts what the other sees. Sisu
> never lets two bindings own one list: the list moves, or `.copy()`
> makes two lists.

## Exercise 5.1: predict the collections

Open
[`manual/exercises/05-collections/predict_collections.sisu`](../exercises/05-collections/predict_collections.sisu):

<!-- file: exercises/05-collections/predict_collections.sisu -->
```sisu
##! Exercise 5.1, predict the collections. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_collections.sisu`. The hints are in chapter 5 of
##! the tutorial, under "Exercise 5.1: predict the collections".

const DISTINCT: int = todo("the size of a set")
const LARGEST: int = todo("the last of a sorted list")
const SLICE_SUM: int = todo("the sum of a slice")
const EVENS: int = todo("the length of a comprehension")
const FOUND: bool = todo("a key in a dict")

test "a set of repeated numbers":
    assert len({3, 1, 3, 2, 1}) == DISTINCT

test "the last of a sorted list":
    assert sorted([5, 2, 8, 1])[^1] == LARGEST

test "the sum of a slice":
    assert sum([4, 8, 15, 16, 23][1:3]) == SLICE_SUM

test "a comprehension with a condition":
    assert len([n for n in range(10) if n % 2 == 0]) == EVENS

test "a name in lowercase among the keys of a dict":
    assert ("oulu" in {"Turku": 1, "Oulu": 2}) == FOUND
```

<!-- output: solutions/05-collections/predict_collections.console -->
```console
$ sisu test predict_collections.sisu
test "a set of repeated numbers" ... blocked by todo("the size of a set")
test "the last of a sorted list" ... blocked by todo("the last of a sorted list")
test "the sum of a slice" ... blocked by todo("the sum of a slice")
test "a comprehension with a condition" ... blocked by todo("the length of a comprehension")
test "a name in lowercase among the keys of a dict" ... blocked by todo("a key in a dict")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

Write out each collection that an expression builds before you count
or add anything.

</details>

<details>
<summary>Hint 2</summary>

A set keeps each value once, a slice stops before its end index, and
`in` on a dict looks at its keys, comparing text exactly.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/05-collections/predict_collections.sisu`](../solutions/05-collections/predict_collections.sisu).

## Exercise 5.2: the team roster

Open
[`manual/exercises/05-collections/roster.sisu`](../exercises/05-collections/roster.sisu):

<!-- file: exercises/05-collections/roster.sisu -->
```sisu
##! Exercise 5.2, the team roster. Kind: fix. Level: 1.
##!
##! `roster` shows a team's members in the order they joined and in
##! the order of their names. `join(" ")` puts a list's items into one
##! text with spaces between them. The file does not compile. Check it
##! with `sisu check roster.sisu`, read the diagnostic, and fix the
##! function so that `sisu test roster.sisu` passes every test. The
##! hints are in chapter 5 of the tutorial, under "Exercise 5.2: the
##! team roster".

## The three members in the order they joined, then sorted by name.
def roster(first: str, second: str, third: str) -> String:
    let joined = [first, second, third]
    var by_name = joined
    by_name.sort()
    return "{joined.join(" ")} / {by_name.join(" ")}"

test "a team of three":
    let text = roster("Roni", "Aino", "Eero")
    assert text == "Roni Aino Eero / Aino Eero Roni"

test "a team that joined in the order of their names":
    let text = roster("Aino", "Eero", "Liisa")
    assert text == "Aino Eero Liisa / Aino Eero Liisa"
```

Check it before you change anything:

<!-- output: solutions/05-collections/roster.console -->
```console
$ sisu check roster.sisu
error[E0400]: `joined` was moved on line 14 and cannot be used afterwards
  --> roster.sisu:16:14
   |
16 |     return "{joined.join(" ")} / {by_name.join(" ")}"
   |              ^^^^^^
   |
14 |     var by_name = joined
   |                   ------ `joined` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

```

<details>
<summary>Hint 1</summary>

The function needs two lists in the end: one in the order of joining
and one sorted. Count how many lists it has.

</details>

<details>
<summary>Hint 2</summary>

The first of the ways that the note names is the fix here: a second
list that `by_name` owns, made before anything is sorted.

</details>

When every test passes, compare your answer with
[`manual/solutions/05-collections/roster.sisu`](../solutions/05-collections/roster.sisu).

## Exercise 5.3: word frequencies

Open
[`manual/exercises/05-collections/word_frequencies.sisu`](../exercises/05-collections/word_frequencies.sisu):

<!-- file: exercises/05-collections/word_frequencies.sisu -->
```sisu
##! Exercise 5.3, word frequencies. Kind: write. Level: 1.
##!
##! Write the body of `word_frequencies` so that every test passes. Run
##! the tests with `sisu test word_frequencies.sisu`. The hints are in
##! chapter 5 of the tutorial, under "Exercise 5.3: word frequencies".

## How many times each word appears in `words`, with the words in the
## order of their first appearance.
def word_frequencies(words: list[str]) -> dict[str, int]:
    return todo("word_frequencies")

test "every word once":
    let counts = word_frequencies(["kuu", "kissa", "kala"])
    assert counts == {"kuu": 1, "kissa": 1, "kala": 1}

test "repeated words":
    let counts = word_frequencies(["a", "b", "a", "c", "a", "b"])
    assert counts == {"a": 3, "b": 2, "c": 1}

test "no words":
    assert len(word_frequencies([])) == 0
```

<!-- output: solutions/05-collections/word_frequencies.console -->
```console
$ sisu test word_frequencies.sisu
in scope at the typed hole `todo("word_frequencies")`:
  words = ["kuu", "kissa", "kala"]
in scope at the typed hole `todo("word_frequencies")`:
  words = ["a", "b", "a", "c", "a", "b"]
in scope at the typed hole `todo("word_frequencies")`:
  words = []
test "every word once" ... blocked by todo("word_frequencies")
test "repeated words" ... blocked by todo("word_frequencies")
test "no words" ... blocked by todo("word_frequencies")
0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

Start from an empty dict with a type annotation, go through the words
with `for`, and return the dict.

</details>

<details>
<summary>Hint 2</summary>

The section "Counting with a dict" counts with one line in the loop.

</details>

When every test passes, compare your answer with
[`manual/solutions/05-collections/word_frequencies.sisu`](../solutions/05-collections/word_frequencies.sisu).

## Exercise 5.4: the second largest value

Open
[`manual/exercises/05-collections/second_largest.sisu`](../exercises/05-collections/second_largest.sisu):

<!-- file: exercises/05-collections/second_largest.sisu -->
```sisu
##! Exercise 5.4, the second largest value. Kind: write. Level: 2.
##!
##! Write the body of `second_largest` so that every test passes. Run
##! the tests with `sisu test second_largest.sisu`. The hints are in
##! chapter 5 of the tutorial, under "Exercise 5.4: the second largest
##! value".

## The second largest of `values`, counting each value once: for
## [5, 9, 9, 3] it is 5. `values` holds at least two different values.
def second_largest(values: list[int]) -> int:
    return todo("second_largest")

test "two values":
    assert second_largest([1, 2]) == 1
    assert second_largest([2, 1]) == 1

test "the largest value twice":
    assert second_largest([5, 9, 9, 3]) == 5

test "negative values":
    assert second_largest([-4, -8, -4]) == -8

test "many repeats":
    assert second_largest([7, 3, 7, 7, 1, 3]) == 3
```

<!-- output: solutions/05-collections/second_largest.console -->
```console
$ sisu test second_largest.sisu
in scope at the typed hole `todo("second_largest")`:
  values = [1, 2]
in scope at the typed hole `todo("second_largest")`:
  values = [5, 9, 9, 3]
in scope at the typed hole `todo("second_largest")`:
  values = [-4, -8, -4]
in scope at the typed hole `todo("second_largest")`:
  values = [7, 3, 7, 7, 1, 3]
test "two values" ... blocked by todo("second_largest")
test "the largest value twice" ... blocked by todo("second_largest")
test "negative values" ... blocked by todo("second_largest")
test "many repeats" ... blocked by todo("second_largest")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

The largest value can appear more than once, and it counts once. Which
collection keeps each value once?

</details>

<details>
<summary>Hint 2</summary>

Once each value appears once and they are in order, the answer is at a
fixed place counted from the end.

</details>

When every test passes, compare your answer with
[`manual/solutions/05-collections/second_largest.sisu`](../solutions/05-collections/second_largest.sisu).

## What you learned

- A list keeps values of one type in order. `xs[0]` is the first item
  and `xs[^1]` the last; `xs[-1]` does not compile. `xs[i:j]` is a new
  list of a run of items.
- A dict maps keys to values in the order they were added, and
  `entry(k).or_insert(0) += 1` counts. A set keeps each value once. A
  tuple groups a fixed number of values, read by position or unpacked.
- `in` asks whether a collection holds a value. Comprehensions build
  lists, sets and dicts. `len`, `sorted`, `sum`, `any`, `all`, `zip`,
  `zip_shortest`, `reversed`, `max` and `min` work on collections;
  `max` and `min` also compare two values.
- An `Array` has a fixed length that is part of its type.
- A list has one owner. Assigning it moves it, `.copy()` makes a second
  one, and passing it to a function lends it.

Chapter 6 is about text: `str` and `String`, format specs, the methods
that cut and search text, and characters.
