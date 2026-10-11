# OCaml

## Snapshot
| | |
|---|---|
| First appeared | Caml 1985 (INRIA), Caml Light 1990, Objective Caml 1.00 in 1996; renamed OCaml 2011 |
| Designed by | Xavier Leroy (core, native compiler), Damien Doligez (GC, runtime), Jérôme Vouillon (objects), Didier Rémy (row types), Jacques Garrigue (polymorphic variants, labels, relaxed value restriction); OCaml 5 multicore by Dolan, Sivaramakrishnan, Madhavapeddy, White and others (OCaml Labs/Tarides) |
| Current version (Oct 2026) | OCaml 5.5.1 (5 Sep 2026); 5.5.0 (19 Jun 2026); the 4.14 branch is still maintained (4.14.4, 15 Jun 2026) [1] |
| Governance | INRIA-rooted core developer team with an RFC repository; OCaml Software Foundation (2018) funds community work; Tarides and Jane Street are the largest industrial contributors. Jane Street maintains OxCaml, a public fork with substantial extensions [2] |
| Typing | Static, Hindley-Milner with principal types, structural module system with signatures and functors, polymorphic variants and objects via row polymorphism, GADTs; no type classes |
| Memory | Generational tracing GC (minor copying heap, incremental major heap); OCaml 5 runtime has per-domain minor heaps and a parallel major GC |
| Compilation | Native AOT (`ocamlopt`, flambda/flambda2 optimisers), bytecode (`ocamlc`), `js_of_ocaml`/`wasm_of_ocaml`, Melange and ReScript for JavaScript |
| Primary domains | Compilers and proof assistants (Rocq/Coq, Hack, Flow), static analysis (Infer, Frama-C), finance (Jane Street), unikernels (MirageOS), blockchain (Tezos) |
| Notable users | Jane Street, Meta (Infer, Hack type checker, Flow originally), Tezos/Nomadic Labs, Ahrefs, Tarides, INRIA (Rocq), Docker (early tooling), Bloomberg |

## Stated design goals
Leroy's line has always been pragmatism: a strict, impure ML with a fast,
predictable native compiler, a small runtime, and a type system that infers
everything so that programs are "mostly types and no annotations". Objective
Caml added objects to ML in 1996 as an experiment in combining the paradigms.
OCaml 5 had a different, explicitly conservative goal, stated in the PLDI
2021 effect-handlers paper: retrofit parallelism and effect handlers "without
breaking existing code", keeping compatibility with stack-inspecting tools and
imposing "a mean 1% overhead on a comprehensive macro benchmark suite that
does not use effect handlers" [3]. The design was deliberately *not* the most
expressive one available; it was the most retrofit-able one.

## Strengths

### Hindley-Milner inference with principal types
OCaml infers a principal type for every expression; a 1,000-line module can
have no annotations and still be fully statically typed. The compiler is fast
because inference is fast. The cost appears at boundaries: type errors are
reported wherever unification happens to fail rather than where the mistake
was made, and inference is left-to-right so `|>` pipelines type-check where
nested application needs annotations (Syme documents the same phenomenon for
F# [4]). The OCaml culture's answer is the `.mli` interface file: inside a
module you write no types, at the module boundary you write all of them. This
is a better division than "annotate every top-level function" (Rust) or
"annotate nothing" (early Haskell), and it is cheap because `ocaml-lsp` can
generate the `.mli` from the inferred types.

### The most powerful module system in a shipping language
Signatures with abstract types, functors (modules parameterised by modules),
`include`, destructive substitution (`with type t := ...`), first-class
modules (3.12, 2010), and now module-dependent functions ("modular
explicits", 5.5, 2026) [5] give OCaml large-scale parametrisation that no
trait system matches: a `Map.Make(Ord)` functor is a module, so it can export
types, nested modules and values together, and two instantiations with
different orderings are distinct types. Jane Street's codebase is organised
around this. Why nobody copied it fully: the type theory (translucent sums,
sharing constraints, generativity, the interaction with HM inference) is hard
to implement and hard to explain; Rossberg's 1ML (2015) showed modules can be
collapsed into a core language with explicit polymorphism, which is roughly
what Scala 3 and Rust's trait+associated-type systems approximate from the
other direction. F# dropped functors entirely to fit .NET [4]. SML has an
equivalent system (see `fsharp-sml.md`); Standard ML's HOPL authors call
modules "heavily associated" with SML, and the implied type sharing and
`:>` opaque ascription in SML '97 are the mature form that OCaml also has.

### Effect handlers: the first mainstream deployment of algebraic effects
OCaml 5.0 (16 Dec 2022) shipped effect handlers as the primitive for
concurrency, with the manual's own caveats: continuations are one-shot
("every captured continuation must be resumed either with a continue or
discontinue exactly once"; "OCaml does not support multi-shot
continuations"), and effects are not tracked by types ("the compiler does not
statically ensure that all the effects performed by the program are handled";
an unhandled effect raises `Effect.Unhandled` at the `perform`) [6]. OCaml
5.3 (Jan 2025) added dedicated syntax for deep handlers [7]. The PLDI 2021
paper gives the rationale for each restriction: continuations are
represented as call stacks rather than via CPS so that "tools that inspect
the program stack" keep working; one-shot because the motivating use cases
(schedulers, generators, coroutines) never resume twice and one-shot
continuations "can be implemented efficiently"; effects cannot cross C frames
because "the complexity budget outweighs the relatively fewer mechanisms
enabled"; and effect safety is explicitly deferred: "our implementation of
effect handlers in OCaml does not guarantee effect safety. We leave the
question of effect safety for future work" [3]. The pay-off is measured: a
direct-style scheduler written with handlers was the fastest in the
chameneos context-switch benchmark, with a concurrency-monad version 1.67x
and Lwt 4.29x slower [3]. The Eio library builds structured concurrency on
this with no function colouring (no `async`/`await`, no `Lwt.t` in types),
which is the main ergonomic argument for effects over monads.

### Multicore domains with a parallel GC that did not break sequential code
The 5.0 runtime adds domains (OS threads with their own minor heap) and a
stop-the-world parallel major GC. The design goal that sequential programs
keep their performance was met for most programs: "the multicore variants
were less than 1% slower than stock" on geometric mean, with 32 of 54
benchmarks within 5% and 8 above 10% overhead [3]. Memory-model work (Dolan,
Sivaramakrishnan, Madhavapeddy, "Bounding data races in space and time",
PLDI 2018) gives OCaml a local data-race-freedom guarantee: racy programs
have bounded, non-crashing behaviour rather than undefined behaviour. This is
a weaker guarantee than Rust's static freedom but stronger than Go's or Java's
("no tearing, no crash, races stay local").

### Polymorphic variants and row polymorphism
Garrigue's polymorphic variants (1998) give structural, open sum types:
`` `Ok x `` works in any type that lists `` `Ok ``, functions can accept
"at least these tags" or "at most these tags", and the compiler infers the
bounds. They are heavily used for extensible ASTs, error-type unions and
phantom-typed APIs. Rémy's row types also underlie objects. The costs are
known: inferred types like `[> `A | `B ] as 'a` baffle beginners, exhaustiveness
is weaker, and the representation is slower than nominal variants. Still,
OCaml is the only mainstream-ish language where you can union two error
types without declaring a third, and TypeScript's string-literal unions are
the closest imitation.

### Predictable performance from a simple compilation model
Uniform value representation (tagged 63-bit ints, boxed floats except in
float arrays/records), no JIT, straightforward native code, and a GC tuned
for allocation-heavy functional code make OCaml's performance easy to reason
about: Jane Street and the Rocq team can predict where allocations happen.
Flambda and flambda2 (the latter developed in OxCaml) add inlining and
unboxing without changing the model. OCaml compiles large codebases quickly,
which matters more in practice than peak throughput.

### OxCaml: an ML growing Rust-like modes (2023-2026)
Jane Street's "Oxidizing OCaml" series (2023) and the OxCaml release
(14 Jun 2025) add a **mode system** orthogonal to types: "Types describe what
the data *is*, while modes describe how it is *used*", and modes "are
inferred from definitions and checked for consistency" [8]. The documented
axes, each with a "legacy" default that makes all existing OCaml code
well-moded [8]:

| Axis | Values (legacy default in bold) | Purpose |
|---|---|---|
| Locality | `local` / **`global`** | stack allocation; a `local` value may not escape its region |
| Uniqueness | **`aliased`** / `unique` | in-place update, strong update, consuming resources |
| Linearity | `once` / **`many`** | closures callable at most once |
| Portability | **`nonportable`** / `shareable` / `corruptible` / `portable` | whether a value may cross threads |
| Contention | `contended` / `shared` / `corrupted` / **`uncontended`** | how a value is shared between threads |
| Forkability, Yielding, Visibility, Statefulness | see docs | function/stack interaction with effects and closed-over mutable state |

The uniqueness post maps the design onto Rust explicitly: unique resembles a
moved value, exclusive resembles `&mut`, shared resembles `&`; but OCaml's
references "are not parameterized over lifetime variables", and closure
restrictions are modes rather than `FnOnce`/`FnMut` traits "so type inference
is unaffected" [9]. Portability and contention give static data-race freedom
("Data Race Freedom à la Mode", POPL 2025 [10]); "mode crossing" (ICFP 2026)
lets immutable data cross threads with no annotations [11]. OxCaml also ships
unboxed types and layouts, SIMD, a `zero_alloc` checker, labeled tuples,
`include functor`, immutable arrays, and templates [12]. It "makes no promises
of stability or backwards compatibility for its extensions", is "Jane Street's
production compiler", and the stated hope is to "contribute these extensions
to upstream OCaml" [2]. For Sisu this is the single most relevant experiment
in the ML family: an existing GC language acquiring ownership-style control
without lifetimes, with inference, and with legacy code unaffected.

### Tooling that became good late but is now coherent
`dune` (2018, from `jbuilder`), `opam` 2.x, Merlin/`ocaml-lsp`, `ocamlformat`,
`odoc`, `utop`, and the "OCaml Platform" bundle now give a single blessed
path. The 5.5 compiler is relocatable, the Windows story improved with opam
2.2 (2024), and the standard library gained around 60 functions in 5.5 alone
[5]. The 2026 user survey dropped the question "OCaml tooling provides a
comfortable workflow for me", which the organisers attribute to the issue
having been largely addressed [13].

## Weaknesses

### No ad-hoc polymorphism: `print`, `compare` and `equal` cannot be generic
OCaml has no type classes, so there is no `show`, no `==` that is both generic
and user-definable, no `+` that works on floats (`+.`), and no `Hashtbl.hash`
that respects abstraction. The built-in polymorphic `compare`/`=` are
structural runtime magic: they raise at runtime on functions, traverse
cyclic data forever, treat `nan` inconsistently, and are slow. Syme's F#
retrospective says of the feature F# inherited from OCaml: "In retrospect,
the whole generic comparison feature could likely have been omitted from F#,
or greatly constrained" [4]. The community workaround is `ppx_deriving`
(`[@@deriving show, eq, ord]`) and explicit dictionary passing
(`List.sort ~compare:Int.compare`). The fix, **modular implicits** (White,
Bour, Yallop, ML Workshop 2014: "an extension to the OCaml language for ad-hoc
polymorphism", based on "type-directed implicit module parameters" [14]), is
the longest-stalled proposal in the language. Status as of October 2026: not
in the compiler. The groundwork, modular explicits (module-dependent
functions), shipped in 5.5 (June 2026), and the JFLA 2026 paper by Vivien,
Rémy and Scherer describes it as "the underlying language in which modular
implicits... should be elaborated" [5][15]. Twelve years from paper to
groundwork is the clearest evidence in this study that bolting ad-hoc
polymorphism onto a module-centric language after the fact is very hard.

### Untyped effects
Because effects are not in types, the compiler cannot tell you that a
function performs `Yield` or that nobody handles `Abort`; you find out with
`Effect.Unhandled` at runtime. The manual also warns there is "no check to
ensure that the continuations are resumed at least once", and that "not
resuming continuations will leak the memory allocated for the fibers" [6].
Effect-typed research from the same group (Tang, White, Dolan, Hillerström,
Lindley, Lorenzen, "Modal Effect Types", 2024-25) targets exactly the known
cost of effect systems, that "attaching effects to function types gets
verbose for higher-order code", with a modal formulation [16]; the OCaml 5
designers said in 2021 that effect safety was "future work" [3]. No shipping
timeline is public. OCaml therefore demonstrates that untyped handlers are
adoptable and fast, and leaves open whether typed ones can be made
unobtrusive.

### Syntax that two forks were created to replace
`;` versus `;;`, `let ... in` chains, nested `match` requiring `begin ... end`
or parentheses (a classic source of silent mis-association), `-` as both
unary and binary with surprising precedence, `+.`/`*.` for floats, labelled
and optional arguments with `~` and `?`, and `fun`/`function` are all
regularly cited. Reason (Facebook, 2016) re-skinned OCaml with a JavaScript
look; BuckleScript then rebranded as ReScript (10 Aug 2020) with its own
syntax, stating that newcomers were overwhelmed by "the sheer amount of extra
incongruent tools and learning overhead" and that "JavaScript programmers
deserved a great typed language with a fast and lean toolchain" [17]. Two
syntax forks for one semantics is strong evidence that the surface syntax,
not the type system, was the adoption barrier.

### An object system nobody uses
Real World OCaml: "many seasoned OCaml programmers rarely use classes and
objects, if at all"; "the heavy syntax and additional runtime cost means that
objects are rarely used in place of records"; first-class modules "are more
expressive (a module can include types, while classes and objects cannot)";
the recommended use is only "situations where open recursion is a big win"
[18]. The structural object system with row polymorphism is a beautiful piece
of type theory that cost implementation complexity, slower dispatch, and a
name ("Objective") for a feature the community abandoned.

### Value restriction and weak type variables
`let r = ref []` gets type `'_weak1 list ref`, not a polymorphic type, and
`let f = List.map g` (partial application) is likewise monomorphic. Garrigue's
relaxed value restriction (2004) recovers polymorphism when the type variable
is covariant, but the `'_a` surprise still greets every learner. SML's
designers concluded the restriction "was found not to be a significant
limitation in practice" because eta-expansion restores polymorphism [19]; it
is still a trap with no counterpart in languages that require explicit
polymorphism at top level.

### Eighteen years without multicore
In 2002 Leroy argued on the mailing list that shared-memory parallelism was
not worth the GC cost and that processes were the answer; the global runtime
lock persisted until OCaml 5.0 (2022). The multicore project ran from roughly
2014 (OCaml Labs) to the 5.0 merge, eight years. In the meantime two
incompatible monadic concurrency libraries (Lwt, Async) coloured the
ecosystem's functions and split it in two, which is the same "function
colouring" problem Eio was later designed to remove. The lesson is less
about OCaml than about retrofits: a runtime assumption made in year 6 cost a
decade to undo in year 26.

### Tooling and documentation arrived late
Before dune (2018) and opam 2 (2018) OCaml had `ocamlbuild`, OASIS, hand-written
makefiles, and a reputation for being unbuildable on Windows. Survey
free-text from 2022 and 2023 repeatedly mentions "documentation", "debugger"
and "opam/dune"; the 2026 survey added a Debugging & Profiling section because
"debugging difficulties came up often in the 2023 free-text responses" [13][20].
The 2023 results were only published in March 2026, with an apology, which
says something about volunteer capacity.

### A small standard library and a split ecosystem
The INRIA `Stdlib` was minimal for decades, so Jane Street's `Base`/`Core`
became a parallel standard library with different conventions (labelled
arguments, `t`-first), alongside Batteries and Containers. A newcomer must
choose a dialect before writing `List.map`. The 5.x releases have been
closing the gap (`Dynarray`, `String.split_*`, ~60 functions in 5.5 [5]).

### Error messages and adoption
HM inference means the error surfaces far from the cause, and OCaml never
invested in Elm-style message design. Stack Overflow usage sits at 0.8%
(2024) to 1.2% (2025) [21][22]. The 2022 OCaml survey "attracted far less
respondents than the previous one" [20]; the community is stable but not
growing fast.

## Designer retrospectives
- Sivaramakrishnan, Dolan, White et al. (PLDI 2021) on what they left out and
  why: effect safety deferred; one-shot by design; no effects across C
  frames; "our continuations are one-shot, and resuming the continuation
  more than once raises an Invalid_argument exception" [3].
- The OCaml manual's effects chapter (5.3) is itself a list of known holes:
  no static handling guarantee, no at-least-once check, fiber leaks, no
  effects from signal handlers or across `caml_callback` [6].
- Real World OCaml (Minsky, Madhavapeddy, Hickey) on objects: rarely used;
  prefer modules and records [18].
- Jane Street on modes: "only a slice of the features described in this
  series are currently implemented" (2023) [9]; by 2025 the production
  compiler shipped them with "no promises of stability" [2].
- Syme (F# HOPL) on what F# inherited and regretted from OCaml: generic
  structural comparison [4]. Syme also notes that F# removed functors and the
  object system's structural typing to fit .NET, which is an external
  reviewer's verdict on which OCaml features transplant.
- Standard ML's HOPL authors contrast SML's "sweet spot" with "Haskell,
  OCaml, and Scala... that have very rich, but complicated, type systems, and
  so many features that most programmers use only a subset of the language"
  [19].

## Influence
Given: F# is a direct descendant (Syme's HOPL: F# began as "Caml for .NET")
[4]; Rust's first compiler was written in OCaml and the Rust Reference credits
"SML, OCaml: algebraic data types, pattern matching, type inference, semicolon
statement separation" [23]; `|>` entered OCaml's Pervasives in 4.01 (2013)
from F#/Isabelle rather than the other way round [24]; Reason, ReScript and
Melange carry OCaml semantics to JavaScript; Scala's implicits inspired
modular implicits, which in turn inspired nothing shipped yet; OCaml 5's
effect handlers are the reference point every new effect-system language
(Koka, Effekt, Flix, Unison) compares against; Hack, Flow and Infer are
written in it. Polymorphic variants influenced TypeScript-style literal
unions only loosely.

Taken: ML core and modules from Standard ML and Caml; objects from Rémy and
Vouillon's research; GADTs (4.00, 2012) from Haskell/research; effect
handlers from Plotkin-Pretnar, Eff, Koka and Links; the parallel GC design
from the Doligez-Leroy-Gonthier concurrent GC lineage.

## Lessons for Sisu
- **Adopt**
  - Inference inside bodies, signatures at module boundaries (the `.mli`
    discipline), with a tool that generates the signature from inference.
  - Direct-style concurrency on one-shot effect handlers as the runtime
    primitive: it removes function colouring, is faster than monadic
    schedulers, and the implementation strategy (continuations as stacks)
    keeps debuggers and profilers working [3].
  - OxCaml's architecture of modes: orthogonal to types, inferred, with
    legacy defaults so unannotated code is valid, and locality/uniqueness
    separated from lifetimes. This is the most promising route to Rust-like
    control without Rust-like annotation burden.
  - An ML-style module system (signatures, abstract types, functors) for
    large-scale structure, because nothing else lets a parameter carry types
    and values together.
  - A single blessed build/package tool from release one (the dune/opam
    end state), and a standard library large enough that no parallel
    standard library forms.
- **Adapt**
  - Ad-hoc polymorphism must exist from day one (traits/type classes with
    deriving for `show`, `eq`, `ord`, `hash`); modular implicits shows the
    cost of retrofitting it is over a decade. If Sisu also has ML modules,
    define traits as sugar over module-dependent functions (the modular
    explicits design) so the two systems share one elaboration.
  - Effects should be typed from the start, but with the lesson of the
    Modal Effect Types work that naive effect-polymorphic signatures are too
    noisy; aim for inferred, mostly-invisible effect rows with handlers as
    the runtime mechanism OCaml validated.
  - Polymorphic variants: offer structural variants for error unions and
    open ASTs, but with nominal variants as the default and better error
    rendering.
  - Keep structural equality/comparison as *derived*, never as runtime
    magic over the uniform representation.
- **Avoid**
  - Syntax that forces `begin ... end` or parentheses for nested matches,
    separate operators per numeric type, and `;`/`;;` ambiguity; two forks
    were created over this.
  - Shipping two concurrency libraries with incompatible monads; one
    runtime primitive and one structured-concurrency library.
  - A second paradigm (objects) bolted on for completeness; OCaml carried
    "Objective" for 15 years for a feature its experts avoid.
  - Implicit let-generalisation that needs a value restriction; if Sisu
    requires explicit polymorphism at top level (as Rust does), the weak-type
    variable trap disappears.
- **Open questions raised**
  - Complexity budget of modes: OxCaml now documents nine modal axes. Can a
    new language pick three (locality, uniqueness, portability/contention)
    and stop, or does each one turn out to be necessary?
  - Can typed effects be made as ergonomic as OCaml's untyped ones? OCaml's
    choice was deliberate; the counter-evidence must come from Koka/Effekt
    (see those files).
  - Modules versus traits: a language with both risks two ways to do
    everything (OCaml with modular implicits, Scala). Is there a design where
    traits are literally first-class modules with inference, or must one be
    primary?
  - HM-style whole-program inference versus error quality: OCaml maximises
    inference and pays in error messages; is the `.mli` boundary enough, or
    should top-level signatures be mandatory?

## Sources
1. OCaml releases page (5.5.1 2026-09-05, 5.5.0 2026-06-19, 5.4.1 2026-02-17, 4.14.4 2026-06-15). https://ocaml.org/releases (2026)
2. Jane Street, "Introducing OxCaml" (14 Jun 2025). https://blog.janestreet.com/introducing-oxcaml/ (2025); project site https://oxcaml.org/
3. Sivaramakrishnan, Dolan, White, Kelly, Jaffer, Madhavapeddy, "Retrofitting Effect Handlers onto OCaml", PLDI 2021. https://kcsrk.info/papers/retro-concurrency_pldi_21.pdf (2021)
4. Syme, "The Early History of F#", HOPL IV (pipelines §9.1, generic comparison §21.2). https://fsharp.org/history/hopl-final/hopl-fsharp.pdf (2020)
5. OCaml 5.5.0 release notes (module-dependent functions, polymorphic parameters, relocatable compiler, ~60 stdlib functions). https://ocaml.org/releases/5.5.0 (2026)
6. OCaml manual, chapter "Effect handlers" (5.3). https://ocaml.org/manual/5.3/effects.html (2025)
7. OCaml 5.3.0 release notes (syntax for deep effect handlers, 2025-01-08). https://ocaml.org/releases/5.3.0 (2025)
8. OxCaml documentation, "Modes: introduction" (modal axes, legacy defaults, inference). https://oxcaml.org/documentation/modes/intro/ (2025-26)
9. Slater (Jane Street), "Oxidizing OCaml: Rust-style ownership" (21 Jun 2023). https://blog.janestreet.com/oxidizing-ocaml-ownership/ (2023); companion post https://blog.janestreet.com/oxidizing-ocaml-locality/
10. Dolan, Lorenzen, White, et al., "Data Race Freedom à la Mode", POPL 2025 (cited on oxcaml.org) (2025)
11. Lorenzen et al., "Mode crossing", ICFP 2026. https://iris-project.org/pdfs/2026-icfp-modecrossing.pdf (2026)
12. OxCaml documentation index (stack allocation, unboxed types, parallelism/capsules, kinds, uniqueness, SIMD, templates, zero_alloc). https://oxcaml.org/documentation/ (2026)
13. OCaml Platform Newsletter / LWN coverage of the 2026 OCaml Users Survey design (31 Mar 2026). https://lwn.net/Articles/1065519/ (2026); survey announcement https://discuss.ocaml.org/t/ann-ocaml-users-survey-2026/18026
14. White, Bour, Yallop, "Modular implicits", ML Workshop 2014 / arXiv:1512.01895. https://arxiv.org/abs/1512.01895 (2015)
15. Vivien, Rémy, Scherer, "Modular explicits", JFLA 2026. https://gallium.inria.fr/~remy/ocamod/modular-explicits.pdf (2026)
16. Tang, White, Dolan, Hillerström, Lindley, Lorenzen, "Modal Effect Types", arXiv:2407.11816 (v3 Mar 2025). https://arxiv.org/abs/2407.11816 (2024-25)
17. ReScript blog, "BuckleScript is rebranding" (10 Aug 2020). https://rescript-lang.org/blog/bucklescript-is-rebranding (2020)
18. Minsky, Madhavapeddy, Hickey, Real World OCaml (2nd ed.), chapter "Objects". https://dev.realworldocaml.org/objects.html (2022)
19. MacQueen, Harper, Reppy, "The History of Standard ML", HOPL IV (value restriction §4; "sweet spot" §9.4). https://www.cs.tufts.edu/~nr/cs257/archive/dave-macqueen/history-of-ml.pdf (2020)
20. OCaml Software Foundation, "Results of the OCaml User Survey 2022" (discussion thread). https://discuss.ocaml.org/t/ann-results-of-the-ocaml-user-survey-2022/11056 (2022); 2023 results announcement https://discuss.ocaml.org/t/ann-ocaml-users-survey-2023-results/17926 (2026)
21. Stack Overflow Developer Survey 2024 (OCaml 0.8%). https://survey.stackoverflow.co/2024/technology (2024)
22. Stack Overflow Developer Survey 2025 (OCaml 1.2%). https://survey.stackoverflow.co/2025/technology (2025)
23. The Rust Reference, "Influences". https://doc.rust-lang.org/reference/influences.html (2026)
24. OCaml 4.01.0 release notes ("Infix application operators (|>) and (@@) are added to Pervasives", 2013-09-12). https://ocaml.org/releases/4.01.0 (2013)
25. OCaml 5.0.0 release notes (domains, effect handlers, 2022-12-16). https://ocaml.org/releases/5.0.0 (2022)
26. Dolan, Sivaramakrishnan, Madhavapeddy, "Bounding data races in space and time", PLDI (2018)
27. Garrigue, "Relaxing the value restriction", FLOPS (2004)
28. Rossberg, "1ML: Core and modules united", ICFP (2015)
