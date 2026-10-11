# Elm and PureScript

Two Haskell-descended languages that compile to JavaScript and made opposite
bets: Elm removed features (type classes, laziness, native code, exceptions)
to optimise learnability and reliability for one domain; PureScript kept
Haskell's type-system power, added row polymorphism, and stayed general.

## Snapshot
| | Elm | PureScript |
|---|---|---|
| First appeared | 2012 (Evan Czaplicki's thesis project) | 2013 (Phil Freeman) |
| Designed by | Evan Czaplicki (sole designer; BDFL) | Phil Freeman, then a volunteer core team (Freeman stepped back around 2019; not re-verified here) |
| Current version (Oct 2026) | 0.19.3 (2 Oct 2026, "Another step towards Elm 1.0"); 0.19.2 (6 Jul 2026, "Faster Builds"); before that 0.19.1 (21 Oct 2019) and 0.19.0 (21 Aug 2018) [1][2][3] | Compiler 0.15.16 (18 Mar 2026); Registry and Spago 1.0 launched Feb 2026; Chez Scheme backend announced June 2026 [4] |
| Governance | BDFL with closed roadmap; a 6.7-year gap between releases; funded since 2026 via Acadia, "the 'Patreon' for Elm" [3] | Volunteer core team, public GitHub, Discourse |
| Typing | Static, HM, no type classes (built-in `comparable`, `appendable`, `number` pseudo-classes), extensible records via row polymorphism, no HKT | Static, HM plus type classes with functional dependencies, higher-kinded types, row polymorphism for records and variants, rank-n |
| Memory | JavaScript GC | JavaScript GC (Chez Scheme GC on the new backend) |
| Compilation | Elm to JavaScript; whole-program dead-code elimination | PureScript to JavaScript (CommonJS/ESM), experimental Chez Scheme and others |
| Primary domains | Browser front-ends only (by design) | Front-ends (Halogen, React bindings), some Node back-ends |
| Notable users | NoRedInk, Bekk, Prezi (early), Lamdera (full-stack Elm) | Small: a handful of companies, mostly front-end teams with Haskell backgrounds |

## Stated design goals
**Elm.** "No runtime exceptions", "great performance", "enforced semantic
versioning", and "friendly error messages" are the four claims Elm's home
page has made since the 0.19 era (the page is JavaScript-rendered and I could
not fetch its text; the claims are reproduced from memory). Czaplicki's design
writing consistently prioritises the experience of a beginner and the
reliability of the deployed artifact over expressiveness: "If the path
exists, people will walk it" is his justification for removing escape hatches
[5]. **PureScript**'s goal is Haskell's type system on the JavaScript platform
with strict evaluation, readable output JavaScript and no runtime system.

## Strengths

### Elm: compiler errors designed as a user interface
"Compiler Errors for Humans" (30 Jun 2015) is the primary source for the
practice every later language copied. Its principles [6]:
1. Show the user's own code with line numbers ("The error shows the code
   exactly as you wrote it"), not a pretty-printed reconstruction.
2. Give every error a specific hint about the likely cause ("Every message
   has a useful hint").
3. Use colour to draw attention to the problem and to separate errors.
4. Put a one-line summary above the snippet and specifics below, so the
   reader can stop early.
5. The cost was small: no change to type inference and no measurable
   slowdown, just one more piece of information attached to each constraint.
6. Emit machine-readable JSON (`--report=json`) so editors can show errors inline.
7. Maintain an "error message catalog" of programs that trigger each error,
   to find confusing messages and evaluate fixes.
"The Syntax Cliff" (Oct 2019, for 0.19.1) extended this to parse errors,
which are where beginners actually get stuck. Rust's diagnostics team cites
Elm, and Rust's `rustc` error format (source excerpt, underline, help text,
error codes) is the most visible descendant; GHC added error codes and
errors.haskell.org only in 2023.

### Elm: no runtime exceptions as a product guarantee
No `null`, no exceptions, no uncaught `undefined`, total standard library
(`List.head : List a -> Maybe a`), exhaustive `case`, immutable data, and
all JavaScript interop through typed ports. The guarantee holds in practice
for compiled Elm code; the honest caveats are `Debug.todo`, infinite
recursion/stack overflow, and bugs in kernel code. NoRedInk's reported
experience of years of production Elm with near-zero runtime errors is the
canonical anecdote. The lesson is that "no runtime exceptions" is achievable
only by removing partial functions from the *standard library*, not just by
having a sound type system.

### The Elm Architecture (TEA), copied everywhere
Model, `update : Msg -> Model -> (Model, Cmd Msg)`, `view : Model -> Html Msg`,
with effects as data (`Cmd`, `Sub`) interpreted by the runtime. Redux's
documentation: "Elm 'updaters' serve the same purpose as reducers in Redux...
Even if you don't plan to use Elm, you should read about the Elm architecture"
[7]. Iced (Rust GUI): "Inspired by The Elm Architecture" [8]. Bubble Tea (Go
TUI): "based on the paradigms of The Elm Architecture" [9]. Elmish and
Fabulous (F#), Bolero, and parts of SwiftUI/Jetpack Compose's unidirectional
data flow follow the same shape. TEA is Elm's most successful export and it
needs no language support, which is itself a lesson: the architecture, not
the language, is what people wanted.

### Elm: enforced semantic versioning from API diffs
Planned in the January 2014 package-manager post ("automatically enforce
semantic versioning by actually comparing APIs between versions and finding
type changes and additional values" [10]) and shipped in the following
years: `elm diff` shows the API difference between two versions, `elm bump`
computes the only legal next version, and `elm publish` refuses a version
number that does not match the diff. Every package starts at 1.0.0 and "Elm
figures out what the next version should be based on how the API changes"
[11]. No other mainstream package manager does this (Rust's `cargo-semver-checks`
is the closest, as an optional lint). The 2014 post already states the limit:
"the *meaning* of a function may change even if its type does not, so this
is an estimate at best" [10].

### Elm: a fast whole-program compiler
The 0.19.2 post reports an 850k-line codebase compiling from scratch in
5.7 seconds with incremental builds under 350 ms, and a 1.9x speedup on a
351-module project from reducing parser allocation [2]. A small language with
no type classes and no type-level computation is cheap to type-check; this is
the quantitative upside of Elm's minimalism.

### Elm: deliberate absence of type classes
Czaplicki rejected type classes to keep the language small and error
messages simple; instead `comparable`, `appendable` and `number` are
compiler-magic constraints. The upside is real (no instance resolution
errors, no "monad tutorial", fast compiler); the downside is below.

### PureScript: row polymorphism for records and variants
Records are `{ name :: String | r }`: functions can accept any record with at
least these fields, with inference and no declarations. The same rows power
`Variant` (open sum types) and, until 0.12, effects. This is the most
practical extensible-record system in a Haskell-like language and the direct
answer to Haskell's records problem; Elm has the record half, OCaml has the
variant half (polymorphic variants), PureScript has both with type classes
on top (`RowToList` for type-level iteration).

### PureScript: Haskell's type system, strict, with readable output
Type classes with functional dependencies and instance chains (0.12 made
overlapping instances an error, resolved via instance chains [12]),
higher-kinded types, `Effect`/`Aff` for sync/async effects, and generated
JavaScript that a JS developer can read and debug. The Registry and Spago 1.0
(2026) finally gave it a package set model like Stackage's [4].

## Weaknesses

### Elm: the 0.19 kernel-code decision and the governance crisis
In March 2018 Czaplicki announced that in 0.19 "only the `elm-lang` and
`elm-explorations` organizations can compile and publish kernel code or
effect managers"; applications and third-party packages lost the ability to
call JavaScript except through ports, for stated reasons of reliability
("arbitrary JavaScript undermines the 'if it compiles, it works' guarantee"),
portability, security, and ecosystem health. He disabled replies on the
announcement and asked for concrete cases in separate threads [5]. Teams that
had wrapped JavaScript libraries with native modules were left with no
upgrade path; the widely circulated "Elm 0.19 broke us" post (2018) and the
"Elm is dead" discourse followed, and forks appeared (Gren, 2022, a cleaned-up
Elm with arrays instead of lists and a Node target [13]). The decision was
defensible on the merits and catastrophic on process: no RFC, no transition
period, no appeal. It is the clearest BDFL-risk case study in this survey.

### Elm: 0.19.1 to 0.19.2 took 6.7 years
From 21 Oct 2019 to 6 Jul 2026 there was no compiler release, no public
roadmap and a closed issue tracker; community members maintained patched
forks (elm-janitor) to fix known compiler bugs. The 0.19.2 and 0.19.3 posts
do not discuss the gap; they announce "a series of small releases" and
describe Acadia, Czaplicki's database-related compiler, as the funding
mechanism that makes Elm sustainable again [2][3]. A language whose only
maintainer can pause it for seven years is not a platform companies can plan
around, regardless of its technical quality; NoRedInk-scale users stayed
because the compiler is stable, not because it was alive.

### Elm: the cost of no type classes
Without classes there is no `map` that works on `List`, `Maybe`, `Dict` and
`Result` (each module has its own `map`), no user-defined `comparable` (so a
`Set` or `Dict` keyed by a custom type is impossible without encoding it as a
tuple or string), no `Functor`/`Monad` abstraction for libraries, and no
numeric abstraction beyond the magic `number`. Community workarounds
(`elm-sorter-experiment`, `AnySet`) exist because the built-in pseudo-classes
cannot be extended. Elm proves that type classes can be omitted for a narrow
domain; it also proves the omission does not scale beyond it.

### Elm: one target, one domain
No server-side story, no WebAssembly, no native target, JavaScript interop
only through asynchronous ports and web components, and no local mutation
(so hot loops allocate). These are consequences of the "small language for
front-ends" goal and are only weaknesses if generality is wanted, which is
exactly the question for Sisu.

### PureScript: effect rows abandoned
Until 0.12 (May 2018) effectful code had types like
`Eff (console :: CONSOLE, random :: RANDOM | e) Unit`; the 0.12 release
introduced "the new simplified `Effect` type" and the ecosystem migrated to
plain `Effect` [12]. The commonly stated reasons (from the migration-era
discussion; I could not retrieve the primary migration guide) were that the
row labels were not enforced at the FFI boundary and so gave no real
guarantee, that they made every signature longer, and that unification errors
over effect rows were hard to read. This is a shipped language *removing* a
fine-grained effect system because its cost exceeded its unenforced benefit,
and it is directly relevant to any effect-system design: effect labels must
be sound at the boundary and mostly inferred, or users will vote them out.

### PureScript: small adoption and a thin team
No company of note runs on PureScript at scale; the ecosystem is a few
hundred active packages; the compiler is maintained by a handful of
volunteers; the Chez Scheme backend (2026) is an attempt to escape the
JavaScript-only niche [4]. Haskell's type system on JavaScript turned out to
appeal mostly to people who already knew Haskell, while TypeScript captured
the JavaScript developers who wanted types.

### Both: the Haskell-syntax tax
Layout rule, `<$>`/`<*>`/`>>=` operator soup (PureScript), and currying with
`|>` versus `<|` (Elm) all read as foreign to the JavaScript developers these
languages target. Elm's error-message investment was in part an attempt to
compensate for syntax that was never going to look like JavaScript.

## Designer retrospectives
- Czaplicki, "Native code in 0.19" (Mar 2018): native modules were "a hack for
  my own needs in 2011" never meant as a public API; "If the path exists,
  people will walk it" [5].
- Czaplicki, "Compiler Errors for Humans" (2015) and "Compilers as
  Assistants" (2015): the compiler should be a helper, not an adversary;
  message quality is a design discipline, not a by-product [6].
- Czaplicki's talks "What is Success?" (Elm Europe 2018), "The Hard Parts of
  Open Source" (Strange Loop 2018) and "The Economics of Programming
  Languages" (Strange Loop 2023) are his public account of why he restricted
  community input, why open-source maintenance burned him out, and why Elm
  needed a commercial funding vehicle (now Acadia). They are retrospectives
  on governance rather than on language features.
- The 0.19.3 post describes making the compiler "correct by construction"
  with distinct name types and non-empty cycle types that exposed "a pretty
  subtle bug" in nested extended-record handling [3], which is a nice
  self-application of Elm's philosophy to its own compiler.
- PureScript 0.12's release notes record the design reversal on effect rows
  and the switch to instance chains for overlapping instances [12].

## Influence
Elm gave: TEA to Redux, Iced, Bubble Tea, Elmish/Fabulous/Bolero and the
general "unidirectional data flow" vocabulary [7][8][9]; error-message
design to Rust and, later, to GHC, Swift and TypeScript; API-diff-enforced
semver to `cargo-semver-checks`-style tools; the "no runtime exceptions"
marketing frame to Roc (Richard Feldman, a former Elm core contributor, built
Roc explicitly as "Elm for everything", see `roc.md`) and Gleam. Elm took:
Haskell's syntax and HM, OCaml-ish records, FRP (removed in 0.17, 2016,
"Farewell to FRP") and then the command/subscription model.

PureScript gave: row-polymorphic records and `Variant` as the demonstration
that the Haskell records problem has a known solution; instance chains; a
model for Haskell-to-JS languages that keep type classes. PureScript took:
almost everything from Haskell, rows from Wand/Rémy/Gaster-Jones TRex,
strictness from the JavaScript host.

## Lessons for Sisu
- **Adopt**
  - Error messages as a designed artifact from the first compiler commit:
    source excerpt, one-line summary, specific hint, colour, JSON output,
    and a catalogue of error-triggering programs as a regression suite [6].
  - API-diff-enforced semantic versioning in the package manager (`diff`,
    `bump`, refuse-to-publish). It is cheap given a typed public interface
    and it is unique enough to be a reason to choose the language.
  - A total standard library with no partial functions and no `null`, which
    is what makes "no runtime exceptions" true rather than aspirational.
  - Row-polymorphic records (and ideally variants) à la PureScript as the
    answer to the Haskell records problem, if the type checker can keep
    errors readable.
- **Adapt**
  - TEA as the *recommended* application architecture in docs and
    frameworks, not as a language feature; its adoption outside Elm shows
    it needs nothing from the language except ADTs and immutability.
  - "No runtime exceptions" should be a tiered guarantee: a pure core where
    it holds absolutely, and an explicitly marked boundary (FFI, `todo`,
    resource exhaustion) where it cannot.
  - Elm's compiler speed comes from a small type system; Sisu should budget
    type-level features against compile time explicitly, not assume they are free.
- **Avoid**
  - Sole-maintainer governance with a closed roadmap and no RFC process.
    Elm lost years and forks to it; the technical merits of 0.19 did not
    matter to the people whose code stopped compiling.
  - Removing an escape hatch without a migration path. Keep an unsafe FFI
    with loud markers rather than none.
  - Omitting ad-hoc polymorphism in a general-purpose language; the
    `comparable` hack is fine for a front-end DSL and nothing else.
  - Fine-grained effect labels in every signature unless they are inferred
    and enforced at the boundary; PureScript shipped them and then deleted them.
- **Open questions raised**
  - How much of Elm's learnability comes from *absence* (no classes, no
    HKT, no effects in types) versus from *presence* (error messages, total
    library)? Roc is testing one answer; Sisu's answer determines whether a
    general-purpose language can be as approachable.
  - Can a BDFL's coherence be preserved under an open process? Elm's design
    quality is partly a product of one person's veto; Haskell's extension
    sprawl is partly a product of its absence.
  - Is API-diff semver enough when behaviour changes without type changes?
    Elm's own 2014 post says no; contracts or property tests on the public
    API might close the gap.

## Sources
1. Elm compiler releases on GitHub (0.19.0 21 Aug 2018; 0.19.1 21 Oct 2019; 0.19.2 6 Jul 2026; 0.19.3 2 Oct 2026). https://github.com/elm/compiler/releases (2026)
2. Czaplicki, "Faster Builds with Elm 0.19.2" (6 Jul 2026). https://elm-lang.org/news/faster-builds (2026); source https://github.com/elm/elm-lang.org/blob/master/pages/news/faster-builds.elm
3. Czaplicki, "Another step towards Elm 1.0" (0.19.3, Oct 2026). https://elm-lang.org/news/another-step-towards-elm-v1 (2026)
4. PureScript Discourse announcements: "PureScript v0.15.16 released!" (18 Mar 2026); "Registry and Spago 1.0 Launch" (25 Feb 2026); "Chez Scheme backend" (21 Jun 2026); "PureScript Analyzer Public Alpha" (11 Sep 2025). https://discourse.purescript.org/c/announcements (2025-26)
5. Czaplicki, "Native code in 0.19", Elm Discourse (11 Mar 2018). https://discourse.elm-lang.org/t/native-code-in-0-19/826 (2018)
6. Czaplicki, "Compiler Errors for Humans" (30 Jun 2015). https://elm-lang.org/news/compiler-errors-for-humans (2015); "Compilers as Assistants" (19 Nov 2015) https://elm-lang.org/blog/compilers-as-assistants; "The Syntax Cliff" (21 Oct 2019) https://elm-lang.org/news/the-syntax-cliff
7. Redux documentation, "Prior Art: Elm". https://redux.js.org/understanding/history-and-design/prior-art (2026)
8. Iced README ("Inspired by The Elm Architecture"). https://github.com/iced-rs/iced (2026)
9. Bubble Tea README ("based on the paradigms of The Elm Architecture"). https://github.com/charmbracelet/bubbletea (2026)
10. Czaplicki, "Elm Package Manager" (1 Jan 2014). https://elm-lang.org/news/package-manager (2014)
11. package.elm-lang.org README ("Elm figures out all version numbers automatically based on your API"). https://github.com/elm/package.elm-lang.org (2026)
12. PureScript v0.12.0 release notes (new `Effect` type; overlapping instances now an error; `ado`). https://github.com/purescript/purescript/releases/tag/v0.12.0 (2018)
13. Gren language site. https://gren-lang.org/ (2026)
14. Czaplicki, "The Hard Parts of Open Source", Strange Loop (2018); "What is Success?", Elm Europe (2018); "The Economics of Programming Languages", Strange Loop (2023). Videos on YouTube; cited for governance context.
15. Czaplicki, "A Farewell to FRP" (0.17, May 2016). https://elm-lang.org/news/farewell-to-frp (2016)
16. Stack Overflow Developer Survey 2024 (Elm 0.6% under web frameworks; not listed as a language). https://survey.stackoverflow.co/2024/technology (2024)
