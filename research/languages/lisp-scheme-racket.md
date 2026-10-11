# Common Lisp, Scheme and Racket

## Snapshot
| | |
|---|---|
| First appeared | Lisp 1958 (McCarthy). Scheme 1975 (Sussman and Steele, MIT). Common Lisp 1984 (CLtL), ANSI standard 1994. PLT Scheme 1995, renamed Racket 2010. |
| Designed by | Common Lisp: ANSI X3J13 committee (Steele, Gabriel, Pitman, Moon et al.). Scheme: Sussman and Steele; later RnRS editors/Steering Committee. Racket: Matthias Felleisen, Matthew Flatt, Robert Findler, Shriram Krishnamurthi and the PLT group. |
| Current version (Oct 2026) | Common Lisp: ANSI INCITS 226-1994, never revised; SBCL 2.6.9 (26 Sept 2026, monthly releases). Scheme: R7RS-small (2013) final; R7RS-large still in development. Racket 9.3 (13 Aug 2026); Rhombus 1.1 (17 Aug 2026) on Racket. |
| Governance | CL: frozen standard; de facto evolution via implementations and quasi-standard libraries (ASDF, Quicklisp, CFFI). Scheme: working groups and public votes (R6RS ratified 67-35 in 2007). Racket: core team plus Racket Programming Language Foundation; releases every three months. |
| Typing | Dynamic, strong, with optional declarations (CL: `declare`, SBCL infers and checks declared types at compile time). Racket: untyped by default, Typed Racket is a sound gradual dialect via `#lang typed/racket`. |
| Memory | Tracing GC in all implementations (SBCL generational; Racket CS uses Chez Scheme's GC). |
| Compilation | CL: native compilers (SBCL, CCL) with full-language image; Scheme: native (Chez), to C (Chicken, Gambit), bytecode (Guile); Racket CS compiles via Chez Scheme to machine code (default since 8.0, Feb 2021). |
| Primary domains | CL: symbolic computation, scheduling/planning, rule engines, long-lived commercial systems. Scheme: teaching, embedding (Guile in GNU tools), research. Racket: teaching (HtDP, Bootstrap), language research, DSLs, Rhombus. |
| Notable users | CL: ITA Software/Google Flights, Grammarly's grammar engine, NASA JPL (Deep Space 1 remote agent, Mars Pathfinder planning), American Express Authorizer's Assistant, Hacker News (moved to SBCL in 2024), pgloader. Racket: Hacker News' Arc was built on it, Bootstrap curriculum, Pyret's first implementation. |

## Stated design goals

Common Lisp was a standardisation effort: unify MacLisp, Lisp Machine Lisp,
Interlisp and others into one portable, industrial language "with a large
library"; Steele and Gabriel's HOPL-II "The Evolution of Lisp" (1993)
describes it as a political compromise that favoured the existing user base
over minimalism [1]. Scheme's goal was the opposite: the smallest language
that supports all major paradigms, from the first report's preface: "Programming
languages should be designed not by piling feature on top of feature, but by
removing the weaknesses and restrictions that make additional features
appear necessary" (R3RS onward) [2]. Racket's goal is *language-oriented
programming*: "Racket is a programming language for creating new programming
languages"; developers should "state and solve problems in the vocabulary of
the domain" by building that vocabulary as a language, with "full-spectrum"
protection from "C-level bit manipulation to soundly typed extensions" [3,
4].

## Strengths

### Homoiconicity and macros: syntactic abstraction that reaches the whole program
Code is a data structure the language can manipulate, so a macro is an
ordinary function from syntax to syntax. The practical payoff is that
language features are libraries: CL's `loop`, `defstruct`, CLOS (`defclass`,
`defmethod`) and the `format` language; Scheme's pattern matching, `do`
loops and object systems; Racket's `match`, class system, contracts, Typed
Racket (the entire type checker is a macro), and `#lang` languages. Internal
DSLs stay inside one toolchain, so a DSL program gets the host's compiler,
debugger and editor. Evidence of value: every later language with a real
macro system (Rust, Elixir, Julia, Nim, Scala 3) cites Lisp, and Racket's
macro-expander research (`syntax-case`, sets of scopes) is what Rust's
macro hygiene borrows from. Evidence of cost: see "The Lisp curse" below.

### Hygienic macros (Scheme/Racket) versus `defmacro` with `gensym` (CL)
CL's `defmacro` is an arbitrary function; variable capture is avoided by
convention with `gensym`, and a macro can shadow a user's binding of `list`
silently. Scheme's `syntax-rules` (R5RS, 1998) is declarative and hygienic by
construction; `syntax-case` (R6RS, 2007; Dybvig) adds procedural power with
hygiene preserved; Racket's "sets of scopes" expander (Flatt, 2016) makes
hygiene work across modules, phases and languages. The evidence that hygiene
is the right default: every widely used macro system designed after 1990 (Racket,
Rust, Elixir, Julia, Scala 3) is hygienic by default with an explicit
unhygienic escape; nobody copied bare `defmacro`.
The evidence that CL's choice is survivable: 40 years of CL code, and
CL macro writers consider anaphoric and intentionally capturing macros a
feature (Graham's *On Lisp*, 1993).

### The condition system: handlers that run before the stack unwinds, plus restarts
Common Lisp separates three roles [5]: *signalling* (`signal`, `error`,
`warn` create a condition object); *handling* (`handler-bind` runs a
handler function *in the dynamic context of the signal, before any
unwinding*, and the handler may decline, so the next handler is tried);
and *restarts* (`restart-case` establishes named recovery points, "named
continuations", that a handler, or the interactive debugger, can choose with
`invoke-restart`). `handler-case` is the exception-like sugar that unwinds.
Concretely: a parser deep in a call stack signals `malformed-record` and
offers restarts `skip-record`, `use-value` and `retry`; a top-level policy
installed with `handler-bind` picks `skip-record` for batch jobs and the
debugger offers all three to a developer; the parser's stack is intact when
the choice is made, so work is not lost. Pitman: condition handling "is
primarily about protocol, rather than mere computational ability" - its value
is letting independently written code cooperate on recovery [5]. Exceptions
offer only the unwinding half. Where it matters: long-running image-based
systems (fix the bug in the debugger, choose `retry`, keep the 6-hour
computation) and library code that cannot know the right recovery policy.
Evidence of influence: Dylan (dropped the "condition firewall", which Pitman
calls the most controversial part), Smalltalk's resumable exceptions, and
Rust's `catch_unwind`-free error design are all positioned relative to it;
no mainstream language adopted restarts, which is weak evidence that the
cost (dynamic scope, debugger integration, unclear static typing) outweighs
the benefit outside image-based systems.

### Interactive, image-based development
In CL and Racket (and Smalltalk) the compiler and the program share a
process. One can redefine a function, a class (`update-instance-for-
redefined-class` migrates live instances), or a macro while the program
runs; SLIME/Sly in Emacs, and DrRacket, evaluate forms into the live image;
SBCL compiles each form to native code on evaluation. The condition system
makes this a workflow: an unhandled error opens a debugger with restarts,
the developer fixes the function and chooses `retry`. What static compiled
languages lose is documented by the imitators: Julia's Revise, Elixir's
IEx, Rust's `cargo watch` and JavaScript's HMR are all attempts to get
part of this back. The costs are also known: an image accumulates state that
is not in the source (see Smalltalk), and compiling at run time requires
the compiler in the deployed binary.

### Racket's `#lang`: languages as modules
A Racket file begins with `#lang <name>`; the name resolves to a module that
supplies a *reader* (lexical syntax) and an *expander* (bindings, including
`#%module-begin`, `#%app`, `#%datum`), so a language can change everything
from tokens to function-call semantics while still being a module that
other languages can `require` [6]. Shipped languages: `racket/base`,
`typed/racket`, `scribble` (documentation, prose-first syntax), `datalog`,
`lazy`, `slideshow`, `plai`, the HtDP teaching languages (BSL, ISL, ASL) and
now `rhombus`. Typed Racket is the proof that a *sound gradual type system*
can be implemented as a `#lang` with contracts at the boundary (Tobin-
Hochstadt and Felleisen, 2006-2008), which is where the gradual typing
literature's "blame" machinery was first built. No other platform lets a
teaching language, a typed language and a prose language share a module
system, IDE and package manager.

### Teaching results: HtDP, Bootstrap and Pyret
DrRacket was built for TeachScheme! (1995-); *How to Design Programs* (2001;
2nd ed. 2018) teaches a "design recipe" (data definition, signature,
purpose, examples, template, tests) using successively larger teaching
languages. Bootstrap (2006-) teaches algebra through Racket-based game
programming; studies report that students improve on algebraic word
problems after the module, including unscaffolded ones (Schanzer, Fisler,
Krishnamurthi; SIGCSE 2015, 2018), and an independent BYU control-group
study found significantly better understanding of variables but no transfer
of function composition or piecewise functions [7, 8]. Pyret (2013-; Brown)
was first implemented in Racket and was designed from HtDP experience:
Python-like syntax, built-in testing (`where:` blocks), no parentheses. The
result that matters for language design: the same team that proved the
teaching method moved *off* S-expression syntax for its own teaching
language.

### Multiple dispatch and the MOP (CLOS)
CLOS (1988; AMOP 1991) dispatches generic functions on all arguments, has
multiple inheritance with a defined class precedence list, method
combination (`:before`/`:after`/`:around`, `+`, `list`), and a metaobject
protocol that lets user code change how classes, slots and dispatch work
(persistent objects, validation, memoisation) without touching the
compiler. Julia's multiple dispatch is explicitly CLOS-derived and shows the model
works in performance-sensitive code; Dylan, Python's `singledispatch` and
Clojure's multimethods are partial copies. No mainstream language copied the
MOP, which suggests its power serves framework authors at the cost of making
every object operation overridable and hard to optimise.

### Scheme's proper tail calls and first-class continuations
Scheme required proper tail calls from the first report, so loops are not
primitives; `call/cc` exposes the continuation as a value, from which
coroutines, generators and backtracking are library code. Delimited
continuations (Racket's `call/prompt`) are the basis of effect handlers in
Koka, Effekt and OCaml 5, so the idea transplanted; full undelimited
`call/cc` did not (OCaml 5, Koka and Effekt all chose delimited control;
Kiselyov's "An argument against call/cc" (2012) documents its interaction
with `dynamic-wind` and resource handling as a source of subtle bugs).

## Weaknesses

### The Lisp curse: expressive power as a social problem
Winestock (2011): "Lisp is so powerful that problems which are technical
issues in other programming languages are social issues in Lisp"; "Making
Scheme object-oriented is a sophomore homework assignment", so every Scheme
had several incompatible object systems and none became standard [9]. The
observable consequences: Common Lisp had no standard package manager until
Quicklisp (2010), no standard FFI (CFFI is a library), no standard threads or
sockets (Bordeaux-threads, usocket), and a frozen 1994 standard; Scheme
fragmented into dozens of implementations with incompatible module systems,
which is exactly what the R6RS fight was about; even Racket, the most
cohesive, has three class systems, several contract idioms and multiple
pattern matchers in the distribution. The lesson is not "macros are bad";
it is that a language with cheap extension needs *stronger* central
curation of the standard library, not weaker, or the ecosystem diverges.

### Parentheses are an adoption barrier: the evidence
(1) The Racket team, after 25 years of teaching with S-expressions, built
Rhombus (1.0, June 2026) whose announcement says Lisp macros' expressiveness
"has been difficult to detangle from Lisp's minimalistic, parenthesis-
oriented notation" and whose goal is to be "as extensible as Racket" with "a
conventional syntax" [10]. (2) Pyret, by HtDP authors, dropped parentheses
for teaching. (3) Dylan (Apple, 1992) started with S-expressions and switched
to infix syntax to gain adoption; it still failed, which shows syntax is
necessary, not sufficient. (4) Clojure's 2025 survey: 48% of new users do
not use structured editing, versus 19% of experienced users, i.e. the syntax
is workable only with editor support that beginners do not have [11]. (5)
Alan Kay on Smalltalk-72: "I never liked this syntax (too many parentheses
and nestings)" [12]. The counter-evidence is Clojure's 70% recommend rate
and Emacs/Racket users who report that parentheses become invisible in
weeks; the honest conclusion is that S-expressions cost adoption at the
margin and that the people most committed to them have, twice, built
non-S-expression front ends.

### Scheme standardisation: the R6RS/R7RS split as a governance failure
R6RS (2007) added a module system, `syntax-case`, Unicode, records, a
condition system and a full numeric tower; it was ratified by 67 votes to
35 of 112 registered voters (65.7% against a 60% threshold), with many
registering specifically to vote no [13]. Several major implementers
declined to implement it; the Steering Committee responded in 2009 by
splitting Scheme into "small" (R5RS-compatible, finalised 2013) and "large"
(a superset, "ongoing" as of 2026, 17 years later) [14, 15]. The result is
that "Scheme" names a family of mutually incompatible languages and no
portable library ecosystem exists outside SRFIs. The lesson: a standard
that a supermajority of implementers will not implement is not a standard;
voting without implementation commitment and without a reference
implementation produced the split.

### Common Lisp is frozen, and its warts are permanent
The 1994 standard has never been revised: no threads, sockets, FFI, Unicode
details, or package manager; `format` directives and `loop` are
mini-languages with their own syntax; two namespaces (Lisp-2) force
`funcall`/`#'`; nil is both false and the empty list; `defmacro` is
unhygienic; the pathname system predates URLs. Implementations extend it
compatibly (SBCL's type inference and compile-time warnings are excellent),
but the "standard" cannot absorb those gains. Industrial users (ITA,
Grammarly) succeeded by owning their stack; new users face a 30-year-old
baseline plus a library ecosystem of individual maintainers.

### Dynamic typing with the compiler in the room
SBCL will warn at compile time when it can prove a type error from
declarations and inference, but the default is no declarations and
run-time checks. Typed Racket exists but is a separate language with
contract-boundary overhead (the "sound gradual typing is dead" result,
Takikawa et al. POPL 2016, measured slowdowns of up to 100x on mixed
programs in Typed Racket, which prompted the Racket team's own research on
shallow and optional types, shipped in Racket 8.x). The evidence from this
family is that retrofitting a sound type system onto a dynamic one is a
research programme with performance costs, consistent with the Elixir
case.

### Tooling and readability costs of macros
Error messages from macro-generated code point at expanded forms; stepping
through a macro-heavy program in a debugger requires expansion tooling
(Racket's macro stepper is the best in class and still research-grade);
every DSL has its own semantics to learn; IDE features such as rename and
go-to-definition require the expander to be run (Racket does this; CL
editors mostly cannot). Racket's own CACM paper ends with "a sketch of
Racket's imperfections" [4], and macro expansion dominates compile time in
large Racket programs (the expander was rewritten in Racket itself in 2018,
as part of the Chez Scheme migration, partly to make it maintainable and
faster).

### Image-based development's hidden state
Redefining a function does not update inlined call sites, changed classes
need instance migration, and an image may work while a fresh load from
source does not. CL projects mitigate with ASDF and CI from clean images;
Racket deliberately chose *not* to be image-based (modules re-run from
source on each `require`), trading interactivity for reproducibility.

## Designer retrospectives

- Steele and Gabriel (HOPL-II, 1993) [1]: in their account Common Lisp
  grew large through accretion of compromises between existing dialects,
  and the committee process rewarded including everything; Scheme's "Lambda the Ultimate" minimalism was the
  reaction.
- Kent Pitman (2001) on the condition system [5]: the design was "a leap of
  faith"; the condition firewall is "probably the most controversial
  semantic component"; handlers cannot *propose* options for confirmation,
  which he calls an open research area; he regrets that MacLisp tied error
  identity to message text ("the identity of an error should not be its
  name").
- Felleisen et al. ("The Racket Manifesto", 2015; CACM 2018) [3, 4]: Racket's
  weaknesses include the cost of macro expansion, the difficulty of good
  error messages across languages, and the absence of a conventional
  syntax; Rhombus (2017-2026) is the fix they built.
- Alex Shinn, "a call for peace" (2011) [16]: the Scheme community was
  divided into R5RS and R6RS camps; the small/large split was explicitly a
  peace treaty rather than a design.
- Matthew Flatt, Rhombus 1.0 announcement (2026) [10]: co-designing syntax
  and macro system "produced a whole that's greater than the sum of the
  parts", which is an acknowledgement that bolting syntax onto an existing
  macro system (Honu, 2012; Dylan) did not.

## Influence

Lisp gave later languages: garbage collection, first-class functions,
conditionals as expressions, the REPL, dynamic typing, symbols/atoms,
macros, multiple dispatch (CLOS to Julia, Dylan), restarts (Dylan,
Smalltalk-style resumable exceptions), proper tail calls and continuations
(to ML, Haskell, Koka, OCaml 5 effects), hygienic macros (Rust's
`macro_rules!` and hygiene, Elixir, Julia, Scala 3), gradual typing with
blame (Typed Racket to TypeScript's and Python's discussions, though they
chose unsoundness), and `#lang`-style language towers (Racket to Rhombus,
to "language workbenches"). Lisp took from: Fortran (arithmetic),
lambda calculus (Church), Simula/Smalltalk (CLOS objects via Flavors and
LOOPS), and ML (Racket's pattern matching and Typed Racket's type theory).

## Lessons for Sisu

- **Adopt**: *hygienic, declarative macros as the default*, with procedural
  macros as a separate, clearly marked tier, and macro expansion exposed to
  the IDE (go-to-definition, rename, step expansion) from the first release.
- **Adopt**: *restart-like recovery for the debugger*, even without the full
  condition system: when a program panics under a debugger, Sisu's runtime
  should allow "edit function, retry frame" in debug builds. This is the
  concrete image-based benefit, and it does not require image persistence.
- **Adopt**: *the Scheme standardisation lesson* as a governance rule: no
  language change is final until it has a reference implementation and the
  major compiler(s) commit to it.
- **Adapt**: *the condition system's separation of "what went wrong" from
  "what to do about it".* In a typed language this can be expressed without
  dynamic handlers: errors carry an enumeration of recovery options
  (`Error::restarts()`), and the caller (not the callee) chooses; libraries
  should return a value that lists restarts rather than unwinding. Full
  dynamic `handler-bind` is probably not worth its interaction with static
  typing, ownership and unwinding; this is an open question below.
- **Adapt**: *`#lang`-style embedded languages* as *typed* internal DSLs:
  the reader-level freedom is what makes Racket languages un-toolable; Sisu
  should allow syntax-extension within expressions (Rust macro style) and
  dedicated block-level DSLs (query, regex, SQL) with the host's type checker
  running on the expansion.
- **Adapt**: *the teaching-language ladder* (HtDP's BSL to ASL): Sisu could
  ship a restricted "teaching profile" (no generics, no unsafe, no
  concurrency) enforced by the compiler, following the evidence that
  limiting the language helps novices and that Pyret-style conventional
  syntax helps more than S-expressions.
- **Avoid**: *S-expression surface syntax*. Two generations of the people
  most invested in it (Pyret, Rhombus) built alternatives; Sisu should keep
  the macro model and not the notation.
- **Avoid**: *leaving the standard library to the ecosystem* when the
  language has macros: the Lisp curse shows that cheap extension plus weak
  curation yields incompatible duplicates. Sisu should own threads, sockets,
  FFI, packaging and formatting in the core distribution.
- **Avoid**: *image-based deployment*. Keep live redefinition for
  development; make the artefact a reproducible build from source.
- **Avoid**: *undelimited continuations*; delimited control (or effect
  handlers) is what every language after 2010 chose.
- **Open questions raised**: (1) Is there a statically typed, zero-cost
  formulation of restarts that is worth the complexity, or do `Result` plus
  an "on error, retry" policy at the call site capture 90%? (2) How much
  language-oriented programming can a typed, AOT language support before
  tooling (errors, IDE, compile time) degrades the way Racket's own authors
  report? (3) Sound gradual typing had a 100x worst case in Typed Racket and
  needed a decade of research; does Sisu need any gradual story at all, or
  only an FFI boundary?

## Sources

1. Guy L. Steele Jr., Richard P. Gabriel, "The Evolution of Lisp", HOPL-II, 1993. https://dl.acm.org/doi/10.1145/155360.155373
2. Revised Reports on Scheme (R3RS-R7RS), introduction. https://standards.scheme.org/
3. Matthias Felleisen et al., "The Racket Manifesto", SNAPL 2015. https://felleisen.org/matthias/manifesto/
4. Matthias Felleisen et al., "A Programmable Programming Language", CACM 61(3), 2018. https://cacm.acm.org/research/a-programmable-programming-language/
5. Kent M. Pitman, "Condition Handling in the Lisp Language Family", in *Advances in Exception Handling Techniques*, 2001. https://www.nhplace.com/kent/Papers/Condition-Handling-2001.html
6. Racket Guide, chapter "Creating Languages" (`#lang`, reader and expander). https://docs.racket-lang.org/guide/languages.html
7. Emmanuel Schanzer, Kathi Fisler, Shriram Krishnamurthi, "Assessing Bootstrap:Algebra Students on Scaffolded and Unscaffolded Word Problems", SIGCSE 2018. https://cs.brown.edu/~sk/Publications/Papers/Published/sfk-bsa-scaff-unscaff-wp/
8. BYU MSc thesis on Bootstrap (control/treatment, three schools). https://scholarsarchive.byu.edu/etd/3519
9. Rudolf Winestock, "The Lisp Curse", 2011. https://www.winestockwebdesign.com/Essays/Lisp_Curse.html
10. "Rhombus 1.0 is now available", Racket blog, 22 June 2026. https://blog.racket-lang.org/2026/06/rhombus-v1.0.html
11. Alex Miller, "State of Clojure 2025 Results", 18 February 2026 (structured editing figures). https://clojure.org/news/2026/02/18/state-of-clojure-2025
12. Alan Kay, "The Early History of Smalltalk", HOPL-II, 1993. https://worrydream.com/EarlyHistoryOfSmalltalk/
13. R6RS ratification results, 2007. https://www.r6rs.org/ratification/
14. Scheme Steering Committee position statement, 2009 (small/large split); InfoQ, "Scheme Split in Two", 2009. https://www.infoq.com/news/2009/08/Scheme-Split-in-Two
15. r7rs.org, status of R7RS small (2013) and large ("ongoing"), retrieved October 2026. https://r7rs.org/
16. Alex Shinn, "a call for peace", scheme-reports, 26 May 2011. https://scheme-reports.simplelists.com/scheme-reports/msg/25747460/
17. Asumu Takikawa et al., "Is Sound Gradual Typing Dead?", POPL 2016.
18. Racket blog, release posts 8.0 (Racket CS default, Feb 2021), 9.0 (Nov 2025), 9.3 (Aug 2026). https://blog.racket-lang.org
19. SBCL news (2.6.9, 26 September 2026). https://sbcl.org/news.html
20. Wikipedia, "Common Lisp" (ANSI 1994 unrevised; industrial users). https://en.wikipedia.org/wiki/Common_Lisp
