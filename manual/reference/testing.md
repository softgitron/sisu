# Testing

This page states what a test is and what the tools around it do: the
forms of `test`, its outcomes, the capabilities a test receives and how
tests are kept apart, assertions, property tests and their generators,
`@refines`, fakes of a program's own traits, doc examples, transcripts,
snapshots, replay tokens, simulation, benchmarks, and what `sisu test`
remembers. The command and its options are
[R16](sisu-command.md#sisu-test).

**See also:** the tutorial's [chapter 1](../tutorial/01-hello.md)
(tests and holes) and [chapter 19](../tutorial/19-testing.md) (testing
in depth); the proposal's [17, Testing](../../proposal/17-testing.md);
the compiler's [architecture notes](../../compiler/docs/architecture.md),
"Testing", "Property tests" and "Simulation, schedulers and replay";
[R17, Diagnostics](diagnostics.md#codes-and-their-ranges) for the codes.
`std.test` and the generators are in the standard library's reference.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Tests in the language and fakes instead of mocks are decided
(D-041, D-057); `@refines` and the replay tokens lean (D-070).

## The forms of a test

`test "NAME":` and an indented body is a test, at the top level of any
module, beside the code it tests ([R3](declarations.md#tests-suites-and-benchmarks)).
`sisu test` runs the tests and `sisu run` leaves them out.

| Form | Is |
|---|---|
| `test "NAME":` | one test |
| `test "NAME" for A, B in CASES:` | a test for each element of `CASES`, named with the element |
| `test "NAME"(clock: TestClock, fs: Fs):` | a test that receives a fresh fake of each capability it names |
| `test "NAME"(sys: System):` | a test that receives the real `System` and runs alone |
| `suite "NAME":` | a group of tests one level deep, whose tests are named `SUITE/NAME` |
| `@tag("slow")` before a test | a tag that `--tag` and `--skip-tag` select by |

A test returns nothing (E0381), takes nothing but capabilities (E0702)
and has no contract (E0807). Beside the `test` blocks, `sisu test` runs
the examples of doc comments ([Doc examples](#doc-examples)), the
properties of `@refines` ([Refinement tests](#refinement-tests)) and the
transcripts in `tests/` ([Transcripts](#transcripts)).

<!-- file: examples/reference-testing/forms.sisu -->
```sisu
# The forms of a test: plain, parametrised, with fakes, in a suite.
def double(n: int) -> int:
    return n * 2

def wait_and_double(clock: Clock, n: int) -> int ! TaskError:
    clock.sleep(Duration.minutes(5))?
    return Ok(double(n))

test "doubles":
    assert double(21) == 42

test "doubles each case" for n, want in [(1, 2), (-3, -6)]:
    assert double(n) == want

test "waits on a test clock"(clock: TestClock):
    let start = clock.now()
    assert wait_and_double(clock, 4) == Ok(8)
    assert clock.now() - start == Duration.minutes(5)

suite "large numbers":
    @tag("slow")
    test "doubles a large number":
        assert double(1 << 40) == 1 << 41
```

<!-- output: examples/reference-testing/forms.test -->
```text
test "doubles" ... ok
test "doubles each case (1, 2)" ... ok
test "doubles each case (-3, -6)" ... ok
test "waits on a test clock" ... ok
test "large numbers/doubles a large number" ... ok
5 passed, 0 failed, 0 blocked
```

**Status.** In stage 0 (D-041).

## Outcomes

Each test ends in one of three outcomes:

| Outcome | When |
|---|---|
| `ok` | the body ended |
| `FAILED` | an assertion failed, the body panicked, or the ownership monitor stopped it |
| `blocked by todo("NAME")` | the body reached a typed hole (D-061) |

`sisu test` lists the tests in the order they are declared, shows each
failure again after the list with its replay token, and ends with the
counts. Its status is 1 when a test failed and 0 otherwise, also when a
test is blocked ([R16](sisu-command.md#sisu-test)).

**Status.** In stage 0 (D-061).

## Isolation and the test clock

Every test runs in an interpreter of its own, with fresh fakes and
fresh module state, so tests cannot see each other. The tests that do
not take the real `System` run at the same time, on up to `--jobs N`
threads, and those that take it run one after another after them; the
output of each is held and written in declaration order, so a run prints
the same at any number of threads.

A test's clock is the run's test clock, which moves only when it is told
to (`advance`, `set`) or when every task of the test waits: then it
jumps to the earliest timer, so a test of a five-minute wait takes no
time ([R10](capabilities.md#fakes)).

**Status.** In stage 0 (D-057).

## Assertions

`assert CONDITION` and `assert CONDITION, MESSAGE` are statements, not
calls, and always run, in tests and in programs alike. A failed one
panics with the condition, the message, and the values of its parts:

<!-- file: examples/reference-testing/failing.sisu -->
```sisu
# A failing assertion shows the value of each part of its condition.
def last(scores: list[int]) -> int:
    return scores[len(scores) - 2]

test "the last score":
    let scores = [7, 9, 4]
    assert last(scores) == 4, "scores {scores}"
```

<!-- output: examples/reference-testing/failing.console -->
```console
$ sisu test failing.sisu
test "the last score" ... FAILED

test "the last score" ... FAILED
panic: assertion failed: last(scores) == 4
  message: scores [7, 9, 4]
  left:  9
  right: 4
  at test "the last score" (failing.sisu:7:5)
replay: sisu test --replay 1-c12c6afda705cec9-0.0.1-t0-s0-r0 failing.sisu

0 passed, 1 failed, 0 blocked
```

`debug_assert` is the form that a release build leaves out. The message
of `assert` is a string (E0379).

**Stage 0 differs.** [17](../../proposal/17-testing.md#assertions) shows
the value of every sub-expression, where stage 0 shows the two sides of
a comparison (`left`, `right`) and nothing for a condition joined with
`and`; `expect CONDITION`, which 17 makes the form that records a
failure and lets the test go on, stops the test as `assert` does; and
`debug_assert` is not a statement yet (E0117).

**Status.** In stage 0, with the differences noted.

## Fixtures

A fixture is a value that a function makes: a test calls the function,
and since each test runs apart, each gets its own. A fixture that needs
cleaning up is a value with a `Drop`.

**Stage 0 differs.** `@fixture`, the attribute of 17 for a fixture
shared by the tests that name it, and `Expectation` are designed.

**Status.** In stage 0 for fixtures as values; `@fixture` designed.

## Property tests

`check(GENERATOR) do x:` runs its block for many cases that the
generator makes, and fails when the block returns `False` or panics.

- `arbitrary[T]` draws cases of `T` at random; the compiler writes the
  generator of `T`, so a refinement's cases satisfy its predicate and a
  struct's its invariant (E0820 for a type it cannot generate).
- `Gen.of(make, cases=100)` makes cases from a function of a `Draws`,
  whose draws are `Draws.gen_in(d, lo, hi)` and the others.
- A failure is shrunk: the case is made smaller while it still fails,
  and the smallest is reported with its replay token, which holds the
  choices that make it.

<!-- file: examples/reference-testing/properties.sisu -->
```sisu
# A property must hold for every case that its generator makes.
type Percent = int where 0 <= self <= 100

def share(total: int, p: int) -> int:
    return total * p // 100

test "a share is never more than the whole":
    check(arbitrary[Percent]) do p:
        return share(1000, p) <= 1000

test "a share of a share is the share of the product":
    let pairs = Gen.of(
        d -> (Draws.gen_in(d, 0, 100), Draws.gen_in(d, 0, 100)),
    )
    check(pairs) do pair:
        let a, b = pair
        return share(share(1000, a), b) == share(1000, a * b // 100)
```

<!-- output: examples/reference-testing/properties.console -->
```console
$ sisu test properties.sisu
test "a share is never more than the whole" ... ok
test "a share of a share is the share of the product" ... FAILED

test "a share of a share is the share of the product" ... FAILED
panic: the property failed for (1, 10) (case 2, shrunk 14 times)
  the property returned False
  at test "a share of a share is the share of the product" (properties.sisu:15:5)
replay: sisu test --replay 1-4c84ac09fd75be02-0.0.1-t1-s0-r0-c1.10 properties.sisu

1 passed, 1 failed, 0 blocked
```

**Stage 0 differs.** The failure database of 17, which runs the cases
that failed before first, is missing, and so is `--fuzz`
([R16](sisu-command.md#options-of-the-existing-commands)).

**Status.** In stage 0 (D-070 leans).

## Every small case

`exhaustive[T](max_len=N)` makes every case of `T` with at most `N`
items in each collection, its integers bounded by their refinements,
instead of drawing at random; `Gen.every(make, max_len)` does the same
for a function of a `Draws`.

<!-- file: examples/reference-testing/exhaustive.sisu -->
```sisu
# `exhaustive` makes every small case; `arbitrary` draws at random.
enum Press:
    On
    Off
    Toggle

def light(presses: list[Press]) -> bool:
    var on = False
    for p in presses:
        on = match p:
            case On: True
            case Off: False
            case Toggle: not on
    return on

test "two toggles change nothing":
    var cases = 0
    check(exhaustive[list[Press]](max_len=3)) do presses:
        cases += 1
        var twice = presses.copy()
        twice.push(Toggle)
        twice.push(Toggle)
        return light(twice) == light(presses)
    assert cases == 40
```

<!-- output: examples/reference-testing/exhaustive.test -->
```text
test "two toggles change nothing" ... ok
1 passed, 0 failed, 0 blocked
```

**Status.** In stage 0 (D-070 leans).

## Refinement tests

`@refines(reference)` before a function adds a property test that the
function gives what `reference` gives for every case of its parameters'
types: a clear function is the specification of a fast one. A
reference whose signature differs is E0821.

<!-- file: examples/reference-testing/refines.sisu -->
```sisu
# `@refines` tests a fast function against a clear one on many cases.
def word_count_ref(text: str) -> int:
    return text.split_whitespace().count()

@refines(word_count_ref)
def word_count(text: str) -> int:
    var count = 0
    var in_word = False
    for c in text.chars():
        if c.is_whitespace():
            in_word = False
        elif not in_word:
            in_word = True
            count += 1
    return count

test "two words":
    assert word_count("hei maailma") == 2
```

<!-- output: examples/reference-testing/refines.test -->
```text
test "two words" ... ok
test "word_count refines word_count_ref" ... ok
2 passed, 0 failed, 0 blocked
```

**Stage 0 differs.** A failure of the test that `@refines` adds is
reported at a place in the module the compiler writes,
`<arbitrary>/main.sisu`, rather than at the `@refines` line.

**Status.** In stage 0 (leaning, D-070).

## Fakes of your own traits

`cap.Fake[Trait].new()` is a fake of one of the program's own traits:
for each method `m`, `on_m(result)` queues the result its next call
gives, and `calls()` lists the calls it received, as text. A trait whose
methods cannot all be forwarded is E0614. The fakes of the standard
capabilities are [R10](capabilities.md#fakes).

<!-- file: examples/reference-testing/own_fake.sisu -->
```sisu
# `cap.Fake[T]` is a fake of a trait of the program's own.
error SendError:
    Refused(to: String)

trait Mailer:
    def send(self, to: str, text: str) -> ! SendError

def remind[M: Mailer](mailer: M, names: list[str]) -> int:
    var failed = 0
    for name in names:
        if mailer.send(name, "the sauna is warm").is_err():
            failed += 1
    return failed

test "a refused message is counted":
    let mailer = cap.Fake[Mailer].new()
    mailer.on_send(Err(Refused("roni")))
    mailer.on_send(Ok(()))
    assert remind(mailer, ["roni", "aino"]) == 1
    assert mailer.calls()[1] == "send(\"aino\", \"the sauna is warm\")"
```

<!-- output: examples/reference-testing/own_fake.test -->
```text
test "a refused message is counted" ... ok
1 passed, 0 failed, 0 blocked
```

**Status.** In stage 0 (D-057).

## Doc examples

A fenced block in a `##` comment is an example: `sisu test` runs it as
a test named `NAME (doc example)`, and `sisu doc` shows it. `sisu test
--std` runs the standard library's ([R16](sisu-command.md#sisu-test)).

<!-- file: examples/reference-testing/doc_example.sisu -->
```sisu
# The examples in a doc comment are tests that `sisu test` runs.

## The initials of `name`, each followed by a dot.
##
## ```
## assert initials("aino maria") == "A.M."
## ```
def initials(name: str) -> String:
    var out = String.new()
    for word in name.split_whitespace():
        let first = word.chars().next() ?? ' '
        out.push("{first}".to_uppercase())
        out.push(".")
    return out
```

<!-- output: examples/reference-testing/doc_example.test -->
```text
test "initials (doc example)" ... ok
1 passed, 0 failed, 0 blocked
```

**Status.** In stage 0.

## Transcripts

A file `tests/NAME.transcript` beside a program is a test of its `main`.
Each line that starts with `$ ` runs `main` with the words after the
program's name as its arguments; the lines after it that start with `> `
are its standard input, and the rest are what it prints, standard output
and standard error as written, with ` (exit N)` on the last line of a
run that fails. The runs of one file share a fake `System`: an
in-memory `Fs` holding the files of `tests/NAME/`, a clock frozen at the
epoch and `Rand.seeded(0)`, with no network. A difference fails with a
diff (D0061); `sisu test --update` rewrites the file.

<!-- file: examples/reference-testing/greet.sisu -->
```sisu
# A program whose transcript in `tests/` is one of its tests.
error UsageError:
    NoNames

def main(sys: System) -> ! UsageError:
    let names = sys.env.args()
    if names.is_empty():
        return Err(NoNames)
    for name in names:
        print("Hei, {name}!")
    return Ok(())
```

<!-- file: examples/reference-testing/greet.files/tests/greet.transcript -->
```text
$ greet Aino Eero
Hei, Aino!
Hei, Eero!
$ greet
error: NoNames (exit 1)
```

<!-- output: examples/reference-testing/greet.console -->
```console
$ sisu test greet.sisu
transcript "tests/greet.transcript" ... ok
1 passed, 0 failed, 0 blocked
```

**Stage 0 differs.** The `.sisu` files of a package's `tests/`
directory, 17's black-box tests, are not run; only its transcripts are.
A failing run's line holds the error's trace when the error has one, so
such a line changes when the program's lines move.

**Status.** In stage 0.

## Snapshots

`assert_snapshot(value)` of `std.test` compares the value's `Debug`
form, or a text as it is, with the snapshot stored under the test's name
in `FILE.snap` beside the program: a change fails with a diff (R0107),
and a missing snapshot fails showing what would be stored (R0108).
`sisu test --update` shows each change and stores it
([R16](sisu-command.md#sisu-test), with an example).

**Status.** In stage 0.

## Replay tokens

Every failing test prints `replay: sisu test --replay TOKEN FILE`. The
token is one word that holds the content hash of the file, the
toolchain, the test's index, the scheduler's seed, the case's seed and,
for a property, the choices of the shrunk case, so `--replay` runs that
test exactly as it failed. A token for other code (D0033), another
toolchain (D0034) or a test that is gone (D0035) is refused, as is a
word that is not a token (D0032), and a test that takes the real
`System` has none (D0031). Its text is for the
command, not for reading.

**Status.** In stage 0 (leaning, D-070).

## Simulation

`sisu test --sim` gives a test that takes a `System` the `sim` profile's
instead of the real one: a file system that keeps only what was synced
when a node crashes, a network inside the process, the test clock and
seeded fakes of the rest. `--faults P` makes each operation of those
objects fail at that rate with an error from its own set. `std.sim` adds
`Sim`, nodes on one network that crash, restart and are partitioned,
`choose(n)`, a choice that the exhaustive scheduler enumerates, and
`always(what, holds, body)`, an invariant checked at every switch
between tasks (R0105). A `sim` build that could reach an `extern` call
without a model does not compile (E0971).
[R16](sisu-command.md#sisu-test) has an example.

**Status.** A prototype of the experiment (D-070; 17, "Simulation and
replay").

## Schedules

A test that spawns tasks runs under the seeded scheduler, so its outcome
is a function of its seed; `sisu test --explore N` runs every test under
`N` seeds, and `--scheduler pct` or `exhaustive` searches schedules on
purpose ([R9](concurrency.md#the-scheduler),
[R16](sisu-command.md#choosing-the-schedule)).

**Status.** In stage 0.

## Benchmarks

`bench "NAME"(b: Bench):` declares a benchmark, whose `for _ in
b.loop():` is the loop it times; `sisu test` checks a benchmark and does
not run it, and `sisu bench` runs it and prints the time of one turn of
the loop. `black_box(x)` of `std.test` uses a value in a way that
nothing can see through, so that the work that made it is kept
([R16](sisu-command.md#sisu-bench)).

**Status.** In stage 0.

## What sisu test remembers

A test that passed is not run again while its code is the same: `sisu
test` keeps its results in `FILE.test-cache` beside a file, or in
`build/debug/test-cache` in a package, and prints what the test printed
when it ran. `--no-cache` runs everything.

**Stage 0 differs.** 17's machine-readable output (JSON, TAP, JUnit XML)
and the reports of coverage and mutation testing are designed
([R16](sisu-command.md#options-of-the-existing-commands)).

**Status.** In stage 0, except the machine-readable output.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| E0381, E0702, E0807 | `type.return-in-test`, `type.test-signature`, `contract.not-in-test` | [The forms of a test](#the-forms-of-a-test) |
| [R0006](diagnostics.md#r0006), E0379 | `panic.assert`, `type.assert-message` | [Assertions](#assertions) |
| E0820 | `test.no-generator` | [Property tests](#property-tests) |
| E0821 | `test.refines-mismatch` | [Refinement tests](#refinement-tests) |
| E0614 | `comptime.no-fake` | [Fakes of your own traits](#fakes-of-your-own-traits) |
| D0061 | `cli.transcript-differs` | [Transcripts](#transcripts) |
| R0107, [R0108](diagnostics.md#r0108) | `test.snapshot-changed`, `test.snapshot-missing` | [Snapshots](#snapshots) |
| D0031, D0032, D0033, D0034, D0035 | `cli.replay-no-token`, `cli.bad-replay-token`, `cli.token-other-code`, `cli.other-toolchain`, `cli.token-no-test` | [Replay tokens](#replay-tokens) |
| R0105, E0971 | `sim.always-failed`, `sim.unmodelled-extern` | [Simulation](#simulation) |
