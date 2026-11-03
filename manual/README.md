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
(`sisu ws`, `#!/usr/bin/env sisu`), `sisu fmt`, and how exercises work
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
division, complete a temperature conversion. Stage 0 runs all of it
except `round_to`, for which the chapter uses `round()` and
`truncate_to_int()` until it lands.

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
`Shared` and its counted `.copy()`, `Shared.same`, `Cell`, `Weak` and
cycles, `Arena[T]` with `Key[T]` for graphs, a table for choosing among
them, `@must_consume` with `close(own self)` and its rule for
containers, and `sisu audit --ownership`. Challenge II: a dependency
graph with cycle detection, first with an arena, then with `Shared` and
`Weak`. Stage 0 runs all of it except `Shared.same` (E0351), which the
chapter calls designed.

### Part III. Abstraction

**14. Traits** ([written](tutorial/14-traits.md)). Declaring a trait, required and provided methods, `impl
Trait for Type`, `Display` and `Debug`, the derives, the fixed set of
operator traits (D-022), supertraits, associated types and constants,
coherence and the orphan rule (with the root package's exception,
D-013), newtypes with `@delegate` and `@derive(Into, From)`, and `impl
Type` in another module (Q-12). Exercises: `Display` for a money type, an
`Add` for vectors, a newtype that stops metres being added to seconds.
Stage 0 runs all of it except `@derive(Into, From)` and a trait's
associated constants, which the chapter calls designed.

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
except generator expressions, `try_map` and `try_collect`.

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
`expect`, fixtures as values and `@fixture`, each test's own fakes and the
clock that advances only when every task waits, property tests (`check`,
`arbitrary[T]`, shrinking, the failure database), `exhaustive[T]`,
snapshots and transcripts with `--update`, doc examples, replay tokens,
`@refines`, fakes of your own traits (`Fake[T]`), benchmarks, and the
JSON output and cache. Exercises: turn three example-based tests into
one property, fix a program against its transcript (a transcript the
learner wrote would be its own judge). Stage 0 runs all of it except
`@fixture`, `Expectation`, an `expect` that lets the test go on (it
stops the test, as `assert` does), the failure database, the JSON
output, `--fuzz`, `--cover` and `--mutate`.

**20. Concurrency** ([written](tutorial/20-concurrency.md)). `task.scope` and `spawn`, `join` and how error sets
flatten, the scope's result, tasks that borrow from the frame, `Send`
and `Sync` in the messages that mention them, `Mutex`, `RwLock`,
`Atomic`, `Once` and `Condvar`, bounded channels and closing them,
`select` with timers and a cancellation arm, cancellation, `task.shield`,
deadlines, `task.race`, panics in tasks and supervision, a mutex's
invariant re-checked when a panic drops its guard (D-077), `log` and the
trace context, and testing all of it under the seeded scheduler
(`--explore`, `--scheduler pct`). Exercises: a fan-out to a slow fake
service on the test clock (a fake network answers at once, so a test
could not tell a fan-out from a loop), a worker pool, a race with a
timeout. Stage 0 runs all of it.

**21. A tour of the standard library** ([written](tutorial/21-standard-library.md)). One section per module a program
reaches for first: `std.fs` and `std.io`, `std.time` (with `Ticker` and
`Timeline`), `std.rand`, `std.env` and `std.process`, `std.json`,
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
it except TLS; a native build prints the executable's absolute path,
which no session can show, so the chapter shows `sisu build`.

### Part V. Further

**23. Contracts** ([written](tutorial/23-contracts.md)). `requires`, `ensures` with `result` and `old`,
`invariant` on structs and the rule that only the type's own methods
assign its fields, refinement types (`type Port = int where ...`) and
`try_from`, the check levels (`--contracts`), contracts in the
documentation and as property tests, contracts on trait methods, and
what waits for the verifier (`decreases`, ghost code). Exercises: a
bank account with an invariant, a binary search with its contract.
Stage 0 runs all of it except the contracts of a trait's methods, which
it accepts and does not check.

**24. Numbers in depth** ([written](tutorial/24-numbers.md)). The sized integers and `uint`, the operator
families for wrapping, saturating and checked arithmetic, conversions
that can lose information, strict floating point (one rounding per
operation, `mul_add`, pairwise `sum`, `@float(fast)`; D-074), NaN and
ordering, and the exact types `BigInt`, `Ratio` and `Decimal` with their
suffix-free literals and named rounding (D-076). Exercises: a checksum
over `u32`, a mortgage table in `Decimal`. Stage 0 runs all of it
except `@float(fast)`, which it accepts and ignores, `total_order`, and
literals that take an exact type beside an operator (`f * 2` with `f` a
`BigInt`), so the chapter types its literals with `let`.

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
the raw pointers and C interop under `sisu run`; `sisu test` does not
read unsafe code yet, so the exercises practise the safe side. The
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
`@deprecated`, `@fix_inline` and `@preview` without effect, so the
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
| G7 | Read and write JSON ([written](guides/json.md)) | `@derive(Encode, Decode)`, `json.encode`, `json.decode` validating refinements at the boundary, `json.parse` (stage 0 reads any struct by reflection, so the derive is not needed and gets E0243, and `decode` does not check refinements yet). |
| G8 | Match text with regular expressions ([written](guides/regex.md)) | `std.regex`, groups, the linear-time guarantee and what it rules out. |
| G9 | Test code that uses the clock ([written](guides/clock.md)) | `Clock.frozen_at`, `advance`, the test clock, `Ticker` under test. |
| G10 | Test code that uses the network ([written](guides/network.md)) | `Net.fake`, `http.FakeNet().respond_to`, request counts. |
| G11 | Write a property test ([written](guides/property-test.md)) | `check`, `arbitrary[T]`, `@derive(Arbitrary)`, reading a shrunk counterexample, the failure database (stage 0 lacks the derive, E0243, and the database). |
| G12 | Test a command-line tool ([written](guides/test-tool.md)) | Snapshots and transcripts, `--update` and reviewing the diff. |
| G13 | Find and fix a borrow error ([written](guides/borrow-errors.md)) | Reading E04xx diagnostics, `sisu explain-borrow`, `--defer-borrow` and the counterexample (Q-73, as for chapter 11). |
| G14 | Find a concurrency bug ([written](guides/concurrency-bug.md)) | `--explore`, `--schedule-seed`, PCT and the exhaustive scheduler, `sisu test --sim` with faults. |
| G15 | Reproduce a failure ([written](guides/replay.md)) | Replay tokens, `sisu run --record`, `sisu replay`. |
| G16 | Change a running program ([written](guides/live.md)) | `sisu run --watch`, epochs, `live.boundary()`, what restarts the program. |
| G17 | Make a loop parallel ([written](guides/parallel.md)) | `std.par`, reducers, `grain`, checking that the result is unchanged. |
| G18 | Use a region for per-request memory | `Region.bump`, `list.with_allocator`, why the list cannot escape (waits for allocators). |
| G19 | Call a C library ([written](guides/c-library.md)) | A trusted wrapper package with a capability-taking safe API (stage 0 runs it in the interpreter and natively; header import and tests of unsafe code wait). |
| G20 | Build a native executable ([written](guides/native.md)) | `sisu build --native`, `--release`, targets and cross compilation (stage 0 lacks `--target`, D0009). |
| G21 | Write a derive ([written](guides/derive.md)) | A trait's `comptime def derive` over `reflect.fields_of`. |
| G22 | Generate configuration | A module of `const` items rendered by `sisu eval` (waits for `sisu eval`). |
| G23 | Publish a package | Documentation coverage, the API diff, semver, provenance (waits for the registry). |
| G24 | Check what a dependency can do ([written](guides/audit.md)) | `sisu audit`, the capability summary, admission and its authority delta (stage 0 audits one file at a time; the summary and admission wait for the registry). |
| G25 | Move to a new edition | `sisu fix --edition` and `sisu fix --deps` (waits for `sisu fix`). |
| G26 | Plan with typed holes ([written](guides/holes.md)) | `todo`, blocked tests, `sisu check --holes` (D0009 in stage 0). |
| G27 | Benchmark and profile ([written](guides/bench.md)) | `bench` blocks, `black_box`, `sisu bench`, `sisu run --profile` (D0009 in stage 0). |
| G28 | Log and trace ([written](guides/log.md)) | `std.log`, levels, the trace context of scopes, `trace.wrap` over a capability (stage 0 writes every level and has no `trace.wrap`, so the guide writes its own interceptor). |
| G29 | Write a script with dependencies ([written](guides/script.md)) | A shebang file with inline metadata. |
| G30 | Embed a file at compile time ([written](guides/embed.md)) | `embed` and what it records for reproducible builds. |
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
| `assert`, `debug_assert`, `expect` | 1, 3, 19 | R5, R15 | |
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
| 13 | The maintenance round: the writers' corrections to this plan and [STYLE.md](STYLE.md), links and anchors (`tools/check_links.py`), the diagnostics entries the pages lacked, the generator of R17 retired, the check of "Every code" against the catalog, `SUMMARY.md`, and the findings in one list by area | in progress: corrections, links (0 broken), 43 new entries of R17, tools, `SUMMARY.md`, the findings list (209 in ten groups) and the review of 27 changed sessions done; the final checks next |
| | Total | about 159 |

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
the rounds R10 to R26 of [stage 1's rules](../compiler/stage1/RULES.md)
settled is no longer listed; each was checked against the compiler
before it went. The last group holds the diagnostics that lack a fix or
mislead, whatever their area.

The list holds 209 findings in ten groups. The rounds R11 to R26
settled 41 that the writers recorded (R24 alone 14, R25 9, R16 5), 12
were corrections to this plan, its conventions or its tools, now made
where they belong, and a finding that several pages recorded is one item
that names each page; 4 come from the maintenance round.

### Syntax and names

- A variadic parameter, `*prices: int`, stops the checker with "beyond the slice: ... variadic parameters", a message with no code, after which `sisu check` exits 0 and `sisu run` 1 (chapter 3, R20).
- [14](../proposal/14-non-goals.md) lists "variadic functions" among what Sisu does not have, where [02](../proposal/02-syntax.md#declarations) gives `*xs: T` (X1).
- Star unpacking, `let first, *rest = xs`, gets E0326, E0329 and E0387, none saying that the form is designed and missing, where a nested `def` gets E0900 (chapter 5, R3).
- `^` in a slice bound, `xs[:^1]`, is E0383 in both front ends, though 02 gives `a[i:^1]` (chapter 5).
- `xs[^1]` with `xs` a `Slice[int]` is E0383 ("appears only inside an index"), though it stands in one; on a `list[int]` it works (the maintenance round).
- A test's name is interpolated, so `test "len({3, 1})":` is E0103, and an escaped quote in a test's name is E0005 (chapter 5).
- `for i, d in enumerate(own drafts)` is E0100 ("expected `->`"), so consuming a list with its positions takes a counter (chapter 13).
- A generator expression is E0900, as planned, and then E0304 ("`list[int]` has no method `collect`"), which types it as a list (chapter 16).
- `it` in a list literal of functions, `[heat, it - 15]` typed `list[pure (int) -> int]`, is E0342, where an explicit closure works (chapter 26).
- Names are not normalised to NFC, so a letter written as a base and a combining mark is E0005 for the mark, and the confusable and mixed-script lints of D-048 are missing (R1).
- E0005 shows an invisible character as it is, so the message for a no-break space reads "`` ` ` `` cannot appear", and a byte-order mark at the start of a file is E0005 (R1).
- A keyword as a field's name is E0100 ("expected a name, found `:`"), and a function declared `def type()` cannot be called (E0103); only a parameter gets E0124 (R1).
- A one-line `if c: print(x)` is E0116 (an `if` expression without `else`) where E0102 would say that a block starts on the next line (R1).
- `debug_assert`, which 02 gives as the assertion a release build strips, is not a statement: `debug_assert x == y` is E0117 (R15).
- An attribute on the line of its declaration, `@derive(Eq) struct Point:`, gets E0117 and E0106, neither saying that an attribute stands on a line of its own (R14).
- An import inside a function is E0900 (R13).
- The parser takes any name in `pub(...)`, and every name but `package` makes the item `pub`, so Rust's `pub(crate)` compiles and exports the item to every package, a silent difference that D-062 rules out (R20).
- `import atlas.geo.point.Point`, an item named as a module, is E0203 ("there is no module") where the catalog has E0223 for it (R13).
- A dev-dependency can be imported by the package's own code, not only by its tests, in both front ends (R13).
- The prelude is wider than 09's fixed list: helpers such as `days_in_month` and `Formatter` are visible to every program (R13).
- A module that imports a function named like the module, `from bookings.demo import demo`, or a root module that imports `clean` from its module `clean`, gets E0202 ("defined more than once") with its "first defined here" label at line 1, column 1 (chapters 17 and 22).
- A parameter named like an imported module, `fs: F` beside `import std.fs`, hides the module, and each call such as `fs.walk(...)` is E0304 with no word about the module (chapter 21).
- `N.to_f32()` on a constant parameter `N` is read as an associated function of a type `N` (E0351); binding `N` with `let` first works (chapter 27).
- `collections.Deque[int].new()`, a type argument after a module's name, is E0200 for `int`, with the fix `uint`; `let q: collections.Deque[int] = collections.Deque.new()` works (the maintenance round).
- `sisu check` and `sisu run` report neither a field declared twice (E0205) nor a variant declared twice (E0206), and keep one of the two; the Rust front end of `sisu test` reports both (R3).
- D-055 keeps two silent differences from Python, a default argument computed at each call and a loop variable bound anew in each iteration, and neither is a row of the false-friend table that the model arm of [20](../proposal/20-pre-registration.md) measures (section 6.9, FF-1 to FF-6), so the experiment cannot see them (X8).

### Types and traits

- `Shared.same(a, b)`, which [02](../proposal/02-syntax.md#expressions-and-statements) gives for the identity of shared values, is E0351 (chapter 13).
- A trait's associated constant, which [03](../proposal/03-types-and-generics.md#traits) gives, is E0100 without a value, and with one it cannot be read as `Self.SIDES` or `Square.SIDES` (E0351) (chapter 14).
- A `const` in an enum's body compiles, and reading it as `Light.COUNT` is E0351 (R3).
- A supertrait cycle through another trait, `trait A(B)` with `trait B(A)`, compiles in both front ends; E0247 comes only for `trait A(A)` (R3).
- `impl Mul[Vector] for f64` compiles, but `2.0 * v` is E0310: an operator with a number on its left never reaches an impl (chapter 14).
- A `pub` type has a `Debug` form without asking, though [10](../proposal/10-tooling.md) makes it opt-in (chapter 7).
- D-060's allocator parameter, `list[T, A]` with `Global` in the prelude, is missing (E0208, E0201), so the chapter shows a defaulted parameter on a type of its own (chapter 15).
- `map(f)` over the `iter()` of a `list[int]`, with `def f(n: int)`, is E0300 ("expected `(ref int) -> _`, found `(int) -> int`"), since the items are references; `map(x -> f(x))` works (chapter 16).
- `sorted(xs)` with `xs` a `Slice[int]` is E0300 ("expected `Iterable[int]`, found `ref Slice[int]`"), though `for x in xs` works; `sorted(xs.iter())` is the spelling (chapter 26).
- A literal beside a `BigInt` or a `Decimal` does not take its type: `f * 2` and `d + 0.25` are E0310, though D-076 gives the exact types literals through `FromLiteral` and the ordinary operators (chapter 24).
- A `Decimal` literal is read from the shortest digits of its `f64`, so `3.50` is `3.5`, where [03](../proposal/03-types-and-generics.md#numbers) says a literal reaches it as written; `Decimal.parse("3.50")` keeps the places (chapter 24).
- A `u32` shifted by an `int`, `s << k`, widens to `int`, so `1u32 << 32` with the amount in an `int` is 4294967296, where a `u32` amount panics (chapter 24).
- `a < b` with a `u8` and an `i32` is E0369, though `a + b` widens (R2).
- `@float(fast)` is accepted without a word and changes nothing (chapter 24).
- `@repr(u8)` on an enum is accepted and changes nothing, an explicit discriminant `Red = 1` is E0117, and `F[_]`, the syntax 03 reserves for higher-kinded types, is E0100 with no word that it is reserved (R2).
- `@derive(Plain)` accepts a `@repr(c)` struct with padding, whose `as_bytes` would read it, though 22 allows no padding in a plain type, and it asks only that a struct has an attribute named `repr`, so `@repr(python)` passes (chapter 27, R14).
- Missing beside 03: `ptrsize` and `uptrsize`, `Sized`, the method form of checked arithmetic (`a.checked_add(b)`), `f64.total_order()`, `round_to`, `max_by` and `min_by` over a partially ordered key, `Decimal`'s `/` giving a `Ratio`, `graphemes` and the normalisation forms, `@derive(Into, From)`, a list literal typed as an `Array`, `FromLiteral` for `Url`, and the sharing of instances with `--explain-size` (R2, chapters 2, 5, 6, 14 and 24).
- [14](../proposal/14-non-goals.md) lists "implicit conversions" among what Sisu does not have, where D-016 keeps implicit lossless widening and the proposal's README says "implicit lossy conversions" (X1).
- Contracts on a trait's methods, which [07](../proposal/07-contracts.md#interaction-with-the-rest-of-the-language) makes binding on every implementation, are accepted and never checked, in either front end and through a generic call: a trait's `requires x >= 0` lets `push(-1)` through (chapter 23).
- A method that takes `own self`, changes a copy of its value and returns it, is no exit that checks the invariant, so `Gauge(level=4).filled(20)` gives a level of 24 under `invariant 0 <= self.level <= 10`; 07 checks construction and `mut self` methods only, so the gap is the design's as well (R11).
- In a function that returns `T ! E`, `result` is the whole result, so `ensures result > 0` is E0369 with no word that `result.is_ok()` or `result == Ok(...)` is what compares (R11).
- A `Decimal` is not `Copy`, so a loop that pushes the same share needs `.copy()` (E0400), which a reader who thinks of it as a number does not expect (G31).

### Ownership

- `[p for p in zip([1, 2], ["a", "b"])]` is E0406 for the two temporary lists, whose borrow ends inside the statement (chapter 5).
- `sorted` of a list whose items are not `Copy`, a list literal among them, gives a `list[ref T]`, which E0438 refuses since R21, with a note about copies that a caller of `sorted` cannot act on; `xs.sort()` works (chapters 7 and 14, R14).
- `var in_order = readings`, with `readings` a borrowed `list[int]` parameter, followed by `in_order.sort()`, is E0341 ("not declared with `var`"), though it is: the binding holds a borrow, and the note names `ref mut` (chapter 5).
- A borrowed parameter passed to an `own` parameter, `total(items)` in a function that borrows `items`, is E0300 ("expected `list[Item]`, found `ref list[Item]`"), whose one fix copies, with no word that `own` moves it or that a borrowed parameter would do (chapter 10).
- A change inside a loop over the same list or dict is E0405, whose note says to move the change "after line N", N being the last line of the loop's body, inside the loop, and whose label points at the loop's head (chapter 11).
- A loop over a `mut` parameter, `for i, k in enumerate(path)`, whose body reads `path` (`len(path)`), is E0405, the checker's known over-rejection 15 (chapter 13).
- A field assigned through a `Shared` is E0341 ("not declared with `var`") with a note about `ref mut`, also when the binding is a `var`; neither says that a `Shared` value changes only through a `Cell` or a `Mutex` inside it (chapter 13).
- A `Weak` is not `Copy`, so a `Cell[list[Weak[T]]]` is read by `replace` and set back (chapter 13).
- `{b.note ?? "-"}`, with `b` the variable of a loop over a list of structs and `note` a `String?`, is E0419 ("cannot move out of `b.note` through a reference"); `if let Some(note) = b.note` reads it (G7).
- E0423's note says to copy the value into a local and capture that, which is E0423 again for a `String`: stage 0's `own` closures keep `Copy` values only (chapter 16, R7).
- A `CString`'s `as_ptr()` passed to a foreign function stops the interpreter with "a pointer to a value that no longer exists (`valid`)", though the `CString` lives across the call, and the native backend does not compile `mem.slice.as_ptr` of a `u8` (chapter 27, G19).
- `@no_alloc`, `@no_block` and `@stack(...)` compile without a word and check nothing, and the allocators are missing: `list.with_allocator` is E0351 and `try_push` E0304 (chapter 27).

### Errors

- An error that `main` returns is printed in its `Debug` form, `Context(message="...", cause="NotANumber(text=\"kolme\")")`, rather than its `Display` form (Q-74), and so is a `cli.Error`, `InvalidValue(name="--total", value="lots")` (chapter 8, G6).
- `.or_else`, `try_map` and `try_collect` are E0304, as planned (chapters 8 and 16).
- An inferred error set, `-> !`, leaves out what a `?` passes on inside a `task.scope` body: each such `?` is E0321 ("`HttpError` errors cannot leave this function, whose error set is `net.UrlError`") (chapter 22).
- A `main` that passes an error on with `?` adds the line of the `?` under it, also in a transcript; returning it from an `else as e:` block keeps the line out (G6).
- A run-time message is printed without its code (`panic: ...`), so a reader has nothing to give `sisu explain`; the R codes are visible only in the catalog (R17).
- R0103 (`contract.refinement-failed`) is in the catalog and nothing raises it (R11).

### Concurrency and capabilities

- When a task fails while the scope's block is joining an earlier sibling, the failure cancels the sibling, the block's `join()?` returns `Err(Cancelled)`, and that becomes the scope's result, hiding the failure that [06](../proposal/06-concurrency-and-capabilities.md#tasks-and-scopes) says the scope returns (chapter 20).
- Tasks spawned in a loop that change a captured `var`, `s.spawn do: count += 1`, get E0402, whose message names the closure's whole text as the value borrowed, says that it "cannot be modified", and says nothing about a `Mutex` (chapter 20).
- E0428's note offers `own () -> ...`, which moves the list into the first task and leaves none for the next; making the value outside the scope's block, which works, is not offered (chapter 20).
- `for x in par.iter(xs):` is a plain loop, whose body can change a variable outside it or print without a word, where [06](../proposal/06-concurrency-and-capabilities.md#data-parallelism) makes a `par` body pure; `par.map` and the reducers' methods are checked (chapter 26, G17).
- `a.load()` of an `Atomic` compiles in a `pure` function, though 06 lists `Atomic` among the operations that observe the schedule; `lock`, `recv`, `try_recv` and `select` are E0333 (chapter 26).
- `xs.par_iter()` is E0304, `par_iter_mut` is missing and `SoaList` is E0201, as planned, so stage 0 spells the parallel loop `par.iter(xs)` and the parallel map `par.map` (chapter 26, R9).
- The unbounded channel of 06 is missing: stage 0's channels are bounded only (R9).
- `System` has no `stdout` and `stderr` fields, which [06](../proposal/06-concurrency-and-capabilities.md#the-rule) lists among the root capabilities; `print` and `print_err` are ambient instead (chapter 18).
- The trace context is the path of spawn indices, `[task 2.1]`, not the trace and span identifiers of [06](../proposal/06-concurrency-and-capabilities.md#interceptors-and-trace-context), and `trace.wrap` with a `Tracer` is missing, so a program writes its own `cap.Interceptor` (G28).

### Testing

- `expect condition` stops a test at its first failure, as `assert` does, where [17](../proposal/17-testing.md#assertions) makes it the form that lets the test go on (chapter 19).
- An assertion whose parts call the program's own functions inside a method call, `assert temps.contains(average(temps))`, prints no values, though 17 shows every sub-expression (chapter 19).
- The `.sisu` files of a package's `tests/` directory, 17's black-box tests, are not run; only its transcripts are (chapter 19).
- The test that `@refines` adds fails at `<arbitrary>/main.sisu:7:5`, in the module the compiler writes, rather than at the `@refines` line (chapter 19).
- The failure database of property tests, `@derive(Arbitrary)` (E0243), `@fixture`, `Expectation`, `--fuzz`, `--cover`, `--mutate` and the JSON output of `sisu test` are missing, as planned (chapter 19, G11).
- A test captures no output, so a test cannot see a `Drop`, and chapter 10's drop-order exercise predicts the order of `defer`s instead (chapter 10).
- A transcript line of a run that fails holds the error's trace (`at counter.sisu:34:19`), so the transcript breaks whenever the program's lines move (chapter 9).
- An `assert` whose expression moves a value, `assert visits.into_inner() == 2`, is E0400 twice ("moved ... in an earlier iteration of the loop") under `sisu test`, whose Rust front end keeps what R22 fixed in stage 1 (chapter 20).
- A hole reached in a task that nothing in the test joins, such as a handler's hole inside `http.serve`'s connection task, fails the test with a deadlock report instead of reporting it blocked by the hole (chapter 22).
- A `sim.always` condition that locks a `Mutex` that a task holds at the switch hangs `sisu test`, with no deadlock report and memory growing past 2 GB (chapter 22).
- A typed hole reached at compile time is E0602, so a test that uses a constant computed from a hole does not compile instead of being blocked (chapter 25).
- `sisu test` and `sisu sir --tests` use the Rust front end, which reads neither `std.mem`, `std.ffi`, `extern` nor `unsafe` (E0220, E0900), so no test of unsafe code runs, though [22](../proposal/22-unsafe.md#interaction-with-the-rest-of-the-language) makes `sisu test` where the checked memory model checks it (chapter 27).
- E0971, the check that a `sim` build cannot escape the simulation, is not reached: `sisu test --sim` uses the Rust front end, which rejects `extern` with E0900 (R22).
- Under `sisu test --explore N`, the log lines printed are those of the first schedule, seed 0, and not of the one that fails (G14).
- Under `--sim`, a task of a scope whose body returns nothing fails the test with "a task failed ... and nobody joined it", where the test meant to look at the state the tasks left (G14).
- `sisu run --explore N` stops only at a run that fails; a seed whose output differs from seed 0's, with status 0, is not reported (R9).
- A replay token's content hash covers the source and not the files it embeds, so after an embedded file changes the old token is accepted and runs the new program (G30).
- `Revoked` is in every I/O error set (Q-51), so `sisu test --sim --faults` injects it into calls on a file system that was never made revocable (X12).
- A test that takes the real `System` runs under `--sim` against a simulated `System`, and outside it prints no token (D0031), where 17 says such a test can be neither simulated nor replayed and its token says so (X12).
- A blocked test and a run that reaches a hole print the hole's name only, not its closure ("the `Debug` rendering of every binding in scope") that 17 and 10 describe (X13).

### Metaprogramming

- Stage 1's compile-time evaluator stops at `String.push`, `list.join`, `str.byte_at`, `sum`, a `for` over a list (`iter.ref`), a comprehension (`list.new`), a `set`, a `dict`, `int.to_f64` and `todo` with "beyond the slice: a `comptime` expression that needs the intrinsic ...", a message with no code, after which `sisu check` exits 0; the Rust front end runs them all (chapter 25, R3, R12, G30, X6).
- The same evaluator is slow on lists: a sieve of 20,000 numbers takes 90 seconds, within the budget, and one of 4,000 takes 3.7 seconds where the Rust front end takes 0.3 (chapter 25).
- `reflect.fields_of(int)` is an empty list, so a template that forgets `if comptime reflect.is_struct(T)` compiles and gives empty output (chapter 25).
- A diagnostic in a derived impl, such as E0347 for a field without `Eq` or E0235 for an impl written beside `@derive(Eq)`, is placed in `<derive>/main.sisu` with no label at the `@derive`, and a hole in a derive's template is reported once for each type (chapter 25, R14).
- An error in code that `comptime for` unrolls is reported in a module the compiler writes, `<total[Team]>/main.sisu:3:5`, with the unrolled line, and nothing points at the loop's line or the call that instantiated it (X7).
- In a template, an interpolated literal made only of compile-time values, `var out = "{reflect.name_of(T)}"`, becomes a `str` in the instance, so `return out` from a function returning a `String` is E0300 in the instance's module (R14).
- A builtin type given to a `comptime def` that takes `T: type`, as `comptime name(bool)`, is E0200 in `sisu run` and `sisu check`, with the fix `uint` for `int`, though `String`, `uint` and the program's own types work and the Rust front end takes every one (R12).
- `reflect.construct(T, {"name": name})` in a template whose `name` is a plain `String` parameter is E0300 ("found `ref String`") in the instance's module; an `own` parameter works, and nothing says so (R12).
- Inside a derive's template a field has no `ty` (E0608, "a field has no `ty`"), though a generic function that reflects reads it, so a derive cannot choose `Display` or `Debug` field by field (G21).
- A derive's body must be the `return impl` alone (E0607), so a check of the type's shape goes inside a method (G21).
- `target.runtime` is `interpreter` in a native build too (`sisu run --native`) (chapter 25).
- `@diagnostic_name(f)` compiles without a word and does nothing, and `sisu eval` is D0003, as planned (chapter 25).
- `@deprecated("...")`, `@fix_inline` and `@preview("...")` compile without a word, and a call of a deprecated function gets no warning, where [13](../proposal/13-evolution-and-governance.md#deprecation) says it warns with the replacement named (chapter 28, X11).
- An attribute that Sisu reads, standing where it means nothing, is accepted without a word: `@derive(Debug)` or `@must_consume` on a function, `@comptime_budget` or `@export` on a struct, and `@tag("slow")` on a suite, which tags none of its tests; any other attribute is too, a misspelt `@deprecatd` among them, and nothing lists the ones the toolchain reads (R14, X11).
- `@cfg(test)` on a field, a variant, or a method in a struct's or an enum's own body is kept in every build, where a method of an impl is left out (R14).
- `reflect.attributes_of` reaches the attributes of a struct, an enum or an error set only, and nothing lists a package's functions, so the framework of [08](../proposal/08-metaprogramming.md) that collects its routed functions at compile time cannot be written (R14).
- `@export("sauna_heat")` checks the signature (E0989) and nothing more: a native build keeps the function a local symbol (`sisu$heat$2`), and the `@repr(c)` types that [12](../proposal/12-interop-and-targets.md#c-interop) asks for at the boundary are not checked (R14).
- In a package, `embed("data/words.txt")` reads `src/data/words.txt`, relative to the module's directory, and a path from the package's root, `../data/words.txt`, is E0612, where [08](../proposal/08-metaprogramming.md) resolves the path from the package's root (G30).

### The standard library

- `max(xs)` and `min(xs)` compare two values, so `max(scores)` is E0308 with a fix that adds a hole, and Python's form over a collection, which 02's prelude suggests, is missing (chapter 5, G2).
- A `dict[String, V]` is looked up with a `String`, so a `str` key, `recipes.get(dish)`, is E0300 whose fix is `.to_string()`, an allocation for each lookup (chapters 10 and 20, G2).
- `prices.get("bun") == Some(280)`, with `prices` a `dict[String, int]`, is E0300 ("expected `ref int`, found `int`"), since `get` gives an optional reference, with no fix; `prices["bun"] == 280` works (chapter 19).
- The iterator methods (`count`, `collect`, `map`, `sum`, `join`) are not `pure`, so `pure def word_count(text: str) -> int` returning `text.split_whitespace().count()` is E0333 (chapter 18).
- `items.is_sorted()` on a `list[int]` is E0304; only `Slice[T]` has it (chapter 23).
- The sized integers have no `truncate_to` and no narrowing methods of their own (E0304), no `MAX` or `MIN` (E0351), and no `count_ones`, `leading_zeros` or `rotate_left` (E0304), which a checksum reaches for (chapter 24).
- The comment of `std.num`'s `Rounding` says that Python's `decimal` names the same seven modes; it has an eighth, `ROUND_05UP` (chapter 24).
- A `Shared[String]` in an interpolation prints its value's `Debug` form, `"Aino"` with its quotes (the maintenance round).
- `cli.help_text[T]()` leaves out the fields' `##` comments, so a tool's help says what kind of value an option takes and not what it is for (G6).
- `@derive(Encode, Decode)`, which [11](../proposal/11-stdlib.md) gives `std.json`, is E0243, and `decode` does not validate refinements at the boundary: a field `hour: Hour` is E0811 at `<std>/<from_json[Hour]>/json.sisu:3:19`, which the reader cannot see, and text that breaks a struct's `invariant` makes `decode` panic (status 101), so untrusted text can stop the program (G7).
- A back-reference in a regular expression, `(\w+) \1`, and a flag or a named group, `(?i)` or `(?P<h>...)`, are `Syntax` errors that name what the parser met or wanted, with no word that back-references are left out for linear time or that flags, named groups and classes such as `\p{L}` are not in stage 0 (G8).
- `DateTime` has no parts (`t.hour` is E0303), and there is no `Instant` and no time zone, which 11 lists for `std.time` (G9).
- `HttpError.Network(message)` holds the `NetError` as text, so a client cannot tell a refused connection from a closed one, and `http.FakeNet` answers only `GET`, only with `200` or `404`, so a test cannot script a `503` or a slow answer (G10).
- `std.log` writes every level, with no setting that leaves out `debug`, and a record is a message without fields, where 11 makes it a structured facade (G28).

### Tooling and the driver

- `sisu explain CODE` prints the catalog's line only, with its escapes, `{{` and `}}`, where the reader sees one brace; the long form with an example that 10 describes is Q-66 (chapter 1, R17).
- The conformance runner cannot pin the human form of a compile error that stops a run (its record-and-replay check needs a log), and a `.repl` capture from piped input does not look like a terminal session, so the manual's sessions cover the first and it shows worksheets rather than REPL sessions (chapter 1).
- `sisu explain` and `sisu new` without their argument are D0004 ("`sisu explain` needs a file"), where they need a code and a name (R16).
- `sisu build` and `sisu sir` of a program with errors end with D0006, "the program was not run" (R16).
- `sisu` has no `--version` and no `version` command (D0003), though Q-67 asks each page to state the version that checked it; the toolchain's version, 0.0.1, appears only inside replay tokens and capability logs (R16).
- `sisu test --std` without a file needs a package (D0064), whose message names the absolute directory, though the standard library's examples need none, and with a file it runs the file's tests too (R16).
- `--stats` followed by a word it does not know takes the word as the file (`sisu run --stats bogus hello.sisu` is D0001 for `bogus`); only `--stats=bogus` gets D0094 (R16).
- The architecture notes put a package's test cache in `build/test-cache`; it is `build/debug/test-cache` (R16).
- `sisu run FILE -- ARGS` passes the `--` to the program as its first argument, where a package's `sisu run -- ARGS` does not (chapter 18).
- `sisu build --native` prints the executable's absolute path, so no session can show it; `--output PATH` prints the path given (chapter 22).
- A single file cannot import the module beside it (E0220), and `sisu test FILE` compiles the file alone even in a package's directory, so an exercise, which `tools/check.py` runs as `sisu test NAME.sisu`, cannot be a workspace (chapter 17).
- `sisu fmt` removes an import that a file does not use yet, so an exercise cannot hand the learner the import its solution needs, and an unused import gets no warning, though 09 says it warns (chapter 21, R13).
- `sisu build --explain-size`, `sisu run --profile`, `sisu check --holes`, `sisu audit --unsafe` and `--target` are D0009, and `sisu fix`, `why`, `review`, `publish`, `add`, `update`, `eval` and `lint` are D0003, as planned, with the registry of D-031 they wait for; stage 1 has `lint` (R11) and `audit --unsafe` (R7), which the driver does not pass on (chapters 15, 25, 27 and 28, G20, G26, G27).
- `sisu build --native --release` builds a program with a typed hole, printing W0001 and `built`, with status 0, though D-061 and 17 say that release builds reject holes "so blocked is never a shipped state" (G26, X13).
- The native executable of a ten-line program is 17.5 MB in both profiles, links dynamically to the C library, `libm` and `libgcc_s`, and keeps its debug information, where [12](../proposal/12-interop-and-targets.md#binary-size-and-startup) links statically and plans dead-code elimination and a `release-size` profile (chapter 27, G20, R22).
- 12 chooses a runtime profile "in the manifest" without naming the key, and stage 0 has none, so `runtime = "single"` in `[package]` is W0005, and no fact of `target` names the profile (R22).
- E1022 asks for `--accept-unsafe`, which `sisu check`, `run` and `build` reject (D0009), though stage 1 reads it; the only way to accept a raised `unsafe` level is to delete `sisu.lock` (R13).
- The manifest's `edition` takes any text, `edition = "1999"` included, without a word (chapter 28).
- `sisu bench` reports a mean of one run, with times of one benchmark that differ between runs by more than two ways of writing it do (153 and 369 µs), and `--stats` prints scheduling latencies that differ between runs, so only `--stats stack-depth` can be shown (chapter 19, G27).
- A counterexample run of `--defer-borrow` prints the whole SIR of the function, 40 lines for a loop of three, between the account of the conflict and its `outcome:` line (G13).
- `sisu audit --caps src/store.sisu` reads the module alone, so its import of the package's root is E0220, followed by an E0321 that calls the error set `{}`, and with no file `sisu audit` is D0004: a package's capabilities cannot be audited (G32).
- The grammar of the architecture notes, "Parsing and the syntax tree", which stage 1's parser repeats in its `##!` comments, is a sketch without `trait_def`, `suite_def`, `bench_def`, `extern_block`, parameters, generic parameters, `where` clauses, visibility, `select`, `yield`, `unsafe:`, `comptime` statements, contracts or `expect`; R20 is written from the parser itself, and nothing checks either text against it (R20).
- [00](../proposal/00-vision-and-principles.md#who-sisu-is-not-for-yet) ("Anyone who needs it today. There is no compiler.") and [14](../proposal/14-non-goals.md#not-finished) ("There is no compiler.") predate stage 0 (X1).

### Diagnostics without fixes

- `'Aino'` is E0016 (`lex.bad-char-literal`) with no fix; the fix is double quotes (chapter 2).
- A keyword used as a name, `let type = "csv"`, gets E0105 and E0103, neither saying that the word is a keyword (chapter 2).
- `let Total = 10` gets E0327, E0329 and E0200, none saying that a binding's name starts with a lowercase letter (chapter 2).
- `let small: u8 = large` is E0300 with no fix, where `large.to_u8()` would teach (chapter 2).
- E0300 for `return total / count` in a function that returns an `f64` has the note "`=` cannot mix `f64` and `int`", naming an `=` that the line does not have (chapter 2).
- A keyword-only argument passed by position, `url("example.org", 443)`, is E0305 counting only the positional parameters, with no word that `port` is keyword-only (chapter 3).
- A positional-only parameter passed by name gets E0306 ("has no parameter named `value`") and E0308, whose fix adds `value=todo("value")`, the same mistake again (chapter 3).
- A misspelt keyword, `prot=443`, is E0306 without the similar-name fix that E0200 has (chapter 3).
- `switch day:` is E0117 ("unexpected `day` after the end of the statement") and `i++` E0103, neither pointing at `match` or `i += 1` (chapter 4).
- An assignment to a parameter, `n //= 2`, is E0312, whose message says to declare it with `var` and whose fix makes the parameter `mut`, which changes every call; a local `var value = n` is not offered (chapter 4).
- E0338 for `let first = queue[0]`, with `queue` a local list, calls the list "a borrowed value", and its fix offers `.copy()` but not `remove` or `pop` (chapter 10).
- E0310 for `a - b` on a struct without `Sub` does not name the trait to implement (chapter 14).
- E0231 for a trait used as a parameter's type offers `dyn Priced`, which is E0395 there; the generic parameter of chapter 15 is not offered (chapter 14).
- `sorted` of a `list[f64]` is E0347, whose fix adds `impl Ord for f64`, which the root package may write (and which panics on NaN) but a library may not (E0236); the fix is offered in a library too (chapter 14).
- A value passed to a `ref dyn Priced` parameter without `ref` is E0300 with no fix that adds it (chapter 15).
- E0335 names the function passed (`noisy_square`) as the impure callee, where 03 asks for the operation responsible (`print`) (chapters 16 and 26).
- E0204 for an item that `@cfg(feature=...)` left out does not name the feature (chapter 17).
- A missing import of a type used as `Deque[int]()` is E0200 for `Deque` and another E0200 for `int`, whose fix offers `uint` (chapter 17).
- E0421 for a `Lent[Fs]` kept in a field of type `Fs` has the note "put an owned value in it instead, such as a `String` made with `.to_string()`", which is written for text; for a lent capability the fixes are a parameter of type `Fs` or the work done inside the call (chapter 18).
- A test that hands a `TestClock` to a function that takes `sys: System` is E0300 ("expected `System`, found `TestClock`") with no fix that narrows the parameter to `Clock` (chapter 18).
- `fs.read_to_string(path)?.lines()`, Rust's habit, is E0370 ("`?.` needs an optional value, found `String ! IoError`"), since `?.` is one token, with no word that `(x?).lines()` or a `let` is the spelling (chapter 18).
- `std.json`'s error set is `json.JsonError`, and `JsonError` alone is E0201 with the fix `IoError` (chapter 21).
- Python's `"0" * 64` is E0310 ("`*` cannot be applied to `str` and `int`") with no word about `repeat` (chapter 21).
- `self.save(x) else as e:` as a statement is E0117 ("unexpected `else` after the end of the statement") with no word that `else` belongs to a `let`, nor the fix `if let Err(e) = ...` (chapter 22).
- `request.body.decode() ?? ""` is E0371 ("`??` needs an optional value on its left, found `String ! Utf8Error`") with no fix offering `.or(...)` (chapter 22).
- E0413 for a function whose `str` result does borrow a parameter, `longer(a: str, b: str) -> str`, offers a `String` result first, which copies, before the `from` clause that fits (chapter 11, the maintenance round).
- E0806, E0810 and E0417 write "a `{ty}`" for every type, so the messages read "part of a `Opening`", "`24` is not a `Hour`" and "a `Order`" (chapter 23, R14).
- E0801 names the function that is not `pure` with no fix that writes `pure def`, and E0806 offers no fix, where the fix is a method of the type (chapter 23).
- A `ref int`, such as the variable of a loop over a `list[int]`, passed where a refinement of `int` is expected is E0300 ("expected `Percent`, found `ref int`") without E0811's note about `try_from`, which an `int` gets (chapter 23).
- E0310 for an `i16` plus a `u32` offers no conversion (chapter 24).
- E0609 for a plain `for` over `reflect.fields_of(T)` says "a list is a compile-time value with no run-time form" with no fix offering `comptime for`, and the template's copy for `int` is E0324 in `<to_csv[int]>/main.sisu` (chapter 25).
- E0981 names a builtin by its internal name, `ptr_mut.read`, where the program writes `ptr mut T` (chapter 27).
- `enumerate(lines, start=1)`, Python's form, is E0306 ("`enumerate` has no parameter named `start`") with no fix (R16, G2).
- `Light.Blue`, for a variant that `Light` does not have, is E0351 ("`Light` has no associated function `Blue`") where E0327 ("`Light` has no variant `Blue`") would say what is wrong; E0327 comes only from a pattern (R17).
- `1.` and `.5`, Python's floats, are E0100 and E0103 with no word about the digits a point needs (R1).
- E0115 (`parse.python-conditional`) comes only where a statement starts, so `print("big" if n > 2 else "small")` is E0100 ("expected `)`, found `if`"), and E0123 (`parse.walrus`) likewise: `if (n := len(xs)) > 2:` is E0100 (R19).
- `int("42")` is E0200 with the fix `uint`, and `str(n)` E0200 with none (R19).
- Most other habits get a diagnostic that does not name the Sisu spelling: `let mut` (E0105), `&&`, `||` and `!` (E0103), `x as f64` (E0100, or E0117 after a call), Rust's `// comment` (E0117), `=>` in a `case` (E0100), `&xs` (E0103), `0..3` (E0100), `Vec<T>` (E0117), `println!` (E0117), `x is None` (E0100), `lambda` (E0117), `f"..."` (E0100), `class` (E0107), `try:` (E0117), `d.get(k, default)` (E0305), and Go's `go f()` (E0117) and `nil` (E0200) (R19, G3, G4).
- A file with a parse error reports nothing of its names, so the `nil` of a file that also holds `go f()` goes unreported until the parse is fixed (G4).
- `let n: int? = 5` is E0300 with no fix that writes `Some(5)` (R2).
- A `BigInt` literal beyond `int` is E0374, whose note gives `int`'s range, with no word about `BigInt.parse` (R2).
- A `version` that is not `MAJOR.MINOR.PATCH` is E1003 and also E1002 ("`[package]` needs `version`"), though the key is there (R13).
- An import of a dependency whose `when` excludes the target is E0260, whose message asks to add it to `[dependencies]`, where it is (R13).
- A script whose inline metadata names a dependency by version, `stove = "1.2"`, gets E0220 first, which names a `sisu.toml` that a script does not have, and then E1010, which says what to write (G29).
- E0203 for `from atlas.geo.points import Point`, one letter from the module `atlas.geo.point`, has no similar-name fix, where E0220 has one (R17).
- E0237 for an impl without its associated type is followed at each use by E0347 ("`Bench.Item` does not implement `Display`"), a second error of the same chain (R17).
- E0812 says to "change a copy of the base", and the `.copy()` of a refined list is still the refinement, so the copy is E0812 again; a copy typed as the base, `var more: list[int] = picks.copy()`, works (R17).
- E0501 has no place in the source, and `sisu check` does not report it at all: only a run, which lowers the program, does (R17).
- E0106 for an attribute before a statement says "expected a declaration, found `let`" and nothing about the attribute (R17).
- E0301's one fix for `sum(points) / len(points)` writes `//`, Python 2's floor division, where a port of Python 3 means the float division that its message names and no fix offers, and FF-3 of [20](../proposal/20-pre-registration.md) expects "a fix-it offering `//` or a float conversion" (G2, X8).
- A `str` variable given a field of the loop variable inside a nested loop, `who = turn.name`, is E0402 pointing at `for turn in turns:`, with no word that the loop binds `turn` anew while `who` still borrows it; `turn.name.copy()` and a `String` are the fix, which the note's `.copy()` only half names (G26).
- `Ledger()` from another module, for a struct whose private field has a default, is E0258, whose only fix makes the field public; an associated function such as `Ledger.new()` is not named (G32).
- `from ledger import Taken`, a variant of a `pub` error set, is E0204 ("module `ledger` has no item `Taken`") with no word that a variant is reached as `BookingError.Taken` (G32).
- E0310 for `/` between two `Decimal`s, which D-076 makes a `Ratio`, offers no fix, where `div(other, places, mode)` and `to_ratio()` are what stage 0 has (G31).
- E0347 for a field type without `Display`, at `<derive>/main.sisu`, offers the fix `:?`, an edit of code that the user of `@derive` cannot reach (G21).
- E0347 for `Pair(Plain(1), Plain(2)) == ...`, where `Pair[T]` derives `Eq` and `Plain` has none, offers "derive it: `@derive(Eq)`" for `Pair`, which has it; the type without `Eq` is `Plain` (R14).
- E0266 for `@cfg(target.os == "linux")` does not name `if comptime target.os == "linux":`, which is how a body tests the target (R14).
- With `[trust]` left out, the build reports E0703 at each `extern` and `unsafe` block of the dependency before the E1021 that names the fix (G19).
- E0310 in code that `comptime for` unrolls says "`+` cannot be applied" for a `+=` (X7).

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
