# Make a loop parallel

This guide turns a loop over a list into a computation over chunks
that `std.par` can run in parallel, and checks that the result is
unchanged, bit for bit. It assumes traits and `pure` functions, from
chapters 14 and 16 of the tutorial.

**See also:** the source of `std.par`,
[`compiler/std/par.sisu`](../../compiler/std/par.sisu); the proposal's
[06, "Data parallelism"](../../proposal/06-concurrency-and-capabilities.md#data-parallelism)
and D-069; [Find a concurrency bug](concurrency-bug.md), for work that
waits on the world, which tasks do instead.

## 1. Start from the serial loop

Keep the loop that works, and its output, as the answer to check
against. This one counts sauna readings, in tenths of a degree, by
bands of ten degrees:

<!-- file: examples/guide-parallel/serial.sisu -->
```sisu
# The loop to make parallel: a histogram, one reading at a time.

## How many `readings`, in tenths of a degree, fall in each band of ten
## degrees, by the band's lowest degree.
def bands(readings: list[int]) -> dict[int, int]:
    var counts: dict[int, int] = {}
    for x in readings:
        counts.entry(x // 100 * 10).or_insert(0) += 1
    return counts

def main():
    let readings = [600 + (i * 37) % 400 for i in range(10_000)]
    print(bands(readings))
```

<!-- output: examples/guide-parallel/serial.out -->
```text
{60: 2500, 70: 2500, 80: 2500, 90: 2500}
```

A loop is worth making parallel when each turn does work of its own on
its own value and reads nothing that another turn writes. A loop that
waits on files or the network wants tasks, not `std.par`.

## 2. Make the body pure

A parallel body runs on chunks of the list in no order you can see, so
it must be `pure`: no I/O, no locks, nothing shared that changes.
`par.map(xs, f)` maps a list with a pure function, and a body that is
not pure is refused:

<!-- file: examples/guide-parallel/impure.sisu -->
```sisu
# A parallel map whose body prints, which a `par` body may not do.
import std.par

def main():
    let readings = [612, 745, 803]
    let celsius = par.map(readings, (t) -> t.to_f64() / 10.0)
    print(celsius)
    let shown = par.map(readings, (t) -> print("reading {t}"))
    print(len(shown))
```

<!-- output: examples/guide-parallel/impure.console -->
```console
$ sisu check impure.sisu
error[E0335]: this function is passed where a pure function is required, but it calls `print`, which is not pure
  --> impure.sisu:8:35
  |
8 |     let shown = par.map(readings, (t) -> print("reading {t}"))
  |                                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^

```

E0335 (`type.impure-argument`) names the impure call. Keep the printing
after the parallel part, on its result.

## 3. Write the loop as a reducer

A loop that builds one result from many values is a reduction.
`par.reduce[R](xs, grain=g)` splits the list into chunks of at most `g`
values, by index, gives each chunk an empty view of the reducer `R`,
adds the chunk's values to it in order, and merges the views of
adjacent chunks, the earlier first, up a tree that the length and the
grain fix. A reducer implements `par.Reducer`: `empty`, `add`, `merge`
and `finish`, each `pure`:

<!-- file: examples/guide-parallel/bands.sisu -->
```sisu
# The histogram as a reducer over chunks, which can run in parallel.
import std.par
from std.par import Reducer

## The bands of the readings of one chunk, or of several chunks merged.
struct Bands:
    counts: dict[int, int] = {}

impl Reducer for Bands:
    type Item = int
    type Output = dict[int, int]

    pure def empty() -> Bands:
        return Bands()

    pure def add(mut self, own x: int):
        self.counts.entry(x // 100 * 10).or_insert(0) += 1

    pure def merge(own self, own later: Bands) -> Bands:
        var counts = self.counts
        for band, n in later.counts.items():
            counts.entry(band).or_insert(0) += n
        return Bands(counts)

    pure def finish(own self) -> dict[int, int]:
        return self.counts

## The bands of `readings`, in chunks of at most `grain` readings.
def bands(readings: list[int], grain: int) -> dict[int, int]:
    return par.reduce[Bands](readings.copy(), grain=grain)

test "every grain gives the answer of one chunk":
    let readings = [600 + (i * 37) % 400 for i in range(2_000)]
    for grain in [1, 7, 100]:
        assert bands(readings, grain) == bands(readings, len(readings))

test "so do random readings":
    check(arbitrary[list[int]]) do readings:
        return bands(readings, 2) == bands(readings, len(readings) + 1)
```

- `add` is the body of the serial loop, and `merge` combines two
  chunks' counts. A merge must give what one chunk holding both would
  give, which is what step 4 checks.
- `par.Sum[T]` and `par.Collect[T]` are the reducers for a sum and for
  a list in order, and `par.join(a, b)` runs two pure closures for
  divide and conquer.
- The grain, 1024 unless you pass one, sets the size of a chunk: small
  enough to share the work, large enough that the merges cost little.
  `par.chunks(n, grain)` shows the split.

## 4. Check that the result is unchanged

The chunks are fixed by index, not by which worker ran which, so the
result depends on the values, the length and the grain only, and
running one chunk is the serial elision: the same reduction without
the parallelism. Test that every grain gives what one chunk gives:

<!-- output: examples/guide-parallel/bands.test -->
```text
test "every grain gives the answer of one chunk" ... ok
test "so do random readings" ... ok
2 passed, 0 failed, 0 blocked
```

For floating point, compare with the prelude's `sum`, which adds in the
same pairwise shape as `par.Sum`, and not with a loop that adds left to
right, which rounds differently:

<!-- file: examples/guide-parallel/floats.sisu -->
```sisu
# A parallel sum of floats equals `sum`, and both can differ from a loop.
import std.par
from std.par import Sum

def main():
    let xs = [1.0e16, 1.0, -1.0e16, 1.0]
    var total = 0.0
    for x in xs:
        total += x
    print("a loop: {total}")
    print("sum: {sum(xs)}")
    for grain in [1, 2, 1024]:
        print("grain {grain}: {par.reduce[Sum[f64]](xs.copy(), grain=grain)}")
```

<!-- output: examples/guide-parallel/floats.out -->
```text
a loop: 1.0
sum: 0.0
grain 1: 0.0
grain 2: 0.0
grain 1024: 0.0
```

**Status.** Stage 0 runs the chunks one after another, in the tree's
order, on the task that asks: it has no pool of workers yet (Q-6), so
a parallel reduction gives the right answer and no speed-up.
[06](../../proposal/06-concurrency-and-capabilities.md#data-parallelism)
spells the parallel loop `xs.par_iter()`, which is E0304 in stage 0,
and `par_iter_mut()`, which is missing; stage 0's `par.iter(xs)` gives
the values for a `for` loop, whose body is checked as `pure` code
(E0446 for an impure call, E0447 for a change of what was declared
before the loop).
