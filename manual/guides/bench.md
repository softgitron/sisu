# Benchmark and profile

This guide times a piece of code with a benchmark, compares two ways of
writing it, and says what stage 0 offers for finding where a program's
time goes. It assumes tests, from chapter 19 of the tutorial.

**See also:** the tutorial's
[chapter 19](../tutorial/19-testing.md#benchmarks) (benchmarks);
[R16](../reference/sisu-command.md) for `sisu bench` and `--stats`; the
proposal's [17, "Benchmarks"](../../proposal/17-testing.md#benchmarks)
and [10, Tooling](../../proposal/10-tooling.md), "Debugging and
profiling".

## 1. Write a benchmark beside the tests

A `bench "name"(b: Bench):` block times the loop `for _ in b.loop():`,
whose number of turns the harness chooses. Write one for each way of
doing the work you want to compare, and a test that the ways agree, so
that you never time a fast wrong answer:

<!-- file: examples/guide-bench/vowels.sisu -->
```sisu
# Two ways to count vowels, a test that they agree, and a benchmark each.
from std.test import Bench, black_box

## The number of vowels in `word`, looked up in a list.
def vowels(word: str) -> int:
    var n = 0
    for c in word.chars():
        if c in ['a', 'e', 'i', 'o', 'u', 'y', 'ä', 'ö']:
            n += 1
    return n

## The same number, by a `match` on each character.
def vowels_matched(word: str) -> int:
    var n = 0
    for c in word.chars():
        match c:
            case 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ä' | 'ö':
                n += 1
            case _:
                pass
    return n

## A long Finnish word: an assistant mechanic of jet engines.
const LONG: str = "lentokonesuihkuturbiinimoottoriapumekaanikko"

test "both ways agree":
    for word in ["sauna", "löyly", LONG, ""]:
        assert vowels(word) == vowels_matched(word)

bench "a list"(b: Bench):
    for _ in b.loop():
        black_box(vowels(LONG))

bench "a match"(b: Bench):
    for _ in b.loop():
        black_box(vowels_matched(LONG))
```

- `Bench` and `black_box` come from `std.test`.
- `black_box(x)` uses the result in a way the compiler cannot see
  through, so that the work that made it is not optimised away.
- `sisu test` checks the benchmarks as it checks the tests, and does
  not run them:

<!-- output: examples/guide-bench/vowels.console -->
```console
$ sisu test vowels.sisu
test "both ways agree" ... ok
1 passed, 0 failed, 0 blocked
```

## 2. Run the benchmarks, and compare

`sisu bench FILE` runs each benchmark once to warm up, then times ten
samples of 100 turns of its loop, and prints the median time of one
turn with the 95% interval of the mean. `--samples K` sets the number
of samples and `--iterations N` the turns in each. The times differ from
run to run, so this output is an example rather than a check:

<!-- unchecked: sisu bench prints times that differ on every run -->
```console
$ sisu bench vowels.sisu
bench "a list" ... 105.3 µs an iteration, the median of 10 samples of 100 iterations (95% interval of the mean 100.3 µs to 110.2 µs, 3 outliers)
bench "a match" ... 70.2 µs an iteration, the median of 10 samples of 100 iterations (95% interval of the mean 64.2 µs to 79.5 µs)
2 benchmarks, 0 failed (stage 0 times the interpreter: compare benchmarks with each other, not with compiled code)
```

- Believe a difference when the two intervals do not overlap, as here:
  the `match` is faster. When they overlap, the run cannot tell the two
  apart; run again with more samples.
- An outlier is a sample far from the median, set aside before the
  interval is computed. Several outliers, or a wide interval, mean that
  something else ran on the machine while it timed.
- The interval is one run's. Two runs on a busy machine can still
  disagree, so run the comparison more than once before acting on it.
- An interval needs at least two samples, so `--samples` below 2 is
  refused with D0136 (`cli.bench-samples`), status 2:

<!-- output: examples/guide-bench/samples.console -->
```console
$ sisu bench --samples 1 vowels.sisu
error[D0136]: `--samples` takes 2 or more, not 1: the interval of the mean needs two samples
```

- Compare benchmarks of one run with each other. Stage 0 times its
  interpreter, so a time says little about the same code compiled.
- Keep the benchmarks with the code, so that the comparison can be made
  again after a change.

## 3. Run one module's benchmarks

In a package, `sisu bench` runs the benchmarks of every module, and
`sisu bench src/MODULE.sisu` runs those of one module only, with the
package compiled around it, as `sisu test src/MODULE.sisu` runs one
module's tests. Here the package `letters` has a benchmark in each of
its two modules:

<!-- file: examples/guide-bench/letters.pkg/src/lib.sisu -->
```sisu
##! Counting the vowels of words, with a benchmark of the count.
from std.test import Bench, black_box

## The number of vowels in `word`, looked up in a list.
pub def vowels(word: str) -> int:
    var n = 0
    for c in word.chars():
        if c in ['a', 'e', 'i', 'o', 'u', 'y', 'ä', 'ö']:
            n += 1
    return n

bench "a list"(b: Bench):
    for _ in b.loop():
        black_box(vowels("lentokonesuihkuturbiinimoottori"))
```

<!-- file: examples/guide-bench/letters.pkg/src/matched.sisu -->
```sisu
##! The same count by a `match`, with its own test and benchmark.
from letters import vowels
from std.test import Bench, black_box

## The number of vowels in `word`, by a `match` on each character.
pub def vowels_matched(word: str) -> int:
    var n = 0
    for c in word.chars():
        match c:
            case 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ä' | 'ö':
                n += 1
            case _:
                pass
    return n

test "both ways agree":
    for word in ["sauna", "löyly", ""]:
        assert vowels(word) == vowels_matched(word)

bench "a match"(b: Bench):
    for _ in b.loop():
        black_box(vowels_matched("lentokonesuihkuturbiinimoottori"))
```

At the package's root, the second module's benchmark runs alone, named
after its module as a test is:

<!-- unchecked: sisu bench prints times that differ on every run -->
```console
$ sisu bench src/matched.sisu
bench "matched/a match" ... 42.8 µs an iteration, the median of 10 samples of 100 iterations (95% interval of the mean 37.1 µs to 45.8 µs, 3 outliers)
1 benchmarks, 0 failed (stage 0 times the interpreter: compare benchmarks with each other, not with compiled code)
```

Run the module you changed while you work on it, and the whole
package's before you compare the two ways.

## 4. Find where the time goes

A profiler that runs a program and reports where its time went, `sisu
run --profile`, is designed and not in stage 0. `--stats` counts what
a run did: its tasks and switches, and with `--stats stack-depth` its
calls by the depth of the stack:

<!-- output: examples/guide-bench/tasks.console -->
```console
$ sisu run --profile tasks.sisu
error[D0009]: unknown option `--profile` for `sisu run`
$ sisu run --stats stack-depth tasks.sisu
[3, 3, 2]
stats stack-depth: 6 calls, the deepest stack 3 frames, about 928 bytes of native stack there (16 per SIR local and 64 per frame)
stats stack-depth: 1 calls at a depth of 1 to 1 frames
stats stack-depth: 5 calls at a depth of 2 to 3 frames
```

Until the profiler lands, benchmark the parts of a slow program one at
a time: a function whose benchmark grows fastest with its input is
where to look first.

**Status.** `bench` blocks, `black_box` and `sisu bench`, of a file, a
package or one of its modules, are in stage 0, timing the interpreter; 17's JSON report of the results is designed.
`sisu run --profile` (D0009 in stage 0) and the warning of
[06](../../proposal/06-concurrency-and-capabilities.md#data-parallelism)
when a parallel loop is slower than its serial form are designed.
