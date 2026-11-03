# Plan with typed holes

This guide writes a program from the top down: the signatures first,
each body a typed hole, then tests that wait for the holes, and then
the bodies, one at a time, until none is left. It assumes functions and
tests, from chapters 3 and 19 of the tutorial.

**See also:** the tutorial's
[chapter 3](../tutorial/03-functions.md#leaving-a-hole) (a first hole)
and [chapter 28](../tutorial/28-code-that-lasts.md#planning-a-change-with-typed-holes)
(a change planned with holes); [R16](../reference/sisu-command.md#sisu-check---holes)
for `sisu check --holes`; the proposal's
[10, Tooling](../../proposal/10-tooling.md) and D-061.

## 1. Write the signatures, with a hole for each body

Start from `main` and write each function it needs with its doc comment
and its type, and `todo("...")` for its body: a typed hole, which checks
as whatever type its place needs. The program compiles, and each hole
is a warning, W0001 (`type.hole`), with its name and the type that its
body must give:

<!-- file: examples/guide-holes/schedule.sisu -->
```sisu
# A program sketched from `main` down, with a typed hole for each body.

## A turn in the sauna: who, and at which hour.
@derive(Eq, Debug)
struct Turn:
    name: String
    hour: int

## The turns that `text` lists, one `HOUR NAME` on each line.
def parse_turns(text: str) -> list[Turn]:
    return todo("read the turns")

## A line for each hour from 17 to 20, with whoever has it, or "free".
def schedule(turns: list[Turn]) -> list[String]:
    return todo("lay out the evening")

def main(sys: System) -> ! IoError:
    let text = sys.fs.read_to_string("turns.txt")?
    for line in schedule(parse_turns(text)):
        print(line)
    return Ok(())

test "turns from text":
    let turns = parse_turns("18 Aino\n20 Eero")
    assert turns == [Turn("Aino", 18), Turn("Eero", 20)]

test "a free hour says so":
    let lines = schedule([Turn("Aino", 18)])
    assert lines == ["17:00 free", "18:00 Aino", "19:00 free", "20:00 free"]
```

<!-- output: examples/guide-holes/schedule.console -->
```console
$ sisu check schedule.sisu
warning[W0001]: typed hole `read the turns` of type `list[Turn]`
  --> schedule.sisu:11:12
   |
11 |     return todo("read the turns")
   |            ^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `lay out the evening` of type `list[String]`
  --> schedule.sisu:15:12
   |
15 |     return todo("lay out the evening")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

$ sisu run schedule.sisu
warning[W0001]: typed hole `read the turns` of type `list[Turn]`
  --> schedule.sisu:11:12
   |
11 |     return todo("read the turns")
   |            ^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `lay out the evening` of type `list[String]`
  --> schedule.sisu:15:12
   |
15 |     return todo("lay out the evening")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

panic: reached the typed hole `todo("read the turns")`
  at parse_turns (schedule.sisu:11:12)
  at main (schedule.sisu:19:26)
```

The program runs until it reaches a hole, and stops there with a panic
that names it, so the parts that are written can be tried at once.
Name each hole by what it must do: the names are the list of what is
left.

## 2. Write the tests, and see them wait

Write the tests next, from the doc comments, before the bodies. A test
that reaches a hole is blocked, not failed: it says nothing yet about
code that does not exist:

<!-- output: examples/guide-holes/blocked.console -->
```console
$ sisu test schedule.sisu
warning[W0001]: typed hole `read the turns` of type `list[Turn]`
  --> schedule.sisu:11:12
   |
11 |     return todo("read the turns")
   |            ^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `lay out the evening` of type `list[String]`
  --> schedule.sisu:15:12
   |
15 |     return todo("lay out the evening")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "turns from text" ... blocked by todo("read the turns")
test "a free hour says so" ... blocked by todo("lay out the evening")
0 passed, 0 failed, 2 blocked
```

The last line counts the tests that pass, fail and wait, which is the
state of the plan.

## 3. Fill the holes one at a time

Fill one hole, and run the tests. Here `parse_turns` is written, and
its test passes while the other still waits:

<!-- file: examples/guide-holes/schedule_read.sisu -->
```sisu
# The sketch with its first hole filled.

## A turn in the sauna: who, and at which hour.
@derive(Eq, Debug)
struct Turn:
    name: String
    hour: int

## The turns that `text` lists, one `HOUR NAME` on each line.
def parse_turns(text: str) -> list[Turn]:
    var turns: list[Turn] = []
    for line in text.lines():
        if let Some((hour, name)) = line.split_once(" "):
            turns.push(Turn(name.to_string(), hour.parse_int() ?? 0))
    return turns

## A line for each hour from 17 to 20, with whoever has it, or "free".
def schedule(turns: list[Turn]) -> list[String]:
    return todo("lay out the evening")

def main(sys: System) -> ! IoError:
    let text = sys.fs.read_to_string("turns.txt")?
    for line in schedule(parse_turns(text)):
        print(line)
    return Ok(())

test "turns from text":
    let turns = parse_turns("18 Aino\n20 Eero")
    assert turns == [Turn("Aino", 18), Turn("Eero", 20)]

test "a free hour says so":
    let lines = schedule([Turn("Aino", 18)])
    assert lines == ["17:00 free", "18:00 Aino", "19:00 free", "20:00 free"]
```

<!-- output: examples/guide-holes/schedule_read.console -->
```console
$ sisu test schedule_read.sisu
warning[W0001]: typed hole `lay out the evening` of type `list[String]`
  --> schedule_read.sisu:19:12
   |
19 |     return todo("lay out the evening")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "turns from text" ... ok
test "a free hour says so" ... blocked by todo("lay out the evening")
1 passed, 0 failed, 1 blocked
```

Fill them in any order: start with the one that the most tests wait
for, or the one whose type you are least sure of, since filling it may
change a signature that the others depend on.

## 4. Check that no hole is left

Fill the last hole, `schedule`, in the same way, as a loop over the
hours. With no hole left there is no W0001, every test runs, and the
program runs to its end:

<!-- output: examples/guide-holes/schedule_done.console -->
```console
$ sisu check schedule_done.sisu
$ sisu test schedule_done.sisu
test "turns from text" ... ok
test "a free hour says so" ... ok
2 passed, 0 failed, 0 blocked
$ sisu run schedule_done.sisu
17:00 free
18:00 Aino
19:00 free
20:00 Eero
```

Read the warnings before you ship: in stage 0, a program with a hole
still builds, with `sisu build --native --release` too, and reaching
the hole is a panic.

**Status.** Holes, W0001 and blocked tests are in stage 0 (D-061).
`sisu check --holes`, which lists each hole with the bindings in scope,
the `requires` around it and the tests that reach it, is designed in
[10](../../proposal/10-tooling.md), and stage 0 answers it with D0009;
so are a release build and a `sisu publish` that refuse a program with
a hole.
