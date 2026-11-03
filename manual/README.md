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

| Directory | Holds | Serves |
|---|---|---|
| `tutorial/` | 28 chapters in five parts, read in order | learning by doing |
| `guides/` | 32 how-to guides, each a task | doing a task |
| `reference/` | 22 pages that describe the language and the toolchain | looking something up |
| `explanations/` | 14 essays on why Sisu is the way it is | understanding |
| `examples/PAGE/` | every program a page shows, with its expected output | the checks |
| `exercises/CHAPTER/` | the tutorial's exercises, as the learner starts them | practice |
| `solutions/CHAPTER/` | a solution for each exercise, and its start-state session | comparison |
| `tools/` | `check.py`, the manual's own checker | the checks |

`PAGE` is a tutorial chapter's file name without `.md` (`01-hello`), or
the kind and the name of any other page (`guide-read-files`,
`reference-patterns`, `explanation-ownership`). The standard library's
reference is not written by hand: `sisu doc --std` generates it from the
`##` comments of [compiler/std](../compiler/std/), and `sisu test --std`
runs its examples.

The manual has 96 pages: 28 tutorial chapters, 32 guides, 22 reference
pages and 14 explanations, besides the generated library reference.

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
ways, predict a drop order. Stage 0 runs all of it.

**11. Borrowing** ([written](tutorial/11-borrowing.md)). The three parameter conventions (borrow, `mut`, `own`)
and `mut` at the call site; `ref` and `ref mut`; the rule of one mutable
or many shared borrows; borrows that end at their last use; returning a
borrow and the `from` clause; `for x in mut xs` and `for x in own xs`;
closures that borrow what they capture; `sisu explain-borrow` and the
monitor's counterexample run (`--defer-borrow`). Exercises: from the
wrong-ownership corpus, each a fix with the diagnostic as the guide.
Stage 0 runs all of it.

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
`Weak`. Stage 0 runs all of it.

### Part III. Abstraction

**14. Traits** ([written](tutorial/14-traits.md)). Declaring a trait, required and provided methods, `impl
Trait for Type`, `Display` and `Debug`, the derives, the fixed set of
operator traits (D-022), supertraits, associated types and constants,
coherence and the orphan rule (with the root package's exception,
D-013), newtypes with `@delegate` and `@derive(Into, From)`, and `impl
Type` in another module (Q-12). Exercises: `Display` for a money type, an
`Add` for vectors, a newtype that stops metres being added to seconds.
Stage 0 runs all of it except `@derive(Into, From)`.

**15. Generics** ([written](tutorial/15-generics.md)). Generic functions and types with bounds, `where`
clauses, defaulted type parameters (D-060), bounds checked at the
definition so that no error appears inside a library at a use,
monomorphisation and its size cost, `dyn Trait` behind `Box` or `ref`,
the rules for object safety, choosing between an enum and a trait
object, and const generics with `Array[T, N]` and expressions as
arguments. Exercises: a generic `largest`, a `Stack[T]`, a ring buffer
over `Array[T, N]`. Stage 0 runs all of it.

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
summary, admission; D-066). Challenge III: split the word counter into a
library and an application in one workspace, with documented `pub` items.
Stage 0 runs all of it except the registry commands (`sisu add`,
`publish`), whose sections wait.

### Part IV. The world outside the program

**18. Capabilities** ([written](tutorial/18-capabilities.md)). `System` and its fields, passing capabilities down
instead of reaching for globals, attenuation (`restrict_to`,
`read_only`, `only_hosts`), capabilities stored in struct fields, lent
capabilities (`fs.lend()`, `Lent[T]`) and revocable ones, the short list
of what is ambient (`print`, `log`, `task.scope`), interceptors
(`Wrap[T]`), what a package with no capability types can do (D-066), and
`sisu audit --caps`. Exercises: narrow a program's file system, make a
function testable by taking a `Clock`. Stage 0 runs all of it.

**19. Testing in depth.** Tests beside the code and in `tests/`,
`suite` and `@tag`, parametrised tests, what a failing `assert` prints,
`expect`, fixtures as values and `@fixture`, each test's own fakes and the
clock that advances only when every task waits, property tests (`check`,
`arbitrary[T]`, shrinking, the failure database), `exhaustive[T]`,
snapshots and transcripts with `--update`, doc examples, replay tokens,
`@refines`, fakes of your own traits (`Fake[T]`), benchmarks, and the
JSON output and cache. Exercises: turn three example-based tests into
one property, write a transcript test for the word counter. Stage 0 runs
all of it except `@fixture`, `Expectation`, `--fuzz`, `--cover` and
`--mutate`.

**20. Concurrency.** `task.scope` and `spawn`, `join` and how error sets
flatten, the scope's result, tasks that borrow from the frame, `Send`
and `Sync` in the messages that mention them, `Mutex`, `RwLock`,
`Atomic`, `Once` and `Condvar`, bounded channels and closing them,
`select` with timers and a cancellation arm, cancellation, `task.shield`,
deadlines, `task.race`, panics in tasks and supervision, a mutex's
invariant re-checked when a panic drops its guard (D-077), `log` and the
trace context, and testing all of it under the seeded scheduler
(`--explore`, `--scheduler pct`). Exercises: a fan-out fetch over a fake
network, a worker pool, a race with a timeout. Stage 0 runs all of it.

**21. A tour of the standard library.** One section per module a program
reaches for first: `std.fs` and `std.io`, `std.time` (with `Ticker` and
`Timeline`), `std.rand`, `std.env` and `std.process`, `std.json`,
`std.regex`, `std.collections` (`SortedDict`, `Deque`, `Heap`,
`BitSet`), `std.hash`, `std.encoding`, `std.sort` and `std.search`, and
`std.log`, each with one example and a link to its generated page.
Exercises: one small task per module. Stage 0 runs all of it.

**22. Project: a small web service.** A JSON service on `std.x.http`:
attenuated capabilities in `main`, a task per connection, logging,
`std.json` with derived encoding, tests over `http.FakeNet`, the
simulation profile with faults (`sisu test --sim`), a failure kept as a
replay token, `sisu run --record` and `sisu replay`, changing a handler
while the server runs (`sisu run --watch`, epochs, `live.boundary()`),
and a native build (`sisu build --native`). Challenge IV: rate limiting
and a cache, proved under the simulator. Stage 0 runs all of it except
TLS.

### Part V. Further

**23. Contracts.** `requires`, `ensures` with `result` and `old`,
`invariant` on structs and the rule that only the type's own methods
assign its fields, refinement types (`type Port = int where ...`) and
`try_from`, the check levels (`--contracts`), contracts in the
documentation and as property tests, contracts on trait methods, and
what waits for the verifier (`decreases`, ghost code). Exercises: a
bank account with an invariant, a binary search with its contract.
Stage 0 runs all of it.

**24. Numbers in depth.** The sized integers and `uint`, the operator
families for wrapping, saturating and checked arithmetic, conversions
that can lose information, strict floating point (one rounding per
operation, `mul_add`, pairwise `sum`, `@float(fast)`; D-074), NaN and
ordering, and the exact types `BigInt`, `Ratio` and `Decimal` with their
suffix-free literals and named rounding (D-076). Exercises: a checksum
over `u32`, a mortgage table in `Decimal`. Stage 0 runs all of it
except `@float(fast)` and `total_order`.

**25. Compile-time code.** `comptime` expressions, `const`, `comptime
def` and `comptime assert`, fuel and `@comptime_budget`, reflection
(`reflect.fields_of`, `if comptime` that narrows a type, `comptime for`,
`value.[f]`), writing a derive with `return impl Trait for T:`,
attributes read as data, `embed`, `target` and `@cfg`, features,
`@diagnostic_name`, `sisu eval` for configuration, and the checked string
literals of [08](../proposal/08-metaprogramming.md). Exercises: a lookup
table built at compile time, a derive that prints a struct's fields.
Stage 0 runs all of it except `@diagnostic_name`, `sisu eval` and the
checked string literals.

**26. Purity and data parallelism.** What `pure` promises and the rules
behind it (insertion-ordered `dict`, strict floats, no operation that
observes the schedule), `pure` function types in higher-order code,
`std.par` (`par_iter`, `par_iter_mut`, `par.join`, reducers, `grain`),
serial elision, the lowest-index failure of `pure` scopes, and
`SoaList[T]`. Exercises: a parallel histogram that gives the same bits on
one worker and on eight. Stage 0 spells the parallel loop `par.iter(xs)`
and the parallel map `par.map`, runs the chunks on one task, and lacks
`par_iter_mut` and `SoaList`; the chapter teaches the proposal's
spelling once it runs.

**27. Systems programming.** `unsafe` blocks and expressions, raw
pointers, C interop (header import, `extern "C"`, `std.ffi`,
`@repr(c)`, `@export`, `@blocking`), `[trust]` and `sisu audit`,
allocators (`list.with_allocator`, `Region`) and `try_push`, the
resource attributes `@no_alloc`, `@no_block` and `@stack`, the runtime
profiles (`full`, `single`, `freestanding`), targets and cross
compilation, `panic = abort`, and binary size. Exercises: a safe wrapper
over a C function, an allocation-free audio callback. Waits for foreign
functions, allocators and the resource attributes.

**28. Code that lasts.** Editions and the compatibility promise, `sisu
fix --edition`, deprecation with `@fix_inline` and `sisu fix --deps`,
`@preview` features (D-072), the false-friend table, the API diff and
behavioural semver of `sisu publish`, `sisu review` and weakened
specifications, `sisu why`, and planning with holes (`sisu check
--holes`). Challenge V: migrate a small library across a breaking change
without breaking its users. Waits for `sisu fix`, `why`, `review` and
the registry.

## The guides

Each guide does one task for a reader who knows the basics, in steps,
with checked examples, and links to the tutorial and the reference
instead of teaching.

| | Guide | What it does, and what it covers |
|---|---|---|
| G1 | Set up your editor | Connects an editor to `sisu lsp`: diagnostics, hover with error sets and capabilities, inlay hints, fixes, explain-borrow, worksheet values inline. |
| G2 | Coming from Python | Ports a Python script and walks through every false friend (D-062, [R19](#the-reference)) and the idioms that replace `class`, `None`, exceptions and truthiness. |
| G3 | Coming from Rust | Maps lifetimes to views and `from`, `Rc` to `Shared`, `Result` to `T ! E`, macros to `comptime`, `async` to tasks. |
| G4 | Coming from Go | Maps goroutines to scopes, `context` to cancellation, interfaces to traits, `if err != nil` to `?`. |
| G5 | Read and write files | `Fs` operations, missing files as `IoError`, walking a directory, testing with `Fs.in_memory`. |
| G6 | Parse command-line arguments | `std.x.cli` derived from a struct, help text, errors, a transcript test. |
| G7 | Read and write JSON | `@derive(Encode, Decode)`, `json.encode`, `json.decode` validating refinements at the boundary, `json.parse`. |
| G8 | Match text with regular expressions | `std.regex`, groups, the linear-time guarantee and what it rules out. |
| G9 | Test code that uses the clock | `Clock.frozen_at`, `advance`, the test clock, `Ticker` under test. |
| G10 | Test code that uses the network | `Net.fake`, `http.FakeNet().respond_to`, request counts. |
| G11 | Write a property test | `check`, `arbitrary[T]`, `@derive(Arbitrary)`, reading a shrunk counterexample, the failure database. |
| G12 | Test a command-line tool | Snapshots and transcripts, `--update` and reviewing the diff. |
| G13 | Find and fix a borrow error | Reading E04xx diagnostics, `sisu explain-borrow`, `--defer-borrow` and the counterexample. |
| G14 | Find a concurrency bug | `--explore`, `--schedule-seed`, PCT and the exhaustive scheduler, `sisu test --sim` with faults. |
| G15 | Reproduce a failure | Replay tokens, `sisu run --record`, `sisu replay`. |
| G16 | Change a running program | `sisu run --watch`, epochs, `live.boundary()`, what restarts the program. |
| G17 | Make a loop parallel | `std.par`, reducers, `grain`, checking that the result is unchanged. |
| G18 | Use a region for per-request memory | `Region.bump`, `list.with_allocator`, why the list cannot escape (waits for allocators). |
| G19 | Call a C library | A trusted wrapper package with a capability-taking safe API (waits for foreign functions). |
| G20 | Build a native executable | `sisu build --native`, `--release`, targets and cross compilation. |
| G21 | Write a derive | A trait's `comptime def derive` over `reflect.fields_of`. |
| G22 | Generate configuration | A module of `const` items rendered by `sisu eval` (waits for `sisu eval`). |
| G23 | Publish a package | Documentation coverage, the API diff, semver, provenance (waits for the registry). |
| G24 | Check what a dependency can do | `sisu audit`, the capability summary, admission and its authority delta. |
| G25 | Move to a new edition | `sisu fix --edition` and `sisu fix --deps` (waits for `sisu fix`). |
| G26 | Plan with typed holes | `todo`, blocked tests, `sisu check --holes`. |
| G27 | Benchmark and profile | `bench` blocks, `black_box`, `sisu bench`, `sisu run --profile`. |
| G28 | Log and trace | `std.log`, levels, the trace context of scopes, `trace.wrap` over a capability. |
| G29 | Write a script with dependencies | A shebang file with inline metadata. |
| G30 | Embed a file at compile time | `embed` and what it records for reproducible builds. |
| G31 | Do exact arithmetic with money | `Decimal`, rounding modes, `Ratio` from division. |
| G32 | Design a library's API | The std rules: capabilities in, values out; specific error sets; no hidden allocation; `Debug` everywhere; iterators over views; Python's names. |

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
| R2 | Types | Every builtin type, numeric conversions, text types, collections and literal typing (`FromLiteral`), tuples, unit and `Never`, `T?` and `T ! E`, error-set subtyping, function types and `pure`, references, views, the library pointer types, `dyn`, refinements, aliases, the auto traits, what the type system leaves out. |
| R3 | Declarations | `let`, `var`, `const`, `def` and its parameters, `pure def`, `comptime def`, `struct`, `view struct`, `enum`, `error`, `trait`, `impl`, `type`, `test`, `suite`, `bench`, `import`, `extern`, visibility. |
| R4 | Expressions and operators | The precedence table and each operator, calls and keyword arguments, indexing, slicing and `^`, comprehensions, closures and `it`, `if` and `match` expressions, `else` fallbacks, `?`, `?.`, `??`, interpolation and format specs, `comptime e`, `unsafe e`. |
| R5 | Statements and control flow | Assignment and places, `if`, chained conditions, `if let`, `while let`, `let ... else`, `while`, the three `for` modes, `break`, `continue`, `return`, `match`, `select`, `with`, `defer`, `assert`, `debug_assert`, `expect`, `pass`, `unsafe` blocks. |
| R6 | Patterns ([written](reference/patterns.md)) | Every pattern form, binding by case, exhaustiveness, guards, alternatives that bind. |
| R7 | Ownership and borrowing ([written](reference/ownership.md)) | Moves, copies, drops and their order, conventions, references, elision and `from`, views, captures, iteration modes, must-consume, guaranteed drop, the library pointer types, allocators. |
| R8 | Errors and panics | Error sets, propagation, traces, context, panics, unwinding, abort, exit statuses. |
| R9 | Concurrency | Tasks, scopes, handles, failures and their order, cancellation, shields, deadlines, races, channels, `select`, the sync types, `Send` and `Sync`, atomics and orderings, `std.par`, the scheduler and its yield points. |
| R10 | Capabilities | `System`, each capability's operations, attenuations, lending, revocation, interceptors, fakes, what is ambient, how it is enforced. |
| R11 | Contracts | `requires`, `ensures`, `invariant`, refinements, check levels, violations. |
| R12 | Compile-time evaluation and reflection | The rules of `comptime`, fuel and budgets, the `reflect` API, `target`, `embed`, the derive protocol, `@cfg` and features. |
| R13 | Modules, packages and the manifest | The module tree, imports and visibility, the prelude's fixed list, every key of `sisu.toml`, the lockfile, workspaces, scripts and inline metadata, editions. |
| R14 | Attributes | Every attribute, with where it may stand and what it does. |
| R15 | Testing | `test` and its forms, outcomes, selection, a test's capabilities, the property-testing API, snapshots, the transcript format, replay tokens, simulation, benchmarks, output and caching. |
| R16 | The `sisu` command ([written](reference/sisu-command.md)) | Every subcommand and option, each with an example and its exit statuses. |
| R17 | Diagnostics ([written](reference/diagnostics.md)) | Every code by range (E00xx lexer to D0xxx driver), with its catalog identifier, message, a checked wrong program and its fix. Generated: the scripts in [tools/diagnostics/](tools/diagnostics/) (`diag_entries.py`, `mk_diag_examples.py`, `mk_diag_page.py`, `expand.py`, the templates in `tmpl/`; `bless_sessions.py` blesses one directory's sessions) rebuild it after a catalog change. |
| R18 | The standard library | Generated by `sisu doc --std`; a page per module. |
| R19 | False friends ([written](reference/false-friends.md)) | Each spelling shared with Python or Rust whose meaning Sisu rejects, with the check that catches it (D-062). |
| R20 | Grammar | The grammar, in the notation of the specification. |
| R21 | Glossary | Each term the manual uses, with the chapter that introduces it. |
| R22 | Targets and runtime profiles | The target tiers, `full`, `single`, `freestanding` and `sim`, the ABI, binary size. |

## The explanations

Each explanation answers one "why" for a user, not a designer: it
states the problem, the choice, what it costs, and the languages that
chose otherwise, and links to the decision log for the evidence.

| | Explanation | About |
|---|---|---|
| X1 | The design in one page | The priority order, the principles, and what Sisu leaves out on purpose ([00](../proposal/00-vision-and-principles.md), [14](../proposal/14-non-goals.md)). |
| X2 | Errors without exceptions, absence without null | Error sets, panics as bugs, `T?`. |
| X3 | Ownership without lifetimes | Second-class references, views, what is given up, compared with Rust, Swift and Python. |
| X4 | Capabilities and the supply chain | No ambient authority, testability, portability, the cost of one more parameter. |
| X5 | Concurrency without colours | No `async`, scopes as supervisors, cancellation without tokens. |
| X6 | Determinism | `pure`, insertion-ordered `dict`, strict floats, and what replay, simulation and `comptime` get from them. |
| X7 | Compile-time code instead of macros | What `comptime` and reflection cover and what they leave out. |
| X8 | Loud divergence | Why a familiar spelling means the same as in Python or Rust or fails to compile. |
| X9 | How the compiler works | Stages 0 to 3, the interpreter, SIR, the native backend, the queries. |
| X10 | Diagnostics as a product | The catalog, identifiers, fixes with consequences, the wrong-program corpus. |
| X11 | How Sisu evolves | Editions, migrations that travel with libraries, governance. |
| X12 | Testing without mocks | Fakes, the test clock, simulation, replay. |
| X13 | Working with unfinished code | Typed holes, blocked tests, the fill engine to come. |
| X14 | Costs you can see | No hidden allocation, monomorphisation, compile speed. |

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

Second person, present tense, short sentences, plain words. A tutorial
chapter is 2,500 to 6,000 words; a section is 150 to 600 words and
teaches one concept; a guide is 400 to 1,500 words; an explanation is
1,500 to 3,000 words; a reference page is as long as completeness needs,
with a heading for each thing a reader looks up. Examples are at most 40
lines, most under 20, and no line in a `.sisu` file is longer than 80
columns; 72 keeps a page readable on a phone.

### One concept per section, one runnable example per concept

A section teaches one concept and shows it in at least one example that
runs. An example is a whole file, shown whole, with what Sisu prints
when it runs, fails to compile or fails a test. A tutorial section ends
with a prompt when one helps: **Predict.** before a run whose result is
worth guessing, **Try it.** for a change the reader makes, and **From
Python.** for a habit that misleads. Explanation stays short in the
tutorial and links to the explanations.

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
with the reason. A package is a directory `NAME.pkg/` with its
`sisu.toml` and `src/`, and the same files beside it, `NAME.args` for
`sisu run -- ARGS` and `NAME.build` for `sisu build` among them. Every example is therefore a conformance case too, and
one that runs is also run under the ownership monitor, recorded and
replayed.

The manual's checker reads one more form, the session: `NAME.console`,
which is what a terminal shows. A line starting `$ sisu ` is a command,
and the lines after it are what the command printed, standard output and
standard error together, as the reader sees them. Sessions show the
human form of diagnostics, `sisu fmt --check`, `sisu test` output and
anything else a reader types; they run in a fresh directory with the
example's files and an empty environment. A session under `solutions/`
runs against the exercise in its start state, which is how a chapter
shows the first run of an exercise, and a session beside a package,
`NAME.console` beside `NAME.pkg/`, runs in a copy of the package, where
a reader types the commands.

A page shows a file with a marker on the line before the fenced block:
`<!-- file: examples/01-hello/hello.sisu -->` for a source file,
`<!-- output: examples/01-hello/hello.out -->` for an expectation or a
session (for `.ws` and `.build`, the standard-output part), and `<!-- unchecked:
REASON -->` for the rare block that cannot be checked, such as an
installation command. A fenced block without a marker fails the check,
so no code reaches a reader unchecked by accident.

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
fixing.

Hints live in the chapter, under the exercise, in folded `<details>`
blocks, two or three per exercise, each narrower than the last; the last
hint names the idea, never the code. The chapter links the solution and
asks the learner to open it after the tests pass, or once the hints have
not helped.

A challenge is one file with a `suite` per part. Part 1 uses the part's
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

Every push checks the whole manual. Two tools do it, and CI runs both
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
formatted and short enough; and that every example, session and
exercise is shown or named by some page. `python3 tools/check_docs.py`
checks the links, as for every Markdown file.

When the compiler changes what it prints, the job fails and the change
is reviewed, not ignored: `sisu-conformance --bless DIR` rewrites the
runner's expectations and `python3 manual/tools/check.py --bless`
rewrites the sessions, and the diff goes into the same commit as the
compiler change, where a reviewer reads it.

## How the manual is rendered

For now the manual is plain Markdown that GitHub renders: relative
links, folded `<details>` blocks, and code fences labelled `sisu`,
`console` and `text`. Nothing depends on a generator, and every page is
readable in an editor.

Later a site generator builds the manual into a website with search, a
print view and a page per chapter. mdBook is the first candidate,
because its layout (one file per page, a `SUMMARY.md` table of contents)
matches this directory and its search and print support are built in;
the markers become run buttons for the playground. The standard
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
| 5 | Chapter 9 with challenge I (3, done); G2 Coming from Python (1); R19 false friends (1, done) | 2 |
| 6 | Chapters 10 (3, done), 11 (4, done), 12 (3, done), 13 with challenge II (3, done); R7 ownership (3, done); G13 borrow errors (1); X3 (1) | 5 |
| 7 | Reader test of Parts I and II: five readers who know Python, their stuck points, the rewrites | 4 |
| 8 | Chapters 14 (3, done), 15 (3, done), 16 (3, done), 17 with challenge III (2.5, done); R2 (3), R3 (2), R6 (1, done), R13 (2.5) | 20 |
| 9 | Chapters 18 (3, done), 19 (3), 20 (4), 21 (3), 22 with challenge IV (4); R9 (2.5), R10 (2), R15 (2); G5 to G16 (9) | 32.5 |
| 10 | R4 (2), R5 (1.5), R8 (1.5), R14 (1.5), R20 (2), R21 (1), R22 (1); X1, X2, X4, X5, X8, X10, X12 (7) | 17.5 |
| 11 | Chapters 23 (2), 24 (2), 25 (3), 26 (2); R11 (1), R12 (2); G17, G20, G21, G26, G27, G28, G29, G30, G31, G32 (7.5); X6, X7, X9, X13, X14 (5) | 26.5 |
| 12 | When their features exist: chapters 27 (3) and 28 with challenge V (2); G18, G19, G22, G23, G24, G25 (4.5); X11 (1); the site (3) | 13.5 |
| | Total | about 159 |

The setup of R18, the generated library reference, belongs to step 12's
site; until then `sisu doc --std` serves it locally. R17 grows with every
chapter: each diagnostic a page shows gets its entry.

## What the plan found in stage 0

Writing the first chapter, and trying the habits chapter 4 will meet,
against stage 0 turned up five things for the compiler's maintainers,
recorded here so that later chapters do not rediscover them:

- `print("Hei,", "maailma!")`, the commonest Python habit, gets E0305
  (`type.argument-count`) with no fix; the fix that teaches is
  interpolation, `print("Hei, {x}")`.
- An assignment to an undeclared name, `greeting = "..."`, gets E0200 on
  each use and no fix; the fix is `let`. A Python loop `else` gets E0103
  (`parse.expected-expression`), where the fix is the `if let` on the
  search's result that [02](../proposal/02-syntax.md) gives.
- `{x=:.1f}` ignores the precision for a float and prints every digit,
  though [02](../proposal/02-syntax.md) says a format spec may follow the
  `=`; `{n=:>5}` works.
- `sisu explain CODE` prints the catalog line only; the long form with
  an example that [10](../proposal/10-tooling.md) describes is Q-66.
- The conformance runner cannot pin the human form of a compile error
  that stops a run (its record-and-replay check needs a log), and a
  `.repl` capture from piped input does not look like a terminal
  session; the manual's sessions cover the first, and the manual shows
  worksheets rather than REPL sessions until the `sisu>` transcripts of
  [17](../proposal/17-testing.md) exist.

Writing the rest of Part I turned up more, by chapter:

- Chapter 2. `'Aino'` gets E0016 (`lex.bad-char-literal`) with no fix;
  the fix is double quotes. A keyword used as a name, `let type =
  "csv"`, gets E0105 and E0103, neither of which says that the word is a
  keyword. `let Total = 10` gets E0327, E0329 and E0200, none of which
  says that a binding's name starts with a lowercase letter. `let small:
  u8 = large` gets E0300 with no fix, where `large.to_u8()` would teach.
  The E0300 for `return total / count` in a function that returns an
  `f64` has the note "`=` cannot mix `f64` and `int`", naming an `=` that
  the line does not have. A bug: `+%=` and `+|=` on a `u8` work at the
  width of `int`, so `level +%= 10` on a `u8` holding 250 leaves 260 in
  it, where `level = level +% 10` gives 4.
- Chapter 3. A keyword-only argument passed by position,
  `url("example.org", 443)`, gets E0305 counting only the positional
  parameters, with no word that `port` is keyword-only. A positional-only
  parameter passed by name gets E0306 ("has no parameter named
  `value`") and E0308, whose fix adds `value=todo("value")`, the same
  mistake again. A misspelt keyword, `prot=443`, gets E0306 without the
  similar-name fix that E0200 has. A variadic parameter (`*prices: int`)
  stops stage 1 with "beyond the slice: beyond the type checker's slice:
  variadic parameters", a message with no code.
- Chapter 4. `switch day:` gets E0117 ("unexpected `day` after the end
  of the statement") and `i++` gets E0103, neither pointing at `match`
  or `i += 1`. An assignment to a parameter, `n //= 2`, gets E0312,
  whose message says to declare it with `var` and whose fix makes the
  parameter `mut`, which changes every call; the fix a beginner wants,
  a local `var value = n`, is not offered.
- Chapter 5. `max(scores)` gets E0308 ("missing the argument `b` of
  `max`") with a fix that adds a hole: stage 0's `max` and `min` compare
  two values, and Python's form over a collection, which the prelude
  list of [02](../proposal/02-syntax.md) suggests, is missing. `^` in a
  slice bound, `xs[:^1]`, gets E0383 in both front ends, though 02
  gives `a[i:^1]`. `[p for p in zip([1, 2], ["a", "b"])]` gets E0406
  ("does not live long enough") for the two temporary lists, whose
  borrow ends inside the statement. `var in_order = readings`, with
  `readings` a borrowed `list[int]` parameter, followed by
  `in_order.sort()`, gets E0341 ("not declared with `var`"), though it
  is; the binding holds a borrow, and `.copy()` is not offered. A test
  name is interpolated, so `test "len({3, 1})":` gets E0103, and an
  escaped quote in a test name gets E0005.
- Chapter 6. `for letter in word` gets E0325 (`type.not-iterable`) with
  no fix, where E0345 for `word[0]` lists `.chars()`, `.bytes()` and
  `.byte_slice(a, b)`. Python's `letter in "aeiou"` with `letter` a
  `char` gets E0369 (`type.comparison-types`) with no fix; a list of
  `char` values, or `"aeiou".contains("{letter}")`, is what works.
- Chapter 7. A `str` field gets E0411, whose fix makes the struct a
  `view struct`, where a beginner wants `String`. A field changed
  through a plain `self` gets E0341 ("not declared with `var`") with a
  note about `ref mut`, and no fix that adds `mut self`. A missing
  `Eq`, `Ord` or `Hash` on a type of the same file gets E0347 or E0368
  with no `@derive` fix, and `sorted([...])` names the type `ref
  Version`. `sorted(hand)` on a borrowed list of items that are not
  `Copy` gets E0300 ("expected `Iterable[Card]`, found `ref
  list[Card]`") without a word about `Copy`. E0314 names one missing
  case at a time, though [03](../proposal/03-types-and-generics.md)
  says it lists them. A variant named like a prelude trait (`Add`,
  `Mul`, `Neg`, `Num`) called where no type is expected gets E0309
  ("a value of type `Add` cannot be called"); `Expr.Add(...)` works.
  `struct point:` compiles without a word, though 02 enforces
  `CamelCase`; `case _` on an enum of the same package does not warn
  (D-043); a `pub` type has a `Debug` form without asking, though
  [10](../proposal/10-tooling.md) makes it opt-in; and a `todo(...)`
  statement in a method that returns nothing is a hole "of type `_`".
- Chapter 8. `{u.team?.name}` moves `name` out of `u.team` when it is
  not `Copy`, so the next use gets E0400 and E0401, where `{u.name}`
  only reads. An error that `main` returns is printed in its `Debug`
  form, so a `.context` error prints as `Context(message="...",
  cause="NotANumber(text=\"kolme\")")` rather than as its `Display`
  form, `reading line 2: NotANumber(text="kolme")`. `?` in a `main`
  that returns nothing gets E0320 with no fix that adds `-> ! E`. An
  error set named like one of the prelude's (`TimeError`) shadows it
  without a word, and diagnostics then name it `main.TimeError`. An
  assignment on the line of a `case`, `case None: label = "x"`, gets
  E0117; it needs a block. `.or_else` gets E0304, as planned.
- Chapter 9. Python's `open(path)` gets E0200 with no word about
  `Fs`. An argument from `sys.env.args()` passed where a `Path` is
  expected gets E0300 ("expected `Path`, found `ref str`") with no fix
  offering `Path.new(arg)`. A transcript line for a run that fails
  holds the error's trace (`at counter.sisu:34:19`), so a transcript
  of a failing run breaks whenever the program's lines move; Challenge
  I's transcript keeps to runs that succeed.
- Chapter 10. A borrowed parameter passed to an `own` parameter,
  `total(items)` inside a function that borrows `items`, gets E0300
  ("expected `list[Item]`, found `ref list[Item]`") with no word about
  `own`, a move or `.copy()`. E0338 for `let first = queue[0]`, with
  `queue` a local list, calls the list "a borrowed value", and its fix
  offers `.copy()` but not `remove` or `pop`. E0400's note names two
  fixes, copying and moving later, without their costs, and never the
  third, a borrowed parameter in place of an `own` one that only reads.
  `with Booking.start("sauna"):` without `as NAME`, Python's `with
  lock:`, gets E0100 ("expected `as`, found `:`"). Python's
  `", ".join(names)` gets E0304 ("`str` has no method `join`") with a
  note that `join` is a method of `Iterator`, where `names.join(", ")`
  is the fix. A `dict[String, int]` looked up with a `str`,
  `recipes.get(dish)`, gets E0300 whose fix calls `.to_lowercase()`. A
  test cannot see a drop (it captures no output, and a `Drop` reaches
  only its own value until chapter 13's `Shared` and `Cell`), so the
  drop-order exercise predicts the order of `defer`s, which the chapter
  shows to be the same order. The formatter's width is 100 columns and
  the manual's limit is 80, so a line of 81 to 100 columns that the
  formatter keeps whole is too long for the checker; a trailing comma in
  the brackets makes the formatter break it.
- Chapter 11. E0407 for a `str` result that borrows a `String` made in
  the function names `.copy()` in its note; following it gets the same
  E0407, since the copy is local too, and the fix, a `String` result,
  is not named. A change inside a loop over the same list or dict gets
  E0405 whose note says to move the change "after line N", N being the
  last line of the loop's body, inside the loop, and whose label "the
  borrow is still used here" points at the loop's head or at the
  changing line itself. A borrowed parameter that the function changes
  gets E0341 ("it is not declared with `var`") with the right fix, `mut`
  on the parameter, while each call that already says `mut` gets E0340,
  whose fix removes it: two fixes that contradict, of which a beginner
  has to pick the one in the signature. A list of references,
  `var names: list[ref String] = []` with `names.push(ref aino)`,
  compiles and runs, where
  [04](../proposal/04-memory-and-ownership.md#the-decision) (rule 3)
  says that a reference cannot be stored in a collection; stage 0
  treats the list as a view, as it treats a list of `str`, so the
  chapter says that a list holding a reference cannot outlive what it
  borrows.
- Chapter 12. A function that reads a file through `fs: Fs` and
  `path: Path` and returns a `str` view of the text it read gets E0413
  (`borrow.from-required`), whose fix names `fs` and `path` in a `from`
  clause; the result borrows from neither, and the fix leads to E0407.
  E0407's note offers `.copy()` for a returned list of views too, which
  copies views that still borrow the local. In the address book of
  exercise 12.3, accepting E0411's fix (`view struct`) twice leads to
  E0410, where the fix that works, `String` fields, is offered by none
  of the three. `print(part)` with `part` a `Slice[int]` gets E0347
  ("does not implement `Display`"), though a list prints and
  `{part:?}` works.
- Chapter 13. `Shared.same(a, b)`, which
  [02](../proposal/02-syntax.md#expressions-and-statements) gives for
  the identity of shared values, gets E0351 ("`Shared[_]` has no
  associated function `same`"), so the chapter calls it designed. A
  field assigned through a `Shared` gets E0341 ("not declared with
  `var`") with a note about `ref mut`, the same when the binding is a
  `var`; neither says that a `Shared` value changes only through a
  `Cell` or a `Mutex` inside it. A `Key[T]` cannot be an element of a
  `set` or a key of a `dict`: E0398 (`type.structural-only`) says it has
  a `Hash` of its own, though the prelude derives `Hash` and `Eq` for
  it, so Challenge II keys its set by `key.index`. A `for` loop over a
  `mut` parameter, `for i, k in enumerate(path)`, with a read of `path`
  in its body gets E0403 (the checker's known over-rejection 15); the
  challenge's hint loops over `range(len(path))`. A `Weak` is not
  `Copy`, so a `Cell[list[Weak[T]]]` is read by `replace` and `set`
  back. `len(b)` with `b` a `Box[list[int]]` gets E0347 ("does not
  implement `Len`"), though a box reaches the methods of its value.
  `for i, d in enumerate(own drafts)` gets E0100; consuming a list with
  its positions takes a counter.
- Chapter 14. `sisu run` and `sisu check` (stage 1) accept an impl
  method whose signature differs from the trait's, `def price(self) ->
  f64` for a trait's `-> int`, and run it; `sisu test` (the Rust front
  end) reports E0234. A trait's associated constant, which
  [03](../proposal/03-types-and-generics.md#traits) gives, gets E0100
  without a value, and with one it cannot be read as `Self.SIDES` or
  `Square.SIDES` (E0351); a type's own `const` in its body works, so
  the chapter calls trait constants designed. `sorted(board)`, with
  `board` a list of a type that is not `Copy` and whose `Ord` is
  written by hand, stops with "internal error in the interpreter:
  Field(1) does not apply to Ref(...)" in both front ends, where
  `board.sort()` works; the chapter sorts in place. `impl Mul[Vector]
  for f64` compiles (in stage 1 without the `type Output = Vector` that
  the Rust front end asks for with E0234), but `2.0 * v` gets E0310:
  an operator with a number on the left never reaches an impl. E0310
  for `a - b` on a struct without `Sub` does not name the trait to
  implement. E0231 for a trait used as a parameter's type offers `dyn
  Priced`, which gets E0395 there; the generic parameter of chapter 15
  is not offered. E0243's list of the traits `@derive` takes leaves out
  `Default`, which it takes. `sorted` of a `list[f64]` gets E0347 whose
  fix adds `impl Ord for f64`, which the root package may write (and
  which panics on NaN) but a library may not (E0236); the fix is
  offered in a library too.
- Chapter 15. A field with a default in a generic struct, `items:
  list[T] = []`, built as `Shelf[int]()`, stops `sisu run` with E0501
  ("invalid SIR in `main`: the type of _2 names a type parameter the
  function does not have"), and the Rust front end with E0901; exercise
  15.4 builds its empty stack in an associated function. The expected
  type does not reach a generic struct's construction: `let r:
  Reading[f64, String] = Reading(21.5, "°C")` gets E0300 ("found
  `Reading[f64, str]`"). `Allocator`, `Global` and `list[T, A]` do not
  exist (E0201, E0208), so D-060's parameter is designed only; the
  chapter shows a default on a type of its own. E0241 is reported where
  `dyn Trait` is written in a signature, but not in a local's type,
  `let shapes: list[Box[dyn Shape]]`, and a call through that `dyn` of
  a method returning `Self` stops with "internal error in the
  interpreter: `Shape.area` on a `dyn` holding Struct(...)". A borrow
  of a temporary, `show(ref Coffee(1))`, gets E0313 ("this expression
  cannot be assigned to"), and a value passed to a `ref dyn Priced`
  parameter without `ref` gets E0300 with no fix that adds it. E0304
  for a method of an unbounded type parameter names the trait in a
  note but offers no fix, where E0347 offers "bound `T` by ...". E0300
  for a `str` returned where a `String` is expected offers
  `.to_lowercase()`, which changes the text, as chapter 10 found for a
  lookup. `sisu build --explain-size` waits, as 03 plans.
- Chapter 16. A generator expression gets E0900, as planned, followed
  by E0304 ("`list[int]` has no method `collect`"), which types it as a
  list. `collect[set[int]]()` compiles and gives a list: the type
  argument is ignored. Python's `xs.map(f)` on a list gets E0304 with
  the note "`map` is a method of `Iterator`" and no fix that inserts
  `.iter()` (for `filter`, the similar-name fix offers `iter` in its
  place), and each `it` in the arguments then gets E0342. `map(f)` over
  the `iter()` of a `list[int]`, with `def f(n: int)`, gets E0300
  ("expected `(ref int) -> _`, found `(int) -> int`"), since the items
  are references; `map(x -> f(x))` works. Python's `sorted(xs,
  key=len)` gets E0306 and E0350 with no word about `sort_by` or
  `max_by`. A closure kept in a `let` with untyped parameters, `(a, b)
  -> a + b`, gets E0310 ("`+` cannot be applied to `_` and `_`") where
  E0324's "add a type annotation" would help. E0423's note suggests
  capturing a copy made in a local, which gets E0423 again for a
  `String`. E0335 names the function passed (`noisy_square`) as the
  impure callee, where 03 asks for the operation responsible
  (`print`).
- Chapter 17. A single file cannot import the module beside it (E0220),
  and `sisu test FILE` compiles the file alone even in a package's
  directory; a script with inline dependencies must start with `#!`,
  where an exercise starts with its `##!` header. So an exercise, which
  `tools/check.py` runs as `sisu test NAME.sisu`, cannot be a
  workspace: Challenge III is one file with the library's documented
  `pub` items, and the chapter's workspace example holds its split
  into a library and an application. The variants that the chapter's
  "Try it" prompts make of a package are sessions over a `NAME.files/`
  tree, since check.py requires every `.sisu` file under a `NAME.pkg/`
  to be shown by a page. `from wordcount import words`, where the root
  module re-exports `words` with `pub from`, gets E0204 in both front
  ends, though `import wordcount` and `wordcount.words` work. E0204 for
  an item that `@cfg(feature=...)` left out does not name the feature.
  `from std.collections import *` gets E0100 ("expected a name, found
  `*`") with no word that Sisu has no glob imports. A missing import of
  a type used as `Deque[int]()` gets E0200 for `Deque` and another
  E0200 for `int`, whose fix offers `uint`. A root module that imports
  `clean` from its own module `clean` gets E0202 ("defined more than
  once") with its "first defined here" label at line 1, column 1.
  `sisu doc` prints Markdown with code fences, which a page's block
  cannot hold, so the chapter describes its pages in prose.
- Chapter 18. E0421 for a `Lent[Fs]` kept in a field of type `Fs` has
  the note "put an owned value in it instead, such as a `String` made
  with `.to_string()`", which is written for text; for a lent
  capability the fixes are a parameter of type `Fs` or doing the work
  inside the call. A test that hands a `TestClock` to a function that
  takes `sys: System` gets E0300 ("expected `System`, found
  `TestClock`") with no fix that narrows the parameter to `Clock`,
  which exercise 18.2 asks for. The iterator methods (`count`,
  `collect`, `map`, `sum`, `join`) are not `pure`, so `pure def
  word_count(text: str) -> int` returning
  `text.split_whitespace().count()` gets E0333; the chapter counts with
  a loop. `fs.read_to_string(path)?.lines()`, Rust's habit, gets E0370
  ("`?.` needs an optional value, found `String ! IoError`"), since
  `?.` is one token, with no word that `(x?).lines()` or a `let` is the
  spelling. `sisu run FILE -- ARGS` passes the `--` to the program as
  its first argument, where a package's `sisu run -- ARGS` does not, so
  the chapter writes a file's arguments after its name. `System` has no
  `stdout` and `stderr` fields, which
  [06](../proposal/06-concurrency-and-capabilities.md#the-rule) lists
  among the root capabilities; `print` and `print_err` are ambient
  instead.

Planning the later chapters turned up the parts of the proposal that
they teach and stage 0 does not run yet, each named in its chapter's
paragraph: variadic parameters, star unpacking in `let`, generator
expressions, list literals typed as `Array`, `round_to`, `.or_else`,
`try_map` and `try_collect`, `@derive(Into, From)`, `@fixture` and
`Expectation`, grapheme iteration, `xs.par_iter()` and `par_iter_mut`,
`SoaList`, `@float(fast)`, `@diagnostic_name`, `--fuzz`, `--cover` and
`--mutate`, `sisu check --holes`, `sisu test`'s JSON output, and the
commands `lint`, `add`, `update`, `publish`, `review`, `fix`, `why` and
`eval`, besides the foreign functions, allocators, resource attributes
and registry that chapters 27 and 28 wait for.

Writing the reference turned up more, by page:

- R16, the `sisu` command. `sisu explain` and `sisu new` without their
  argument get D0004 ("`sisu explain` needs a file"), where they need a
  code and a name. `sisu build` and `sisu sir` of a program with errors
  end with D0006, "the program was not run". `sisu` has no `--version`
  and no `version` command, though Q-67 asks each page to state the
  version of `sisu` that checked it; the toolchain's version, 0.0.1,
  appears only inside replay tokens and capability logs. `sisu test
  --std` without a file needs a package (D0064), though the standard
  library's examples need none, and with a file it runs the file's
  tests too. D0064 names the absolute directory, so no session can show
  it. `--stats` followed by a word it does not know takes the word as the
  file (`sisu run --stats bogus hello.sisu` gets D0001 for `bogus`); only
  `--stats=bogus` gets D0094. Python's `enumerate(lines, start=1)` gets
  E0306 ("`enumerate` has no parameter named `start`") with no fix. The
  architecture notes put a package's test cache in `build/test-cache`;
  it is `build/debug/test-cache`.
- R17, diagnostics. `Light.Blue`, for a variant that `Light` does not
  have, gets E0351 ("`Light` has no associated function `Blue`") where
  E0327 ("`Light` has no variant `Blue`") would say what is wrong; E0327
  comes only from a pattern. `sisu explain` prints a message's template
  with the catalog's escapes, `{{` and `}}`, where the reader sees one
  brace. A run-time message is printed without its code (`panic: ...`),
  so a reader has nothing to give `sisu explain`; the R codes are
  visible only in the catalog. The tables of R17's "Every code" are
  written from the catalog and nothing checks them against it.
- R1, lexical structure. Names are not normalised to NFC, so a letter
  written as a base and a combining mark gets E0005 for the mark, and
  the confusable and mixed-script lints of D-048 are missing: a name
  that mixes Latin and Cyrillic letters compiles without a word. E0005
  shows an invisible character as it is, so the message for a no-break
  space reads "`` ` ` `` cannot appear", and a byte-order mark at the
  start of a file is E0005 too. A keyword as a field's name gets E0100
  ("expected a name, found `:`"), and a function declared `def type()`
  cannot be called (E0103); only a parameter gets E0124. `def Main()`
  and `const pi` compile without a word, like `struct point`. A one-line
  `if c: print(x)` gets E0116 (an `if` expression without `else`) where
  E0102 says that a block starts on the next line, as it does for
  `for`. `1.` and `.5`, Python's floats, get E0100 and E0103 with no word
  about the digits a point needs.
- R19, false friends. E0115 (`parse.python-conditional`) comes only
  where a statement starts, as in `let x = a if c else b`; inside a
  call's parentheses, `print("big" if n > 2 else "small")` gets E0100
  ("expected `)`, found `if`"). E0123 (`parse.walrus`) likewise: `if (n
  := len(xs)) > 2:` gets E0100. `int("42")` gets E0200 with the fix
  `uint`, and `str(n)` E0200 with none. Most other habits get a
  diagnostic that does not name the Sisu spelling: `let mut` (E0105),
  `&&`, `||` and `!` (E0103), `x as f64` (E0100), Rust's `// comment`
  (E0117), `.clone()` (E0304, no `.copy()`), `=>` in a `case` (E0100),
  `&xs` (E0103), `0..3` (E0100), `Vec<T>` (E0117), `println!` (E0117),
  `x is None` (E0100), `lambda` (E0117), `f"..."` (E0100), `class`
  (E0107), `try:` (E0117), `append`, `strip` and `upper` (E0304, with no
  similar method named), and `d.get(k, default)` (E0305).
- R7, ownership. An `own` closure that captures a `String` gets E0423,
  whose note says to copy the value into a local and capture that,
  which is E0423 again: stage 0's `own` closures keep `Copy` values only.
  `sisu check` of `unsafe 1 + 1` gives W0009 now that the front end has
  `unsafe`, so the tutorial's "waits for" of chapter 27 is moving.

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
