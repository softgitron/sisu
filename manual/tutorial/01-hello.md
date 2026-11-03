# 1. Hello, Sisu

In this chapter you write and run your first Sisu program, read what the
compiler says when something is wrong, give a value a name, put values
into text, try ideas in a worksheet, and solve your first exercise. You
need a terminal, a text editor and about an hour. The chapter assumes
that you have written programs before, in Python or in another language,
and nothing about Sisu.

## Get the `sisu` command

Everything in this manual goes through one command, `sisu`. It runs
programs, checks them, formats them and runs their tests. There is no
released build yet, so you build it from the repository. You need Git and
a Rust toolchain of version 1.97 or later.

<!-- unchecked: installation, run by hand on a new machine -->
```console
$ git clone https://github.com/softgitron/sisu.git
$ cd sisu/compiler
$ cargo build --release
$ export PATH="$PWD/target/release:$PATH"
```

The build takes a few minutes. When it is done, `sisu --help` lists the
commands. The `export` line lasts as long as the terminal; add it to your
shell's startup file to keep `sisu` on your path.

## Your first program

Create a file named `hello.sisu` that holds these three lines:

<!-- file: examples/01-hello/hello.sisu -->
```sisu
# The first program: it prints one line.
def main():
    print("Hei, maailma!")
```

In the directory that holds the file, run `sisu run hello.sisu`. It
prints:

<!-- output: examples/01-hello/hello.out -->
```text
Hei, maailma!
```

*Hei, maailma* is Finnish for "hello, world". You have written and run a
Sisu program.

## What the program says

The program has three lines, and each says one thing.

- `# The first program: it prints one line.` is a comment. Everything
  from `#` to the end of a line is for the people who read the code, and
  the compiler skips it.
- `def main():` defines a function named `main`. Running a program means
  running its `main`. The colon at the end of the line opens a block.
- The block is the indented line after it. Sisu marks blocks by
  indentation, as Python does, and indents with spaces only. Four spaces
  per level is the layout every Sisu file uses, and the formatter writes
  it for you, as the section "Formatting" shows.
- `print("Hei, maailma!")` calls the function `print` with one argument,
  a string: text between double quotes. `print` writes it to the
  terminal's standard output and ends the line.

> **From Python.** A Python file runs its statements from the top. A
> Sisu program file holds definitions, and its statements live in
> functions, starting with `main`. The section "Trying ideas in a
> worksheet" shows the one kind of file that does run from the top.

## When something is wrong

Sisu checks the whole program before it runs any of it. Misspell `print`
to see what happens when the check fails:

<!-- file: examples/01-hello/misspelled.sisu -->
```sisu
# A program with one mistake: `print` is misspelled.
def main():
    prnt("Hei, maailma!")
```

Run it, and then look up the diagnostic's code with `sisu explain`:

<!-- output: examples/01-hello/misspelled.console -->
```console
$ sisu run misspelled.sisu
error[E0200]: cannot find `prnt` in this scope
  --> misspelled.sisu:3:5
  |
3 |     prnt("Hei, maailma!")
  |     ^^^^
  = fix: a similar name is in scope: `print`

error[D0006]: the program was not run: errors found (1)
$ sisu explain E0200
E0200 (error) resolve.undefined-name: cannot find `{name}` in this scope
```

A diagnostic reads from the top down.

- `error[E0200]` gives its kind and its code. The code is stable: it
  means the same thing in every version of Sisu, so you can search for
  it.
- The message names the thing that is wrong, here `prnt`.
- `--> misspelled.sisu:3:5` says where: the file, line 3, column 5. The
  line itself follows, with `^` under the part the message is about.
- The line that starts `= fix:` is what the compiler suggests. A name
  close to `prnt` is in scope, so it offers `print`. An editor connected
  to Sisu's language server applies such a fix for you.
- The last line comes from the `sisu` command rather than from the
  language, which is why its code starts with `D`: the program had an
  error, so it was not run.

`sisu explain E0200` prints the entry for the code in Sisu's message
catalog: the code, the message's identifier, `resolve.undefined-name`,
and the message, with `{name}` standing for the name. Every message the
compiler writes comes from that catalog under an identifier that does not
change, and this manual refers to messages by their codes.

> **From Python.** Python reports a misspelled name only when it runs
> the line that holds it, which can be in a branch your tests never
> take. Sisu reports it before anything runs.

## Break it on purpose

The fastest way to learn what the messages mean is to cause them. Make
each of the following changes to `hello.sisu`, decide what you expect
Sisu to say, and then run the program. Open the box that follows each
change to compare. Each box shows the change saved as a file of its own,
so the file's name in it differs from yours; the line and the column are
the same. Undo each change before you make the next one.

**1.** Delete the colon at the end of `def main():`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/01-hello/no_colon.console -->
```console
$ sisu run no_colon.sisu
error[E0100]: expected `:`, found the end of the line
  --> no_colon.sisu:2:11
  |
2 | def main()
  |           ^
  = fix: write the missing token: `:`

error[D0006]: the program was not run: errors found (1)
```

</details>

**2.** Replace the four spaces before `print` with a tab.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/01-hello/tab.console -->
```console
$ sisu run tab.sisu
error[E0001]: a tab in indentation; Sisu indents with spaces only
  --> tab.sisu:3:1
  |
3 | 	print("Hei, maailma!")
  | ^
  = note: replace each tab with spaces

error[D0006]: the program was not run: errors found (1)
```

</details>

**3.** Delete the closing quote after `maailma!`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/01-hello/open_string.console -->
```console
$ sisu run open_string.sisu
error[E0003]: this string literal is not closed before the end of the line
  --> open_string.sisu:3:11
  |
3 |     print("Hei, maailma!)
  |           ^^^^^^^^^^^^^^^
  = fix: close it on its line: `"`

error[D0006]: the program was not run: errors found (1)
```

</details>

**4.** Give `print` two strings, as Python's `print` allows:
`print("Hei,", "maailma!")`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/01-hello/two_arguments.console -->
```console
$ sisu run two_arguments.sisu
error[E0305]: wrong number of arguments to `print`: expected 1, found 2
  --> two_arguments.sisu:3:5
  |
3 |     print("Hei,", "maailma!")
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: write the value into the text: `"Hei, maailma!"`

error[D0006]: the program was not run: errors found (1)
```

</details>

`print` takes exactly one string. To print several values together, you
put them into one string, which is what the next two sections are about.

## Naming a value

`let` gives a value a name, and the name then stands for the value:

<!-- file: examples/01-hello/name.sisu -->
```sisu
# `let` gives a value a name.
def main():
    let greeting = "Hei, maailma!"
    print(greeting)
    print(greeting)
```

`sisu run name.sisu` prints the greeting twice:

<!-- output: examples/01-hello/name.out -->
```text
Hei, maailma!
Hei, maailma!
```

A name made with `let` keeps its value for as long as it exists.
Chapter 2 introduces `var`, for names whose value changes.

> **From Python.** In Python, the first assignment to a name creates
> the name. In Sisu only `let` and `var` create one, so a misspelled
> name in an assignment is an error rather than a new variable.

**Try it.** Delete `let ` from the line `let greeting = "Hei, maailma!"`
and run the program again.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/01-hello/no_let.console -->
```console
$ sisu run no_let.sisu
error[E0200]: cannot find `greeting` in this scope
  --> no_let.sisu:3:5
  |
3 |     greeting = "Hei, maailma!"
  |     ^^^^^^^^
  = fix: declare it with `let`: `let `

error[D0006]: the program was not run: errors found (1)
```

Without `let` nothing creates `greeting`, so every line that names it
names something that does not exist.

</details>

## Putting values into text

A string can hold values. Write an expression between braces inside the
quotes, and the string holds the expression's value at that point:

<!-- file: examples/01-hello/interpolation.sisu -->
```sisu
# Values go into text between braces.
def main():
    let name = "Aino"
    let year = 2026
    print("Hei, {name}!")
    print("{name} started learning Sisu in {year}.")
    print("Next year is {year + 1}.")
    print("{year=}")
    print("Braces are written {{ and }}.")
```

**Predict.** Before you run it, decide what the last three lines print.
Then run `sisu run interpolation.sisu` and compare:

<!-- output: examples/01-hello/interpolation.out -->
```text
Hei, Aino!
Aino started learning Sisu in 2026.
Next year is 2027.
year=2026
Braces are written { and }.
```

- `{name}` is replaced by the value of `name`.
- Any expression can stand between the braces, such as `{year + 1}`.
- `{year=}` writes the expression, an `=` and then the value, which is
  handy when you want to see what a name holds.
- `{{` and `}}` write the braces themselves.

Putting values into a string this way is called interpolation. A string
without braces, such as `"Aino"`, is a `str`. A string with braces builds
new text each time its line runs, and that text is a `String`. Chapter 6
explains why Sisu keeps the two apart; for now it is enough to know that
both print the same way.

> **From Python.** There is no `f` before the quotes. Every string in
> Sisu can interpolate, so the prefix has nothing to switch on.

## Trying ideas in a worksheet

Writing a `main` for each experiment is slow. A worksheet is quicker. It
is a file whose first line is `#!/usr/bin/env sisu`, which makes it a
script: its lines run from the top, as a Python file's do.

<!-- file: examples/01-hello/worksheet.sisu -->
```sisu
#!/usr/bin/env sisu
# A worksheet: `sisu ws` writes the value of each line after it.
let name = "Aino"
let greeting = "Hei, {name}!"
print(greeting)
len(greeting)
greeting.to_uppercase()
```

`sisu ws worksheet.sisu` runs the file and prints it back with what each
line produced:

<!-- output: examples/01-hello/worksheet.ws -->
```text
#!/usr/bin/env sisu
# A worksheet: `sisu ws` writes the value of each line after it.
let name = "Aino"  # => "Aino"
let greeting = "Hei, {name}!"  # => "Hei, Aino!"
print(greeting)
# | Hei, Aino!
len(greeting)  # => 10
greeting.to_uppercase()  # => "HEI, AINO!"
```

A comment starting `# =>` shows a line's value, and one starting `# |`
shows what the line printed. The file itself does not change. A
worksheet runs against a pretend file system and a pretend clock, so
nothing it does reaches your files; chapter 18 explains how Sisu can
promise that. `sisu run worksheet.sisu` runs the same file as an
ordinary script, and `sisu repl` offers the same thing one line at a
time.

Use worksheets to try things out. Programs you keep go in files with a
`main`.

## Formatting

Every Sisu file has one layout, and `sisu fmt` writes it. It has no
options, so there is no style to argue about. This file is indented by
two spaces instead of four:

<!-- file: examples/01-hello/messy.sisu -->
```sisu
def main():
  print("Hei, maailma!")
  print("Formatted by sisu fmt.")
```

`sisu fmt --check` reports what it would change, without changing
anything. `sisu fmt` makes the change, after which the check is quiet:

<!-- output: examples/01-hello/messy.console -->
```console
$ sisu fmt --check messy.sisu
`messy.sisu` is not formatted; `sisu fmt` would change it:
@@ line 1 @@
  def main():
-   print("Hei, maailma!")
-   print("Formatted by sisu fmt.")
+     print("Hei, maailma!")
+     print("Formatted by sisu fmt.")
$ sisu fmt messy.sisu
$ sisu fmt --check messy.sisu
```

Lines starting `-` are what the file has, and lines starting `+` are what
the formatter writes instead. The formatter changes only the layout: it
never moves a line into a different block, because in Sisu indentation
is part of the meaning. Run `sisu fmt` on your files before you share
them.

## How exercises work

Each tutorial chapter ends with exercises. An exercise is a Sisu file in
the manual's `exercises` directory, with two things in it:

- A typed hole, `todo("...")`, where your code goes. The compiler accepts
  a hole in place of any value, so the file checks as it is, and it warns
  you that the hole is still there.
- One or more tests. A test is a block that starts `test "a name":`, and
  its `assert` lines state what must be true when your code is right.

`sisu test` followed by a file name runs the file's tests. Each test ends
in one of three ways:

- **blocked**: the test reached a hole, because you have not answered
  yet;
- **FAILED**: the test ran, and an `assert` was false;
- **ok**: the test ran, and everything it asserts holds.

Here is a failure. `shout` should end a word with an exclamation mark, but
it uses a full stop:

<!-- file: examples/01-hello/wrong_shout.sisu -->
```sisu
# A test that fails: `shout` ends the word with the wrong mark.
def shout(word: str) -> String:
    return "{word}."

test "shout ends with an exclamation mark":
    assert shout("hei") == "hei!"
```

<!-- output: examples/01-hello/wrong_shout.console -->
```console
$ sisu test wrong_shout.sisu
test "shout ends with an exclamation mark" ... FAILED

test "shout ends with an exclamation mark" ... FAILED
panic: assertion failed: shout("hei") == "hei!"
  left:  "hei."
  right: "hei!"
  at test "shout ends with an exclamation mark" (wrong_shout.sisu:6:5)
replay: sisu test --replay 1-32eed1c1c3c9d5d9-0.0.1-t0-s0-r0 wrong_shout.sisu

0 passed, 1 failed, 0 blocked
```

The report shows both sides of the `==`: `left` is what `shout("hei")`
gave and `right` is what the test expected. The `replay` line is a
command that reruns exactly this failure; chapter 19 explains it, and you
can ignore it until then.

Three rules hold for every exercise:

- Change only the code, never the tests. The solution passes the same
  tests that you see.
- The first line of the file names the exercise, its kind and its level.
  Level 1 practises one idea from the chapter, level 2 combines the
  chapter's ideas, and level 3 brings in earlier chapters as well.
- The hints are in the chapter, folded away, and the solutions are in the
  manual's `solutions` directory. Look at a solution once your tests
  pass, to compare, or when the hints have not helped.

The line `def shout(word: str) -> String:` defines a function that takes
a `str` named `word` and gives back a `String`, and `return` gives the
value back. Chapter 3 covers functions properly; the exercises before it
only ask you to fill in the value after `return`.

## Exercise 1.1: a name tag

Open
[`manual/exercises/01-hello/name_tag.sisu`](../exercises/01-hello/name_tag.sisu):

<!-- file: exercises/01-hello/name_tag.sisu -->
```sisu
##! Exercise 1.1, a name tag. Kind: complete. Level: 1.
##!
##! A conference prints a tag for each visitor. `name_tag` takes the
##! visitor's name and home town and gives back the text of the tag.
##!
##! Replace the hole, `todo("name tag")`, with an interpolated string
##! that puts the name and the town into the text, so that both tests
##! pass. Run the tests with `sisu test name_tag.sisu`. The hints are in
##! chapter 1 of the tutorial, under "Exercise 1.1: a name tag".

## The text of the name tag for `name`, who comes from `town`.
def name_tag(name: str, town: str) -> String:
    return todo("name tag")

test "a tag for Aino from Oulu":
    assert name_tag("Aino", "Oulu") == "Hei! I am Aino from Oulu."

test "a tag for Eero from Turku":
    assert name_tag("Eero", "Turku") == "Hei! I am Eero from Turku."
```

Lines that start `##` are documentation comments, which chapter 3
returns to. In the file's directory, run the tests before you change
anything:

<!-- output: solutions/01-hello/name_tag.console -->
```console
$ sisu test name_tag.sisu
warning[W0001]: typed hole `name tag` of type `String`
  --> name_tag.sisu:13:12
   |
13 |     return todo("name tag")
   |            ^^^^^^^^^^^^^^^^

test "a tag for Aino from Oulu" ... blocked by todo("name tag")
test "a tag for Eero from Turku" ... blocked by todo("name tag")
0 passed, 0 failed, 2 blocked
```

The warning says that the hole is still there and that whatever replaces
it must be a `String`. Both tests are blocked. Replace the hole and run
the tests again until the last line reads `2 passed, 0 failed, 0
blocked`.

<details>
<summary>Hint 1</summary>

The answer is one string after `return`, with two pairs of braces in it.

</details>

<details>
<summary>Hint 2</summary>

Look at what the first test expects, `"Hei! I am Aino from Oulu."`, and
find the two words that would differ for another visitor. Each of them
is the value of one of the function's parameters, and a value goes into
a string between braces.

</details>

When both tests pass, compare your answer with
[`manual/solutions/01-hello/name_tag.sisu`](../solutions/01-hello/name_tag.sisu).

## What you learned

- `sisu run FILE` checks a program and runs its `main`. Sisu checks the
  whole program first and runs nothing if it finds an error.
- A diagnostic has a stable code, a message, a place and often a fix.
  `sisu explain CODE` describes a code.
- `let` names a value. Braces in a string put values into the text.
- `sisu ws FILE` runs a worksheet and shows each line's value, `sisu fmt`
  writes the one layout, and `sisu test FILE` runs a file's tests, each
  of which is blocked, failed or ok.

Chapter 2 introduces numbers, truth values and the other basic types,
and `var` for names that change. The chapters to come are listed in
[the manual's plan](../README.md#the-tutorial).
