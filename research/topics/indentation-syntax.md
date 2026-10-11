# Indentation-based syntax

## The question

The designer has chosen indentation-delimited blocks with Python idioms
followed where possible, so the question is no longer braces versus
indentation (Q-8, D-027) but how to do indentation *well* in a statically
typed, compiled language with no macros, explicit `return`, errors as
values and multi-line closures passed as arguments: which tokenisation,
which tab policy, how a line continues, how a trailing block attaches to a
call, what the formatter and parser can and cannot do, and what every
language that tried it later had to fix.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| INDENT/DEDENT tokens from the lexer, `:` opens a block, brackets suspend layout | Python, Nim, Mojo | Layout decided before parsing; what the reader sees is what the compiler sees; no brace-style wars | No block inside an expression, hence no multi-line lambda; generated code must track depth; a tab policy is needed | [python.md](../languages/python.md), [nim.md](../languages/nim.md), [mojo.md](../languages/mojo.md) |
| Layout rule that *inserts* `{ ; }`, explicit braces kept as fallback | Haskell, Koka, F# (verbose syntax), Elm/PureScript (no fallback) | One-liners and code generators keep working | Haskell's rule consults the parser (`parse-error(t)`), which produced the `DoAndIfThenElse` and `NondecreasingIndentation` fixes | [haskell.md](../languages/haskell.md), [koka-effekt.md](../languages/koka-effekt.md), [fsharp-sml.md](../languages/fsharp-sml.md), [elm-purescript.md](../languages/elm-purescript.md) |
| Optional braces bolted onto a brace language, `end` markers, `:` for trailing lambdas | Scala 3.0 (2021), "fewer braces" standard in 3.3 (2023) | Multi-line trailing lambdas with indentation; automatic rewrite between styles | Two dialects in one ecosystem; paste and diff complaints; the designers' 2026 style guide retreats to braces for long scopes | [scala.md](../languages/scala.md) |
| Indentation plus optional call parentheses plus implicit return | CoffeeScript | Terse | Spacing inside a line changes meaning; 82.7% "dreaded" (SO 2018) | [javascript.md](../languages/javascript.md) |
| Whitespace-sensitive data format | YAML | Readable configuration | Norway problem, base-60 numbers, templating hazards | [designer-retrospectives.md](../evidence/designer-retrospectives.md) for the genre |
| Braces or `do`/`end` chosen deliberately | Gleam, Elixir, Carbon, Kotlin, Swift, Rust, Go, Zig, Roc's new syntax | Robust to paste, generation and closures | Brace placement style wars; `goto fail` class of bugs when braces are optional | [gleam.md](../languages/gleam.md), [erlang-elixir.md](../languages/erlang-elixir.md), [carbon.md](../languages/carbon.md), [roc.md](../languages/roc.md) |

## What the evidence says

1. **Decide layout in the lexer, never with parser feedback.** Python's
   reference specifies a stack of indentation widths: "if it is larger, it
   is pushed on the stack, and one INDENT token is generated. If it is
   smaller, it *must* be one of the numbers occurring on the stack"; blank
   and comment lines are ignored, and inside brackets "the indentation of
   the continuation lines is not important" [1]
   ([python.md](../languages/python.md)). Nim reaches the same property by a different route: its lexer
   annotates each token with the preceding indentation rather than emitting
   tokens, and the parser queries that annotation through the `IND{>}`,
   `IND{=}` and `DED` grammar pseudo-terminals, so layout still never
   depends on parsing state [8] ([nim.md](../languages/nim.md)).
   Koka's specification argues the point: its layout runs "in-between
   lexing and parsing, and is independent of both. In particular, there are
   no intricate dependencies with the parser (which leads to very complex
   layout rules, as is the case in languages like Haskell or JavaScript)",
   and unlike "purely token-based layout rules (as in Scala or Go)" the
   visual indentation "corresponds directly to how the compiler interprets
   the statements" [19] ([koka-effekt.md](../languages/koka-effekt.md)).
   Haskell's `L` function has a `parse-error(t)` side condition [14], and
   its layout-inserted tokens are why `if`/`then`/`else` inside `do` needed
   `DoAndIfThenElse` (GHC 7.0.1, Haskell 2010) [15] and same-column `do`
   blocks needed `NondecreasingIndentation` (GHC 7.2.1) [16]: both are costs
   of a layout rule that inserts tokens the grammar must then tolerate
   ([haskell.md](../languages/haskell.md)).
2. **Spaces only; every language that allowed tabs later restricted them.**
   Python expands tabs to multiples of eight and raises `TabError` when the
   meaning "depend[s] on the worth of a tab in spaces" [1]; Python 2's
   optional `-tt` behaviour became the default in 3.0 (the only documentary trace is bpo-4603,
   "3.0 document tab interpretation change", 8 Dec 2008, which quotes a comp.lang.python report
   of the change and asked for `TabError` to be documented; "What's New in Python 3.0" does
   not mention it) [2]. PEP 8:
   "Spaces are the preferred indentation method" [4]. Guido's 2002 regrets:
   "tabs? require all spaces or very restricted use of tabs (not mixed)"
   [5]. Nim: "Indentation consists only of spaces; tabulators are not
   allowed" [8]. YAML 1.2.2 §6.1: "tab characters must not be used in
   indentation" [22]. Scala 3 allows mixing, compares widths as string
   prefixes and errors when they are incomparable [10]. The claim is about
   indentation-layout languages: brace languages such as Go let `gofmt` emit
   tabs because indentation there carries no meaning.
3. **Continuation rules decide whether long expressions are pleasant.**
   Python joins lines implicitly inside `()[]{}` and explicitly with a
   trailing backslash [1]; Guido's 2002 regrets include the backslash ("use
   (...) continuation") and `if expression: statement` on one line [5].
   PEP 8 switched in April 2016 (bpo-26780) to Knuth's style, breaking
   *before* binary operators [4][6]; Black does too [7]. Scala 3 lets a
   line starting with an infix
   operator continue the previous expression, and infers layout tokens only
   where newlines separate statements, never inside parentheses, patterns
   or types [10]. Koka defines "expression continuation" tokens on both
   sides: a line starting with an operator or `.`, or a previous line
   ending with an operator, open bracket or comma, continues [19].
4. **The multi-line closure is the known hole, and Python's answer is
   "don't".** Guido, February 2006: "I find any solution unacceptable that
   embeds an indentation-based block in the middle of an expression. Since
   I find alternative syntax for statement grouping (e.g. braces or
   begin/end keywords) equally unacceptable, this pretty much makes a
   multi-line lambda an unsolvable puzzle" [3]. PEP 3099: "Adding support
   for statements is a non-starter... a multi-line expression could
   suddenly exist. That is just plain ugly" [9]. The design FAQ: "Python's
   syntactic framework can't handle statements nested inside expressions",
   and notes that functions "can be declared in a local scope", so the
   standing advice is a nested `def` [11]. Guido never liked `lambda`: "crippled (only one
   expression), confusing (no argument list parentheses)" [5]. Mojo
   inherited the limit: its 1.0 lambdas are "anonymous, single-expression
   closures that desugar to a nested def", named closures carry a capture
   list such as `{mut histogram}`, escaping closures are unsupported, and
   there is no trailing-block form [20][21]
   ([mojo.md](../languages/mojo.md)). What Mojo's team has said about
   indentation itself I could not retrieve [unverified].
5. **A trailing block works when it is the last thing on the line.** Nim's
   `do` "is written after the parentheses enclosing the regular proc
   parameters" and "is appended to the routine call as the last argument":
   `sort(cities) do (x, y: string) -> int:` then an indented body; inline
   closures are `proc (x, y: string): int = ...` or `sugar`'s `=>` [8].
   Scala 3.3 recognises `:` "where a function argument is expected", so
   `xs.map: x =>` and `xs.foldLeft(0): (x, y) =>` take an indented body
   [10] ([scala.md](../languages/scala.md)); Koka's `for(1,10) fn(i)` is
   the same shape [19]. Roc, indentation-only in its alphas, now writes
   closures as `|x| { ... }`: "every Roc function has one expression after its
   arguments ... but that expression can be a block" delimited by braces
   [18] ([roc.md](../languages/roc.md)); Kotlin and Swift require braces
   ([kotlin.md](../languages/kotlin.md)). The shared invariant: the block
   begins at a line end after a complete call. The ban on blocks inside
   expressions is Python's lexer-token design, not a property of layout as
   such: F# (`xs |> List.map (fun x ->` with an indented body), Haskell
   (`\x -> do` blocks inside arguments), Elm, PureScript, Koka and Lean 4
   (typed, layout-sensitive `do` and `fun x => do` blocks inside expressions)
   all allow them with position-checking parsers. Python's own deferred
   proposals for a trailing block, PEP 403 (`@in`) and PEP 3150 (`given`),
   and Ruby's `do |x| ... end` are the direct precedents for R6 [40][41].
6. **Optional parentheses and implicit return sank CoffeeScript, not
   indentation.** Its documentation: "You don't need to use parentheses to
   invoke a function if you're passing arguments"; "The implicit call wraps
   forward to the end of the line or block expression"; functions "return
   their final value", with the warning "Be careful that you're not
   accidentally returning the results of the comprehension" [23]. Its own
   gotchas page explains why `foo +a` differs from `foo + a` and why
   `foo /a/g` may be a regex argument or a division [24]. It was the third
   most dreaded language in the 2018 Stack Overflow survey (82.7%) [25]
   ([javascript.md](../languages/javascript.md)). Nim's command syntax shows
   the same hazard in a typed language: `echo f 1, f 2` parses as
   `echo(f(1), f(2))`, zero-argument calls still need `()`, and since
   0.13.0 whitespace decides prefix versus binary (`echo $foo` is
   `echo($foo)`; `echo -1` is one token, `echo x-1` two) [8]. Explicit
   `return` (Q-9) and mandatory call parentheses remove both hazards.
7. **Soft keywords are a compatibility device, not a design goal.** PEP 634
   (September 2020, Python 3.10) makes `match` and `case` soft keywords
   "recognized as keywords when part of a match statement or case block
   only" [12], a distinction "done at the parser level, not when
   tokenizing", to preserve "compatibility with existing code that uses
   these names as identifier names" [1]. It needed the PEG parser of PEP 617
   (March 2020, 3.9) [13], and IDLE's highlighting of them "is not perfect
   and will be incorrect in some rare cases" [17].
8. **Indentation errors are a real but minor class; bracket errors top the
   other side.** Pritchard's 2015 corpus ranks "IndentationError: unindent
   does not match any outer indentation level" fifth (20,758 occurrences
   against 179,624 for "invalid syntax") [26]; in Java, Altadmri and Brown's
   37-million-compilation study found unbalanced brackets the most frequent
   novice mistake (secondary summary) [27]
   ([studies.md](../evidence/studies.md)). Python 3.10 added targeted
   messages: "expected ':'", "'{' was never closed", "IndentationError:
   expected an indented block after 'if' statement in line 2" [17]; Elm's
   Syntax Cliff work (2019) did the same for a layout language
   ([elm-purescript.md](../languages/elm-purescript.md)). Scala 3 warns when
   braces and indentation disagree even under `-no-indent` [10].
9. **Formatting is simpler and also more limited.** Black (first published
   March 2018) uses 88 columns, splits at the outermost brackets first,
   then one item per line, honours a "magic trailing comma" (19.10b0,
   reworked in 20.8b0), breaks before binary operators, and in `--safe`
   mode checks that the AST is unchanged [7]. It never changes which block
   a statement belongs to, because that is semantics; the only indentation
   it touches is inside docstrings [7]. The flip side is paste: alexn's
   2025 Scala critique is that editors keep the previous line's indentation,
   pasted blocks land at the wrong level, and scalafmt then fails [29].
10. **Scala 3 is the richest recent experiment.** Odersky, 2017: "Braces
    are a weak signal, much weaker than indentation... when brace structure
    and indentation differ, we misunderstand what was written"; objections
    were "Mandatory significant spacing makes a program brittle" and "you
    can break your code just by opening it up in a new editor" [28].
    Optional braces shipped in 3.0 (May 2021); fewer braces was
    experimental in 3.2.0, voted stable in October 2022 and standard in
    3.3.0 (May 2023, the first LTS) [10][30]. Odersky, 2022: "feedback on
    indentation was very positive so far, so there's no way we would go
    back to braces only" [31]. End markers are recommended for regions with
    blank lines, 15-20+ lines, or 4+ levels of depth [10]. In February 2026
    Odersky, Regnell and Kerr proposed a common style: "Prefer braces over
    end markers", "Use braces around a long scope that is not terminated by
    some keyword", and that the catch phrase is "use fewer braces", not "use
    no braces at all" [32] ([scala.md](../languages/scala.md)). Lessons:
    two dialects are the cost of retrofitting; long scopes need a visible
    terminator, for which Scala's designers now reach for braces.
11. **Every typed layout language except Python's family kept an
    explicit-token fallback.** Syme: F#'s light syntax "simply lets you
    leave out certain tokens such as `in` and `;;` by having the parser
    take indentation into account", released 23 September 2006 with Simon
    Marlow's help; "I made a private judgement that the presence of these
    extra tokens was a limiting factor in F# adoptability"; default in F#
    2.0 (2010) [33]. Verbose syntax "is always enabled" and "less sensitive
    to indentation" [34] ([fsharp-sml.md](../languages/fsharp-sml.md)).
    Haskell keeps explicit braces [14]; Koka requires a `--nolayout` mode
    "to facilitate code generation or source code compression" [19].
12. **The languages that chose delimiters did so for reasons Sisu has
    already answered.** Elixir's `do ... end` "blocks are equivalent to
    keyword lists", so a block is data a macro receives [35]
    ([erlang-elixir.md](../languages/erlang-elixir.md)); Sisu has no macros.
    Carbon's P0623 requires braces to avoid "two ways of doing the same
    thing" and the `goto fail` class, where "developers are known to make
    mistakes adding statements to conditionals missing braces, keeping
    consistent indentation", having adopted C++ syntax "primarily for easy
    consistency with C++"; indentation blocks are not discussed [36]
    ([carbon.md](../languages/carbon.md)); that bug class is exactly the
    brace/indentation disagreement indentation syntax removes. Gleam's FAQ
    concedes only "some syntactic similarities" with Rust; no published
    rationale for its braces was found [37] [unverified]
    ([gleam.md](../languages/gleam.md)).
13. **The empirical base is thin and cuts neither way.** Miara, Musselman,
    Navarro and Shneiderman (CACM 1983) found indentation *depth* matters,
    2-4 spaces best, blocking style not at all, in Pascal [38]. Bauer,
    Siegmund, Peitek, Hofmeister and Apel (2019; 22 participants, Java, eye
    tracking): "We did not find any indication that the indentation levels
    affect program comprehension or visual effort, so we could not
    replicate the findings of Miara and others" [39]. Stefik and Siebert
    (2013) measured keywords, not delimiters; Hanenberg's experiments
    concern typing ([studies.md](../evidence/studies.md)). No controlled
    study compares indentation blocks with braces for working programmers;
    Python's readability claim rests on adoption, on the FAQ's argument that
    with no brackets "there cannot be a disagreement between grouping
    perceived by the parser and the human reader" [11], and on Odersky's
    "weak signal" [28]. The hard evidence is about what not to *combine*
    with indentation: optional parentheses, implicit returns, tabs, a
    second dialect.

## Recommendation for Sisu

- **R1 Block introducer and tokens.** A logical line ending in `:` opens a
  statement block (the colon is kept for readability and so editors "can
  look for colons to decide when indentation needs to be increased" [11]).
  The lexer emits INDENT/DEDENT with Python's stack algorithm, only outside
  `()[]{}`, ignoring blank and comment-only lines; layout never consults
  the parser (Koka's argument, Haskell's warning).
- **R2 Tab policy.** Indentation is ASCII spaces only; a tab, NBSP or other
  whitespace in indentation is an error, with no width expansion. Any
  consistent increase is legal to the grammar; the formatter emits four.
- **R3 Dedent errors.** A dedent matching no enclosing level is an error
  naming the candidate columns; the LSP snaps to the nearest level.
- **R4 Continuation.** Implicit inside brackets; a line whose first token
  is a binary operator, `.`, `?.` or `|>` continues the previous line; a
  line ending in `,` or an open bracket continues; no backslash. The
  formatter breaks before operators and honours a magic trailing comma.
- **R5 One statement per line.** No `;`, no simple statement after `:` on
  the same line, `match` and `case` are hard keywords, call parentheses are
  mandatory, `return` is explicit (Q-9): no CoffeeScript or Nim
  command-syntax ambiguity can arise.
- **R6 Closures.** Inline closures are single expressions: `x -> x + 1`,
  `(acc, x) -> acc + x`, keeping D-026's `->` without its braces. A
  multi-statement closure is a *trailing block*: `xs.each do x:` or
  `xs.fold(0) do (acc: int, x: int):` followed by an indented body, bound
  to the call's last function-typed parameter; `do:` alone uses the
  implicit parameter (Q-25). A trailing block may appear only at the end of
  a logical line, so no indented block ever sits inside an expression
  (Guido's rule) while `let total = xs.fold(0) do acc, x:` stays legal.
  One trailing block per call; anything else is a nested `def`.
- **R7 No `end` markers in the grammar.** A default lint on bodies over
  about 40 lines and editor indent guides instead; revisit on prototype code.
  Pyret is the designed counter-example (mandatory `end`, indentation-
  insensitive, with a published rationale about paste and editor
  robustness) and should be re-read if the lint proves insufficient [42].
- **R8 Formatter.** No options; four spaces; Black's bracket-first
  splitting and trailing-comma rule; break before operators; AST-equality
  self-check; it never changes block membership. The LSP ships a
  reindent-on-paste action because the formatter cannot.
- **R9 Parser.** Hand-written recursive descent with error productions
  ("expected ':'", "expected an indented block after `if` on line N").
- **R10 No layout-off dialect.** Compile-time construction is AST-based and
  nothing emits source text but the formatter, so no `--nolayout` or
  verbose fallback; adding one later would recreate Scala's split.
- **R11 Expressions with blocks.** `if`, `match` and trailing-block closures
  may end a statement (`let x = match v:` with indented arms); a single-line
  conditional expression needs its own spelling (open).

These revise D-027 to indentation and D-026's inline form, and close Q-8.
See [proposal/02](../../proposal/02-syntax.md).

## Open questions

- Continuing an expression after a trailing block (`.sum()` after a `do`
  body): forbid, allow a leading-`.` line after the dedent, or `|>` next.
- The trailing-block keyword (`do`, `with`, bare `:`); typed parameters;
  multiple trailing blocks per call (Nim allows; Kotlin and Swift one).
- Single-line conditional expression: `a if c else b`, `if c then a else
  b`, or none.
- End markers versus a size lint, judged on prototype code; line width;
  indentation stripping in multi-line string literals.
- Q-25 implicit parameter spelling (`it`, `_`) inside `do:` blocks.

## Sources

1. Python Language Reference, "Lexical analysis" (2.1.5-2.1.8 line joining, indentation; 2.3.2 soft keywords). https://docs.python.org/3/reference/lexical_analysis.html (2026)
2. bpo-4603, "3.0 document tab interpretation change" (T. J. Reedy; fixed by G. Brandl in r67786, 15 Dec 2008). https://bugs.python.org/issue4603 (2008)
3. G. van Rossum, "Language Design Is Not Just Solving Puzzles". https://www.artima.com/weblogs/viewpost.jsp?thread=147358 (10 Feb 2006)
4. PEP 8, "Style Guide for Python Code" (created 2001; tabs vs spaces; binary operators). https://peps.python.org/pep-0008/ (2001-2016)
5. G. van Rossum, "Python Regrets", OSCON lightning talk slides. https://legacy.python.org/doc/essays/ppt/regrets/PythonRegrets.pdf (25 Jul 2002)
6. bpo-26780, "Illustrate both binary operator conventions in PEP-8" (Rhodes, van Rossum). https://bugs.python.org/issue26780 (Apr 2016)
7. Black, "The Black code style: current style" and change log (18.3a0, 19.10b0, 20.8b0). https://black.readthedocs.io/en/stable/the_black_code_style/current_style.html ; https://black.readthedocs.io/en/stable/change_log.html (2018-2026)
8. Nim Manual: Lexical analysis (indentation, number literals), Precedence (0.13.0 prefix rule), Command invocation syntax, Anonymous procedures, Do notation. https://nim-lang.org/docs/manual.html (2026)
9. PEP 3099, "Things that will Not Change in Python 3000". https://peps.python.org/pep-3099/ (Apr 2006)
10. Scala 3 Reference, "Optional Braces" (indentation rules, fewer braces since 3.3, end markers, tabs, `-no-indent`). https://docs.scala-lang.org/scala3/reference/other-new-features/indentation.html (2026)
11. Python Design and History FAQ (indentation, colons, lambda). https://docs.python.org/3/faq/design.html (2026)
12. PEP 634, "Structural Pattern Matching: Specification" (created 12 Sep 2020, Python 3.10). https://peps.python.org/pep-0634/ (2020)
13. PEP 617, "New PEG parser for CPython" (created 24 Mar 2020, Python 3.9). https://peps.python.org/pep-0617/ (2020)
14. Haskell 2010 Language Report, §10.3 "Layout". https://www.haskell.org/onlinereport/haskell2010/haskellch10.html (2010)
15. GHC User's Guide, "DoAndIfThenElse" (since 7.0.1; part of Haskell 2010). https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/doandifthenelse.html (2026)
16. GHC User's Guide, "Known bugs and infelicities" (NondecreasingIndentation). https://ghc.gitlab.haskell.org/ghc/doc/users_guide/bugs.html (2026)
17. "What's New In Python 3.10", Better error messages; IDLE soft-keyword note. https://docs.python.org/3/whatsnew/3.10.html (2021)
18. Roc, "Mini Tutorial for Roc" (new compiler; `|x| { ... }` closures, blocks). https://github.com/roc-lang/roc/blob/main/docs/mini-tutorial-new-compiler.md (2026)
19. Koka Language Book, §3.1.4 "Anonymous Functions and Trailing Lambdas", §4.3 "Layout". https://koka-lang.github.io/koka/doc/book.html (2026)
20. Mojo Manual, "Lambda expressions". https://mojolang.org/docs/manual/functions/lambda/ (2026)
21. Mojo v1.0.0 release notes (lambda desugars to nested def; closure capture lists). https://mojolang.org/releases/v1.0.0/ (2026)
22. YAML 1.2.2 specification, §6.1 "Indentation Spaces". https://yaml.org/spec/1.2.2/ (1 Oct 2021)
23. CoffeeScript documentation (significant whitespace, implicit calls, implicit returns). https://coffeescript.org/ (2026)
24. CoffeeScript wiki, "Common Gotchas". https://github.com/jashkenas/coffeescript/wiki/Common-Gotchas (2026)
25. Stack Overflow Developer Survey 2018, "Most Dreaded Languages". https://survey.stackoverflow.co/2018/ (2018)
26. D. Pritchard, "Frequency distribution of error messages", PLATEAU 2015, doi:10.1145/2846680.2846681; summary https://neverworkintheory.org/2016/06/12/error-message-frequency.html (2015)
27. A. Altadmri, N. C. C. Brown, "37 Million Compilations: Investigating Novice Programming Mistakes in Large-Scale Student Data", SIGCSE 2015. https://kar.kent.ac.uk/46742 (2015)
28. M. Odersky, "Consider syntax with significant indentation", scala/scala3 issue #2491. https://github.com/scala/scala3/issues/2491 (20 May 2017)
29. A. Nedelcu, "Scala 3 / No Indent". https://alexn.org/blog/2025/10/26/scala-3-no-indent/ (26 Oct 2025)
30. "Scala 3.3.0 released" (fewer braces experimental in 3.2.0, SIP vote Oct 2022; first LTS). https://www.scala-lang.org/blog/2023/05/30/scala-3.3.0-released.html (30 May 2023); "Scala 3 is here" https://www.scala-lang.org/blog/2021/05/14/scala3-is-here.html (14 May 2021)
31. M. Odersky, post #34 in "Make fewerBraces available outside snapshot releases", Scala Contributors. https://contributors.scala-lang.org/t/make-fewerbraces-available-outside-snapshot-releases/5024/34 (3 May 2022)
32. M. Odersky, B. Regnell, R. Kerr, "Towards a Common Scala Style Recommendation". https://contributors.scala-lang.org/t/towards-a-common-scala-style-recommendation/7383 (12 Feb 2026)
33. D. Syme, "The Early History of F#", Proc. ACM Program. Lang. 4 (HOPL), §9.10 and §21. https://fsharp.org/history/hopl-final/hopl-fsharp.pdf (2020)
34. F# Language Reference, "Verbose Syntax". https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/verbose-syntax (2016, updated 2026)
35. Elixir documentation, "Optional syntax sheet". https://hexdocs.pm/elixir/optional-syntax.html (2026)
36. Carbon proposal #623, "Require braces". https://docs.carbon-lang.dev/proposals/p000623-require-braces.html (2021)
37. Gleam, "Frequently asked questions". https://gleam.run/frequently-asked-questions/ (2026)
38. R. J. Miara, J. A. Musselman, J. A. Navarro, B. Shneiderman, "Program indentation and comprehensibility", CACM 26(11):861-867, doi:10.1145/182.358437. https://www.cs.umd.edu/~ben/papers/Miara1983Program.pdf (1983)
39. J. Bauer, J. Siegmund, N. Peitek, J. C. Hofmeister, S. Apel, "Indentation: Simply a Matter of Style or Support for Program Comprehension?", ICPC 2019 (Replications track), pp. 154-164. https://www.infosun.fim.uni-passau.de/publications/docs/Bauer19.pdf (2019)
40. PEP 403, "General purpose decorator clause (aka "in" clause)" (Deferred); PEP 3150, "Statement local namespaces (aka "given" clause)" (Deferred). https://peps.python.org/pep-0403/ ; https://peps.python.org/pep-3150/ (2011)
41. Lean 4 documentation, "do"-notation; F# Language Reference, "Lambda Expressions". https://lean-lang.org/doc/reference/latest/ ; https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/functions/lambda-expressions-the-fun-keyword (2026)
42. Pyret documentation, "Blocks" and the language rationale. https://pyret.org/docs/latest/ (2026)
