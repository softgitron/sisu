# 26. Purity and data parallelism

Chapter 16 met `pure` functions: they use no capability and give the
same result for the same arguments. This chapter is about what that
promise is worth. You see what a `pure` function may not do and the
rules of the language that keep its results the same on every machine,
then use those results to work on data in parallel with `std.par`:
mapping a list, reducing it in chunks with reducers of your own, and
dividing work in two with `par.join`. Last, you see why a scope of
tasks in `pure` code reports the same failure under any order of its
tasks. You need about an hour and a half.

## What pure promises

A `pure` function takes no capability, changes nothing outside itself,
and gives the same bits for the same arguments: on every run, every
machine and any number of workers. Anything it computes from its
arguments it may compute however it likes, with local variables that
change:

<!-- file: examples/26-purity/pure.sisu -->
```sisu
# A `pure` function uses no capability, changes nothing shared, and
# gives the same result for the same arguments, on every machine.

## How hot a turn in the sauna felt: the mean of `temps`, rounded down,
## and the highest of them.
pure def feel(temps: Slice[int]) -> (mean: int, highest: int):
    var total = 0
    var highest = temps[0]
    for t in temps:
        total += t
        highest = max(highest, t)
    return (total // len(temps), highest)

def main():
    let turn = feel([78, 85, 92, 88])
    print("{turn.mean} degrees on average, {turn.highest} at the most")
```

<!-- output: examples/26-purity/pure.out -->
```text
85 degrees on average, 92 at the most
```

`feel` changes `total` and `highest`, which are its own, and that
keeps it pure. Printing would not: what a program prints is part of
the world. A `pure` function may still allocate memory, loop for as
long as it needs, and panic on a bug, such as `temps[0]` of an empty
list; for the same arguments it panics in the same way every time.

**Try it.** Print `"feeling {len(temps)} readings"` at the start of
`feel`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/26-purity/pure_print.console -->
```console
$ sisu run pure_print.sisu
error[E0333]: `print` is not pure, so the pure function `feel` cannot call it
  --> pure_print.sisu:9:5
  |
9 |     print("feeling {len(temps)} readings")
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
6 | pure def feel(temps: Slice[int]) -> (mean: int, highest: int):
  |          ---- `feel` is declared `pure` here

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0333 (`type.impure-call`) names the call that is not pure, and its
label points at the `pure` that promised otherwise. The note belongs in
the code that calls `feel`. A `pure` function that takes a capability,
such as `clock: Clock`, is E0334 (`type.impure-capability`).

</details>

## What makes pure hold

A promise of the same bits everywhere needs more than a ban on
printing. Three rules of the language keep it, and you have met two of
them. A `dict` and a `set` keep the order in which their keys were
added, whatever the hash function does, so a loop over one gives the
same order every time:

<!-- file: examples/26-purity/ordered.sisu -->
```sisu
# What makes `pure` hold: a `dict` and a `set` keep insertion order.

## The words of `text` in the order they first appear, and how often
## each appears.
pure def tally(text: str) -> dict[String, int]:
    var counts: dict[String, int] = {}
    for word in text.split_whitespace():
        counts.entry(word.to_lowercase()).or_insert(0) += 1
    return counts

def main():
    let counts = tally("Löyly kiuas löyly vihta kiuas löyly")
    for word, n in counts:
        print("{word} {n}")
    print({3, 1, 2})
```

<!-- output: examples/26-purity/ordered.out -->
```text
löyly 3
kiuas 2
vihta 1
{3, 1, 2}
```

Floating point is strict, as chapter 24 showed: each operation rounds
once, in the same way on every machine. The third rule is that nothing
in `pure` code may see the order in which tasks happen to run: a
`Mutex`, `select` and a channel's `try_recv` all could, so none of them
is pure:

<!-- file: examples/26-purity/pure_lock.sisu -->
```sisu
# Nothing in `pure` code may observe the order in which tasks run.

## The visits so far, read from a counter that tasks share.
pure def visits(counter: Mutex[int]) -> int:
    return counter.lock().value

def main():
    print(visits(Mutex(3)))
```

<!-- output: examples/26-purity/pure_lock.console -->
```console
$ sisu run pure_lock.sisu
error[E0333]: `lock` is not pure, so the pure function `visits` cannot call it
  --> pure_lock.sisu:5:12
  |
5 |     return counter.lock().value
  |            ^^^^^^^^^^^^^^
  |
4 | pure def visits(counter: Mutex[int]) -> int:
  |          ------ `visits` is declared `pure` here

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

A lock gives its value to whichever task asks first, so a function that
reads one can give different results in two runs. So does an `Atomic`,
whose value is whatever a task wrote last: its `load` and its other
methods are E0333 in `pure` code too, with a note that says why. `task.scope`, `spawn`
and `join` stay allowed in `pure` code, under a rule of the section
"Tasks in pure code".

> **From Python.** Python's `dict` keeps insertion order too, since
> Python 3.7, while its `set` iterates in an order that depends on the
> hash of each item and can change between runs. A Sisu `set` keeps
> insertion order like a `dict`.

## Functions passed to pure code

A `pure` function that takes a function can call it only if the
function's type says `pure`, as chapter 16 showed for one function. The
same holds for a list of them:

<!-- file: examples/26-purity/steps.sisu -->
```sisu
# A `pure` function can call only the functions whose type says `pure`.

## The temperature after each step of a sauna evening is applied in
## turn to `start`.
pure def evening(start: int, steps: list[pure (int) -> int]) -> int:
    var celsius = start
    for step in steps:
        celsius = step(celsius)
    return celsius

## Heats by ten degrees, to 110 at most.
pure def heat(celsius: int) -> int:
    return min(celsius + 10, 110)

def main():
    let steps: list[pure (int) -> int] = [heat, heat, c -> c - 15, heat]
    print(evening(70, steps))
```

<!-- output: examples/26-purity/steps.out -->
```text
85
```

`steps` holds `heat`, a `pure def`, and a closure that only computes,
which is pure too. The evening starts at 70 degrees, heats twice to 90,
cools by 15 when the door opens, and heats once more to 85.

**Try it.** Write `(int) -> int` without `pure` in both places.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/26-purity/steps_any.console -->
```console
$ sisu run steps_any.sisu
error[E0392]: `evening` is pure, so it cannot call a function value whose type is not `pure`
  --> steps_any.sisu:8:19
  |
8 |         celsius = step(celsius)
  |                   ^^^^^^^^^^^^^
  |
5 | pure def evening(start: int, steps: list[(int) -> int]) -> int:
  |          ------- `evening` is declared `pure` here

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0392 (`type.impure-value-call`) refuses the call: a function value of
a type without `pure` may print or take a lock, and `evening` promised
not to. The fix is the type, not the function: with `pure` in the type,
the caller can pass only pure functions, which E0335 checks where they
are passed.

</details>

## A parallel map

`std.par` works on data in parallel, with `pure` functions. `par.map`
gives `f` of each value of a list, in order. Each call of `f` could run
on a worker of its own, so `f` must be pure, and the function type that
`par.map` takes says so: `pure (ref T) -> U`:

<!-- file: examples/26-purity/par_map.sisu -->
```sisu
# `par.map` gives `f` of each value, and `f` must be `pure`.
import std.par

## The temperature in Fahrenheit.
pure def fahrenheit(celsius: ref f64) -> f64:
    return celsius * 9.0 / 5.0 + 32.0

def main():
    let readings = [72.5, 80.0, 85.0, 90.0]
    print(par.map(readings, fahrenheit))
    print(par.map(readings, it >= 85.0))
```

<!-- output: examples/26-purity/par_map.out -->
```text
[162.5, 176.0, 185.0, 194.0]
[False, False, True, True]
```

`par.map(readings, fahrenheit)` passes a function by its name, and
`par.map(readings, it >= 85.0)` an implicit closure, which is pure
because it only computes. `par.map` borrows the list, so each call of
`f` gets a reference to its value, `ref f64` here, and the readings
stay where they are. The result is a new list, in the order of the
readings, however the work was divided.

> **From Python.** `multiprocessing.Pool.map` runs a function on worker
> processes, copies each value to them and back, and gives no promise
> about functions with side effects. `par.map` shares the list with its
> workers, and the compiler rejects a function with side effects before
> the program runs.

**Try it.** Remove `pure` from `fahrenheit`, and print a note in it.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/26-purity/par_map_impure.console -->
```console
$ sisu run par_map_impure.sisu
error[E0335]: this function is passed where a pure function is required, but it calls `print`, which is not pure
  --> par_map_impure.sisu:11:29
   |
11 |     print(par.map(readings, fahrenheit))
   |                             ^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0335 (`type.impure-argument`) refuses a function that is not pure
where `par.map` needs one. Its place is the function passed,
`fahrenheit`, and its message names the operation inside it that makes
it impure, `print`. When the impure call is further away, in a function
that `fahrenheit` calls, a note gives the way to it.

</details>

## A parallel loop

`for x in par.iter(xs):` runs its body for each value of `xs`, as a
parallel map runs its function, and its body is `pure` in the same way:
it may work with its own values and call `pure` functions, and nothing
more. A body that changed what was declared before the loop would race
with the other bodies, so it does not compile:

<!-- file: examples/26-purity/par_loop.sisu -->
```sisu
# The body of a parallel loop changes nothing declared before the loop.
import std.par

def main():
    let heats = [70, 80, 90]
    var total = 0
    for heat in par.iter(heats):
        total += heat
    print(total)
```

<!-- output: examples/26-purity/par_loop.console -->
```console
$ sisu run par_loop.sisu
error[E0447]: the body of a `par` loop changes nothing but its own values, and `total` is declared before the loop: compute each value with `par.map` and combine them with `sum` or a reducer (06)
  --> par_loop.sisu:8:9
  |
8 |         total += heat
  |         ^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0447 (`type.par-body-changes`) names the binding and the way out:
compute each value with `par.map` and combine the values with `sum` or
a reducer, which the next section shows. A body that calls a function
that is not `pure`, such as `print`, is E0446 (`type.par-body-impure`).

## Reducing in chunks

A reduction combines the values of a list into one result. `reduce`
splits the list into chunks by index, halving it until a chunk holds at
most `grain` values; each chunk makes a view of its own, and the views
are merged pairwise up the same tree. `Sum` adds and `Collect` collects.
Sums of floats are where parallel results usually drift from run to
run, so they show the guarantee best:

<!-- file: examples/26-purity/reduce.sisu -->
```sisu
# A reduction splits a list into chunks by index, and merges in a tree.
import std.par
from std.par import Collect, Sum, reduce

def main():
    print(par.chunks(10, 3))
    let xs = [1.0e16, 1.0, -1.0e16, 1.0]
    var folded = 0.0
    for x in xs:
        folded += x
    print("folded {folded}, summed {sum(xs)}")
    for grain in [1, 2, 4]:
        let total = reduce[Sum[f64]](xs.copy(), grain=grain)
        print("grain {grain}: {total}")
    print(reduce[Collect[int]]([3, 1, 4, 1, 5], grain=2))
```

<!-- output: examples/26-purity/reduce.out -->
```text
[(0, 2), (2, 5), (5, 7), (7, 10)]
folded 1.0, summed 0.0
grain 1: 0.0
grain 2: 0.0
grain 4: 0.0
[3, 1, 4, 1, 5]
```

- `par.chunks(10, 3)` shows the split: ten values halve into five and
  five, and each five into two and three.
- The list holds a huge number, a one, the huge number negated, and
  another one. Added one by one, the first one is lost in rounding and
  the second survives, which gives 1.0. `sum` adds in pairs and loses
  both, which gives 0.0. Neither is the exact 2.0; what matters is that
  `reduce` with `Sum` gives `sum`'s answer whatever the grain.
- `Collect` merges the earlier chunk's view first, so the values stay in
  the order of the list.

The tree depends on the length and the grain, never on which worker
ran which chunk, so removing the parallelism, running the chunks one
after another, gives the same result, bit for bit. In the compiler today
that is how the chunks run: one after another, on the task that asks,
with the result a pool of workers would give.

`grain` is 1,024 unless a call says otherwise. A small grain makes many
chunks, which spreads the work over more workers and spends more time
merging views; a large one makes few. Choose it by how much work each
value takes, and measure: for a reducer whose `merge` is exact, as a
sum of integers is, the grain changes the speed and never the result.

## A reducer of your own

A reducer is a type that implements `Reducer`: its `Item`, the values it
takes, its `Output`, what it gives, and four methods. `empty` makes a
chunk's view, `add` adds the chunk's next value to it, `merge` joins a
view with the view of the chunk after it, and `finish` gives the result:

<!-- file: examples/26-purity/longest.sisu -->
```sisu
# A reducer of your own: each chunk keeps a view, and views are merged.
from std.par import Reducer, reduce

## The longest word seen, the earliest of them on a tie.
struct Longest:
    word: String

impl Reducer for Longest:
    type Item = String
    type Output = String

    pure def empty() -> Longest:
        return Longest(word="")

    pure def add(mut self, own word: String):
        if len(word) > len(self.word):
            self.word = word

    pure def merge(own self, own later: Longest) -> Longest:
        if len(later.word) > len(self.word):
            return later
        return self

    pure def finish(own self) -> String:
        return self.word

def main():
    let words = ["sauna", "kiuaskivi", "löyly", "saunatupa", "vihta"]
    for grain in [1, 2, 4]:
        let copy = [w.to_string() for w in words]
        print("{grain}: {reduce[Longest](copy, grain=grain)}")
```

<!-- output: examples/26-purity/longest.out -->
```text
1: kiuaskivi
2: kiuaskivi
4: kiuaskivi
```

*Kiuaskivi* is a stone of the stove and *saunatupa* a sauna's cabin,
and both have nine letters. `add` keeps a word only when it is longer,
so the earlier word of a tie stays, and `merge` keeps its own word
unless the later view's is longer. So the earlier word wins a tie
inside a chunk and across chunks alike, and every grain gives
`kiuaskivi`. A `merge` that preferred the later view on a tie would
give an answer that changed with the grain.

The grain decides the tree, so a reducer whose result must not depend
on it needs a `merge` that does not care how the views are grouped:
merging `a` with `b` and then with `c` must give what merging `a` with
the merge of `b` and `c` gives. Counting, taking the longest and
collecting in order have that property. Adding floats one by one does
not, which is why `Sum` adds in the pairwise shape of `sum` instead.

## Dividing the work in two

`par.join` runs two pure closures and gives both results, which suits
divide and conquer: split a problem in two, solve the halves side by
side, and combine:

<!-- file: examples/26-purity/join.sisu -->
```sisu
# `par.join` runs two pure closures and gives both of their results.
import std.par

## Whether `n` is prime, by trial division.
pure def is_prime(n: int) -> bool:
    if n < 2:
        return False
    var d = 2
    while d * d <= n:
        if n % d == 0:
            return False
        d += 1
    return True

## How many primes there are from `lo` up to `hi`, the range halved until
## a part is small.
pure def count_primes(lo: int, hi: int) -> int:
    if hi - lo <= 1000:
        return len([n for n in range(lo, hi) if is_prime(n)])
    let mid = lo + (hi - lo) // 2
    let (below, above) = par.join(
        () -> count_primes(lo, mid),
        () -> count_primes(mid, hi),
    )
    return below + above

def main():
    print(count_primes(0, 10_000))
```

<!-- output: examples/26-purity/join.out -->
```text
1229
```

`count_primes` halves its range until a part holds a thousand numbers
at most, counts those directly, and adds the halves' counts. The
closures use `lo`, `mid` and `hi`, and they cannot block: a `pure`
closure cannot wait for a file, a timer or a lock, so it runs to its
end on the worker that started it. For more than two parts,
`par.join_all(fs)` runs a list of pure closures and gives their results
in the list's order.

Tasks and `std.par` answer different needs. Tasks wait: for the
network, a file, a timer or each other, and many of them can wait at
once while few run. `std.par` computes: it keeps every worker busy with
arithmetic over data, and its bodies never wait. A program that fetches
pages and then analyses them uses tasks for the fetching and `std.par`
for the analysis.

## Tasks in pure code

A `pure` function may open a scope and spawn tasks, and a rule keeps
its result the same however the tasks are ordered: when tasks fail,
the scope reports the failure of the earliest one spawned, not of the
first one to fail. Here two parts of a batch are bad, and the earlier
one is slow:

<!-- file: examples/26-purity/pure_scope.sisu -->
```sisu
# A `pure` scope reports the failure of the earliest task, not the first.
error CheckError:
    Bad(part: int)

## Checks part `part` of a batch, after `work` steps; odd parts are bad.
pure def check(part: int, work: int) -> int ! CheckError:
    var total = 0
    for k in range(work):
        total += k
    if part % 2 == 1:
        return Err(Bad(part))
    return Ok(total)

## Checks the parts of a batch at once, as a `pure` function.
pure def check_pure(work: list[int]) -> int ! {CheckError, TaskError}:
    return task.scope do s:
        for i in range(len(work)):
            let n = work[i]
            s.spawn(() -> check(i, n))
        return Ok(len(work))

## The same, in a function that is not `pure`.
def check_any(work: list[int]) -> int ! {CheckError, TaskError}:
    return task.scope do s:
        for i in range(len(work)):
            let n = work[i]
            s.spawn(() -> check(i, n))
        return Ok(len(work))

def main():
    let work = [5, 50_000, 10, 10]
    print(check_pure(work))
    print(check_any(work))
```

<!-- output: examples/26-purity/pure_scope.console -->
```console
$ sisu run pure_scope.sisu
Err(Bad(part=1))
Err(Bad(part=3))
$ sisu run --explore 5 pure_scope.sisu
Err(Bad(part=1))
Err(Bad(part=3))
explored 5 schedules (seeds 0 to 4): every run exited with status 0
```

- Parts 1 and 3 are bad, and part 1 takes far longer to check, so part
  3 fails first in time.
- `check_pure` reports `Bad(part=1)`: in a `pure` scope, a failure
  cancels only the tasks spawned after the failing one, the tasks
  before it run to their end, and the lowest-index failure is the
  result.
- `check_any`, the same code in a function that is not `pure`, reports
  the first failure in time, `Bad(part=3)`, and cancels the rest at
  once, as chapter 20 showed.
- `sisu run --explore 5` runs the program under five orders of its
  tasks, and each run prints the same. A test of `check_pure` under
  `sisu test --explore` passes under every order too.

Effectful code keeps the first failure in time because there stopping
the other work quickly matters more than reproducing the answer. In
`pure` code the answer comes first.

> **From Python.** `asyncio.gather` raises the first exception in
> time, which can change from one run to the next when the tasks race.
> A scope in `pure` code reports the same failure on every run, which is
> what lets a test of it give the same result every time.

## What is still to come

The proposal spells a parallel loop as a method, `for x in
xs.par_iter():`, with `par_iter_mut` to change each element in place:
each body may change its own element and nothing else, which the
compiler can check because two bodies never share one. `SoaList[T]`
keeps a list of structs as one list for each field, so that a loop
that reads two fields of a million particles reads two runs of memory
rather than a million scattered structs. None of the three is in the
compiler yet.
Stage 0 spells the loop `for x in par.iter(xs):`, as the section "A
parallel loop" does, and runs its bodies one after another. A pool of
workers to run the chunks, and
`sisu bench` warning when a parallel loop is slower than its serial
form, wait for the runtime of [06](../../proposal/06-concurrency-and-capabilities.md#data-parallelism).

## Exercise 26.1: predict the parallel results

Open
[`manual/exercises/26-purity/predict_par.sisu`](../exercises/26-purity/predict_par.sisu):

<!-- file: exercises/26-purity/predict_par.sisu -->
```sisu
##! Exercise 26.1, predict the parallel results. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of what an expression gives, written
##! as `print` would write it. Write each as a literal, then run
##! `sisu test predict_par.sisu`. The hints are in chapter 26 of the
##! tutorial, under "Exercise 26.1: predict the parallel results".

import std.par
from std.par import Collect, Sum, reduce

error CheckError:
    Bad(part: int)

## Checks part `part` after `work` steps; parts above 1 are bad.
pure def check(part: int, work: int) -> int ! CheckError:
    var total = 0
    for k in range(work):
        total += k
    if part > 1:
        return Err(Bad(part))
    return Ok(part)

## Checks every part at once, in a `pure` scope.
pure def check_all(work: list[int]) -> int ! {CheckError, TaskError}:
    return task.scope do s:
        for i in range(len(work)):
            let n = work[i]
            s.spawn(() -> check(i, n))
        return Ok(len(work))

const CHUNKS: str = todo("par.chunks(6, 2)")
const ONE_CHUNK: str = todo("par.chunks(4, 4)")
const COLLECTED: str = todo("reduce[Collect[int]]([5, 3, 8], grain=1)")
const SUMMED: str = todo("reduce[Sum[int]]([5, 3, 8], grain=2)")
const FAILED: str = todo("check_all([10, 10, 90_000, 10])")

test "chunks of two":
    assert "{par.chunks(6, 2)}" == CHUNKS

test "a chunk that holds them all":
    assert "{par.chunks(4, 4)}" == ONE_CHUNK

test "collecting keeps the order":
    assert "{reduce[Collect[int]]([5, 3, 8], grain=1)}" == COLLECTED

test "a sum":
    assert "{reduce[Sum[int]]([5, 3, 8], grain=2)}" == SUMMED

test "the failure a pure scope reports":
    assert "{check_all([10, 10, 90_000, 10])}" == FAILED
```

<!-- output: solutions/26-purity/predict_par.console -->
```console
$ sisu test predict_par.sisu
warning[W0001]: typed hole `par.chunks(6, 2)` of type `str`
  --> predict_par.sisu:31:21
   |
31 | const CHUNKS: str = todo("par.chunks(6, 2)")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `par.chunks(4, 4)` of type `str`
  --> predict_par.sisu:32:24
   |
32 | const ONE_CHUNK: str = todo("par.chunks(4, 4)")
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `reduce[Collect[int]]([5, 3, 8], grain=1)` of type `str`
  --> predict_par.sisu:33:24
   |
33 | const COLLECTED: str = todo("reduce[Collect[int]]([5, 3, 8], grain=1)")
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `reduce[Sum[int]]([5, 3, 8], grain=2)` of type `str`
  --> predict_par.sisu:34:21
   |
34 | const SUMMED: str = todo("reduce[Sum[int]]([5, 3, 8], grain=2)")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `check_all([10, 10, 90_000, 10])` of type `str`
  --> predict_par.sisu:35:21
   |
35 | const FAILED: str = todo("check_all([10, 10, 90_000, 10])")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "chunks of two" ... blocked by todo("par.chunks(6, 2)")
test "a chunk that holds them all" ... blocked by todo("par.chunks(4, 4)")
test "collecting keeps the order" ... blocked by todo("reduce[Collect[int]]([5, 3, 8], grain=1)")
test "a sum" ... blocked by todo("reduce[Sum[int]]([5, 3, 8], grain=2)")
test "the failure a pure scope reports" ... blocked by todo("check_all([10, 10, 90_000, 10])")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

`par.chunks` halves a range, rounding the first half down, until each
part holds at most `grain` values. Draw the tree for six values.

</details>

<details>
<summary>Hint 2</summary>

A reduction's result does not depend on the chunks, for `Collect` and
`Sum` alike. In the scope, two parts fail; ask which was spawned first,
not which finishes first.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/26-purity/predict_par.sisu`](../solutions/26-purity/predict_par.sisu).

## Exercise 26.2: a note in a parallel map

Open
[`manual/exercises/26-purity/vat.sisu`](../exercises/26-purity/vat.sisu):

<!-- file: exercises/26-purity/vat.sisu -->
```sisu
##! Exercise 26.2, a note in a parallel map. Kind: fix. Level: 1.
##!
##! `with_vat` adds 25.5 % VAT to prices with `par.map`, and the function
##! it maps prints a note for each price. The file does not compile. Run
##! `sisu test vat.sisu`, read the diagnostic, and fix the program, not
##! the tests, so that every test passes and the program still prints a
##! note of how many prices it changed. The hints are in chapter 26 of the
##! tutorial, under "Exercise 26.2: a note in a parallel map".

import std.par

## `cents` with VAT added, rounded down to a cent, and a note of it.
def add_vat(cents: ref int) -> int:
    print("adding VAT to {cents}")
    return cents * 1255 // 1000

## The `prices` with VAT added, worked out in parallel.
def with_vat(prices: list[int]) -> list[int]:
    return par.map(prices, add_vat)

test "prices with VAT":
    assert with_vat([1000, 250, 399]) == [1255, 313, 500]

test "no prices":
    assert with_vat([]) == []
```

<!-- output: solutions/26-purity/vat.console -->
```console
$ sisu test vat.sisu
error[E0335]: this function is passed where a pure function is required, but it calls `print`, which is not pure
  --> vat.sisu:19:28
   |
19 |     return par.map(prices, add_vat)
   |                            ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

E0335 says that the function given to `par.map` must be pure, and a
function that prints is not.

</details>

<details>
<summary>Hint 2</summary>

Keep the arithmetic in the function that `par.map` calls, and make it
`pure`. One note for the whole list can come from `with_vat`, which is
not pure and may print.

</details>

When every test passes, compare your answer with
[`manual/solutions/26-purity/vat.sisu`](../solutions/26-purity/vat.sisu).

## Exercise 26.3: a histogram in parallel

Open
[`manual/exercises/26-purity/histogram.sisu`](../exercises/26-purity/histogram.sisu):

<!-- file: exercises/26-purity/histogram.sisu -->
```sisu
##! Exercise 26.3, a histogram in parallel. Kind: write. Level: 2.
##!
##! A sauna logs its temperature each minute, and a histogram counts the
##! readings in each band of ten degrees: below 10, 10 to 19, 20 to 29,
##! and so on, with every reading from 120 up in the last band. Write the
##! methods of `Histogram`, a reducer for `reduce`, so that every test
##! passes: the counts must not depend on how the readings are split into
##! chunks. Run the tests with `sisu test histogram.sisu`. The hints are
##! in chapter 26 of the tutorial, under "Exercise 26.3: a histogram in
##! parallel".

from std.par import Reducer, reduce

## How many bands a histogram has.
const BANDS: int = 13

## How many readings fell in each band of ten degrees.
struct Histogram:
    counts: list[int]

impl Reducer for Histogram:
    type Item = int
    type Output = list[int]

    pure def empty() -> Histogram:
        return todo("empty")

    pure def add(mut self, own celsius: int):
        todo("add")

    pure def merge(own self, own later: Histogram) -> Histogram:
        return todo("merge")

    pure def finish(own self) -> list[int]:
        return todo("finish")

test "an evening's readings":
    let readings = [22, 45, 78, 81, 85, 90, 104, 98, 87, 125, 5]
    let want = [1, 0, 1, 0, 1, 0, 0, 1, 3, 2, 1, 0, 1]
    for grain in [1, 2, 1024]:
        assert reduce[Histogram](readings.copy(), grain=grain) == want

test "no readings":
    assert reduce[Histogram]([]) == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

test "the chunks change nothing":
    check(arbitrary[list[int]]) do readings:
        let one = reduce[Histogram](readings.copy(), grain=1)
        let three = reduce[Histogram](readings.copy(), grain=3)
        return one == three and sum(one) == len(readings)
```

<!-- output: solutions/26-purity/histogram.console -->
```console
$ sisu test histogram.sisu
warning[W0001]: typed hole `empty` of type `Histogram`
  --> histogram.sisu:26:16
   |
26 |         return todo("empty")
   |                ^^^^^^^^^^^^^

warning[W0001]: typed hole `add` of type `()`
  --> histogram.sisu:29:9
   |
29 |         todo("add")
   |         ^^^^^^^^^^^

warning[W0001]: typed hole `merge` of type `Histogram`
  --> histogram.sisu:32:16
   |
32 |         return todo("merge")
   |                ^^^^^^^^^^^^^

warning[W0001]: typed hole `finish` of type `list[int]`
  --> histogram.sisu:35:16
   |
35 |         return todo("finish")
   |                ^^^^^^^^^^^^^^

test "an evening's readings" ... blocked by todo("empty")
test "no readings" ... blocked by todo("empty")
test "the chunks change nothing" ... blocked by todo("empty")
0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

A view is a histogram of the readings of one chunk: a count for each of
the 13 bands, all zero to start with. A reading's band is its tens,
kept between the first band and the last.

</details>

<details>
<summary>Hint 2</summary>

Merging two views adds their counts band by band. Counting does not
depend on the order of the readings, so any split gives the same
counts, which the property test checks for a hundred lists.

</details>

When every test passes, compare your answer with
[`manual/solutions/26-purity/histogram.sisu`](../solutions/26-purity/histogram.sisu).

## Exercise 26.4: a sort by halves

Open
[`manual/exercises/26-purity/halves.sisu`](../exercises/26-purity/halves.sisu):

<!-- file: exercises/26-purity/halves.sisu -->
```sisu
##! Exercise 26.4, a sort by halves. Kind: write. Level: 3.
##!
##! Merge sort sorts a list by sorting its two halves and merging them.
##! The halves do not depend on each other, so `par.join` can sort them
##! side by side. Write `merge`, which merges two sorted lists, and
##! `sort_halves`, which sorts a list that way, halving it until a part
##! is small enough for `sorted`, so that every test passes. Run the tests
##! with `sisu test halves.sisu`. The hints are in chapter 26 of the
##! tutorial, under "Exercise 26.4: a sort by halves".

import std.par

## The items of `a` and `b`, which are both sorted, in one sorted list.
pure def merge(a: Slice[int], b: Slice[int]) -> list[int]:
    return todo("merge")

## `xs` sorted: its halves sorted side by side with `par.join`, and a part
## of at most `small` items sorted with `sorted`.
pure def sort_halves(xs: Slice[int], small: int = 8) -> list[int]:
    requires small >= 1
    return todo("sort_halves")

test "merging":
    assert merge([1, 4, 9], [2, 3, 10]) == [1, 2, 3, 4, 9, 10]
    assert merge([7, 7], [7]) == [7, 7, 7]
    assert merge([], [5]) == [5]

test "sorting":
    assert sort_halves([5, 3, 8, 1, 9, 2], small=1) == [1, 2, 3, 5, 8, 9]
    assert sort_halves([4, 2], small=8) == [2, 4]
    assert sort_halves([]) == []

test "sorting agrees with sorted":
    check(arbitrary[list[int]]) do xs:
        return sort_halves(xs, small=2) == sorted(xs)
```

<!-- output: solutions/26-purity/halves.console -->
```console
$ sisu test halves.sisu
warning[W0001]: typed hole `merge` of type `list[int]`
  --> halves.sisu:15:12
   |
15 |     return todo("merge")
   |            ^^^^^^^^^^^^^

warning[W0001]: typed hole `sort_halves` of type `list[int]`
  --> halves.sisu:21:12
   |
21 |     return todo("sort_halves")
   |            ^^^^^^^^^^^^^^^^^^^

test "merging" ... blocked by todo("merge")
test "sorting" ... blocked by todo("sort_halves")
test "sorting agrees with sorted" ... blocked by todo("sort_halves")
test "contract of sort_halves" ... blocked by todo("sort_halves")

test "merging" ... blocked by todo("merge")
what the test printed:
  in scope at the typed hole `todo("merge")`:
    a = [1, 4, 9]
    b = [2, 3, 10]

test "sorting" ... blocked by todo("sort_halves")
what the test printed:
  in scope at the typed hole `todo("sort_halves")`:
    xs = [5, 3, 8, 1, 9, 2]
    small = 1

test "sorting agrees with sorted" ... blocked by todo("sort_halves")
what the test printed:
  in scope at the typed hole `todo("sort_halves")`:
    xs = [5]
    small = 2

test "contract of sort_halves" ... blocked by todo("sort_halves")
what the test printed:
  in scope at the typed hole `todo("sort_halves")`:
    xs = [5]
    small = 4

0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

`merge` walks the two lists with an index each, takes the smaller of
the two items it points at, and copies what is left of either list at
the end. Taking an item of `a` when the two are equal keeps the merge
stable.

</details>

<details>
<summary>Hint 2</summary>

`sort_halves` has the shape of `count_primes` in the section "Dividing
the work in two": a small part is sorted directly, and a larger one is
cut at its middle, `xs[:mid]` and `xs[mid:]`, and its halves sorted
with `par.join` and merged.

</details>

<details>
<summary>Hint 3</summary>

`sorted` takes an iterable, and a `Slice` gives one with `iter()`.

</details>

When every test passes, compare your answer with
[`manual/solutions/26-purity/halves.sisu`](../solutions/26-purity/halves.sisu).

## What you learned

- A `pure` function takes no capability (E0334), calls only pure
  functions (E0333) and gives the same bits for the same arguments.
- `pure` holds because a `dict` and a `set` keep insertion order,
  floating point is strict, and nothing that sees the order of tasks,
  such as a lock or an `Atomic`, is pure.
- `par.map` maps a pure function over a list (E0335 for one that is
  not); `reduce` splits a list into chunks by index and merges their
  views in a fixed tree, so its result never depends on the workers.
  The body of a `par.iter` loop is pure too, and changes nothing
  declared before the loop (E0447).
- A reducer implements `Reducer` with `empty`, `add`, `merge` and
  `finish`; a `merge` that keeps the earlier view first keeps the order.
- `par.join` runs two pure closures for divide and conquer.
- A scope in `pure` code reports the lowest-index failure, so its
  result is the same under any order of its tasks.

Chapter 27 is about systems programming: `unsafe` code and raw
pointers, calling C, allocators, the attributes that keep code from
allocating or blocking, runtime profiles, targets and the size of a
binary.
