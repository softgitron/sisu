# Nim

## Snapshot
| | |
|---|---|
| First appeared | 2008 (as "Nimrod"; renamed Nim in 2014); 1.0 September 2019; 2.0 August 2023; 2.2 October 2024 |
| Designed by | Andreas Rumpf ("Araq"), who remains the sole language designer |
| Current version (Oct 2026) | 2.2.12 (8 September 2026), the sixth bug-fix release of the 2.2 line; a `version-2-4` branch exists; "Nimony", the new compiler intended to become Nim 3, published preview 0.2 in October 2025 |
| Governance | BDFL. A small core team; no formal foundation; funded by donations and commercial support, with a Status (Nimbus Ethereum client) partnership as the 2025 funding priority [8] |
| Typing | Static, strong, nominal with structural tuples; local type inference; generics, `distinct` types, `concept`s (experimental since 2015) |
| Memory | ORC by default since 2.0 (ARC = scope-based reference counting with move semantics, plus a trial-deletion cycle collector); legacy `refc` tracing GC still selectable; also `--mm:arc`, `none`, Boehm, Go |
| Compilation | AOT: Nim emits C, C++ or Objective-C and invokes a C compiler; a JavaScript backend; a VM for compile-time evaluation |
| Primary domains | CLI tools, systems and embedded, games, web (via JS backend), blockchain clients |
| Notable users | Status (Nimbus Ethereum client, Chronos async library); otherwise individual and small-company projects |

## Stated design goals
Nim's 2.0 announcement says "Nim is a programming language that is good for
everything, but not for everybody", focusing on imperative programming
"extended by a macro system" with "customizable memory management" suited to
hard real-time and systems work [1]. Rumpf's "Zen of Nim" (2021) gives the
principles: "Copying bad design is not good design"; "If the compiler cannot
reason about the code, neither can the programmer"; "Don't get in the
programmer's way"; "Move work to compile-time: programs are run more often
than they are compiled"; "Customizable memory management"; "Concise code is
not in conflict with readability, it enables readability"; "Leverage meta
programming to keep the language small"; and, half in jest, "There should be
one and only one programming language for everything. That language is
Nim." [2] The stated tension - "we want the language to be complete" vs
"we want the language to be minimal" - is the one Nim has not resolved.

## Strengths

### Python-like surface syntax over a C-speed native compiler
Indentation-based blocks, `proc`, `var`/`let`/`const`, no semicolons or
braces, with static types and C performance. Survey respondents name
"performance/speed" and "syntax" as the two things they like most, followed
by ease of use and self-contained binaries [8]. Compiling through C gives
Nim the full set of C targets (embedded, WebAssembly via Emscripten,
every OS with a C compiler) for very little implementation effort; the 2024
survey notes embedded now outranks JavaScript as a target [8].

### One of the most powerful macro systems in a statically typed language
Nim has `template` (hygienic substitution of AST, with `untyped` and `typed`
parameters), `macro` (arbitrary compile-time Nim code operating on the AST,
run in the compiler's VM), and `static` evaluation of ordinary procedures.
Macros receive *typed* ASTs when they ask for them, so they can inspect
resolved symbols and types, which is what lets libraries implement `async`,
pattern matching, serialisation, ORMs and `{.pragma.}`-driven DSLs without
compiler changes. This is the mechanism behind Rumpf's "keep the language
small" principle [2]: Nim's `async`/`await` is a library, not syntax.
`std/macros` exposes the compiler's own node kinds (`nnkIdent`, `nnkCall`,
...), which is both the strength (full power) and the weakness (tied to
compiler internals; 2.2's `--experimental:openSym` added a node kind that
"macro authors may need to handle" [3]).

### ARC/ORC: deterministic destruction with moves, and a case study in "ARC + moves without a borrow checker"
ARC (Nim 1.2, 2020) replaced the tracing GC with compiler-injected
`=destroy`/`=copy`/`=sink`/`=dup`/`=wasMoved` hooks and a "last read"
dataflow analysis that turns the final use of a variable into a move;
`sink` parameters transfer ownership and `lent` returns a compiler-checked
borrow [4]. ORC (Nim 1.4, default since 2.0) adds a trial-deletion cycle
collector that the compiler only wires in for types it cannot prove acyclic;
`{.acyclic.}` opts a type out, `{.cursor.}` marks a non-owning reference
[4][5]. Rumpf's benchmark (HTTP server holding 135 MB of live JSON): with a
forced full collection every 10 requests ORC averaged 320 us latency vs 65 ms
for mark-and-sweep; without forced collections ORC had 275 us vs 1.49 ms
average latency and 137 MiB vs 333 MiB peak memory, while mark-and-sweep had
higher throughput (39.6k vs 34.9k req/s) [5]. The design properties that
matter for Sisu: destructors run at scope exit (files and sockets close
without `close`), no stop-the-world phase, performance independent of heap
size, works on embedded and WebAssembly, and plays well with sanitizers.
The 2.0 notes report "no measurable benchmark gain" from the final ARC/ORC
refinements with GCC or LLVM but better separate compilation [1].

### A real effect system for exceptions and side effects
`{.raises: [IOError].}` on a routine is checked by the compiler against the
inferred exception set of its body; `raises: []` means "cannot throw";
proc types carry raises lists, so passing a throwing callback where a
non-throwing one is expected is a type error. `effectsOf` lets a
higher-order proc inherit its callback's effects instead of assuming the
worst. `tags` and `forbids` track user-defined effects (an `IO` tag, say),
`noSideEffect`/`func` is checked purity (parameters-only side effects),
and `gcsafe` is a checked effect for thread safety [6]. Since 2.0 "strict
effects" are always on [1]. This is the most complete statically-checked
exception tracking in a mainstream-syntax language, and it coexists with
ordinary `try`/`except`.

### Uniform function call syntax
`f(a, b)`, `a.f(b)` and `a.f b` are the same call. Any procedure can be
called in method style on its first argument, so "methods" are just
procedures and anyone can extend any type with new dot-callable operations
without touching its definition. It makes pipelines read left to right and
removes the OOP question of where a function "belongs". The cost is in the
weaknesses (ambiguity, discoverability, and templates/macros that cannot be
called with dot syntax on unresolved identifiers [6]).

### C and C++ interop with almost no ceremony
`{.importc, header: "foo.h".}` binds a C function directly; the compiler
emits C so struct layouts and calling conventions are the C compiler's own.
2.0 and 2.2 added C++ pragmas (`virtual`, `constructor`, `member`,
`noInit`, custom constructor initialisers) so that Nim can define C++
classes and methods callable from C++ [1][3]. Because the output is C, Nim
also *embeds* easily into C projects.

## Weaknesses

### Style insensitivity
Identifiers are compared with the first character case-sensitive and the
rest case- and underscore-insensitive: `fooBar`, `foo_bar` and `foobar` are
the same name, as are the keywords `notin`, `notIn` and `not_in` [6]. The
manual's rationale is that programmers can "use their own preferred spelling
style" and that an editor can display identifiers in each reader's style;
Rumpf's 2013 remark was that it "ensures a painless interoperability" with
libraries written in different styles [7]. Historically Nim was fully
insensitive (`foo` == `Foo`); the first-letter exception was added so that
`var foo: Foo` parses [6]. The costs are concrete: `grep` and every editor
search miss references; code review sees three spellings of one identifier
in one file; every tool that touches Nim source (formatters, LSP, syntax
highlighters, documentation generators) must reimplement the normalisation
algorithm; and newcomers reliably list it as the strangest thing about the
language. No later language has copied it, and the proposed replacement
(a project-wide `--styleCheck:error` enforcing one style) is an admission
that the feature creates a problem it then needs a switch to suppress.

### Too many ways to do everything, and features that stay "experimental" for years
Nim has `proc`, `func`, `method` (dynamic dispatch), `iterator` (inline and
closure variants), `template`, `macro` and `converter` (implicit
conversions); `object`, `ref object`, `ptr`, `var` and `lent`; exceptions
*and* `Option[T]` *and* effect tracking; implicit `result` variables; and
dozens of pragmas (`{.inline.}`, `{.noSideEffect.}`, `{.gcsafe.}`,
`{.acyclic.}`, `{.cursor.}`, `{.byref.}`, `{.discardable.}`, ...).
`concept`s (structural generic constraints) have been experimental since
2015 and were reworked without becoming stable; `strictDefs`, `strictFuncs`,
`vtables`, `openSym` and `views` are all experimental switches in 2.x [1][3].
Nim 2.2's own release notes say generics, `typedesc` and `static` bugs were
fixed "effectively making these features stable" [3] - five years after 1.0.
The "leverage metaprogramming to keep the language small" principle [2] did
not prevent the core from growing; it added a second, equally large, surface.

### Compiler bugs and memory-safety holes in the ownership transformation
The ARC/ORC design gives deterministic destruction without a borrow checker
by relying on compiler analyses (last-read, sink, cursor inference) that are
not sound by construction. The evidence is the bug stream: 2.2.12 (Sep
2026) is a bug-fix release of 142 commits whose headline items are
"memory and crash issues under ARC/ORC and refc, including use-after-free
and double-destroy problems" and "copy regressions related to return-value
optimisation" [9]; 2.2.0 fixed "memory corruption with `lent` and ORC"
(#23973) [3]. `{.cursor.}` is documented as "not a `weak_ptr`: there are no
runtime checks, so misuse is unsafe" [4]; `ptr` and `cast` are available
everywhere without an `unsafe` marker; `lent` is only "proven" for simple
cases. Two-thirds of 2024 survey respondents rate "fixing compiler bugs" as
very important, ahead of any feature [8]. The lesson is that ARC + moves is
an excellent *performance* model, but without a borrow checker (or
pervasive runtime checks) it is not a *safety* model: the compiler's own
optimisations become the source of use-after-free.

### Configuration-dependent semantics
Whether `IndexDefect` is catchable depends on `--panics`; whether cycles
leak depends on `--mm`; whether bounds are checked depends on `-d:release`
vs `-d:danger`; whether a `raises` analysis is exact depends on
`--exceptions`; `Defect`s are deliberately excluded from `raises: []`
because "with `--panics:on`, Defects become unrecoverable" [6]. Library
code cannot know which world it runs in, which is why libraries written
for `refc` broke under ORC (the 2.0 migration) and why the same source can
be memory-safe or not depending on flags.

### Tooling and the two-package-manager problem
"Tooling" is the top improvement priority in the 2024 survey, and its share
rises every year [8]. `nimsuggest` (the LSP backend) crashes are a recurring
release-note item [3][9]. Nim 2.0 shipped *both* Nimble (with lock files
from 0.14) and Atlas, "an alternative package manager", in the same
distribution [1]; Nimble 0.16 then added an opt-in SAT solver [3]. Shipping
two package managers with different models is a governance decision that
the ecosystem has to absorb.

### Data races are not prevented
Threads are on by default since 2.0 (per the 2.0 changelog; the
announcement post does not mention it) and ORC's reference counting is
non-atomic by design (which is why it is fast) [4][5]. Safety therefore
rests on the `gcsafe` effect (no access to global GC'd memory) and on
convention: `Isolated[T]` for transferring ownership between threads is
experimental, channels copy, and sharing a `ref` between threads is
undefined behaviour the compiler does not catch. Nim is honest that this is
the programmer's job, but a language whose selling point is "the compiler
reasons about the code" [2] has no story here.

### Small and not growing community under a single designer
The 2024 community survey had 367 respondents, "fewer than in previous
years"; 57% had used Nim for two or more years and only 15% for under a
year, which the report itself reads as "we're not attracting enough new
users"; among non-users the top reasons are "Nim seems immature, not ready
for production", missing libraries, and "too risky for production" [8].
Fifteen-plus years in, the causes visible in the evidence are: (a) BDFL
decisions that surprised users (style insensitivity, the ARC migration, the
2.0 move of `db_*`, `md5`, `smtp` and others out of the standard library
[1]); (b) a compiler whose bugs outrun its features; (c) governance
disputes that produced forks - `nimskull` (2021-22) explicitly "aims to
become a different programming language" and lists "not locking in poor
early choices" and avoiding "heavy unpaid labor on a few people" among its
reasons [10]; and (d) Nim 3. Nimony is "a new implementation of the Nim
language" with its own documented feature set, "type-checked generics", no
forward declarations, incremental and parallel compilation, and a
continuation-based concurrency model ("passive procs") [11]. Each of those
goals is an implicit retrospective on Nim 2 (untyped generics, forward
declarations, non-incremental compilation, `async` vs threads), and the
plan to reach Nim 3 through a new compiler with a different feature set is
exactly the Python 2/3 transition pattern for a community that cannot
afford one.

## Designer retrospectives
- Rumpf frames 2.0 as "an evolution (not revolution) of Nim" [1] but the
  Nimony README describes Nim 3 as a new compiler for projects "that can
  work within its documented feature set rather than Nim 2's" [11]; the
  feature list (typed generics, no forward declarations, incremental
  compilation, one concurrency model, plugin-based metaprogramming) is the
  clearest statement of what he now considers wrong with Nim 2.
- The ORC post admits the cycle collector's overhead "can be measurable in
  practice", that mark-and-sweep wins on raw throughput, and that the
  benchmark "is not a realistic workload" [5].
- The Zen of Nim concedes the minimal-vs-complete friction is unresolved
  and that "leverage metaprogramming to keep the language small" is "hard
  to keep" [2].
- The 2.0 removal of standard-library modules into Nimble packages and the
  2.0 `strictFuncs` tightening are documented as breaking changes [1],
  which is an admission that the 1.0 stability promise was partially
  traded for design corrections.

## Influence
- **Copied from others**: syntax from Python; `distinct` types and the
  module system from Modula/Oberon and Ada; UFCS from D; `sink`/`lent` and
  moves from C++11 and Rust; the effect system from Koka and Java checked
  exceptions (with inference); macros from Lisp via Scheme-style hygiene;
  `scope(exit)`-style `defer` from D/Go.
- **Copied by others**: Nim's ORC is cited in D-community discussions as
  the model for "deterministic RC plus a cycle collector" [5]; the
  `raises`-list style of exception tracking is the closest existing model
  for checked-but-inferred exceptions.

## Lessons for Sisu
- **Adopt**:
  - Statically checked, *inferred* exception/effect sets on routines and on
    function types, with `effectsOf` for higher-order functions [6]. This
    is the single most transferable Nim feature: errors stay visible in
    signatures without Java-style annotation burden.
  - Deterministic destruction with compiler-inserted moves at last use,
    `sink` parameters and borrowed returns as the *baseline* resource model
    [4]; the performance case is made [5].
  - A compile-time VM that runs the real language, so compile-time code is
    not a second language.
- **Adapt**:
  - UFCS: keep the "any function is callable in method position" property,
    but make it resolvable without a typed receiver (Nim's dot syntax fails
    on templates/macros and on qualified names [6]) and define overload
    resolution so that `a.f(b)` and `f(a, b)` cannot resolve differently.
  - Cycle handling: Nim shows ARC + a cycle collector works, but the
    collector's cost and the `{.acyclic.}`/`{.cursor.}` escape hatches
    push correctness onto the user. Sisu should either prevent ownership
    cycles statically (weak/unowned references, as Swift) or make the
    collector's presence a visible type property, not a compiler flag.
  - Typed AST macros: expose a *stable* macro API (not the compiler's node
    kinds), as Nimony's "plugin-based metaprogramming" is now attempting
    [11].
- **Avoid**:
  - Style-insensitive identifiers. The tooling and review costs are
    certain; the benefit (mixed-style libraries) is better served by a
    formatter.
  - Semantics that depend on build flags (`--panics`, `--mm`, `-d:danger`).
    Library code must have one meaning.
  - "Experimental" features that live for a decade; either stabilise or
    remove within a fixed number of releases.
  - Relying on compiler optimisations (move/cursor inference) for memory
    safety without a sound check; the 2.2.x use-after-free fixes [9] are
    the direct cost.
  - A single designer without a written decision process; both forks
    (nimskull, and in effect Nimony) cite process, not technology.
- **Open questions raised**:
  - Can "Swift-like ARC plus Rust-like moves without a borrow checker" be
    made *safe* rather than merely fast? Nim's answer so far is no; the
    question for Sisu is whether lightweight borrow checking of `lent`/
    `cursor`-style references is enough, or whether runtime checks are
    needed at the gaps.
  - Exceptions plus static tracking vs result values: Nim shows tracked
    exceptions can be ergonomic, but it still ended up with `Option`,
    `Result`-style libraries and `Defect` vs `Exception` split. Is a
    single mechanism possible?

## Sources
1. "Nim v2.0 released", 1 August 2023. https://nim-lang.org/blog/2023/08/01/nim-v20-released.html
2. A. Rumpf, "Zen of Nim", 15 November 2021. https://nim-lang.org/blog/2021/11/15/zen-of-nim.html
3. "Nim versions 2.2.0 and 2.0.10 released", 2 October 2024. https://nim-lang.org/blog/2024/10/02/nim-220-2010.html
4. Nim documentation, "Nim Destructors and Move Semantics". https://nim-lang.org/docs/destructors.html
5. A. Rumpf, "ORC - Vorsprung durch Algorithmen", 8 December 2020. https://nim-lang.org/blog/2020/12/08/introducing-orc.html
6. Nim Manual: Identifier equality; Effect system; Method call syntax. https://nim-lang.org/docs/manual.html
7. Nim IRC logs, Araq on style insensitivity and interoperability, 15 September 2013. https://irclogs.nim-lang.org/15-09-2013.html
8. "Nim Community Survey 2024 Results" (n=367), 23 January 2025. https://nim-lang.org/blog/2025/01/23/community-survey-results-2024.html
9. "Nim version 2.2.12 released", 8 September 2026. https://nim-lang.org/blog/2026/09/08/nim-2212.html
10. nim-works/nimskull README and FAQ (fork rationale). https://github.com/nim-works/nimskull
11. nim-lang/nimony README (Nim 3 compiler goals), 2025-2026. https://github.com/nim-lang/nimony
12. Nim blog index (release dates 2024-2026). https://nim-lang.org/blog.html
