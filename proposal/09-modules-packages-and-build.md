# 09 - Modules, packages and build

The research is more unanimous here than anywhere else. Every language
whose packaging was left to the ecosystem (C, C++, Python, JavaScript,
Haskell, Java) spent a decade or more in pain, and every language that
shipped one blessed tool (Go, Rust, Elixir, Dart, Zig, and belatedly
Python via `uv`) is praised for it in its surveys
([`../research/evidence/surveys.md`](../research/evidence/surveys.md)).
Sisu ships one tool and designs the module system for it.

## Modules

- A **file** is a module. A **directory** is a module whose children are
  the files and subdirectories in it. The module tree mirrors the
  file tree with no `mod` declarations (Rust's `mod foo;` is a frequent
  beginner question; Go's directory-is-package is simpler and sufficient).
- A **package** is a directory with a `sisu.toml` and a `src/` tree. A
  package has one root module (`src/lib.sisu` or `src/main.sisu`).
- Items are private to their module by default. `pub` exports to the
  parent and, transitively, to the package's public API; `pub(package)`
  exports within the package only.
- `import` and `from ... import` bring items into scope with Python's
  spelling: `import std.json` (binds `json`, the last component),
  `from http import Client, Request as Req`, `from weather.parser import
  parse_int` (package-absolute paths; a package names its own modules by
  its own name, with no `crate` or `super`). Paths are dot-separated.
  There is no `from x import *`: the research on Rust and Python shows
  that unrestricted glob imports make name resolution unreadable; enum
  variants are usable unqualified where the type is known instead.
- **Package dependency cycles are errors.** Modules within a package may
  reference each other freely. This is Go's rule and the reason Go builds
  are parallel at package granularity; it also forces layered designs.
- There is no `prelude` magic beyond a fixed, documented list of names
  (`Option`, `Result`, `Some`, `None`, `Ok`, `Err`, `list`, `dict`, `set`,
  `String`, `Box`, `Shared`, `Weak`, `Cell`, `Mutex`, `Atomic`, `Arena`,
  `task`, `channel`, `Duration`, `Path`, the capability types `System`,
  `Fs`, `Net`, `Clock`, `Rand`, `Env`, `Proc`, and the prelude functions
  `print`, `len`, ...), the same in every module.

## Package manifest

```toml
[package]
name = "weather"
version = "0.3.1"
edition = "2027"
description = "Fetches and caches weather data"
license = "MIT"

[dependencies]
http = "1.2"                                     # semver: >=1.2.0, <2.0.0
json = "2"
tz = { version = "0.9", features = ["embedded-db"] }
winapi = { version = "0.3", when = { os = "windows" } }   # structured, not a cfg() string
mylib = { path = "../mylib" }
other = { git = "https://...", rev = "abc123" }

[dev-dependencies]
fakes = "1"

[features]
default = ["tls"]
tls = ["http/tls"]

[trust]
unsafe = ["http", "tz"]          # packages allowed to contain unsafe/extern

[profile.release]
contracts = "preconditions"
```

Design points, each with its source:

- **Semver ranges with a lockfile** (Cargo, Bundler, npm). `sisu.lock` is
  committed by applications and ignored by libraries. Resolution is
  deterministic given the lockfile and picks the newest version that
  satisfies every range (Q-20). Go's minimal version selection was
  considered and not adopted: it avoids a lockfile and a SAT solver,
  but it also means a library author's declared minimum is what everyone
  gets unless they upgrade, which has made Go module upgrades a manual
  chore in large graphs.
- **Features** are additive booleans (Cargo). A feature can never remove
  an API; the tool checks this at publish time.
- **The manifest stays data** (leaning, D-063). `sisu.toml` has no
  conditionals, templating or expressions. A dependency that applies only on
  some targets says so with a structured condition over the build target's
  facts, `winapi = { version = "0.3", when = { os = "windows" } }`, not with a
  Cargo-style `cfg(...)` string, which is an expression language growing
  inside a TOML key. Anything that has to be computed lives in `comptime`
  Sisu, not in the manifest. `sisu add` can edit data reliably and expressions
  not at all, and the field has moved the same way: Gradle is building
  Declarative Gradle, a tiny subset of Kotlin, and HashiCorp archived CDK for
  Terraform
  ([`config-and-total-languages.md`](../research/unconventional/config-and-total-languages.md)).
- **One version of a package per build**, unless two major versions are
  needed, in which case both are linked with mangled names (Cargo's
  behaviour). Node's one-version-per-dependent nesting is the reason
  `node_modules` is a meme.
- **`[trust]` is where capabilities become real.** A package not listed
  there compiles with `unsafe` and `extern` disabled, and therefore can only
  do what the capabilities it is handed allow. `sisu add` prints the
  trust requirement of a new dependency and refuses silently extending it;
  Admission below lists what else it reports.
- **No build scripts.** Rust's `build.rs` and npm's `postinstall` are the
  two most exploited supply-chain vectors. Anything a build script would do
  (code generation, embedding files, platform detection) is done by
  `comptime` (sandboxed) or by a tool the developer runs explicitly and
  commits the output of.

## The registry

- **Content-addressed, immutable.** A published version is identified by
  the hash of its source archive. It cannot be changed or deleted, only
  *yanked* (hidden from resolution for new users, still available to
  lockfiles). Go's module proxy and sumdb, Unison's content addressing, and
  the `left-pad` incident decide this.
- **Transparency log.** Every publish is appended to a public, verifiable
  log (Go sumdb design). A client can prove a given hash was the one
  published and that everyone sees the same one.
- **Namespaced by owner.** `roni/weather`, not `weather`. crates.io's and
  PyPI's flat namespaces produced squatting, typo-squatting and name
  disputes; npm's scopes were added later for this reason. The namespace is
  the publisher's verified identity.
- **API-diff-enforced semver.** On `sisu publish`, the tool compares the
  package's public API (types, signatures, trait impls, error sets,
  contracts) with the previous version. A breaking change without a major
  version bump is rejected. Elm's package manager has done this since 2014
  and its ecosystem has never had an accidental breaking patch release
  ([`elm-purescript.md`](../research/languages/elm-purescript.md)). It
  cannot see a semantic change behind an unchanged signature. The next
  bullet covers part of that gap, and what remains is the author's
  responsibility, with `sisu publish --breaking` recording the reason in
  the changelog.
- **Behavioural semver** (tooling with the registry, N-44; see
  [19](19-novel-features.md)). `sisu publish` also runs the previous version's
  black-box tests and doc examples against the candidate. A failure needs a
  major version or a recorded, public `--supersede "<test>" --reason "..."`
  that appears in the changelog; the record exists for Hyrum's law, since a
  fix can break a test that encoded the bug. NoRegrets classified at least 90%
  of npm updates correctly from clients' tests and found 26 breaking minor or
  patch releases; Dependabot's compatibility score is missing for 83% of
  updates; no package manager replays a package's own previous tests at
  publish
  ([`ai-era-and-evolution.md`](../research/ideation/ai-era-and-evolution.md),
  A5).
- **Migrations travel with the library.** A library deprecates an item by
  rewriting its body in terms of the new API and marking it `@fix_inline`;
  `sisu fix --deps` replaces dependents' calls with that body, so behaviour is
  preserved by construction, with no template language and no macros (leaning,
  D-067). The inliner is ownership-aware: borrows, `own` moves and `defer`
  move into the caller's scope. A major release's API diff marks the removals
  that had such a form as migratable. Go's `//go:fix inline`, Dart's
  data-driven fixes and Kotlin's `ReplaceWith` are the precedents.
- **Provenance.** Publishing is tied to a signed build attestation (the
  npm/Sigstore model), so a package can be traced to the source commit and
  the publishing identity.
- **Capability summary.** The registry displays, for each version, which
  capabilities its public API takes and whether it is `unsafe`-trusted by
  default. A library that suddenly starts asking for `Net` in a patch
  release is visible before it is installed. A package whose API takes no
  capability types and that is not in `[trust]` (so it has no `unsafe` or
  `extern`) is labelled *pure*: it can only compute, and it needs no
  capability review (leaning, D-066). `sisu audit --authority` computes the
  same label.
- **Review ledger** (a prototype with the registry, N-43). An audit record in
  `supply-chain/reviews.toml` holds a Merkle root over the normalised-AST
  hashes of its scope, which by default is the *capability slice*: the
  functions that can reach a capability, `unsafe` or `extern`. If the root is
  unchanged in the next version, the review carries over; otherwise only the
  functions whose hashes changed are listed. A pure parser has an empty slice.
  cargo-vet records audits per version and as deltas between versions, and
  crev's proofs can cover parts of code, but neither is keyed by
  function-level content hashes. The hash would become a specified artefact
  versioned with editions, and slice size and churn are unmeasured.

### Admission

Adding or updating a dependency is the moment a package gains authority, so
`sisu add` and `sisu update` say what changes before they touch the lockfile.
The *authority delta* lists, per package and per public function, the
capabilities that become reachable (through parameters, fields, closures and
`dyn` values), new `unsafe` and `extern`, changes to `[trust]`, new
dependencies and new panic sites. With no ambient authority beyond the
standard streams, reach follows from types, so the report is exact up to
`unsafe` in trusted packages, and it reuses the computation behind the
capability summary. `sisu review --authority` ([10](10-tooling.md)) gives the
same report between any two revisions. The signal is worth reading: Capslock
computes capabilities for Go, and deps.dev found that fewer than 2% of version
updates add one
([`capabilities-distribution-and-resources.md`](../research/unconventional/capabilities-distribution-and-resources.md),
idea 4).

Three rules apply before a version is admitted:

- **Cooldown.** A seven-day cooldown is applied before resolution: a version
  younger than seven days is not a candidate, and advisory fixes are exempt.
  This refines the newest-satisfying resolution of Q-20, which still picks the
  newest version that satisfies every range, among the versions that have
  passed the cooldown. Eight of ten recent supply-chain attacks had windows
  shorter than a week, and pnpm's `minimumReleaseAge` and uv's `exclude-newer`
  ship the same idea.
- **New packages.** A package younger than the cooldown, or one that no other
  package depends on, is refused unless `--accept-new-package` is given.
  Language models invent package names in 4.6 to 21.7% of suggestions across
  studies, and 53 of the names that five models all invented were still
  registrable
  ([`ai-era-and-evolution.md`](../research/ideation/ai-era-and-evolution.md),
  A2 and A3).
- **Unattended runs.** In CI or in an agent run, nothing adds a package or
  widens `[trust]` against the project's policy. The Nx compromise used
  `postinstall` to drive the victim's AI tools; the rule against install
  scripts and capability confinement each block that route.

## Build

- `sisu build` compiles the current package and its dependency graph, in
  parallel at package granularity and, within a package, at module
  granularity. Dependencies are compiled once and cached globally by
  content hash (so two projects depending on `http@1.2.3` with the same
  features share the artefact). Unison's and Bazel's content-addressed
  caching are the model.
- Incremental compilation is query-based (rustc's Salsa model, Roslyn): the
  compiler memoises per-item results keyed on the hash of their inputs, so
  editing a function body recompiles that body and nothing else. The
  explicit-signature rule ([03](03-types-and-generics.md)) is what makes
  this possible. The cache key is the hash of the *normalised* AST (names,
  comments and formatting stripped, dependencies replaced by their hashes),
  which is Unison's content-addressing applied to ordinary text files
  ([`unison.md`](../research/languages/unison.md)): a rename or reformat
  recompiles nothing, and test results are cached by the same key.
- **Targets**: `sisu build --target wasm32-wasi` cross-compiles without a
  separately installed toolchain, as Zig and Go do. Cross-compilation that
  requires installing a second compiler is cross-compilation nobody does.
- **Profiles**: `debug` (fast compile, all checks, own backend), `release`
  (LLVM, optimised, overflow and bounds checks on, contracts per manifest),
  `release-size`. There is no profile that disables bounds or overflow
  checks; Zig's `ReleaseFast` was studied and rejected because safety
  that can be turned off in release is safety that is turned off in release
  (see D's `@safe` adoption data in [`d.md`](../research/languages/d.md)).

## Workspaces

A directory with a `sisu.toml` containing `[workspace] members = [...]`
builds its members together with one lockfile. Cargo's design, adopted
unchanged.

## Documentation

`sisu doc` renders `##` comments (Markdown), signatures, contracts,
inferred error sets, capability parameters, and examples. Doc examples are
compiled and run by `sisu test`, which is how Rust's and Python's doctests
keep documentation honest. A package without documentation on its `pub`
items gets a warning at publish time; the registry shows documentation
coverage.

## What was rejected

| Alternative | Reason |
|---|---|
| Header files, separate interface files (`.mli`, `.h`) | OCaml's `.mli` is the strongest argument for them (explicit, stable interfaces); but explicit signatures plus `pub` give the same information without a second file to keep in sync. |
| Implicit module declaration via `mod` (Rust) | Beginner friction with no benefit over the file tree. |
| Global namespace / flat registry | Squatting; see above. |
| Vendoring as the primary distribution (Go pre-modules, C) | Replaced by the content-addressed cache, which gives reproducibility without copying. |
| Multiple competing build tools | The C++/Java/Python outcome. |
| `cfg()` strings, executable manifests (`build.zig`, `Package.swift`, Gradle scripts) | A manifest that computes grows into a weaker programming language, and `sisu add` can edit data reliably and expressions not at all. Target conditions are structured keys and anything computed is `comptime` Sisu (D-063). |
| `postinstall`/`build.rs` | Supply-chain vector; replaced by comptime. |
| Unused import/variable as a hard error (Go) | The Go survey's most-cited daily annoyance. Sisu warns, `sisu fmt` removes unused imports, and CI runs `sisu check --deny-warnings`. |
