# APL, J, K/q and BQN (array programming)

## Snapshot
| | |
|---|---|
| First appeared | APL notation 1962 (Iverson's *A Programming Language*); APL\360 1966; J 1990 (Iverson, Hui); K 1993 (Whitney), kdb+ 1998, q 2003; BQN 2020 (Lochbaum); Uiua 2023. |
| Designed by | Kenneth Iverson (APL, J with Roger Hui); Arthur Whitney (K, q, Shakti); Marshall Lochbaum (BQN); Dyalog Ltd (Dyalog APL). |
| Current version (Oct 2026) | Dyalog APL 20.0 (November 2025, "array notation"); kdb+ 4.1 (February 2024, KX); CBQN 0.12.0 (30 June 2026); GNU APL and NARS2000 maintained; J 9.x (version not verified). |
| Governance | Dyalog and KX are companies; ISO/IEC 13751:2001 (Extended APL) is the only standard and is largely ignored; BQN and J are single-designer open-source projects. |
| Typing | Dynamic; the only "type" is the array (rank, shape, element kind); K/q add tables and dictionaries as first-class values. |
| Memory | Reference-counted or traced arrays with copy-on-write; vectors are contiguous and SIMD-friendly. |
| Compilation | Interpreters that execute whole-array primitives in compiled C (and in BQN's case Singeli-generated SIMD); Dyalog has a bytecode layer; no mainstream JIT. |
| Primary domains | Finance (kdb+/q is the de facto tick database at investment banks and exchanges), actuarial and insurance (APL), data analysis, code golf and education (BQN, Uiua). |
| Notable users | KX customers (vendor claims most tier-one banks and exchanges; not independently verified), SimCorp (its Dimension product is written in Dyalog APL), insurance and actuarial firms (Dyalog), Morgan Stanley (A+, K's predecessor). |

## Stated design goals

Iverson's 1979 Turing lecture, "Notation as a Tool of Thought", argues that a
programming notation should be judged as mathematical notation is: it must
be *executable*, *universal*, and have "ease of expressing constructs
arising in problems", "suggestivity", "ability to subordinate detail",
"economy", and "amenability to formal proofs" [1]. The notation came first
(1962, as a way to describe computer hardware and algorithms on paper); the
implementation followed four years later. K's goal was to put the same power
in ASCII with the smallest possible interpreter for financial time series;
q (2003) added SQL-like readability on top of K for the same market. BQN's
author states he wanted a language that is "clean and modern" *and* an array
language, "much simpler than both" APL and J, with first-class functions,
a context-free grammar and no interpreter state, while having "no intention
of being the last word in programming" [2].

## Strengths

### Rank polymorphism and implicit iteration
Every primitive applies to arrays of any rank: `+` adds scalars, vectors
and matrices; `⌈/` is max-reduce over a vector or over the leading axis of a
matrix; the *rank operator* (`⍤` in Dyalog, `"` in J, `⎉` in BQN) applies a
function at a chosen rank ("leading-axis theory", Hui/Iverson, J 1990).
Loops disappear: `+/` sums, `⍳n` enumerates, `∘.×` is an outer product,
`⊂⍤1` encloses rows. This is the ancestor of NumPy broadcasting, MATLAB's
vectorisation, Julia's dot-broadcasting and pandas; those systems adopted
the *operations* with conventional syntax, which is the adaptation evidence.
The gain is in code size (idiomatic APL/K solutions are often reported as
10-50x shorter than C or Java for data-shaping problems; not measured here)
and in implementation: the
primitives are implemented once in tuned C with SIMD, so the interpreter
overhead is paid per primitive, not per element.

### Notation as a tool of thought, with evidence in the idiom libraries
Because every primitive is one glyph and composition is juxtaposition, APL
programmers memorise *idioms* (`+/⍳`, `{⍵[⍋⍵]}` sort, `≠⊆⊢` split) and read
expressions as formulas. Dyalog's and J's idiom lists, and the "tacit"
(point-free) style in J/BQN (trains: `(+/÷≢)` is mean), are the practical
form of Iverson's claim that notation shapes thinking. The honest evidence
for the claim is mostly expert testimony and code-golf results; the one
quantitative study Wikipedia cites found that typing APL characters did not
hinder high-school students [3]. APL's influence on mathematics education
and on MATLAB's whole-array operations is better documented than any
productivity figure.

### Where it wins: kdb+ and in-memory columnar data
q/kdb+ is a vector language, a columnar in-memory database and a query
language in one process; tables are dictionaries of columns, queries are
array expressions, and the interpreter is a few hundred kilobytes. It has
owned the tick-data niche in finance since the early 2000s (vendor claims:
most of the top investment banks; third-party confirmations are consistent
but not independent). The design lesson is that a tiny, uniform data model
(everything is an array; a table is a flipped dictionary of arrays) plus
whole-array primitives can beat general-purpose databases on latency by
removing abstraction layers, not by adding them.

### Fusion and performance from the array model
Because a program is a chain of whole-array primitives, an implementation
can fuse adjacent operations, pick SIMD kernels by element width (CBQN
generates them with Singeli) and update in place when a reference count is
one. NumPy cannot (each ufunc materialises); Numba, JAX/XLA and Julia's
broadcast fusion reinvented it. The REPL-with-immediate-array-display
workflow (Dyalog 20.0's array notation and inline tracing target it) is the
ancestor of the notebook.



## Weaknesses

### Terseness is a readability and maintenance cost
The same density that makes expressions short makes them hard to read for
anyone but the author: K overloads each ASCII symbol with two or more
meanings chosen by context, which "can make expressions opaque and
difficult to parse for humans" [4]; APL requires a special character set
(historically a barrier of keyboards and fonts, now of input methods);
"write-only code" is the standing joke, which the APL community attributes
to bad practice but which recurs in every discussion of adoption [3]. There
is no evidence of large multi-team APL/K codebases maintained over decades
outside the firms that hire specifically for it. BQN's design is itself an
admission: Lochbaum removed APL's overloading (reduction separated from
compress, stranding replaced by an explicit `‿`) because the ambiguity hurt
reading [2].

### Nested arrays and the "everything is an array" model break down
Smalltalk's uniformity problem recurs: APL2/Dyalog nested arrays (arrays
whose elements are arrays) need enclose/disclose (`⊂`/`⊃`), "simple
scalar" rules, and `⎕ML` migration levels to decide what `↑` means; J and K
chose boxes/lists with different rules; BQN introduced "based arrays"
(elements are not arrays unless enclosed) to remove the paradoxes [2]. The
cost is that strings, records, trees and heterogeneous data, which are most
of ordinary programming, are second-class and awkward; q bolts dictionaries
and tables on as separate types.

### No static types, no modules to speak of, weak error messages
Rank or shape mismatches are run-time errors with terse names (`LENGTH
ERROR`, `RANK ERROR`, `'type` in K); nothing checks shapes before running;
namespaces and packaging arrived late (Dyalog namespaces; BQN has a
dedicated namespace syntax precisely because Dyalog's were awkward [2]);
there is no shared package ecosystem across dialects. The absence of any
successful typed array language (except Futhark, Dex and research systems)
shows that typing ranks and shapes is still an open research problem,
which is why NumPy and PyTorch are also untyped in this dimension.

### Not general purpose, and each dialect is one person's
APL is weak at I/O, strings, concurrency and systems work; K/q are
proprietary (Shakti and KX licences) and tied to one vendor each; J, BQN
and Uiua are single-designer languages with small communities. The
inability to agree even on a character set (APL glyphs vs J's ASCII
digraphs vs K's overloaded ASCII vs BQN's own Unicode set) means the family
has four incompatible notations for the same ideas and no portable code.

## Designer retrospectives

- Iverson, "A Personal View of APL" (IBM Systems Journal, 1991): the
  special character set was a deliberate choice for notation quality and
  acknowledged as an obstacle; J was his own answer (ASCII, functional
  "tacit" style, rank operator) - a founder replacing his notation is the
  strongest retrospective available.
- Lochbaum, "Why BQN?" [2]: lists what he changed from APL and why - first-
  class functions, based arrays, context-free grammar, no interpreter state
  (fixed index origin, no comparison tolerance), separated primitives -
  each item a documented pain point of Dyalog/J.
- Whitney, in interviews: the goal was the smallest possible system; K's
  terseness is a feature for its author, and q exists because customers
  needed readable queries, which is an implicit retrospective.

## Influence

APL gave: whole-array operations and broadcasting (NumPy, MATLAB, S/R,
Julia, Wolfram Language, pandas), reduce/scan as primitives (`+/`, `+\`
became `reduce` and `scan`/`prefix sum` in every data-parallel library and
in GPU programming), the outer product, boolean masks for selection,
`⍳`/`iota` (C++'s `std::iota` is named after it), the rank concept, tacit
programming (point-free style in Haskell was independent, but J's trains
influenced BQN and Uiua), the notebook/REPL exploration style, and the
idea that an interpreter can be fast if the primitives are coarse. A+ and
K at Morgan Stanley shaped financial computing; kdb+ anticipated the columnar,
vectorised execution model later generalised by MonetDB/X100, ClickHouse
and DuckDB. APL took from matrix algebra notation and from Iverson's teaching
needs at Harvard.

## Lessons for Sisu

- **Adopt**: *rank-polymorphic numeric operations and broadcasting in the
  standard library*, with conventional syntax (as Julia's `.+` or NumPy's
  rules): elementwise ops on arrays, `reduce`/`scan`/`outer`, and shape-
  aware slicing. This is the part every successful language borrowed.
- **Adopt**: *operator-level fusion as a compiler goal* for array
  pipelines: a chain of map/filter/reduce over arrays should compile to one
  loop; iterator adapters (Rust) and broadcast fusion (Julia) show how to do
  it with types rather than with an interpreter.
- **Adapt**: *the idiom culture* into typed library functions with names:
  the value of `≠⊆⊢` is that "split on delimiter" is one operation; Sisu
  should have the operation, named, not the glyph.
- **Adapt**: *tables as first-class values* (q's dictionary-of-columns): a
  typed columnar table/dataframe in the standard library or a blessed
  package, since this is the data model that wins in analytics.
- **Avoid**: *symbol-dense notation and context-dependent overloading*:
  four dialects could not converge on one character set, and K's ASCII
  overloading is the documented readability failure; Sisu should allow
  Unicode in identifiers but not build the core language on glyphs.
- **Avoid**: *"everything is an array" as the data model*; nested-array
  paradoxes and the awkwardness of strings, records and trees are the
  family's recurring failure; arrays should be one type among records,
  sums and maps.
- **Open questions raised**: (1) Can shapes/ranks be typed usefully in a
  general-purpose language (dependent-ish shape types as in Dex/Futhark)
  without making ordinary code painful? No mainstream language has done it.
  (2) Is there a middle notation (APL-like density for numeric kernels,
  conventional elsewhere) that survives code review? Julia's
  broadcasting dot is the closest existing answer and is still contested.
  (3) How much of the kdb+ performance story is the language versus the
  columnar engine? The lesson may be about data layout, not notation.

## Sources

1. Kenneth E. Iverson, "Notation as a Tool of Thought", 1979 ACM Turing Award lecture, CACM 23(8), 1980. https://www.jsoftware.com/papers/tot.htm
2. Marshall Lochbaum, "Why BQN?" and "BQN's development history", BQN documentation. https://mlochbaum.github.io/BQN/commentary/why.html
3. Wikipedia, "APL (programming language)" (origins, character set, criticism, implementations). https://en.wikipedia.org/wiki/APL_(programming_language)
4. Wikipedia, "K (programming language)" (Whitney, kdb+/q, ASCII overloading). https://en.wikipedia.org/wiki/K_(programming_language)
5. Dyalog Ltd, "Dyalog Version 20.0" (November 2025; array notation, inline tracing). https://www.dyalog.com/dyalog/dyalog-versions/200.htm
6. KX, kdb+ release history (4.1, 12 February 2024). https://code.kx.com/q/releases/
7. dzaima/CBQN tags (v0.12.0, 30 June 2026). https://github.com/dzaima/CBQN/tags
8. Kenneth E. Iverson, "A Personal View of APL", IBM Systems Journal 30(4), 1991.
9. Adin D. Falkoff, Kenneth E. Iverson, "The Evolution of APL", HOPL-I, 1978.
10. Roger Hui, Kenneth Iverson et al., "APL\?" (introducing J), APL90, 1990; Hui, "Rank and Uniformity", 1995 (leading-axis theory).
