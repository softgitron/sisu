# Coming from Python

This guide ports a short Python script to Sisu, one habit at a time,
and ends with the list of Python habits that Sisu spells another way.
It assumes that you read Python well and have met Sisu's basics, in
chapters 1 to 9 of the tutorial.

**See also:** [R19, False friends](../reference/false-friends.md), with
a wrong program and its diagnostic for each habit; the tutorial's "From
Python" notes, from [chapter 1](../tutorial/01-hello.md) onwards; the
proposal's [02, "Departures from Python, with
reasons"](../../proposal/02-syntax.md#departures-from-python-with-reasons)
and D-062.

## The script

The script reads lines `NAME POINTS` from `scores.txt` and prints each
player's best and average points, skipping the lines it cannot read:

<!-- file: examples/guide-from-python/scores.py -->
```python
# Each player's best and average points, from the lines of scores.txt.


class Player:
    def __init__(self, name):
        self.name = name
        self.points = []

    def best(self):
        return max(self.points) if self.points else None


def parse(line):
    name, points = line.split()
    return name, int(points)


def main():
    players = {}
    with open("scores.txt") as f:
        for line in f:
            try:
                name, points = parse(line)
            except ValueError:
                print(f"skipped: {line.strip()}")
                continue
            players.setdefault(name, Player(name)).points.append(points)
    for player in players.values():
        average = sum(player.points) / len(player.points)
        print(f"{player.name}: best {player.best()}, average {average:.1f}")


main()
```

Its input is this file:

<!-- file: examples/guide-from-python/scores.files/scores.txt -->
```text
Aino 12
Eero 7
Aino 9
Eero seven
Roni 10
Eero 11
```

Python prints the line it skipped and a line for each player, which is
what the port of step 4 prints too.

## 1. Turn each class into a struct

A `struct` declares its fields with their types, and calling its name
builds one, so there is no `__init__` to port
([chapter 7](../tutorial/07-structs-and-enums.md)). This program builds
a player both ways and prints it:

<!-- file: examples/guide-from-python/player.sisu -->
```sisu
# `class Player` and its `__init__`, as a struct.
struct Player:
    name: String
    points: list[int] = []

def main():
    var aino = Player("Aino")
    aino.points.push(12)
    print("{aino.name}: {aino.points}")
    print(Player(name="Roni").points)
```

<!-- output: examples/guide-from-python/player.out -->
```text
Aino: [12]
[]
```

- A field with a default, `points: list[int] = []`, may be left out
  when a player is built. The default is computed for each new player.
- A player is built by position, `Player("Aino")`, or by keyword,
  `Player(name="Roni")`.
- `append` is `push`. A method takes `self` first, as in Python, with
  no type written.
- There is no inheritance (D-012). What a base class shares, a trait
  shares ([chapter 14](../tutorial/14-traits.md)).

## 2. Return an optional value instead of None

A function that may have no answer returns `T?`, and every caller says
what to do without one ([chapter 8](../tutorial/08-errors.md)). This
program asks two players for their best game and prints the answers:

<!-- file: examples/guide-from-python/best.sisu -->
```sisu
# `None` as an optional value: `best` gives an `int?`.
struct Player:
    name: String
    points: list[int] = []

    ## The most points in one game, `None` before the first.
    def best(self) -> int?:
        if self.points.is_empty():
            return None
        return Some(max(self.points))

def main():
    let aino = Player("Aino", [12, 9])
    print(aino.best() ?? 0)
    if let Some(points) = Player("Roni").best():
        print(points)
    else:
        print("Roni has not played")
```

<!-- output: examples/guide-from-python/best.out -->
```text
12
Roni has not played
```

- `best` returns `Some(points)` or `None`, and its type, `int?`, says
  so. An `int?` cannot be used as an `int` by mistake, which in Python
  is a `TypeError` at run time.
- `if self.points:` would be E0302, which step 5 shows: a condition is
  a `bool`, since Sisu has no truthiness. Test the list with
  `is_empty()`.
- `max(self.points)` is Python's, and as Python's raises on an empty
  list, Sisu's panics, so `best` tests the list first.
- The caller gives a default with `??`, or runs a block for a value with
  `if let`.

## 3. Return errors instead of raising exceptions

A function that can fail declares an error set, returns `T ! E`, and
writes each `raise` as `return Err(...)`
([chapter 8](../tutorial/08-errors.md)). This program parses three
lines and prints what became of each:

<!-- file: examples/guide-from-python/parse.sisu -->
```sisu
# `parse` returns an error where Python's raised `ValueError`.
error LineError:
    NotTwoWords
    NotANumber(text: String)

## The name and the points of a line `NAME POINTS`.
def parse(line: str) -> (String, int) ! LineError:
    let words = line.split_whitespace().collect()
    if len(words) != 2:
        return Err(NotTwoWords)
    let points = words[1].parse_int() else:
        return Err(NotANumber(words[1].to_string()))
    return Ok((words[0].to_string(), points))

def main():
    for line in ["Aino 12", "Eero seven", "Roni"]:
        match parse(line):
            case Ok((name, points)):
                print("{name} has {points}")
            case Err(e):
                print("skipped {line}: {e}")
```

<!-- output: examples/guide-from-python/parse.out -->
```text
Aino has 12
skipped Eero seven: NotANumber(text="seven")
skipped Roni: NotTwoWords
```

- `LineError` names both ways a line can be wrong, which Python's
  `ValueError` covered: a word that is not a number, and a line without
  two words.
- `parse_int()` replaces `int(...)`. It gives an `int?`, and its `else`
  block runs when there is no number.
- A caller handles the error with `match`, as here, or with `else`, or
  passes it on with `?`. There is no `try`.

## 4. Give main the file system

A Sisu program reaches a file only through the capabilities that `main`
receives ([chapter 9](../tutorial/09-word-counter.md)), so
`open("scores.txt")` becomes `sys.fs.read_to_string("scores.txt")`.
Here is the whole port, with the struct of step 2 and the `parse` of
step 3. It prints what the script prints:

<!-- file: examples/guide-from-python/scores.sisu -->
```sisu
# scores.py in Sisu: each player's best and average points.

struct Player:
    name: String
    points: list[int] = []

    ## The most points in one game, `None` before the first.
    def best(self) -> int?:
        if self.points.is_empty():
            return None
        return Some(max(self.points))

error LineError:
    NotTwoWords
    NotANumber(text: String)

## The name and the points of a line `NAME POINTS`.
def parse(line: str) -> (String, int) ! LineError:
    let words = line.split_whitespace().collect()
    if len(words) != 2:
        return Err(NotTwoWords)
    let points = words[1].parse_int() else:
        return Err(NotANumber(words[1].to_string()))
    return Ok((words[0].to_string(), points))

def main(sys: System) -> ! IoError:
    var players: dict[String, Player] = {}
    let text = sys.fs.read_to_string("scores.txt")?
    for line in text.lines():
        let (name, points) = parse(line) else:
            print("skipped: {line.trim()}")
            continue
        let player = players.entry(name.copy()).or_insert(Player(name))
        player.points.push(points)
    for player in players.values():
        let total = sum(player.points).to_f64()
        let average = total / len(player.points).to_f64()
        let best = player.best() ?? 0
        print("{player.name}: best {best}, average {average:.1f}")
    return Ok(())
```

<!-- output: examples/guide-from-python/scores.out -->
```text
skipped: Eero seven
Aino: best 12, average 10.5
Eero: best 11, average 9.0
Roni: best 10, average 10.0
```

- `main(sys: System) -> ! IoError` takes the capabilities and may fail
  with an `IoError`. `?` passes the error on.
- The `try` that skips a line is `let ... else:` with `continue`.
- `setdefault` is `entry(...).or_insert(...)`. The name is copied, since
  the dict keeps one copy and the player the other.
- `/` divides floats only, so the total and the count become `f64`
  first.

Without `scores.txt`, the error leaves `main`, and Sisu prints it with
the line it came from and exits with status 1, as Python does with an
uncaught exception:

<!-- output: examples/guide-from-python/missing.console -->
```console
$ sisu run scores.sisu
error: NotFound(path="scores.txt")
  at scores.sisu:28:16
```

## 5. Check for the habits that remain

`sisu check` stops a Python habit that would mean something else, with
a fix or a note that names the Sisu spelling (D-062). This program has
three of them:

<!-- file: examples/guide-from-python/habits.sisu -->
```sisu
# Three Python habits, which `sisu check` refuses with a fix or a note.
def main():
    let points = [12, 9, 11]
    print(points[-1])
    print(sum(points) / len(points))
    if points:
        print("played")
```

<!-- output: examples/guide-from-python/habits.console -->
```console
$ sisu check habits.sisu
error[E0344]: a constant negative index does not compile; count from the end with `^`: `points[^1]`
  --> habits.sisu:4:18
  |
4 |     print(points[-1])
  |                  ^^
  = fix: count from the end with `^`: `^1`

error[E0301]: `/` divides floats; for integers use `//`, or convert to `f64` first
  --> habits.sisu:5:11
  |
5 |     print(sum(points) / len(points))
  |           ^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: use `//` for integer division: `//`
  = fix: divide them as floats: `.to_f64()`, `.to_f64()`

error[E0302]: a condition must be a `bool`, not `list[int]`; Sisu has no truthiness
  --> habits.sisu:6:8
  |
6 |     if points:
  |        ^^^^^^
  = note: write the test explicitly, as in `not xs.is_empty()` or `x != 0`

```

Read E0301's two fixes with care. The first writes `//`, which floors,
Python 2's meaning; the second converts both sides to `f64`, Python 3's
meaning, as step 4 does. A port of Python 3 takes the second.

These are the habits, with the Sisu spelling of each. R19 shows the
diagnostic that each one gets, and which ones carry a fix that writes
the Sisu spelling, `.is_none()` for `x is None`, `x ->` for `lambda
x:` and `struct` for `class` among them, or a note, as `try:` does:

| Python | Sisu |
|---|---|
| `xs[-1]` | `xs[^1]` |
| `7 / 2` | `7 // 2`, or `7.0 / 2.0` |
| `if xs:` | `if not xs.is_empty():` |
| `a if c else b` | `if c: a else: b` |
| `-7 % 3`, the order of a `dict` | the same as Python: 2, and the order of insertion |
| `x = 1`, declaring `x` | `let x = 1`, or `var x = 1` when it changes |
| `'text'` | `"text"`; `'a'` is a `char` |
| `f"{x}"` | `"{x}"` |
| `print(a, b)` | `print("{a} {b}")` |
| `int(text)`, `str(n)` | `text.parse_int()`, `"{n}"` |
| `for c in word`, `word[0]` | `word.chars()`; a `str` has no integer index |
| `", ".join(names)` | `names.join(", ")` |
| `append`, `strip`, `upper` | `push`, `trim`, `to_uppercase` |
| `d.get(k, default)` | `d.get(k) ?? default` |
| `d.get(k) == v` | `d.get(k).copied() == Some(v)`, since `get` gives a reference; a `str` looks up a `String` key |
| `x or default`, `x is None` | `x ?? default`, `x.is_none()` or `if let` |
| `enumerate(xs, start=1)` | `enumerate(xs)`, adding 1 where the number is used |
| `lambda x: e` | `x -> e`, or `it` |
| `:=`, `else` on a loop | `if let`, or a chained condition |
| `a < b > c` | a chain that points one way |
| `class`, `try` and `except` | steps 1 and 3 |

Four habits compile and mean something else, each to remove a trap: a
default argument is computed at each call, each iteration of a loop
binds its own variable, a closure cannot outlive a borrowed loop
variable, and an `int` has 64 bits and panics past them. R19's section
"Python habits whose meaning Sisu changes" shows each.

**Status.** In stage 0, with one of Python's builtins narrower than
[02](../../proposal/02-syntax.md)'s prelude suggests: `enumerate` has
no `start`.
