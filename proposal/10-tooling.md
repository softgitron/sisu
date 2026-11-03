# 10 - Tooling

Priority 4 says tooling is a design constraint. This document states what
ships, what it must achieve, and which language decisions exist to make it
achievable.

## One binary

```
sisu new <name>          create a package
sisu build               compile
sisu run [file.sisu]     build and run; a single file with a shebang runs as a script
sisu test                unit tests, doc tests, property tests, contract checks
sisu check               type-check without code generation (sub-second on most edits)
sisu check --holes       report each typed `todo` hole with its context
sisu fmt                 format; no options
sisu lint                lints beyond the compiler's warnings, with fix-its
sisu doc                 render documentation; `--open` serves it locally
sisu add/remove <pkg>    edit the manifest and lockfile
sisu update              resolve newer versions within ranges
sisu publish             API diff, provenance, upload
sisu audit               trusted packages, unsafe blocks, capability summary, holes, advisories
sisu review base..head   weakened specifications and authority deltas (leaning, D-066)
sisu fix                 apply compiler-suggested migrations (editions, deprecations);
                         `--deps` applies libraries' `@fix_inline` migrations (leaning, D-067)
sisu why <fact> <loc>    explain an inferred fact (leaning, D-065)
sisu eval <file>         render a `pure` module as JSON, TOML or YAML (leaning, D-063)
sisu explain E0123       long-form explanation of a diagnostic
sisu lsp                 language server
sisu repl                interactive evaluator (built on the comptime interpreter)
sisu replay <log>        re-run a recorded run (experiment prototype, Q-49)
sisu bench               benchmarks with statistical reporting
```

Go, Rust (Cargo), Zig, Elixir (mix), Dart, Deno and `uv` are the evidence
that this list, in one tool with one configuration file, is what users
rate highest. The counter-examples (C++, Java, Python before `uv`,
JavaScript before Vite/Bun consolidation, Haskell's Cabal/Stack split) are
the evidence for what happens otherwise.

`sisu run --record` and `sisu test --sim` are prototypes in the experiment of
[18-next-steps.md](18-next-steps.md), like `sisu replay` (Q-49), and `sisu
fill`, which proposes fills for typed holes and judges them, is later tooling
with no row yet. Each is described in its own section below. `sisu eval` is
described with configuration in
[08-metaprogramming.md](08-metaprogramming.md), and
[19-novel-features.md](19-novel-features.md) assesses the candidates behind
the newer rows.

## Compile speed as a requirement

Targets, stated so that the prototype can be measured against them:

| Scenario | Target |
|---|---|
| `sisu check` after editing one function body in a 100 kLOC package | < 300 ms |
| Debug rebuild and relink after the same edit | < 1 s |
| Clean debug build of 100 kLOC | < 10 s on a laptop |
| Clean release build of 100 kLOC | < 60 s |
| Language server response to a keystroke | < 100 ms |

Language decisions made for compile speed, each with its source:

- Explicit signatures: bodies type-check independently and in parallel
  (Go, Carbon's "no whole-program inference" principle).
- No package cycles: packages build in dependency order with full
  parallelism across the graph (Go).
- No macros, bounded comptime: no arbitrary code generation in the
  front end (Rust proc macros, Swift macros as the counter-example).
- Trait-checked generics: a generic body is checked once, not per
  instantiation (C++ templates and Zig comptime as counter-examples).
- Instantiation sharing and a monomorphisation budget (Rust's unshared
  monomorphisation as the counter-example; Go's GC-shape stenciling as the
  model).
- A non-LLVM backend for debug builds. Zig's self-hosted x86 backend and
  Cranelift for Rust show 5-10x faster debug compilation than LLVM. LLVM is
  used for release builds where its optimisations matter.
- A query-based incremental front end from the first commit (rustc added
  it in year 7, Roslyn had it from the start; the difference shows).
- Separate compilation at module granularity, with a stable
  per-module interface hash so that a body edit never invalidates
  dependents.
- Compile-time and time-to-first-run benchmarks in the compiler's CI from
  the first commit. Julia's time-to-first-plot problem defined its public
  image for a decade because it was not measured until it was large
  ([`julia.md`](../research/languages/julia.md)).

Compiler architecture rules, taken from Roc's post-mortem of the compiler
it abandoned ([`roc.md`](../research/languages/roc.md)) and from Hylo's
two rewrites: an error-tolerant parser from day one; no internal panics on
bad input (every compiler error is a diagnostic); no analysis pass that
cannot be re-run incrementally; the backend behind a stable internal IR so
that LLVM can be swapped; and a conformance test suite that runs on every
commit.

Three more rules cannot be retrofitted, so they belong to the first diagnostic
and the first checker: message identifiers with parameters instead of English
built in code, a lossless syntax tree, and a checker that produces types for
broken code (D-065). Diagnostics and Language server below describe them.

The TypeScript team's 2025 port of its compiler from TypeScript to Go for a
10x speedup ([`typescript.md`](../research/languages/typescript.md)) is
noted: Sisu's compiler is written in Sisu (after bootstrapping), with the
expectation that a language designed for predictable performance can
compile itself quickly. Until the bootstrap, the prototype compiler is
written in Rust (closest semantics, mature backend bindings).

## Diagnostics

Every diagnostic has:

- A stable code (`E0381`) and a `sisu explain` page with examples.
- A message identifier with parameters. The text comes from a catalog and
  English is never concatenated in code, so a message can be tested, rendered
  for another tool or translated without parsing prose.
- A one-line summary in plain language naming the thing that is wrong,
  not the compiler phase that noticed it.
- The source span, with labels on each relevant location (Rust's
  multi-span layout).
- Where possible, a suggested fix as a diff the LSP can apply. When a
  diagnostic offers several fixes, each carries a one-line consequence:
  `add_user(mut team, roni.copy())` keeps `roni` usable and gives `team` its
  own copy.
- Nothing after the first error in a dependency chain: if `x` has an
  unknown type, uses of `x` are not reported.

Elm's "Compiler Errors for Humans" (2015) and Rust's diagnostics working
group are the models. The evidence is honest about its shape: controlled
studies of enhanced error messages for novices (Becker et al.'s review,
Denny et al.) are mixed, while the survey evidence is strong (only 9% of
Rust users call compiler messages a big problem after a decade of
investment, against Clojure's and Julia's years at the top of their
complaint lists; [`../research/evidence/surveys.md`](../research/evidence/surveys.md),
[`studies.md`](../research/evidence/studies.md)). Diagnostics are therefore
treated as a satisfaction feature and tested like any other: a corpus of
wrong programs with expected messages, and a review process for new
message text. Novices misread even carefully designed messages, largely
over vocabulary (Marceau, Fisler and Krishnamurthi;
[`end-user-and-live.md`](../research/ideation/end-user-and-live.md)), which
is why a message is an identifier with parameters that can be re-worded or
translated without touching the compiler, and why every fix says what it
does.

Two specific commitments:

- **Borrow and `Send`/`Sync` errors name the fix.** "`xs` is borrowed by
  `first` (line 12) and cannot be modified until that borrow ends (line 15).
  Move the `push` after line 15, or copy the value: `let first = xs[0].copy()`."
  The research on Rust's learning curve is specific that the *explanation*
  of borrow errors, not the rule, is what improved over time.
- **Missing-trait errors list the candidates.** "`Point` does not implement
  `Display`. It has all the methods of `Display`; add `impl Display for
  Point`." (Go's implicit satisfaction gives this convenience; Sisu gives
  it as a fix-it.)

Two more devices explain what the compiler inferred or built rather than what
the user wrote.

`sisu why` explains an inferred fact, in human and JSON form (the language
server serves the JSON). It covers error sets, `Send` and `Sync`, `pure`,
capability reach, impl selection and comptime fuel:

```
$ sisu why send cache.sisu:31
Job is not Send: Job.cache: Shared[Cache] -> Cache.hits: Cell[int], and Cell is not Sync.
fixes: hits: Atomic[int], or cache: Shared[Mutex[Cache]]
```

Tracing runs only in explain mode, by re-running the query, so ordinary
compiles pay nothing. Later, with the recording described under Debugging and
profiling, it can also say why a value had the value it had. Argus's
participants localised 2.2 times as many trait-inference faults, 3.3 times
faster, and the Whyline cut debugging time by nearly a factor of eight;
rustc's "required because" notes and Scala's `-explain` each cover one kind of
fact, and no language has one query over all of them
([`ai-era-and-evolution.md`](../research/ideation/ai-era-and-evolution.md),
A9).

A library that constructs types at compile time (Q-27) can say how they print.
`@diagnostic_name(f)` names a comptime function that the compiler calls when
it prints the type in a diagnostic, and `sisu explain --raw-types` shows the
underlying type. Without the hook a units error shows an exponent vector,
which is much of what makes units libraries in C++ and Rust hard to use.
Lean's delaborators and Rust's `#[diagnostic::on_unimplemented]` are the
precedents.

## Formatter

No options. Four spaces, 100 columns, trailing commas in multi-line
constructs, sorted imports, one blank line between items, canonical
spelling of sugar (`T?` not `Option[T]`; `T ! E` not `Result[T, E]`). The
formatter is part of the compiler's parser so it never disagrees with it,
and it preserves comments exactly. `gofmt`'s and `rustfmt`'s histories
show that the moment an option exists it becomes a team argument.

## Language server

The LSP is the same front end as the compiler (Roslyn, rust-analyzer's
eventual convergence with rustc, Zig's `zls` struggles as the
counter-example: a separate implementation always lags). It provides
hover with inferred error sets and capability requirements, go-to-
definition across packages including into the standard library's source,
rename, fix-its for every suggested diagnostic, inline hints for inferred
types and parameter conventions (`mut`/`own` at call sites), and a
"explain this borrow" command that shows the span of each borrow.

Code is broken for most of the time it is being written, so the server has to
work in broken code. The checker returns a typed tree for every input and
flags ill-typed subterms instead of failing, so hover, completion and
go-to-definition keep working (Luau's first design goal) and the live
evaluation below has something to run. It fits the rule that nothing
downstream of the first error in a chain is reported. The syntax tree is
lossless, with comments and spacing kept and node identities that survive
edits, and it is a library: the formatter uses it, and so do the structural
editing commands below.

Four features follow 0.1, in this order. None is in the experiment, whose
principle 6 admits nothing beyond the bets before the verdict
([18-next-steps.md](18-next-steps.md)).

1. **Always-on evaluation** (N-25). A few hundred milliseconds after typing
   stops, the server reruns every doc example and `test` block that reaches
   the function under the cursor, in the interpreter, with fresh capability
   fakes and a fuel limit, and shows the outcome inline: a result per example,
   the actual values beside a failing `assert`, probes for the example the
   user selects (per-line values, one column per loop iteration) and, for a
   function with contracts, a background `check` with a small budget whose
   shrunk counterexample appears on the `ensures` line. The gate is static:
   code runs unasked only when every capability it can reach is a fake, so
   tests that take the real `System`, calls into trusted `unsafe` packages and
   examples that need real capabilities run on request only.
2. **Worksheets** (N-34), described under REPL and scripts.
3. **Structural editing and a typed palette** (N-31). Commands act on the
   tree: expand and shrink the selection by node, move a statement past its
   siblings with correct re-indentation, wrap a selection in `if`, `for`,
   `with` or `task.scope`, unwrap it, extract a function that takes the
   capability parameters the selection uses. A palette request lists what is
   callable at the cursor, grouped by module and trait, and shows what is not
   available and why ("Network: needs a `Net`; `save()` was not given one").
4. **The notional-machine stepper** (N-28), for the teaching profile (Q-41),
   the compiler-enforced subset of Sisu for learners. It steps forwards and
   backwards through the interpreter's replay and shows each frame's named
   values.

The teaching profile is a monotone ladder of levels, `profile =
"teaching-3"` in the manifest, each level a strict subset of the next and
of full Sisu, so a valid program stays valid as the learner climbs; using a
construct from a later level gives a level-up diagnostic naming the
construct and the level; diagnostics in the profile carry a deterministic
gloss of dense signatures; top-level statements are confined to scripts and
teaching levels; and teachers define course ladders as data. The levels
themselves are settled by the Q-41 trial (leaning, D-073).

The permissions overlay, which shows on each line the read, write and own
permissions each binding gains and loses, waits for the bet-1 verdict: giving
the bet-1 authors an overlay while the Rust controls have none would confound
the comparison.

## Testing

```sisu
test "accepts leading zeros":
    assert parse_int("007") == Ok(7)

test "any list round-trips through JSON":
    check(arbitrary[list[int]], xs -> json.decode(json.encode(xs)) == Ok(xs))

bench "parse a 9-digit number"(b: Bench):
    for _ in b.loop():
        black_box(parse_int("123456789"))
```

The full design (discovery, the `assert` intrinsic, fixtures, parametrised
tests, fuzzing, snapshots, simulation, replay, coverage, mutation,
caching) is in [17-testing.md](17-testing.md).

- Test blocks live next to the code (Zig, D, Rust) and are compiled only by
  `sisu test`.
- Property-based testing is in the standard library with shrinking
  (QuickCheck's lineage via Hypothesis and proptest), because the research
  on its adoption shows it is used an order of magnitude more when it needs
  no dependency.
- Doc examples are tests, and `sisu doc` renders each example's expression
  statements with their values, as Unison and Go do.
- Contract checks are enabled in tests, so every test also tests the
  contracts of everything it calls.
- Tests run in parallel by default; each test gets fresh capability fakes
  (`Fs.in_memory()`, `Clock.frozen_at(...)`), which is what capabilities
  buy for testing.
- Snapshot testing (`assert_snapshot`) with reviewable updates is
  included; it is the most common third-party test addition in Rust and
  Jest's most-used feature.
- Two prototypes are accepted into the experiment (decided, Q-49; D-070).
  `sisu test --sim` runs a test in a runtime profile where `System` is
  entirely simulated: a clock that advances only when every task is blocked,
  an in-process `Net`, a crash-consistent `Fs` and a seeded, pluggable
  scheduler. Faults come from types: at each capability call the simulator may
  return any error in that method's declared error set and nothing else, and a
  `sim` build fails to compile if reachable code could escape the simulation.
  Every failing test also prints a replay token (content hash, toolchain
  version, seeds and the shrunk case), and `sisu test --replay <token>`
  reproduces the failure anywhere. A test that takes the real `System` can be
  neither simulated nor replayed, and its token says so.

## Debugging and profiling

DWARF output with full type information; the debug backend emits code that
single-steps predictably. `sisu run --profile` samples with the platform
profiler and attributes to Sisu source including monomorphised instances.
Panics and error return traces are symbolised without a separate tool.
A `Debug` rendering exists for every type (derived by default for private
types, opt-in for public ones).

`sisu run --record` writes the run to a log, and `sisu replay` re-executes it
with the log standing in for the world. In the interpreter every source of
nondeterminism is a capability call, the seeded scheduler or standard input,
so recording wraps each root capability in a logger (the interception point
the test fakes use) and writes the scheduler seed, the per-process hash key,
standard input, every capability result in order and every hot-replacement
swap. A log holds file contents and environment values, so it is as sensitive
as the data and needs per-capability redaction. The log, `sisu replay` and the
replay tokens of Testing are prototypes in the experiment (decided, Q-49;
D-070).

After the verdict they can grow into more (N-04). A cut below `System`
(`--record-at Db`) has a completeness the tool checks, because code outside
`Db`'s methods was never handed another route to the world. Native runs can
record only the order of synchronisation events and replay in the interpreter
(RecPlay's method, which needs the data-race freedom that Sisu guarantees
statically). A flight recorder can keep the tail of the log in release builds,
and `sisu test new --from-trace` can turn a recorded failure into a regression
test with preloaded fakes. The debugger can run backwards, and `sisu why` can
answer with the dynamic slice that produced a value. rr and R2 are the
precedents; no imperative language enforces the recording boundary.

Hot function replacement is committed for debug builds (D-053, decided with
Q-22): the calling convention keeps stack frames reifiable and every top-level
definition has an indirection point that release builds remove. `sisu run
--watch` swaps in the bodies that change on each save, in epochs. A successful
check publishes every changed body whose signature is unchanged as one
epoch, and a changed signature or layout restarts the program with a
message. A task adopts the newest
epoch when it starts and at `live.boundary()`, which `Ticker.wait` calls, and
resolves every call in between in the epoch it adopted, so one loop iteration
never mixes an old body with a new helper (leaning, D-068). A save that fails
to check publishes nothing. A function used as a value holds its slot, not its
body; a changed `const` or `comptime` result re-swaps every body that uses it
through the query graph; and nothing the runtime persists for the user may
contain frames, code addresses or epochs. Sonic Pi, Extempore, TidalCycles and
Erlang converged on swapping at a loop boundary; the `const` rule is
Flutter's, and the rule for saved state is the lesson of Papyrus. Resuming
from a chosen frame after a fix is the one part of Smalltalk's live
environment with no modern equal
([`smalltalk.md`](../research/languages/smalltalk.md)); it is a stated goal of
the debug ABI, not a promise, and image-based development stays out of scope.

## REPL and scripts

`sisu repl` evaluates expressions with the comptime interpreter, including
loading a package's modules. It is not a Smalltalk/Lisp live image (Q-22 keeps
image-based development out of scope) but it covers the exploratory use that
drives Python and Julia adoption. A single `.sisu` file with a
`#!/usr/bin/env sisu` line runs as a script, with dependencies declared in a
leading comment block (the Rust `cargo script` / Python PEP 723
inline-metadata design).

A *worksheet* is a script-mode `.sisu` file that the language server re-runs
top to bottom after each edit, showing each value as virtual text that is
never saved. It has no cells and no hidden state: what is on screen is what
the file computes, from the top, every time. Code that can reach a real
capability runs only on an explicit run, by the gate of the language server. A
worksheet is where one computes before any function exists, and it is tooling
after 0.1 (decided, Q-48). A notebook, with cells that rerun in any order over
hidden state, stays a non-goal
([00-vision-and-principles.md](00-vision-and-principles.md)).

## AI-assisted development

A new language starts with no training data, and model assistance tracks
training-data share ([`../research/evidence/studies.md`](../research/evidence/studies.md),
Cassano et al.). Two things reduce the penalty and are planned for: a
test-validated corpus translated from Python and Rust programs (the
MultiPL-T method) published with the language, and the type checker
exposed as a library so that type-constrained generation (Mündler et al.
2025, which halves compilation errors) can use it. A sound, fast, decidable
type system is an asset here; an unsound or slow one is not.

**Context.** Two more measures help a model. `sisu doc --digest` writes one
plain-text file per package with each `pub` signature, its inferred error set,
capability parameters, contracts and first doc example, and the toolchain
ships a pocket language reference on a token budget (about 25k tokens for the
language, about 50k for `std`), so that a model can hold a package's API in
its context window (N-50; planned for v1). The loud-divergence rule of P10
(leaning, D-062) keeps a model's Python and Rust habits from silently
producing wrong Sisu: where the spelling is shared the meaning is shared, or
the program does not compile.

The experiment measures both with a model arm (decided, Q-49). Several
frontier models solve the corpus tasks with nothing, the reference, or the
reference plus the digest, each with and without compiler feedback, and the
arm records compile and hidden-test pass rates, false-friend bugs per row of
the table, and tokens to a pass. It runs in the sandbox of the experiment's
browser playground, apart from the human results. The risk is tuning the
language to this year's models; the arm measures and the design does not chase
it. If the reference and digest do not bring the rates near those for the same
tasks in Python, the MultiPL-T corpus is necessary, not optional.

**Holes.** Unfinished code says so with a typed hole. `todo("name")` is a
compiler intrinsic of type `todo[T](name: str = "") -> T` (D-061, decided with
Q-46; there is no `?name` form). The body type-checks around it, the
interpreter runs until a test reaches it and then stops with the hole's
closure (the `Debug` rendering of each binding in scope), and `sisu test`
reports that test as blocked by the hole, a third outcome beside passed and
failed, with a status of its own: `sisu test` exits with 3 when no test
failed and one is blocked, 1 when one failed, and 0 when all passed. A
placeholder meant to ship is `panic("...")`.

`sisu check --holes` reports, per hole, the expected type and the definitions
of the types it mentions, the bindings in scope with their conventions and
capabilities, the enclosing `requires` as facts and `ensures` as obligations,
the tests that reach it with the values seen, and the expressions that fit to
depth two. It prints a human form and a JSON form and answers as a
language-server query, so the same report serves a person and a model. `sisu
build --release` and `sisu publish` reject holes, `sisu audit` lists them and
`sisu doc` marks the functions that contain one. Hazel's measured context
found that type definitions lift a model furthest in a low-resource language,
so they come first, and rust-analyzer's term search was little used, which
favours the report over enumeration
([`holes-synthesis-and-ai-era.md`](../research/unconventional/holes-synthesis-and-ai-era.md)).

**Filling.** The fill engine, `sisu fill`, is later tooling. It asks proposers
for candidates (a built-in enumerator or any external process that speaks a
JSON protocol; no model or vendor is built in, which keeps the one binary
vendor-neutral), type- and ownership-checks them, and runs the survivors in
the interpreter against the tests that reach the hole, the function's
contracts as properties over inputs that satisfy `requires`, and the property
tests that call it, all under capability fakes. A failing property's shrunk
counterexample goes back to the proposer, survivors are split by
distinguishing inputs, and a person chooses by behaviour and reads the code.

The judge is out of the filler's reach. A fill changes only the hole's span.
Tests, contracts, generators and snapshots are read-only during a fill, names
must resolve in the lockfile, and a candidate that compares an input with a
literal from a test is flagged as special-casing. `sisu test --mutate` reports
mutants of the fill that the tests do not kill, and `unfillable: <reason>` is
a legal answer. Fills judged by tests alone overfit (roughly two thirds to
four fifths of PropR's test-passing repairs in a manual sample), and models
edit tests when they can (ImpossibleBench). Fill acceptance and later
defects, with and without this filtering, are measured once `sisu fill`
exists, after 0.1; the experiment's model arm does not measure them
([20](20-pre-registration.md), section 9.2).

**Review.** The same protection applies to a whole change. `sisu review
base..head` lists, beside the code diff, each change that weakens a promise: a
removed or edited `ensures`, `invariant` or refinement; an added or edited
`requires`; a deleted test, changed expected value, new skip, narrowed
generator or smaller case budget; an updated snapshot; a dropped doc example,
`pure` or `@must_consume`. Tests and contracts are syntax, so the report is
exact where other languages need heuristics: direction is syntactic where
possible and "unknown" for an edited clause until a verifier proves
implication, and the normalised hash recognises moves and renames.

With `[review] spec-weakening = "require-trailer"` in the manifest, CI fails
unless the commit carries a human-written `Spec-Weakened-By:` trailer, which
agent harnesses can be set never to write. `sisu review --authority` adds the
authority delta of [09](09-modules-packages-and-build.md) between the two
revisions. ImpossibleBench, whose tests contradict their specifications, found
that frontier models cheat often and stronger ones more, and METR saw a
frontier model reward-hack in 30.4% of RE-Bench runs
([`ai-era-and-evolution.md`](../research/ideation/ai-era-and-evolution.md),
A2).

## Migration

`sisu fix --edition 2030` rewrites a package for a new edition using the
compiler's own understanding of the code, the same way `go fix`, `cargo
fix`, Dart's null-safety migrator and the 2to3 tool did, with the Dart
migrator's measured outcome (the whole ecosystem migrated in about a year)
as the target and Python's (twelve years) as the thing to avoid.

Libraries migrate their users the same way. A library deprecates an item by
rewriting its body in terms of the new API and marking it `@fix_inline`, and
`sisu fix --deps` replaces dependents' calls with that body, so behaviour is
preserved by construction, with no template language and no macros. The
inliner is ownership-aware: borrows, `own` moves and `defer` move into the
caller's scope. A major release's API diff marks the removals that had such a
form as migratable ([09](09-modules-packages-and-build.md)). Go's `//go:fix
inline` (applied by `go fix -inline`), Dart's data-driven fixes (verified
against goldens) and Kotlin's `ReplaceWith` are the precedents.
