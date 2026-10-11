# F# and Standard ML

Two MLs at opposite ends of the pragmatism axis: F# is ML re-engineered to
live inside a corporate platform; Standard ML is ML frozen by a formal
definition. Both are analysed here because each isolates one variable that
the other MLs (OCaml, Haskell) blur.

## Snapshot
| | F# | Standard ML |
|---|---|---|
| First appeared | 2002 (internal, "Caml for .NET"); F# 1.0 released 2005; productised in Visual Studio 2010 (F# 2.0) [1] | 1983 (Milner's proposal); The Definition of Standard ML 1990; Revised Definition (SML '97) 1997 [2] |
| Designed by | Don Syme (Microsoft Research Cambridge) with Andrew Kennedy, James Margetson, Luke Hoban, Gregory Neverov, Tomas Petricek and the F# team | Robin Milner, Mads Tofte, Robert Harper, David MacQueen (Definition); Burstall, Cardelli, Appel, Reppy and others |
| Current version (Oct 2026) | F# 10 (shipped with .NET 10 and Visual Studio 2026, Nov 2025) [3]; F# 9 (.NET 9, Nov 2024) [4] | SML '97 is final; no language release since 1997. Implementations: SML/NJ, MLton, Poly/ML, MLKit, Moscow ML |
| Governance | Open source since 2010-2014 (dotnet/fsharp on GitHub); Microsoft F# team with the F# Software Foundation (FSSF, US non-profit, 2012-13); language changes via fslang-suggestions and fslang-design RFCs [1] | Frozen. The HOPL paper: because of the copyright on the Definition and unavailable TeX sources "there was to be no further evolution of the language"; Successor ML exists as a patch set implemented partially by MLton/SML-NJ [2, §9.3] |
| Typing | Static, HM-style inference with .NET nominal objects, no higher-kinded types; statically resolved type parameters (SRTP) for operator overloading; units of measure; nullable reference types since F# 9 | Static, HM with principal types, datatypes, eqtypes, value restriction, full module system with signatures and functors |
| Memory | .NET tracing GC | Implementation-defined tracing GC (MLton whole-program, SML/NJ generational, MLKit regions + GC) |
| Compilation | .NET IL, JIT or Native AOT; Fable to JavaScript/TypeScript/Python/Rust targets | AOT native (MLton, Poly/ML), bytecode (Moscow ML) |
| Primary domains | Finance, data/analytics, web (SAFE stack, Fable), scripting in .NET shops | Teaching (CMU 15-150, many textbooks), compiler and proof-assistant implementation (Isabelle/HOL on Poly/ML, HOL4, Twelf) |
| Notable users | Microsoft (parts of Azure tooling), fintech and insurance firms via FSSF testimonials; Jet.com/Walmart (historical) | Isabelle, HOL, CakeML's ancestry, AT&T 5ESS tooling (historical) [2, §9.2] |

## Stated design goals
**F#.** Syme's HOPL paper frames F# as the answer to Wadler's 1998 "Why no
one uses functional languages" checklist (compatibility, libraries,
portability, availability, packagability, tools, training, popularity): "The
early development of F# was essentially an effort to address each of these"
[1, §5]. The method was "functional-first programming for the enterprise":
functional prototyping, then object programming for interop, then imperative
code for performance [1, §10]. The strategic bet was that direct compilation
to a mainstream VM with generics (which Syme and Kennedy had added to .NET
precisely to make this possible [1, §4]) would succeed where MLj, Harlequin
MLWorks and Poly/ML had not.

**Standard ML.** A language whose definition would be "an integral part of the
design process", in the Strachey tradition that prose manuals are "not good
enough" [2, §6]. The Definition is a complete static and dynamic semantics in
natural-deduction style; the goal was that multiple independent
implementations and formal proofs of safety would follow from it, which they did.

## Strengths

### The pipeline operator (F#) and left-to-right inference
`let (|>) x f = f x` was added to the F# library in 2003; the symbol
originates with Tobias Nipkow in Isabelle/ML in May 1994 [1, §9.1]. It
matters for more than readability: "type-inference is propagated
left-to-right and name resolution occurs based on information available
earlier", so `data |> List.map (fun s -> s.Length)` type-checks where
`List.map (fun s -> s.Length) data` needs an annotation [1, §9.1]. Copied by
OCaml (4.01, 2013), Elixir, Elm, Julia, R (native `|>` in 4.1, 2021), and
the JavaScript TC39 pipeline proposal (stage 2) which describes the "F#
language's pipe syntax" as one of its two candidate designs [5]. Few language
features have been copied this widely this fast.

### Computation expressions: async before C# had it
After a 2007 sabbatical with Odersky, Syme added `async { ... }` to F# in
October 2007; the feature "directly influenced the async/await mechanism
added to C# 5.0 in 2012 - the F# version was first presented to the C#
designers in 2007" [1, §9.7, §21.2]. Computation expressions generalise
Haskell's do-notation into a syntax over builder objects (`seq`, `async`,
`task`, `query`, user-defined), so one syntax covers generators, async,
queries and parsers. F# 9 and 10 continued refining them (empty-bodied CEs,
more consistent syntax) [3][4]. Syme's own caveat is worth keeping: C#'s
version added state-machine compilation and "was not a simple copy" [1,
§21.2 n.40].

### Units of measure
Added in F# 2.0 (2009) from Andrew Kennedy's PhD work on integrating unit
inference with HM inference: `1.0<m/s>` has a type that prevents adding
metres to seconds, with inference and generic unit variables; "the major
challenge... is its integration with F#'s version of Hindley-Milner type
inference" [1, §12]. It remains the only mainstream implementation with
inference; other languages offer library approximations (Rust `uom`, C++
`mp-units`) without inference. Evidence that it works: it is a one-line
feature to use, and it has stayed in the language for 17 years without
complaint. Evidence of limits: nobody else copied it, which suggests the
niche (scientific and financial code) is narrow enough to leave to libraries
in a language with strong enough phantom types.

### Type providers
F# 3.0 (2012) added compile-time metaprogramming plug-ins that "provide
information about a programmatically-generated space of nominal object type
definitions" on demand, so referencing a database, CSV, JSON endpoint or
spreadsheet gives typed access with autocomplete [1, §13]. This is
metaprogramming that performs I/O at type-checking time, which no other
mainstream language allows; the cost is that the compiler, IDE and build must
all host the plug-in, and offline/reproducible builds become harder. The
feature is loved by its users and uncopied by other languages, which is the
usual signature of "powerful but hard to productise".

### Active patterns
Added in 2006 (ICFP 2007 paper) after Peyton Jones advised "bang for buck"
and recounted Haskell's failed attempts at views: partial, complete and
multi-case patterns that let any .NET value be matched as if it were a
datatype [1, §9.5]. Haskell later got view patterns and pattern synonyms;
Scala has extractors; Swift and Kotlin have nothing comparable. Active
patterns are the simplest shipped answer to Wadler's 1987 "views" problem.

### Indentation-aware syntax as a deliberate adoption decision
Released September 2006 and default from F# 2.0: Syme "made a private
judgement that the presence of these extra tokens [`in`, `;;`] was a limiting
factor in F# adoptability", citing Python; Simon Marlow helped design it [1,
§9.10]. Together with `|>`, this is why F# code reads as a sequence of
statements to a C# programmer while remaining an ML.

### Succinctness with full platform access
Syme cites a delivered project where "a reduction in implementation size from
350,000 lines of code (C#) to 30,000 (F#)" was reported [1, §12 n.34]; this is
a single anecdote and should be read as such. The structural claim is
stronger: F# gets every .NET library, GC, JIT, debugger and profiler for
free, which is the "Wadler checklist" answer that Haskell and OCaml never had.

### Standard ML: a formal definition that was actually used
"One purpose of the Definition was to support rigorous proof of the
properties of the language, and in this regard it has been remarkably
successful" [2, §7.3]: Milner and Tofte's Commentary proved principal
signatures; VanInwegen and Gunter attempted mechanised safety in HOL (1993-96);
Lee, Crary and Harper achieved a mechanised safety proof in Twelf (2007).
"The existence of multiple implementations targeting the same language
specification was a real success for the Definition" [2, §8.1]. What a formal
spec bought, concretely: five actively maintained implementations that run
the same programs; a language whose teaching value comes from stability
("its stability allows its users to focus attention on their own work" [2,
§9.4]); and a reference point for every later soundness proof.

### Standard ML: the lesson of mechanisation
"Perhaps the biggest lesson of the Twelf mechanization is that it is
essential to define a language hand-in-hand with the verification of its
metatheory... The critical step required to apply these methods to another
language is to isolate a well-behaved internal language whose safety is
easily stated and proved, and then to define the language in question by
elaboration into the internal form. Absent such a formulation... we know of
no other mechanization for a language of comparable scale" [2, §7.3]. This is
a direct, actionable design instruction for any new language that wants a
soundness story.

## Weaknesses

### F#: a first-class language with second-class backing
Management promised F# would be "a 'first-class language in Visual Studio'",
which users read as tooling parity with C#; Visual Studio tooling was then
judged "not realistic" for F# 2.0 and "the dissonance between these positions
caused frustration" [1, §10]. Microsoft's 2009 workforce cut put "projects
across the board... at risk", DevDiv "didn't feel able to push F# as a choice
for app programming on to the powerful Windows team", and "a reduction in
resourcing occurred" even as F# 3.0 shipped [1, §10, §14]. Syme's own
"greatest mistake" is "that neither .NET nor the language were open source or
using open engineering" until 2011-2014, which also meant "most early
contributors to F# soon moved on" and could not keep contributing [1, §21.2].

### "C# good enough": the feature-absorption trap
Syme lists what C# took: generics (C# 2.0), `var` (3.0), async (5.0), tuples
and pattern matching (7.0), enhanced patterns (8.0), non-null by default
(9.0) - "all heavily influenced by F#" [1, §21.1]. Records (C# 9), switch
expressions and a discriminated-union proposal under active design follow
the same path (the union proposal's exact status in October 2026 was not
verified in this research pass). Each absorption reduces the reason to switch
languages, and each creates a compatibility tension: "F# 1.9 added Async<T> in
2007... .NET added Task<T> in 2010... even at the time of writing, in 2019,
these sit awkwardly together"; the same happened with boxed vs unboxed tuples
(2017) and with quotations vs LINQ expression trees [1, §21.2]. A language
that lives on someone else's platform does not control its own abstractions.

### SRTP and generic comparison: features Syme would cut
"The 'statically resolved type parameter' mechanism is perhaps the one causing
most corner-case niggles. Originally designed just for operator overloading,
the mechanism is also used by some advanced F# users as a type constraint
mechanism akin to Haskell type classes... The combination of complex SRTP
constraints with algorithm-based type inference is, however, fragile" [1,
§21.2]. On OCaml's inheritance: "the whole generic comparison feature could
likely have been omitted from F#, or greatly constrained" [1, §21.2]. F# has
no type classes and no higher-kinded types, so abstraction over `Async` vs
`Task` or over numeric types goes through SRTP hacks or `inline` tricks.

### Two worlds in one language
Functional-first F# and object-oriented .NET meet at every API boundary:
`null` from C# libraries (addressed only in F# 9 with `string | null` types
and flow-sensitive narrowing [4]), exceptions versus `Result`, mutable
classes versus records, `Task` versus `Async`, curried versus tupled
arguments, and the top-down file ordering requirement that .NET programmers
find alien. Each is manageable; together they are the learning curve.

### Adoption stayed small
Stack Overflow: 0.9% (2024), 1.3% (2025) [6][7]. Download rates were
"18,000/year in early 2007: far from mainstream" [1, §10]. The causes above
(sponsor ambivalence, C# absorption) are structural, not technical. F# is the
strongest evidence in this study that a technically excellent language with a
large platform does not win adoption without an independent reason to exist.

### Standard ML: frozen, under-specified in places, and academic
The Definition's own defects: Kahrs (1993) found errors; Rossberg's catalogue
(2013) shows "the Definition was ambiguous in several important places. For
example, the specification of overload resolution is not precisely defined,
which leads to incompatibilities between implementations"; "there is no
specification of the full syntax... the grammar is ambiguous in a number of
places"; and "the lack of a standard code-assembly mechanism and the lack of
a standard foreign-function mechanism are two other serious omissions" [2,
§9.1]. The Basis Library only arrived in 2004, fourteen years after the
Definition, and before it "even basic code written for one implementation was
not portable to others" [2, §8.1]. Polymorphic equality (eqtypes) was a
design Harper and MacQueen wanted removed or deprecated [2, §4.5]. Harlequin's
commercial MLWorks "saw little adoption" [1, §5]. The governance failure is
unique: the language could not evolve because the Definition's copyright and
TeX sources were unavailable, so Successor ML became a wiki of patches [2,
§9.3]. SML shows that a formal definition without a revision process is a
monument, not a living standard.

## Designer retrospectives
- Syme (2020): open source came too late; SRTP fragile; generic comparison a
  mistake; the Task/Async and boxed/unboxed tuple tensions with C# are
  structural costs of platform-hosting [1, §21.2]. He also credits F#'s
  survival 2002-2011 to "the recognition of its qualities by decision makers
  at Microsoft", i.e. individual sponsors, not institutional strategy.
- MacQueen, Harper, Reppy (2020): the Definition succeeded at its purpose
  (proofs, multiple implementations) but was ambiguous in overloading,
  under-specified in syntax, and silent on FFI and separate compilation;
  mechanisation taught them to design the internal language first [2, §7.3,
  §9.1]. Their summary verdict: SML "occupies a 'sweet spot' in the design
  space; it is flexible and expressive while not being overly complicated"
  [2, §9.4]. (Note that this is the designers' own assessment.)

## Influence
F# gave: `|>` to half the industry; async workflows to C# 5 and from there to
JavaScript, Python, Rust and Swift (C# was the vector, as Syme notes [1,
§21.2]); units of measure to no one; type providers to no one; active
patterns to Haskell's pattern-synonym discussion; nullable-reference-type
syntax `T | null` is itself borrowed from TypeScript. F# took: core from
OCaml, objects and generics from .NET/C#, indentation from Python/Haskell,
`async` ideas from Haskell monads and Scala.

Standard ML gave: datatypes with pattern matching, HM inference and
parametric polymorphism as "features that functional-language programmers
take for granted today" [2, §9.2.2]; the module system to OCaml and (via
Harper-Lillibridge and Leroy's 1994 papers) to all later module research; the
value restriction to OCaml; the idea of a mechanised definition to CakeML,
WebAssembly (whose spec and reference interpreter follow the SML style) and
Rust's later formalisation efforts. SML took: ISWIM's syntax, Hope's datatypes,
LCF/ML's exceptions and strict evaluation [2, §1-2].

## Lessons for Sisu
- **Adopt**
  - `|>` with left-to-right inference as a first-class design constraint:
    design the inference order so pipelines never need annotations.
  - Indentation-sensitive, token-light syntax; Syme's judgement that `in`
    and `;;` cost adoption is backed by OCaml's two syntax forks.
  - Active patterns / views: extensible pattern matching over abstract
    types from the first release, kept simple ("bang for buck").
  - The SML mechanisation recipe: a small internal language whose safety is
    stated and proved, with the surface language defined by elaboration.
    Write the core calculus before the parser.
  - One generic "builder" syntax for monadic/effectful blocks (computation
    expressions) rather than a dedicated keyword per effect (`async`,
    `seq`, `query` each hard-coded).
- **Adapt**
  - Units of measure: provide the type-level machinery (phantom parameters
    with inference over a free abelian group) only if it falls out of the
    generics design; otherwise leave to a library. The feature is excellent
    and unimitated, which suggests a library is sufficient.
  - Type providers: the goal (typed access to external schemas) is right;
    the mechanism (I/O during type checking inside the compiler process)
    is not. Prefer generated code checked into the tree, or a sandboxed,
    cacheable compile-time step with declared inputs.
  - Nullable interop types `T | null` are the right shape for FFI with
    null-ful platforms; Sisu should have the same at its C/JS boundary and
    nowhere else.
- **Avoid**
  - Building the language's identity on a platform it does not control;
    every feature C# absorbed removed a reason to use F#, and every feature
    C# added differently (Task, tuples) created a permanent seam.
  - Two abstractions for one job (`Async`/`Task`, boxed/unboxed tuples); if
    the host platform forces one, use it.
  - Structural generic comparison and SRTP-style constraint hacks instead of
    real type classes; both designers (Syme, and SML's Harper/MacQueen on
    eqtypes) regret the magic variant.
  - A frozen specification with no revision process and no owner; SML's
    standard became unamendable through copyright, which is a governance
    failure a new language can avoid with a public-domain or CC spec and a
    versioning process from day one.
  - Promising tooling parity before it exists.
- **Open questions raised**
  - Can a formal definition be kept in sync with a fast-moving language?
    SML's precision came with freezing; Haskell's and OCaml's evolution came
    without a definition. WebAssembly suggests both are possible with a
    mechanised spec in the repository; no general-purpose language has done
    it yet.
  - Is platform-hosting (JVM, .NET, JS) a net positive for a new language?
    F#, Scala and Kotlin got libraries and adoption; all three inherited
    `null`, exceptions and a foreign object model. Sisu's interop story must
    decide whether to be hosted or to be a native language with FFI.

## Sources
1. Syme, "The Early History of F#", Proc. ACM Program. Lang. 4, HOPL, Article 75 (June 2020; presented HOPL IV June 2021). https://fsharp.org/history/hopl-final/hopl-fsharp.pdf (2020)
2. MacQueen, Harper, Reppy, "The History of Standard ML", Proc. ACM Program. Lang. 4, HOPL, Article 86 (June 2020). https://www.cs.tufts.edu/~nr/cs257/archive/dave-macqueen/history-of-ml.pdf (2020); also https://standardml.org/history
3. Microsoft, "Introducing F# 10" (17 Nov 2025). https://devblogs.microsoft.com/dotnet/introducing-fsharp-10/ (2025)
4. Microsoft Learn, "What's new in F# 9" (nullable reference types, DU `.Is*` properties, partial active patterns returning bool). https://learn.microsoft.com/en-us/dotnet/fsharp/whats-new/fsharp-9 (2024)
5. TC39, "Pipeline operator" proposal README (stage 2; F#-style vs Hack-style). https://github.com/tc39/proposal-pipeline-operator (2026)
6. Stack Overflow Developer Survey 2024, Technology (F# 0.9%). https://survey.stackoverflow.co/2024/technology (2024)
7. Stack Overflow Developer Survey 2025, Technology (F# 1.3%). https://survey.stackoverflow.co/2025/technology (2025)
8. OCaml 4.01.0 release notes (`|>` added to Pervasives, 2013). https://ocaml.org/releases/4.01.0 (2013)
9. Milner, Tofte, Harper, MacQueen, The Definition of Standard ML (Revised), MIT Press (1997)
10. Kennedy, "Types for units-of-measure: theory and practice", CEFP lecture notes (2009)
11. Syme, Neverov, Margetson, "Extensible pattern matching via a lightweight language extension", ICFP (2007)
12. Syme et al., "Strongly-typed language support for internet-scale information sources" (type providers), MSR technical report (2012)
13. Lee, Crary, Harper, "Towards a mechanized metatheory of Standard ML", POPL (2007)
14. Wadler, "Why no one uses functional languages", SIGPLAN Notices 33(8) (1998)
