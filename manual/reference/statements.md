# Statements and control flow

This page states what each statement does: assignment and the places it
changes, the forms of `if` and its conditions, the loops and the
statements that leave them, `return`, `with` and `defer`, generators,
typed holes, and the statements that other pages describe in full
(`match`, `select`, `assert`, `unsafe`). Bindings are
[R3](declarations.md#bindings), and expressions
[R4](expressions.md).

**See also:** the tutorial's [chapter 4](../tutorial/04-control-flow.md)
(control flow), [chapter 8](../tutorial/08-errors.md) (`if let`,
`let ... else`, `while let`), [chapter 10](../tutorial/10-ownership.md)
(`with`, `defer`) and [chapter 16](../tutorial/16-closures-and-iterators.md)
(generators); the proposal's [02, "Control
flow"](../../proposal/02-syntax.md#control-flow); [R17,
Diagnostics](diagnostics.md#parsing) for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. What control flow leaves out is decided (D-021).

## Assignment

`PLACE = VALUE` and the augmented forms, `PLACE += VALUE` and the others
of [R1](lexical-structure.md#operators-and-punctuation), change a place:

- a `var` binding, or a `mut` parameter;
- a field of a place, `t.total`, or an element or a key of one,
  `xs[^1]`, `grid[1][0]`, `words[k]`;
- a call that gives `ref mut`, as `words.entry(k).or_insert(0) += 1`.

A tuple of places takes a tuple, `a, b = b, a`. Anything else is E0313;
a `let` binding is E0312, whose fix declares it `var`, and a place
reached through a plain borrow is E0341 ([R17](diagnostics.md#e0341)).
An assignment is a statement and has no value (E0114).

<!-- file: examples/reference-statements/assignment.sisu -->
```sisu
# An assignment changes a place: a `var`, a field, an element or a key.
struct Tally:
    total: int

def main():
    var count = 0
    count += 2
    var a, b = 1, 2
    a, b = b, a
    var t = Tally(0)
    t.total += 5
    var xs = [1, 2, 3]
    xs[^1] = 30
    var words: dict[String, int] = {}
    for w in ["sauna", "kiuas", "sauna"]:
        words.entry(w.to_string()).or_insert(0) += 1
    print("{count} {a} {b} {t.total} {xs} {words}")
```

<!-- output: examples/reference-statements/assignment.out -->
```text
2 2 1 5 [1, 2, 30] {"sauna": 2, "kiuas": 1}
```

<!-- file: examples/reference-statements/not_a_place.sisu -->
```sisu
# A value that is not a place cannot be assigned to.
def total() -> int:
    return 3

def main():
    total() = 4
    print(total())
```

<!-- output: examples/reference-statements/not_a_place.console -->
```console
$ sisu run not_a_place.sisu
error[E0313]: this expression cannot be assigned to
  --> not_a_place.sisu:6:5
  |
6 |     total() = 4
  |     ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0.

## Conditions

- `if CONDITION:` runs its block when the condition is `True`; `elif`
  and `else` follow it. A condition is a `bool` (E0302).
- `if let PATTERN = VALUE:` runs its block when the pattern matches,
  with its names bound there ([R6](patterns.md#where-patterns-stand)).
- A condition may chain `let` patterns and tests with commas, `if let
  Some(age) = ages.get(name), age >= 18:`; the parts run left to right,
  each binding is in scope for the rest and for the block, and the chain
  fails at the first part that does not hold (leaning, D-071).
- `let PATTERN = VALUE else:` binds the pattern's names for the rest
  of the block, or runs its `else` block, which must leave the scope with
  `return`, `break`, `continue` or a panic (E0331); a value that cannot
  fail is E0330.

<!-- file: examples/reference-statements/conditions.sisu -->
```sisu
# `if` with `elif`, `if let`, chained conditions and `let ... else`.
def describe(ages: dict[str, int], name: str) -> String:
    if let Some(age) = ages.get(name), age >= 18:
        return "{name} is an adult"
    elif name in ages:
        return "{name} is a child"
    else:
        return "who is {name}?"

def first_word(text: str) -> String:
    let Some(word) = text.split_whitespace().next() else:
        return "nothing"
    return word.to_string()

def main():
    let ages = {"Aino": 7, "Eero": 30}
    print("{describe(ages, "Eero")}; {describe(ages, "Aino")}")
    print("{describe(ages, "Roni")}; {first_word("hei maailma")}")
    var stack = [1, 2]
    while let Some(top) = stack.pop():
        print("popped {top}")
```

<!-- output: examples/reference-statements/conditions.out -->
```text
Eero is an adult; Aino is a child
who is Roni?; hei
popped 2
popped 1
```

**Status.** In stage 0 (D-071 leans for chains).

## Loops

- `while CONDITION:` runs its block while the condition holds, and
  `while let PATTERN = VALUE:` while the pattern matches, the condition
  chaining as `if`'s does.
- `for PATTERN in VALUE:` runs its block once for each item: `for x in
  xs` borrows each, `for x in mut xs` lets the block change each, and
  `for x in own xs` takes each, consuming `xs`
  ([R7](ownership.md#iterating-in-three-modes)). A value that cannot be
  iterated is E0325 ([R17](diagnostics.md#e0325)).
- `range(stop)`, `range(start, stop)` and `range(start, stop, step)` are
  the integers from `start` up to `stop`, without it, by `step`, which
  may be negative and not zero (R0021); `enumerate(xs)` pairs each item
  with its position.
- `break` leaves the innermost loop and `continue` goes to its next
  turn; either outside a loop is E0322 or E0323.
- Each turn of a `for` binds its names anew, so a closure made in one
  turn keeps that turn's value.

<!-- file: examples/reference-statements/loops.sisu -->
```sisu
# `while` and `for`, with `range`, `enumerate`, `break` and `continue`.
def main():
    var n = 27
    var steps = 0
    while n != 1:
        n = if n % 2 == 0: n // 2 else: 3 * n + 1
        steps += 1
    print("{steps} steps")
    for i in range(10, 0, -3):
        print(i)
    for i, word in enumerate(["sauna", "", "kiuas", "stop", "x"]):
        if word == "":
            continue
        if word == "stop":
            break
        print("{i}: {word}")
```

<!-- output: examples/reference-statements/loops.out -->
```text
111 steps
10
7
4
1
0: sauna
2: kiuas
```

**Status.** In stage 0.

## What control flow leaves out

Sisu has no `switch` (use `match`), no `do-while` (use `while True:`
with a `break`), no C `for` (use `range`), no `else` on a loop, no loop
labels (put the inner loop in a function and `return`) and no `goto`
(D-021). A loop's `else` is E0127, whose message names the way to write
it:

<!-- file: examples/reference-statements/loop_else.sisu -->
```sisu
# Python's `else` on a loop is not Sisu.
def main():
    for name in ["Aino", "Eero"]:
        if name == "Roni":
            break
    else:
        print("no Roni")
```

<!-- output: examples/reference-statements/loop_else.console -->
```console
$ sisu run loop_else.sisu
error[E0127]: a loop has no `else` in Sisu; keep what the loop finds in a variable and test it after the loop, as in `if let Some(i) = found:`
  --> loop_else.sisu:6:5
  |
6 |     else:
  |     ^^^^

error[D0006]: the program was not run: errors found (1)
```

**Status.** Decided (D-021).

## Return

`return VALUE` ends the function with its result, and `return` alone
ends one that returns nothing; it is the only way a function gives a
value, since a block has none (D-056). A function with a result whose
end can be reached is E0315 ([R17](diagnostics.md#e0315)); a `return`
with a value where none is returned is E0316, and one without where a
value is needed E0317. In a closure, `return` returns from the closure.

**Status.** In stage 0 (D-056).

## With and defer

- `with VALUE as NAME:` binds the value for its block and drops it when
  the block ends, so a file or a lock is released at a line a reader
  can see ([R7](ownership.md#drops-and-their-order)).
- `defer STATEMENT` and `defer:` with a block run when the enclosing
  block ends, however it ends, the last deferred first; a `defer` is
  scoped to its block, not to the function.

<!-- file: examples/reference-statements/scoped.sisu -->
```sisu
# `with` drops its value when its block ends; `defer` runs at the end.
struct Door:
    name: String

impl Drop for Door:
    def drop(mut self):
        print("{self.name} closes")

def main():
    defer print("the evening ends")
    with Door("sauna") as door:
        print("in through the {door.name} door")
        defer:
            print("the towel is hung")
        print("on the bench")
    print("out again")
```

<!-- output: examples/reference-statements/scoped.out -->
```text
in through the sauna door
on the bench
the towel is hung
sauna closes
out again
the evening ends
```

**Stage 0 differs.** `with VALUE:` without `as NAME`, Python's `with
lock:`, is E0100 ("expected `as`").

**Status.** In stage 0 (D-008).

## Generators

A function whose body contains `yield VALUE` is a generator: it returns
an `Iter[T]` (E0436 otherwise), and its body runs only as its items are
asked for, up to each `yield`, ending when the body ends. `yield`
stands only in such a body, not in a closure (E0435). A generator
dropped before its end runs its drops and `defer`s at once.

<!-- file: examples/reference-statements/generators.sisu -->
```sisu
# A function that yields is a generator, which runs as items are asked.
def countdown(n: int) -> Iter[int]:
    var i = n
    while i > 0:
        yield i
        i -= 1

def main():
    for x in countdown(3):
        print(x)
    print(countdown(10).take(2).collect())
```

<!-- output: examples/reference-statements/generators.out -->
```text
3
2
1
[10, 9]
```

**Status.** In stage 0.

## Typed holes

`todo("NAME")` is a typed hole: an expression that checks as whatever
type is expected where it stands, so the code around it compiles
(D-061). `sisu check` and `sisu run` report each as W0001 with its type
([R17](diagnostics.md#w0001)); a run that reaches one prints its closure,
each binding in scope with its value (R0160), and stops there (R0013),
and a test that reaches one is blocked rather than failed. A release build rejects a program with a hole (E0445), and
`sisu publish` will.

<!-- file: examples/reference-statements/holes.sisu -->
```sisu
# A typed hole checks as the type expected and stops the run if reached.
def median(xs: list[int]) -> int:
    let s = sorted(xs)
    if len(s) == 1:
        return s[0]
    return todo("middle")

def main():
    print(median([4]))
    print(median([4, 1, 3]))
```

<!-- output: examples/reference-statements/holes.console -->
```console
$ sisu run holes.sisu
warning[W0001]: typed hole `middle` of type `int`
  --> holes.sisu:6:12
  |
6 |     return todo("middle")
  |            ^^^^^^^^^^^^^^

4
in scope at the typed hole `todo("middle")`:
  xs = [4, 1, 3]
  s = [1, 3, 4]
panic: reached the typed hole `todo("middle")`
  at median (holes.sisu:6:12)
  at main (holes.sisu:10:11)
```

**Stage 0 differs.** `sisu check --holes`, which reports every hole's
type, bindings and tests at once, is designed
([R16](sisu-command.md#sisu-check---holes)).

**Status.** In stage 0 (D-061).

## Statements described elsewhere

| Statement | Page |
|---|---|
| `match VALUE:` with `case` arms | [R6](patterns.md) |
| `select:` | [R9](concurrency.md#waiting-for-the-first-of-several) |
| `assert CONDITION` and `assert CONDITION, MESSAGE` | [R15](testing.md#assertions) |
| `expect CONDITION`, which records its failure and lets a test go on | [R15](testing.md#assertions) |
| `debug_assert CONDITION` and `debug_assert CONDITION, MESSAGE`, left out below the contract level `all` | [R15](testing.md#assertions), [R11](contracts.md#check-levels) |
| `unsafe:` with a block | [R7](ownership.md#unsafe-code-and-raw-pointers) |
| `pass`, the empty block | [R1](lexical-structure.md#indentation-and-blocks) |
| an expression statement, such as a call | [R4](expressions.md) |

An expression whose value is a result or an optional value must be
used; as a statement on its own it is E0318, and `_ = VALUE` discards
it on purpose ([R17](diagnostics.md#e0318)).

**Status.** In stage 0.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0313](diagnostics.md#e0313), [E0312](diagnostics.md#e0312), [E0341](diagnostics.md#e0341), E0114 | `type.not-a-place`, `type.assign-immutable`, `type.not-mutable`, `parse.assignment-in-expression` | [Assignment](#assignment) |
| [E0302](diagnostics.md#e0302), E0330, E0331 | `type.not-bool`, `type.else-not-fallible`, `type.else-must-diverge` | [Conditions](#conditions) |
| [E0325](diagnostics.md#e0325), E0322, E0323, R0021 | `type.not-iterable`, `type.break-outside-loop`, `type.continue-outside-loop`, `panic.range-step` | [Loops](#loops) |
| [E0127](diagnostics.md#e0127) | `parse.loop-else` | [What control flow leaves out](#what-control-flow-leaves-out) |
| [E0315](diagnostics.md#e0315), E0316, E0317 | `type.missing-return`, `type.return-value-in-unit`, `type.return-needs-value` | [Return](#return) |
| E0435, E0436 | `type.yield-outside-generator`, `type.generator-result` | [Generators](#generators) |
| [W0001](diagnostics.md#w0001), [R0013](diagnostics.md#r0013) | `type.hole`, `panic.hole` | [Typed holes](#typed-holes) |
| [E0318](diagnostics.md#e0318) | `type.unused-result` | [Statements described elsewhere](#statements-described-elsewhere) |
