# Logic, constraint, functional-logic and rewriting languages

## Why this family matters for Sisu

These languages make search, failure, constraints and state-space
exploration part of the language. Sisu has two things the family depends
on: an interpreter that owns every yield point and explores schedules
(`--explore N`, [18-next-steps.md](../../proposal/18-next-steps.md) Phase 6),
and contracts restricted to `pure` expressions that a solver can read
([07-contracts.md](../../proposal/07-contracts.md)). Which ideas do they
make cheap, and which would bring in a second language? Prolog, Datalog and
miniKanren are in [`../languages/prolog.md`](../languages/prolog.md).

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Oz / Mozart | 1991-; Mozart 1999; CTM book 2004 | Mozart 2.0.1 (Sep 2018) is the last release; still taught | Dataflow variables, first-class computation spaces, a kernel language per paradigm |
| Curry | 1995- | KiCS2 3.5.0 (Dec 2025), PAKCS, Curry2Go; academic (Kiel) | Haskell plus free variables, choice and needed narrowing |
| Mercury | 1995- | 22.01.x stable plus snapshots (manual rotd-2026-06-09) | Typed, moded logic programming with checked determinism |
| Verse | 2022-; UEFN since Mar 2023 | V1 stable since UEFN 32.00 (Nov 2024), V2 unstable; to be central to Unreal Engine 6 (early access "late 2027") | Deterministic functional logic: failure instead of booleans, choice, transactional effects |
| Icon | 1977- | 9.5.25a (Sep 2025); Unicon dialect | Goal-directed evaluation: expressions are generators that succeed or fail |
| SNOBOL4 | 1962-1967 | Historical | Patterns as first-class values; success/failure gotos |
| Raku junctions | Perl 6 design; Raku since 2019 | In Rakudo; no imitators | `any`/`all`/`one`/`none` values that autothread through calls |
| MiniZinc | 2007- | 2.9.7 (Apr 2026); annual MiniZinc Challenge | Solver-independent models compiled to FlatZinc |
| clingo (ASP) | Stable models 1988 | clingo 5.8 (2025); Spack's dependency solver | Generate, define, test; answer sets are solutions |
| Z3, Rosette | 2008; 2013 | Active; AWS makes about a billion SMT calls a day | SMT as a back end; an interpreter that emits solver queries |
| Cedar | 2023- | Open source; Amazon Verified Permissions | A policy language designed to be analysable, with machine-checked models |
| Alloy | 1997- | Alloy 6.2.0 (Jan 2025) | Relational logic checked in small scopes by SAT |
| TLA+, PlusCal, Quint | 1999; 2009; 2022- [unverified] | TLA+ Foundation (AWS, Oracle, NVIDIA); Quint at quint.sh | Systems as state machines checked by TLC or Apalache |
| P | 2013- | Used across AWS (S3, EBS, DynamoDB, Aurora, EC2) | Communicating state machines as a program, systematically tested |
| Promela / SPIN | 1980- | SPIN 6.5.x | Explicit-state model checking with partial-order reduction |
| Maude, K | 1992-; 2000s- | Maintained; K by Runtime Verification | Semantics as rewrite rules; executable C, Java, EVM |
| Wolfram Language | 1988- | Active | Evaluation as repeated pattern rewriting |
| Cypher, GQL | 2011; 2024 | ISO/IEC 39075:2024; SQL/PGQ in SQL:2023 | ASCII-art graph patterns |

## The ideas

### Failure and choice as control flow

**What it is.** In Icon an expression produces values or *fails*; `if`,
`while` and `every` consume success and failure, and generators resume on
failure: `every write(find("a", "banana"))` prints 2, 4, 6 [15]. Verse does
this in a typed language: failable expressions (indexing, comparisons,
calls to `<decides>` functions, written with brackets) may appear only in
*failure contexts* such as `if` conditions and `for` filters, which are
"speculative execution" rolled back on failure, so every function called in
one must be `<transacts>` [2]:

```verse
GetItem(Items:[]item, Index:int)<decides><transacts>:item = Items[Index]
if (Item := GetItem[Inventory, Slot], Item.Durability > 0):
    Equip(Item)
```

The Verse calculus has no booleans ("failure ... plays the role of
falsity", "following the lead of Icon"); choice `e1 | e2` yields the values
of `e1` then `e2`; `for(x := (2|3|5); x > 2) do (x + 1)` reduces to ⟨4, 6⟩;
`one{e}` and `all{e}` capture a search as a value. "Deterministic" means a
defined order of results, which makes capture possible and "pretty much
rules out laziness"; the claimed gain is "an elegant economy of concepts"
[1]. Verse has been UEFN's scripting language since March 2023; V1 was
stabilised and an unstable V2 added in UEFN 32.00 (November 2024) [4]; the
designers were still presenting new parts in 2026 [7]; Unreal Engine 6
(early access late 2027) moves gameplay code from C++ and Blueprints to
Verse [6]. Epic's 2022 promise of an open-source compiler is unfulfilled as
far as I could find [unverified].

**Where it failed.** Tratt built Icon-style evaluation into Converge [16]:
failure as a procedure's default is "useful for a small subset of procedures
(generators), but dangerous for the majority"; a reader cannot tell which
calls are generators; backtracking is "too weak to be useful for complex
operations on anything other than strings"; failure-frame operations were
25-30% of executed opcodes. He would keep generators and failure in `if`
and drop the rest. Verse narrowed propagation too: UEFN 28.10 (January
2024) deprecated failure escaping a `set` right-hand side or a map-literal
key [3]. Rollback is the costly part: Epic built AutoRTFM, a clang-based C++
compiler that registers an undo handler for every store so that Fortnite's
C++ servers can join Verse transactions, single-threaded only [5].

**Transplants.** Generators spread everywhere (PEP 255 cites Icon, "a
novel language where every expression _is a generator_" [17]). Failure
survives as typed conditions of fallible bindings (Swift's `if let a = f(),
let b = g(a), cond`; Rust's let chains, 1.88, June 2025 [39]); choice as a
library is Haskell's list monad and LogicT [38].
Verse alone transplants choice itself; I could not confirm whether the UEFN
dialect exposes general `|` choice or only failure and `for`.

### Logic variables, dataflow and narrowing

**What it is.** An Oz dataflow variable is single-assignment; a thread that
reads it unbound suspends until it is bound:

```oz
declare X Y in
thread Y = X + 1 end   % suspends until X is bound
X = 41                 % the thread resumes and binds Y to 42
```

Records (`person(name:"Ada" age:36)`) are the universal data structure, and
*computation spaces* make search programmable: `NewSpace`, `Choose`, `Ask`,
`Commit`, `Clone` and `Merge` let a parent run a problem in a child space and
choose which alternative to commit, so search engines are ordinary programs
separate from the problem [8, §5.1.6]. Curry adds free variables and
narrowing to Haskell [10, 12], so functions run backwards by needed
narrowing [11]: `last xs | ys ++ [x] =:= xs = x  where x, ys free`.

**Evidence.** The Oz history (HOPL 2020) says the multiparadigm goal "was
largely achieved", and UCL "used Mozart to teach programming to all
students of engineering for 15 years" with the CTM book [8, 9]. It also
says "we failed in creating a self-sustaining Oz community": the syntax was
"widely different from existing syntaxes", and, "much more important", the
move from funded research to volunteer open source failed (64-bit came in
2012, multicore in 2014). A company built on an Oz sports scheduler found
that Mozart's solver "did not scale" and replaced it with local search over
max-SAT [8]. For Curry, the Verse paper notes two equalities and set
functions needing "careful explanation about sharing", which it blames on
laziness [1]; I found no industrial Curry user. Dataflow variables live on
as futures; Verse is the one industrial attempt at logic variables in a
general-purpose language [1].

### Determinism as a checked discipline

Mercury declares and checks a determinism for every mode of a predicate:
`det`, `semidet`, `multi`, `nondet`, `failure`, `erroneous`, and the
committed-choice `cc_multi` and `cc_nondet` [13]:

```mercury
:- mode append(in, in, out) is det.       % exactly one solution
:- mode append(out, out, in) is multi.    % one or more
```

It replaces Prolog's cut, and "determinism checking catches many program
errors at compile time" [13]; Prince, the HTML-to-PDF formatter, is written
in it (and Rust) [14]. For Sisu the categories are already types (`det` is
`T`, `semidet` `T?`, `nondet` `Iter[T]`, `erroneous` `Never`), and the other
idea, several modes per predicate, needs a second semantics per function.

### Implicit lifting: junctions

A Raku junction is "an unordered composite value"; passing one where a
single value is expected repeats the call per member, so with
`sub double(Int $x) { $x * 2 }`, `double(1|2)` returns `any(2, 4)` [19].
The cost is in the types: junctions "fall outside the Any hierarchy", so
every parameter typed `Any` or narrower autothreads and a function wanting
the junction itself must accept `Mu`; "introspection of junctions is not
supported", and autothreading over side effects "is usually an error" [19].
Conway's Quantum::Superpositions (Perl 5, 2000) is the ancestor; no other
language adopted the lifting.

### Patterns as values and rewriting as evaluation

SNOBOL4 patterns are values built by concatenation and alternation, and a
match's success or failure picks the next statement [18]:

```snobol
        DIGITS = SPAN('0123456789') . NUM
        LINE DIGITS                          :S(FOUND)F(NONE)
```

LPeg "follows the SNOBOL tradition, where patterns are first-class values",
and for grammars "many LPEG users prefer this SNOBOL-like syntax" to regex
strings [18]. The Wolfram Language defines rules by pattern
(`f[x_] := x^2`), applies them with `/.` and `//.`, and evaluates "until it
no longer changes", bounded by `$IterationLimit` [36]. Maude implements
rewriting logic (Meseguer 1992), with `search` over reachable states [34].
K defines a language as rewrite rules and derives an interpreter, a
symbolic executor and a verifier from one definition; its C semantics
powers `kcc`, on which undefined programs "get stuck", and there are
complete definitions of Java (POPL 2015) and the EVM (KEVM, CSF 2018, run
against the Ethereum VMTests) [35]. Cypher's ASCII-art patterns became GQL
(ISO/IEC 39075, 12 April 2024) and SQL/PGQ (ISO/IEC 9075-16:2023) [37]:

```cypher
MATCH (a:Person)-[:KNOWS]->(b:Person)-[:KNOWS]->(c:Person)
WHERE NOT (a)-[:KNOWS]->(c) AND a <> c
RETURN c.name
```

Patterns as values compose, rewriting makes semantics executable (K's
definitions double as test oracles), and graph patterns are standard now;
but SNOBOL's gotos did not survive into Icon [15], Wolfram's rules make
evaluation depend on every rule in scope, and no language has them in `match`.

### Separating the model from the solver

MiniZinc compiles a declarative model to FlatZinc, which many solvers read
[20]:

```minizinc
var 1..9: x;  var 1..9: y;
constraint x + y = 10 /\ x < y;
solve maximize x * y;
```

Answer set programming (stable models, Gelfond and Lifschitz 1988) is
generate-and-test with a solver behind it; in clingo, graph colouring is
`1 { assign(N, C) : color(C) } 1 :- node(N).` plus the constraint
`:- edge(N, M), assign(N, C), assign(M, C).` [21]. SMT solvers (Z3, 2008)
decide formulas over integers, bit-vectors, arrays and datatypes [22];
Rosette runs a host language over symbolic values on "a lightweight
symbolic virtual machine", with the queries `verify`, `solve`, `synthesize`
and `debug` [23].

**Evidence.** The split let solver technology change under unchanged
models: in the 2025 and 2026 MiniZinc Challenges OR-Tools CP-SAT, a
SAT-based hybrid, won gold in the Fixed, Free and Parallel categories [20].
Spack's concretizer became "a complete, optimizing solver, written in
around 800 lines of declarative ASP", most solves under 10 seconds over
nearly 64,000 packages [21]. AWS's Zelkova went "from a thousand SMT
invocations daily to an unprecedented billion SMT calls" in five years
[22]. Cedar is "designed to be ergonomic, fast, safe, and analyzable"; its
design supports precise analysis, such as proving that refactored policies
keep the same permissions; its models and proofs are in Dafny and Lean, and
proofs found 4 bugs and differential and property-based testing 21 more
[24]. All of it runs behind APIs or in restricted languages. Python
libraries (z3py, OR-Tools) get natural syntax because `__eq__` may return a
constraint, but `and`/`or` cannot be overloaded, so z3py writes `And(a, b)`;
Sisu has the same wall, as `==` comes from `Eq` and `and`/`or` are
`bool`-only (D-025).

### Executable specifications and model checking

**TLA+ and its relatives.** The AWS paper (Newcombe et al., 2014) applied
TLA+ to 10 large systems in 7 teams, and engineers "from entry level to
Principal" got "useful results in 2 to 3 weeks". An S3 network algorithm
(804 lines of PlusCal) "found 2 bugs"; DynamoDB replication (939 lines of
TLA+) "found 3 bugs, some requiring traces of 35 steps"; EBS volume
management (102 lines) found 3 [26]. The same paper omits snippets "because
their unfamiliar syntax can be off-putting" and knows of no tool that
checks code against a specification at AWS's scale. MongoDB found trace
checking against an abstract TLA+ specification "impractical", while test
generation from a specification "was highly successful" for Realm Sync
[28]. The TLA+ Foundation (Linux Foundation, members AWS, Oracle and
NVIDIA) now stewards the tools [29], and Quint gives TLA's logic "an
alternative surface syntax", typed and checked through Apalache [30]. Alloy
checks relational models in small scopes (Alloy 6 added mutable state and
temporal logic) [25]; Zave's Alloy model found serious bugs in Chord, which
led AWS to try formal methods, but AWS judged Alloy "not expressive enough
for many of our use cases" [26]. SPIN (Bell Labs from 1980; ACM Software
System Award 2002) checks Promela models with partial-order reduction [33].

**P: the turn to programming languages.** P is "more of a programming
language than a mathematical modelling language": state machines whose
checker explores interleavings and returns a reproducible trace [31]. It
began with Windows 8's USB 3.0 drivers, has been developed at AWS since
2019, backed the claim that S3's strong consistency (December 2020) is
"provably correct", not "probably correct", found 3 bugs in the FreeRTOS
over-the-air protocol, and is used for S3, EBS, DynamoDB, MemoryDB, Aurora,
EC2 and IoT; PObserve checks that production systems conform to their P
specifications [31]. Brooker and Desai's 2025 CACM account sets TLA+ and P beside
property-based testing, fault injection and simulation [27]. S3's
ShardStore team wrote "executable reference models as specifications" in
Rust, which "prevented 16 issues from reaching production" [32]. From
2014 to 2025 the checked artefact moved from a separate specification
language to models written as programs and checked by exploring schedules
and inputs, which closes the 2014 gap and is what Sisu's interpreter can do.

## Transferable ideas for Sisu

The answer to the central question: model checking and small-scope search
pay off, because the interpreter and capability fakes already route all
nondeterminism through values a test controls; constraint solving belongs in
libraries; choice as an expression form does not, apart from the typed
residue of failure contexts; graph patterns and relations stay libraries.

1. **The schedule explorer as a P-style model checker for ordinary code.**
   Make the scheduler, `Clock`, `Rand` and `Net.fake` *choice points* that
   the explorer can enumerate; add explicit `choose`, invariants checked
   after every step, and an exhaustive bounded mode (depth limit, state
   hashing, partial-order reduction as in SPIN) beside random `--explore N`.
   A failure is a list of choices that replays (names illustrative):
   ```sisu
   test "a lease is never held twice"(ex: Explorer):  # a std.test fixture
       let net = ex.net(drop=True, reorder=True)      # every delivery is a choice
       let server = LeaseServer(net.endpoint("s"), ex.clock())
       ex.always(() -> len(server.holders()) <= 1)    # checked after every step
       task.scope do s:
           for name in ["a", "b"]:
               s.spawn(() -> client(net.endpoint(name), retries=ex.choose(range(3))))
   ```
   *Cost*: medium, in the interpreter (replay, state hashing, dynamic
   partial-order reduction). *Conflicts*: none; it extends Phase 6, and each
   capability fake must expose its choices. *Confidence*: high (P, ShardStore,
   SPIN; the model is the code, so there is no gap to close).
2. **Small-scope exhaustive inputs for `check`.** Alloy's hypothesis applied
   to property tests: `check(exhaustive[list[Op]](max_len=4)) do ops:` runs
   every history of up to four operations against a reference model, which
   is ShardStore's method; refinement types (`type Key = int where 0 <= self
   < 3`) bound the domains. *Cost*: low (a derived `Exhaustive` beside
   `Arbitrary`). *Conflicts*: none. *Confidence*: medium-high (Alloy, the
   Chord bugs, Andoni et al. and SmallCheck on small scopes [25, 40]).
3. **Failure contexts without rollback: chained fallible conditions.** Keep
   what Tratt says to keep (failure in `if`) in the form Swift and Rust ship:
   a condition is a comma-separated sequence of `let` bindings and boolean
   tests, left to right, every binding in scope in the branch; `while` too.
   ```sisu
   if let Some(user) = users.get(id), let Some(team) = user.team, team.active:
       notify(team)
   ```
   Nothing is rolled back, unlike Verse. *Cost*: low. *Conflicts*: a comma,
   not `and`, because `and` is `bool`-only (D-025). *Confidence*: high.
4. **Solver-aided queries over `pure` code on the same interpreter.**
   Rosette's architecture: evaluate `pure` functions over symbolic values
   in the SIR interpreter and give path conditions to an SMT solver (a
   trapping `int` is a bit-vector plus an assertion). `verify` finds inputs
   that break `ensures` or trap, `solve` finds inputs reaching a branch to
   seed `check`; counterexamples go to `tests/corpus/`.
   ```sisu
   pure def midpoint(lo: int, hi: int) -> int:
       requires 0 <= lo <= hi
       ensures lo <= result <= hi
       return (lo + hi) // 2  # verify: lo = hi = 4611686018427387904 overflows
   ```
   *Cost*: high; it is the post-1.0 `std.x.verify` of Q-16, with
   counterexamples as a first deliverable before proofs. *Conflicts*: none;
   Cedar shows that a language restricted for analysis, as `pure` contracts
   are, can be analysed precisely in production [24]. *Confidence*: medium.
5. **Search combinators over generators.** Choice is `Iter[T]`, `one{}`
   `first`, `all{}` `collect`; `std.search` adds fair interleaving, iterative
   deepening and bounded search (LogicT [38]). *Cost*: low. *Conflicts*:
   none. *Confidence*: medium (useful, not transformative).
6. **One executable semantics, three engines** (K): keep the SIR
   interpreter definitional so the runner, explorer and symbolic evaluator
   share it and it can anchor Q-38's conformance suite. *Cost*: a design
   rule. *Conflicts*: interpreter optimisation. *Confidence*: medium-low.
7. **Captured expressions for constraint and query libraries.** Turn the
   expression capture `std.test.Expectation` already has into a parameter
   type, as C# expression trees do for LINQ [41], so that
   `m.require(x + y == 10 and x < y)` builds a constraint. *Cost*: medium.
   *Conflicts*: an argument that looks evaluated but is not is close to the
   macro line of D-018; methods (`x.le(y)`) need nothing. *Confidence*: low.

## Dead ends

- **Goal-directed evaluation by default** (Icon): "dangerous for the
  majority" of procedures [16]; `T?`, `?`, `??` and `let else` cover the use.
- **Transactional rollback in conditions** (Verse): effects in every
  signature and an undo log in the runtime [2, 5]; Q-17 rejected effects.
- **Logic variables, narrowing, several modes per function** (Oz, Curry,
  Mercury): subtle equality, laziness and sharing semantics [1], and Oz's
  own verdict [8]; Datalog ([prolog.md](../languages/prolog.md)) is the
  bounded form with users.
- **Junctions** (Raku): a type outside the hierarchy, side-effect-free
  callees, no introspection [19].
- **Graph patterns or relations in `match` or the types** (Cypher, Alloy):
  a non-goal ([14-non-goals.md](../../proposal/14-non-goals.md)); a GQL
  query is a comptime-checked string with the existing mechanism.
- **A separate specification language** (TLA+, Alloy, Quint): AWS moved
  towards models in a programming language [26, 27, 32].
- **Global rewrite rules as evaluation** (Wolfram, Maude): any rule changes
  evaluation everywhere, against local reasoning.
- **A solver inside the language** (Oz): it did not scale for the company
  built on it [8]; solvers improve faster than languages [20].

## Sources

The web-search limit was reached during this research: unchecked claims are flagged in the text, and entries without a URL are cited from memory.

1. L. Augustsson, J. Breitner, K. Claessen, R. Jhala, S. Peyton Jones, O. Shivers, G. L. Steele Jr., T. Sweeney, "The Verse Calculus: A Core Calculus for Deterministic Functional Logic Programming", PACMPL 7 (ICFP), art. 203, Aug 2023 (extended version). https://doi.org/10.1145/3607845 ; https://www.cs.tufts.edu/~nr/cs257/archive/simon-peyton-jones/verse-icfp23.pdf
2. Epic Games, "Failure in Verse", UEFN documentation (2023-2026). https://dev.epicgames.com/documentation/en-us/uefn/failure-in-verse
3. Epic Games forums, "Verse Deprecation Messaging" (23 Jan 2024). https://forums.unrealengine.com/t/verse-deprecation-messaging/1636569
4. Epic Games, "32.00 Fortnite Ecosystem Updates and Release Notes" (Nov 2024); "Verse Language Version Updates and Deprecations". https://dev.epicgames.com/documentation/en-us/fortnite/32-00-fortnite-ecosystem-updates-and-release-notes-in-creative-and-unreal-editor-for-fortnite ; https://dev.epicgames.com/documentation/en-us/uefn/verse-language-version-updates-and-deprecations-in-verse
5. P. Pizlo, "Bringing Verse Transactional Memory Semantics to C++", Unreal Engine tech blog (15 Mar 2024). https://www.unrealengine.com/tech-blog/bringing-verse-transactional-memory-semantics-to-c
6. Game Developer, "Unreal Engine 6 will merge UE5 and UEFN into a single unified engine" (17 Jun 2026). https://www.gamedeveloper.com/programming/unreal-engine-6-will-merge-ue5-and-uefn-into-a-single-unified-engine-
7. S. Peyton Jones, "A Taste of Verse", JuliaCon 2026 (abstract). https://pretalx.com/juliacon-2026/talk/8M3UVS/
8. P. Van Roy, S. Haridi, C. Schulte, G. Smolka, "A History of the Oz Multiparadigm Language", PACMPL 4 (HOPL), art. 83, Jun 2020. https://www.ps.uni-saarland.de/Publications/documents/vanRoyHaridiSchulteSmolka2020.pdf
9. P. Van Roy, S. Haridi, *Concepts, Techniques, and Models of Computer Programming*, MIT Press, 2004; Mozart 2.0.1 (Sep 2018). https://mozart2.org/
10. M. Hanus, H. Kuchen, J. J. Moreno-Navarro, "Curry: A Truly Functional Logic Language", ILPS'95 workshop, 1995.
11. S. Antoy, R. Echahed, M. Hanus, "A Needed Narrowing Strategy", JACM 47(4), 2000 (POPL 1994).
12. Curry implementations overview; KiCS2 3.5.0 manual (15 Dec 2025). https://curry-lang.org/implementations/overview/ ; https://curry-lang.org/kics2/download/kics2-3.5.0-manual.pdf
13. Z. Somogyi, F. Henderson, T. Conway, "The Execution Algorithm of Mercury", J. Logic Programming 29(1-3), 1996; "About Mercury" and reference manual rotd-2026-06-09. https://mercurylang.org/about.html
14. Wikipedia, "Prince (software)" (2026). https://en.wikipedia.org/wiki/Prince_(software)
15. R. E. Griswold, M. T. Griswold, "History of the Icon Programming Language", HOPL-II, 1993; Wikipedia, "Icon (programming language)" (9.5.25a, Sep 2025). https://en.wikipedia.org/wiki/Icon_(programming_language)
16. L. Tratt, "Experiences with an Icon-like Expression Evaluation System", DLS 2010. https://tratt.net/laurie/research/pubs/html/tratt__experiences_with_an_icon_like_expression_evaluation_system/
17. PEP 255, "Simple Generators" (2001). https://peps.python.org/pep-0255/
18. R. E. Griswold, J. F. Poage, I. P. Polonsky, *The SNOBOL4 Programming Language*, 2nd ed., 1971; R. Ierusalimschy, "A Text Pattern-Matching Tool based on Parsing Expression Grammars", SP&E 39, 2009. https://www.inf.puc-rio.br/~roberto/docs/peg.pdf
19. Raku documentation, "class Junction" (2026). https://docs.raku.org/type/Junction
20. N. Nethercote et al., "MiniZinc: Towards a Standard CP Modelling Language", CP 2007; MiniZinc 2.9.7 change log (30 Apr 2026); MiniZinc Challenge 2025 and 2026 results. https://docs.minizinc.dev/en/2.9.7/changelog.html ; https://www.minizinc.org/challenge/2025/results ; https://minizinc.org/challenge/2026/results
21. M. Gelfond, V. Lifschitz, "The Stable Model Semantics for Logic Programming", ICLP/SLP 1988; T. Gamblin, M. Culpo, G. Becker, S. Shudler, "Using Answer Set Programming for HPC Dependency Solving", SC22, 2022. https://arxiv.org/abs/2210.08404
22. L. de Moura, N. Bjørner, "Z3: An Efficient SMT Solver", TACAS 2008; N. Rungta, "A Billion SMT Queries a Day", CAV 2022. https://www.amazon.science/publications/a-billion-smt-queries-a-day
23. E. Torlak, R. Bodik, "Growing Solver-Aided Languages with Rosette", Onward! 2013; "A Lightweight Symbolic Virtual Machine for Solver-Aided Host Languages", PLDI 2014; *The Rosette Guide*. https://docs.racket-lang.org/rosette-guide/ch_essentials.html
24. J. W. Cutler, C. Disselkoen et al., "Cedar: A New Language for Expressive, Fast, Safe, and Analyzable Authorization", OOPSLA 2024, https://arxiv.org/abs/2403.04651 ; C. Disselkoen et al., "How We Built Cedar: A Verification-Guided Approach", FSE Companion 2024, https://arxiv.org/abs/2407.01688
25. Alloy Tools (6.2.0, 9 Jan 2025), https://alloytools.org/ ; D. Jackson, J. Wing, "Lightweight Formal Methods", IEEE Computer 29(4), 1996; D. Jackson, *Software Abstractions*, MIT Press, 2006.
26. C. Newcombe, T. Rath, F. Zhang, B. Munteanu, M. Brooker, M. Deardeuff, "Use of Formal Methods at Amazon Web Services" (29 Sep 2014; CACM 58(4), 2015). https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf
27. M. Brooker, A. Desai, "Systems Correctness Practices at Amazon Web Services", CACM 68(6), Jun 2025 (abstract only; full text not retrievable). https://cacm.acm.org/practice/systems-correctness-practices-at-amazon-web-services
28. A. J. J. Davis, M. Hirschhorn, J. Schvimer, "eXtreme Modelling in Practice", PVLDB 13(9), 2020. https://arxiv.org/abs/2006.00915
29. TLA+ Foundation (members AWS, Oracle, NVIDIA), retrieved Oct 2026. https://foundation.tlapl.us/
30. Quint site and README (now under the quint-co organisation), retrieved Oct 2026. https://quint.sh/ ; https://github.com/informalsystems/quint
31. A. Desai, V. Gupta, E. Jackson, S. Qadeer, S. Rajamani, D. Zufferey, "P: Safe Asynchronous Event-Driven Programming", PLDI 2013; P repository and case studies, retrieved Oct 2026. https://github.com/p-org/P ; https://p-org.github.io/P/casestudies/
32. J. Bornholt et al., "Using Lightweight Formal Methods to Validate a Key-Value Storage Node in Amazon S3", SOSP 2021. https://www.amazon.science/publications/using-lightweight-formal-methods-to-validate-a-key-value-storage-node-in-amazon-s3
33. Spin, "What is Spin" (6.5.x), retrieved Oct 2026. https://spinroot.com/spin/whatispin.html
34. J. Meseguer, "Conditional Rewriting Logic as a Unified Model of Concurrency", TCS 96(1), 1992; M. Clavel et al., *All About Maude*, LNCS 4350, 2007.
35. K Framework, https://kframework.org/ ; C. Ellison, G. Roşu, POPL 2012, and C. Hathhorn, C. Ellison, G. Roşu, "Defining the Undefinedness of C", PLDI 2015, https://github.com/kframework/c-semantics ; D. Bogdănaş, G. Roşu, "K-Java", POPL 2015; E. Hildenbrandt et al., "KEVM", CSF 2018, https://github.com/runtimeverification/evm-semantics
36. Wolfram Language documentation, "Patterns" and "Evaluation". https://reference.wolfram.com/language/tutorial/Patterns.html ; https://reference.wolfram.com/language/tutorial/Evaluation.html
37. ISO/IEC 39075:2024 (GQL, 12 Apr 2024); ISO/IEC 9075-16:2023 (SQL/PGQ); Wikipedia, "Graph Query Language". https://en.wikipedia.org/wiki/Graph_Query_Language
38. O. Kiselyov, C. Shan, D. Friedman, A. Sabry, "Backtracking, Interleaving, and Terminating Monad Transformers", ICFP 2005; J. McCarthy, "A Basis for a Mathematical Theory of Computation", 1963.
39. "Announcing Rust 1.88.0" (let chains, 26 Jun 2025). https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/
40. A. Andoni, D. Daniliuc, S. Khurshid, D. Marinov, "Evaluating the Small Scope Hypothesis", MIT, 2002; C. Runciman, M. Naylor, F. Lindblad, "SmallCheck and Lazy SmallCheck", Haskell Symposium 2008.
41. Microsoft, "Expression trees" (C# 3.0 and LINQ, 2007), C# documentation. https://learn.microsoft.com/en-us/dotnet/csharp/advanced-topics/expression-trees/
