# 13 - Evolution and governance

A language's biggest decisions are made after it ships, by whoever is
allowed to make them, under whatever rules exist for breaking things. The
research ([`../research/evidence/designer-retrospectives.md`](../research/evidence/designer-retrospectives.md))
contains more regret about this than about any syntax choice.

## Compatibility

### The promise

From version 1.0: a program that compiles on 1.x compiles on every later
1.y with the same behaviour, with these documented exceptions: security
fixes, changes to unspecified behaviour, and bugs in the compiler where the
fix is announced one release in advance. This is Go's compatibility promise
(2012), word for word in spirit. The research on Go is that the promise is
the reason enterprises trust it; the research on Python 3, Swift 1-3,
Scala 2-3 and Perl 6 is the cost of not having one.

### Editions

Every three years a new edition may change syntax and defaults in
incompatible ways. A package declares its edition in its manifest; packages
on different editions link together because the compiler understands all
of them and the ABI and the type system are shared. `sisu fix --edition`
migrates automatically. This is Rust's mechanism (2015, 2018, 2021, 2024),
which the research identifies as the only known way to fix early mistakes
without a Python-3-style split. The constraint that makes it work: an
edition cannot change the *semantics* of a type or trait that other
editions see, only surface syntax, lints, defaults and the prelude.

### Deprecation

Deprecated items warn with the replacement named and a `sisu fix` rewrite
where mechanical. Nothing in `std` is ever removed; it is moved to
`std.deprecated` after two editions, still compiling. Java's permanent
`Date` is the honest model; Python's removal of modules in 3.13 broke code
that had worked for twenty years.

### Specification

A written specification exists before 1.0, is updated with every change,
and is the arbiter when the compiler and the document disagree. Standard
ML's formal definition, C's and Go's specifications, and Ferrocene's
retroactive Rust specification (needed for safety certification) are the
evidence; Rust's decade without one is the counter-evidence. The
specification is prose plus a reference test suite, not a formal semantics
in version 1 (open question Q-38 asks whether a mechanised core semantics,
as Austral and WebAssembly have, should be a goal).

## Governance

### Phase 1 (now until 1.0): a small design team with a lead

A benevolent-dictator model with a written decision process. Every
language with a coherent design had a single mind or a very small group at
the start (Python, Go, Rust before 2015, Clojure, Elm, Zig, Elixir); the
committee-from-the-start languages (C++, Scheme R6RS/R7RS, Ada 83) show
the alternative. The risk is the BDFL walking away or stalling (Elm's 6.7-year gap
between 0.19.1 and 0.19.2, Python after PEP 572) or becoming the bottleneck (Zig's RFC-free process is
praised for coherence and criticised for opacity).

Mitigations from the start:

- Every decision is written down with its alternatives and evidence (this
  repository's decision log is the first instance). Carbon's governance
  docs are the model.
- A public proposal process for changes, with a time-boxed decision: a
  proposal is accepted, rejected or deferred within 90 days, with reasons.
  Rust's RFC process is the model for openness; its documented burnout and
  multi-year stalls are the reason for the time box.
- The lead's authority is over *design*, not over the project's
  resources: the foundation (below) owns the trademark, the registry, and
  the infrastructure from day one, so the project survives the person.

### Phase 2 (after 1.0): foundation plus a language team

A non-profit foundation holds the assets. A language team of 5-9 people
owns the specification, with the former lead as one member. Changes go
through the proposal process; the team decides by consensus with a vote as
fallback; the foundation board cannot overrule design decisions and the
language team cannot spend the foundation's money. Rust Foundation and
Python Software Foundation are the models; the Rust 2023 governance crisis
(unclear boundaries between the foundation, the core team and the project)
is the reason the boundaries are written here before they matter.

### Corporate sponsorship

Accepted; no single sponsor may hold a majority of the language team or of
the board. Go (Google), Swift (Apple), Kotlin (JetBrains), C# (Microsoft)
and Dart (Google) are all excellent languages whose roadmaps are visibly
the sponsor's roadmap. The research is clear that single-vendor control
helps velocity and hurts trust; the trade is made for trust.

## Release cadence

- A release every six months, each supported for one year; every fourth
  release (two years) is a long-term-support release supported for three
  years. Java's cadence since 2017, which its research identifies as the
  change that revived the language.
- Features ship behind `@preview` gates for at least one release before
  stabilisation; previews may change or be removed. Java's preview
  features, Rust's nightly features and Python's `__future__` are the
  models; shipping a feature straight to stable and discovering it is
  wrong (Go's `defer` scoping, Swift's early concurrency) is the
  counter-evidence.

## Naming the risks

The honest list of ways this fails, so that the people doing it can watch
for them:

1. The design lead burns out or stalls before 1.0 (Elm 2019-2026, Vale,
   Hylo's two compiler rewrites without a release). Mitigation: the
   decision log makes the design transferable; two people must understand
   every subsystem.
2. The proposal process becomes a bottleneck (Rust 2020-2023) or a
   rubber stamp (various). Mitigation: the time box; publishing acceptance
   and rejection rates.
3. A sponsor captures the roadmap. Mitigation: the majority rule above.
4. The compatibility promise prevents fixing a serious mistake. Mitigation:
   editions, and a preview period long enough to find serious mistakes.
5. The standard library accretes (Python) or starves (Rust). Mitigation:
   the `std`/`std.x` split, with the explicit rule that promotion from
   `std.x` to `std` requires two years without a major version.
