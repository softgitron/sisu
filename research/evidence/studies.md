# Academic and industrial studies on language design and defects

Compiled October 2026. For each study: citation, method, findings, known critiques, and an assessment of how strong
the evidence is for designing a language. Figures were re-read from the papers where the PDF was obtainable;
`[abstract only]` means checked against the published abstract, `[not re-verified]` means the entry relies on prior
knowledge of the paper and should be re-checked before being quoted in the proposal. Strength scale: **strong**
(controlled experiment or large industrial dataset, replicated or consistent with independent data), **moderate**
(single well-designed study, or industrial data with undisclosed methodology), **weak** (small samples, unreplicated,
or refuted on reanalysis).

---

## 1. Does the language affect defect rates? (GitHub mining and its refutation)

### Ray, Posnett, Filkov, Devanbu, "A Large Scale Study of Programming Languages and Code Quality in GitHub", FSE 2014 (CACM 2017 research highlight)

- **Method.** 729 GitHub projects, 17 languages, 80 million SLOC, 29,000 authors, 1.5 million commits (1996-2014).
  Commits labelled "bug-fixing" by keyword matching on messages; negative binomial regression of defective-commit
  counts on language, controlling for size, age, commits and contributors; languages grouped into classes
  (static/dynamic, strong/weak, managed/unmanaged, functional/procedural).
- **Claimed findings.** "language design does have a significant, but modest effect on software quality"; "strong
  typing is modestly better than weak typing, and among functional languages, static typing is also somewhat better
  than dynamic typing". Haskell, Clojure, Ruby and Scala associated with fewer defects; C, C++, Objective-C,
  JavaScript, PHP and Python with more. Language explained less than 1 % of deviance; the authors cautioned that "even
  these modest effects might quite possibly be due to other intangible process factors". The paper was nonetheless
  widely cited as showing functional and static languages "cause" fewer bugs.

### Berger, Hollenbeck, Maj, Vitek, Vitek, "On the Impact of Programming Languages on Code Quality: A Reproduction Study", ACM TOPLAS 41(4), 2019 (arXiv 1901.10220)

- **Method.** (a) Repetition with the original data and scripts; (b) independent reanalysis correcting data and
  statistical problems.
- **Findings.** The repetition reproduced ten of eleven language associations. The reanalysis found projects
  duplicated across languages, TypeScript files that were type declarations, V8 classified as JavaScript, and
  misclassified commits; a manual check estimated a **36 % false-positive rate** on "buggy" commit labels (11 % false
  negatives). After corrections, Bonferroni/FDR adjustment for 16 simultaneous hypotheses and bootstrapping of label
  noise, "only 4 out of the original 11 languages correlated with abnormal defect rates, and even for those the effect
  size is exceedingly small"; the language-class results could not be reproduced at all. Conclusion: it is "not
  possible to establish a causal link between programming language and code quality based on the available data".
- **Assessment: strong (as a negative result).** The best-known large-scale evidence that language choice reduces bugs
  evaporates under reanalysis. Any Sisu claim of the form "feature X reduces defects" must rest on something other
  than GitHub mining.

---

## 2. Static vs dynamic typing

- **Hanenberg, "An experiment about static and dynamic type systems", OOPSLA 2010** `[described via Kleinschmager et
  al. 2012, ref. 8]`. Subjects (49 `[not re-verified]`) implemented a scanner and a parser in two variants of a
  purpose-built language over ~27 hours; the dynamically typed group was significantly faster on the scanner, with no
  significant difference on the parser. Early evidence *against* a productivity benefit of static types for small,
  well-specified tasks.
- **Kleinschmager, Hanenberg, Robbes, Tanter, Stefik, "Do Static Type Systems Improve the Maintainability of Software
  Systems?", ICPC 2012.** 36 subjects (33 analysed), within-subject, Java vs Groovy as "dynamically typed Java", nine
  maintenance tasks. Static typing gave a significant time benefit for tasks requiring use of new, undocumented
  classes and for fixing type errors (vs no-such-method errors); no difference for fixing semantic errors. Earlier
  experiments in the series found type casts slow trivial tasks and that fixing type errors is faster with a static
  checker.
- **Mayer, Hanenberg, Robbes, Tanter, Stefik, "An Empirical Study of the Influence of Static Type Systems on the
  Usability of Undocumented Software", OOPSLA 2012.** 27 subjects on an undocumented API: static types gave faster
  completion on three of five tasks; on the smaller tasks the dynamically typed group was faster. The benefit scales
  with task complexity and the need to discover an API.
- **Endrikat, Hanenberg, Robbes, Stefik, "How do API documentation and static typing affect API usability?", ICSE
  2014** `[not re-verified]`. 2x2 experiment (types x documentation): static typing helped even when documentation was
  present; types and docs were complementary.

### Gao, Bird, Barr, "To Type or Not to Type: Quantifying Detectable Bugs in JavaScript", ICSE 2017

- **Method.** 400 bugs sampled uniformly from fixed public bugs in GitHub JavaScript projects (sized for a 95 %
  confidence interval of +/- 5 % over 3.9 million repositories). For each bug the authors spent up to 10 minutes
  adding Flow 0.30 or TypeScript 2.0 annotations to the buggy version and checked whether the checker flagged it.
- **Finding.** Both Flow and TypeScript detect **15 %** of public bugs (roughly 11-19 %); many were detected with no
  annotations at all (reading undefined variables).
- **Critique.** Public (post-release) bugs only, so bugs that types catch before commit are invisible; the 10-minute
  budget under-counts bugs needing deep annotation; TypeScript 2.0 lacked today's `strictNullChecks` by default. The
  authors treat 15 % as a lower bound.
- **Assessment: strong.** The best quantified estimate of what a gradual, unsound type system catches in a dynamic
  codebase: worthwhile but not transformative.

**Overall assessment of the typing literature: moderate.** The Hanenberg-group experiments show static types help when
programmers must *discover* or *maintain* code, and do not help, or slightly hurt, on small greenfield tasks. The
GitHub mining literature is uninformative after Berger et al. Nothing supports claims that static typing eliminates
large fractions of defects; 15 % is the number to quote.

---

## 3. Syntax, readability and novices

### Stefik, Siebert, Stefik, Slattery, "An Empirical Comparison of the Accuracy Rates of Novices using the Quorum, Perl, and Randomo Programming Languages", PLATEAU 2011

- **Method.** 18 novices with no programming experience, three groups, six tasks (loops, conditionals, functions) in
  Quorum (an "evidence-based" language), Perl, and Randomo, a placebo language whose keywords and symbols were drawn
  at random from the ASCII table; accuracy scored by artefact encoding.
- **Findings.** Language effect large and significant, F(2,15) = 7.759, p = .005. Quorum (M = .628) was significantly
  more accurate than Perl (M = .432, p = .047) and Randomo (M = .341, p = .004); **Perl was not significantly more
  accurate than the random language** (p = .458).

### Stefik and Siebert, "An Empirical Investigation into Programming Language Syntax", ACM TOCE 13(4), 2013 `[abstract-level; ACM page blocked]`

- Two studies: ratings of the intuitiveness of individual constructs by novices and programmers, and a replication of
  the accuracy study extended to Python, Ruby, Java, PHP, Smalltalk and Go `[not re-verified]`. The widely reported
  result: Java and Perl accuracy was statistically indistinguishable from Randomo for novices, while Quorum, Python
  and Ruby were significantly better; novices rated C-family tokens poorly, with `for` loops scoring lowest among
  iteration constructs.
- **Critique.** Small samples, first-contact novices only, short tasks; nothing about experienced programmers or
  readability after a year; replicated within the group, not widely outside it.
- **Assessment: moderate for novices, weak for professionals.** The defensible claim is narrow and important: C-family
  syntax conventions carry no measurable intuitive benefit for beginners; readable keywords measurably help.

---

## 4. API usability and generics

- **Stylos and Clarke, "Usability Implications of Requiring Parameters in Objects' Constructors", ICSE 2007.** 30
  professional programmers, think-aloud tasks. Programmers "strongly preferred and were more effective with APIs that
  did not require constructor parameters" (the "create-set-call" pattern); required parameters were unexpected and
  disrupted opportunistic programmers most.
- **Ellis, Stylos, Myers, "The Factory Pattern in API Design: A Usability Evaluation", ICSE 2007.** 12 participants.
  Users "require significantly more time (p = 0.005) to construct an object with a factory than with a constructor" in
  both context-sensitive and context-free tasks; the authors recommend avoiding factories where constructors or class
  clusters will do.
- **Parnin, Bird, Murphy-Hill, "Java generics adoption: how new features are introduced, championed, or ignored", MSR
  2011; "Adoption and use of Java generics", EMSE 18, 2013** `[not re-verified; publisher pages blocked]`. Mined 20
  open-source Java projects after Java 5. Widely reported findings: generics adopted slowly and unevenly; in most
  projects one or two developers "championed" the migration while others did not use the feature; use of generic
  *collections* common, *definition* of new generic classes rare; the expected reduction in casts was marginal.
- **Assessment: moderate.** Small-n lab studies from one group (CMU/Microsoft), consistent in direction and reflected
  in the .NET design guidelines. Lessons: defaults and incremental construction beat up-front required configuration;
  indirection has a measurable cost; a feature added to an existing language is taken up by champions, not by the
  median user, and promised benefits may not materialise.

---

## 5. Compiler error messages

- **Becker et al., "Compiler Error Messages Considered Unhelpful: The Landscape of Text-Based Programming Error
  Message Research", ITiCSE Working Group Report 2019** `[not re-verified]`. Systematic review of ~300 papers.
  Evidence on *enhanced* messages is mixed: Denny, Luxton-Reilly and Carpenter, "Enhancing syntax error messages
  appears ineffectual", ITiCSE 2014 (no measurable effect on novices' error rates); Becker, SIGCSE 2016 (fewer
  repeated errors); Pettit, Homer and Gee, SIGCSE 2017 (no effect). Message *design* principles (context, specificity,
  positive tone, showing the fix) are supported qualitatively; large quantitative effects are unproven.
- **Industry corroboration.** Rust's 2024 survey: only 9.1 % call unclear compiler errors a big problem, 52.4 % say
  they are not an issue; Google's 2022 internal survey found most Rust developers "come to value" the messages. Elm's
  "Compiler Errors for Humans" (2015) started the modern push and is cited by Rust's diagnostics work.
- **Assessment: moderate.** Good messages are clearly a satisfier in surveys; controlled evidence that they change
  learning outcomes is weak. Invest anyway (cheap relative to language features), but do not claim a measurable cut in
  bugs.

---

## 6. Memory safety in industry (summary; full dossier in `memory-safety.md`)

| Source | Year | Population | Finding |
|---|---|---|---|
| Microsoft (M. Miller, BlueHat IL; MSRC blog) | 2019 | CVEs Microsoft assigned 2006-2018 | ~70 % are memory-safety issues "each year" |
| Chromium security team | 2020 | 912 high/critical bugs since 2015 | ~70 % memory unsafety; half of those use-after-free |
| Mozilla (D. Hosfelt) | 2019 | 69 security bugs in Firefox's style component | 51 (73.9 %) would have been prevented by Rust |
| Google Project Zero (M. Stone) | 2022 | 58 in-the-wild 0-days in 2021 | 39 (67 %) memory corruption; 17 use-after-free |
| Android 13 blog | 2022 | Android bulletin, critical/high | 223 memory-safety vulns (76 %) in 2019 -> 85 (35-36 %) in 2022; zero in 1.5 MLOC of Rust |
| "Eliminating Memory Safety Vulnerabilities at the Source" | 2024 | same | 76 % (2019) -> 24 % (2024); Rust rollback rate "less than half that of C++" |
| "Rust in Android: move fast and fix things" | Nov 2025 | same; ~5 MLOC Rust | below 20 % in 2025; ~1000x lower vuln density; rollback rate ~4x lower; ~25 % less review time; ~4 % of Rust is `unsafe` |

Google's productivity data ("Rust fact vs. fiction", 2023: >1,000 Googlers, two-thirds confident within two months, 85
% confident their Rust is correct, only ~40 % find build speed acceptable; L. Bergstrom, Rust Nation UK 2024: C++
rewrites needed "more than 2x" less effort, Go rewrites "about the same size team") are internal, unreviewed and from
an organisation with a stated agenda, as are the vulnerability shares. The Android series is nonetheless the best
longitudinal dataset that exists, with a consistent denominator and a seven-year trend.

- **Assessment: strong for the problem** (memory-unsafe languages dominate severe vulnerabilities), **moderate for the
  solution** (that MSL adoption caused the decline), **moderate-to-weak for productivity parity**.

---

## 7. Concurrency bugs

### Lu, Park, Seo, Zhou, "Learning from Mistakes: A Comprehensive Study on Real World Concurrency Bug Characteristics", ASPLOS 2008

- 105 randomly selected real concurrency bugs (74 non-deadlock, 31 deadlock) from MySQL, Apache, Mozilla and
  OpenOffice (C/C++). Findings: 97 % of non-deadlock bugs are atomicity or order violations; 32 % are order violations
  not covered by prior tools; 96 % of bugs manifest with only two threads; 66 % of non-deadlock bugs involve a single
  variable; 97 % of deadlocks involve at most two resources; 73 % of non-deadlock bugs were *not* fixed by just adding
  a lock.
- **Assessment: strong** (pattern replicated in later studies). The bug classes are narrow (two threads, one variable,
  atomicity/order), which is what makes static prevention feasible.

### Tu, Liu, Song, Zhang, "Understanding Real-World Concurrency Bugs in Go", ASPLOS 2019

- 171 concurrency bugs from Docker, Kubernetes, etcd, CockroachDB, gRPC and BoltDB, classified by cause (shared memory
  vs message passing) and effect (blocking vs non-blocking). Observation 3: "Contrary to the common belief that
  message passing is less error-prone, more blocking bugs in our studied Go applications are caused by wrong message
  passing than by wrong shared memory protection": ~58 % of blocking bugs came from message passing (channel misuse,
  unbuffered channels, `select`), 42 % from shared-memory primitives; non-blocking bugs were dominated by
  shared-memory misuse. Shared-memory primitives are still used more often than channels (channels are 18-43 % of
  synchronisation sites). Go's built-in detectors caught few of the bugs.
- **Assessment: strong.** Directly refutes "channels make concurrency safe": message passing moves the bug class from
  races to deadlocks and leaks.

### Chabbi and Ramanathan, "A Study of Real-World Data Races in Golang", PLDI 2022

- Uber's 46-million-line Go monorepo; a dynamic detector deployed for six months found ~2,000 data races, of which
  over 1,000 were fixed. Catalogues Go-specific race patterns (loop-variable capture, named return values, `WaitGroup`
  misuse, maps).
- **Assessment: strong** (industrial scale). A GC'd, channel-oriented language with a race *detector* still
  accumulates thousands of races; only static prevention (Rust's `Send`/`Sync`, Pony's capabilities, Swift 6
  isolation) removes the class.

### Qin, Chen, Yu, Song, Zhang, "Understanding Memory and Thread Safety Practices and Issues in Real-World Rust Programs", PLDI 2020

- 850 `unsafe` usages, 70 memory-safety bugs and 100 thread-safety bugs from Servo, TiKV, Parity Ethereum, Redox, Tock
  and RustSec. "All memory-safety bugs involve unsafe code" (most also involve safe code interacting with it); all
  blocking bugs studied are in *safe* code (deadlocks and channel/lock misuse are not prevented); all non-blocking
  bugs involve interior-unsafe or unsafe code. Companion: Evans, Campbell, Soffa, ICSE 2020: fewer than 30 % of crates
  use `unsafe` directly, but more than half cannot be fully checked by the compiler because of `unsafe` in their
  dependency chain.
- **Assessment: strong.** Ownership + borrowing does what it claims for memory and data races and does not address
  deadlocks; the unsafe surface is small per crate but transitive.

---

## 8. Exceptions and error handling

- **Cabral and Marques, "Exception Handling: A Field Study in Java and .NET", ECOOP 2007** `[not re-verified;
  publisher page blocked]`. 32 applications and libraries (16 Java, 16 .NET). Widely reported: the great majority of
  handlers do nothing specific, log, or rethrow; fewer than ~10 % attempt recovery; catch blocks are typically one or
  two statements; Java's checked exceptions did not produce more meaningful handling than .NET's unchecked ones.
- **Ebert, Castor, Serebrenik, "An exploratory study on exception handling bugs in Java programs", JSS 106, 2015**
  `[not re-verified]`. 220 exception-handling bugs from Eclipse and Tomcat plus a developer survey: bugs frequently
  involve *missing* handling or swallowed exceptions; developers rate exception handling as poorly understood; the
  projects had no systematic policy.
- **Related evidence.** Hejlsberg's 2003 argument that checked exceptions degrade into `throws Exception` and empty
  catch blocks (see `designer-retrospectives.md`) matches the field data. Go's surveys show the opposite pain:
  explicit `if err != nil` is the top complaint. Android cites `Result` as a reason for fewer unhandled-error bugs
  (2021), while Cloudflare's November 2025 outage (`Result::unwrap()` on an `Err` panicked the proxy fleet) shows the
  failure mode of value-based errors.
- **Assessment: moderate.** Handlers are mostly degenerate whatever the mechanism; no study shows one mechanism
  producing measurably fewer *faults*. The design question is visibility and ergonomics, not correctness.

---

## 9. Defect density by language (Coverity and similar)

Coverity's "Open Source Report" (2012-2014) published defect densities for C/C++ and Java projects scanned by its
analyser, often quoted as Java ~2.7 vs C/C++ ~0.6 per KLOC `[unverified]`. The comparison is meaningless for language
design: the checkers differ by language, Java checkers count resource leaks and null dereferences that C checkers do
not, and projects self-select. **Assessment: weak; do not cite.**

---

## 10. Programmer productivity across languages

### Prechelt, "An empirical comparison of C, C++, Java, Perl, Python, Rexx, and Tcl", IEEE Computer 33(10), 2000

- 80 implementations of one specification ("phonecode") by 74 programmers; C/C++/Java from a controlled experiment,
  script languages from a public call. Script programs were about half as long and took about half the time to write;
  their run time was within a small factor of Java and "not much worse than C or C++" on this I/O-bound task; their
  memory use was often better than Java's; and variability *between programmers in the same language* was larger than
  the difference between language medians. Prechelt lists the script group's a-priori advantages (self-selected, more
  experienced, looser conditions).
- **Assessment: moderate**, but still the best-known controlled multi-language comparison. Robust lesson:
  inter-programmer variance dwarfs language effects, so productivity claims need large samples. Google's internal data
  (section 6) and METR (section 11) are the only recent productivity data of comparable scope.

---

## 11. LLM-assisted coding and language choice (2022-2026)

- **Cassano et al., "MultiPL-E", IEEE TSE 2023 (arXiv 2208.08227).** HumanEval and MBPP translated into 18 languages;
  Codex "matches or even exceeds its performance on Python for several other languages" (JavaScript, TypeScript, C++),
  while low-resource languages score markedly lower; pass rates correlate with language frequency in training data.
- **Cassano et al., "Knowledge Transfer from High-Resource to Low-Resource Programming Languages for Code LLMs",
  OOPSLA 2024 (arXiv 2308.09895).** Code LLMs "struggle with low-resource languages" such as OCaml, Racket, Lua, R and
  Julia; MultiPL-T (test-validated translation of Python training data) substantially improves them and is
  "significantly more efficient and effective than alternatives such as training longer". A new language starts with
  zero training data and can bootstrap a synthetic corpus if its semantics and test tooling are clear.
- **Mündler, He, Wang, Sen, Song, Vechev, "Type-Constrained Code Generation with Language Models", PLDI 2025 (arXiv
  2504.09246).** Constrains decoding with the target type system (prefix automata over typing derivations,
  instantiated for TypeScript); across models including >30B open-weight ones, compilation errors fall "by more than
  half" and functional correctness "significantly increases" on synthesis, translation and repair. A sound, decidable,
  cheap-to-check static type system is directly exploitable as a generation constraint; unsound or expensive ones are
  not.
- **GitHub Octoverse, October 2025.** TypeScript became the most-used language on GitHub by monthly contributors in
  August 2025 (~2.64 million, +66.6 % year on year), overtaking Python (~2.6 million, +48.8 %) and JavaScript (+24.8
  %); GitHub attributes this to frameworks scaffolding TypeScript by default and to "AI-assisted development that
  benefits from stricter type systems", explicitly as an observational claim.
- **Productivity.** Peng, Kalliamvakou, Cihon, Demirer (arXiv 2302.06590, 2023): one simple task (HTTP server in
  JavaScript), Copilot group 55.8 % faster; vendor-run. METR, July 2025: RCT with 16 experienced maintainers and 246
  real issues in large repositories (~22k stars, ~1 MLOC), Cursor Pro with Claude 3.5/3.7 Sonnet; developers took **19
  % longer** with AI while forecasting a 24 % speed-up and believing afterwards they had been 20 % faster; small
  sample, a February 2026 update is flagged. DORA 2024 (survey): AI adoption raises individual productivity and
  satisfaction but "negatively impacts software delivery stability and throughput".
- **Assessment: moderate and fast-moving.** Stable across 2023-2025: model quality tracks training-data share, so new
  languages pay an AI penalty that can be reduced but not removed; static types are an asset for constraining and
  validating generated code, and the market is moving toward typed languages partly for that reason; productivity
  gains are real for simple greenfield tasks and unproven or negative for expert maintenance of large codebases.
  Surveys add that developers' top AI complaint is code that is "almost right".

---

## Synthesis for Sisu

1. **"Language X causes fewer bugs" is unsupported by mining studies** (high confidence): the flagship result was
   refuted on reanalysis with a 36 % label error rate; the proposal must not cite Ray et al. without Berger et al.
   alongside.
2. **Static types catch roughly 15 % of post-release bugs in dynamic code and speed up API discovery and maintenance,
   not greenfield coding** (high): Gao et al. and the Hanenberg series agree; design the type system for maintainers
   and tooling, and advertise it honestly.
3. **C-family syntax has no measurable novice benefit; readable keywords do** (medium): Stefik's Perl-equals-random
   result is small-n but unopposed; Sisu can choose syntax for readability without fearing a learnability cost from
   departing from C.
4. **Memory-unsafe languages produce ~70 % of severe vulnerabilities and switching eliminates them in new code** (high
   for the problem, medium for productivity parity): seven years of Android data plus Microsoft, Chromium, Mozilla and
   Project Zero; the productivity claims are internal and unreviewed but point the same way.
5. **Channels do not make concurrency safe; static race prevention does** (high): Tu et al. (58 % of Go blocking bugs
   from message passing), Uber's 2,000 races, and Qin et al. (no data races outside `unsafe` in Rust). Deadlocks
   remain unaddressed by every mainstream approach.
6. **No error-handling mechanism has been shown to reduce faults; the field data show degenerate handlers under every
   regime** (medium): optimise for visibility of failure paths and cheap correct handling, and make "abort on
   unexpected error" an explicit, auditable choice (Cloudflare).
7. **Indirection costs usability: required constructor parameters, factories and heavy type-level machinery measurably
   slow programmers** (medium): small CMU studies, consistent direction; prefer defaults, incremental construction and
   discoverable APIs.
8. **A new language will have weaker AI assistance until it has a corpus; a sound, fast type checker partly
   compensates** (medium): plan for a synthetic translated corpus and for type-constrained generation from the start.
9. **Inter-programmer variance exceeds inter-language variance** (high): Prechelt 2000; every productivity claim in
   the proposal needs that caveat.

## Sources

1. B. Ray et al., FSE 2014 (web.cs.ucdavis.edu/~filkov/papers/lang_github.pdf); E. Berger et al., TOPLAS 2019
   (arxiv.org/abs/1901.10220).
2. S. Kleinschmager et al., ICPC 2012 and C. Mayer et al., OOPSLA 2012 (pleiad.cl/papers/); S. Endrikat et al., ICSE
   2014; S. Hanenberg, OOPSLA 2010.
3. Z. Gao, C. Bird, E. Barr, ICSE 2017 (earlbarr.com/publications/typestudy.pdf).
4. A. Stefik et al., PLATEAU 2011 (ecs.wgtn.ac.nz); A. Stefik, S. Siebert, TOCE 2013, doi:10.1145/2534973.
5. J. Stylos, S. Clarke, ICSE 2007; B. Ellis, J. Stylos, B. Myers, ICSE 2007 (cs.cmu.edu/~NatProg/papers/); C. Parnin,
   C. Bird, E. Murphy-Hill, MSR 2011 / EMSE 2013.
6. B. Becker et al., ITiCSE-WGR 2019; P. Denny et al., ITiCSE 2014; B. Becker, SIGCSE 2016; R. Pettit et al., SIGCSE
   2017.
7. Google security blog, 6 Apr 2021, 1 Dec 2022, 25 Sep 2024, 13 Nov 2025; Google Open Source blog, 27 Jun 2023; The
   Register, 31 Mar 2024.
8. S. Lu et al., ASPLOS 2008; T. Tu et al., ASPLOS 2019; M. Chabbi, M. K. Ramanathan, PLDI 2022 (arxiv 2204.00764); B.
   Qin et al., PLDI 2020; A. N. Evans et al., ICSE 2020 (arxiv 2007.00752).
9. B. Cabral, P. Marques, ECOOP 2007; F. Ebert, F. Castor, A. Serebrenik, JSS 2015; L. Prechelt, IEEE Computer 2000
   (page.mi.fu-berlin.de/prechelt/Biblio/).
10. F. Cassano et al., arXiv 2208.08227 and 2308.09895; N. Mündler et al., arXiv 2504.09246; GitHub Octoverse, 28 Oct
    2025; METR, 10 Jul 2025; S. Peng et al., arXiv 2302.06590; DORA 2024 report.
