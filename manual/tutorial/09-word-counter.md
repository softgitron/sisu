# 9. Project: a word counter

In this chapter you build a complete program: a word counter that reads
text files, counts how often each word appears, and prints a table, the
most common words first. You use what chapters 1 to 8 taught, and meet
four new things on the way: packages, the first capability, options on
the command line, and tests that run against files held in memory. At
the end, Challenge I asks you to improve the counter. You need about two
hours.

## A new package

A program that is more than one file, or that you want to keep, lives
in a package. `sisu new` makes one:

<!-- output: examples/09-word-counter/new.console -->
```console
$ sisu new wordcount
created the package `wordcount`, with its manifest `sisu.toml` and its root module `src/main.sisu`
```

The package is a directory, `wordcount`, with two files in it. The
first is the manifest, `sisu.toml`, which says what the package is:

<!-- file: examples/09-word-counter/hello.pkg/sisu.toml -->
```toml
[package]
name = "wordcount"
version = "0.1.0"
edition = "2027"

[dependencies]
```

`name` and `version` name the package, `edition` says which edition of
the language its code is written in, and `[dependencies]` will list the
other packages it uses; chapter 17 covers the manifest in full. The
second file is the root module, `src/main.sisu`, which holds the
program's `main`:

<!-- file: examples/09-word-counter/hello.pkg/src/main.sisu -->
```sisu
def main():
    print("Hei, maailma!")
```

Go into the package's directory with `cd wordcount`. There, `sisu
run` without a file name runs the package:

<!-- output: examples/09-word-counter/hello.console -->
```console
$ sisu run
Hei, maailma!
```

Every command that takes a file works on the whole package when you
leave the file out: `sisu check`, `sisu test`, `sisu fmt` and the rest.

## Reading files through a capability

The counter's first job is to read the files named on its command line.
Put this text into a file named `poem.txt` in the package's directory:

<!-- file: examples/09-word-counter/files.pkg/poem.txt -->
```text
The sauna is warm, the lake is cold.
After the sauna, the lake is warm too.
Warm is the sauna. Cold is the lake.
```

Then replace `src/main.sisu` with a program that counts the words of
each file it is given:

<!-- file: examples/09-word-counter/files.pkg/src/main.sisu -->
```sisu
##! wordcount: counts the words in text files.

def main(sys: System) -> ! IoError:
    for arg in sys.env.args():
        let path = Path.new(arg)
        let text = sys.fs.read_to_string(path)?
        let words = text.split_whitespace().count()
        print("{words} words in {path}")
    return Ok(())
```

Arguments for the program follow `--` on the command line, so that
`sisu` does not read them as its own:

<!-- output: examples/09-word-counter/files.console -->
```console
$ sisu run -- poem.txt
24 words in poem.txt
$ sisu run -- poem.txt missing.txt
24 words in poem.txt
error: NotFound(path="missing.txt")
  at src/main.sisu:6:20
```

- `main(sys: System)` receives the program's root capabilities in one
  value. A capability is a value that lets the code that holds it act
  on the world outside the program. `sys.env` gives the arguments and
  the environment, and `sys.fs` gives the file system.
- `sys.env.args()` is the list of arguments after `--`, each a `str`.
  `Path.new(arg)` makes a `Path` of one: a string literal becomes a
  `Path` where one is expected, as chapter 6 showed, but an argument is
  not a literal.
- `sys.fs.read_to_string(path)` gives the file's text as a
  `String ! IoError`, and `?` passes the error on. So `main` returns
  `! IoError`, and a file that is not there ends the program with the
  error and its trace, and with status 1, as chapter 8 showed.
- `split_whitespace().count()` counts the pieces that
  `split_whitespace` gives, without making a list of them.

> **From Python.** Python's `open` works in any function, so any
> function, and any library it calls, can read any file. In Sisu a
> function reads files only through an `Fs` it is given. `print` is one
> of the few things a program does without a capability. Chapter 18
> explains what this buys; in this chapter it lets a test hand the
> program files of its own, as the section "Testing with files in
> memory" shows.

## Counting every word

Next, the counter counts each word and prints a table:

<!-- file: examples/09-word-counter/count.pkg/src/main.sisu -->
```sisu
##! wordcount: how often each word appears in text files.

## How often each word appears in the files at `paths`, in lower case.
def count_words(fs: Fs, paths: list[Path]) -> dict[String, int] ! IoError:
    var counts: dict[String, int] = {}
    for path in paths:
        let text = fs.read_to_string(path)?
        for word in text.split_whitespace():
            counts.entry(word.to_lowercase()).or_insert(0) += 1
    return Ok(counts)

def main(sys: System) -> ! IoError:
    let paths = [Path.new(arg) for arg in sys.env.args()]
    let counts = count_words(sys.fs, paths)?
    let by_count = sorted([(-n, word) for word, n in counts])
    for minus_n, word in by_count:
        print("{-minus_n:>4}  {word}")
    return Ok(())
```

<!-- output: examples/09-word-counter/count.console -->
```console
$ sisu run -- poem.txt
   6  the
   5  is
   2  lake
   2  warm
   1  after
   1  cold
   1  cold.
   1  lake.
   1  sauna
   1  sauna,
   1  sauna.
   1  too.
   1  warm,
```

- `count_words` takes the file system as a parameter, `fs: Fs`, along
  with the paths to read. It counts the words in a dict, as chapter 5
  did, in lower case, so that "The" and "the" are one word.
- `main` hands it `sys.fs`. A function can pass a capability on to the
  functions it calls, and those functions can use it, and nothing else
  can.
- Tuples compare item by item, as a derived order compares fields in
  chapter 7. A list of `(-n, word)` pairs therefore sorts by count,
  the largest first because of the minus sign, and words with the same
  count in alphabetical order.
- `{-minus_n:>4}` turns the count back and aligns it to the right in
  four columns, with the format spec of chapter 6.

The table shows the counter's weak point: "sauna", "sauna," and
"sauna." count as three different words. Challenge I fixes that.

## Options from a struct

A tool usually takes options as well as files. The standard library's
`cli` module builds the options from a struct: you declare what the
command line can say, and it reads the arguments into that struct.
Here the counter gets one option, `--min-count N`, which leaves out
the words that appear fewer than N times:

<!-- file: examples/09-word-counter/options.pkg/src/main.sisu -->
```sisu
##! wordcount: how often each word appears in text files.

import cli

## What the command line can say.
struct Options:
    min_count: int = 1
    paths: list[Path]

## How often each word appears in the files at `paths`, in lower case.
def count_words(fs: Fs, paths: list[Path]) -> dict[String, int] ! IoError:
    var counts: dict[String, int] = {}
    for path in paths:
        let text = fs.read_to_string(path)?
        for word in text.split_whitespace():
            counts.entry(word.to_lowercase()).or_insert(0) += 1
    return Ok(counts)

def main(sys: System) -> ! {IoError, cli.Error}:
    let options = cli.parse[Options](sys.env.args())?
    let counts = count_words(sys.fs, options.paths)?
    let by_count = sorted([(-n, word) for word, n in counts])
    for minus_n, word in by_count:
        if -minus_n >= options.min_count:
            print("{-minus_n:>4}  {word}")
    return Ok(())
```

<!-- output: examples/09-word-counter/options.console -->
```console
$ sisu run -- --min-count 2 poem.txt
   6  the
   5  is
   2  lake
   2  warm
$ sisu run -- --min-count many poem.txt
error: InvalidValue(name="--min-count", value="many")
  at src/main.sisu:20:19
```

- `import cli` makes the module `cli` usable in the file, under that
  name; chapter 17 covers imports. It is the standard library's
  `std.x.cli`.
- `cli.parse[Options](sys.env.args())` reads the arguments into an
  `Options`. The type in square brackets says what to read them into;
  chapter 15 shows how a function takes a type this way.
- Each field with a default is an option that may be left out:
  `min_count` is `--min-count`, with the underscore written as a dash.
  The field of type `list[Path]` collects the arguments that are not
  options, already as paths.
- Arguments that do not fit are a `cli.Error`, such as
  `InvalidValue(name="--min-count", value="many")`, so `main` returns
  `! {IoError, cli.Error}`, the union of chapter 8.

## Testing with files in memory

`count_words` reads only through the `Fs` it is given, so a test can
give it a file system that exists only in memory. The finished
program has two tests at its end:

<!-- file: examples/09-word-counter/tested.pkg/src/main.sisu -->
```sisu
##! wordcount: how often each word appears in text files.

import cli

## What the command line can say.
struct Options:
    min_count: int = 1
    paths: list[Path]

## How often each word appears in the files at `paths`, in lower case.
def count_words(fs: Fs, paths: list[Path]) -> dict[String, int] ! IoError:
    var counts: dict[String, int] = {}
    for path in paths:
        let text = fs.read_to_string(path)?
        for word in text.split_whitespace():
            counts.entry(word.to_lowercase()).or_insert(0) += 1
    return Ok(counts)

def main(sys: System) -> ! {IoError, cli.Error}:
    let options = cli.parse[Options](sys.env.args())?
    let counts = count_words(sys.fs, options.paths)?
    let by_count = sorted([(-n, word) for word, n in counts])
    for minus_n, word in by_count:
        if -minus_n >= options.min_count:
            print("{-minus_n:>4}  {word}")
    return Ok(())

test "a word counts in any case":
    let fs = Fs.in_memory({"a.txt": "Kissa kissa KISSA koira"})
    let counts = count_words(fs, ["a.txt"]).expect("a.txt is there")
    assert counts == {"kissa": 3, "koira": 1}

test "every file counts":
    let fs = Fs.in_memory({"a.txt": "kala", "b.txt": "kala kissa"})
    let counts = count_words(fs, ["a.txt", "b.txt"]).expect("both are there")
    assert counts == {"kala": 2, "kissa": 1}
```

`sisu test` in the package's directory runs every test of the package:

<!-- output: examples/09-word-counter/tested.console -->
```console
$ sisu test
test "a word counts in any case" ... ok
test "every file counts" ... ok
2 passed, 0 failed, 0 blocked
```

- `Fs.in_memory({"a.txt": "..."})` makes a file system that holds the
  files of the dict, by path. The tests never touch the disk, so they
  give the same result on every machine and can run side by side.
- A list literal becomes a `list[Path]` where one is expected, so
  `["a.txt", "b.txt"]` can be passed to `count_words` as it is.
- `.expect("a.txt is there")` takes the counts out of the result. A
  missing file here would be a mistake in the test, not an error the
  test is about, so a panic, which fails the test, is the right
  response.

## Building the program

`sisu build` checks the whole package and prepares it to run:

<!-- output: examples/09-word-counter/tested.build -->
```text
built the package `wordcount` 0.1.0 (modules: 1)
```

It also writes `sisu.lock`, the lockfile, which records the exact
packages a build used; chapter 17 explains it. `sisu build --native`
compiles the program into an executable, `build/debug/wordcount`, that
runs without `sisu`; chapter 22 covers native builds.

## Exercise 9.1: reading a file

Open
[`manual/exercises/09-word-counter/lines.sisu`](../exercises/09-word-counter/lines.sisu):

<!-- file: exercises/09-word-counter/lines.sisu -->
```sisu
##! Exercise 9.1, reading a file. Kind: fix. Level: 1.
##!
##! `line_count` counts the lines of a file, written the way a Python
##! programmer would write it. The file does not compile. Run
##! `sisu test lines.sisu`, read the diagnostics, and fix `line_count`
##! so that every test passes. The hints are in chapter 9 of the
##! tutorial, under "Exercise 9.1: reading a file".

## The number of lines in the file at `path`, read from `fs`.
def line_count(path: Path) -> int ! IoError:
    let text = open(path).read()
    return Ok(text.lines().count())

test "a file of three lines":
    let fs = Fs.in_memory({"notes.txt": "one\ntwo\nthree\n"})
    assert line_count(fs, "notes.txt") == Ok(3)

test "an empty file":
    let fs = Fs.in_memory({"empty.txt": ""})
    assert line_count(fs, "empty.txt") == Ok(0)

test "a file that is not there":
    let fs = Fs.in_memory({})
    assert line_count(fs, "missing.txt") == Err(NotFound("missing.txt"))
```

<!-- output: solutions/09-word-counter/lines.console -->
```console
$ sisu test lines.sisu
error[E0200]: cannot find `open` in this scope
  --> lines.sisu:11:16
   |
11 |     let text = open(path).read()
   |                ^^^^
  = note: a program reaches files through the capability `Fs`, which `def main(sys: System)` holds as `sys.fs`: `sys.fs.read_to_string("data.txt")`

error[E0305]: wrong number of arguments to `line_count`: expected 1, found 2
  --> lines.sisu:16:12
   |
16 |     assert line_count(fs, "notes.txt") == Ok(3)
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0300]: expected `Path`, found `Fs`
  --> lines.sisu:16:23
   |
16 |     assert line_count(fs, "notes.txt") == Ok(3)
   |                       ^^

error[E0305]: wrong number of arguments to `line_count`: expected 1, found 2
  --> lines.sisu:20:12
   |
20 |     assert line_count(fs, "empty.txt") == Ok(0)
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0300]: expected `Path`, found `Fs`
  --> lines.sisu:20:23
   |
20 |     assert line_count(fs, "empty.txt") == Ok(0)
   |                       ^^

error[E0305]: wrong number of arguments to `line_count`: expected 1, found 2
  --> lines.sisu:24:12
   |
24 |     assert line_count(fs, "missing.txt") == Err(NotFound("missing.txt"))
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0300]: expected `Path`, found `Fs`
  --> lines.sisu:24:23
   |
24 |     assert line_count(fs, "missing.txt") == Err(NotFound("missing.txt"))
   |                       ^^

error[D0006]: the program was not run: errors found (7)
```

The first diagnostic is the one to read first, and its note says how a
program reaches files. The others come from the tests, which call
`line_count` with a file system.

<details>
<summary>Hint 1</summary>

Sisu has no function that opens a file from anywhere. The doc comment
of `line_count` says where the file is read from.

</details>

<details>
<summary>Hint 2</summary>

The tests pass a file system as the first argument. The section
"Reading files through a capability" shows how a function reads a file
from the file system it is given, and what the result's error is.

</details>

When every test passes, compare your answer with
[`manual/solutions/09-word-counter/lines.sisu`](../solutions/09-word-counter/lines.sisu).

## Exercise 9.2: finding lines

Open
[`manual/exercises/09-word-counter/find_lines.sisu`](../exercises/09-word-counter/find_lines.sisu):

<!-- file: exercises/09-word-counter/find_lines.sisu -->
```sisu
##! Exercise 9.2, finding lines. Kind: complete. Level: 2.
##!
##! `find_lines` lists the lines of a file that hold a word, each with
##! its number, as the command `grep -n` does. Replace the two holes so
##! that every test passes. Run the tests with
##! `sisu test find_lines.sisu`. The hints are in chapter 9 of the
##! tutorial, under "Exercise 9.2: finding lines".

## Each line of the file at `path` that holds `word`, as "N: line",
## where N counts the lines from 1.
def find_lines(fs: Fs, path: Path, word: str) -> list[String] ! IoError:
    let text: String = todo("the text of the file")
    var found: list[String] = []
    for i, line in enumerate(text.lines()):
        if todo("whether the line holds the word"):
            found.push("{i + 1}: {line}")
    return Ok(found)

## A file system that holds one short poem, for the tests.
def poem() -> Fs:
    let text = "the sauna is warm\nthe lake is cold\nwarm is the sauna\n"
    return Fs.in_memory({"poem.txt": text})

test "lines that hold the word":
    let found = find_lines(poem(), "poem.txt", "warm")
    assert found == Ok(["1: the sauna is warm", "3: warm is the sauna"])
    assert find_lines(poem(), "poem.txt", "lake") == Ok(["2: the lake is cold"])

test "no line holds it":
    assert find_lines(poem(), "poem.txt", "snow") == Ok([])

test "a file that is not there":
    let found = find_lines(poem(), "song.txt", "warm")
    assert found == Err(NotFound("song.txt"))
```

<!-- output: solutions/09-word-counter/find_lines.console -->
```console
$ sisu test find_lines.sisu
warning[W0001]: typed hole `the text of the file` of type `String`
  --> find_lines.sisu:12:24
   |
12 |     let text: String = todo("the text of the file")
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `whether the line holds the word` of type `bool`
  --> find_lines.sisu:15:12
   |
15 |         if todo("whether the line holds the word"):
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "lines that hold the word" ... blocked by todo("the text of the file")
test "no line holds it" ... blocked by todo("the text of the file")
test "a file that is not there" ... blocked by todo("the text of the file")

test "lines that hold the word" ... blocked by todo("the text of the file")
what the test printed:
  in scope at the typed hole `todo("the text of the file")`:
    path = "poem.txt"
    word = "warm"

test "no line holds it" ... blocked by todo("the text of the file")
what the test printed:
  in scope at the typed hole `todo("the text of the file")`:
    path = "poem.txt"
    word = "snow"

test "a file that is not there" ... blocked by todo("the text of the file")
what the test printed:
  in scope at the typed hole `todo("the text of the file")`:
    path = "song.txt"
    word = "warm"

0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

The first hole is the text of the file at `path`, read from `fs`. A
file that is not there must reach the caller as an error.

</details>

<details>
<summary>Hint 2</summary>

The second hole is a `bool`. Chapter 5's section "Asking with in"
shows how to ask whether one text holds another.

</details>

When every test passes, compare your answer with
[`manual/solutions/09-word-counter/find_lines.sisu`](../solutions/09-word-counter/find_lines.sisu).

## Challenge I: a better word counter

The challenge improves the counter in three parts. It is one file,
with a `suite` of tests for each part: a `suite` groups tests under a
name, which `sisu test` prints before each test's own. Part 3 has a
transcript as well, `tests/counter.transcript` beside the file: a list
of commands, each a `$` line, and what each prints. `sisu test` runs
each command as a run of `main`, with a file system in memory that
holds the files of `tests/counter/`, and compares what it prints;
chapter 19 covers transcripts. Open
[`manual/exercises/09-word-counter/counter.sisu`](../exercises/09-word-counter/counter.sisu):

<!-- file: exercises/09-word-counter/counter.sisu -->
```sisu
##! Exercise 9.3, Challenge I: a better word counter. Kind: challenge. Level: 3.
##!
##! Chapter 9's counter counts "sauna," and "sauna" as two words, counts
##! the shortest words too, and prints every word. This challenge
##! improves it in three parts, with a suite of tests for each. Solve
##! them in order: each part's tests fail or are blocked until it is
##! done. Run the tests with `sisu test counter.sisu`.
##!
##! Part 1. `count_words` counts the words of `text` that have at least
##! `min_length` characters, in lower case. A word is what
##! `split_whitespace` gives.
##!
##! Part 2. Punctuation is not part of a word: a word is a run of
##! letters and digits, so "Sauna," and "sauna." both count as "sauna",
##! and "well-known" is two words. Change `count_words` to count such
##! words; the tests of part 1 still pass.
##!
##! Part 3. `top_words` gives the `n` most common words with their
##! counts, the most common first, and words with the same count in
##! alphabetical order. `main` takes the options `--top N` (10 if it is
##! not given) and `--min-length N` (1 if it is not given) and the paths
##! of the files, and prints the top words of all the files together,
##! in the layout of chapter 9's counter. The transcript
##! `tests/counter.transcript` runs it on `tests/counter/poem.txt`.
##!
##! The hints are in chapter 9 of the tutorial, under "Challenge I: a
##! better word counter".

import cli

## What the command line can say.
struct Options:
    top: int = 10
    min_length: int = 1
    paths: list[Path]

## How often each word of `text` with at least `min_length` characters
## appears in it, in lower case.
def count_words(text: str, min_length: int) -> dict[String, int]:
    return todo("count_words")

## The `n` most common words of `counts`, with their counts.
def top_words(counts: dict[String, int], n: int) -> list[(String, int)]:
    return todo("top_words")

def main(sys: System) -> ! {IoError, cli.Error}:
    return todo("main")

suite "part 1, a minimum length":
    test "short words are left out":
        assert count_words("a bb ccc bb", 2) == {"bb": 2, "ccc": 1}
        assert count_words("I saw a cat", 3) == {"saw": 1, "cat": 1}

    test "every word counts in lower case":
        assert count_words("Hei hei HEI", 1) == {"hei": 3}

suite "part 2, punctuation":
    test "punctuation around a word":
        let counts = count_words("Sauna, sauna. (SAUNA!)", 1)
        assert counts == {"sauna": 3}

    test "punctuation inside a word":
        let counts = count_words("a well-known 2-minute walk", 2)
        assert counts == {"well": 1, "known": 1, "minute": 1, "walk": 1}

suite "part 3, the top words":
    test "the most common first":
        let counts: dict[String, int] = {"kissa": 1, "koira": 3, "kala": 2}
        assert top_words(counts, 2) == [("koira", 3), ("kala", 2)]

    test "equal counts in alphabetical order":
        let counts: dict[String, int] = {"tuli": 1, "meni": 1, "kevät": 1}
        let top = top_words(counts, 5)
        assert top == [("kevät", 1), ("meni", 1), ("tuli", 1)]
```

The transcript, `tests/counter.transcript`, runs the program three
times on the poem of this chapter:

<!-- file: exercises/09-word-counter/counter.files/tests/counter.transcript -->
```text
$ counter --top 3 poem.txt
   6  the
   5  is
   3  lake
$ counter --top 2 --min-length 4 poem.txt
   3  lake
   3  sauna
$ counter --min-length 5 poem.txt
   3  sauna
   1  after
```

Before you change anything, every test is blocked and the transcript
fails, since `main` is still a hole:

<!-- output: solutions/09-word-counter/counter.console -->
```console
$ sisu test counter.sisu
warning[W0001]: typed hole `count_words` of type `dict[String, int]`
  --> counter.sisu:40:12
   |
40 |     return todo("count_words")
   |            ^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `top_words` of type `list[(String, int)]`
  --> counter.sisu:44:12
   |
44 |     return todo("top_words")
   |            ^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `main` of type `! {IoError, cli.Error}`
  --> counter.sisu:47:12
   |
47 |     return todo("main")
   |            ^^^^^^^^^^^^

test "part 1, a minimum length/short words are left out" ... blocked by todo("count_words")
test "part 1, a minimum length/every word counts in lower case" ... blocked by todo("count_words")
test "part 2, punctuation/punctuation around a word" ... blocked by todo("count_words")
test "part 2, punctuation/punctuation inside a word" ... blocked by todo("count_words")
test "part 3, the top words/the most common first" ... blocked by todo("top_words")
test "part 3, the top words/equal counts in alphabetical order" ... blocked by todo("top_words")
transcript "tests/counter.transcript" ... FAILED

test "part 1, a minimum length/short words are left out" ... blocked by todo("count_words")
what the test printed:
  in scope at the typed hole `todo("count_words")`:
    text = "a bb ccc bb"
    min_length = 2

test "part 1, a minimum length/every word counts in lower case" ... blocked by todo("count_words")
what the test printed:
  in scope at the typed hole `todo("count_words")`:
    text = "Hei hei HEI"
    min_length = 1

test "part 2, punctuation/punctuation around a word" ... blocked by todo("count_words")
what the test printed:
  in scope at the typed hole `todo("count_words")`:
    text = "Sauna, sauna. (SAUNA!)"
    min_length = 1

test "part 2, punctuation/punctuation inside a word" ... blocked by todo("count_words")
what the test printed:
  in scope at the typed hole `todo("count_words")`:
    text = "a well-known 2-minute walk"
    min_length = 2

test "part 3, the top words/the most common first" ... blocked by todo("top_words")
what the test printed:
  in scope at the typed hole `todo("top_words")`:
    counts = {"kissa": 1, "koira": 3, "kala": 2}
    n = 2

test "part 3, the top words/equal counts in alphabetical order" ... blocked by todo("top_words")
what the test printed:
  in scope at the typed hole `todo("top_words")`:
    counts = {"tuli": 1, "meni": 1, "kevät": 1}
    n = 5

transcript "tests/counter.transcript" ... FAILED
what the program printed differs from the transcript (`sisu test --update` accepts it):
  $ counter --top 3 poem.txt
-    6  the
-    5  is
-    3  lake
+ panic: reached the typed hole `todo("main")` (exit 101)
  $ counter --top 2 --min-length 4 poem.txt
-    3  lake
-    3  sauna
+ panic: reached the typed hole `todo("main")` (exit 101)
  $ counter --min-length 5 poem.txt
-    3  sauna
-    1  after
+ panic: reached the typed hole `todo("main")` (exit 101)

0 passed, 1 failed, 6 blocked
```

Solve the parts in order. When part 1 passes, the tests of part 2
fail rather than wait, because part 2 changes what a word is.

<details>
<summary>Hint for part 1</summary>

Start from `count_words` in this chapter's counter, which reads files;
this one counts the words of one text, and skips the short ones. `len`
counts the characters of a word.

</details>

<details>
<summary>Hint for part 2</summary>

If part 1 split the text and counted in one loop, give the splitting a
function of its own, which returns the words of a text. Go through the
characters with `chars()`, and collect each run of letters and digits
in a `String`; `is_alphabetic` and `is_digit` tell the characters
apart. Only that function changes.

</details>

<details>
<summary>Hint for part 3</summary>

`top_words` sorts as the section "Counting every word" does, then
keeps the first `n`; a slice can do that. `main` is the counter of the
section "Options from a struct", with a second option, and it adds the
counts of each file into one dict before it ranks them.

</details>

When every test passes, and the transcript too, compare your answer
with
[`manual/solutions/09-word-counter/counter.sisu`](../solutions/09-word-counter/counter.sisu).

## What you learned

- `sisu new` makes a package: a manifest, `sisu.toml`, and a root
  module, `src/main.sisu`. Without a file name, `sisu run`, `sisu test`
  and `sisu build` work on the package of the current directory, and
  `sisu run -- ARGS` passes arguments to the program.
- `main(sys: System)` receives the root capabilities: `sys.fs` reads
  files, and `sys.env.args()` gives the arguments. A function acts on
  the world only through the capabilities it is given.
- `cli.parse[Options]` reads the command line into a struct: a field
  with a default is an option, and a list field collects the paths.
- A function that takes an `Fs` can be tested with `Fs.in_memory`,
  without touching the disk.

That is the end of Part I. You can write, test and build a useful
command-line tool. Part II, which starts with chapter 10, is about
ownership: who owns each value, when a value moves and when it is
copied, and how to lend a value without giving it away.
