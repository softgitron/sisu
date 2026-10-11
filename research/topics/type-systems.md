# Type systems

## The question

Static or dynamic; sound or convenient; how much inference; which
primitives (sums, products, optionals, refinements) are in the language
rather than the library.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Dynamic | Python, Ruby, JS, Clojure, Erlang/Elixir, Lua, Smalltalk, Lisp | Fast start, REPL-driven work, flexible data | No static guarantees; every one of these later added or is adding a type retrofit; performance ceiling (~1.5x from five years of Faster CPython) | [python.md](../languages/python.md), [erlang-elixir.md](../languages/erlang-elixir.md), [ruby.md](../languages/ruby.md), [clojure.md](../languages/clojure.md) |
| Optional / gradual | Dart 1, TypeScript, Python typing, Elixir set-theoretic types, Typed Racket | Migration path for an existing ecosystem | Dart reversed it at a major version; `any` becomes permanent; sound gradual typing had a 100x worst case in Typed Racket | [dart.md](../languages/dart.md), [typescript.md](../languages/typescript.md), [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md) |
| Static, unsound by design | TypeScript, Java (array covariance), Eiffel | Adoption; interop with a dynamic host | Runtime type errors in typed code; cannot rely on types for codegen | [typescript.md](../languages/typescript.md), [java.md](../languages/java.md) |
| Static, sound, nominal, local inference | Rust, Swift, Kotlin, Go, Carbon, Dart 2+ | Local errors, separate compilation, codegen can trust types (Dart: getter 47 to 3 instructions) | Signatures must be written | [dart.md](../languages/dart.md), [rust.md](../languages/rust.md), [swift.md](../languages/swift.md) |
| Static, sound, global inference | OCaml, Haskell, SML, F#, Crystal | Almost no annotations | Error locality; whole-program compilation (Crystal); communities recommend writing signatures anyway | [ocaml.md](../languages/ocaml.md), [haskell.md](../languages/haskell.md), [crystal.md](../languages/crystal.md) |
| Dependent | Idris, Agda, Lean | Properties in types | Proof burden, elaboration cost; ecosystems exist only with research funding | [idris-agda-lean.md](../languages/idris-agda-lean.md) |

## What the evidence says

1. **Every successful dynamic language has become a typed one or is
   trying to.** Python (PEP 484 onward, 83% of newcomers use hints per
   the 2025 typing survey), Ruby (RBS/Sorbet, stalled), JavaScript
   (TypeScript, the largest typed population on earth), Elixir
   (set-theoretic types, four years in and still without signatures),
   PHP (typed properties, enums), Lua (Luau at Roblox). Retrofits take a
   decade and remain partial. The research found no counter-example.
2. **Optional typing was tested industrially and reversed.** Dart 1's
   optional types were replaced by sound types in Dart 2 at the cost of a
   major version, with the measured benefit that the compiler could rely on
   them ([dart.md](../languages/dart.md)). Gilad Bracha's pluggable-types
   position has this one data point against it and none for it.
3. **Unsoundness is tolerated but not loved.** TypeScript users do not list
   unsoundness as a top pain point, but the ecosystem had to invent a
   validator-library genre (zod, valibot) because types do not exist at the
   boundary ([typescript.md](../languages/typescript.md)). Java's array
   covariance is a textbook soundness hole nobody would add today.
4. **The empirical effect of static types on defects is real but
   modest.** Gao, Bird & Barr 2017 found that TypeScript or Flow would have
   caught about 15% of shipped JavaScript bugs; Ray et al. 2014's language-
   vs-defect correlations largely did not survive the 2019 replication
   ([../evidence/studies.md](../evidence/studies.md)). The strong evidence
   is for *specific* guarantees (null safety, memory safety, exhaustiveness),
   not for "static typing" in the abstract.
5. **Inference stops at boundaries in every language that scaled.** The
   OCaml community's `.mli` discipline, Haskell's top-level-signature
   convention, Crystal's inability to do incremental compilation, and the
   independent convergence of Go, Rust, Swift, Kotlin and Carbon on
   "explicit signatures, inferred bodies" are the same finding from five
   directions.
6. **Swift's type checker is the warning about inference power.** Operator
   overloading plus polymorphic literals plus whole-expression bidirectional
   inference produces exponential blow-up; Swift's "expression too complex"
   is the result ([swift.md](../languages/swift.md)). Any two of the three
   are fine.
7. **Sum types, exhaustive matching and non-nullable references are the
   settled shape.** Rust, Swift, Kotlin, Dart, Scala, Haskell, OCaml, Elm,
   Gleam have them; Java added records/sealed/patterns (JEP 395/409/441),
   C# added patterns and is adding unions (C# 15 preview), Go has had an
   open sum-types issue since 2017 with 65% of its users having them in
   their other language ([go.md](../languages/go.md)). No language that has
   them has removed them.
8. **Null is the one mistake every language with it regrets.** Hoare's own
   retrospective; Java's `Optional` failing to displace `null`; C#'s
   warning-only nullable reference types; Kotlin, Swift and Dart showing
   that `T?` with `?.`, `??` and flow typing is the solved form
   ([kotlin.md](../languages/kotlin.md), [swift.md](../languages/swift.md),
   [java.md](../languages/java.md), [csharp.md](../languages/csharp.md)).
9. **Structural vs nominal.** TypeScript's structural typing buys interop
   and loses identity (`UserId` vs `OrderId` need branding hacks); Go's
   structural interfaces cause accidental satisfaction. Nominal types with
   cheap newtypes are what Rust, Swift, Haskell, F# (units of measure) and
   Ada (derived types) use, and the bug class they prevent (unit and ID
   confusion) is concrete.
10. **Refinement types are the practical end of dependent typing.** Ada's
    subtypes with predicates, Liquid Haskell, Flux, Dafny: industry uses
    SMT-checked refinements and contracts, not full dependent types
    ([idris-agda-lean.md](../languages/idris-agda-lean.md),
    [ada-spark.md](../languages/ada-spark.md)).

## Recommendation for Sisu

Static, sound, nominal; local bidirectional inference with explicit
signatures on every item; `enum` + exhaustive `match`; `T?` for optionals;
cheap newtypes with delegation; refinement subtypes; no `any`, no gradual
mode, no dependent types. Numeric literals default early and operators are
not overloaded on return type alone, to stay out of Swift's inference
triangle. See [proposal/03](../../proposal/03-types-and-generics.md);
decisions D-002, D-003, D-010, D-034, D-035, D-043.

## Open questions

- Q-10 higher-kinded types; Q-13 implicit widening vs inference; Q-14
  string unit; Q-16 verification tier; Q-26 anonymous/row-polymorphic
  records (the Haskell records problem and Clojure's open maps argue for
  them; error-message quality argues against).
