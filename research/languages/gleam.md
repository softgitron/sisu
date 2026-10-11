# Gleam

## Snapshot
| | |
|---|---|
| First appeared | 2016 (Louis Pilfold's prototypes); first public release April 2019; 1.0 on 4 March 2024. |
| Designed by | Louis Pilfold, with a small core team (Giacomo Cavalieri and others); community-funded (GitHub Sponsors, corporate sponsors). |
| Current version (Oct 2026) | 1.19.0 (5 October 2026). Releases roughly every 6-8 weeks: 1.14 (Dec 2025), 1.15 (Mar 2026), 1.16 (Apr 2026), 1.17 (Jun 2026), 1.18 (Jul 2026), 1.19 (Oct 2026). |
| Governance | BDFL (Pilfold); no foundation; semantic versioning with a stated backwards-compatibility commitment since 1.0. |
| Typing | Static, sound, Hindley-Milner style inference with full exhaustiveness checking; no subtyping, no typeclasses, no null. |
| Memory | Inherited from target: BEAM per-process GC, or JavaScript engine GC. |
| Compilation | Compiler in Rust. Targets: Erlang (since 1.19 emits Erlang abstract forms, not Erlang source) and JavaScript (Node, Deno, Bun, browsers). |
| Primary domains | Web backends (Wisp, Mist), front ends (Lustre), CLIs; early-stage production use. |
| Notable users | Small companies; 8% of 2024 survey respondents report production use. No large public reference deployment verified. |

## Stated design goals

The 1.0 announcement says Gleam exists to make software work that is
"predictable, stress-free, and enjoyable" for the people who write and
maintain it; it lists "a small, consistent surface area", "a lack of magic",
"one way of doing things", and "reading code is prioritised over writing it"
as the means. "Simplicity is a feature in Gleam and that will not change
going forward"; new features must be "generally useful and enable new things
not otherwise possible in Gleam" and will be added "very conservatively"
[1]. The type system is credited to Elm, OCaml and Rust; the compiler is
positioned as "a programming assistant" so refactoring is low risk [1]. The
FAQ codifies the omissions: "Gleam will always be a small and cohesive
language with a minimal feature set" [2].

## Coverage of the 15 evaluation dimensions

1. **Syntax and readability.** Rust/Elm-like with `fn`, `case`, `let`, `|>`, labelled arguments and `use`; no `if`, no loops, no return keyword; one formatter with no options. Reads uniformly; the main "after a year" cost is boilerplate from the absence of ad-hoc polymorphism.
2. **Type system.** Static, sound, Hindley-Milner inference; custom types (sums with records), generics, opaque types, exhaustive matching; no subtyping, typeclasses, null or implicit conversions. `Dynamic` for untyped host data with decoder libraries.
3. **Memory management and safety.** Target's GC (BEAM or JavaScript); no unsafe code in Gleam itself; FFI (`@external`) is the trust boundary.
4. **Mutability, aliasing and value semantics.** All values immutable; locals are rebindable names; mutation only through host FFI, actors or ETS wrappers.
5. **Error handling.** `Result(a, e)` and `Option` everywhere, composed with `result.try`/`use`; `panic`, `todo`, `assert`, `let assert` for bugs; no exceptions; host exceptions can be caught only via FFI.
6. **Concurrency and parallelism.** On the BEAM via `gleam_otp` typed actors and tasks, inheriting preemption and isolation; on JavaScript, promises via FFI. No language-level concurrency syntax.
7. **Polymorphism.** Parametric only; no typeclasses, overloading or interfaces; "pass the function" (explicit dictionary passing) is the idiom.
8. **Metaprogramming.** None; explicitly not planned (no macros); code generation is delegated to language-server code actions and external generators.
9. **Modules, packages, build.** One `gleam.toml`, Hex packages, lockfile, `gleam add/deps/publish/export`; git dependencies since 1.9; monorepo support; `dev/` directory since 1.11.
10. **Tooling.** Compiler, formatter, LSP, test runner (`gleeunit`), docs generator and package publishing all in one `gleam` binary; language server is the most actively developed component.
11. **Interop and platforms.** `@external` FFI to Erlang and JavaScript with typed signatures; generates `.app` files for Mix/rebar3 (1.19), escripts (1.17), TypeScript declarations; runs wherever BEAM or JS runs, including AtomVM.
12. **Performance and compilation model.** Compiles to Erlang abstract forms (1.19) or JavaScript; inherits host performance; fast compiler written in Rust with incremental builds.
13. **Standard library and ecosystem.** Deliberately small `gleam_stdlib`; ecosystem young (Lustre, Wisp, Mist, `gleam_json`, `gleam_time`); heavy reliance on wrapping Erlang/JS libraries.
14. **Learnability.** Very fast for newcomers (the language tour is the main onboarding); traps are recursion instead of loops, decoding `Dynamic`, and the lack of generic `to_string`/`compare`.
15. **Evolution and governance.** BDFL; semver with a backwards-compatibility commitment since 1.0 (March 2024); a release every six to eight weeks that adds diagnostics and tooling far more than language features.

## Strengths

### A feature set small enough to learn in an afternoon, verified by omission
Gleam has no exceptions (`panic` and `let assert` exist for bugs, `Result`
for expected failure), no `if`/`else` or loops (`case` and recursion only),
no typeclasses, no operator overloading, no macros, no implicit conversions,
no inheritance, no `null`, no mutation of locals. Rather than restraint by
promise, there is evidence of pruning: v0.27 (March 2023) *removed* the
`try` syntax ("Hello panic, goodbye try") in favour of the general `use`
expression added in v0.25 [3]; v0.32 "Polishing syntax for stability" and
v0.33 "Exhaustive Gleam" tightened the language before 1.0. The FAQ
explicitly lists typeclasses, function/operator overloading, exception-based
error handling and Lisp-style macros as "not planned" [2]; the reason given
for typeclasses is that they "can make it easy to make challenging to
understand code", confusing errors, and run-time cost on the BEAM.

### `use` expressions: one general construct replacing many special ones
`use x <- f(args)` rewrites the rest of the block into a callback passed as
the last argument to `f`. With `result.try` it is `?`/do-notation; with
`list.each` it is a loop body; with a connection pool it is `with`/RAII; with
a test framework it is `setup`. It covers early-return, async callbacks,
resource scoping and monadic binding with no new syntax per use case. The
cost: the rest of the block becomes a closure, which affects performance and
makes the control flow slightly less obvious than a dedicated `?`.

### Labelled arguments and records designed for readability
Function parameters may have external labels (`fn replace(in string, each
pattern, with replacement)`), call sites read as prose, and the language
server has "fill labels" code actions. Record update syntax, variant-safe
field access (a field is accessible only if all variants have it, with the
same type) and the ban on positional ambiguity are small decisions that add
up to the "reads well after a year" quality. v1.4 "Supercharged labels"
extended label shorthand (`f(name:)`).

### Error messages and language server as design priorities, not afterthoughts
Compiler errors show source spans, name the fix, and detect mistakes from
other languages (v1.19 added dedicated errors for `+=`, Java-style `|` in
patterns, and git merge markers) [4]. v1.2 and v1.6 made the compiler keep
analysing invalid code so the language server works mid-edit [3]; almost
every release since 1.5 added code actions (fill labels, generate decoder,
global rename in 1.10, label rename in 1.19). Friendliness is treated as a
feature with release-note space, not a bug class.

### Soundness and exhaustiveness without annotation burden
Inference covers whole modules; `case` must be exhaustive (since 0.33),
including bit-array patterns (v1.11 detects unreachable bit-array clauses
[5]); unused results of pure expressions warn (v1.11). Because there is no
subtyping or typeclass resolution, inference is complete and error messages
are local. `Result(a, e)` everywhere with `result.try`/`use` and the
standard-library convention that every fallible function returns `Result`
gives a consistent error story without the ceremony of checked exceptions.

### Two targets from one language, done by lowering not emulation
Gleam compiled to Erlang source and to JavaScript since v0.16 (2021). v1.19
replaced the Erlang backend with one that emits Erlang abstract forms
directly, so builds are faster and BEAM stack traces point at Gleam lines;
the post explains why not BEAM bytecode: "Bytecode changes between VM
releases ... abstract forms are the cost-benefit sweet spot for a
community-funded project" [4]. The JavaScript backend got decision-tree
pattern compilation (v1.11, ~30% faster on a Lustre benchmark [5]) and source
maps (v1.16). This is a useful data point: a small team can keep two targets
if the language is small and the IR boundary is chosen pragmatically.

### Newcomer adoption disproportionate to size
Stack Overflow 2025 ranks Gleam second most admired language at 70%,
between Rust (72%) and Elixir (66%) [6]. The 2024 Gleam survey (841
responses) shows the audience is *not* the BEAM community: previous
languages are TypeScript (385), JavaScript (338), Python (314), Rust (243),
Go (230), with only 190 respondents from Erlang/Elixir; 583 of 841 had under
two years of Gleam [7]. Gleam is evidence that a tiny, consistent language
with excellent errors and tooling is attractive to people coming from
TypeScript and Python, i.e. that *minimalism sells to newcomers* - with the
caveat that admiration surveys measure enthusiasm, not retention or
production use (52 of 841, about 8%, used it in production in 2024 [7]).

## Weaknesses

### The things it refused are the things users ask for
The 2024 survey's requested features include macros/metaprogramming, easier
JSON decoding (the pain point that led to a redesigned decoder API), a larger
standard library and a Rails-style framework; the core team answered "no
plans" for the last two and "no current plans" for single-binary builds [7].
Without typeclasses, there is no generic `to_string`, `compare`, `equal` or
`encode`: every type ships hand-written `encode_user`, `decode_user`,
`compare_user` functions, or uses the language server to *generate* them
(code actions for JSON encoders and dynamic decoders, v1.9 [8]). This is
honest - the cost of no ad-hoc polymorphism is paid in generated code rather
than hidden - but it is a real tax, and the growth of code actions is an
admission that the language needs tooling to compensate for an omission.

### OTP integration is partial and must be typed around
`gleam_otp` provides typed actors, but the BEAM's `gen_server` semantics
(selective receive, `handle_info` for arbitrary messages, `sys` system
messages, hot code upgrades, named processes with untyped registries) do not
fit a sound type system. Gleam cannot call `gen_server`-based Erlang
libraries without an FFI shim, and features like hot upgrades are simply
unsupported. The 2024 survey lists "expanded `gleam_otp`" among the top
requests [7]. This is the concrete cost of adding static types on top of a
dynamic runtime whose protocols were not designed for them.

### The ecosystem is young and thin
The package ecosystem is small (no reliable count was verified here); the
standard library intentionally stays small (`gleam_stdlib`); time (`gleam_time`) only
got a core library in 2025; there is no official HTTP client parity with
Erlang's `httpc` or Elixir's `Req` without wrapping. `gleam dev` (v1.11, a
`dev/` directory for development-only code [5]) and `gleam export escript`
(v1.17 [9]) show the build tool still filling basic gaps in 2025-2026.

### Panics are the hidden exception system
`panic`, `todo`, `let assert` and `assert` (v1.11 [5]) all crash the
process: on the BEAM a supervisor catches it, on JavaScript it is an uncaught
`Error`. Gleam has no typed, catchable exceptions, but it has an untyped
crash path whose semantics differ per target.

### Growth pressure on the "no new features" promise
Since 1.0 the language gained: label shorthand `f(name:)` (1.4), `echo`
(1.9), `assert` with `as` messages (1.11), constant `todo` (1.17), unaligned
bit arrays (1.9), formalised external APIs (1.13), plus a steady stream of
warnings. None are large, but the cadence of
a keyword a year is faster than "very conservatively" suggests, and each
one came from a real gap (debug printing, test assertions with good
failure output). The honest reading is that a minimal language still needs
a minimal *debugging and testing* vocabulary, which 1.0 lacked.

## Designer retrospectives

- Pilfold, 1.0 announcement: backwards compatibility is "now a priority",
  with the carve-out that security and soundness bugs may be fixed even if
  programs relied on them [1]. No regrets stated.
- The v0.27 removal of `try` [3] is the clearest retrospective: a
  special-purpose construct was replaced by a general one *before* 1.0, at
  the cost of breaking every program, precisely so that it would not have to
  happen after 1.0.
- The v1.19 backend rewrite post concedes the original Erlang code generator
  "did not meet current standards" and that the choice of Erlang *source* as
  output had cost: wrong line numbers in crash reports and slow builds [4].
- On typeclasses, the FAQ's reasoning is explicitly about error messages and
  readability, not implementation difficulty [2].

## Influence

Gleam takes its type system from Elm/OCaml/Rust, `Result`-everywhere and
no exceptions from Elm and Rust, labelled arguments from OCaml/Swift, bit
arrays and the runtime from Erlang, tooling and Hex from Elixir, and the
error-message culture from Elm and Rust. Its influence so far is as a proof
point that a language with almost no features can top "admired" surveys, and
`use` is discussed as a model for callback-flattening syntax elsewhere.

## Lessons for Sisu

- **Adopt**: *one general callback/`use` form instead of a family of
  special forms*, so early return, resource scoping, async and iteration
  share one mechanism; Sisu can keep a `?` operator for the overwhelmingly
  common `Result` case, but `use` shows the general form is cheap to add.
- **Adopt**: *labelled arguments with call-site labels*; they are the single
  cheapest readability feature in this group and are also in Swift and
  OCaml.
- **Adopt**: *error messages and code actions as release-note features.* The
  Gleam cadence (every release lists new diagnostics) is the practice to
  copy; Sisu's compiler should have an owner for "mistakes people make coming
  from Python/TypeScript/Rust".
- **Adopt**: *exhaustiveness and unused-result warnings on by default*; Gleam
  shows they are not controversial when present from day one.
- **Adapt**: *the minimal feature set as a starting point, not a creed.*
  Gleam's post-1.0 additions (`echo`, `assert`, `todo` in constants) are all
  debugging/testing vocabulary; Sisu should ship that vocabulary in 1.0.
- **Adapt**: *ad-hoc polymorphism.* Gleam's absence of typeclasses is paid
  for by generated boilerplate and code actions. Sisu should have *some*
  constrained polymorphism (traits/interfaces) but can take Gleam's warning
  seriously: keep resolution local and errors readable; avoid implicit
  instance search beyond one level.

- **Avoid**: *bolting static types onto an untyped actor protocol.* If Sisu
  has actors, design the message protocol typed from the start rather than
  inheriting an untyped mailbox and then being unable to use half the
  runtime's library.
- **Avoid**: *an untyped crash path that behaves differently per target.*
  Define what a panic means on every target before 1.0.
- **Open questions raised**: (1) Does newcomer admiration convert to
  production use and retention? The 8% production figure and the age of the
  ecosystem mean this is unanswered until a 2027-2028 survey. (2) Can a
  language stay this small once it has corporate users demanding
  metaprogramming, or is "no macros" only stable while the community is
  small? (3) Is `Result`-everywhere ergonomic without `?`, or does `use`
  carry it only because Gleam programs are still short?

## Sources

1. Louis Pilfold, "Gleam version 1", 4 March 2024. https://gleam.run/news/gleam-version-1/
2. Gleam, "Frequently asked questions" (not-planned list, typeclasses, exceptions, macros). https://gleam.run/frequently-asked-questions/
3. Gleam news archive (release list with dates and headlines, v0.3 to v1.19). https://gleam.run/news/
4. "Gleam doesn't compile to Erlang source anymore" (v1.19.0), 5 October 2026. https://gleam.run/news/gleam-doesnt-compile-to-erlang-source-anymore/
5. "Gleam JavaScript gets 30% faster" (v1.11.0, `assert`, `gleam dev`), 2 June 2025. https://gleam.run/news/gleam-javascript-gets-30-percent-faster/
6. Stack Overflow Developer Survey 2025, "Technology": admired languages (Rust 72%, Gleam 70%, Elixir 66%). https://survey.stackoverflow.co/2025/technology/
7. "Developer Survey 2024 results" (841 responses). https://gleam.run/news/developer-survey-2024-results/
8. "Hello echo! Hello git!" (v1.9.0), 8 March 2025. https://gleam.run/news/hello-echo-hello-git/
9. "Single file Gleam BEAM programs with escript" (v1.17.0), 2 June 2026. https://gleam.run/news/single-file-gleam-beam-programs-with-escript/
10. "Introducing use expressions!" (v0.25), 24 November 2022; "Hello panic, goodbye try" (v0.27), 1 March 2023. https://gleam.run/news/
