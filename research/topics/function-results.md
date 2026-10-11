# How functions return values

## The question

How a function hands its result to its caller: an explicit `return`
statement, the value of the last expression, or a result variable named in
the signature (MATLAB, Go, Nim, Pascal); whether several results are a
tuple or a special multi-value form; and whether `if`/`match` can have a
value when blocks do not. The decisions already taken (indentation, Python
idioms, explicit `return`, `?`, contracts) fix the frame; this document
checks them against the evidence and settles the MATLAB-style named-output
question.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Explicit `return` only | Python, Java, C, Go (unnamed results), Kotlin block bodies, Swift < 5.1, C# < 6, Zig, Carbon, Odin | Exit points visible; no statement/expression ambiguity; `return` plus newline is unambiguous in indentation syntax | One-liner boilerplate: every one of these languages later added single-expression sugar; Python falls off the end to `None` (mypy `[return]`, PEP 8, pylint R1710) | [python.md](../languages/python.md), [swift.md](../languages/swift.md), [csharp.md](../languages/csharp.md), [kotlin.md](../languages/kotlin.md), [zig.md](../languages/zig.md) |
| Implicit last expression | Rust, Ruby, Scala, Haskell, OCaml, Lisp, Kotlin expression bodies, Gleam, Elm, Roc, Nim blocks | No keyword; blocks, `if` and `match` compose as values | Rust's semicolon trap (E0308 "expected `i32`, found `()`"); a lint for each side of the style war (clippy `needless_return` vs `implicit_return`); Ruby's "avoid `return`"; CoffeeScript's accidental returns | [rust.md](../languages/rust.md), [ruby.md](../languages/ruby.md), [gleam.md](../languages/gleam.md), [roc.md](../languages/roc.md), [indentation-syntax.md](indentation-syntax.md) |
| Named result variable(s) | MATLAB/Octave output arguments, Go named results + naked `return`, Nim `result`, Pascal/Delphi `Result`, Fortran name or `RESULT()`, VB function name, Eiffel `Result`, Odin, Mojo `out`, Carbon `returned var` | Self-documenting signatures; `defer` can modify the result; contracts can name it; in-place construction (NRVO) | Default-initialised result returns zero/nil silently (Nim RFC 378, Go zero values); shadowing (Go spec restriction, Nim `ResultShadowed`); naked returns restricted by every Go style guide; MATLAB checks only at run time; name-as-result ambiguous with recursion (Pascal, Fortran) | [go.md](../languages/go.md), [nim.md](../languages/nim.md), [fortran.md](../languages/fortran.md), [odin-jai-c3.md](../languages/odin-jai-c3.md), [mojo.md](../languages/mojo.md), [carbon.md](../languages/carbon.md) |
| Named result in signature, not a variable | Dafny `returns (r: int)`, Whiley `-> (int r)`, SPARK `F'Result`, D `out (r)`, C++26 `post (r : ...)` | Contracts and docs get a name; no mutable state, no naked return | Two spellings (declared name vs reserved `result`) | [ada-spark.md](../languages/ada-spark.md), [d.md](../languages/d.md), [cpp.md](../languages/cpp.md), [contracts-and-verification.md](contracts-and-verification.md) |
| Multiple values, not first-class | Go, Lua | Cheap; no tuple type needed | Cannot be stored or passed on; Go needs a spec special case for `f(g())`; Lua's result-adjustment rules | [go.md](../languages/go.md), [lua.md](../languages/lua.md) |
| Tuples, optionally labelled | Python, Swift, Rust, Kotlin (`Pair`), C# 7 | First-class, destructurable, generic; Swift labels document each element | Positional access `.0` when unlabelled | [python.md](../languages/python.md), [swift.md](../languages/swift.md) |
| `if`/`match` as expressions | Rust, Kotlin, Scala, Nim, Haskell, OCaml, Swift 5.9 (single-expression branches), Java 14 `switch`, C# 8 | Removes the ternary and the `let x; if ... x = ...` dance | Swift: "usability cliff" when a branch needs a statement; Python kept `match` a statement (PEP 635); Go refuses even `?:` | [swift.md](../languages/swift.md), [java.md](../languages/java.md), [python.md](../languages/python.md), [nim.md](../languages/nim.md), [go.md](../languages/go.md) |

## What the evidence says

1. **The explicit-return languages with a large application-developer
   base (Kotlin, Swift, C#) added single-expression sugar; the systems ones
   (Go, C, Zig, Odin, Carbon) have not, and none has gone the other way.** C# 6 (2015) expression-bodied members,
   extended in 7.0 [21]; Kotlin's `fun foo() = 1`, preferred by its
   conventions "for functions with the body consisting of a single
   expression" [16]; Swift SE-0255 (5.1, 2019), whose motivation counts that
   of 19 characters in a `sum()` body "7 are consumed by the `return`
   keyword" [17]. SE-0255 is also the clearest statement of why *full*
   implicit-last-expression was rejected by a statement-oriented language:
   since Swift's conditionals are statements, "doing so would lead to
   asymmetry in code resulting from the fact that Swift is not
   expression-oriented as Rust is", with the proposal's own example noting
   that "only one of the two returns from the function uses the return
   keyword" [17]
   ([swift.md](../languages/swift.md), [csharp.md](../languages/csharp.md),
   [kotlin.md](../languages/kotlin.md)).
2. **Statement-only languages also add `if`/`switch` expressions later, and
   the shipped form is "single expression per branch".** Swift SE-0380 (5.9,
   2023): "Each branch of the `if`, or each `case` of the `switch`, must be a
   single expression", calling that rule "an unfortunate usability cliff"
   and leaving multi-statement branches for later [18]. Java JEP 361
   (JDK 14, 2020) added `switch` expressions with a `yield` statement for
   block arms [20]; C# 8 (2019) added `switch` expressions [21]. Python took
   from 2003 to 2005 and a 518-ballot vote to get `X if C else Y` (PEP 308)
   [23], and PEP 635 (3.10) kept `match` a statement because an expression
   form "would fit poorly with Python's statement-oriented nature" and case
   clauses "could no longer have arbitrary blocks of code attached" [24].
   Go's FAQ refuses the ternary: "A language needs only one conditional
   control flow construct" [7]. Nim is the existence proof that `if`/`case`
   expressions work in indentation syntax: "An if expression always results
   in a value, so the else part is required", and a `case` expression branch
   may hold statements followed by the value line [8]
   ([python.md](../languages/python.md), [swift.md](../languages/swift.md),
   [java.md](../languages/java.md), [go.md](../languages/go.md),
   [nim.md](../languages/nim.md)).
3. **The cost of implicit last-expression values is the statement/expression
   boundary, not the missing keyword.** The Rust Book's own example
   `fn plus_one(x: i32) -> i32 { x + 1; }` fails with "expected `i32`,
   found `()`" and rustc grew a dedicated hint, "help: remove this semicolon
   to return this value" [10]. Clippy ships `needless_return` in the
   default-on `style` group (pre-1.29) and `implicit_return` in the opt-in
   `restriction` group (1.33): a lint for each side, and forum threads from
   2016 and 2020 rehearse the split [11]. Ruby's style guide: "Avoid
   `return` where not required for flow of control" [15]. Gleam has no
   `return` at all and admits the cost, "excessive indentation", which its
   `use` expression exists to relieve [39]; Haskell's `return`, which is not
   a return, is a documented beginner confusion [40]. Roc,
   which has both block values and `return`, merged a warning plus formatter
   removal of a redundant trailing `return` in October 2026 while keeping
   early returns [33]. The ecosystems converge on one convention: early
   exits explicit, final value implicit. No controlled study of error rates
   was found; this is style-guide and diagnostic evidence ([rust.md](../languages/rust.md), [ruby.md](../languages/ruby.md),
   [gleam.md](../languages/gleam.md), [haskell.md](../languages/haskell.md),
   [roc.md](../languages/roc.md)).
4. **The bug class that actually ships around `return` is non-local return
   from closures**, from Smalltalk's `^` in blocks and Ruby's proc-versus-
   lambda `return` (`LocalJumpError`) onward. Kotlin: "A bare `return` is forbidden inside a lambda
   because a lambda cannot make the enclosing function `return`", unless the
   receiver is `inline`, in which case it *does* return from the enclosing
   function, and `return@forEach` labels exist to choose [16]. Scala
   implemented non-local return with exceptions, deprecated the language
   feature in 3.2.0, and in 3.3.0 (2023) added `boundary`/`break` and
   deprecated the `NonLocalReturns` library API in its favour [19]. Rust's `?` desugars to a jump to the
   innermost enclosing function (`'fn`), so inside a closure it returns from
   the closure, and E0277 fires when that closure does not return `Result`
   [12][10]. SE-0380's motivation is the same: returns inside an
   immediately-executed closure "require extra cognitive load to understand
   they are returning from a closure, not the outer function" [18]
   ([kotlin.md](../languages/kotlin.md), [scala.md](../languages/scala.md),
   [rust.md](../languages/rust.md)).
5. **Named results: the documentation benefit is accepted by every Go style
   guide; the naked return is the part they all restrict.** Effective Go:
   "they're documentation" [1]. Code Review Comments: "Naked returns are
   okay if the function is a handful of lines. Once it's a medium sized
   function, be explicit" and "it's not worth it to name result parameters
   just because it enables you to use naked returns" [3]. Google's style
   decisions repeat both [4]; golangci-lint offers `nakedret` and
   `nonamedreturns` [5]. Uber's guide has no naked-return rule at all [6].
   The proposal to remove bare returns (golang/go #21291, 2017) was closed in
   2023 with "I don't see people reporting bugs due to bare returns" [5]. Odin copied
   named results and the same warning: naked returns "should only be used in
   short procedures as it reduces clarity when reading" [32]
   ([go.md](../languages/go.md), [odin-jai-c3.md](../languages/odin-jai-c3.md)).
6. **Named result + default initialisation + shadowing is the documented
   footgun, and the fixes shipped are the evidence.** Go spec: "all the
   result values are initialized to the zero values"; its implementation
   restriction lets a compiler reject a naked `return` when a result is
   shadowed, and the spec's own example is `err` shadowed by `:=` [2]. The
   `shadow` analyzer "generates too many false positives and is not yet
   enabled by default" [5]. Nim: `result` "is initialized to (binary) zero"
   [8]; the compiler has `ResultShadowed` ("Special variable 'result' is
   shadowed.") and `ResultUsed` warnings; Nim's designer wrote in RFC 378
   (2021) that "I think it's the most frequent source of nil bugs", and Nim 2.0 (2023) shipped `strictDefs`, still opt-in
   (`{.experimental: "strictDefs".}`), with "We found Nim's default
   initialization rule to be one major source of bugs" [9]. Eiffel
   16.05 changed `Result` and locals to be "treated as if they were
   detachable" and made `Result` "required to be set" at routine end for
   types that need initialisation [30]. MATLAB never checks statically:
   `return` takes no value, and "Output argument ‹variable› (and maybe
   others) not assigned during call to ‹function›" is raised at run time,
   and, according to MathWorks community answers rather than the reference pages, only for outputs the caller requested (`nargout`) [27]
   ([go.md](../languages/go.md), [nim.md](../languages/nim.md)).
7. **The function-name-as-result model was abandoned by each of its own
   lineages.** Free Pascal keeps name assignment but adds `Result` "only in
   ObjFPC or Delphi mode" and `Exit(12)`; Fortran 90 added `RESULT(name)`
   because with it "all references to the function name are recursive
   calls"; VB.NET added `Return expression`, demoting name assignment to
   "the alternative" [28]. Three independent fixes to the same design
   ([fortran.md](../languages/fortran.md)).
8. **Where named results earn their keep: `defer` and in-place
   construction.** Effective Go's `Compile` sets `regexp = nil` and
   `err = e.(Error)` inside a deferred `recover`: "deferred functions can
   modify named return values" [1]. Mojo 24.5 added named result bindings
   "for directly emplacing function results into the output slot" and 24.6
   respelled them as `out` arguments, at most one per function; Carbon has
   `returned var c: Circle; ... return var;` explicitly "instead of the
   'named return value optimization' of C++" [31]. Nim's `result` is its
   NRVO mechanism [8]. Zig gets the same effect from result location
   semantics with no surface syntax at all [32]. For Sisu, with no GC and
   value semantics, the in-place need is real, but Zig shows it needs no
   name in the source ([mojo.md](../languages/mojo.md),
   [carbon.md](../languages/carbon.md), [zig.md](../languages/zig.md),
   [nim.md](../languages/nim.md)).
9. **Contracts need a name for the result; there are two working
   spellings.** A reserved identifier: SPARK "attribute Result refers to the
   value returned by the function" (`Post => Increment'Result = X + 1`) [29];
   D `out (r; r > 0, "...")`, where "the optional identifier ... is set to
   the return value" and is implicitly const, and C++26 P2900R14
   `post(r : r == x && r != 2)` [34]; Rust's nightly `ensures` takes a
   closure over a reference to the return value [13]. Or a name in the
   signature: Dafny `method Search(ll: Node?) returns (r: int) ensures ll ==
   null ==> r == 0` and `function c2(): (r: int) ensures r > 0`; Whiley
   `function max(int x, int y) -> (int r) ensures (r >= x) && (r >= y):`
   with an indented body and explicit `return x` [35]. Whiley is the
   existence proof of the designer's MATLAB-flavoured idea done safely: the
   name lives in the signature, is not a mutable local, and creates no
   naked return. SPARK points the other way on out-parameters: a function
   with `out` needs the `Side_Effects` aspect and "can be called only as the
   right-hand side of an assignment, or as part of a list of statements"
   [29]; Ada 2012 allowed all modes on
   functions (AI05-0143) and at once needed order-dependence rules
   (AI05-0144) [36] [unverified: the Rationale chapter could not be read from
   a primary source during checking]
   ([ada-spark.md](../languages/ada-spark.md), [d.md](../languages/d.md),
   [cpp.md](../languages/cpp.md), [rust.md](../languages/rust.md)).
10. **Multiple results: first-class tuples with labels won.** Go's results
    cannot be stored, so the spec carries a special case for `f(g())` [2];
    Lua "always adjusts the number of results from a function to the
    circumstances of the call" and "a function call that is not the last
    element in the list always produces one result", a standing trap [37].
    Swift's `-> (min: Int, max: Int)`: "Because the tuple's member values
    are named as part of the function's return type, they can be accessed
    with dot syntax" [22]; C# 7 (2017) copied labelled tuples and
    deconstruction [21]. Labels give MATLAB's self-documenting outputs
    without a variable ([go.md](../languages/go.md),
    [lua.md](../languages/lua.md), [swift.md](../languages/swift.md),
    [csharp.md](../languages/csharp.md)).
11. **Generators change what `return` means, so later languages mark them
    in the signature.** PEP 255: "A function that contains a `yield`
    statement is called a generator function", `return expr` was forbidden
    in one, and Guido's pronouncement on a `gen` keyword was "`def` it
    stays"; PEP 380 (3.3) later made `return expr` mean
    `raise StopIteration(expr)` [25]. C# rejects a valued `return` in an
    iterator (CS1622) [26]; Rust's `gen` blocks (RFC 3513, keyword reserved
    in edition 2024) allow `return` only without a value [14]; Nim declares
    `iterator` separately and "The yield statement is used instead of the
    return statement in iterators" [8] ([python.md](../languages/python.md),
    [csharp.md](../languages/csharp.md), [rust.md](../languages/rust.md),
    [nim.md](../languages/nim.md)).
12. **Indentation syntax interacts in two places, both already solved.**
    (a) `return` followed by a newline: JavaScript's automatic semicolon
    insertion returns `undefined` ([javascript.md](../languages/javascript.md));
    Python keeps the value on the `return` line. (b) Falling off the end: Python returns
    `None`, which mypy reports as "Missing return statement" when a return
    type is declared, PEP 8 asks for consistent returns and pylint R1710
    enforces it [38]; Kotlin requires explicit return types on block bodies
    because "their control flow can be complex, which makes the return type
    unclear to the reader" [16]. With a required return type and explicit
    `return`, this whole bug class is a compile error
    ([python.md](../languages/python.md), [kotlin.md](../languages/kotlin.md)).

## Recommendation for Sisu

- **Explicit `return expr` is the only way a function yields a value.**
  A non-unit function whose control flow can reach the end without
  `return` is a compile error (mypy's rule, in the compiler). `return`
  without a value is allowed only in unit functions; the value must begin
  on the `return` line. Return types are mandatory in signatures (Rust, Go,
  Kotlin block bodies). This supersedes the "block's value is its last
  expression" rule in [proposal/02](../../proposal/02-syntax.md) §Expressions
  and interacts with D-027; both need a decision-log entry.
- **No `=`-body sugar in v1.** Swift's saving was seven characters of
  `return`; `def f(x: int) -> int: return x + 1` is already one line, so the
  Kotlin/Swift/C# pressure (evidence 1) is weaker. Revisit after the stdlib.
- **`if` and `match` are expressions exactly when every branch is a single
  expression** (SE-0380's rule, Nim's required `else`): `x = if c: a else: b`
  on one line, and a multi-line `match` whose arms are `case p: expr`. Arms
  that need statements make it a statement, and the value is assigned in
  each arm. Python's `a if c else b` is kept for inline use (evidence 2).
  Block values (Rust, Nim, Java's `yield`) are deferred; Swift shows the
  cliff is tolerable and can be filled later without breaking code.
- **Named results are names, not variables.** The MATLAB signature idea is
  adopted in Whiley/Dafny/Swift form: `def divmod(a: int, b: int) -> (q: int, r: int)`
  declares tuple labels; the body still writes `return (q, r)`; the caller
  gets `res.q`; `ensures q * b + r == a` can name them. A single result may
  be labelled (`-> (n: int)`) or, when it is not, `ensures` uses the
  contextual name `result` (Eiffel, SPARK, D, C++26), reserved only inside
  contract clauses so Nim's `ResultShadowed` cannot arise. There is no naked
  `return`, no default initialisation, and no assignment to a result name
  (evidence 5-7, 9): the documentation and contracts without the bug class.
- **In-place construction is the compiler's job** (Zig result locations;
  C++17 guarantees elision only for prvalues and named-variable NRVO stays
  optional, so Zig's mechanism is the model), not surface syntax like
  `returned var` or `out self` (evidence 8); Ada's extended return
  statement (`return R : T do ... end return`, mandatory for build-in-place
  of limited types) is the closest precedent for a named in-place result and
  is the form to study if the compiler cannot do it alone. The memory
  document owns the guarantee.
- **Multiple values are tuples**, first-class and labelled, destructured
  with `let q, r = divmod(a, b)`, `_` to discard, must-use as for
  `Result` (Rust's `#[must_use]`, Swift's `@discardableResult` default,
  C++ `[[nodiscard]]` as the mechanisms) (evidence 10). No Go/Lua
  multi-value form; Perl's `wantarray` and Lisp's `values` are the other
  context-adjusted designs that lost to first-class tuples.
- **`?` and `return` target the innermost function, and closures are
  functions.** `?` desugars to `return Err(e)` of the closure it is in
  (Rust); no Kotlin-style non-local return from inlined closures
  (evidence 4). Early exit from an iterator-style API is a separate
  construct (open question).
- **Generators, if added, are marked in the signature** (return type
  `Iter[T]` or a `gen` marker), and a valued `return` inside one is an
  error (C#, Rust `gen`), never Python's silent retyping (evidence 11).
- The decision rests on evidence classes 2 and 6 (fixes shipped, style
  guides and lints); no error-rate study exists for any model, and preferring
  explicit `return` to Rust's form is, at bottom, a readability judgement.

## Open questions

- Block-valued `match` arms: Java's `yield`, Nim's last-line rule, or
  never? Swift's deferred "multi-statement branches" is the model to watch.
- Early exit out of a closure-taking API (`forEach`, `use`): labelled
  `break` to a named block (Scala `boundary`, Zig labelled blocks) or a
  Gleam-style `use` form.
- Spelling of labelled results: `-> (q: int, r: int)` versus `-> q: int, r: int`,
  and whether a label on a single result is allowed or `result` alone.
- Whether `defer` ever needs to modify the returned value (Go's `recover`
  case has no analogue without exceptions); generators and `yield`, which
  the proposal has not decided.

## Sources

1. Effective Go, "Named result parameters", "Recover". https://go.dev/doc/effective_go (2009, current)
2. The Go Programming Language Specification, "Return statements", "Calls". https://go.dev/ref/spec (2024)
3. Go wiki, Code Review Comments, "Named Result Parameters". https://go.dev/wiki/CodeReviewComments (2024)
4. Google Go Style Decisions, "Named result parameters", "Naked returns". https://google.github.io/styleguide/go/decisions (2024)
5. golang/go #21291 "proposal: Go 2: remove bare return" (2017, closed 2023); x/tools `shadow` analyzer; golangci-lint linters. https://github.com/golang/go/issues/21291 ; https://pkg.go.dev/golang.org/x/tools/go/analysis/passes/shadow ; https://golangci-lint.run/docs/linters/ (2026)
6. Uber Go Style Guide. https://github.com/uber-go/guide/blob/master/style.md (2026)
7. Go FAQ, "Why does Go not have the ?: operator?". https://go.dev/doc/faq (2024)
8. Nim Manual: "Return statement", "If expression", "Case expression", "NRVO", "Iterators and the for statement". https://nim-lang.org/docs/manual.html (2026)
9. Nim RFC #378 "Definite assignment analysis and out parameters" (Araq, 2021); "Nim v2.0 released" (2023-08-01); compiler `lineinfos.nim` (`warnResultShadowed`, `warnResultUsed`). https://github.com/nim-lang/RFCs/issues/378 ; https://nim-lang.org/blog/2023/08/01/nim-v20-released.html ; https://github.com/nim-lang/Nim/blob/devel/compiler/lineinfos.nim
10. The Rust Programming Language, ch. 3.3 "Functions" and ch. 9.2 "Recoverable Errors with Result". https://doc.rust-lang.org/book/ch03-03-how-functions-work.html ; https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html (2024)
11. rust-clippy sources `returns/mod.rs` (`needless_return`, style) and `implicit_return.rs` (restriction); users.rust-lang.org threads on explicit vs implicit return (2016, 2020); internals.rust-lang.org "Named returns" (2023). https://github.com/rust-lang/rust-clippy/tree/master/clippy_lints/src ; https://users.rust-lang.org/t/implicit-or-explicit-return-in-non-closures-what-is-more-idiomatic/6357 ; https://users.rust-lang.org/t/clippy-warnings-to-remove-return-a-bit-too-much/52999 ; https://internals.rust-lang.org/t/named-returns/18787
12. Rust RFC 243, trait-based exception handling (`?`). https://rust-lang.github.io/rfcs/0243-trait-based-exception-handling.html (2014)
13. Rust project goal "Instrument the Rust standard library with safety contracts" (2025H1); `core::contracts::ensures` (nightly). https://rust-lang.github.io/goals/2025h1/std-contracts.html ; https://doc.rust-lang.org/nightly/core/contracts/attr.ensures.html (2025)
14. Rust RFC 3513, `gen` blocks. https://rust-lang.github.io/rfcs/3513-gen-blocks.html (2023)
15. The Ruby Style Guide, "Flow of Control". https://rubystyle.guide/ (2026)
16. Kotlin docs: Coding conventions; Functions ("Return types"); Inline functions ("Non-local jump expressions"); Returns and jumps. https://kotlinlang.org/docs/coding-conventions.html ; https://kotlinlang.org/docs/functions.html ; https://kotlinlang.org/docs/inline-functions.html ; https://kotlinlang.org/docs/returns.html (2026)
17. Swift Evolution SE-0255, "Implicit returns from single-expression functions" (Swift 5.1). https://github.com/apple/swift-evolution/blob/main/proposals/0255-omit-return.md (2019)
18. Swift Evolution SE-0380, "`if` and `switch` expressions" (Swift 5.9). https://github.com/apple/swift-evolution/blob/main/proposals/0380-if-switch-expressions.md (2023)
19. Scala 3.3.0 released (`boundary`/`break`; `NonLocalReturns` deprecated). https://scala-lang.org/blog/2023/05/30/scala-3.3.0-released.html (2023)
20. JEP 361, Switch Expressions (JDK 14). https://openjdk.org/jeps/361 (2020)
21. C# docs: lambda operator and expression body definitions; The history of C# (C# 6 2015, 7.0 2017, 8.0 2019). https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/operators/lambda-operator ; https://learn.microsoft.com/en-us/dotnet/csharp/whats-new/csharp-version-history (2026)
22. The Swift Programming Language, "Functions" (multiple return values; implicit return). https://docs.swift.org/swift-book/documentation/the-swift-programming-language/functions/ (2024)
23. PEP 308, Conditional Expressions (Python 2.5). https://peps.python.org/pep-0308/ (2003-2005)
24. PEP 635, Structural Pattern Matching: Motivation and Rationale (Python 3.10). https://peps.python.org/pep-0635/ (2020)
25. PEP 255, Simple Generators (2001); PEP 380, Syntax for Delegating to a Subgenerator (Python 3.3, 2009). https://peps.python.org/pep-0255/ ; https://peps.python.org/pep-0380/
26. Compiler Error CS1622. https://learn.microsoft.com/en-us/dotnet/csharp/misc/cs1622 (2023)
27. MATLAB documentation: `function`, `return`, `nargout`, Function Argument Validation (output validation since R2022b); MathWorks Support Team on "Output argument ... not assigned" (2019). https://www.mathworks.com/help/matlab/ref/function.html ; https://www.mathworks.com/help/matlab/ref/return.html ; https://www.mathworks.com/help/matlab/ref/nargout.html ; https://www.mathworks.com/help/matlab/matlab_prog/function-argument-validation-1.html ; https://www.mathworks.com/matlabcentral/answers/97825 (R2026b)
28. Free Pascal Reference Guide 14.3 "Function results" (2015); Intel Fortran Developer Guide, "RESULT" (2025); Visual Basic "Return Statement" (2021). https://www.freepascal.org/docs-html/3.0.0/ref/refse89.html ; https://intel.com/content/www/us/en/docs/fortran-compiler/developer-guide-reference/2025-0/result.html ; https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/statements/return-statement
29. SPARK User's Guide, 5.5 "Specification Features" (`'Result`, `Side_Effects`). https://docs.adacore.com/spark2014-docs/html/ug/en/source/specification_features.html (2025)
30. Eiffel, "Major changes between ISE Eiffel 15.12 and ISE Eiffel 16.05" (Result attachment). https://www.eiffel.org/doc/uuid/54814BE5-3591-46C0-B883-42F1FC265873 (2016)
31. Carbon language design, "`returned var`" (2026); Mojo changelog v24.5 (2024-09-13, named result bindings) and v24.6 (2024-12-17, `out` syntax). https://github.com/carbon-language/carbon-lang/blob/trunk/docs/design/README.md#returned-var ; https://www.mojolang.org/releases/v0.24.5/ ; https://www.mojolang.org/releases/v0.24.6/
32. Odin overview, "Named results"; Zig Language Reference, "Blocks", "Result Location Semantics". https://odin-lang.org/docs/overview/ ; https://ziglang.org/documentation/master/ (2026)
33. Roc mini tutorial (`return`, `?`); roc-lang/roc PR #12117 "Warn on redundant returns and remove them during formatting" (merged 2026-10-07). https://github.com/roc-lang/roc/blob/main/docs/mini-tutorial-new-compiler.md ; https://github.com/roc-lang/roc/pull/12117
34. D Language Specification, "Functions: Contracts" (2026); P2900R14 "Contracts for C++" (2025-02-13). https://dlang.org/spec/function.html ; https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2025/p2900r14.pdf
35. Dafny Reference Manual (method `returns`, function result names); Whiley front-page example. https://dafny.org/latest/DafnyRef/DafnyRef ; https://whiley.org/ (2026)
36. AdaCore, Rationale for Ada 2012, chapter 3 (AI05-0143 parameters of all modes for functions; AI05-0144 order dependences). https://www.adacore.com/uploads/technical-papers/Ada2012_Rationale_Chp3_structure.pdf (2012)
37. Programming in Lua, 5.1 "Multiple Results". https://www.lua.org/pil/5.1.html (2003)
38. mypy error code `[return]`; PEP 8 "Programming Recommendations"; Pylint `inconsistent-return-statements` R1710. https://mypy.readthedocs.io/en/stable/error_code_list.html ; https://peps.python.org/pep-0008/ ; https://pylint.readthedocs.io/en/stable/user_guide/messages/refactor/inconsistent-return-statements.html (2026)
39. Gleam language tour, "Functions" and "Use". https://tour.gleam.run/functions/functions/ ; https://tour.gleam.run/advanced-features/use/ (2026)
40. haskell-beginners list, threads on `return` not being a control construct. https://mail.haskell.org/pipermail/beginners/2010-July/004707.html (2010)
