# Test a command-line tool

This guide tests a tool by what it prints: a snapshot for a piece of
its output, a transcript for whole sessions at a terminal, and the
review of a change with `--update`. It assumes tests and the command
line, from chapters 9 and 19 of the tutorial.

**See also:** the tutorial's
[chapter 19](../tutorial/19-testing.md#snapshots) (snapshots and
transcripts); [R16](../reference/sisu-command.md) for `sisu test` and
its options; the proposal's
[17, "Snapshots and transcripts"](../../proposal/17-testing.md#snapshots-and-transcripts);
[Parse command-line arguments](command-line.md) for the options of a
tool.

## 1. Make the output a value

Write what the tool prints as functions that return text, and keep
`main` to reading the arguments and printing. This to-do list keeps its
items in `todo.txt`:

<!-- file: examples/guide-test-tool/todo.sisu -->
```sisu
# A to-do list kept in `todo.txt`, tested by a snapshot and a transcript.
from std.test import assert_snapshot

## What the command line can get wrong.
error TodoError:
    Usage
    NoSuchItem(number: int)

## The items of `todo.txt`, none when there is no such file.
def items(fs: Fs) -> list[String]:
    let text = fs.read_to_string("todo.txt") else:
        return []
    return [line.to_string() for line in text.lines()]

## The list as `todo list` prints it.
def listing(todo: list[String]) -> String:
    if todo.is_empty():
        return "nothing to do"
    return ["{i + 1}. {item}" for i, item in enumerate(todo)].join("\n")

def main(sys: System) -> ! {IoError, TodoError}:
    var todo = items(sys.fs)
    match sys.env.args():
        case ["add", item]:
            todo.push(item.to_string())
            print("added {item}")
        case ["done", n]:
            let i = (n.parse_int() ?? 0) - 1
            if i < 0 or i >= len(todo):
                return Err(NoSuchItem(i + 1))
            print("done: {todo.remove(i)}")
        case ["list"]:
            print(listing(todo))
        case _:
            return Err(Usage)
    return sys.fs.write_text("todo.txt", todo.join("\n"))

test "the listing of three items":
    assert_snapshot(listing(["logs", "towels", "water"]))
```

`listing` gives the text of `todo list`, so a test can check it without
running the tool, and `main` returns its errors without `?`, so that a
failed run prints no line of the source.

## 2. Snapshot a piece of the output

`assert_snapshot(value)` of `std.test` compares a value with the
snapshot stored under the test's name. The first run has none, fails,
and shows what it would store; `sisu test --update` stores it, in
`todo.snap` beside the program:

<!-- output: examples/guide-test-tool/first.console -->
```console
$ sisu test todo.sisu
test "the listing of three items" ... FAILED

test "the listing of three items" ... FAILED
panic: there is no snapshot `the listing of three items` yet (`sisu test --update` stores this one):
1. logs
2. towels
3. water
  at test "the listing of three items" (todo.sisu:39:5)
replay: sisu test --replay 1-e85d306a0afd07ed-0.0.1-t0-s0-r0 todo.sisu

0 passed, 1 failed, 0 blocked
$ sisu test --update todo.sisu
test "the listing of three items" ... ok
1 passed, 0 failed, 0 blocked
stored the new snapshot `the listing of three items`:
1. logs
2. towels
3. water
$ sisu test todo.sisu
test "the listing of three items" ... ok
1 passed, 0 failed, 0 blocked
```

Read what `--update` stored before you keep it: from then on the test
passes while the listing looks the same, right or wrong. Keep the
`.snap` file with the code. A value that is not text is stored in its
`Debug` form.

## 3. Write a transcript of a session

A transcript, `tests/NAME.transcript` beside the program, is a session
at a terminal: each `$` line runs `main` with the words after the
tool's name as its arguments, and the lines after it are what the run
prints. The fake `System` it runs on lasts from one `$` line to the
next, so each run sees what the one before wrote, and its file system
starts with the files of `tests/NAME/`; here `tests/todo/todo.txt`
holds two items, `logs` and `towels`:

<!-- file: examples/guide-test-tool/todo.files/tests/todo.transcript -->
```text
$ todo list
1. logs
2. towels
$ todo add water
added water
$ todo done 1
done: logs
$ todo list
1. towels
2. water
$ todo done 7
error: NoSuchItem(number=7) (exit 1)
```

- A run that fails ends with ` (exit N)` on its last line.
- Lines that start with `>`, right after a `$` line and before what
  the run prints, are its standard input.
- The clock is frozen at the start of 1970, `Rand` is seeded and there
  is no network, so a transcript prints the same on every machine.

You need not type the expected lines: write the `$` lines, run `sisu
test --update`, and read the transcript it writes. `sisu test` runs the
snapshot test and the transcript:

<!-- output: examples/guide-test-tool/todo.console -->
```console
$ sisu test todo.sisu
test "the listing of three items" ... ok
transcript "tests/todo.transcript" ... ok
2 passed, 0 failed, 0 blocked
```

## 4. Review a change

When the output changes, both kinds of test fail with a diff: the lines
that start with `-` are what was stored, those with `+` what the tool
prints now. Here the listing's numbers changed from `1.` to `1)`:

<!-- output: examples/guide-test-tool/todo_changed.console -->
```console
$ sisu test todo_changed.sisu
test "the listing of three items" ... FAILED
transcript "tests/todo.transcript" ... FAILED

test "the listing of three items" ... FAILED
panic: the snapshot `the listing of three items` changed (`sisu test --update` accepts the new one):
- 1. logs
- 2. towels
- 3. water
+ 1) logs
+ 2) towels
+ 3) water
  at test "the listing of three items" (todo_changed.sisu:39:5)
replay: sisu test --replay 1-106b62ee9ca69bd4-0.0.1-t0-s0-r0 todo_changed.sisu

transcript "tests/todo.transcript" ... FAILED
what the program printed differs from the transcript (`sisu test --update` accepts it):
  $ todo list
- 1. logs
- 2. towels
+ 1) logs
+ 2) towels
  $ todo add water
  added water
  $ todo done 1
  done: logs
  $ todo list
- 1. towels
- 2. water
+ 1) towels
+ 2) water
  $ todo done 7
  error: NoSuchItem(number=7) (exit 1)

0 passed, 2 failed, 0 blocked
$ sisu test --update todo_changed.sisu
updated the transcript "tests/todo.transcript":
  $ todo list
- 1. logs
- 2. towels
+ 1) logs
+ 2) towels
  $ todo add water
  added water
  $ todo done 1
  done: logs
  $ todo list
- 1. towels
- 2. water
+ 1) towels
+ 2) water
  $ todo done 7
  error: NoSuchItem(number=7) (exit 1)
test "the listing of three items" ... ok
transcript "tests/todo.transcript" ... ok
2 passed, 0 failed, 0 blocked
updated the snapshot `the listing of three items`:
- 1. logs
- 2. towels
- 3. water
+ 1) logs
+ 2) towels
+ 3) water
$ sisu test todo_changed.sisu
test "the listing of three items" ... ok
transcript "tests/todo.transcript" ... ok
2 passed, 0 failed, 0 blocked
```

- If the change is a bug, fix the code and run the tests again.
- If it is what you meant, `sisu test --update` accepts it, showing the
  same diffs as it writes the transcript and the snapshot anew.
- Read each diff before you accept it, and commit the `.snap` file and
  the transcript with the change, so that a reviewer reads the new
  output beside the code that makes it. Accepting every change unread
  turns the tests into a record of whatever the tool did last.
