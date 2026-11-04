# 21. A tour of the standard library

The standard library is a set of modules under `std`, which a program
imports by name, as chapter 17 showed. This chapter visits the ones a
program reaches for first, one section each, with one example: files
and streams, time, randomness, the environment and other programs,
JSON, regular expressions, collections, hashing, encodings, sorting
and searching, and logging. Each section names the module's source,
whose doc comments are its documentation: `sisu doc --std std.fs`
writes the page of `std.fs`, and `sisu test --std` runs the examples
in it. You need about an hour and a half.

## Files and paths

The file system capability of chapter 18 reads and writes whole files.
`std.fs` adds what is made of several of its operations, such as
walking a directory, and the operations on a path's text, which need
no capability:

<!-- file: examples/21-standard-library/files.sisu -->
```sisu
# `std.fs` walks a directory, takes paths apart and reads lines.
import std.fs

def main(sys: System) -> ! IoError:
    if sys.fs.exists("shopping.txt"):
        sys.fs.remove("shopping.txt")?
    for path in fs.walk(sys.fs, "recipes")?:
        if fs.extension(path) != Some("txt"):
            continue
        let lines = fs.read_lines(sys.fs, path)?
        print("{fs.stem(path) ?? "?"}: {len(lines)} ingredients")
        fs.append(sys.fs, "shopping.txt", "{lines.join("\n")}\n".encode())?
    let list = fs.read_lines(sys.fs, "shopping.txt")?
    print("to buy: {len(list)} things")
    return Ok(())
```

With a directory `recipes` that holds two recipes, a `README.md`, and a
directory `old` with one more recipe in it, it prints:

<!-- output: examples/21-standard-library/files.out -->
```text
bread: 3 ingredients
porridge: 2 ingredients
pie: 2 ingredients
to buy: 7 things
```

- `fs.walk(sys.fs, "recipes")` gives every path under the directory,
  depth first, each directory before what is in it.
- `fs.extension(path)` and `fs.stem(path)` take a file name apart:
  `"txt"` and `"bread"` for `recipes/bread.txt`. `fs.parent`,
  `fs.file_name`, `fs.join` and `fs.components` are the others.
- `fs.read_lines` reads a file's lines, and `fs.append` adds bytes at
  the end of a file, making it when it is missing.

The module's functions take any form of a file system, as chapter 18's
`[F: FsOps]` does, so a test passes them `Fs.in_memory`. Its source is
[`compiler/std/fs.sisu`](../../compiler/std/fs.sisu).

## Streams

`std.io` reads and writes streams, data that comes a piece at a time:
standard input, a network connection, bytes in memory. A `BufReader`
reads lines from any of them:

<!-- file: examples/21-standard-library/lines.sisu -->
```sisu
# `std.io` reads a stream a line at a time, here standard input.
from std.io import BufReader, StdinReader

def main(sys: System) -> !:
    var input = BufReader.new(StdinReader.new(sys.stdin))
    var total = 0
    while let Some(line) = input.read_line()?:
        let minutes = line.trim().parse_int() ?? 0
        total += minutes
        print("{minutes:>3} min, {total:>3} in all")
    return Ok(())
```

Given the lines `15`, `20` and `10` on its standard input, it prints:

<!-- output: examples/21-standard-library/lines.out -->
```text
 15 min,  15 in all
 20 min,  35 in all
 10 min,  45 in all
```

- `StdinReader.new(sys.stdin)` makes a stream of the standard input,
  through its capability, and `BufReader.new(...)` reads it in lines.
- `read_line()` gives the next line, `None` at the end of the input,
  or an error, so `while let Some(line) = input.read_line()?:` reads
  them all.
- `main` returns `!` with no set after it, so the compiler works out
  the set from the errors that `?` passes on, as in chapter 8.

A `Reader` gives bytes and a `Writer` takes them; `BytesReader` and
`BytesWriter` are streams over bytes in memory, for tests, and
`BufWriter` gathers small writes into larger ones. The source is
[`compiler/std/io.sisu`](../../compiler/std/io.sisu).

## Time

`DateTime`, `Duration` and the clock are in the prelude. `std.time`
adds two ways of keeping time over a `Clock` that do not drift: a
`Ticker` that ticks on a fixed schedule, and a `Timeline` that keeps
its own time between sleeps:

<!-- file: examples/21-standard-library/ticker.sisu -->
```sisu
# `std.time`'s `Ticker` and `Timeline` keep time that does not drift.
from std.time import Ticker, Timeline

test "a ticker skips the ticks a slow turn missed"(clock: TestClock):
    let start = clock.now()
    var ticker = Ticker.every(clock, Duration.seconds(10))
    var ticks: list[int] = []
    for turn in range(4):
        ticker.wait().expect("not cancelled")
        ticks.push((clock.now() - start).as_seconds())
        if turn == 1:
            clock.advance(Duration.seconds(25))
    assert ticks == [10, 20, 50, 60]
    assert ticker.missed == 2

test "a timeline does not add up the work between sleeps"(clock: TestClock):
    var timeline = Timeline.start(clock)
    let start = timeline.now
    timeline.sleep(Duration.seconds(30)).expect("not cancelled")
    clock.advance(Duration.seconds(5))
    timeline.sleep(Duration.seconds(30)).expect("not cancelled")
    assert clock.now() - start == Duration.seconds(60)
```

<!-- output: examples/21-standard-library/ticker.test -->
```text
test "a ticker skips the ticks a slow turn missed" ... ok
test "a timeline does not add up the work between sleeps" ... ok
2 passed, 0 failed, 0 blocked
```

- `Ticker.every(clock, period)` ticks at the start plus one period,
  two periods, and so on, whatever each turn of the loop took. A tick
  that passes while the loop is busy is skipped, not fired late, and
  counted in `missed`: the 25 seconds of work passed the ticks at 30
  and 40, so the next tick is at 50.
- `Timeline.start(clock)` keeps a time of its own, and `sleep(d)` moves
  it on by exactly `d`, then waits until the clock is there. The five
  seconds of work between the two sleeps do not push the second one
  later.

Both run in real time on `sys.clock` and at once on the test clock, so
the same code is tested without waiting. The source is
[`compiler/std/time.sisu`](../../compiler/std/time.sisu).

## Randomness

`Rand`, the capability, gives random numbers: `between(lo, hi)`,
`unit()`, `coin()` and `shuffle(xs)`. `std.rand` adds choosing and
sampling, and other generators:

<!-- file: examples/21-standard-library/dice.sisu -->
```sisu
# `std.rand` chooses, samples and shuffles with any `Rand`.
import std.rand

def main():
    let r = Rand.seeded(2026)
    print("a die: {[r.between(1, 7) for _ in range(6)]}")
    print("first to the sauna: {rand.choose(r, ["Aino", "Eero", "Roni"])}")
    print("three of ten: {rand.sample(r, 10, 3)}")
    var turns = ["Aino", "Eero", "Roni", "Liisa"]
    r.shuffle(mut turns)
    print("the order: {turns}")
```

<!-- output: examples/21-standard-library/dice.out -->
```text
a die: [6, 6, 1, 1, 2, 2]
first to the sauna: Some("Eero")
three of ten: [9, 3, 4]
the order: ["Liisa", "Aino", "Roni", "Eero"]
```

- `Rand.seeded(2026)` gives the same numbers on every run and every
  machine, which suits tests and examples; a program that must be
  unpredictable uses `sys.rand`.
- `rand.choose(r, xs)` picks one item, `None` for an empty list, and
  `rand.sample(r, n, k)` picks `k` different numbers below `n`.
- `rand.fast(seed)` is a faster seeded generator, and
  `rand.secure(source)` one fit for keys and passwords.

The source is [`compiler/std/rand.sisu`](../../compiler/std/rand.sisu).

## The environment and other programs

`std.env` reads the variables a program needs, and `std.process` runs
other programs, each through its capability:

<!-- file: examples/21-standard-library/environment.sisu -->
```sisu
# `std.env` reads variables a program needs; `std.process` runs programs.
import std.env
from std.process import Command

test "variables as numbers, lists and text":
    let vars = Env.fake(vars={"SAUNA_HEAT": "85", "PATH": "/bin:/usr/bin"})
    assert env.get_int(vars, "SAUNA_HEAT") == Ok(Some(85))
    assert env.get_list(vars, "PATH") == ["/bin", "/usr/bin"]
    assert env.expand(vars, "heat: $SAUNA_HEAT") == "heat: 85"
    assert env.require(vars, "HOME") == Err(Missing("HOME"))

test "a command run to its end":
    let proc = Proc.fake({"date": "Tue Nov 3"})
    assert Command.new("date").output(proc) == Ok("Tue Nov 3")
    assert Command.new("cal").arg("-3").output(proc) == Err(NotFound("cal"))
```

<!-- output: examples/21-standard-library/environment.test -->
```text
test "variables as numbers, lists and text" ... ok
test "a command run to its end" ... ok
2 passed, 0 failed, 0 blocked
```

- `env.get_int`, `env.get_list` and `env.expand` read a variable as a
  number, as a list split at `:`, and into text where `$NAME` stands.
  `env.require` gives `Missing` for a variable that is not set, so a
  program can say which one it needs.
- `Command.new("date").arg(...)` builds a command, `run(proc)` runs it
  to its end, and `output(proc)` gives its standard output, or an
  error when it fails or is not there.
- `Proc.fake({...})` runs only the programs it was given, each printing
  its text. Under `sisu run`, `sys.proc` runs real programs.

The sources are [`compiler/std/env.sisu`](../../compiler/std/env.sisu)
and [`compiler/std/process.sisu`](../../compiler/std/process.sisu).

## JSON

`std.json` writes a value as JSON and reads JSON back into a type. The
type is the schema: text that does not fit it is an error that says
where:

<!-- file: examples/21-standard-library/booking_json.sisu -->
```sisu
# `std.json` writes a value as JSON, and reads it back into its type.
import std.json

struct Booking:
    name: String
    hour: int
    towels: int = 1

def main():
    let aino = Booking(name="Aino", hour=18)
    print(json.encode(aino))
    print(json.decode[Booking](r"""{"name": "Eero", "hour": 19}"""))
    print(json.decode[Booking](r"""{"name": "Roni", "hour": "late"}"""))
    let hours = json.parse(r"""{"open": [17, 18, 19], "closed": false}""")
    if let Ok(value) = hours, let Some(open) = value.get("open"):
        print("open at {open}")
```

<!-- output: examples/21-standard-library/booking_json.out -->
```text
{"name":"Aino","hour":18,"towels":1}
Ok(Booking(name="Eero", hour=19, towels=1))
Err(Mismatch(path="$.hour", expected="an integer"))
open at [17,18,19]
```

- `json.encode(value)` writes a struct as an object of its fields, a
  list as an array, an optional value as the value or `null`.
- `json.decode[Booking](text)` reads the text into a `Booking`. A field
  that the text leaves out takes its default, as `towels` does, and a
  value of the wrong kind is `Mismatch`, with the path to it: `$.hour`
  is the field `hour` of the top object. A refinement type's predicate
  and a struct's `invariant` are checked as the text is read, too.
- `json.parse(text)` reads any JSON into a `Json` value, for text whose
  shape is not known in advance; `get` finds a member of an object.

The JSON is written in raw literals, `r"""..."""`, in which quotes need
no backslash and braces are not interpolation. The source is
[`compiler/stage1/std/json.sisu`](../../compiler/stage1/std/json.sisu),
stage 1's copy of the module.

## Regular expressions

`std.regex` finds patterns in text, in time linear in the length of the
text, whatever the pattern:

<!-- file: examples/21-standard-library/times.sisu -->
```sisu
# `std.regex` finds, takes apart and replaces patterns in text.
from std.regex import Regex, RegexError

def main() -> ! RegexError:
    let time = Regex.new(r"(\d{1,2}):(\d\d)")?
    let text = "Sauna at 17:30, swim at 18:05, tea at 9:15."
    for m in time.find_all(text):
        print("{m.group(1) ?? ""} hours, {m.group(2) ?? ""} minutes")
    print(time.replace_all(text, "$1.$2"))
    print(time.is_match("no times here"))
    return Ok(())
```

<!-- output: examples/21-standard-library/times.out -->
```text
17 hours, 30 minutes
18 hours, 05 minutes
9 hours, 15 minutes
Sauna at 17.30, swim at 18.05, tea at 9.15.
False
```

- `Regex.new(pattern)` compiles a pattern, and gives an error for one
  that is not valid. `\d` is a digit, `{1,2}` one or two of what comes
  before it, and parentheses make groups.
- `find_all` gives each match, and `m.group(1)` the text of its first
  group. `replace_all` replaces each match, with `$1` for a group, and
  `is_match` asks whether there is one.

The engine never backtracks, so no pattern can make a match take
exponential time; in exchange, it has no back-references and no
look-around. The source, with the whole syntax in its first comment,
is [`compiler/std/regex.sisu`](../../compiler/std/regex.sisu).

## Collections

Beside the prelude's `list`, `dict` and `set`, `std.collections` has
collections for a few common jobs:

<!-- file: examples/21-standard-library/queues.sisu -->
```sisu
# `std.collections` has a queue, a heap, a sorted dict and a set of bits.
from std.collections import BitSet, Deque, Heap, SortedDict

def main():
    var door: Deque[str] = Deque.new()
    door.push_back("Aino")
    door.push_back("Eero")
    door.push_front("Roni")
    print("first in: {door.pop_front()}")
    var bookings: Heap[(int, str)] = Heap.new()
    bookings.push((19, "Eero"))
    bookings.push((17, "Liisa"))
    bookings.push((18, "Aino"))
    print("earliest: {bookings.pop()}")
    var visits: SortedDict[str, int] = SortedDict.new()
    _ = visits.insert("Roni", 2)
    _ = visits.insert("Aino", 5)
    print("by name: {visits.keys()}")
    var hours = BitSet.new()
    hours.insert(17)
    hours.insert(19)
    print("17 booked: {hours.contains(17)}, 18 booked: {hours.contains(18)}")
```

<!-- output: examples/21-standard-library/queues.out -->
```text
first in: Some("Roni")
earliest: Some((17, "Liisa"))
by name: ["Aino", "Roni"]
17 booked: True, 18 booked: False
```

- A `Deque` is a queue with two ends: `push_back`, `push_front`,
  `pop_front` and `pop_back` take a short time on average, however long
  the queue.
- A `Heap` gives its smallest value first, whatever order the values
  came in: here the booking with the earliest hour.
- A `SortedDict` keeps its keys in order, where a `dict` keeps them in
  the order they were added; `SortedSet` is the same for a set.
- A `BitSet` is a set of small numbers, one bit each.

The source is
[`compiler/std/collections.sisu`](../../compiler/std/collections.sisu).

## Hashing

A hash function turns any amount of data into a short value that
changes when the data does. `std.hash` has three kinds:

<!-- file: examples/21-standard-library/digests.sisu -->
```sisu
# `std.hash` digests bytes: a checksum, a fast hash, a secure one.
import std.hash
from std.encoding import hex_encode

def main():
    let data = b"Hei, maailma!"
    print("crc32:  {hash.crc32(data)}")
    print("fnv1a:  {hash.fnv1a_64(data)}")
    print("sha256: {hex_encode(hash.sha256(data))}")
    print("changed: {hex_encode(hash.sha256(b"Hei, maailma?"))}")
```

<!-- output: examples/21-standard-library/digests.out -->
```text
crc32:  2497165204
fnv1a:  2103061409021818564
sha256: 77603fd56cfb80f362cf02f5adf29a79a4e9a1535a60da886a9899a72b545cc9
changed: 8ebd12a8814ae6f81949d1dc33f9c054d97df7f0be65d325c96ec4744c0e673d
```

- `crc32` is a checksum, the one of zip and PNG, which catches data
  damaged by accident.
- `fnv1a_64` is a fast hash for keys of a table of your own.
- `sha256` is cryptographic: no one knows how to find two texts with
  the same digest, so a digest can stand for its data. `sha512` and `blake3` are the
  others. Changing one character changes the whole digest.

The source is [`compiler/std/hash.sisu`](../../compiler/std/hash.sisu).

## Encodings

`std.encoding` writes bytes as text, and numbers as bytes, and reads
them back:

<!-- file: examples/21-standard-library/encodings.sisu -->
```sisu
# `std.encoding` writes bytes as text and numbers as bytes, and back.
from std.encoding import base64_decode, base64_encode, hex_encode
from std.encoding import varint_decode, varint_encode

def main():
    let data = "löyly".encode()
    print(hex_encode(data))
    let text = base64_encode(data)
    print(text)
    print(base64_decode(text) == Ok(data))
    print(base64_decode("not base64!"))
    print(hex_encode(varint_encode(300)))
    print(varint_decode(varint_encode(300)))
```

<!-- output: examples/21-standard-library/encodings.out -->
```text
6cc3b6796c79
bMO2eWx5
True
Err(Invalid(at=3))
ac02
Ok((300, 2))
```

- `hex_encode` writes each byte as two hexadecimal digits: *ö* is two
  bytes in UTF-8, `c3b6`.
- `base64_encode` and `base64_decode` write bytes as letters, digits,
  `+` and `/`, as email and data URLs do; text that is not base64 is an
  error with the place where it went wrong.
- `varint_encode` writes a number in as few bytes as it needs, as
  Protocol Buffers does, and `varint_decode` gives the number and how
  many bytes it took.

The source is
[`compiler/std/encoding.sisu`](../../compiler/std/encoding.sisu).

## Sorting and searching

`list.sort()` and `sorted` sort by a type's order. `std.sort` sorts by
a key, or without keeping equal values in their order, and
`std.search` finds values in a sorted list:

<!-- file: examples/21-standard-library/ordering.sisu -->
```sisu
# `std.sort` sorts in place; `std.search` finds in sorted lists.
import std.search
import std.sort

def main():
    var words = ["löyly", "kiuas", "sauna", "vihta", "lauteet", "kiulu"]
    sort.sort_by_key(mut words, (w) -> len(w))
    print(words)
    let hours = [9, 12, 15, 17, 18, 21]
    print(search.binary_search(hours, 17))
    print(search.binary_search(hours, 16))
    print(search.bisect_left(hours, 16))
    var temps = [80, 95, 70, 88, 101, 76]
    search.select_nth(mut temps, 2)
    print(temps[2])
```

<!-- output: examples/21-standard-library/ordering.out -->
```text
["löyly", "kiuas", "sauna", "vihta", "kiulu", "lauteet"]
Some(3)
None
3
80
```

- `sort.sort_by_key(mut words, (w) -> len(w))` sorts by length and is
  stable: words of the same length keep their order. `sort_unstable`
  needs no extra room, and may not.
- `search.binary_search` finds a value in a sorted list, `Some` of its
  index or `None`, in time logarithmic in the list's length.
  `bisect_left` gives where a value would go.
- `search.select_nth(mut xs, k)` puts the value that would be at index
  `k` in a sorted list there, without sorting the rest: here the third
  lowest temperature.

The sources are [`compiler/std/sort.sisu`](../../compiler/std/sort.sisu)
and [`compiler/std/search.sisu`](../../compiler/std/search.sisu).

## Logging

`log`, from `std.log`, is one of the few modules that every module sees
without an import, with `task` and `cap`, as chapter 18 said. It writes
records to standard error, each with its level:

<!-- file: examples/21-standard-library/logging.sisu -->
```sisu
# `std.log` writes records to standard error, each with its level.
def heat(degrees: int) -> bool:
    log.debug("asked to heat to {degrees}")
    if degrees > 110:
        log.error("{degrees} degrees is too hot; the stove stays off")
        return False
    log.info("heating to {degrees}")
    return True

def main():
    print(heat(80))
    print(heat(120))
```

<!-- output: examples/21-standard-library/logging.console -->
```console
$ sisu run logging.sisu
debug: asked to heat to 80
info: heating to 80
True
debug: asked to heat to 120
error: 120 degrees is too hot; the stove stays off
False
```

The levels are `debug`, `info`, `warn` and `error`. The records go to
standard error and the program's results to standard output, so a
shell can keep them apart, and inside a task each record names the
task, as chapter 20 showed. The source is
[`compiler/std/log.sisu`](../../compiler/std/log.sisu).

## Exercise 21.1: predict the library's answers

Open
[`manual/exercises/21-standard-library/predict_library.sisu`](../exercises/21-standard-library/predict_library.sisu):

<!-- file: exercises/21-standard-library/predict_library.sisu -->
```sisu
##! Exercise 21.1, predict the library's answers. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_library.sisu`. The hints are in chapter 21 of the
##! tutorial, under "Exercise 21.1: predict the library's answers".

import std.fs
import std.json
import std.search
from std.encoding import hex_encode
from std.regex import Regex

const STEM: str = todo("the stem of a file name")
const PARENT: str = todo("the directory that holds a file")
const HEX: str = todo("two bytes in hexadecimal")
const PLACE: int = todo("where 25 would go in a sorted list")
const MATCHES: int = todo("how many runs of digits there are")
const ENCODED: str = todo("a list of words as JSON")

test "the stem of a file name":
    assert (fs.stem(Path.new("notes/todo.txt")) ?? "") == STEM

test "the directory that holds a file":
    let parent = fs.parent(Path.new("notes/old/todo.txt"))
    assert "{parent ?? Path.new("")}" == PARENT

test "two bytes in hexadecimal":
    assert hex_encode(b"AB") == HEX

test "where 25 would go":
    assert search.bisect_left([10, 20, 30, 40], 25) == PLACE

test "runs of digits":
    let digits = Regex.new(r"\d+").expect("a valid pattern")
    assert len(digits.find_all("1 sauna, 2 swims, 10 coffees")) == MATCHES

test "words as JSON":
    assert json.encode(["sauna", "löyly"]) == ENCODED
```

<!-- output: solutions/21-standard-library/predict_library.console -->
```console
$ sisu test predict_library.sisu
warning[W0001]: typed hole `the stem of a file name` of type `str`
  --> predict_library.sisu:14:19
   |
14 | const STEM: str = todo("the stem of a file name")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the directory that holds a file` of type `str`
  --> predict_library.sisu:15:21
   |
15 | const PARENT: str = todo("the directory that holds a file")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `two bytes in hexadecimal` of type `str`
  --> predict_library.sisu:16:18
   |
16 | const HEX: str = todo("two bytes in hexadecimal")
   |                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `where 25 would go in a sorted list` of type `int`
  --> predict_library.sisu:17:20
   |
17 | const PLACE: int = todo("where 25 would go in a sorted list")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `how many runs of digits there are` of type `int`
  --> predict_library.sisu:18:22
   |
18 | const MATCHES: int = todo("how many runs of digits there are")
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a list of words as JSON` of type `str`
  --> predict_library.sisu:19:22
   |
19 | const ENCODED: str = todo("a list of words as JSON")
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "the stem of a file name" ... blocked by todo("the stem of a file name")
test "the directory that holds a file" ... blocked by todo("the directory that holds a file")
test "two bytes in hexadecimal" ... blocked by todo("two bytes in hexadecimal")
test "where 25 would go" ... blocked by todo("where 25 would go in a sorted list")
test "runs of digits" ... blocked by todo("how many runs of digits there are")
test "words as JSON" ... blocked by todo("a list of words as JSON")
0 passed, 0 failed, 6 blocked
```

<details>
<summary>Hint 1</summary>

A file's stem is its name without the extension; its parent is the
path without its last part. Each byte is two hexadecimal digits, and
`A` is 65, or `41` in hexadecimal.

</details>

<details>
<summary>Hint 2</summary>

`bisect_left` gives the index of the first value not smaller than the
one asked for. `\d+` matches one digit or more, as many as there are
in a row. JSON writes no spaces between items.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/21-standard-library/predict_library.sisu`](../solutions/21-standard-library/predict_library.sisu).

## Exercise 21.2: a field of the wrong type

Open
[`manual/exercises/21-standard-library/booking.sisu`](../exercises/21-standard-library/booking.sisu):

<!-- file: exercises/21-standard-library/booking.sisu -->
```sisu
##! Exercise 21.2, a field of the wrong type. Kind: fix. Level: 1.
##!
##! A sauna's booking system sends each booking as JSON, and
##! `parse_booking` reads one into a `Booking`. Two of its tests fail.
##! Run `sisu test booking.sisu`, read the reports, and fix the struct
##! `Booking`, not the tests or the JSON, so that every test passes. The
##! hints are in chapter 21 of the tutorial, under "Exercise 21.2: a
##! field of the wrong type".

import std.json

## A booking of the sauna: who, at which hour, and whether it is paid.
struct Booking:
    name: String
    hour: String
    paid: bool = False

## The booking that the JSON `text` writes.
def parse_booking(text: str) -> Booking ! json.JsonError:
    return json.decode[Booking](text)

test "a booking":
    let booking = parse_booking(r"""{"name": "Aino", "hour": 18}""")
    assert booking.is_ok()

test "a booking that is paid":
    let text = r"""{"name": "Eero", "hour": 19, "paid": true}"""
    let booking = parse_booking(text)
    assert booking.is_ok()

test "text that is not JSON":
    assert parse_booking("Aino at six").is_err()
```

<!-- output: solutions/21-standard-library/booking.console -->
```console
$ sisu test booking.sisu
test "a booking" ... FAILED
test "a booking that is paid" ... FAILED
test "text that is not JSON" ... ok

test "a booking" ... FAILED
panic: assertion failed: booking.is_ok()
    booking.is_ok()
    │
    Err(Mismatch(path="$.hour", expected="a string"))
  at test "a booking" (booking.sisu:24:5)
replay: sisu test --replay 1-8803fbe97066ac53-0.0.1-t0-s0-r0 booking.sisu

test "a booking that is paid" ... FAILED
panic: assertion failed: booking.is_ok()
    booking.is_ok()
    │
    Err(Mismatch(path="$.hour", expected="a string"))
  at test "a booking that is paid" (booking.sisu:29:5)
replay: sisu test --replay 1-8803fbe97066ac53-0.0.1-t1-s0-r0 booking.sisu

1 passed, 2 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

The error under `booking` names the path in the JSON where the text
does not fit the type, and what the type expected there.

</details>

<details>
<summary>Hint 2</summary>

The JSON writes the hour as a number. The struct is the schema, so the
struct is what changes.

</details>

When every test passes, compare your answer with
[`manual/solutions/21-standard-library/booking.sisu`](../solutions/21-standard-library/booking.sisu).

## Exercise 21.3: times in a text

Open
[`manual/exercises/21-standard-library/times.sisu`](../exercises/21-standard-library/times.sisu):

<!-- file: exercises/21-standard-library/times.sisu -->
```sisu
##! Exercise 21.3, times in a text. Kind: complete. Level: 2.
##!
##! `times` finds every time of day in a text, written as an hour of one
##! or two digits, a colon and two digits of minutes, and gives each as
##! minutes after midnight. Replace the hole with the pattern so that
##! every test passes. Run the tests with `sisu test times.sisu`. The
##! hints are in chapter 21 of the tutorial, under "Exercise 21.3: times
##! in a text".

from std.regex import Regex, RegexError

## The times of day in `text`, in minutes after midnight.
def times(text: str) -> list[int] ! RegexError:
    let pattern = Regex.new(todo("the pattern"))?
    var found: list[int] = []
    for m in pattern.find_all(text):
        let hours = (m.group(1) ?? "").parse_int() ?? 0
        let minutes = (m.group(2) ?? "").parse_int() ?? 0
        found.push(hours * 60 + minutes)
    return Ok(found)

test "times with two digits":
    assert times("Sauna at 17:30, swim at 18:05.") == Ok([1050, 1085])

test "an hour of one digit":
    assert times("Tea at 9:15 and 21:00") == Ok([555, 1260])

test "no time":
    assert times("no times here, and 12:3 has one digit") == Ok([])
```

<!-- output: solutions/21-standard-library/times.console -->
```console
$ sisu test times.sisu
warning[W0001]: typed hole `the pattern` of type `str`
  --> times.sisu:14:29
   |
14 |     let pattern = Regex.new(todo("the pattern"))?
   |                             ^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("the pattern")`:
  text = "Sauna at 17:30, swim at 18:05."
in scope at the typed hole `todo("the pattern")`:
  text = "Tea at 9:15 and 21:00"
in scope at the typed hole `todo("the pattern")`:
  text = "no times here, and 12:3 has one digit"
test "times with two digits" ... blocked by todo("the pattern")
test "an hour of one digit" ... blocked by todo("the pattern")
test "no time" ... blocked by todo("the pattern")
0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

The code after the hole reads two groups from each match: the hours,
then the minutes. The pattern needs a group for each, with a colon
between them.

</details>

<details>
<summary>Hint 2</summary>

The section "Regular expressions" shows how many digits a part
matches. A raw literal keeps the backslashes of `\d` as they are.

</details>

When every test passes, compare your answer with
[`manual/solutions/21-standard-library/times.sisu`](../solutions/21-standard-library/times.sisu).

## Exercise 21.4: sizes by kind of file

Open
[`manual/exercises/21-standard-library/sizes.sisu`](../exercises/21-standard-library/sizes.sisu):

<!-- file: exercises/21-standard-library/sizes.sisu -->
```sisu
##! Exercise 21.4, sizes by kind of file. Kind: write. Level: 2.
##!
##! `sizes_by_extension` adds up the sizes of the files under a directory,
##! at any depth, for each extension, so that a user sees what fills a
##! disk. Write its body so that every test passes. Run the tests with
##! `sisu test sizes.sisu`. The hints are in chapter 21 of the tutorial,
##! under "Exercise 21.4: sizes by kind of file".

## The total size in bytes of the files under `dir`, at any depth, for
## each extension; a file without an extension counts under "".
def sizes_by_extension[F: FsOps](
    files: F,
    dir: Path,
) -> dict[String, int] ! IoError:
    return todo("sizes_by_extension")

## Photos in two directories, a note and a file without an extension.
def disk() -> Fs:
    return Fs.in_memory(
        {
            "photos/a.jpg": "12345",
            "photos/b.jpg": "123",
            "photos/notes.txt": "ab",
            "photos/2025/c.jpg": "1234567",
            "photos/README": "x",
        }
    )

test "every kind of file":
    let sizes = sizes_by_extension(disk(), "photos")
    assert sizes == Ok({"jpg": 15, "txt": 2, "": 1})

test "a directory deeper down":
    assert sizes_by_extension(disk(), "photos/2025") == Ok({"jpg": 7})

test "a directory that is not there":
    let sizes = sizes_by_extension(disk(), "music")
    assert sizes == Err(NotFound("music"))
```

<!-- output: solutions/21-standard-library/sizes.console -->
```console
$ sisu test sizes.sisu
warning[W0001]: typed hole `sizes_by_extension` of type `dict[String, int] ! IoError`
  --> sizes.sisu:15:12
   |
15 |     return todo("sizes_by_extension")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("sizes_by_extension")`:
  dir = "photos"
in scope at the typed hole `todo("sizes_by_extension")`:
  dir = "photos/2025"
in scope at the typed hole `todo("sizes_by_extension")`:
  dir = "music"
test "every kind of file" ... blocked by todo("sizes_by_extension")
test "a directory deeper down" ... blocked by todo("sizes_by_extension")
test "a directory that is not there" ... blocked by todo("sizes_by_extension")
0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

`import std.fs` gives the walk and the extension of the section "Files
and paths". The parameter is named `files` so that the name `fs` stays
the module's.

</details>

<details>
<summary>Hint 2</summary>

A walk gives directories too; `metadata` tells them apart and gives
each file's size. Add the sizes up in a dict with
`entry(kind).or_insert(0)`, as chapter 5 counted words.

</details>

When every test passes, compare your answer with
[`manual/solutions/21-standard-library/sizes.sisu`](../solutions/21-standard-library/sizes.sisu).

## Exercise 21.5: turns in the saunas

Open
[`manual/exercises/21-standard-library/turns.sisu`](../exercises/21-standard-library/turns.sisu):

<!-- file: exercises/21-standard-library/turns.sisu -->
```sisu
##! Exercise 21.5, turns in the saunas. Kind: write. Level: 2.
##!
##! Bathers arrive at a spa with several saunas. Each takes the sauna that
##! is free first, waits for it if none is free, and stays a fixed number
##! of minutes. `start_times` works out when each bather goes in. Write
##! its body so that every test passes; a `Heap` from `std.collections`
##! keeps the time each sauna is free again. Run the tests with
##! `sisu test turns.sisu`. The hints are in chapter 21 of the tutorial,
##! under "Exercise 21.5: turns in the saunas".

## When each bather goes in, for bathers who arrive at the minutes of
## `arrivals`, in that order, each staying `stay` minutes in whichever
## of `saunas` saunas is free first.
def start_times(arrivals: list[int], stay: int, saunas: int) -> list[int]:
    return todo("start_times")

test "one sauna, everyone at once":
    assert start_times([0, 0, 0], 10, 1) == [0, 10, 20]

test "two saunas":
    assert start_times([0, 0, 0, 5], 10, 2) == [0, 0, 10, 10]
    assert start_times([0, 3, 4, 30], 15, 2) == [0, 3, 15, 30]

test "a sauna free before the bather comes":
    assert start_times([0, 30], 10, 1) == [0, 30]

test "nobody comes":
    assert start_times([], 10, 3) == []
```

<!-- output: solutions/21-standard-library/turns.console -->
```console
$ sisu test turns.sisu
warning[W0001]: typed hole `start_times` of type `list[int]`
  --> turns.sisu:15:12
   |
15 |     return todo("start_times")
   |            ^^^^^^^^^^^^^^^^^^^

in scope at the typed hole `todo("start_times")`:
  arrivals = [0, 0, 0]
  stay = 10
  saunas = 1
in scope at the typed hole `todo("start_times")`:
  arrivals = [0, 0, 0, 5]
  stay = 10
  saunas = 2
in scope at the typed hole `todo("start_times")`:
  arrivals = [0, 30]
  stay = 10
  saunas = 1
in scope at the typed hole `todo("start_times")`:
  arrivals = []
  stay = 10
  saunas = 3
test "one sauna, everyone at once" ... blocked by todo("start_times")
test "two saunas" ... blocked by todo("start_times")
test "a sauna free before the bather comes" ... blocked by todo("start_times")
test "nobody comes" ... blocked by todo("start_times")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Put one time in the heap for each sauna, the minute it is free again,
which is 0 for all of them at the start.

</details>

<details>
<summary>Hint 2</summary>

For each bather, `pop` gives the sauna that is free first. The bather
goes in when both the bather and that sauna are there, and the sauna
is free again `stay` minutes later.

</details>

When every test passes, compare your answer with
[`manual/solutions/21-standard-library/turns.sisu`](../solutions/21-standard-library/turns.sisu).

## Exercise 21.6: a store named by contents

Open
[`manual/exercises/21-standard-library/store.sisu`](../exercises/21-standard-library/store.sisu):

<!-- file: exercises/21-standard-library/store.sisu -->
```sisu
##! Exercise 21.6, a store named by contents. Kind: write. Level: 3.
##!
##! A backup tool keeps each piece of data in a file named by the data's
##! SHA-256 digest, written in hexadecimal. The same data then gets the
##! same name and is kept once, and a file that changed after it was
##! kept no longer matches its name. Write the bodies of `put` and `get`
##! so that every test passes. `std.hash`, `std.encoding` and the file
##! system's operations are what they need. Run the tests with
##! `sisu test store.sisu`. The hints are in chapter 21 of the tutorial,
##! under "Exercise 21.6: a store named by contents".

## A kept file whose contents no longer match its name.
error StoreError:
    Corrupt(name: String)

## Keeps `data` in the file `objects/NAME`, where NAME is the
## hexadecimal SHA-256 digest of `data`, and gives NAME.
def put(fs: Fs, data: bytes) -> String ! IoError:
    return todo("put")

## The data kept under `name`; `Corrupt` when the file's digest is no
## longer `name`.
def get(fs: Fs, name: str) -> bytes ! {IoError, StoreError}:
    return todo("get")

test "data comes back"(fs: Fs):
    let name = put(fs, b"rye bread").expect("in memory")
    assert len(name) == 64
    assert fs.exists(Path.new("objects/{name}"))
    assert get(fs, name) == Ok(b"rye bread")

test "the same data is kept once"(fs: Fs):
    let first = put(fs, b"blueberry pie").expect("in memory")
    let second = put(fs, b"blueberry pie").expect("in memory")
    let other = put(fs, b"porridge").expect("in memory")
    assert first == second and first != other
    assert fs.list_dir("objects").map((paths) -> len(paths)) == Ok(2)

test "a file that changed is corrupt"(fs: Fs):
    let name = put(fs, b"salt").expect("in memory")
    fs.write_text(Path.new("objects/{name}"), "sugar").expect("in memory")
    assert get(fs, name) == Err(Corrupt(name))

test "a name never kept"(fs: Fs):
    let missing = "0".repeat(64)
    let path = Path.new("objects/{missing}")
    assert get(fs, missing) == Err(NotFound(path))
```

<!-- output: solutions/21-standard-library/store.console -->
```console
$ sisu test store.sisu
warning[W0001]: typed hole `put` of type `String ! IoError`
  --> store.sisu:19:12
   |
19 |     return todo("put")
   |            ^^^^^^^^^^^

warning[W0001]: typed hole `get` of type `bytes ! {IoError, StoreError}`
  --> store.sisu:24:12
   |
24 |     return todo("get")
   |            ^^^^^^^^^^^

in scope at the typed hole `todo("put")`:
  data = b"rye bread"
in scope at the typed hole `todo("put")`:
  data = b"blueberry pie"
in scope at the typed hole `todo("put")`:
  data = b"salt"
in scope at the typed hole `todo("get")`:
  name = "0000000000000000000000000000000000000000000000000000000000000000"
test "data comes back" ... blocked by todo("put")
test "the same data is kept once" ... blocked by todo("put")
test "a file that changed is corrupt" ... blocked by todo("put")
test "a name never kept" ... blocked by todo("get")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

A name is the hexadecimal text of the data's digest, as the section
"Hashing" printed one. Writing the same data under the same name a
second time leaves one file.

</details>

<details>
<summary>Hint 2</summary>

The directory `objects` must exist before a file goes in it. `get`
reads the file and works out its digest again: a name that no longer
matches is `Corrupt`.

</details>

When every test passes, compare your answer with
[`manual/solutions/21-standard-library/store.sisu`](../solutions/21-standard-library/store.sisu).

## What you learned

- `std.fs` walks directories and takes paths apart; `std.io` reads and
  writes streams, a line at a time with `BufReader`.
- `std.time`'s `Ticker` and `Timeline` keep time without drift, in real
  time and on the test clock alike.
- `std.rand` chooses and samples over any `Rand`; `Rand.seeded` repeats
  its numbers.
- `std.env` reads variables, and `std.process` runs programs through
  `Proc`.
- `std.json` encodes values and decodes text into a type that is its
  schema; `std.regex` matches in linear time.
- `std.collections` has `Deque`, `Heap`, `SortedDict`, `SortedSet` and
  `BitSet`; `std.hash` and `std.encoding` digest and encode bytes;
  `std.sort` and `std.search` sort by key and search sorted lists.
- `log` writes leveled records to standard error.
- Each module's source holds its documentation, which `sisu doc --std`
  writes and `sisu test --std` checks.

Chapter 22 puts Part IV together: a small web service with
capabilities, tasks, JSON and logging, tested over a fake network and
under the simulator, with Challenge IV at its end.
