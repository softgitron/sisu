# Clojure

## Snapshot
| | |
|---|---|
| First appeared | 17 October 2007 (public release); designed 2005-2007. |
| Designed by | Rich Hickey. Core team at Cognitect (acquired by Nubank, July 2020); Alex Miller runs releases and the survey. |
| Current version (Oct 2026) | 1.12.5 (12 May 2026); 1.12.0 shipped 5 September 2024; 1.13.0-alpha5 (21 July 2026) raises the minimum to Java 17. |
| Governance | BDFL. Rich Hickey decides; contributors sign a CA; issues go through ask.clojure.org and JIRA; no RFC process and no breaking changes as a stated value. |
| Typing | Dynamic, strong. Optional runtime contracts: `clojure.spec` (alpha since 1.9, 2017), `malli` (community). Type hints for interop performance only. |
| Memory | Host GC (JVM, JS engine, CLR). |
| Compilation | Compiled to JVM bytecode at load time (no interpreter); ClojureScript compiles via Google Closure; Babashka interprets (GraalVM native image); jank compiles natively via LLVM. |
| Primary domains | Back-end services, fintech, data processing, web (full-stack with ClojureScript), scripting (Babashka). |
| Notable users | Nubank (largest, Brazil's biggest digital bank), Walmart, Funding Circle, CircleCI, Metabase, Datomic. |

## Stated design goals

Hickey's rationale: a "functional Lisp for the JVM with strong concurrency
support"; "a Lisp not constrained by backwards compatibility"; "all data
structures immutable & persistent"; "for the concurrent programming future,
pervasive, unmoderated mutation simply has to go"; "locking is too hard to
get right over and over again"; polymorphism "decoupled from OO and types"
via multimethods (and later protocols) [1]. The HOPL IV paper restates the
goal as a practical, general-purpose functional language "suitable for use
by professionals wherever its host language, e.g., Java, would be" [2]. Three
non-negotiables run through every talk: data over objects, values over
places, and simplicity (lack of interleaving) over ease (familiarity).

## Coverage of the 15 evaluation dimensions

1. **Syntax and readability.** S-expressions with literal vectors `[]`, maps `{}` and sets `#{}`; almost no syntax; readable with structured editing, hard without; destructuring and threading macros carry most readability.
2. **Type system.** Dynamic, strong; optional runtime contracts (`spec`, alpha since 2017; `malli`); type hints only for interop performance; core.typed failed to gain adoption.
3. **Memory management and safety.** Host GC (JVM/JS/CLR); no memory unsafety; `nil` is pervasive and nil-punning is idiomatic rather than checked.
4. **Mutability, aliasing and value semantics.** Immutable persistent collections by default; values compare by structure; mutation only through reference types (`atom`, `ref`, `agent`, `var`) with explicit coordination semantics; transients for local batch mutation.
5. **Error handling.** Host exceptions (`try`/`catch`/`throw`, `ex-info` with data maps); no `Result` idiom in core; error messages and stack traces are the long-standing top complaint.
6. **Concurrency and parallelism.** Immutable data plus atoms/STM/agents; `future`, `pmap`, `core.async` channels and `go` blocks (as a macro); Java virtual threads usable since 1.12; no data-race-freedom guarantee for mutable host objects.
7. **Polymorphism.** Protocols (type-dispatched, open to extension), multimethods (arbitrary dispatch function), `reify`/`deftype`/`defrecord` for host interfaces; no inheritance in Clojure code.
8. **Metaprogramming.** `defmacro` with syntax-quote and auto-gensym; no user reader macros; macros are the implementation vehicle for `core.async`, `spec`, threading forms.
9. **Modules, packages, build.** Namespaces mapped to files; `deps.edn`/tools.deps (official) and Leiningen; Maven Central and git deps; `add-lib` at the REPL (1.12); no lockfile by default in tools.deps.
10. **Tooling.** nREPL/CIDER/Calva/Cursive editor integration; `clj-kondo` linter; `cljfmt`/`zprint`; test frameworks (`clojure.test`); no official formatter; Babashka for scripting; startup time addressed outside the core.
11. **Interop and platforms.** Zero-overhead Java interop (1.12 method values, functional interfaces); ClojureScript to JS with Google Closure; ClojureCLR, ClojureDart; GraalVM native-image for CLIs.
12. **Performance and compilation model.** Compiled to JVM bytecode at load; JIT performance close to Java for hinted code; persistent-collection overhead versus arrays; slow startup; `*warn-on-reflection*` needed to avoid reflective calls.
13. **Standard library and ecosystem.** Small, stable `clojure.core` (sequence library is the heart); contrib libraries; rich ecosystem for web/data (Ring, Reitit, next.jdbc, Datomic); conventions over frameworks.
14. **Learnability.** Hard first weeks (parentheses, REPL workflow, error messages), then high retention; 82% of users have 6+ years of professional experience; only 1% arrive via university courses.
15. **Evolution and governance.** BDFL (Hickey) with Cognitect/Nubank; roughly one minor release a year; no breaking changes as policy ("accrete, relax, fix"); slow issue intake is the recurring complaint.

## Strengths

### Persistent immutable collections as the default, copied widely
Clojure's vectors (32-way bit-partitioned tries), hash maps and sets (hash
array mapped tries after Bagwell's 2001 "Ideal Hash Trees", made persistent
by path copying) made immutable collections cheap enough to be the only
collections. Structural sharing gives O(log32 n) update and effectively O(1)
read; transients give batch-mutation performance inside a pure API. This is
the group's strongest adoption evidence: Scala's immutable `Vector` and
`HashMap` (2.8, 2010) are Bagwell/Clojure-derived; Immutable.js (Facebook, 2014) names Clojure among its inspirations;
Erlang/OTP 18 (2015) large maps are HAMTs; Elixir (via Erlang) and Kotlin's
kotlinx.collections.immutable use the same structures. Hickey's "The Value of Values" (2012) is the
argument: values can be shared, cached, compared and sent across threads and
time without coordination; places cannot [3].

### Simplicity as a measurable design criterion
"Simple Made Easy" (Strange Loop 2011) defines simple as "one fold, one
braid" (not interleaved) versus easy (near at hand, familiar), and argues
that reliability requires simplicity, not ease [4]. The concrete design
consequences in Clojure: data as plain maps rather than classes (no
"complecting" of data with behaviour), functions rather than methods,
namespaces rather than object hierarchies, values rather than state, queues
rather than direct calls. The talk is among the most-cited language-design
talks of the 2010s and is the stated inspiration for 32% of new Clojure
users in the 2025 survey ("inspired by a conference talk or video by Rich
Hickey or others") [5].

### Controlled state: reference types with explicit semantics
Mutation lives in four reference types with distinct concurrency semantics:
`atom` (uncoordinated, synchronous, compare-and-swap), `ref` (coordinated
via software transactional memory), `agent` (asynchronous, serialised),
`var` (thread-local rebinding). Each holds an immutable value; `swap!`
applies a pure function. The STM was the headline in 2008 and is rarely used
in practice (atoms suffice), but the design principle - mutation is an
explicit, typed choice of coordination model, never the default - held up
and was copied by Elixir's `Agent` and by Rust's naming of `Cell`/`RefCell`
as opt-in interior mutability.

### Hosted language strategy and interop
Clojure runs on the JVM, JavaScript (ClojureScript, 2011), the CLR, Dart
(ClojureDart), natively via GraalVM (Babashka) and LLVM (jank). Hickey's
decision to embrace "an industry-standard, open platform" rather than build
a runtime [1] gave Clojure Java's libraries, GC, JIT and deployment story at
zero cost, and is why a one-person language reached fintech production.
1.12 (2024) deepened it: Java method values as first-class functions,
functional-interface interop without `reify`, `add-lib` to load dependencies
into a running REPL, Java `Stream` adapters [6]. The cost shows in error
messages (see Weaknesses).

### REPL-driven development as a workflow, not a tool
Because namespaces, vars and the compiler are live at run time, the normal
workflow is to connect an editor to a running process (nREPL/CIDER/Calva)
and evaluate forms into it: redefine a function, re-run a request, inspect
a value, all without restarting. The 2025 survey shows the tool culture this
produced: Emacs plus Vim is roughly half of respondents; new users use VS
Code with Calva; 81% of experienced developers use structured editing
(paredit-style) [5]. The productivity claim is anecdotal but the imitation
is not: Elixir's IEx `recompile`, Julia's Revise, and JavaScript hot-module
reload all cite Lisp REPLs; the difference is that in Clojure the REPL is the
primary interface, and the language (vars as indirection, `defn` as
rebinding) is designed for it.

### Macros that are "just data": homoiconicity without reader macros
Clojure has `defmacro` with `syntax-quote` (auto-namespace-qualifying symbols
and `gensym` via `x#`), giving most of hygiene with none of Scheme's
machinery, and deliberately no user reader macros. The standard library's
`->`, `->>`, `cond->`, `core.async`'s `go` (a CPS transform written as a
macro in user space, 2013) and `clojure.spec`'s instrumentation are all
ordinary macros. Threading macros are the Lisp form of the pipeline idiom that
Elixir's `|>` (which Valim credits to F#) and JavaScript pipeline proposals
also adopted.

### Stability: no breaking changes, by policy and in practice
Code written for Clojure 1.0 (2009) runs on 1.12. "Spec-ulation" (Clojure/conj
2016) argues that semantic versioning's "breaking change" is a euphemism for
breaking users, that a library should only *accrete* (add), *relax* (accept
more) and *fix* (bugs), never remove or narrow, and that a changed function
should get a new name or namespace [7]. "Maybe Not" (2018) extends this to
types: changing an argument from `A` to `Maybe A` or a return from `Maybe A`
to `A` is a compatible relaxation that `Maybe`-typed signatures make
needlessly breaking [8]. Clojure 1.12 kept Java 8 bytecode "expected to be
the last" [6]; 1.13 moves to Java 17, nine years after Java 8 reached end of
public updates. The 2025 survey's 70% "very likely to recommend" and the
16% with 11-15 years on the language are consistent with a platform people
stay on [5].

## Weaknesses

### Error messages and stack traces: the top complaint since 2011
Clojure has no error-reporting layer of its own: most errors are JVM
exceptions thrown from `clojure.lang.*` internals, with stack traces showing
the compiler's and runtime's Java frames rather than the user's forms.
Secondary sources agree that error messages were the most-cited frustration
in the State of Clojure survey for several consecutive years and appear as
far back as 2011-2015 [9, 10]; the official 2024 and 2025 result pages only
publish the charts as images, so the exact percentages could not be
verified here. Fixes shipped: Clojure 1.9 (2017) added spec-based checking
of `defn`/`ns`/`let` syntax, which made some messages *worse* (a wall of
`spec/explain` output for a missing bracket); 1.10 (2018) added error
*phases* (read, macroexpand, compile, eval, print) and `ex-triage`/`ex-str`
to format them [11]; 1.13 alphas add `:keys!` checked destructuring. The
jank project (native Clojure) treats "beat Clojure's error reporting" as a
headline goal [9]. The lesson is structural: a hosted language inherits the
host's error model unless it builds its own, and retrofitting it after ten
years is hard.

### Startup time
Loading `clojure.core` compiles and initialises thousands of vars; a bare
JVM Clojure process takes on the order of a second before user code runs,
and a real application with many namespaces takes several seconds (common
measurements; not independently benchmarked here). This excluded Clojure
from scripting and serverless until Babashka (2019), whose README shows
`bb -e` completing in about 17 ms, and which 3 in 5 survey respondents used
in 2025 [5, 12]. Babashka is a *second implementation* (interpreter on
GraalVM native image) maintained outside the core team; the need for it is
the admission.

### Dynamic typing with alpha-forever contracts
`clojure.spec` (1.9, 2017) provides runtime contracts, generative testing
and instrumentation, but was labelled alpha, has a known redesign ("spec 2")
in progress since 2019 that has not shipped, and ranks "quite a bit lower"
among new users' priorities in 2024 [13]. The community's de facto standard
is `malli` (data-driven schemas, 2019-), a third-party library. Without
static types, the IDE cannot offer completion on map keys, refactoring is
textual, and the "data over objects" stance means a function's input shape
is documented only by convention (keyword namespaces, `:user/id`) or by a
spec that is rarely written. core.typed (2012-) failed to gain adoption; CircleCI, its best-known
production user, dropped it in 2015 citing slow checking and poor fit with
idiomatic code (heterogeneous maps, higher-order sequence functions).

### Parentheses and tooling dependence
The 2025 survey finds 48% of new users (one year or less) do not use or are
unsure about structured editing, versus 19% of experienced users [5]: the
syntax is learnable, but only with editor support, which is a cost on the
first week. Clojure's indentation and bracket discipline are not enforced by
the compiler; a formatter (`cljfmt`, `zprint`) is not part of the
distribution. See `lisp-scheme-racket.md` for the broader parentheses
evidence (Rhombus).

### Governance: a small core, slow issue flow
All design decisions pass through Hickey; patches require a signed agreement
and often wait years in JIRA. The upside is coherence and no breaking
changes; the downside is that the top community complaint (errors) has
been open for 15 years, `spec` has been alpha for nine, and 1.12 took two and a half
years. The survey's "would you like to see improved" open answers
consistently include docs, errors and "a clearer contribution process";
the project has not adopted an RFC or working-group model.

### Small language, large host: what the community wishes it had
Across surveys, the recurring wishes are better error messages, better
documentation for beginners, hiring/jobs (fintech concentration: fintech,
enterprise software and healthcare are 51% of use [5]), static typing or
better editor inference, and faster startup. Only 1% of new users came via
a university class [5], which is a pipeline problem for a 19-year-old
language.

## Designer retrospectives

- HOPL IV "A History of Clojure" (2020) [2]: Hickey describes the goals as
  met and does not list regrets in the "mistakes" sense (the paper is a
  history, not a retrospective); he defends dynamic typing, the hosted
  strategy, and the no-breaking-changes policy, and acknowledges that error
  reporting and the learning curve of a REPL-centric workflow are ongoing
  weaknesses. (Direct quotation was not possible: the ACM copy is paywalled
  from this environment.)
- "Spec-ulation" (2016) [7] is a retrospective on the *ecosystem's*
  practices rather than the language: Hickey argues that the Clojure core's
  refusal to break has been validated and that dependency-versioning
  schemes are solving the wrong problem.
- "Maybe Not" (2018) [8]: a retrospective on `spec` itself - Hickey argues
  that specs should describe *shapes* (which keys may be present) separately
  from *selections* (which keys a given function requires), which is the
  motivation for spec 2; this is an acknowledgement that spec 1 conflated
  the two.
- Alex Miller's survey posts (2022-2026) acknowledge new-user challenges in
  error messages and tooling each year without proposing a core fix [5, 13].

## Influence

Copied from Clojure: persistent HAMT/vector collections (Scala, Immutable.js,
Erlang maps, Kotlin, Elixir); threading/pipeline macros (Elixir `|>`, R,
JavaScript proposals); transducers (2014; Ramda implements the protocol, Rust iterator
adapters are the typed analogue); `core.async` channels and `go` blocks
(explicitly modelled on Go, implemented as a macro in user space); protocols as the "Rust trait"-like answer to the
expression problem in a dynamic language (Elixir protocols are a direct
copy); EDN as a data format; the "data over objects" style (Elixir, Janet,
Fennel); and the reluctance to break (the "accrete, don't break" argument is now
standard vocabulary in library-versioning debates [7]). Clojure copied from: Common Lisp (macros, multimethods,
dynamic vars), Scheme (lexical scope, `fn`), Haskell (lazy sequences,
`Maybe`-free `nil` punning is its own), Erlang (agents as a nod to actors),
Java (the host), and ML (destructuring).

## Lessons for Sisu

- **Adopt**: *persistent collections with structural sharing* as the default
  collection types, with a transient/builder API for batch mutation. This
  is the most-copied idea in this group and composes with value semantics
  and with message passing (no copy on send).
- **Adopt**: *the "accrete, relax, fix; never break" policy for the standard
  library*, and the corollary from "Maybe Not": design signatures so that
  accepting more or returning more is not a breaking change (e.g. optional
  fields in records rather than `Option` wrappers on every argument).
- **Adopt**: *reference types with explicit coordination semantics* (`atom`
  CAS, agent/mailbox, transactional) rather than one `mutable` keyword;
  Sisu's ownership model should name the sharing mode in the type.
- **Adopt**: *explicit "simple vs easy" review of each feature*: does it
  interleave two concerns (syntax with semantics, data with behaviour, state
  with identity)? Hickey's taxonomy is a usable design checklist.
- **Adapt**: *REPL-driven development* into a compiled language: hot
  function replacement in debug builds, expression evaluation against a
  running process, and value inspection. The requirement is an
  indirection point per top-level definition (Clojure's var) that release
  builds can remove.
- **Adapt**: *macros as functions on data* with automatic namespace
  qualification, but with the Gleam/Rust caveat: Sisu should prefer
  hygienic, declarative macros and keep procedural macros rare.
- **Adapt**: *data over objects.* Plain record/map literals with named
  fields and no behaviour should be the default data model; Sisu can keep
  this *and* static types by making structural record types first-class.
- **Avoid**: *inheriting the host's error model.* Error messages must be a
  first-class subsystem from day one; Clojure shows that a hosted language
  that does not own its diagnostics never fully recovers.
- **Avoid**: *shipping core features as "alpha" indefinitely* (`spec`); a
  contract/validation story should be either in the language or clearly a
  library.
- **Avoid**: *dynamic typing justified by "data is the interface"*: the
  community's wish list (editor inference, map-shape documentation, static
  checks) is the list of things types provide; Sisu can keep the open
  record idiom with row-polymorphic or structural record types.
- **Open questions raised**: (1) Is "never break" compatible with fixing
  early mistakes in a young language? Clojure chose never; Gleam chose
  pre-1.0 breakage then never; Sisu needs a pre-1.0 policy and a post-1.0
  policy. (2) Can a statically typed language keep Clojure's open,
  heterogeneous maps as a core idiom without losing inference quality?
  (3) BDFL governance delivered coherence and 17 years of compatibility
  while leaving the top complaint unfixed; what structure fixes both?

## Sources

1. Rich Hickey, "Clojure Rationale". https://clojure.org/about/rationale (2008-)
2. Rich Hickey, "A History of Clojure", Proc. ACM Program. Lang. 4, HOPL, 2020. https://dl.acm.org/doi/10.1145/3386321
3. Rich Hickey, "The Value of Values", JaxConf 2012 (video/transcript widely mirrored).
4. Rich Hickey, "Simple Made Easy", Strange Loop 2011. https://www.infoq.com/presentations/Simple-Made-Easy/
5. Alex Miller, "State of Clojure 2025 Results", 18 February 2026. https://clojure.org/news/2026/02/18/state-of-clojure-2025
6. "Clojure 1.12.0 release", 5 September 2024. https://clojure.org/news/2024/09/05/clojure-1-12-0
7. Rich Hickey, "Spec-ulation", Clojure/conj 2016; critique by Edward Z. Yang, "Thoughts about Spec-ulation", 2016. https://blog.ezyang.com/2016/12/thoughts-about-spec-ulation-rich-hickey/
8. Rich Hickey, "Maybe Not", Clojure/conj 2018.
9. jank blog, "Can jank beat Clojure's error reporting?", 28 March 2025 (states error reporting has topped survey improvement areas since 2015). https://jank-lang.org/blog/2025-03-28-error-reporting/
10. ClojureVerse, "Improving error messages in Clojure (as a library)?" (community reading of surveys). https://clojureverse.org/t/improving-error-messages-in-clojure-as-a-library/1765
11. Clojure 1.10 changelog, "Error messages" (phases, `ex-triage`), December 2018. https://github.com/clojure/clojure/blob/master/changes.md
12. Babashka README (startup example, motivation). https://github.com/babashka/babashka
13. Alex Miller, "State of Clojure 2024 Results", 2 December 2024. https://clojure.org/news/2024/12/02/state-of-clojure-2024
14. Clojure 1.13.0-alpha5 announcement (Java 17 minimum), 21 July 2026. https://clojure.org/news/2026/07/21/clojure-1-13-alpha5
15. Phil Bagwell, "Ideal Hash Trees", EPFL technical report, 2001.
16. Wikipedia, "Clojure" (infobox 1.12.5, 12 May 2026; dialects). https://en.wikipedia.org/wiki/Clojure
