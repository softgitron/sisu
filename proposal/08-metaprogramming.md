# 08 - Metaprogramming

## Decision

Sisu has **compile-time evaluation of ordinary code** and **typed
reflection**, and no macro system in version 1.

| Mechanism | Sisu | Rationale |
|---|---|---|
| Textual macros (C preprocessor) | No | Universally regretted; C23 and C++ both keep trying to replace it |
| Pattern macros (Rust `macro_rules!`) | No | A second language with its own syntax, hygiene rules and error messages |
| Procedural/token macros (Rust proc macros, Swift macros) | No | Opaque to tooling, slow to compile (separate crate, separate compilation, often a syn/quote parse per invocation), and the main contributor to Rust compile-time complaints; Swift's macros reproduced the compile-time cost |
| AST macros (Lisp, Elixir, Nim, Julia) | Not in v1 | The most powerful form; also the one with the strongest evidence that it fragments ecosystems into DSLs ("the Lisp curse"). Open question Q-15 |
| Compile-time function evaluation | **Yes** (`comptime`) | Zig, D (CTFE), C++ `constexpr`/`consteval`, Jai; the Zig experience shows it covers generics, lookup tables, derive and conditional compilation without a second language |
| Reflection at compile time | **Yes** | Zig's `@typeInfo`; what makes `derive` a library function |
| Reflection at runtime | Minimal (`type_name`, enum variant names) | Java/C# style runtime reflection breaks dead-code elimination and the capability model |
| Templates (C++) / duck-typed comptime generics (Zig) | No for generics | Instantiation-time errors; Sisu's generics are trait-checked at definition ([03-types-and-generics.md](03-types-and-generics.md)) |

## Compile-time evaluation

Any function whose parameters are all compile-time known and which uses no
capabilities, no `unsafe`, and no `Shared` can be evaluated at compile time.
`comptime` is the keyword that *requests* it:

```sisu
def crc_table() -> Array[u32, 256]: ...            # an ordinary function
const CRC: Array[u32, 256] = comptime crc_table()   # evaluated by the compiler

comptime def assert_power_of_two(n: int):           # may only be called at compile time
    assert n > 0 and n & (n - 1) == 0, "{n} is not a power of two"

struct RingBuffer[T, const N: int]:
    items: Array[T, N]

    def empty() -> Self:
        comptime assert_power_of_two(N)
        ...
```

Rules:

- The evaluator is an interpreter over the compiler's typed IR, not a
  separate language. What runs at compile time is exactly what would run at
  run time, with the same semantics (same integer widths, same overflow
  traps, same `Drop` order, same `dict` and `set` iteration order, same
  floating-point results). The last two hold because of D-074: `dict` and
  `set` iterate in insertion order (the per-process hash key still resists
  flooding, but it no longer decides the order), and floating point is
  strict, with no implicit fused multiply-add and no reassociation, so a
  `comptime` result has the same bits as the run-time result on every
  target, except inside a function that opts out with `@float(fast)`. The
  correctly rounded `std.math` that carries this to `sin` and `exp` waits
  for Q-23, which is open.
- There is no I/O at compile time. No file reads, no environment, no
  network, no clock. A build that evaluates the same source produces the
  same output. Zig allows `@embedFile`; Sisu provides `embed("path")` as a
  compiler intrinsic with the path resolved relative to the package root and
  recorded in the build's input set, which keeps builds reproducible.
- Evaluation has a fuel limit per expression and a memory limit. Fuel is
  counted in steps of the typed IR (SIR steps). Exhausting it raises a
  deny-by-default diagnostic that names the expression and the package that
  spent the fuel (leaning, D-063), and a long evaluation prints its progress
  instead of looking like a hang. The budget belongs to the package that
  spends it: `@comptime_budget(steps=N)` on the item raises that item's own
  limit, so a dependent never edits its manifest for a dependency's internals.
  Rust 1.72 removed its const-evaluation cap for exactly that reason, because
  whether code hit the limit depended on the libraries it invoked. C++'s and
  Zig's experience is that unlimited compile-time computation becomes the
  build-time problem, and the designers of Dhall and Starlark concede that
  totality does not bound running time.
- `const` items and array-length expressions require compile-time
  evaluation; everything else is opportunistic (the optimiser may
  constant-fold, but the semantics do not depend on it).

A package that needs more fuel says so on the item that spends it:

```sisu
@comptime_budget(steps=50_000_000)       # this package's tables are expensive
const TABLES: CaseTables = comptime build_tables(embed("data/UnicodeData.txt"))
```

## Reflection

```sisu
comptime def field_names(T: type) -> list[str]:
    return [f.name for f in reflect.fields_of(T)]

def to_json[T](value: T) -> String:
    if comptime reflect.is_struct(T):                # narrows T inside the branch
        var out = String("{{")                     # doubled: a lone brace opens an interpolation
        comptime for f in reflect.fields_of(T):      # unrolled at compile time
            out.push("\"{f.name}\":{to_json(value.[f])}")   # field access by compile-time name
        out.push("}}")
        return out
    else:
        return "{value}"                              # numbers, strings and booleans via Display
```

`reflect` is a compile-time-only module, and `target` (`target.os`,
`target.arch`) describes the build target. An `if comptime` on a fact about
`T` narrows `T` inside the branch, which is the one way a generic body may
depend on the shape of its type argument; both branches still type-check. It exposes types, fields, variants,
methods, attributes, and the documentation of items, as values. It cannot
create new *declarations*; it can only produce values and types for use in
expressions. This is the line that keeps reflection from becoming a macro
system: a `comptime` function can compute an implementation, but it cannot
inject a new `struct` into your module.

### Derive

`@derive(Eq, Hash, Debug)` is sugar for calling the trait's `derive`
comptime function, which uses reflection to build the implementation:

```sisu
trait Hash:
    def hash(self, mut h: Hasher)

    comptime def derive(T: type) -> impl Hash for T:
        return impl Hash for T:
            def hash(self, mut h: Hasher):
                comptime for f in reflect.fields_of(T):
                    self.[f].hash(mut h)
```

`return impl ... for T:` with an indented body is an expression form (a
trailing block, like `do`), and it is the one place where compile-time
code produces a declaration; it is constrained: it can
only produce an `impl` of the enclosing trait for the type it was given,
which preserves coherence. User-defined traits get `derive` the same way;
there is no privileged set.

## Conditional compilation

```sisu
if comptime target.os == "linux":
    ...
else:
    ...

@cfg(test)
def helper(): ...

@cfg(feature="tls")
import tls
```

`@cfg` on items and `if comptime` in bodies. Both branches of `if comptime`
must type-check (unlike C `#if`), which keeps dead platforms compiling. The
Zig experience is that this is enough and that `#ifdef` soup is avoidable.

## Configuration files

`sisu eval` makes Sisu its own configuration language. It takes a module of
`const` items and `pure` functions, evaluates it with the comptime interpreter
and renders the result as JSON, TOML or YAML through `Encode` (leaning,
D-063). The types, refinements and invariants the module declares are the
schema: a port outside its range or a replica count of zero fails evaluation
with a located diagnostic instead of reaching a deployment.

```sisu
## deploy/prod.sisu, rendered by `sisu eval deploy/prod.sisu --to yaml`
type Port = int where 1 <= self <= 65535

@derive(Encode, Decode)
struct Service:
    name: String
    port: Port
    replicas: int = 1
    invariant self.replicas >= 1

pub const SERVICES: list[Service] = [Service(name="web", port=8080, replicas=3)]
```

This is the Borg retrospective's recommendation (a data-only format, produced
by a real programming language) and Pkl's model (typed configuration with
constraints), without a second language or a Starlark-style dialect. It works
because `comptime` is already hermetic and deterministic: it has no I/O, and
`pure` means the same bits on every run (D-074). The budget above still
applies, because a configuration module is ordinary code and can run long. The
package manifest, by contrast, stays data
([09](09-modules-packages-and-build.md)); anything that has to be computed
lives in a module like this one.

## What you cannot do, and the intended workaround

| Want | Workaround |
|---|---|
| A new syntax (`sql!("...")`, `html! { }`) | A `comptime` function that parses a string literal and returns a typed value; the string is checked at compile time, errors point into it. Covers SQL, regex, format strings, routes. Not covered: new *statement* forms. |
| A new control structure | Closures with trailing blocks: `retry(3) do:` and `with_lock(m) do:` read like control structures. |
| Generating N similar functions | `comptime for` over a table, or generics. |
| Attributes with behaviour (`@test`, `@route("/x")`) | Attributes are data that `comptime` code and tooling can read via `reflect.attributes_of`; a framework collects routed functions by scanning its own package at compile time. |
| Serialisation for all types | `derive` via reflection; the Zig standard library's JSON encoder is the existence proof that this covers what `serde` covers, without macros. |
| A configuration language (Starlark, Dhall, Pkl) | A module of `const` items and `pure` functions, rendered by `sisu eval`; see Configuration files above. |

## Evidence summary

- Zig ([`zig.md`](../research/languages/zig.md)) is the strongest case
  that comptime plus reflection replaces macros *and* templates for most
  purposes, with one language, excellent tooling, and fast compilation. Its
  weakness, errors at instantiation, comes from using comptime for
  generics, which Sisu does not.
- Rust ([`rust.md`](../research/languages/rust.md)): proc macros are the
  most powerful mainstream metaprogramming and the single feature most often
  blamed for compile times, opaque errors and IDE difficulties; the
  ecosystem's heaviest users (`serde`, `tokio`, `sqlx`) could be served by
  reflection + comptime.
- Lisp family ([`lisp-scheme-racket.md`](../research/languages/lisp-scheme-racket.md)):
  AST macros are genuinely the most expressive tool in programming and
  genuinely correlate with ecosystem fragmentation. Deferred, not rejected.
- D ([`d.md`](../research/languages/d.md)): CTFE and string mixins; the
  designers' retrospective is that CTFE was right and string mixins were a
  mistake (untyped text generation).
- Dart ([`dart.md`](../research/languages/dart.md)): cancelled its macro
  system in 2025 after three years of work because of compile-time and
  tooling cost; a data point that macros are harder to ship than to design.
- Configuration languages
  ([`config-and-total-languages.md`](../research/unconventional/config-and-total-languages.md)):
  Pkl's typed model and the Borg retrospective's advice are what `sisu eval`
  is built from, and Rust 1.72's removal of its const-evaluation cap, with
  Dhall's and Starlark's admissions that totality does not bound running time,
  is why budgets belong to the package that spends them.
