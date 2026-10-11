# Pony

## Snapshot
| | |
|---|---|
| First appeared | 2014 (design at Causality Ltd for financial systems); open-sourced 2015; Clebsch's PhD thesis "Pony: Co-designing a Type System and a Runtime", Imperial College, 2017 [5][6]. |
| Designed by | Sylvan Clebsch, with Sophia Drossopoulou (Imperial), Sebastian Blessing, Andy McNeil; later the ponylang core team (Sean T. Allen and others). |
| Current version (Oct 2026) | ponyc 0.74.0, 30 Sep 2026; four minor releases in September 2026 alone (0.71-0.74), several with breaking changes (new `net` package, PonyCheck API) [2]. Still pre-1.0 after 11 years. |
| Governance | Community-run ponylang organisation with RFC process; originally Causality Ltd. Clebsch left for Microsoft Research in 2017 [5]. |
| Typing | Static, nominal (classes, traits) and structural (interfaces), union/intersection types, generics, no null, *reference capabilities* on every type. |
| Memory | Per-actor tracing GC with the ORCA protocol (no stop-the-world, no barriers), plus actor cycle detection; pony runtime with its own scheduler [5]. |
| Compilation | AOT via LLVM (bundled LLVM 23.1.2 as of 0.74.0) [2]. |
| Primary domains | Low-latency, high-concurrency services (the origin is financial trading infrastructure) [5]. |
| Notable users | Wallaroo Labs' "Wally" stream-processing framework ("The primary API for Wally is written in Pony"; 1.5k stars, 5,383 commits) [10]. Beyond that, no notable users could be verified. |

## Stated design goals
From the ORCA paper: "Pony is an object-oriented programming language designed
from the ground up to support low-latency, highly concurrent applications
written in the actor model"; the requirements were "scalable concurrency, from
tens to thousands of concurrent components", "performance approaching that of
low-level languages" and "ease of development"; Erlang was rejected for
performance and Java because "pauses due to garbage collection were a stumbling
block" [5]. The website's one-liner: "an open-source, object-oriented,
actor-model, capabilities-secure, high-performance programming language" [1].
The deeper bet, stated in the thesis title, is *co-design*: the type system
guarantees data-race freedom so the runtime can skip locks and barriers [5][6].

## The central bet: reference capabilities

Every reference carries one of six capabilities. They are best understood as
*denials*: "reference capabilities are about denying you the ability to do
things with memory references" [3]. The tutorial's guarantees [4]:

| Cap | Kind | Denies to other aliases in *this* actor | Denies to *other* actors | Sendable |
|---|---|---|---|---|
| `iso` | mutable | read and write | read and write | yes |
| `trn` | mutable | write | read and write | no |
| `ref` | mutable | nothing | read and write | no |
| `val` | immutable | write | write | yes |
| `box` | immutable | write *or* other-actor read, not both | - | no |
| `tag` | opaque | nothing ("makes no guarantees about other variables at all") | nothing | yes |

The matrix (rows: local denial, columns: global denial) has `iso` at the
strictest corner and the sendable capabilities on its diagonal; "global
properties are as restrictive or more restrictive than the local properties"
[4]. Only `iso`, `val` and `tag` may cross an actor boundary, which is what
makes zero-copy message passing race-free [4][5].

The supporting machinery is where the cost lives:
- **Aliasing**: `iso` "can only be aliased as `tag`", `trn` only as `box`.
  Alias types are written `T!` ("whatever type this can safely be aliased
  as": `iso!` is `tag`) [8].
- **Ephemeral types** `T^`: the result of `consume a` or of a constructor,
  "because the value no longer has a name" [8].
- **consume and destructive read**: `var b: Wombat iso = consume a` moves;
  fields cannot be consumed, so moving out of a field uses the assignment-
  returns-old-value trick `var b: Wombat iso = buddy = consume a` [7].
- **recover**: inside `recover ... end` only sendable values of the enclosing
  scope are visible, so the result can be lifted to `iso` or `val` [9].
- **Automatic receiver recovery**: calling a `ref` method on an `iso` is
  allowed if all arguments are sendable and the result is sendable or unused
  [9].
- **Viewpoint adaptation** (arrow types): `fun friend(): this->Wombat`
  returns a `Wombat` seen through the receiver's capability so the function
  need not be written three times; `box->A`, `N->A` for generics [11].

```pony
class Wombat
  var _friend: Wombat
  fun friend(): this->Wombat => _friend     // viewpoint adaptation

actor Main
  new create(env: Env) =>
    let w: Wombat iso = recover Wombat end // iso: no other alias anywhere
    let s = recover String end             // String iso
    s.append("hi")                          // automatic receiver recovery
    let v: String val = consume s           // freeze: now shareable
    Worker.go(v)                            // send immutable data by reference
```

### Per-actor GC: ORCA
"Orca is a concurrent and parallel garbage collector for actor programs, which
does not require any stop-the-world steps, or synchronisation mechanisms, and
which has been designed to support zero-copy message passing and sharing of
mutable data" [5]. Each actor collects its own heap during its idle time;
objects shared with other actors are tracked with "deferred reference counts"
piggy-backed on messages; "Orca relies on the absence of race conditions in
order to avoid read/write barriers" [5]. Because counts model "actors'
interest in an object, as opposed to actual reference topologies, cycles are
not" a problem for objects; actors themselves need a separate cycle detector
(0.73.0 fixed a "cycle detector memory leak for mutually referencing actors")
[2][5]. The evaluation used micro-benchmarks against Erlang and Akka (with the
C4 collector) and the authors concede it "has the limitations one would expect
from a new language, namely few benchmarks" [5].

### Errors without exceptions
`error` raises an untyped error; functions that may raise are *partial* and
marked `?` in the signature and, since 0.16.0, at every call site
(`factorial(x - 1)?`); `try ... else ... then ... end` handles it; errors carry
"no type or instance", which "simplifies error handling and gives better
runtime performance" [12]. Actor behaviours and constructors cannot be partial
because the caller has already gone [12].

## Strengths

### Data-race freedom proven at the type level, with mutable sharing
Unlike Erlang (copy everything) or Rust (`Send`/`Sync` plus `Mutex`), Pony
lets actors hand each other *mutable* data by reference and still has no
locks, because `iso` guarantees uniqueness and `val` guarantees immutability.
The formalisation ("Deny capabilities for safe, fast actors", AGERE 2015, and
"A Principled Design of Capabilities in Pony") is the evidence; the runtime's
barrier-free GC is the payoff [5][6].

### No pauses, no locks, no null, no exceptions-with-unwinding-cost
The runtime has no global GC phase, a work-stealing scheduler, and the
language has union types instead of null and partial functions instead of
exceptions, so "an error can't crash the program" unhandled [12].

### Capabilities-secure in the object-capability sense
Ambient authority is passed in through `Env` to `Main`; 0.71.0 even made the
terminal's raw mode require a `TerminalAuth` [2]. This is the same philosophy
as Austral's capabilities, implemented through the object model.

### A research lineage that keeps paying out
Pony's ideas fed directly into Microsoft Research's Project Verona and, via
Verona, into the 2025 PLDI paper "Dynamic Region Ownership for Concurrency
Safety" co-authored by Clebsch with Guido van Rossum, Eric Snow and Brandt
Bucher, which is now being prototyped for CPython ("Lungfish"), presented at
the Python Language Summit in May 2025 [14][15][16].

## Weaknesses

### The capability lattice is hard to learn and hard to use
This is the weakness everyone, including the designers, converges on. The
tutorial itself says the matrix "is a mnemonic, not a complete rule set. Some
cases, such as viewpoint adaptation, don't fit it, so the compiler's behavior
is the authority" [4]. A working Pony programmer must juggle six capabilities,
two modifiers (`!`, `^`), `consume`, destructive reads, `recover`, automatic
receiver recovery and arrow types, and the error messages reason about all of
them at once. Designer-level evidence: Verona's FAQ asks whether "linear
regions" can "replace per-object linearity", i.e. whether one capability per
*region* can replace one per *reference* [14]; the 2024 OOPSLA paper
"Reference Capabilities for Flexible Memory Management" (Clebsch, Drossopoulou,
Parkinson, Wrigstad, Noble and others) revisits the design [15]; and the
Python work is "based completely on dynamic checks" precisely because static
capabilities were judged unacceptable for Python programmers [16]. I could not
run a systematic survey of forum discourse with the tools available; the
above is designer evidence plus the tutorial's own caveats.

### Viral `iso` and the "isolation tax"
To send mutable data you must build it isolated (inside `recover`, touching
only sendable values) and `consume` it on every hand-off. Reading one field
out of an `iso` structure to keep locally forces a copy or a `tag`. Patterns
such as a mutable cache shared by two actors are impossible by design; the
only shared mutable thing is an actor, so shared state becomes message
round-trips.

### Untyped errors
`error` carries no payload; recovering *why* something failed requires
returning union types instead, which the standard library does
inconsistently. The 0.74.0 notes still touch this area ("`Randomness` draw
methods no longer raise errors") [2].

### Eleven years pre-1.0 with routine breaking changes
Weekly releases in September 2026 rewrote the networking stack ("Existing
networking code must be rewritten") and the test framework [2]. A language
whose selling point is reliability cannot also be the one whose stdlib churns.

### Why it did not take off
The verifiable facts: the designer moved to Microsoft Research in 2017 and
started Verona (the Verona FAQ names Pony as an inspiration alongside Rust
and Cyclone) [5][14]; the one flagship production user, Wallaroo Labs' stream
processor, is a mono-repo whose company later moved to ML-ops products (the
pivot is my recollection and could not be verified from primary sources);
there is no corporate sponsor, no package registry of note, and the Vale
author's taxonomy recommends Pony only for "high-reliability systems" and
"strong concurrency" web servers [13]. The lesson is not that the type system
is wrong—Verona and CPython are still mining it—but that a high-ceremony
static discipline for *all* references sold to a general audience found too
few buyers. Swift 6's strict concurrency retreat (covered by the Swift
researcher) is the mainstream replay of the same lesson.

## Designer retrospectives
- ORCA paper (2017): the evaluation "has the limitations one would expect
  from a new language, namely few benchmarks"; "Given that Pony is still a
  young language with a small set of users" [5].
- Verona FAQ: Verona is "inspired by ideas from other languages", "Many of the
  ideas we built on have been popularised by Rust, such as borrowing and
  linearity" and Pony's reference capabilities; the research question is
  whether linear *regions* can replace per-object linearity, and whether
  "language level regions" can support compartmentalisation [14].
- Verona's regions idea, as far as its public material states it: a region is
  a group of objects with one external owner; isolation is per region rather
  than per object; "Behaviour-Oriented Concurrency" (OOPSLA 2024) replaces
  actors with *cowns* (concurrent owners) and `when` blocks that "enable
  asynchronously creating atomic and ordered units of work" over several
  cowns at once, addressing the actor-model pain that "updating multiple
  actors as a single atomic operation is a challenging task" [17]. The PLDI
  2025 Python work enforces the region discipline dynamically: "Ownership can
  turn what would have been a race into a deterministic failure that can be
  explained to the programmer" [16].
- Tutorial on errors: the 0.16.0 change making `?` mandatory at call sites
  was so "readers can see where control may jump" [12]: an admission that
  invisible partiality was a mistake.

## Influence
- **Took from**: Erlang (actors, no shared state), E and object-capability
  security, Cyclone, Clarke/Wrigstad's external uniqueness ("External
  Uniqueness Is Unique Enough", ECOOP 2003, cited by Verona), Gordon et al.'s
  "Uniqueness and Reference Immutability for Safe Parallelism" (OOPSLA 2012,
  Microsoft's earlier C# experiment) [15].
- **Gave to**: Project Verona (regions, cowns, BoC) and the CPython region-
  ownership prototype [14][16][17]; Vale's taxonomy credits Pony's `iso` as
  the origin of its region idea [13]; Swift's `sending`/`Sendable` and
  region-based isolation (SE-0414, "Region based isolation", 2023) are the
  same shape of analysis, cited by the Swift team as inspired by this
  literature (verification left to the Swift researcher).

## Lessons for Sisu
- **Adopt**: The *concept* of three sendable states: unique (`iso`),
  deeply immutable (`val`), opaque identity (`tag`). Any Sisu concurrency
  story that moves data between threads needs exactly these three, and Pony
  proves they suffice for lock-free, barrier-free, zero-copy actors.
- **Adopt**: Per-domain garbage collection as a design option: ORCA shows that
  if the type system rules out races, each isolated heap can be collected
  independently with no global pause. If Sisu has any tracing GC at all, it
  should be scoped to isolation domains, not the whole process.
- **Adopt**: Mandatory call-site marking of partial calls (`f()?`), which
  Pony adopted after shipping without it.
- **Adapt**: Do *not* put a capability on every reference. Follow Verona's
  conclusion: isolation per region/domain, with ordinary aliasing inside.
  Sisu's default world should be the single-threaded `ref` world; `iso`/`val`
  appear only at the boundary where a value leaves its domain. This keeps the
  Pony guarantee at thread boundaries without Pony's everyday ceremony.
- **Adapt**: `recover` and viewpoint adaptation are solving real problems
  (building an isolated value; writing getters once). Sisu should solve them
  with inference (a value whose construction touched only sendable data is
  sendable) rather than explicit syntax.
- **Avoid**: Untyped errors. Pony's `error` without payload pushed error
  information into ad-hoc union returns.
- **Avoid**: Pre-1.0 forever. Pony's stability story undercut its reliability
  pitch; Sisu should define its 1.0 criteria before features.
- **Open questions raised**: (1) How much static concurrency safety do
  developers tolerate? Pony (all references), Swift 6 (all actor crossings,
  later relaxed), Verona (regions), Python/Lungfish (dynamic checks) form a
  descending ladder; where Sisu sits depends on whether it targets systems
  programmers or everyone. (2) Is BoC's multi-cown `when` the right primitive
  for Sisu's structured concurrency, since it fixes the actor model's
  multi-object atomicity problem? (3) Can "shared mutable cache between
  domains" be expressed without reintroducing locks?

## Sources
1. Pony website, https://www.ponylang.io/discover/ (2026).
2. ponyc releases (0.71.0-0.74.0, Sep 2026), https://github.com/ponylang/ponyc/releases.
3. Pony tutorial, "Reference Capabilities" overview, https://tutorial.ponylang.io/reference-capabilities/ (2026).
4. Pony tutorial, "Reference Capability Guarantees" and "Reference Capability Matrix", https://tutorial.ponylang.io/reference-capabilities/guarantees.html and .../capability-matrix.html (2026).
5. S. Clebsch, J. Franco, S. Drossopoulou, A. M. Yang, T. Wrigstad, J. Vitek, "Orca: GC and Type System Co-Design for Actor Languages", OOPSLA 2017, https://www.ponylang.io/media/papers/orca_gc_and_type_system_co-design_for_actor_languages.pdf.
6. Pony papers page (Clebsch's thesis; "Deny Capabilities for Safe, Fast Actors", AGERE 2015; "A Principled Design of Capabilities in Pony"; "Fully Concurrent Garbage Collection of Actors on Many-Core Machines"), https://www.ponylang.io/learn/papers/.
7. Pony tutorial, "Consume and Destructive Read", https://tutorial.ponylang.io/reference-capabilities/consume-and-destructive-read.html.
8. Pony tutorial, "Aliasing", https://tutorial.ponylang.io/reference-capabilities/aliasing.html.
9. Pony tutorial, "Recovering Capabilities", https://tutorial.ponylang.io/reference-capabilities/recovering-capabilities.html.
10. Wallaroo Labs, "Wally" repository, https://github.com/WallarooLabs/wally.
11. Pony tutorial, "Arrow Types aka Viewpoints", https://tutorial.ponylang.io/reference-capabilities/arrow-types.html.
12. Pony tutorial, "Errors", https://tutorial.ponylang.io/expressions/errors.html.
13. E. Ovadia, "Memory Safety Approaches and Development Velocity", https://verdagon.dev/blog/when-to-use-memory-safe-part-2 (2023).
14. Project Verona FAQ, https://microsoft.github.io/verona/faq.html (2026).
15. Project Verona publications page, https://microsoft.github.io/verona/publications.html (2026).
16. F. Stoldt, B. Bucher, S. Clebsch, M. Johnson, M. J. Parkinson, G. van Rossum, E. Snow, T. Wrigstad, "Dynamic Region Ownership for Concurrency Safety", PLDI 2025; and the Verona "Lungfish"/Python page, https://microsoft.github.io/verona/pyrona.html.
17. L. Cheeseman, M. J. Parkinson, S. Clebsch et al., "When Concurrency Matters: Behaviour-Oriented Concurrency", OOPSLA 2024, https://www.microsoft.com/en-us/research/publication/when-concurrency-matters-behaviour-oriented-concurrency/.
