# Koka and Effekt

Two research languages built around *algebraic effects and handlers*. Koka
(Microsoft Research) is the reference point for typed effect rows, effect
inference and the Perceus memory model; Effekt (University of Tübingen) is the
reference point for *lexical* handlers and "effects as capabilities". They are
treated together because Sisu's effect-system questions need both.

## Snapshot
| | Koka | Effekt |
|---|---|---|
| First appeared | 2012 (Leijen, MSR); v2 with Perceus 2020; v3.0 January 2024 [1][2] | 2020 (OOPSLA paper; earlier Scala library 2017) [10] |
| Designed by | Daan Leijen, with Ningning Xie, Alex Reinking, Anton Lorenzen and others | Jonathan Brachthäuser, Philipp Schuster, Klaus Ostermann, Marius Müller, Matthew Lutze and others |
| Current version (Oct 2026) | v3.2.9, 18 Sep 2026; roughly monthly point releases since 3.0 [3] | v0.82.0, 28 Sep 2026; weekly releases [11] |
| Governance | MSR research project; "Koka v3 is a research language that is currently under development and not ready for production use" [1] | University research project; "Effekt is a research-level language — use at your own risk!" [9] |
| Typing | Static, HM inference extended with effect rows; effects inferred | Static; effects as sets of required capabilities; second-class blocks with capture sets |
| Memory | Perceus precise reference counting with reuse, no GC, no cycle collector [4] | Backend-dependent (JS, Chez Scheme GC, LLVM runtime) |
| Compilation | AOT: Koka → C → native; also JS and wasm targets [1] | Compiler in Scala; backends JS, Chez, LLVM [11] |
| Primary domains | Research; "The main things lacking at the moment are (async) libraries and package management" [1] | Research; case studies (compiler phases, autodiff, probabilistic inference, build systems) [9] |
| Notable users | None; LWN: "No large, notable programs have been written in Koka" [2] | None |

## Stated design goals
Koka: "a strongly typed functional-style language with effect types and
handlers"; the full type of a function should "correspond directly to the
type signature of the mathematical function that describes its denotational
semantics", enabling "a layered semantics where we can easily separate out
nicely behaved parts" [1]. Perceus' goal: "cross this gap" between manual
memory management and GC and "be within 2x of the performance of C/C++" [1].

Effekt: "make effect handlers practical for working programmers, since
existing effect systems are often too complicated"; "effect types express
which capabilities a computation requires from its context" rather than what
side effects it may cause, which "simplifies the treatment of effect
polymorphism" at the acknowledged price that it "impedes reasoning about
purity" [10].

## Mechanism: effects and handlers in Koka

Effect types are rows. `total` (or `<>`) is no effect; `exn` may throw; `div`
may diverge; `pure = <div,exn>`; `ndet`, `console`, `st<h>`, `io` [1]:

```koka
fun square1( x : int ) : total int   { x*x }
fun square3( x : int ) : div int     { x * square3( x ) }
fun square4( x : int ) : exn int     { throw( "oops" ); x*x }
fun square6( x : int ) : _e int      { println("..."); x*x }   // _e: infer it

map : (xs : list<a>, f : (a) -> e b) -> e list<b>              // effect polymorphic
while : ( pred : () -> <div|e> bool, action : () -> <div|e> () ) -> <div|e> ()
```

`<l|e>` extends row `e` with label `l`; at call sites "the effects of
predicate and action are extended automatically until they match", so the
union is inferred [1]. Local `var` state is a built-in handler and escaping
`st<h>` is discharged by a rank-2 `run` (like `runST`) when the heap type is
unreachable from the signature [1].

Handlers define new effects as libraries; "no need anymore to add special
compiler extensions for exceptions, iterators, async-await, probabilistic
programming, etc." [1]:

```koka
effect raise
  ctl raise( msg : string ) : a            // general control operation

fun safe-divide( x : int, y : int ) : raise int
  if y==0 then raise("div-by-zero") else x / y

fun raise-const() : int
  with ctl raise(msg) 42                   // handler: abort with 42
  8 + safe-divide(1,0)                     // evaluates to 42, type is total

effect ask<a>
  ctl ask() : a
fun add-twice() : ask<int> int
  ask() + ask()
fun ask-const() : int
  with ctl ask() resume(21)                // resume once: 42
  add-twice()
fun ask-once() : int                       // resume zero times after fuel runs out
  var count := 0
  with ctl ask()
    count := count + 1
    if count <= 1 then resume(42) else 0
  add-twice()
```

`fun` operations are tail-resumptive by construction and "much more
efficient than general ctl operations": with evidence passing they are called
"directly as a regular function", "a performance cost very similar to virtual
method calls"; general `ctl` must "yield upward to the handler, capture the
stack, and eventually resume again" [1]. `mask<l>` skips the innermost handler
for `l`, which is needed because rows are ordered and duplicate labels are
meaningful [1]. "Linear effects" (never capturing a resumption) avoid the
monadic transformation entirely [1]. The section on named and scoped handlers
in the official book reads "Todo." [1].

## Mechanism: Perceus and FBIP
Perceus (PLDI 2021, distinguished paper) "emits precise reference counting
instructions such that (cycle-free) programs are garbage free, where only live
references are retained", then applies drop specialisation, *reuse analysis*
("pairs pattern matches with constructors of the same size and reuses them
in-place if possible") and reuse specialisation [4]. Because the surface
language has immutable types but the runtime knows uniqueness, `map` over an
unshared list "is updated in-place without any extra allocation", and combined
with tail-recursion-modulo-cons becomes an in-place loop [1]. "Importantly,
the reuse optimization is guaranteed and a programmer can see when the
optimization applies", which is what makes FBIP ("functional but in-place") a
*paradigm*: Okasaki's red-black tree insertion "adapts at runtime to an
in-place mutating re-balancing algorithm", and if the tree is shared it copies
"exactly the shared spine of the tree (and no more)" [1][4]. FP² (ICFP 2023)
formalises a "fully in-place" calculus with the proof that such functions
"require no (de)allocation and use constant stack space" [6]; frame-limited
reuse (ICFP 2022) fixed the finding that "current implementations of reuse
analysis are fragile with respect to small program transformations" and could
cause "an arbitrary increase in the peak heap usage" [5].

Performance evidence [4]: on allocation-heavy benchmarks against OCaml,
Haskell, Swift, Java and C++, "the purely functional Koka implementation is
within 10% of the performance of the in-place mutating algorithm in C++
(using std::map)" on rbtree, with Java at "almost 10× the memory of Koka
(1.7GiB vs. 170MiB)"; "the 'no-opt' version is more than 2× slower"; but "in
benchmarks with lots of sharing, like deriv and nqueens, the optimizations are
less effective" and "we expect a GC to do better when reuse optimization is
not triggered, and there is lots of short-lived object allocation"; forcing
atomic reference counts cost "from 5% (rbtree) up to 59% (nqueens)". The
authors frame the results "mostly as evidence that the Perceus reference
counting technique is viable and can be competitive and not as a direct
comparison". Cycles: "we currently do not supply a cycle collector"; only
mutable `ref` cells can create them and programmers are "on their own" [2][4].

## Mechanism: Effekt's lexical handlers and effects as capabilities

```effekt
effect raise(msg: String): Unit

def divide(n: Int, m: Int): Int / { raise } =
  if (m == 0) { do raise("Cannot divide by zero"); 0 } else { n / m }

try { divide(4, 0) } with raise { msg => println(msg); 42 }

// Contextual effect polymorphism: no effect variable on the block parameter.
def eachLine(file: File) { f: String => Unit / {} }: Unit / {}
eachLine(someFile) { line => do log(line) }   // whole expression: Unit / { Logger }
```

`Int / { raise }` lists what the computation *requires from its context*.
`eachLine`'s signature "says nothing about effects": the block's effects "are
handled at the call site", in its *lexical* scope, so higher-order functions
need no effect variables [9][12]. The price is that functions (blocks) are
second-class: they "can be passed but not returned"; to store or return one
you must `box` it, and the boxed type carries a *capture set*,
`() => Unit at {io}`, which is checked for scope on `unbox`; `at {}` means
"must be pure" [13]. "Effects are requirements: callers must still provide the
capabilities they need... Captures are restrictions: they fix where a
computation can be used" [13]. This is the same box/unbox and capture-set
machinery as Scala 3's capture checking; the OOPSLA 2022 paper "Effects,
Capabilities, and Boxes" is co-authored with Aleksander Boruch-Gruszecki of
the Scala capture-checking project [10]. Lexical scoping also fixes the
classic dynamic-handler bug: a handler installed inside a higher-order
function cannot accidentally capture the operations of a block passed in from
outside, because the block's capabilities were bound where it was written.

Implementation research shows the cost of the idea: capability-passing style
(ICFP 2020), typed CPS translation (PLDI 2022), "From Capabilities to Regions"
(OOPSLA 2023), "Multiple Resumptions and Local Mutable State, Directly" (ICFP
2025), a tracing JIT for handlers (OOPSLA 2025) and monomorphisation (OOPSLA
2025) are all about making lexical handlers fast [10][11].

## Strengths

### One mechanism for exceptions, generators, async, state, nondeterminism
Both languages demonstrate exceptions (`ctl` without `resume`), readers/
state (`fun`/`resume` once), generators and backtracking (`resume` many
times) as libraries, composable with defined interaction semantics [1][9].
This is adoption-as-evidence: OCaml 5 (2022) added handlers to the runtime to
implement fibres and async; Unison's abilities are the same construct in
production use (see `unison.md`).

### Effect inference removes most annotation (Koka)
Effects are inferred bottom-up; `_` asks the compiler to fill in; LWN notes
this "contrasts with Java's checked exceptions", whose pain came from no
inference and no polymorphism [2]. Effect polymorphism of `map` is free.

### A memory model that is neither tracing GC nor ownership (Koka)
Perceus gives deterministic, garbage-free reclamation, in-place updates of
immutable data when unique, no runtime system ("Koka programs can readily
link with other C/C++ libraries"), and a reasoning rule (FBIP) programmers
can rely on [4].

### Lexical handlers are simpler to reason about (Effekt)
No ordering of rows, no `mask`, no accidental capture; higher-order functions
need no effect variables [9][12].

## Weaknesses

### Signature noise and inference surprises (Koka)
Recursion the checker cannot prove terminating acquires `div` ("the type
inference engine is currently not powerful enough to prove that this recursive
function always terminates") [1]; rows are ordered lists reordered during
specialisation, duplicates are meaningful and `mask` exists to skip handlers;
LWN calls these "a few complexities and wrinkles, including order-sensitivity
between effects" [2]. Local `var` and heap `ref` behave differently under
multiple resumption, and "Effect ordering can make these differences
observable or invisible" [2].

### Purity reasoning and first-class functions (Effekt)
The designers concede effects-as-capabilities "impedes reasoning about
purity" [10]. Second-class blocks mean closures cannot be returned or stored
without `box` and a capture set, moving Rust-like lifetime reasoning into the
capture system.

### Handler performance is a research programme, not a solved problem
General `ctl` operations capture the stack; Koka compiles them through
evidence passing and a monadic transformation and offers `fun`/`val`/linear
effects to opt out [1]. Effekt needed six papers on compilation strategies
[10]. OCaml 5 sidestepped the cost by making continuations one-shot:
"OCaml does not support multi-shot continuations" and "Attempting to use a
continuation more than once raises a Continuation_already_resumed exception"
[7]. Flix reports that even *print debugging* collides with a strict effect
system (see `flix.md`).

### No cycle collection, and reuse is workload-dependent (Koka)
Cycles via `ref` leak; reuse analysis helps allocation-heavy functional code
and is "less effective" with sharing [2][4].

### Ecosystem and syntax (Koka)
"few useful libraries", "idiosyncratic" syntax mixing Haskell, C and Scala
[2]; no package manager [1].

## How OCaml 5 simplified
"Retrofitting Effect Handlers onto OCaml" (PLDI 2021) set requirement R1,
backwards compatibility, and left "the question of effect safety for future
work" [8]. The OCaml 5.3 manual states plainly: "the compiler does not
statically ensure that all the effects performed by the program are handled";
an unhandled effect raises `Effect.Unhandled` at the `perform`; continuations
are one-shot; "there is no check to ensure that the continuations are resumed"
so unresumed continuations can leak; effects cannot cross C callbacks [7].
OCaml thus took the *runtime* mechanism (fibres for async, generators) and
none of the type-level guarantees, which is what let it ship in a 25-year-old
ecosystem.

## Production use of typed algebraic effects in 2026
Verified: Unison's abilities are typed algebraic effects and underpin Unison
Cloud, a commercial product (open-sourced October 2026) [14]. Koka and Effekt
both self-describe as not production-ready [1][9]. Flix is academic with
industrial-style tooling (see `flix.md`). OCaml 5 effects are used in
production libraries (Eio) but are untyped. Scala 3's capture checking is
experimental (covered by the Scala researcher). No mainstream language ships
typed effect handlers; the typed variants have one production user, Unison,
whose whole stack was built around them.

## Designer retrospectives
- Leijen et al.: "FBIP is still active research. In particular we'd like to
  add ways to add annotations to ensure reuse is taking place" [1]; Perceus
  results are "evidence that the ... technique is viable" rather than a
  performance ranking [4]; Swift's reference counting "seems less effective
  than Koka" because "the static guarantees of the language are not strong
  enough" for the optimisations [4].
- Brachthäuser et al.: Effekt "impedes reasoning about purity" and trades it
  for simpler polymorphism [10]; the language "is very likely to change" [9].
- Sivaramakrishnan et al. (OCaml): typed effects were deliberately dropped
  for backwards compatibility [8].

## Influence
- **Took from**: Plotkin-Pretnar-Power algebraic effects, Eff, Frank (Koka's
  row types from Leijen's earlier work on extensible records), Lean's reference
  counting (Perceus is "closely based on" Ullrich and de Moura's algorithm)
  [4]; Effekt from capability-passing in Scala and Koka's evidence passing
  (joint papers with Leijen) [10].
- **Gave to**: OCaml 5 handlers (same research community; "Effect Handlers, Evidently"
  is co-authored by Leijen and Hillerström), Unison abilities, Flix effects ("in
  the style of Eff and Koka"), Scala capture checking (Effekt's boxes), Roc's
  and Lean 4's reference-counting-with-reuse (Lean's algorithm, extended by a joint Lean/Koka team),
  Perceus-style RC in Roc (see `roc.md`).

## Lessons for Sisu
- **Adopt**: Effects as *inferred* types with polymorphism built in from day
  one. Checked exceptions failed for lack of inference and polymorphism, both
  solved here; any Sisu effect system must infer `map`'s effect.
- **Adopt**: Tail-resumptive (`fun`) operations as the common, cheap case and
  a separate, visibly more expensive multi-shot case. Most effects in practice
  (reader, state, logging, I/O) are tail-resumptive; make that the fast path
  and keep full continuations optional (or one-shot, as OCaml did).
- **Adopt**: Lexical (Effekt-style) handler scoping, so that a block's effects
  are handled where it was written. It removes Koka's ordering/`mask` wrinkles
  and the accidental-capture bug class.
- **Adapt**: Perceus-style precise RC with reuse is attractive for Sisu's
  *immutable* data; it should be combined with MVS/ownership for mutable data
  rather than used alone, since RC without reuse loses to GC on sharing-heavy
  code and cycles need a separate story.
- **Adapt**: Effekt's second-class blocks plus `box`/capture sets is the
  honest way to keep effects sound with closures, but it must be presented as
  "closures that escape declare what they capture", not as a theory of boxes.
- **Avoid**: Ordered effect rows with duplicate labels and `mask`; set-based
  effects (Flix) or lexical capabilities (Effekt) are both better.
- **Avoid**: Shipping a strict effect system without an escape hatch for
  debugging and FFI (`unsafe-total` in Koka, Flix's `Debug` effect); Sisu
  needs these designed, not bolted on.
- **Open questions raised**: (1) Is a typed effect system worth its
  signature cost for application programmers, given that the only production
  user built a closed world around it? (2) Can handler performance be made
  predictable enough for systems code without the "linear effect" opt-out
  becoming the de-facto default? (3) Should Sisu track purity (Koka) or
  requirements (Effekt)? They are not the same system, and Effekt admits it
  gives up purity reasoning.

## Sources
1. Koka book (effect types §3.2, handlers §3.4, Perceus §2.4-2.5, FBIP §3.5, news with release dates), https://koka-lang.github.io/koka/doc/book.html (2026).
2. LWN, "Koka: a functional language with effect types" (Aug 2025), https://lwn.net/Articles/1033050/.
3. Koka releases (v3.2.9), https://github.com/koka-lang/koka/releases (2026).
4. A. Reinking, N. Xie, L. de Moura, D. Leijen, "Perceus: Garbage Free Reference Counting with Reuse", PLDI 2021 / MSR-TR-2020-42, https://www.microsoft.com/en-us/research/uploads/prod/2020/11/perceus-tr-v4.pdf.
5. A. Lorenzen, D. Leijen, "Reference Counting with Frame Limited Reuse", ICFP 2022, https://www.microsoft.com/en-us/research/publication/reference-counting-with-frame-limited-reuse/.
6. A. Lorenzen, D. Leijen, W. Swierstra, "FP²: Fully in-Place Functional Programming", ICFP 2023, https://www.microsoft.com/en-us/research/publication/fp2-fully-in-place-functional-programming/.
7. OCaml 5.3 manual, "Effect handlers", https://ocaml.org/manual/5.3/effects.html (2025).
8. K. C. Sivaramakrishnan, S. Dolan, L. White, T. Kelly, S. Jaffer, A. Madhavapeddy, "Retrofitting Effect Handlers onto OCaml", PLDI 2021.
9. Effekt website (overview, status), https://effekt-lang.org/ (2026).
10. Effekt publications page (Effects as Capabilities, OOPSLA 2020; Effects, Capabilities, and Boxes, OOPSLA 2022; compilation papers 2020-2025), https://effekt-lang.org/publications; abstract at https://pl.cs.uni-tuebingen.de/publications/brachthaeuser20effects/.
11. Effekt releases (v0.78.0-v0.82.0, Aug-Sep 2026), https://github.com/effekt-lang/effekt/releases.
12. Effekt docs, "Effect Polymorphism", https://effekt-lang.org/docs/concepts/effect-polymorphism (2026).
13. Effekt tour, "Captures", https://effekt-lang.org/tour/captures (2026).
14. P. Chiusano, "Unison Cloud is now open source", https://www.unison-lang.org/blog/unison-cloud-open-source/ (8 Oct 2026); LWN, "Unison: a new language with content-addressed code" (2024), https://lwn.net/Articles/978955/.
