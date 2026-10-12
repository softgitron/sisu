# 03 - Types and generics

## Summary of decisions

| Decision | Choice | Main evidence |
|---|---|---|
| Static vs. dynamic | Static, sound, no `any` | Dart's reversal from optional typing; TypeScript's `any` escape hatch being the top source of runtime errors; Gao et al. 2017 (15% of bugs caught by types) |
| Inference | Local and bidirectional; signatures explicit | Go, Rust, Swift, Kotlin, Carbon consensus; OCaml/Haskell error-locality problems; Crystal's whole-program compile cost |
| Nominal vs. structural | Nominal types; explicit trait implementation | Go's accidental interface satisfaction; Rust/Swift coherence |
| Sum types | First-class `enum` with payloads, exhaustive `match` | ML family; Rust, Swift, Kotlin sealed classes; Go's and Java's belated additions |
| Null | `Option[T]` with `T?` sugar | Hoare; Kotlin and Swift ergonomics; Java `Optional` retrofit failure |
| Polymorphism | Traits (bounded parametric), monomorphised, with explicit `dyn` | Haskell typeclasses, Rust traits, Swift protocols; C++ template error messages; Zig's instantiation-time errors |
| Inheritance | None | "composition over inheritance" consensus, Go, Rust, Kotlin final-by-default |
| Higher-kinded types | Not in version 1; syntax reserved | Rust gets by; Scala/Haskell show the cost; open question Q-10 |
| Integers | `int` is 64-bit and the default; sized types for layout; overflow traps | Swift's `Int`; Rust `usize` casting friction; overflow CVE data |
| Floating point | IEEE 754, evaluated strictly: no implicit fused multiply-add, no reassociation, fixed-shape reductions; `@float(fast)` opts out per function | Java 17's strict floating point (JEP 306); WebAssembly's deterministic scalars; Box2D v3's cross-platform determinism |
| Exact arithmetic | `BigInt`, `Ratio` and `Decimal` in `std.num`, with ordinary literals; `int` stays fixed-width | Python's `int`, `fractions` and `decimal`; Java's `BigDecimal`; Julia's `BigInt` and `Rational` |
| Strings | UTF-8; `str` view / `String` owned; no integer indexing | Go, Rust, Zig convergence; Swift's correctness vs. ergonomics lesson |

## Type inference

Function signatures, struct fields, trait members, constants and anything
`pub` are explicitly typed. Within a function body, `let` bindings, closure
parameters and generic arguments are inferred bidirectionally: information
flows both from the expression to the binding and from the expected type
to the expression. This is what lets `let xs: Array[int, 3] = [1, 2, 3]`
pick the array type and `[1, 2, 3]` alone pick `list[int]`.

Inference never crosses a function boundary. The evidence:

- OCaml and Haskell's global inference produces errors far from their cause
  and makes signatures an afterthought; both communities recommend writing
  top-level signatures anyway (see
  [`../research/languages/ocaml.md`](../research/languages/ocaml.md),
  [`haskell.md`](../research/languages/haskell.md)).
- Crystal's whole-program inference makes separate and incremental
  compilation impossible ([`crystal.md`](../research/languages/crystal.md)).
- Swift's bidirectional solver, when combined with overloaded operators and
  literal polymorphism, produces "expression too complex" timeouts
  ([`swift.md`](../research/languages/swift.md)). Sisu avoids the trigger:
  numeric literals default to `int`/`f64` early, operators are not
  overloaded on return type alone, and the solver is required to be
  polynomial in expression size (a design constraint on the trait system).

Integer literals without context are `int`. Float literals are `f64`. A
literal takes another numeric type only when the expected type says so,
which is also how `BigInt`, `Ratio` and `Decimal` take literals with no
suffix (see [Numbers](#numbers)).

## Algebraic data types

`struct` is a product; `enum` is a sum. Enum variants may be unit, tuple-like
or record-like, exactly as in Rust and Swift. Variants are namespaced under
the enum but can be used unqualified in a `match` on that enum and wherever
the expected type is known, which removes Rust's `use Enum::*` dance without
Swift's leading-dot syntax.

```sisu
enum Json:
    Null
    Bool(bool)
    Number(f64)
    String(String)
    Array(list[Json])
    Object(dict[String, Json])
```

Recursive enums are allowed without an explicit box: the compiler inserts
the indirection for variants that would otherwise be infinitely sized and
reports it in documentation. This was chosen over Rust's explicit `Box`
after noting that every beginner hits "recursive type has infinite size" and
that the fix is mechanical.

`match` is exhaustive. Missing-case errors list the missing patterns. A
`_` wildcard is allowed but the linter warns when matching on an enum from
the same package, since a new variant should be a compile error there.

Enums may be `@repr(u8)` for FFI and may declare explicit discriminants.
There is no separate "C-style enum" feature: an enum with only unit variants
is one.

## Option

`T?` is sugar for `Option[T]`, an ordinary enum with variants `Some(T)` and `None`. The
sugar exists because optionals are pervasive, and the Kotlin and Swift
experience is that `?.`, `??` and `if let` make them pleasant where Java's
`Optional` method chains do not.

Layout optimisation: `Option[ref T]`, `Option[Box[T]]`, `Option[NonZero]`
and `Option` of an enum with spare discriminants are the same size as the
payload (Rust's niche optimisation).

## Error sets

See [05-errors.md](05-errors.md). From the type system's point of view, an
`error` declaration defines a set of variants; any set is a subtype of any
superset; `T ! E` is `Result[T, E]`.

## Traits

A trait declares required and provided methods, associated types, and
associated constants.

```sisu
trait Iterator:
    type Item
    def next(mut self) -> Item?                                   # required

    def map[U](own self, f: (Item) -> U) -> MapIter[Self, U]: ...  # provided; Iter[T] is `dyn Iterator` with Item = T
    def count(own self) -> int: ...
```

Implementation is explicit: `impl Trait for Type`. Go's implicit
satisfaction was rejected because the research shows two concrete problems:
accidental satisfaction with wrong semantics, and the impossibility of
attaching trait-specific documentation or default methods to the
implementing type. The compiler offers a fix-it when a type has all the
methods of a trait but no `impl`.

### Coherence

There is at most one implementation of a trait for a type, globally. An
`impl` must live in the package that defines the trait or the package that
defines the type (the orphan rule). This is Haskell's and Rust's rule and it
is what makes `dict[K, V]` behave the same everywhere. The cost, that you
cannot implement a foreign trait for a foreign type, is paid for with cheap
newtypes and delegation:

```sisu
@delegate(Add, Sub, PartialOrd, to="value")   # forward these traits to the wrapped field
struct Celsius:
    value: f64
```

Scoped instances (Scala `given`, OCaml's proposed modular implicits) are the
alternative; they are listed as open question Q-11.

### Dispatch

Generic functions are monomorphised: `def f[T: Display](x: T)` is compiled once
per `T` used. Dynamic dispatch is opt-in and visible: `dyn Display` is a type,
unsized, used behind `ref` or `Box`. A trait is object-safe (usable as
`dyn`) under the same rules as Rust; the compiler says why when it is not.

Monomorphisation costs compile time and binary size (see
[`rust.md`](../research/languages/rust.md); Graydon Hoare lists it among the
things he would not have chosen). Sisu keeps it because the alternatives
(dictionary passing, as in Swift's unspecialised generics and Haskell) make
performance unpredictable, which violates priority 3. Two mitigations are
part of the design:

1. The compiler shares instantiations whose types have the same layout and
   do not call trait methods differently (Go's GC-shape stenciling, Rust's
   polymorphisation RFC that never landed), which removes most duplication
   for pointer-shaped types.
2. A per-package instantiation budget, reported by `sisu build --explain-size`.

### Auto traits

`Send`, `Sync`, `Sized` and `Escapable` are inferred structurally:
a struct is `Send` if all its fields are. `Copy` is the exception: the
compiler infers *eligibility* and suggests it, but a type is `Copy` only
when it asks to be. This is Rust's quiet success:
thread safety composes without annotations. Swift adopted the same idea as
`Sendable` in 2021-2024; the research notes that its retrofit was painful
precisely because the inference was not there from the start.

`Copy` is opt-in via `@derive(Copy)` and only for types whose fields are all
`Copy`; the compiler suggests it for small plain structs. The alternative,
Swift's implicit copying of everything with copy-on-write under the hood,
was rejected because it hides allocation (priority 3). The trade-off is
recorded as open question Q-3.

### Generic syntax

```sisu
def max[T: Ord](a: T, b: T) -> T: ...
def show_all[T](xs: list[T]) where T: Display + Hash: ...
struct Grid[T, const W: int, const H: int]:
    cells: Array[T, W * H]
struct Stack[T, A: Allocator = Global]:   # a defaulted type parameter (leaning, D-060)
    items: list[T, A]
impl[T: Display] Display for list[T]: ...
```

Square brackets avoid the `<` ambiguity and are already used by Go, Scala,
Nim, Python and Carbon. Const generics are supported from the start because
arrays need them and Rust's late addition left a decade of `[T; N]`
workarounds.

A generic type's parameter may have a default, written after its bound.
Defaulted parameters come after the others, and a use site that omits one
gets the default, so `Stack[int]` is `Stack[int, Global]`. The standard
collections are declared this way, as `list[T, A: Allocator = Global]`:
everyday code never names an allocator, and a list in a borrowed bump
region is `list[Header, ref Region]`, which holds a `ref` and is therefore
a view under the rules of [04](04-memory-and-ownership.md). An omitted
parameter means the default everywhere, signatures and `impl` headers
included, so code meant to work on region-backed collections is generic
over the allocator as well:
`impl[T: Display, A: Allocator] Display for list[T, A]`.
The number of corpus functions that have to become allocator-generic is
the cost that settles D-060, since Go shelved its arena experiment because
explicit arenas spread through APIs; the parameter goes in before 1.0
because Rust needed ten years, from RFC 1398 to Rust 1.100, to add it to
`Vec` ([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
candidate 8).

### Trait objects vs. enums

The guidance, enforced by nothing but documented: closed sets of variants
are enums; open sets of behaviours are traits. The research on Java
(`instanceof` chains) and on Go (type switches on interfaces) shows what
happens when a language has only one of the two.

## Numbers

| Type | Width | Use |
|---|---|---|
| `int` | 64 | the default; indices, lengths, counts |
| `uint` | 64 | bit manipulation, sizes from FFI |
| `i8 i16 i32 i64 i128`, `u8 u16 u32 u64 u128` | as named | layout, protocols, FFI |
| `f32 f64` | IEEE 754 | |

Decisions:

- **64-bit `int` as the default and the index type.** Rust's `usize` for
  indices causes a steady stream of casts (`as usize`) and is the single
  most common `as` in Rust code; Swift uses `Int` for everything and nobody
  complains. On 32-bit targets `int` is still 64-bit; `isize`/`usize`
  equivalents exist as `ptrsize`/`uptrsize` for FFI only. A negative index
  computed at run time traps, which doubles as the bounds check; a
  constant one is a compile error with the fix-it `xs[^1]`, as
  [02](02-syntax.md) describes (leaning, D-062).
- **Overflow traps in all build modes.** Rust wraps in release, which is a
  documented footgun and the subject of ongoing RFC debate; Zig makes it
  undefined behaviour in `ReleaseFast`; Swift traps always and the measured
  cost is a few percent on arithmetic-heavy code, zero elsewhere. The
  security evidence on integer-overflow CVEs (see
  [`../research/evidence/memory-safety.md`](../research/evidence/memory-safety.md))
  is what decides it. Wrapping, saturating and checked operations are
  spelled: `+%`, `+|`, `.checked_add()`.
- **Implicit widening only.** `i32` to `int`, `u8` to `i32`, `f32` to `f64`
  happen silently; they cannot lose information. `int` to `i32`, `f64` to
  `int`, signed to unsigned require a method call. Rust's refusal of even
  lossless widening is a known paper-cut; C's silent lossy conversions are
  a known bug source; Zig and Swift sit in the middle and Sisu follows them.
  Whether implicit widening complicates inference is open question Q-13.
- **Division by zero and `int.MIN / -1` trap.** Shifts by more than the
  width trap.
- **Floats** are never implicitly converted to or from integers. `f64`
  implements `PartialOrd` not `Ord`, so a `dict[f64, _]` needs an explicit
  total-order wrapper (`f64.total_order()`); ordering algorithms such as
  `sort_by` and `max_by` accept `PartialOrd` keys and panic on an
  unordered comparison, so NaN handling is documented rather than
  surprising.
- **Floating point is strict** (D-074, decided with Q-44). One source gives
  the same bits on every target, at every optimisation level and with any
  number of workers, which is what lets `comptime` run float code exactly
  as run time would ([08](08-metaprogramming.md)) and keeps `pure`
  deterministic ([06](06-concurrency-and-capabilities.md)). Each operation
  rounds once: there is no implicit fused multiply-add, and
  `a.mul_add(b, c)` is the one-rounding form, written where it is wanted.
  There is no reassociation and no fast-math mode, and NaN payloads are
  canonical wherever a program can observe them. `sum`, `product` and
  `dot` are fixed-shape pairwise reductions, the same tree sequentially
  and in `std.par`, whose chunks are fixed by index rather than by worker
  count; pairwise summation is also more accurate than a left fold.
  `@float(fast)` on a function allows contraction and reassociation inside
  it and visibly gives up the guarantee there. Java made all floating
  point strict in Java 17 (JEP 306), after more than twenty years of a
  non-strict default that existed for x87; Clang contracts within a
  statement by default, the kind of default that makes x86 and ARM
  disagree; Box2D v3 reached cross-platform determinism with these rules
  ([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
  candidate 4). A correctly rounded `std.math`, in which `sin(x)` depends
  only on `x` and not on the platform's maths library, waits for the
  numerics research of Q-23, which is open.
- **Exact arithmetic is a library, not a mode** (D-076). `BigInt`
  (integers of any size), `Ratio` (exact fractions) and `Decimal` (base-10
  values held exactly) are `std.num` types ([11](11-stdlib.md)), written
  in Sisu as `pure` code, so they also work at compile time. They take
  ordinary literals with no suffix through `FromLiteral`, and a literal
  reaches them as written: `0.1` as a `Decimal` is exactly one tenth, not
  the nearest binary fraction, and a `BigInt` literal too large for `int`
  is checked at compile time. The fixed operator set (D-022) gives them
  the ordinary operators. `int` stays fixed-width and trapping (D-009): an
  unbounded `int` by default, as in Python and Ruby, would hide allocation
  inside arithmetic, which priority 3 rules out. The semantics of
  `Decimal` (unbounded under a precision context, a fixed decimal128, or a
  scaled `BigInt` whose division yields a `Ratio`) and how far "symbolic"
  goes are open (Q-53); symbolic mathematics is a `std.x` package after
  1.0, and the numeric traits the exact types implement come from Q-23.

```sisu
let e = a * b + c                     # two roundings, on every target and at every optimisation level
let f = a.mul_add(b, c)               # one rounding, only when asked for
let t = xs.par_iter().map(it * w).sum()   # a fixed pairwise tree: the same bits on 1 or 64 workers

@float(fast)                          # contraction and reassociation allowed; outside the guarantee
def blend(mut out: Slice[f32], a: Slice[f32], b: Slice[f32]): ...

let n: BigInt = 123456789012345678901234567890   # no suffix; checked at compile time
let tenth: Decimal = 0.1                          # exactly one tenth
```

## Strings and text

- `str` is an immutable view of valid UTF-8 (a `view` type, `Copy`).
  `String` is owned and growable, and views as `str` wherever a `str` is
  expected.
- No integer indexing on `str`. `s.bytes()`, `s.chars()` (Unicode scalar
  values) and `s.graphemes()` are iterators; `s.byte_slice(a, b)` is
  explicit and checks boundaries. Swift's grapheme-first string is the most
  *correct* design in any language and also the one with the most
  complaints about ergonomics and performance; Rust's scalar-value model is
  the pragmatic middle and Sisu takes it, with graphemes one method away.
- `char` is a Unicode scalar value (Rust, Swift's `Unicode.Scalar`).
- Interpolation `"{x}"` uses `Display`; `"{x:?}"` uses `Debug`. Format
  specs follow Python/Rust: `{x:>10.2}`.
- Byte strings `b"..."` are `bytes`, an immutable owned byte sequence that
  views as `Slice[u8]`.
- `String` is not `Send`-restricted or special in any way; it is a `list[u8]`
  with a validity invariant.

## Tuples, unit, never

`(A, B)` tuples with `.0`, `.1` access and destructuring. `()` is the unit
type and the return type of functions with no `->`. `Never` is the type of
`panic(...)`, `return`, `break`, `continue`, and `while True` without `break`; it
coerces to any type.

## Function types

`(A, B) -> C` is the type of a function or closure that takes an `A` and a
`B` and returns a `C`; a fallible one returns `C ! E`. `pure` is part of
the type (D-074, decided with Q-43): `(A) -> B` admits any function, and
`pure (A) -> B` only one the checker has proved pure. A `pure` function
that takes a function parameter must type it `pure`, so `fold`'s callback
in `pure def fold(self, init: B, f: pure (B, T) -> B)` keeps `fold` pure;
a plain function type in a parameter makes the function impure, since the
checker cannot see what it will call. Higher-order `pure`
code needs the distinction, because a function that calls a callback is
only as pure as the callback, and a closure can carry a capability it
captured.

```sisu
pure def apply_twice(f: pure (int) -> int, x: int) -> int:
    return f(f(x))

let y = apply_twice(n -> n * 3, 2)          # 18: the closure is pure
```

A closure is pure when its body uses no capability, no `unsafe`, no
shared mutation and nothing that observes the schedule
([06](06-concurrency-and-capabilities.md) lists what `pure` excludes); that
is inferred, like everything else inside a body. Passing an impure
function where a `pure` one is required is an error that names the
operation responsible. Whether the resource
attributes `@no_alloc`, `@no_block` and `@stack(max=N)` (leaning, D-059)
and closure capture kinds also enter function types is open (Q-52).

## Type aliases and newtypes

`type Id = int` is a transparent alias. A one-field `struct Id:` with
`value: int` is a distinct type with no runtime cost. The research on Haskell (`newtype`), Rust
(`struct X(T)`), Ada (derived types), and F# (units of measure) agrees that
cheap distinct types prevent a real bug class (mixing up units and IDs);
Sisu makes the pattern cheaper with `@delegate` and with `@derive(Into,
From)`.

## Refinement subtypes

```sisu
type Port = int where 1 <= self <= 65535
type NonEmpty[T] = list[T] where not self.is_empty()
```

A refined type is a subtype of its base; constructing one checks the
predicate (at compile time when the value is constant). This is Ada's
subtype with a predicate, in Sisu syntax. It is the lightweight end of
dependent typing and is discussed in [07-contracts.md](07-contracts.md).

## What is not in the type system

- No implicit conversions except lossless numeric widening and owned-to-view;
  *literals* take their expected type (`"x"` as a `Path`, `[1, 2]` as an
  `Array`, `0.1` as a `Decimal` through `FromLiteral`), which is typing,
  not conversion.
- No subtyping between structs; no inheritance; no variance annotations
  needed by users (the only variance is in `ref`, error sets, refined
  types and `pure` function types, all handled by the compiler).
- No effect system: `pure` is the one tracked property (Q-17), and it is
  part of function types ([above](#function-types)).
- No higher-kinded types in version 1; the syntax `F[_]` is reserved.
- No dependent types; refinement types and contracts cover the common
  cases, and open question Q-16 covers the rest.
- No type-level computation beyond `const` generics and `comptime` functions
  producing types (see [08-metaprogramming.md](08-metaprogramming.md)).
  The TypeScript and C++ experience is that Turing-complete type-level
  programming becomes a language of its own, with its own debugging
  problems.
