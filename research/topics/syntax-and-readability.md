# Syntax and readability

## The question

What the language looks like, and which of the thousand small choices
have evidence behind them.

## What the evidence says

1. **For novices, syntax measurably matters; for experts, familiarity
   dominates.** Stefik & Siebert 2013 found Perl's and Java's syntax no
   more intuitive to novices than a randomly generated one, while word-
   based operators and consistent constructs (Quorum, Python) measured
   better ([../evidence/studies.md](../evidence/studies.md)). Beyond
   novices there is almost no controlled evidence; the Python typing
   survey's finding that 83% of newcomers use type hints says annotations
   are not the barrier ([python.md](../languages/python.md)).
2. **Defaults are forever.** JavaScript's coercion, `this`, ASI and `var`;
   Python's mutable defaults and late-binding closures; Lua's global-by-
   default (fixed in 5.5 after thirty years); PHP's loose `==` (fixed
   2020); Perl's sigils; C's precedence of `&` below `==`; Go's by-
   reference loop capture (fixed in 1.22 via the module version gate).
   Every one is a permanent tax or a breaking fix ([javascript.md](../languages/javascript.md),
   [python.md](../languages/python.md), [lua.md](../languages/lua.md),
   [perl-php.md](../languages/perl-php.md), [go.md](../languages/go.md)).
3. **One way to write each thing.** Perl's TIMTOWTDI versus Python's "one
   obvious way"; C#'s five ways to declare a type with a constructor and
   query-vs-method LINQ; C++'s multiple initialisation syntaxes; Scala's
   two idiomatic ecosystems; D's feature accretion. The formatter cannot
   fix overlapping features ([csharp.md](../languages/csharp.md),
   [cpp.md](../languages/cpp.md), [scala.md](../languages/scala.md),
   [d.md](../languages/d.md)).
4. **A single enforced formatter is the cheapest, most-copied tooling
   decision.** gofmt, rustfmt, Black, Prettier, Elixir's formatter, Zig's:
   every language that shipped one is praised for it and no language has
   removed one ([go.md](../languages/go.md)).
5. **Braces vs indentation is unresolved on the evidence; tooling decided
   the first draft, the owner decided the language.** Python, Nim, F# and Haskell users love indentation and
   it is readable; braces survive copy-paste, code generation, multi-line
   closures and trailing-closure syntax, and every formatter-first
   language chose them. F#'s designer judged that OCaml's `in`/`;;` cost
   adoption, which argues for token-light syntax within braces rather than
   for indentation ([fsharp-sml.md](../languages/fsharp-sml.md),
   [python.md](../languages/python.md)).
6. **Parsing without symbol-table feedback is a feature.** C++'s and
   Rust's `<` ambiguity (`template`/turbofish), Go's choice of `[]` for
   generics after studying it, and the general finding that an LL grammar
   gives better error messages and faster, more robust tooling
   ([go.md](../languages/go.md), [rust.md](../languages/rust.md)).
7. **S-expressions lost even among their own advocates.** Pyret and
   Rhombus are conventional-syntax front ends built by the Racket
   community; the macro model is worth keeping, the notation is not
   ([lisp-scheme-racket.md](../languages/lisp-scheme-racket.md)). APL's
   glyphs similarly: the operations transplanted everywhere, the symbols
   nowhere ([apl-bqn-k.md](../languages/apl-bqn-k.md)).
8. **Trailing closures and labelled arguments are the cheapest
   readability wins.** Ruby blocks, Kotlin and Swift trailing lambdas,
   Gleam's `use`, Smalltalk's keyword messages surviving into Swift and
   Objective-C; named arguments in Python, Kotlin, Swift, Gleam, OCaml
   ([ruby.md](../languages/ruby.md), [kotlin.md](../languages/kotlin.md),
   [gleam.md](../languages/gleam.md), [smalltalk.md](../languages/smalltalk.md)).
9. **Dropping the ternary without measuring the replacement cost was a
   regret** (Breslav on Kotlin); but `if` as an expression is the
   replacement that works in Rust, Kotlin and Scala
   ([kotlin.md](../languages/kotlin.md)).
10. **Style-insensitive identifiers (Nim) and non-ASCII identifiers
    (Rust, Go) each cost tooling and security** (Trojan Source, homoglyphs)
    for benefits that a formatter serves better ([nim.md](../languages/nim.md)).
11. **Operator definition and custom precedence** produce symbol soup
    (Haskell, Scala) and type-checker cost (Swift); overloading a fixed
    set via traits is the compromise every newer language chose
    ([haskell.md](../languages/haskell.md), [scala.md](../languages/scala.md),
    [swift.md](../languages/swift.md)).
12. **Mixing bitwise, boolean and arithmetic operators without parentheses
    should be an error** (Austral's rule; C's fifty-year bug class)
    ([austral.md](../languages/austral.md), [c.md](../languages/c.md)).
13. **Pipelines read left to right.** F#'s `|>` was copied everywhere;
    Forth's and the concatenative family's one surviving idea is left-to-
    right composition with names ([fsharp-sml.md](../languages/fsharp-sml.md),
    [forth.md](../languages/forth.md)). Method chains give this in a
    brace language.

## Recommendation for Sisu

The first draft recommended braces on the tooling argument (item 5); the
project owner chose indentation with Python idioms (Q-8), and the research
in [indentation-syntax.md](indentation-syntax.md),
[python-idioms.md](python-idioms.md) and
[function-results.md](function-results.md) says how to do that without the
documented failures. The result: indentation blocks opened by `:`, layout
decided in the lexer, spaces only, no brace dialect; `def`, `let`/`var`,
`elif`, `pass`, `and`/`or`/`not`, `in`, same-direction chained comparisons;
one closure form `x -> e` with `it` and `do` trailing blocks; explicit
`return`; `[T]` generics; Python's literal forms (`[]`, `{k: v}`, `{a}`,
comprehensions); `@` attributes; keyword arguments; a fixed overloadable
operator set with no custom precedence and an error on mixed operator
classes; PEP 8 naming with lowercase builtin types; a formatter with no
options. See [proposal/02](../../proposal/02-syntax.md); decisions D-020
through D-027, D-044 through D-046, D-055, D-056.

## Open questions

- Q-8 braces vs indentation; Q-9 last-expression values; Q-12 UFCS; Q-25
  implicit closure parameter; Q-33 non-ASCII identifiers; Q-35 shadowing.
