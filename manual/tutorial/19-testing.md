# 19. Testing in depth

Every chapter since the first has used `test` blocks.
In this chapter you organise tests into suites, run a test once for
each case of a list, read what a failing assertion shows, and give each
test the fixtures, fakes and clock it needs. Then you test properties rather
than examples, check a fast function against a clear one, fake your
own traits, and keep whole outputs as snapshots and transcripts. Last,
you replay a failure from its token and time code with benchmarks. You
need about two hours.

## Suites, tags and choosing tests

A test is a declaration in the module it tests, so it can call the
module's private functions, and only `sisu test` compiles it. A
`suite` groups tests under a name, one level deep, and `@tag` marks a
test so that a run can choose it or leave it out:

<!-- file: examples/19-testing/feels.sisu -->
```sisu
# A suite groups tests under a name, and a tag marks some of them.

## How a sauna at `celsius` degrees feels.
def feels(celsius: int) -> str:
    if celsius < 60:
        return "cool"
    if celsius < 80:
        return "warm"
    if celsius <= 100:
        return "hot"
    return "too hot"

suite "feels":
    test "the edges of warm":
        assert feels(60) == "warm"
        assert feels(79) == "warm"

    test "hot and too hot":
        assert feels(100) == "hot"
        assert feels(101) == "too hot"

    @tag("slow")
    test "every degree feels like something":
        for celsius in range(0, 200):
            assert feels(celsius) in ["cool", "warm", "hot", "too hot"]
```

<!-- output: examples/19-testing/feels.console -->
```console
$ sisu test feels.sisu
test "feels/the edges of warm" ... ok
test "feels/hot and too hot" ... ok
test "feels/every degree feels like something" ... ok
3 passed, 0 failed, 0 blocked
$ sisu test --skip-tag slow feels.sisu
test "feels/the edges of warm" ... ok (cached)
test "feels/hot and too hot" ... ok (cached)
2 passed, 0 failed, 0 blocked
1 not selected
$ sisu test --filter edges feels.sisu
test "feels/the edges of warm" ... ok (cached)
1 passed, 0 failed, 0 blocked
2 not selected
```

- A test in a suite is named `suite/test`, as `feels/the edges of
  warm`.
- `--skip-tag slow` leaves out the tests tagged `slow`, and `--tag
  slow` runs only those. `--filter TEXT` runs the tests whose names
  hold the text. The last line counts the tests left out.
- A test that passed in the first run and has not changed since is
  `ok (cached)` in the later ones ([What sisu test
  remembers](#what-sisu-test-remembers)).

A name is fixed text, since runs choose and replay by it, so a brace in
it is written twice:

<!-- file: examples/19-testing/names.sisu -->
```sisu
# A test's name is fixed text: a brace in it is written twice.
const STEPS: dict[int, str] = {60: "warm", 80: "hot"}

test "{{60: warm, 80: hot}} has two steps":
    assert len(STEPS) == 2
```

<!-- output: examples/19-testing/names.console -->
```console
$ sisu test names.sisu
test "{60: warm, 80: hot} has two steps" ... ok
1 passed, 0 failed, 0 blocked
```

**Try it.** Name the test `"steps at {60, 80}"`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/19-testing/names_once.console -->
```console
$ sisu test names_once.sisu
error[E0132]: the name of a test, a suite or a benchmark is fixed text, and `{` starts no interpolation in it; a brace is written `{{` or `}}`
  --> names_once.sisu:4:16
  |
4 | test "steps at {60, 80}":
  |                ^^^^^^^^
  = fix: write the braces as text: `{{60, 80}}`

error[D0006]: the program was not run: errors found (1)
```

E0132 (`parse.interpolated-name`) points at the braces; a parametrised
test, in the next section, is how a value reaches a name.

</details>

In a package, `sisu test` runs the tests of every module. The
directory `tests/` beside the manifest holds tests from outside: each
file `tests/NAME.sisu` is a program of its own that imports the
package by its name, as a user of the library would, and so sees only
its `pub` items. Here the library `stove` keeps its helper `step`
private and tests it beside the code:

<!-- file: examples/19-testing/stove.pkg/src/lib.sisu -->
```sisu
##! A sauna stove's heat, in steps.

## The heat after `throws` ladles of water on a stove at `base` degrees.
pub def heat_after(base: int, throws: int) -> int:
    return base + step(throws)

def step(throws: int) -> int:
    return 3 * throws

test "a throw adds three degrees":
    assert step(1) == 3
```

<!-- file: examples/19-testing/stove.pkg/tests/public.sisu -->
```sisu
# A black-box test: it sees only what `stove` makes public.
from stove import heat_after

test "no throw keeps the heat":
    assert heat_after(80, 0) == 80

test "two throws":
    assert heat_after(80, 2) == 86
```

<!-- output: examples/19-testing/stove.console -->
```console
$ sisu test
tests of `stove`:
test "a throw adds three degrees" ... ok
1 passed, 0 failed, 0 blocked
tests of `tests/public.sisu`:
test "no throw keeps the heat" ... ok
test "two throws" ... ok
2 passed, 0 failed, 0 blocked
```

- The package's own tests run first, then each file of `tests/` in the
  order of its name, each with its own list and counts under a line
  that names it: the package by its name, a file by its path.
- An import of a private item, `from stove import step`, is E0255 in
  a file of `tests/`, as in any user of the library.

## A test for each case

A test that checks many cases with a loop stops at the first case that
fails, and its report does not say which case it was. A parametrised
test is a test of its own for each case of a list:

<!-- file: examples/19-testing/cases.sisu -->
```sisu
# A parametrised test is a test of its own for each case.

## The Finnish word for a number from 1 to 3, or `None`.
def word(n: int) -> str?:
    match n:
        case 1:
            return Some("yksi")
        case 2:
            return Some("kaksi")
        case 3:
            return Some("kolme")
        case _:
            return None

test "number words" for n, w in [(1, "yksi"), (2, "kaksi"), (4, "neljä")]:
    assert word(n) == Some(w)
```

<!-- output: examples/19-testing/cases.console -->
```console
$ sisu test cases.sisu
test "number words (1, \"yksi\")" ... ok
test "number words (2, \"kaksi\")" ... ok
test "number words (4, \"neljä\")" ... FAILED

test "number words (4, \"neljä\")" ... FAILED
panic: assertion failed: word(n) == Some(w)
  left:  None
  right: Some("neljä")
  at test "number words (4, \"neljä\")" (cases.sisu:16:5)
replay: sisu test --replay 1-c87a4c0eef313560-0.0.1-t2-s0-r0 cases.sisu

2 passed, 1 failed, 0 blocked
```

`for n, w in [...]` takes the pattern and the list of a `for` loop.
Each case is named by its value, so the report says that the case `(4,
"neljä")` fails and the others pass; `--filter` can run that case
alone. *Neljä* is Finnish for "four", and `word` does not know it.

> **From Python.** This is pytest's `@pytest.mark.parametrize`, written
> as part of the test: the names before `in` are the parameters, and
> each element of the list is a case.

## What a failing assertion shows

When an `assert` fails, `sisu test` shows the values that made it
fail, so that you rarely need to add a `print` to find out:

<!-- file: examples/19-testing/asserts.sisu -->
```sisu
# A failing `assert` shows the values that made it fail.

## The average of `temps`, rounded down.
def average(temps: list[int]) -> int:
    var total = 0
    for t in temps:
        total += t
    return total // len(temps)

test "a comparison shows both sides":
    assert average([80, 85, 120]) == 90

test "the parts that only read show their values":
    let temps = [80, 85, 120]
    assert len(temps) == 4

test "a message says what was meant":
    let temps = [70, 75]
    assert average(temps) >= 80, "a sauna at {average(temps)} is too cool"
```

<!-- output: examples/19-testing/asserts.console -->
```console
$ sisu test asserts.sisu
test "a comparison shows both sides" ... FAILED
test "the parts that only read show their values" ... FAILED
test "a message says what was meant" ... FAILED

test "a comparison shows both sides" ... FAILED
panic: assertion failed: average([80, 85, 120]) == 90
  left:  95
  right: 90
  at test "a comparison shows both sides" (asserts.sisu:11:5)
replay: sisu test --replay 1-e4f3e873c7a7fcfa-0.0.1-t0-s0-r0 asserts.sisu

test "the parts that only read show their values" ... FAILED
panic: assertion failed: len(temps) == 4
    len(temps) == 4
    │   │
    3   [80, 85, 120]
  at test "the parts that only read show their values" (asserts.sisu:15:5)
replay: sisu test --replay 1-e4f3e873c7a7fcfa-0.0.1-t1-s0-r0 asserts.sisu

test "a message says what was meant" ... FAILED
panic: assertion failed: average(temps) >= 80
  message: a sauna at 72 is too cool
  left:  72
  right: 80
  at test "a message says what was meant" (asserts.sisu:19:5)
replay: sisu test --replay 1-e4f3e873c7a7fcfa-0.0.1-t2-s0-r0 asserts.sisu

0 passed, 3 failed, 0 blocked
```

- For a comparison, `left:` and `right:` are the values of its two
  sides.
- When parts of the expression only read, such as a name or `len`, a
  tree under the expression shows each part's value: `len(temps)` is 3,
  and `temps` is the list.
- `assert condition, "message"` adds a message, with interpolation, on
  the line `message:`. Write one that says what the condition means.
- `replay:` is a command that runs the failing test again; the section
  "Replaying a failure" is about it.

`expect condition` checks as `assert` does, but a failure does not stop
the test: it is recorded, the test goes on, and at its end the test
fails with every failure it recorded. One run then reports every broken
case:

<!-- file: examples/19-testing/expects.sisu -->
```sisu
# `expect` records a failure, and the test goes on to its end.

## The average of `temps`, rounded down.
def average(temps: list[int]) -> int:
    var total = 0
    for t in temps:
        total += t
    return total // len(temps)

test "an average for each evening":
    expect average([80, 85, 120]) == 95
    expect average([70, 75]) == 73
    expect average([90]) == 90
    expect average([60, 61]) == 61, "two readings a degree apart"
```

<!-- output: examples/19-testing/expects.console -->
```console
$ sisu test expects.sisu
test "an average for each evening" ... FAILED

test "an average for each evening" ... FAILED
expectation failed: average([70, 75]) == 73
  left:  72
  right: 73
  at test "an average for each evening" (expects.sisu:12:5)
expectation failed: average([60, 61]) == 61
  message: two readings a degree apart
  left:  60
  right: 61
  at test "an average for each evening" (expects.sisu:14:5)
replay: sisu test --replay 1-8d28ef2a938b01d6-0.0.1-t0-s0-r0 expects.sisu

0 passed, 1 failed, 0 blocked
```

Both failures are reported, with the sides a failed `assert` shows;
the checks that hold say nothing. An `expect` takes a message too. Use `expect` for checks that do not
depend on each other, and `assert` where a failure makes the lines
after it meaningless.

What a test prints is kept with its result and shown only if it fails,
under `what the test printed:` after the test's name, so a passing
suite stays quiet ([R15](../reference/testing.md#what-a-test-prints)).

## Fixtures are values

A test often needs something set up first, such as a file system with files in it. In Sisu that setup
is a fixture, and a fixture is a value: a function makes it, the test
takes it, and it is dropped when the test ends. Capability fakes are
the commonest fixtures:

<!-- file: examples/19-testing/fixtures.sisu -->
```sisu
# A fixture is a value that a test makes; fakes are the commonest.

## The prices in cents of the lines `name = cents` of `menu.txt`.
def load_prices(fs: Fs) -> dict[String, int] ! IoError:
    var prices: dict[String, int] = {}
    let text = fs.read_to_string("menu.txt")?
    for line in text.lines():
        if let Some((name, cents)) = line.split_once(" = "):
            _ = prices.insert(name.to_string(), cents.parse_int() ?? 0)
    return Ok(prices)

## A file system with a menu of three items, for the tests.
def cafe() -> Fs:
    return Fs.in_memory({"menu.txt": "coffee = 350\ntea = 300\nbun = 280"})

test "every line is a price":
    let prices = load_prices(cafe()).expect("the menu is there")
    assert len(prices) == 3

test "a price in cents":
    let prices = load_prices(cafe()).expect("the menu is there")
    assert prices.get("bun").copied() == Some(280)

test "a test's own file system starts empty"(fs: Fs):
    assert load_prices(fs) == Err(NotFound("menu.txt"))
```

<!-- output: examples/19-testing/fixtures.test -->
```text
test "every line is a price" ... ok
test "a price in cents" ... ok
test "a test's own file system starts empty" ... ok
3 passed, 0 failed, 0 blocked
```

- `cafe()` is the fixture: each test that calls it gets a new file
  system holding the menu, so what one test writes no other test sees.
- A test can name capabilities as its parameters, one of each, and
  receives a fresh fake of each: `(fs: Fs)` gets an empty in-memory file
  system. The same goes for `net: Net`, an in-process network of its
  own, `clock: Clock` or `clock: TestClock`, the test clock, `rand:
  Rand`, a generator seeded by the test, and `env: Env`, `proc: Proc`
  and `stdin: Stdin`, which hold nothing.
- `System.fake(seed)` makes a whole set of fakes in the shape of a
  `System`, for code that takes one.

`get` takes a `str` for a `String` key and gives an optional
reference, which `.copied()` makes an optional `int` to compare.

**Try it.** Leave `.copied()` out, and run the tests.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/19-testing/compare_ref.console -->
```console
$ sisu test fixtures.sisu
error[E0300]: expected `ref int`, found `int`
  --> fixtures.sisu:22:38
   |
22 |     assert prices.get("bun") == Some(280)
   |                                      ^^^
  = fix: make the optional reference an optional value to compare them: `.copied()`

error[D0006]: the program was not run: errors found (1)
```

</details>

Each test runs in an interpreter of its own, and the tests run side
by side, on up to 8 threads (`--jobs N`), their output written in the
order of the file. A test that declares `sys: System` receives the real
one instead: it runs alone, after the others, and cannot be repeated
exactly.

## Fixtures that tests ask for

Mark a function that takes nothing `@fixture`, and a test with a
parameter of its name receives what it returns, made for that test:

<!-- file: examples/19-testing/menu.sisu -->
```sisu
# A `@fixture` makes a value for each test that names it.

## A menu of prices in cents, which says when it is put away.
struct Menu:
    prices: dict[String, int]

impl Drop for Menu:
    def drop(mut self):
        print("menu put away")

@fixture
def menu() -> Menu:
    print("menu set up")
    return Menu(prices={"coffee".to_string(): 350, "bun".to_string(): 280})

test "a bun costs 280"(menu: Menu):
    assert menu.prices.get("bun").copied() == Some(280)

test "tea is not on the menu"(menu: Menu):
    print("looking for tea")
    assert menu.prices.get("tea").copied() == None
```

<!-- output: examples/19-testing/menu.test -->
```text
test "a bun costs 280" ... ok
test "tea is not on the menu" ... ok
2 passed, 0 failed, 0 blocked
```

Each test gets a `Menu` of its own, made by a call of `menu()` before
its first line and dropped when it ends, so the value's `Drop` is the
teardown. `scope="module"` is accepted and still makes one for each
test.

**Try it.** Make the second test expect `Some(300)`, and run it.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/19-testing/menu_tea.console -->
```console
$ sisu test menu.sisu
test "a bun costs 280" ... ok
test "tea is not on the menu" ... FAILED

test "tea is not on the menu" ... FAILED
what the test printed:
  menu set up
  looking for tea
  menu put away
panic: assertion failed: menu.prices.get("tea").copied() == Some(300)
    menu.prices.get("tea").copied() == Some(300)
    │           │          │           │
    │           None       None        Some(300)
    {"coffee": 350, "bun": 280}
  at test "tea is not on the menu" (menu.sisu:21:5)
replay: sisu test --replay 1-f14e765a85b22b4f-0.0.1-t1-s0-r0 menu.sisu

1 passed, 1 failed, 0 blocked
```

The fixture is set up before the test and put away after the failed
`assert`. A `@fixture` that takes a parameter is E0281
(`type.fixture-signature`).

</details>

## A clock that moves when every task waits

The test clock starts at 1970-01-01T00:00:00Z and stands still. It
moves when a test moves it, with `advance` or `set`, and when every
task of the test is waiting: then it jumps to the earliest timer they
wait for. Code that sleeps runs at once under it, and gives the same
result every time:

<!-- file: examples/19-testing/brewing.sisu -->
```sisu
# The test clock moves only when every task waits.

## Brews for `minutes` on `clock`, and gives the minutes back.
def brew(clock: Clock, minutes: int) -> int ! TaskError:
    clock.sleep(Duration.minutes(minutes))?
    return Ok(minutes)

test "two pots brew at once"(clock: TestClock):
    let start = clock.now()
    let total = task.scope do s:
        let coffee = s.spawn(() -> brew(clock, 4))
        let tea = s.spawn(() -> brew(clock, 3))
        return Ok(coffee.join()? + tea.join()?)
    assert total == Ok(7)
    assert clock.now() - start == Duration.minutes(4)

test "one pot after the other"(clock: TestClock):
    let start = clock.now()
    brew(clock, 4).expect("not cancelled")
    brew(clock, 3).expect("not cancelled")
    assert clock.now() - start == Duration.minutes(7)
```

<!-- output: examples/19-testing/brewing.test -->
```text
test "two pots brew at once" ... ok
test "one pot after the other" ... ok
2 passed, 0 failed, 0 blocked
```

In the first test two tasks sleep at once, four minutes and three, and
the clock moves four minutes in all: when both wait, it jumps to three
minutes, the tea task finishes, and when the coffee task is the only
one left waiting, it jumps to four. The second test brews one pot after
the other, and the clock moves seven minutes. Neither test takes any
real time.

## Properties instead of examples

An example-based test checks one input. A property says what must hold
for every input, and `check` tries it on a hundred random ones that a
generator makes, `arbitrary[T]` for a type `T`:

<!-- file: examples/19-testing/sorting.sisu -->
```sisu
# A property must hold for every case that a generator makes.

## The numbers of `xs`, smallest first.
def sort_numbers(xs: Slice[int]) -> list[int]:
    var out: list[int] = []
    for x in xs:
        var i = 0
        while i < len(out) and out[i] < x:
            i += 1
        if i == len(out) or out[i] != x:
            out.insert(i, x)
    return out

test "the numbers come out in order":
    check(arbitrary[list[int]]) do xs:
        let out = sort_numbers(xs)
        return all([out[i] <= out[i + 1] for i in range(len(out) - 1)])

test "every number is kept":
    check(arbitrary[list[int]]) do xs:
        return len(sort_numbers(xs)) == len(xs)
```

The sort leaves out a number equal to one it has already placed, which
no example with distinct numbers would show:

<!-- output: examples/19-testing/sorting.console -->
```console
$ sisu test sorting.sisu
test "the numbers come out in order" ... ok
test "every number is kept" ... FAILED

test "every number is kept" ... FAILED
panic: the property failed for [0, 0] (case 3, shrunk 10 times)
  the property returned False
  at test "every number is kept" (sorting.sisu:20:5)
replay: sisu test --replay 1-9c5428daab4aad00-0.0.1-t1-s0-r0-c1.0.0.1.0.0.0 sorting.sisu

1 passed, 1 failed, 0 blocked
$ sisu test --replay 1-9c5428daab4aad00-0.0.1-t1-s0-r0-c1.0.0.1.0.0.0 sorting.sisu
test "every number is kept" ... FAILED

test "every number is kept" ... FAILED
panic: the property failed for [0, 0] (case 1, shrunk 0 times)
  the property returned False
  at test "every number is kept" (sorting.sisu:20:5)
replay: sisu test --replay 1-9c5428daab4aad00-0.0.1-t1-s0-r0-c1.0.0.1.0.0.0 sorting.sisu

0 passed, 1 failed, 0 blocked
```

- `check(arbitrary[list[int]]) do xs:` runs the block for each case,
  and the block returns whether the property holds for it.
- The first test passes: whatever the bug, the numbers that come out
  are in order.
- The second fails, and `check` shrinks the case that failed, removing
  and lowering what it can while the property still fails: it reports
  `[0, 0]`, the smallest list that shows the bug, two equal numbers.

`arbitrary` makes cases of the program's own types too: the compiler
writes the generator from the type's fields and variants, and draws a
`T: Arbitrary` of generic code through the bound
([G11](../guides/property-test.md)). Saving each failing case to try it
first on the next run, as Python's Hypothesis does, is designed in
[17](../../proposal/17-testing.md#property-based-testing-and-fuzzing)
and not in the compiler.

> **From Python.** `check` and `arbitrary` are Hypothesis's `@given` and
> its strategies, built in: there is nothing to install, and a type
> needs no code of yours to have a generator.

## Replaying a failure

Each failing test prints a `replay:` line, a command whose token holds
everything that decides the run: the code's content hash, the
toolchain's version, which test, the scheduler's seed, the seed of the
case and, for a property, the shrunk case. The second command of the
session in the previous section ran it, and failed the same way at
once, on the shrunk case.

A token fits in a bug report: whoever has the same code can run the
failure again on any machine, since in a test every source of chance
is a fake or the seeded scheduler. It replays one version of the code
only. Once the bug is fixed, by inserting every number, the token is
refused, and the tests are run as usual:

<!-- file: examples/19-testing/sorting_fixed.sisu -->
```sisu
# The sort with its bug fixed: an equal number is kept, not dropped.

## The numbers of `xs`, smallest first.
def sort_numbers(xs: Slice[int]) -> list[int]:
    var out: list[int] = []
    for x in xs:
        var i = 0
        while i < len(out) and out[i] < x:
            i += 1
        out.insert(i, x)
    return out

test "the numbers come out in order":
    check(arbitrary[list[int]]) do xs:
        let out = sort_numbers(xs)
        return all([out[i] <= out[i + 1] for i in range(len(out) - 1)])

test "every number is kept":
    check(arbitrary[list[int]]) do xs:
        return len(sort_numbers(xs)) == len(xs)
```

<!-- output: examples/19-testing/sorting_fixed.console -->
```console
$ sisu test --replay 1-9c5428daab4aad00-0.0.1-t1-s0-r0-c1.0.0.1.0.0.0 sorting_fixed.sisu
error[D0033]: the token is for other code (content hash 9c5428daab4aad00); `sorting_fixed.sisu` has content hash 0b5213361e20538c
$ sisu test sorting_fixed.sisu
test "the numbers come out in order" ... ok
test "every number is kept" ... ok
2 passed, 0 failed, 0 blocked
```

D0033 (`cli.token-other-code`) names the content hash the token was made for and the file's
own, so a replay can never run a different program by mistake, nor one of another toolchain. A test that
takes the real `System` gets it even under `--sim`, and has no token.

## Every small case

Random cases can miss a rare one. `exhaustive[T](max_len=n)` makes
every case instead: every list of at most `n` items, and every number
within `n` of zero. Most bugs show up in a small case, so all the small
ones are a strong test:

<!-- file: examples/19-testing/switches.sisu -->
```sisu
# `exhaustive` makes every small case, rather than random ones.
enum Press:
    On
    Off
    Toggle

## Whether the light is on after `presses`, starting from off.
def light(presses: list[Press]) -> bool:
    var on = False
    for p in presses:
        match p:
            case On:
                on = True
            case Off:
                on = False
            case Toggle:
                on = not on
    return on

test "two toggles at the end change nothing":
    var cases = 0
    check(exhaustive[list[Press]](max_len=4)) do presses:
        cases += 1
        var twice = presses.copy()
        twice.push(Toggle)
        twice.push(Toggle)
        return light(twice) == light(presses)
    assert cases == 121
```

<!-- output: examples/19-testing/switches.test -->
```text
test "two toggles at the end change nothing" ... ok
1 passed, 0 failed, 0 blocked
```

Each of the 121 cases is a list of up to four presses: one empty list,
3 lists of one press, 9 of two, 27 of three and 81 of four. The test counts them to show it. The compiler writes the
generator of `Press` from its variants, as it does for any enum.

## A clear function as the specification

A fast function is often harder to read than a slow one that does the
same. `@refines(reference)` keeps the clear one as the specification of
the fast one, and `sisu test` adds a property test that the two agree
on `arbitrary` cases:

<!-- file: examples/19-testing/words.sisu -->
```sisu
# `@refines` checks a fast function against a clear one on many cases.

## The number of words in `text`, as `split_whitespace` finds them.
def word_count_ref(text: str) -> int:
    return text.split_whitespace().count()

## The same number, from one pass over the characters.
@refines(word_count_ref)
def word_count(text: str) -> int:
    var count = 0
    var in_word = False
    for c in text.chars():
        if c == ' ':
            in_word = False
        elif not in_word:
            in_word = True
            count += 1
    return count

test "two words":
    assert word_count("hei maailma") == 2
```

<!-- output: examples/19-testing/words.console -->
```console
$ sisu test words.sisu
test "two words" ... ok
test "word_count refines word_count_ref" ... FAILED

test "word_count refines word_count_ref" ... FAILED
panic: the property failed for ("a\na",) (case 48, shrunk 8 times)
  the property returned False
  at test "word_count refines word_count_ref" (words.sisu:8:1)
replay: sisu test --replay 1-87c6db98a4f7b08a-0.0.1-t1-s0-r0-c1.0.1.25.1.0.0 words.sisu

1 passed, 1 failed, 0 blocked
```

The test is named `word_count refines word_count_ref`, and it finds
that a line break between two words is not a space to the fast
version: `"a\na"` is one word to it and two to `split_whitespace`. Its
place is the `@refines` line.

**Try it.** Change `c == ' '` to `c.is_whitespace()`, and run the tests
again.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/19-testing/words_fixed.console -->
```console
$ sisu test words_fixed.sisu
test "two words" ... ok
test "word_count refines word_count_ref" ... ok
2 passed, 0 failed, 0 blocked
```

The two functions now agree on every case tried. A case that takes the
reference too long is left out rather than failed, since a clear
function is often slow. For functions that return an `f64`,
`@refines(reference, tolerance=Ulps(2))` lets the results differ in the last two places.

</details>

## Fakes of your own traits

The capability fakes stand in for the world. For a trait of your own,
`cap.Fake[T]` is a fake that the compiler writes from the trait's
methods: it records each call, and returns what the test told it to:

<!-- file: examples/19-testing/reminders.sisu -->
```sisu
# `cap.Fake[T]` is a fake of one of the program's own traits.
error SendError:
    Refused(to: String)

trait Mailer:
    def send(self, to: str, text: str) -> ! SendError

## Reminds each borrower of `loans` that a book is due, and gives the
## number of reminders that could not be sent.
def remind[M: Mailer](mailer: M, loans: list[(str, str)]) -> int:
    var failed = 0
    for who, book in loans:
        if mailer.send(who, "{book} is due tomorrow").is_err():
            failed += 1
    return failed

test "a reminder for each loan":
    let mailer = cap.Fake[Mailer].new()
    mailer.on_send(Ok(()))
    assert remind(mailer, [("aino", "Kalevala"), ("eero", "Sinuhe")]) == 0
    let calls = mailer.calls()
    assert calls[0] == "send(\"aino\", \"Kalevala is due tomorrow\")"
    assert len(calls) == 2

test "a refused reminder is counted":
    let mailer = cap.Fake[Mailer].new()
    mailer.on_send(Err(Refused("roni")))
    mailer.on_send(Ok(()))
    assert remind(mailer, [("roni", "Sinuhe"), ("aino", "Sinuhe")]) == 1
```

<!-- output: examples/19-testing/reminders.test -->
```text
test "a reminder for each loan" ... ok
test "a refused reminder is counted" ... ok
2 passed, 0 failed, 0 blocked
```

- `cap.Fake[Mailer].new()` makes a fake `Mailer`, which `remind` takes
  as it would take a real one.
- `on_send(result)` queues what `send` returns. The results are used
  in order, and the last one again once the others are used: in the
  second test the first reminder is refused and every later one sent.
- `calls()` lists the calls made, each written `method(arguments)`, so
  a test can check what was sent and to whom.
- A method called with nothing queued panics, which fails the test with
  the method's name.

A fake can be made for a trait whose methods all take `self` or `mut
self`, have no type parameters of their own, and have no borrow or
`Self` in their signatures; other traits get E0614
(`comptime.no-fake`). The same machinery gives `cap.Wrap` of chapter
18 for your traits: an interceptor around a real implementation.

> **From Python.** There is no `unittest.mock`. A mock replaces what a
> function reached for by itself; a Sisu function is handed what it
> uses, so a test hands it a fake, and the fake is checked against the
> trait like any other implementation.

## Snapshots

Some results are long to write out in a test: a report, a page of
text, a large struct. `assert_snapshot(value)` stores what the value
looks like the first time, and compares with it on every run after:

<!-- file: examples/19-testing/receipt.sisu -->
```sisu
# A snapshot keeps what a value looked like, and fails when it changes.
from std.test import assert_snapshot

## A receipt: each item and its price in euros, and the total.
def receipt(items: list[(str, int)]) -> String:
    var lines: list[String] = []
    var total = 0
    for name, cents in items:
        lines.push("{name:<8}{cents // 100:>3},{cents % 100:02}")
        total += cents
    lines.push("{"total":<8}{total // 100:>3},{total % 100:02}")
    return lines.join("\n")

test "a receipt for two":
    assert_snapshot(receipt([("coffee", 350), ("bun", 280)]))
```

<!-- output: examples/19-testing/receipt.console -->
```console
$ sisu test receipt.sisu
test "a receipt for two" ... FAILED

test "a receipt for two" ... FAILED
panic: there is no snapshot `a receipt for two` yet (`sisu test --update` stores this one):
coffee    3,50
bun       2,80
total     6,30
  at test "a receipt for two" (receipt.sisu:15:5)
replay: sisu test --replay 1-230ba086fa32ec26-0.0.1-t0-s0-r0 receipt.sisu

0 passed, 1 failed, 0 blocked
$ sisu test --update receipt.sisu
test "a receipt for two" ... ok
1 passed, 0 failed, 0 blocked
stored the new snapshot `a receipt for two`:
coffee    3,50
bun       2,80
total     6,30
$ sisu test receipt.sisu
test "a receipt for two" ... ok
1 passed, 0 failed, 0 blocked
```

- The first run fails, since there is no snapshot yet, and shows what
  `--update` would store.
- `sisu test --update` stores it, in `receipt.snap` beside the program,
  under the test's name, and shows what it stored.
- From then on, the test passes while the receipt looks the same. Keep
  the `.snap` file with the code.

A value that is not text is stored in its `Debug` form, as `{x:?}`
writes it.

**Try it.** Write a euro sign after each amount, `{cents % 100:02} €`,
and run the tests.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/19-testing/receipt_changed.console -->
```console
$ sisu test receipt_changed.sisu
test "a receipt for two" ... FAILED

test "a receipt for two" ... FAILED
panic: the snapshot `a receipt for two` changed (`sisu test --update` accepts the new one):
- coffee    3,50
- bun       2,80
- total     6,30
+ coffee    3,50 €
+ bun       2,80 €
+ total     6,30 €
  at test "a receipt for two" (receipt_changed.sisu:15:5)
replay: sisu test --replay 1-0c3a2103f89cb8aa-0.0.1-t0-s0-r0 receipt_changed.sisu

0 passed, 1 failed, 0 blocked
```

The report is a diff: the lines starting `-` are the stored snapshot,
and those starting `+` what the program gives now. If the change is
what you meant, `sisu test --update` accepts it; read the diff first.

</details>

## Transcripts

A transcript tests a whole program: it runs `main` with arguments, as a
user would at a terminal, and compares what it prints with what the
transcript says. `sisu test` runs every file `tests/NAME.transcript`
beside the program. Here is a sauna's booking tool:

<!-- file: examples/19-testing/sauna.sisu -->
```sisu
# A program whose transcript, `tests/sauna.transcript`, tests it whole.
error BookingError:
    Taken(hour: int)
    Usage

## The bookings in `bookings.txt`, one `HOUR NAME` on each line.
def bookings(fs: Fs) -> dict[int, String]:
    var booked: dict[int, String] = {}
    let text = fs.read_to_string("bookings.txt") else:
        return booked
    for line in text.lines():
        if let Some((hour, name)) = line.split_once(" "):
            _ = booked.insert(hour.parse_int() ?? 0, name.to_string())
    return booked

def main(sys: System) -> ! {BookingError, IoError}:
    var booked = bookings(sys.fs)
    match sys.env.args():
        case ["book", name, hour]:
            let h = hour.parse_int() else:
                return Err(Usage)
            if h in booked:
                return Err(Taken(h))
            _ = booked.insert(h, name.to_string())
            let lines = ["{t} {who}" for t, who in booked]
            sys.fs.write_text("bookings.txt", lines.join("\n"))?
            print("booked {h}:00 for {name}")
        case ["list"]:
            for hour in sorted(booked.keys()):
                print("{hour}:00 {booked[hour]}")
        case _:
            return Err(Usage)
    return Ok(())
```

Its transcript, `tests/sauna.transcript`, books two turns and lists
them:

<!-- file: examples/19-testing/sauna.files/tests/sauna.transcript -->
```text
$ sauna list
17:00 Roni
$ sauna book Aino 18
booked 18:00 for Aino
$ sauna book Eero 18
error: Taken(hour=18) (exit 1)
$ sauna list
17:00 Roni
18:00 Aino
```

<!-- output: examples/19-testing/sauna.console -->
```console
$ sisu test sauna.sisu
transcript "tests/sauna.transcript" ... ok
1 passed, 0 failed, 0 blocked
```

- A `$` line runs `main`, with the words after the program's name as
  its arguments. The lines after it, up to the next `$`, are what it
  prints, standard output and standard error together. A line that
  starts with `>` would be a line of its standard input.
- A run that fails ends with ` (exit N)` on its last line, as the
  third run does.
- The program runs on a fake `System`: an in-memory file system that
  starts with the files of `tests/sauna/`, here a `bookings.txt` that
  books 17:00 for Roni; a clock frozen at the start of 1970; a seeded
  `Rand`; and no network. The fakes last from one `$` line to the next,
  so the second run's booking is there for the last.

**Try it.** Change the message of a booking to `"{name} has the sauna
at {h}:00"`, run the tests, and accept the change with `--update`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/19-testing/sauna_changed.console -->
```console
$ sisu test sauna_changed.sisu
transcript "tests/sauna.transcript" ... FAILED

transcript "tests/sauna.transcript" ... FAILED
what the program printed differs from the transcript (`sisu test --update` accepts it):
  $ sauna list
  17:00 Roni
  $ sauna book Aino 18
- booked 18:00 for Aino
+ Aino has the sauna at 18:00
  $ sauna book Eero 18
  error: Taken(hour=18) (exit 1)
  $ sauna list
  17:00 Roni
  18:00 Aino

0 passed, 1 failed, 0 blocked
$ sisu test --update sauna_changed.sisu
updated the transcript "tests/sauna.transcript":
  $ sauna list
  17:00 Roni
  $ sauna book Aino 18
- booked 18:00 for Aino
+ Aino has the sauna at 18:00
  $ sauna book Eero 18
  error: Taken(hour=18) (exit 1)
  $ sauna list
  17:00 Roni
  18:00 Aino
transcript "tests/sauna.transcript" ... ok
1 passed, 0 failed, 0 blocked
$ sisu test sauna_changed.sisu
transcript "tests/sauna.transcript" ... ok
1 passed, 0 failed, 0 blocked
```

The diff shows the line the program now prints in place of the one in
the transcript. `--update` shows the same diff, writes the transcript
anew, and runs it.

</details>

## Benchmarks

A benchmark sits beside the tests and times a piece of code:

<!-- file: examples/19-testing/bench.sisu -->
```sisu
# A benchmark times a loop; `sisu test` checks it and does not run it.
from std.test import Bench, black_box

## The number of vowels in `word`.
def vowels(word: str) -> int:
    var n = 0
    for c in word.chars():
        if c in ['a', 'e', 'i', 'o', 'u', 'y', 'ä', 'ö']:
            n += 1
    return n

test "the vowels of a short word":
    assert vowels("sauna") == 3

bench "the vowels of a long word"(b: Bench):
    for _ in b.loop():
        black_box(vowels("lentokonesuihkuturbiinimoottoriapumekaanikko"))
```

<!-- output: examples/19-testing/bench.console -->
```console
$ sisu test bench.sisu
test "the vowels of a short word" ... ok
1 passed, 0 failed, 0 blocked
```

- `bench "name"(b: Bench):` declares a benchmark, and `for _ in
  b.loop():` is the loop it times; the harness chooses how many times
  it runs. `Bench` and `black_box` come from `std.test`.
- `black_box(x)` uses a value in a way that the compiler cannot see
  through, so that the work that made it is not optimised away.
- `sisu test` checks a benchmark and does not run it. `sisu bench
  bench.sisu` runs each one once to warm up, times ten samples of its
  loop, and prints the median time of one turn with a 95% interval.
  Two benchmarks differ when their intervals do not overlap. The
  compiler times its interpreter, so compare benchmarks with each
  other, not with compiled code. *Lentokonesuihkuturbiinimoottoriapumekaanikko* is a Finnish
  word for an assistant mechanic of jet engines.

## What sisu test remembers

A test that passed is not run again while the code it reaches is the
same: `sisu test` keeps the results in a file beside the program,
`NAME.test-cache`, and reports a remembered one as `ok (cached)`.
`--no-cache` runs everything:

<!-- file: examples/19-testing/tally.sisu -->
```sisu
# Tests whose results `sisu test` remembers until the code changes.
def tally(prices: list[int]) -> int:
    var total = 0
    for p in prices:
        total += p
    return total

test "an empty bill is nothing":
    assert tally([]) == 0

test "a bun and a coffee":
    assert tally([280, 350]) == 630
```

<!-- output: examples/19-testing/tally.console -->
```console
$ sisu test tally.sisu
test "an empty bill is nothing" ... ok
test "a bun and a coffee" ... ok
2 passed, 0 failed, 0 blocked
$ sisu test tally.sisu
test "an empty bill is nothing" ... ok (cached)
test "a bun and a coffee" ... ok (cached)
2 passed, 0 failed, 0 blocked
$ sisu test --no-cache tally.sisu
test "an empty bill is nothing" ... ok
test "a bun and a coffee" ... ok
2 passed, 0 failed, 0 blocked
```

`sisu test --native` runs the same tests compiled, each in a process of
its own ([G20](../guides/native.md)). A JSON report, coverage
(`--cover`), mutation testing (`--mutate`) and fuzzing (`--fuzz`) are
designed in [17](../../proposal/17-testing.md); of mutation testing,
the compiler already makes the mutants, and the runner that tries the
tests against them is still to come.

## Exercise 19.1: predict the test clock

Open
[`manual/exercises/19-testing/predict_clock.sisu`](../exercises/19-testing/predict_clock.sisu):

<!-- file: exercises/19-testing/predict_clock.sisu -->
```sisu
##! Exercise 19.1, predict the test clock. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of how many seconds the test clock
##! moves in its test, or of how many cases a generator makes. Write
##! each as a literal, then run `sisu test predict_clock.sisu`. The hints
##! are in chapter 19 of the tutorial, under "Exercise 19.1: predict the
##! test clock".

## Waits `seconds` on `clock`.
def wait(clock: Clock, seconds: int) -> ! TaskError:
    clock.sleep(Duration.seconds(seconds))?
    return Ok(())

const ONE_AFTER_THE_OTHER: int = todo("seconds for two waits in a row")
const SIDE_BY_SIDE: int = todo("seconds for two waits in two tasks")
const IN_STEPS: int = todo("seconds for two steps beside one step")
const ADVANCED: int = todo("seconds that the test moved the clock")
const BUSY: int = todo("seconds that a busy loop takes")
const CASES: int = todo("lists of at most two booleans")

test "one wait after the other"(clock: TestClock):
    let start = clock.now()
    wait(clock, 10).expect("not cancelled")
    wait(clock, 20).expect("not cancelled")
    assert (clock.now() - start).as_seconds() == ONE_AFTER_THE_OTHER

test "two waits in two tasks"(clock: TestClock):
    let start = clock.now()
    let done = task.scope do s:
        s.spawn(() -> wait(clock, 10))
        s.spawn(() -> wait(clock, 20))
        return Ok(())
    assert done.is_ok()
    assert (clock.now() - start).as_seconds() == SIDE_BY_SIDE

test "a task of two steps beside a task of one"(clock: TestClock):
    let start = clock.now()
    let done = task.scope do s:
        s.spawn do:
            wait(clock, 5)?
            return wait(clock, 5)
        s.spawn(() -> wait(clock, 8))
        return Ok(())
    assert done.is_ok()
    assert (clock.now() - start).as_seconds() == IN_STEPS

test "the test moves the clock"(clock: TestClock):
    let start = clock.now()
    clock.advance(Duration.minutes(2))
    assert (clock.now() - start).as_seconds() == ADVANCED

test "a busy loop"(clock: TestClock):
    let start = clock.now()
    var total = 0
    for i in range(100000):
        total += i
    assert total > 0
    assert (clock.now() - start).as_seconds() == BUSY

test "every short list of booleans":
    var cases = 0
    check(exhaustive[list[bool]](max_len=2)) do xs:
        cases += 1
        return len(xs) <= 2
    assert cases == CASES
```

<!-- output: solutions/19-testing/predict_clock.console -->
```console
$ sisu test predict_clock.sisu
warning[W0001]: typed hole `seconds for two waits in a row` of type `int`
  --> predict_clock.sisu:14:34
   |
14 | const ONE_AFTER_THE_OTHER: int = todo("seconds for two waits in a row")
   |                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `seconds for two waits in two tasks` of type `int`
  --> predict_clock.sisu:15:27
   |
15 | const SIDE_BY_SIDE: int = todo("seconds for two waits in two tasks")
   |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `seconds for two steps beside one step` of type `int`
  --> predict_clock.sisu:16:23
   |
16 | const IN_STEPS: int = todo("seconds for two steps beside one step")
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `seconds that the test moved the clock` of type `int`
  --> predict_clock.sisu:17:23
   |
17 | const ADVANCED: int = todo("seconds that the test moved the clock")
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `seconds that a busy loop takes` of type `int`
  --> predict_clock.sisu:18:19
   |
18 | const BUSY: int = todo("seconds that a busy loop takes")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `lists of at most two booleans` of type `int`
  --> predict_clock.sisu:19:20
   |
19 | const CASES: int = todo("lists of at most two booleans")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "one wait after the other" ... blocked by todo("seconds for two waits in a row")
test "two waits in two tasks" ... blocked by todo("seconds for two waits in two tasks")
test "a task of two steps beside a task of one" ... blocked by todo("seconds for two steps beside one step")
test "the test moves the clock" ... blocked by todo("seconds that the test moved the clock")
test "a busy loop" ... blocked by todo("seconds that a busy loop takes")
test "every short list of booleans" ... blocked by todo("lists of at most two booleans")
0 passed, 0 failed, 6 blocked
```

<details>
<summary>Hint 1</summary>

The clock moves only when the test moves it, or when every task is
waiting, and then only as far as the earliest timer. Work out, for
each test, the moments at which every task waits.

</details>

<details>
<summary>Hint 2</summary>

A loop that computes waits for nothing. A list of at most two
booleans can have no items, one or two, and each item is one of two
values.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/19-testing/predict_clock.sisu`](../solutions/19-testing/predict_clock.sisu).

## Exercise 19.2: the cases that fail

Open
[`manual/exercises/19-testing/tickets.sisu`](../exercises/19-testing/tickets.sisu):

<!-- file: exercises/19-testing/tickets.sisu -->
```sisu
##! Exercise 19.2, the cases that fail. Kind: fix. Level: 1.
##!
##! A swimming hall's ticket costs 3 euros for a child under 7, 5 euros
##! from 7 to 17 years, 8 euros from 18 to 64, and 5 euros again from 65
##! on. `price` is wrong for some of the cases of its parametrised test.
##! Run `sisu test tickets.sisu`, read which cases fail, and fix `price`,
##! not the test, so that every case passes. The hints are in chapter 19
##! of the tutorial, under "Exercise 19.2: the cases that fail".

## The price in euros of a ticket for someone `age` years old.
def price(age: int) -> int:
    if age < 7:
        return 3
    if age < 17:
        return 5
    if age > 65:
        return 5
    return 8

test "ticket prices" for age, euros in [
    (3, 3),
    (7, 5),
    (17, 5),
    (18, 8),
    (64, 8),
    (65, 5),
    (80, 5),
]:
    assert price(age) == euros
```

<!-- output: solutions/19-testing/tickets.console -->
```console
$ sisu test tickets.sisu
test "ticket prices (3, 3)" ... ok
test "ticket prices (7, 5)" ... ok
test "ticket prices (17, 5)" ... FAILED
test "ticket prices (18, 8)" ... ok
test "ticket prices (64, 8)" ... ok
test "ticket prices (65, 5)" ... FAILED
test "ticket prices (80, 5)" ... ok

test "ticket prices (17, 5)" ... FAILED
panic: assertion failed: price(age) == euros
  left:  8
  right: 5
  at test "ticket prices (17, 5)" (tickets.sisu:29:5)
replay: sisu test --replay 1-6d968e649cb4a43a-0.0.1-t2-s0-r0 tickets.sisu

test "ticket prices (65, 5)" ... FAILED
panic: assertion failed: price(age) == euros
  left:  8
  right: 5
  at test "ticket prices (65, 5)" (tickets.sisu:29:5)
replay: sisu test --replay 1-6d968e649cb4a43a-0.0.1-t5-s0-r0 tickets.sisu

5 passed, 2 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

The name of each failing test is its case: an age and the price the
rule gives for it. Compare each with the rule in the exercise's first
lines.

</details>

<details>
<summary>Hint 2</summary>

Both failing ages are at an edge between two prices. Check whether
each comparison in `price` puts its edge on the right side.

</details>

When every test passes, compare your answer with
[`manual/solutions/19-testing/tickets.sisu`](../solutions/19-testing/tickets.sisu).

## Exercise 19.3: one property for three examples

Open
[`manual/exercises/19-testing/dedupe.sisu`](../exercises/19-testing/dedupe.sisu):

<!-- file: exercises/19-testing/dedupe.sisu -->
```sisu
##! Exercise 19.3, one property for three examples. Kind: write. Level: 2.
##!
##! `dedupe` keeps the first of each number and leaves out the numbers
##! that repeat one before them. Three tests with examples could check it
##! on three lists: [3, 1, 2], [1, 2, 1, 3, 2] and []. A property checks
##! every list instead. Write the body of `dedupe_ok`, which says whether
##! `out` is what `dedupe` should give for `xs`, so that every test
##! passes: the property holds of `dedupe` on a hundred random lists,
##! accepts the three examples, and rejects wrong answers. Run the tests
##! with `sisu test dedupe.sisu`. The hints are in chapter 19 of the
##! tutorial, under "Exercise 19.3: one property for three examples".

## `xs` without the numbers that repeat one before them.
def dedupe(xs: list[int]) -> list[int]:
    var seen: set[int] = set()
    var out: list[int] = []
    for x in xs:
        if not seen.contains(x):
            seen.add(x)
            out.push(x)
    return out

## Whether `out` holds each number of `xs` once, in the order of the
## numbers' first places in `xs`, and nothing else.
def dedupe_ok(xs: list[int], out: list[int]) -> bool:
    return todo("dedupe_ok")

test "the property holds of dedupe":
    check(arbitrary[list[int]]) do xs:
        return dedupe_ok(xs, dedupe(xs))

test "the property accepts the three examples":
    assert dedupe_ok([3, 1, 2], [3, 1, 2])
    assert dedupe_ok([1, 2, 1, 3, 2], [1, 2, 3])
    assert dedupe_ok([], [])

test "the property rejects wrong answers":
    assert not dedupe_ok([1, 2, 1], [1, 2, 1])
    assert not dedupe_ok([1, 2], [1])
    assert not dedupe_ok([2, 1, 2], [1, 2])
    assert not dedupe_ok([1], [1, 5])
```

<!-- output: solutions/19-testing/dedupe.console -->
```console
$ sisu test dedupe.sisu
warning[W0001]: typed hole `dedupe_ok` of type `bool`
  --> dedupe.sisu:26:12
   |
26 |     return todo("dedupe_ok")
   |            ^^^^^^^^^^^^^^^^^

test "the property holds of dedupe" ... blocked by todo("dedupe_ok")
test "the property accepts the three examples" ... blocked by todo("dedupe_ok")
test "the property rejects wrong answers" ... blocked by todo("dedupe_ok")

test "the property holds of dedupe" ... blocked by todo("dedupe_ok")
what the test printed:
  in scope at the typed hole `todo("dedupe_ok")`:
    xs = [5]
    out = [5]

test "the property accepts the three examples" ... blocked by todo("dedupe_ok")
what the test printed:
  in scope at the typed hole `todo("dedupe_ok")`:
    xs = [3, 1, 2]
    out = [3, 1, 2]

test "the property rejects wrong answers" ... blocked by todo("dedupe_ok")
what the test printed:
  in scope at the typed hole `todo("dedupe_ok")`:
    xs = [1, 2, 1]
    out = [1, 2, 1]

0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

The doc comment of `dedupe_ok` names three things that must all hold.
Check each on its own, as a `bool`, and combine them with `and`. The
last test has a wrong answer for each one.

</details>

<details>
<summary>Hint 2</summary>

"Each number once" is a question of counting: a set of the numbers of
`out` is as long as `out` when nothing repeats. "In the order of their
first places" needs the place of each number of `out` in `xs`, which
a small helper function can find.

</details>

<details>
<summary>Hint 3</summary>

Describe the result without computing it the way `dedupe` does. A
property that calls `dedupe` itself, or does what it does, passes
whether `dedupe` is right or wrong.

</details>

When every test passes, compare your answer with
[`manual/solutions/19-testing/dedupe.sisu`](../solutions/19-testing/dedupe.sisu).

## Exercise 19.4: a program and its transcript

Open
[`manual/exercises/19-testing/tally.sisu`](../exercises/19-testing/tally.sisu):

<!-- file: exercises/19-testing/tally.sisu -->
```sisu
##! Exercise 19.4, a program and its transcript. Kind: fix. Level: 2.
##!
##! `tally` prints how often each word of a file appears: the most
##! common first, words with the same count in alphabetical order, and
##! with `--top N` only the first N. Its transcript,
##! `tests/tally.transcript`, records what it must print for a short
##! poem, and the program does not print it yet. Run
##! `sisu test tally.sisu`, read the transcript's diff, and fix the
##! program, not the transcript or the test, so that everything passes.
##! The hints are in chapter 19 of the tutorial, under "Exercise 19.4: a
##! program and its transcript".

import cli

## What the command line can say.
struct Options:
    top: int = 10
    paths: list[Path]

## How often each word of `text` appears, in lower case.
def count_words(text: str) -> dict[String, int]:
    var counts: dict[String, int] = {}
    for word in text.split_whitespace():
        counts.entry(word.to_lowercase()).or_insert(0) += 1
    return counts

def main(sys: System) -> ! {IoError, cli.Error}:
    let options = cli.parse[Options](sys.env.args())?
    for path in options.paths:
        let text = sys.fs.read_to_string(path)?
        let counts = count_words(text)
        for word, n in counts:
            print("{n:>4}  {word}")
    return Ok(())

test "words in any case count together":
    assert count_words("Kissa kissa koira") == {"kissa": 2, "koira": 1}
```

Its transcript, `tests/tally.transcript`, runs it on a poem of three
lines, `tests/tally/poem.txt`:

<!-- file: exercises/19-testing/tally.files/tests/tally.transcript -->
```text
$ tally poem.txt
   3  is
   3  the
   2  sauna
   2  warm
   1  cold
   1  lake
$ tally --top 2 poem.txt
   3  is
   3  the
```

<!-- output: solutions/19-testing/tally.console -->
```console
$ sisu test tally.sisu
test "words in any case count together" ... ok
transcript "tests/tally.transcript" ... FAILED

transcript "tests/tally.transcript" ... FAILED
what the program printed differs from the transcript (`sisu test --update` accepts it):
  $ tally poem.txt
-    3  is
     3  the
     2  sauna
+    3  is
     2  warm
-    1  cold
     1  lake
+    1  cold
  $ tally --top 2 poem.txt
-    3  is
     3  the
+    2  sauna
+    3  is
+    2  warm
+    1  lake
+    1  cold

1 passed, 1 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

The lines with `-` are what the transcript wants and those with `+`
what the program printed. The words and counts are right; their order
and their number are not.

</details>

<details>
<summary>Hint 2</summary>

Chapter 9's section "Counting every word" sorts the counts the way the
transcript wants. `--top` keeps the first N of that order, and a slice
can do it.

</details>

When every test passes, compare your answer with
[`manual/solutions/19-testing/tally.sisu`](../solutions/19-testing/tally.sisu).

## Exercise 19.5: a retry that waits longer each time

Open
[`manual/exercises/19-testing/retry.sisu`](../exercises/19-testing/retry.sisu):

<!-- file: exercises/19-testing/retry.sisu -->
```sisu
##! Exercise 19.5, a retry that waits longer each time. Kind: write. Level: 3.
##!
##! A weather service is busy now and then. `ask` asks it for a town's
##! forecast, and when the service is busy, waits and asks again: 1
##! second after the first try, 2 after the second, 4 after the third,
##! each wait twice the one before, for at most `tries` tries. Write the
##! body of `ask` so that every test passes. The tests give it a fake of
##! the service and the test clock, so they run at once and see every
##! wait. Run them with `sisu test retry.sisu`. The hints are in chapter
##! 19 of the tutorial, under "Exercise 19.5: a retry that waits longer
##! each time".

error ServiceError:
    Busy
    UnknownTown(name: String)

trait Forecasts:
    def today(self, town: str) -> String ! ServiceError

## The forecast for `town`, asked of `service` at most `tries` times
## while it is busy, waiting on `clock` before each try after the first.
## An unknown town is not asked again.
def ask[F: Forecasts](
    service: F,
    clock: Clock,
    town: str,
    tries: int,
) -> String ! {ServiceError, TaskError}:
    return todo("ask")

test "an answer at once"(clock: TestClock):
    let service = cap.Fake[Forecasts].new()
    service.on_today(Ok("snow"))
    let start = clock.now()
    assert ask(service, clock, "Oulu", 3) == Ok("snow")
    assert clock.now() == start
    assert service.calls() == ["today(\"Oulu\")"]

test "busy twice, then an answer"(clock: TestClock):
    let service = cap.Fake[Forecasts].new()
    service.on_today(Err(Busy))
    service.on_today(Err(Busy))
    service.on_today(Ok("rain"))
    let start = clock.now()
    assert ask(service, clock, "Turku", 5) == Ok("rain")
    assert clock.now() - start == Duration.seconds(3)
    assert len(service.calls()) == 3

test "busy every time"(clock: TestClock):
    let service = cap.Fake[Forecasts].new()
    service.on_today(Err(Busy))
    let start = clock.now()
    assert ask(service, clock, "Oulu", 4) == Err(Busy)
    assert clock.now() - start == Duration.seconds(7)
    assert len(service.calls()) == 4

test "an unknown town is asked once"(clock: TestClock):
    let service = cap.Fake[Forecasts].new()
    service.on_today(Err(UnknownTown("Atlantis")))
    let answer = ask(service, clock, "Atlantis", 3)
    assert answer == Err(UnknownTown("Atlantis"))
    assert len(service.calls()) == 1
```

<!-- output: solutions/19-testing/retry.console -->
```console
$ sisu test retry.sisu
warning[W0001]: typed hole `ask` of type `String ! {TaskError, ServiceError}`
  --> retry.sisu:29:12
   |
29 |     return todo("ask")
   |            ^^^^^^^^^^^

test "an answer at once" ... blocked by todo("ask")
test "busy twice, then an answer" ... blocked by todo("ask")
test "busy every time" ... blocked by todo("ask")
test "an unknown town is asked once" ... blocked by todo("ask")

test "an answer at once" ... blocked by todo("ask")
what the test printed:
  in scope at the typed hole `todo("ask")`:
    town = "Oulu"
    tries = 3

test "busy twice, then an answer" ... blocked by todo("ask")
what the test printed:
  in scope at the typed hole `todo("ask")`:
    town = "Turku"
    tries = 5

test "busy every time" ... blocked by todo("ask")
what the test printed:
  in scope at the typed hole `todo("ask")`:
    town = "Oulu"
    tries = 4

test "an unknown town is asked once" ... blocked by todo("ask")
what the test printed:
  in scope at the typed hole `todo("ask")`:
    town = "Atlantis"
    tries = 3

0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Read the tests first. The fake answers with the results queued for
`today`, in order, so each test describes a service: busy twice and
then an answer, or busy every time. The clock tells how long `ask`
waited in all.

</details>

<details>
<summary>Hint 2</summary>

A loop over the tries asks the service once each time. A `match` on
the answer tells three cases apart: an answer, a busy service with
tries left, and anything else, which ends the loop.

</details>

<details>
<summary>Hint 3</summary>

The wait doubles after each one, and there is none after the last try:
four tries wait 1, 2 and 4 seconds, 7 in all. `clock.sleep` can fail
when the task is cancelled, and `?` passes that on.

</details>

When every test passes, compare your answer with
[`manual/solutions/19-testing/retry.sisu`](../solutions/19-testing/retry.sisu).

## What you learned

- Tests live beside the code. `suite` groups them, `@tag` marks them,
  and `--tag`, `--skip-tag` and `--filter` choose them.
- `test "name" for x in [...]:` is a test for each case, named by the
  case.
- A failing `assert` shows both sides of a comparison, the values of
  the parts that only read, and its message. `expect` records its
  failure and lets the test go on.
- A fixture is a value a function makes, or a `@fixture` that a test
  names. A test that names
  capabilities as parameters gets fresh fakes of them; one that takes
  `sys: System` gets the real one, runs alone and has no token.
- The test clock moves only when told to or when every task waits, so
  code that sleeps is tested at once.
- `check` tries a property on the cases of `arbitrary[T]`, and shrinks
  a failing one; `exhaustive[T](max_len=n)` tries every small case.
- `@refines(reference)` tests a fast function against a clear one, and
  `cap.Fake[T]` fakes a trait of your own.
- `assert_snapshot` and transcripts keep whole outputs, and
  `sisu test --update` accepts a change after showing its diff.
- Every failure has a replay token for `sisu test --replay`; `bench`
  and `sisu bench` time code; passing tests are cached.

Chapter 20 is about concurrency: tasks and scopes, what a task may
share, channels and `select`, cancellation and deadlines, and testing
all of it under the seeded scheduler.
