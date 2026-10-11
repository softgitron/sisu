# Evolution and governance

## The question

How a language changes after it ships, who decides, and how trust is kept.

## What the evidence says

### Compatibility and breaking changes

1. **A successor that cannot run existing code kills momentum.** Python 3
   (twelve years of dual-run), Perl 6/Raku (fifteen years, killed Perl's
   momentum), Swift 1-3 (three years of ecosystem rot), Scala 2-3 (macros
   blocked migration for years) ([python.md](../languages/python.md),
   [perl-php.md](../languages/perl-php.md), [swift.md](../languages/swift.md),
   [scala.md](../languages/scala.md)).
2. **Never breaking is also a trap.** JavaScript's "never break the web"
   makes every mistake permanent; Java's `Date`, `Vector` and serialization
   live forever; Fortran took sixty-five years to deprecate implicit
   typing; Clojure's and Go's "never break" left their top complaints
   unfixed for a decade ([javascript.md](../languages/javascript.md),
   [java.md](../languages/java.md), [fortran.md](../languages/fortran.md),
   [clojure.md](../languages/clojure.md), [go.md](../languages/go.md)).
3. **Editions are the synthesis.** Rust's guarantee ("warning-free code on
   edition N compiles on N+1 with the same behaviour") with the "skin deep"
   constraint; Go 1.22's loop-variable fix via the module version gate;
   Dart's per-package language versioning plus an interactive migration
   tool (98% of top packages migrated within a year); Perl's `use v5.NN`
   bundles; PHP's deprecate-then-remove cadence
   ([rust.md](../languages/rust.md), [go.md](../languages/go.md),
   [dart.md](../languages/dart.md)). Per-file feature flags (GHC) produce
   dialects instead ([haskell.md](../languages/haskell.md)).
4. **Preview features with a hard deadline.** Java's nine previews of
   structured concurrency produced a better API; Kotlin's context receivers
   sat "experimental" for three years while production code adopted them,
   then were revoked; D's `-preview` graveyard; Nim's decade-long
   experimental features; Clojure's `spec` alpha. Time-box or make
   previews unusable from published libraries ([java.md](../languages/java.md),
   [kotlin.md](../languages/kotlin.md), [d.md](../languages/d.md),
   [nim.md](../languages/nim.md)).
5. **A specification from the first release.** Rust's decade without one
   was filled by a vendor (Ferrocene) and safety-critical users still feel
   it; Standard ML's frozen Definition could not evolve and became
   unamendable through copyright; Go's and C's specifications are assets;
   WebAssembly shows a mechanised spec can live in the repository
   ([rust.md](../languages/rust.md), [fsharp-sml.md](../languages/fsharp-sml.md)).
6. **Build-flag-dependent semantics destroy library trust.** Nim's
   `--panics`/`--mm`/`-d:danger`, D's `-preview`, Zig's release modes:
   library code must have one meaning ([nim.md](../languages/nim.md)).
7. **A removal mechanism must exist before the first release.** C# proves
   "add only" produces dialects; Python's PEP 594 removals broke twenty-
   year-old code because no cadence existed; Scala 3 removed sixteen
   features because `-rewrite` tooling existed ([csharp.md](../languages/csharp.md),
   [python.md](../languages/python.md), [scala.md](../languages/scala.md)).

### Governance

8. **Coherent designs came from one mind or a tiny group**: Python, Go,
   Rust before 2015, Clojure, Elm, Zig, Elixir, Lua's triumvirate. The
   committee-from-the-start languages (C++ "150 cooks", Scheme's R6RS/R7RS
   split, Ada 83) show the alternative ([cpp.md](../languages/cpp.md),
   [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md),
   [lua.md](../languages/lua.md)).
9. **BDFLs stall, leave, or bottleneck**: Elm's 6.7-year release gap and
   the Gren fork; Python after PEP 572; D's two-person approval bottleneck
   costing its co-designer and a fork (OpenD) within a year; Nim's forks
   (nimskull, Nimony); Vale pausing when its author took a job; Zig's
   praised coherence and criticised opacity ([elm-purescript.md](../languages/elm-purescript.md),
   [d.md](../languages/d.md), [nim.md](../languages/nim.md),
   [vale.md](../languages/vale.md), [zig.md](../languages/zig.md)).
10. **RFC processes need written decision rules.** Rust's two public
    governance failures (2021 mod team resignation, 2023 RustConf
    incident) were about unclear authority, not about the RFC mechanism;
    Go's proposal process (public issues, weekly review, cooling-off week,
    evidence template) and Swift Evolution's mandatory sections are the
    models; PHP's two-thirds votes make additive decisions but never fixed
    function naming ([rust.md](../languages/rust.md), [go.md](../languages/go.md),
    [swift.md](../languages/swift.md), [perl-php.md](../languages/perl-php.md)).
11. **Single-vendor control helps velocity and hurts trust.** Go, Swift,
    Kotlin, C#, Dart are excellent and their roadmaps are visibly the
    sponsor's; F# withered under an ambivalent sponsor; Akka's relicensing
    split Scala's ecosystem; the 2021 .NET hot-reload incident was
    reversed only by public pressure; Ada's single-vendor ecosystem size
    reflects the risk ([csharp.md](../languages/csharp.md),
    [scala.md](../languages/scala.md), [fsharp-sml.md](../languages/fsharp-sml.md)).
12. **Foundations that pay maintainers work**: Lean's FRO, the Rust and
    Python foundations, Perl's; Elixir and Erlang's foundation surveys show
    the community infrastructure it buys ([idris-agda-lean.md](../languages/idris-agda-lean.md),
    [erlang-elixir.md](../languages/erlang-elixir.md)).
13. **Written, ordered goals and proposals with alternatives.** Carbon's
    governance documents, Rust's RFC template, Python's PEPs with rejected-
    alternatives sections, Austral's spec rationale, Roc's written refusals
    ([carbon.md](../languages/carbon.md), [austral.md](../languages/austral.md),
    [roc.md](../languages/roc.md)).
14. **Years of design with no runnable code is its own failure mode**:
    Carbon (four years, 0.1 at end of 2026 at the earliest), Hylo (two
    rewrites, no release), Roc (no 0.1, compiler rewrite), Pony (pre-1.0
    for a decade), Vale, Austral ([carbon.md](../languages/carbon.md),
    [hylo.md](../languages/hylo.md), [roc.md](../languages/roc.md),
    [pony.md](../languages/pony.md)).

## Recommendation for Sisu

Specification before 1.0; a Go-style compatibility promise; three-yearly
editions with Rust's guarantee and `sisu fix` migration; six-month releases
with LTS; previews gated and time-boxed; one meaning per library regardless
of build flags; a removal mechanism from day one; phase-1 small team with a
lead and written decision rules, phase-2 foundation plus language team with
written boundaries, a 90-day proposal time box, and no sponsor majority; a
small, runnable 0.1 early. See
[proposal/13](../../proposal/13-evolution-and-governance.md) and
[proposal/16](../../proposal/16-validation-plan.md); decisions D-036, D-037.

## Open questions

- Q-21 governance details; Q-38 mechanised semantics; the pre-1.0 vs
  post-1.0 breaking-change policy (Clojure chose never, Gleam chose
  pre-1.0 breakage then never) is answered by the edition mechanism but
  the pre-1.0 cadence is not yet stated.
