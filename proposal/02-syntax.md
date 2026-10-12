# 02 - Syntax

Sisu's syntax is indentation-based and follows Python's idioms wherever a
Python idiom is not itself a documented mistake. The evidence behind each
rule is in [`indentation-syntax.md`](../research/topics/indentation-syntax.md),
[`python-idioms.md`](../research/topics/python-idioms.md) and
[`function-results.md`](../research/topics/function-results.md), and in
the assessments of [19-novel-features.md](19-novel-features.md); the
decisions are D-020 through D-027, D-044 through D-046, D-055, D-056 and
D-061, and three leaning decisions are marked where they apply. Every
departure from Python is listed at the end with its reason, and none is
silent: where Sisu spells something as Python does, or as Rust does for
syntax the two share, it means the same or fails to compile
(leaning, D-062).

## Lexical structure

- Source is UTF-8. Identifiers are Unicode letters, digits and `_`,
  normalised to NFC; the confusable and mixed-script lints are on by
  default (D-048).
- Naming follows PEP 8 and is enforced: `snake_case` for values,
  functions, methods, modules and fields; `CamelCase` for types, traits,
  enum variants and the prelude constants `True`, `False`, `None`, `Some`,
  `Ok`, `Err`; `SCREAMING_CASE` for `const` items. As in Python, the
  builtin types that have their own literal or primitive form are
  lowercase (`int`, `f64`, `bool`, `char`, `str`, `list`, `dict`, `set`)
  and every other type is `CamelCase` (`String`, `Array`, `Option`,
  `Shared`). Because variants are always capitalised, a lowercase
  name in a pattern is always a binding and a capitalised one is always a
  constant, which is the ambiguity PEP 635 could not resolve.
- Comments: `#` to end of line. `##` on its own line is a documentation
  comment attached to the item or field that follows; at the end of a
  declaration line it documents that line; `##!` documents the enclosing
  module. (`//` is floor division, as in Python.)
- One statement per line. No semicolons. A logical line continues inside
  `()`, `[]` and `{}`, after a line ending in `,` or an open bracket, and
  on a line whose first token is a binary operator, `.`, `?.` or `??`.
  There is no backslash continuation.
- Blocks: a logical line ending in `:` opens a block; the block is the
  following lines indented more than the line that opened it. The lexer
  emits INDENT and DEDENT tokens from a stack of indentation widths
  exactly as Python's reference does, ignores blank and comment-only
  lines, and never consults the parser. Indentation is ASCII spaces only;
  a tab anywhere in indentation is an error. Any consistent width is
  legal; the formatter emits four. A dedent that matches no enclosing
  level is an error naming the candidate columns.
- `pass` is the empty block.
- Keywords (reserved): `and as assert break case comptime const continue
  def defer do dyn elif else enum error extern for from if impl import in
  let match mut not or own pass pub ref return select self Self struct
  test trait type unsafe var view where while with yield`. Forty-seven.
  Contextual (keywords only in their position): `requires ensures
  invariant decreases old result it bench suite expect pure`. After a `.`
  and inside a module path a keyword is an ordinary identifier
  (`log.error`, `x.or(d)`, `std.test`).

## Literals

| Kind | Examples | Type |
|---|---|---|
| Integer | `42`, `1_000`, `0xFF`, `0b1010`, `0o17`, `42u8` | `int` unless suffixed or inferred |
| Float | `1.0`, `2.5e-3`, `1.0f32` | `f64` unless suffixed or inferred |
| Boolean | `True`, `False` | `bool` |
| Character | `'a'`, `'\n'`, `'\u{1F600}'` | `char` |
| String | `"plain"`, `"text {interp}"`, `r"raw\no escapes"`, `"""multi-line"""`, `b"bytes"` | `str`, `String`, `str` (no escapes, no interpolation), `str`, `bytes` |
| List | `[1, 2, 3]` | `list[T]`, or `Array[T, N]`/`Slice[T]` when the expected type says so |
| Map | `{"a": 1, "b": 2}`, `{}` | `dict[K, V]` |
| Set | `{1, 2, 3}` | `set[T]` (the empty set is `set()`) |
| Tuple | `(1, "a")`, `()` | `(int, str)`, unit |
| Comprehension | `[x * x for x in xs if x > 0]`, `{k: v for k, v in pairs}`, `{x for x in xs}`, `(x for x in xs)` | `list`, `dict`, `set`, lazy `Iter` |

String interpolation is `{expr}` in any ordinary string literal, with an
optional format spec after a colon in Python's mini-language: `{x:.2f}`,
`{x:>8}`, `{x:?}` for the `Debug` rendering. `{expr=}` is Python 3.8's
self-documenting form
([`end-user-and-live.md`](../research/ideation/end-user-and-live.md),
candidate 12), which writes the expression's text and `=` before its
value, so `"{total=}"` renders as `total=42`; the value takes its `Debug`
rendering, as Python uses `repr`, and a format spec may follow, as in
`{x=:.2f}` (leaning, D-071). Literal
braces are `{{` and `}}`. There is no `f` prefix: interpolation is part
of the grammar (PEP 701's conclusion), so an interpolated literal is a
`String` and a plain one is a `str`. Strings use double quotes only;
single quotes are for `char`. A string literal takes the expected type
when that type implements `FromLiteral[str]` (`Path`, `Url`, `String`),
the same way `[1, 2, 3]` becomes an `Array` when the expected type says
so; this applies to literals only and is not a conversion of values.
Numeric literals reach the exact types of `std.num` the same way and need
no suffix: `let tenth: Decimal = 0.1` is exactly one tenth (D-076,
[03](03-types-and-generics.md)). `String` implements `Add` for
concatenation, as in Python; interpolation is the idiom for formatting.

## Declarations

```sisu
let x = expr                     # immutable binding
let x: T = expr
let x: T                         # declared now, definitely assigned on every path before use
var x = expr                     # mutable binding
let a, b = pair                  # tuple unpacking
let a, b = 1, 2                  # parentheses may be omitted around a tuple in let, var, assignment, return
let first, *rest = xs            # star unpacking
let x = parse(x)?                # rebinding in the same scope is allowed; shadowing an outer scope warns
const NAME: T = expr             # compile-time constant, type required

def name[T: Bound](p: T, mut q: U, own r: V, d: W = default) -> R ! E:
    body
pure def f(x: int) -> int: ...   # pure: no capabilities, no unsafe, no shared mutation; usable at compile time
comptime def g(T: type) -> type: ...   # compile-time only; `type` is the type of types

struct Name[T]:
    field: T                     ## documented field
    def method(self) -> T: ...   # methods live in the body
    def make() -> Name[T]: ...   # no self: an associated function, Name.make()

view struct Name:                # may hold references
    text: str

enum Name:
    A
    B(T)
    C(x: T, y: U)

error Name:
    Variant
    Variant(x: T)
    ..                           # a trailing `..` line makes the set open to other packages

trait Name[T]:
    def m(self) -> T             # no body: required
    def n(self) -> T:            # body: provided
        return self.m()

trait Container(Len):            # supertrait: implementing Container requires Len, Python's class syntax
    type Item                    # associated type
    def get(self, i: int) -> ref Item?

impl[T: Display] Display for list[T]:   # generic implementation
    ...
struct Grid[T, const W: int]:    # const generics
    cells: Array[T, W]
struct Stack[T, A: Allocator = Global]:   # a defaulted type parameter (leaning, D-060)
    items: list[T, A]
def longest(a: str, b: str) -> ref str from a, b: ...   # which parameter a returned borrow comes from
def show[T](x: T) where T: Display + Hash: ...        # bounds in a where clause

test "name":                     # a test; `suite "name":` groups one level
    assert f(1) == 2
test "name" for a, b in cases:   # parametrised: one reported case per element
    assert f(a) == b
test "name"(sys: System):        # a test that needs a real capability runs serially
    ...
bench "name"(b: Bench):          # run by `sisu bench`, not `sisu test`
    for _ in b.loop():
        black_box(f(1))

impl Trait for Type:             # trait implementation
    def m(self) -> T: ...

impl Type:                       # extension from another module (Q-12)
    def extra(self) -> int: ...

type Alias = Existing[int]       # alias, not a new type
type Port = int where 1 <= self <= 65535     # refinement subtype

import std.json                  # binds json
import std.collections.SortedDict as Sorted
from std.net import TcpListener, Url as U
from weather.parser import parse_int         # package-absolute
import c "zlib.h" as zlib with link="z"     # C header import (trusted packages only)

extern "C":
    def strlen(s: ptr[c_char]) -> c_size
```

Constructing a value is a call on its type with positional or keyword
arguments in field order: `Point(x=1.0, y=2.0)`, `Mutex(0)`, `Shared(cfg)`,
`String()`, `list()`, `dict()`. Named constructors are associated
functions: `list.with_capacity(n)`, `Clock.frozen_at(t)`, `Url.parse(s)`.
A `let x: T` without a value declares a binding that the compiler checks
is definitely assigned before use on every path (Swift, Kotlin); this is
how a `match` statement produces a value without blocks having one.

Struct fields may have defaults in any position; positional arguments
fill fields in declaration order and keyword arguments fill by name.

`pub` precedes any item or field to export it; `pub(package)` limits
visibility to the current package. Attributes are `@name(args)` on the
line before the item: `@derive(Eq, Hash)`, `@deprecated("use parse_v2")`,
`@inline`, `@export("c_name")`, `@cfg(test)`. They are compile-time data,
not run-time calls (D-046).

Parameters are positional-or-keyword by default. A bare `*` makes the
parameters after it keyword-only; a bare `/` makes those before it
positional-only (PEP 3102, PEP 570). `*xs: T` collects variadic
arguments into a `Slice[T]`; there is no `**kwargs`. Default values are
evaluated at each call, not at definition.

A generic type's parameters may have defaults. Defaulted type
parameters come after the others, and a use site that omits one gets the
default: `Stack[int]` is `Stack[int, Global]`. The standard collections
take their allocator this way, `list[T, A: Allocator = Global]`, so
ordinary code never names an allocator and code that puts a list in a
borrowed region writes `list[Header, ref Region]` (D-060;
[03](03-types-and-generics.md), [04](04-memory-and-ownership.md)).

## Expressions and statements

Blocks never have a value. A function yields a value only with
`return expr`; a non-unit function whose control flow can reach the end of
its body without a `return` is a compile error, and `return` without a
value is allowed only in unit functions. A generator (a `def` that
contains `yield` and returns `Iter[T]`) ends when its body ends. A `return` inside a closure
returns from the closure. This is D-056; the alternatives, Rust's
last-expression rule and MATLAB/Go/Nim named result variables, are
assessed in [`function-results.md`](../research/topics/function-results.md).

`if` and `match` have an expression form exactly when every branch is a
single expression:

```sisu
let sign = if x > 0: 1 elif x < 0: -1 else: 0
let area = match shape:
    case Circle(r): PI * r ** 2
    case Rect(w, h): w * h
    case Empty: 0.0
```

An `if` expression must have an `else`; a `match` expression must be
exhaustive. An arm that needs statements makes the whole thing a
statement, and the value is assigned or returned in each arm. Three other
forms take a value from an expression on the same line: `let x = e else: v`
and `let x = e else as err: v` (the fallback `v` is an expression, or a
single diverging statement such as `return None`; `as err` binds the
error), `unsafe e` (an `unsafe:` block is a statement; `unsafe e` marks
one expression), and `comptime e` (evaluate `e` at compile time; `comptime
for`, `comptime if` and `comptime assert` are the statement forms). The
one block that yields a value is `return impl Trait for T:` inside a
`comptime def` whose result type is `impl Trait for T`
([08-metaprogramming.md](08-metaprogramming.md)). The target of an assignment or augmented
assignment is a place: a variable, a field, an index, or a call that
returns `ref mut` (`counts.entry(k).or_insert(0) += 1`). Python's
`a if c else b` is not adopted: `if c: a else: b` is the same length,
works on one line and across lines, and mirrors the statement form.

Operator precedence, highest first:

| Level | Operators | Notes |
|---|---|---|
| Postfix | `x.f`, `x[i]`, `x[i:j]`, `f(x)`, `x?`, `x?.f`, `x.[f]` (field named by a compile-time value) | |
| Power | `**` | right-associative; mixing with unary minus requires parentheses |
| Unary | `-x`, `~x`, `ref x`, `ref mut x` | |
| Multiplicative | `* / // % *% *\|` | `//` is floor division; `%` has the sign of the divisor |
| Additive | `+ - +% -% +\| -\|` | `%` suffix wraps, `\|` suffix saturates |
| Shift | `<< >>` | mixing with arithmetic requires parentheses |
| Bitwise | `& ^ \|` | mixing with each other requires parentheses |
| Comparison | `== != < <= > >= in not in` | chained; see below |
| `not` | | |
| `and` | | |
| `or` | | |
| `??` | null coalescing | |
| `->` | closure | `x -> expr`, `(a, b) -> expr`, `() -> expr` |
| Assignment | `= += -= ...` | statements, not expressions |

- `a < b <= c` is a chained comparison with Python's meaning, allowed
  only when every operator points the same way (`a < b > c` is an error).
- Mixing bitwise and arithmetic operators, or `and` with `or`, without
  parentheses is an error. `-2 ** 2` is an error; write `-(2 ** 2)` or
  `(-2) ** 2`.
- `int / int` is a type error with a fix-it offering `//` or a float
  conversion. `/` is defined on floats only; `//` on integers only (PEP
  238's lesson, kept rather than regressed as Mojo and Codon did).
- `and` and `or` take and return `bool`. There is no truthiness: `if xs`
  is a type error; write `if not xs.is_empty()`.
- `==` is structural equality via `Eq`. There is no `is`; identity of
  shared references is `Shared.same(a, b)`.
- `x in xs` is `xs.contains(x)` via the `Contains` trait.
- Assignment is a statement; `if x = y:` is a parse error. There is no
  walrus.

## Control flow

```sisu
if cond:
    ...
elif cond:
    ...
else:
    ...

if let Some(x) = opt:            # pattern binding
    ...
if let Some(user) = users.get(id), let Some(team) = user.team, team.active:
    notify(team)                 # chained: each binding is in scope to its right and here
let Some(x) = opt else:          # bind or diverge; the block must return, break, continue or panic
    return Err(NotFound)

while cond:
    ...
while let Some(x) = iter.next():
    ...
while let Some(line) = lines.next(), not line.is_empty():
    ...                          # stops at the end of the input or at the first empty line
for x in xs:                     # x: ref T
    ...
for x in mut xs:                 # x: ref mut T
    ...
for x in own xs:                 # consumes xs; x: T
    ...
for i, x in enumerate(xs):
    ...
for i in range(10):              # 0..9; range(a, b), range(a, b, step)
    ...

match value:
    case Some(Rect(w, h)) if w == h: ...
    case [first, *rest]: ...
    case 1 | 2 | 3: ...
    case Point(x=0, y=y): ...   # keyword patterns, as in Python
    case Point(x=0, ..): ...     # `..` ignores the remaining fields
    case _: ...

select:                          # waits on channel, timer, join and cancellation operations
    case e = rx.recv():
        handle(e)
    case _ = clock.after(Duration.seconds(1)):
        timeout()

assert cond                      # a statement, no parentheses; panics with sub-expression values
assert cond, "message {x}"
expect cond                      # in tests: records the failure and continues

with fs.open(path)? as f:        # f is dropped at the end of the block
    ...

defer:                           # runs at the end of the enclosing block
    cleanup()
defer cleanup()                  # single-statement form

unsafe:
    ...
```

A condition in `if`, `elif` or `while` may chain `let` bindings and
boolean tests with commas (leaning, D-071). The parts run left to right,
and the chain fails at the first pattern that does not match or the first
test that is `False`; each binding is in scope for the rest of the chain
and in the branch it guards, and nothing an earlier part did is undone
(unlike Verse, whose failure contexts roll back their effects). The
separator is a comma because `and` takes and returns `bool` only (D-025),
and a long chain breaks after a comma by the continuation rule above.
This is Swift's condition list and Rust 1.88's let chains
([`logic-constraint-and-rewriting.md`](../research/unconventional/logic-constraint-and-rewriting.md),
idea 3), and it covers what nested `if let` blocks and Python's walrus
were used for.

No `switch`, no `goto`, no `do-while`, no C-style `for`, no `else` on
loops, no loop labels (extract a function; the linter says so). `match`
and `case` are hard keywords. `with` is a scoped binding whose value is
dropped at the end of the block; it is never a loop and needs no
`__enter__`/`__exit__` protocol because every type has `Drop`. `defer` is
block-scoped, not function-scoped.

## Closures

```sisu
x -> x + 1                       # one parameter
(acc, x) -> acc + x              # several
() -> compute()                  # none
xs.map(it * 2)                   # implicit parameter `it` in a single-parameter closure

xs.each do x:                    # trailing block: a closure whose body is a block
    print(x)

let total = xs.fold(0) do acc, x:
    return acc + x               # `return` returns from the closure

task.scope do s:
    for url in urls:
        s.spawn(() -> http.get(net, url))
```

An inline closure is `params -> expr` and is a single expression. A
closure whose body needs statements is written as a trailing `do` block
on the call that receives it: `call(args) do params:` followed by an
indented body, bound to the call's last function-typed parameter. `do:`
with no parameters uses `it` if the function takes one argument. A
trailing block may appear only at the end of a logical line, so no block
ever sits inside an expression (Guido's rule), and a call that needs two
multi-line closures passes a nested `def`. Closures capture by reference;
`own` before the parameter list (`own x -> ...`, `do own x:`) captures by
move.

`it` is a contextual keyword: an argument expression that mentions `it`
where a one-parameter function is expected is a closure over `it`. In
nested calls `it` belongs to the innermost such argument.

## Types

```sisu
int  uint  i8 ... i128  u8 ... u128  f32  f64  bool  char  str  String
T?                 # Option[T]
T ! E              # Result[T, E] with error set E
T !                # Result[T, inferred]
! E                # () ! E, for functions that return nothing or fail
{A, B}             # in type position: the union of error sets A and B
ref T  ref mut T   # borrows
list[T]  dict[K, V]  set[T]      # builtin collections, with literal forms
Array[T, N]  Slice[T]  Iter[T]   # fixed-size array, view of a sequence, lazy iterator
(A, B)  (a: A, b: B)  ()   # tuples, labelled tuples, unit
(A, B) -> C        # function type
pure (A, B) -> C   # a function type that admits only pure functions (D-074)
dyn Trait          # trait object (unsized; used behind ref or Box)
Box[T]  Shared[T]  Weak[T]  Cell[T]  Mutex[T]
Never              # the type of expressions that do not return
```

`ref` binds tighter than `?`, so `ref T?` is `(ref T)?`, an optional
reference, which is the common case. A function's result may be a labelled
tuple, `-> (q: int, r: int)`, which
documents the results, lets `ensures` name them and lets callers write
`res.q`; the body still returns with `return (q, r)`. Inside `ensures`,
the contextual name `result` refers to an unlabelled result (D-056).

## Prelude functions and intrinsics

Python's builtin functions are kept where they read better than a method:
`print`, `print_err`, `len`, `range`, `enumerate`, `zip`, `zip_shortest`,
`reversed`, `sorted`, `min`, `max`, `sum`, `any`, `all`, `abs`, plus
`panic` and `unreachable`. They are ordinary generic functions over
traits (`len[T: Len](x: T) -> int`,
`sorted[T: Ord](xs: Iter[T]) -> list[T]`), not a dunder protocol, so a user
type joins them by implementing the trait. The method spelling exists
because the trait method does: the function form is idiomatic at the
start of an expression (`len(xs)`, `sorted(xs)`), the method form at the
end of a chain (`xs.iter().map(f).sum()`), and the linter flags only
`xs.len()`. Everything else (`xs.push(x)`, `xs.iter().map(...)`,
`s.trim()`) is a method, as in Python. `print(x, y)`
takes one argument: write `print("{x} {y}")`.

`assert`, `debug_assert` and `todo` are compiler intrinsics, not
functions. `assert` shows the value of every sub-expression when it fails
([17](17-testing.md)), and `debug_assert` is the form that release builds
strip. `todo("name")` is a typed hole (D-061, decided with Q-46; N-27 has
the design). Its signature is `todo[T](name: str = "") -> T`, and it
checks against whatever type its position expects, so the code around
it type-checks. `sisu check` warns about each hole, and `sisu check
--holes` reports it in full: its expected type, the bindings in scope, the
contracts that constrain it and the tests that reach it
([10](10-tooling.md)); the interpreter runs until it reaches a
hole and stops there, and a test that reaches one is reported as blocked
rather than failed ([17](17-testing.md)); `sisu build --release` and
`sisu publish` reject a program that contains one. A placeholder meant to
ship is `panic("...")`. The hole adds no syntax: `todo` is a prelude name,
not a keyword. Holes are the interaction model of every dependently typed
language and have been on in GHC for twelve years
([`holes-synthesis-and-ai-era.md`](../research/unconventional/holes-synthesis-and-ai-era.md)).

```sisu
def median(xs: list[int]) -> int:
    requires len(xs) > 0
    let s = sorted(xs)
    return todo("middle")        # checks as an int; a test that reaches it stops here
```

## Slicing and indexing

`a[i]` is bounds-checked. `a[i:j]` is a half-open slice, `a[i:]` runs to
the end, `a[^1]` is the last element and `a[i:^1]` drops the last element
(Nim's spelling), replacing Python's negative indices, whose silent
wrap-around is a bug source. A negative index computed at run time traps.
A constant negative index or slice bound is a compile error whose fix-it
is the `^` form, `xs[^1]` for `xs[-1]` and `xs[:^1]` for `xs[:-1]`,
because Python's spelling must not compile to a different meaning (D-062;
see the departures below). `zip` stops with an error on unequal lengths;
`zip_shortest` is explicit (PEP 618).

## Formatting

`sisu fmt` has no configuration. Four spaces, 100 columns, Black's
bracket-first line splitting, breaks before binary operators, a magic
trailing comma, double quotes, sorted and pruned imports, one blank line
between items, and an AST-equality self-check. It never changes which
block a statement belongs to, because that is semantics; the language
server offers a reindent-on-paste action instead. Tabs are not a formatter
option because they are a lexer error.

## Grammar notes

Layout is decided entirely in the lexer, outside brackets only, with no
parser feedback (Python, Nim and Koka's rule, not Haskell's). The parser
is hand-written recursive descent and never fails: every production can
yield an error node, recovery synchronises at NEWLINE and DEDENT, and the
messages name the expectation ("expected ':'", "expected an indented
block after `if` on line 12"). Mandatory call parentheses, explicit
`return`, no truthiness and no optional parentheses remove every
ambiguity CoffeeScript and Nim's command syntax are known for. A
condition runs from `if`, `elif` or `while` to its `:`, and a comma at its
top level always separates the parts of a chain, so the bare tuples that
`let`, `var`, assignment and `return` allow are parenthesised there:
`if let (a, b) = pair, a < b:`. There is
no layout-off dialect and no explicit-brace fallback; adding one later
would recreate Scala 3's two-dialect split.

## Departures from Python, with reasons

No departure is silent (D-062). Where Sisu keeps a spelling from Python,
or from Rust for syntax the two share, it keeps the meaning; where the
meaning differs, the spelling fails to compile. A run-time failure is
acceptable only where no static check exists, and a silent difference is
a design bug. The specification keeps a false-friend table with a check
per row, and its first rows are these: `xs[-1]` with a constant index
does not compile and offers `xs[^1]`; `7 / 2`, `if xs:` and
`a if c else b` do not compile; `dict` iterates in insertion order, as
Python's does (Q-42), so code that relies on the order is correct; and
`-7 % 3` is 2, as in Python, which is silent only for code written with
Rust's meaning (-1) and is flagged in the Rust-to-Sisu corpus. Rows added
after 1.0 go through editions ([13](13-evolution-and-governance.md)).
Python 3's silent change to `/` is the cautionary tale, and people and
models alike read a familiar spelling with its familiar meaning first
([`ai-era-and-evolution.md`](../research/ideation/ai-era-and-evolution.md),
A11).

| Python | Sisu | Reason |
|---|---|---|
| Assignment declares a variable | `let`/`var` | Implicit declaration is the root of `global`, `nonlocal`, `UnboundLocalError` and the typo-creates-a-variable bug (Lua, JS); Nim and Mojo require it too |
| `class`, `__init__` | `struct` with declared fields, no inheritance | PEP 557 is the admission that classes are boilerplate; D-012 |
| `a if c else b` | `if c: a else: b` | One spelling for one line and many; mirrors the statement form |
| `lambda x: e` | `x -> e`, `it`, `do` blocks | `lambda` is "crippled" by its designer's own account; `do` gives the multi-line closure Python refused |
| Truthiness | `bool` only | Nim, Codon, Swift, Rust and Go refuse it and Mojo warns about it; Python's own `datetime.time` midnight bug |
| `None` | `T?` with `?.`, `??`, `if let` | PEP 505's own counts of the boilerplate `None` creates |
| `bytes` literal is immutable and indexable by `int` | `b"..."` is `bytes`, viewed as `Slice[u8]` | kept; one owned byte string type beside `String` |
| `is` | none | A dynamic-language operator; the small-int cache trap |
| `try`/`except`/`finally` | `T ! E`, `?`, `else`, `defer`, `Drop` | D-011; PEP 765's 73% incorrect `finally` sites |
| Mutable default arguments, late-binding closures | Defaults evaluated per call; per-iteration loop bindings | The two best-known Python traps |
| `/` on integers | Type error; `//` | PEP 238, kept where Mojo and Codon regressed |
| Negative indices | `a[^1]`; a constant `a[-1]` does not compile | A computed index drifting below zero silently reads the other end |
| `f"..."` | interpolation in every literal | The prefix is an artefact of the old tokeniser (PEP 701) |
| Docstrings | `##` comments | Attribute docstrings were rejected (PEP 224); `##` documents items and fields uniformly |
| Decorators as calls | `@attr` compile-time attributes | Registration by reflection; wrapping is a plain call; type checkers needed `dataclass_transform` to see through decorators |
| `yield` silently makes a generator | `yield` requires an `Iter[T]` return type; a valued `return` inside is an error | PEP 255's own argument for a marker |
| `assert` stripped by `-O` | `assert` always on; `debug_assert` for the stripped form | Google's and Nim's rule |
| `import a.b.c` binds `a` | binds `c` | The Python rule is a known surprise |
| `_name`, `__all__` | `pub`, `pub(package)` | A convention a compiler replaces |
| `__dunder__` protocols | traits with plain names | Same uniformity, named |
| `else` on loops | `if let` on the search result | The tutorial's own "Yes, this is the correct code. Look closely" |
| `:=` | `if let`, `while let`, chained conditions | PEP 572's cost |
| `for i in range(...)` with `else` | without | see above |
| `print(x, y)` concatenation | `print("{x} {y}")` | one way to format |
| Tabs allowed | spaces only | Python 3 itself made mixing an error; every later layout language banned tabs |
| Chained comparisons in any direction | same direction only | `a < b > c` is "perfectly legal (though perhaps not pretty)" by Python's own admission |
