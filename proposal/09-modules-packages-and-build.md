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
  deterministic given the lockfile. Go's minimal version selection was
  considered (open question Q-20): it avoids a lockfile and a SAT solver,
  but it also means a library author's declared minimum is what everyone
  gets unless they upgrade, which has made Go module upgrades a manual
  chore in large graphs.
- **Features** are additive booleans (Cargo). A feature can never remove
  an API; the tool checks this at publish time.
- **One version of a package per build**, unless two major versions are
  needed, in which case both are linked with mangled names (Cargo's
  behaviour). Node's one-version-per-dependent nesting is the reason
  `node_modules` is a meme.
- **`[trust]` is where capabilities become real.** A package not listed
  there compiles with `unsafe` and `extern` disabled, and therefore can only
  do what the capabilities it is handed allow. `sisu add` prints the
  trust requirement of a new dependency and refuses silently extending it.
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
  cannot catch semantic changes behind an unchanged signature; those remain
  the author's responsibility, and `sisu publish --breaking` records the
  reason in the changelog.
- **Provenance.** Publishing is tied to a signed build attestation (the
  npm/Sigstore model), so a package can be traced to the source commit and
  the publishing identity.
- **Capability summary.** The registry displays, for each version, which
  capabilities its public API takes and whether it is `unsafe`-trusted by
  default. A library that suddenly starts asking for `Net` in a patch
  release is visible before it is installed.

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
| `postinstall`/`build.rs` | Supply-chain vector; replaced by comptime. |
| Unused import/variable as a hard error (Go) | The Go survey's most-cited daily annoyance. Sisu warns, `sisu fmt` removes unused imports, and CI runs `sisu check --deny-warnings`. |

## Additions proposed in October 2026

From [19-novel-features.md](19-novel-features.md): the manifest stays
data, with structured target conditions (`winapi = { version = "0.3",
when = { os = "windows" } }`) rather than `cfg()` strings, and anything
computed lives in `comptime` Sisu (D-063). `sisu add` and `sisu update`
report an authority delta (newly reachable capabilities, `unsafe`,
`extern`, `[trust]` changes, new dependencies and panic sites), apply a
seven-day cooldown before resolution with advisory fixes exempt, refuse
packages younger than that or without dependents unless told otherwise,
and in CI or agent runs never add a package or widen `[trust]` against
policy; packages whose API takes no capability types and that are not in
`[trust]` are labelled pure in the registry (D-066). Libraries ship migrations as `@fix_inline`
functions applied by `sisu fix --deps` (D-067). With the registry come
one prototype, a review ledger keyed by content hashes of the capability
slice (N-43), and one tool, behavioural semver, where `sisu publish` runs
the previous version's black-box tests (N-44).
