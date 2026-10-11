# 01 - A tour of Sisu

This is the fastest way to get a feel for the language. Every construct shown
here is specified in the later documents; where the syntax is still an open
question, the text says so. Nothing here compiles yet.

## Hello

```sisu
fn main() {
    println("Hello, world!")
}
```

Files end in `.sisu`. There are no semicolons; a newline ends a statement.
`println` writes to the process's standard output, which is one of the very
few things a Sisu program can do without being handed a capability (see
[Capabilities](#capabilities) below for why that matters).

## Bindings and basic types

```sisu
let answer = 42                 // immutable, type inferred as int
var count = 0                   // mutable
count += 1

let ratio: f64 = 0.75           // explicit type
let name = "Sisu"               // str: an immutable UTF-8 string view
var greeting = String.new()     // String: an owned, growable string
greeting.push("Hei ")
greeting.push(name)

let big = 1_000_000             // underscores for readability
let byte: u8 = 0xFF
let flags = 0b1010
let c = 'ä'                     // char: one Unicode scalar value
```

Integers are `int` (64-bit signed, the default and the type of all indices
and lengths), `uint`, and the sized `i8`..`i128`, `u8`..`u128`. Floats are
`f32` and `f64`. Arithmetic traps on overflow in every build mode; wrapping
and saturating arithmetic are spelled out: `a +% b`, `a +| b`. Lossless
widening (`i32` to `int`, `f32` to `f64`) is implicit; anything lossy must be
written (`x.truncate_to[u8]()`, `x.round_to[int]()`).

Booleans use the words `and`, `or`, `not`. `!` is reserved for error types
(below), so there is no `!x`.

## Functions

```sisu
fn add(a: int, b: int) -> int {
    a + b                        // last expression is the value
}

fn greet(name: str, excited: bool = false) {   // default argument, no return type = unit
    let mark = if excited { "!" } else { "." }
    println("Hello, {name}{mark}")             // string interpolation
}

greet("Roni")
greet("Roni", excited: true)     // any argument can be passed by name
```

Signatures are always fully written out. Locals are inferred. `if`, `match`
and blocks are expressions; `return` exists for early exits.

## Structs, enums, pattern matching

```sisu
struct Point {
    x: f64
    y: f64
}

impl Point {
    fn origin() -> Point { Point { x: 0.0, y: 0.0 } }
    fn length(self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }
    fn scale(mut self, k: f64) { self.x *= k; self.y *= k }
}

enum Shape {
    Circle { radius: f64 }
    Rect { w: f64, h: f64 }
    Empty
}

fn area(s: Shape) -> f64 {
    match s {
        Circle { radius } -> PI * radius * radius
        Rect { w, h } if w == h -> w * w        // guard
        Rect { w, h } -> w * h
        Empty -> 0.0
    }
}
```

`match` must be exhaustive; the compiler lists the missing cases. Patterns
nest, destructure, and bind: `Some(Rect { w, .. })`, `[first, ..rest]`,
`n @ 1..=9`, `A | B`.

## No null

```sisu
fn find_user(users: List[User], name: str) -> ref User? {
    for u in users {
        if u.name == name { return Some(u) }
    }
    None
}

let u = find_user(users, "roni")
let age = u?.age ?? 0                        // optional chaining and default
if let Some(user) = u { println(user.name) } // pattern binding
let Some(user) = u else { return }           // bind or bail out
```

`T?` is `Option[T]`. There is no `null`, no `nil`, no undefined. The `?`
suffix on a type binds to the type immediately before it, so `ref User?`
is an optional reference, which is the common case.

## Errors are values

```sisu
error ParseError {
    UnexpectedEnd
    BadDigit { at: int, found: char }
}

fn parse_int(text: str) -> int ! ParseError {
    if text.is_empty() { return Err(UnexpectedEnd) }
    var n = 0
    for (i, c) in text.chars().enumerate() {
        let d = c.to_digit() else { return Err(BadDigit { at: i, found: c }) }
        n = n * 10 + d
    }
    Ok(n)
}

fn parse_pair(text: str) -> (int, int) ! ParseError {
    let (a, b) = text.split_once(",") else { return Err(UnexpectedEnd) }
    Ok((parse_int(a)?, parse_int(b)?))        // ? propagates
}

fn load_port(fs: Fs, path: Path) -> int ! {    // trailing ! alone: error set inferred
    let text = fs.read_to_string(path)?        // IoError joins the inferred set
    let port = parse_int(text.trim())?         // ParseError joins it too
    Ok(port)
}

// handling
let port = load_port(fs, "port.txt") else { e ->
    log.warn("using default port: {e}")
    8080
}
match parse_int(input) {
    Ok(n) -> use(n)
    Err(BadDigit { at, found }) -> println("bad digit {found} at {at}")
    Err(UnexpectedEnd) -> println("empty input")
}
```

`T ! E` is `Result[T, E]` where `E` is an *error set*: a sum type whose
variants can carry data and which is a subtype of any error set that
includes its variants. So `ParseError` coerces to `{ ParseError, IoError }`
without conversion boilerplate. A trailing `!` with no set infers the set
from the body; the compiler and documentation tool show the inferred set,
and the public-API linter asks you to spell it out at package boundaries.

Panics are different: an out-of-bounds index, an overflow, a failed
`assert`, or an explicit `panic("...")` is a bug. It unwinds the current
task to its enclosing scope; it is not caught by `match`. See
[05-errors.md](05-errors.md).

## Ownership without lifetimes

```sisu
fn summarise(users: List[User]) -> String {          // users is borrowed (read)
    var out = String.new()
    for u in users { out.push_line(u.name) }
    out                                               // out is moved to the caller
}

fn add_user(mut users: List[User], u: own User) {     // users borrowed mutably, u consumed
    users.push(u)
}

let roni = User { name: "Roni", age: 30 }
add_user(mut team, roni)                              // mut is visible at the call site
// roni is gone here: it was moved into team. Using it is a compile error.

fn oldest(users: List[User]) -> ref User {            // returns a borrow of its argument
    users.iter().max_by { u -> u.age }.expect("non-empty")
}
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
let a = List.of(1, 2, 3)
let b = a                      // move: a is no longer usable
let c = b.copy()               // explicit deep copy
let p = Point { x: 1.0, y: 2.0 }
let q = p                      // Point is Copy, so this copies
```

Shared ownership is a type you ask for:

```sisu
let config = Shared.new(load_config())   // reference-counted, immutable
let c2 = config                           // cheap: bumps the count
let counter = Shared.new(Mutex.new(0))    // shared *and* mutable: needs a lock
counter.lock().value += 1
```

## Traits and generics

```sisu
trait Shape {
    fn area(self) -> f64
    fn describe(self) -> String { "a shape of area {self.area()}" }   // default
}

impl Shape for Circle {
    fn area(self) -> f64 { PI * self.radius * self.radius }
}

fn largest[S: Shape](shapes: List[S]) -> ref S {
    shapes.iter().max_by { s -> s.area() }.expect("non-empty")
}

fn print_all(shapes: List[Box[dyn Shape]]) {      // dynamic dispatch is spelled out
    for s in shapes { println(s.describe()) }
}

trait Container {
    type Item                                       // associated type
    fn get(self, i: int) -> ref Item?
    fn len(self) -> int
}

fn sum_of[C: Container](c: C) -> int
    where C.Item: Into[int]
{
    var total = 0
    for i in 0..c.len() { total += c.get(i)?.into() ?? 0 }
    total
}
```

Generic code is type-checked once, at its definition, against the bounds.
Instantiation cannot fail with an error inside the library. Generics use
square brackets to keep parsing and reading unambiguous.

```sisu
@derive(Eq, Ord, Hash, Debug, Copy)
struct Version { major: int, minor: int, patch: int }
```

`@derive` is not a macro. It is a compile-time function that inspects the
struct's fields through reflection and emits the implementation. You can
write your own.

## Collections and iteration

```sisu
let primes = [2, 3, 5, 7, 11]                     // List[int]
let fixed: Array[int, 3] = [1, 2, 3]              // fixed size when the type says so
let ages = ["roni": 30, "aino": 27]               // Map[str, int]
let seen = Set.of("a", "b")

let big = primes.iter()
    .filter { p -> p > 3 }
    .map { p -> p * p }
    .collect[List[int]]()

for (name, age) in ages { println("{name} is {age}") }
for i in 0..10 { }                                // half-open range
for i in (0..=10).step(2) { }

gen fn fib() -> Iter[int] {                       // generator: a lazy iterator
    var (a, b) = (0, 1)
    loop { yield a; (a, b) = (b, a + b) }
}
let first_ten = fib().take(10).collect[List[int]]()
```

Closures are `{ params -> body }`. A closure passed as the last argument
can be written after the call's parentheses, so `xs.map { x -> x * 2 }` and
`scope.spawn { ... }` read as control structures.

## Structured, colourless concurrency

```sisu
fn fetch_all(net: Net, urls: List[Url]) -> List[Bytes] ! HttpError {
    task.scope { s ->
        let handles = urls.map { url -> s.spawn { net.get(url) } }
        var results = List.with_capacity(handles.len())
        for h in handles { results.push(h.join()?) }
        Ok(results)
    }
}
```

There is no `async` and no `await`. `net.get` is an ordinary function that
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
let hits = Shared.new(Atomic.new(0))
task.scope { s ->
    for _ in 0..8 {
        s.spawn { hits.fetch_add(1) }
    }
}
println(hits.load())        // 8

let (tx, rx) = channel[Job](capacity: 64)
task.scope { s ->
    s.spawn { for job in rx { handle(job) } }
    for j in jobs { tx.send(j)? }
    tx.close()
}
```

## Capabilities

```sisu
fn main(sys: System) -> ! AppError {
    let fs = sys.fs.restrict_to("./data")     // attenuate: this Fs cannot leave ./data
    let net = sys.net.only_hosts(["api.example.com"])
    let clock = sys.clock

    let cfg = load_config(fs, "config.toml")?
    run_server(net, clock, cfg)
}
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
fn binary_search[T: Ord](items: Slice[T], target: T) -> int?
    requires items.is_sorted()
    ensures result.map { i -> items[i] == target } ?? true
{
    ...
}

struct Percent(int)
    invariant self.0 in 0..=100
```

Preconditions, postconditions and type invariants are part of the signature,
documented automatically, checked at runtime in debug and test builds, and
available to static analysis. They are not an optional add-on library.

## Compile-time evaluation

```sisu
comptime fn table(n: int) -> Array[int, n] {
    var t = Array.filled(0)
    for i in 0..n { t[i] = i * i }
    t
}

const SQUARES = table(256)                 // computed by the compiler, baked into the binary

fn type_name[T](x: T) -> str {
    comptime reflect.name_of(T)            // reflection is a compile-time API
}
```

Ordinary Sisu code runs at compile time when asked. There is no separate
macro language. Compile-time code cannot perform I/O and has a fuel limit,
so builds stay fast and reproducible.

## Tests and documentation

```sisu
/// Parses a decimal integer.
///
/// ```
/// assert_eq(parse_int("42"), Ok(42))
/// ```
fn parse_int(text: str) -> int ! ParseError { ... }

test "rejects empty input" {
    assert_eq(parse_int(""), Err(UnexpectedEnd))
}

test "round-trips any int" {
    check(any[int]) { n -> parse_int("{n}") == Ok(n) }   // property test
}
```

`sisu test` runs tests, doc examples and property tests. `sisu doc` renders
the comments. `sisu fmt` has no options.

## Modules and packages

```sisu
use std.collections.HashMap
use std.net.{Http, Url}
use json                              // a dependency from sisu.toml
use crate.parser.{parse_int, ParseError}
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
json = "2"

[trust]
unsafe = ["http"]       # http may use unsafe/FFI; nothing else may
```

Publishing a version whose public API changed incompatibly without bumping
the major version is rejected by the registry, because the tool diffs the
API. Package contents are content-addressed and recorded in a transparency
log.

## Interop

```sisu
extern "C" {
    fn strlen(s: ptr[c_char]) -> usize
}

@export("sisu_checksum")
fn checksum(data: Slice[u8]) -> u32 { ... }

use c "zlib.h" as zlib                    // header import, no bindings to write
```

Calling C is `unsafe`, and `unsafe` is a block, a package-level permission,
and a line in `sisu audit`'s output.

## What you did not see

No classes, no inheritance, no `null`, no exceptions, no `async`, no macros,
no header files, no implicit conversions that lose information, no
undefined behaviour in safe code, no global mutable state without a lock, no
build scripts, no formatter options. Each of those absences is a decision
with a rationale in [15-decision-log.md](15-decision-log.md).
