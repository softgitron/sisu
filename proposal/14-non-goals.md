# 14 - Non-goals

A language that tries to be everything becomes C++. These are the things
Sisu deliberately does not try to be, with the reason. A non-goal can be
revisited, but only by an explicit decision with evidence, not by drift.

## Not a dynamic language

Sisu does not have dynamic typing, runtime type mutation, monkey patching,
`eval`, or a mode where types are optional. Dart's reversal from optional
types to sound types, TypeScript's `any` being the leading source of
runtime errors, and the Python and Ruby typing retrofits' partial adoption
are the evidence that gradual typing is a migration strategy, not a design.
The exploratory workflow that dynamic languages serve is addressed by the
REPL, scripts and fast compilation, not by weakening the type system.

## Not a Python superset or a drop-in replacement for anything

Mojo's walk-back from "Python superset" is the evidence: compatibility with
an existing language's semantics imports its mistakes. Sisu interoperates
with Python through the C ABI.

## Not a C++ interop language

See [12-interop-and-targets.md](12-interop-and-targets.md). Carbon is
designed for that and Sisu is not.

## Not a research vehicle

Every feature has a predecessor that shipped. Algebraic effect types,
dependent types, linear types as the primary discipline, content-addressed
code storage, and live image-based development are each discussed in the
open questions, and each is deferred because the research found them
unproven at scale in 2026. Sisu's job is to combine what works, not to find
out what works.

## Not a kitchen sink

Features Sisu does not have and does not plan to add: class inheritance,
exceptions, `null`, macros (v1), operator definition, implicit conversions,
properties, function overloading by parameter type (ambiguity, slower
inference; named arguments and traits cover the use cases), variadic
functions (slices and tuples cover them), default interface method
conflicts via multiple inheritance, `goto`, union types outside of enums
and error sets, global mutable variables without synchronisation. Each
absence is in the decision log.

## Not optimised for one domain

Sisu does not have language-level support for matrices (Julia, Fortran),
relations (Prolog, SQL), actors as a keyword (Erlang, Pony), reactive
streams, GPU kernels (Mojo), or GUI description. These are libraries, and
the language is designed so that libraries can express them well (traits,
operator overloading on the fixed set, comptime-checked string DSLs,
trailing closures). Where a library cannot express something well, that is
an open question for the language, recorded when found. Nor is Sisu a
notebook environment: a worksheet (a script re-run top to bottom with its
values shown inline and never saved) is tooling after 0.1, but a notebook
with cells and stored outputs stays outside the project unless Q-48's
conditions are met ([00](00-vision-and-principles.md)).

## Not fastest at all costs

Sisu does not have undefined behaviour in safe code, a build mode without
bounds checks, or an `unsafe`-free way to disable overflow checks. The
measured cost of these checks (a few percent in the Swift and Rust data) is
accepted. Code that needs the last few percent uses `unsafe` with a
justification comment, in a trusted package, where `sisu audit` can find
it.

## Not finished

There is no compiler. The proposal is a hypothesis; the prototype is the
experiment; the open questions are the things the experiment must answer.
