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
sisu fmt                 format; no options
sisu lint                lints beyond the compiler's warnings, with fix-its
sisu doc                 render documentation; `--open` serves it locally
sisu add/remove <pkg>    edit the manifest and lockfile
sisu update              resolve newer versions within ranges
sisu publish             API diff, provenance, upload
sisu audit               trusted packages, unsafe blocks, capability summary, advisories
sisu fix                 apply compiler-suggested migrations (editions, deprecations)
sisu explain E0123       long-form explanation of a diagnostic
sisu lsp                 language server
sisu repl                interactive evaluator (built on the comptime interpreter)
sisu bench               benchmarks with statistical reporting
```

Go, Rust (Cargo), Zig, Elixir (mix), Dart, Deno and `uv` are the evidence
that this list, in one tool with one configuration file, is what users
rate highest. The counter-examples (C++, Java, Python before `uv`,
JavaScript before Vite/Bun consolidation, Haskell's Cabal/Stack split) are
the evidence for what happens otherwise.

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

The TypeScript team's 2025 port of its compiler from TypeScript to Go for a
10x speedup ([`typescript.md`](../research/languages/typescript.md)) is
noted: Sisu's compiler is written in Sisu (after bootstrapping), with the
expectation that a language designed for predictable performance can
compile itself quickly. Until the bootstrap, the prototype compiler is
written in Rust (closest semantics, mature backend bindings).

## Diagnostics

Every diagnostic has:

- A stable code (`E0381`) and a `sisu explain` page with examples.
- A one-line summary in plain language naming the thing that is wrong,
  not the compiler phase that noticed it.
- The source span, with labels on each relevant location (Rust's
  multi-span layout).
- Where possible, a suggested fix as a diff the LSP can apply.
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
message text.

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
tests, fuzzing, snapshots, coverage, mutation, caching) is in
[17-testing.md](17-testing.md).

- Test blocks live next to the code (Zig, D, Rust) and are compiled only by
  `sisu test`.
- Property-based testing is in the standard library with shrinking
  (QuickCheck's lineage via Hypothesis and proptest), because the research
  on its adoption shows it is used an order of magnitude more when it needs
  no dependency.
- Doc examples are tests.
- Contract checks are enabled in tests, so every test also tests the
  contracts of everything it calls.
- Tests run in parallel by default; each test gets fresh capability fakes
  (`Fs.in_memory()`, `Clock.frozen_at(...)`), which is what capabilities
  buy for testing.
- Snapshot testing (`assert_snapshot`) with reviewable updates is
  included; it is the most common third-party test addition in Rust and
  Jest's most-used feature.

## Debugging and profiling

DWARF output with full type information; the debug backend emits code that
single-steps predictably. `sisu run --profile` samples with the platform
profiler and attributes to Sisu source including monomorphised instances.
Panics and error return traces are symbolised without a separate tool.
A `Debug` rendering exists for every type (derived by default for private
types, opt-in for public ones). The debug build's calling convention keeps
stack frames reifiable so that a debugger can replace a function and resume
from a chosen frame, the one part of Smalltalk's live environment with no
modern equal ([`smalltalk.md`](../research/languages/smalltalk.md)); this
is a stated goal of the debug ABI, not a promise (open question Q-22).

## REPL and scripts

`sisu repl` evaluates expressions with the comptime interpreter, including
loading a package's modules. It is not a Smalltalk/Lisp live image (open
question Q-22) but it covers the exploratory use that drives Python and
Julia adoption. A single `.sisu` file with a `#!/usr/bin/env sisu` line
runs as a script, with dependencies declared in a leading comment block
(the Rust `cargo script` / Python PEP 723 inline-metadata design).

## AI-assisted development

A new language starts with no training data, and model assistance tracks
training-data share ([`../research/evidence/studies.md`](../research/evidence/studies.md),
Cassano et al.). Two things reduce the penalty and are planned for: a
test-validated corpus translated from Python and Rust programs (the
MultiPL-T method) published with the language, and the type checker
exposed as a library so that type-constrained generation (Mündler et al.
2025, which halves compilation errors) can use it. A sound, fast, decidable
type system is an asset here; an unsound or slow one is not.

## Migration

`sisu fix --edition 2030` rewrites a package for a new edition using the
compiler's own understanding of the code, the same way `go fix`, `cargo
fix`, Dart's null-safety migrator and the 2to3 tool did, with the Dart
migrator's measured outcome (the whole ecosystem migrated in about a year)
as the target and Python's (twelve years) as the thing to avoid.
