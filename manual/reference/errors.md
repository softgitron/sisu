# Errors and panics

This page states how a program fails: errors, which are values that a
function's type declares and its callers handle, and panics, which are
bugs that stop the task. It covers error sets and their unions,
propagation and inferred sets, results that must be used, traces and
context, how an error that reaches `main` ends the program, unwinding,
aborts and exit statuses, and cleanup that can fail.

**See also:** the tutorial's [chapter 8](../tutorial/08-errors.md); the
proposal's [05, Errors](../../proposal/05-errors.md); [R2](types.md#results-and-error-sets)
for the types; [R4](expressions.md#optional-values-and-results) for `?`,
`else` and the other operators; [R9](concurrency.md#panics-in-tasks) for
panics in tasks; [R17, Diagnostics](diagnostics.md#codes-and-their-ranges)
for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Errors as values and panics for bugs are decided (D-011,
D-040, D-042).

## Errors and panics

An error is an expected failure that a correct program handles: a file
that is missing, input that does not parse. It is a value, in the
result of the function that can fail, `T ! E`, and the caller sees it in
the type. A panic is a bug: an index out of range, an overflow, a
failed assertion, a `todo` reached. It stops the task that hit it,
running its drops, and nothing catches it. There are no exceptions.

**Status.** In stage 0 (D-011, D-040).

## Error sets

`error NAME:` declares an error set, whose variants may carry fields
([R3](declarations.md#error-sets)). In a type:

- `T ! E` is a result of `T` or an error of the set `E`, and `! E` one
  of nothing or an error;
- `T ! {A, B}` takes the errors of both sets: a union, which stands
  only after `!` (E0227);
- `T !` with no set infers the set from the body, the union of every
  error it returns or passes on (E0226 anywhere but a function's result);
- a set is a subtype of every set that holds all its variants, so an
  error passes into a wider set with no conversion code.

A `match` on an error covers its variants like any enum's; matching
some variants and passing the rest on is the common shape. A set whose
body ends with `..` is open to other packages, and a `match` on it needs
a `case _` arm (E0356).

<!-- file: examples/reference-errors/error_sets.sisu -->
```sisu
# Matching some errors of a set and passing the rest on.
error ConfigError:
    Missing(key: String)
    Invalid(key: String, value: String)

def setting(text: str, key: str) -> String ! ConfigError:
    for line in text.lines():
        let k, v = line.split_once("=") else:
            continue
        if k.trim() == key:
            return Ok(v.trim().to_string())
    return Err(Missing(key.to_string()))

def port(text: str) -> int !:
    let raw = setting(text, "port")?
    let n = raw.parse_int() else:
        return Err(Invalid("port".to_string(), raw))
    return Ok(n)

def port_or_default(text: str) -> int ! ConfigError:
    return match port(text):
        case Err(Missing(_)): Ok(8080)
        case other: other

def main():
    for text in ["port = 443", "host = a", "port = x"]:
        print(port_or_default(text))
```

<!-- output: examples/reference-errors/error_sets.out -->
```text
Ok(443)
Ok(8080)
Err(Invalid(key="port", value="x"))
```

**Stage 0 differs.** 05 has the public-API linter ask for an explicit
set on a `pub` function; stage 0 takes an inferred set there.

**Status.** In stage 0 (D-011).

## Propagation

`x?` on a `T ! E` gives the value, or returns `Err(e)` from the
function with `e` widened to the function's set; on a `T?` it gives the
value or returns `None` ([R4](expressions.md#optional-values-and-results)).
An error that the function's set cannot hold is E0321, whose fix widens
the set, and `?` in a function whose result is no result is E0320
([R17](diagnostics.md#e0320)). `?` inside a closure returns from the
closure.

**Stage 0 differs.** `try_map` and `try_collect`, which pass an error
out of an iterator's pipeline, are E0304.

**Status.** In stage 0 (D-042).

## Results that must be used

A result or an optional value that a statement makes and nothing uses
is E0318, since an ignored error is the most common way errors are lost;
`_ = VALUE` discards one on purpose:

<!-- file: examples/reference-errors/unused.sisu -->
```sisu
# A result that nobody looks at does not compile.
error Closed:
    Shop

def open_at(hour: int) -> ! Closed:
    if hour < 8:
        return Err(Shop)
    return Ok(())

def main():
    open_at(6)
    _ = open_at(7)
```

<!-- output: examples/reference-errors/unused.console -->
```console
$ sisu run unused.sisu
error[E0318]: this `! Closed` is not used; handle it, or discard it with `_ = ...`
  --> unused.sisu:11:5
   |
11 |     open_at(6)
   |     ^^^^^^^^^^
  = fix: discard it explicitly: `_ = `

error[D0006]: the program was not run: errors found (1)
```

**Status.** In stage 0.

## Traces and context

Every error carries a return trace: the places where `?` passed it on,
innermost first, which `e.trace()` gives as text. `x.context("what was
happening")` wraps an error in `Contextual`, whose message names what the
program was doing, and the trace goes on from there; `{e}` shows the
message with the error, as in `reading line 2: NotANumber(text="kolme")`
([chapter 8](../tutorial/08-errors.md#context-and-traces) shows one).

**Status.** In stage 0.

## When main fails

`main` may return `! E`. When it returns `Err(e)`, the program prints
`error:` and the error on the standard error, then its trace, and exits
with status 1; `Ok(())` exits with 0.

<!-- file: examples/reference-errors/traces.sisu -->
```sisu
# An error that reaches `main` ends the run with its trace and status 1.
error BankError:
    TooLittle(balance: int, wanted: int)

def withdraw(balance: int, wanted: int) -> int ! BankError:
    if wanted > balance:
        return Err(TooLittle(balance, wanted))
    return Ok(balance - wanted)

def pay_twice(balance: int, price: int) -> int ! BankError:
    let once = withdraw(balance, price)?
    return Ok(withdraw(once, price)?)

def main() -> ! BankError:
    print(pay_twice(100, 30)?)
    print(pay_twice(100, 60)?)
    return Ok(())
```

<!-- output: examples/reference-errors/traces.console -->
```console
$ sisu run traces.sisu
40
error: TooLittle(balance=40, wanted=60)
  at traces.sisu:12:15
  at traces.sisu:16:11
```

**Stage 0 differs.** The error is printed in its `Debug` form, so a
`Contextual` error shows its fields rather than its message; whether
the `Display` form is printed instead is Q-74.

**Status.** In stage 0 (Q-74 open).

## Panics and unwinding

`panic(message)` panics, and so do the failed checks of the language:
an overflow, an index out of bounds, a division by zero, a failed
`assert`, a typed hole reached, a contract broken. A panic unwinds the
task: every value on the way is dropped, in the order its scope would
drop it, and every `defer` runs. Then a panic in `main` prints `panic:`,
the message and the trace of the calls on the standard error, and the
program exits with status 101; one in a task reaches its scope
([R9](concurrency.md#panics-in-tasks)). `unreachable()` panics with
the message that the code was not to be reached.

<!-- file: examples/reference-errors/unwind.sisu -->
```sisu
# A panic unwinds the task, dropping each value on the way, and stops.
struct Guest:
    name: String

impl Drop for Guest:
    def drop(mut self):
        print("{self.name} goes home")

def serve(count: int) -> int:
    let late = Guest("Eero")
    if count > 2:
        panic("too many guests: {count}")
    return count

def main():
    let early = Guest("Aino")
    print(serve(1))
    print(serve(5))
```

<!-- output: examples/reference-errors/unwind.console -->
```console
$ sisu run unwind.sisu
Eero goes home
1
Eero goes home
Aino goes home
panic: too many guests: 5
  at serve (unwind.sisu:12:9)
  at main (unwind.sisu:18:11)
```

A must-consume value need not be consumed while unwinding
([R7](ownership.md#values-that-must-be-consumed)).

**Status.** In stage 0 (D-040).

## Aborts

A panic during unwinding, such as one in a `Drop` that unwinding runs,
aborts the program at once, with no more drops (R0014), and the status
is 134. A build with `panic = abort` aborts at the first panic instead
of unwinding.

<!-- file: examples/reference-errors/double_panic.sisu -->
```sisu
# A panic while unwinding aborts the program.
struct Fragile:
    name: String

impl Drop for Fragile:
    def drop(mut self):
        panic("{self.name} broke while being put away")

def main():
    let cup = Fragile("the cup")
    panic("the shelf fell")
```

<!-- output: examples/reference-errors/double_panic.console -->
```console
$ sisu run double_panic.sisu
panic while unwinding: the cup broke while being put away
  the first panic: the shelf fell
  a panic during unwinding aborts the program
```

**Stage 0 differs.** `panic = abort`, a setting of a build profile in
[12](../../proposal/12-interop-and-targets.md), is designed.

**Status.** In stage 0, except `panic = abort`.

## Exit statuses

| Status | When |
|---|---|
| 0 | `main` returned, or returned `Ok(())` |
| 1 | `main` returned `Err(e)`, or the program did not compile ([D0006](diagnostics.md#d0006), [D0021](diagnostics.md#d0021)) |
| 2 | a usage error of the command line ([R16](sisu-command.md#exit-statuses)) |
| 101 | a panic that unwound out of `main` |
| 134 | an abort: a panic during unwinding, or a deadlock ([R9](concurrency.md#the-scheduler)) |

**Status.** In stage 0.

## Cleanup that can fail

A `Drop` cannot fail, so cleanup that can, such as flushing a file or
committing a transaction, is a method that consumes the value and
returns a result: `close(own self) -> ! E`. When the type is
`@must_consume`, every value of it is consumed by such a method on every
path, and the caller handles the error (D-054;
[R7](ownership.md#values-that-must-be-consumed)).

<!-- file: examples/reference-errors/close.sisu -->
```sisu
# Cleanup that can fail is a method that consumes the value.
error SaveError:
    Full(pending: int)

@must_consume
struct Batch:
    lines: list[String]

    def add(mut self, line: str):
        self.lines.push(line.to_string())

    def close(own self, room: int) -> int ! SaveError:
        if len(self.lines) > room:
            return Err(Full(len(self.lines)))
        return Ok(len(self.lines))

def main():
    var batch = Batch([])
    batch.add("löyly")
    batch.add("vihta")
    print(batch.close(1))
```

<!-- output: examples/reference-errors/close.out -->
```text
Err(Full(pending=2))
```

**Status.** In stage 0 (D-054).

## When allocation fails

Allocation failure panics. A collection's `try_push` and its kind give
`! OutOfMemory` for code that must handle it, such as code on a fixed
budget ([R7](ownership.md#allocators)).

**Status.** Designed (Q-4); stage 0 has no `try_push`.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| E0226, E0227, E0356 | `resolve.inferred-set-position`, `resolve.union-outside-result`, `type.open-set-needs-wildcard` | [Error sets](#error-sets) |
| E0321, [E0320](diagnostics.md#e0320), E0319 | `type.error-not-in-set`, `type.try-context`, `type.try-needs-fallible` | [Propagation](#propagation) |
| [E0318](diagnostics.md#e0318) | `type.unused-result` | [Results that must be used](#results-that-must-be-used) |
| [R0018](diagnostics.md#r0018), R0041 | `run.error-returned`, `run.trace-header` | [When main fails](#when-main-fails) |
| [R0001](diagnostics.md#r0001), R0009, R0029 | `panic.explicit`, `panic.unreachable`, `panic.unreachable-plain` | [Panics and unwinding](#panics-and-unwinding) |
| R0014 | `panic.during-unwind` | [Aborts](#aborts) |
