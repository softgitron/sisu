# 12. Views

In this chapter you work with views: values that borrow, such as a
`str`, which views the text of a `String`. You declare structs that
hold views, build a tokenizer whose tokens point into the text they
came from, and learn the rule that keeps a view safe: it cannot leave
the frame that owns what it borrows. When a view has to go further,
there are two ways out. You need about an hour.

## A view borrows what it shows

Chapter 6 said that a `str` is a view of text that lives somewhere
else. A view is a borrow with a type of its own. The methods that cut
text, such as `trim` and `split_whitespace`, give views of the text
they cut, and that text cannot change while a view of it is used:

<!-- file: examples/12-views/words.sisu -->
```sisu
# A `str` views text that a `String` owns, and keeps it borrowed.
def first_word(text: str) -> str:
    return text.split_whitespace().next() ?? ""

def main():
    var line: String = "  Hei maailma  "
    let word = first_word(line)
    let trimmed = line.trim()
    print("{word} | {trimmed}")
    line.push("!")
    print("{line:?}")
```

<!-- output: examples/12-views/words.out -->
```text
Hei | Hei maailma
"  Hei maailma  !"
```

- `first_word` takes a `str` and returns a `str`, so its result
  borrows from its one borrowed parameter, as chapter 11 showed: `word`
  views the text of `line`.
- `line.trim()` gives a view too, without the spaces at the ends.
  Neither view copies a character.
- `line.push("!")` comes after the last use of both views, so the
  `String` may change there.

**Try it.** Move `line.push("!")` before the first `print`, and run
the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/12-views/words_changed.console -->
```console
$ sisu run words_changed.sisu
error[E0405]: `line` is already borrowed on line 7, so it cannot be borrowed with `mut` until that borrow ends on line 10
  --> words_changed.sisu:9:5
   |
 9 |     line.push("!")
   |     ^^^^^^^^^^^^^^
   |
 7 |     let word = first_word(line)
   |                           ---- borrowed here by `word`
   |
10 |     print("{word} | {trimmed}")
   |             ---- the borrow is still used here
  = note: move this after line 10, where the borrow ends, or work on a copy made with `.copy()`

error[E0405]: `line` is already borrowed on line 8, so it cannot be borrowed with `mut` until that borrow ends on line 10
  --> words_changed.sisu:9:5
   |
 9 |     line.push("!")
   |     ^^^^^^^^^^^^^^
   |
 8 |     let trimmed = line.trim()
   |                   ---- borrowed here by `trimmed`
   |
10 |     print("{word} | {trimmed}")
   |                      ------- the borrow is still used here
  = note: move this after line 10, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (2); `--defer-borrow` runs it under the monitor
```

Each view is a borrow of `line`, so E0405 reports both, each with the
line where it is still used. A `push` can move the text to a larger
block of memory, and a view would then point to memory that the
`String` no longer uses.

</details>

A view is cheap to make and to pass, which is why most functions that
read text take a `str`. What a view cannot do is outlive the text it
views, and the rest of this chapter is about that limit.

## A view of part of a list

`Slice[T]` does for a list what `str` does for a `String`: it views a
run of the list's elements without copying them:

<!-- file: examples/12-views/readings.sisu -->
```sisu
# A `Slice` views a run of a list's elements, without copying them.
def total(values: Slice[int]) -> int:
    var sum_so_far = 0
    for value in values:
        sum_so_far += value
    return sum_so_far

def main():
    var readings = [3, 5, 8, 2, 7]
    let all = readings.as_slice()
    let middle = all[1:4]
    print("{total(all)} in all, {total(middle)} in {middle:?}")
    readings.push(1)
    print(total(readings))
```

<!-- output: examples/12-views/readings.out -->
```text
25 in all, 15 in [5, 8, 2]
26
```

`readings.as_slice()` views the whole list, and `all[1:4]` views three
of its elements: slicing a `Slice` gives a view, where slicing a list,
as in chapter 5, gives a new list. A `Slice[int]` parameter accepts a
list as well, as the last call shows, so a function that reads a run of
values can take a `Slice` and serve both.

A slice holds no values of its own: it is the place of the first value
it views and how many it views, so making one copies nothing, however
long the run. Nor does it drop anything when it ends. The values stay
the list's, and the list drops them when it ends:

<!-- file: examples/12-views/towels.sisu -->
```sisu
# A slice views the list's values; only the list drops them.
struct Towel:
    name: String

impl Drop for Towel:
    def drop(mut self):
        print("{self.name} put away")

def count(towels: Slice[Towel]) -> int:
    return len(towels)

def main():
    let towels = [Towel("Aino's"), Towel("Eero's"), Towel("Roni's")]
    for _ in range(2):
        let first_two = towels.as_slice()[:2]
        print("{count(first_two)} towels in view")
    print("the list ends")
```

<!-- output: examples/12-views/towels.out -->
```text
2 towels in view
2 towels in view
the list ends
Aino's put away
Eero's put away
Roni's put away
```

Each turn of the loop makes a slice of two towels and ends it, and no
towel is put away until `towels` itself ends, after the last line of
`main`. `towels.as_slice()[:2]` slices a slice that has no name and
ends with the line, and `first_two` is still used on the next line:
since a slice views the list itself, the slice of it borrows `towels`,
not the slice it was cut from, and lives as long as the list may lend.

## Structs that borrow

A plain struct owns all of its fields. A `view struct` may hold views
as well, and is itself a view. Here each token of a small language
views a word of its source text:

<!-- file: examples/12-views/tokens.sisu -->
```sisu
# A `view struct` holds borrows: each token views the source text.
view struct Token:
    text: str
    line: int

def tokenize(source: str) -> list[Token]:
    var tokens: list[Token] = []
    for i, line in enumerate(source.lines()):
        for word in line.split_whitespace():
            tokens.push(Token(word, i + 1))
    return tokens

def main():
    let source: String = "let total = 4\nprint total"
    let tokens = tokenize(source)
    for token in tokens:
        print("line {token.line}: {token.text}")
```

<!-- output: examples/12-views/tokens.out -->
```text
line 1: let
line 1: total
line 1: =
line 1: 4
line 2: print
line 2: total
```

- `view struct Token` declares a struct that may hold borrows, and its
  field `text: str` views the source.
- `tokenize` cuts the source into lines and the lines into words, all
  views, and keeps each word in a token with its line number. No word
  is copied: a token is a view and a number.
- A `list[Token]` holds views, so the list is a view too, and
  `tokenize` returns it as a borrow of `source`, its one borrowed
  parameter.
- `main` owns the source, and uses the tokens while the source is
  there.

A view has a single lifetime, the shortest of the lifetimes of what it
borrows, which is how the compiler checks a token without lifetime
names (D-004). A struct whose fields would borrow from two places that
end at different times is two structs.

## A view stays where its data lives

A view cannot leave the frame that owns what it borrows. This function
owns its source, and tries to return the tokens:

<!-- file: examples/12-views/tokens_escape.sisu -->
```sisu
# Tokens try to leave the function whose text they view.
view struct Token:
    text: str
    line: int

def tokenize(source: str) -> list[Token]:
    var tokens: list[Token] = []
    for i, line in enumerate(source.lines()):
        for word in line.split_whitespace():
            tokens.push(Token(word, i + 1))
    return tokens

def example_tokens() -> list[Token]:
    let source: String = "let total = 4\nprint total"
    return tokenize(source)

def main():
    for token in example_tokens():
        print("line {token.line}: {token.text}")
```

<!-- output: examples/12-views/tokens_escape.console -->
```console
$ sisu run tokens_escape.sisu
error[E0407]: this returns a borrow of `source`, which belongs to `example_tokens` and is gone when it returns
  --> tokens_escape.sisu:15:5
   |
15 |     return tokenize(source)
   |     ^^^^^^^^^^^^^^^^^^^^^^^
   |                     ------ borrowed here
  = note: return an owned value instead, such as a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

`source` belongs to `example_tokens` and is dropped when the function
returns, so tokens that viewed it would point to text that is gone.
E0407 (`borrow.return-local`) refuses the `return`. Its note asks for
an owned value; a `.copy()` of the list would copy tokens that still
view `source`, so the section "Two ways out" shows the fixes that work.

A view also always ends by being dropped, at the end of its scope or
when a panic unwinds it: the standard library has no way to forget a
view without dropping it (D-058). The same holds for a task and its
scope, which chapter 20 covers. The runtime relies on that promise when
it lets the operating system read straight into a buffer that your
function owns.

## Views make their holders views

A value that holds a view is tied to what the view borrows, so the type
that holds it must say so:

<!-- file: examples/12-views/program.sisu -->
```sisu
# A plain struct that holds tokens, which are views.
view struct Token:
    text: str
    line: int

struct Program:
    name: String
    tokens: list[Token]

def main():
    let program = Program("empty", [])
    print("{program.name}: {len(program.tokens)} tokens")
```

<!-- output: examples/12-views/program.console -->
```console
$ sisu run program.sisu
error[E0411]: `Program` holds `tokens: list[Token]`, which borrows; only a `view struct` may hold a borrow
  --> program.sisu:8:5
  |
8 |     tokens: list[Token]
  |     ^^^^^^^^^^^^^^^^^^^
  |
6 | struct Program:
  |        ------- `Program` is declared here
  = fix: declare it a view: `view `

error[D0006]: the program was not run: errors found (1)
```

`Program` is a plain struct, which owns its fields, and a
`list[Token]` borrows. E0411 (`borrow.view-field-in-plain-struct`)
offers to make `Program` a `view struct`, which would make every
`Program` a view in its turn, tied to the source of its tokens. An enum
has no view form, so a variant cannot hold a view at all (E0412,
`borrow.view-in-enum`). A view deep inside a structure makes every
level that holds it a view, so before you accept the fix, decide
whether the structure should borrow at all, or own its data, as the
next section shows.

## Two ways out

When a view must go where its data cannot, there are two ways out: own
the data, or finish the work while the data is there:

<!-- file: examples/12-views/ways_out.sisu -->
```sisu
# Two ways out: own the text, or finish the work where the text lives.
view struct Token:
    text: str
    line: int

struct Word:
    text: String
    line: int

def tokenize(source: str) -> list[Token]:
    var tokens: list[Token] = []
    for i, line in enumerate(source.lines()):
        for word in line.split_whitespace():
            tokens.push(Token(word, i + 1))
    return tokens

def owned_words(source: str) -> list[Word]:
    return [Word(t.text.to_string(), t.line) for t in tokenize(source)]

def longest_token(source: str) -> int:
    var longest = 0
    for token in tokenize(source):
        longest = max(longest, len(token.text))
    return longest

def example() -> (list[Word], int):
    let source: String = "let total = 4\nprint total"
    return (owned_words(source), longest_token(source))

def main():
    let words, longest = example()
    print("{len(words)} words, the last is {words[^1].text:?}")
    print("the longest token has {longest} characters")
```

<!-- output: examples/12-views/ways_out.out -->
```text
6 words, the last is "total"
the longest token has 5 characters
```

- `owned_words` makes a `Word` of each token, a plain struct whose
  `text` is a `String`. `t.text.to_string()` copies each token's text,
  which costs an allocation for each word, and in return a `Word` can
  go anywhere.
- `longest_token` uses the tokens where the source lives, and returns
  only a number, which borrows nothing.
- `example` owns the source and returns both results, since neither
  borrows from it.

Choose by what the caller needs. If it needs the words themselves after
the text is gone, own them. If it needs something computed from them, a
count, a total or a decision, compute it in the frame that owns the
text: the tokens never have to leave, and nothing is copied.

> **From Python.** A Python object that holds a piece of text keeps the
> text alive for as long as the object lives. A Sisu view never keeps
> its data alive: the data's owner decides how long the data lives, and
> every view of it must end first.

## Exercise 12.1: predict the views

Open
[`manual/exercises/12-views/predict_views.sisu`](../exercises/12-views/predict_views.sisu):

<!-- file: exercises/12-views/predict_views.sisu -->
```sisu
##! Exercise 12.1, predict the views. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_views.sisu`. The hints are in chapter 12 of the
##! tutorial, under "Exercise 12.1: predict the views".

## A setting such as "name = Aino", as two views of the text it was
## read from.
view struct Setting:
    name: str
    value: str

## The settings of `line`, which separates them with `;`. A part
## without `=` is not a setting and is left out.
def settings(line: str) -> list[Setting]:
    var found: list[Setting] = []
    for part in line.split(";"):
        let name, value = part.split_once("=") else:
            continue
        found.push(Setting(name.trim(), value.trim()))
    return found

const COUNT: int = todo("how many settings")
const SECOND_NAME: str = todo("the name of the second setting")
const LAST_VALUE: str = todo("the value of the last setting")
const NONE_AT_ALL: int = todo("how many settings an empty line has")

test "how many":
    assert len(settings("name = Aino; age=34; town")) == COUNT

test "the second name":
    let found = settings("name = Aino; age=34; town")
    assert found[1].name == SECOND_NAME

test "the last value":
    let found = settings("lang=fi;  theme = dark mode ;size=")
    assert found[^1].value == LAST_VALUE

test "an empty line":
    assert len(settings("")) == NONE_AT_ALL
```

<!-- output: solutions/12-views/predict_views.console -->
```console
$ sisu test predict_views.sisu
warning[W0001]: typed hole `how many settings` of type `int`
  --> predict_views.sisu:24:20
   |
24 | const COUNT: int = todo("how many settings")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the name of the second setting` of type `str`
  --> predict_views.sisu:25:26
   |
25 | const SECOND_NAME: str = todo("the name of the second setting")
   |                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the value of the last setting` of type `str`
  --> predict_views.sisu:26:25
   |
26 | const LAST_VALUE: str = todo("the value of the last setting")
   |                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `how many settings an empty line has` of type `int`
  --> predict_views.sisu:27:26
   |
27 | const NONE_AT_ALL: int = todo("how many settings an empty line has")
   |                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "how many" ... blocked by todo("how many settings")
test "the second name" ... blocked by todo("the name of the second setting")
test "the last value" ... blocked by todo("the value of the last setting")
test "an empty line" ... blocked by todo("how many settings an empty line has")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

`split(";")` gives the parts between the semicolons, and
`split_once("=")` cuts a part at its first `=`, or gives `None` when
the part has none.

</details>

<details>
<summary>Hint 2</summary>

`trim` removes the spaces at both ends, and a view of nothing is the
empty text, `""`. Even an empty line has one part.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/12-views/predict_views.sisu`](../solutions/12-views/predict_views.sisu).

## Exercise 12.2: the longest line

Open
[`manual/exercises/12-views/longest_line.sisu`](../exercises/12-views/longest_line.sisu):

<!-- file: exercises/12-views/longest_line.sisu -->
```sisu
##! Exercise 12.2, the longest line. Kind: fix. Level: 1.
##!
##! `longest_line` reads a file and gives its longest line. The file does
##! not compile. Run `sisu test longest_line.sisu`, read the diagnostic,
##! and fix `longest_line`, not the test, so that the test passes. The
##! hints are in chapter 12 of the tutorial, under "Exercise 12.2: the
##! longest line".

## The longest line of the file at `path`, read from `fs`; the first of
## them when two are as long.
def longest_line(fs: Fs, path: Path) -> str ! IoError:
    let text = fs.read_to_string(path)?
    var longest = ""
    for line in text.lines():
        if len(line) > len(longest):
            longest = line
    return Ok(longest)

test "the longest line":
    let fs = Fs.in_memory({"poem.txt": "Hei\nmaailma, hei\nsisu", "e.txt": ""})
    assert longest_line(fs, "poem.txt") == Ok("maailma, hei")
    assert longest_line(fs, "e.txt") == Ok("")
```

<!-- output: solutions/12-views/longest_line.console -->
```console
$ sisu test longest_line.sisu
error[E0413]: `longest_line` returns `str ! IoError`, which borrows, and has more than one borrowed parameter (fs, path); name the ones it borrows from with `from`
  --> longest_line.sisu:11:41
   |
11 | def longest_line(fs: Fs, path: Path) -> str ! IoError:
   |                                         ^^^^^^^^^^^^^
   |                  ------ `fs` is a borrowed parameter
   |                          ---------- `path` is a borrowed parameter
  = fix: return owned text instead: `String ! IoError`
  = fix: name `fs, path` in the `from` clause: ` from fs, path`

error[D0006]: the program was not run: errors found (1)
```

<details>
<summary>Hint 1</summary>

The diagnostic asks which of the two parameters the `str` result
borrows from, and offers two fixes. Before you choose, read the body:
does the line it returns view `fs`, `path`, or something else?

</details>

<details>
<summary>Hint 2</summary>

The lines view `text`, which the function owns and drops when it
returns. The section "Two ways out" says what a result can be instead.

</details>

When the test passes, compare your answer with
[`manual/solutions/12-views/longest_line.sisu`](../solutions/12-views/longest_line.sisu).

## Exercise 12.3: an address book

Open
[`manual/exercises/12-views/address_book.sisu`](../exercises/12-views/address_book.sisu):

<!-- file: exercises/12-views/address_book.sisu -->
```sisu
##! Exercise 12.3, an address book. Kind: fix. Level: 2.
##!
##! An address book keeps the contacts it reads from lines of text. The
##! file does not compile. Run `sisu test address_book.sisu`, read the
##! diagnostics, and fix the types and `add`, not the tests, so that
##! every test passes. The hints are in chapter 12 of the tutorial, under
##! "Exercise 12.3: an address book".

## A person in the address book.
struct Contact:
    name: str
    email: str

struct AddressBook:
    contacts: list[Contact] = []

    ## Adds the contact that `line` describes, such as
    ## "Aino <aino@example.org>"; a line without a "<" is left out.
    def add(mut self, line: str):
        let name, rest = line.split_once("<") else:
            return
        let email = rest.strip_suffix(">")
        self.contacts.push(Contact(name.trim(), email))

test "two contacts":
    var book = AddressBook()
    book.add("Aino <aino@example.org>")
    book.add("Eero Virtanen <eero@example.org>")
    book.add("not a contact")
    assert len(book.contacts) == 2
    assert book.contacts[1].name == "Eero Virtanen"
    assert book.contacts[0].email == "aino@example.org"

test "a line that the caller makes":
    var book = AddressBook()
    for name in ["Roni", "Ella"]:
        book.add("{name} <{name.to_lowercase()}@example.org>")
    assert book.contacts[1].email == "ella@example.org"
```

<!-- output: solutions/12-views/address_book.console -->
```console
$ sisu test address_book.sisu
error[E0411]: `Contact` holds `name: str`, which borrows; only a `view struct` may hold a borrow
  --> address_book.sisu:11:5
   |
11 |     name: str
   |     ^^^^^^^^^
   |
10 | struct Contact:
   |        ------- `Contact` is declared here
  = fix: own the text instead: `String`
  = fix: declare it a view: `view `

error[E0411]: `Contact` holds `email: str`, which borrows; only a `view struct` may hold a borrow
  --> address_book.sisu:12:5
   |
12 |     email: str
   |     ^^^^^^^^^^
   |
10 | struct Contact:
   |        ------- `Contact` is declared here
  = fix: own the text instead: `String`
  = fix: declare it a view: `view `

error[D0006]: the program was not run: errors found (2)
```

<details>
<summary>Hint 1</summary>

E0411 offers two fixes. The second makes `Contact` a view, and then the
book that holds contacts is one too: it would hold views of lines that
its caller makes and drops.

</details>

<details>
<summary>Hint 2</summary>

An address book keeps its contacts after the lines are gone. The
section "Views make their holders views" says what to decide before
you accept the fix.

</details>

When every test passes, compare your answer with
[`manual/solutions/12-views/address_book.sisu`](../solutions/12-views/address_book.sisu).

## Exercise 12.4: a tokenizer

Open
[`manual/exercises/12-views/tokenizer.sisu`](../exercises/12-views/tokenizer.sisu):

<!-- file: exercises/12-views/tokenizer.sisu -->
```sisu
##! Exercise 12.4, a tokenizer. Kind: write. Level: 3.
##!
##! A tokenizer cuts the source of a small language into tokens, each a
##! view of a piece of the source, so that no text is copied. Write the
##! body of `tokenize` so that every test passes. Run the tests with
##! `sisu test tokenizer.sisu`. The hints are in chapter 12 of the
##! tutorial, under "Exercise 12.4: a tokenizer".

@derive(Eq)
enum Kind:
    Name
    Number
    Symbol

## A piece of the source, and the number of its line, counted from 1.
view struct Token:
    kind: Kind
    text: str
    line: int

## The tokens of `source`. Spaces separate the tokens of a line, and a
## `#` starts a comment, which runs to the end of its line and gives no
## tokens. A token is a `Number` if all of its characters are digits, a
## `Name` if its first character is a letter, and a `Symbol` otherwise.
def tokenize(source: str) -> list[Token]:
    return todo("tokenize")

test "one line":
    let tokens = tokenize("total = 12 + x")
    assert len(tokens) == 5
    assert tokens[0].text == "total" and tokens[0].kind == Name
    assert tokens[1].text == "=" and tokens[1].kind == Symbol
    assert tokens[2].text == "12" and tokens[2].kind == Number
    assert tokens[4].kind == Name

test "lines are counted":
    let tokens = tokenize("a = 1\n\nprint a")
    assert [t.line for t in tokens] == [1, 1, 1, 3, 3]

test "comments give no tokens":
    let tokens = tokenize("x = 2  # the start\n# a whole line\nx2")
    assert [t.text for t in tokens] == ["x", "=", "2", "x2"]
    assert tokens[^1].line == 3
    assert tokens[^1].kind == Name
```

<!-- output: solutions/12-views/tokenizer.console -->
```console
$ sisu test tokenizer.sisu
warning[W0001]: typed hole `tokenize` of type `list[Token]`
  --> tokenizer.sisu:26:12
   |
26 |     return todo("tokenize")
   |            ^^^^^^^^^^^^^^^^

test "one line" ... blocked by todo("tokenize")
test "lines are counted" ... blocked by todo("tokenize")
test "comments give no tokens" ... blocked by todo("tokenize")

test "one line" ... blocked by todo("tokenize")
what the test printed:
  in scope at the typed hole `todo("tokenize")`:
    source = "total = 12 + x"

test "lines are counted" ... blocked by todo("tokenize")
what the test printed:
  in scope at the typed hole `todo("tokenize")`:
    source = "a = 1\n\nprint a"

test "comments give no tokens" ... blocked by todo("tokenize")
what the test printed:
  in scope at the typed hole `todo("tokenize")`:
    source = "x = 2  # the start\n# a whole line\nx2"

0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

Start from the tokenizer of the section "Structs that borrow", which
cuts lines and words and gives each token its line.

</details>

<details>
<summary>Hint 2</summary>

Cut each line at its `#` before you cut it into words: `split_once`
gives the part before the `#` as a view.

</details>

<details>
<summary>Hint 3</summary>

A small function that takes a token's text and gives its `Kind` keeps
the loop short.

</details>

When every test passes, compare your answer with
[`manual/solutions/12-views/tokenizer.sisu`](../solutions/12-views/tokenizer.sisu).

## What you learned

- A view borrows what it shows: a `str` views text, a `Slice[T]` views
  a run of a list, and a `view struct` holds views. A list of views is
  a view too.
- A view cannot outlive what it borrows, so it cannot leave the frame
  that owns its data (E0407), a plain struct cannot hold one (E0411),
  and an enum cannot hold one at all (E0412).
- A view always ends by being dropped (D-058).
- When a view must go further, own the data, with `to_string()` and a
  `String` field, or finish the work in the frame that owns the data
  and return an owned result.

Chapter 13 is about values with more than one owner, `Shared` and
`Weak`, about graphs, and about values that must be consumed.
