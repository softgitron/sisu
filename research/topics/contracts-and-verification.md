# Contracts and verification

## The question

How far beyond types a language should go in letting programmers state
and check properties: assertions, contracts, refinement types, dependent
types, external provers.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Assertions only | Most languages | Cheap | Not documented, not in the signature, not usable by tools | [rust.md](../languages/rust.md), [zig.md](../languages/zig.md) |
| Contracts as declaration aspects | Ada 2012, Eiffel, D, C3, C++26 (after removal from C++20) | Executable *and* a proof surface; rendered in documentation; Liskov-checked on overriding | Verbosity if the syntax is heavy; C++ spent a decade on "when to check" and "what is a violation" | [ada-spark.md](../languages/ada-spark.md), [d.md](../languages/d.md), [odin-jai-c3.md](../languages/odin-jai-c3.md), [cpp.md](../languages/cpp.md) |
| Range subtypes and predicates | Ada | Off-by-one and unit errors removed; bounds for the prover | None observed beyond declaration | [ada-spark.md](../languages/ada-spark.md) |
| Refinement types with SMT | Liquid Haskell, Flux (Rust), Dafny, F* | Bounds, arithmetic and ordering invariants with little annotation; what industry uses (AWS Encryption SDK in Dafny, Cedar in Lean) | Solver unpredictability needs fallbacks; no production-scale data on inference staying invisible | [idris-agda-lean.md](../languages/idris-agda-lean.md) |
| SPARK-style proof of absence of runtime errors | SPARK | Proven absence of overflow, bounds and initialisation errors at scale (NVIDIA firmware, rail, avionics) | Preconditions on most non-trivial subprograms; semantics must be prover-friendly from the start (no aliasing, bounded recursion) or the subset stays tiny | [ada-spark.md](../languages/ada-spark.md) |
| Full dependent types | Idris 2, Agda, Lean 4 | Properties in types; Lean's Mathlib shows a large ecosystem is possible | Proof burden, erasure and elaboration cost documented by the designers; the ecosystem exists because of mathematics funding | [idris-agda-lean.md](../languages/idris-agda-lean.md) |
| Runtime contracts in dynamic languages | Clojure `spec`, Racket contracts | Validation at boundaries | `spec` alpha for a decade; sound gradual contracts had a 100x worst case | [clojure.md](../languages/clojure.md), [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md) |
| Property-based testing | QuickCheck lineage, Hypothesis, proptest | Cheap partial verification; purity makes it nearly free | Only as good as the generators | [haskell.md](../languages/haskell.md) |

## What the evidence says

1. **Contracts in the language, in the ordinary expression syntax, are
   mature and cheap.** Ada 2012 made them aspects after decades as
   pragmas; D's designers keep them among the features they would not
   drop; C3 ships them; C++26 finally adopted them. The C++ decade of
   papers (P0542, P2900) catalogues the design questions: check in which
   builds, what a violation is (panic, not UB), whether the optimiser may
   assume them (only when proven) ([cpp.md](../languages/cpp.md)).
2. **Verification is tiered in practice.** Types for structure; SMT-
   checked refinements for arithmetic and bounds; runtime checks where the
   solver cannot decide; an external prover for the rare full proof. This
   is the Cedar / Encryption-SDK pattern and the SPARK assurance ladder
   (Stone to Platinum); nobody uses a dependently typed core for ordinary
   software ([idris-agda-lean.md](../languages/idris-agda-lean.md),
   [ada-spark.md](../languages/ada-spark.md)).
3. **Prover-friendliness must be designed in.** SPARK's subset works
   because Ada's semantics (no aliasing in the subset, bounded recursion,
   checked arithmetic) were close to provable; a language with pervasive
   aliasing and UB cannot add this later. Sisu's ownership and overflow
   rules are coincidentally the SPARK restrictions
   ([ada-spark.md](../languages/ada-spark.md)).
4. **Contracts and semver.** Strengthening a precondition is a breaking
   change; Elm's API-diff semver cannot see behaviour changes, and
   contracts are the part of behaviour that *can* be diffed
   ([elm-purescript.md](../languages/elm-purescript.md)).
5. **Totality checking should be opt-in per function** (Idris 2), and
   required where compile-time evaluation or type-level computation
   depends on termination ([idris-agda-lean.md](../languages/idris-agda-lean.md)).
6. **Bounds-check elision should be proven, not asserted.** Julia's
   `@inbounds` is a documented correctness hazard; indices derived from
   `eachindex`-style protocols or proven by contracts are the safe form
   ([julia.md](../languages/julia.md)).

## Recommendation for Sisu

`requires`/`ensures`/`invariant` as part of signatures, documented and
checked (debug and test fully; release preconditions by default);
refinement subtypes `type X = T where pred`; contract expressions
restricted to pure code; violations are panics; proven contracts feed the
optimiser; `decreases`/loop invariants reserved; a verifier as later
tooling. See [proposal/07](../../proposal/07-contracts.md); decision D-028.

## Open questions

- Q-16 ghost code, loop invariants, the verifier; whether a SPARK-grade
  verifiable subset should be a stated 1.0 goal (the Ada research says
  "decide semantics now or the subset stays tiny"; the proposal's
  semantics are compatible but the goal is not committed).
