# 02 - Syntax

Sisu's syntax is indentation-based and follows Python's idioms wherever a
Python idiom is not itself a documented mistake. The evidence behind each
rule is in [`indentation-syntax.md`](../research/topics/indentation-syntax.md),
[`python-idioms.md`](../research/topics/python-idioms.md) and
[`function-results.md`](../research/topics/function-results.md); the
decisions are D-020 through D-027, D-044 through D-046, D-055 and D-056.
Every departure from Python is listed at the end with its reason.

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
- Comments: `#` to end of line. `##` is a documentation comment attached to
  the item or field that follows; `##!` documents the enclosing module.
  (`//` is floor division, as in Python.)
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
- Keywords (reserved): `and as break case comptime const continue def
  defer do elif else enum error extern for from if impl import in let
  match mut not or own pass pub ref return self Self struct test trait
  type unsafe var view where while with yield`. Forty-one. Contextual
  (keywords only in their position): `requires ensures invariant result
  it bench suite`.

## Literals

| Kind | Examples | Type |
|---|---|---|
| Integer | `42`, `1_000`, `0xFF`, `0b1010`, `0o17`, `42u8` | `int` unless suffixed or inferred |
| Float | `1.0`, `2.5e-3`, `1.0f32` | `f64` unless suffixed or inferred |
| Boolean | `True`, `False` | `bool` |
| Character | `'a'`, `'\n'`, `'\u{1F600}'` | `char` |
| String | `"text {interp}"`, `r"raw\no escapes"`, `"""multi-line"""`, `b"bytes"` | `str`, `str`, `str`, `Slice[u8]` |
| List | `[1, 2, 3]` | `list[T]`, or `Array[T, N]`/`Slice[T]` when the expected type says so |
| Map | `{"a": 1, "b": 2}`, `{}` | `dict[K, V]` |
| Set | `{1, 2, 3}` | `set[T]` (the empty set is `set()`) |
| Tuple | `(1, "a")`, `()` | `(int, str)`, unit |
| Comprehension | `[x * x for x in xs if x > 0]`, `{k: v for (k, v) in pairs}`, `{x for x in xs}`, `(x for x in xs)` | `List`, `Map`, `Set`, lazy `Iter` |

String interpolation is `{expr}` in any ordinary string literal, with an
optional format spec after a colon in Python's mini-language: `{x:.2f}`,
`{x:>8}`, `{x:?}` for the `Debug` rendering. Literal braces are `{{` and
`}}`. There is no `f` prefix: interpolation is part of the grammar (PEP
701's conclusion), so an interpolated literal is a `String` and a plain
one is a `str`. Strings use double quotes only; single quotes are for
`char`.

## Declarations

```sisu
let x = expr                     # immutable binding
let x: T = expr
let x: T                         # declared now, definitely assigned on every path before use
var x = expr                     # mutable binding
let a, b = pair                  # tuple unpacking
let first, *rest = xs            # star unpacking
const NAME: T = expr             # compile-time constant

def name[T: Bound](p: T, mut q: U, own r: V, d: W = default) -> R ! E:
    body

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

trait Name[T]:
    def m(self) -> T             # no body: required
    def n(self) -> T:            # body: provided
        return self.m()

impl Trait for Type:             # trait implementation
    def m(self) -> T: ...

impl Type:                       # extension from another module (Q-12)
    def extra(self) -> int: ...

type Alias = Existing[int]       # alias, not a new type
type Port = int where 1 <= self <= 65535     # refinement subtype

import std.json                  # binds json
import std.collections.SortedDict as Sorted
from std.net import Http, Url as U
from weather.parser import parse_int         # package-absolute

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

## Expressions and statements

Blocks never have a value. A function yields a value only with
`return expr`; a non-unit function whose control flow can reach the end of
its body without a `return` is a compile error, and `return` without a
value is allowed only in unit functions. A `return` inside a closure
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
statement, and the value is assigned or returned in each arm. Python's
`a if c else b` is not adopted: `if c: a else: b` is the same length,
works on one line and across lines, and mirrors the statement form.

Operator precedence, highest first:

| Level | Operators | Notes |
|---|---|---|
| Postfix | `x.f`, `x[i]`, `x[i:j]`, `f(x)`, `x?`, `x?.f` | |
| Power | `**` | right-associative; mixing with unary minus requires parentheses |
| Unary | `-x`, `~x`, `ref x`, `ref mut x` | |
| Multiplicative | `* / // % *% /%` | `//` is floor division; `%` has the sign of the divisor |
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
let Some(x) = opt else:          # bind or diverge; the block must return, break, continue or panic
    return Err(NotFound)

while cond:
    ...
while let Some(x) = it.next():
    ...
for x in xs:
    ...
for i, x in enumerate(xs):
    ...
for i in range(10):              # 0..9; range(a, b), range(a, b, step)

match value:
    case Some(Rect(w, h)) if w == h: ...
    case [first, *rest]: ...
    case 1 | 2 | 3: ...
    case Point(x=0, y): ...
    case _: ...

with fs.open(path)? as f:        # f is dropped at the end of the block
    ...

defer:                           # runs at the end of the enclosing block
    cleanup()
defer cleanup()                  # single-statement form

unsafe:
    ...
```

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
        s.spawn(() -> net.get(url))
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
ref T  ref mut T   # borrows
own T              # only in parameter position; elsewhere T is owned
list[T]  dict[K, V]  set[T]      # builtin collections, with literal forms
Array[T, N]  Slice[T]  Iter[T]   # fixed-size array, view of a sequence, lazy iterator
(A, B)  (a: A, b: B)  ()   # tuples, labelled tuples, unit
(A, B) -> C        # function type
dyn Trait          # trait object (unsized; used behind ref or Box)
Box[T]  Shared[T]  Weak[T]  Cell[T]  Mutex[T]
Never              # the type of expressions that do not return
```

`?` binds tighter than `ref`, so `ref T?` is an optional reference. A
function's result may be a labelled tuple, `-> (q: int, r: int)`, which
documents the results, lets `ensures` name them and lets callers write
`res.q`; the body still returns with `return (q, r)`. Inside `ensures`,
the contextual name `result` refers to an unlabelled result (D-056).

## Prelude functions

Python's builtin functions are kept where they read better than a method:
`print`, `print_err`, `len`, `range`, `enumerate`, `zip`, `reversed`,
`sorted`, `min`, `max`, `sum`, `any`, `all`, `abs`. They are ordinary
generic functions over traits (`len[T: Len](x: T) -> int`,
`sorted[T: Ord](xs: Iter[T]) -> list[T]`), not a dunder protocol, so a user
type joins them by implementing the trait. The method spelling exists
because the trait method does (`xs.len()`), and the linter prefers the
function form for these names; everything else (`xs.push(x)`,
`xs.iter().map(...)`, `s.trim()`) is a method, as in Python. `print(x, y)`
takes one argument: write `print("{x} {y}")`.

## Slicing and indexing

`a[i]` is bounds-checked and traps on a negative index. `a[i:j]` is a
half-open slice; `a[^1]` is the last element and `a[i:^1]` runs to the
end (Nim's spelling), replacing Python's negative indices, whose silent
wrap-around is a bug source. `zip` stops with an error on unequal
lengths; `zip_shortest` is explicit (PEP 618).

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
ambiguity CoffeeScript and Nim's command syntax are known for. There is
no layout-off dialect and no explicit-brace fallback; adding one later
would recreate Scala 3's two-dialect split.

## Departures from Python, with reasons

| Python | Sisu | Reason |
|---|---|---|
| Assignment declares a variable | `let`/`var` | Implicit declaration is the root of `global`, `nonlocal`, `UnboundLocalError` and the typo-creates-a-variable bug (Lua, JS); Nim and Mojo require it too |
| `class`, `__init__` | `struct` with declared fields, no inheritance | PEP 557 is the admission that classes are boilerplate; D-012 |
| `a if c else b` | `if c: a else: b` | One spelling for one line and many; mirrors the statement form |
| `lambda x: e` | `x -> e`, `it`, `do` blocks | `lambda` is "crippled" by its designer's own account; `do` gives the multi-line closure Python refused |
| Truthiness | `bool` only | Every typed descendant refused it; Python's own `datetime.time` midnight bug |
| `None` | `T?` with `?.`, `??`, `if let` | PEP 505's own counts of the boilerplate `None` creates |
| `is` | none | A dynamic-language operator; the small-int cache trap |
| `try`/`except`/`finally` | `T ! E`, `?`, `else`, `defer`, `Drop` | D-011; PEP 765's 73% incorrect `finally` sites |
| Mutable default arguments, late-binding closures | Defaults evaluated per call; per-iteration loop bindings | The two best-known Python traps |
| `/` on integers | Type error; `//` | PEP 238, kept where Mojo and Codon regressed |
| Negative indices | `a[^1]` | A computed index drifting below zero silently reads the other end |
| `f"..."` | interpolation in every literal | The prefix is an artefact of the old tokeniser (PEP 701) |
| Docstrings | `##` comments | Attribute docstrings were rejected (PEP 224); `##` documents items and fields uniformly |
| Decorators as calls | `@attr` compile-time attributes | Registration by reflection; wrapping is a plain call; type checkers needed `dataclass_transform` to see through decorators |
| `yield` silently makes a generator | `yield` requires an `Iter[T]` return type; a valued `return` inside is an error | PEP 255's own argument for a marker |
| `assert` stripped by `-O` | `assert` always on; `debug_assert` for the stripped form | Google's and Nim's rule |
| `import a.b.c` binds `a` | binds `c` | The Python rule is a known surprise |
| `_name`, `__all__` | `pub`, `pub(package)` | A convention a compiler replaces |
| `__dunder__` protocols | traits with plain names | Same uniformity, named |
| `else` on loops | `if let` on the search result | The tutorial's own "Yes, this is the correct code. Look closely" |
| `:=` | `if let`, `while let` | PEP 572's cost |
| `for i in range(...)` with `else` | without | see above |
| `print(x, y)` concatenation | `print("{x} {y}")` | one way to format |
| Tabs allowed | spaces only | Python 3 itself made mixing an error; every later layout language banned tabs |
| Chained comparisons in any direction | same direction only | `a < b > c` is "perfectly legal (though perhaps not pretty)" by Python's own admission |
