# Error handling

## The question

How a function reports that it could not do its job, how callers are made
to notice, how error types compose across layers, and what happens when
the program is in a state its author did not anticipate.

## Approaches found

| Approach | Languages | Buys | Costs | Evidence |
|---|---|---|---|---|
| Unchecked exceptions | C#, Python, Ruby, JS, C++, Kotlin, Scala, Swift pre-2 | Terse happy path; stack traces | Invisible control flow; empty or over-broad catch blocks; cross-thread and generic composition problems; 52% of C++ shops ban them (P0709 survey) | [cpp.md](../languages/cpp.md), [csharp.md](../languages/csharp.md), [python.md](../languages/python.md) |
| Checked exceptions | Java | Failures visible in signatures | No propagation operator, no union/inference, broke with lambdas; universally cited as the failed experiment (Hejlsberg 2003) | [java.md](../languages/java.md) |
| Untyped `throws` | Swift 2-5, Go's `error` interface | Visible fallibility, cheap | Callers cannot match on what can fail; Swift added typed throws (SE-0413) after nine years | [swift.md](../languages/swift.md), [go.md](../languages/go.md) |
| `(T, error)` tuples by convention | Go | Visible at every call site | Nothing forces the check; `if err != nil` is the top survey complaint; the Go team froze the syntax in 2025 after `try`, `check/handle` and others failed | [go.md](../languages/go.md) |
| `Result[T, E]` sum type + `?` | Rust, Gleam, Roc, Elm, Haskell (`Either`), OCaml | Forced handling, propagation operator, exhaustive matching; Rust's `?` is its most-copied feature | Composing error types needs `From` impls or `anyhow`/`thiserror`; Gleam has `Result` without `?` and leans on `use` | [rust.md](../languages/rust.md), [gleam.md](../languages/gleam.md), [roc.md](../languages/roc.md) |
| Error sets with inference | Zig; Nim's tracked exception sets; Roc's inferred tag unions | Error types compose by set union; inferred sets remove boilerplate; error return traces | Zig errors carry no payload (its most-cited limitation) | [zig.md](../languages/zig.md), [nim.md](../languages/nim.md), [roc.md](../languages/roc.md) |
| Typed effects | Koka, Effekt, Flix, Unison | Exceptions as one effect among many; effect-polymorphic higher-order functions solved | Signature noise; no mainstream production use | [koka-effekt.md](../languages/koka-effekt.md), [flix.md](../languages/flix.md) |
| Condition system with restarts | Common Lisp | Signaller offers recoveries, handler chooses, no unwinding | Dynamic scope; not integrated with static types or ownership anywhere | [lisp-scheme-racket.md](../languages/lisp-scheme-racket.md) |
| Let it crash + supervision | Erlang/Elixir | Decades of reliability record; isolation as the unit of failure | Needs isolated processes; not a model for *expected* failures | [erlang-elixir.md](../languages/erlang-elixir.md) |

## What the evidence says

1. **Every mainstream language is converging on a typed value channel plus
   a panic channel.** Swift added typed throws (2024); Kotlin, which chose
   unchecked exceptions by design, is adding typed error unions fifteen
   years in (KEEP-0462); C# 15 previews union types motivated by
   result-or-error; C++ 26 contracts and P0709's value-based exceptions; Go
   gave up on syntax but not on the complaint ([kotlin.md](../languages/kotlin.md),
   [csharp.md](../languages/csharp.md), [cpp.md](../languages/cpp.md)).
   Nim, which has tracked exceptions, still ended up with `Option`/`Result`
   libraries and a `Defect`/`Exception` split ([nim.md](../languages/nim.md)).
   The lesson from D's Phobos v3 and from C++ is to ship *one* mechanism
   rather than three parallel channels.
2. **Checked exceptions failed on mechanism, not goal.** Hejlsberg's two
   objections, versionability and scalability, are answered by inference
   (Zig, Nim, Roc) and by union/subtyping of error sets, not by giving up on
   declared errors ([java.md](../languages/java.md)).
3. **A propagation operator is not optional.** Go's history is the
   controlled experiment: errors-as-values without `?` produces the
   complaint that every Go survey records, and cannot be fixed after 1.0
   because any new syntax becomes de facto mandatory and splits the
   ecosystem (Griesemer 2025) ([go.md](../languages/go.md)).
4. **Must-use is the single most effective lint.** Rust's `#[must_use]`
   on `Result`, Gleam's unused-result warnings on by default, and the
   exception-handling bug studies (empty catch blocks) all say the language
   has to refuse silent discard ([../evidence/studies.md](../evidence/studies.md)).
5. **Error composition is where `Result` alone falls short.** Rust's
   ecosystem went through `failure`, `error-chain`, `anyhow`, `eyre`,
   `thiserror` ([rust.md](../languages/rust.md)); Zig's error sets and Roc's
   inferred tag unions compose by construction; Zig's missing payloads are
   the one thing to add ([zig.md](../languages/zig.md), [roc.md](../languages/roc.md)).
6. **Where the error came from matters.** Zig's error return traces give
   the location chain at near-zero success-path cost; Rust needed `anyhow`'s
   backtraces; Go errors have no location unless wrapped
   ([zig.md](../languages/zig.md)).
7. **Bugs and failures need different mechanisms.** Erlang's isolation,
   Rust's panics, Zig's checked illegal behaviour, Gleam's panic-vs-Result
   split, and Go's `recover` misuse (panics used as exceptions) agree:
   expected failures are values, bugs stop the unit of work, and the unit
   of work should be small (a task, not the process)
   ([erlang-elixir.md](../languages/erlang-elixir.md), [gleam.md](../languages/gleam.md)).
8. **The condition system is genuinely more powerful and nobody has made
   it fit a typed, ownership-based language.** Its "signaller offers,
   handler chooses" separation can be approximated with error values that
   carry recovery options ([lisp-scheme-racket.md](../languages/lisp-scheme-racket.md)).

## Recommendation for Sisu

`T ! E` where `E` is an error set: variants with payloads, structurally
subtyped so sets union without conversion code, inferred when the set is
omitted; postfix `?` propagation; `else as e:` handling; `Result`
and `Option` must be used; error return traces in debug builds and a
standard `context` wrapper; panics unwind the task to its structured scope
and cannot be caught inside the task. No exceptions, no checked exceptions,
no untyped `throws`. See [proposal/05](../../proposal/05-errors.md);
decisions D-011, D-040, D-042.

## Open questions

- Q-7 residual syntax (inferred sets on public functions; `else` vs
  `catch`; `?` on `Option` in a `Result` function); Q-36 a typed restart
  mechanism; Q-37 must-consume resource types.
