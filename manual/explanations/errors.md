# Errors without exceptions, absence without null

Sisu has no `null` and no exceptions. A value that may be missing has
the type `T?`, a function that may fail has the type `T ! E`, whose
errors are ordinary values drawn from error sets, and a bug panics,
which stops the task it happens in. This page explains why the language
splits failure three ways, what the alternatives cost the languages that
chose them, what Sisu's design costs in turn, and what would change it.

**See also:** the tutorial's [chapter 8](../tutorial/08-errors.md)
(absent values and errors) and [chapter
20](../tutorial/20-concurrency.md) (panics in tasks); the reference's
[R2, Types](../reference/types.md#optional-values), with optional
values, results and error sets, and [R8, Errors and
panics](../README.md#the-reference); the proposal's [05,
Errors](../../proposal/05-errors.md) and decisions D-010, D-011, D-040
and D-042 in its [decision log](../../proposal/15-decision-log.md); the
research in
[error-handling.md](../../research/topics/error-handling.md).

## Two kinds of failure

Every language studied ends up telling two things apart. Some failures
are expected, and a correct program must handle them: a file is missing,
input is malformed, the network is down. Others are bugs: an index out
of range, a broken invariant, a case that "cannot happen". After a bug
the program is in a state its author did not foresee, and the only safe
response is to stop the work that went wrong. Sisu calls the first kind
errors and the second panics, and gives them different mechanisms
([05](../../proposal/05-errors.md#the-two-kinds-of-failure)).

What the evidence says about handling them is mostly about attention.
Field studies of exception handling find that handlers are empty,
over-broad or missing under every regime: in Cabral and Marques's study
of 32 Java and .NET programs, fewer than one handler in ten attempted
any recovery, and Ebert, Castor and Serebrenik's 220 exception bugs in
Eclipse and Tomcat frequently involved missing or swallowed handling
([studies.md](../../research/evidence/studies.md), section 8). No
mechanism makes programmers handle failures well. What a language can do
is make the failure visible where it happens and make correct handling
cheap, so that ignoring it takes a decision. That is the goal behind
each rule on this page.

## Why there is no null

Tony Hoare added the null reference to ALGOL W in 1965 because it cost
little to implement, and in 2009 called it his billion-dollar mistake:
when null is a member of every type, every use of every reference needs
a check that the type does not ask for
([designer-retrospectives.md](../../research/evidence/designer-retrospectives.md)).
The alternative, a type that says "maybe" and a test to get the value
out, was known in 1965. Every later designer agreed with him. Kotlin and
Swift shipped with types that exclude null; C# 8, Dart and TypeScript's
`strictNullChecks` retrofitted them at a cost; and Java's `Optional` was
meant, in Brian Goetz's words, as "a limited mechanism for library
method return types", so that thirty years after Java 1.0 its
null-restricted types are still a draft
([java.md](../../research/languages/java.md)).

So in Sisu a value that may be absent has the type `T?`, which is the
ordinary enum `Option[T]` with `Some` and `None` (D-010). An `int` is
always an int. Rust shows that such an enum is enough when the language
gives it good syntax, and Kotlin and Swift show which syntax: `x?.f` for
"f of x, if x is present", `a ?? b` for a default, `if let` to test and
bind at once, and `let ... else` to leave early when the value is
missing.

Absence and failure are different things, and Sisu keeps them apart.
Kotlin's proposal for typed errors puts the problem with null plainly:
it is ambiguous between "no value" and "error"
([kotlin.md](../../research/languages/kotlin.md)). This function reads a
setting that may be missing, which is normal, and that may be malformed,
which is an error that says why:

<!-- file: examples/explanation-errors/setting.sisu -->
```sisu
# A missing setting is `None`; a setting that does not parse is an
# error that says why. The types keep the two apart.
error SettingError:
    NotANumber(key: String, text: String)

def timeout(settings: dict[str, str]) -> int? ! SettingError:
    let text = settings.get("timeout") else:
        return Ok(None)
    let seconds = text.parse_int() else:
        return Err(NotANumber("timeout".to_string(), text.to_string()))
    return Ok(Some(seconds))

def main():
    print(timeout({"timeout": "30"}))
    print(timeout({"retries": "3"}))
    print(timeout({"timeout": "half a minute"}))
```

It prints the three outcomes, each in its own shape:

<!-- output: examples/explanation-errors/setting.out -->
```text
Ok(Some(30))
Ok(None)
Err(NotANumber(key="timeout", text="half a minute"))
```

`int? ! SettingError` reads "an optional int, or error `SettingError`".
A caller that only wants a number writes `timeout(settings)? ?? 60`:
pass the error on, and use 60 when the setting is absent. With null
standing for both, the caller could not tell the two apart.

## Why there are no exceptions

An exception travels from wherever it is thrown to whichever handler
catches it, through every frame between, and nothing at a call site says
that it might. That invisible path is the first objection. The second is
that exceptions compose badly with the rest of a modern language: Java's
checked exceptions do not pass through lambdas, C++ exceptions do not
cross threads, and Swift's `throws` named no type until 2024. The third
is practice: in a survey for the C++ proposal P0709, 52% of C++ shops
banned exceptions, and Google and LLVM compile without them
([error-handling.md](../../research/topics/error-handling.md)).

Java tried to make exceptions visible by checking them, and its failure
is the most cited experiment in the field. Anders Hejlsberg named the
two reasons in 2003: versionability, since adding an exception to a
`throws` clause breaks every caller, and scalability, since in a large
system the lists grow to dozens of types and programmers give up and
write `throws Exception` or an empty `catch`. The research's verdict is
that checked exceptions failed on mechanism, not on goal (finding 2):
there was no propagation operator, no union of exception types, and no
inference. Sisu's error sets are checked exceptions done with types, and
each part answers one of those gaps.

The rest of the field has been moving the same way. Swift added typed
throws after nine years, Kotlin, which chose unchecked exceptions by
design, is designing typed error unions fifteen years in, and C# is
previewing union types for results (finding 1). Sisu starts where they
are going.

## Error sets, and the operator that passes them on

A fallible function returns `T ! E`, which is `Result[T, E]`, an
ordinary enum with `Ok` and `Err` (D-011). `E` is an error set: a
declaration like an enum's, whose variants may carry data. Sets combine
by union, `{IoError, ParseError}`, and a set is a subtype of any set
that contains all of its variants, so a function that can fail in two
ways returns errors of either kind with no conversion code. This is
Zig's design, with the payloads Zig lacks, which are its most cited
limitation. Rust, with a plain `Result`, needs `From` impls to turn one
error type into another, and its ecosystem went through `failure`,
`error-chain`, `anyhow`, `eyre` and `thiserror` looking for a better way
(finding 5). Writing `T !` with no set asks the compiler to infer the
union from the body, which answers Hejlsberg's scalability objection;
for a `pub` function a lint asks for the set to be written out, because
an inferred set changes when a dependency's does, and that is an API
change (Q-7).

Propagation is a postfix `?`: `x?` is "x, or return early" with the
error (D-042). Go shows what errors as values cost without it.
`if err != nil { return err }` has topped Go's user surveys as the main
complaint for years, and in June 2025 the Go team stopped looking for a
syntax, partly because, in its words, "virtually everybody will need to
start using it, lest their code become unidiomatic", which splits an
installed base (finding 3). A propagation operator has to exist before
the first release, so Sisu has one. It is postfix rather than Zig's and
Swift's prefix `try` because a postfix operator chains:
`a()?.b()?.c()?`.

A result has to be used. Discarding a `T ! E` or a `T?` without saying
so, as `_ = save(file)`, is a compile error, not a warning. Rust's
experience is that `#[must_use]` on `Result` is the most effective lint
it has, and the empty `catch` blocks of the field studies are the same
mistake in another form (finding 4). And an error says where it came
from: in debug and test builds each `?` that passes one on records its
place, which the error's trace prints, as Zig's error return traces do,
and `.context("loading {path}")` wraps an error with a sentence of its
own, as Rust's `anyhow` does, kept in the standard library so that the
ecosystem has one way.

## Panics are bugs

A failed bounds check, an integer overflow, a failed `assert` or
contract, an `expect` on an error and `panic("...")` are panics. A panic
unwinds the task it happens in, running the drop of every value on the
task's stack, and stops at the task's scope, which sees it as
`TaskPanicked` when it joins the task (D-040). Nothing inside a task can
catch a panic. Go's `recover` and Rust's `catch_unwind` show what a
catchable panic becomes: an exception under another name. Erlang shows
the alternative that works: the unit of failure is small and isolated,
and a supervisor outside it decides what happens next. In Sisu the
supervisor is the scope that started the task ([chapter
20](../tutorial/20-concurrency.md) shows a scope watching a task that
panics). When `main` itself panics, the program prints the message and a
trace and exits with a status that is not 0.

Unwinding leaves one problem, which Rust solves by poisoning a mutex
that was locked when its task panicked: the data behind the lock may be
half updated. Sisu re-checks the protected type's `invariant` instead,
and poisons the mutex only if the check fails (D-077), so a panic that
left the data consistent does not take the data with it; a type with no
invariant unlocks as usual.

Errors as values move a failure mode rather than remove it. In November
2025 a Cloudflare outage came from a `Result::unwrap()` on an `Err` that
the code assumed could not happen
([studies.md](../../research/evidence/studies.md), section 8). In Sisu
`expect` is the spelling of that assumption, and the design has
`sisu audit --panics` list every place a function can panic, so that the
assumptions can be read in one list.

## What it costs

- Every fallible call is marked. Code that would call five functions in
  a row under exceptions writes five `?`s, and a function that learns a
  new way to fail changes its type, and so every caller's. That is the
  point of the design, and it is also work that exceptions do not ask
  for.
- The error set is part of a public interface. Adding a variant to a
  closed set is a breaking change, as adding an exception to a Java
  `throws` clause was. An error set can be declared open, with a
  trailing `..`, so that other packages may add variants, at the price
  that every match on it needs a wildcard.
- A `?` inside a closure returns from the closure, not from the function
  around it, so an iterator pipeline that can fail needs `try_map` and
  `try_collect`, as in Rust; stage 0 does not have them yet.
- Cleanup cannot fail in a drop. A type whose cleanup can fail, such as
  a file with a final flush, offers `close(own self) -> ! IoError` and
  is marked `@must_consume`, so that forgetting to close it is a compile
  error (D-054).
- There is no way to recover from a panic where it happens. Code that
  must survive a bug in a library runs that library's work in a task and
  lets the scope decide; code that wants to retry uses an error, not a
  panic.
- Sisu has no condition system. Common Lisp's lets the code that fails
  offer ways to recover and a handler further up choose one, without
  unwinding, which is more powerful than errors or exceptions. It needs
  dynamic scope, which fits neither static types nor ownership, and the
  owner decided against it (Q-36).

## What would change it

The design commits to errors as values, error sets and panics; what
stays open is detail, to be settled by evidence from code written in
Sisu. The standard library is the first large body of fallible code, so
the friction of `else`, `match` and `?` is counted while it is written,
and the residual syntax of Q-7 follows from the count: whether an
inferred set on a `pub` function is a lint or an error, whether
`else as e:` is the right form for a handler, and whether `?` on an
optional inside a function that returns a result should convert `None`
itself. The validation plan names the threshold for the first: an
inferred set on a public function that changes and breaks a dependent
reopens the question of making the lint an error
([16](../../proposal/16-validation-plan.md#then-in-parallel)).

How a program reports an error that `main` returns is open too (Q-74).
Stage 0 prints the error's `Debug` form, every field and no sentence,
where readers of this manual's examples expect the message a person
would write; the lean is to print the `Display` form with the trace.

A condition system would return only with a typed design that fits
ownership, which nobody has shown. And null and exceptions would return
only if the languages now adding typed error channels and null-free
types turned back, which none has done.
