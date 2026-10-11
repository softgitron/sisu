# Prolog and logic/relational programming (miniKanren, Datalog)

## Snapshot
| | |
|---|---|
| First appeared | Prolog 1972 (Marseille); DEC-10 Prolog and the WAM 1977-1983 (Warren); Datalog named 1977-1986 (database theory); miniKanren 2005 (*The Reasoned Schemer*). |
| Designed by | Alain Colmerauer and Philippe Roussel, on Robert Kowalski's procedural reading of Horn clauses. Datalog: database community (Ullman, Gallaire, Minker). miniKanren: Dan Friedman, William Byrd, Oleg Kiselyov. |
| Current version (Oct 2026) | ISO/IEC 13211-1:1995 (corrigenda 2007, 2012, 2017), 13211-2:2000 (modules), 13211-3:2025 (DCGs). SWI-Prolog 10.0.2 stable (2026), 10.1.x development; SICStus 4.x; Scryer, Trealla, GNU Prolog, Ciao, Tau active. Soufflé, Flix, Datomic, CodeQL as Datalog systems. |
| Governance | ISO WG17 for the standard; each implementation independent; no shared package ecosystem beyond SWI's packs. |
| Typing | Untyped (terms); mode/type annotations in Ciao and Mercury; Flix and Datalog engines are statically typed. |
| Memory | Term heap with backtracking (trail) and GC in all major systems. |
| Compilation | WAM-based bytecode (SICStus, SWI, GNU to native via WAM-to-C); tabling (SLG resolution) in XSB, SWI, Ciao; Datalog engines compile to C++ (Soufflé) or run bottom-up semi-naive evaluation. |
| Primary domains | Natural-language processing, constraint solving (CLP(FD)), theorem proving and verification tooling, expert/rule systems, program analysis (Datalog), databases (Datomic), compilers and type checkers as internal engines. |
| Notable users | IBM Watson (parse-tree pattern matching in Prolog), TerminusDB (implemented in Prolog), the first Erlang (a Prolog interpreter, 1986), GitHub CodeQL (Datalog-derived QL), Soufflé-based Doop/Securify analyses, Datomic (Nubank), Rust's Polonius borrow checker prototype (Datalog). |

## Stated design goals

Prolog was built for natural-language question answering: Colmerauer wanted
a language in which a grammar and its interpreter were the same program;
Kowalski supplied the idea that a Horn clause can be read both declaratively
(as a logical implication) and procedurally (as a procedure call). The goal
was "algorithm = logic + control", with the programmer writing only the
logic (Kowalski, 1979). Datalog's goal was the opposite direction: a
restriction of Prolog (no function symbols, no cut, no order dependence) for
which every query *terminates* and can be evaluated bottom-up as database
joins. miniKanren's goal is to embed relational programming into an existing
functional language with a handful of operators (`==`, `fresh`, `conde`),
keeping completeness (interleaving search) that Prolog's depth-first search
lacks [1].

## Strengths

### Unification: pattern matching that runs both ways
Unification solves equations between terms containing variables, binding
whichever side is unknown. `append(X, Y, [1,2,3])` enumerates every split of
the list; the same `append/3` concatenates, splits, checks a prefix or
generates. Pattern matching in ML, Haskell, Rust and Elixir is one-way
unification (pattern against value, no variables on the value side); Erlang
retains two-way matching of *bound* variables (`{ok, X} = f(), {ok, X} =
g()` checks equality) as a Prolog inheritance. The practical gains outside
logic programming: type inference (Hindley-Milner *is* unification over
type terms, and every ML-family compiler contains a unifier), term
rewriting, and bidirectional serialisation. Mainstream languages have no way
to express "this function is a relation, run it in whatever direction the
caller leaves open".

### Backtracking search as a control structure
A Prolog predicate can have several clauses; on failure the engine resumes
at the most recent choice point. Combined with unification this gives
generate-and-test, parsers (DCGs: a grammar *is* a program; ISO 13211-3:2025
finally standardised them), planners and constraint solving in a few lines.
Constraint logic programming (CLP(FD), CLP(R)) layers propagation on the
same machinery and is still competitive for scheduling and puzzles. The
mainstream analogue is generators/iterators plus explicit recursion, which
is the control without the declarative reading; SQL and LINQ are the
declarative reading without the control.

### Relations and the database view: Datalog's real-world win
Datalog restricts Prolog so that programs are sets of relations with a
unique least fixpoint, evaluated bottom-up with semi-naive iteration,
stratified negation and no cut. This is why it is used where Prolog is not:
program analysis (Soufflé compiles Datalog to parallel C++ and powers Doop's
Java points-to analysis and smart-contract analysers; Flix adds Datalog
with lattices for static analysis); security code query (Semmle QL, now
GitHub CodeQL, is an object-oriented Datalog); databases (Datomic's query
language is Datalog over an immutable fact store; LogicBlox for retail
planning); and compilers (Rust's Polonius borrow checker was prototyped as
Datalog rules; chalk, the trait solver, is Prolog-like) [2]. The evidence is
that the declarative *subset* found more industrial use than the full
language, because termination and order-independence are what engineers
need from "declarative".



### Embedding as a sublanguage works
miniKanren is a few hundred lines in Scheme and has been ported to dozens
of hosts (Clojure's `core.logic` is the best known) [1]; Datalog engines
embed into Rust (`crepe`, `ascent`), Java and Racket (`#lang datalog`).
Mainstream languages already embed restricted logic sublanguages without
naming them: SQL, LINQ, regular expressions, pattern matching, type
inference, make's rules. Logic programming is most useful as a *bounded
sublanguage with terminating semantics*, not as the general language.

## Weaknesses

### The cut and the non-logical core
`!` commits to the current clause and discards choice points; it is needed
for performance and for if-then-else, but it destroys the declarative
reading: a predicate with cuts cannot be run "backwards". The standard
critique (Wikipedia summarises it; the TPLP "Fifty Years" survey's SWOT
makes the same point) is that real Prolog programs are "not purely
declarative", clause order affects termination, and `assert`/`retract`,
`var/1`, `is/2` (arithmetic that only works left-to-right) and `findall`
make most programs procedural in disguise [3, 4]. Attempts to fix it
shipped: Mercury (1995) removed cut and added modes, determinism and types
at the cost of being a different language; tabling (SLG) removes most
termination order-dependence for left-recursive predicates; Datalog removes
cut entirely by restricting the language; miniKanren's `conda`/`condu` are
cut-like and are documented as impure.

### Depth-first search is incomplete and the performance model is opaque
Prolog's SLD resolution is depth-first with no occurs check by default
(`X = f(X)` succeeds, creating a cyclic term), so a logically correct
program can loop forever or answer wrongly depending on clause order. The
cost of a query is hard to predict: a small change in goal order turns a
linear program exponential. Wikipedia's summary of criticism: "a high
performance penalty compared with conventional languages, particularly with
its non-deterministic evaluation" [3]. Datalog's bottom-up evaluation and
miniKanren's interleaving search are both reactions.

### Untyped terms and no module story
Terms are untyped; an atom, a string, a number and a compound can all
appear where a list was expected and fail silently via backtracking rather
than an error. The ISO module standard (2000) is not what most
implementations use; SWI, SICStus, YAP and Ciao have incompatible module
systems, and the TPLP survey names portability as the field's main
problem: systems "differ in other respects, which creates portability
problems" [4]. SWI-Prolog's decision to add a native string type (7.x,
2013) was a deliberate ISO-incompatible fix that split the ecosystem
further. Ciao's assertion language and Mercury's types are the fixes;
neither reached mainstream Prolog.

### Small industrial footprint after 50 years
Most Prolog applications are "small by industrial standards" and logic
programming "has not significantly impacted the computer industry overall"
[3]. The niche is durable (SWI-Prolog is actively developed, there are more
than ten live implementations) but the pattern is clear: Prolog's ideas
leave in restricted forms (Datalog, unification in type checkers, pattern
matching, CLP solvers behind APIs), the language itself does not.

## Designer retrospectives

- Colmerauer and Roussel, "The birth of Prolog" (HOPL-II, 1992): the
  language was built for NLP and its control strategy (depth-first,
  left-to-right) was a pragmatic choice for the 1972 hardware, not a
  logical one; they acknowledge the cut as a necessary impurity from the
  start [5].
- Kowalski, "Algorithm = Logic + Control" (CACM 1979): the ideal of writing
  only logic is stated as a goal and acknowledged as unmet; Prolog fixes the
  control component.
- Körner et al., "Fifty Years of Prolog and Beyond" (TPLP 2022) [4]: a
  SWOT by the maintainers of SWI, Ciao, YAP, GNU and others; threats named
  are fragmentation, portability, and the lack of a shared library
  ecosystem; the proposed direction is a common core plus a shared test
  suite.
- The miniKanren authors (Byrd's 2009 thesis) present complete interleaving
  search and the absence of cut as corrections to Prolog, i.e. a
  retrospective by design.

## Influence

Prolog gave: unification-based pattern matching (Erlang, then Elixir;
ML-family one-way matching developed separately, but Erlang's two-way form
is Prolog's);
Erlang's syntax and atoms; CHR, Mercury, Oz, Logtalk, Visual Prolog;
Datalog, which in turn produced Soufflé, CodeQL, Datomic, Flix, DDlog;
Clojure's `core.logic`; constraint solvers; DCGs as parser combinators'
ancestor; type checkers written as logic programs (Haskell's type-class
resolution is explicitly Horn-clause resolution; Rust's chalk). Prolog took
from: resolution theorem proving (Robinson 1965), Planner (Hewitt), and
Q-systems (Colmerauer).

## Lessons for Sisu

- **Adopt**: *Datalog as a first-class embedded query/rule sublanguage*
  (typed, terminating, order-independent) for the cases where Sisu programs
  today write SQL strings or hand-rolled fixpoint loops: program analysis,
  build graphs, configuration rules, in-memory queries. The industrial
  record (CodeQL, Soufflé, Datomic) is for Datalog, not Prolog.
- **Adopt**: *two-way matching of already-bound variables in patterns*
  (Erlang's `^pin`/bound-variable equality), a cheap Prolog idea that
  removes a class of accidental shadowing bugs.
- **Adapt**: *unification as the engine of type inference and of a
  `match`-with-guards design*: Sisu's pattern language should be a proper
  subset of first-order unification (nested, with `|` alternatives and
  bindings), which keeps exhaustiveness checking decidable while covering
  what Prolog users actually use patterns for.
- **Adapt**: *generators and search*: Prolog's backtracking is best offered
  as an explicit library (streams/iterators with `yield`, or a `search`
  effect) rather than an implicit control structure; miniKanren shows the
  whole thing fits in a library given closures and laziness.
- **Avoid**: *an untyped, order-dependent "declarative" layer.* A
  declarative sublanguage in Sisu must guarantee termination and
  order-independence (Datalog's restrictions), or it will be used
  procedurally, as Prolog is, and lose its reasoning benefits.
- **Avoid**: *cut-like escape hatches inside the declarative layer*; put
  commit/priority semantics in the host language instead (Mercury's
  determinism modes are the principled version if ever needed).
- **Open questions raised**: (1) Is a typed Datalog in the standard library
  worth its weight for a general-purpose language, or is it a package? Flix
  bet on the former, Rust (`ascent`) on the latter; neither has broad data
  yet. (2) Can relational semantics (run a function backwards) be given to a
  *typed* subset of ordinary functions, as Mercury's modes do, without a
  second language? (3) Should the type checker itself be written as a logic
  program (as Haskell's and chalk are), for extensibility and provable
  termination, or as a hand-written solver for error-message quality?

## Sources

1. Daniel P. Friedman, William E. Byrd, Oleg Kiselyov, *The Reasoned Schemer*, MIT Press, 2005 (2nd ed. 2018); William Byrd, "Relational Programming in miniKanren", PhD thesis, Indiana University, 2009.
2. Wikipedia, "Datalog" (systems: Soufflé, Doop, QL/Semmle, Datomic, LogicBlox, Flix, DDlog, Nemo; restrictions and termination). https://en.wikipedia.org/wiki/Datalog
3. Wikipedia, "Prolog" (history, ISO parts 1-3, criticism). https://en.wikipedia.org/wiki/Prolog
4. Philipp Körner, Michael Leuschel, João Barbosa, Vítor Santos Costa, Verónica Dahl, Manuel V. Hermenegildo, Jose F. Morales, Jan Wielemaker, Daniel Diaz, Salvador Abreu, Giovanni Ciatto, "Fifty Years of Prolog and Beyond", *Theory and Practice of Logic Programming* 22(6), 2022. https://arxiv.org/abs/2201.10816
5. Alain Colmerauer, Philippe Roussel, "The birth of Prolog", HOPL-II, 1993.
6. Robert Kowalski, "Algorithm = Logic + Control", CACM 22(7), 1979.
7. SWI-Prolog download page (10.0.2 stable), retrieved October 2026. https://www.swi-prolog.org/download/stable
8. Herbert Jordan, Bernhard Scholz, Pavle Subotić, "Soufflé: On Synthesis of Program Analyzers", CAV 2016.
9. Magnus Madsen, Ming-Ho Yee, Ondřej Lhoták, "From Datalog to Flix", PLDI 2016.
10. Niko Matsakis, "An alias-based formulation of the borrow checker" (Polonius, Datalog), 2018. https://smallcultfollowing.com/babysteps/blog/2018/04/27/an-alias-based-formulation-of-the-borrow-checker/
