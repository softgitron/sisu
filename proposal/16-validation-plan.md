# 16 - Validation plan

The research is consistent on one more point: languages that spend years
in design without runnable code (Carbon, Hylo, Roc, Pony, Vale, Austral)
lose their audience before they ship, and languages that ship a small
runnable thing early (Zig, Gleam, Elixir, Go) keep it. This document says
what the first prototype must answer, in what order, and what evidence
would change the proposal.

## Order of attack

The three bets in [`README.md`](README.md) are tested first, because a
negative result on any of them changes the design more than anything
else could.

### 1. Ownership without named lifetimes (D-004, Q-1)

**Build:** the type checker for parameter conventions, `ref`/`ref mut`
with non-lexical scopes, `view struct`, `Shared`, `Arena` with handles.
No code generation needed beyond an interpreter.

**Corpus:** five programs written by people who did not design the
language, each chosen because it is where Rust programmers meet lifetimes:
a tokenizer and parser producing an AST that borrows from the source; a
graph algorithm with parent pointers; an HTTP server with a shared cache;
a GUI-style widget tree with event callbacks; an iterator adapter library.

**Measure:** how often each program needs `Shared`, `Weak`, arena handles
or `unsafe`; how many borrow errors occur per hundred lines and how many
the author considered "fighting the checker"; whether any program could
not be written without a feature Sisu lacks.

**Changes the proposal if:** more than one program needs `unsafe` for an
ordinary pattern, or the authors report friction comparable to Rust's.
Then: add a Vale-style generation-checked reference type outside arenas
(Q-1), or add named lifetimes as an opt-in annotation on `view struct`.

### 2. Capability parameters at scale (D-015, Q-5)

**Build:** `std.cap` with real and fake implementations; the `[trust]`
mechanism in the package manager.

**Corpus:** the HTTP server and the command-line tool from the ownership
corpus, plus a port of a mid-sized existing Go or Rust service.

**Measure:** the median and maximum number of capability parameters on
functions that do I/O; how far up the call graph a capability is threaded
for a single leaf use; how often code stores capabilities in structs to
avoid threading; test-writing time with fakes versus with a mocking
library in the source language.

**Changes the proposal if:** the median I/O-doing function carries more
than two capability parameters, or authors routinely bundle all
capabilities into one struct to avoid the parameter list (which is ambient
authority under another name). Then: implement the reserved `with Fs,
Clock` signature clause.

### 3. Colourless tasks without a moving GC (D-014, Q-6)

**Build:** the `full` runtime: M:N scheduler, lazily committed stacks,
compiler-inserted yield points, structured scopes, cancellation.

**Measure:** task creation cost and memory per idle task against Go and
Java virtual threads; latency distribution for a 10k-connection echo
server; behaviour under deep recursion in a task; starvation under mixed
compute and I/O loads (Q-34).

**Changes the proposal if:** per-task memory is more than four times
Go's, or stack exhaustion is common in ordinary code. Then: stackless
compilation of task bodies (C#-style state machines) behind the same
`scope`/`spawn` API, accepting the Wasm-friendly trade.

## Then, in parallel

| Question | Experiment | Threshold |
|---|---|---|
| Q-3 implicit copies | Count `.copy()` in the corpus; prototype a Swift-style implicit-copy variant and count "why is this slow" reports | More than one `.copy()` per 30 lines in application code reopens it |
| Q-7 error syntax | Write the standard library; count `else`/`match`/`?` friction and inferred-set surprises | Inferred-set changes in public APIs that broke a dependent reopen the `pub` lint as an error |
| Q-9 last-expression values | Diagnostic corpus: frequency of "expected (), found T" | Top-ten diagnostic reopens it |
| Q-13 implicit widening | Type-checker ambiguity cases in the standard library | Any defaulting rule needed to disambiguate reopens it |
| Q-11 coherence | Count newtype wrappers in the corpus | More than one per 200 lines reopens scoped instances |
| D-038 compile speed | Measure against the targets in [10-tooling.md](10-tooling.md) on the corpus | Missing any target by 2x is a design review, not an optimisation task |
| Q-2 `Shared` atomics | Benchmark escape-analysis elision against a manual `Rc`/`Arc` split | Within 10% keeps one type |

## What a 0.1 contains

Following the Zig/Gleam/Elixir pattern rather than the Carbon/Hylo one:

- The core language: bindings, functions, structs, enums, match, traits,
  generics, error sets, `?`, parameter conventions, `ref`, `view`,
  `Shared`, `Arena`, `scope`/`spawn`, capabilities, contracts (checked at
  runtime), `comptime` (interpreter), `derive` for the core traits.
- `std.core`, `std.collections`, `std.string`, `std.fmt`, `std.task`,
  `std.cap` with in-memory fakes, `std.test`, `std.io`, `std.fs`,
  `std.net` (TCP only).
- `sisu build/run/test/fmt/check` with the debug backend only; the
  language server with diagnostics and hover.
- One target (x86-64 Linux), one runtime profile (`full`).
- A specification draft covering everything above.

Not in 0.1: LLVM release builds, WebAssembly, `freestanding`, the
registry, API-diff semver, header import, the verifier, `std.x`.

## What would make us stop

The honest list. The project should be abandoned, or its memory model
replaced with a tracing GC, if after the ownership experiment:

- the corpus authors cannot write the widget tree or the graph without
  `unsafe`, and no second-tier reference design fixes it without
  reintroducing named lifetimes; or
- the borrow-error rate and the authors' reported friction are not
  materially below Rust's for the same programs.

In that case Sisu's remaining contributions (capabilities, error sets,
structured colourless concurrency, contracts, tooling) are still valid on
top of a GC, and the proposal should be rewritten as "Go with sum types,
capabilities and contracts", which is a smaller but still useful
language.
