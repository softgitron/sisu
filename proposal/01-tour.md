# 01 - A tour of Sisu

This is the fastest way to get a feel for the language. Every construct
shown here is specified in the later documents; where the syntax is still
an open question, the text says so. Nothing here compiles yet.

## Hello

```sisu
def main():
    print("Hei, maailma!")
```

Files end in `.sisu`. Blocks are indented; a line ending in `:` opens one.
There are no semicolons and no braces. `print` writes a line to the
process's standard output, which is one of the very few things a Sisu
program can do without being handed a capability (see
[Capabilities](#capabilities) below for why that matters).

## Bindings and basic types

```sisu
let answer = 42                 # immutable, type inferred as int
var count = 0                   # mutable
count += 1

let ratio: f64 = 0.75           # explicit type
let name = "Sisu"               # str: an immutable UTF-8 string view
var greeting = String()     # String: an owned, growable string
greeting.push("Hei ")
greeting.push(name)

let big = 1_000_000             # underscores for readability
let byte: u8 = 0xFF
let flags = 0b1010
let c = 'ä'                     # char: one Unicode scalar value
let a, b = 1, 2                 # tuple unpacking
```

Integers are `int` (64-bit signed, the default and the type of all indices
and lengths), `uint`, and the sized `i8`..`i128`, `u8`..`u128`. Floats are
`f32` and `f64`. Arithmetic traps on overflow in every build mode; wrapping
and saturating arithmetic are spelled out: `a +% b`, `a +| b`. `//` is
floor division on integers and `/` is division on floats; dividing two
integers with `/` is a type error. Lossless widening (`i32` to `int`,
`f32` to `f64`) is implicit; anything lossy must be written
(`x.truncate_to[u8]()`, `x.round_to[int]()`).

Booleans are `True` and `False` and use the words `and`, `or`, `not`.
There is no truthiness: `if xs:` does not compile, `if not xs.is_empty():`
does.

## Functions

```sisu
def add(a: int, b: int) -> int:
    return a + b

def greet(name: str, excited: bool = False):   # default argument; no -> means unit
    let mark = if excited: "!" else: "."
    print("Hello, {name}{mark}")                # string interpolation

greet("Roni")
greet("Roni", excited=True)     # any argument can be passed by name
```

Signatures are always fully written out. Locals are inferred. A function
returns a value only with `return`; reaching the end of a non-unit
function without one is a compile error. `if` and `match` can be used as
expressions when each branch is a single expression.

```sisu
def divmod(a: int, b: int) -> (q: int, r: int):   # labelled results
    return (a // b, a % b)

let res = divmod(17, 5)
print(res.q)                    # 3
let q, r = divmod(17, 5)        # or unpack
```

## Structs, enums, pattern matching

```sisu
from std.math import PI

@derive(Copy, Debug)
struct Point:
    x: f64
    y: f64

    def origin() -> Point:                   # no self: Point.origin()
        return Point(x=0.0, y=0.0)

    def length(self) -> f64:
        return (self.x ** 2 + self.y ** 2).sqrt()

    def scale(mut self, k: f64):
        self.x *= k
        self.y *= k

enum Figure:
    Circle(radius: f64)
    Rect(w: f64, h: f64)
    Empty

def area(s: Figure) -> f64:
    match s:
        case Circle(radius):
            return PI * radius ** 2
        case Rect(w, h) if w == h:           # guard
            return w * w
        case Rect(w, h):
            return w * h
        case Empty:
            return 0.0
```

`match` must be exhaustive; the compiler lists the missing cases. Patterns
nest, destructure, and bind: `Some(Rect(w, _))`, `[first, *rest]`,
`1 | 2 | 3`, `Point(x=0, y=y)`. A lowercase name in a pattern always binds;
a capitalised name is always a variant or constant.

## No null

```sisu
def find_user(users: list[User], name: str) -> ref User? from users:
    for u in users:
        if u.name == name:
            return Some(u)
    return None

let u = find_user(users, "roni")
let age = u?.age ?? 0                        # optional chaining and default
if let Some(user) = u:                       # pattern binding
    print(user.name)
let Some(user) = u else:                     # bind or bail out
    return
```

`T?` is `Option[T]`. There is no `null`, no `nil`, no undefined. `ref`
binds tighter than `?`, so `ref User?` is an optional reference, which is
the common case.

## Errors are values

```sisu
error ParseError:
    UnexpectedEnd
    BadDigit(at: int, found: char)

def parse_int(text: str) -> int ! ParseError:
    if text.is_empty():
        return Err(UnexpectedEnd)
    var n = 0
    for i, c in enumerate(text.chars()):
        let d = c.to_digit() else:
            return Err(BadDigit(at=i, found=c))
        n = n * 10 + d
    return Ok(n)

def parse_pair(text: str) -> (int, int) ! ParseError:
    let a, b = text.split_once(",") else:
        return Err(UnexpectedEnd)
    return Ok((parse_int(a)?, parse_int(b)?))   # ? propagates

def load_port(fs: Fs, path: Path) -> int !:     # trailing ! alone: error set inferred
    let text = fs.read_to_string(path)?         # IoError joins the inferred set
    let port = parse_int(text.trim())?          # ParseError joins it too
    return Ok(port)

# handling
let port = load_port(fs, "port.txt") else: 8080            # fallback expression
let port = load_port(fs, "port.txt") else as e: fallback_for(e)
match parse_int(input):
    case Ok(n):
        show(n)
    case Err(BadDigit(at, found)):
        print("bad digit {found} at {at}")
    case Err(UnexpectedEnd):
        print("empty input")
```

`T ! E` is `Result[T, E]` where `E` is an *error set*: a sum type whose
variants can carry data and which is a subtype of any error set that
includes its variants. So `ParseError` coerces to `{ParseError, IoError}`
without conversion boilerplate. A trailing `!` with no set infers the set
from the body; the compiler and documentation tool show the inferred set,
and the public-API linter asks you to spell it out at package boundaries.

Panics are different: an out-of-bounds index, an overflow, a failed
`assert`, or an explicit `panic("...")` is a bug. It unwinds the current
task to its enclosing scope; it is not caught by `match`. See
[05-errors.md](05-errors.md).

## Ownership without lifetimes

```sisu
def summarise(users: list[User]) -> String:          # users is borrowed (read)
    var out = String()
    for u in users:
        out.push("{u.name}\n")
    return out                                       # out is moved to the caller

def add_user(mut users: list[User], own u: User):    # users borrowed mutably, u consumed
    users.push(u)

let roni = User(name="Roni", age=30)
add_user(mut team, roni)                             # mut is visible at the call site
# roni is gone here: it was moved into team. Using it is a compile error.

def oldest(users: list[User]) -> ref User:           # returns a borrow of its argument
    return users.iter().max_by(it.age).expect("non-empty")
```

Three parameter conventions, written on the parameter, not on the type:

| Convention | Caller writes | Callee may |
|---|---|---|
| (none) `x: T` | `f(v)` | read `x` |
| `mut x: T` | `f(mut v)` | read and modify `x` in place |
| `own x: T` | `f(v)` (moves) | do anything, including destroy or return it |

A function can return a reference (`ref T`, `ref mut T`) to something it was
given; the compiler ties it to the borrowed parameter. When there is more
than one candidate, say which: `-> ref str from a, b`. There are no named
lifetimes anywhere. References can be stored only in types declared as
`view struct`, which carry one implicit lifetime and cannot escape into the
heap. Everything else is a value, and values are not aliased. The full
model, including what you give up, is in
[04-memory-and-ownership.md](04-memory-and-ownership.md).

```sisu
let a = [1, 2, 3]
let b = a                      # move: a is no longer usable
let c = b.copy()               # explicit deep copy
let p = Point(x=1.0, y=2.0)
let q = p                      # Point is Copy, so this copies
```

Shared ownership is a type you ask for:

```sisu
let config = Shared(Config.default())    # reference-counted, immutable
let c2 = config.copy()                    # cheap: bumps the count; copies are always written
let counter = Shared(Mutex(0))    # shared *and* mutable: needs a lock
counter.lock().value += 1
```

## Traits and generics

```sisu
trait Shape:
    def area(self) -> f64                              # required
    def describe(self) -> String:                      # provided
        return "a shape of area {self.area():.2f}"

@derive(Copy, Debug)
struct Circle:
    radius: f64

impl Shape for Circle:
    def area(self) -> f64:
        return PI * self.radius ** 2

def largest[S: Shape](shapes: list[S]) -> ref S:
    return shapes.iter().max_by(it.area()).expect("non-empty")

def print_all(shapes: list[Box[dyn Shape]]):      # dynamic dispatch is spelled out
    for s in shapes:
        print(s.describe())

trait Container(Len):                              # supertrait: a Container is also Len
    type Item                                       # associated type
    def get(self, i: int) -> ref Item?

def sum_of[C: Container](c: C) -> int where C.Item: Into[int]:
    var total = 0
    for i in range(len(c)):
        total += c.get(i)?.into() ?? 0
    return total
```

Generic code is type-checked once, at its definition, against the bounds.
Instantiation cannot fail with an error inside the library. Generics use
square brackets, as Python 3.12 does.

```sisu
@derive(Eq, Ord, Hash, Debug, Copy)
struct Version:
    major: int
    minor: int
    patch: int
```

`@derive` is not a macro. It is a compile-time function that inspects the
struct's fields through reflection and emits the implementation. You can
write your own.

## Collections and iteration

```sisu
let primes = [2, 3, 5, 7, 11]                     # list[int]
let fixed: Array[int, 3] = [1, 2, 3]              # fixed size when the type says so
let ages = {"roni": 30, "aino": 27}               # dict[str, int]
let seen = {"a", "b"}                             # set[str]

let big = [p * p for p in primes if p > 3]        # comprehension
let lazy = (p * p for p in primes)                # generator expression: Iter[int]
let same = primes.iter().filter(it > 3).map(it * it).collect[list[int]]()

for name, age in ages:
    print("{name} is {age}")
for i in range(10):
    pass
for i in range(0, 11, 2):
    pass

def fib() -> Iter[int]:                           # yield makes a generator
    var a, b = 0, 1
    while True:
        yield a
        a, b = b, a + b

let first_ten = fib().take(10).collect[list[int]]()
```

Closures are `x -> expr` inline, or `it` for a one-parameter closure
(`xs.map(it * 2)`). A closure with a multi-line body is a trailing `do`
block on the call that receives it, so `xs.each do x:` and `task.scope do s:`
read as control structures.

## Structured, colourless concurrency

```sisu
def fetch_all(net: Net, urls: list[Url]) -> list[bytes] ! {HttpError, TaskError}:
    return task.scope do s:
        let handles = [s.spawn(() -> http.get(net, url)) for url in urls]
        var results = list.with_capacity(len(handles))
        for h in handles:
            results.push(h.join()?)      # join flattens: the task's error set plus TaskError
        return Ok(results)
```

There is no `async` and no `await`. `http.get` is an ordinary function that
blocks the *task*, not the thread; the runtime multiplexes tasks over OS
threads. A task can only be spawned inside a `scope`, and the scope does not
return until every task in it has finished or been cancelled, which is why
the tasks may borrow `net` and `urls` from the enclosing frame. Cancelling a
scope cancels all of its children; there is no `context` object to thread
through.

Data-race freedom is a type-system property: a value may be sent to another
task only if its type is `Send`, and shared between tasks only if it is
`Sync`. These are inferred structurally, so you only notice them when you try
to share something you should not.

```sisu
let hits = Shared(Atomic(0))
task.scope do s:
    for _ in range(8):
        s.spawn(() -> hits.fetch_add(1))
print(hits.load())        # 8

let tx, rx = channel[Job](capacity=64)
task.scope do s:
    s.spawn do:
        for job in rx:
            handle(job)
    for j in jobs:
        tx.send(j).expect("the receiver lives until the scope ends")
    tx.close()
```

## Capabilities

```sisu
def main(sys: System) -> ! AppError:
    let fs = sys.fs.restrict_to("./data")     # attenuate: this Fs cannot leave ./data
    let net = sys.net.only_hosts(["api.example.com"])
    let clock = sys.clock

    let cfg = load_config(fs, "config.toml")?
    return run_server(net, clock, cfg)
```

`System` is handed to `main` and nobody else. Anything that touches the
world outside the process takes a capability parameter: `Fs`, `Net`,
`Clock`, `Rand`, `Env`, `Proc`. A library that is not given `Net` cannot
make a network request, not by policy but because there is no function it
could call. Capabilities can be narrowed before being passed on. Tests pass
in-memory fakes. The compiler enforces that `unsafe` code and foreign
function calls, which could bypass this, are only allowed in packages your
manifest explicitly trusts.

## Contracts

```sisu
def binary_search[T: Ord](items: Slice[T], target: T) -> int?:
    requires items.is_sorted()
    ensures result.map(items[it] == target) ?? not items.contains(target)
    ...

struct Percent:
    value: int
    invariant 0 <= self.value <= 100
```

Preconditions, postconditions and type invariants are the first lines of a
body, documented automatically, checked at runtime in debug and test
builds, and available to static analysis. They are not an optional add-on
library.

## Compile-time evaluation

```sisu
def table[const N: int]() -> Array[int, N]:
    var t = Array.filled(0)
    for i in range(N):
        t[i] = i * i
    return t

const SQUARES: Array[int, 256] = comptime table[256]()   # computed by the compiler, baked in

def type_name[T](x: T) -> str:
    return comptime reflect.name_of(T)     # reflection is a compile-time API
```

Ordinary Sisu code runs at compile time when asked. There is no separate
macro language. Compile-time code cannot perform I/O and has a fuel limit,
so builds stay fast and reproducible.

## Tests and documentation

```sisu
## Parses a decimal integer.
##
## ```
## assert parse_int("42") == Ok(42)
## ```
def parse_int(text: str) -> int ! ParseError: ...

test "rejects empty input":
    assert parse_int("") == Err(UnexpectedEnd)

test "round-trips any int":
    check(arbitrary[int].filter(it >= 0), n -> parse_int("{n}") == Ok(n))   # property test
```

`assert` shows the value of every sub-expression when it fails. `sisu
test` runs tests, doc examples and property tests, each with its own fake
file system and clock. `sisu doc` renders the `##` comments. `sisu fmt`
has no options. See [17-testing.md](17-testing.md).

## Modules and packages

```sisu
from std.collections import SortedDict
import std.json                          # core library
import http                              # a dependency from sisu.toml (the blessed std.x.http)
from weather.parser import parse_int, ParseError   # this package's own module
```

A file is a module. A directory is a module containing its files. Items are
private unless marked `pub`. Packages cannot depend on each other in a cycle.

```toml
# sisu.toml
[package]
name = "weather"
version = "0.3.1"
edition = "2027"

[dependencies]
http = "1.2"
toml = "1"

[trust]
unsafe = ["http"]       # http may use unsafe/FFI; nothing else may
```

Publishing a version whose public API changed incompatibly without bumping
the major version is rejected by the registry, because the tool diffs the
API. Package contents are content-addressed and recorded in a transparency
log.

## Interop

```sisu
extern "C":
    def strlen(s: ptr[c_char]) -> c_size

@export("sisu_checksum")
def checksum(data: ptr[u8], len: c_size) -> u32: ...

import c "zlib.h" as zlib                 # header import, no bindings to write
```

Calling C is `unsafe`, and `unsafe` is a block, a package-level permission,
and a line in `sisu audit`'s output.

## What you did not see

No classes, no inheritance, no `null`, no exceptions, no `async`, no macros,
no header files, no truthiness, no implicit conversions that lose
information, no undefined behaviour in safe code, no global mutable state
without a lock, no build scripts, no formatter options. Each of those
absences is a decision with a rationale in [15-decision-log.md](15-decision-log.md).
