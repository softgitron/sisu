# Strings and numerics

## Strings

| Model | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Bytes, no validity | C (NUL-terminated), Go (mostly) | Simple, fast | Buffer overflows (C); invalid UTF-8 leaks into logic (Go) | [c.md](../languages/c.md), [go.md](../languages/go.md) |
| UTF-16 code units | Java, JS, C#, Qt | Historical | Surrogate pairs break indexing; nobody would choose it today | [java.md](../languages/java.md), [javascript.md](../languages/javascript.md) |
| `[Char]` linked list | Haskell | Elegant | Universally regretted; five replacement libraries | [haskell.md](../languages/haskell.md) |
| Validated UTF-8, scalar-value iteration, no integer indexing | Rust, Zig, Swift (view) | Correct and fast; the convergence point | Two types (`String`/`&str`) confuse beginners for a week | [rust.md](../languages/rust.md), [zig.md](../languages/zig.md) |
| Grapheme-cluster default | Swift | Most correct for user-visible text | Slow; no `s[i]`; the most-complained-about part of Swift's API; `Substring` at every boundary | [swift.md](../languages/swift.md) |
| Autodecoding in the library | D | Attempted correctness in `front` | Regretted; semantics belong in types | [d.md](../languages/d.md) |
| Multiple string types | Ada | Fixed, bounded, unbounded, wide | Verbosity; the research says one UTF-8 type with views | [ada-spark.md](../languages/ada-spark.md) |

Findings: one validated UTF-8 type from day one with an owned and a view
form; scalar values as the default iteration unit with graphemes one call
away; no integer indexing; interpolation with format specs in the literal
(Python f-strings, C# handlers, Swift, Kotlin); raw and multi-line literals;
regex as a syntax-checked literal with a linear-time engine (Perl's one
transplanted idea; ReDoS as the reason for linear time)
([perl-php.md](../languages/perl-php.md)).

## Integers

| Decision | Options and evidence |
|---|---|
| Widths | C's implementation-defined `int` was portability in 1978 and a bug source since ([c.md](../languages/c.md)); fixed-width names plus one default. Rust's `usize` for indexing causes constant casts; Swift's `Int` (64-bit everywhere) is unremarked upon ([rust.md](../languages/rust.md), [swift.md](../languages/swift.md)). |
| Overflow | Silent wrap (C, Go, Java with opt-in `addExact`, Rust release) is a measured CVE class ([../evidence/memory-safety.md](../evidence/memory-safety.md)); UB (C signed, Zig `ReleaseFast`) is worse; trapping (Swift always, Rust debug) costs a few percent on arithmetic-heavy code. Explicit wrapping/saturating/checked operators (Zig `+%`, `+|`) make the choice per operation, which is the C "UB as a budget" lesson ([c.md](../languages/c.md), [zig.md](../languages/zig.md)). |
| Conversions | C's implicit lossy conversions and promotions are a bug class; Rust's refusal of even lossless widening is a paper cut; Zig and Swift allow lossless widening only. |
| Index base | 0 for a general-purpose language; Julia's 1-based and optional bases are liked by its audience and disliked by everyone else; Lua's designers would not choose 1 again ([julia.md](../languages/julia.md), [lua.md](../languages/lua.md)). |
| Negative indices | Mojo rejects them statically; Python's wrap-around is convenient and a silent-bug source ([mojo.md](../languages/mojo.md)). |
| Arbitrary precision | Python's default integers are safe and slow; a `BigInt` in the standard library is the compromise every static language chose. |

## Floats and arrays

- IEEE 754 with the precision in the type name (`f32`, `f64`; Fortran's
  `kind` parameter is the same idea) and no implicit integer-float
  conversion ([fortran.md](../languages/fortran.md)).
- `f64` is not totally ordered; the standard library should make that
  visible (`PartialOrd`, an explicit total-order wrapper for maps) rather
  than let NaN surprise.
- Multi-dimensional arrays with rank and shape in the type, element-wise
  expressions, and declared non-aliasing are why Fortran still vectorises
  where C cannot; Rust's exclusivity gives the backend the same `noalias`
  information with enforcement ([fortran.md](../languages/fortran.md)).
- APL's operations (rank-polymorphic elementwise ops, reduce/scan/outer,
  broadcasting) transplanted everywhere; its glyphs nowhere. NumPy and
  Julia's broadcasting dot are the conventional-syntax forms, and operator-
  level fusion (one loop for a map/filter/reduce chain) is a compiler goal
  Rust's iterator adapters and Julia's broadcast fusion both reach
  ([apl-bqn-k.md](../languages/apl-bqn-k.md)).
- Units of measure (F#) are excellent and unimitated, which suggests a
  library over phantom type parameters suffices
  ([fsharp-sml.md](../languages/fsharp-sml.md)).

## Recommendation for Sisu

`str`/`String`, UTF-8, scalar iteration, no integer indexing, graphemes via
iterator; `int` 64-bit default and index type with sized types for
layout; overflow traps always with `+%`/`+|`/`.checked_add()`; implicit
lossless widening only; 0-based, negative indices trap; `BigInt`,
`Decimal`, `Ratio` in `std.num`; arrays as a library over a good slice
type with SIMD vector types in `std.simd` and an n-dimensional array in
`std.x`. See [proposal/03](../../proposal/03-types-and-generics.md);
decisions D-009, D-016, D-034, D-035.

## Open questions

- Q-13 implicit widening vs inference; Q-14 grapheme default; Q-23 SIMD,
  rank-polymorphic arrays, broadcasting, and whether enforced
  non-aliasing (Rust) or trusted non-aliasing (Fortran) is right for
  numerical kernels with mutable sub-views.
