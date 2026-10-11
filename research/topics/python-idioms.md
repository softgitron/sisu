# Python idioms: which to keep

## The question

The designer has chosen indentation blocks with "Python syntax idioms
followed where possible" ([indentation-syntax.md](indentation-syntax.md)).
Sisu is otherwise Python's opposite: statically typed, compiled, memory
safe, no exceptions, no `None`, no macros, explicit `return`. Which surface
forms carry their value across that gap, which need a typed
reinterpretation, and which exist only because Python is dynamic? The test
is the methodology's: a PEP that fixed it, a linter or style-guide ban, or
a typed Python-syntax language that dropped it ([python.md](../languages/python.md),
[mojo.md](../languages/mojo.md), [nim.md](../languages/nim.md)).

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Python plus a typed sublanguage | Cython (`cdef` beside `def`) | Gradual speed-up of real Python | Two semantics per file: C-typed values "have C semantics"; `cdef` functions callable only from Cython; `noexcept` contracts | [35] |
| Restricted subset, same syntax | RPython, Codon | "if you know Python, you already know 99% of Codon" | Every dynamic idiom is a trap: one type per variable, no run-time class definition, limited generators, no `**kwargs`, 64-bit `int` with silent wrap-around, C numeric semantics by default | [35] |
| New language wearing Python's surface | Boo (2003), Cobra (2006), Nim (2008), Mojo (2023) | Familiarity for the largest language population; indentation, `def`, `elif`, comprehensions, keyword arguments transplant cleanly | Boo and Cobra died (last releases 2013). Nim rewrote what depends on dynamism (`var`/`let`, `discard`, `iterator`, bool-only conditions, no loop `else`). Mojo made types mandatory, replaced `class` with `struct`, kept `with`, loop `else`, `__dunder__` methods and one-expression lambdas; no `match`, no custom decorators, no f-strings | [mojo.md](../languages/mojo.md), [nim.md](../languages/nim.md), [34], [36] |

## What the evidence says

1. **Annotations and `->` transplant unchanged; only their meaning flips.**
   PEP 3107 (2006) introduced `def f(a: T) -> R` while stating "By itself,
   Python does not attach any particular meaning or significance to
   annotations"; PEP 484 (2014) gave them one [5]. Mojo 1.1: "In Mojo, you
   must declare types explicitly" [34] ([nim.md](../languages/nim.md)).
2. **`class` is boilerplate; dataclasses are the admission.** PEP 557
   (2017): "Data Classes save you from writing and maintaining these
   methods"; it raises `TypeError` for `list`, `dict` or `set` defaults [6].
   Mojo replaced `class` with `struct`, "a statically typed alternative that
   the compiler can optimize aggressively" [34] ([mojo.md](../languages/mojo.md)).
3. **Indentation, `:`, `elif`, `pass`, `and`/`or`/`not` are what every
   descendant kept.** The FAQ: the colon lets editors "look for colons to
   decide when indentation needs to be increased" [4]; Nim's `discard`
   doubles as its unused-value check [33]. One change is forced: Python's
   `and`/`or` "return the last evaluated argument" [23]; without truthiness
   they return `bool`, and `x or default` becomes `??`
   ([studies.md](../evidence/studies.md) on word operators).
4. **Chained comparisons: Julia copied them, nobody else did.** Python
   admits "`x < y > z` is perfectly legal (though perhaps not pretty)" [23];
   Julia finds chains "often quite convenient in numerical code" but leaves
   evaluation order "undefined" [32] ([julia.md](../languages/julia.md)).
5. **`in` as sugar for a named method was copied everywhere.** Nim: "`a in
   b` is transformed into `contains(b, a)`" [33]; Kotlin: `b.contains(a)` [32].
6. **`is` is a dynamic-language operator and a known trap.** Python 3.8
   added a `SyntaxWarning` for `is` with literals, which "can often work by
   accident in CPython, but are not guaranteed by the language spec" [21];
   the accident is the small-integer cache, `-5` to `256` for decades and
   `1024` since gh-133059 (merged 24 Sep 2025) [22]. PEP 8 needs `is` only
   for `None` [2] ([python.md](../languages/python.md)).
7. **Truthiness is the idiom every typed language refused.** Python treats
   `False`, `None`, "numeric zero of all types, and empty strings and
   containers" as false [23] and PEP 8 says to rely on it [2]. Python's own
   record: "Before Python 3.5, a `datetime.time` object was considered to be
   false if it represented midnight in UTC. This behavior was considered
   obscure and error-prone and has been removed" [21]; Google: for integers
   "implicit false may involve more risk than benefit" [28]. Swift's "type
   safety prevents non-Boolean values from being substituted for `Bool`"
   [30]; Rust "will not automatically try to convert non-Boolean types to a
   Boolean" [29]; Go and Nim require a `bool` [31][33]; only Mojo keeps a
   `Boolable` hook [34] ([swift.md](../languages/swift.md),
   [rust.md](../languages/rust.md), [go.md](../languages/go.md)). `None`
   generated the demand `T?`, `?.` and `??` answer: PEP 505 (2015, Deferred)
   counted 449 None-coalescing `if` blocks and 120 coalescing `or`
   expressions in the 3.7 standard library [7].
8. **The conditional expression's order was contested, then copied.** PEP
   308's 2003 vote (518 ballots, 82 rejecting all options) had no majority;
   Guido chose `X if C else Y` in 2005 after an expression-first draft "was
   found to be too uncomfortable" [17]. Mojo kept it [34]; Kotlin regretted
   dropping its ternary ([kotlin.md](../languages/kotlin.md)).
9. **Comprehensions beat `map`/`filter` by the designer's judgement, and
   both stayed.** PEP 202, 274 and 289: "List comprehensions greatly reduced
   the need for filter() and map()" [8]. Guido, 2005: "filter(P, S) is
   almost always written clearer as [x for x in S if P(x)]", then "lambda,
   filter and map will stay... Only reduce will be removed" [9]. Copied by
   Julia, Nim (`collect` [33]) and Mojo ("The syntax is essentially
   identical" [34]) after Haskell's originals ([haskell.md](../languages/haskell.md)).
10. **`lambda` is crippled by design; closures and defaults are
    evaluation-time traps.** PEP 3099: "Adding support for statements is a
    non-starter" [3]; PEP 8: "Always use a def statement instead of an
    assignment statement that binds a lambda" [2]; Nim's `=>` and Mojo's
    lambdas are the same sugar [33] ([mojo.md](../languages/mojo.md)). The
    FAQ: "Default values are created exactly once, when the function is
    defined", and a loop variable "is accessed when the lambda is called —
    not when it is defined" [4]; ruff B006 and pylint W0102 exist for the
    former [27] (no frequency study found [unverified]). Nim re-evaluates a
    default "every time the function is called" [33]
    ([go.md](../languages/go.md) on Go 1.22's per-iteration variables).
11. **Declaration-by-assignment is the root of `global`, `nonlocal` and
    `UnboundLocalError`.** The FAQ: "If a variable is assigned a value
    anywhere within the function's body, it's assumed to be a local" [4];
    PEP 3104 (2006) added `nonlocal` after rejecting about twenty spellings
    [18]. Lua 5.5 added `global` declarations after thirty years
    ([lua.md](../languages/lua.md)); JavaScript needed strict mode
    ([javascript.md](../languages/javascript.md)); Go's `:=` shadows
    outside its block [31] ([go.md](../languages/go.md)).
12. **Keyword, keyword-only and positional-only parameters each have a
    reason; `**kwargs` has no type.** PEP 570 (2018): "Library authors
    would have the flexibility to change the name of positional-only
    parameters without breaking callers"; PEP 3132: "Many algorithms
    require splitting a sequence in a 'first, rest' pair" [10]. RPython
    allows "defaults and `*args`, but not `**keywords`" [35]
    ([python.md](../languages/python.md)).
13. **Half-open slices are well argued; negative indices and silent `zip`
    are not.** Guido (2013): `a[:n]` and `a[i:i+n]` "are beautiful" [26].
    Go requires `0 <= low <= high <= len(a)` [31]; Nim writes from-the-end
    explicitly, `fib[^1]` [33]; the objection to `a[-1]` is a computed
    index drifting below zero [anecdotal]. PEP 618 (2020) added `strict=`
    to `zip` because "faulty refactoring or logic errors could easily result
    in silently losing data" [19] ([go.md](../languages/go.md)).
14. **f-strings won, and PEP 701 says how to parse them.** PEP 498 (2015):
    older methods were "either error prone, inflexible, or cumbersome";
    PEP 701 (2022) lifted the quote, backslash and nesting limits because
    tokenising f-strings "as `STRING` tokens" with a hand-written
    post-parser "has led to inconsistencies and bugs" [11]. Mojo shipped
    without f-strings [34, secondary] ([mojo.md](../languages/mojo.md)).
15. **Docstrings as runtime strings fail for attributes.** PEP 257: strings
    outside the first statement "are not recognized by the Python bytecode
    compiler"; PEP 224 (2000) was rejected ("It's not the implementation,
    it's the syntax"); Sphinx needs `#:` comments [12]. Nim's `##` documents
    items and fields uniformly ([nim.md](../languages/nim.md)).
16. **Decorators are a run-time call wearing declaration syntax.** PEP 318
    (2003): `@` was "a fairly arbitrary choice"; the dominant use is
    registration (Flask: "Use the `route()` decorator to bind a function to
    a URL") [13] ([python.md](../languages/python.md)). Mojo: "Mojo doesn't
    support custom decorators. The decorators in this section are built
    into the compiler" [34]; Nim uses `{.pragma.}` [33].
17. **`with` is the scoped-resource idiom GC languages copied; RAII
    languages did not need it.** PEP 343 (2005) exists "to factor out
    standard uses of `try/finally`" and rejected PEP 340 because "the
    with-statement should not be usable as a loop" [14]. Java, C#, Kotlin
    and JavaScript copied it ([python.md](../languages/python.md)); Rust and
    Swift destroy at scope end; Mojo kept it [34]
    ([memory-management.md](memory-management.md)).
18. **Implicit generator functions were argued against in their own PEP.**
    PEP 255 (2001): "A `yield` statement buried in the body is not enough
    warning that the semantics are so different", before the ruling "`def`
    it stays" [15]. Nim chose `iterator`; `yield` "is only valid in
    iterators" [33] ([nim.md](../languages/nim.md)).
19. **`match` as a statement and bare-name captures are the complaints PEP
    635 anticipated.** An expression form "would fit poorly with Python's
    statement-oriented nature"; dotted-name value patterns "preclude...
    local variables and global variables defined in the current module from
    acting as constants"; an uppercase convention "has been met with
    scepticism since there is no similar precedence in core Python" [16].
    Rust has the same hole and a deny-by-default lint,
    `bindings_with_variant_name` [29] ([rust.md](../languages/rust.md));
    Swift added `if`/`switch` expressions in 5.9 because statement-only
    forms cause "ceremony to make a return" [30]
    ([swift.md](../languages/swift.md)). Sisu's enforced casing supplies
    the missing precedent.
20. **`finally` has measured bugs; `assert` under `-O` is a trap; `assert`
    in tests is loved.** PEP 765 (2024) found 203 `return`/`break`/
    `continue`-in-`finally` sites in 120,964,221 lines of the top 8,000
    PyPI packages, 149 (73%) "clearly incorrect" [20]
    ([error-handling.md](error-handling.md)). The compiler "emits no code
    for an `assert` statement when optimization is requested" [23]; Google:
    "Do not use `assert` statements in place of conditionals or validating
    preconditions" [28]; Nim's `doAssert` is "always turned on regardless of
    `--assertions`" [33]; pytest "allows you to use the standard Python
    `assert`" and rewrites it to show subexpression values [37]
    ([testing.md](testing.md)).
21. **`/` versus `//` was a bug class, and the typed descendants
    regressed.** PEP 238 (2001): `/` "has an ambiguous meaning for numerical
    arguments"; PEP 465 (2014) justified `@` with counts (`dot` 0 per
    10,000 stdlib lines, 99 in scikit-learn) [17]. Mojo: "`/` returns a
    value that always matches the type of the operands" [34]; Codon defaults
    to C semantics [35] ([strings-and-numerics.md](strings-and-numerics.md)).
22. **The walrus is the one PEP 3099 promise Python broke**: "There will be
    no alternative binding operators such as `:=`" [3]; PEP 572 (2018)
    added it and ended the BDFL role ([python.md](../languages/python.md)).
    PEP 695's `def f[T]` and `type` went the other way, with variance
    inferred because it "is not well understood by most Python developers"
    [18].
23. **Underscore privacy, `__all__`, dunders, imports and `__main__` are
    conventions a compiler replaces.** PEP 8: internal names "should still
    be prefixed with a single leading underscore", modules "should
    explicitly declare the names in their public API using the `__all__`
    attribute" [2]. The data model's uniformity is why NumPy feels built in
    ([python.md](../languages/python.md)); Kotlin and Rust reach it with
    named `operator`/trait methods [32] ([rust.md](../languages/rust.md)).
    PEP 328 (2003) made imports absolute because "it's not clear whether
    `import foo` refers to a module within the package or some module
    outside" [19]; `__main__`'s docs say to put "as few statements as
    possible" under the guard and use a `main` function [24].
24. **Explicit `self` survived every challenge; quotes, loop `else` and
    semicolons did not earn their place.** PEP 3099: "`self` will not
    become implicit" [3]; Guido (2008): it "reinforces the theoretical
    equivalency between these two ways of calling a method" [25]; Mojo's
    `mut self`/`out self` hang the ownership convention on it
    ([mojo.md](../languages/mojo.md)). PEP 8 on quotes: "Pick a rule and
    stick to it" [2]; Black normalises to double [27]; a language with `'c'`
    literals cannot. The tutorial on loop `else`: "(Yes, this is the
    correct code. Look closely: the `else` clause belongs to the `for`
    loop, **not** the `if` statement.)" [24]; PEP 3099 keeps it [3], Mojo
    kept it [34], Nim's grammar has no loop `else` [33]; Hettinger's
    `nobreak` is second-hand [38, unverified].
25. **The Zen is the part with the best record.** "Explicit is better than
    implicit", "Errors should never pass silently", "refuse the temptation
    to guess" and "one-- and preferably only one --obvious way" [1] are the
    rationale for declared types, must-use results, the operator-mixing
    error and the formatter ([designer-retrospectives.md](../evidence/designer-retrospectives.md));
    Python's own breaks of them (truthiness, operand-returning `or`) are
    the items above.
## Recommendation for Sisu

| Idiom | Verdict | Sisu form and reason |
|---|---|---|
| `def name(p: T) -> R:` | keep | mandatory types on signatures (1); the proposal's `fn` becomes `def` |
| `class`, `__init__`, `self.x = x` | drop | `struct` with declared fields, `@derive`, no inheritance (2) |
| Indentation, `:`, `elif`, `pass`; `and`/`or`/`not` | keep | R1-R10 of [indentation-syntax.md](indentation-syntax.md); `bool`-only operands, `??` replaces `x or default` (3) |
| Chained comparisons | adapt | same-direction chains only; mixed is an error (4); revises the blanket error |
| `in` | keep | `Contains` trait (5) |
| `is`; truthiness | drop | `==` via `Eq`, identity is a named function; `if` requires `bool`, `is_empty()`, no `__bool__` (6, 7) |
| `None`/`Optional` | adapt | `T?`, `?.`, `??`, `if let` (7) |
| `a if c else b` | keep, provisionally | one-line form; the `if` expression covers multi-line (8) |
| Comprehensions, generator expressions | keep | lazy `(...)`, eager `[...]`, `{k: v}`; adaptors stay for the rest (9) |
| `lambda` | adapt | `x -> expr` inline, `do` trailing block (10) |
| Nested `def`, closures; mutable defaults | keep; drop | per-iteration loop bindings; defaults evaluated at each call (10) |
| `global`/`nonlocal`, declaration by assignment | drop | `let`/`var`; undeclared assignment is an error; no shadowing (11) |
| Keyword arguments, `*`, `/`, `*args`; `**kwargs` | keep; drop | positional-or-keyword by default; bare `*` and `/`; typed `*xs: T` and spread; no `**kwargs` (12) |
| Slicing `a[i:j]`; negative indices | keep; adapt | half-open, bounds-checked; `a[^1]`, `a[i:^1]` from Nim (13) |
| `range`, `enumerate`, `zip` | keep | `0..n`; `zip` strict by default, `zip_shortest` explicit (13) |
| f-strings, format specs | keep | `"{x:.2}"` with no prefix; interpolation in the grammar (14) |
| Docstrings, `#:` | adapt | `##` doc comments above items and fields; `#` line comments (15, 21) |
| Decorators | adapt | `@attr` compile-time attributes; registration by reflection; wrapping is a plain call (16) |
| `with` | adapt | scoped binding dropped at block end; never a loop; no `__enter__`/`__exit__` (17) |
| `yield` in a plain `def` | adapt | explicit generator declaration, `gen def` or equivalent (18) |
| `match`/`case` | adapt | expression and statement; hard keywords; guards, `\|`, struct patterns; lowercase captures, capitalised constants (19) |
| `try`/`except`/`else`/`finally`, `raise from` | drop | `T ! E`, `?`, `else`, `context`, `defer`, `Drop` (20) |
| `assert` | adapt | always on; `debug_assert` for the stripped form; test asserts show subexpressions (20) |
| `//`, `/`, `**`; `@` matmul | keep, adapt `/`; reserve | `int / int` is a type error; `//` floor; `**` keeps Python precedence; `MatMul` trait if asked (21) |
| `:=` | drop | `if let`, `while let` (22) |
| `def f[T]`, `type X = ...` | keep | already the proposal's syntax (22) |
| `_name`, `__all__`, name mangling; `__dunder__` protocols | drop; adapt | `pub`, `pub(package)`; traits with plain names, one per syntax form (23) |
| `import`, `from ... import`, relative dots | adapt | `use path.{a, b as c}`; package-absolute paths; no `*` (23) |
| `if __name__ == "__main__"` | drop | `def main(...)` entry point; `sisu run` for scripts (23) |
| Explicit `self`; tuple and star unpacking | keep | `self` carries the receiver convention; `let (a, *rest) = xs` is a pattern (12, 24) |
| `'...'`, `"..."`, `"""..."""`, `r"..."` | adapt | double quotes only; `'c'` is a character; keep triple and raw (24) |
| `for`/`while` ... `else`; semicolons | drop | `if let x = xs.find(p):`; one statement per line (24) |
| Zen of Python | keep | as written principles, with its three self-violations noted (25) |

Consequences for [proposal/02](../../proposal/02-syntax.md): comments move
from `//` to `#` (so `//` is floor division, `##` documentation); `fn`,
closures and `match` arms are restated in indentation form.

## Open questions

- The one-line conditional: `a if c else b` (Python, Mojo), `if c then a
  else b`, or none; the only evidence is PEP 308's inconclusive vote.
- Whether `with` earns a keyword when `Drop` plus block scope already does
  the work (Rust says no, Mojo says yes); the spelling of generator
  declarations (`gen def`, `iterator`) alongside explicit `return`.
- Comprehensions beside iterator adaptors: accept the overlap as Python
  did, or let the linter pick one; `**` against unary minus; Q-25; the
  residual items in [indentation-syntax.md](indentation-syntax.md).

## Sources

1. PEP 20, "The Zen of Python". https://peps.python.org/pep-0020/ (19 Aug 2004)
2. PEP 8, "Style Guide for Python Code". https://peps.python.org/pep-0008/ (2001, updated)
3. PEP 3099, "Things that will Not Change in Python 3000". https://peps.python.org/pep-3099/ (4 Apr 2006)
4. Python Design and History FAQ; Programming FAQ. https://docs.python.org/3/faq/design.html ; https://docs.python.org/3/faq/programming.html (2026)
5. PEP 3107, "Function Annotations" (2 Dec 2006); PEP 484, "Type Hints" (2014). https://peps.python.org/pep-3107/ ; https://peps.python.org/pep-0484/
6. PEP 557, "Data Classes". https://peps.python.org/pep-0557/ (2 Jun 2017)
7. PEP 505, "None-aware operators" (Deferred). https://peps.python.org/pep-0505/ (18 Sep 2015)
8. PEP 202 (13 Jul 2000), PEP 274 (25 Oct 2001), PEP 289 (30 Jan 2002). https://peps.python.org/pep-0202/ ; https://peps.python.org/pep-0274/ ; https://peps.python.org/pep-0289/
9. G. van Rossum, "The fate of reduce() in Python 3000". https://www.artima.com/weblogs/viewpost.jsp?thread=98196 (10 Mar 2005)
10. PEP 3102, "Keyword-Only Arguments" (22 Apr 2006); PEP 570, "Positional-Only Parameters" (20 Jan 2018); PEP 3132, "Extended Iterable Unpacking" (30 Apr 2007). https://peps.python.org/pep-3102/ ; https://peps.python.org/pep-0570/ ; https://peps.python.org/pep-3132/
11. PEP 498, "Literal String Interpolation" (1 Aug 2015); PEP 701, "Syntactic formalization of f-strings" (15 Nov 2022). https://peps.python.org/pep-0498/ ; https://peps.python.org/pep-0701/
12. PEP 257, "Docstring Conventions" (29 May 2001); PEP 224, "Attribute Docstrings" (Rejected, 23 Aug 2000); Sphinx, "sphinx.ext.autodoc". https://peps.python.org/pep-0257/ ; https://peps.python.org/pep-0224/ ; https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html (2026)
13. PEP 318, "Decorators for Functions and Methods" (5 Jun 2003); Flask 3.1 Quickstart, "Routing". https://peps.python.org/pep-0318/ ; https://flask.palletsprojects.com/en/stable/quickstart/ (2026)
14. PEP 343, "The 'with' Statement". https://peps.python.org/pep-0343/ (13 May 2005)
15. PEP 255, "Simple Generators". https://peps.python.org/pep-0255/ (18 May 2001)
16. PEP 634 and PEP 635, "Structural Pattern Matching" (12 Sep 2020). https://peps.python.org/pep-0634/ ; https://peps.python.org/pep-0635/
17. PEP 308, "Conditional Expressions" (7 Feb 2003; decided 29 Sep 2005); PEP 238, "Changing the Division Operator" (11 Mar 2001); PEP 465, "A dedicated infix operator for matrix multiplication" (20 Feb 2014). https://peps.python.org/pep-0308/ ; https://peps.python.org/pep-0238/ ; https://peps.python.org/pep-0465/
18. PEP 3104, "Access to Names in Outer Scopes" (12 Oct 2006); PEP 572, "Assignment Expressions" (28 Feb 2018); PEP 695, "Type Parameter Syntax" (15 Jun 2022). https://peps.python.org/pep-3104/ ; https://peps.python.org/pep-0572/ ; https://peps.python.org/pep-0695/
19. PEP 618, "Add Optional Length-Checking To zip" (1 May 2020); PEP 328, "Imports: Multi-Line and Absolute/Relative" (21 Dec 2003). https://peps.python.org/pep-0618/ ; https://peps.python.org/pep-0328/
20. PEP 765, "Disallow return/break/continue that exit a finally block" (15 Nov 2024); PEP 3134, "Exception Chaining and Embedded Tracebacks" (12 May 2005). https://peps.python.org/pep-0765/ ; https://peps.python.org/pep-3134/
21. "What's New In Python 3.8", SyntaxWarning for `is` with literals (bpo-34850, 2019); "What's New In Python 3.5", `datetime.time` midnight (bpo-13936, 2015). https://docs.python.org/3/whatsnew/3.8.html ; https://docs.python.org/3/whatsnew/3.5.html
22. Python C API, "Integer Objects"; gh-133059 / GH-133160, "Increase the small positive integer cache to 1024". https://docs.python.org/3/c-api/long.html ; https://github.com/python/cpython/issues/133059 (merged 24 Sep 2025)
23. Python Language Reference, "Expressions" and "Simple statements". https://docs.python.org/3/reference/expressions.html ; https://docs.python.org/3/reference/simple_stmts.html (2026)
24. Python Tutorial §4.5 (loop `else`) and §4.7 (`match`); Library Reference, "`__main__`". https://docs.python.org/3/tutorial/controlflow.html ; https://docs.python.org/3/library/__main__.html (2026)
25. G. van Rossum, "Why explicit self has to stay". https://neopythonic.blogspot.com/2008/10/why-explicit-self-has-to-stay.html (26 Oct 2008)
26. G. van Rossum, "Why Python uses 0-based indexing". https://python-history.blogspot.com/2013/10/why-python-uses-0-based-indexing.html (24 Oct 2013)
27. Black, "The Black code style", Strings; Ruff B006; pylint W0102. https://black.readthedocs.io/en/stable/the_black_code_style/current_style.html ; https://docs.astral.sh/ruff/rules/mutable-argument-default/ ; https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/dangerous-default-value.html (2026)
28. Google Python Style Guide (§2.2, 2.4, 2.5, 2.6, 2.7, 2.10-2.14, 2.17). https://google.github.io/styleguide/pyguide.html (2026)
29. The Rust Programming Language, ch. 3.5; rustc deny-by-default lints, `bindings_with_variant_name`. https://doc.rust-lang.org/book/ch03-05-control-flow.html ; https://doc.rust-lang.org/rustc/lints/listing/deny-by-default.html (2026)
30. The Swift Programming Language, "The Basics", Booleans; SE-0380, "`if` and `switch` expressions" (Swift 5.9, 2023). https://github.com/swiftlang/swift-book/blob/main/TSPL.docc/LanguageGuide/TheBasics.md ; https://github.com/swiftlang/swift-evolution/blob/main/proposals/0380-if-switch-expressions.md
31. The Go Programming Language Specification (if statements, short variable declarations, index and slice expressions). https://go.dev/ref/spec (2026)
32. Julia Manual, "Chaining comparisons"; Kotlin docs, "Operator overloading". https://docs.julialang.org/en/v1/manual/mathematical-operations/ ; https://kotlinlang.org/docs/operator-overloading.html (2026)
33. Nim Manual (grammar, Boolean type, discard, iterators, defaults, `in`/`contains`, pragmas); `std/assertions`; `std/sugar`. https://nim-lang.org/docs/manual.html ; https://nim-lang.org/docs/assertions.html ; https://nim-lang.org/docs/sugar.html (2026)
34. Mojo 1.1 docs: "Compound statements", "Decorators", "Control flow", "Python to Mojo", `Boolable`; cheat sheet (t-strings, 1.0.0b2). https://mojolang.org/docs/reference/compound-statements/ ; https://mojolang.org/docs/reference/decorators/ ; https://mojolang.org/docs/manual/control-flow/ ; https://mojolang.org/docs/manual/python-to-mojo/ ; https://docs.modular.com/mojo/std/builtin/bool/Boolable (2026)
35. Codon, "Language overview: Differences with Python"; RPython, "RPython Language"; Cython User Guide, "Language Basics". https://docs.exaloop.io/language/overview/ ; https://rpython.readthedocs.io/en/latest/rpython.html ; https://cython.readthedocs.io/en/latest/src/userguide/language_basics.html (2026)
36. Wikipedia, "Boo (programming language)" (2003; 0.9.7, Mar 2013) and "Cobra (programming language)" (2006; 0.9.6, Dec 2013). https://en.wikipedia.org/wiki/Boo_(programming_language) ; https://en.wikipedia.org/wiki/Cobra_(programming_language) (2026)
37. pytest, "How to write and report assertions in tests". https://docs.pytest.org/en/stable/how-to/assert.html (2026)
38. T. Hunner, "Using else with a loop in Python" (Hettinger's `nobreak`, second-hand). https://www.pythonmorsels.com/for-else/ (2026) [unverified]
