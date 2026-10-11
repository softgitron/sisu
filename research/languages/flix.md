# Flix

## Snapshot
| | |
|---|---|
| First appeared | 2016 (PLDI 2016 paper "From Datalog to Flix"); general-purpose language since ~2019 [1]. |
| Designed by | Magnus Madsen (Aarhus University) with Ondřej Lhoták (Waterloo), Matthew Lutze, Jonathan Lindegaard Starup, Jaco van de Pol and students [1]. |
| Current version (Oct 2026) | v0.77.0, 28 Sep 2026 ("Added experimental support for polymorphic effects"); releases every few weeks (0.75.2 Aug 7, 0.75.3 Aug 15, 0.76.0 Sep 13 with GraalVM native-image builds, 0.76.1/2 Sep 16/20) [2]. |
| Governance | Academic open source (Apache-2.0), Madsen as lead; principle "One language: no feature flags or compiler plugins that change the language's semantics" [3]. |
| Typing | Static, HM inference locally but "all functions must have full type signatures"; traits (type classes) with laws; extensible records; polymorphic type-and-effect system with Boolean/set effect formulas [3][4]. |
| Memory | JVM garbage collection; region-scoped mutable memory [5]. |
| Compilation | Compiles to JVM bytecode; native images via GraalVM since 0.76.0 [2]. |
| Primary domains | Research vehicle; "developer productivity over runtime performance" [3]. |
| Notable users | Academic; used in teaching (Aarhus, Japanese FP workshops in 2026). No production users identified. |

## Stated design goals
Flix is "a principled effect-oriented functional, imperative, and logic
programming language". Its principles page is unusually explicit [3]:
"Simple is not easy"; "Everything is an expression"; "Separate pure and
impure code"; "Developer productivity over runtime performance"; "Correctness
over performance"; "Principle of least surprise"; "Local type inference"
(signatures required "for documentation, error blame, and parallel
checking"); "Syntax vs. Semantics" (solve syntactic problems with syntax);
"Human-readable errors"; "Private by default"; "Closed world assumption"
(enables tree shaking); "Illegal states should be unrepresentable"; "Nothing
is executed before main"; "Declare before use"; "No global state"; "Share
memory by communicating"; "Bugs are not recoverable errors"; "Fail fast, fail
hard"; and a long list of refusals: no pre-processor, no null, no implicit
coercions, no reflection, "No warnings, only errors", no unused declarations
or variables, no variable shadowing, no unprincipled overloading, no varargs,
no labelled arguments, exhaustive matches, "Timeless design" (no built-in
JSON/HTML), built-in documentation and unit tests. Library principles: "No
blessed library", "Mutable data is functional data", destructive operations
end in `!` (`Array.reverse!` vs `Array.reverse`). Compiler-message principles:
"Compiler messages are the main interface", "Straight to the Point", "never
blame the programmer" [3].

## The central bets

### A polymorphic effect system with set formulas
Effects are written after `\`: `def divide(x: Int32, y: Int32): Int32 \ DivByZero`,
`\ IO`, `\ {Ask, Say}`; a function without `\` is pure (`\ {}`) [4][6].
Effect variables make higher-order functions effect-polymorphic, and formulas
combine them [4]:

```flix
def map(f: a -> b \ ef, l: List[a]): List[b] \ ef = ...
List.map(x -> x + 1, l)                  // pure
List.map(x -> {println(x); x + 1}, l)    // \ IO

def >>(f: a -> b \ ef1, g: b -> c \ ef2): a -> c \ (ef1 + ef2) = x -> g(f(x))

// Effect exclusion (ICFP 2023, "With or Without You"):
def onClick(listener: KeyEvent -> Unit \ (ef - Block), ...): ...
def recoverWith(f: Unit -> a \ Throw, h: ErrMsg -> a \ (ef - Throw)): a = ...
```

Formulas support union `+`, intersection `&`, complement `~` and difference
`-`; "union is by far the most common" [4]. Unification of these formulas is
Boolean/set unification (OOPSLA 2020; made fast in OOPSLA 2023), which is what
makes `ef1 + ef2` equal to `ef2 + ef1` and removes Koka's ordered-row wrinkles
[1][4]. Sub-effecting (widening) is "not yet enabled" and will apply to
lambdas and instances but not top-level functions [4]. Associated effects
(PLDI 2024) let trait methods carry per-instance effects; polymorphic effects
(effects parameterised by types) arrived experimentally in 0.77.0 [1][2].

### Algebraic effects and handlers
"in the style of Eff and Koka": dynamically scoped, deep handlers, multiple
resumptions; `eff` declares, `run { } with handler E { }` handles, operations
receive `resume`; an operation returning `Void` cannot resume and so acts as
an exception; handler nesting order changes results (the "drunk flip" example)
[6]. Restriction: "Flix does not allow unhandled effects in `new object` or
`spawn` expressions" (a compile-time safety error) [6].

### Regions: scoped mutation that is pure from outside
"All mutable memory belongs to a region tied to its lexical scope":

```flix
def sort(l: List[a]): List[a] with Order[a] =
    region rc {
        let arr = List.toArray(rc, l);
        Array.sort(arr);
        Array.toList(arr)
    }                       // pure from the outside

def toMutDeque(rc: Region[r], l: List[a]): MutDeque[a, r] \ r = ...
```

A region handle is a value that can be passed; mutation inside carries the
region effect `\ r`, which is discharged when the region closes; escaping
data is rejected: "The region variable 'rc' escapes its scope" [5]. This is
the `runST` idea made first-class and ergonomic, and it is how Flix's standard
library implements fast functional operations [5].

### First-class Datalog constraints
Constraint systems `#{ Path(x, y) :- Edge(x, y). ... }` are values with
row-polymorphic types (`#{ ParentOf(String, String) | r }`), composed with
`<+>`, populated with `inject l into Edge/2`, solved with `query ... select`
or `solve ... project`; predicates are type-inferred and stratified negation
is checked (ECOOP 2023) [7]. OOPSLA 2025 "Flix: A Design for Language-
Integrated Datalog" received a distinguished artifact award [1].

### Structured concurrency
`spawn e @ rc` ties a lightweight process to a region: "A region does not
exit until every process spawned in it has finished"; channels (buffered and
unbuffered) with `Sender`/`Receiver` endpoints, `select` with `default` and
`Channel.timeout`; channel operations carry the `Chan` effect and `recv`
additionally `NonDet` [8].

## Strengths

### Set-based effects solve Koka's ordering problems
Because effects are sets with Boolean unification, there is no label order,
no duplicate labels and no `mask`; polymorphism is algebraic (`ef1 + ef2`,
`ef - Throw`) [4]. Effect exclusion expresses "may do anything except X",
which row systems cannot state directly.

### Regions make local mutation free of effect noise
A function that uses arrays internally is simply pure; the region effect never
appears in its signature [5]. This answers the most common objection to pure
languages (quicksort, string builders) without `unsafePerformIO`.

### The debugging escape hatch is designed, not bolted on
The 2025 post on print debugging documents the problem precisely: adding
`println` to a pure function fails with "Unable to unify the effect formulas:
'IO' and 'Pure'"; lying with `unchecked_cast` makes the optimiser delete the
call. The fix is a marker `Debug` effect: if a function fails to typecheck
with its declared effects, "the compiler retries with `Debug` added, but
leaves the function's published signature unchanged"; production mode
disables it; a `d"..."` interpolator reports file and line [9]. Madsen's rule:
"Lying to an effect system has worse consequences than lying to a type
system, because the compiler relies on effect information for dead code
elimination, inlining, and parallelization" [9].

### A written design philosophy and a strict compiler
Principles with rationale plus "no warnings, only errors" and compile errors
for unused declarations and shadowing make the language consistent and
reviewable [3]. The Jan 2026 post on error messages continues the "compiler
messages are the main interface" principle [10].

### Effects used for supply-chain security
"Fighting Supply Chain Attacks with Effect Systems" (OOPSLA 2026, Madsen et
al.) applies the effect system to the problem Austral attacks with
capabilities (see `austral.md`) [1]. Independent convergence on the same use
case is evidence it is real.

## Weaknesses

### Annotation burden moves into signatures; it does not disappear
Every top-level function must be fully annotated, including effect variables
(`\ ef`) on every higher-order function; "requiring a function to be pure
without need is considered a bad programming style, so effect polymorphic
signatures are preferred" [4]. Flix's claim is that inference *inside* bodies
is complete and that signatures are good documentation, not that annotations
are rare. Compared with Effekt's contextual polymorphism (no effect variables
on block parameters), Flix signatures are noisier; compared with Koka, they
are clearer and order-free. The print-debugging post is the admission that
the strictness bites in practice [9].

### Handlers are dynamically scoped
Flix chose Koka-style dynamic scoping [6], so Effekt's accidental-capture
argument applies; the `spawn`/`new object` restriction exists precisely
because unhandled effects cannot be allowed to escape into another process.

### Performance is explicitly secondary and the platform is the JVM
"Developer productivity over runtime performance"; handlers and regions are
compiled to JVM bytecode; no published benchmarks of handler cost; native
image support is new [2][3]. Flix is not evidence about systems-level cost.

### Strictness that users may not accept
No warnings, no unused variables, no shadowing, mandatory signatures: these
are principled but each is a known friction point in other languages (Go's
unused-variable errors are the mainstream data point).

### Many features for a small team
Datalog, effects, regions, traits with laws, records, concurrency, a package
manager, LSP, documentation generator and native images are maintained by one
research group; "experimental" features (polymorphic effects, sub-effecting
"not yet enabled") show the surface is still moving [2][4].

## Designer retrospectives
- "The Principles of the Flix Programming Language" (Onward! 2022) is the
  self-assessment; the principles page is its living form [1][3].
- The print-debugging post: "we have to support print debugging" because
  users need it; users want escape hatches but "nothing must ever break",
  which is the tension the `Debug` effect resolves [9].
- Sub-effecting "not yet enabled" after years indicates that widening effects
  at top level was judged harmful to error blame [4].

## Influence
- **Took from**: Haskell (type classes, purity), OCaml/Scala (syntax, JVM
  interop), Koka and Eff (effects and handlers), Datalog and Soufflé (logic),
  Go (channels and select), `runST` (regions), Elm (error messages).
- **Gave to**: Boolean-unification effect systems are now cited by Scala's
  capture-checking and by the effect-exclusion literature; Flix's region
  design is the most ergonomic published version of scoped mutation and is
  the obvious model for any language wanting "pure outside, mutable inside".

## Evaluation: does Flix solve Koka's ergonomics problems?
Partly. Set formulas with Boolean unification remove ordering, duplication
and `mask`; regions remove the `st<h>`/`run` dance for local mutation;
effect exclusion adds expressiveness. What remains: effect variables on every
higher-order signature (Flix mandates signatures anyway), dynamically scoped
handlers, the debugging collision (solved only with a special-cased effect),
and no evidence on handler performance. Flix shows the *type-level* problems
of Koka rows are fixable; it does not show that a typed effect system is
cheap enough in annotation or runtime for a general audience.

## Lessons for Sisu
- **Adopt**: Set-based effect formulas with union and difference, and
  Boolean-unification-style inference, as the representation of effects in
  Sisu's signatures if Sisu has an effect system at all.
- **Adopt**: Region-scoped mutable memory that is pure from outside, with
  escape checking, as the mechanism for "imperative inside, functional
  outside" and as the natural scope for structured concurrency (`spawn` tied
  to a region).
- **Adopt**: A designed debugging escape hatch (`Debug` effect retried by the
  compiler, disabled in production) rather than `unsafe` casts.
- **Adapt**: Mandatory full signatures on top-level functions are right for
  libraries and for parallel compilation; Sisu can require them for exported
  functions and infer private ones.
- **Adapt**: Datalog as a library-level or compile-time facility rather than
  core syntax; it is a genuine strength for program analysis tools but a
  large surface for a general-purpose language.
- **Avoid**: Dynamically scoped handlers if Sisu adopts handlers; prefer
  lexical scoping (see `koka-effekt.md`).
- **Avoid**: "No warnings, only errors" for unused code and shadowing in the
  default profile; keep them as a strict/CI profile.
- **Open questions raised**: (1) How far does set-based effect inference
  scale to effects parameterised by types and to associated effects without
  undecidable unification? (2) Is the signature burden acceptable to
  application programmers, or only to library authors? Flix has no user data.
  (3) Can regions double as Sisu's isolation domains for concurrency, unifying
  Flix's region effect with Pony/Verona-style isolation?

## Sources
1. M. Madsen, publications list (PLDI 2016; OOPSLA 2020 "Polymorphic Types and Effects with Boolean Unification"; Onward! 2022 "The Principles of the Flix Programming Language"; ECOOP 2023 "Programming with Purity Reflection"; ICFP 2023 "With or Without You: Programming with Effect Exclusion"; OOPSLA 2023 fast Boolean unification; PLDI 2024 "Associated Effects"; OOPSLA 2025 "Flix: A Design for Language-Integrated Datalog"; OOPSLA 2026 "Fighting Supply Chain Attacks with Effect Systems"), https://cs.au.dk/~magnusm/ (2026).
2. Flix releases (v0.75.2-v0.77.0, Aug-Sep 2026), https://github.com/flix/flix/releases.
3. Flix principles, https://flix.dev/principles/ (2026).
4. Flix documentation, "Effect Polymorphism" and effect formulas, https://doc.flix.dev/effect-polymorphism.html (2026).
5. Flix documentation, "Regions", https://doc.flix.dev/regions.html (2026).
6. Flix documentation, "Effects and Handlers", https://doc.flix.dev/effects-and-handlers.html (2026).
7. Flix documentation, "Fixpoints" (first-class Datalog), https://doc.flix.dev/fixpoints.html (2026).
8. Flix documentation, "Concurrency", https://doc.flix.dev/concurrency.html (2026).
9. M. Madsen, "Effect Systems vs Print Debugging: A Pragmatic Solution", https://blog.flix.dev/blog/effect-systems-vs-print-debugging/ (15 Sep 2025).
10. Flix blog, "Making Flix Compiler Errors Helpful and Delightful", https://blog.flix.dev/blog/making-flix-compiler-errors-helpful-and-delightful/ (30 Jan 2026).
