# 18. Capabilities

Part IV is about the world outside the program: files, the network,
the clock, random numbers and other programs. A Sisu program reaches
that world only through capabilities, values that it is handed and
passes on. Chapter 9 used the first one, `sys.fs`. In this chapter you
meet them all, pass them down to the code that needs them, narrow
them, keep them in structs, lend them for one call, take them back, and
watch every call that goes through one. Last, you see what code without
a capability can do, and how to count the capabilities a program
passes around. You need about an hour and a half.

## What a program is given

`main(sys: System)` receives a `System`, which holds the program's
root capabilities, one for each part of the world:

| Field | Type | What it reaches |
|---|---|---|
| `sys.fs` | `Fs` | files and directories |
| `sys.net` | `Net` | network connections and listeners |
| `sys.clock` | `Clock` | the time, and timers to wait for |
| `sys.rand` | `Rand` | random numbers |
| `sys.env` | `Env` | the arguments and the environment variables |
| `sys.proc` | `Proc` | other programs, run to their end |
| `sys.stdin` | `Stdin` | the lines of standard input |

A program that greets the guests of a sauna evening uses two of them:

<!-- file: examples/18-capabilities/guests.sisu -->
```sisu
# `main` receives the program's capabilities in one value, a `System`.
def main(sys: System) -> ! IoError:
    let guests = sys.env.args()
    let heat = sys.env.get("SAUNA_HEAT") ?? "80"
    sys.fs.write_text("guests.txt", guests.join("\n"))?
    let saved = sys.fs.read_to_string("guests.txt")?
    print("{saved.lines().count()} guests, the sauna at {heat} degrees")
    return Ok(())
```

The arguments for a single file follow its name:

<!-- output: examples/18-capabilities/guests.console -->
```console
$ sisu run guests.sisu Aino Eero Roni
3 guests, the sauna at 80 degrees
```

- `sys.env.args()` gives the arguments, and `sys.env.get("SAUNA_HEAT")`
  the environment variable `SAUNA_HEAT`, or `None` when it is not set,
  so `?? "80"` gives the default. In a shell, `SAUNA_HEAT=90 sisu run
  guests.sisu Aino` prints 90 degrees.
- `sys.fs.write_text` creates or replaces the file `guests.txt` in the
  current directory, and `read_to_string` reads it back.

The runtime makes the `System` and hands it to `main`. No code can
make one that reaches the world: the only other `System` a program can
make is `System.fake(seed)`, a set of fakes that chapter 19 shows. So a
function that `main` does not hand a capability to cannot touch what
the capability reaches.

## Passing capabilities down

No function in Sisu opens a file, reads the clock or looks at an
environment variable out of nowhere. A function that needs one of
them takes the capability as a parameter, and its caller passes it on,
from `main` down to the code that uses it. This program reads a setting
from a variable, or else from a file, `settings.txt`:

<!-- file: examples/18-capabilities/settings.sisu -->
```sisu
# A function takes the capabilities it uses, and its caller passes them.

## The setting `name`: the variable SAUNA_NAME if it is set, or else the
## line `name = value` of the file `settings.txt`.
def setting(env: Env, fs: Fs, name: str) -> String?:
    if let Some(value) = env.get("SAUNA_{name.to_uppercase()}"):
        return Some(value)
    let text = fs.read_to_string("settings.txt") else:
        return None
    for line in text.lines():
        if let Some((key, value)) = line.split_once(" = "), key == name:
            return Some(value.to_string())
    return None

def main(sys: System):
    for name in ["heat", "steam", "music"]:
        print("{name}: {setting(sys.env, sys.fs, name) ?? "not set"}")

test "a variable comes before the file":
    let env = Env.fake(vars={"SAUNA_HEAT": "90"})
    let fs = Fs.in_memory({"settings.txt": "heat = 80\nsteam = gentle"})
    assert setting(env, fs, "heat") == Some("90")
    assert setting(env, fs, "steam") == Some("gentle")
    assert setting(env, Fs.in_memory(), "steam") == None
```

With a `settings.txt` that sets the heat and the steam, and no
variables set, it prints:

<!-- output: examples/18-capabilities/settings.out -->
```text
heat: 80
steam: gentle
music: not set
```

- The signature of `setting` says what it reaches: variables, through
  `env: Env`, and files, through `fs: Fs`. A reader of `main` sees
  `setting(sys.env, sys.fs, name)` and knows that the call cannot reach
  the network.
- The test hands `setting` two fakes: `Env.fake(vars={...})`, an
  environment that holds the variables the test gives it, and
  `Fs.in_memory({...})`, the in-memory file system of chapter 9. Neither
  depends on the machine the test runs on:

<!-- output: examples/18-capabilities/settings_test.console -->
```console
$ sisu test settings.sisu
test "a variable comes before the file" ... ok
1 passed, 0 failed, 0 blocked
```

Each capability has a fake like these, and chapter 19 shows them all.

> **From Python.** In Python, `os.environ`, `open`, `time.time()` and
> `random.random()` work in any function, and a test that wants other
> values patches the module with `unittest.mock.patch`. In Sisu each of
> them is reached through a parameter, so a test passes another value,
> and a function's signature lists what it can touch.

## Narrowing a capability

A capability can be narrowed into one that does less, which is called
attenuation. The file system narrows to a directory with
`restrict_to(dir)` and to reading with `read_only()`, and the network
to some hosts with `only_hosts([...])`. Here the program has a
directory `data` with a file `notes.txt` in it, and a file `secret.txt`
beside the directory:

<!-- file: examples/18-capabilities/narrow.sisu -->
```sisu
# A narrowed capability refuses what the wider one allowed.
def main(sys: System):
    let data = sys.fs.restrict_to("data")
    print(data.read_to_string("notes.txt"))
    print(data.read_to_string("../secret.txt"))
    let notes = data.read_only()
    print(notes.write_text("notes.txt", "Forget the whisks."))
    let net = sys.net.only_hosts(["weather.example.com"])
    match net.connect("tracker.example.org", 443):
        case Ok(_):
            print("connected")
        case Err(e):
            print("refused: {e}")
```

**Predict.** Which of the four operations work? Then compare:

<!-- output: examples/18-capabilities/narrow.out -->
```text
Ok("Buy birch whisks.\n")
Err(PermissionDenied(path="../secret.txt"))
Err(PermissionDenied(path="notes.txt"))
refused: NotAllowed(address="tracker.example.org:443")
```

- `restrict_to("data")` makes a file system whose paths are relative
  to `data`: `notes.txt` is `data/notes.txt`. A path that climbs out
  with `..`, or an absolute one, is refused as `PermissionDenied`, with
  the path as the caller wrote it.
- `read_only()` refuses every change, and narrows what was already
  narrowed: `notes` reads `data` and nothing else, and writes nothing.
- `only_hosts([...])` refuses a connection to any other host as
  `NotAllowed`, before the program reaches the network.

Narrowing never widens: whatever is made from `data` reaches nothing
outside `data`, `data.restrict_to("..")` included. The usual place to narrow is `main`, the one function that
holds every capability, so that the rest of the program gets only what
it needs. Exercise 18.3 narrows a program's file system this way.

## Keeping capabilities in a struct

A function deep in a program may need a capability that only `main`
has, and passing it through every call on the way is noise. A struct
can hold the capabilities its methods use, given once when it is made:

<!-- file: examples/18-capabilities/journal.sisu -->
```sisu
# A struct can hold the capabilities its methods use.
struct Journal:
    fs: Fs
    clock: Clock

impl Journal:
    ## Adds `entry` to the journal, after the time it was added.
    def add(self, entry: str) -> ! IoError:
        let old = self.fs.read_to_string("journal.txt") else: String()
        let line = "{self.clock.now()} {entry}\n"
        self.fs.write_text("journal.txt", "{old}{line}")?
        return Ok(())

def main(sys: System) -> ! IoError:
    let journal = Journal(fs=sys.fs, clock=sys.clock)
    for entry in ["lit the sauna", "went for a swim"]:
        journal.add(entry)?
        print("added: {entry}")
    print("journal.txt is there: {sys.fs.exists("journal.txt")}")
    return Ok(())

test "each entry has its time":
    let fs = Fs.in_memory()
    let start = DateTime.parse("2026-11-03T17:00:00Z").expect("a time")
    let clock = Clock.frozen_at(start)
    let journal = Journal(fs=fs, clock=clock)
    journal.add("lit the sauna").expect("in memory")
    clock.advance(Duration.minutes(40))
    journal.add("went for a swim").expect("in memory")
    let text = fs.read_to_string("journal.txt").expect("written")
    let lines: list[str] = text.lines().collect()
    assert lines[0] == "2026-11-03T17:00:00Z lit the sauna"
    assert lines[1] == "2026-11-03T17:40:00Z went for a swim"
```

The program adds two entries, and its test passes:

<!-- output: examples/18-capabilities/journal.console -->
```console
$ sisu run journal.sisu
added: lit the sauna
added: went for a swim
journal.txt is there: True
$ sisu test journal.sisu
test "each entry has its time" ... ok
1 passed, 0 failed, 0 blocked
```

- A capability is `Copy`. `Journal(fs=sys.fs, ...)` copies the handle
  into the struct, and `main` goes on using `sys.fs`. Both copies reach
  the same file system: copying a capability gives nothing that the
  copy did not already have.
- `self.clock.now()` reads the time as a `DateTime`, which prints in
  the form `2026-11-03T17:00:00Z`.
- In the test, `Clock.frozen_at(start)` is a test clock: it stands still
  at `start` until the test moves it with `advance`. The journal holds a
  copy of it, and sees it move, because what a fake knows lives behind
  the handle, where every copy reaches it.

A struct that holds capabilities is how a larger program keeps its
signatures short: a `Service` made in `main` with its file system,
network and clock, whose methods need no capability parameters.

## Lending a capability for one call

A struct keeps what it is given, which is right for a journal but more
than a function that only reads during one call needs. `lend()` gives
a capability for the length of a call, and no longer:

<!-- file: examples/18-capabilities/lend.sisu -->
```sisu
# A lent capability lets a function use it for one call, and no longer.
def total_words[F: FsOps](fs: F, paths: list[Path]) -> int:
    var total = 0
    for path in paths:
        let text = fs.read_to_string(path) else: String()
        total += text.split_whitespace().count()
    return total

def main(sys: System):
    print(total_words(sys.fs.lend(), ["a.txt", "b.txt"]))
    print(total_words(sys.fs.read_only(), ["a.txt"]))
```

With a file `a.txt` of two words and `b.txt` of one, it prints:

<!-- output: examples/18-capabilities/lend.out -->
```text
3
2
```

- `sys.fs.lend()` is a `Lent[Fs]`, a view of the file system in the
  sense of chapter 12: it borrows, so the function that receives it can
  use it while it runs and cannot keep it after it returns. Lending
  costs nothing when the program runs; the compiler checks it.
- The operations of a file system are a trait, `FsOps`, which every
  form of one implements: the root capability, a narrowed one, a lent
  one, a fake and an interceptor. A parameter of type `Fs` accepts every
  form but the lent one, since an `Fs` can be stored; a generic
  parameter `[F: FsOps]` accepts every form, the lent one too. The
  other capabilities have their traits in the same way: `NetOps`,
  `ClockOps`, `RandOps`, `EnvOps`, `ProcOps` and `StdinOps`.

**Try it.** Write a function `keep(fs: Lent[Fs]) -> Reader` that
returns a struct `Reader` holding the lent file system in a field of
type `Fs`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/18-capabilities/keep_lent.console -->
```console
$ sisu run keep_lent.sisu
error[E0421]: `Lent[Fs]` borrows, so it cannot be put in `Fs`, which can outlive what it borrows (04, rule 3)
  --> keep_lent.sisu:6:22
  |
6 |     return Reader(fs=fs)
  |                      ^^
  = note: put an owned value in it instead, such as a `String` made with `.to_string()`

error[D0006]: the program was not run: errors found (1)
```

E0421 (`borrow.view-in-owner`) is the error of chapter 12 for a view
kept in a value that can outlive it. Its note is written for text and
does not fit a capability. The two fixes are a parameter of type `Fs`,
for a function that must keep what it is given, or doing the work
inside the call, without the struct.

</details>

## Taking a capability back

A lent capability ends with the call. A capability that is kept for a
while, and must stop working at a time the program chooses, comes from
a caretaker: `revocable()` gives a capability and its `Revoker`, and
after `revoke()` every copy of that capability fails. Here a plugin
gets its own directory, `plugins/clock`, until the program revokes it:

<!-- file: examples/18-capabilities/plugin.sisu -->
```sisu
# A caretaker's capability fails once its revoker revokes it.
struct Plugin:
    fs: Fs

impl Plugin:
    def settings(self) -> String ! IoError:
        return self.fs.read_to_string("plugin.toml")

def main(sys: System):
    let plugin_dir = sys.fs.restrict_to("plugins/clock")
    let plugin_fs, revoker = plugin_dir.revocable()
    let plugin = Plugin(fs=plugin_fs)
    print(plugin.settings())
    revoker.revoke()
    print(plugin.settings())
    print(plugin_dir.read_to_string("plugin.toml"))
```

<!-- output: examples/18-capabilities/plugin.out -->
```text
Ok("face = \"round\"\n")
Err(Revoked)
Ok("face = \"round\"\n")
```

- `plugin_dir.revocable()` gives a pair: the caretaker's file system,
  which the plugin keeps, and the revoker, which `main` keeps.
- After `revoker.revoke()`, the plugin's copy fails with `Revoked`, a
  variant of `IoError` like `NotFound`, and so would any other copy the
  plugin had made.
- `plugin_dir`, which the caretaker was made from, still works.
  Revoking takes back what was handed out, and nothing else.

Choose the form by how long the code that receives a capability should
have it: a plain capability for as long as it likes, a lent one for one
call, a revocable one until the program says so.

## What needs no capability

A few things need no capability. They write to the program's own
standard streams, or start tasks, and reach nothing else:

<!-- file: examples/18-capabilities/ambient.sisu -->
```sisu
# Printing, logging and opening a scope need no capability.
def main():
    print("the sauna is warm")
    print_err("the stove needs wood")
    log.info("added two logs")
    task.scope do s:
        s.spawn(() -> log.warn("the water bucket is empty"))
```

<!-- output: examples/18-capabilities/ambient.console -->
```console
$ sisu run ambient.sisu
the sauna is warm
the stove needs wood
info: added two logs
warn [task 1]: the water bucket is empty
```

- `print` writes to standard output, and `print_err` to standard
  error.
- `log` writes records to standard error, each with its level:
  `log.debug`, `log.info`, `log.warn` and `log.error`. A record written
  inside a task names the task, `[task 1]`.
- `task.scope` opens a scope for tasks, which chapter 20 is about.

Everything else needs a capability: files, the network, the clock,
random numbers, variables and other programs. A clause `with Fs, Clock`
on a function's signature, which would pass capabilities without
writing them at each call, is reserved syntax that the language may add
once the cost of the extra parameters has been measured (Q-5); it is
not in the compiler.

## Watching every call

Every effect on the world passes through a capability, so a capability
is also the place to watch the effects. An interceptor is shown each
call through a capability before it runs and after it returns:

<!-- file: examples/18-capabilities/wrap.sisu -->
```sisu
# An interceptor is shown each call through a capability.
struct Printer:
    pass

impl cap.Interceptor for Printer:
    def before(self, call: cap.Call):
        print("> {call.capability}.{call.method}({call.args})")

    def after(self, call: cap.Call, outcome: str):
        print("< {outcome}")

def load(fs: Fs) -> String ! IoError:
    return fs.read_to_string("settings.txt")

def main(sys: System):
    let fs: Fs = cap.Wrap(inner=sys.fs, hook=Printer())
    print(load(fs))
    print(fs.exists("other.txt"))
```

<!-- output: examples/18-capabilities/wrap.out -->
```text
> Fs.read_to_string("settings.txt")
< Ok("heat = 80\n")
Ok("heat = 80\n")
> Fs.exists("other.txt")
< False
False
```

- `cap` is the standard library's module of capabilities, which every
  module sees without an import. It grants nothing by itself.
- `cap.Interceptor` is a trait with two methods: `before` gets a
  `cap.Call`, which names the capability, the method and its arguments
  as text, and `after` gets the call again with its outcome.
- `cap.Wrap(inner=sys.fs, hook=Printer())` puts the interceptor around
  the file system. Where an `Fs` is expected, the `Wrap` becomes one,
  so `load` takes it as it takes any other file system and does not
  know that it is watched.

The interception is written in `main`, where a reader sees it, and it
sees every call, so it suits tracing, auditing and counting. An
interceptor must be safe to share between tasks, since a capability can
reach every task; one that is not gets E0700 (`cap.not-shareable`).

## Code that takes no capability

A function that takes no capability, and calls only functions that take
none, can only compute: it can read its arguments and return a value,
and it cannot touch a file, the network or the clock. `pure` says more:
a `pure` function returns the same value for the same arguments, every
time:

<!-- file: examples/18-capabilities/compute.sisu -->
```sisu
# Code that takes no capability can only compute.
pure def word_count(text: str) -> int:
    var n = 0
    for _ in text.split_whitespace():
        n += 1
    return n

def file_word_count(fs: Fs, path: Path) -> int ! IoError:
    let text = fs.read_to_string(path)?
    return Ok(word_count(text))

def main(sys: System) -> ! IoError:
    print(word_count("hei kaunis maailma"))
    print(file_word_count(sys.fs, "poem.txt")?)
    return Ok(())
```

With a `poem.txt` of eight words, it prints:

<!-- output: examples/18-capabilities/compute.out -->
```text
3
8
```

*Hei kaunis maailma* is Finnish for "hello, beautiful world".
`word_count` does the work, and `file_word_count` only reads the file
and hands the text over. That split keeps most of a program in
functions that are quick to test and to reuse, with the capabilities at
its edges.

The same holds for a whole package. A library whose `pub` functions
take no capability, and that its users do not trust with `unsafe`
(chapter 17's `[trust]`), can only compute, whatever its code does
(D-066). The registry is planned to show this as the library's
capability summary, which chapter 17's "Publishing, a first look"
describes.

**Try it.** Mark `file_word_count` as `pure`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/18-capabilities/pure_file.console -->
```console
$ sisu run pure_file.sisu
error[E0334]: the pure function `file_word_count` cannot use the capability `Fs`
  --> pure_file.sisu:8:26
  |
8 | pure def file_word_count(fs: Fs, path: Path) -> int ! IoError:
  |                          ^^^^^^
  = fix: remove `pure`

error[D0006]: the program was not run: errors found (1)
```

E0334 (`type.impure-capability`) refuses a capability in a `pure`
function, since a file can change between two calls, and its fix
removes `pure`. A function that reads a file is not pure, and the fix
says so.

</details>

`word_count` counts with a loop rather than with `count()`, because the
iterator methods of chapter 16 are not marked `pure` in the compiler
yet, and a `pure` function can call only `pure` functions.

## Counting a program's capabilities

`sisu audit --caps` lists, for each file, the functions that use a
capability and how far each is from `main`:

<!-- output: examples/18-capabilities/audit.console -->
```console
$ sisu audit --caps settings.sisu compute.sisu
settings.sisu function main caps=1 io=calls depth=0
settings.sisu function setting caps=2 io=direct depth=1
settings.sisu function test#0 caps=0 io=calls depth=-
settings.sisu use setting EnvOps.get depth=1
settings.sisu use setting FsOps.read_to_string depth=1
settings.sisu summary functions=3 median=1 max=2 uses=2 deepest=1 structs=0
compute.sisu function file_word_count caps=1 io=direct depth=1
compute.sisu function main caps=1 io=calls depth=0
compute.sisu use file_word_count FsOps.read_to_string depth=1
compute.sisu summary functions=2 median=1 max=1 uses=1 deepest=1 structs=0
```

- A `function` line names a function that does I/O, the number of its
  parameters that hold a capability (`caps`), whether it uses one
  itself (`io=direct`) or only calls functions that do (`io=calls`),
  and its depth: the fewest calls from `main`, or `-` when `main` does
  not reach it, as for a test.
- A `use` line names each operation a function calls, as a method of
  its trait: `EnvOps.get` and `FsOps.read_to_string`.
- The `summary` counts them; `structs` counts the structs that hold
  three capabilities or more, which may be doing too many jobs.

`word_count` is in no line: it does no I/O. The audit is data rather
than advice, and its use is to watch a program over time: a function
whose `caps` grows is gathering authority, and a function deep in the
program with `io=direct` may be using a capability that `main` should
have narrowed first.

## Exercise 18.1: predict the narrowed file systems

Open
[`manual/exercises/18-capabilities/predict_narrowing.sisu`](../exercises/18-capabilities/predict_narrowing.sisu):

<!-- file: exercises/18-capabilities/predict_narrowing.sisu -->
```sisu
##! Exercise 18.1, predict the narrowed file systems. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the value of the expression in
##! its test. Write each as a literal, then run
##! `sisu test predict_narrowing.sisu`. The hints are in chapter 18 of
##! the tutorial, under "Exercise 18.1: predict the narrowed file
##! systems".

## A file system with a note under `notes/`, and a secret beside it.
def disk() -> Fs:
    return Fs.in_memory({"notes/todo.txt": "heat the sauna", "pin.txt": "1234"})

const INSIDE: bool = todo("whether the narrowed one finds the note")
const REFUSED: str = todo("what the narrowed one says about the secret")
const READ_ONLY_WRITE: bool = todo("whether the read-only write works")
const WRITTEN_THROUGH: bool = todo("whether the file is under notes/")
const COPY_SEES: bool = todo("whether the original sees the copy's file")
const AFTER_REVOKE: bool = todo("whether the caretaker finds the note")

test "inside the narrowed directory":
    let notes = disk().restrict_to("notes")
    assert notes.exists("todo.txt") == INSIDE

test "outside it":
    let notes = disk().restrict_to("notes")
    assert "{notes.read_to_string("../pin.txt")}" == REFUSED

test "a write through a read-only one":
    let ro = disk().read_only()
    assert ro.write_text("notes/new.txt", "x").is_ok() == READ_ONLY_WRITE

test "a write through the narrowed one":
    let fs = disk()
    fs.restrict_to("notes").write_text("new.txt", "x").expect("allowed")
    assert fs.exists("notes/new.txt") == WRITTEN_THROUGH

test "a copy of a capability":
    let fs = disk()
    let copy = fs
    copy.write_text("draft.txt", "x").expect("allowed")
    assert fs.exists("draft.txt") == COPY_SEES

test "a revoked caretaker":
    let caretaker, revoker = disk().revocable()
    revoker.revoke()
    assert caretaker.exists("notes/todo.txt") == AFTER_REVOKE
```

<!-- output: solutions/18-capabilities/predict_narrowing.console -->
```console
$ sisu test predict_narrowing.sisu
warning[W0001]: typed hole `whether the narrowed one finds the note` of type `bool`
  --> predict_narrowing.sisu:13:22
   |
13 | const INSIDE: bool = todo("whether the narrowed one finds the note")
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `what the narrowed one says about the secret` of type `str`
  --> predict_narrowing.sisu:14:22
   |
14 | const REFUSED: str = todo("what the narrowed one says about the secret")
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `whether the read-only write works` of type `bool`
  --> predict_narrowing.sisu:15:31
   |
15 | const READ_ONLY_WRITE: bool = todo("whether the read-only write works")
   |                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `whether the file is under notes/` of type `bool`
  --> predict_narrowing.sisu:16:31
   |
16 | const WRITTEN_THROUGH: bool = todo("whether the file is under notes/")
   |                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `whether the original sees the copy's file` of type `bool`
  --> predict_narrowing.sisu:17:25
   |
17 | const COPY_SEES: bool = todo("whether the original sees the copy's file")
   |                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `whether the caretaker finds the note` of type `bool`
  --> predict_narrowing.sisu:18:28
   |
18 | const AFTER_REVOKE: bool = todo("whether the caretaker finds the note")
   |                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "inside the narrowed directory" ... blocked by todo("whether the narrowed one finds the note")
test "outside it" ... blocked by todo("what the narrowed one says about the secret")
test "a write through a read-only one" ... blocked by todo("whether the read-only write works")
test "a write through the narrowed one" ... blocked by todo("whether the file is under notes/")
test "a copy of a capability" ... blocked by todo("whether the original sees the copy's file")
test "a revoked caretaker" ... blocked by todo("whether the caretaker finds the note")
0 passed, 0 failed, 6 blocked
```

<details>
<summary>Hint 1</summary>

A narrowed file system sees paths relative to its directory, and a
path that climbs out of it is refused. A refusal is an error with the
path as the caller wrote it.

</details>

<details>
<summary>Hint 2</summary>

Copying a capability copies the handle, not the files behind it. A
revoked caretaker fails at everything, and a failure to look at a file
reads as "not there".

</details>

When every test passes, compare your reasoning with
[`manual/solutions/18-capabilities/predict_narrowing.sisu`](../solutions/18-capabilities/predict_narrowing.sisu).

## Exercise 18.2: a clock to test with

Open
[`manual/exercises/18-capabilities/days_left.sisu`](../exercises/18-capabilities/days_left.sisu):

<!-- file: exercises/18-capabilities/days_left.sisu -->
```sisu
##! Exercise 18.2, a clock to test with. Kind: fix. Level: 1.
##!
##! `days_left` says how many whole days are left until a library book is
##! due. It takes the whole `System` to read the clock, so a test cannot
##! hand it a clock of its own. The file does not compile. Run
##! `sisu test days_left.sisu`, read the diagnostics, and change
##! `days_left`, not the tests, so that every test passes. The hints are
##! in chapter 18 of the tutorial, under "Exercise 18.2: a clock to test
##! with".

## The whole days from now until `due`, or 0 once it has passed.
def days_left(sys: System, due: DateTime) -> int:
    let left = due - sys.clock.now()
    if left < Duration.seconds(0):
        return 0
    return left.as_seconds() // 86400

## The time that `text` writes, for the tests.
def at(text: str) -> DateTime:
    return DateTime.parse(text).expect("a time")

test "a book due in three days":
    let clock = Clock.frozen_at(at("2026-11-03T12:00:00Z"))
    assert days_left(clock, at("2026-11-06T12:00:00Z")) == 3
    assert days_left(clock, at("2026-11-06T11:00:00Z")) == 2

test "a book that is late":
    let clock = Clock.frozen_at(at("2026-11-10T09:00:00Z"))
    assert days_left(clock, at("2026-11-06T12:00:00Z")) == 0
```

<!-- output: solutions/18-capabilities/days_left.console -->
```console
$ sisu test days_left.sisu
error[E0300]: expected `System`, found `TestClock`
  --> days_left.sisu:24:22
   |
24 |     assert days_left(clock, at("2026-11-06T12:00:00Z")) == 3
   |                      ^^^^^

error[E0300]: expected `System`, found `TestClock`
  --> days_left.sisu:25:22
   |
25 |     assert days_left(clock, at("2026-11-06T11:00:00Z")) == 2
   |                      ^^^^^

error[E0300]: expected `System`, found `TestClock`
  --> days_left.sisu:29:22
   |
29 |     assert days_left(clock, at("2026-11-06T12:00:00Z")) == 0
   |                      ^^^^^

error[D0006]: the program was not run: errors found (3)
```

E0300 (`type.mismatch`) says that the tests pass a test clock where
`days_left` expects a whole `System`. It offers no fix, since the
change that helps is to the signature.

<details>
<summary>Hint 1</summary>

`days_left` uses one capability of the `System` it is given. The
tests give it that capability alone.

</details>

<details>
<summary>Hint 2</summary>

A test clock becomes a `Clock` where one is expected. The section
"Passing capabilities down" shows a function that takes only the
capabilities it uses.

</details>

When every test passes, compare your answer with
[`manual/solutions/18-capabilities/days_left.sisu`](../solutions/18-capabilities/days_left.sisu).

## Exercise 18.3: a narrow file system

Open
[`manual/exercises/18-capabilities/reports.sisu`](../exercises/18-capabilities/reports.sisu):

<!-- file: exercises/18-capabilities/reports.sisu -->
```sisu
##! Exercise 18.3, a narrow file system. Kind: complete. Level: 2.
##!
##! A report generator writes its reports under `reports/`, and a preview
##! shows the first line of one. Neither should see anything outside
##! `reports/`, and the preview should change nothing. Replace the two
##! holes so that every test passes. Run the tests with
##! `sisu test reports.sisu`. The hints are in chapter 18 of the tutorial,
##! under "Exercise 18.3: a narrow file system".

## The file system the generator gets: the files under `reports/`, to
## read and write, and nothing outside it.
def for_generator(fs: Fs) -> Fs:
    return todo("the generator's file system")

## The file system the preview gets: the same files, read only.
def for_preview(fs: Fs) -> Fs:
    return todo("the preview's file system")

## Writes the report `name`, a line for each of `lines`.
def generate(fs: Fs, name: str, lines: list[str]) -> ! IoError:
    fs.write_text(Path.new("{name}.txt"), lines.join("\n"))?
    return Ok(())

## The first line of the report `name`.
def preview(fs: Fs, name: str) -> String ! IoError:
    let text = fs.read_to_string(Path.new("{name}.txt"))?
    let first = text.lines().next() ?? ""
    return Ok(first.to_string())

## A disk with last week's report and a file that is none of theirs.
def disk() -> Fs:
    return Fs.in_memory({"reports/w44.txt": "quiet\nwarm", "pin.txt": "1234"})

test "the generator writes under reports/":
    let fs = disk()
    generate(for_generator(fs), "w45", ["busy", "full"]).expect("allowed")
    assert fs.read_to_string("reports/w45.txt") == Ok("busy\nfull")

test "the generator sees nothing outside":
    let fs = for_generator(disk())
    assert fs.exists("w44.txt")
    assert not fs.exists("pin.txt")
    assert not fs.exists("../pin.txt")

test "the preview reads and does not write":
    let fs = disk()
    assert preview(for_preview(fs), "w44") == Ok("quiet")
    assert generate(for_preview(fs), "w46", ["x"]).is_err()
    assert not fs.exists("reports/w46.txt")
    assert not for_preview(fs).exists("../pin.txt")
```

<!-- output: solutions/18-capabilities/reports.console -->
```console
$ sisu test reports.sisu
warning[W0001]: typed hole `the generator's file system` of type `Fs`
  --> reports.sisu:13:12
   |
13 |     return todo("the generator's file system")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the preview's file system` of type `Fs`
  --> reports.sisu:17:12
   |
17 |     return todo("the preview's file system")
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "the generator writes under reports/" ... blocked by todo("the generator's file system")
test "the generator sees nothing outside" ... blocked by todo("the generator's file system")
test "the preview reads and does not write" ... blocked by todo("the preview's file system")
0 passed, 0 failed, 3 blocked
```

<details>
<summary>Hint 1</summary>

Each hole is one expression on `fs`. The generator's paths are
relative to `reports/`, as `generate` writes `w45.txt` and the test
finds `reports/w45.txt`.

</details>

<details>
<summary>Hint 2</summary>

Narrowing composes: a narrowed file system can be narrowed again, and
never gets wider. The preview's file system is the generator's, with
one more restriction.

</details>

When every test passes, compare your answer with
[`manual/solutions/18-capabilities/reports.sisu`](../solutions/18-capabilities/reports.sisu).

## Exercise 18.4: any form of a file system

Open
[`manual/exercises/18-capabilities/total_size.sisu`](../exercises/18-capabilities/total_size.sisu):

<!-- file: exercises/18-capabilities/total_size.sisu -->
```sisu
##! Exercise 18.4, any form of a file system. Kind: write. Level: 2.
##!
##! `total_size` adds up the sizes of the files directly in a directory.
##! It only reads, and only while it runs, so it accepts every form of a
##! file system: a plain one, a narrowed one, and one lent for the call.
##! Write its body so that every test passes. Run the tests with
##! `sisu test total_size.sisu`. The hints are in chapter 18 of the
##! tutorial, under "Exercise 18.4: any form of a file system".

## The sum of the sizes, in bytes, of the files directly in `dir`; the
## directories in it do not count.
def total_size[F: FsOps](fs: F, dir: Path) -> int ! IoError:
    return todo("total_size")

## Two photos, and an older one in a directory of its own.
def disk() -> Fs:
    return Fs.in_memory(
        {
            "photos/a.jpg": "12345",
            "photos/b.jpg": "123",
            "photos/old/c.jpg": "1234567",
        }
    )

test "the files directly in a directory":
    assert total_size(disk(), "photos") == Ok(8)
    assert total_size(disk(), "photos/old") == Ok(7)

test "a lent file system":
    let fs = disk()
    assert total_size(fs.lend(), "photos") == Ok(8)

test "narrowed file systems":
    let fs = disk()
    assert total_size(fs.restrict_to("photos"), ".") == Ok(8)
    assert total_size(fs.read_only(), "photos/old") == Ok(7)

test "a directory that is not there":
    assert total_size(disk(), "music") == Err(NotFound("music"))
```

<!-- output: solutions/18-capabilities/total_size.console -->
```console
$ sisu test total_size.sisu
warning[W0001]: typed hole `total_size` of type `int ! IoError`
  --> total_size.sisu:13:12
   |
13 |     return todo("total_size")
   |            ^^^^^^^^^^^^^^^^^^

test "the files directly in a directory" ... blocked by todo("total_size")
test "a lent file system" ... blocked by todo("total_size")
test "narrowed file systems" ... blocked by todo("total_size")
test "a directory that is not there" ... blocked by todo("total_size")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

`list_dir` gives the paths in a directory, and `metadata` tells a
file's size (`len`) and whether it is a directory (`is_dir`). Both can
fail, and the last test wants the first failure passed on.

</details>

<details>
<summary>Hint 2</summary>

`fs` has the type `F`, bounded by `FsOps`, so it has exactly the
methods of that trait, whichever form of a file system the caller
passed. The body is the same as for a parameter of type `Fs`.

</details>

When every test passes, compare your answer with
[`manual/solutions/18-capabilities/total_size.sisu`](../solutions/18-capabilities/total_size.sisu).

## Exercise 18.5: a cache that expires

Open
[`manual/exercises/18-capabilities/cache.sisu`](../exercises/18-capabilities/cache.sisu):

<!-- file: exercises/18-capabilities/cache.sisu -->
```sisu
##! Exercise 18.5, a cache that expires. Kind: write. Level: 3.
##!
##! A weather widget asks a slow service for the forecast. It keeps each
##! answer in a file under `cache/`, so that it can answer again without
##! asking, until the answer is older than the widget accepts. `Cache`
##! holds the capabilities it needs. Write the bodies of `put` and `get`
##! so that every test passes. Run the tests with `sisu test cache.sisu`.
##! The hints are in chapter 18 of the tutorial, under "Exercise 18.5: a
##! cache that expires".

## Answers kept in files under `cache/`, a file for each key, and how
## long each stays fresh.
struct Cache:
    fs: Fs
    clock: Clock
    max_age: Duration

impl Cache:
    ## Keeps `text` as the answer for `key`, in the file `cache/KEY`.
    def put(self, key: str, text: str) -> ! IoError:
        return todo("put")

    ## The answer kept for `key`, if there is one and it is no older than
    ## `max_age`.
    def get(self, key: str) -> String?:
        return todo("get")

## A clock that stands at noon on 3 November 2026 until a test moves it.
def noon() -> TestClock:
    let t = DateTime.parse("2026-11-03T12:00:00Z").expect("a time")
    return Clock.frozen_at(t)

test "an answer is kept in a file":
    let fs = Fs.in_memory()
    let ten = Duration.minutes(10)
    let cache = Cache(fs=fs, clock=noon(), max_age=ten)
    cache.put("helsinki", "snow").expect("in memory")
    assert fs.exists("cache/helsinki")
    assert cache.get("helsinki") == Some("snow")

test "a key that was never kept":
    let hour = Duration.hours(1)
    let cache = Cache(fs=Fs.in_memory(), clock=noon(), max_age=hour)
    assert cache.get("turku") == None

test "an answer goes stale":
    let clock = noon()
    let ten = Duration.minutes(10)
    let cache = Cache(fs=Fs.in_memory(), clock=clock, max_age=ten)
    cache.put("oulu", "rain").expect("in memory")
    clock.advance(Duration.minutes(10))
    assert cache.get("oulu") == Some("rain")
    clock.advance(Duration.seconds(1))
    assert cache.get("oulu") == None

test "a new answer replaces a stale one":
    let clock = noon()
    let five = Duration.minutes(5)
    let cache = Cache(fs=Fs.in_memory(), clock=clock, max_age=five)
    cache.put("oulu", "rain").expect("in memory")
    clock.advance(Duration.minutes(30))
    cache.put("oulu", "sleet").expect("in memory")
    assert cache.get("oulu") == Some("sleet")
```

<!-- output: solutions/18-capabilities/cache.console -->
```console
$ sisu test cache.sisu
warning[W0001]: typed hole `put` of type `! IoError`
  --> cache.sisu:21:16
   |
21 |         return todo("put")
   |                ^^^^^^^^^^^

warning[W0001]: typed hole `get` of type `String?`
  --> cache.sisu:26:16
   |
26 |         return todo("get")
   |                ^^^^^^^^^^^

test "an answer is kept in a file" ... blocked by todo("put")
test "a key that was never kept" ... blocked by todo("get")
test "an answer goes stale" ... blocked by todo("put")
test "a new answer replaces a stale one" ... blocked by todo("put")
0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

`put` writes one file, and the directory `cache` must exist first:
`create_dir` makes it, and does nothing when it is already there.

</details>

<details>
<summary>Hint 2</summary>

`get` needs the time the answer was written. A file's `metadata` has
it as `modified`, and the in-memory file system stamps each write with
the test clock, so a test that moves the clock moves the answer's age.

</details>

<details>
<summary>Hint 3</summary>

The age of an answer is the time now minus the time it was written, a
`Duration`, which compares with `max_age`. A missing file, and a stale
one, both give `None`.

</details>

When every test passes, compare your answer with
[`manual/solutions/18-capabilities/cache.sisu`](../solutions/18-capabilities/cache.sisu).

## What you learned

- `main(sys: System)` receives the root capabilities: `fs`, `net`,
  `clock`, `rand`, `env`, `proc` and `stdin`. No code can make a
  `System` that reaches the world, and a function reaches it only
  through the capabilities it is passed.
- A function takes the capabilities it uses as parameters, so its
  signature says what it can touch and a test can pass fakes.
- `restrict_to`, `read_only` and `only_hosts` narrow a capability, and
  narrowing never widens; `main` is the place to narrow.
- A capability is `Copy`; a struct can keep one, and every copy reaches
  the same thing, a fake's state included.
- `lend()` gives a capability for one call, as a `Lent[T]` that cannot
  be kept (E0421); `[F: FsOps]` accepts every form. `revocable()` gives
  one that fails with `Revoked` once its revoker revokes it.
- `print`, `print_err`, `log` and `task.scope` need no capability.
- `cap.Wrap` puts an interceptor around a capability, which sees every
  call through it.
- Code without capabilities can only compute, and a `pure` function
  cannot take one (E0334). `sisu audit --caps` counts what a program
  passes around.

Chapter 19 is about testing in depth: suites and tags, the fake of
every capability, a clock that moves only when every task waits,
property tests, snapshots, transcripts and replay tokens.
