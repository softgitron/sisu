# Compile-time code instead of macros

Sisu has no macros. Code that writes code is ordinary Sisu that the
compiler runs: `comptime` evaluates a call while the program compiles,
reflection reads a type's fields and attributes as values, and a trait's
`derive` is a compile-time function that returns an `impl`. This page
explains why Sisu chose that over the macro systems of other languages,
what it covers and what it leaves out, what it costs, and what would
bring macros back.

**See also:** the tutorial's [chapter
25](../tutorial/25-compile-time.md) (compile-time code); the reference's
[R12, Compile-time evaluation and
reflection](../reference/compile-time.md); the proposal's [08,
Metaprogramming](../../proposal/08-metaprogramming.md) and decisions
D-018, D-046 and D-063 in its [decision
log](../../proposal/15-decision-log.md); the research in
[metaprogramming.md](../../research/topics/metaprogramming.md).

## The problem: what macros are for, and what they cost

Programs need code that writes code for a short list of jobs: the same
method for every field of a struct (equality, hashing, printing,
serialising), tables computed from data, code that differs by platform,
small languages inside the language (a regular expression, a query, a
format string), and new shapes of control flow. Every mainstream
language answers some of the list with a macro system, and each kind of
macro has a documented cost
([metaprogramming.md](../../research/topics/metaprogramming.md)).

C's textual preprocessor is regretted so widely that both C and C++ keep
trying to replace it. C++ templates turned out to be a programming
language by accident, with errors reported deep in an instantiation and
fifteen years of work before concepts could state what a template needs.
Rust's procedural macros are the most powerful macros in mainstream use,
behind `serde`, `tokio` and `sqlx`, and the feature most often blamed
for its compile times: each lives in a crate of its own and parses its
input again on every use, and tools cannot see what it generates without
running it. Swift's macros reproduced the build cost, and Scala 2's
macros held back the move to Scala 3 for years. Dart spent three years
on macros that could see types and cancelled them in 2025, because they
could not be made fast enough for hot reload. D's designers call their
string mixins, which paste generated text into the program, a mistake.

The most expressive kind, the AST macros of Lisp, Elixir and Julia, has
the strongest evidence of a different cost. When every library can
extend the language cheaply and nothing curates the extensions,
libraries grow incompatible dialects of the same idea, the pattern known
as the Lisp curse. Elixir shows that AST macros can carry a healthy
ecosystem when the core library is strongly curated, and Lisp shows the
alternative (finding 5).

One form of metaprogramming has no regrets in the record: running
ordinary code at compile time. D did it a decade before Zig, and Zig,
Jai, C++'s `constexpr`, Nim's virtual machine and Lean all converged on
it; the regrets are about what was built on top, such as string mixins
and generics checked only at instantiation (finding 1).

## What Sisu has instead

`comptime` asks the compiler to evaluate an expression while compiling,
and a `const` item or an array length requires it (D-018). The evaluator
is an interpreter over the same typed IR as the program, so what runs at
compile time is exactly what would run at run time: the same integer
widths, overflow traps and drop order, and, since D-074, the same
dictionary order and the same floating-point bits
([Determinism](determinism.md)). Compile-time code is sandboxed and
bounded, because F#'s type providers, Rust's build scripts and npm's
install scripts show what unbounded build-time code costs (finding 4).
It has no I/O: `embed("path")` reads a file into the program and records
it among the build's inputs, so builds stay reproducible. And it has a
budget counted in steps of the IR, owned by the item that spends it:
`@comptime_budget(steps=N)` on a package's own constant raises its own
limit, so that no dependent ever edits its manifest for a library's
internals, which is why Rust 1.72 removed its global limit (D-063).

Reflection turns types into values. `reflect.fields_of(T)` lists a
struct's fields, `reflect.attributes_of(T)` its attributes, and
`comptime for` unrolls a loop over them, with `value.[f]` reading the
field that `f` names. This function prints any struct it is given:

<!-- file: examples/explanation-compile-time/describe.sisu -->
```sisu
# One ordinary generic function, unrolled at compile time over the
# fields of whatever struct it is given: no macro, no second language.
def describe[T: Debug](value: T) -> String:
    var parts: list[String] = []
    comptime for f in reflect.fields_of(T):
        parts.push("{f.name}={value.[f]:?}")
    return "{reflect.name_of(T)}({parts.join(", ")})"

struct Booking:
    name: String
    hour: int

struct Stove:
    heat: f64
    lit: bool

def main():
    print(describe(Booking(name="Aino", hour=18)))
    print(describe(Stove(heat=82.5, lit=True)))
```

<!-- output: examples/explanation-compile-time/describe.out -->
```text
Booking(name="Aino", hour=18)
Stove(heat=82.5, lit=True)
```

Zig's standard library encodes JSON this way, which the research takes
as the proof that serialisation, derives and formatting need no macros,
and those are exactly the jobs that Rust's heaviest users of procedural
macros do (finding 3).

Reflection stops at one line: it computes values and types, and it
cannot add declarations to a module. A compile-time function may compute
an implementation, but it cannot inject a new `struct` or function into
your code. The one exception is a trait's `derive`, a `comptime def`
that returns `impl Trait for T:`, which `@derive(Trait)` calls; it can
produce only an `impl` of its own trait for the type it was given, so
coherence holds, and any trait can have one, so the standard library's
derives have no privilege over a user's. Attributes are data, not calls
(D-046). `@table(bookings)` on a struct is something that compile-time
code reads with `reflect.attributes_of`, not a function that runs, which
is the difference from Python's decorators, whose effects type checkers
needed `dataclass_transform` to see through.

The rest of the list has answers that are not macros. Code for each
platform is `if comptime target.os == "linux":`, with both branches
checked, as Zig's `comptime if` and D's `static if` do, rather than C's
`#ifdef`. A new control structure is a function with a trailing block,
`retry(3) do:`. And a small language inside the language is a string
literal that a compile-time function parses, with errors pointing into
the string, as Perl's regular expressions and C#'s interpolated string
handlers check theirs (finding 6); those checked literals are designed
and not yet in stage 0.

## What it leaves out

The line that keeps reflection from becoming a macro system also limits
what it can do. There is no new syntax: no `sql!(...)`, no `html! {}`,
and no new kind of statement, only strings checked at compile time and
trailing blocks. Compile-time code cannot generate a set of functions or
a struct for a module; it can generate values, tables and the `impl` of
a derive, and generics cover the rest. Reflection at run time is
minimal, a type's name and an enum's variant names, because the run-time
reflection of Java and C# defeats dead-code elimination and would give
code a way around the capability model. And there are no build scripts:
code that must be generated outside the language is a separate tool that
you run, whose output is ordinary source.

## What it costs

Errors inside reflective code are found late. Sisu checks a generic
function's body once, against its bounds, so that a library's errors
never appear at its users' call sites (D-017). Code unrolled by
`comptime for` is the exception: its body is checked once for each
struct it is unrolled for, which is how Zig checks all its generics.
This function works for one struct and not for the other:

<!-- file: examples/explanation-compile-time/total.sisu -->
```sisu
# Code unrolled over a struct's fields is checked for each struct it is
# unrolled for, so a field that does not fit is found at the call.
def total[T](value: T) -> int:
    var sum_so_far = 0
    comptime for f in reflect.fields_of(T):
        sum_so_far += value.[f]
    return sum_so_far

struct Scores:
    aino: int
    eero: int

struct Team:
    name: String
    points: int

def main():
    print(total(Scores(aino=3, eero=4)))
    print(total(Team(name="Owls", points=7)))
```

<!-- output: examples/explanation-compile-time/total.console -->
```console
$ sisu check total.sisu
error[E0310]: `+=` cannot be applied to `int` and `String`
  --> total.sisu:19:11
   |
19 |     print(total(Team(name="Owls", points=7)))
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
 6 |         sum_so_far += value.[f]
   |         ----------------------- the template's line that wrote it
  = note: in the instance of `total[Team]` that this call makes: `sum_so_far += value.name`

```

E0310 (`type.binary-operator`) is right that a `String` cannot be added
to an `int`. It stands at line 19, the call that asked for `Team`, and
its note quotes the line that the compiler wrote by unrolling the loop
for `Team`, since that line exists only in the instance. A label
points at line 6, the statement of the loop that wrote it, "the
template's line that wrote it" (`type.instance-line`), so the reader
sees both the line that failed and the line to change. What the error
cannot say is which struct the template's author meant to refuse:
`total` promised nothing about `T`, so `Team` reaches the loop, and the
error is found at the call, for the type, rather than at the template's
signature. That is the price of checking at instantiation.

There are other costs. Compile-time code runs at every build that needs
its result, and a budget bounds it but does not make it free. Some
libraries are harder to write than with macros: the research names an
HTML templating library and an embedded query language as the first
candidates that compile-time code, reflection and trailing blocks may
not serve well (Q-15). Ergonomics that Rust users take for granted, such
as field attributes that tune a derived serialiser, are possible with
attributes read as data, but less proven.

## What would change it

Q-15 asked whether hygienic macros should follow version 1, and the
owner answered no, to be revisited only when the validation corpus
produces a need that compile-time code and reflection cannot meet. The
first tests are named: an HTML templating library and an embedded query
language. If macros ever come, the research says how: hygienic and
declarative by default, a procedural tier marked separately, their
expansion visible to tools, and a strongly curated standard library
against the Lisp curse, with the cost model decided before the feature
is promised, as Dart's cancellation warns (finding 2). Q-27 has already
allowed one extension, compile-time functions that construct types, with
every generic bound still going through traits; a library that builds
types that way, such as one for units of measure, is what
`@diagnostic_name` is designed to make readable in errors (D-065).
