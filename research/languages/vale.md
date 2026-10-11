# Vale

## Snapshot
| | |
|---|---|
| First appeared | Private work from January 2013 ("VLang", then "GelLLVM"); public announcement 2020; last release v0.2 on 10 May 2022 [1][2]. |
| Designed by | Evan Ovadia (verdagon), essentially alone. |
| Current version (Oct 2026) | v0.2 (2022) remains the last release. The repository README now reads: "Vale is archived and no longer being worked on, but its spirit lives on in the new Valen programming language" [1][2]. Ovadia worked on the Mojo compiler team from July 2024 to December 2025 and announced Valen's "golden spike" milestone on 17 Sep 2026 [3][4]. |
| Governance | Single author; Apache-2.0; ~2.0k stars, 1,185 commits [1]. |
| Typing | Static, nominal, generics with "concept functions"; "Higher RAII" linear types (opt-in). |
| Memory | Single ownership plus *generational references* (runtime generation check on dereference), with planned region borrow checking to elide checks; no GC, no RC, no borrow checker on ordinary references [1][5]. |
| Compilation | AOT via LLVM. |
| Primary domains | Intended for games and servers ("high-level" systems language); only the author's own 7DRL roguelike and samples exist [7]. |
| Notable users | None. |

## Stated design goals
Vale aimed to be "fast, safe, and easy", specifically "memory-safe single
ownership without garbage collection or a borrow checker" [1][2]. Ovadia's
broader thesis, argued across his blog, is that language designers should
optimise *development velocity*, "how fast we can expand, modify, and
maintain our codebase to deliver value to the user", and that borrow checking
pays for its safety with velocity in stateful, interconnected domains [6].
Vale was the attempt to get Rust-class safety without Rust-class
restrictions.

## The central bets

### Generational references
Each heap allocation carries a *generation* counter; a reference is a
(pointer, remembered generation) pair; "always call `__check` before
dereferencing a generational reference": if the allocation's current
generation differs, the object was freed and the program halts [5]. On free,
the generation is bumped (or, in the "random generational references"
variant, a 64-bit pseudo-random tag is overwritten with zero), so stale
references can never silently alias new objects [5][8]. Costs: "It
occasionally adds an 8-byte generation number to the top of allocations";
the check is 4-5 instructions; the single published benchmark (a terrain
generator) measured unsafe 43.82 s, reference counting 54.90 s (+25.29 %),
generational references 48.57 s (+10.84 %), i.e. "less than half the cost of
reference counting" [5]. Owned values and moves need no check ("Moving
ownership into a function... avoids checks entirely") [5]. Ovadia's taxonomy
lists this as approach 8 of 17, compares the random-tag variant to ARM memory
tagging "but much more reliable because of a wider tag (64 bits instead of
4)", and notes Rust re-implementations (`genref`, `generational-box`) [8].

### Regions and region borrow checking (prototyped, never shipped)
Regions "group data that can be frozen (immutable) or unfrozen (mutable) as a
unit"; "while a region is immutable it's much faster to read from" because
the compiler drops generation checks. A `pure` function tells the compiler
"its parameters are in an immutable region, which eliminates every single
generation check in that function"; region markers (`<r'>`) extend this to
non-pure callees; *isolates* (`'Cannon`) are hierarchies where "nothing inside
can point out, and nothing outside can point in, except for the owning
reference" [9]. The key difference from Rust: regions "don't impose the same
aliasability-xor-mutability restrictions as borrow checking", are opt-in, and
"restrictions don't spread to callers" [9]. Milestones were announced in
March-July 2023 ("only one [stage] left") and then the project went quiet;
parts 2-5 of the design were "a preview, not implemented" [2][9].

### Higher RAII (linear types for liveness)
Ordinary RAII guarantees a destructor runs; Higher RAII makes the compiler
require that *a specific function with arguments* is eventually called:
"ensure that we eventually call any function". A struct marked
`#!DeriveStructDrop` cannot be implicitly dropped, so forgetting
`SetValueAndDestroy(promise, v)` or `remove(map, token)` is a compile error
("No function named `drop` exists!"); in the author's roguelike it caught a
forgotten cache removal [7]. Unlike C++/Rust, such objects "can't be dropped"
during unwinding or panic [7]. This is Austral's use-once rule applied
selectively (see `austral.md`).

### Ovadia's taxonomy of memory-safety approaches
The "grimoire" article is the most complete practitioner map of the design
space and worth summarising because it is directly usable [8]. Baselines:
tracing GC ("easy, flexible, has high throughput" but "more energy, more
memory, and has nondeterministic pauses"), reference counting ("slow and can
leak cycles"), borrow checking ("can cause complexity and can't do patterns
like" observers and intrusive structures). Then seventeen approaches:
(1) move-only programming (Rust, Austral, Vale's linear-aliasing style);
(2) RC with immutable region borrowing (Koka's Perceus noted; combination
"no language has done yet"); (3) borrow checking with linear types (Austral,
Higher RAII); (4) arena-only (Zig, Odin, Cyclone, Verona); (5) Ada/SPARK scope
rules; (6) regions (Pony `iso`, Forty2, Verona); (7) stack arenas; (8)
generational references; (9) random generational references; (10) "MMM++"
(global-array slot reuse, TigerBeetle-like); (11) tracing GC with per-actor
worlds (Pony ORCA, Verona); (12) interaction nets (HVM); (13) constraint
references (assert no references at destruction); (14) linear reference
counting (static-rc); (15) "not-MVS" (exactly one reference per object, lent
to calls; distinct from Hylo's MVS); (16) CHERI; (17) never free. The
companion velocity essay ranks the mainstream four for day-to-day cost:
GC is "probably the best approach for developer velocity", RC is "nearly as
good", borrow checking is "much slower" because of "extra constraints",
"forced coupling", "unsupported patterns" (graphs, observers, back-references,
delegates, intrusive data structures) and "leaky abstractions" (a `&mut`
requirement propagating up the call chain), while manual management is
"notoriously difficult" without arenas and sanitisers [6]. Evidence cited:
Google Earth had only 3-5 % memory-safety bug reports with ASan available;
CHERI could mitigate "at least two thirds" of Microsoft's 2019 memory-safety
issues at 6.8 % overhead; ASan "increases run-time by 73 %". The author labels
the Rust-velocity claims "just everyday people's experiences" and asks for a
controlled experiment [6].

## Strengths

### Memory safety with ordinary aliasing
Generational references allow arbitrary graphs, observers and back-references
with single ownership and no `Rc<RefCell<>>`; the cost is a bounded,
predictable check rather than a type-system restriction. The 10.84 % figure,
though from one benchmark, is a plausible price for "write it like C++ but it
cannot corrupt memory".

### A principled story for eliding checks
Owned values, moves, `pure` functions and immutable regions remove checks
exactly where data is provably not freed, so the model degrades gracefully
towards zero-cost rather than requiring zero-cost up front [5][9].

### Liveness guarantees nobody else offers
Higher RAII gives "must eventually call" obligations at compile time, which
neither destructors (C++/Rust) nor finalisers (GC languages) can express [7].

### Honest, quantified design writing
Ovadia's essays measure costs, list the patterns each approach cannot do and
admit where he was wrong (he "corrected an earlier claim about MVS") [8]. The
taxonomy alone is a better map than most surveys.

## Weaknesses

### The project stalled and the successor abandons the central mechanism
Vale is archived; Valen uses "group borrowing, which is purely compile-time,
instead of Vale's generational references plus region borrowing", targets
Rust interop through a patched `rustc`, and in Valen "Generational references
are temporarily disabled" [3][4]. Ovadia says Vale's "memory-safety model
changed several times, which would confuse people if he changed it again"
[3]. Valen itself is "barely past the proof-of-concept stage" with 497 stars
[4]. Generational references therefore never received a second, independent
benchmark or a real workload.

### Runtime failure instead of compile-time rejection
A stale reference halts the program. Ovadia's own taxonomy notes for the
related "constraint references" approach that "Halting in release mode is
often undesirable" [8]; the same applies here. This is safety in the sense of
"no memory corruption", not "no crash".

### Determinism and interop constraints
Random generational references "require perfect determinism" for the tag
scheme to be reliable, and an 8-byte header on allocations breaks layout
compatibility with C structs unless inline data is handled specially [5][8].

### The regions design was never validated
Everything beyond the basic check (immutable regions, isolates, cells,
"hybrid-generational memory") remained design posts and partial prototypes;
the 2023 milestone list stops at "only one left" [2][9]. The claim that the
blend "could have as little memory overhead as borrow checking in practice"
is a hope, explicitly [9].

### Group borrowing (Valen) is also unproven
"This model hasn't been implemented yet and therefore hasn't been proven to
work"; it has "no unique references", so `swap` must prove `distinct(x, y)`
and the compiler "may struggle to emit faster `noalias` pointers" [10].

## Designer retrospectives
- On Vale: "my greatest achievement, my own personal crown jewel of memory
  safety design", but the model changed too often to change again; Valen is
  "different enough from Vale to deserve its own name" and is "a systems
  language, not a high-level one like Vale", "less strictly safe, since it
  includes unsafe code like Rust" [3].
- On terminology: he "admits regrets over Vale's use of the term 'regions'"
  because Verona and others use it differently [10].
- On zero-overhead: "any non-trivial stateful program is guaranteed to have
  some memory safety overhead" [9]; see also his "Chasing the Myth of
  Zero-Overhead Memory Safety" [11].
- On evidence: wants "an actual controlled experiment" on borrow checking and
  velocity rather than anecdote [6].

## Influence
- **Took from**: Rust (single ownership, moves), Pony's `iso` and Verona
  (regions), Austral (linear types for Higher RAII), Lobster/Nim (RC with
  optimisation), game-engine "handle + generation" slot maps (the same idea
  predates Vale in entity-component systems).
- **Gave to**: Rust crates `genref` and `generational-box` reimplement the
  mechanism [8]; Ante's blend of borrowing and RC is discussed in dialogue with
  Vale's ideas [11]; the taxonomy is widely cited in language-design
  discussions. No shipped language adopted generational references as its
  primary mechanism.

## Lessons for Sisu
- **Adopt**: Generation-checked references as the *second tier* for the
  patterns MVS and borrowing cannot express (observers, back-references,
  graphs, long-lived handles), costed at roughly 5-11 % on the one benchmark
  available and elidable where ownership is static. This is a credible
  "escape hatch that is still memory-safe", better than `unsafe`.
- **Adopt**: Ovadia's taxonomy as the checklist for Sisu's memory topic
  synthesis; in particular, list for each candidate mechanism which patterns
  it *cannot* express (his "unsupported patterns" test).
- **Adapt**: Higher RAII as opt-in linear types on specific resource types,
  not a global discipline (same conclusion as `austral.md`).
- **Adapt**: Region-based check elision is promising but should be derived
  from Sisu's existing isolation/concurrency domains rather than a separate
  region annotation system; Vale's experience shows a stand-alone regions
  design is a multi-year project with uncertain payoff.
- **Avoid**: Changing the core memory model repeatedly in public. Vale lost
  its audience to three model changes and a pause; Sisu must pick the tiered
  model early and keep it.
- **Avoid**: Single-maintainer bus factor for the compiler of a memory-model
  experiment; Vale paused when its author took a job.
- **Open questions raised**: (1) Is halting on a stale generation acceptable
  for Sisu's target domains, or must stale access be a recoverable error?
  (2) Can generation checks be elided well enough in hot loops without a
  separate region type system? (3) Does group borrowing (mutable aliasing
  within a group, invalidation only of child groups) offer a third way
  between MVS and generational checks worth tracking as Valen matures?

## Sources
1. Vale repository README (archived notice, history), https://github.com/ValeLang/Vale (2026).
2. Vale website (status, roadmap milestones, v0.2 download), https://vale.dev/ (2026).
3. E. Ovadia, "The Golden Spike, and Resurrecting the Vale(n) Programming Language", https://verdagon.dev/blog/golden-spike-reviving-vale-valen (17 Sep 2026).
4. Valen repository README, https://github.com/valen-lang/valen (2026).
5. E. Ovadia, "Generational References (Updated)", https://verdagon.dev/blog/generational-references (2021, updated 2023).
6. E. Ovadia, "Memory Safety Approaches and Development Velocity", https://verdagon.dev/blog/when-to-use-memory-safe-part-2 (2023).
7. E. Ovadia, "Higher RAII, and the Seven Arcane Uses of Linear Types", https://verdagon.dev/blog/higher-raii-7drl (2022).
8. E. Ovadia, "Borrow checking, RC, GC, and the Eleven (!) Other Memory Safety Approaches", https://verdagon.dev/grimoire/grimoire (2023).
9. E. Ovadia, "Zero-Cost Borrowing with Vale Regions (Preview)", https://verdagon.dev/blog/zero-cost-borrowing-regions-overview (2023).
10. E. Ovadia, "Group Borrowing: Zero-Cost Memory Safety with Fewer Restrictions", https://verdagon.dev/blog/group-borrowing (2025).
11. E. Ovadia, blog index (Mojo tenure July 2024-December 2025; "Chasing the Myth of Zero-Overhead Memory Safety"; "Ante: A New Way to Blend Borrow Checking and Reference Counting"), https://verdagon.dev/ (2026).
