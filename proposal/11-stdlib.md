# 11 - Standard library

## Decision

A **core** standard library (`std`) that is small, stable and shipped with
the compiler, plus a set of **blessed packages** (`std.x.*`) maintained by
the core team, versioned independently and installed on demand.

The evidence pulls in two directions and this split is the resolution:

- Go's large, high-quality standard library is one of the most cited
  reasons for its adoption, and it kept the ecosystem coherent (one HTTP
  server, one JSON encoder). Python's "batteries included" did the same for
  twenty years.
- Python's batteries then rotted (PEP 594 removed 19 dead modules in
  3.13); Java's `java.util.Date` and `Vector` are permanent. A standard
  library cannot be fixed without a breaking change to the language's
  compatibility promise.
- Rust's minimal `std` avoided rot but pushed essentials (`rand`, `regex`,
  `serde`, HTTP, async runtimes) to the ecosystem, where the async runtime
  split caused the fragmentation its survey complains about.

The `std.x` tier is Go's `golang.org/x` and Rust's "nursery" made official:
maintained by the same people, held to the same quality bar, but allowed to
have major versions.

## `std` (core, compatibility-promised)

| Module | Contents |
|---|---|
| `std.core` (prelude) | `Option`, `Result`, error sets, `Copy`/`Drop`/`Eq`/`Ord`/`Hash`/`Len`/`Contains`/`Display`/`Debug`/`Default`/`Into`/`From`, `Iterator`, `Range`, `Never`, `Box`, `Shared`, `Weak`, `Cell`, `panic`, `assert`, the sync and task primitives `Mutex`, `Atomic`, `channel`, `task`, the capability types, `Duration`, `Path`, and the prelude functions `print`, `print_err`, `len`, `range`, `enumerate`, `zip`, `sorted`, `reversed`, `min`, `max`, `sum`, `any`, `all`, `abs` |
| `std.collections` | `list`, `Array`, `Slice`, `dict` (hash), `SortedDict` (B-tree), `set`, `SortedSet`, `Deque`, `Heap`, `Arena` with generational keys (`Key[T]`), `BitSet`; `persistent.List`, `persistent.Dict`, `persistent.Set` (HAMT/RRB structural-sharing collections for data shared through `Shared` or sent between tasks without copying, from Clojure via Scala and Elm; [`clojure.md`](../research/languages/clojure.md)) |
| `std.string` | `String`, `str`, `char`, Unicode scalar and grapheme iteration, case mapping, normalisation forms, `Builder` |
| `std.fmt` | Formatting traits and the interpolation machinery |
| `std.num` | Integer and float traits, checked/wrapping/saturating ops, `BigInt`, `Decimal`, `Ratio` |
| `std.mem` | Allocator trait, `Arena`, layout; raw pointer types live in `std.ffi` |
| `std.task` | `scope`, `spawn`, `Handle`, `channel`, `Mutex`, `RwLock`, `Atomic`, `Condvar`, `Once`, cancellation, `shield` |
| `std.cap` | Capability types: `System`, `Fs`, `Net`, `Clock`, `Rand`, `Env`, `Proc`, their attenuations, and in-memory fakes for all of them |
| `std.io` | `Reader`, `Writer`, `BufReader`, `BufWriter`, `Seek`, byte and text streams |
| `std.fs` | Paths, files, directories, metadata, watching (via `Fs`) |
| `std.net` | TCP, UDP, Unix sockets, DNS, TLS client (via `Net`) |
| `std.time` | `Instant`, `Duration`, `DateTime`, time zones (data embedded, updatable via `std.x.tzdata`) |
| `std.rand` | CSPRNG and fast PRNG behind `Rand` |
| `std.test` | assertions, property testing with shrinking, snapshots, benches |
| `std.reflect` | compile-time reflection |
| `std.encoding` | UTF-8/16, base64, hex, varint |
| `std.hash` | SipHash (default), FNV, CRC32, SHA-2, BLAKE3 |
| `std.sort`, `std.search` | stable and unstable sort, binary search, selection |
| `std.process` | spawning, pipes, signals (via `Proc`) |
| `std.env` | arguments, variables, working directory (via `Env`) |
| `std.log` | structured logging facade with levels and context; one implementation |
| `std.json` | encoder/decoder with `@derive(Encode, Decode)`; `Decode` validates untrusted input against the type (including refinement predicates) at the boundary, so that "parse, don't validate" is the default and the TypeScript-style validator-library genre is unnecessary ([`typescript.md`](../research/languages/typescript.md)) |
| `std.regex` | linear-time (RE2-style) regex; no backtracking engine in core |

Design points:

- **`dict` iteration order is randomised** (Go, Rust): programs that depend
  on hash order break in production. `SortedDict` exists for when order
  matters; Python's insertion-order guarantee is not promised, because it
  lets code depend on it silently.
- **Hashing is SipHash with a per-process random key** (Rust, Python):
  hash-flooding DoS has been exploited in every language that used a fast
  predictable hash.
- **`list` growth is geometric, `with_capacity` exists, and `shrink_to_fit`
  is explicit.** No hidden reallocation beyond the documented amortised
  growth.
- **One logging facade, in core.** Rust's `log` crate and Go's `slog`
  (added in 1.21 after a decade of fragmentation) show that logging is a
  coordination point that the ecosystem will not converge on by itself.
- **JSON and regex in core** because every program needs them and
  because a linear-time regex engine is a security property (ReDoS) that
  should not be a dependency choice. Go made the same call.
- **Time zone data embedded**, because `DateTime` without zones is a bug
  generator, and updatable because zones change. Java's `java.time` (the
  JSR-310 redesign after `Date`/`Calendar`) is the model.
- **No HTTP server or client in core.** They move too fast. `std.x.http`
  is the blessed implementation.

## `std.x` (blessed, independently versioned)

`http` (client and server), `tls` (server side), `sql` (driver interface
and the common drivers), `cli` (argument parsing with derive), `toml`,
`yaml`, `csv`, `xml`, `compress`, `crypto` (beyond hashing), `tzdata`,
`unicode-data`, `image` (decoding), `wasm` (component model bindings),
`ffi-gen` (header import support), `verify` (the contract verifier, when
it exists).

## Principles for `std` API design

1. Capabilities in, values out. Any function that touches the world takes
   the capability as its first parameter.
2. Errors are specific error sets, never a stringly-typed `Error`.
3. No allocation without a visible owned return type.
4. Every type has `Debug`; public types have `Display` where there is an
   obvious rendering.
5. Iterators over views, not callbacks, for traversal (Rust's experience
   with external iterators versus Ruby's internal ones: external compose).
6. Names follow PEP 8 and Python's vocabulary where it exists (`len`,
   `append` is `push`, `get`, `items`, `keys`, `sorted`, `enumerate`), and
   the Rust/Go iterator vocabulary where Python has none (`iter`, `map`,
   `filter`, `fold`, `collect`).
   PHP's `str_replace` versus `strpos` argument-order inconsistency is the
   cautionary tale; the linter enforces a naming table on `std` itself.

## Compatibility

`std` follows the language compatibility promise
([13-evolution-and-governance.md](13-evolution-and-governance.md)):
nothing is removed; deprecations are warnings with `sisu fix` migrations;
replacements live beside the old thing. `std.x` packages follow semver with
the API-diff enforcement everyone else has.

## Additions proposed in October 2026

[19-novel-features.md](19-novel-features.md) asks the owner to reverse the
first design point above (Q-42): `dict` and `set` would iterate in
insertion order while keeping the per-process SipHash key, because three
analyses found that per-process order makes `pure` functions
non-reproducible and breaks the comptime promise, replay and memoisation.
Library shapes decided provisionally: `std.par` (`par_iter`,
`par_iter_mut`, `par.join`, reducers with a fixed merge order) and
`SoaList[T]` in `std.collections` (D-069); `list[T, A: Allocator =
Global]` with a bump `Region` distinct from `Arena[T]` (D-060); `task.race`,
a drift-free `Ticker` with an explicit missed-tick policy and a `Timeline`
with virtual-time sleep in `std.time`, `std.search`, and capability
interceptors (D-075); `exhaustive[T]`, transcript tests and `@refines` in
`std.test` (D-070); `View` in `std.x` (D-071); and, later in `std.x`,
revocable and lent capabilities, typed tables, units of measure with
`@diagnostic_name`, `std.x.peg`, `std.x.incr`, `std.x.reactor`, schema
locks with migrations, a `Model` capability with `derive(Schema)`, and a
first-party `sketch` package.
