# Learnability

## The question

How long it takes to become productive, what the beginner traps are, and
which design decisions trade learnability for something else.

## What the evidence says

1. **Google's Rust data is the best measurement available, and it is
   internal and unreviewed**: two-thirds of over a thousand Googlers
   reported confidence within two months, teams were reported as
   productive as C++ teams, with four times fewer rollbacks and 25% less
   review time ([../evidence/memory-safety.md](../evidence/memory-safety.md)).
   The cost is real and front-loaded; the benefit is plausible and points
   one way, with the caveat that inter-programmer variance exceeds
   inter-language variance in every productivity study (Prechelt 2000) ([rust.md](../languages/rust.md)).
2. **The Rust learning curve is attributable to specific features and is
   paid mostly once**: first-class references and named lifetimes, async,
   and the trait system's corner cases are what non-adopters and ex-users
   name; among current users only about one in ten calls the borrow
   checker a big problem, while build times, debugging and async rank
   higher ([../evidence/surveys.md](../evidence/surveys.md)); single
   ownership and moves are the cheap part
   ([rust.md](../languages/rust.md), [../evidence/surveys.md](../evidence/surveys.md)).
3. **Error messages are the lever with the most survey evidence, and
   mixed experimental evidence.** Elm's designed messages, Rust's
   diagnostics work (the *explanation* of borrow errors, not the rule, is
   what improved; only 9% of users now call messages a big problem),
   Gleam's per-release diagnostic additions and Clojure's decade-long top
   complaint are the survey side; the controlled studies on novices
   (Becker et al.'s review, Denny et al.) are mixed or null, so the claim
   is "strong satisfier", not "measured learning gain" ([elm-purescript.md](../languages/elm-purescript.md),
   [gleam.md](../languages/gleam.md), [clojure.md](../languages/clojure.md),
   [../evidence/studies.md](../evidence/studies.md)).
4. **Simplicity is the size of the mental model, not the keyword
   count.** Go's users answered its missing features with code
   generation; Rich Hickey's simple-versus-easy distinction (does a
   feature interleave two concerns?) is a usable design checklist;
   Gleam's minimalism earns newcomer admiration whose conversion to
   retention is unmeasured (8% production use in 2024)
   ([go.md](../languages/go.md), [clojure.md](../languages/clojure.md),
   [gleam.md](../languages/gleam.md)).
5. **Absence teaches.** Elm's learnability comes partly from what it
   lacks (classes, HKT, effects in types) and partly from what it has
   (messages, a total library); Python's dominance in teaching comes from
   one obvious way and readable syntax; Racket's HtDP ladder and Pyret
   show that a restricted teaching subset helps novices and conventional
   syntax helps more than S-expressions ([elm-purescript.md](../languages/elm-purescript.md),
   [python.md](../languages/python.md), [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md)).
6. **Beginner traps are design defaults**: Python's mutable default
   arguments and late-binding closures; JavaScript's coercion and `this`;
   Go's typed nil and loop capture; Rust's "recursive type has infinite
   size" and `String` vs `&str` in week one; Lua's global-by-default;
   C's precedence ([python.md](../languages/python.md), [javascript.md](../languages/javascript.md),
   [go.md](../languages/go.md), [rust.md](../languages/rust.md), [lua.md](../languages/lua.md)).
7. **The "monad tutorial" cost is real** and effect handlers or
   capabilities deliver the composability without it
   ([haskell.md](../languages/haskell.md)).
8. **Static types are not the barrier for newcomers**: 83% of Python
   newcomers use type hints; inference quality and error messages are
   ([python.md](../languages/python.md)).
9. **Expert users want complexity; newcomers leave.** Scala kept its
   well-paid experts and lost newcomers; a language can choose to be
   Scala-shaped but should know it is choosing ([scala.md](../languages/scala.md)).
10. **Familiarity has a measurable value** only once there is a large
    pool to be familiar with; the Go/Rust/Swift/Kotlin consensus syntax is
    that pool; Mojo's "looks like Python" dividend has not been measured
    ([mojo.md](../languages/mojo.md)).
11. **Large language models change the calculus slightly**: the 2024-2026
    studies in [../evidence/studies.md](../evidence/studies.md) suggest
    that strong static checking helps generated code be correct and that
    training-data share affects assistance quality; a new language has
    little of the latter and should maximise the former.

## Recommendation for Sisu

Design for learnability by removing the documented traps rather than by
removing features: no lifetimes, no null, no exceptions, no `async`, no
inheritance, no macros; one way to write each thing; Elm-grade diagnostics
with codes and fix-its tested as a product; a small consensus syntax; a
total standard library; a documented "first week" path (`int`, `str`,
`List`, `?`, `!`, `scope`) that does not meet `ref`, `view`, `Shared` or
`comptime` until needed. See [proposal/00](../../proposal/00-vision-and-principles.md)
and [proposal/10](../../proposal/10-tooling.md); principles P1, P8.

## Open questions

- Q-41 a compiler-enforced teaching profile (no generics, no `unsafe`, no
  concurrency) following HtDP's evidence; whether the two-to-four-month
  Rust ramp is the ceiling Sisu must beat or whether days is the target.
