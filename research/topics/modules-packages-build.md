# Modules, packages and build

## The question

How code is organised into units, how dependencies are declared, resolved,
distributed and trusted, and how a program is built reproducibly.

## What the evidence says

1. **This is the most unanimous finding in the research.** Every survey's
   tooling questions and every retrospective agree: languages that shipped
   one blessed dependency and build tool (Go, Rust/Cargo, Elixir/mix, Dart,
   Zig, Julia/Pkg, and belatedly Python via `uv`) are praised for it, and
   languages that left it to the ecosystem (C, C++, Java's Maven/Gradle
   split, Python's two decades, JavaScript's bundler churn, Haskell's
   Cabal/Stack split, Lua's per-host fragmentation) spent a decade or more
   in pain ([../evidence/surveys.md](../evidence/surveys.md),
   [python.md](../languages/python.md), [java.md](../languages/java.md),
   [haskell.md](../languages/haskell.md), [lua.md](../languages/lua.md)).
   `uv` is the cleanest natural experiment: zero to 43% of Django survey
   respondents in under two years by being the integrated tool Python
   never had.
2. **The module system is a build-speed decision.** Go's acyclic imports
   and directory-is-package give package-granularity parallel builds;
   C++20 modules are still not routinely usable in 2026 because the
   language did not own the build graph; Fortran's `.mod` and C++'s BMIs
   are compiler-specific artefacts twice over ([go.md](../languages/go.md),
   [cpp.md](../languages/cpp.md), [fortran.md](../languages/fortran.md)).
   The compiler or a first-party driver must own scanning, caching and
   the dependency DAG.
3. **Headers and separate interface files are a verbosity cost nobody
   else accepted.** Ada's spec/body split and OCaml's `.mli` give explicit
   interfaces; explicit signatures plus visibility markers give the same
   information in one file ([ada-spark.md](../languages/ada-spark.md),
   [ocaml.md](../languages/ocaml.md)).
4. **Registries must be immutable, content-addressed and tamper-
   evident.** left-pad (an unpublish broke the ecosystem), Go's module
   proxy and sumdb transparency log (the answer), Unison's content
   addressing, Julia's content-addressed artifacts
   ([javascript.md](../languages/javascript.md), [go.md](../languages/go.md),
   [unison.md](../languages/unison.md), [julia.md](../languages/julia.md)).
5. **Flat namespaces get squatted.** crates.io, PyPI and RubyGems name
   disputes and typo-squatting; npm added scopes later; owner-namespacing
   from the start costs nothing ([rust.md](../languages/rust.md),
   [python.md](../languages/python.md), [ruby.md](../languages/ruby.md)).
6. **Install-time code execution is the attack vector.** npm
   `postinstall`, Rust `build.rs`, Python `setup.py`: the 2025-2026 npm
   worm wave used install scripts and long-lived publish tokens
   ([javascript.md](../languages/javascript.md)). No install scripts,
   short-lived tokens, provenance, and per-dependency capabilities are the
   measures the research recommends.
7. **Lockfiles versus minimal version selection.** Cargo, Bundler and npm
   use lockfiles with newest-matching resolution; Go's MVS is reproducible
   from the manifest alone with no SAT solver, at the cost of manual
   upgrades in large graphs. Both work; the research slightly favours the
   lockfile because API-diff semver enforcement makes "newest compatible"
   trustworthy ([go.md](../languages/go.md), [ruby.md](../languages/ruby.md)).
8. **Semver can be enforced from the type signatures.** Elm's package
   manager has refused breaking changes without a major bump since 2014
   and its ecosystem has never had an accidental breaking patch; the gap
   (behaviour changes behind unchanged types) is real and Elm's own 2014
   post names it ([elm-purescript.md](../languages/elm-purescript.md)).
9. **A standard library big enough to avoid micro-packages is a security
   property.** The npm micro-package culture and its attack surface
   followed directly from shipping without a standard library; Go's
   stdlib kept its ecosystem coherent ([javascript.md](../languages/javascript.md),
   [go.md](../languages/go.md)).
10. **Unused imports as hard errors is Go's most-cited daily annoyance**;
    warnings plus auto-fix plus a strict CI mode gets the benefit without
    the friction ([go.md](../languages/go.md), [flix.md](../languages/flix.md)).
11. **Content-addressed caching without abandoning files.** Unison proves
    the benefits (never compile the same code twice, cached tests, free
    renames) and the cost (seven years rebuilding every tool); hashing the
    normalised AST of ordinary files captures most of the benefit
    ([unison.md](../languages/unison.md)).
12. **Per-package language versioning is the migration mechanism that
    worked.** Dart's null-safety migration (98% of top packages within a
    year) and Go 1.22's loop-variable fix both used a per-module version
    gate ([dart.md](../languages/dart.md), [go.md](../languages/go.md)).

## Recommendation for Sisu

File-is-module, directory-is-module, no `mod` declarations, no headers, no
package cycles; `sisu.toml` with semver ranges, a lockfile, additive
features and a `[trust]` list; an owner-namespaced, content-addressed,
immutable registry with a transparency log, provenance and API-diff-
enforced semver; no build scripts; query-based incremental compilation
keyed on normalised-AST hashes; cross-compilation built in; warnings not
errors for unused items. See
[proposal/09](../../proposal/09-modules-packages-and-build.md); decisions
D-019, D-031, D-032.

## Open questions

- Q-19 registry federation; Q-20 lockfile vs MVS; Q-29 default visibility;
  Q-32 unused-item severity.
