# 05 - Errors

## The two kinds of failure

Every language studied eventually distinguishes two things. The field
studies of exception handling
([`../research/evidence/studies.md`](../research/evidence/studies.md))
find degenerate handlers under every regime, so no mechanism can claim to
reduce faults; what they show is that handlers are empty, over-broad or
missing wherever the language lets the error path be ignored at the point
it occurs. The design goal is therefore visibility and cheap correct
handling, and the distinction is:

- **Expected failures** that a correct program must handle: a file is
  missing, input is malformed, the network is down. These are part of a
  function's contract and must be visible in its type.
- **Bugs**: an index out of range, a violated invariant, a `None` that
  "cannot happen". The program is already in a state its author did not
  anticipate; the only safe response is to stop the affected unit of work.

Sisu calls the first *errors* and the second *panics*, and gives them
different mechanisms.

## Errors are values

A fallible function's return type is `T ! E`. This is `Result[T, E]`, an
ordinary enum `{ Ok(T), Err(E) }`, with sugar in signatures. The evidence
for errors-as-values over exceptions:

- Control flow is visible. Every call that can fail has a `?` or a `match`
  at the call site. Go's and Rust's designers both cite this as the primary
  reason, and the C++ community's partial retreat from exceptions
  (`-fno-exceptions` in Google, LLVM, game engines; Herb Sutter's P0709
  proposing *value-based* exceptions) confirms that even C++ practitioners
  want it ([`../research/languages/cpp.md`](../research/languages/cpp.md)).
- Exceptions compose badly with generics, closures and concurrency (Java's
  checked exceptions inside lambdas; C++ exceptions across threads; Swift's
  untyped `throws` until 2024).
- The studies on exception-handling bugs (Ebert et al., Cabral & Marques)
  find that catch blocks are routinely empty, over-broad, or wrong, because
  the language lets the error path be ignored at the point where it occurs.
  Errors-as-values do not remove the failure mode, they move it: Cloudflare's
  November 2025 outage was a `Result::unwrap()` on an unexpected `Err`. The
  consequence for Sisu is that `expect` and `unwrap` are panic sites that
  `sisu audit` lists, and that internal configuration is validated like
  user input.

The evidence against the naive version of errors-as-values comes from Go:
`if err != nil { return err }` is the number-one complaint in every Go
survey, and in 2025 the Go team formally stopped trying to fix it
([`go.md`](../research/languages/go.md)). The cause is the lack of a
propagation operator and of a type that forces the check. Rust's `?` and
`#[must_use]` are the fix and Sisu adopts both.

## Error sets

The second problem with errors-as-values is composing error types. Rust
users write `From` impls or reach for `anyhow`/`thiserror`; Java's checked
exceptions failed in part because every layer had to declare the union of
everything below it. Zig's error sets solve this and Sisu takes the design,
adding payloads (Zig's lack of them is its most-cited limitation):

```sisu
error IoError:
    NotFound(path: Path)
    PermissionDenied(path: Path)
    Interrupted

error ParseError:
    UnexpectedEnd
    BadDigit(at: int, found: char)

def load(fs: Fs, p: Path) -> Config ! {IoError, ParseError}: ...
```

Rules:

- An `error` declaration defines a named error set. Variants may carry
  data. Each variant is globally unique (`IoError.NotFound`), so sets can be
  unioned without collision.
- `{ A, B }` is the union of two sets. A set is a subtype of any set that
  includes all its variants, so `IoError` coerces to `{ IoError, ParseError }`
  with no conversion code. This is the only subtyping in the language
  besides refinement types.
- `T !` with no set means the set is inferred from the body: the union of
  every error returned or propagated. The compiler can print it; `sisu doc`
  shows it; the LSP shows it on hover. The public-API linter requires an
  explicit set on `pub` functions by default (an inferred set changes when a
  dependency's does, which is an API change), but this can be relaxed per
  package.
- Matching on an error set is exhaustive like any enum. Matching a subset
  and propagating the rest is the common pattern:

```sisu
let cfg: Config                        # definitely assigned below, or the function returns
match load(fs, path):
    case Ok(c):
        cfg = c
    case Err(NotFound(path)):
        cfg = Config.default_at(path)
    case Err(e):
        return Err(e)                  # e still has the full set type
```

- An error set may be *open* (`error AppError { ..., .. }`) to allow other
  packages to add variants; matching an open set requires a wildcard.
  Closed is the default.

## Propagation

`expr?` on a `T ! E` yields `T` or returns `Err(e)` from the enclosing
function, coercing `e` to the function's error set. `expr?` on a `T?` in a
function returning `T?` propagates `None`. The postfix form was chosen over
Zig's and Swift's prefix `try` because it chains: `a()?.b()?.c()?`.

`?` inside a closure propagates from the closure, which is why iterator
chains that can fail use `try_map`/`try_collect`, as in Rust.

## Handling

```sisu
let x = fallible() else as e: fallback(e)     # handle with the error in scope
let x = fallible() else: fallback              # handle without it
let x = fallible() else:                       # a block after else must diverge
    return Err(Unavailable)
let x = fallible().or(default)
let x = fallible().or_else(e -> recover(e))
let x = fallible().expect("invariant: config always loads")   # panics with message
let x = fallible()?                            # propagate
match fallible():
    case Ok(v): ...
    case Err(e): ...
```

`else` is the universal fallback form for both `T?` and `T ! E`. An
expression on the same line as `else:` is the fallback value; an indented
block after `else:` must diverge (`return`, `break`, `continue`, `panic`),
because blocks have no value (D-056); `else as e:` binds the error. This is
Rust's `let-else` with the error made available.

A `Result` or `Option` that is neither used nor explicitly discarded
(`_ = fallible()`) is a compile error, not a warning. Go's unchecked
errors, and Rust's experience that `#[must_use]` on `Result` is the single
most effective lint it has, decide this.

## Context and traces

A frequent complaint about errors-as-values is that the error loses the
story of where it happened. Sisu adopts two things:

- **Error return traces** (Zig): in debug and test builds, each `?` that
  propagates an error records the source location; `e.trace()` prints the
  chain. There is no runtime cost in release builds and no stack-unwinding
  machinery.
- **Context wrapping** as a standard-library convention, not a language
  feature: `fallible().context("loading {path}")?` wraps the error in a
  `Contextual` variant carrying the message. Rust's `anyhow` proved the
  pattern; putting one blessed version in `std` prevents the ecosystem
  churn (`failure`, `error-chain`, `anyhow`, `eyre`, `snafu`) that Rust went
  through.

## Panics

A panic is raised by failed bounds checks, arithmetic overflow, failed
`assert`, `unreachable`, `expect` on an error, failed contracts in checked
builds, and `panic("msg")`.

- A panic unwinds the current task, running `Drop` for every value on its
  stack, to the task's enclosing scope. The scope sees the panic as a
  `TaskPanicked` error on `join`. There is no `catch` for panics inside a
  task. This is Erlang's model (a process crashes; its supervisor decides)
  applied to tasks, and it is why structured concurrency and panics fit
  together: the scope *is* the supervisor
  ([06-concurrency-and-capabilities.md](06-concurrency-and-capabilities.md)).
- If `main`'s task panics, the process prints the panic message and a stack
  trace and exits with a non-zero status.
- The `freestanding` profile may be built with `panic = abort` (no unwinding,
  no `Drop`), as Rust and Zig allow.
- A panic in a `Drop` during unwinding aborts the process. Double-panic has
  no good answer in any language; abort is the honest one.

## What was rejected

| Alternative | Reason |
|---|---|
| Exceptions (Java, C#, Python, C++) | Invisible control flow; empty/over-broad catch blocks in the studies; interaction with generics and concurrency. |
| Checked exceptions (Java) | Right goal, wrong mechanism: no propagation operator, no union types, no inference, broke with lambdas. Error sets are checked exceptions done with types. |
| Untyped `throws` (Swift until 6, Go's `error` interface) | Callers cannot match on what can go wrong; Swift added typed throws in SE-0413 for exactly this reason. |
| Plain `Result[T, E]` with `From` conversions (Rust) | Composing error types is boilerplate; the ecosystem had to invent `thiserror`/`anyhow`. Error-set subtyping removes the need. |
| Condition system with restarts (Common Lisp) | Genuinely more powerful; see [`lisp-scheme-racket.md`](../research/languages/lisp-scheme-racket.md). Rejected because it requires dynamic scope and makes control flow non-local in ways that interact badly with ownership. Recorded as open question Q-36 for the application tier. |
| Multiple return values (Go) | A tuple `(T, error)` does not force the check; `Result` does. |
| Catchable panics (`catch_unwind`, Go `recover`) | Encourages using panics as exceptions. Only scopes observe panics. |

## Interaction with the rest of the language

- Contracts ([07-contracts.md](07-contracts.md)) panic when violated; they
  are bugs, not errors.
- A `Drop` cannot return an error; a type whose cleanup can fail (a file
  needing flush) provides an explicit `close(own self) -> ! IoError`, and
  the linter warns if a value of such a type is dropped without being
  closed (linear-type discipline, from Austral, applied as a lint rather
  than a type rule; see open question Q-37).
- Capabilities ([06](06-concurrency-and-capabilities.md)) return `IoError`
  families defined in `std`; third-party code extends them with its own
  sets, which union freely.
