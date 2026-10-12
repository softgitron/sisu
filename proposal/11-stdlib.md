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

The library also carries features that other languages build into the
language. The owner's rule, given with Q-47 and recorded under P1 in
[00](00-vision-and-principles.md), keeps the core small: a feature goes
into the standard library wherever that is reasonable, and into the
language only when a library cannot express it or check it. Parallel
loops, exact arithmetic and lent capabilities are library types below for
that reason.

## `std` (core, compatibility-promised)

| Module | Contents |
|---|---|
| `std.core` (prelude) | `Option`, `Result`, error sets, `Copy`/`Drop`/`Eq`/`Ord`/`Hash`/`Len`/`Contains`/`Display`/`Debug`/`Default`/`Into`/`From`, `Iterator`, `Range`, `Never`, `Box`, `Shared`, `Weak`, `Cell`, `panic`, `assert`, the sync and task primitives `Mutex`, `Atomic`, `channel`, `task`, the capability types, `Duration`, `Path`, and the prelude functions `print`, `print_err`, `len`, `range`, `enumerate`, `zip`, `sorted`, `reversed`, `min`, `max`, `sum`, `any`, `all`, `abs` |
| `std.collections` | `list`, `Array`, `Slice`, `dict` and `set` (hashed, iterating in insertion order), `SortedDict` and `SortedSet` (B-tree), `Deque`, `Heap`, `Arena[T]` with generational keys (`Key[T]`), `BitSet`, `SoaList[T]` (one array per field of `T`); `persistent.List`, `persistent.Dict`, `persistent.Set` (HAMT/RRB structural-sharing collections for data shared through `Shared` or sent between tasks without copying, from Clojure via Scala and Elm; [`clojure.md`](../research/languages/clojure.md)); the collections that allocate take their allocator as a type parameter that defaults to `Global` (leaning, D-060) |
| `std.string` | `String`, `str`, `char`, Unicode scalar and grapheme iteration, case mapping, normalisation forms, `Builder` |
| `std.fmt` | Formatting traits and the interpolation machinery |
| `std.num` | Integer and float traits, checked/wrapping/saturating ops, and the exact types `BigInt`, `Ratio` and `Decimal` with suffix-free literals |
| `std.mem` | The `Allocator` trait, the `Global` allocator, the bump allocator `Region`, layout; raw pointer types live in `std.ffi` |
| `std.task` | `scope`, `spawn`, `Handle`, `race` (the first task to finish wins; leaning, D-075), `channel`, `Mutex`, `RwLock`, `Atomic`, `Condvar`, `Once`, cancellation, `shield` |
| `std.par` | `par_iter` and `par_iter_mut` over work-stealing chunks, `par.join`, reducers (`par.Sum`, `par.Collect`, the `par.Reducer` trait) with a fixed merge order |
| `std.cap` | Capability types: `System`, `Fs`, `Net`, `Clock`, `Rand`, `Env`, `Proc`, their attenuations, their lent (`Lent[T]`) and revocable forms, the interceptor `Wrap[T]` (D-075), and in-memory fakes for all of them |
| `std.io` | `Reader`, `Writer`, `BufReader`, `BufWriter`, `Seek`, byte and text streams |
| `std.fs` | Paths, files, directories, metadata, watching (via `Fs`) |
| `std.net` | TCP, UDP, Unix sockets, DNS, TLS client (via `Net`) |
| `std.time` | `Instant`, `Duration`, `DateTime`, time zones (data embedded, updatable via `std.x.tzdata`), and the drift-free `Ticker` and virtual-time `Timeline` over `Clock` (D-075) |
| `std.rand` | CSPRNG and fast PRNG behind `Rand` |
| `std.test` | assertions, property testing with shrinking, snapshots, benches; small-scope `exhaustive[T]` generators, `@refines` equivalence tests and transcript tests (leaning, D-070); see [17](17-testing.md) |
| `std.reflect` | compile-time reflection |
| `std.encoding` | UTF-8/16, base64, hex, varint |
| `std.hash` | SipHash (default), FNV, CRC32, SHA-2, BLAKE3 |
| `std.sort` | stable and unstable sort |
| `std.search` | binary search, selection, and search combinators over `Iter[T]`: fair interleaving, iterative deepening, bounded search (D-075) |
| `std.process` | spawning, pipes, signals (via `Proc`) |
| `std.env` | arguments, variables, working directory (via `Env`) |
| `std.log` | structured logging facade with levels and context, including the trace context a `task.scope` passes to its tasks (D-075); one implementation |
| `std.json` | encoder/decoder with `@derive(Encode, Decode)`; `Decode` validates untrusted input against the type (including refinement predicates) at the boundary, so that "parse, don't validate" is the default and the TypeScript-style validator-library genre is unnecessary ([`typescript.md`](../research/languages/typescript.md)) |
| `std.regex` | linear-time (RE2-style) regex; no backtracking engine in core |

Design points:

- **`dict` and `set` iterate in insertion order, and hashing stays SipHash
  with a per-process random key** ([D-074](15-decision-log.md), which
  reverses the iteration half of D-049). The key is the defence against
  hash flooding, which has been exploited in every language that used a
  fast predictable hash; Rust and Python key their SipHash for the same
  reason. The order is specified because three promises depend on it: a
  `pure` function returns the same bits for the same arguments in every
  process ([06](06-concurrency-and-capabilities.md)), `comptime` runs
  exactly what would run at run time ([08](08-metaprogramming.md)), and a
  replayed run repeats the recorded one. A per-process order breaks all
  three as soon as a `pure` function lists a `dict`'s keys
  ([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
  finding F1; [`config-and-total-languages.md`](../research/unconventional/config-and-total-languages.md),
  idea 1). Python 3.7 separated flooding resistance from iteration order
  the same way, and Ruby, JavaScript's `Map` and Starlark also iterate in
  insertion order. Programs break in production on an unspecified order,
  which is why Go and Rust randomise theirs; a specified order is safe to
  depend on. The cost is an index array and tombstones (Python 3.6's
  compact layout was still 20-25% smaller than its predecessor), and the
  target is to stay within 10% of a plain SipHash map on the corpus.
  `SortedDict` and `SortedSet` remain for key order.
- **`list` growth is geometric, `with_capacity` exists, and `shrink_to_fit`
  is explicit.** No hidden reallocation beyond the documented amortised
  growth.
- **Collections take their allocator as a type parameter with a default,
  `list[T, A: Allocator = Global]`** (D-060), so ordinary code never names
  one. A collection built on a borrowed bump `Region`,
  `list[T, ref Region]`, is a view: it cannot leave the frame that owns
  the region, which makes per-request and per-query memory safe without
  lifetime names ([04](04-memory-and-ownership.md) has the rules and the
  corpus measurement). The two names stay apart: a `Region` is an
  allocator whose memory is released all at once, an `Arena[T]` is a
  collection of one type addressed by `Key[T]` handles. The parameter is
  cheap before 1.0 and slow to add after; Rust merged the stabilisation of
  `Vec<T, A>` in September 2026, ten years after RFC 1398.
- **`SoaList[T]` makes struct-of-arrays layout a type** (D-069). A
  `comptime` function builds it from the fields of `T`, which is the type
  construction Q-27 allows: each field gets its own array, code is written
  against the logical struct, and element access returns a `view struct`
  of field references, which the ownership model already expresses. The
  layout stays visible where it is chosen; a `@layout(soa)` attribute on
  the struct would change what `list[Particle]` costs everywhere. Odin's
  `#soa` and Zig's `MultiArrayList` are the precedents
  ([`algorithm-schedule-separation.md`](../research/unconventional/algorithm-schedule-separation.md),
  idea 1), and the design holds if `SoaList[T]` with view elements passes
  the ownership corpus without `unsafe`.
- **Exact arithmetic is a set of `std.num` types, not a different `int`**
  (D-076). `int` stays 64-bit, fixed-width and trapping (D-009, D-035).
  `BigInt` holds integers of any size, `Ratio` exact fractions, and
  `Decimal` base-10 values exactly. They take the ordinary operators
  through the fixed operator set (D-022) and take literals without a
  suffix through `FromLiteral` ([03](03-types-and-generics.md)), so
  `let price: Decimal = 0.1` holds exactly one tenth and a 30-digit
  literal is accepted where a `BigInt` is expected, both checked at
  compile time. They are written in Sisu as `comptime`-eligible `pure`
  code, with host kernels in the interpreter. Python's `int`, `decimal`
  and `fractions`, Java's `BigDecimal` and Julia's `BigInt` and `Rational`
  are the precedents, and every language in the research that lacks such
  types grows a library for them. Two questions are open (Q-53): what
  `Decimal` means (a scaled `BigInt` whose division returns a `Ratio`
  unless a rounding context is given, an unbounded decimal under a
  precision-and-rounding context, or a fixed decimal128), and how far
  symbolic mathematics goes beyond exact rationals. The numeric traits
  that every exact type implements come from Q-23, which is also open.
- **Parallel loops are a library, `std.par`** (D-069, decided with Q-47).
  `par_iter`, `par_iter_mut` and `par.join` split work into work-stealing
  chunks with `Send` and `Sync` checked. Reducers give each worker its own
  view and merge the views in a tree fixed by the input length, not by
  which worker stole what, and `sum`, `product` and `dot` keep the fixed
  pairwise shape that strict floating point gives them
  ([03](03-types-and-generics.md)), so results have the same bits in
  every run and on any number of cores. The guarantee is serial elision
  made exact: removing the parallelism gives the same result, and a
  failing loop reports the lowest-index error. A `par` body is `pure` and
  cannot block, so it runs to completion on a worker's stack, on a pool
  apart from the task scheduler; concurrent I/O stays with `task.scope`.
  There is no `par:` block, because `task.scope` already is one, and no
  `par` keyword until the corpus count of Q-47 asks for one.
  [06](06-concurrency-and-capabilities.md) has the rules and the evidence.
- **`task.race` and search combinators are library functions, not
  syntax** (D-075). `task.race` is Céu's `par/or` over a scope: the first
  task to finish wins, and the others get `Cancelled` at their next
  blocking point and run their `Drop`
  ([06](06-concurrency-and-capabilities.md)). A search is an `Iter[T]` of
  candidates, and `std.search` adds the combinators LogicT showed such
  searches need: fair interleaving, iterative deepening and bounded search
  ([`logic-constraint-and-rewriting.md`](../research/unconventional/logic-constraint-and-rewriting.md),
  idea 5).
- **Capabilities stay `Copy`, and `std.cap` adds the forms that bound
  them** (Q-51). A lent capability, `fs.lend()`, is a view of type
  `Lent[Fs]`, so the callee cannot keep it and its authority ends with the
  call. A caretaker revokes at run time: `fs.revocable()` returns a
  capability and its revoker, after `revoker.revoke()` every retained copy
  fails with `Revoked`, and `Revoked` joins the I/O error sets. Because
  plain capabilities stay `Copy`, std APIs accept both the plain and the
  lent form ([06](06-concurrency-and-capabilities.md)).
- **Every capability has one interception point** (D-075). `Wrap[T]`
  intercepts each call through a capability trait `T`. It is built from
  the reflection behind `Fake[T]` ([17](17-testing.md)) and installed
  visibly in `main`, typed and with no pointcuts, so test fakes, tracing,
  auditing and the capability log behind `sisu replay` share one mechanism
  where other languages reach for aspects
  ([06](06-concurrency-and-capabilities.md)).
- **One logging facade, in core.** Rust's `log` crate and Go's `slog`
  (added in 1.21 after a decade of fragmentation) show that logging is a
  coordination point that the ecosystem will not converge on by itself.
  A `task.scope` passes its trace and span identifiers to the tasks it
  spawns, and `log` attaches them to every record, as Kotlin's coroutine
  context and Erlang's logger metadata do (D-075). The context carries
  observation, never authority.
- **JSON and regex in core** because every program needs them and
  because a linear-time regex engine is a security property (ReDoS) that
  should not be a dependency choice. Go made the same call.
- **Time zone data embedded**, because `DateTime` without zones is a bug
  generator, and updatable because zones change. Java's `java.time` (the
  JSR-310 redesign after `Date`/`Calendar`) is the model.
- **Time does not drift** (D-075). `Ticker` fires on deadlines with an
  explicit missed-tick policy (tokio's burst, delay and skip), and
  `Timeline` gives Sonic Pi's virtual-time sleep: `tl.sleep(d)` advances
  virtual time by exactly `d` and waits only for the remainder, so timing
  errors do not accumulate. Both take a `Clock`, so the same code is
  real-time in production and deterministic and instant under the test
  clock ([`esoteric-and-live-coding.md`](../research/unconventional/esoteric-and-live-coding.md),
  idea 2). `Ticker.wait` is also a hot-replacement boundary: in debug
  builds a task adopts newly published code there (leaning, D-068).
  `clock.sleep` stays the primitive for timeouts.
- **No HTTP server or client in core.** They move too fast. `std.x.http`
  is the blessed implementation.

## `std.x` (blessed, independently versioned)

`http` (client and server), `tls` (server side), `sql` (driver interface
and the common drivers), `cli` (argument parsing with derive), `toml`,
`yaml`, `csv`, `xml`, `compress`, `crypto` (beyond hashing), `tzdata`,
`unicode-data`, `image` (decoding), `wasm` (component model bindings),
`ffi-gen` (header import support), `verify` (the contract verifier, when
it exists), and `view`.

`std.x.view` holds `View` (leaning, D-071), a `pure` trait with one
method, `to_node(self) -> ViewNode`. A `ViewNode` is a small document tree
(text, record, table, list, tree, image bytes, plot specification) that
every tool showing a value renders: the REPL, hover and probes in the
language server, doc examples, and assertion and snapshot diffs.
`@derive(View)` renders a struct as a record and a list of structs as a
table, and a type without `View` falls back to `Debug`. Because `to_node`
is `pure`, any tool may call it at any time. `View` is the third rendering
trait and the last: `Display` is for a program's users, `Debug` for
programmers as text, `View` for tools as structure. IPython's display
protocol and the moldable inspector are the precedents
([`end-user-and-live.md`](../research/ideation/end-user-and-live.md),
candidate 12). `View` ships after 0.1, which has no `std.x`
([16](16-validation-plan.md)), and is a candidate for `std` at 1.0.

Symbolic mathematics comes to `std.x` after 1.0 (D-076). Its scope is
open (Q-53): exact algebraic numbers could join `Ratio` in `std.num` for
v1, while a computer algebra system with expression trees,
simplification, differentiation and equation solving is a project the
size of SymPy.

Later candidates, each assessed in [19](19-novel-features.md) and none in
0.1:

- `table` (N-37): `Table[Row]`, columnar over an ordinary struct through
  comptime reflection, with rows validated at the boundary by
  `@derive(Decode)` and a row type that can be inferred at compile time
  from a sample file, F#'s type-provider trick done with Q-27's type
  construction and `embed`. After Q-23
  ([`end-user-and-live.md`](../research/ideation/end-user-and-live.md),
  candidate 6).
- `units` (N-37): a comptime-constructed quantity type whose `*` and `/`
  compute dimensions at compile time and whose `+` requires equal
  dimensions, with constants such as `m`, `s` and `km` through ordinary
  operator overloading. Its errors stay readable through
  `@diagnostic_name(f)` (leaning, D-065), an attribute naming a comptime
  function the compiler calls when it prints the type in a diagnostic,
  with `sisu explain --raw-types` to show the real type; without it, a
  units error shows an exponent vector. After Q-23; units built into the
  type system are rejected
  ([`end-user-and-live.md`](../research/ideation/end-user-and-live.md),
  candidate 7).
- `peg` (N-59): grammars as ordinary values built from combinators,
  compiled at comptime into a parsing table with LPeg's predictable cost
  model and run the same at compile time and at run time, so the checked
  string DSLs of [08](08-metaprogramming.md) share one mechanism. Grammars
  produce values and cannot extend Sisu's syntax
  ([`natural-language-and-literate.md`](../research/unconventional/natural-language-and-literate.md),
  idea 3).
- `incr` (N-63): incremental computation in Salsa's shape, with an
  explicit `Db` handle, inputs, and memoised `pure` queries with early
  cutoff. `Db`'s caches mutate, so its reads need the audited
  "observationally pure" declaration of
  [06](06-concurrency-and-capabilities.md), which only a `[trust]`ed
  package can make; there is no `@memo` decorator, because attributes
  cannot wrap functions
  ([`incremental-reversible-and-exotic-paradigms.md`](../research/unconventional/incremental-reversible-and-exotic-paradigms.md),
  idea 1).
- `reactor` (N-62): Lingua Franca's logical-time reactors built from
  parts Sisu has: `pure` reactions that cannot block, read the clock or
  touch shared state, capabilities as the physical inputs, a check that
  rejects a cycle without a delay, and replay in tests. There is no
  reactor mode in the language
  ([`synchronous-and-dataflow.md`](../research/unconventional/synchronous-and-dataflow.md),
  idea 3).
- Schema locks (N-46): a stored or transmitted type says so
  (`@persisted("config", version=3)`), its shape hash goes into a
  committed `schemas.lock`, and a shape change needs a new version with
  either a compatible change or a migration from the frozen old shape,
  which `sisu schema freeze` writes as source; `sisu test` checks that
  decoding old bytes equals migrating the old value. A library and
  tooling, after the experiment
  ([`ai-era-and-evolution.md`](../research/ideation/ai-era-and-evolution.md),
  A7).
- A `Model` capability (N-52): `model.generate[Triage](prompt)`, where
  `@derive(Schema)` on `Triage` emits a JSON Schema or grammar for
  constrained output and refinements are re-checked on the parsed value.
  `Model` appears in signatures as `Net` does and is replaced by a
  scripted fake in tests; model calls at build time stay forbidden
  ([`holes-synthesis-and-ai-era.md`](../research/unconventional/holes-synthesis-and-ai-era.md),
  idea 7).
- `sketch` (N-35): the playground's first-party creative-coding package,
  with a canvas and input, `update` and `draw` functions that hot-swap, a
  `Canvas` fake whose recorded commands make pictures snapshot-testable,
  and scrubbers on numeric literals in the editor
  ([`end-user-and-live.md`](../research/ideation/end-user-and-live.md),
  candidate 11).

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
