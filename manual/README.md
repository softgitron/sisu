# The Sisu manual

This directory holds the manual and the plan for writing it. Readers
start at [the tutorial's first chapter](tutorial/01-hello.md). Writers
start here, then read [STYLE.md](STYLE.md); the first chapter is the
model that every other chapter follows. The research behind the plan is
[research/topics/manuals.md](../research/topics/manuals.md).

## Audience

The tutorial is for programmers who know one language, most often
Python, and know no Sisu. It assumes variables, functions, loops and
lists, and it explains everything that is Sisu's own: blocks that have no
value, optional values instead of `null`, errors as values, ownership,
capabilities, tasks. It points out each place where a Python habit
misleads ("From Python" notes; D-062 keeps the list short and exact).

The reference is for experts and for readers coming back: complete,
precise, consulted rather than read. The guides are for readers who know
the basics and have a task. The explanations are for anyone who wants to
know why Sisu is the way it is. A newcomer to programming is not the
audience; Sisu's teaching profile (D-073) and its course material are a
separate project that this manual can feed.

## What the manual is made of

The structure is Diátaxis's four kinds of document, one directory each,
with the files they show beside them.

| Path | Holds | Serves |
|---|---|---|
| `tutorial/` | 28 chapters in five parts, read in order | learning by doing |
| `guides/` | 32 how-to guides, each a task | doing a task |
| `reference/` | 22 pages that describe the language and the toolchain | looking something up |
| `explanations/` | 14 essays on why Sisu is the way it is | understanding |
| `examples/PAGE/` | every program a page shows, with its expected output | the checks |
| `exercises/CHAPTER/` | the tutorial's exercises, as the learner starts them | practice |
| `solutions/CHAPTER/` | a solution for each exercise, and its start-state session | comparison |
| `tools/` | `check.py` and `check_links.py`, the manual's own checkers | the checks |
| `SUMMARY.md` | every page in reading order, in mdBook's form | the site |

`PAGE` is a tutorial chapter's file name without `.md` (`01-hello`), or
the kind and the name of any other page (`guide-read-files`,
`reference-patterns`, `explanation-ownership`). The standard library's
reference is not written by hand: `sisu doc --std` generates it from the
`##` comments of [compiler/std](../compiler/std/), and `sisu test --std`
runs its examples.

The manual has 96 pages: 28 tutorial chapters, 32 guides, 22 reference
pages (one of them, R18, the generated library reference) and 14
explanations. Every page is written except four guides that wait for
their features (G18, G22, G23 and G25).

## The tutorial

Each chapter teaches one area in sections of one concept each, ends with
exercises, and says what the reader can now do. Part I ends with a
complete program, so that a reader who stops there has written a useful
tool; the ownership part comes after it because the hardest idea is
where readers of a manual stop
([manuals.md](../research/topics/manuals.md), finding 8). Each part ends
with a challenge. "Stage 0" in a paragraph says what the current
compiler can run; a chapter is written when its examples can be checked.

### Part I. First steps

**1. Hello, Sisu** ([written](tutorial/01-hello.md)). Getting `sisu`,
the first program, what its three lines mean, reading a diagnostic (code,
message, place, fix; `sisu explain`), breaking the program on purpose,
`let`, interpolation with `{expr}` and `{expr=}`, worksheets and scripts
(`sisu ws`, `#!/usr/bin/env sisu`), a REPL session (`sisu repl`, shown
with `--echo`), `sisu fmt`, and how exercises work
(`todo` holes, `test` blocks, blocked, failed and ok). One exercise.
Stage 0 runs all of it.

**2. Values and types** ([written](tutorial/02-values.md)). The basic types and what Sisu does with them:
`int` (64-bit, the type of indices), `f64`, `bool`, `char`, `str`;
literals with underscores, bases and suffixes; arithmetic with `//` and
`%` on integers and `/` on floats only (E0301 and its fix), overflow that
traps, the spelled-out wrapping and saturating operators `+%` and `+|`;
comparisons and same-direction chains; `and`, `or`, `not` and the absence
of truthiness; `var` and augmented assignment; `const`; type annotations
and local inference; implicit lossless widening and explicit conversions
(`truncate_to`, `round_to`); rebinding in a scope and the shadowing
warning. Exercises: predict the values of mixed expressions, fix a
division, complete a temperature conversion. Stage 0 runs all of it;
the chapter uses `round()` and `truncate_to_int()`, and `round_to`,
which came with stage 1's eleventh round, is taught in chapter 24.

**3. Functions** ([written](tutorial/03-functions.md)). `def` with typed parameters and a result type, the
rule that a value leaves only through `return` (D-056), functions that
return nothing, default arguments evaluated at each call, keyword
arguments, keyword-only `*` and positional-only `/` parameters, variadic
`*xs`, no overloading (D-050), labelled tuple results and unpacking, `if`
as an expression, recursion, doc comments `##` with examples that `sisu
test` runs, `assert`, and typed holes in your own code (`todo`, D-061).
Exercises: write small functions from their tests, including one whose
doc example must pass. Stage 0 runs all of it except variadic `*xs`
parameters and `sisu check --holes`, which the chapter mentions as
coming.

**4. Control flow** ([written](tutorial/04-control-flow.md)). `if`, `elif` and `else`; `while`; `for` over
`range` (with a step) and over lists; `enumerate`; `break` and
`continue`; `match` on values with literal patterns, `|`, guards and `_`;
`pass`; and what Sisu leaves out (no `switch`, `do-while`, C-style
`for`, `else` on loops or loop labels; D-021), with the idiom for each.
Exercises: FizzBuzz written as a function returning a `String`, a
Collatz step count, a fix exercise built on a loop `else` from Python.
Stage 0 runs all of it.

**5. Collections** ([written](tutorial/05-collections.md)). `list` (literals, `push`, indexing, `xs[^1]` for the
last element and the compile error for `xs[-1]`, slices), `dict`
(literals, lookups, insertion-order iteration, `entry(k).or_insert(0) +=
1`), `set`, tuples and star unpacking, `in`, list, dict and set
comprehensions, the prelude functions (`len`, `sorted`, `sum`, `min`, `max`,
`any`, `all`, `zip` and `zip_shortest`, `reversed`), `Array[T, N]` in
one section, and the first meeting with ownership: assigning a list moves
it, the diagnostic that says so, and `.copy()`. Exercises: word
frequencies, the second largest value, a fix exercise on a moved list.
Stage 0 runs all of it except star unpacking in `let a, *rest = xs` (the
list pattern `[a, *rest]` works in `match` and in `let ... else`) and
list literals typed as `Array` (stage 0 builds one with `Array.filled`).

**6. Text** ([written](tutorial/06-text.md)). `str` and `String` and why both exist, interpolation in
full (format specs such as `{x:>8}` and `{x:.2f}`, `{x:?}` for `Debug`),
the common methods (`split`, `split_whitespace`, `trim`, `lines`, `find`,
`replace`, `starts_with`, `strip_prefix`, `to_lowercase`), iterating
`chars()` and `bytes()`, why a string has no integer index and what to do
instead (`byte_slice`), `char`, raw and multi-line literals, `bytes`
literals, string literals that become a `Path` or a `Url` through
`FromLiteral`, building text with `Builder`, and parsing numbers out of
text. Exercises: a title-case function, a CSV line splitter, a Caesar
cipher over `char`. Stage 0 runs all of it except grapheme iteration and
the normalisation forms.

**7. Structs and enums** ([written](tutorial/07-structs-and-enums.md)). `struct` with fields and defaults,
construction by keyword and by position, methods with `self` and `mut
self`, associated functions such as `Point.origin()`, `@derive(Debug,
Eq, Copy, ...)`, `enum` with unit, tuple and record variants, `match`
with destructuring, nested patterns, keyword patterns and `..`, the
exhaustiveness check and its list of missing cases, recursive enums
(boxed by the compiler), and type aliases. Exercises: a shape area by
`match`, a playing-card type with an ordering, a tiny expression
evaluator over a recursive enum. Stage 0 runs all of it.

**8. Absent values and errors** ([written](tutorial/08-errors.md)). `T?` with `Some` and `None`, `?.` and
`??`, `if let`, chained conditions (D-071), `let ... else`, `while let`;
error sets with `error`, `T ! E`, `Ok` and `Err`, `?` to propagate, the
`else` and `else as e` fallbacks, `.or` and `.or_else`, `expect`,
matching on errors, unions `{A, B}` and the subtyping that makes them
free, inferred sets `T !`, results that must be used and `_ =`, `main`
returning `! E` and the exit status, `.context(...)`, error traces, and
panics as bugs, not errors. Exercises: parse a version string, a
configuration lookup with a fallback, propagate through three layers.
Stage 0 runs all of it except `.or_else`.

**9. Project: a word counter** ([written](tutorial/09-word-counter.md)). The first complete program, built step
by step: `sisu new`, the package layout (`sisu.toml`, `src/main.sisu`),
`main(sys: System)` and the first capability, reading files through
`Fs`, arguments through `sys.env.args()` and `std.x.cli`, counting with a
`dict`, sorting and printing a table, `sisu run -- ARGS`, tests that hand
the program an in-memory file system (`Fs.in_memory`), and `sisu build`.
It follows [proposal/examples/wordcount.sisu](../proposal/examples/wordcount.sisu).
Exercises: read a file through an `Fs` (a fix), find the lines that
hold a word. Challenge I: extend the counter in three parts (a minimum length, ignore
punctuation, a `--top` option with a transcript test). Stage 0 runs all
of it.

### Part II. Ownership

**10. Ownership and moves** ([written](tutorial/10-ownership.md)). Every variable owns its value; assignment
and `own` parameters move; `Copy` types and `@derive(Copy)`; `.copy()`;
values are dropped at the end of their scope in reverse order; `Drop`;
`with` blocks; `defer`; reading move diagnostics and their fixes, which
say what each fix costs. Exercises: fix three move errors three different
ways, predict the order in which values end (the order of `defer`s,
which is the order of drops, since a test cannot see a drop). Stage 0
runs all of it.

**11. Borrowing** ([written](tutorial/11-borrowing.md)). The three parameter conventions (borrow, `mut`, `own`)
and `mut` at the call site; `ref` and `ref mut`; the rule of one mutable
or many shared borrows; borrows that end at their last use; returning a
borrow and the `from` clause; `for x in mut xs` and `for x in own xs`;
closures that borrow what they capture; `sisu explain-borrow` and the
monitor's counterexample run (`--defer-borrow`), which the experiment
keeps from its participants, so whether the manual may teach it is
Q-73. Exercises: from the wrong-ownership corpus, each a fix with the
diagnostic as the guide. Stage 0 runs all of it.

**12. Views** ([written](tutorial/12-views.md)). `view struct`, `str` and `Slice[T]` as views, tokens that
point into a source buffer, why a view cannot leave the frame that owns
what it borrows, views that make the structures holding them views, and
the two ways out (own the data, or keep the work in the frame).
Guaranteed drop (D-058) gets one paragraph. Exercises: a tokenizer
returning views, a fix exercise where a view escapes. Stage 0 runs all
of it.

**13. Sharing and graphs** ([written](tutorial/13-sharing.md)). `Box` and the boxing of recursive enums,
`Shared` and its counted `.copy()`, `Shared.same` and `==` by value, `Cell`, `Weak` and
cycles, `Arena[T]` with `Key[T]` for graphs, a table for choosing among
them, `@must_consume` with `close(own self)` and its rule for
containers, and `sisu audit --ownership`. Challenge II: a dependency
graph with cycle detection, first with an arena, then with `Shared` and
`Weak`. Stage 0 runs all of it.

### Part III. Abstraction

**14. Traits** ([written](tutorial/14-traits.md)). Declaring a trait, required and provided methods, `impl
Trait for Type`, `Display` and `Debug`, the derives, the fixed set of
operator traits (D-022), supertraits, associated types and constants,
coherence and the orphan rule (with the root package's exception,
D-013, and a library's impl through a trait's type argument, `impl
Mul[Vector] for f64`), newtypes with `@delegate` and `@derive(Into, From)`, and `impl
Type` in another module (Q-12). Exercises: `Display` for a money type, an
`Add` for vectors, a newtype that stops metres being added to seconds.
Stage 0 runs all of it except `@derive(Into, From)`, which the chapter
calls designed.

**15. Generics** ([written](tutorial/15-generics.md)). Generic functions and types with bounds, `where`
clauses, defaulted type parameters (D-060), bounds checked at the
definition so that no error appears inside a library at a use,
monomorphisation and its size cost, `dyn Trait` behind `Box` or `ref`,
the rules for object safety, choosing between an enum and a trait
object, and const generics with `Array[T, N]` and expressions as
arguments. Exercises: a generic `largest`, a `Stack[T]`, a ring buffer
over `Array[T, N]`. Stage 0 runs all of it except D-060's allocator
parameter (`list[T, A]`), so the chapter shows a defaulted parameter on
a type of its own, and `sisu build --explain-size`.

**16. Closures and iterators** ([written](tutorial/16-closures-and-iterators.md)). `x -> expr`, `it`, trailing `do` blocks,
captures and `own` closures, function types and `pure` function types,
the `Iterator` trait and its adapters (`map`, `filter`, `fold`, `take`,
`collect[T]`), laziness, generators with `yield` and `Iter[T]`,
generator expressions, and `?` inside closures with `try_map` and
`try_collect`. Exercises: a pipeline over records, an infinite generator
taken lazily, a fallible parse of many lines. Stage 0 runs all of it
except `try_map` and `try_collect`.

**17. Modules and packages** ([written](tutorial/17-modules-and-packages.md)). Files and directories as modules, `pub`
and `pub(package)`, `import` and `from ... import`, re-exports, the
prelude's fixed list, no glob imports and no package cycles, `sisu.toml`
and its sections, path dependencies, features, the lockfile, workspaces,
`[trust]`, documentation with `##` and `##!` and `sisu doc`, and a first
look at publishing (the API diff and its semver rule, the capability
summary, admission; D-066). Challenge III: the word counter as a library
with documented `pub` items, in one file, since an exercise is a file
that `sisu test` runs alone; the chapter's workspace example then splits
it into a library and an application. Stage 0 runs all of it except the
registry commands (`sisu add`, `publish`), whose sections wait.

### Part IV. The world outside the program

**18. Capabilities** ([written](tutorial/18-capabilities.md)). `System` and its fields, passing capabilities down
instead of reaching for globals, attenuation (`restrict_to`,
`read_only`, `only_hosts`), capabilities stored in struct fields, lent
capabilities (`fs.lend()`, `Lent[T]`) and revocable ones, the short list
of what is ambient (`print`, `log`, `task.scope`), interceptors
(`Wrap[T]`), what a package with no capability types can do (D-066), and
`sisu audit --caps`. Exercises: narrow a program's file system, make a
function testable by taking a `Clock`. Stage 0 runs all of it.

**19. Testing in depth** ([written](tutorial/19-testing.md)). Tests beside the code and in `tests/`,
`suite` and `@tag`, parametrised tests, what a failing `assert` prints,
`expect`, a test's output shown with its failure, fixtures as values and `@fixture`, each test's own fakes and the
clock that advances only when every task waits, property tests (`check`,
`arbitrary[T]`, shrinking, the failure database), `exhaustive[T]`,
snapshots and transcripts with `--update`, doc examples, replay tokens,
`@refines`, fakes of your own traits (`Fake[T]`), benchmarks and their
statistics, tests run natively, and the JSON output and cache. Exercises: turn three example-based tests into
one property, fix a program against its transcript (a transcript the
learner wrote would be its own judge). Stage 0 runs all of it except
`@fixture`, `Expectation`, the failure database, the JSON output,
`--fuzz`, `--cover` and `--mutate`.

**20. Concurrency** ([written](tutorial/20-concurrency.md)). `task.scope` and `spawn`, `join` and how error sets
flatten, the scope's result, tasks that borrow from the frame, `Send`
and `Sync` in the messages that mention them, `Mutex`, `RwLock`,
`Atomic`, `Once` and `Condvar`, bounded and unbounded channels and closing them,
`select` with timers and a cancellation arm, cancellation, `task.shield`,
deadlines, `task.race`, panics in tasks and supervision, a mutex's
invariant re-checked when a panic drops its guard (D-077), `log` and the
trace context, and testing all of it under the seeded scheduler
(`--explore`, `--scheduler pct`). Exercises: a fan-out to a slow fake
service on the test clock (a fake network answers at once, so a test
could not tell a fan-out from a loop), a worker pool, a race with a
timeout. Stage 0 runs all of it.

**21. A tour of the standard library** ([written](tutorial/21-standard-library.md)). One section per module a program
reaches for first: `std.fs` and `std.io`, `std.time` (with `Ticker`,
`Timeline`, `Instant` and `Zone`), `std.rand`, `std.env` and `std.process`, `std.json`,
`std.regex`, `std.collections` (`SortedDict`, `Deque`, `Heap`,
`BitSet`), `std.hash`, `std.encoding`, `std.sort` and `std.search`, and
`std.log`, each with one example and a link to its source, which holds
the documentation that `sisu doc --std` writes, until the site publishes
the generated pages. Exercises: one small task per module. Stage 0 runs
all of it.

**22. Project: a small web service** ([written](tutorial/22-web-service.md)). A JSON service on `std.x.http`:
attenuated capabilities in `main`, a task per connection, logging,
`std.json` with derived encoding, tests over `http.FakeNet`, the
simulation profile with faults (`sisu test --sim`), a failure kept as a
replay token, `sisu run --record` and `sisu replay`, changing a handler
while the server runs (`sisu run --watch`, epochs, `live.boundary()`),
and a native build (`sisu build --native`). Challenge IV: rate limiting
and a cache, proved under fifty orders of its tasks. Stage 0 runs all of
it except TLS.

### Part V. Further

**23. Contracts** ([written](tutorial/23-contracts.md)). `requires`, `ensures` with `result` and `old`,
`invariant` on structs and the rule that only the type's own methods
assign its fields, refinement types (`type Port = int where ...`) and
`try_from`, the check levels (`--contracts`) and `debug_assert`, contracts in the
documentation and as property tests, contracts on trait methods, and
what waits for the verifier (`decreases`, ghost code). Exercises: a
bank account with an invariant, a binary search with its contract.
Stage 0 runs all of it.

**24. Numbers in depth** ([written](tutorial/24-numbers.md)). The sized integers and `uint`, the operator
families for wrapping, saturating and checked arithmetic, conversions
that can lose information, strict floating point (one rounding per
operation, `mul_add`, pairwise `sum`, `@float(fast)`; D-074), NaN and
ordering, and the exact types `BigInt`, `Ratio` and `Decimal` with their
suffix-free literals, which take an exact type beside an operator too
(`f * 2` with `f` a `BigInt`), and named rounding (D-076), with the
bit methods (`count_ones`, `rotate_left`, `swap_bytes`, `to_bits`).
Exercises: a checksum over `u32`, a rotating hash with a parity bit, a
mortgage table in `Decimal`. Stage 0 runs all of
it, `total_order`, `round_to` and the checked methods of `int`
included; it accepts `@float(fast)` and keeps the arithmetic strict.

**25. Compile-time code** ([written](tutorial/25-compile-time.md)). `comptime` expressions, `const`, `comptime
def` and `comptime assert`, fuel and `@comptime_budget`, reflection
(`reflect.fields_of`, `if comptime` that narrows a type, `comptime for`,
`value.[f]`), writing a derive with `return impl Trait for T:`,
attributes read as data, `embed`, `target` and `@cfg`, features,
`@diagnostic_name`, `sisu eval` for configuration, and the checked string
literals of [08](../proposal/08-metaprogramming.md). Exercises: a lookup
table built at compile time, a derive that prints a struct's fields.
Stage 0 runs all of it except `@diagnostic_name`, `sisu eval` and the
checked string literals.

**26. Purity and data parallelism** ([written](tutorial/26-purity.md)). What `pure` promises and the rules
behind it (insertion-ordered `dict`, strict floats, no operation that
observes the schedule), `pure` function types in higher-order code,
`std.par` (`par_iter`, `par_iter_mut`, `par.join`, reducers, `grain`),
serial elision, the lowest-index failure of `pure` scopes, and
`SoaList[T]`. Exercises: a parallel histogram that gives the same bits on
one worker and on eight. Stage 0 spells the parallel loop `par.iter(xs)`
and the parallel map `par.map`, runs the chunks on one task, and lacks
`par_iter_mut` and `SoaList`; the chapter teaches the proposal's
spelling once it runs.

**27. Systems programming** ([written](tutorial/27-systems.md)). `unsafe` blocks and expressions, raw
pointers, C interop (header import, `extern "C"`, `std.ffi`,
`@repr(c)`, `@export`, `@blocking`), `[trust]` and `sisu audit`,
allocators (`list.with_allocator`, `Region`) and `try_push`, the
resource attributes `@no_alloc`, `@no_block` and `@stack`, the runtime
profiles (`full`, `single`, `freestanding`), targets and cross
compilation, `panic = abort`, and binary size. Exercises: the layouts of
`@repr(c)` structs, the order of bytes, an allocation-free audio
callback, records from a device's bytes. Stage 0 runs the unsafe code,
the raw pointers and C interop, under `sisu run` and in tests, checking
raw memory in the interpreter and in a native debug build, and the
exercises practise the safe side. The
allocators (`list.with_allocator`, `Region`, `try_push`), header import,
`--target` and the profiles other than `full` are designed, and the
resource attributes compile without checking anything.

**28. Code that lasts** ([written](tutorial/28-code-that-lasts.md)). Editions and the compatibility promise, `sisu
fix --edition`, deprecation with `@fix_inline` and `sisu fix --deps`,
`@preview` features (D-072), the false-friend table, the API diff and
behavioural semver of `sisu publish`, `sisu review` and weakened
specifications, `sisu why`, and planning with holes (`sisu check
--holes`). Challenge V: migrate a small library across a breaking change
without breaking its users. Stage 0 has none of `sisu fix`, `why`,
`review`, `publish`, `sisu check --holes` or the registry, and accepts
`@fix_inline` without effect (a call of a deprecated function warns,
W0023, and a use of an item marked `@preview`, W0028), so the
chapter teaches compatibility through what the language already checks
(defaults, keyword-only parameters, exhaustive matches over error sets,
doc examples) and gives each tool a sentence.

## The guides

Each guide does one task for a reader who knows the basics, in steps,
with checked examples, and links to the tutorial and the reference
instead of teaching.

| | Guide | What it does, and what it covers |
|---|---|---|
| G1 | Set up your editor ([written](guides/editor.md)) | Connects an editor to `sisu lsp`: diagnostics, hover with error sets and capabilities, inlay hints, fixes, explain-borrow, worksheet values inline. |
| G2 | Coming from Python ([written](guides/from-python.md)) | Ports a Python script and walks through every false friend (D-062, [R19](reference/false-friends.md)) and the idioms that replace `class`, `None`, exceptions and truthiness. |
| G3 | Coming from Rust ([written](guides/from-rust.md)) | Maps lifetimes to views and `from`, `Rc` to `Shared`, `Result` to `T ! E`, macros to `comptime`, `async` to tasks. |
| G4 | Coming from Go ([written](guides/from-go.md)) | Maps goroutines to scopes, `context` to cancellation, interfaces to traits, `if err != nil` to `?`. |
| G5 | Read and write files ([written](guides/read-files.md)) | `Fs` operations, missing files as `IoError`, walking a directory, testing with `Fs.in_memory`. |
| G6 | Parse command-line arguments ([written](guides/command-line.md)) | `std.x.cli` derived from a struct, help text, errors, a transcript test. |
| G7 | Read and write JSON ([written](guides/json.md)) | `@derive(Encode, Decode)`, `json.encode`, `json.decode` validating refinements and invariants at the boundary, a form of a type's own, `json.parse` (stage 0 encodes an enum and does not decode one). |
| G8 | Match text with regular expressions ([written](guides/regex.md)) | `std.regex`, groups, the linear-time guarantee and what it rules out. |
| G9 | Test code that uses the clock ([written](guides/clock.md)) | `Clock.frozen_at`, `advance`, the test clock, `Ticker` under test, an `Instant` read from the clock given. |
| G10 | Test code that uses the network ([written](guides/network.md)) | `Net.fake`, `http.FakeNet().respond_to`, request counts, `HttpError` and the `NetError` it holds. |
| G11 | Write a property test ([written](guides/property-test.md)) | `check`, `arbitrary[T]`, `@derive(Arbitrary)` and an `Arbitrary` of a type's own, reading a shrunk counterexample, the failure database (stage 0 lacks the database). |
| G12 | Test a command-line tool ([written](guides/test-tool.md)) | Snapshots and transcripts, `--update` and reviewing the diff. |
| G13 | Find and fix a borrow error ([written](guides/borrow-errors.md)) | Reading E04xx diagnostics, `sisu explain-borrow`, `--defer-borrow` and the counterexample (Q-73, as for chapter 11). |
| G14 | Find a concurrency bug ([written](guides/concurrency-bug.md)) | `--explore`, `--schedule-seed`, PCT and the exhaustive scheduler, `sisu test --sim` with faults. |
| G15 | Reproduce a failure ([written](guides/replay.md)) | Replay tokens, `sisu run --record`, `sisu replay`. |
| G16 | Change a running program ([written](guides/live.md)) | `sisu run --watch`, epochs, `live.boundary()`, what restarts the program. |
| G17 | Make a loop parallel ([written](guides/parallel.md)) | `std.par`, reducers, `grain`, checking that the result is unchanged. |
| G18 | Use a region for per-request memory | `Region.bump`, `list.with_allocator`, why the list cannot escape (waits for the allocators of collections: `std.mem` has `Region` and the unsafe `RawBuf`, which takes it by value, and `list.with_allocator` is E0351). |
| G19 | Call a C library ([written](guides/c-library.md)) | A trusted wrapper package with a capability-taking safe API, and its tests (stage 0 runs it in the interpreter and natively; header import waits). |
| G20 | Build a native executable ([written](guides/native.md)) | `sisu build --native`, `--release`, `sisu test --native`, targets and cross compilation (stage 0 lacks `--target`, D0009). |
| G21 | Write a derive ([written](guides/derive.md)) | A trait's `comptime def derive` over `reflect.fields_of`. |
| G22 | Generate configuration | A module of `const` items rendered by `sisu eval` (waits for `sisu eval`, D0003 in stage 0). |
| G23 | Publish a package | Documentation coverage, the API diff, semver, provenance (waits for the registry: `sisu publish` is D0003). |
| G24 | Check what a dependency can do ([written](guides/audit.md)) | `sisu audit`, the capability summary, admission and its authority delta (stage 0 audits a file or a whole package; the summary and admission wait for the registry). |
| G25 | Move to a new edition | `sisu fix --edition` and `sisu fix --deps` (waits for `sisu fix`, D0003 in stage 0). |
| G26 | Plan with typed holes ([written](guides/holes.md)) | `todo`, blocked tests and the closure of each hole, the release build's E0445, `sisu check --holes` (D0009 in stage 0). |
| G27 | Benchmark and profile ([written](guides/bench.md)) | `bench` blocks, `black_box`, `sisu bench` with its samples, median and interval, `sisu run --profile` (D0009 in stage 0). |
| G28 | Log and trace ([written](guides/log.md)) | `std.log`, levels, the trace context of scopes, `trace.wrap` over a capability (stage 0 writes every level and has no `trace.wrap`, so the guide writes its own interceptor). |
| G29 | Write a script with dependencies ([written](guides/script.md)) | A shebang file with inline metadata. |
| G30 | Embed a file at compile time ([written](guides/embed.md)) | `embed` from the package's root, and what it records for reproducible builds. |
| G31 | Do exact arithmetic with money ([written](guides/money.md)) | `Decimal`, rounding modes, `Ratio` from division. |
| G32 | Design a library's API ([written](guides/library-api.md)) | The std rules: capabilities in, values out; specific error sets; no hidden allocation; `Debug` everywhere; iterators over views; Python's names. |

## The reference

Each page describes one part of the language or the toolchain completely
and in the same order every time: syntax, meaning, the diagnostics that
guard it, examples, and links to the tutorial and to the rationale. Every
heading is a stable anchor that diagnostics and other pages link to, and
every feature carries its status: in stage 0, designed, leaning, or a
prototype in the experiment ([STYLE.md](STYLE.md)).

| | Page | Contents |
|---|---|---|
| R1 | Lexical structure ([written](reference/lexical-structure.md)) | Source text, identifiers and the naming rules, the 47 keywords and the contextual ones, comments and doc comments, every literal, line continuation, indentation, the formatter's layout. |
| R2 | Types ([written](reference/types.md)) | Every builtin type, numeric conversions, text types, collections and literal typing (`FromLiteral`), tuples, unit and `Never`, `T?` and `T ! E`, error-set subtyping, function types and `pure`, references, views, the library pointer types, `dyn`, refinements, aliases, the auto traits, what the type system leaves out. |
| R3 | Declarations ([written](reference/declarations.md)) | `let`, `var`, `const`, `def` and its parameters, `pure def`, `comptime def`, `struct`, `view struct`, `enum`, `error`, `trait`, `impl`, `type`, `test`, `suite`, `bench`, `import`, `extern`, visibility. |
| R4 | Expressions and operators ([written](reference/expressions.md)) | The precedence table and each operator, calls and keyword arguments, indexing, slicing and `^`, comprehensions, closures and `it`, `if` and `match` expressions, `else` fallbacks, `?`, `?.`, `??`, interpolation and format specs, `comptime e`, `unsafe e`. |
| R5 | Statements and control flow ([written](reference/statements.md)) | Assignment and places, `if`, chained conditions, `if let`, `while let`, `let ... else`, `while`, the three `for` modes, `break`, `continue`, `return`, `match`, `select`, `with`, `defer`, `assert`, `debug_assert`, `expect`, `pass`, `unsafe` blocks. |
| R6 | Patterns ([written](reference/patterns.md)) | Every pattern form, binding by case, exhaustiveness, guards, alternatives that bind. |
| R7 | Ownership and borrowing ([written](reference/ownership.md)) | Moves, copies, drops and their order, conventions, references, elision and `from`, views, captures, iteration modes, must-consume, guaranteed drop, the library pointer types, allocators. |
| R8 | Errors and panics ([written](reference/errors.md)) | Error sets, propagation, traces, context, panics, unwinding, abort, exit statuses. |
| R9 | Concurrency ([written](reference/concurrency.md)) | Tasks, scopes, handles, failures and their order, cancellation, shields, deadlines, races, channels, `select`, the sync types, `Send` and `Sync`, atomics and orderings, `std.par`, the scheduler and its yield points. |
| R10 | Capabilities ([written](reference/capabilities.md)) | `System`, each capability's operations, attenuations, lending, revocation, interceptors, fakes, what is ambient, how it is enforced. |
| R11 | Contracts ([written](reference/contracts.md)) | `requires`, `ensures`, `invariant`, refinements, check levels, violations. |
| R12 | Compile-time evaluation and reflection ([written](reference/compile-time.md)) | The rules of `comptime`, fuel and budgets, the `reflect` API, `target`, `embed`, the derive protocol, `@cfg` and features. |
| R13 | Modules, packages and the manifest ([written](reference/modules.md)) | The module tree, imports and visibility, the prelude's fixed list, every key of `sisu.toml`, the lockfile, workspaces, scripts and inline metadata, editions. |
| R14 | Attributes ([written](reference/attributes.md)) | Every attribute, with where it may stand and what it does. |
| R15 | Testing ([written](reference/testing.md)) | `test` and its forms, outcomes, selection, a test's capabilities, the property-testing API, snapshots, the transcript format, replay tokens, simulation, benchmarks, output and caching. |
| R16 | The `sisu` command ([written](reference/sisu-command.md)) | Every subcommand and option, each with an example and its exit statuses. |
| R17 | Diagnostics ([written](reference/diagnostics.md)) | Every code by range (E00xx lexer to D0xxx driver) in tables with its catalog identifier and message, and an entry for each code a page shows: the rule, a checked wrong program, its session and the fixed program. Written by hand; `tools/check.py` checks the tables against the catalog. |
| R18 | The standard library | Generated by `sisu doc --std`; a page per module. |
| R19 | False friends ([written](reference/false-friends.md)) | Each spelling shared with Python or Rust whose meaning Sisu rejects, with the check that catches it (D-062). |
| R20 | Grammar ([written](reference/grammar.md)) | The grammar, in the notation of the specification. |
| R21 | Glossary ([written](reference/glossary.md)) | Each term the manual uses, with the chapter that introduces it. |
| R22 | Targets and runtime profiles ([written](reference/targets.md)) | The target tiers, `full`, `single`, `freestanding` and `sim`, the ABI, binary size. |

## The explanations

Each explanation answers one "why" for a user, not a designer, in one
shape: an opening paragraph that says what Sisu does and what the page
explains, the **See also:** line, a section "The problem: ...", Sisu's
choice in one or more sections, the choices other languages made, "What
it costs" and "What would change it" (X1, the whole design on one page,
ends with "Where the design could be wrong"). It cites the decision log
and the research by number for the evidence, says in its text which
parts are designed rather than in stage 0, and carries no status lines.

| | Explanation | About |
|---|---|---|
| X1 | The design in one page ([written](explanations/design.md)) | The priority order, the principles, and what Sisu leaves out on purpose ([00](../proposal/00-vision-and-principles.md), [14](../proposal/14-non-goals.md)). |
| X2 | Errors without exceptions, absence without null ([written](explanations/errors.md)) | Error sets, panics as bugs, `T?`. |
| X3 | Ownership without lifetimes ([written](explanations/ownership.md)) | Second-class references, views, what is given up, compared with Rust, Swift and Python. |
| X4 | Capabilities and the supply chain ([written](explanations/capabilities.md)) | No ambient authority, testability, portability, the cost of one more parameter. |
| X5 | Concurrency without colours ([written](explanations/concurrency.md)) | No `async`, scopes as supervisors, cancellation without tokens. |
| X6 | Determinism ([written](explanations/determinism.md)) | `pure`, insertion-ordered `dict`, strict floats, and what replay, simulation and `comptime` get from them. |
| X7 | Compile-time code instead of macros ([written](explanations/compile-time.md)) | What `comptime` and reflection cover and what they leave out. |
| X8 | Loud divergence ([written](explanations/loud-divergence.md)) | Why a familiar spelling means the same as in Python or Rust or fails to compile. |
| X9 | How the compiler works ([written](explanations/compiler.md)) | Stages 0 to 3, the interpreter, SIR, the native backend, the queries. |
| X10 | Diagnostics as a product ([written](explanations/diagnostics.md)) | The catalog, identifiers, fixes with consequences, the wrong-program corpus. |
| X11 | How Sisu evolves ([written](explanations/evolution.md)) | Editions, migrations that travel with libraries, governance. |
| X12 | Testing without mocks ([written](explanations/testing.md)) | Fakes, the test clock, simulation, replay. |
| X13 | Working with unfinished code ([written](explanations/unfinished-code.md)) | Typed holes, blocked tests, the fill engine to come. |
| X14 | Costs you can see ([written](explanations/costs.md)) | No hidden allocation, monomorphisation, compile speed. |

## Every feature has a place

Each feature of the proposal is taught by a tutorial chapter (a number),
described by a reference page and, where one helps, done by a guide or
explained by an explanation. A feature that no row names is a gap in this
plan.

| Feature | Tutorial | Reference | Guides, explanations |
|---|---|---|---|
| **[02](../proposal/02-syntax.md) Syntax** | | | |
| Source text, identifiers, NFC, confusable lints (D-048), naming rules | 2, 7 | R1 | |
| Comments, `##` and `##!` doc comments | 1, 3, 17 | R1 | |
| Blocks by indentation, spaces only, `pass`, line continuation | 1, 2, 4 | R1 | |
| Keywords and contextual keywords | 2 | R1 | |
| Numeric, boolean and character literals | 2, 24 | R1 | |
| String literals: plain, interpolated, raw, multi-line, bytes | 1, 6 | R1 | |
| Interpolation, format specs, `{x:?}`, `{x=}`, `{{` | 1, 6 | R4 | |
| List, dict, set and tuple literals; comprehensions | 5, 16 | R1, R4 | |
| `FromLiteral` for `Path`, `Url`, `String`, `Decimal`, `BigInt` | 6, 24 | R2 | |
| `let`, `var`, typed and deferred `let`, unpacking, rebinding and shadowing (D-024) | 1, 2, 5, 8 | R3 | |
| `const` | 2, 25 | R3 | |
| `def`: defaults, keywords, `*`, `/`, `*xs`; no overloading (D-050) | 3 | R3 | |
| `pure def`, `comptime def` | 26, 25 | R3 | X6, X7 |
| `struct`, methods, associated functions; `view struct` | 7, 12 | R3 | |
| `enum`; `error` with open sets | 7, 8 | R3, R8 | |
| `trait`, supertraits, associated items; `impl`, generic `impl`, `impl Type` (Q-12) | 14, 15 | R3 | |
| `from` clauses; `where` clauses; const and defaulted type parameters | 11, 15 | R2, R7 | |
| `test`, `suite`, parametrised tests, tests with capabilities, `bench` | 1, 3, 19 | R15 | G11, G12 |
| `type` aliases and refinements | 7, 23 | R2, R11 | |
| `import`, `from ... import`, C header import, `extern "C"` | 9, 17, 27 | R13 | G19 |
| Construction, named constructors | 7 | R4 | |
| `pub`, `pub(package)` | 17 | R13 | |
| Attributes as data (D-046) | 7, 25 | R14 | |
| Blocks have no value; `return` only (D-056); `if` and `match` expressions | 3, 4 | R4, R5 | |
| `else` and `else as e` fallbacks; `unsafe e`; `comptime e` | 8, 27, 25 | R4 | |
| Places, assignment, augmented assignment, no walrus | 2, 5 | R5 | G2 |
| Operators and precedence, `//`, `%`, `+%`, `+\|`, mixing rules, chains | 2, 24 | R4 | |
| No truthiness, `and`, `or`, `not` (D-025); `==` without `is`; `in` | 2, 5, 13 | R4 | X8 |
| `if`, chained conditions (D-071), `if let`, `let ... else`, `while`, `while let` | 4, 8 | R5 | |
| `for` in three modes, `enumerate`, `range` | 4, 5, 11 | R5 | |
| `match`, guards, alternatives, star, keyword patterns, `..`, exhaustiveness | 4, 7 | R6 | |
| `select` | 20 | R5, R9 | |
| `assert`, `debug_assert`, `expect` | 1, 3, 19, 23 | R5, R11, R15 | |
| `with`, `defer` | 10 | R5 | |
| What is left out: `switch`, `do-while`, C `for`, loop `else`, labels (D-021) | 4 | R5 | G2 |
| Closures, `it`, `do` blocks, `own` captures (D-026) | 16 | R4 | |
| Labelled tuple results and `result` | 3, 23 | R2 | |
| Prelude functions | 1 to 5 | R13 | |
| Typed holes `todo` (D-061), `sisu check --holes` | 1, 3 | R5, R16 | G26, X13 |
| Indexing, slicing, `^1`, the constant negative index error (D-062) | 5 | R4 | X8 |
| The formatter (D-030) | 1 | R16 | |
| Departures from Python | 1 to 8 | R19 | G2, X8 |
| **[03](../proposal/03-types-and-generics.md) Types and generics** | | | |
| Static sound typing (D-002), local inference (D-003) | 2, 3 | R2 | X1 |
| Sum types, recursive enums boxed (D-043), `@repr`, discriminants | 7, 13, 27 | R2, R14 | |
| `T?` and its layout | 8 | R2 | X2 |
| Traits, explicit `impl` and its fix, coherence and the orphan rule (D-013) | 14 | R2 | |
| `@delegate`, newtypes, `@derive(Into, From)` | 14 | R14 | |
| Monomorphisation, sharing, `--explain-size`; `dyn` and object safety (D-017) | 15 | R2 | X14 |
| Auto traits `Send`, `Sync`, `Sized`, `Escapable`; opt-in `Copy` (D-007) | 10, 20 | R2 | |
| Enums against trait objects | 15 | | |
| `int`, `uint`, sized integers (D-035); traps (D-009); widening only (D-016) | 2, 24 | R2, R4 | |
| Strict floating point, `mul_add`, pairwise `sum`, `@float(fast)` (D-074) | 24 | R2 | X6 |
| `BigInt`, `Ratio`, `Decimal` (D-076) | 24 | R2 | G31 |
| `str`, `String`, `char`, `bytes`, no integer indexing (D-034) | 6 | R2 | |
| Tuples, unit, `Never`; function types and `pure` function types | 2, 5, 16, 26 | R2 | |
| Refinement types | 23 | R2, R11 | |
| What the type system leaves out | | R2 | X1 |
| **[04](../proposal/04-memory-and-ownership.md) Memory and ownership** | | | |
| Owners, moves, `.copy()`, `Copy` (D-004, D-007) | 5, 10 | R7 | X3 |
| Parameter conventions (D-006); references; the aliasing rule; `from` | 11 | R7 | G13 |
| Views (D-004); guaranteed drop (D-058) | 12 | R7 | X3 |
| `Box`, `Shared` (D-005), `Cell`, `Weak`, `Mutex`, `Atomic`, `Arena[T]`, `Key[T]` | 13, 20 | R7 | |
| Destruction at scope end, `Drop` order (D-008) | 10 | R7 | |
| Closures and captures; iteration modes | 11, 16 | R7 | |
| `@must_consume` and containers (D-054, D-064) | 13 | R7 | |
| `unsafe`, raw pointers, `[trust]` | 27 | R7 | G19 |
| Allocators, `Region` (D-060); allocation failure and `try_push` | 27 | R7, R8 | G18 |
| `@no_alloc`, `@no_block`, `@stack` (D-059); `sisu audit --panics` | 27 | R14, R16 | X14 |
| **[05](../proposal/05-errors.md) Errors** | | | |
| Errors and panics (D-011, D-040); error sets, unions, inference | 8 | R8 | X2 |
| `?` (D-042), `try_map`, `try_collect`; `else`, `or`, `or_else`, `expect` | 8, 16 | R4 | |
| Results that must be used; `_ =` | 8 | R8 | |
| Traces and `.context()` | 8 | R8 | |
| Unwinding, `main`'s exit status, `panic = abort`, double panics | 8, 27 | R8 | |
| `close(own self)` for cleanup that can fail | 13 | R8 | |
| **[06](../proposal/06-concurrency-and-capabilities.md) Concurrency and capabilities** | | | |
| Tasks, scopes, handles, joins, failures (D-014) | 20 | R9 | X5 |
| `task.race`, cancellation, shields, deadlines (D-075) | 20 | R9 | G14 |
| Panics in tasks and supervision; poisoning by invariant (D-077) | 20, 23 | R9 | |
| `std.par` (D-069) | 26 | R9 | G17 |
| Colourless I/O, runtime profiles, stacks, `@blocking` | 20, 27 | R22 | X5 |
| `Send` and `Sync`; atomics and orderings | 20 | R9 | |
| Channels and `select` | 20 | R9 | |
| `System`, root capabilities, attenuation (D-015) | 9, 18 | R10 | X4 |
| Lent and revocable capabilities (Q-51) | 18 | R10 | |
| What is ambient (Q-5, Q-6); the reserved `with` clause | 1, 18 | R10 | |
| Interceptors and trace context (D-075) | 18, 20 | R10 | G28 |
| Enforcement: `[trust]`, no I/O at compile time, no build scripts, the `sim` check | 17, 18, 22 | R10 | G24 |
| `pure` and determinism (D-074) | 26 | R2 | X6 |
| **[07](../proposal/07-contracts.md) Contracts** (D-028) | | | |
| `requires`, `ensures`, `old`, `result`, `invariant` | 23 | R11 | |
| Refinements, `try_from` | 23 | R11 | |
| Check levels, contracts in docs and semver, on trait methods | 23, 17 | R11 | |
| `decreases`, the verifier, ghost code (later) | 23 | R11 | |
| **[08](../proposal/08-metaprogramming.md) Metaprogramming** (D-018) | | | |
| `comptime`, `comptime def`, `comptime assert` | 25 | R12 | X7 |
| No compile-time I/O, `embed`; fuel and budgets (D-063) | 25 | R12 | G30 |
| Reflection, `if comptime`, `comptime for`, `value.[f]`, `target` | 25 | R12 | |
| Derive as a trait's comptime function | 25 | R12 | G21 |
| `@cfg`, features | 25 | R12, R13 | |
| `sisu eval` configuration | 25 | R16 | G22 |
| Checked string literals | 25 | R12 | |
| **[09](../proposal/09-modules-packages-and-build.md) Modules, packages and build** | | | |
| Modules from files (D-019), visibility (D-045), imports, the prelude, no cycles | 9, 17 | R13 | |
| The manifest, features, conditions as data (D-063), lockfile, one version per build | 17 | R13 | |
| `[trust]`, no build scripts (D-032) | 17, 27 | R13 | X4 |
| The registry: content addressing, transparency log, namespaces, API-diff and behavioural semver, provenance, capability summary, review ledger (D-031) | 17, 28 | R13 | G23 |
| Admission, cooldown, new packages (D-066) | 17 | R16 | G24 |
| Builds: caching, incremental queries, targets, profiles | 22, 27 | R16 | G20, X9 |
| Workspaces | 17 | R13 | |
| `sisu doc`, doc examples as tests | 3, 17 | R16 | |
| **[10](../proposal/10-tooling.md) Tooling** (D-029) | | | |
| The commands of `sisu` | 1 onwards | R16 | |
| Compile-speed targets (D-038) | | | X9, X14 |
| Diagnostics, codes, fixes and their consequences, `sisu explain` (D-065) | 1, 11, 14 | R17 | X10 |
| `sisu why`; `@diagnostic_name` | 28, 25 | R16, R14 | |
| The language server, worksheets, always-on evaluation, structural editing, the stepper | 1 | R16 | G1 |
| The teaching profile (Q-41, D-073) | | R13 | |
| Debugging, `--profile`, `Debug` for every type | 22 | R16 | G27 |
| `--record`, `replay`, replay tokens (Q-49, D-070) | 19, 22 | R15, R16 | G15 |
| Hot replacement, epochs (D-053, D-068) | 22 | R16 | G16 |
| The REPL, scripts and inline metadata (D-052) | 1 | R13, R16 | G29 |
| `doc --digest`, the pocket reference, `sisu fill`, `sisu review` | 28 | R16 | G26 |
| Migration: `sisu fix`, `--deps` (D-067) | 28 | R16 | G25 |
| **[11](../proposal/11-stdlib.md) Standard library** (D-033) | | | |
| Each `std` module | 5, 6, 9, 13, 18 to 21, 24, 26 | R18 | |
| `std.x` packages: `http`, `cli` and the rest as they exist | 9, 22 | R18 | G6 |
| The API design principles | | | G32 |
| **[12](../proposal/12-interop-and-targets.md) Interop and targets** | | | |
| Targets, tiers, cross compilation | 27 | R22 | G20 |
| Runtime profiles | 27 | R22 | |
| C interop, `std.ffi`, `@repr(c)`, `@export`, `@blocking`, C sources; no C++ (Q-24) | 27 | R13, R14 | G19 |
| Other languages, the unstable ABI (D-039), binary size, `Lazy[T]` | 27 | R22 | |
| **[13](../proposal/13-evolution-and-governance.md) Evolution and governance** | | | |
| Editions, the compatibility promise (D-036), `@preview` (D-072), the false-friend table | 28 | R13, R19 | X11 |
| Governance (D-037) | | | X11 |
| **[17](../proposal/17-testing.md) Testing** (D-041, D-057) | | | |
| Tests beside code, `tests/`, suites, tags, parametrised tests | 19 | R15 | |
| `assert`'s report, `expect`, `Expectation` | 1, 19 | R15 | |
| Outcomes, blocked by a hole | 1 | R15 | X13 |
| Fixtures, isolation, the test clock, fakes, `Fake[T]` | 19 | R15 | G9, G10, X12 |
| Schedules, `--explore`, the `sim` profile and its faults | 20, 22 | R15 | G14 |
| Property tests, `exhaustive[T]`, fuzzing, `@refines` (D-070) | 19, 26 | R15 | G11 |
| Snapshots and transcripts | 19 | R15 | G12 |
| Coverage, mutation, benchmarks, JSON output, caching | 19 | R15 | G27 |

The principles of [00](../proposal/00-vision-and-principles.md) and the
non-goals of [14](../proposal/14-non-goals.md) are explanation X1. The
candidates of [19](../proposal/19-novel-features.md) that the owner
adopted are rows above under the document they changed; the ones that
are later tooling are in R16 with that status.

## Conventions

[STYLE.md](STYLE.md) has the rules for sentences, terms, code and
diagnostics. This section has the rules for pages and files.

### Voice and length

Second person, present tense, short sentences, plain words. A page's
length counts its prose only, the words outside its fenced blocks: its
examples, sessions and outputs take the room they need and are kept
short by their own limits. A tutorial chapter is 1,400 to 3,500 words
of prose; a section teaches one concept in 50 to 400; a guide is 350 to
1,000; an explanation is 1,500 to 3,000; a reference page is as long as
completeness needs, with a heading for each thing a reader looks up.
Examples are at most 40 lines, most under 20, and no line in a `.sisu`
file is longer than 80 columns; 72 keeps a page readable on a phone.
`sisu fmt` breaks a line only past 100 columns, so a line of 81 to 100
columns that it keeps whole fails the check: put a trailing comma
inside the line's brackets, and the formatter breaks it there, one item
to a line.

### The shape of a page

A tutorial chapter opens with what the reader will do and what it
assumes, teaches in sections, gives its exercises, and ends with "What
you learned" and a paragraph that names the next chapter; it has no
**See also:** line, since it links where it teaches. A guide, a
reference page and an explanation open with a paragraph that says what
the page covers, then a paragraph that starts **See also:**, before the
first heading: the tutorial chapters that teach the subject, the
reference pages that state its rules, the guides that do its tasks, the
proposal's documents and decisions, and the research, each linked to the
page itself, never to this plan's list of pages while the page exists.
A guide's sections are its steps, numbered ("## 1. Read a whole file"),
and it ends with a paragraph that starts **Status.** when something it
uses is designed, a prototype, or different in stage 0. A reference page
says after its **See also:** line that each of its sections ends with a
status, in the words of [STYLE.md](STYLE.md#features-and-their-status);
a page that is a list (R17's codes, R19's false friends, R21's glossary)
gives one status for the whole page, in its section "How to read this
page" or in a closing section "Status", as R19 does.
An explanation has the shape that [the list of
explanations](#the-explanations) describes.

### One concept per section, one runnable example per concept

A section teaches one concept and shows it in at least one example that
runs. An example is a whole file, shown whole, with what Sisu prints
when it runs, fails to compile or fails a test. A tutorial section ends
with a prompt when one helps: **Predict.** before a run whose result is
worth guessing, **Try it.** for a change the reader makes, and **From
Python.** for a habit that misleads. Explanation stays short in the
tutorial and links to the explanations. A page that is a list, as R21's
glossary is, has entries in place of concepts and no examples of its
own: each entry links to the page whose examples show it.

### Examples

An example is `examples/PAGE/NAME.sisu`, shown by a page, with the files
that state what Sisu does with it. Two checkers read those files.

The conformance runner reads the forms of
[compiler/conformance](../compiler/conformance/README.md): `NAME.out`,
`NAME.err` and `NAME.status` for a program that runs (with `NAME.args`,
`NAME.stdin` and `NAME.files/`), `NAME.diag` for the diagnostics by
identifier (required whenever a program has diagnostics, since an absent
`.diag` means none), `NAME.test` for `sisu test`, `NAME.ws` for a
worksheet, and `NAME.unformatted` for a file kept unformatted on purpose,
with the reason. A package example is a directory `NAME.pkg/` with its
`sisu.toml` and `src/`, and the same files beside it, `NAME.args` for
`sisu run -- ARGS` and `NAME.build` for `sisu build` among them; a page
shows its runs with a session `NAME.console` and its build with
`NAME.build`. Every example is therefore a conformance case too, and
one that runs is also run under the ownership monitor, recorded and
replayed.

The manual's checker reads one more form, the session: `NAME.console`,
which is what a terminal shows. A line starting `$ sisu ` is a command,
and the lines after it are what the command printed, standard output and
standard error together, as the reader sees them. Sessions show the
human form of diagnostics, `sisu fmt --check`, `sisu test` output and
anything else a reader types. These rules say where a session runs:

- It runs with an empty environment in a fresh directory that holds
  every regular file of the session's directory, not only its own
  example's, and then the contents of `NAME.files/`, named after the
  session, whose files replace any of the same name.
- So a file's name is unique in its directory, and a variant of a file
  that one session needs, such as a program after a change, is a file
  of another name or a file in that session's own `NAME.files/`.
- Nothing changes a file between a session's commands. A change that
  the reader makes is a second session over the changed files.
- Standard input is empty. A command that ends `< FILE` reads FILE of
  the session's directory instead, as a shell's redirection does, which
  is how a page shows a REPL session: `sisu repl --echo < inputs.txt`.
- A session under `solutions/` runs against the exercise in its start
  state, which is how a chapter shows the first run of an exercise.
- A session beside a package, `NAME.console` beside `NAME.pkg/`, runs
  at the root of a copy of the package's tree, where a reader types the
  commands. The runner's `NAME.cwd` is ignored there: a session that
  works in one package of several names its directory, as `sisu run
  till` does.
- Every `.sisu` file under a `NAME.pkg/` must be shown by a page. A
  package that does not compile, as in R17's entries for the codes of
  packages, or a variant of a package that a page only runs, is a
  `NAME.files/` tree beside a session, with no `NAME.sisu`; the runner
  does not read it, and its files are shown where the page shows them.
- A block cannot show output that holds a fence of its own, such as the
  Markdown of `sisu doc`, since the page's block would end there: the
  page describes such output in prose.

A page shows a file with a marker on the line before the fenced block:
`<!-- file: examples/01-hello/hello.sisu -->` for a source file,
`<!-- output: examples/01-hello/hello.out -->` for an expectation or a
session (for `.ws` and `.build`, the standard-output part), and
`<!-- unchecked: REASON -->` for the rare block that cannot be checked.
Four reasons are accepted: an installation command, an editor's
configuration, output that differs on every run (a benchmark's times),
and grammar notation, which no command runs. A fenced block without a
marker fails the check, so no code reaches a reader unchecked by
accident.

### Exercises

An exercise is `exercises/CHAPTER/NAME.sisu`, a file the learner edits
until its tests pass. Its first line reads `##! Exercise N.M, TITLE.
Kind: KIND. Level: L.`, the next `##!` lines state the task, and the
file holds typed holes (`todo("...")`) where the learner's code goes and
`test` blocks that judge it. The solution is
`solutions/CHAPTER/NAME.sisu` with the same `test` and `suite` blocks,
byte for byte, its `NAME.test` (every test ok), and `NAME.console`, the
start-state session the chapter shows. The tests are the judge and the
learner never edits them; a solution that needed other tests would be a
different exercise.

Kinds follow the order in which programming skill grows, from reading
to writing ([manuals.md](../research/topics/manuals.md), findings 3 and
4):

| Kind | The learner | The start state |
|---|---|---|
| `predict` | writes the value an expression will have, as a constant (`const PREDICTION: int = todo("...")`), and a test compares it with the expression | compiles; every test blocked |
| `fix` | repairs a program that does not compile or fails a test, guided by the diagnostic | does not compile, or a test fails |
| `complete` | fills a hole inside code that is already written | compiles; every test blocked |
| `write` | writes a whole body from a signature, a doc comment and tests | compiles; every test blocked |
| `challenge` | solves a problem in parts, at the end of a part of the tutorial | compiles; every test blocked or failing |

Levels: **1** practises one idea of the chapter and takes minutes; **2**
combines the chapter's ideas; **3** brings in earlier chapters and asks
for a design. A chapter has three to eight exercises, starting at level
1, with at least one `fix`; a `predict` exercise comes before the first
`write` in any chapter that has one. Chapter 1 is the exception, with one
exercise, because its section "Break it on purpose" is its practice in
fixing. The project chapters, 9 and 22, are the other: they build one
program, so their exercises change that program, start where it does
(22.1 is at level 2), and need no `predict`.

Hints live in the chapter, under the exercise, in folded `<details>`
blocks, two or three per exercise, each narrower than the last; the last
hint names the idea, never the code. The chapter links the solution and
asks the learner to open it after the tests pass, or once the hints have
not helped.

A challenge is the last exercise of its chapter, numbered as the
chapter's others are, and its title starts with its part's numeral:
`##! Exercise 9.3, Challenge I: a better word counter. Kind: challenge.
Level: 3.` A challenge is one file with a `suite` per part. Part 1 uses the part's
chapters directly; part 2 changes the problem the way Advent of Code's
second halves do, so that a good part-1 design pays; part 3, when there
is one, is a stretch. Its inputs are text in the file or files under
`NAME.files/`, read through `Fs.in_memory`, so that the tests stay
deterministic. The challenges are at the ends of chapters 9, 13, 17, 22
and 28.

### Running the exercises

Today a learner runs `sisu test NAME.sisu` in the exercise's directory,
as chapter 1 teaches. The plan is a `sisu learn` command (Q-63) that:

- copies the exercises into a working directory once (`sisu learn
  init`), so that the manual's own files stay clean;
- lists each chapter's exercises as not started (every test blocked),
  in progress (a test fails or the file does not compile) or solved
  (every test ok), from `sisu test`'s JSON output;
- watches the files and reruns the current exercise on every save, as
  Rustlings does;
- shows the next hint on request, read from the chapter's `<details>`
  blocks so that hints have one source;
- shows the solution's path once an exercise is solved, and on request
  with a confirmation before that;
- appends a solution's hidden tests, if it has any (Q-65), before
  calling an exercise solved.

Later the playground of [proposal/18](../proposal/18-next-steps.md)
(the interpreter in WebAssembly, with in-memory capability fakes) runs
the same examples and exercises in the browser: an example's marker
becomes a run button, an exercise an editor with its tests, and the same
files are checked by the same CI.

## How the manual is checked

Every push checks the whole manual. Three tools do it, and CI runs them
in a job of its own (to add to [ci.yml](../.github/workflows/ci.yml)):

```yaml
  manual:
    name: manual pages, examples, exercises and solutions
    runs-on: ubuntu-latest
    timeout-minutes: 30
    defaults:
      run:
        working-directory: compiler
    steps:
      - uses: actions/checkout@v4
      - name: Toolchain
        run: |
          rustup toolchain install 1.97.0 --profile minimal
          rustup override set 1.97.0
      - name: Build
        run: cargo build -p sisu-driver -p sisu-conformance
      - name: Stage 1, the front end of `sisu` (built natively on first use)
        run: ./target/debug/sisu tokens std/prelude.sisu > /dev/null
      - name: Examples, as conformance cases
        run: ./target/debug/sisu-conformance --sisu target/debug/sisu ../manual/examples
      - name: Solutions, as conformance cases
        run: ./target/debug/sisu-conformance --sisu target/debug/sisu ../manual/solutions
      - name: Pages, sessions, exercises and sources
        run: python3 ../manual/tools/check.py --sisu target/debug/sisu
      - name: Links and anchors
        run: python3 ../manual/tools/check_links.py
```

The conformance runner checks every example and solution as a case:
output and status, diagnostics by identifier, `sisu test` and worksheet
output, a parse that reprints the file, the SIR round trip, the
ownership monitor, record and replay, and the formatter.
[tools/check.py](tools/check.py) checks what the runner cannot see: that
every fenced block in a page is marked and equals the file it names;
that every session prints what it says; that every exercise has a
well-formed first line, a solution with the same tests that passes them
all, and a start state that is not already solved; that every source is
formatted and short enough; that every example, session and exercise
is shown or named by some page; and that the tables of R17's "Every
code" list each code of the [catalog](../compiler/catalog/messages.catalog)
once, with its identifier, and no code the catalog lacks.
[tools/check_links.py](tools/check_links.py) checks that every relative
link in the manual's Markdown reaches a file and every `#anchor` a
heading of the page it names, as GitHub makes anchors from headings;
`python3 tools/check_docs.py` at the repository root checks the links of
every Markdown file of the repository, but not their anchors.

When the compiler changes what it prints, the job fails and the change
is reviewed, not ignored: `sisu-conformance --bless DIR` rewrites the
runner's expectations and `python3 manual/tools/check.py --bless`
rewrites the sessions (or, for some directories or files only,
[tools/diagnostics/bless_sessions.py](tools/diagnostics/README.md), whose
`--check` lists the sessions that differ), and the diff goes into the
same commit as the compiler change, where a reviewer reads it.

## How the manual is rendered

For now the manual is plain Markdown that GitHub renders: relative
links, folded `<details>` blocks, and code fences labelled `sisu`,
`console`, `text` and `toml` (and the language of a file in another
language, such as G2's Python script). Nothing depends on a generator,
and every page is readable in an editor.

Later a site generator builds the manual into a website with search, a
print view and a page per chapter. mdBook is the first candidate,
because its layout (one file per page, a `SUMMARY.md` table of contents)
matches this directory and its search and print support are built in;
the markers become run buttons for the playground.
[SUMMARY.md](SUMMARY.md) already lists every page in reading order in
mdBook's form, the tutorial's parts as its parts, with the four guides
that wait and R18, which the build generates, as draft entries; so the
site needs a `book.toml` (with `no-section-label`, since the chapters
number themselves) and the build, and nothing else of the manual
changes. The standard
library's pages are generated at build time by `sisu doc --std --output`
with the same compiler that checked the manual, so the generated
reference always matches the checked examples. The site states the
version of `sisu` it was built with (Q-67).

## Order of writing

The order follows what readers reach first ([manuals.md](../research/topics/manuals.md),
finding 8): Part I and the reference pages it leans on, then the
ownership part with a test on real readers before moving on, then the
rest in tutorial order, with guides and explanations written after the
chapters they build on. Estimates are writer-days: one writer who knows
Sisu, working one day, produces the page with its examples, exercises,
solutions and passing checks.

| Step | Pages | Estimate |
|---|---|---|
| 1 | Chapter 1, this plan, [STYLE.md](STYLE.md), `tools/check.py` | done |
| 2 | Chapters 2, 3 and 4 | done |
| 3 | R16 the `sisu` command (2, done), R17 diagnostics, first codes (2, done), R1 lexical structure (2, done) | done |
| 4 | Chapters 5 (2.5), 6 (2), 7 (2.5), 8 (3) | done |
| 5 | Chapter 9 with challenge I (3, done); G1 Set up your editor (0.5, done), G2 Coming from Python (1, done), G3 Coming from Rust and G4 Coming from Go (2, done); R19 false friends (1, done) | done |
| 6 | Chapters 10 (3, done), 11 (4, done), 12 (3, done), 13 with challenge II (3, done); R7 ownership (3, done); G13 borrow errors (1, done); X3 (1, done) | done |
| 7 | Reader test of Parts I and II: five readers who know Python, their stuck points, the rewrites | 4 |
| 8 | Chapters 14 (3, done), 15 (3, done), 16 (3, done), 17 with challenge III (2.5, done); R2 (3, done), R3 (2, done), R6 (1, done), R13 (2.5, done) | done |
| 9 | Chapters 18 (3, done), 19 (3, done), 20 (4, done), 21 (3, done), 22 with challenge IV (4, done); R9 (2.5, done), R10 (2, done), R15 (2, done); G5 to G16 (9, done) | done |
| 10 | R4 (2, done), R5 (1.5, done), R8 (1.5, done), R14 (1.5, done), R20 (2, done), R21 (1, done), R22 (1, done); X1, X2, X4, X5, X8, X10, X12 (7, done) | done |
| 11 | Chapters 23 (2, done), 24 (2, done), 25 (3, done), 26 (2, done); R11 (1, done), R12 (2, done); G17, G20, G21, G26, G27, G28, G29, G30, G31, G32 (7.5, done); X6, X7, X9, X13, X14 (5, done) | done |
| 12 | When their features exist: chapters 27 (3, done) and 28 with challenge V (2, done); G18, G19, G22, G23, G24, G25 (4.5; G19 and G24 done); X11 (1, done); the site (3) | 6 to go: G18, G22, G23, G25 and the site |
| 13 | The maintenance round: the writers' corrections to this plan and [STYLE.md](STYLE.md), links and anchors (`tools/check_links.py`), the diagnostics entries the pages lacked, the generator of R17 retired, the check of "Every code" against the catalog, `SUMMARY.md`, and the findings in one list by area | done (1.5): corrections applied, links and anchors checked (0 broken), 43 entries added to R17, the generator retired, `SUMMARY.md`, the findings in ten groups (209), 27 changed sessions reviewed and blessed, every check passing |
| 14 | Stage 1's third round (R27 to R36) and the decisions 354 to 360 in the manual: the codes they added to R17, the pages whose rules changed, every changed session reviewed, the waiting guides and R18 read against their features, the findings checked again | done (1.5): 11 codes in R17's tables, each with an entry, 145 sessions blessed (124 changed, 21 new, 3 retired), the start sessions of nine `fix` exercises moved to `sisu check`, G18, G22, G23, G25 and R18 still waiting (allocators, `sisu eval`, the registry, `sisu fix`, the site), the findings at 199 (22 settled, 12 new), every check passing |
| 15 | Stage 1's fourth round (R37 to R46) and the decisions 361 to 388 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1.5): 9 codes in R17's tables, each with an entry (E0271 to E0273, E0449, W0026, D0128 to D0131), 151 sessions blessed (135 changed, 16 new, 3 retired), the start sessions of the nine `fix` exercises back on `sisu test` and the exit status 3 for blocked tests written without "Stage 0 differs"; associated constants (chapter 14, R3), `expect` going on (chapter 19, R15), generator expressions (chapter 16, R4), `@derive(Arbitrary)` and an `Arbitrary` of a type's own (G11, R15), `@deprecated` on types, W0026 and a suite's tags (R14), `xs[1:^1]` (chapter 5, R4), `@cfg` on members (R14), a trait's contract across modules (R11) and the display of a `Shared` (chapter 13) taught with checked examples; the findings at 172 (38 settled, 11 new), every check passing |
| 16 | Stage 1's fifth round (R47 to R55) and the stage 0 round's decisions 420 to 429 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1.5): 3 codes in R17's tables (E0129 to E0131), 5 entries added (E0129, E0130, E0131, E0239, W0015) and E0106's rewritten, 41 sessions blessed or moved (28 changed by the rounds, 7 new, 5 given a command more, and the second of E0106's two attribute sessions moved with the first to E0129); `xs[^1]` on a `Slice` and `bytes` (chapter 5, R4), the spelling fixes of other languages' habits (R19, G2, G3, G4), `pub(crate)` (R13, R19, R20), `Fs.rename` (R10, chapter 21, G5), `sisu lint`, `sisu audit --unsafe` and the audit of a package (R16, R6, chapter 27, G19, G24, G32), replay tokens over embedded files (G30, R12, R15), `--faults` without `Revoked` (X12, G14, R15, chapter 22), `@deprecated` on traits and fields and E0129 to E0131 (R14, R20), a type's constant as a type's constant argument (chapter 14, R3) and transcripts without traces (R15) taught with checked examples; chapter 2 cut to 3,497 words; the findings at 161 (17 settled, 2 stale, 8 new), every check passing |
| 17 | Stage 1's sixth round (R56 to R64) and the stage 0 round's decisions 460 to 470 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1.5): 3 codes in R17's tables (D0132, D0134, D0135) and the new texts of D0057 and R0051, 4 entries added (E0223, D0132, D0134, D0135), 88 sessions blessed (81 changed by the rounds, 6 new, 1 changed with its example) and 10 conformance expectations; `sisu test --native` (R15, R16, G20, chapter 19), `debug_assert` (chapter 23, R11, R15, R5, R20), `unbounded_channel` (chapter 20, R9), `sisu bench --samples` and its statistics (G27, R15, R16, chapter 19), a test's captured output (R15, R16, chapter 19), `import a.b.Item`'s fix (R13, R17), the new fixes of E0231, E0801, E0306, E0115 and E0123 (chapters 14, 15 and 23, R1, R17, R19), `==` and `hash` of `Shared` by value (chapter 13, R7), a builtin type as a `comptime` argument (R12) and the variadic and star texts (R19, R3, R6) taught with checked examples; the hole closures that `sisu test` no longer prints taken out of the prose (chapters 1 and 3, G26, X13, R5, R15, R16, R17); the findings at 146 (19 settled, 4 narrowed, 4 new), every check passing |
| 18 | Stage 1's seventh round (R65 to R72) and the stage 0 round's decisions 500 to 508 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1.5): 2 codes in R17's tables, each with an entry (E0274, E0275), E0239's entry retired with the constant it named and the new texts of E0418, E0420, E0812, R0069 and R0103, 94 sessions blessed (86 changed by the rounds, 54 of them for the blocked tests' closures, 6 new, 2 given a command more) and 16 conformance expectations; black-box tests in `tests/*.sisu` (chapters 19 and 22, R13, R15, G12), a failed `expect` going on natively and the native schedulers, faults and `sim.always` (R15, G20, R16, D0134), the blocked tests' closures back in the prose (chapters 1 and 3, G26, X13, R5, R15, R16, R17), the trace's names of a trait's impl methods (R8, R11), `lambda`, `f"..."`, `class`, `try:` and `go f()` (R19, G2, G3, G4), the spelling fixes in brackets and after a value (R19, G3), E0274 (chapter 14, R3) and E0275 (R13), the derive diagnostics at the derive (chapter 25, R12, R14, G21, G7, R17's E0347, E0235 and E0608) and the audit's names and heading (chapter 18, R16, G32) taught with checked examples; the findings at 135 (18 settled, 4 narrowed, 7 new), every check passing |
| 19 | Stage 1's eighth round (R73 to R78, decisions 520 to 525) and the stage 0 round's decisions 540 to 547 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1.5): 5 codes in R17's tables (E0132, E0276, R0154, R0155, D0136), 4 entries added (E0132, E0276, R0154, D0136), 24 sessions blessed (13 changed by the rounds, one of them given a command more, 3 others given commands more, 8 new) and 7 conformance expectations with 6 new; test names as fixed text and E0132 (chapter 19, R15), a child module behind an import and a local that hides a module (R13, R17's E0304), the dev-dependency rule and E0276 (R13), literals beside `BigInt` and `Decimal` (chapter 24, R2), the explicit discriminant's E0900 (R2, R3, R14, R20), `expect`'s own report (chapter 19, R15), `sisu run --native --scheduler` (R16, G20, G14, chapter 20), native debug builds checking raw memory (chapter 27, G19, G20, R16), D0136 (G27, R16), the regex messages (G8), the eighth rounding mode (chapter 24), and the new notes and fixes of E0306 (chapter 3), E0117 and `else as e` (chapter 22, R17), E0201, E0304, E0342, E0405, E0438 (R17, chapter 7, G13), E0369 (R11), E0305 (R19) and of a template's instance (R12, X7, chapter 25) taught with checked examples; the findings at 118 (20 settled, 4 narrowed, 3 new), every check passing |
| 20 | Stage 1's ninth round (R79 to R94, decisions 580 to 595) and the stage 0 round's decisions 560 to 567 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1.5): 1 code in R17's tables, with its entry (W0027), 20 sessions blessed (12 changed by the rounds, one of them given a command more, 4 changed with their examples, 4 new) and 22 conformance expectations, 13 of them new, one retired with the program it checked (chapter 5's slice made a worksheet); `sisu lint` for unused imports and `sisu fmt` keeping them (R16, R13, R1, chapter 17), the REPL and worksheets on stage 1 (R16, R4, chapter 5, X9), sized shifts keeping their type (chapter 24, R2, R4), `expect` with a message and the assertion's evaluated call (chapter 19, R15, R5), the `@refines` test at its attribute (chapter 19, R15), a hole reached at compile time (R12), the edition (R13, chapter 28), the inferred error set of a scope's body (R8), the new texts and notes of E0335, E0402, E0428, E0204 and E0124 (chapters 16, 17, 20 and 26, R1, R2, R9, R13, R17), a field's `ty` in a derive (R12, G21), the native checks of pointers to variables and of C's memory (chapter 27, G19, R16), the consumers of iterators that are `pure` (chapter 18), `DateTime`'s parts (chapter 21), `FakeNet`'s scripts (G10) and `std.log`'s `Logger` (G28, chapter 21) taught with checked examples; the findings at 104 (17 settled, 6 narrowed, 3 new), every check passing |
| 21 | The stage 0 round's decisions 620 to 627 in the manual: the features they made teachable, every changed session reviewed, the findings checked again | done (1): no code added to the catalog, 3 sessions blessed (2 changed by the round, `sisu help`'s list of stage 1's commands and the doc examples of `std.time`, and 1 new) and 4 conformance expectations, all new, with G10's refused connection matched as `Network(Refused(_))`; the bit methods (chapter 24 with exercise 24.6, chapter 25's exercise 25.4, R2, R4), `Instant` and `Zone` (chapter 21, G9, R2, R10), `HttpError`'s `Network(cause)` and `Interrupted(cause)` (G10, R10), the REPL and worksheets on stage 1's own runs with `<input>` and `<repl>` and `:why` from stage 1's trees (R16) and `sisu help`'s list with `parse --shape` (R16) taught with checked examples; the findings at 102 (4 settled, 2 narrowed, 2 new), every check passing |
| 22 | Stage 1's tenth round (R95 to R103, decisions 600 to 608) and the stage 0 round's decisions 640 to 646 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1): 1 code in R17's tables, with its entry (D0137), and an entry for E0005, 37 sessions blessed (33 changed by the rounds: 16 for the five fix texts that no longer repeat their replacement, 7 for R101's other fixes and notes, 4 for the named test lists, 2 for the similar names, 2 for E0812's copy, 1 for `.to_int()` beside two sized integers, 1 for the standard library's two new doc examples; 3 new, 1 changed with its example) and 16 conformance expectations (7 changed by the rounds, 2 with their examples, 7 new); `:why` from stage 1 and the REPL's program standing in the working directory and named by its file (R16), a variant's fields' `ty` and the kinds of the program's own types in a derive (R12, G21), the similar names of E0203 and E0271 (R17, R13), E0005 naming an invisible character (R1, R17), R101's fixes and notes at their codes (R17's E0100, E0117, E0200, E0204, E0266, E0273, E0300, E0310, E0347, E0374, E0811, E0812 and E0105, chapters 14, 16 and 24, R2, R14, R19), the adapters that are not `pure` and the spelling with `fold` or a loop (chapter 18, R17's E0333), a worksheet showing an option or a result (chapter 6, R16), `sisu lint` leaving a refused import to its error (R16), `MIN`, `MAX` and `truncate_to` of every integer type (chapter 24 with exercise 24.3's solution and hint, R2), `Instant` on the monotonic clock (chapter 21, G9, R10) and the named test lists (chapters 17 and 19, R15, R16) taught with checked examples; the findings at 85 (21 settled, 3 narrowed, 4 new), every check passing |
| 23 | Stage 1's eleventh round (R104 to R110, decisions 660 to 666) in the manual: the codes it added to R17, the features it made teachable, every changed session reviewed, the findings checked again | done (1): 1 code in R17's tables, with its entry (E0277), 31 sessions blessed (24 changed by the round: 6 for E0301's float division, 3 for E0806's notes, 3 for E0310 naming `+=`, 2 each for E0338's, E0421's and E0236's notes and for E1010 and E1021 standing alone, 1 for the narrowed capability's note, 1 for the standard library's new doc examples; 5 new, 2 changed with their examples) and 20 conformance expectations (8 changed by the round, 3 with their examples, 9 new, among them a package's output and lockfile); coherence through a trait's type argument, `impl Mul[Vector] for f64` in a library (chapter 14 with its package `scale`, chapter 17's orphan rule, R2, R17's E0236 and E0310, the glossary), imports under a `@cfg` on the target with their uses in `if comptime` and E0277 (chapter 17, R13, R12, R14, R17's E0200, E0266, E0275 and E0277), `arbitrary[T]` through its bound (G11, chapter 19), `f64.total_order()`, `max_by` and `min_by` over a float, `round_to` and the checked methods of `int` (chapter 24 with `total` and `lossy`, chapter 16, R2, R17's E0347), the reserved `F[_]` (R2, R17's E0100), `@derive(Plain)` asking for `@repr(c)` and `@repr(u8)` kept (chapter 27, R2, R14), E0235's label at a derive (R14, R17), the new fixes and notes at their codes (chapters 2, 10, 18 and 23, G2, G19, G29, G32, X7, X8, R10, R19, R17's E0204, E0237, E0258, E0300, E0301, E0338, E0402, E0421, E0806, E1010 and E1021), `json.decode`'s expected type (G7) and Q-78 (R11) taught with checked examples; the findings at 65 (20 settled, 1 decided against, 1 stale, 3 narrowed, 2 new), every check passing |
| 24 | Stage 1's twelfth round (R111 to R119, decisions 700 to 708) and the stage 0 round's decisions 680 to 694 in the manual: the codes they added to R17, the features they made teachable, every changed session reviewed, the findings checked again | done (1): 4 codes in R17's tables, each with an entry (E0343, E0991, W0028, W0029), and E0220's note at its entry, 16 sessions blessed (4 changed by the rounds: E0343 for a `Shared`'s field, `sisu help`'s `--echo`, the standard library's new doc example, and G6's transcript, which the Rust front end now refuses with E0609; 2 changed with their examples; 10 new) and 25 conformance expectations (2 changed by the rounds, `E0343` and G6's failing transcript, 3 with their examples, 20 new); a session's command may end `< FILE` (`tools/check.py`), so chapter 1 and R16 show REPL sessions with `sisu repl --echo`; lookups by `str` and `.copied()` (chapters 10 and 19, G2, R4, and the code of chapters 13, 20 and 22 and G10 without `.to_string()`), E0343 (chapter 13, R7), `@preview` and W0028 and W0029 (chapter 28, R14, R13, X11, the glossary), `f.doc` and `cli.help_text`'s option text (G6, R12, R15), the `@export` symbol, Sisu's convention and E0991 (chapter 27, R14, R22, G19), `Zone`'s `Debug` in a worksheet (chapter 21), the executable's size and what remains in it (chapter 27, G20, R22), G14's joined tasks and R0079, the manifest's `runtime` key (R22), `target.runtime` and `--target-runtime` (chapter 25, R12, R22), E0220's note (chapter 17, R13), `Decimal` and Q-79 (G31) and the exact `truncate_to` (chapter 24, R2) taught with checked examples; the findings at 59 (5 settled, 1 found intended, 1 decided against, 8 narrowed, 1 new), every check passing |
| | Total | about 160 |

The setup of R18, the generated library reference, belongs to step 12's
site; until then `sisu doc --std` serves it locally. R17 grows with every
chapter: each diagnostic a page shows gets its entry.

## What the plan found in stage 0

Writing the manual turned up findings for the compiler's maintainers,
recorded here by area so that the compiler's rounds can work from them
and later pages do not rediscover them. Each item is one finding: what
stage 0 does, and in parentheses the page whose writing found it (a
chapter by its number, a reference page, guide or explanation by its
letter and number, "the maintenance round" for step 13). A finding that
the rounds R10 to R119 of [stage 1's rules](../compiler/stage1/RULES.md),
or the decisions 354 to 708 of the driver, the runtime and the test
runner in the [architecture notes](../compiler/docs/architecture.md),
settled is no longer listed; each was checked against the compiler
before it went. An item marked "assigned to stage 1" or "assigned to
stage 0" is one that the compiler's rounds in progress have taken on.
The last group holds the diagnostics that lack a fix or mislead,
whatever their area.

The list holds 59 findings in ten groups. The rounds R11 to R26
settled 41 that the writers recorded (R24 alone 14, R25 9, R16 5),
stage 1's third round, R27 to R36, with the decisions 354 to 360,
settled 22 more (R33 alone 4, decision 357 3), and its fourth round,
R37 to R46, with the decisions 361 to 388, settled 38 (R45 alone 13,
R43 7, decision 386 6, decision 380 5), and its fifth round, R47 to R55,
with the stage 0 round's decisions 420 to 429, settled 17 (R55 5, R54 4,
R48 3, R53 1, and the decisions 421, 426, 428 and 429 one each) and
narrowed four more, while two were found stale (`sisu run
--explore` prints the output of the seed it reports, and R0103 is
raised), and its sixth round, R56 to R64, with the stage 0 round's
decisions 460 to 470, settled 19 (R63 6, R61 3, R64 2, R57 to R60 and
R62 one each, and the decisions 464, 465 and 467 one each) and narrowed
four more (R63 two, the decisions 466 and 468 one each), and its
seventh round, R65 to R72, with the stage 0 round's decisions 500 to
508, settled 18 (R69 3, R70 3, R68 2, R71 2, R67 and R72 one each,
decision 507 2, and the decisions 503, 504, 505 and 508 one each) and
narrowed four more, and its eighth round, R73 to R78, with the decisions
520 to 525 and the stage 0 round's decisions 540 to 547, settled 20
(R77 8, R76 4, R74 2, decision 547 2, R73, R75, R78 and decision 545
one each) and narrowed four more (R76, R77, R78 and decision 546 one
each), and its ninth round, R79 to R94, with the decisions 580 to 595
and the stage 0 round's decisions 560 to 567, settled 17 (R88 2, R79
to R87, R89 to R91, R93 and R94 one each, and decision 567 one) and
narrowed six more (R92, R94, the decisions 560 and 561 together, and
564, 565 and 566 one each), and the stage 0 round's decisions 620 to
627 settled 4 (decision 622 2, the decisions 624 and 627 one each) and
narrowed two more (the decisions 623 and 626), and stage 1's tenth
round, R95 to R103, with the decisions 600 to 608 and the stage 0
round's decisions 640 to 646, settled 21 (R101 11, R97 2, R103 2, R96,
R98, R99, R100, decision 644 one each, and R102, which decided against
the `pure` adapters the item asked for, one) and narrowed three more
(R101, and the decisions 642 and 643), and its eleventh round, R104
to R110, with the decisions 660 to 666, settled 20 (R104 12, R106 2,
R108 2, R105, R107, R109 and R110 one each), decided against one
(E0501 in `sisu check`, R104) and narrowed three more (R108), while one
was found stale (the compile-time evaluator's speed: a sieve of 4,000
takes 1.2 seconds and one of 20,000 1.8), and its twelfth round, R111
to R119, with the stage 0 round's decisions 680 to 694, settled 5 (R113
2, R114, and the decisions 684 and 685 one each), found one to be the
runtime's intended behaviour (decision 691: a scope whose body has no
result cannot carry a failure nobody joined, so G14's test joins its
tasks), decided against one (the `zip` temporaries, R114) and narrowed
eight more (R115 two, R116, R117, R118 with decision 683, R119, and the
decisions 682 and 694); 12 were
corrections to this
plan, its conventions or its tools, now made where they belong, and a
finding that several pages recorded is one item that names each page;
4 come from the maintenance round, 12 from step 14, which checked the
manual against the third round, 11 from step 15, which checked it
against the fourth, 8 from step 16, which checked it against the
fifth, 4 from step 17, which checked it against the sixth, 7 from
step 18, which checked it against the seventh, 3 from step 19,
which checked it against the eighth, 3 from step 20, which checked
it against the ninth, 2 from step 21, which checked it against the
stage 0 round's decisions 620 to 627, and 4 from step 22, which checked
it against stage 1's tenth round and the decisions 640 to 646, 2
from step 23, which checked it against the eleventh, and 1 from step
24, which checked it against the twelfth and the decisions 680 to 694.

### Syntax and names

- [14](../proposal/14-non-goals.md) lists "variadic functions" among what Sisu does not have, where [02](../proposal/02-syntax.md#declarations) gives `*xs: T` (X1).
- `for i, d in enumerate(own drafts)` is E0100 ("expected `->`"), so consuming a list with its positions takes a counter (chapter 13).
- Names are not normalised to NFC, so a letter written as a base and a combining mark is E0005 for the mark, and the confusable and mixed-script lints of D-048 are missing (R1).
- The prelude is wider than 09's fixed list: helpers such as `days_in_month` and `Formatter` are visible to every program (R13).
- D-055 keeps two silent differences from Python, a default argument computed at each call and a loop variable bound anew in each iteration, and neither is a row of the false-friend table that the model arm of [20](../proposal/20-pre-registration.md) measures (section 6.9, FF-1 to FF-6), so the experiment cannot see them (X8).

### Types and traits

- A `pub` type has a `Debug` form without asking, though [10](../proposal/10-tooling.md) makes it opt-in (chapter 7).
- D-060's allocator parameter, `list[T, A]` with `Global` in the prelude, is missing (E0208, E0201), so the chapter shows a defaulted parameter on a type of its own (chapter 15).
- `@derive(Plain)` accepts a `@repr(c)` struct with padding, whose `as_bytes` would read it, though 22 allows no padding in a plain type; R108 left it open, since the derive's template has no compile-time size of a field until the layouts are lowered (chapter 27, R14).
- Missing beside 03: `ptrsize` and `uptrsize`, `Sized`, `Decimal`'s `/` giving a `Ratio`, `graphemes` and the normalisation forms, `@derive(Into, From)`, a list literal typed as an `Array`, `FromLiteral` for `Url`, the sharing of instances with `--explain-size`, and the checked methods of the sized integers (`int` has `checked_add`, `checked_sub` and `checked_mul`, and `checked_pow` is `std.num`'s function only); R108 left the rest to the runtime, the design or the owner (R2, chapters 5, 6 and 24).
- [14](../proposal/14-non-goals.md) lists "implicit conversions" among what Sisu does not have, where D-016 keeps implicit lossless widening and the proposal's README says "implicit lossy conversions" (X1).
- A method that takes `own self`, changes a copy of its value and returns it, is no exit that checks the invariant, so `Gauge(level=4).filled(20)` gives a level of 24 under `invariant 0 <= self.level <= 10`; 07 checks construction and `mut self` methods only, so the gap is the design's, now the owner's question Q-78 (R11).
- A `Decimal` is not `Copy`, so a loop that pushes the same share needs `.copy()` (E0400), which a reader who thinks of it as a number does not expect; stage 1's twelfth round kept it, since a `Decimal` is of any size, and whether a fixed-size decimal type that is `Copy` stands beside it is the owner's question Q-79 (G31).
- `f64.total_order()`'s `TotalF64` is `Eq` and `Ord` but not `Hash`, so it sorts floats and is E0347 as the key of a `dict` or a `set`, which 03 says it serves too (R2, chapter 24).

### Ownership

- A loop over a `mut` parameter, `for i, k in enumerate(path)`, whose body reads `path` (`len(path)`), is E0405, the checker's known over-rejection 15, which needs a read of a `mut` parameter lowered as a shared reborrow in both front ends (chapter 13; assigned to stage 1).
- A `Weak` is not `Copy`, so a `Cell[list[Weak[T]]]` is read by `replace` and set back (chapter 13).
- A `CString`'s `as_ptr()` passed to a foreign function stops the interpreter with "a pointer to a value that no longer exists (`valid`)", though the `CString` lives across the call, because `bytes.as_slice()` makes a slice of copies of the bytes that dies when `as_ptr` returns; the native backend does not compile `mem.slice.as_ptr` of a `u8`; the fix, a slice that views its list's elements in place, waits (decision 688; chapter 27, G19; assigned to stage 0).
- A native debug build checks a pointer by its address, not by its provenance, the borrow it came from: a write through a pointer made from a shared borrow passes when a `mut` borrow also made a pointer to an overlapping place, a pointer into a frame that has returned passes when a later call of the same depth covers its stack, and a reference read after a drop through the pointer goes unchecked, where the interpreter stops each; decision 626 made the checks faster and left provenance out, and decision 686 measured what tagged pointers would change, every inline use of a pointer's word in both lowerings and the C calls, and left them for a round of their own (chapter 27; pointer provenance assigned to stage 0).
- `@no_alloc`, `@no_block` and `@stack(...)` compile without a word and check nothing, and are to warn that nothing checks them yet (decision 693), and the allocators are missing: `list.with_allocator` is E0351 and `try_push` E0304 (chapter 27; the warning assigned to stage 1).

### Errors

- An error that `main` returns is printed in its `Debug` form, `Context(message="...", cause="NotANumber(text=\"kolme\")")`, rather than its `Display` form (Q-74), and so is a `cli.Error`, `InvalidValue(name="--total", value="lots")` (chapter 8, G6).
- `.or_else`, `try_map` and `try_collect` are E0304, as planned (chapters 8 and 16).
- A `main` that passes an error on with `?` adds the line of the `?` under it, also in a transcript; returning it from an `else as e:` block keeps the line out (G6).
- A run-time message is printed without its code (`panic: ...`), so a reader has nothing to give `sisu explain`; the R codes are visible only in the catalog, and decision 689 keeps the form for now, since the codes would change 81 of the suite's expectations and 193 of the manual's sessions in one round (R17; decided against for now).

### Concurrency and capabilities

- `xs.par_iter()` is E0304, `par_iter_mut` is missing and `SoaList` is E0201, as planned, so stage 0 spells the parallel loop `par.iter(xs)` and the parallel map `par.map` (chapter 26, R9).
- `System` has no `stdout` and `stderr` fields, which [06](../proposal/06-concurrency-and-capabilities.md#the-rule) lists among the root capabilities; `print` and `print_err` are ambient instead (chapter 18); what they are is now the owner's question Q-76.
- The trace context is the path of spawn indices, `[task 2.1]`, not the trace and span identifiers of [06](../proposal/06-concurrency-and-capabilities.md#interceptors-and-trace-context), and `trace.wrap` with a `Tracer` is missing, so a program writes its own `cap.Interceptor` (G28).

### Testing

- The failure database of property tests, `@fixture`, `Expectation`, `--fuzz`, `--cover`, `--mutate` and the JSON output of `sisu test` are missing, as planned (chapter 19, G11).
- A test's output is captured and shown only with its failure (decision 466), and a test cannot read it, so a test cannot check what a `Drop` printed, and chapter 10's drop-order exercise predicts the order of `defer`s instead (chapter 10).
- A test that takes the real `System` runs under `--sim` against a simulated `System`, and outside it prints no token (D0031), where 17 says such a test can be neither simulated nor replayed and its token says so; decision 692 leaves the test runner's choice of a `System` and the token's message to the next round (X12; assigned to stage 0).
- A program's transcripts run on the Rust front end's build of it (decision 380), so a program that writes a form only stage 1 has, `xs[1:^1]`, a generator expression or a trait's constant, fails every transcript with D0128, though `sisu check` and its tests pass; decision 687 keeps them there until stage 1's `sir --tests` keeps the functions of the transcripts' world (R15, R17; assigned to stage 0 and stage 1).
- The Rust front end, whose build the transcripts run on, has neither R113's lookups by a `str` nor R117's `f.doc`, so a program that looks a `String` key up by a `str` fails each transcript with E0300, and one that calls `cli.help_text` with E0609, though `sisu run` and its tests pass (G6, R15; assigned to stage 0, with transcripts on stage 1's build).
- The conformance runner replays a failing test's token in a package case without the file the token names, so the replay of a failing test of `tests/NAME.sisu` runs no test and the case fails; a manual's package can show such a failure only in a session over a `NAME.files/` tree (R15).

### Metaprogramming

- An error in code that `comptime for` unrolls stands at the call that made the instance, with the unrolled line in a note, and nothing points at the line of the template that wrote it, the loop's (X7; assigned to stage 1).
- A derive's body must be the `return impl` alone (E0607), so a check of the type's shape goes inside a method (G21).
- `target.runtime` is `interpreter` in a native build too (`sisu run --native`): stage 1 takes `--target-runtime native` since R116, and the driver does not pass it yet (chapter 25, R12, R22; assigned to stage 0); when `sisu run` itself runs natively is now the owner's question Q-77.
- `@diagnostic_name(f)` is E0900 until compile-time code can make types, and `sisu eval` is D0003, as planned (chapter 25).
- `@fix_inline` compiles without a word and does nothing, and `@preview` marks an item whose uses warn (W0028), where D-072 has it ask for a feature of the language, of which there are none yet (chapter 28, X11, R14).
- `reflect.attributes_of` reaches the attributes of a struct, an enum or an error set only, and nothing lists a package's functions, so the framework of [08](../proposal/08-metaprogramming.md) that collects its routed functions at compile time cannot be written (R14).
- `@export("sauna_heat")` makes a global symbol that C calls with Sisu's convention, the addresses of the result and of each argument and a status byte, not with C's values, and stage 1's SIR does not name the export yet, which its thirteenth round writes (decision 683; R14, G19; the convention assigned to stage 0, the SIR's line to stage 1).

### The standard library

- The sized integers have no narrowing methods of their own but `truncate_to` (`to_u8()` of a `u32` is E0304), so a `u32` is narrowed through `to_int()` (chapter 24, R2).
- Reflection gives a struct's fields their `##` comment, `f.doc`, but not an item's own comment or a variant's fields' (R12; an item's `##` assigned to stage 1).
- `filter` hands its function a reference to each item, so no named function fits it: `filter(is_big)` with `def is_big(x: ref int)` is E0300 ("expected `(ref ref int) -> bool`"), and one that takes `word: ref str` is shown as `(str) -> bool` and refused too; the fix wraps it in a closure, `x -> is_big(x)` (chapter 18).
- `Zone.named` knows 52 zones, each with today's rules, which it gives an earlier year too, and the tz database with each zone's history, `std.x.tzdata` of 11, is not written (G9, chapter 21).

### Tooling and the driver

- `sisu explain CODE` prints the code's message alone, without the long form with an example that 10 describes, whose text is Q-66 (chapter 1, R17).
- `sisu test FILE` compiles the file alone even in a package's directory, so a file cannot import the module beside it (E0220, whose note now says that only a package makes them one program), and an exercise, which `tools/check.py` runs as `sisu test NAME.sisu`, cannot be a workspace (chapter 17).
- `sisu build --explain-size`, `sisu run --profile`, `sisu check --holes` and `--target` are D0009, and `sisu fix`, `why`, `review`, `publish`, `add`, `update` and `eval` are D0003, as planned, with the registry of D-031 they wait for (chapters 15, 25 and 28, G20, G26, G27).
- The native executable of a ten-line program is 6.25 MB in both profiles (21 MB when the driver is a debug build), 1.74 MB stripped: the link drops what nothing reaches, and what remains is the runtime's code, Rust's standard library, 3.8 MB of debug information and 0.4 MB of symbols, with the C library and `libgcc_s` linked dynamically, where [12](../proposal/12-interop-and-targets.md#binary-size-and-startup) links statically and plans a `release-size` profile (chapter 27, G20, R22).
- The manifest's runtime profile is the key `runtime` of `[package]` (`"full"`, the default, `"single"` or `"freestanding"`), which decision 694 names, and stage 1 reads it only from its thirteenth round, so `runtime = "single"` is still W0005 and no fact of `target` names the profile (R22; assigned to stage 1).
- `sisu run --stats` prints scheduling latencies that differ between runs, so only `--stats stack-depth` can be shown (chapter 19, G27).
- A worksheet's expression statement that shows an optional value or a result, `line.find("M")`, is E0318 under `sisu run` of the same file, so since R103 such a worksheet is no longer a script, though it starts with `#!/usr/bin/env sisu`, and the conformance runner records its E0318 (chapter 6, R16).
- `sisu repl FILE` names the program by the file in a panic's trace, `r2.sisu:4:12`, with the line of the session's program, one more than the file's own, which stage 1's `sir --repl` is to give from a source of its own for each input (decision 690; R16; assigned to stage 1).
- The interval that `sisu bench` reports is one run's, so on a busy machine two runs of one benchmark gave 100.3 to 110.2 µs and 134.1 to 184.7 µs (G27).
- The grammar of the architecture notes, "Parsing and the syntax tree", which stage 1's parser repeats in its `##!` comments, is a sketch without `trait_def`, `suite_def`, `bench_def`, `extern_block`, parameters, generic parameters, `where` clauses, visibility, `select`, `yield`, `unsafe:`, `comptime` statements, contracts or `expect`; R20 is written from the parser itself, and nothing checks either text against it (R20).
- [00](../proposal/00-vision-and-principles.md#who-sisu-is-not-for-yet) ("Anyone who needs it today. There is no compiler.") and [14](../proposal/14-non-goals.md#not-finished) ("There is no compiler.") predate stage 0 (X1).

### Diagnostics without fixes

- E0117 for Python's `f"..."` after `=`, `let s = f"n is {n}"`, has the fix now and still the message "unexpected a string after the end of the statement", the catalog's `unexpected {found}` filled with a noun phrase (R19, G2).
- A file with a parse error reports nothing of its names, so the `nil` of a file that also holds `go f()` goes unreported until the parse is fixed (G4).
- E0333 is the ownership checker's, so a run ends with D0021, which offers `--defer-borrow`; under it the Rust front end refuses `print` in a `pure` function with D0006, and runs `Atomic.load` as if it were pure (chapter 26).
- E0241 for a trait with a constant used as `dyn Shape` calls the constant a method: "its method `SIDES` cannot be called through it, because it is a constant" (R3).
- In a `pure` function, the consumer after an adapter, the `count()` of `split_whitespace().filter(...).count()`, is a second E0333 of the same chain, without the note `type.pure-adapter` that the adapter's has (chapter 18).
- `Ledger()` from another module, for a struct whose private field has no default, is E0258, whose note names `Ledger.new()`, and then E0337 for the same call, whose fix passes the private field as a hole, `bookings=todo("bookings")`, which E0258 would refuse again (G32).

## Open questions

Questions this plan could not settle are Q-62 to Q-70 in
[open-questions.md](../open-questions.md#questions-from-the-manual-plan-november-2026):
whether the reference is the specification (Q-62), where the exercise
runner lives (Q-63), how the tutorial and the teaching ladder relate
(Q-64), how hidden solutions and tests are (Q-65), where `sisu explain`'s
text is written (Q-66), which version a page describes and how it marks
unimplemented features (Q-67), whether learning is measured (Q-68),
translation (Q-69), and whether the examples join the conformance suite
(Q-70).
