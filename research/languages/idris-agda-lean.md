# Idris 2, Agda and Lean 4

Three dependently typed languages that answer the same question
differently: Agda is a proof assistant that happens to run programs, Idris 2
is a programming language that happens to prove things, and Lean 4 is both,
with the largest ecosystem any dependently typed system has ever had. The
section that matters most for Sisu is the last one: what the lightweight
alternatives (refinement types, contracts, SMT-backed verification) buy
without full dependent types.

## Snapshot
| | Idris 2 | Agda | Lean 4 |
|---|---|---|---|
| First appeared | Idris 1 2009; Idris 2 announced 2020 (self-hosting rewrite on QTT) [1] | Agda 1 1999 (Coquand); Agda 2 2007 (Norell) | Lean 2013 (Lean 1-3); Lean 4 rewrite announced 2021, stable 4.0.0 Sept 2023 |
| Designed by | Edwin Brady (St Andrews) | Ulf Norell, Andreas Abel, Jesper Cockx and the Agda team (Chalmers/Gothenburg and others) | Leonardo de Moura (Microsoft Research, then AWS, then Lean FRO) and Sebastian Ullrich; Lean FRO team |
| Current version (Oct 2026) | 0.8.0 (31 Oct 2025, "Hallowe'en release"); the project site warns it is "severely out of date" [2] | 2.8.0 (July 2025) with point releases to 2.8.0.2 (13 Sep 2026); standard library 2.x [3][4] | v4.34.1 (24 Sep 2026); v4.35.0 release candidates in progress [5] |
| Governance | Brady plus GitHub contributors; no foundation | Academic volunteer team; GitHub | Lean FRO (non-profit under Convergent Research, founded July 2023 by de Moura and Ullrich; "more than ten employees"); funders include the Sloan, Simons and Richard Merkin foundations, Alex Gerko ($10M gift in 2025, half to the FRO) and Amazon (2026, "the single largest donation") [6][7] |
| Typing | Full dependent types over Quantitative Type Theory (multiplicities 0/1/ω); interfaces (type classes); optional totality checking | Full dependent types (Martin-Löf with universes, inductive families, cubical option); runtime-irrelevance annotations since 2.6.1 [8] | Full dependent types (CIC-style with `Prop`), type classes with out-params, universe polymorphism; `partial`/`unsafe` escape hatches |
| Memory | Backend GC (Chez Scheme default; also Racket, JavaScript, RefC reference counting) | GHC's GC via the MAlonzo backend; JavaScript backend | Reference counting with reuse and borrowing ("Counting Immutable Beans"), compiled to C [9] |
| Compilation | Compiles to Scheme/JS/C; interactive editing via holes | Type-check-centric; compile to Haskell then native; interactive via Emacs/VS Code | Self-hosted (about 90% of Lean is written in Lean [10]); compiles to C; new compiler pipeline default since v4.22.0 (Aug 2025) [11] |
| Primary domains | Research on dependently typed programming; teaching; small verified programs | Research in type theory, homotopy type theory, formalised mathematics, PL metatheory | Formalised mathematics (Mathlib), verified software (AWS Cedar, SampCert), AI-assisted theorem proving, general-purpose programming (Lean's own compiler and tooling) |
| Notable users | Academic; Brady's book *Type-Driven Development with Idris* | Academic (Agda stdlib, cubical library, PLFA textbook) | Mathlib community (772 contributors, 289,891 theorems, 137,938 definitions as of Oct 2026 [12]; "over 1.5 million lines" [10]); AWS (Cedar authorization language formalised in Lean [13]); Google DeepMind (AlphaProof, 2024) |

## Stated design goals
**Idris 2**: "Dependent types allow us to express precisely what a function
is intended to do"; the new core is QTT because "a limitation of Idris 1 is
that it is not always clear which arguments to functions and constructors are
required at run time, and which are erased", and "QTT allows us to state
clearly, in a type, which arguments are erased" [1, §1]. Pragmatically,
"Idris 1 has outgrown the requirements of its initial experimental
implementation" [1]. **Agda** is a research vehicle for intuitionistic type
theory; its goals are expressiveness and trust (the `--safe` flag), not
deployment. **Lean 4**: "combine the high level of trust provided by a small,
independently-implementable logical kernel" with automation while scaling to
large problems; "improved automation, improved performance, and
user-friendliness" are the FRO's stated priorities [10]; de Moura's stated
aim since 2021 is that Lean 4 is a general-purpose programming language whose
first large program is Lean itself.

## Strengths

### What dependent types buy in practice
Three patterns recur in every Idris/Agda/Lean codebase and are the realistic
payoff for a general-purpose language:
- **Indexed data**: `Vect n a`, `append : Vect n a -> Vect m a -> Vect (n+m) a`;
  out-of-bounds and length-mismatch bugs become type errors, and the index
  arithmetic is checked by normalisation [1, §3].
- **State machines in types**: Idris's `ST`/`Control.Linear` and Brady's
  session-type example encode protocols so that misuse (writing to a closed
  file, sending out of order) is rejected at compile time; with QTT's
  multiplicity 1 this becomes "resource tracking in the type system leading
  to type-safe concurrent programming with session types" [1, abstract].
- **Totality**: a function proven total (structurally recursive, covering)
  can be run at type-checking time and trusted as a proof. Idris 2 makes it
  opt-in per module (`%default total`), Agda defaults to total with
  `{-# TERMINATING #-}` escape, Lean requires `partial` for non-total
  definitions. Opt-in totality is the setting that survived contact with
  programmers.

### Idris 2: Quantitative Type Theory as a practical linear/dependent fusion
QTT (Atkey, LICS 2018, "a Type Theory that records usage information for each
variable in a judgement", building on McBride's "I Got Plenty o' Nuttin'")
gives every binder a multiplicity: 0 (erased, usable only in types), 1
(used exactly once), or ω (unrestricted) [14]. In Idris 2 "a function which
takes an argument with multiplicity 1 promises that it will not share the
argument in the future; there is no requirement that it has not been shared
in the past" [1, §3]. Brady's assessment after building a full language on
it: "For most day to day programming tasks, expressing erasure at the type
level is the most valuable user-visible new feature enabled by QTT, in that it
is unambiguous which function and data arguments will be erased at run time.
Erasure has been a difficulty for dependently typed languages for decades"
[1, §7]; the 0 multiplicity "provides the cleanest solution to the erasure
problem, although we no longer infer which other arguments can be erased"
[1, §6]. Agda independently adopted the same `@0` erasure annotations in
2.6.1, citing McBride and Atkey [8]. For Sisu the relevant fact is that
linearity annotations on binders, orthogonal to the type, were implemented
in a full language with type classes, holes and an interactive editor, and
that the authors found the 1 multiplicity useful for resource protocols,
not just as theory. Idris 2 also demonstrated that a dependently typed
compiler can bootstrap in "around 90 seconds" (2021) [1, §7].

### Lean 4: the first dependently typed language with a large ecosystem
Mathlib alone has 289,891 theorems from 772 contributors [12], grown from one
million lines at the end of Lean 3 to over 1.5 million [10]; it is the only
software artifact in this study that proves a dependently typed language can
sustain a community the size of a mid-sized mainstream language's. The
drivers are not the type theory but the engineering around it: the FRO's
paid team, a fast self-hosted compiler, `lake` build tool and package
manager, cached builds (`lake exe cache`), an LSP with live goal display, and
a culture of library review. The language also carries a company-grade use:
AWS formalised Cedar in Lean and runs "differential randomized testing (DRT)
between the formalization and Rust production implementation" [13]; the FRO
page lists Amazon as its largest donor [6].

### Lean 4: macros, elaborators and `do`
Lean 4's syntax is extensible from user code: `syntax`/`macro_rules` declare
hygienic macros, `elab` rules run arbitrary Lean code at elaboration time,
and the standard notation (`∀`, `∑`, `do`, `calc`, tactics) is defined this
way rather than in the parser. `do`-notation includes mutable locals
(`let mut`), `for` loops, early `return`, `try`/`catch` and nested actions,
compiled to pure monadic code; the result reads like Python inside a pure
language, which is how most of the compiler is written. This is the most
complete "same-language metaprogramming" system shipped outside Lisp and
Racket, and it is typed and hygienic.

### Lean 4: Perceus-style reference counting with reuse
"Counting Immutable Beans" (Ullrich, de Moura, IFL 2019) describes Lean's
runtime: reference counting where unshared values are updated in place
("destructive updates"), borrowed references avoid count updates, borrow
annotations are inferred heuristically, and the implementation "supports
multi-threading"; "preliminary results suggest the approach is competitive
with leading compilers and often performs better" [9]. The same idea,
formalised as Perceus ("garbage free" precise counting, reuse analysis
guaranteeing in-place updates, "functional but in-place" programming) was
proven sound in Koka (Reinking, Xie, de Moura, Leijen, PLDI 2021) [15] and
adopted by Roc. Viability evidence: Lean's compiler, Mathlib's tactic
framework and `lake` all run on it; the known costs are cycle leaks (absent
in immutable data, present with closures over mutable refs), atomic count
updates when objects are shared across threads (Lean marks objects as
single- or multi-threaded to avoid atomics on the common path), and
throughput loss versus tracing GC in allocation-heavy code with long-lived
shared structures. The benefit that matters for a systems-ish language is
determinism: no pauses, memory freed at last use, and in-place updates of
"immutable" arrays when uniquely owned.

### Lean 4: a fast compiler for a dependently typed language
Lean 4 "is faster and scales to larger problems than Lean 3" [10]; the
August 2025 release (v4.22.0) stated that "the old compiler has been replaced
by the new compiler" and shipped the SMT-style `grind` tactic [11]. Type
checking of a Mathlib file is seconds; a full Mathlib build is hours, but the
cache makes that rare. The engineering lesson is that dependent type
checking is tractable when definitional unfolding is controlled (reducibility
attributes, `irreducible`, `simp` sets) and elaboration is incremental.

### Agda: the research instrument
Agda's value to this survey is as the place where ideas are tried first:
cubical type theory, sized types, copatterns, instance arguments (its name
for type-class resolution), erasure annotations, and the `--safe` and
`--without-K` flags that make the trusted core explicit. PLFA (Wadler, Kokke,
Siek) made it the textbook language for programming-language metatheory.

## Weaknesses

### The proof burden
Every precise type is a proof obligation. `append` on vectors needs `n + m`
to normalise; `reverse` needs `n + 0 = n` and `n + (m + 1) = (n + m) + 1`,
which do not hold definitionally, so the programmer writes or finds a
rewrite. Real programs accumulate hundreds of such lemmas, and the lemmas are
the part that experienced programmers in non-dependent languages never had
to write. Lean's `omega`, `simp`, `decide` and `grind` automate the common
cases; Idris 2 and Agda have less automation. There is no empirical study of
productivity with dependent types; the evidence that the burden is real is
that every one of these languages invested heavily in automation and
escape hatches (`believe_me`, `postulate`, `sorry`, `partial`).

### Erasure, phase distinction and performance
"Tradtionally, this phase distinction has been clear: types are erased at
run time, values preserved. But this correspondence is merely a coincidence,
arising from the special (non first-class) status of types!" [1, §3]. With
first-class types the compiler must decide what runs; Idris 1's whole-program
erasure analysis "works well in practice but doesn't allow a programmer to
require specific arguments to be erased, and means that separate compilation
is difficult" [1, §6]. Idris 2 (QTT) and Agda (`@0`) solve this with
annotations; Lean erases `Prop` and types but not `Nat` indices unless
unused. The underlying cost remains: a `Vect n a` whose `n` is needed at
runtime is a linked list with a unary natural beside it unless the compiler
is told otherwise.

### Type checking cost and decidability
Type checking with dependent types requires evaluating programs; it is
decidable only when all such programs are total. Agda enforces this
(termination checking is conservative and rejects valid programs), Idris 2
makes it optional (so type checking can diverge on a non-total function used
in a type), and Lean separates `partial` definitions (opaque to the kernel)
from total ones. In practice the bigger cost is not undecidability but slow
elaboration: Agda's interactive checking of large modules takes minutes,
Idris 1 was slow enough that Brady listed performance as a primary reason for
the rewrite [1], and Lean's Mathlib needs a distributed build cache.

### Idris: the island problem
Idris 2's site is self-described as "severely out of date" [2]; releases are
roughly yearly; the package ecosystem (`pack`) is small; the default backend
is Chez Scheme, which limits deployment. The language demonstrated QTT and
type-driven development to the world and then stayed a research artifact.
Brady's 2021 paper already says of session-type protocols that "realistically
it will need to handle timeouts, exceptions and more sophisticated protocols"
[1, §7], and that work did not materialise at scale.

### Agda: no path to deployment
Compilation via GHC is an afterthought, the standard library is designed for
proofs rather than programs, there is no package manager of note, and
performance is unmeasured. This is by design, and it is why Agda is
excluded from Sisu's "adopt" list except as a source of ideas.

### Lean: a mathematics-shaped ecosystem
Mathlib dominates, so library conventions, naming, automation and even
performance work are tuned for proofs. The general-purpose side (networking,
async, FFI, Windows support) is thin; Lean programs in the wild are mostly
Lean tooling, Cedar-style specifications and verification harnesses.
Reference counting without a tracing collector also means that a Lean program
with cyclic mutable structures leaks unless the programmer breaks cycles.

### All three: the learning cliff
Dependent types reward users who already know type theory. Error messages
involve unification failures with metavariables, implicit arguments, and
universe levels. Lean's goal view and `?`-holes make this interactive, which
is the best mitigation anyone has found, but there is no equivalent of Elm's
"Compiler Errors for Humans" for a dependent type checker.

## The key question: lightweight verification without full dependent types
Survey of what exists, with its evidence:

| Approach | Example | What it checks | Cost | Status / evidence |
|---|---|---|---|---|
| **Refinement (liquid) types** | Liquid Haskell (GHC plugin) [16]; Flux for Rust [17] | Predicates over base types (`{v:Int \| v > 0}`, array bounds, ordering, termination), discharged by an SMT solver, with inference of refinements | Annotations on function signatures; SMT unpredictability on non-linear arithmetic; a trusted solver | Liquid Haskell active (a March 2025 post reports "a hundred pull requests"); Flux has a playground and tutorial and "eliminat[es] various classes of run-time problems" at compile time [17]; both are research tools with small user bases |
| **SMT-backed program verifiers** | Dafny [18]; Verus for Rust [19]; F* | Pre/post-conditions, invariants, termination, memory safety beyond the type system; Verus checks "code that goes beyond Rust's standard type system, such as code that manipulates raw pointers" [19] | Specification writing and proof hints; solver timeouts; a separate language or dialect | Dafny verified code ships in the AWS Encryption SDK, transpiled to .NET and Rust ("There is no Dafny runtime") [18]; Verus is "under active development" with "industry and academic projects" using it [19] |
| **Design by contract** | Eiffel, Ada 2012/SPARK, D, Kotlin contracts | Runtime-checked (or SPARK-proved) pre/post/invariants | Runtime cost or SPARK's proof effort | Mature; SPARK is the only one with industrial proof at scale (see `ada-spark.md`) |
| **Property-based testing** | QuickCheck family | Executable properties, no proof | None at compile time | Universal; the cheapest 80% |
| **Formal model + differential testing** | AWS Cedar (Lean model vs Rust implementation) [13] | The production code agrees with a verified model on generated inputs | Two implementations | "Testing can find disagreements but does not by itself prove the implementation correct" [13]; this is the pattern industry actually uses with Lean |
| **Full dependent types** | Idris 2, Agda, Lean | Anything, by proof | Proof burden, erasure, elaboration time | Lean shows the ecosystem can exist; nobody shows it for application code |

Reading of the evidence: the properties people verify in practice are
bounds, non-nullness, ordering, size relationships, resource protocols and
simple arithmetic invariants. Refinement types with SMT cover these with
decidable checking and inference, and they attach to an ordinary type system
without changing its phase distinction; Flux shows they compose with
ownership. Full dependent types are needed for the remaining 10% (indexed
data with real arithmetic, proofs about programs as values) and that 10% is
where the proof burden and erasure problems live. Where the evidence is
thin: there is no controlled study of annotation burden for refinement types
in production code, and SMT-based tools' failure mode (a timeout on a
correct program) has no good user-experience answer yet.

## Designer retrospectives
- Brady (2021): erasure was "handled in partial and unsatisfying ways" for
  decades and QTT's 0 is "the cleanest solution"; the price is losing
  inference of erasability; performance "will be a primary concern in the
  near future" [1, §6-7].
- Atkey (2018): QTT's point is that usage tracking can refine the semantics
  of type theory "by accurately tracking resource behaviour" without
  changing the logic [14].
- The Agda documentation credits McBride and Atkey for its erasure typing
  rules and notes the GHC backend substitutes a placeholder for erased
  higher-order arguments to avoid space leaks [8].
- Ullrich and de Moura (2019) chose reference counting over tracing GC for a
  pure language precisely because it enables destructive updates [9]; the
  Lean FRO's 2025 compiler replacement [11] is the retrospective on the first
  Lean 4 compiler: it was rewritten.
- The Lean reference manual frames Lean 4 as a reaction to Lean 3's limits:
  "faster and scales to larger problems" [10].

## Influence
Given: QTT multiplicities to Agda's erasure annotations [8] and to the
linear-types discussion in Haskell (`LinearTypes`, GHC 9.0, is based on the
same McBride lineage); dependent pattern matching and holes-driven
development (Agda, Idris) to Haskell's typed holes and to Lean; Lean's
`do`-notation with mutation and early return to Koka and Roc discussions;
Perceus/Counting-Immutable-Beans to Koka and Roc; Lean's macro system to the
general demand for hygienic, typed macros in new languages; Mathlib to AI
theorem proving (AlphaProof, and most LLM-based provers target Lean). Taken:
Martin-Löf type theory, Coq's calculus of inductive constructions (Lean),
Epigram's holes and views (Idris), Haskell's syntax and type classes (all
three), Rust-style `lake`/`cargo` project model (Lean).

## Lessons for Sisu
- **Adopt**
  - Multiplicity annotations on binders (Idris 2's `1`, QTT) as the model for
    Sisu's linear/affine resources: orthogonal to the type, checked
    structurally, proven workable in a full language with type classes and
    inference. Combine with OxCaml's mode inference (see `ocaml.md`) so the
    default is unannotated.
  - Opt-in totality checking per function or module (`total` attribute),
    with the compiler refusing to use non-total functions where termination
    matters (constant evaluation, type-level computation).
  - Lean's `do`-notation design: `let mut`, `for`, `return`, `try` inside a
    pure effectful block, desugared to ordinary code.
  - Lean-style hygienic, typed macros and elaborator extensions as the
    metaprogramming story, over template/text-based systems.
  - Perceus-style reference counting with reuse as a candidate memory
    model for the pure/immutable core, with the known caveats (cycles,
    atomic sharing) designed around rather than discovered.
- **Adapt**
  - Verification tiering: types for structure, refinement types (SMT) for
    arithmetic/bounds/ordering invariants with inference, contracts checked
    at runtime where the solver cannot decide, and an export path to an
    external prover (Lean) for the rare full proof. This is the Cedar/
    Encryption-SDK pattern, which is what industry demonstrably uses.
  - Erasure must be explicit when types can mention values; if Sisu has
    compile-time evaluation or `const` generics, adopt a 0-multiplicity-like
    marker rather than whole-program erasure analysis (Idris 1's regret).
  - Indexed types: support length/size indices via `const` generics and
    refinements (Rust-like), not via first-class dependent types.
- **Avoid**
  - Full dependent types as the base type system of a general-purpose
    language. The proof burden, erasure and elaboration-time costs are
    documented by the designers themselves, and Lean's ecosystem exists
    because of mathematics funding and a paid team, not because application
    programmers wanted it.
  - Unbounded type-level evaluation during type checking without a totality
    boundary (Idris 2's optional totality in types).
  - Treating SMT as magic: refinement checking must have predictable
    fallbacks (explicit proof hints or runtime checks) when the solver fails.
- **Open questions raised**
  - Can refinement types be inferred well enough to stay invisible in
    ordinary code? Liquid Haskell and Flux say mostly yes on small corpora;
    no production-scale data.
  - Is reference counting with reuse competitive with a modern tracing GC
    for long-running services with large shared heaps? Lean and Koka
    benchmarks are compute-bound; the server case is untested.
  - Does a dependently typed *core* (as Lean's kernel) with a simple surface
    offer the best of both, or does the core leak into every error message?
    Lean is the live experiment; its mathematics-shaped ecosystem makes the
    answer hard to read for general programming.
  - How much of Idris 2's session-type/resource-protocol story transfers to
    an affine-typed language without dependent types? Rust's typestate
    pattern suggests most of it, with more boilerplate.

## Sources
1. Brady, "Idris 2: Quantitative Type Theory in Practice", ECOOP 2021 / arXiv:2104.00480. https://arxiv.org/abs/2104.00480 (2021)
2. Idris 2 releases on GitHub (v0.8.0, 31 Oct 2025; v0.7.0, 22 Dec 2023); project news page with "severely out of date" notice. https://github.com/idris-lang/Idris2/releases ; https://www.idris-lang.org/category/news.html (2025-26)
3. Agda releases on GitHub (v2.8.0, v2.8.0.1, v2.8.0.2). https://github.com/agda/agda/releases (2026)
4. Hackage, Agda package (2.8.0.2 uploaded 2026-09-13). https://hackage.haskell.org/package/Agda (2026)
5. Lean 4 releases on GitHub (v4.34.1 "Latest", 24 Sep 2026; v4.33.1 21 Aug 2026). https://github.com/leanprover/lean4/releases (2026)
6. Lean FRO, "About" (founded July 2023; funders; Amazon 2026 donation). https://lean-lang.org/fro/about/ (2026)
7. Wikipedia, "Lean (proof assistant)" (stable release 4.33.1, Aug 2026; developer Lean FRO). https://en.wikipedia.org/wiki/Lean_(proof_assistant) (2026)
8. Agda documentation, "Run-time irrelevance" (erasure annotations since 2.6.1; cites McBride and Atkey). https://agda.readthedocs.io/en/latest/language/runtime-irrelevance.html (2026)
9. Ullrich, de Moura, "Counting Immutable Beans: Reference Counting Optimized for Purely Functional Programming", IFL 2019 / arXiv:1908.05647. https://arxiv.org/abs/1908.05647 (2019)
10. The Lean Language Reference, "Introduction" (Mathlib over 1.5 million lines; ~90% of Lean written in Lean; FRO more than ten employees). https://lean-lang.org/doc/reference/latest/Introduction/ (2026)
11. Lean v4.22.0 release notes ("The old compiler has been replaced by the new compiler"; `grind`). https://lean-lang.org/doc/reference/latest/releases/v4.22.0/ (2025)
12. Mathlib statistics page (137,938 definitions; 289,891 theorems; 772 contributors). https://leanprover-community.github.io/mathlib_stats.html (Oct 2026)
13. cedar-policy/cedar-spec README (Lean formalisation of Cedar; differential randomized testing against the Rust implementation). https://github.com/cedar-policy/cedar-spec (2026)
14. Atkey, "The Syntax and Semantics of Quantitative Type Theory", LICS 2018. https://bentnib.org/quantitative-type-theory.html (2018)
15. Reinking, Xie, de Moura, Leijen, "Perceus: Garbage Free Reference Counting with Reuse", PLDI 2021 (MSR-TR-2020-42). https://www.microsoft.com/en-us/research/publication/perceus-garbage-free-reference-counting-with-reuse/ (2021)
16. Liquid Haskell project site (GHC plugin; "A hundred pull requests for Liquid Haskell", Mar 2025). https://ucsd-progsys.github.io/liquidhaskell/ (2025)
17. Flux: refinement types for Rust (project site, playground, tutorial). https://flux-rs.github.io/flux/ (2026); Lehmann et al., "Flux: Liquid Types for Rust", PLDI (2023)
18. aws/aws-encryption-sdk-dafny README (Dafny verification; transpiled to .NET and Rust; "There is no Dafny runtime"). https://github.com/aws/aws-encryption-sdk-dafny (2026)
19. verus-lang/verus README ("under active development"; "industry and academic projects use Verus"). https://github.com/verus-lang/verus (2026)
20. McBride, "I Got Plenty o' Nuttin'", in A List of Successes That Can Change the World (2016)
21. Wadler, Kokke, Siek, Programming Language Foundations in Agda. https://plfa.github.io/ (2018-)
22. de Moura, Ullrich, "The Lean 4 Theorem Prover and Programming Language", CADE 28 (2021)
