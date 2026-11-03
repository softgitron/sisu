# 6. Text

In this chapter you work with text: the two text types, `str` and
`String`, and why Sisu has both; interpolation with format specs; the
methods that trim, split, search and replace; characters and bytes, and
why text has no integer index; the other kinds of string literal; and
building text and reading numbers out of it. You need about an hour.

## Two types of text

Chapter 1 met both text types. A literal without braces, such as
`"hei"`, is a `str`. A literal with braces builds new text when its line
runs, and that text is a `String`. Most functions that read text take a
`str`, and they accept a `String` as well:

<!-- file: examples/06-text/two_types.sisu -->
```sisu
# A `str` parameter accepts a literal and a `String` alike.
def shout(text: str) -> String:
    return "{text.to_uppercase()}!"

def main():
    let literal = "hei"
    let built = "hei, {"Aino"}"
    print(shout(literal))
    print(shout(built))
```

<!-- output: examples/06-text/two_types.out -->
```text
HEI!
HEI, AINO!
```

- A `str` is a view of text that lives somewhere else: in the program
  itself for a literal, or in a `String`. Passing a `str` copies only
  the view, never the text.
- A `String` owns its text and can grow. A function that makes new
  text, such as `shout` or `to_uppercase`, returns a `String`.
- A `String` is accepted wherever a `str` is expected, as the second
  call to `shout` shows. The other way needs a call: `to_string()` makes
  a `String` from a `str`.

So a function's signature says what it does with text: a `str`
parameter only reads it, and a `String` result is text the caller now
owns. Chapter 12 explains views in full.

> **From Python.** Python has one text type, `str`, and every string is
> an object of its own. Sisu splits the job: `str` for reading text
> wherever it lives, `String` for owning it.

## Format specs

After an interpolated value, a colon starts a format spec, which says
how to lay the value out:

<!-- file: examples/06-text/price_list.sisu -->
```sisu
# A format spec after a colon sets width, alignment and digits.
def main():
    let items = ["coffee", "cloudberries", "rye bread"]
    let prices = [3.5, 12.0, 2.25]
    for item, price in zip(items, prices):
        print("{item:<14}{price:>8.2f} €")
    print("{"total":<14}{sum(prices):>8.2f} €")
```

<!-- output: examples/06-text/price_list.out -->
```text
coffee            3.50 €
cloudberries     12.00 €
rye bread         2.25 €
total            17.75 €
```

- `{item:<14}` pads the item to a width of 14 characters, aligned to
  the left. `>` aligns to the right and `^` centres.
- `{price:>8.2f}` aligns the price to the right in 8 characters, with 2
  digits after the decimal point.

Integers have specs of their own:

<!-- file: examples/06-text/number_specs.sisu -->
```sisu
#!/usr/bin/env sisu
# Format specs for integers.
let n = 42
"[{n:5}] [{n:<5}] [{n:^5}] [{n:05}]"
"{n:x} {n:#x} {n:b} {n:o}"
"{1234567:,}"
```

<!-- output: examples/06-text/number_specs.ws -->
```text
#!/usr/bin/env sisu
# Format specs for integers.
let n = 42  # => 42
"[{n:5}] [{n:<5}] [{n:^5}] [{n:05}]"  # => "[   42] [42   ] [ 42  ] [00042]"
"{n:x} {n:#x} {n:b} {n:o}"  # => "2a 0x2a 101010 52"
"{1234567:,}"  # => "1,234,567"
```

A number is aligned to the right unless the spec says otherwise, `05`
pads with zeros, `x`, `b` and `o` write hexadecimal, binary and octal,
`#x` adds the `0x` prefix, and `,` separates the thousands. The specs
are Python's format mini-language.

`{x:?}` is a spec of its own. It shows a value the way it would be
written in code, which is called its `Debug` form:

<!-- file: examples/06-text/debug_specs.sisu -->
```sisu
# `{x:?}` shows a value the way it is written in code.
def main():
    let name = "Aino"
    let initial = 'A'
    print("{name} {initial}")
    print("{name:?} {initial:?}")
    print("{name=} {["Aino", "Eero"]}")
```

<!-- output: examples/06-text/debug_specs.out -->
```text
Aino A
"Aino" 'A'
name="Aino" ["Aino", "Eero"]
```

The `Debug` form puts quotes around text, so you can see where a string
starts and ends. `{name=}` uses it after the `=`, and a list shows its
items in it, so the strings in a list have their quotes.

## Methods of text

The methods that trim, test, search and replace text are Python's, with
Sisu's names:

<!-- file: examples/06-text/methods.sisu -->
```sisu
#!/usr/bin/env sisu
# The common methods of text.
let line = "  Hei, Maailma!  "
line.trim()
line.trim().to_lowercase()
line.trim().starts_with("Hei")
line.trim().replace("Maailma", "Aino")
let at = line.find("M")
let missing = line.find("x")
"v1.2.0".strip_prefix("v")
[part for part in "red,green,,blue".split(",")]
[word for word in "one two\tthree".split_whitespace()]
[row for row in "first\nsecond".lines()]
```

<!-- output: examples/06-text/methods.ws -->
```text
#!/usr/bin/env sisu
# The common methods of text.
let line = "  Hei, Maailma!  "  # => "  Hei, Maailma!  "
line.trim()  # => "Hei, Maailma!"
line.trim().to_lowercase()  # => "hei, maailma!"
line.trim().starts_with("Hei")  # => True
line.trim().replace("Maailma", "Aino")  # => "Hei, Aino!"
let at = line.find("M")  # => Some(7)
let missing = line.find("x")  # => None
"v1.2.0".strip_prefix("v")  # => "1.2.0"
[part for part in "red,green,,blue".split(",")]  # => ["red", "green", "", "blue"]
[word for word in "one two\tthree".split_whitespace()]  # => ["one", "two", "three"]
[row for row in "first\nsecond".lines()]  # => ["first", "second"]
```

- `trim` removes spaces at both ends; `trim_start` and `trim_end`
  remove them at one end.
- `to_lowercase` and `to_uppercase` change the case of every letter, in
  any alphabet.
- `starts_with` and `ends_with` test the start and the end, and
  `strip_prefix` and `strip_suffix` remove them if they are there.
- `replace` replaces every occurrence of a piece of text.
- `find` gives the position, counted in characters, where a piece of
  text starts, as an optional value: `None` when it is not there.
  Chapter 8 shows how to use one; a worksheet shows an optional value
  only in a `let`.
- `split(",")` cuts at every comma, `split_whitespace` at every run of
  spaces, tabs and line breaks, and `lines` at every line break. Each
  gives its pieces one at a time to a `for` loop or a comprehension.

Each method leaves the text it is called on as it was. The ones that
change text return a new `String`, and the ones that cut it return
views of the original.

## Characters and bytes

Sisu stores text as UTF-8, in which a character takes one to four
bytes. `chars()` goes through the characters and `bytes()` through the
bytes:

<!-- file: examples/06-text/characters.sisu -->
```sisu
# `chars()` goes through the characters, and `bytes()` the bytes.
def main():
    let word = "Äiti"
    print("{len(word)} characters, {word.byte_len()} bytes")
    for c in word.chars():
        print("{c}: code {c.code()}, alphabetic {c.is_alphabetic()}")
    var total = 0
    for b in word.bytes():
        total += b.to_int()
    print("the bytes add up to {total}")
```

<!-- output: examples/06-text/characters.out -->
```text
4 characters, 5 bytes
Ä: code 196, alphabetic True
i: code 105, alphabetic True
t: code 116, alphabetic True
i: code 105, alphabetic True
the bytes add up to 653
```

`len` counts characters, and `byte_len` counts bytes: `Ä` takes two
bytes, so "Äiti" has four characters and five bytes. A `char` has
methods of its own, such as `code`, its number in Unicode,
`is_alphabetic`, `is_digit` and `is_whitespace`. A byte is a `u8`, and
`to_int()` widens it to an `int`.

## Why text has no index

Python's `word[0]` is the first character. In Sisu it does not compile:

<!-- file: examples/06-text/first_letter.sisu -->
```sisu
# Text cannot be indexed by position.
def main():
    let word = "Äiti"
    print("the first letter is {word[0]}")
```

<!-- output: examples/06-text/first_letter.console -->
```console
$ sisu run first_letter.sisu
error[E0345]: `str` cannot be indexed by position; use `.chars()`, `.bytes()` or `.byte_slice(a, b)`
  --> first_letter.sisu:4:33
  |
4 |     print("the first letter is {word[0]}")
  |                                 ^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E0345 (`type.str-not-indexable`) lists the ways to reach the parts of a
text. An index by characters would have to count them from the start,
because they differ in size, so `word[i]` would be slow on long text
while looking fast. An index by bytes would be fast, but it could land
in the middle of a character. Sisu offers each way under its own name:

- `chars()` for the characters, in order.
- `bytes()` for the bytes.
- `byte_slice(start, end)` for the text between two byte positions,
  such as positions that `byte_find` gave, or the fixed places of a
  date:

<!-- file: examples/06-text/dates.sisu -->
```sisu
# `byte_slice` cuts text at byte positions.
def main():
    let date = "2026-11-03"
    let year = date.byte_slice(0, 4)
    let month = date.byte_slice(5, 7)
    let day = date.byte_slice(8, 10)
    print("day {day}, month {month}, year {year}")
```

<!-- output: examples/06-text/dates.out -->
```text
day 03, month 11, year 2026
```

`byte_slice` checks that both positions fall between characters. A
position inside a character stops the program:

<!-- file: examples/06-text/half_letter.sisu -->
```sisu
# `byte_slice` cannot cut a character in two.
def main():
    let word = "Äiti"
    print(word.byte_slice(0, 1))
```

<!-- output: examples/06-text/half_letter.console -->
```console
$ sisu run half_letter.sisu
panic: the byte index 1 is inside a character of the text
  at main (half_letter.sisu:4:11)
```

Iterating over the user-perceived characters, the graphemes, where a
letter and its accent count as one, is designed in
[03](../../proposal/03-types-and-generics.md#strings-and-text) and is
not in the compiler yet.

> **From Python.** `for c in word` goes through the characters in
> Python. In Sisu, write `for c in word.chars()`: a `for` loop does not
> go through text by itself, because it could mean characters or bytes.

## Other string literals

Three more kinds of literal cover text that is awkward to write in
plain quotes:

<!-- file: examples/06-text/literal_kinds.sisu -->
```sisu
# Raw, multi-line and byte literals.
def main():
    let folder = r"C:\new\temp"
    print(folder)
    let poem = """Tuuli tuli,
ja tuuli meni."""
    print(poem)
    let magic = b"PNG"
    print("{magic:?} has {len(magic)} bytes")
```

<!-- output: examples/06-text/literal_kinds.out -->
```text
C:\new\temp
Tuuli tuli,
ja tuuli meni.
b"PNG" has 3 bytes
```

- `r"..."` is a raw literal: a backslash is a backslash, and nothing
  between braces is interpolated.
- `"""..."""` holds several lines, with the line breaks in it.
- `b"..."` is a `bytes` value, a sequence of bytes rather than text, for
  data such as a file's header.

A plain string literal can also become a value of another type, where
the code expects that type. A file path is the common case:

<!-- file: examples/06-text/paths.sisu -->
```sisu
# A string literal becomes a `Path` where a `Path` is expected.
def describe(file: Path) -> String:
    return "the file {file}"

def main():
    let config: Path = "settings/app.toml"
    print(describe(config))
    print(describe("notes.txt"))
```

<!-- output: examples/06-text/paths.out -->
```text
the file settings/app.toml
the file notes.txt
```

`"settings/app.toml"` becomes a `Path` because the annotation asks for
one, and `"notes.txt"` because `describe` takes one. This works for
literals only, not for a `str` held in a binding. Chapter 9 reads files
by their paths. The same for web addresses, a literal that becomes a
`Url`, is designed in [02](../../proposal/02-syntax.md#literals) and is
not in the compiler yet.

## Building text

A `String` grows. `String()` makes an empty one, `push` adds text,
`push_char` adds a character, and `+=` adds text too. For text built
from many pieces, `Builder` from the standard library's `std.string`
keeps the pieces and joins them once at the end:

<!-- file: examples/06-text/building.sisu -->
```sisu
# A `String` grows; `Builder` joins many pieces at once.
from std.string import Builder

def main():
    var greeting = String()
    greeting.push("Hei")
    greeting.push_char(',')
    greeting += " maailma"
    print(greeting)
    var report = Builder.new()
    for n in range(1, 4):
        if n > 1:
            report.push(", ")
        report.push("step {n}")
    print(report.build())
```

<!-- output: examples/06-text/building.out -->
```text
Hei, maailma
step 1, step 2, step 3
```

`from std.string import Builder` brings `Builder` into the file;
chapter 9 meets imports properly. Adding to a `String` one piece at a
time can copy the text each time it outgrows its memory, while a
`Builder` copies each byte once. For a few pieces either is fine, and
interpolation is usually the clearest of all.

## Numbers out of text

`parse_int` and `parse_float` read a number out of text. Text that does
not hold a number is common, so the result is an optional value:

<!-- file: examples/06-text/parsing.sisu -->
```sisu
#!/usr/bin/env sisu
# Numbers out of text: the result is optional.
let answer = "42".parse_int()
let below_zero = "-7".parse_int()
let not_a_number = "4x".parse_int()
let ratio = "2.5".parse_float()
let padded = " 42 ".trim().parse_int()
```

<!-- output: examples/06-text/parsing.ws -->
```text
#!/usr/bin/env sisu
# Numbers out of text: the result is optional.
let answer = "42".parse_int()  # => Some(42)
let below_zero = "-7".parse_int()  # => Some(-7)
let not_a_number = "4x".parse_int()  # => None
let ratio = "2.5".parse_float()  # => Some(2.5)
let padded = " 42 ".trim().parse_int()  # => Some(42)
```

`Some(42)` holds the number, and `None` says that the text was not one.
The text must be the number and nothing else, so text from a user or a
file usually needs `trim` first. Chapter 8 shows how to take the number
out of `Some`, and what to do with `None`.

## Exercise 6.1: predict the text

Open
[`manual/exercises/06-text/predict_text.sisu`](../exercises/06-text/predict_text.sisu):

<!-- file: exercises/06-text/predict_text.sisu -->
```sisu
##! Exercise 6.1, predict the text. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_text.sisu`. The hints are in chapter 6 of the
##! tutorial, under "Exercise 6.1: predict the text".

const LETTERS: int = todo("the length of a word")
const BYTES: int = todo("the bytes of a word")
const PIECES: int = todo("the pieces of a split")
const PADDED: str = todo("a number with a format spec")
const DISTANCE: int = todo("the distance between two letters")

test "len of a Finnish word":
    assert len("Äiti") == LETTERS

test "the bytes of the same word":
    assert "Äiti".byte_len() == BYTES

test "splitting at every comma":
    assert len([piece for piece in "a,b,,c".split(",")]) == PIECES

test "a width with zeros":
    assert "{7:03}" == PADDED

test "codes of characters":
    assert 'e'.code() - 'a'.code() == DISTANCE
```

<!-- output: solutions/06-text/predict_text.console -->
```console
$ sisu test predict_text.sisu
warning[W0001]: typed hole `the length of a word` of type `int`
  --> predict_text.sisu:8:22
  |
8 | const LETTERS: int = todo("the length of a word")
  |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the bytes of a word` of type `int`
  --> predict_text.sisu:9:20
  |
9 | const BYTES: int = todo("the bytes of a word")
  |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the pieces of a split` of type `int`
  --> predict_text.sisu:10:21
   |
10 | const PIECES: int = todo("the pieces of a split")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `a number with a format spec` of type `str`
  --> predict_text.sisu:11:21
   |
11 | const PADDED: str = todo("a number with a format spec")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the distance between two letters` of type `int`
  --> predict_text.sisu:12:23
   |
12 | const DISTANCE: int = todo("the distance between two letters")
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "len of a Finnish word" ... blocked by todo("the length of a word")
test "the bytes of the same word" ... blocked by todo("the bytes of a word")
test "splitting at every comma" ... blocked by todo("the pieces of a split")
test "a width with zeros" ... blocked by todo("a number with a format spec")
test "codes of characters" ... blocked by todo("the distance between two letters")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

Two of the holes are about what `len` and `byte_len` count. The
section "Characters and bytes" has a word like this one.

</details>

<details>
<summary>Hint 2</summary>

Between two commas that stand next to each other there is an empty
piece. Letters have consecutive codes, so the distance between two
codes is the distance in the alphabet.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/06-text/predict_text.sisu`](../solutions/06-text/predict_text.sisu).

## Exercise 6.2: counting vowels

Open
[`manual/exercises/06-text/vowels.sisu`](../exercises/06-text/vowels.sisu):

<!-- file: exercises/06-text/vowels.sisu -->
```sisu
##! Exercise 6.2, counting vowels. Kind: fix. Level: 1.
##!
##! `vowels` counts the vowels of a Finnish word, written the way a
##! Python programmer would. It does not compile. Run
##! `sisu test vowels.sisu`, read each diagnostic, and fix the function
##! until every test passes; a second diagnostic appears once the first
##! is fixed. The hints are in chapter 6 of the tutorial, under
##! "Exercise 6.2: counting vowels".

## The number of vowels in `word`, in either case. Finnish has eight:
## a, e, i, o, u, y, ä and ö.
def vowels(word: str) -> int:
    var count = 0
    for letter in word.to_lowercase():
        if letter in "aeiouyäö":
            count += 1
    return count

test "plain vowels":
    assert vowels("kissa") == 2
    assert vowels("Aino") == 3

test "vowels with dots and capitals":
    assert vowels("ÄITI") == 3
    assert vowels("yö") == 2

test "no vowels":
    assert vowels("tsk") == 0
```

<!-- output: solutions/06-text/vowels.console -->
```console
$ sisu test vowels.sisu
error[E0325]: a `for` loop cannot iterate over `String`
  --> vowels.sisu:14:19
   |
14 |     for letter in word.to_lowercase():
   |                   ^^^^^^^^^^^^^^^^^^^

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

The first diagnostic is about what a `for` loop can go through. The
section "Why text has no index" ends with the fix.

</details>

<details>
<summary>Hint 2</summary>

The second diagnostic says that a `char` cannot be compared with a
`str`. `in` needs a collection of characters to look in.

</details>

When every test passes, compare your answer with
[`manual/solutions/06-text/vowels.sisu`](../solutions/06-text/vowels.sisu).

## Exercise 6.3: title case

Open
[`manual/exercises/06-text/title_case.sisu`](../exercises/06-text/title_case.sisu):

<!-- file: exercises/06-text/title_case.sisu -->
```sisu
##! Exercise 6.3, title case. Kind: write. Level: 1.
##!
##! Write the body of `title_case` so that every test passes. Run the
##! tests with `sisu test title_case.sisu`. The hints are in chapter 6
##! of the tutorial, under "Exercise 6.3: title case".

## `text` with each word's first letter in upper case and the rest in
## lower case, and one space between each two words.
def title_case(text: str) -> String:
    return todo("title_case")

test "two words":
    assert title_case("hei maailma") == "Hei Maailma"

test "capitals in the middle":
    assert title_case("tAMPERE AND turku") == "Tampere And Turku"

test "letters with dots and extra spaces":
    assert title_case("  äiti   ja isä ") == "Äiti Ja Isä"

test "no words":
    assert title_case("") == ""
```

<!-- output: solutions/06-text/title_case.console -->
```console
$ sisu test title_case.sisu
warning[W0001]: typed hole `title_case` of type `String`
  --> title_case.sisu:10:12
   |
10 |     return todo("title_case")
   |            ^^^^^^^^^^^^^^^^^^

test "two words" ... blocked by todo("title_case")
test "capitals in the middle" ... blocked by todo("title_case")
test "letters with dots and extra spaces" ... blocked by todo("title_case")
test "no words" ... blocked by todo("title_case")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

Split the text into words, change each word, and join the words again
with spaces. A list of `String` values has a `join` method.

</details>

<details>
<summary>Hint 2</summary>

A `char` has no method that changes its case, but a text of one
character does, and interpolation makes one.

</details>

When every test passes, compare your answer with
[`manual/solutions/06-text/title_case.sisu`](../solutions/06-text/title_case.sisu).

## Exercise 6.4: a CSV line

Open
[`manual/exercises/06-text/csv_fields.sisu`](../exercises/06-text/csv_fields.sisu):

<!-- file: exercises/06-text/csv_fields.sisu -->
```sisu
##! Exercise 6.4, a CSV line. Kind: write. Level: 2.
##!
##! A line of CSV holds fields separated by commas. A field in double
##! quotes can hold commas, and its quotes are not part of the field.
##! Write the body of `csv_fields` so that every test passes. Run the
##! tests with `sisu test csv_fields.sisu`. The hints are in chapter 6
##! of the tutorial, under "Exercise 6.4: a CSV line".

## The fields of `line`, each without the spaces around it.
def csv_fields(line: str) -> list[String]:
    return todo("csv_fields")

test "plain fields":
    assert csv_fields("a,b,c") == ["a", "b", "c"]
    assert csv_fields(" Oulu , 214000 ") == ["Oulu", "214000"]

test "an empty field":
    assert csv_fields("a,,b") == ["a", "", "b"]

test "a comma inside quotes":
    let fields = csv_fields("\"Oulu, Finland\",214000")
    assert fields == ["Oulu, Finland", "214000"]
```

<!-- output: solutions/06-text/csv_fields.console -->
```console
$ sisu test csv_fields.sisu
warning[W0001]: typed hole `csv_fields` of type `list[String]`
  --> csv_fields.sisu:11:12
   |
11 |     return todo("csv_fields")
   |            ^^^^^^^^^^^^^^^^^^

test "plain fields" ... blocked by todo("csv_fields")
test "an empty field" ... blocked by todo("csv_fields")
test "a comma inside quotes" ... blocked by todo("csv_fields")
0 passed, 0 failed, 3 blocked
```

In the last test, `\"` is a double quote inside a string literal.

<details>
<summary>Hint 1</summary>

`split(",")` cannot tell a comma inside quotes from one between fields.
Go through the characters one at a time instead, and build each field
in a `String`.

</details>

<details>
<summary>Hint 2</summary>

Keep a `bool` that says whether the loop is inside quotes. Whether a
comma ends a field depends on it, and the last field ends with the line
rather than with a comma.

</details>

When every test passes, compare your answer with
[`manual/solutions/06-text/csv_fields.sisu`](../solutions/06-text/csv_fields.sisu).

## Exercise 6.5: a Caesar cipher

Open
[`manual/exercises/06-text/caesar.sisu`](../exercises/06-text/caesar.sisu):

<!-- file: exercises/06-text/caesar.sisu -->
```sisu
##! Exercise 6.5, a Caesar cipher. Kind: write. Level: 2.
##!
##! A Caesar cipher moves each letter a fixed number of places along
##! the alphabet, and starts again from `a` after `z`. Write the body of
##! `caesar` so that every test passes. Run the tests with
##! `sisu test caesar.sisu`. The hints are in chapter 6 of the tutorial,
##! under "Exercise 6.5: a Caesar cipher".

## `text` with each letter from `a` to `z` moved `shift` places, each
## capital letter from `A` to `Z` the same way, and every other
## character as it is. `shift` is 0 to 25.
def caesar(text: str, shift: int) -> String:
    return todo("caesar")

test "one place along":
    assert caesar("abc", 1) == "bcd"

test "past the end of the alphabet":
    assert caesar("xyz", 3) == "abc"

test "capitals and punctuation":
    assert caesar("Hei, Aino!", 13) == "Urv, Nvab!"

test "there and back":
    assert caesar(caesar("Kissa", 5), 21) == "Kissa"
```

<!-- output: solutions/06-text/caesar.console -->
```console
$ sisu test caesar.sisu
warning[W0001]: typed hole `caesar` of type `String`
  --> caesar.sisu:13:12
   |
13 |     return todo("caesar")
   |            ^^^^^^^^^^^^^^

test "one place along" ... blocked by todo("caesar")
test "past the end of the alphabet" ... blocked by todo("caesar")
test "capitals and punctuation" ... blocked by todo("caesar")
test "there and back" ... blocked by todo("caesar")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

A letter's place in the alphabet is the difference between its code and
the code of `a`. Adding the shift and taking the remainder by 26 wraps
around past `z`.

</details>

<details>
<summary>Hint 2</summary>

To turn a place back into a letter, cut that place out of a text that
holds the alphabet. A capital letter can go through the same steps in
lower case.

</details>

When every test passes, compare your answer with
[`manual/solutions/06-text/caesar.sisu`](../solutions/06-text/caesar.sisu).

## What you learned

- A `str` is a view of text and a `String` owns text; a `str` parameter
  accepts both.
- A format spec after a colon sets the width, the alignment, the digits
  and the base, and `{x:?}` shows the `Debug` form.
- `trim`, `split`, `split_whitespace`, `lines`, `find`, `replace`,
  `starts_with`, `strip_prefix` and `to_lowercase` work on text and
  leave it as it was.
- `chars()` and `bytes()` go through text. Text has no integer index;
  `byte_slice` cuts it between byte positions that fall between
  characters.
- Raw, multi-line and byte literals; a literal becomes a `Path` where a
  `Path` is expected.
- `String()`, `push` and `Builder` build text, and `parse_int` and
  `parse_float` read numbers out of it as optional values.

Chapter 7 is about your own types: structs with fields and methods,
enums with variants, and `match` to take them apart.
