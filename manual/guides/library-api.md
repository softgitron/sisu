# Design a library's API

This guide shapes the public part of a library by the rules that
Sisu's standard library follows, on a small library of sauna bookings:
its types first, then the functions that touch the world, then tests
that use it as its users will. It assumes packages, traits and
capabilities, from chapters 14, 17 and 18 of the tutorial.

**See also:** the proposal's
[11, "Principles for `std` API design"](../../proposal/11-stdlib.md#principles-for-std-api-design);
the tutorial's [chapter 17](../tutorial/17-modules-and-packages.md)
(`pub`, documentation and publishing) and
[chapter 18](../tutorial/18-capabilities.md) (capabilities);
[R13, Modules](../reference/modules.md) for visibility; the standard
library's source, [`compiler/std`](../../compiler/std/), as worked
examples.

## 1. Write the types: values out, parts private

The library's root module, `src/lib.sisu`, holds its types:

<!-- file: examples/guide-library-api/ledger.pkg/src/lib.sisu -->
```sisu
##! A sauna's ledger of bookings, written to the rules of `std`'s API.
## Why a booking could not be made.
pub error BookingError:
    Taken(hour: int)
    Closed(hour: int)

## A turn in the sauna: who, and at which hour.
@derive(Debug, Eq)
pub struct Booking:
    pub name: String
    pub hour: int

impl Display for Booking:
    def fmt(self, mut f: Formatter):
        f.write("{self.hour}:00 {self.name}")

## The bookings of one evening, from 17:00 to 21:00.
@derive(Debug, Eq)
pub struct Ledger:
    bookings: list[Booking]

    pub def new() -> Ledger:
        return Ledger([])

    ## Books `hour` for `name`, unless it is taken or the sauna is closed.
    pub def book(mut self, name: str, hour: int) -> ! BookingError:
        if hour < 17 or hour > 21:
            return Err(Closed(hour))
        if any([b.hour == hour for b in self.bookings]):
            return Err(Taken(hour))
        self.bookings.push(Booking(name.to_string(), hour))
        return Ok(())

    ## The bookings, as views, in the order they were made.
    pub def iter(self) -> Iter[ref Booking]:
        return self.bookings.iter()

impl Len for Ledger:
    def len(self) -> int:
        return len(self.bookings)
```

- **Give values out, and keep their parts private.** `Ledger`'s list is
  private, so only `book` adds to it, and `book` checks the hour. A
  private field cannot be set from another module, even through a
  default (E0258), so the library gives `Ledger.new()`: a caller who
  cannot build a value by hand cannot build a wrong one.
- **Make allocation visible.** Take views where the library only reads,
  `name: str`, and return owned types where it makes something, so that
  a signature shows what a call allocates.
- **Traverse with iterators over views.** `iter()` gives references to
  the bookings rather than a copied list or a callback: the caller
  decides whether to copy, and iterators compose with `map`, `filter`
  and comprehensions.
- **Derive `Debug` for every type**, so that a failing test or an error
  can show it, and write `Display` where a type has an obvious form as
  text, as a booking's `18:00 Aino`; a ledger has none, so it has no
  `Display`.
- **Use Python's names.** `impl Len` makes `len(ledger)` work. Where
  Python has a name, use it (`get`, `items`, `keys`, `push`, `sorted`),
  and the iterator vocabulary where it has none (`iter`, `map`,
  `filter`, `fold`, `collect`).

## 2. Take the capabilities in, and name each failure

The library's file format is in `src/store.sisu`:

<!-- file: examples/guide-library-api/ledger.pkg/src/store.sisu -->
```sisu
##! The ledger kept in a file, through the file system it is given.
from ledger import BookingError, Ledger

## Why a file is not a ledger.
pub error LoadError:
    BadLine(number: int, text: String)

## Writes `ledger` to `path` on `fs`, a line `HOUR NAME` for each booking.
pub def save(fs: Fs, path: Path, ledger: Ledger) -> ! IoError:
    let lines = ["{b.hour} {b.name}" for b in ledger.iter()]
    return fs.write_text(path, lines.join("\n"))

## The ledger that `save` wrote to `path` on `fs`.
pub def load(
    fs: Fs,
    path: Path,
) -> Ledger ! {IoError, LoadError, BookingError}:
    var ledger = Ledger.new()
    let text = fs.read_to_string(path)?
    for i, line in enumerate(text.lines()):
        let (hour, name) = line.split_once(" ") else:
            return Err(BadLine(i + 1, line.to_string()))
        let h = hour.parse_int() else:
            return Err(BadLine(i + 1, line.to_string()))
        ledger.book(name, h)?
    return Ok(ledger)
```

- **Capabilities in.** A function that touches the world takes the
  capability as its first parameter and touches nothing else. `save`
  and `load` work on any `Fs`: the real one, one restricted to a
  directory, or `Fs.in_memory()`. A library that takes no capability at
  all can only compute, which its signatures show at once, and which
  `sisu audit --caps` in the package's directory lists (step 3).
- **Specific error sets.** Name each way a call can fail, with the data
  that the caller needs to act on it: `Taken(hour)` and `Closed(hour)`
  for a booking, `BadLine(number, text)` for a file. The signature,
  `Ledger ! {IoError, LoadError, BookingError}`, says everything that
  can go wrong, and a caller matches the case it handles and passes on
  the rest. A `String` that says what failed is no error set: no caller
  can match it.

## 3. Test from outside, through the pub items

Put the tests in a module of their own, so that they see what a user
sees: only the `pub` items, with each error named by its set:

<!-- file: examples/guide-library-api/ledger.pkg/src/tests.sisu -->
```sisu
##! The library's tests, of its `pub` items.
from ledger import BookingError, Ledger
from ledger.store import LoadError, load, save

test "a turn is booked once":
    var ledger = Ledger.new()
    assert ledger.book("Aino", 18) == Ok(())
    assert ledger.book("Eero", 18) == Err(BookingError.Taken(18))
    assert ledger.book("Roni", 23) == Err(BookingError.Closed(23))
    assert len(ledger) == 1
    assert ["{b}" for b in ledger.iter()] == ["18:00 Aino"]

test "a ledger comes back from its file"(fs: Fs):
    var ledger = Ledger.new()
    _ = ledger.book("Aino", 18)
    _ = ledger.book("Eero", 20)
    assert save(fs, "ledger.txt", ledger) == Ok(())
    let back = load(fs, "ledger.txt").expect("the file just written")
    assert ["{b}" for b in back.iter()] == ["18:00 Aino", "20:00 Eero"]

test "a line that is not a booking"(fs: Fs):
    _ = fs.write_text("ledger.txt", "18 Aino\nsoon Eero")
    assert load(fs, "ledger.txt") == Err(LoadError.BadLine(2, "soon Eero"))
```

<!-- output: examples/guide-library-api/ledger.console -->
```console
$ sisu test
test "tests/a turn is booked once" ... ok
test "tests/a ledger comes back from its file" ... ok
test "tests/a line that is not a booking" ... ok
3 passed, 0 failed, 0 blocked
$ sisu audit --caps
. function ledger.store.load caps=1 io=direct depth=-
. function ledger.store.save caps=1 io=direct depth=-
. function test "tests/a ledger comes back from its file" caps=1 io=calls depth=-
. function test "tests/a line that is not a booking" caps=1 io=direct depth=-
. use ledger.store.load FsOps.read_to_string depth=-
. use ledger.store.save FsOps.write_text depth=-
. use test "tests/a line that is not a booking" FsOps.write_text depth=-
. summary functions=4 median=1 max=1 uses=3 deepest=- structs=0
```

The audit of the package, the session's last command, lists every
function that takes a capability, its tests among them by their names:
only `save` and `load` of the library do, each through one `Fs` operation,
and `depth=-` says that a library has no `main` to reach them from.

From another module a variant is named through its error set,
`BookingError.Taken(18)`; `from ledger import Taken` is E0204, since a
module's items are its types and functions, not their variants. If a
test needs what the module keeps private, the API probably lacks
something its users will need too.

**Status.** The rules are [11](../../proposal/11-stdlib.md)'s and hold
in stage 0; the linter that 11 says enforces a naming table on `std`
is designed.
