# Spreadsheets, formula languages and end-user programming

## Why this family matters for Sisu

Spreadsheet formulas are the most widely used programming system, and their
users succeed without a compile step, a run button, or abstractions written
before any value exists. Sisu plans an interpreter first (D-052) and a
teaching profile (Q-41), so this family can answer which spreadsheet
properties (always-live values, concrete-first work, every intermediate
visible, no abstraction at the start) a compiled textual language can offer
through tooling, and whether that helps; it also measures end-user errors.

## Systems covered

| System | Years | Status (Oct 2026) | Central idea |
|---|---|---|---|
| Excel formulas | 1985-; LET 2020; LAMBDA 2020-21 | Microsoft 365, Excel 2024; Python in Excel since 2023 | A live, first-order functional grid that gained names and functions |
| Google Sheets | 2006-; LAMBDA Aug 2022 | Active | Excel-compatible formulas in the browser |
| Notion, Airtable formulas | 2010s-; Notion Formulas 2.0 in 2023 | Active | Per-record formula fields over typed tables |
| Power Fx (Power Apps); App Maker | 2016-; named, open-sourced 2021; App Maker 2016-2021 | Power Fx MIT on GitHub, extraction unfinished; App Maker shut down | Excel semantics, static types, separate behaviour formulas; low-code builders |
| Forms/3, WYSIWYT | 1990s-2000s | Research; inactive since mid-2000s | Visual spreadsheet language; testing by ticking values |
| FlashFill, Formula by Example | 2011-; Excel 2013 | Shipping | Programs synthesised from input-output examples |
| HyperCard / HyperTalk | 1987-2004 | Withdrawn 2004; LiveCode open edition ended 2021; Decker (MIT) | Stacks of cards, user levels, English-like scripts |
| Hedy | 2020- | ~500,000 monthly users (2024, self-reported); 54 languages | Syntax added level by level until it is Python |
| Scratch | 2007- | 103M+ registered users (2023) | Low floor, high ceiling, wide walls (blocks: `visual-and-structure-editing.md`) |

## The ideas

### Spreadsheets are code, with code's error rates

**What it is.** Hermans et al. argue that spreadsheets share domains,
expressive power and maintenance problems with software, so testing, smell
detection and refactoring transfer [13]. Their formula smells are code
smells renamed (*Multiple Operations*, *Multiple References*, *Conditional
Complexity*, *Long Calculation Chain*, *Duplicated Formula*) [12], and
BumbleBee refactors whole groups of formulas [14].

**Evidence.** In ten real spreadsheets the smells found two actual faults
and owners showed "little awareness of the risks of complicated formulas"
[12]. In the Enron corpus (15,770 spreadsheets, 9,120 with formulas) about a
quarter of those with formulas contain an Excel error, and 76% use the same
15 functions [15]. Panko's "88% of spreadsheets contain errors" is thinner
than its fame: it is his compilation of 1995-2007 field audits (113
spreadsheets) as reported secondhand [11] (that revision not opened); his
2006 version said 94% of 88 spreadsheets in seven audits, his 2015 paper 94%
of 85 in six, mostly commercial "audits" [9][10]. Powell, Baker and Lawson
found three of the seven sources unpublished, most giving "little or no
information on their definition of errors or on the methods used", and
concluded that the literature "does not provide sufficient grounds for a
reliable, quantitative estimate" [10]. The robust number is per cell: 14
laboratory studies (967 participants) average a 3.9% cell error rate, close
to the 1.9-3.7% faulty lines found by industrial code inspections; long
dependency chains compound it into wrong bottom lines [9]. My reading: end-user
code is about as wrong as uninspected professional code, and rarely tested.

Incidents add mechanisms. Automatic conversion turned gene names into dates
in about one in five genomics papers with Excel gene lists (2016); the HGNC
renamed 27 genes in 2020 (`SEPT1` to `SEPTIN1`) and Microsoft added an
opt-out in October 2023 [16]. Excel "tends to produce output even in the
presence of errors", which Ko et al. tie to overconfidence [17].
**Transplant:** Excel's shipped fix was textual language features (LET in
2020, then LAMBDA), not smell detectors [1].

### Always live, no run button

**What it is.** Power Fx states the principle: worksheets are "always live
... There's no compile or run mode ... Any detected errors surface
immediately and don't interfere with the rest of the worksheet", kept so by
an incremental compiler [25]. Gordon: "It's live programming ... You make a
little change and you can instantly see the results" [2].

**What it buys.** Reach: Excel formulas are written by "an order of
magnitude more users than all the C, C++, C#, Java, and Python programmers
in the world combined" [1], or "a hundred times as many ... as there are
professional programmers" (Peyton Jones) [2]. Scaffidi, Shaw and Myers
projected 90 million US workplace end users by 2012, 55 million using
spreadsheets or databases, against fewer than 3 million professional
programmers [8][17]; the oft-quoted 750 million to 1.2 billion monthly users
traces to a training company's estimate [16].

**Where it failed.** Liveness without correctness feedback makes people
wrong with confidence: "immediate feedback about the values a program
computes, without feedback about correctness, leads to significantly higher
overconfidence", and 5-23% of end users' judgments of shown values are
wrong, mostly by accepting wrong ones [17]. Panko: "every empirical study
has found overconfidence in the face of many errors" [9].

**The safety condition.** Re-running on every edit is safe only if
evaluation cannot act on the world. Power Fx confines `Patch`, `Collect`,
`Back` and similar functions to *behaviour formulas* in `On...`
properties because they "change the state of the app, they can't be
automatically recalculated" [26]. Python in Excel runs in a container where
"the Python code doesn't have network access" and reads the workbook only
through references passed to `xl()` [7]. Both are capability disciplines.

**Transplant.** Xcode playgrounds (2014; "heavily influenced by Bret
Victor's ideas, by Light Table", Lattner) and the Swift Playgrounds app
(2016) [41]; Unison's `>` watch expressions ([`unison.md`](../languages/unison.md)).
No controlled outcome study exists; the lineage is in
[`live-and-exploratory-environments.md`](live-and-exploratory-environments.md).

### Concrete first, abstraction later

**What it is.** Microsoft's own LAMBDA workflow is concrete-first: test the
formula in a cell, wrap and call it at once (`=LAMBDA(number, number +
1)(1)` returns 2), then "move it to the Name Manager" [3]. With LAMBDA,
"Excel has become Turing-complete" [1]:

```
=LET(gross, B2, rate, 0.24, gross - gross * rate)                  (LET, March 2020)
REVERSE = LAMBDA(s, IF(LEN(s) < 2, s, REVERSE(RIGHT(s, LEN(s) - 1)) & LEFT(s, 1)))
```

Peyton Jones, Blackwell and Burnett's 2003 sheet-defined functions were
"ruthlessly" user-centred: "nominate certain cells as the input and one cell
as the output and give it a name" [4][2]. Elastic sheet-defined functions
(JFP 2020) generalise a function written over a fixed-size example array to
any size; users showed lower cognitive load than with map/reduce [5].

**What it buys.** Blackwell's attention-investment model: users weigh
benefit, pay-off, cost and risk before investing in abstraction, so a
concrete start defers the investment until the pay-off shows [17]. Myers et
al.: "high-threshold systems have not been adopted because their intended
audience never makes it past the initial threshold" [18].

**Where it failed.** Abstraction came 35 years late and hides what made the
sheet legible: Evaluate "cannot step into" a LAMBDA [3]. At their limits,
formula languages converge on textual features: Google Sheets added LAMBDA
and named functions (August 2022) [28]; Notion's Formulas 2.0 (2023) added
multi-line formulas with comments, editor type checking, `let`/`lets`,
lists and `map`/`filter` [29]; Airtable's reference still lists no
user-defined functions [30]. A program-like textual view beside the grid
"can significantly improve user performance" in authoring and debugging [6].

**Transplant.** IDEs have "extract function"; none I found turns the
concrete values in scope into the new function's first example.

### Every intermediate visible, and testable: Forms/3 and WYSIWYT

**What it is.** In Forms/3, a declarative visual spreadsheet language [20],
"What You See Is What You Test" lets a user tick any value they believe
correct; the system measures definition-use coverage behind ticked values
and colours cells by testedness; range assertions on cells propagate by
interval arithmetic and conflicts are circled [17][21].

**Evidence and limits.** Studies at ICSE 2000, ICSM 2001 and ICSE/CHI 2003 back it;
assertions were "shown to increase people's effectiveness at testing and
debugging"; Ko et al. call WYSIWYT "the most mature error-detecting approach
for end-user programmers" and note users' crosses are more reliable than
their ticks [17][21]. It stayed in prototypes, inactive since the mid-2000s
[20]. Its industrial cousin is snapshot testing, which Sisu has
([17-testing](../../proposal/17-testing.md)); WYSIWYT adds approval of
single values while writing, and coverage of the approved set.

### Programming by example

FlashFill (Gulwani, POPL 2011) synthesises a string program from a few
input-output pairs and asks about ambiguous inputs; it shipped in Excel 2013
[22]. Its output is static values that do not update when "the source data
changes" [23]: the program is invisible and liveness is lost. The correction
emits code: Formula by Example proposes a formula to review (`=LEFT([@Name])
& MID([@Name], FIND(" ", [@Name]) + 1, 1)`) [23], and FlashFill++ (POPL 2023)
lists "readable code generation" [24]. General synthesis: [`holes-synthesis-and-ai-era.md`](holes-synthesis-and-ai-era.md).

### The gentle slope: levels from user to programmer

MacLean et al. (CHI 1990, not opened) are the usual source of the "gentle
slope" [19]; Myers et al. ask for low threshold, high ceiling and a smooth
path [18]; Ko et al. define end-user programming by "the intent" [17].

**HyperCard** (11 August 1987, Atkinson's "software erector set"; HyperTalk
by Dan Winkler) was free with every new Mac [38]. Scripts read like English
(`put 5 * 4 into theResult`), the Home stack offered user levels from
browsing to scripting (level names not re-verified), Myst was built in it and
Cunningham traced the wiki to a stack [38]. Its successors lost three things.
The free slope: Claris shipped "a crippled, low user-level, Home stack",
unlocked by typing "magic", then a Player with authoring sold separately.
The network: "HyperCard could have become the first Web browser" (Atkinson,
Wired 2002). Continuity: last update 1998, withdrawn March 2004 [38];
LiveCode's open edition (2013) ended on 31 August 2021, and Decker (MIT,
scripted in Lil) revives the model [39].

**Hedy** (Hermans, ICER 2020) changes the syntax level by level until it is
Python, modelled on how children learn punctuation; its principles include
"The initial offering of a concept is the simplest form possible", "Only one
aspect of a concept changes at a time" and deferring "brackets and colons ...
to the latest moment possible" [31]. Level 1 is `print Hello !`; 13 levels in
2020, 18 later. A 2026 talk shows `choices is stone, paper, scissors` at
level 3 becoming `choices = ['stone','paper','scissors']` at level 16 [34].

Outcomes are early and descriptive. Of 9,714 programs in spring 2020, 1,363
failed (printed as 12.2%; the ratio is 14.0%), led by incomplete code
(18.6%), *code ran at the wrong level* (18.4%) and misspelled keywords [31].
Thirty-nine children aged 11-14 "appreciate the gradual nature", with
commands that "change or overlap" the main difficulty [32]; a
quasi-experiment with 87 undergraduates found "comparable programming
performance" with and without Hedy [36]. Adoption: about 300,000 monthly users in 2023-24 abstracts [35],
"about 500,000" and 54 languages in Hermans' 2024 abstract [33].

**Scratch**: Papert's "low floor" and "high ceiling" plus "wide walls", and
"more tinkerable, more meaningful, and more social"; 103 million users by
2023 [37]. **Transplant:** HtDP's levels, behind Q-41 ([`lisp-scheme-racket.md`](../languages/lisp-scheme-racket.md)).

### Low-code churn

Power Fx promises "no-code ... to 'pro-code' ... with no learning or
rewriting cliffs": Excel consistency, static types with inference, pure
formulas (`If( Value(TextBox1.Text) >= 0, Color.Black, Color.Red )` needs no
`OnChange` handler [26]), locale-sensitive decimals (`3,14` in France, so
lists use `;`), "all data types can take on the value of blank", and a
version stamp per document so a "back compat converter" rewrites old
formulas on next edit [25]. The churn: Google App Maker shut down on 19
January 2021 "due to low usage" [40]; Amazon Honeycode (2020-2024)
[unverified]; Power Fx, open-sourced in 2021, still says in 2026 "we are in
the process of extracting the language" [27]. The formula language outlives
every builder built on it.

### Synthesis: what a compiled language can offer

Sisu has the precondition that made liveness safe in Power Fx and Python in
Excel: a function given no capability cannot touch the world, and 08 already
defines the subset the comptime interpreter may evaluate ("no capabilities,
no `unsafe`, and no `Shared`"); test fakes (`Fs.in_memory`) cover the rest.

| Property | How Sisu could offer it | Evidence it helps | Verdict |
|---|---|---|---|
| Always live | Worksheet files re-run on edit; only capability-free code runs unasked | Adoption; Swift and Unison in typed languages; no controlled study | Offer (1) |
| Concrete first | Extract function turns concrete bindings into parameters and a first example | Excel's own workflow; elastic-SDF study; attention investment | Offer (3) |
| Every intermediate visible | Per-binding values for one example inside a function | Calculation View [6]; Babylonian prototype, response time only [42] | Offer cautiously (4) |
| No abstraction at the start | Teaching levels; worksheets without `def` | Threshold argument; Hedy, Scratch adoption; HtDP | Offer (2) |
| Errors do not stop the sheet | `Err` or panic shown at its line; independent lines keep values | Power Fx design statement | Offer, never as silent propagation |
| Correctness feedback | Approve shown values into tests | Overconfidence findings [17] | Required with liveness (5) |
| Grid, positional references | Not offered | Pointing errors are a recurring class [10] | Dead end |

Beginners: it is moderate that gradual, immediate tools help people start
(adoption, HtDP); there is no evidence on liveness as a learning gain in a
compiled language. Professionals: no controlled evidence either way. The
ideas below are tooling, cheap to retract and measurable in [18-next-steps](../../proposal/18-next-steps.md).

## Transferable ideas for Sisu

### 1. Worksheet files: live values, re-run top to bottom

A file opened as a worksheet (`sisu ws prices.sisu`, or an LSP mode) allows
top-level statements; after each edit the interpreter re-runs it and the
editor shows each value as virtual text (the `┊` column is never saved):

```sisu
from prices import parse_price

let samples = ["€12,50", "€0,99", "12,50"]           ┊ ["€12,50", "€0,99", "12,50"]
let parsed = [parse_price(s) for s in samples]       ┊ [Ok(1250), Ok(99), Err(MissingCurrency)]
let total = parsed.iter().filter_map(it.ok()).sum()  ┊ 1349
let fs = Fs.in_memory({"stock.csv": "apples,3"})     ┊ Fs(1 file); fakes run unasked, System never
```

The whole file re-runs, so there is no hidden state or out-of-order cell;
real capabilities run only on an explicit "run"; comptime fuel limits apply;
a failing line shows its `Err` and dependent lines grey out. *Cost:* the LSP
hosts the interpreter, re-evaluating incrementally and asynchronously within
the 100 ms budget. *Conflicts:* the notebook non-goal (00, 14) needs a
sentence separating a worksheet (one file, re-run whole, no saved output)
from a notebook; script mode must allow top-level statements. *Confidence:*
medium (feasible, as Swift and Unison show; benefit unmeasured).

### 2. The teaching profile as monotone levels

Q-41's profile becomes a short ladder set per package:

```toml
[package]
profile = "teaching"
level = 2   # 1: let/var, print, if, for over range   2: + def, list, dict, one-for comprehensions
            # 3: + struct, enum, match                4: + T?, T ! E, ?, test blocks
```

A program accepted at level n is accepted with the same meaning at every
higher level; levels only reject and never change syntax, which avoids
Hedy's "wrong level" errors and its commands that "change or overlap"
[31][32]. Diagnostics name the level: "`match` arrives at level 3; this
package is at level 2". *Cost:* the AST filter is cheap; the per-level
diagnostic corpus is the work. *Conflicts:* none with D-027 (subsets, not
dialects); Q-41 grows from one profile to several. *Confidence:* medium.

### 3. Concrete first, then "extract function from example"

Extracting worksheet lines that compute on `let raw = "€12,50"` (shown
value `1250`) makes `raw` the parameter, infers the error set, and turns the
concrete case into the first doc example, Excel's "test in a cell, then name
it" in text:

```sisu
## ```
## assert parse_price("€12,50") == Ok(1250)
## ```
def parse_price(raw: str) -> int ! ParseError:
    let digits = raw.strip_prefix("€") ?? raw
    let euros, cents = digits.split_once(",") ?? (digits, "0")
    return Ok(parse_int(euros)? * 100 + parse_int(cents)?)
```

*Cost:* one refactoring. *Conflicts:* none. *Confidence:* medium.

### 4. Example lens: every intermediate value inside a function

A code lens runs a function on one chosen example (a doc example, a
parametrised case, or a saved property-test counterexample) and shows each
binding; untaken branches are dimmed:

```sisu
def parse_price(text: str) -> int ! {PriceError, ParseError}:   ┊ example 1/3: "€12,50"
    let digits = text.strip_prefix("€") else:                   ┊ "12,50"
        return Err(MissingCurrency)
    let euros, cents = digits.split_once(",") else:             ┊ ("12", "50")
        return Err(BadFormat)
    return Ok(parse_int(euros)? * 100 + parse_int(cents)?)      ┊ Ok(1250)
```

It supplies inside an abstraction what Excel loses when a formula becomes a
LAMBDA. *Cost:* a tracing interpreter mode mapped to spans; loops shown as a
last value plus a table on demand; capability parameters bound to fakes.
*Conflicts:* LSP latency (run asynchronously). *Confidence:* medium for
feasibility [42], low for measured benefit.

### 5. Approve a value, get a test

Every value shown by ideas 1 and 4 offers *this is right*, which writes a
test from the fully rendered value (`test "parse_price accepts €0,99":` with
`assert parse_price("€0,99") == Ok(99)`), and *this is wrong*, which writes a
failing stub with the observed value in a comment; the lens marks branches
no approved example has exercised (WYSIWYT's coverage, by branch). *Cost:*
small on top of 1 and 4. *Conflicts:* 17-testing's snapshot ceremony
applies: only fully displayed values, never in bulk. *Confidence:* low (the
studies are of spreadsheet end users, and ticks become rubber-stamps when
5-23% of judgments are wrong [17]).

## Dead ends

- **The grid and positional references.** Pointing errors recur in the
  error studies and Excel's own fix was names (LET) [1][10].
- **Silent errors, blanks and helpful conversions.** Excel computes on
  through errors [17], Power Fx lets every type be "blank" [25], and gene
  names became dates until their owners renamed 27 genes [16]; Sisu's `T?`,
  `T ! E` and explicit conversions are the opposite decisions, in worksheets too.
- **Liveness without validation** [17]: idea 1 is only safe with idea 5.
- **Syntax that changes between levels** (Hedy's unquoted strings, `is`
  becoming `=`): the "wrong level" error class [31], and a second dialect,
  which D-027 rules out.
- **Example-only programs.** FlashFill's invisible static result was fixed by
  emitting formulas [23]; elastic generalisation [5] has no shipped use yet.
- **A visual low-code builder.** Builders churn; the language survives [27][40].
- **Locale-dependent syntax** (Power Fx's `;` and `;;` [25]): one formatter,
  no options.

## Sources

1. A. Gordon, S. Peyton Jones, "LAMBDA: The ultimate Excel worksheet function", Microsoft Research blog, https://www.microsoft.com/en-us/research/blog/lambda-the-ultimatae-excel-worksheet-function/ (25 Jan 2021).
2. Microsoft Research Podcast, "Advancing Excel as a programming language with Andy Gordon and Simon Peyton Jones", https://www.microsoft.com/en-us/research/?p=743248 (5 May 2021).
3. Microsoft Support, "LAMBDA function", https://support.microsoft.com/en-us/office/lambda-function-bd212d27-1cd1-4321-a34a-ccbf254b8b67 (2026).
4. S. Peyton Jones, A. Blackwell, M. Burnett, "A user-centred approach to functions in Excel", ICFP 2003, https://www.microsoft.com/en-us/research/?p=259302 (2003).
5. M. McCutchen, J. Borghouts, A. D. Gordon, S. Peyton Jones, A. Sarkar, "Elastic sheet-defined functions", JFP 30 e26, https://www.microsoft.com/en-us/research/?p=553137 (2020).
6. A. Sarkar, A. D. Gordon, S. Peyton Jones, N. Toronto, "Calculation View: multiple-representation editing in spreadsheets", VL/HCC 2018, https://www.microsoft.com/en-us/research/publication/calculation-view-multiple-representation-editing-in-spreadsheets/ (2018).
7. Microsoft Support, "Data security and Python in Excel", https://support.microsoft.com/en-us/office/data-security-and-python-in-excel-33cc88a4-4a87-485e-9ff9-f35958278327 (2026).
8. C. Scaffidi, M. Shaw, B. Myers, "Estimating the numbers of end users and end user programmers", VL/HCC 2005, 207-214 (2005); figures as cited in [17] and https://en.wikipedia.org/wiki/End-user_development.
9. R. Panko, "What we don't know about spreadsheet errors today", EuSpRIG 2015, https://arxiv.org/abs/1602.02601 (2015).
10. S. G. Powell, K. R. Baker, B. Lawson, "A critical review of the literature on spreadsheet errors", Decision Support Systems 46, 128-138, http://mba.tuck.dartmouth.edu/spreadsheet/product_pubs_files/literature.pdf (2008).
11. Journal of Accountancy, "How to debug Excel spreadsheets" (88% of 113 spreadsheets, 1995-2007, attributed to Panko), https://www.journalofaccountancy.com/issues/2015/nov/how-to-debug-excel-spreadsheets/ (Nov 2015).
12. F. Hermans, M. Pinzger, A. van Deursen, "Detecting code smells in spreadsheet formulas", ICSM 2012; summary https://www.felienne.com/archives/394 (2012).
13. F. Hermans, B. Jansen, S. Roy, E. Aivaloglou, A. Swidan, D. Hoepelman, "Spreadsheets are code", SANER 2016, https://serg.ewi.tudelft.nl/publications/spreadsheets-are-code-an-overview-of-software-engineering-approaches-applied-to-spreadsheets/ (2016).
14. F. Hermans, D. Dig, "BumbleBee: a refactoring environment for spreadsheet formulas", FSE 2014, https://research.vu.nl/en/publications/bumblebee-a-refactoring-environment-for-spreadsheet-formulas/ (2014).
15. F. Hermans, E. Murphy-Hill, "Enron's spreadsheets and related emails: a dataset and analysis", ICSE 2015, https://figshare.com/articles/Enron_s_Spreadsheets_and_Related_Emails_A_Dataset_and_Analysis/1222882 (2015).
16. Wikipedia, "Microsoft Excel" (gene names, HGNC 2020, October 2023 setting, user estimate and its source), https://en.wikipedia.org/wiki/Microsoft_Excel (2026).
17. A. J. Ko et al., "The state of the art in end-user software engineering", ACM Computing Surveys 43(3), https://faculty.washington.edu/ajko/papers/Ko2011EndUserSoftwareEngineering.pdf (2011).
18. B. Myers, S. E. Hudson, R. Pausch, "Past, present and future of user interface software tools", ACM TOCHI 7(1), https://www.cs.cmu.edu/~amulet/papers/futureofhci.pdf (2000).
19. A. MacLean, K. Carter, L. Lövstrand, T. Moran, "User-tailorable systems: pressing the issues with buttons", CHI 1990, https://dl.acm.org/doi/10.1145/97243.97271 (1990; not opened).
20. M. Burnett et al., "Forms/3: a first-order visual language to explore the boundaries of the spreadsheet paradigm", JFP 11(2); https://web.engr.oregonstate.edu/~burnett/Forms3/forms3.html (2001).
21. G. Rothermel, M. Burnett et al., "A methodology for testing spreadsheets", TOSEM 10(1) (2001); K. Rothermel et al., ICSE 2000; M. Burnett et al., "End-user software engineering with assertions in the spreadsheet paradigm", ICSE 2003 (as summarised in [17]).
22. S. Gulwani, "Automating string processing in spreadsheets using input-output examples", POPL 2011, https://www.microsoft.com/en-us/research/publication/automating-string-processing-spreadsheets-using-input-output-examples/ (2011).
23. S. Hurst, "Formula by Example - like Flash Fill but actually useful", ICAEW, https://www.icaew.com/technical/technology/excel-community/excel-community-articles/2025/formula-by-example-like-flash-fill-but-actually-useful (9 Dec 2025).
24. J. Cambronero, S. Gulwani et al., "FlashFill++", POPL 2023; S. Gulwani, SEMLA 2023 slides, https://semla.polymtl.ca/wp-content/uploads/2023/08/semla23-SumitGulwani.pdf (2023).
25. Microsoft Learn, "Microsoft Power Fx overview", https://learn.microsoft.com/en-us/power-platform/power-fx/overview (updated Aug 2026).
26. Microsoft Learn, "Understand imperative logic in Microsoft Power Fx", https://learn.microsoft.com/en-us/power-platform/power-fx/imperative (2024-2025).
27. microsoft/Power-Fx repository README, https://github.com/microsoft/Power-Fx (2026).
28. Google Workspace Updates, named functions and LAMBDA in Sheets, https://workspaceupdates.googleblog.com/2022/08/named-functions-google-sheets.html (24 Aug 2022).
29. Notion Help, "Formulas 2.0: what's changed", https://www.notion.com/help/guides/new-formulas-whats-changed (2023; month unverified).
30. Airtable Support, "Formula field reference", https://support.airtable.com/docs/formula-field-reference (2026).
31. F. Hermans, "Hedy: a gradual language for programming education", ICER 2020, https://hedy.org/research/Hedy_A_Gradual_Language_for_Programming_Education_2020.pdf (2020).
32. M. Gilsing, F. Hermans, "Gradual programming in Hedy: a first user study", VL/HCC 2021, https://www.hedy.org/research/Gradual_Programming_in_Hedy_A_First_User_Study_2021.pdf; M. Gilsing, J. Pelay, F. Hermans, J. Computer Languages, https://hedy.org/research/Design_Implementation_and_evaluation_of_the_Hedy_programming_language_2022.pdf (2021-2022).
33. F. Hermans, "Hedy: an inclusive, multi-lingual, and gradual programming language", ICPEC 2024, OASIcs 122, https://drops.dagstuhl.de/entities/document/10.4230/OASIcs.ICPEC.2024.1 (2024).
34. F. Weijsenfeld, "Hedy: textual programming made easy", FOSDEM 2026, https://archive.fosdem.org/2026/events/attachments/QVRM9K-hedy-gradual-programming/slides/267717/fosdem202_lzsalhl.pdf (2026); https://www.hedy.org/ (47 languages listed, 2026).
35. Hedy talk abstracts: GOTO 2024, https://gotopia.tech/sessions/3108/hedy-creating-a-programming-language-for-everyone (2024); SPLASH 2021 REBASE, https://2021.splashcon.org/details/splash-2021-rebase/2/Hedy-Creating-a-gradual-programming-language (2021).
36. Springer chapter, two-year quasi-experiment with 87 undergraduates using Hedy, https://link.springer.com/10.1007/978-3-031-53022-7_19 (2024; authors and venue not verified).
37. M. Resnick et al., "Scratch: programming for all", CACM 52(11), https://web.media.mit.edu/~mres/papers/Scratch-CACM-final.pdf (2009); Wikipedia, "Scratch (programming language)", https://en.wikipedia.org/wiki/Scratch_(programming_language) (figures Feb 2023).
38. Wikipedia, "HyperCard" (distribution, Claris, "magic", withdrawal, Atkinson via Wired Aug 2002), https://en.wikipedia.org/wiki/HyperCard; "HyperTalk", https://en.wikipedia.org/wiki/HyperTalk; "Bill Atkinson" (died 5 June 2025), https://en.wikipedia.org/wiki/Bill_Atkinson (2026).
39. Wikipedia, "LiveCode", https://en.wikipedia.org/wiki/LiveCode (2026); Decker, https://beyondloom.com/decker/ (2026).
40. Google Workspace Updates, "App Maker update", https://workspaceupdates.googleblog.com/2020/01/app-maker-update.html (27 Jan 2020).
41. Wikipedia, "Swift Playgrounds", https://en.wikipedia.org/wiki/Swift_Playgrounds (2026); C. Lattner, homepage, https://nondot.org/sabre/ (2026).
42. D. Rauch, P. Rein, S. Ramson, J. Lincke, R. Hirschfeld, "Babylonian-style programming", Programming 3(3), https://arxiv.org/abs/1902.00549 (2019).
