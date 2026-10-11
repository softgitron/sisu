# Julia

## Snapshot
| | |
|---|---|
| First appeared | Work began 2009 at MIT; public announcement February 2012; 1.0 in August 2018 |
| Designed by | Jeff Bezanson, Stefan Karpinski, Viral B. Shah, Alan Edelman |
| Current version (Oct 2026) | 1.13.0 (10 September 2026); 1.12 now unmaintained; 1.10.x is the long-term-support line (1.10.11, March 2026) |
| Governance | MIT-licensed open source; informal core team of long-time committers; JuliaHub (the founders' company) employs many of them; NumFOCUS fiscal sponsorship; no formal RFC process - design happens in GitHub issues, PRs and "triage" calls |
| Typing | Dynamic, with a rich parametric nominal type system; type inference is used for specialisation, not for checking |
| Memory | Tracing, non-moving, generational mark-and-sweep GC; parallel marking since 1.10; no ownership or RC story |
| Compilation | LLVM-based JIT (strictly "just-ahead-of-time": every method is compiled for the concrete argument types on first call); native-code caching for packages since 1.9; experimental AOT trimming via `juliac` since 1.12 |
| Primary domains | Scientific and numerical computing, simulation, differential equations (SciML), optimisation (JuMP), data science, HPC |
| Notable users | SciML and JuMP ecosystems, CliMA climate model, Pumas (pharmacometrics), Federal Reserve Bank of New York DSGE model, ASML, numerous national labs |

## Stated design goals
The 2012 announcement "Why We Created Julia" is unusually explicit [1]:
"We want a language that's open source, with a liberal license. We want the
speed of C with the dynamism of Ruby. ... We want something as usable for
general programming as Python, as easy for statistics as R, as natural for
string processing as Perl ... We want it interactive and we want it compiled."

The implicit goal, which the community later named, is to solve the
"two-language problem": scientists prototype in Python/R/MATLAB and then
rewrite hot loops in C/Fortran. Julia's bet was that a dynamic language with
aggressive type specialisation and multiple dispatch could be both the
prototyping language and the fast language. The 2025 user survey still lists
"solves the two language problem" as a liked feature, chosen by 34-53% of
respondents depending on year [5].

## Strengths

### Multiple dispatch as the single organising principle
Every function in Julia is a generic function; methods are selected on the
runtime types of *all* positional arguments. There are no classes with
methods, so a function `f(x::A, y::B)` can be added by anyone, for any
combination of types defined anywhere. Karpinski's JuliaCon 2019 talk "The
Unreasonable Effectiveness of Multiple Dispatch" argues this is why Julia's
ecosystem composes: it solves the expression problem in both directions.
A package can add new types that work with existing generic algorithms, and
can add new functions over existing types, without touching either [2].
The manual's own framing is modest ("particularly useful for mathematical
code" because operations do not naturally "belong" to one argument) [3].
Evidence that users value it: "multiple dispatch" is chosen by 46-54% as a
most-liked technical feature in every survey since 2019, and "composable" by
36-45% [5]. The canonical demonstration is that automatic differentiation
(ForwardDiff's dual numbers), uncertainty propagation (Measurements.jl) and
physical units (Unitful.jl) all work through differential-equation solvers
that were never written with them in mind.

### Speed from a dynamic language: the two-language problem, partly solved
Julia specialises each method on concrete argument types, infers types
through the body, and hands LLVM code that is often indistinguishable from C.
Pure-Julia libraries routinely match or beat C/Fortran incumbents
(DifferentialEquations.jl vs SUNDIALS, JuMP's pure-Julia solvers), and "speed,
performance" is the top-liked feature (83-86% every year) [5]. Within its
niche the promise held: there is no C layer in most of the SciML stack.
The caveat is in the weaknesses: the promise holds only when the code is
*type-stable*, and that is a skill users have to learn.

### A parametric type system that is expressive without being a proof system
Abstract types form a hierarchy used only for dispatch; concrete types can
be parametric (`Array{Float64,2}`), with `Union` types, `UnionAll` types,
`Tuple` types and value parameters. The compiler optimises small unions
(`Union{T, Missing}`, `Union{T, Nothing}`), which is how `missing` (borrowed
from R) and `nothing` get Option-like ergonomics without a sum type feature.
Users rate the type system highly (29-35% most-liked) [5].

### Hygienic AST macros and generated functions
Julia's `Expr` objects are ordinary data; macros are functions from AST to
AST with hygiene by default (`esc` to opt out). The language also has
`@generated` functions (methods whose body is generated from the argument
*types* at specialisation time). Because macros are visibly marked with `@`,
DSL-heavy packages (JuMP's `@variable`/`@constraint`, Turing's `@model`,
`@threads`, `@simd`) remain readable. Metaprogramming is a most-liked feature
for 20-24% [5].

### Pkg: environments, manifests, registries and artifacts
Pkg (rewritten for 1.0) has `Project.toml` (direct deps with compat bounds)
and `Manifest.toml` (the full resolved graph), stacked environments, multiple
registries, and since 2019 a content-addressed **artifacts** system
(`Artifacts.toml` maps names to tree hashes; binaries live in a global
depot keyed by hash; JLL packages wrap BinaryBuilder outputs with Yggdrasil
recipes) [6]. The stated goal was packages that "can be installed completely
read-only" with no stateful `build.jl`. It is the second or third most-liked
technical feature (53-68%) [5]. 1.12 added workspaces (several projects
sharing one manifest) and "apps" (packages that install command-line
programs) [7].

### Interactive and compiled at once
The REPL, Revise.jl (hot reload), Pluto and IJulia notebooks, and the ability
to inspect `@code_warntype` / `@code_llvm` / `@code_native` for any call make
the compiler legible. 1.12 extended "world age" to bindings so that structs
and constants can finally be redefined in a session, with a `MethodError`
pointing at the old world; before 1.12, redefining a struct required a
restart [7]. 1.13 ships built-in REPL syntax highlighting and fuzzy history
search [8].

### Composable task-based multithreading
`Threads.@spawn` (1.3+) schedules tasks on a thread pool with a depth-first
scheduler modelled on Cilk; 1.12 adds a separate interactive thread pool by
default and respects cgroup/taskset affinity, fixing oversubscription on HPC
nodes; 1.13 fixes a set of lost-task and deadlock races and makes `@spawn`
wake one idle thread rather than all of them (10-300x speedup on
oversubscribed Windows machines) [7][8]. Data races are the user's problem
(see weaknesses).

## Weaknesses

### Compile latency ("time to first plot") - the top complaint for seven years
Because every method is compiled per concrete signature on first use, a
fresh session that loads Plots or Makie used to take tens of seconds before
the first figure appeared. "Slow compile times / too long to generate the
first plot" is the number one technical complaint in every survey
2019-2025, chosen by 41-50% [5]. The fixes shipped are the best evidence of
how serious it was:
- 1.9 (May 2023): package images cache *native code*, not just inferred
  types. Measured TTFX dropped from 11.66 s to 0.08 s for CSV, 17.39 s to
  0.38 s for DataFrames, 64.6 s to 1.66 s for GLMakie, 73.5 s to 4.8 s for
  ModelingToolkit; cost is 10-50% longer precompilation and larger caches [9].
- 1.10 (Dec 2023): parallel precompilation and faster package loading
  (OmniPackage, ~650 packages: 48 s to 19 s) [10].
- 1.13 (Sep 2026): precompilation ~30% faster than 1.12, startup ~20%
  faster (69 ms to 57 ms for `julia -e ''`), and TTFX is now tracked in CI
  on every PR with 39 community workflows at perf.julialang.org [8].
The lesson is not only that JIT latency is painful but that it took eleven
years of releases and a dedicated CI benchmark to get it under control, and
the survey numbers have not yet moved.

### No self-contained binaries
"Cannot generate self-contained binaries or libraries" is the second-ranked
technical complaint (35-52%) [5]. PackageCompiler.jl produces hundreds of
megabytes because it must ship the whole runtime and LLVM. 1.12's
experimental `--trim` / JuliaC.jl can produce a 1.1 MB hello-world, but only
if code reachable from the entry point contains **no dynamic dispatch** -
any unresolved call makes trimming "unsafe" and compilation errors out
[7]. Dynamic dispatch is Julia's central feature, so this is a real tension:
the static-compilation story requires giving up the thing that makes the
language what it is. 1.13 widened what can be trimmed (finalizers,
`@cfunction`, `mapreduce`) but it remains experimental [8].

### Correctness bugs from unrestricted generic composition
Yuri Vishnevsky's May 2022 post "Why I no longer recommend Julia" [11] is the
most important external data point on Julia's design. It lists concrete
bugs he and others hit: `sum!`/`prod!`/`any!`/`all!` silently returning
wrong results (JuliaLang #39385), `prod` wrong for 8/16/32-bit integers
(#39183), `copyto!` not checking aliasing (#39460), biased sampling in
StatsBase (#642), wrong quantiles, an off-by-one in `dayofquarter`, and a
class of failures where OffsetArrays (arrays whose indices need not start at
1) were passed to code that loops `1:length(A)` under `@inbounds`, producing
out-of-bounds memory reads that either segfault or silently return garbage.
He also collects accounts (Patrick Kidger, Samuel Ainsworth) of months lost
to incorrect gradients from Zygote.jl. His diagnosis: Julia composes generic
code written by different people, but has **no formal interfaces**;
"generic functions often leave edge-case semantics unspecified" and the
community has not even agreed on what counts as a number.

The community response on Discourse [12] is instructive because it largely
conceded the point. Jakob Nissen: "Unfortunately, I think the author is
right", citing "no clear way of specifying what is internal behaviour of a
package/function/struct". John Myles White: "I fear the ever-growing push
for more generic code makes both performance and correctness worse with no
end in sight." Others pointed out that the `@inbounds for i in 1:length(A)`
code was eight years old and that `eachindex(A)` is the correct idiom,
which is true but is exactly the problem: the correct idiom is a convention,
not something the compiler checks. The survey added "No built-in support
for interfaces or traits" as a complaint in 2023; 17-23% select it [5].
Workarounds exist as packages (Interfaces.jl, BinaryTraits.jl, RequiredInterfaces.jl,
Aqua.jl and JET.jl for static checks), none standard.

### `@inbounds` and performance-over-safety defaults
`@inbounds` removes bounds checks in a block; it is pervasive in Base and
the ecosystem because bounds checks block SIMD vectorisation. The manual's
own example of correct `@inbounds` use was unsafe for offset arrays
(JuliaLang #39367, now fixed) [11]. Because the check is removed at the
source and not re-verified by anything, a wrong `@inbounds` is a silent
memory-safety hole in an otherwise memory-safe language. The pressure of the
latency problem has even pushed defaults the wrong way: in 1.13, `Pkg.test`
no longer forces `--check-bounds=yes` so that existing precompile caches can
be reused; users must opt back in [8].

### 1-based indexing, and the worse problem of optional index bases
Julia indexes from 1 to match mathematical convention and MATLAB/Fortran/R.
Issue #558 ("Why do Julia arrays use 1-based indexing?") was opened in
2012; Bezanson argued the choice "doesn't matter that much"; the issue was
locked in 2015 with "there's no real possibility of this being changed"
[13]. In the survey one-based indexing is *liked* by 17-22% and *disliked*
by 6-9% [5] - it is a mild preference for its audience, not a technical
disaster. The real damage came from making the base *optional*:
OffsetArrays.jl lets arrays start anywhere, the generic interface
(`axes`, `eachindex`, `firstindex`, `begin`) supports it, but the overwhelming
majority of code was written assuming `1:length(A)`. The result is an
interface that is simultaneously documented as the contract and violated
by most implementations, which is the mechanism behind many of
Vishnevsky's bugs [11][12].

### Garbage collection with no static memory story
Julia's GC is non-moving mark-and-sweep; it was single-threaded until 1.10
and "slow garbage collection" is a listed complaint (8-13%) [5]. More
fundamentally there is no language-level way to express ownership, stack
allocation of mutable structs, arenas or RC: performance-sensitive code is
written "allocation-free" by convention (`StaticArrays`, pre-allocated
buffers, `Bumper.jl` arenas, in-place `!` functions), and GPU kernels must
avoid allocation entirely. 1.13 improved full-collection cost by
permanently marking system- and package-image objects (`GC.gc()` after
`using GLMakie`: 187 ms to 68 ms) [8], but the design space for predictable
memory remains closed.

### Type instability as a hidden performance cliff
The speed promise depends on inference succeeding. Untyped struct fields,
non-constant globals, containers of abstract element type, or a function
that returns different types on different branches all silently fall back
to dynamic dispatch and boxing, with slowdowns of 10-100x and no compiler
error. "Writing performant code is difficult and requires specialized
knowledge" is selected by 11-20% [5], and the manual's "Performance Tips"
chapter is one of its longest. Tools (`@code_warntype`, JET.jl,
Cthulhu.jl) exist to find instabilities after the fact.

### Error messages, debugging and static analysis
"Confusing or overly verbose error messages" (22-36%), "debugger too slow
and/or not fully featured" (17-27%), and "poor linter / syntax checking
compared to statically typed languages" (13-27%) are persistent survey
complaints [5]. A `MethodError` on a deeply nested generic call prints
dozens of candidate signatures; because there are no declared interfaces,
the compiler cannot say *which* contract was violated. 1.10's new parser
(JuliaSyntax.jl) improved syntax error locations [10].

### Global method tables: type piracy and ambiguities
Because any package may add a method for any function on any type, a
package can redefine behaviour for types it does not own ("type piracy")
and change the semantics of unrelated code in the same session. Method
ambiguities between two packages that each extend `f` for one of two
arguments are detected only at call time. Aqua.jl exists to lint for both.
This is the other side of the expression-problem coin: the same openness
that lets packages compose lets them silently interfere.

### Confinement to the scientific niche
"There are not enough Julia users in my field or industry" is the top
non-technical complaint, chosen by 36-64% [5]. The survey itself had only
721 respondents in 2025 (682 in 2024), and respondents report Python usage
declining among themselves, which indicates a community that is deepening
rather than broadening. The reasons are the weaknesses above: startup
latency and binary deployment rule out CLI tools and most services, GC and
no ownership rule out systems work, the lack of interfaces and static
checking make large non-numerical codebases risky, and the standard library
(Base) is heavily numerical. The language is excellent at what its
designers do and was never adopted by people who do other things.

## Designer retrospectives
- Bezanson's JuliaCon 2019 talk "What's Bad About Julia" (a core-developer
  list of regrets; recalled from the talk, not re-verified in this research)
  named compile latency, the lack of static compilation, the amount of
  dynamic dispatch left in Base, immutability and aliasing semantics, and the
  absence of interfaces/traits as the main problems; the 1.9-1.13 roadmap
  is essentially that list.
- On indexing, Bezanson in 2012: Julia chose 1 to be "more similar to
  existing math software" and the topic "really isn't that interesting"
  [13]. The later OffsetArrays design implicitly concedes that a fixed base
  was the wrong thing to make optional.
- The 1.12 "world age for bindings" change [7] is an admission that the
  inability to redefine a struct in a running session was a long-standing
  usability defect of the compilation model.
- The core team's response to the 2022 correctness debate was not a
  language change but process: more composition testing, `eachindex`
  linting, Aqua/JET adoption, and 1.13's TTFX CI [8][12]. No interface
  feature has been added to the language as of 1.13.

## Influence
- **Copied from others**: multiple dispatch and generic functions from
  Common Lisp CLOS and Dylan; the type lattice from Dylan and ML; macros
  from Lisp; `missing` from R; array syntax and 1-based indexing from
  MATLAB/Fortran; the task scheduler from Cilk; Project/Manifest from
  Cargo and Bundler.
- **Copied by others**: Julia's `Project.toml`/`Manifest.toml` plus
  content-addressed artifacts is the clearest model for binary-dependency
  management in a package manager; its dot-broadcast syntax (`f.(x)`) is
  unique and has not spread; full multiple dispatch has still not been
  adopted by any mainstream language after Julia, which is weak evidence
  that its costs (no interfaces, global method tables) are felt. Mojo's
  original "Why Mojo" document discussed Julia as an inspiration whose
  non-Python syntax limited uptake (recalled; the page is no longer at its
  original URL).

## Lessons for Sisu
- **Adopt**:
  - Pkg's environment model: a direct-dependency file with compat bounds,
    a fully resolved lock file, stacked environments, and content-addressed
    artifacts for non-source dependencies [6].
  - Hygienic, visibly-marked AST macros operating on a first-class
    expression type; `@`-prefixed call sites keep DSLs readable.
  - Compile-latency and time-to-first-X benchmarks in CI from the first
    release [8]; Julia shows that if you do not measure it, it grows.
  - Index-agnostic iteration protocols (`eachindex`, `axes`) as the *only*
    way to iterate a container, so that a wrong assumption cannot be
    expressed.
- **Adapt**:
  - Dispatch on all arguments is genuinely valuable for mathematical code
    and for the expression problem; Sisu should allow multi-argument
    specialisation but *through declared traits/interfaces* so that each
    generic function has a checkable contract. Julia's experience is that
    generic composition without contracts produces silent wrong answers at
    scale [11][12].
  - Small-union optimisation (`Union{T, Nothing}`) is a good implementation
    strategy for Option/Result types; take the implementation, not the
    absence of a sum type.
  - `@inbounds`: unsafe bounds-check elision must be in a clearly marked
    unsafe region and ideally proven by the compiler (index derived from
    `eachindex`) rather than asserted by the programmer.
- **Avoid**:
  - A JIT-only compilation model with no AOT story; the deployment and
    latency problems defined Julia's public image for a decade and are
    still unsolved for code that uses dynamic dispatch [5][7].
  - Optional index bases. Pick one base (0 for a general-purpose language)
    and do not allow containers to override it.
  - Global, open method tables without ownership rules (type piracy) and
    without ambiguity checking at definition time.
  - Letting performance pressure erode safety defaults (1.13 `Pkg.test`
    dropping `--check-bounds=yes` [8]).
  - Type instability as a silent 100x cliff: a statically typed language
    avoids this by construction, which is a strong argument for static
    typing in Sisu.
- **Open questions raised**:
  - Multiple dispatch vs traits for the expression problem: Julia proves
    multimethods compose better than single-dispatch OOP, but has not
    found a way to add contracts after the fact. Can Sisu get
    multi-argument dispatch *and* definition-checked generics?
  - Can a language serve interactive, exploratory scientific use (REPL,
    redefinition, dynamic dispatch) and still produce small static
    binaries? Julia's `--trim` suggests the two modes may need different
    subsets of the language.
  - 1-based indexing is liked by Julia's audience and disliked by
    general programmers; if Sisu targets both, which audience loses?

## Sources
1. Bezanson, Karpinski, Shah, Edelman, "Why We Created Julia", 2012. https://julialang.org/blog/2012/02/why-we-created-julia/
2. S. Karpinski, "The Unreasonable Effectiveness of Multiple Dispatch", JuliaCon 2019 (talk, JuliaCon YouTube channel).
3. Julia manual, "Methods" (trait-based dispatch, design rationale), v1. https://docs.julialang.org/en/v1/manual/methods/
4. Julia v1.13.0 release announcement, Discourse, 10 Sep 2026. https://discourse.julialang.org/t/julia-v1-13-0-has-been-released/139326
5. A. Claster, "Julia User & Developer Survey 2025" (n=721, June-July 2025) with 2019-2024 tracking. https://julialang.org/assets/2025-julia-user-developer-survey.pdf
6. E. Saba, S. Byrne, "Pkg + BinaryBuilder - The Next Generation" (artifacts), 2019. https://julialang.org/blog/2019/11/artifacts/
7. "Julia 1.12 Highlights", October 2025. https://julialang.org/blog/2025/10/julia-1.12-highlights/
8. "Julia 1.13 Highlights", September 2026. https://julialang.org/blog/2026/09/julia-1.13-highlights/
9. "Julia 1.9 Highlights" (native code caching, TTFX table), May 2023. https://julialang.org/blog/2023/04/julia-1.9-highlights/
10. "Julia 1.10 Highlights", December 2023. https://julialang.org/blog/2023/12/julia-1.10-highlights/
11. Y. Vishnevsky, "Why I no longer recommend Julia", May 2022. https://yuri.is/not-julia/
12. Julia Discourse, "Discussion on 'Why I no longer recommend Julia' by Yuri Vishnevsky", May 2022. https://discourse.julialang.org/t/discussion-on-why-i-no-longer-recommend-julia-by-yuri-vishnevsky/81151
13. JuliaLang/julia issue #558, "Why do Julia arrays use 1-based indexing?", 2012-2015. https://github.com/JuliaLang/julia/issues/558
14. endoflife.date, Julia release lines (1.12.6, 1.10.11). https://endoflife.date/julia
