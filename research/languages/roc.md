# Roc

## Snapshot
| | |
|---|---|
| First appeared | Started by Richard Feldman around 2018-2019 as "a direct descendant of Elm" [1]; public from 2020. |
| Designed by | Richard Feldman and a volunteer core team (Folkert de Vries, Brendan Hansknecht, Anton, Luke Boswell and others). |
| Current version (Oct 2026) | No 0.1 release: "Roc is not ready for a 0.1 release yet" [2]. Rolling "alpha" builds of the old Rust compiler (`alpha4-rolling`, 26 Aug; `alpha2`, 29 Jan 2025, replaced `Task` with effectful `!` functions) [3]; nightlies of the new Zig compiler live in a separate `roc-lang/nightlies` repo "to avoid bloating roc-lang/roc" [3][4]. 6.1k stars, 51,219 commits [2]. |
| Governance | Open source (Universal Permissive License, chosen for MIT-like brevity plus Apache-2-like patent protection and GPLv2 compatibility) [1]; Feldman is BDFL in practice. |
| Typing | Static, full HM inference ("it infers the types of everything you write"; "All type annotations in Roc are optional"), anonymous tag unions, rank-1 only, no higher-kinded polymorphism ever [1][5]. |
| Memory | Automatic reference counting with opportunistic in-place mutation; "automatic memory management ... has some unavoidable runtime overhead"; platforms "get to determine how memory gets allocated and deallocated" [6]. |
| Compilation | AOT to machine code or WebAssembly only ("The plan is for Roc to compile only to very low-level targets"); dev backend/interpreter for fast feedback; compiler "will never" be self-hosted [1][7]. |
| Primary domains | CLI tools, web services, scripting embedded in a host (via platforms). |
| Notable users | None significant; the community's own platforms (basic-cli, basic-webserver). |

## Stated design goals
"A fast, friendly, functional language" [8]: fast means it "is designed to
build and run quickly"; friendly means syntax, semantics and tooling; functional
means "a small set of simple primitives" and "a single-paradigm functional
language" [8]. The compile-time goal is concrete: builds should "almost always
complete in under 1 second on the median computer being used to write Roc" [6].
Feldman's FAQ gives the governing attitude on type-system scope: "Culturally,
to support HKP is to take a side, and to decline to support it is also to take
a side" [1].

## The central bets

### Platforms and applications
Every program is an *application* built on exactly one *platform*:
`app [main!] { pf: platform "https://github.com/roc-lang/basic-cli/releases/..." }`.
"The selected platform provides the app's I/O primitives"; "Roc's standard
library does not include any effectful functions; they all come from the
platform"; "Every Roc application specifies exactly one platform" [5]. The
platform is written in Rust, Zig, C or anything with a C ABI, owns `main`,
decides how and when Roc code runs ("The host controls when Roc code runs,
enabling domain-specific APIs"), and owns allocation: "skipping deallocations
for command-line scripts and using arena allocation for web servers" [1][6].
Multiple platforms per app: "The short answer to each of these questions is
'No.'"; "Exclusivity is the point!" [1]. The result is purity without a
monad: Roc code is pure or effectful by *inference*; effectful functions end
in `!` and use `=>`:

```roc
read_str! : Path => Try(Str, ReadFileErr)   # thick arrow: effectful
main! = |_args| {
    echo!("Hello, World!")
    Ok({})
}
```

"Pure functions cannot call effectful functions, while effectful functions
can call both"; "The compiler infers effectfulness and checks annotations
against it" [7]. This replaced the earlier monadic `Task` type in January
2025: "Unlike the former `Task`-based design, current Roc code calls
effectful functions directly" [3][7]. Note that `->`/`=>` is still a
two-colour system; what Roc removed is the monadic plumbing and the
platform's freedom to implement effects synchronously or asynchronously ("A
platform can use synchronous blocking I/O, asynchronous I/O, or another
strategy appropriate to its domain") [7].

### Anonymous tag unions
Tags need no declaration: `birds_or_numbers = [Bird("eagle"), Number(1)]`
infers a union type; "tags don't have to wrap anything"; errors are
`Try(ok, err)` with `Ok`/`Err` tags and "It's very common to see a `Try` with
a tag union for its error type, even if it only has one tag in it" [5]. There
is no `Maybe`/`Option`: "By design, Roc does not have one of these"; prefer
`[Loading, Loaded(Artist)]` because explicit tags "say why a value is missing
and can be extended without breaking code" [1]. In the previous compiler,
tag unions were *open* (`[A, B]*`, extensible by callers) or *closed*; the new
compiler's tutorial does not yet restate the open/closed rules, so treat the
current semantics as in flux [5].

### Opportunistic in-place mutation and abilities
Values are immutable; the runtime reference-counts and mutates in place when
the count is one (the same idea as Koka's Perceus; see `koka-effekt.md`), with
a borrow-inference pass (originally the Morphic analysis, now "unmaintained"
and slated for rewrite) deciding which arguments are owned or borrowed
[6][9]. Local mutation is limited to a function's own `var` bindings, which
"callers can't observe" [7]. Ad-hoc polymorphism is through *abilities*
(now surfaced as `where` constraints on type variables: "You can also
constrain type variables to types that have specific methods using `where`")
rather than typeclasses, because HKP is excluded [5].

### The 2025 decision to rewrite the compiler in Zig
Feldman's announcement (gist, around 4 Feb 2025) [9]: nearly every stage was
already slated for a rewrite for independent reasons (parser not error-
tolerant and moving from combinators to recursive descent; formatter without
line-width enforcement; canonicalisation needing deferred name resolution and
a flat ID-based structure for disk caching; docs generation; monomorphisation
to fix lambda sets and replace unmaintained Morphic; LLVM codegen to emit
bitcode directly; dev backend to become an interpreter). Given a scratch
rewrite, Zig won on: compile times as "the central reason" (a Rust parser
change "takes 20+ seconds to rebuild", while Zig's x86-64 backend rebuilds its
own ~300K-line compiler "in a second or two"); memory safety "less valuable
here, since the compiler passes allocators around and data structures are
isolated by stage"; an ecosystem with "more of the dependencies the project
actually wants"; easier static musl builds; `MultiArrayList`, arbitrary-width
integers and tagless unions [9][10]. Process goals: "Fuzz early", document as
you go, "Avoid `unwrap()`-style panics", "Prioritize debuggability over
short-term performance" [9]. Secondary coverage claims the rewrite reached
feature parity in early 2026 after ~487 days; I could not reach a primary
source for that figure, so treat it as unverified [11].

## Strengths

### Purity without monads, with effects decided by the host
The platform split gives application code referential transparency,
testability and optimisation ("dead-code elimination, loop fusion, and
hoisting work out of loops") while the host decides the I/O model [7]. It is
also a security story: an application can only do what its platform exposes.

### Inference everywhere, including errors
Full inference plus anonymous tag unions means error types accumulate
automatically and need no declaration; `Try` with a one-tag union is normal
[5]. This is a demonstrated improvement on Elm's `Result`/`Maybe` ergonomics.

### Principled refusals, publicly argued
No currying ("Currying lowers error message quality"), no HKP ("The explicit
plan is that Roc will never support higher-kinded polymorphism"), rank-1 only,
"never have linear types, dependent types, refinement types, or uniqueness
types" ("Dependent types are too risky of a bet"), no function equality
("undecidable"), no wildcard imports (mainly "compiler performance", since
name resolution can then run per module in parallel) [1]. Each refusal has a
written rationale.

### Host-controlled memory is a real performance lever
Arena-per-request web servers and no-free CLI scripts are decisions the
platform author makes once, not every application [6].

## Weaknesses

### Seven years without a 0.1 and a full rewrite
Roc has had rolling alphas, a syntax overhaul (alpha2), the `Task` removal,
and now a second compiler; "Roc is still a work in progress and has not
reached a 0.1 release, so expect missing features and bugs" [5]. The Rust
codebase accumulated enough unfixable structure (lambda sets, Morphic, panics)
that a scratch rewrite was cheaper than repair [9].

### Exactly one platform is a hard constraint
"Exclusivity is the point!" means an app cannot combine a web-server platform
with a database platform; composition must happen inside a platform or via
platform-agnostic packages [1]. Platform authors must write Rust/Zig/C, so the
ecosystem bottleneck is non-Roc code.

### Reference counting without a cycle story and with host complexity
Roc's RC relies on immutability to avoid cycles (as Koka does) but hosts must
implement allocation and respect refcount conventions across the FFI, which
is where platform bugs concentrate (the alpha release notes are dominated by
such fixes) [3][6].

### Two function colours remain
`->` vs `=>` and the `!` suffix are a colouring discipline; it is lighter than
async/await but still splits the function space, and effectful code cannot be
called from pure code even in a debugging context.

## Designer retrospectives
- The rewrite post is the retrospective: the parser "wasn't error-tolerant
  enough", the formatter lacked width enforcement, `unwrap()` panics "caused
  problems before", Morphic was an unmaintained dependency, and Rust's
  memory-safety value was low for a compiler that already isolates data by
  stage [9].
- On `Task`: replacing the monadic design with inferred effectfulness (alpha2,
  Jan 2025) is an admission that monadic effects were the wrong ergonomics for
  Roc's audience [3][7].
- On HKP: "Roc will never support higher-kinded polymorphism" is argued from
  decidable principal type inference and from culture, not from inability [1].

## Influence
- **Took from**: Elm (syntax, friendliness, no typeclasses, `Result`
  lineage), Haskell/OCaml (inference), Koka/Lean (RC with in-place reuse;
  Feldman's "Outperforming Imperative with Pure Functional Languages" talk
  presents the same argument, cited from memory), Zig (compiler engineering
  and `MultiArrayList`).
- **Gave to**: The platform/application split is the clearest articulation of
  "host owns effects"; Gleam's and Unison's approaches to effects differ, but
  the idea that the *standard library has no effectful functions* is Roc's own
  and has not been copied by a shipped language yet.

## Lessons for Sisu
- **Adopt**: Anonymous, inferred sum types for error channels (`Try` with
  tag unions) so that error types compose without declarations; this is
  Roc's best-validated ergonomic win over Elm/Haskell.
- **Adopt**: Each language-level refusal (no currying, no HKP, no wildcard
  imports) should come with Roc-style written rationale; "no wildcard imports
  because name resolution parallelises" is the kind of reason Sisu's spec
  should record.
- **Adapt**: The platform split's *idea*—the host owns `main`, I/O and
  allocation policy; application code is pure by default—maps well onto Sisu's
  capability/effect design, but Sisu should allow multiple capability
  providers rather than exactly one platform, and should let platforms be
  written in Sisu itself (Roc's hosts cannot be written in Roc).
- **Adapt**: Inferred effectfulness with a visible marker (`!`/`=>`) is a good
  compromise; Sisu should make sure pure code can still call debug/trace
  effects (see Flix's `Debug` effect in `flix.md`).
- **Avoid**: Rolling alphas for years. Roc's lesson is the same as Pony's and
  Hylo's: define a small 0.1 and ship it.
- **Avoid**: A compiler architecture that cannot be fixed incrementally. The
  concrete anti-patterns Feldman lists (panicking on internal errors,
  non-error-tolerant parser, unmaintained analysis dependency, LLVM API
  coupling) should be Sisu's compiler-design checklist from day one.
- **Open questions raised**: (1) Is "exactly one platform" a feature
  (security, simplicity) or a limitation (composition)? Roc says feature; no
  second language has tested the alternative. (2) Can RC-with-reuse hold up
  when hosts and FFI code share ownership, which is where Roc's bugs cluster?
  (3) Does refusing HKP cost real abstraction in a language with inferred
  effects, or do abilities/`where` constraints cover the practical cases?

## Sources
1. Roc FAQ, https://www.roc-lang.org/faq (2026).
2. Roc repository README ("Work in progress!", stars, commits), https://github.com/roc-lang/roc (2026).
3. Roc releases page (alpha1-alpha4-rolling; alpha2 replaced `Task`), https://github.com/roc-lang/roc/releases.
4. Roc nightlies repository (new compiler builds), https://github.com/roc-lang/nightlies (2026).
5. Roc mini-tutorial for the new compiler, https://github.com/roc-lang/roc/blob/main/docs/mini-tutorial-new-compiler.md (2026).
6. Roc, "Fast", https://www.roc-lang.org/fast (2026).
7. Roc, "Functional", https://www.roc-lang.org/functional (2026).
8. Roc homepage, https://www.roc-lang.org/ (2026).
9. R. Feldman, "rewrite.md" (announcement of the Zig rewrite), https://gist.github.com/rtfeldman/77fb430ee57b42f5f2ca973a3992532f (Feb 2025).
10. Rust Bytes, "Why Roc Is Moving Away From Rust to Zig", https://weeklyrust.substack.com/p/why-roc-is-moving-away-from-rust (10 Feb 2025); Ziggit thread, https://ziggit.dev/t/roc-lang-to-be-rewritten-in-zig/8363 (4 Feb 2025).
11. Secondary coverage of feature parity (unverified), e.g. https://logicity.in/en/blog/roc-rewrites-300k-lines-of-rust-to-zig-hits-feature-parity (2026).
