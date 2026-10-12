# Incremental, reversible and exotic paradigms

## Why this family matters for Sisu

Most of these paradigms need something mainstream languages lack and Sisu
has: a checked `pure`, capabilities as the only source of non-determinism,
and an interpreter owning every yield point and capability call
([proposal/06](../../proposal/06-concurrency-and-capabilities.md),
[proposal/18](../../proposal/18-next-steps.md)). Per paradigm: language
feature, library those properties make easy, or dead end? The sharpest case
is `@memo` on `pure` functions: Skip failed as a language, Salsa and
Incremental succeed as libraries.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Self-adjusting computation (Acar et al.); Adapton | 2002-2009; 2014- | Research; ideas live in libraries | Dependence graphs and change propagation; demand-driven variant |
| Jane Street Incremental, Bonsai | 2015- | Production at Jane Street | Self-adjusting computation as an OCaml library |
| rustc queries; Salsa | 2016-; 2018- | rustc's own engine; Salsa in rust-analyzer | Memoised queries with early cutoff |
| Skip (Facebook), Skiplang | 2017-2018; 2026 | Abandoned as a language; runtime language of SkipLabs' framework | Reactive memoisation built into the language |
| Differential dataflow, Materialize | 2013-; 2019- | Production | Incremental view maintenance over `(data, time, diff)` |
| Janus; R, RFUN, ROOPL, Hermes | 1982/2007-; 1997-2022 | Academic | Reversible languages |
| rr, WinDbg TTD, Replay.io, Goblins | 2014-; 2017-; 2020-; 2023 | Production tools | Time travel by record and replay |
| Stan | 2012- | Widely used in applied statistics | Standalone DSL for log densities, NUTS |
| Church, Anglican, WebPPL | 2008-2014 | Research and teaching | Universal probabilistic languages |
| Gen; Pyro/NumPyro | 2019-; 2017- | Active | Programmable inference; effect handlers |
| JAX; Zygote; Enzyme; Dex; Swift for TensorFlow | 2018-; 2018-; 2020-; 2021-; 2018-2021 | JAX dominant; Enzyme in Rust nightly; Dex early; S4TF archived | AD by tracing, source transform, LLVM IR, typed arrays, compiler |
| Q#; Silq | 2017-; 2020- | Q# in Microsoft's QDK; Silq research | Qubits as linear values; automatic uncomputation |
| Boomerang; Augeas | 2008- | Research; Augeas in production | Lenses: bidirectional transformations |
| AspectJ | 2001- | Maintained by Eclipse | Pointcuts and advice |

## The ideas

### Self-adjusting computation: re-run only what changed

**What it is.** Acar's self-adjusting computation (with Blelloch and Harper,
POPL 2002; thesis, CMU 2005) records a dynamic dependence graph during a
run and, when an input changes, *change propagation* re-executes only the
affected parts [1]. Push-based systems such as Jane Street's Incremental
bring observed outputs up to date on `stabilize`; demand-driven ones such as
Adapton (PLDI 2014) and Salsa recompute only what is asked for [2][5].

```ocaml
let x = Inc.Var.create 3
let y = Inc.map (Inc.Var.watch x) ~f:(fun v -> v * v)
let o = Inc.observe y                        (* observers say what is needed *)
let () = Inc.Var.set x 4; Inc.stabilize ()   (* recomputes y and nothing else *)
```

**What it buys.** Jane Street uses Incremental for risk calculations over
live market data and for UIs, at about "30ns" per node fired [3]; Bonsai
"is used to build almost all web applications inside Jane Street" [4].
Salsa ("Inspired by adapton, glimmer, and rustc's query system") runs
rust-analyzer, whose invariant is that "typing inside a function's body
never invalidates global derived data" [5][6]. Differential dataflow "only
acts where changes in collections occur": one update to a 10-million-node
graph took about 230 µs, "about one hundred thousand times faster than
re-running the computation" [9]; Materialize maintains SQL views this way
with "correct and consistent answers" [10].

**Where it is hard.** Memory: rust-analyzer keeps "everything in memory"
because its input is "tens of megabytes at most" [6]. Overhead: a node pays
off only if its work dwarfs the per-node cost [3]. Churn: Jane Street wrote
seven implementations [3]. Non-determinism breaks the model: procedural
macros "may be non-deterministic which conflict how `salsa` works" [6].
And rustc kept its own engine: Salsa "is not used directly in rustc" [5].

**The language-level attempt: Skip.** Facebook's Skip was "a
general-purpose programming language that tracks side effects to provide
caching with reactive invalidation" [7], which is `@memo` on pure functions
with runtime dependency tracking. "memoized functions need to be
transitively side-effect-free, which the Skip compiler enforces
statically", mutation goes through a "Mutator" layer, and "the compiler
does not allow such side effects to cross memoization points" [7]. Built by
veterans of Hack, HHVM, Flow and React Native, it targeted Facebook's HHVM
backend; in July 2018 "the interop is incomplete", and the blog ends that
August [7]. I found no primary account of why it was dropped [unverified].
Its authors' next move is on record: SkipLabs ships "an open-source
framework for building _reactive_ backend services" with "TypeScript
interfaces", whose "native runtime is implemented in Skiplang" [8]. The
runtime survived; the language became an implementation detail.

**Transplants.** Every success is a library or a separate system; build
systems are the same idea [11], and Sisu's compiler and test cache memoise
on normalised-AST hashes ([proposal/09](../../proposal/09-modules-packages-and-build.md)).
No library can *check* its precondition: JAX's transformations "are
designed to work only on Python functions that are functionally pure" [12],
Salsa documents "pure functions (no side effects)" [5], and Skip needed a
new language. Sisu's `pure` already checks it, the argument Waterken's
ETags make for HTTP ([capabilities-distribution-and-resources.md](capabilities-distribution-and-resources.md)).

### Reversible computation, and time travel without it

**What it is.** In Janus (Lutz and Derby, Caltech 1982; formalised by
Yokoyama and Glück, 2007) every statement is invertible: updates are `+=`,
`-=`, `^=` and swap `<=>`, conditionals carry an exit assertion (`if ...
fi`), loops an entry assertion (`from ... until`), and `uncall` runs a
procedure backwards [13]:

```janus
procedure fib
    from i = n
    do
        x1 += x2
        x1 <=> x2
        i -= 1
    until i = 2
```

Successors include R (MIT, for the Pendulum reversible processor), RFUN,
ROOPL (2017), Eel (2016, logs irreversible steps), Hermes (cryptography) and
Revs (F# to reversible circuits) [14]; I could not verify a language named
"Bob" (perhaps Copenhagen's BobISA processor) [unverified]. The motivations
are Landauer's principle (Eel prices irreversible steps in energy) and free
inverses [14]; no reversible hardware ships and the languages stay academic
[inference].

**Time-travel debugging delivers the developer benefit without reversible
semantics** (the tools are surveyed in
[live-and-exploratory-environments.md](live-and-exploratory-environments.md)).
rr "captures all inputs to those processes from the kernel" at slowdowns
"down to ≤ 1.2x" while emulating "a single-core machine" [15]; WinDbg's TTD
costs "about a 10x-20x performance hit" [16]; Replay.io's site now leads
with an autonomous QA product built on "deterministic runtime recordings"
[17]. None needs invertible code: each records every non-deterministic
input at one boundary and reaches earlier states by replaying from a
snapshot. Goblins shows the cheap form for actors, snapshotting a vat
"before each event is processed", opt-in per vat, one process only [18].
Sisu's interpreter owns such a boundary: capabilities plus Phase 6's
seedable scheduler.

### Probabilistic programming: language or library?

**What it is.** Random variables and conditioning become constructs and
inference is the interpreter. "A Stan program imperatively defines a log
probability function over parameters conditioned on specified data and
constants", sampled by "the No-U-Turn sampler, an adaptive form of
Hamiltonian Monte Carlo" [19]. Church (2008), Anglican (2014) and WebPPL
(2014) are universal languages on Lisp and JavaScript subsets [20]. Gen
(PLDI 2019) embeds models in Julia with addressed choices, and its
"Inference algorithms are programmable" [21]:

```julia
@gen function line(xs::Vector{Float64})
    slope = ({:slope} ~ normal(0, 1))
    for (i, x) in enumerate(xs)
        {(:y, i)} ~ normal(slope * x, 0.1)   # observations arrive as a choice map
    end
end
```

Pyro, "a universal probabilistic programming language (PPL) written in
Python", is built on effect handlers ("Most of Pyro's internals are
implemented on top of Poutine"); NumPyro, on JAX, claims "over 100x speedup
for HMC and NUTS" [22].

**Language or library.** What is in wide use is a *restricted* standalone
language or a library. Stan's restriction (a fixed parameter space, a
differentiable log density) is what makes NUTS work, and Stan ships
interfaces "for Python, Julia, R, and the Unix shell" [19]; Pyro and Gen
reinterpret `sample` through handlers or a trace interface; the universal
languages stayed in research and teaching [inference]. No general-purpose
language has `sample`/`observe` keywords. The libraries need a handler, and
Sisu (no effect system, Q-17) gets that shape from a trait parameter, as
in Effekt's "effects as capabilities" ([koka-effekt.md](../languages/koka-effekt.md)).

### Differentiable programming and Swift for TensorFlow

**What it is.** Derivatives as a program transformation, by four routes.
JAX traces Python into "composable function transformations" (`grad`,
`jit`, `vmap`) that "are designed to work only on Python functions that are
functionally pure", and impurity fails silently: side effects "appear
during the first run", and "Subsequent runs may silently use the cached
value of the globals" [12]. Zygote transforms Julia source; "Mutation and
exception handling are currently not supported" [26]. Enzyme works on LLVM
IR: "By differentiating code after optimization, Enzyme is able to create
substantially faster derivatives", for "C, C++, Swift, Julia, Rust,
Fortran" and more [24]; Rust's nightly `std::autodiff` uses it, so
`#[autodiff_forward(dsquare1, Dual, Const, Dual)]` on `fn square` generates
`dsquare1` [25]. Dex (ICFP 2021) puts AD in a typed array language and is
"an experimental research project at an early stage of development" [23].

**The post-mortem.** Swift for TensorFlow, "an experiment in the
next-generation platform for machine learning", "was archived in February
2021"; its AD "continues in the official Swift compiler" [27], yet in 2026
the design document still reads "Partially implemented on main, feature
gated under `import _Differentiation`" [27]. I could not reach the
designers' own account of the shutdown [unverified]. Observably: AD in a
general-purpose language is compiler work across type checking, the IR and
the standard library [27]; it outlived its project without becoming a
language feature in five years; and ML users stayed with Python libraries.
Rust's route, Enzyme on the IR behind a nightly attribute, keeps nearly
all of that cost out of the language [25][inference].

### Quantum languages: uncomputation as linearity

In Q#, "you can't directly copy or access the qubit state"; qubits from
`use q = Qubit();` "must be in the |0⟩ state when they're released", and
"Failure to reset a qubit results in a runtime error" [28]. Silq (PLDI
2020) offers "Safe automatic uncomputation": `!τ` "ensures we can freely
duplicate and drop values of that type", and a `qfree` function "neither
introduces nor destroys superpositions" and "enables automatic
uncomputation" [29]:

```silq
def MyOr(x:𝔹, y:!𝔹)lifted{ // x and y are implicitly const
  return x||y;
}
```

The lesson is about linearity: no-cloning is "no `Copy`", Silq's `!` is
Move's `copy` plus `drop`, Q#'s release rule is a run-time must-consume
check, and uncomputation is a compiler-synthesised destructor, legal
because the computation is reversible. `@must_consume` is the static form
of Q#'s rule; nothing else transfers.

### Bidirectional transformations: lenses

Boomerang (POPL 2008) is "A bidirectional programming language for ad-hoc,
textual data" whose programs are "lenses—well-behaved bidirectional
transformations": read left to right a lens maps input to output; right to
left it maps "a modified output, together with the original input, back to
a modified input" [30]. Its production home is Augeas, a configuration API
that "uses Boomerang's core combinators" [30]. An Augeas lens is "a record
of three functions get, put and create", and `del` lets Augeas restore "the
exact value it saw during parsing" on the way back, so untouched text keeps
its original spelling [31]:

```augeas
let entry = [ key /[a-z]+/ . del /[ \t]*=[ \t]*/ " = " . store /[^\n]*/ . del /\n/ "\n" ]
```

Optics libraries such as Haskell's `lens` mostly serve updates of nested
*immutable* data [inference]; mutable value semantics does that directly
(`cfg.server.port = 8080` mutates an owned value; see
[hylo.md](../languages/hylo.md)). Round-trip editing for `sisu fmt`, `sisu
migrate` and config tools is a tooling job for lossless syntax trees.

### Aspect-oriented programming and what replaced it

Kiczales and colleagues at Xerox PARC introduced AOP at ECOOP 1997, and
AspectJ followed in 2001 [32]; a *pointcut* selects join points, *advice*
runs there:

```java
aspect Tracing { before(): execution(* Account.*(..)) { log("enter " + thisJoinPoint); } }
```

The joke came first: R. Lawrence Clark's 1973 *Datamation* article proposed
`COME FROM` ("We don't know where to GOTO if we don't know where we've COME
FROM"), and C-INTERCAL implemented it [33]. Constantinides et al.'s "AOP
Considered Harmful" (2004) compared advice's hidden control flow to GOTO
and to that joke, and the *fragile pointcut problem* names the other
failure: quantification is "extremely sensitive to changes in the program"
[32]. Obliviousness ("the advice is not visible, in contrast to an explicit
method call" [32]) was both the feature and the defect; Steimann (OOPSLA
2006) questioned the claimed benefits [34]. Weaving survives where
obliviousness is wanted: OpenTelemetry's Java agent "dynamically injects
bytecode to capture telemetry from many popular libraries and frameworks"
[35]; elsewhere decorators, middleware and framework proxies won. Sisu
has no runtime reflection and its attributes are data
([proposal/08](../../proposal/08-metaprogramming.md)), so AOP's concerns
(logging, tracing, auditing, retries) attach at the capability boundary.

## Transferable ideas for Sisu

1. **Incremental computation as `std.x.incr`, Salsa-shaped and gated on
   `pure`:** the `@memo` answer is a library, not a feature. Plain
   memoisation is a generic function over `Hash` and `Eq`; dependency
   tracking goes through an explicit handle, Salsa's API and
   capability-passing style at once.
   ```sisu
   const FILES: Input[Path, String] = Input("files")

   pure def parse(db: Db, path: Path) -> Ast ! ParseError:
       return parser.parse(db.read(FILES, path))   # the read is recorded as a dependency

   pure def symbols(db: Db, path: Path) -> list[Symbol] ! ParseError:
       let ast = db.get(parse, path)?              # memoised on (parse, path); early cutoff
       return collect_symbols(ast)

   var db = Db()
   db.set(FILES, "main.sisu", text)                # a new revision
   let syms = db.get(symbols, "main.sisu")?        # re-runs only what the edit invalidated
   ```
   *Cost:* a library of Salsa's size; derived `Hash`/`Eq` on keys and
   results. *Conflicts:* `pure` forbids shared mutation and `Db`'s caches
   mutate, so `Db.read`/`Db.get` need a trusted "observationally pure"
   declaration in a `[trust]` package, an audited hole in `pure` (Haskell
   memo libraries lean on `unsafePerformIO` alike [inference]). Attributes
   cannot wrap functions (proposal/08), so no `@memo` decorator; the
   non-goal making "reactive streams" libraries agrees (proposal/14).
   *Confidence:* high that it is a library (Skip versus Salsa,
   Incremental, Bonsai); medium that it belongs in `std.x`.

2. **Capability interceptors as the sanctioned replacement for AOP.** A
   generic `Wrap[T]` for any capability trait intercepts every call to the
   outside world, visibly in `main`, typed, with no pointcuts:
   ```sisu
   def main(sys: System) -> ! AppError:
       let tracer = Tracer.to(sys.stderr)
       let net = trace.wrap(sys.net, tracer)        # every connect, read and write is a span
       let fs = audit.wrap(sys.fs.restrict_to("./data"), tracer)
       return serve(fs, net, sys.clock)
   ```
   Built from the reflection behind `Fake[MyTrait]` (proposal/17), it is
   also the recording adapter of the capability-level replay proposed in
   [live-and-exploratory-environments.md](live-and-exploratory-environments.md)
   (idea 2): tests, tracing and replay share one interception point.
   *Cost:* low (a dynamic call per wrapped call). *Conflicts:* none.
   *Confidence:* medium-high.

3. **Reverse stepping by snapshots at turn boundaries, layered on that
   replay (Goblins, rr).** Reversible languages are not needed for time
   travel. The interpreter's tasks are heap-allocated frame stacks
   ([proposal/18](../../proposal/18-next-steps.md), Phase 6), so it can take
   copy-on-write snapshots at yield points, as Goblins does per event, and
   reach an earlier state by restoring one and replaying the log forward:
   ```
   sisu debug --replay=crash.trace             # deterministic; no disk or network touched
   (sisu) break parse.sisu:42 if line.is_empty()
   (sisu) reverse-continue                     # nearest snapshot, then replay to the last hit
   ```
   *Cost:* medium (snapshot memory, a snapshot interval policy).
   *Conflicts:* `extern` state is not captured; hot replacement (Q-22)
   changes code mid-trace; native builds get this only via rr.
   *Confidence:* medium (rr and Goblins show the method).

4. **AD and probabilistic programming as libraries over numeric traits and
   handler-style parameters.** Forward mode is operator overloading on
   `Dual`; a model takes its handler as a trait parameter, Gen-style, and
   inference algorithms implement it:
   ```sisu
   pure def cube[T: Float](x: T) -> T:
       return x * x * x
   let d = cube(Dual(2.0, 1.0)).tangent              # 12.0

   def line[M: Model](mut m: M, xs: list[f64]):
       let slope = m.sample("slope", Normal(0.0, 1.0))
       for i, x in enumerate(xs):
           m.sample("y{i}", Normal(slope * x, 0.1))  # constrained by the inference engine
   ```
   Reverse mode is a tape library, or Enzyme on the LLVM release backend.
   *Cost:* none for the language if numeric traits are generic enough
   (Q-23). *Conflicts:* none; the fixed operator set suffices.
   *Confidence:* low-medium (Sisu's numeric traits are untested).

## Dead ends

- **Reactive memoisation in the language (Skip):** now a TypeScript framework [7][8].
- **Transparent memoisation of every `pure` function:** overhead [3],
  memory [6] and unpredictable costs, against Sisu's priorities.
- **Reversible languages (Janus line):** no hardware, no adoption [14].
- **AD as a language feature (Swift for TensorFlow):** still feature-gated
  five years after the project was archived [27].
- **`sample`/`observe` keywords:** every working system is a restricted DSL
  or a library [19][21][22].
- **Quantum types; lenses as a core abstraction:** outside Sisu's domain,
  and value semantics covers nested update.
- **Pointcuts, advice and weaving:** obliviousness and fragile pointcuts
  [32]; a reflection-free Sisu should not add them.

## Sources

1. "Incremental computing", Wikipedia (2026), citing U. A. Acar, *Self-Adjusting Computation*, PhD thesis, CMU, 2005, and Acar, Blelloch, Harper, "Adaptive Functional Programming", POPL 2002. https://en.wikipedia.org/wiki/Incremental_computing
2. Adapton: M. Hammer et al., "Adapton: composable, demand-driven incremental computation", PLDI 2014; "Incremental computation with names", OOPSLA 2015. http://adapton.org/
3. Y. Minsky, "Introducing Incremental", 18 July 2015, https://blog.janestreet.com/introducing-incremental/ ; "Seven implementations of Incremental", 9 March 2016. https://blog.janestreet.com/seven-implementations-of-incremental/
4. Jane Street, Bonsai README (2026). https://github.com/janestreet/bonsai
5. Salsa README (2026), https://github.com/salsa-rs/salsa ; rustc-dev-guide, "Salsa" (2026). https://rustc-dev-guide.rust-lang.org/queries/salsa.html
6. rust-analyzer, "Architecture" (2026). https://rust-analyzer.github.io/book/contributing/architecture.html
7. Skip site and blog, including "How memoization works" (2017) and "Async/Await" (July 2018). http://skiplang.com/ ; http://skiplang.com/blog/ ; https://github.com/skiplang/skip
8. SkipLabs, Skip framework README (2026). https://github.com/SkipLabs/skip
9. Differential dataflow README (2026), https://github.com/TimelyDataflow/differential-dataflow ; F. McSherry et al., "Differential Dataflow", CIDR 2013.
10. Materialize README (2026). https://github.com/MaterializeInc/materialize
11. A. Mokhov, N. Mitchell, S. Peyton Jones, "Build Systems à la Carte", ICFP 2018 (not re-fetched).
12. JAX README and "Common Gotchas in JAX" (2026). https://github.com/jax-ml/jax ; https://docs.jax.dev/en/latest/notebooks/Common_Gotchas_in_JAX.html
13. "Janus (time-reversible computing programming language)", Wikipedia (2026); T. Yokoyama, R. Glück, PEPM 2007. https://en.wikipedia.org/wiki/Janus_(time-reversible_computing_programming_language)
14. "Reversible programming language", Wikipedia (2026). https://en.wikipedia.org/wiki/Reversible_programming_language
15. rr project page (2026), https://rr-project.org/ ; R. O'Callahan et al., "Engineering Record and Replay for Deployability", USENIX ATC 2017, arXiv:1705.05937.
16. Microsoft, "Time Travel Debugging Overview" (updated November 2025). https://learn.microsoft.com/en-us/windows-hardware/drivers/debuggercmds/time-travel-debugging-overview
17. Replay.io home page (2026). https://www.replay.io/
18. D. Thompson, "Introducing: A distributed debugger for Goblins with time travel!", Spritely Institute, 12 April 2023. https://spritely.institute/news/introducing-a-distributed-debugger-for-goblins-with-time-travel.html
19. B. Carpenter et al., "Stan: A Probabilistic Programming Language", *Journal of Statistical Software* 76(1), 2017, https://www.jstatsoft.org/article/view/v076i01 ; https://mc-stan.org/
20. N. Goodman et al., "Church: a language for generative models", UAI 2008; F. Wood et al., Anglican, AISTATS 2014; N. Goodman, A. Stuhlmüller, *The Design and Implementation of Probabilistic Programming Languages* (WebPPL), 2014 (not re-fetched).
21. M. Cusumano-Towner et al., "Gen: A General-Purpose Probabilistic Programming System with Programmable Inference", PLDI 2019. https://www.gen.dev/ ; https://github.com/probcomp/Gen.jl
22. Pyro home page; "Poutine: A Guide to Programming with Effect Handlers in Pyro" (2026). https://pyro.ai/ ; https://pyro.ai/examples/effect_handlers.html
23. A. Paszke et al., "Getting to the Point: Index Sets and Parallelism-Preserving Autodiff for Pointful Array Programming", ICFP 2021, arXiv:2104.05372; Dex README. https://github.com/google-research/dex-lang
24. W. Moses, V. Churavy, "Instead of Rewriting Foreign Code for Machine Learning, Automatically Synthesize Fast Gradients", NeurIPS 2020; Enzyme. https://enzyme.mit.edu/
25. Rust standard library (nightly), `std::autodiff` (2026). https://doc.rust-lang.org/nightly/std/autodiff/index.html
26. Zygote.jl README (2026). https://github.com/FluxML/Zygote.jl
27. Swift for TensorFlow README (archived), https://github.com/tensorflow/swift ; R. Wei, D. Zheng, M. Rasi, B. Chrzaszcz, "Differentiable Programming Manifesto" (2026). https://github.com/swiftlang/swift/blob/main/docs/DifferentiableProgramming.md
28. Microsoft Learn, "Introduction to the quantum programming language Q#" (updated 2026). https://learn.microsoft.com/en-us/azure/quantum/qsharp-overview
29. B. Bichsel, M. Baader, T. Gehr, M. Vechev, "Silq: A High-Level Quantum Language with Safe Uncomputation and Intuitive Semantics", PLDI 2020; Silq documentation. https://silq.ethz.ch/documentation
30. Harmony/Boomerang page: A. Bohannon et al., "Boomerang: Resourceful Lenses for String Data", POPL 2008; J. N. Foster et al., TOPLAS 29(3), 2007. https://www.engineering.upenn.edu/~harmony/
31. Augeas, "Lenses" documentation (2026). https://augeas.net/docs/lenses.html
32. "Aspect-oriented programming", Wikipedia (2026), citing G. Kiczales et al., ECOOP 1997, and C. Constantinides et al., "AOP Considered Harmful", 2004. https://en.wikipedia.org/wiki/Aspect-oriented_programming ; AspectJ. https://eclipse.dev/aspectj/
33. "COMEFROM", Wikipedia (2026), citing R. L. Clark, *Datamation*, 1973. https://en.wikipedia.org/wiki/COMEFROM
34. F. Steimann, "The Paradoxical Success of Aspect-Oriented Programming", OOPSLA 2006 (not re-fetched).
35. OpenTelemetry, "Java Agent" documentation (2026). https://opentelemetry.io/docs/zero-code/java/agent/
