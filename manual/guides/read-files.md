# Read and write files

This guide reads a file, handles one that is missing, writes, adds to
and replaces a file, walks a directory, and tests the code with a file
system in memory. It assumes that you know how `main` receives its
capabilities, from [chapter 9](../tutorial/09-word-counter.md) of the
tutorial.

**See also:** [chapter 18](../tutorial/18-capabilities.md) (passing
and narrowing capabilities) and
[chapter 21](../tutorial/21-standard-library.md#files-and-paths)
(`std.fs`) of the tutorial; the source of `std.fs`,
[`compiler/std/fs.sisu`](../../compiler/std/fs.sisu), which holds its
documentation; the proposal's
[11, Standard library](../../proposal/11-stdlib.md).

## 1. Read a whole file

A program reaches files through `sys.fs`, the file system that `main`
receives; a function that needs files takes an `Fs` parameter.
`read_to_string(path)` gives a file's text and `read(path)` its bytes.
This program prints the lines of `notes/sauna.txt` and its size:

<!-- file: examples/guide-read-files/read.sisu -->
```sisu
# Reads a whole file, through the file system that `main` receives.
def main(sys: System) -> ! IoError:
    let text = sys.fs.read_to_string("notes/sauna.txt")?
    for line in text.lines():
        print("- {line}")
    let data = sys.fs.read("notes/sauna.txt")?
    print("{len(data)} bytes")
    return Ok(())
```

<!-- output: examples/guide-read-files/read.out -->
```text
- Heat the sauna by six.
- Buy birch logs.
39 bytes
```

Each operation returns a result, `T ! IoError`, and `?` passes an error
on to the caller, here out of `main`. A path is written with `/` on
every system, and a text literal becomes a `Path` where one is
expected.

## 2. Handle a missing file

A missing file is the error `NotFound(path)` of `IoError`, so a program
that can do without the file matches it. This one has no
`settings.txt`:

<!-- file: examples/guide-read-files/missing.sisu -->
```sisu
# A missing file is an `IoError`, and `match` tells it from the others.
def main(sys: System):
    match sys.fs.read_to_string("settings.txt"):
        case Ok(text):
            print(text)
        case Err(NotFound(path)):
            print("no {path}, so the defaults")
        case Err(e):
            print("cannot read the settings: {e}")
```

<!-- output: examples/guide-read-files/missing.out -->
```text
no settings.txt, so the defaults
```

The other errors are `PermissionDenied`, `AlreadyExists`, `InvalidData`
(a file that is not UTF-8, from `read_to_string`), `Interrupted`,
`Revoked` and `Other`. `sys.fs.exists(path)` asks first, but the answer
can be out of date by the time the file is read, so matching the error
is the safer test.

## 3. Write a file, add to it and replace it

`write_text(path, text)` creates or replaces a file, and `write(path,
data)` does the same with bytes. `create_dir` makes a directory and its
missing parents. This program writes a log, adds a line to it, and
prints the result:

<!-- file: examples/guide-read-files/write.sisu -->
```sisu
# Writes a file in a new directory, makes it durable, and adds to it.
import std.fs

def main(sys: System) -> ! IoError:
    sys.fs.create_dir("log/2026")?
    sys.fs.write_text("log/2026/sauna.txt", "sauna at six\n")?
    sys.fs.sync("log/2026/sauna.txt")?
    fs.append(sys.fs, "log/2026/sauna.txt", "swim at seven\n".encode())?
    let text = sys.fs.read_to_string("log/2026/sauna.txt")?
    print(text.trim())
    print(sys.fs.list_dir("log")?)
    return Ok(())
```

<!-- output: examples/guide-read-files/write.out -->
```text
sauna at six
swim at seven
["log/2026"]
```

- `sync(path)` makes what was written durable: a crash after it cannot
  lose the file's contents. Sync the directory too, `sync("log/2026")`,
  when the file is new and its name must survive a crash.
- `fs.append` of `std.fs` adds bytes at the end of a file, and creates
  the file when it is missing.
- `list_dir` gives a directory's entries, sorted, as paths that start
  with the directory.

A program that replaces a file a reader may open at any moment, such as
a settings file, writes the new text beside it and then gives it the
old name with `rename(source, target)`. The rename takes the place of
the file already there at once, so a reader finds the old text or the
new one, never part of either:

<!-- file: examples/guide-read-files/save.sisu -->
```sisu
# Replaces a file whole: a reader finds the old text or the new one.

## Writes `text` beside `path`, then renames it over `path`.
def save(files: Fs, path: Path, text: str) -> ! IoError:
    let part = Path.new("{path}.part")
    files.write_text(part, text)?
    files.sync(part)?
    files.rename(part, path)?
    return Ok(())

test "the new text replaces the old":
    let files = Fs.in_memory({"plan.txt": "sauna at six"})
    assert save(files, "plan.txt", "sauna at seven") == Ok(())
    assert files.read_to_string("plan.txt") == Ok("sauna at seven")
    assert not files.exists("plan.txt.part")

test "a rename of a file that is not there"(files: Fs):
    assert files.rename("old.txt", "new.txt") == Err(NotFound("old.txt"))
```

<!-- output: examples/guide-read-files/save.test -->
```text
test "the new text replaces the old" ... ok
test "a rename of a file that is not there" ... ok
2 passed, 0 failed, 0 blocked
```

A failed rename names `source` when it is not there, as the second test
shows, and `target` for most other failures. To make the new name
survive a crash, sync its directory after the rename.

## 4. Walk a directory

`fs.walk(files, dir)` of `std.fs` gives every path under a directory,
each directory before what it holds, and `metadata(path)` says whether
a path is a directory and how long a file is. This program lists the
directories and the `.txt` files under `notes`, which also holds
`ideas.md`:

<!-- file: examples/guide-read-files/walk.sisu -->
```sisu
# Walks a directory, and gives the size of each `.txt` file under it.
import std.fs

def main(sys: System) -> ! IoError:
    for path in fs.walk(sys.fs, "notes")?:
        let meta = sys.fs.metadata(path)?
        if meta.is_dir:
            print("{path}/")
        elif fs.extension(path) == Some("txt"):
            print("{path}: {meta.len} bytes")
    return Ok(())
```

<!-- output: examples/guide-read-files/walk.out -->
```text
notes/2026/
notes/2026/lake.txt: 15 bytes
notes/sauna.txt: 39 bytes
```

`fs.extension(path)` is the text after the file name's last `.`;
`fs.stem`, `fs.file_name`, `fs.parent` and `fs.join` take a path apart
and put one together, with no capability. When you import `std.fs`, do
not name a parameter `fs`: it would hide the module.

## 5. Test with a file system in memory

Code that takes an `Fs` takes `Fs.in_memory(files)` in a test: a file
system that holds the files you give it, by path, and nothing else. A
test that names `files: Fs` as a parameter receives an empty one. After
the call, read the fake back to see what the code wrote:

<!-- file: examples/guide-read-files/report.sisu -->
```sisu
# Code that takes `Fs` is tested with a file system in memory.
import std.fs

## Writes `report.txt`: a line `PATH: N lines` for each `.txt` file
## under `dir`.
def write_report(files: Fs, dir: Path) -> ! IoError:
    var lines: list[String] = []
    for path in fs.walk(files, dir)?:
        if fs.extension(path) == Some("txt"):
            let text = files.read_to_string(path)?
            lines.push("{path}: {text.lines().count()} lines")
    return files.write_text("report.txt", lines.join("\n"))

test "a line for each note":
    let files = Fs.in_memory(
        {
            "notes/sauna.txt": "Heat the sauna.\nBuy logs.",
            "notes/ideas.md": "# Ideas",
        }
    )
    assert write_report(files, "notes") == Ok(())
    let written = files.read_to_string("report.txt")
    assert written == Ok("notes/sauna.txt: 2 lines")

test "no notes at all"(files: Fs):
    assert write_report(files, "notes") == Err(NotFound("notes"))
    assert not files.exists("report.txt")
```

<!-- output: examples/guide-read-files/report.test -->
```text
test "a line for each note" ... ok
test "no notes at all" ... ok
2 passed, 0 failed, 0 blocked
```

Each test gets its own file system, so what one test writes no other
test sees, and the tests never touch the disk. The second test checks
the missing case of step 2 without any setup.
