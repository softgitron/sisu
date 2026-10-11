# 02 - Syntax

Syntax is the part of a language that everyone has an opinion about and
almost nobody has evidence about. The small amount of evidence that exists
(see [`../research/evidence/studies.md`](../research/evidence/studies.md),
Stefik & Siebert 2013 and follow-ups) says that for novices, word-based
operators and consistent constructs measurably help, and that C-family
punctuation is no better than a randomly generated syntax. For experienced
programmers the dominant effect is familiarity. Sisu's syntax therefore:

1. Stays close to the Go/Rust/Swift/Kotlin consensus where those agree,
   because that consensus is the largest pool of familiarity.
2. Departs from it only where there is a specific reason, and records the
   reason.
3. Has exactly one way to write each thing, and a formatter that produces it.

## Lexical structure

- Source is UTF-8. Identifiers are ASCII letters, digits and `_`. Non-ASCII
  identifiers are rejected (homoglyph confusion; see open question Q-33).
- Naming is enforced by the compiler as warnings and by `sisu fmt` as
  errors: `snake_case` for values, functions, modules and fields;
  `CamelCase` for types, traits and enum variants; `SCREAMING_CASE` for
  constants. Style-insensitive identifiers (Nim) are rejected: one spelling.
- Comments: `//` line, `/* */` block (nestable), `///` documentation on the
  following item, `//!` documentation for the enclosing module.
- No semicolons. A statement ends at a newline unless the line is
  syntactically incomplete (open bracket, trailing operator, trailing `.`).
  This is Go's and Swift's rule and it has not caused problems in either.
- Keywords (reserved): `and as break comptime const continue defer dyn else
  enum error extern false fn for gen if impl in let loop match mut not or own
  pub ref return self Self struct test trait true type unsafe use var view
  where while yield`. Thirty-seven. (Rust has 39 strict + reserved; Go 25;
  Swift about 100 including contextual.) `requires`, `ensures`, `invariant`,
  `from`, `with` are contextual.

## Literals

| Kind | Examples | Type |
|---|---|---|
| Integer | `42`, `1_000`, `0xFF`, `0b1010`, `0o17`, `42u8`, `7i128` | `int` unless suffixed or inferred |
| Float | `1.0`, `2.5e-3`, `1f32` | `f64` unless suffixed or inferred |
| Boolean | `true`, `false` | `bool` |
| Character | `'a'`, `'\n'`, `'\u{1F600}'` | `char` |
| String | `"text {interp}"`, `r"raw\no escapes"`, `"""multi-line"""`, `b"bytes"` | `str`, `str`, `str`, `Slice[u8]` |
| List | `[1, 2, 3]` | `List[T]`, or `Array[T, N]`/`Slice[T]` when the expected type says so |
| Map | `["a": 1, "b": 2]`, `[:]` | `Map[K, V]` |
| Tuple | `(1, "a")`, `()` | `(int, str)`, unit |
| Range | `0..n`, `0..=n` | `Range[int]` |

String interpolation is `{expr}` with an optional format spec after a colon,
`{x:.2}`, `{x:>8}`, `{x:?}` for the `Debug` rendering. Literal braces are
`{{` and `}}`. Interpolation is evaluated at the literal, so a `str` literal
with interpolation has type `String`.

## Declarations

```sisu
let x = expr                 // immutable binding
let x: T = expr
var x = expr                 // mutable binding
const NAME: T = expr         // compile-time constant; expr must be comptime-evaluable
fn name[T: Bound](p: T, mut q: U, own r: V, d: W = default) -> R ! E { body }
struct Name[T] { field: T }
struct Name(T)               // tuple struct / newtype
view struct Name[T] { field: ref T }   // may hold references
enum Name { A, B(T), C { x: T } }
error Name { Variant, Variant { x: T } }
trait Name[T] { fn m(self) -> T }
impl[T] Trait for Type[T] { ... }
impl Type { ... }            // inherent methods
type Alias = Existing[int]   // alias, not a new type
use path.to.{a, b as c}
extern "C" { ... }
```

`pub` precedes any item or field to export it; `pub(package)` limits
visibility to the current package. Go's capitalisation convention was
considered and rejected because it couples naming to visibility and makes
renaming a visibility change.

Attributes are `@name(args)` on the preceding line:

```sisu
@derive(Eq, Hash)
@deprecated("use parse_v2")
@inline
```

Rust's `#[...]` was rejected as noisier; Python/Kotlin/Swift `@` is the
majority form.

## Expressions

Everything that produces a value is an expression, including `if`, `match`,
blocks, `loop` (via `break value`), and `unsafe { }`. A block's value is its
last expression if that expression is not followed by a newline-terminated
statement. `return expr` exits early. The Rust rule that a trailing
semicolon turns an expression into a statement does not exist because there
are no semicolons; the formatter and compiler treat a block ending in a
`let` or assignment as unit-typed.

Operator precedence, highest first:

| Level | Operators | Notes |
|---|---|---|
| Postfix | `x.f`, `x[i]`, `f(x)`, `x?`, `x?.f`, `x as T` | |
| Unary | `-x`, `~x`, `ref x`, `ref mut x` | `not` is lower, below comparison |
| Multiplicative | `* / % *% /%` | |
| Additive | `+ - +% -% +\| -\|` | `%` suffix wraps, `\|` suffix saturates |
| Shift | `<< >>` | mixing with arithmetic requires parentheses |
| Bitwise | `& ^ \|` | mixing with each other requires parentheses |
| Range | `.. ..=` | |
| Comparison | `== != < <= > >=` | non-associative: `a < b < c` is an error |
| `not` | | |
| `and` | | |
| `or` | | |
| `??` | null coalescing | |
| `else` | `expr else { ... }` fallback | |
| Assignment | `= += -= ...` | statements, not expressions |

Rules worth noting:

- Mixing bitwise and arithmetic operators, or `and`/`or`, without
  parentheses is an error. C's precedence between `&` and `==` has been a bug
  source for fifty years; the fix is to refuse to guess.
- Assignment is a statement. `if x = y` is a parse error, not a bug.
- `a < b < c` is an error. Python's chained comparison is nice but rare;
  C's silent `(a < b) < c` is a bug.
- `==` always means structural equality via the `Eq` trait. There is no
  `===`, no identity comparison on values. `Shared.same(a, b)` compares
  identity of shared references.

## Control flow

```sisu
if cond { } else if cond { } else { }
if let Pattern = expr { } else { }
let Pattern = expr else { diverge }
match expr { pattern if guard -> expr  ...  }
while cond { }
while let Pattern = expr { }
for pattern in iterable { }
loop { break value }
outer: for x in xs { for y in ys { continue outer } }
defer { cleanup() }          // runs at end of the enclosing *block*
```

No parentheses around conditions; braces are mandatory. There is no
`switch` with fallthrough, no `goto`, no `do-while`, no C-style `for(;;)`,
no ternary (`if` is an expression). `defer` is block-scoped, not
function-scoped; Go's function-scoped `defer` inside loops is a documented
trap.

## Closures

```sisu
{ x -> x + 1 }
{ x: int, y: int -> x + y }     // parameter types when inference needs help
{ -> compute() }                // zero parameters
{ x -> 
    let y = x * 2
    y + 1
}
xs.map { x -> x * 2 }           // trailing closure
xs.fold(0) { acc, x -> acc + x }
own { x -> consume(captured) }  // capture by move instead of by reference
```

One closure syntax, used inline and trailing. Kotlin's form was chosen over
Rust's `|x|` and Swift's `{ x in }` because it reads the same in both
positions and because `->` already means "maps to" in `match` arms and
function return types. There is no implicit `it` parameter (open question
Q-25).

## Blocks vs. closures

`{ ... }` in expression position is a block. `{ ... }` in argument position,
or with a `->`, is a closure. The only ambiguity, a zero-parameter closure
versus a block passed where a function is expected, is resolved by the
expected type: if the parameter type is a function type, it is a closure.

## Types

```sisu
int  uint  i8 ... i128  u8 ... u128  f32  f64  bool  char  str  String
T?                 // Option[T]
T ! E              // Result[T, E] with error set E
T !                // Result[T, inferred]
ref T  ref mut T   // borrows
own T              // only in parameter position; elsewhere T is owned
List[T]  Array[T, N]  Slice[T]  Map[K, V]  Set[T]
(A, B)  ()         // tuples, unit
fn(A, B) -> C      // function type
dyn Trait          // trait object (unsized; used behind ref or Box)
Box[T]  Shared[T]  Weak[T]  Cell[T]  Mutex[T]
Never              // the type of expressions that do not return
```

Type expressions are read left to right and `?` binds tighter than `ref`
(so `ref T?` is "optional reference"), because optional references are
common and references to optionals are a code smell.

## Things deliberately absent

| Absent | Present instead | Reason (see decision log) |
|---|---|---|
| Semicolons | newlines | D-020 |
| `null` | `T?` | D-010 |
| `++`, `--` | `+= 1` | D-021: two spellings of one thing, and `i++ + ++i` |
| Ternary `?:` | `if` expression | D-021 |
| Implicit numeric conversion that loses information | explicit methods | D-016 |
| Operator definition | overloading of the fixed set via traits | D-022: Scala/Haskell symbol soup |
| Custom precedence | none | D-022 |
| Properties / getters | fields and methods | D-023: hidden calls |
| Inheritance | traits, composition, delegation | D-012 |
| Exceptions | `T ! E` | D-011 |
| `async`/`await` | tasks and scopes | D-014 |
| Macros | `comptime` and reflection | D-018 |
| Header files, forward declarations | module system | D-019 |
| `goto` | labelled `break`/`continue` | D-021 |
| Variable shadowing across scopes | error | D-024: Rust's shadowing is loved by some; the evidence on bugs is against it. Shadowing within pattern bindings (`let x = x?`) is allowed. |

## Formatting

`sisu fmt` has no configuration. Indentation is four spaces, line width is
100 columns, trailing commas in multi-line lists, one blank line between
items. The formatter also sorts and prunes `use` lines. The research is
unanimous (Go, Rust, Zig, Elixir, Black for Python, Prettier for JS) that a
single enforced style removes a category of review discussion at no cost
anyone has been able to measure. Tabs versus spaces was decided by coin flip
and is not an open question.

## Grammar notes

The grammar is LL(k) for small k with no symbol-table feedback: the parser
never needs to know whether a name is a type to parse an expression. This
is why generics use `[]` rather than `<>` (the `a < b > (c)` problem that
forced Rust's `::<>` turbofish and C++'s `template` disambiguator), why map
literals use `[k: v]` rather than `{}`, and why closures use `{ x -> }`
rather than `(x) -> ` (which would be ambiguous with tuples). The full
grammar will be published with the specification; a parser should be
writable in a weekend.
