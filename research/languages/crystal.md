# Crystal

## Snapshot
| | |
|---|---|
| First appeared | Development began 2011-2012 at Manas.Tech (Buenos Aires); first public release 2014; 1.0 on 22 March 2021 |
| Designed by | Ary Borenszweig, Juan Wajnerman, Brian J. Cardiff (Manas.Tech) |
| Current version (Oct 2026) | 1.21.0 (16 July 2026), the release that turns execution contexts on by default; 1.20.x in May 2026 |
| Governance | Core team at Manas.Tech, with 84codes as principal sponsor since 2021; RFC repository (`crystal-lang/rfcs`) for larger changes; 1.x backwards-compatibility promise |
| Typing | Static, with *global* type inference: method bodies need no annotations, union types arise from control flow, `Nil` is an ordinary type in unions |
| Memory | Boehm-Demers-Weiser conservative tracing GC |
| Compilation | AOT via LLVM; whole-program, every build re-analyses the whole program |
| Primary domains | Web services, CLI tools, message brokers |
| Notable users | 84codes (LavinMQ message broker, CloudAMQP tooling), Lucky and Kemal web frameworks, Invidious |

## Stated design goals
Crystal "began as an experiment to see what compiled Ruby would look like"
(Martin Verzilli) and its core goals are "type safety, type inference, and
syntax" (Brian Cardiff) [1]. The bet was that Ruby's syntax and feel could be
kept while adding static types, native compilation and C-level performance,
and that the types could be *inferred globally* so that Ruby programmers
would never have to write them.

## Strengths

### Ruby syntax with static types, without annotations
Programs look like Ruby (blocks, `do ... end`, `?` and `!` methods, open
classes, `Struct`-like `record`) but compile to native code 10-100x faster
than MRI. Because inference is global, `def add(x, y); x + y; end` is a
valid, fully typed method: it is instantiated per concrete argument-type
combination, like a template [2]. For small and medium programs this
delivers the promised "Ruby without the slowness".

### Union types and flow typing as the nil-safety mechanism
A variable that may be `nil` has type `String | Nil`; calling `.size` on it
is a compile error until an `if x` / `x.try` / `case x` narrows it. Nil
safety therefore comes from ordinary union types plus flow-sensitive
narrowing, not from a special Option type. The same machinery gives
ad-hoc sum types (`Int32 | String`) and `case ... in` exhaustiveness.
Crystal 1.21 tightened `when A, B` to mean `is_a?(A) || is_a?(B)` rather
than a union test [4]. This design is simple to learn and was later echoed
by TypeScript's narrowing and Kotlin's smart casts (independently).

### Compile-time macros over the AST
`{{ ... }}` macros run at compile time with access to the program's types
(`@type.instance_vars`, `TypeNode#all_methods` as of 1.21 [4]), which is
how serialisation (`JSON::Serializable`), ORMs and DSLs are built without
runtime reflection. Cardiff notes macros can express type operations the
grammar cannot and that the macro language "can't be blended with the
language's own grammar" [1], i.e. it is a separate template language.

### Fibers, channels and a well-designed execution-context model
Concurrency is CSP-style: `spawn` creates a fiber, `Channel(T)` and
`select` coordinate them. RFC 0002 (February 2024, released July 2026)
replaces the pinned-thread `preview_mt` with *execution contexts*,
"drawing inspiration from Kotlin contexts": a **concurrent** context
(single thread, no parallelism, cheap synchronisation), a **parallel**
context (work-stealing M:N scheduling that auto-scales), and an
**isolated** context (one fiber owning one thread, for GUI/game loops and
blocking C libraries) [5][6]. The interface is designed to be public so
third parties can write schedulers. Designing the *interface* rather than
one scheduler is the transferable idea.

## Weaknesses

### Global type inference costs whole-program compilation
This is Crystal's defining trade-off and its designers are candid about it.
Borenszweig (2023): "the compiler compiles the entire program, from
scratch, every time you invoke it"; semantic analysis always covers your
code, every shard and the standard library, and only object-file emission
is cached [2]. Per-file compilation is impossible because `def add(x, y)`
cannot be typed without its callers, `Foo.bar` may refer to a class defined
in a file never explicitly required, and monomorphisation creates hidden
dependencies (`io.cr` ends up depending on `foo.cr` because `IO#<<` called a
user-defined `to_s`) [2]. Verzilli in 2017: "the type inference magic of
Crystal depends on knowing all the source code ahead of time, and that
makes it difficult to identify proper compilation units"; he was unsure
incremental compilation was feasible "without a big change to the
language" [1]. A forum user reports ~3 min 50 s for a 241k-line project in
debug and ~60 min with `--release --single-module` [3]. Incremental
compilation has been an open sponsorship item since 2019 and remains
unsolved in 1.21. The only known way out, visible in the comments on
Borenszweig's post, is explicit type annotations on public APIs or module
boundaries [2], which is exactly what the design set out to avoid.

### No separate compilation means no stable library interfaces
Because every method is re-instantiated per program, a shard has no
compiled form, no ABI, and no checked interface; a shard that compiles in
its own tests may fail to compile when a user passes a new type. Error
messages point into library bodies. This is the same "composition without
contracts" failure Julia exhibits, caused here by inference rather than
by dynamic typing.

### Multithreading arrived twelve years late and is still off by default
`preview_mt` shipped in 0.31 (2019) with fibers pinned to threads,
round-robin placement, no work stealing, and an explicit statement that
shared unions, tuples and `Array`s were unsafe ("interleaved writes can
produce invalid union representations, which can cause null-pointer
crashes") [7]. 1.0 (2021) listed multithreading as "preliminary" [8].
Execution contexts (1.21, 2026) fix the scheduler but the default context
still has parallelism 1 "to avoid a breaking change to existing
applications that may not be ready for MT" [5][6]. The RFC itself says
multi-threaded-by-default "may need to wait for a major release" [6].
There is no data-race prevention in the type system: the guidance is
"protect shared resources or, preferably, communicate through channels"
[6]. A language that was single-threaded for its first decade accumulated
a standard library and ecosystem that assume it.

### Conservative GC with no alternative
The BDW collector is conservative and stop-the-world; in 2017 "the GC is
staying" and Cardiff was "not confident enough to adopt a Rust-style
ownership model" [1]. Its thread support was a measured bottleneck in the
2019 MT work [7].

### Funding and size
In 2017 the core team was "about 5 or 6 Manas.Tech developers" plus three
outside members, who estimated "about 1,400 more hours" to a 1.0 they would
be proud of and called sustainable funding "the main non-technical
challenge" [1]. 1.0 came four years later, after 84codes sponsorship; 1.21
lists 21 contributors [4]; Windows support arrived only in 1.x [8].

## Designer retrospectives
- Borenszweig's three-part "Incremental compilation for Crystal" (2023) is
  an honest post-mortem of the global-inference decision: the algorithms
  are fast, but the design means compile time grows with the ecosystem,
  and no solution is in sight without annotations [2].
- Verzilli (2017): compile time is "very important" and "one reason
  development slowed"; the open question is "whether the team must make
  breaking language changes to keep compile times acceptable" [1].
- The execution-context RFC [6] reads as a retrospective on `preview_mt`:
  pinned fibers, round-robin dispatch and non-resizable thread pools were
  design errors, and the same-thread assumption had leaked into the
  standard library (`ThreadLocalValue`, `IO::Evented`).

## Influence
- **Copied from others**: Ruby's syntax and object model; Go's channels
  and `select`; Kotlin's coroutine contexts; TypeScript-style union
  narrowing (contemporaneous); C's `lib` declarations.
- **Copied by others**: Crystal is the reference negative example for
  global type inference in a compiled language; no later language has
  adopted whole-program inference without annotations at module
  boundaries. Its union-type nil handling is a clean demonstration that
  `T | Nil` plus flow typing can replace `Option` for most purposes.

## Lessons for Sisu
- **Adopt**:
  - Nil as an ordinary member of a union type with flow-sensitive
    narrowing; it reads better than `Option` for the common case and
    composes with sum types.
  - The execution-context *interface* (concurrent / parallel / isolated
    contexts, fibers that inherit their parent's context, a public
    scheduler API) [6]; it is the best-documented design for mixing
    single-threaded, parallel and blocking work in one program.
- **Adapt**:
  - Local (per-function) type inference is fine; *global* inference must
    stop at module or public-API boundaries, with annotations required
    there, so that separate and incremental compilation exist from day
    one. Crystal shows that the cost is not just build time but the
    absence of library interfaces.
  - Compile-time macros should operate on the language's own AST (as Nim
    and Julia do), not a separate template syntax; Cardiff's "can't be
    blended with the language's own grammar" is the cost of `{{ }}` [1].
- **Avoid**:
  - Shipping single-threaded and bolting parallelism on later; the
    standard library, the GC choice and the ecosystem all calcify around
    the first runtime model [6][7].
  - A memory-unsafe concurrency model (torn unions, unsynchronised
    arrays) in a language whose selling point is type safety.
- **Open questions raised**:
  - Where exactly should inference stop? Crystal's experience argues for
    annotated public signatures, but it is unclear whether module-internal
    global inference buys enough ergonomics to justify its compile-time
    cost.

## Sources
1. "We created the Crystal language. Ask us anything!", DEV Community AMA with Manas.Tech core team, 2017. https://dev.to/crystal-lang/we-created-the-crystal-language-ask-us-anything
2. A. Borenszweig, "Incremental compilation for Crystal, part 1", 3 January 2023. https://dev.to/asterite/incremental-compilation-for-crystal-part-1-414k
3. Crystal forum, "Incremental compilation exploration" (build-time reports). https://forum.crystal-lang.org/t/incremental-compilation-exploration/5224
4. "Crystal 1.21.0 released", 16 July 2026. https://crystal-lang.org/2026/07/16/1.21.0-released/
5. "Releasing Execution Contexts", 12 July 2026. https://crystal-lang.org/2026/07/12/releasing-execution-contexts/
6. RFC 0002, "Execution Contexts", crystal-lang/rfcs, started 5 February 2024, updated 27 October 2025. https://github.com/crystal-lang/rfcs/blob/master/text/0002-execution-contexts.md
7. "Parallelism in Crystal" (preview_mt design and known unsafety), 6 September 2019. https://crystal-lang.org/2019/09/06/parallelism-in-crystal/
8. "Crystal 1.0 - what to expect", 22 March 2021. https://crystal-lang.org/2021/03/22/crystal-1.0-what-to-expect/
