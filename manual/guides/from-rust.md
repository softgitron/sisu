# Coming from Rust

This guide carries a Rust programmer's habits over to Sisu, one idea at
a time: errors, borrows without lifetimes, shared ownership, tasks in
place of `async`, and compile-time code in place of macros, and ends
with the spellings that differ. It assumes that you read Rust well and
have met Sisu's basics, in chapters 1 to 13 of the tutorial.

**See also:** [R19, False friends](../reference/false-friends.md#rust-habits),
"Rust habits", with the diagnostic of each; [R7, Ownership and
borrowing](../reference/ownership.md) for the rules; the proposal's
[04, Memory and ownership](../../proposal/04-memory-and-ownership.md),
which compares the model with Rust's at each step.

## 1. Write Result as T ! E

`Result<T, E>` is `T ! E`, and `?` returns early as it does in Rust.
The error type is an error set, a closed list of variants with data;
there is no `From` to write, since a function that can fail in several
ways names the union, `! {IoError, PortError}`, and an error set is a
subtype of any union that holds it. This program parses ports and stops
at the first that is not one:

<!-- file: examples/guide-from-rust/ports.sisu -->
```sisu
# Rust's `Result<u16, PortError>` and `?`: an error set and `T ! E`.

## Why a text is not a port.
error PortError:
    NotANumber(text: String)
    OutOfRange(value: int)

## The port that `text` names.
def port(text: str) -> int ! PortError:
    let n = text.parse_int() else:
        return Err(NotANumber(text.to_string()))
    if n < 1 or n > 65535:
        return Err(OutOfRange(n))
    return Ok(n)

## The ports that `texts` name, or the first error.
def ports(texts: list[str]) -> list[int] ! PortError:
    var out: list[int] = []
    for text in texts:
        out.push(port(text)?)
    return Ok(out)

def main():
    print(ports(["80", "443"]))
    print(ports(["80", "http", "99999"]))
```

<!-- output: examples/guide-from-rust/ports.out -->
```text
Ok([80, 443])
Err(NotANumber(text="http"))
```

`let ... else:` is Rust's `let ... else`, for a result as for an
option. Panics are bugs, as in Rust, and `main` that returns an error
exits with status 1.

## 2. Borrow without lifetimes

A parameter borrows by default, read-only; `mut` borrows to change and
`own` takes ownership, each written in the signature, and the call
writes `mut` where the callee changes the value. References are second
class: they are parameters, results and the fields of a `view struct`,
never stored in an ordinary struct or a collection that could outlive
them, so no lifetime is ever written:

<!-- file: examples/guide-from-rust/borrows.sisu -->
```sisu
# Borrows without lifetimes: a parameter borrows, and a result that
# borrows says from what.

## The longer of `a` and `b`, which it borrows from.
def longer(a: str, b: str) -> str from a, b:
    return if len(a) >= len(b): a else: b

## A word of a text, and where it starts: a view into the text.
view struct Word:
    text: str
    start: int

## The words of `text`, each viewing the text.
def words(text: str) -> list[Word]:
    var out: list[Word] = []
    var start = 0
    for part in text.split(" "):
        out.push(Word(part, start))
        start += len(part) + 1
    return out

## Adds `name` to `names`, which belong to the caller.
def add(mut names: list[String], name: str):
    names.push(name.to_string())

def main():
    print(longer("sauna", "sisu"))
    for w in words("hot and dry"):
        print("{w.text} at {w.start}")
    var names: list[String] = []
    add(mut names, "Aino")
    print(names)
```

<!-- output: examples/guide-from-rust/borrows.out -->
```text
sauna
hot at 0
and at 4
dry at 8
["Aino"]
```

- A result that borrows says from what, `-> str from a, b`, where Rust
  writes `'a`; with one borrowed parameter, or `self`, the compiler
  works it out, as Rust's elision does.
- A `view struct` is a struct with a lifetime parameter: it may hold
  borrows, and it cannot outlive what they borrow.
- `.clone()` is `.copy()`, and `#[derive(Clone, Copy)]` is
  `@derive(Copy)`: a type is moved unless it is `Copy`.

## 3. Share with Shared, Cell and Weak

`Rc<T>` is `Shared[T]`, whose `.copy()` counts another owner, and
`RefCell<T>` is `Cell[T]`, whose value is replaced rather than borrowed
out, so there is no `borrow_mut` to panic. `Weak` is `Weak`, made with
`Shared.downgrade`:

<!-- file: examples/guide-from-rust/shared.sisu -->
```sisu
# Rust's `Rc<RefCell<T>>` is a `Shared` value with a `Cell` in it.

## A sauna that several bookings point to, and how many use it.
struct Sauna:
    name: String
    guests: Cell[int]

## A booking, which shares the sauna with the others.
struct Booking:
    who: String
    sauna: Shared[Sauna]

def main():
    let kota = Shared(Sauna("Kota", Cell(0)))
    let aino = Booking("Aino", kota.copy())
    let eero = Booking("Eero", kota.copy())
    for b in [aino, eero]:
        b.sauna.guests.set(b.sauna.guests.get() + 1)
    print("{kota.name}: {kota.guests.get()} guests")
    let note = Shared.downgrade(kota)
    print(note.upgrade().is_some())
```

<!-- output: examples/guide-from-rust/shared.out -->
```text
Kota: 2 guests
True
```

Between tasks, `Arc<Mutex<T>>` is a `Mutex[T]` that the tasks borrow
from the frame of their scope, and a graph whose nodes point at each
other is an `Arena[T]` with `Key[T]`s, as chapter 13 shows.

## 4. Replace async with tasks

Sisu has no `async` and no `.await`: a call that waits blocks its task,
and the other tasks run meanwhile, so a function is never of two
colours. `task.scope` runs tasks side by side and waits for every one
of them, as a structured `join!` would:

<!-- file: examples/guide-from-rust/tasks.sisu -->
```sisu
# Rust's `async` and `.await`: tasks in a scope, with no colour.

## The temperature of `sauna` after `minutes` on `clock`.
def heat(clock: Clock, sauna: str, minutes: int) -> String ! TaskError:
    clock.sleep(Duration.minutes(minutes))?
    return Ok("{sauna} is warm after {minutes} minutes")

def main() -> ! TaskError:
    let clock = Clock.fake()
    let start = clock.now()
    let both = task.scope do s:
        let kota = s.spawn(() -> heat(clock, "Kota", 40))
        let ranta = s.spawn(() -> heat(clock, "Ranta", 25))
        return Ok([kota.join()?, ranta.join()?])
    for line in both?:
        print(line)
    print("{(clock.now() - start).as_seconds() // 60} minutes in all")
    return Ok(())
```

<!-- output: examples/guide-from-rust/tasks.out -->
```text
Kota is warm after 40 minutes
Ranta is warm after 25 minutes
40 minutes in all
```

The example sleeps on the test clock, `Clock.fake()`, so that it runs
at once; with `sys.clock` the same code takes forty real minutes. A
task may borrow what the frame around its scope owns, since the scope
outlives it, so there is no `'static` bound and no `move` to write.

## 5. Replace macros with compile-time code

`#[derive(...)]` is `@derive(...)`, and a trait's own derive is a
`comptime def derive` over reflection. A `macro_rules!` that computes a
value is a `comptime def` called with `comptime`, and `println!` is
`print` with interpolation:

<!-- file: examples/guide-from-rust/macros.sisu -->
```sisu
# Rust's macros: derives, compile-time functions, and `print`.

## A booking, with `Debug` and `Eq` written by their derives.
@derive(Debug, Eq)
struct Booking:
    name: String
    hour: int

## The squares of 0 to `n - 1`, computed while compiling.
comptime def squares(n: int) -> list[int]:
    var out: list[int] = []
    for i in range(n):
        out.push(i * i)
    return out

const SQUARES: list[int] = comptime squares(6)

def main():
    let b = Booking("Aino", 18)
    print("{b:?}")
    print(b == Booking("Aino", 18))
    print(SQUARES)
```

<!-- output: examples/guide-from-rust/macros.out -->
```text
Booking(name="Aino", hour=18)
True
[0, 1, 4, 9, 16, 25]
```

Compile-time code cannot declare a new item or take apart tokens, so a
macro that writes syntax has no direct port: write the code, or a
derive. See [Write a derive](derive.md).

## 6. Check the spellings that differ

Much of Rust's syntax that Sisu shares means the same; the rest does
not compile, and `sisu check` names it:

<!-- file: examples/guide-from-rust/habits.sisu -->
```sisu
# Three Rust habits, which `sisu check` refuses.
def main():
    let names = ["Aino", "Eero"]
    let copied = names.clone()
    let ratio = len(copied) as f64
    print(ratio)
```

<!-- output: examples/guide-from-rust/habits.console -->
```console
$ sisu check habits.sisu
error[E0304]: `list[str]` has no method `clone`
  --> habits.sisu:4:24
  |
4 |     let copied = names.clone()
  |                        ^^^^^
  = fix: Sisu spells it this way: `copy`

error[E0117]: unexpected `as` after the end of the statement
  --> habits.sisu:5:29
  |
5 |     let ratio = len(copied) as f64
  |                             ^^
  = fix: Sisu writes it `.to_f64()`: `.to_f64()`

```

Each names the Sisu spelling as a fix: `.copy()` for the first, and
`.to_f64()` for the cast.
The table of R19's "Rust habits" lists every such spelling with its
code, and most get the same kind of fix: `let mut` is `var`, `&&` is
`and`, `i++` is `i += 1`, `x as f64` is `x.to_f64()`, `// note` is
`# note`, `=>` in a match is `case ...:`, `0..3` is `range(3)`, `Vec<T>`
is `list[T]`, `println!` is `print`, `pub(crate)` is `pub(package)`,
and `7 / 2` on integers is `7 // 2`. A cast inside a call's parentheses
and a range in a `for` header get no fix yet.
One spelling compiles and means something else: `-7 % 3` is 2, as in
Python, where Rust gives -1.
