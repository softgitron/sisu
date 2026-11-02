# Programming manuals

## The question

What makes the manual of a programming language good, and how should
Sisu's be built? The question covers the manual's structure and its
audiences, its voice, the place of a runnable example in each section,
the design of exercises (small challenges with checked solutions, hints
and a difficulty curve), what reference material needs that a tutorial
does not, how a manual stays correct while the language changes under
it, accessibility and search, and how a manual becomes interactive,
first without a web playground and later with one. Givens: one `sisu`
command with an interpreter, a REPL, worksheets, a test runner whose
tests end passed, failed or blocked by a typed hole, capability fakes,
`sisu doc`, a message catalog with stable codes, and a conformance
runner that compares what a program prints byte for byte (D-029, D-061,
D-065; [proposal/10](../../proposal/10-tooling.md),
[17](../../proposal/17-testing.md),
[compiler/conformance](../../compiler/conformance/README.md)). The
readers are programmers who know one language, such as Python, and
experts who want a reference.

## Approaches found

| Manual | Form | Buys | Costs | Evidence |
|---|---|---|---|---|
| K&R, *The C Programming Language* (1978, 1988) | A tutorial chapter, a chapter per feature, a reference appendix; numbered exercises after sections | Short ("C is not a big language, and it is not well served by a big book"); every example tested from the machine-readable text; Exercise 1-1 breaks "hello, world" on purpose | No answers; the appendix "is not the standard" | [2] |
| SICP (1985, 1996) | Exercises woven into the text, a REPL throughout | Prediction from the first exercise (what does the interpreter print?) | Demanding for newcomers; Scheme-specific | [3] |
| Python: Tutorial, Library Reference, Language Reference | Three documents with stated audiences; interactive sessions; `doctest` | The tutorial "does not attempt to be comprehensive"; the reference is browsed or searched, not read | Three places to keep in step | [4] |
| Go: Tour, Effective Go, the Spec, example tests | Runnable pages with exercises checked by harness functions (`wc.Test`); an idiom guide; a versioned reference manual; `Example` functions with `// Output:` | Exercises check themselves; examples in the API docs are tests | Effective Go "is not actively updated" and says so | [5] |
| Rust: the Book, Rust by Example, Rustlings, the Reference | Concept and project chapters, marked failing listings, regenerated outputs; runnable examples; local exercises with watch mode, hints and solutions; a reference beside a separate specification | Each audience has its own book; the Book's code blocks run in CI | Only 2% of instrumented readers reach chapter 19; error outputs are regenerated per toolchain bump; a reference and a specification to keep in step | [6][7][8][9][10][11] |
| Swift: A Swift Tour, Language Guide, Language Reference | Tour, then a chapter per feature, then a reference with the grammar | Progressive: the tour gives "enough information to start writing code" | | [12] |
| Kotlin: the tour, Koans | Browser tour; koans as failing unit tests to make pass | No installation; tests judge the answer | Code is compiled on a server | [13] |
| Elixir guides | Sequential guides around `iex`; doctests in `@doc` | The shell is the first tool | Doctests "do not run in any kind of sandbox" | [14] |
| *Learn You a Haskell* | Informal, illustrated chapters | A voice readers enjoy | "Desperately needs exercises" | [15] |
| Zig: Language Reference, Ziglings | One page; every sample in the main test suite; tiny broken programs checked by their output | Samples cannot rot; browser search works on one page | One long page; no exercises in the reference | [16] |
| Julia manual | Feature chapters, "Noteworthy differences from other languages", Documenter doctests with a fix mode | A false-friend page per neighbouring language | | [17] |
| Racket Guide and Reference | A friendlier guide beside a reference that "defines"; Scribble evaluates examples at build time | Precision and approachability kept apart | Two documents to keep in step | [18] |
| *Real World OCaml* (2nd ed., 2022) | Every OCaml and shell block validated by mdx; examples split into `correct/` and `erroneous/` | A promote workflow for changed output | Tooling of its own | [19] |
| Elm guide | Short chapters, an online editor, the compiler's messages as teachers | Small and friendly | Web-specific | [20] |
| Exercism | Concept exercises in a syllabus tree; practice exercises; mentoring | One concept per exercise, with hints, an exemplar and tests | Aims at fluency in a language, not at proficiency | [21] |
| Advent of Code | A daily two-part puzzle with an answer checker | A difficulty curve and a reason to come back | Checks the answer, not the code | [22] |

## What the evidence says

1. **Separate documents for separate needs, each saying whom it
   serves.** Diátaxis sorts documentation by two questions, action or
   cognition and acquisition or application, into tutorials, how-to
   guides, reference and explanation [1]. The long-lived manuals already
   had that split: K&R's "tutorial introduction ... separate chapters on
   each major feature, and a reference manual" [2]; Python's tutorial,
   "designed for programmers that are new to the Python language, not
   beginners", beside a library reference that "you don't have to read
   ... like a novel" [4]; Racket's Guide, "friendlier (though less
   precise and less complete)" than the Reference that "defines the
   core Racket language" [18]; Swift's tour, guide and reference [12];
   Rust's Book, Rust by Example, Rustlings and Reference [6][8][9]. The
   learning research explains why one document cannot do both jobs:
   guidance that helps novices hinders experienced learners (the
   expertise reversal effect, Kalyuga et al.) [30]. And the failures of
   API documentation that Uddin and Robillard catalogued include "bloat"
   and "tangled information", the symptoms of a page written for two
   readers at once [44]. Diátaxis's own claim of "several hundred"
   adopting projects is testimony, not measurement [1].
2. **Learning happens by doing, in small steps that each work.** "The
   only way to learn a new programming language is by writing programs
   in it" opens K&R's first section [2]. Diátaxis asks a tutorial to
   "deliver visible results early and often", to "ruthlessly minimise
   explanation" and to work "for every user, every time" [1]; Exercism
   holds that "understanding comes primarily through doing, much less so
   through explanations" [21]; minimalist instruction starts learners on
   real tasks at once and includes error recognition and recovery in the
   instruction [42]. Hermans and Smit argue that programming education
   should borrow explicit direct instruction from maths and science
   instead of leaving learners to explore [36]. The two positions meet
   in guided doing: a worked example, then a completion problem, then a
   problem (finding 3).
3. **Worked examples first, then completion problems, then problems.**
   Studying worked examples beats solving problems for novices (Sweller
   and Cooper's algebra studies [28]; the review of Atkinson et al.
   [29]), and the benefit fades and reverses with expertise [30], so
   guidance should be withdrawn as the reader advances. In programming,
   van Merriënboer's completion strategy, in which students modified and
   extended existing programs, beat a strategy of writing new ones on
   program construction and had lower attrition (two groups of 28 and 29
   high-school students, ten lessons) [31]. Labelling the subgoals of a
   worked example improves performance and transfer (Margulieux, Guzdial
   and Catrambone) [32], which the ideation round already proposed for
   `sisu doc --worked` ([end-user-and-live.md](../ideation/end-user-and-live.md)).
   A typed hole is a completion problem by construction: the
   code around it is the worked part, and it type-checks before the
   learner writes anything.
4. **Reading and tracing come before writing, and prediction is the
   cheapest exercise.** Lopez et al. found that tracing and explaining
   code in plain English correlate with writing it, and proposed a
   hierarchy with knowledge of constructs at the bottom and explaining,
   Parsons puzzles and tracing in between [33]; tracing explained only a
   part of the variance in writing, so the hierarchy is a guide, not a
   law. Novices read code nearly as linearly as prose, experts much less
   (Busjahn et al.: about 70% linear eye movements for novices on code,
   60% for experts) [34], and professionals spend about 58% of their
   time comprehending programs [35]. PRIMM (predict, run, investigate,
   modify, make) was trialled in 13 schools with 493 pupils [37]; SICP's
   first exercise asks what the interpreter prints for thirteen
   expressions [3]; in the instrumented Rust Book, the questions that
   discriminated best were conceptual, asking why a program does not
   compile rather than whether it does [11]. Reading code aloud with a
   shared vocabulary is a learnable skill (code phonology) [36].
5. **Exercises carry the learning, and a manual without them is
   criticised for it.** Retrieval beats rereading at a week's delay
   (Roediger and Karpicke) [40]. K&R and SICP number their exercises
   [2][3]; Rustlings is "recommended in parallel to reading the official
   Rust book" [9]; the most widely read informal Haskell book was faulted
   by a Haskell teacher because it "desperately needs exercises" [15].
6. **Mistakes are part of the lesson, and the mistakes to teach should
   come from data.** K&R's Exercise 1-1 is "Experiment with leaving out
   parts of the program, to see what error messages you get" [2].
   Rustlings says that "most exercises contain an error that keeps them
   from compiling" [9] and Ziglings is "a series of tiny broken
   programs" [16]. The Rust Book marks listings that do not compile,
   panic or misbehave with an icon, and still warns that "if you enter
   and run a random example, it may not compile!" [6]. Which mistakes to
   teach is not something writers judge well: educators formed only a
   weak consensus on the most frequent novice mistakes and matched the
   Blackbox data of over 100,000 students only moderately, however
   experienced they were (Brown and Altadmri) [41].
7. **The exercise formats that work share four parts: a checker the
   learner runs, a fixed order, hints on demand, and a solution to
   compare with afterwards.** Rustlings reruns the current exercise on
   every save, lists exercises as done or pending, shows a hint on `h`,
   and since 6.0 (3 July 2024) has dropped its "I AM NOT DONE" comments
   and shows a solution file's path once an exercise is finished [9].
   Ziglings passes an exercise when its output, "excluding trailing
   whitespace", equals the expected text, prints the hint only on
   failure, and keeps progress in a file [16]; Ziglings is "directly
   inspired by the brilliant and fun rustlings project" [16], evidence by
   imitation. The Ruby koans stop "at the first place you need to
   correct" [23]; each Kotlin koan "is created as a failing unit test,
   and your job is to make it pass" [13]; the Go Tour's `wc.Test` "runs a
   test suite against the provided function" [5]; an Exercism concept
   exercise has an introduction, instructions, hints, an exemplar and
   tests, "ideally teaches only one concept", and unlocks later ones in
   a syllabus tree [21]; Advent of Code's puzzles "get more difficult
   over time", very generally [22]; Eloquent JavaScript folds a hint
   under each exercise [48]; Crafting Interpreters ends chapters with
   challenges that "help you learn *more* than what's in the chapter"
   [47]; HtDP's design recipe writes the examples before the function
   and turns them into its tests [46]. On hints, Marwan et al. found that
   hints with explanations improved performance at once but carried over
   to a later task without hints only when paired with prompts to
   explain them [39]. Parsons problems teach as much as fixing or writing
   code in less time [38], but they need an interface that a text file
   does not have.
8. **Readers leave early, and they leave at the hardest idea.** In the
   instrumented Rust Book, 62,526 readers answered questions 1,140,202
   times over 13 months; 2% answered one in chapter 19; most of the
   readers who tried in earnest stopped by chapter 4, ownership, after
   which drop-offs were roughly uniform; and twelve rewrites aimed at
   hard questions raised the targeted scores by 20% on average [11]. A
   manual should give a complete result before its hardest part, and it
   can measure where readers stop and repair that place, but only with
   a question per section and the readers' consent.
9. **Voice: second person, present tense, plain words, friendliness
   without jokes that do not travel.** Google asks for a voice that is
   "conversational, friendly, and respectful without using slang or
   being overly colloquial or frivolous", for "you" rather than "we", and
   against "please", culturally specific references and "simply" or
   "it's easy" in procedures [26]. Microsoft asks writers to "write like
   you speak", "be brief" and lead with the point [27]; its guide also
   sets dashes closed up, which this repository's convention of no em
   dashes overrides. This evidence is practice, class 6 in
   [research/README.md](../README.md), and it agrees across guides.
10. **Readers want examples above all, and what fails is content, not
    presentation.** In a study of over 440 Microsoft developers, some of
    the severest obstacles to learning an API were its documentation and
    learning resources, and code examples were one of five factors for
    designing them (Robillard and DeLine) [43]. In two surveys of 323
    IBM professionals, "the three severest problems were ambiguity,
    incompleteness, and incorrectness of content"; the six problems
    that respondents called blockers, which made them pick another API,
    were all problems of content, unexplained examples and obsolete
    documentation among them [44]. Adding three usage examples to an
    API's documentation reduced errors and raised success and
    satisfaction in a controlled study of 26 engineers [45]. Practice
    says the same: show the expected output and "always compile and test
    your code" [27]; K&R's examples "are complete, real programs, rather
    than isolated fragments" [2]; Write the Docs asks for exemplary
    content and says to "consider incorrect documentation to be worse
    than missing documentation" [25].
11. **Tested documentation is established, and its weak spots are error
    output, side effects and prose that nothing runs.** K&R's examples
    were "tested directly from the text" in 1978 [2]. Python's `doctest`
    checks docstrings, runs regression tests and supports "executable
    documentation", and CPython runs the documentation's doctests in CI
    [4]; rustdoc runs examples with `ignore`, `should_panic`, `no_run`
    and `compile_fail` [7]; a Go example "passes if its output matches
    its output comment" [5]; Zig compiles and tests every sample of its
    reference "as part of the main test suite" [16]; Julia's doctests
    fail the build and have a fix mode whose rewrite the docs ask writers
    to review [17]; *Real World OCaml* validates each block with mdx and
    promotes changed output [19]; Racket evaluates examples at build time
    in a sandbox [18]. Three gaps recur. The Rust Book's CI runs `mdbook
    test`, a spelling check and a link check, but the compiler output of
    its failing listings is regenerated by a script when the toolchain
    is bumped and reviewed as a diff, so a change in an error message is
    caught at the next bump rather than on the push that made it [6].
    Elixir's doctests "do not run in any kind of sandbox" and are "not
    recommended when your code examples contain side effects" [14]. And
    guidance that nothing runs goes stale: Effective Go "was written for
    Go's release in 2009 and is not actively updated" and "does not
    cover ... generics ... modules" [5].
12. **Reference material needs completeness, precision, stable addresses
    and a stated status; a tutorial needs none of them.** Diátaxis asks
    reference to "describe and only describe", to be austere, to mirror
    the structure of the product, and to be consistent, because "one
    hardly *reads* reference material; one *consults* it" [1]. Go's spec
    calls itself "the reference manual for the Go programming language"
    and carries its language version and date at the top [5]. K&R's
    appendix "is not the standard" but is "meant for easy comprehension
    by programmers" [2]. Rust adopted the FLS as a specification in
    March 2025 while "adopting the FLS does not change the status of the
    Reference" [10], so Rust has two documents with normative weight.
    Addresses matter because other documents and tools link into the
    reference: Write the Docs lists addressability as a principle [25],
    and CPython checks on every change that no HTML identifier of its
    documentation disappears [4]. Zig keeps its reference on one page "so
    you can search with your browser's search tool" [16]; mdBook builds a
    search index and a print page [7]. Julia's pages of differences from
    Python, MATLAB, R and C are a reference of false friends [17].
13. **Accessibility is a list of concrete rules.** Use a heading
    hierarchy; write link text that makes sense out of context; give
    every image alternative text; never carry information by colour,
    size or position alone; introduce a table before it; use text, never
    images of code or terminal output; avoid directional language; test
    with a screen reader [26]. WCAG 2.2 is the standard behind such rules
    [51]. Output captured from a terminal should carry no colour codes,
    which the `NO_COLOR` convention requests [50]. HTML's `<details>`
    element folds a hint away and stays keyboard-operable, and GitHub
    renders it in plain Markdown. A spoken form of each operator helps
    learners and screen-reader users alike [36].
14. **Without a playground, interactivity is local: a checker, a shell
    and a watch loop.** Rustlings, Ziglings, the koans and Exercism's
    command-line client all run on the learner's machine [9][16][21][23];
    the Go Tour runs as a local program too [5]; Kotlin's koans run
    inside an IDE plugin [13]. REPL-centred teaching is as old as SICP's
    read-eval-print loop [3] and is the core of Elixir's guides [14]
    and of Python's tutorial [4].
15. **With a playground, the cost is the sandbox, and WebAssembly moves
    it to the reader's browser.** The Go Tour and the Kotlin playground
    compile on a server [5][13]; mdBook's run button sends Rust code to
    the Rust playground [7]; Eloquent JavaScript runs its examples in the
    page [48]; Pyodide is "a port of CPython to WebAssembly/Emscripten"
    for the browser [53]. The instrumented Rust Book kept highlights in a
    cookie, and mdbook-quiz can cache answers in the browser's storage
    [11].
16. **Progressive disclosure orders a manual, and a teaching subset can
    enforce the order.** Progressive disclosure "defers advanced or
    rarely used features to a secondary screen", making software "easier
    to learn and less error-prone" [49]; Swift's vision for approachable
    concurrency names three phases on "the progressive disclosure path"
    (sequential code, asynchronous code without data-race errors, then
    parallelism) [12]; HtDP starts from "Beginning Student Language" and
    climbs to richer dialects [46]; Python's tutorial is incomplete on
    purpose [4].
17. **A manual outlives its first readers.** Go's spec names the
    language version it describes [5]; the Rust Book pins its toolchain
    and regenerates its outputs on each bump [6]; Python's translations
    are gettext catalogues hosted on python.org (PEP 545) [52]. A guide
    that nothing checks needs a note saying it is old [5].

## Recommendation for Sisu

The plan is [manual/README.md](../../manual/README.md) and its style
guide [manual/STYLE.md](../../manual/STYLE.md); the first chapter,
[manual/tutorial/01-hello.md](../../manual/tutorial/01-hello.md), is the
model for the rest. What Sisu should take:

- **Four documents and two trees** (findings 1, 12). A tutorial for
  programmers who know one language, how-to guides, a reference for
  experts and explanations, as `manual/tutorial/`, `guides/`,
  `reference/` and `explanations/`, beside `examples/`, `exercises/` and
  `solutions/`. The standard library's reference is generated by `sisu
  doc --std`, whose doc examples `sisu test --std` already runs; the
  manual links to it rather than retyping it.
- **A first complete program before ownership in depth** (finding 8).
  Part I of the tutorial ends with a working command-line tool, and
  ownership has a part of its own, the part to test hardest with
  readers. This follows the learnability study's first-week path
  ([learnability.md](learnability.md)) and is close to the order
  proposed for the teaching ladder (D-073), whose levels the Q-41 trial
  settles; how the two relate is Q-64.
- **One concept per section, each with a runnable, checked example and a
  prediction prompt** (findings 3, 4, 10). Errors are shown as Sisu
  prints them and named by code (E0200) and catalog identifier
  (`resolve.undefined-name`), and the mistakes taught come from the
  wrong-program corpus and, later, from the playground's logs, not from
  the writers' guesses (finding 6).
- **Exercises built on typed holes** (findings 3, 5, 7). An exercise is a
  file with `todo(...)` holes and `test` blocks; its three states, blocked,
  failed and passed, are what `sisu test` already reports. Kinds follow
  the reading-before-writing order: predict, fix, complete, write, and a
  multi-part challenge at the end of each part. Levels 1 to 3. The
  solution has the same tests, byte for byte; hints are folded in the
  chapter; solutions live in a separate tree, to compare with after the
  tests pass.
- **Every block checked on every push** (findings 10, 11). Each fenced
  block in a chapter names the file it shows; the conformance runner
  checks the examples and solutions as cases, and the manual's checker
  checks the pages, the terminal sessions and the exercises. Blessing
  rewrites the expectations and is reviewed as a diff. Examples that
  touch files or the clock run against capability fakes, which is the
  sandbox Elixir's doctests lack.
- **A reference that is complete, austere and addressable** (finding
  12): a page per part of the language, every diagnostic with a checked
  wrong example, the false-friend table of D-062, the grammar, the
  `sisu` command, and a status for each feature (in stage 0, designed,
  leaning, prototype). Whether it is the specification is Q-62.
- **Accessibility rules in the style guide** (finding 13): heading
  hierarchy, descriptive links, no meaning carried by colour or
  position, no images of code or output, output captured without colour,
  a spoken form for each operator.
- **Interactivity in three steps** (findings 14, 15): now, `sisu test
  FILE`, worksheets (`sisu ws`) and the REPL; next, a `sisu learn`
  command that runs a chapter's exercises and reports which are solved
  (Q-63); later, the playground of
  [proposal/18](../../proposal/18-next-steps.md), the interpreter in
  WebAssembly over the same files, with fakes as the sandbox and opt-in
  measurement of where readers stop (Q-68).
- **The voice of the style guides** (finding 9): second person, present
  tense, short sentences, plain words, no "simply", no em dashes.

## Open questions

- Q-62 whether the reference is the specification, a companion to it,
  or generated from it.
- Q-63 where the exercise runner lives, and whether the toolchain
  bundles the exercises.
- Q-64 how the tutorial's order and the teaching ladder of D-073 relate,
  and whether exercise files compile under it.
- Q-65 how hidden solutions and hidden tests are, and when the runner
  reveals a solution.
- Q-66 whether `sisu explain`'s long text is written in the manual or in
  the catalog.
- Q-67 which version a page describes, and how it shows features that
  are designed but not implemented.
- Q-68 whether learners' progress and stuck points are measured, with
  what consent, and how that meets the experiment's rules.
- Q-69 translation: English only until 1.0, or translatable from the
  start.
- Q-70 whether the manual's examples join the conformance suite.

## Sources

1. D. Procida, *Diátaxis*: start page https://diataxis.fr/ ; compass https://diataxis.fr/compass/ ; tutorials https://diataxis.fr/tutorials/ ; how-to guides https://diataxis.fr/how-to-guides/ ; reference https://diataxis.fr/reference/ ; explanation https://diataxis.fr/explanation/ ; colophon https://diataxis.fr/colophon/ (checked November 2026)
2. B. W. Kernighan, D. M. Ritchie, *The C Programming Language*, 2nd ed., Prentice Hall (1988): prefaces to both editions, §1.1 "Getting Started", Exercise 1-1, Appendix A (checked in a scanned copy, November 2026)
3. H. Abelson, G. J. Sussman, *Structure and Interpretation of Computer Programs*, 2nd ed., MIT Press (1996): preface to the first edition https://mitp-content-server.mit.edu/books/content/sectbyfn/books_pres_0/6515/sicp.zip/full-text/book/book-Z-H-7.html ; §1.1 and Exercise 1.1 https://mitp-content-server.mit.edu/books/content/sectbyfn/books_pres_0/6515/sicp.zip/full-text/book/book-Z-H-10.html
4. Python documentation: The Python Tutorial https://docs.python.org/3/tutorial/index.html ; The Python Standard Library https://docs.python.org/3/library/index.html and its introduction https://docs.python.org/3/library/intro.html ; `doctest` https://docs.python.org/3/library/doctest.html ; CPython's documentation CI, jobs "Doctest" and "Check for removed HTML IDs" https://github.com/python/cpython/blob/main/.github/workflows/reusable-docs.yml , https://github.com/python/cpython/blob/main/.github/workflows/reusable-check-html-ids.yml (2026)
5. Go: A Tour of Go, sources `welcome.article` and `moretypes.article` https://github.com/golang/website/tree/master/_content/tour (2026); Effective Go https://go.dev/doc/effective_go (2026); The Go Programming Language Specification, language version go1.27 (26 May 2026) https://go.dev/ref/spec ; A. Gerrand, "Testable Examples in Go", The Go Blog https://go.dev/blog/examples (7 May 2015)
6. *The Rust Programming Language*: Introduction https://doc.rust-lang.org/book/ch00-00-introduction.html ; ADMIN_TASKS.md https://github.com/rust-lang/book/blob/main/ADMIN_TASKS.md ; CI workflow https://github.com/rust-lang/book/blob/main/.github/workflows/main.yml (2026)
7. mdBook: `mdbook test` https://rust-lang.github.io/mdBook/cli/test.html ; HTML renderer options (search, print, playground) https://rust-lang.github.io/mdBook/format/configuration/renderers.html ; The rustdoc book, "Documentation tests" https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html (2026)
8. Rust by Example https://doc.rust-lang.org/rust-by-example/ (2026)
9. Rustlings: README https://github.com/rust-lang/rustlings ; usage https://rustlings.rust-lang.org/usage/ ; CHANGELOG, 6.0.0 (3 July 2024) https://github.com/rust-lang/rustlings/blob/main/CHANGELOG.md
10. The Rust Project, "Adopting the FLS", Rust Blog https://blog.rust-lang.org/2025/03/26/adopting-the-fls/ (26 March 2025)
11. W. Crichton, S. Krishnamurthi, "Profiling Programming Language Learning", OOPSLA 2024 https://arxiv.org/abs/2401.01257 ; the experimental book https://rust-book.cs.brown.edu/experiment-intro.html ; mdbook-quiz https://github.com/cognitive-engineering-lab/mdbook-quiz (2024-2026)
12. *The Swift Programming Language*, "A Swift Tour", source https://github.com/swiftlang/swift-book (TSPL.docc/GuidedTour/GuidedTour.md, 2026); Swift vision "Improving the approachability of data-race safety" https://github.com/swiftlang/swift-evolution/blob/main/visions/approachable-concurrency.md (2025)
13. Kotlin: Koans https://kotlinlang.org/docs/koans.html ; the tour https://kotlinlang.org/docs/kotlin-tour-welcome.html ; Kotlin Playground https://github.com/JetBrains/kotlin-playground (2026)
14. Elixir: Getting started, "Introduction" https://elixir.hexdocs.pm/introduction.html ; ExUnit.DocTest https://ex-unit.hexdocs.pm/ExUnit.DocTest.html (2026)
15. M. Lipovača, *Learn You a Haskell for Great Good!*, No Starch Press (2011), http://learnyouahaskell.com/ ; C. Allen, "Functional Education" https://bitemyapp.com/blog/functional-education/ (31 December 2014)
16. Zig Language Reference (master, 0.17 development) https://ziglang.org/documentation/master/ ; Ziglings README https://codeberg.org/ziglings/exercises and checker `rivendell/elrond.zig` (2026)
17. Julia manual, "Noteworthy Differences from other Languages" https://docs.julialang.org/en/v1/manual/noteworthy-differences/ ; Documenter.jl, "Doctests" https://documenter.juliadocs.org/stable/man/doctests/ (2026)
18. *The Racket Reference* https://docs.racket-lang.org/reference/index.html ; Scribble, "Evaluation and Examples" https://docs.racket-lang.org/scribble/eval.html (2026)
19. Y. Minsky, A. Madhavapeddy, *Real World OCaml*, 2nd ed., Cambridge University Press (October 2022) https://dev.realworldocaml.org/ ; the book's README https://github.com/realworldocaml/book ; mdx https://github.com/realworldocaml/mdx (2026)
20. An Introduction to Elm https://guide.elm-lang.org/ (2026)
21. Exercism: "Concept Exercises" https://exercism.org/docs/building/tracks/concept-exercises ; "Syllabus" https://exercism.org/docs/building/tracks/syllabus ; "Fluency" https://exercism.org/docs/using/product/fluency (2026)
22. E. Wastl, Advent of Code, "About" https://adventofcode.com/2025/about (2025); "Advent of Code: Behind the Scenes", FOSDEM https://archive.fosdem.org/2021/schedule/event/adventofcode/ (2021)
23. EdgeCase, Ruby Koans http://rubykoans.com/ (2009, checked 2026)
24. M. McGranaghan, E. Bendersky, Go by Example https://gobyexample.com/ (2026)
25. Write the Docs: "Documentation principles" https://www.writethedocs.org/guide/writing/docs-principles/ ; "Docs as Code" https://www.writethedocs.org/guide/docs-as-code/ (2026)
26. Google developer documentation style guide: "Voice and tone" https://developers.google.com/style/tone ; "Second person" https://developers.google.com/style/person ; "Code samples" https://developers.google.com/style/code-samples ; "Write accessible documentation" https://developers.google.com/style/accessibility (2026)
27. Microsoft Writing Style Guide: "Top 10 tips for Microsoft style and voice" https://learn.microsoft.com/en-us/style-guide/top-10-tips-style-voice (2 July 2026); "Code examples" https://learn.microsoft.com/en-us/style-guide/developer-content/code-examples (2019, checked 2026)
28. J. Sweller, G. A. Cooper, "The use of worked examples as a substitute for problem solving in learning algebra", *Cognition and Instruction* 2(1), 59-89 (1985)
29. R. K. Atkinson, S. J. Derry, A. Renkl, D. Wortham, "Learning from examples: Instructional principles from the worked examples research", *Review of Educational Research* 70(2), 181-214 (2000)
30. S. Kalyuga, P. Ayres, P. Chandler, J. Sweller, "The expertise reversal effect", *Educational Psychologist* 38(1), 23-31 (2003)
31. J. J. G. van Merriënboer, "Strategies for programming instruction in high school: program completion vs. program generation", *Journal of Educational Computing Research* 6(3), 265-285 (1990), abstract https://research.utwente.nl/en/publications/strategies-for-programming-instruction-in-high-school-program-com/
32. L. E. Margulieux, M. Guzdial, R. Catrambone, "Subgoal-labeled instructional material improves performance and transfer in learning to develop mobile applications", ICER 2012
33. M. Lopez, J. Whalley, P. Robbins, R. Lister, "Relationships between reading, tracing and writing skills in introductory programming", ICER 2008, https://doi.org/10.1145/1404520.1404531
34. T. Busjahn, R. Bednarik, A. Begel, M. Crosby, J. H. Paterson, C. Schulte, B. Sharif, S. Tamm, "Eye Movements in Code Reading: Relaxing the Linear Order", ICPC 2015, 255-265
35. X. Xia, L. Bao, D. Lo, Z. Xing, A. E. Hassan, S. Li, "Measuring Program Comprehension: A Large-Scale Field Study with Professionals", *IEEE TSE* 44(10) (2018)
36. F. Hermans, *The Programmer's Brain*, Manning (2021); F. Hermans, M. Smit, "Explicit Direct Instruction in Programming Education", PPIG 2018 https://www.cl.cam.ac.uk/~mcm79/ppig/files/2018-PPIG-29th-hermans.pdf ; F. Hermans, A. Swidan, E. Aivaloglou, "Code Phonology: An Exploration into the Vocalization of Code", ICPC 2018, https://doi.org/10.1145/3196321.3196355
37. S. Sentance, J. Waite, M. Kallia, "Teaching computer programming with PRIMM: a sociocultural perspective", *Computer Science Education* 29(2-3), 136-176 (2019), https://doi.org/10.1080/08993408.2019.1608781
38. B. J. Ericson, L. E. Margulieux, J. Rick, "Solving Parsons problems versus fixing and writing code", Koli Calling 2017, 20-29
39. S. Marwan, J. J. Williams, T. Price, "An Evaluation of the Impact of Automated Programming Hints on Performance and Learning", ICER 2019, https://doi.org/10.1145/3291279.3339420
40. H. L. Roediger, J. D. Karpicke, "Test-Enhanced Learning: Taking Memory Tests Improves Long-Term Retention", *Psychological Science* 17(3), 249-255 (2006)
41. N. C. C. Brown, A. Altadmri, "Investigating novice programming mistakes: educator beliefs vs. student data", ICER 2014, https://doi.org/10.1145/2632320.2632343 ; extended as "Novice Java Programming Mistakes: Large-Scale Data vs. Educator Beliefs", *ACM TOCE* (2017)
42. H. van der Meij, J. M. Carroll, "Principles and heuristics for designing minimalist instruction", *Technical Communication* 42(2), 243-261 (1995); J. M. Carroll, *The Nurnberg Funnel*, MIT Press (1990)
43. M. P. Robillard, R. DeLine, "A field study of API learning obstacles", *Empirical Software Engineering* 16, 703-732 (2011), https://www.cs.mcgill.ca/~martin/papers/ese2011.pdf ; M. P. Robillard, "What Makes APIs Hard to Learn? Answers from Developers", *IEEE Software* 26(6) (2009)
44. G. Uddin, M. P. Robillard, "How API Documentation Fails", *IEEE Software* 32(4) (2015), https://www.cs.mcgill.ca/~martin/papers/ieeesw2015.pdf
45. S. M. Sohan, F. Maurer, C. Anslow, M. P. Robillard, "A study of the effectiveness of usage examples in REST API documentation", VL/HCC 2017, 53-61, https://www.cs.mcgill.ca/~martin/papers/vlhcc2017.pdf
46. M. Felleisen, R. B. Findler, M. Flatt, S. Krishnamurthi, *How to Design Programs*, 2nd ed., preface https://htdp.org/2024-11-6/Book/part_preface.html (2024)
47. R. Nystrom, *Crafting Interpreters*, "Introduction" https://craftinginterpreters.com/introduction.html (2021)
48. M. Haverbeke, *Eloquent JavaScript*, 4th ed.: "Introduction" https://eloquentjavascript.net/00_intro.html ; "Program Structure", exercises https://eloquentjavascript.net/02_program_structure.html (2024)
49. J. Nielsen, "Progressive Disclosure", Nielsen Norman Group https://www.nngroup.com/articles/progressive-disclosure/ (3 December 2006)
50. NO_COLOR https://no-color.org/ (2017)
51. W3C, Web Content Accessibility Guidelines (WCAG) 2.2 https://www.w3.org/TR/WCAG22/ (Recommendation, 5 October 2023)
52. J. Palard et al., PEP 545, "Python Documentation Translations" https://peps.python.org/pep-0545/ (2017)
53. Pyodide https://pyodide.org/en/stable/ (2026)
