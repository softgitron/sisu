# Natural-language-like and literate programming

## Why this family matters for Sisu

Sisu already reads partly as English (`and`, `not`, `in`, `requires`) and
its doc examples are tests. This family shows how far "reads like English"
helps before a language becomes easy to read and hard to write, which
literate practices survived, and what the 2023-2026 claim that English is
now the programming language does to the case for a precise one.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Inform 7 | 2006- (Inform 1993) | Open source (Artistic 2.0) since April 2022; 10.1.2 (Aug 2022); 10.2.0 beta (June 2026) | English sentences declare a world, rules and relations |
| AppleScript | 1993- | Ships with macOS; English-only since 1.3 (1998) | English-like scripting of applications via Apple Events |
| COBOL | 1959- | In production | English verbs over business records |
| SQL | 1974 (SEQUEL)- | Universal; pipe syntax in GoogleSQL (2024) and Spark | Declarative queries in English keyword phrases |
| Rebol, Red | 1997-, 2011- | Rebol 3 community (Apache 2.0, 2012); Red 0.6.6 beta (Mar 2025) | Dialects: small DSLs interpreted from blocks |
| Tcl | 1988- | Tcl 9.0 (Sept 2024) | Everything is a string; commands build control flow |
| Wolfram Language | 1988- | 15.0 (June 2026), 15.0.1 (July 2026) | Everything is an expression; rewriting; notebooks |
| Raku grammars | 2000-2015 design; renamed 2019 | Rakudo active, niche | Grammars as classes |
| WEB, CWEB, inweb; Jupyter, Org Babel, nbdev | 1980s-; inweb 2004-; notebooks 2010s- | inweb carries Inform's 300,000 lines; Jupyter widespread, the rest niche | The document is the program: essays (tangle, weave) or notebooks |
| Doctest, Go examples, Unison docs | 2000s-, 2015-, 2020s- | Mainstream | Documentation that runs |
| "English is the language" | 2023- | Vibe coding; spec-driven kits | Prompts and specifications as programs |

## The ideas

### English-like syntax: easy to read, hard to write

**What it is.** AppleScript (conceived 1989) made scripts read as English and
stored them in "a language-independent internal representation" shown
through dialects [1]:

```
the first character of every word whose style is bold          -- English
le premier caractère de tous les mots dont style est gras      -- French
{ words | style == bold }.character[1]                         -- "professional", never shipped
```

COBOL's 1959 committee chose to "make maximal use of English" and be "easy
to use, even at the expense of power" (`ADD TAX TO PRICE GIVING TOTAL.`) [5];
HyperTalk is in [`spreadsheets-and-end-user.md`](spreadsheets-and-end-user.md).
**What it buys:** reading by people who do not write. AppleScript "is now
widely used" (2006) [1]; COBOL had an estimated 200 billion lines in 1997 and
still ran most card transactions in 2020 [5], which domain fit and migration
cost explain better than English does [assessment].

**Where it failed: the read-only language.** Cook's HOPL retrospective is the
strongest designer evidence here: "The experiment in designing a language
that resembled natural languages (English and Japanese) was not successful
... AppleScript only appears to be a natural language. In fact is an
artificial language, like any other programming language. ... It is easy to
read AppleScript, but quite hard to write it." Users "prefer a more
conventional programming language structure"; AppleScript "should have
adopted the Professional Dialect"; application developers could not "create
terminologies that work well in the naturalistic grammar"; obscure error
messages were "one of the biggest impediments"; and "readability was no
substitute for an effect[ive] security mechanism. Most people just run
scripts" [1]. Dialects were dropped in AppleScript 1.3 (October 1998) [1].
Gruber: the syntax "is optimized for English-likeness", so near-identical
phrases differ (`path of fonts folder of user domain` versus `path to fonts
folder from user domain`) [2]. COBOL has "over 300 reserved words" [5];
Dijkstra's 1975 polemic: "The use of COBOL cripples the mind" [4].
**Transplant:** the working middle is conventional structure with a few
English words (Python's `and`, `or`, `not`, `in`), Sisu's present choice.

### Natural language that works because the domain is prose: Inform 7

**What it is.** Inform 7 (Graham Nelson, 2006; open source since April 2022,
with 2,500 compiler tests [9]) reads as English but parses a closed
vocabulary that the source itself declares:

```
The Cellar is a room. "Cold stone, and a brass lamp on a hook."
The brass lamp is a device in the Cellar.
Instead of taking the lamp when the lamp is switched on:
	say "It is far too hot to hold."
Loving relates various people to one person.
The verb to love means the loving relation.
Test me with "examine lamp / switch lamp on / take lamp".
```

*Rules with preconditions* sit in rulebooks (`Before`, `Instead`, `Check`,
`Carry out`, `After`, `Report`); clashes "are resolved by sorting the
rulebooks in order of specificity" through four Laws (aspects constrained,
when/while clauses, how narrowly the action is described, scenes), source
order breaking ties [12]. *Phrases* are its functions, named by sentence
patterns with typed slots (`To decide which number is double (N - a
number): ...`). *Relations* have declared cardinality (each person loves one
other at a time), and `(called the owner)` names the inverse so that `now
Flaubert is the owner of Loulou` reads and checks [12]. *Test
scripts* (`Test balloon with "get balloon / blow balloon / drop balloon"`,
with stipulations such as "holding the balloon") exist because "the only way
to thoroughly test a work of IF is to run a complete solution through it";
the Skein keeps every play-through as a replayable tree [12].

**What it buys.** Nelson (2018): "The chief originality of Inform 7 is that
it is based entirely on natural language", giving familiarity, "a low
barrier to entry, so that novices, even schoolkids, can get at least
something to work quickly", and conciseness; "about 2000 downloads a month"
(Mac App Store) and "between 1000 and 2000 finished works" [10].

**Where it does not generalise.** Nelson's costs: "a high barrier to entry,
as a result", "unfamiliarity", "verbosity, in that some arithmetical or
data-heavy computations probably are better expressed in formulae"; "the
benefits are double-edged. But they are real". The ambiguity gap "is one of
degree, and is not an absolute"; Inform is "not primarily intended to solve
conventional tasks" [10], and generalising its input is "still immensely challenging" [11]. My
reading: it works because its subject is a world in prose, the author
declares the lexicon so the parser never meets open English, and arithmetic
is rare; general software inverts all three.
**Transplant:** specificity ordering reached general-purpose languages only
for types (Julia's dispatch) and CSS; predicate rulebooks stayed in rule
engines [assessment]; test scripts spread as transcript tests [19].

### English keyword phrases and the order problem: SQL

SEQUEL (1974) was a "structured English query language" whose "walk up and
read" goal motivated its pseudo-English syntax [6][8]. Google's 2024 paper:
"SQL's syntactic structure for composing queries ... is terrible", while
"SQL's localized syntax with English-like keyword phrases is an anachronism,
but we can live with it"; the order `SELECT ... FROM ... WHERE` "is rigid and
arbitrary and doesn't reflect the actual data flow", and the walk-up-and-read
choice "has aged poorly" [6]. The shipped fix is pipe syntax in GoogleSQL,
being explored for public BigQuery and Spanner [6], and in Apache Spark [7]:

```sql
FROM customer
|> LEFT OUTER JOIN orders ON c_custkey = o_custkey
|> AGGREGATE COUNT(o_orderkey) c_count GROUP BY c_custkey
|> AGGREGATE COUNT(*) AS custdist GROUP BY c_count
|> ORDER BY custdist DESC, c_count DESC;
```

**Transplant.** Python's comprehension inherits the output-first order
(`[p * p for p in primes if p > 3]` uses `p` before binding it); Google's
style guide allows comprehensions but "multiple `for` clauses or filter
expressions are not permitted" [34]. Sisu's method chains have pipe order.

### Dialects and command languages: Rebol and Tcl

**Rebol** (Carl Sassenrath, 1997) builds "small, optimized,
domain-specific languages for code and data", dialects: a block stays
unevaluated until a function interprets it, as in `parse` or the GUI dialect
`view layout [text "Hello world!" button "Quit" [quit]]` [14]. Rebol/Core is
about 300 KB; Rebol 3 went Apache 2.0 in December 2012; Red (2011) is at
0.6.6 beta (March 2025) [14]. A dialect is cheap and unchecked: its grammar
is whatever the interpreting function accepts (`parse "2026-10-12" [copy y
4 digit "-" copy m 2 digit "-" copy d 2 digit]`, with `digit` a charset).

**Tcl** (John Ousterhout, 1988) made everything a string and every construct
a command; `uplevel` and `upvar` run code in the caller's frame, so a
procedure can define a loop [16]:

```tcl
proc repeat {n body} { for {set i 0} {$i < $n} {incr i} { uplevel 1 $body } }
repeat 3 { puts "hei" }
```

Ousterhout's 1998 manifesto called "typeless" glue languages "just as safe
as system programming languages", on "anecdotal support" ("a factor of two
to a factor of 60", "more like a factor of five to 10") [15]. Tcl 8.0 (1997)
added a bytecode compiler, a fix for re-parsing strings; Tcl 9.0 arrived in
September 2024; Tcl survives in embedding (Tk, Expect, EDA tools) [16].

### Everything is an expression: Wolfram Language

```wolfram
FullForm[a + b c]               (* Plus[a, Times[b, c]] *)
area[Circle[r_]] := Pi r^2      (* a rewrite rule; area[Circle[2]] gives 4 Pi *)
```

Mathematica 1.0 (June 1988) made every value a tree `head[args]` rewritten
until nothing matches; the notebook front end appears in the 2.0 entry
(1991); 10.0 (2014) was the "first version based on the complete Wolfram
Language" [17]. Natural language is an input method: free-form
"Wolfram|Alpha-like input" yields an expression the user sees [17]. Uniform
expressions buy symbolic mathematics; global, order-dependent rewriting
makes evaluation hard to predict, so typed languages took constructor
patterns (`match`) and left open rewriting [assessment].

### First-class grammars: Raku

```raku
grammar TestGrammar { token TOP { \d+ } }
class TestActions { method TOP($/) { make(2 + $/) } }
say TestGrammar.parse('40', actions => TestActions.new).made;   # 42
```

A Raku grammar is a class whose methods are named regexes; `token` and
`rule` ratchet (no backtracking, which "isn't appropriate for all cases");
actions build values with `make`/`made`; parsers become inheritable code
[26]. The cost is the language around them: Perl 6 took from July 2000 to 25
December 2015 to ship ([`perl-php.md`](../languages/perl-php.md)) [26], and a
grammar that modules extend at compile time is a macro system every tool must
run [assessment; Q-15]. The library form travelled better: LPeg made PEGs
ordinary Lua values compiled to a parsing machine with "a clear performance
model" [27], and CPython's own parser became a PEG parser in 3.9 [28].

### Literate programming, and what survived

Knuth: "The main idea is to treat a program as a piece of literature,
addressed to human beings rather than to a computer"; WEB's TANGLE extracts
code and WEAVE typesets the essay, making programs "more robust, more
portable, more easily maintained" [18]. Inform's inweb has carried a
300,000-line compiler since 2004 [13], yet Nelson says it "is not a popular
choice today", "most programmers do not like to write essays", and "there
are no really first-rate tools for it" [11]. Notebooks revived the
document-as-program and showed the cost: of 863,878 attempted executions of
GitHub notebooks, 24.11% ran without errors and 4.03% reproduced their
results; 36.36% ran cells out of order [23]. Org Babel [22] and nbdev
(vendor claims only) [24] stay niche.

**What survived** is prose attached to code units and checked by the
toolchain: doc comments (even Nix is standardising `/** */` CommonMark ones,
RFC 145, begun 2023 [25]); Python's doctest ("literate testing" or "executable
documentation", "serious about requiring exact matches", used by 6% of
Python developers [19]); Go's examples, whose `// Output:` is compared with
captured output [20]; Rust's and Elixir's doctests
([`testing.md`](../topics/testing.md)); and Unison docs, whose snippets "are
typechecked" and whose examples render with their evaluated results [21].

### "English is the new programming language" (2023-2026)

Karpathy: "the hottest new programming language is English" (2023), then
"vibe coding" (2 February 2025): "forget that the code even exists" [29].
EWD667 is the classic reply: native tongues make nonsense hard to notice, so
formal symbols are "a privilege" [3]. The evidence since agrees:

- *Ambiguity.* ClarifyGPT flags a requirement when sampled programs
  disagree, then asks; GPT-4's Pass@1 rose from 70.96% to 80.80% [30].
  EvalPlus added 80 times more tests to HumanEval and pass@k fell by up to
  19.3-28.9%: programs matched the English and its examples and were wrong [31].
- *Precision helps generation.* Type-constrained decoding more than halves
  compilation errors (Mündler et al., PLDI 2025) [32].
- *Productivity and quality.* METR (July 2025): experienced developers 19%
  slower with AI while believing they were 20% faster [32]; a December 2025
  review of 470 pull requests found about 1.7 times more major issues in AI
  co-authored code (secondary report) [29].
- *Novices.* A Hedy learner "kindly asked the computer to select a number",
  the known misconception that computers "can process natural language" [35].

Practice converged on English as input, code as artefact: Spec Kit (141k
stars) runs "specify → plan → tasks → implement → converge" [33].

### Synthesis: worth having, and the read-only trap

| Idea | In Sisu | Why |
|---|---|---|
| A few single-word English keywords, conventional structure | Keep (already) | Python; Cook: users "prefer a more conventional programming language structure" [1] |
| Multi-word keywords, noise words, English word order, dialects | Trap | AppleScript retrospective; COBOL's 300 reserved words |
| Rules with preconditions | Already as `requires` and `match` guards; not as open rulebooks | Inform's specificity laws fit a world model, not a program |
| Relations with cardinality | Library only | Relations are a Sisu non-goal at language level |
| First-class grammars | Library values evaluated at comptime (idea 3) | Raku's cost against LPeg and PEP 617 |
| Literate documentation that is tested | Yes: rendered examples (1), transcripts (2) | Doctest, Go, Unison survived; WEB and notebooks did not |
| Natural language as the program | Trap; tooling input only (5) | EWD667, ClarifyGPT, EvalPlus, METR |

## Transferable ideas for Sisu

### 1. Doc examples that show their values

`sisu doc` runs each doc example in the interpreter (with test fakes for
capabilities) and renders the value of every expression statement beneath
it, as Unison and Go do. `assert` lines stay the checked claims, so nothing
is compared as text, which is doctest's fragility:

```sisu
## ```
## parse_price("€12,50")
## assert parse_price("€0,99") == Ok(99)
## ```
def parse_price(text: str) -> int ! PriceError: ...
```

renders `parse_price("€12,50")` ⇒ `Ok(1250)`. *Cost:* small; examples already
compile as tests. *Conflicts:* none. *Confidence:* medium (adopted
precedents; benefit unmeasured).

### 2. Transcript tests: "Test me with" for whole programs

`sisu test` also runs `tests/*.transcript` files. A `$` line runs the
package's `main` with those arguments and a fake `System` (in-memory `Fs`
from a fixture directory, frozen `Clock`, seeded `Rand`, no `Net`); `>`
lines are standard input; other lines are expected output; a mismatch fails
with a diff and `sisu test --update` re-blesses after showing it:

```
$ shop buy apples 2
ok: 1 left
$ shop buy pears 1
error: no pears in stock (exit 1)
```

With `sisu>` prompts it tests REPL sessions in tutorials and the teaching
path; the fakes make it deterministic, as in 17-testing.
*Cost:* a runner and a fake `System`. *Conflicts:* overlaps `assert_snapshot`,
whose update ceremony applies. *Confidence:* medium (Inform's TEST and
doctest's sessions are precedents; fakes remove their fragility).

### 3. Grammars as values: a blessed PEG library for comptime DSLs

08 already routes new syntax into "a `comptime` function that parses a
string literal and returns a typed value". A `std.x.peg` library makes those
parsers uniform: grammars are ordinary values built from combinators,
compiled at comptime into a parsing table with LPeg's predictable cost
model, and run the same at compile time and run time:

```sisu
from std.x.peg import Grammar, digits, lit, seq
const ISO_DATE: Grammar[Date] = comptime seq(digits(4), lit("-"), digits(2), lit("-"), digits(2))
    .map((y, _, m, _, d) -> Date(year=y, month=m, day=d))
const LAUNCH = comptime ISO_DATE.parse("2027-01-01").expect("valid date")   # a typo fails the build
let d = ISO_DATE.parse(input)?                                               # Date ! peg.ParseError
```

*Cost:* a library (combinators, parsing machine, errors with spans into
literals) and comptime fuel. *Conflicts:* none with "no macros": grammars
produce values and cannot extend Sisu's syntax. *Confidence:* medium.

### 4. Data-flow order over English order

Lint comprehensions to one `for` and one `if`, with a fix-it to a loop or a
chain; give any future query library pipe order, not SQL's [6][34]. *Cost:*
one lint. *Conflicts:* none; comprehensions stay ([`python-idioms.md`](../topics/python-idioms.md)).
*Confidence:* medium that it is cheap and harmless, low that it helps.

### 5. Natural language as input to the tools, never the source

A `sisu draft` action sends a signature, its `##` text, contracts and
examples to a model under type-constrained decoding (planned in 10-tooling),
runs the examples and contracts, and, when sampled bodies disagree on an
input, asks about that input instead of choosing (ClarifyGPT's check [30]):

```sisu
## Converts a price like "€12,50" to cents; rejects a missing "€".
def parse_price(text: str) -> int ! PriceError:
    ensures result.is_ok() == text.starts_with("€")
    ...      # draft: candidates disagree on "€12" (Ok(1200) or Err(BadFormat)?): asks
```

The output is ordinary Sisu under review. *Cost:* glue over the planned
type-checker library. *Conflicts:* none; overlaps
[`holes-synthesis-and-ai-era.md`](holes-synthesis-and-ai-era.md).
*Confidence:* low (the evidence moves monthly).

## Dead ends

- **English-like surface syntax** (multi-word keywords, noise words, English
  word order): its designer calls AppleScript's experiment unsuccessful [1];
  natural-language text as the program fails for the same reason [3][30][31].
- **Localized dialects of the source**: dropped by AppleScript in 1998 [1];
  Hedy's localized keywords live inside its own browser environment.
- **Open rulebooks with specificity ordering as dispatch**: four laws suit a
  world model [12]; Sisu has no overloading, and `match` order is visible.
- **Everything is a string, and `uplevel`/`upvar`**: Tcl paid with a bytecode
  rewrite [16]; reaching into a caller's frame defeats ownership and capabilities.
- **Unevaluated-block dialects and extensible grammars** (Rebol, Raku):
  macros by another name, with unchecked DSLs [14][26].
- **Global rewrite rules as the evaluation model** (Wolfram) [17]; **literate
  files or notebooks as the source of truth** [11][23].

## Sources

1. W. R. Cook, "AppleScript", HOPL III (2007), draft of 29 Sept 2006, https://www.cs.utexas.edu/~wcook/Drafts/2006/ashopl.pdf.
2. J. Gruber, "The English-Likeness Monster", Daring Fireball, https://daringfireball.net/2005/09/englishlikeness_monster (27 Sept 2005).
3. E. W. Dijkstra, "On the foolishness of 'natural language programming'", EWD667, https://www.cs.utexas.edu/users/EWD/transcriptions/EWD06xx/EWD667.html (late 1970s).
4. E. W. Dijkstra, "How do we tell truths that might hurt?", EWD498, https://www.cs.utexas.edu/users/EWD/transcriptions/EWD04xx/EWD498.html (18 June 1975).
5. Wikipedia, "COBOL" (1959 goals, reserved words, Gartner 1997 estimate, 2020 use), https://en.wikipedia.org/wiki/COBOL (2026).
6. J. Shute et al., "SQL has problems. We can fix them: pipe syntax in SQL", PVLDB 17, 4051-4063, https://research.google/pubs/sql-has-problems-we-can-fix-them-pipe-syntax-in-sql/ (2024).
7. Apache Spark documentation, "SQL pipe syntax", https://spark.apache.org/docs/latest/sql-pipe-syntax.html (2026).
8. D. Chamberlin, R. Boyce, "SEQUEL: a structured English query language", SIGFIDET 1974 (not opened; cited via [6]).
9. Inform repository README and releases, https://github.com/ganelson/inform and https://github.com/ganelson/inform/releases (2022-2026).
10. G. Nelson, "Assessing Inform", London, https://ganelson.github.io/inform-website/talks/2018/06/09/london.html (9 June 2018).
11. G. Nelson, "Opening Inform", Narrascope, https://ganelson.github.io/inform-website/talks/2019/06/14/narrascope.html (14 June 2019).
12. Writing with Inform: §1.7 "The Skein", §2.8 "The TEST command", §11.17 "Phrases to decide other things", §13.5 "Making new relations", §19.16 "The Laws for Sorting Rulebooks", https://ganelson.github.io/inform-website/book/WI_2_8.html (and WI_1_7, WI_11_17, WI_13_5, WI_19_16) (2026).
13. inweb repository, https://github.com/ganelson/inweb (2026).
14. Wikipedia, "Rebol", https://en.wikipedia.org/wiki/Rebol; "Red (programming language)", https://en.wikipedia.org/wiki/Red_(programming_language) (2026).
15. J. K. Ousterhout, "Scripting: higher-level programming for the 21st century", IEEE Computer, https://web.stanford.edu/~ouster/cgi-bin/papers/scripting.pdf (March 1998).
16. Wikipedia, "Tcl (programming language)" (EIAS, uplevel/upvar, 8.0, 9.0 on 26 Sept 2024), https://en.wikipedia.org/wiki/Tcl_(programming_language) (2026).
17. Wolfram, "Mathematica quick revision history", https://www.wolfram.com/mathematica/quick-revision-history/ (2026).
18. D. E. Knuth, "Literate programming", https://www-cs-faculty.stanford.edu/~knuth/lp.html (page 2026; method 1984).
19. Python documentation, "doctest", https://docs.python.org/3/library/doctest.html (2026); 6% usage in [`testing.md`](../topics/testing.md).
20. A. Gerrand, "Testable examples in Go", https://go.dev/blog/examples (7 May 2015).
21. Unison documentation, "Documentation", https://www.unison-lang.org/docs/usage-topics/documentation/ (2026).
22. E. Schulte, D. Davison, T. Dye, C. Dominik, "A multi-language computing environment for literate programming and reproducible research", J. Statistical Software 46(3), https://www.jstatsoft.org/article/view/v046i03 (2012).
23. J. F. Pimentel, L. Murta, V. Braganholo, J. Freire, "A large-scale study about quality and reproducibility of Jupyter notebooks", MSR 2019, https://leomurta.github.io/papers/pimentel2019a.pdf (2019).
24. nbdev, https://nbdev.fast.ai/ (2026).
25. NixOS RFC 0145, "Doc-comments", https://github.com/NixOS/rfcs/blob/master/rfcs/0145-doc-strings.md (started 27 Mar 2023).
26. Raku documentation, "Grammars", https://docs.raku.org/language/grammars (2026); Wikipedia, "Raku (programming language)", https://en.wikipedia.org/wiki/Raku_(programming_language) (2026).
27. R. Ierusalimschy, "A text pattern-matching tool based on parsing expression grammars", Software: Practice and Experience, preprint http://www.inf.puc-rio.br/~roberto/docs/peg.pdf (2008).
28. G. van Rossum, P. Galindo Salgado, L. Nikolaou, PEP 617, "New PEG parser for CPython", https://peps.python.org/pep-0617/ (2020).
29. Wikipedia, "Vibe coding" (Karpathy 2023 and 2 Feb 2025; CodeRabbit Dec 2025 review), https://en.wikipedia.org/wiki/Vibe_coding (2026).
30. F. Mu et al., "ClarifyGPT: empowering LLM-based code generation with intention clarification", https://arxiv.org/abs/2310.10996 (2023).
31. J. Liu, C. S. Xia, Y. Wang, L. Zhang, "Is your code generated by ChatGPT really correct?", NeurIPS 2023, https://arxiv.org/abs/2305.01210 (2023).
32. N. Mündler et al., "Type-constrained code generation with language models", PLDI 2025, https://arxiv.org/abs/2504.09246; METR, early-2025 experienced-developer study (10 Jul 2025); both summarised in [`studies.md`](../evidence/studies.md) §11.
33. GitHub Spec Kit, https://github.com/github/spec-kit (2025-2026).
34. Google Python Style Guide §2.7, https://google.github.io/styleguide/pyguide.html (2026).
35. F. Hermans, "Hedy: a gradual language for programming education", ICER 2020, https://hedy.org/research/Hedy_A_Gradual_Language_for_Programming_Education_2020.pdf (2020).
