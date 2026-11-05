# Contracts

This page states the rules of contracts: the preconditions and
postconditions of functions, the invariants of types, refinement types,
how much a build checks, what a violation does, and what the compiler
does with contracts besides checking them. A contract is a `bool`
expression that the language checks, part of a declaration and of its
documentation (D-028).

**See also:** the tutorial's [chapter 23](../tutorial/23-contracts.md);
the proposal's [07, Contracts and
verification](../../proposal/07-contracts.md); the compiler's
[architecture notes](../../compiler/docs/architecture.md), "Contracts
and refinements"; [R2](types.md#refinement-types) for refinements as
types; [R16](sisu-command.md#what-a-run-checks---contracts) for
`--contracts`; [R17, Diagnostics](diagnostics.md#codes-and-their-ranges)
for the codes, E08xx.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Contracts in the language are decided (D-028); the verifier
that would prove them waits until after version 1 (Q-16).

## Where clauses stand

| Clause | Stands | Holds |
|---|---|---|
| `requires EXPR` | among the first lines of a function's body | on entry |
| `ensures EXPR` | among the first lines of a function's body | at every exit |
| `invariant EXPR` | among a struct's fields | for every value of the struct |

- A function's `requires` and `ensures` come before its first statement,
  in any number and order, where Python puts a docstring; one after a
  statement is E0800 (`contract.misplaced`).
- A clause is a `bool` (E0302, [R17](diagnostics.md#e0302), for
  anything else), and so is a refinement's predicate
  ([Refinement types](#refinement-types)).
- A test has no contract (E0807, `contract.not-in-test`): it says what it
  needs with `assert`. A generator has no `ensures` (E0805,
  `contract.generator-ensures`), since what it returns is its iterator.
- `requires`, `ensures` and `invariant` are contextual keywords: a
  clause starts where an expression follows the word
  ([R20](grammar.md#other-statements)).

`sisu check` builds a program with its tests, so it reports E0807,
which `sisu run` leaves out with the tests:

<!-- file: examples/reference-contracts/mistakes.sisu -->
```sisu
# Clauses that stand where no contract belongs, or say too much.
def half(n: int) -> int:
    let h = n // 2
    requires n >= 0
    return h

def twice(n: int) -> int:
    ensures result == old(n, n)
    return n * 2

def evens(limit: int) -> Iter[int]:
    ensures limit > 0
    for i in range(limit):
        yield i * 2

test "halves":
    requires True
    assert half(4) == 2
```

<!-- output: examples/reference-contracts/mistakes.console -->
```console
$ sisu check mistakes.sisu
error[E0800]: `requires` is a function's contract and belongs at the start of its body, before its first statement
  --> mistakes.sisu:4:5
  |
4 |     requires n >= 0
  |     ^^^^^^^^^^^^^^^

error[E0804]: `old` takes one expression, as in `old(self.balance)`
  --> mistakes.sisu:8:23
  |
8 |     ensures result == old(n, n)
  |                       ^^^^^^^^^

error[E0805]: a generator has no `ensures`: what it returns is its iterator
  --> mistakes.sisu:12:5
   |
12 |     ensures limit > 0
   |     ^^^^^^^^^^^^^^^^^

error[E0807]: a test has no contract; check what it needs with `assert`
  --> mistakes.sisu:17:5
   |
17 |     requires True
   |     ^^^^^^^^^^^^^

```

**Status.** In stage 0 (D-028).

## Preconditions

`requires` states what a caller must give. Its clauses are checked
each time the function is entered, after its arguments are evaluated
and before its first statement, in the order written. A precondition
that fails blames the caller.

## Postconditions

`ensures` states what the function gives back. Its clauses are checked
at every exit, a `return`, a `?` that passes an error on, or the end of
the body, after the result is made:

- `result` is the value returned. In a function that returns a result,
  `T ! E`, it is the whole result, `Ok` or `Err`, so `ensures result >
  0` is E0369, whose note says to compare with `Ok(...)` or ask
  `.is_ok()`; in one with a labelled
  tuple, `-> (each: int, rest: int)`, the labels name its parts instead.
- `old(EXPR)` is the value of `EXPR` on entry, which the compiler copies
  then; only an `ensures` asks for it (E0803, `contract.old-outside`),
  and it takes one expression (E0804, `contract.old-arguments`).
- A `mut` parameter, or `self` of a `mut self` method, is read as the
  call leaves it.

A postcondition that fails blames the function.

<!-- file: examples/reference-contracts/clauses.sisu -->
```sisu
# `requires` on entry, `ensures` at every exit, `old` for entry values.
struct Account:
    cents: int

    ## Takes `amount` out, and gives what is left.
    def withdraw(mut self, amount: int) -> int:
        requires 0 < amount <= self.cents
        ensures self.cents == old(self.cents) - amount
        ensures result == self.cents
        self.cents -= amount
        return self.cents

## `total` split into `parts` shares, and what is left over.
def share(total: int, parts: int) -> (each: int, rest: int):
    requires parts > 0
    ensures each * parts + rest == total
    return (total // parts, total % parts)

def main():
    var account = Account(cents=5000)
    print(account.withdraw(1250))
    let each, rest = share(1000, 3)
    print("{each} each, {rest} over")
```

<!-- output: examples/reference-contracts/clauses.out -->
```text
3750
333 each, 1 over
```

An exit by `?` is checked like any other:

<!-- file: examples/reference-contracts/fallible.sisu -->
```sisu
# In a function that can fail, `result` is the whole result, and an
# exit by `?` is checked too.
error Reading:
    Missing(name: String)

def find(name: str) -> int ! Reading:
    if name == "kiuas":
        return Ok(80)
    return Err(Missing(name.to_string()))

def doubled(name: str) -> int ! Reading:
    ensures result.is_ok()
    let celsius = find(name)?
    return Ok(celsius * 2)

def main():
    print(doubled("kiuas"))
    print(doubled("lauteet"))
```

<!-- output: examples/reference-contracts/fallible.console -->
```console
$ sisu run fallible.sisu
Ok(160)
panic: postcondition of `doubled` failed: result.is_ok()
    result.is_ok()
    │
    Err(Missing(name="lauteet"))
  at doubled (fallible.sisu:12:13)
  at main (fallible.sisu:18:11)
```

**Status.** In stage 0 (D-028).

## What a clause may do

A clause only reads. It may call only `pure` functions (E0801,
`contract.not-pure`), change nothing (E0802, `contract.changes`), as
`requires xs.pop() != None` would, and hold no `unsafe` (E0988,
`unsafe.in-contract`). So a build may leave every clause out without
changing what the program does, and a clause that fails can be
evaluated again, part by part, for its report. A `pure` function may
change what it owns, so a helper that builds a set to compare against
is a function that a clause can call
([chapter 23](../tutorial/23-contracts.md#contracts-only-read)).

**Status.** In stage 0 (D-028, D-074).

## Invariants

An `invariant` line in a struct's body states what holds of every value
of the struct. It is checked:

- when a value is built, by any construction;
- when a `mut self` method of the struct returns, by any exit;
- when a panic drops the guard of a `Mutex` that holds the value, in
  every build whatever its level, and a value that fails it poisons the
  mutex (D-077, [R9](concurrency.md#sharing-a-value-that-changes)).

A struct may have several `invariant` lines, and each is checked. Inside
a method the invariant may be broken for a while, as long as it holds
again when the method returns. A method that takes `self` only reads, so
it is not checked.

<!-- file: examples/reference-contracts/invariant.sisu -->
```sisu
# An invariant holds after construction and each `mut self` method.
struct Gauge:
    level: int
    invariant 0 <= self.level <= 10

    def fill(mut self, amount: int):
        self.level += amount

def main():
    var gauge = Gauge(level=4)
    gauge.fill(5)
    print(gauge.level)
    gauge.fill(5)
    print(gauge.level)
```

<!-- output: examples/reference-contracts/invariant.console -->
```console
$ sisu run invariant.sisu
9
panic: invariant of `Gauge` failed: 0 <= self.level <= 10
    0 <= self.level <= 10
         │
         14
  at the invariant of Gauge (invariant.sisu:4:15)
  at Gauge.fill (invariant.sisu:7:29)
  at main (invariant.sisu:13:5)
```

Only the struct's own methods change its fields: an assignment to a
field from anywhere else, a function of the same module included, is
E0806 (`contract.invariant-field`), since nothing would check the
invariant after it. A construction is allowed everywhere, because it
checks the invariant.

A method that takes `own self`, changes a copy of its value and returns
it, is not one of the checked exits, so its result can break the
invariant unseen:

<!-- file: examples/reference-contracts/own_self.sisu -->
```sisu
# A method that takes `own self` and returns a changed value is not
# checked against the invariant.
struct Gauge:
    level: int
    invariant 0 <= self.level <= 10

    def filled(own self, amount: int) -> Gauge:
        var changed = self
        changed.level += amount
        return changed

def main():
    let gauge = Gauge(level=4).filled(20)
    print(gauge.level)
```

<!-- output: examples/reference-contracts/own_self.out -->
```text
24
```

**Status.** In stage 0 (D-028, D-077).

## Refinement types

`type X = T where PREDICATE` declares a refinement type: the base `T`
with a predicate on `self`, the base value. The predicate is a `bool`
(E0814, `refine.predicate-type`) that only reads, as a clause does.

- An `X` is a subtype of `T`: it is accepted wherever a `T` is, with
  `T`'s impls of the traits that read it, and an impl of another trait
  for `X` is E0813 (`refine.own-impl`).
- A `T` becomes an `X` only where the predicate is checked. A constant is
  checked while the program compiles (E0810, `refine.not-satisfied`);
  any other value by `X.try_from(value)`, which gives an `X` or the error
  `Unsatisfied(ty, predicate)` of `RefinementError`. A `T` where an `X`
  is expected is E0811 (`refine.needs-try-from`), whose note names
  `try_from`.
- A change through the base that could break the predicate, such as
  `push` on a refined list, is E0812 (`refine.change-through-base`):
  change a copy of the base and check it again with `try_from`.
- What is computed from an `X`, such as `x + 1`, is a `T` again.
- A refinement of a refinement keeps the chain: its values satisfy every
  predicate down to the base, and `try_from` checks them in order, the
  innermost first, so its error names the first that fails.

<!-- file: examples/reference-contracts/refinements.sisu -->
```sisu
# A refinement is its base with a predicate, checked where a value
# comes in.
type Hour = int where 0 <= self < 24
type Evening = Hour where self >= 17

def label(hour: Hour) -> String:
    return "{hour}:00, {24 - hour} hours left"

def main():
    let opens: Evening = 18
    print(label(opens))
    for n in [20, 9, 30]:
        match Evening.try_from(n):
            case Ok(hour): print(label(hour))
            case Err(e): print("{n}: {e:?}")
```

<!-- output: examples/reference-contracts/refinements.out -->
```text
18:00, 6 hours left
20:00, 4 hours left
9: Unsatisfied(ty="Evening", predicate="self >= 17")
30: Unsatisfied(ty="Hour", predicate="0 <= self < 24")
```

<!-- file: examples/reference-contracts/refine_mistakes.sisu -->
```sisu
# Refinements whose predicate, impls or changes do not hold together.
type Short = list[int] where len(self) <= 3
type Half = int where self // 2
type Small = int where self < 10

impl Display for Small:
    def fmt(self, mut f: Formatter):
        f.write("small")

def main():
    var xs: Short = [1, 2]
    xs.push(3)
    print(xs)
```

<!-- output: examples/reference-contracts/refine_mistakes.console -->
```console
$ sisu run refine_mistakes.sisu
error[E0814]: the predicate of `Half` must be a `bool`, found `int`
  --> refine_mistakes.sisu:3:23
  |
3 | type Half = int where self // 2
  |                       ^^^^^^^^^

error[E0813]: `Small` refines `int` and has its reading impls; it cannot have an impl of `Display` of its own
  --> refine_mistakes.sisu:6:1
  |
6 | impl Display for Small:
  | ^^^^^^^^^^^^^^^^^^^^^^

error[E0812]: `xs` is of type `Short`, which a change through its base could take outside `len(self) <= 3`; change a copy of the base and check it again with `try_from`
  --> refine_mistakes.sisu:12:5
   |
12 |     xs.push(3)
   |     ^^
  = note: a copy of `xs` is a `Short` too: change a copy typed as the base, `var copy: list[int] = xs.copy()`, and check it again with `Short.try_from(copy)`

error[D0006]: the program was not run: errors found (3)
```

**Status.** In stage 0 (D-028).

## Check levels

A build checks the clauses its level names:

| Level | Checks |
|---|---|
| `all` | every `requires`, `ensures` and `invariant` |
| `preconditions` | the `requires` clauses only |
| `off` | none |

- `sisu run` checks `all` unless `--contracts=LEVEL` says otherwise
  (D0028, `cli.bad-contract-level`, for another word); `sisu sir` and
  `sisu build --native` take the option too.
- A release build checks `preconditions` unless the root package's
  `[profile.release] contracts` names another level
  ([R13](modules.md#profilerelease)): a violated precondition is a
  caller's bug, cheap to find and costly to miss. `[profile.debug]`
  takes the key too.
- `sisu test` checks every clause, always, and refuses `--contracts`
  (D0029, `cli.test-checks-contracts`).
- The invariant check of a `Mutex` whose guard a panic drops ignores the
  level ([Invariants](#invariants)).
- `debug_assert CONDITION[, MESSAGE]` follows the level: at `all` it is
  checked as `assert` is, with the values of its parts; at
  `preconditions` and `off` its condition is not evaluated
  ([R15](testing.md#assertions);
  [chapter 23](../tutorial/23-contracts.md#how-much-is-checked) shows
  one in a release build).

<!-- file: examples/reference-contracts/levels.sisu -->
```sisu
# The level of a run decides which clauses are checked.
def per_hour(minutes: int, hours: int) -> int:
    requires hours > 0
    ensures result * hours <= minutes
    return minutes // hours + 1

def main():
    print(per_hour(90, 2))
    print(per_hour(90, 0))
```

<!-- output: examples/reference-contracts/levels.console -->
```console
$ sisu run levels.sisu
panic: postcondition of `per_hour` failed: result * hours <= minutes
    result * hours <= minutes
    │      │ │        │
    46     │ 2        90
           92
  at per_hour (levels.sisu:4:13)
  at main (levels.sisu:8:11)
$ sisu run --contracts=preconditions levels.sisu
46
panic: precondition of `per_hour` failed: hours > 0
    hours > 0
    │
    0
  at per_hour (levels.sisu:3:14)
  at main (levels.sisu:9:11)
$ sisu run --contracts=off levels.sisu
46
panic: division by zero
  at per_hour (levels.sisu:5:12)
  at main (levels.sisu:9:11)
$ sisu run --contracts=some levels.sisu
error[D0028]: `--contracts` takes `off`, `preconditions` or `all`, not `some`
```

At `all`, the first call breaks the postcondition; at `preconditions`
it passes and the second call breaks the precondition; at `off` the
division by zero that the precondition guarded is reached.

**Status.** In stage 0 (D-028).

## Violations

A violated contract is a bug, so it panics: the task unwinds to its
scope, as for any panic ([R8](errors.md#panics-and-unwinding)), and a
panic in `main` exits with status 101. The message names the function
or the type and the clause, then draws the value of each part of the
clause that only reads, as a failed `assert` does, and the trace starts
at the clause:

| Code | Identifier | When |
|---|---|---|
| [R0100](diagnostics.md#r0100) | `contract.requires-failed` | a precondition fails |
| [R0101](diagnostics.md#r0101) | `contract.ensures-failed` | a postcondition fails |
| [R0102](diagnostics.md#r0102) | `contract.invariant-failed` | an invariant fails |

<!-- file: examples/reference-contracts/violation.sisu -->
```sisu
# A broken precondition panics, naming the function and the clause.
struct Account:
    cents: int

    def withdraw(mut self, amount: int) -> int:
        requires 0 < amount <= self.cents
        self.cents -= amount
        return self.cents

def main():
    var account = Account(cents=5000)
    print(account.withdraw(1250))
    print(account.withdraw(9000))
```

<!-- output: examples/reference-contracts/violation.console -->
```console
$ sisu run violation.sisu
3750
panic: precondition of `Account.withdraw` failed: 0 < amount <= self.cents
    0 < amount <= self.cents
        │         │
        9000      3750
  at Account.withdraw (violation.sisu:6:18)
  at main (violation.sisu:13:11)
```

**Status.** In stage 0 (D-028, D-040).

## Contracts as tests

For each function with a contract whose parameters all have generators
([R15](testing.md#property-tests)), `sisu test` adds a test named
`contract of NAME`. It calls the function on `arbitrary` cases that its
`requires` accepts, skipping the others, and lets its `ensures` and the
invariants judge each call:

<!-- file: examples/reference-contracts/as_test.sisu -->
```sisu
# `sisu test` checks each function's contract on cases it makes.
def clamp(n: int, low: int, high: int) -> int:
    requires low <= high
    ensures low <= result <= high
    if n < low:
        return low
    if n > high:
        return high
    return n

test "clamps from above":
    assert clamp(30, 0, 24) == 24
```

<!-- output: examples/reference-contracts/as_test.test -->
```text
test "clamps from above" ... ok
test "contract of clamp" ... ok
2 passed, 0 failed, 0 blocked
```

**Status.** In stage 0 ([07](../../proposal/07-contracts.md#decision)).

## Contracts in documentation and versions

`sisu doc` shows each function's clauses under its signature, a
struct's invariants under its fields, a refinement's predicate with the
`try_from` that checks it, and, under each `mut self` method of a type
with an invariant, that the method checks it again
([R16](sisu-command.md#sisu-doc)). A contract is part of a function's
promise, so strengthening a precondition or weakening a postcondition
breaks its callers, and the API diff of `sisu publish` counts the change
as a breaking one ([R13](modules.md#the-registry)).

**Stage 0 differs.** `sisu publish` and its API diff are designed
(D0003, [R16](sisu-command.md#sisu-publish)).

**Status.** `sisu doc` in stage 0; the API diff designed (D-031).

## Contracts on trait methods

A trait's method may declare a contract, which binds every impl of the
method. An impl may weaken the precondition and strengthen the
postcondition, as Liskov's rule allows, and no more: its `requires` must
follow from the trait's, which a static verifier would prove and which,
until there is one, the run checks against both clauses
([07](../../proposal/07-contracts.md#interaction-with-the-rest-of-the-language)).

The trait's clauses are checked in each impl's method, the trait's
`requires` on entry before the impl's own and its `ensures` on exit
after them, with the trait's names for the parameters, and a failure
names the trait's method, whether the call is direct, through a
generic bound or through `dyn`. A method whose body is its contract
alone is required, as one without a body is; a provided method's
contract binds the impls that write their own, as `Dial`'s does:

<!-- file: examples/reference-contracts/trait_contract.sisu -->
```sisu
# A contract on a trait's method binds every impl of it.
trait Meter:
    def read(self, at: int) -> int:
        requires at >= 0
        return at

struct Dial:
    scale: int

impl Meter for Dial:
    def read(self, at: int) -> int:
        return at * self.scale

def main():
    print(Dial(2).read(-5))
```

<!-- output: examples/reference-contracts/trait_contract.console -->
```console
$ sisu run trait_contract.sisu
panic: precondition of `Meter.read` failed: at >= 0
    at >= 0
    │
    -5
  at Meter.read for Dial (trait_contract.sisu:4:18)
  at main (trait_contract.sisu:15:11)
```

The trace names a method of a trait's impl by the trait and the type,
`Meter.read for Dial`, as a trace does for any trait's impl
([R8](errors.md#panics-and-unwinding)).

An impl in another module than the trait's, or in another package, is
held to the contract all the same. The trait's clauses are checked with
the names of the trait's module, so a clause may call a private helper
of that module, and the check stands at the name of the impl's method,
which the failure's place names:

<!-- file: examples/reference-contracts/trait_elsewhere.files/src/meter.sisu -->
```sisu
# A trait whose method has a contract.

## Something that reads a heat at a moment of the evening.
pub trait Meter:
    ## The heat at minute `at`, which is not negative.
    def read(self, at: int) -> int:
        requires at >= 0
```

<!-- file: examples/reference-contracts/trait_elsewhere.files/src/main.sisu -->
```sisu
# An impl of a trait in another module, held to the trait's contract.
from kiuas.meter import Meter

struct Dial:
    scale: int

impl Meter for Dial:
    def read(self, at: int) -> int:
        return at * self.scale

def main():
    print(Dial(2).read(3))
    print(Dial(2).read(-5))
```

<!-- output: examples/reference-contracts/trait_elsewhere.console -->
```console
$ sisu run
6
panic: precondition of `Meter.read` failed: at >= 0
    at >= 0
    │
    -5
  at Meter.read for Dial (src/main.sisu:8:9)
  at main (src/main.sisu:13:11)
```

The verifier that would prove an impl's clauses against the trait's is
after version 1, and W0021 ([R17](diagnostics.md#w0021)) is kept for an
impl whose trait's module the check cannot read, which no module of a
program is.

**Status.** In stage 0 (07).

## What waits for the verifier

The grammar of a clause is the grammar of expressions, restricted to
`pure` code, so that a static verifier can prove contracts before the
program runs, with no second language for specifications. The verifier,
backed by an SMT solver, is a tool for after version 1; the language
needs no change for it.

- A contract that the verifier proves lets the optimiser leave out the
  checks of bounds and overflow it covers; a contract only declared is
  never assumed, since an assumption that is false would make a bug
  undefined behaviour.
- `decreases`, a measure that shows that a recursion or a loop ends, and
  an `invariant` inside a loop's body are reserved; stage 0 reports
  them as E0900 ([chapter 23](../tutorial/23-contracts.md#what-waits-for-the-verifier)).
- Ghost code, variables that exist only for a proof, waits for the
  verifier (Q-16).

**Status.** Designed (Q-16); `decreases` and loop invariants are E0900.

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0800](diagnostics.md#e0800), [E0302](diagnostics.md#e0302), [E0805](diagnostics.md#e0805), [E0807](diagnostics.md#e0807) | `contract.misplaced`, `type.not-bool`, `contract.generator-ensures`, `contract.not-in-test` | [Where clauses stand](#where-clauses-stand) |
| [E0803](diagnostics.md#e0803), [E0804](diagnostics.md#e0804) | `contract.old-outside`, `contract.old-arguments` | [Postconditions](#postconditions) |
| [E0801](diagnostics.md#e0801), E0802, E0988 | `contract.not-pure`, `contract.changes`, `unsafe.in-contract` | [What a clause may do](#what-a-clause-may-do) |
| [E0806](diagnostics.md#e0806) | `contract.invariant-field` | [Invariants](#invariants) |
| [E0810](diagnostics.md#e0810), [E0811](diagnostics.md#e0811), [E0812](diagnostics.md#e0812), [E0813](diagnostics.md#e0813), [E0814](diagnostics.md#e0814) | `refine.not-satisfied`, `refine.needs-try-from`, `refine.change-through-base`, `refine.own-impl`, `refine.predicate-type` | [Refinement types](#refinement-types) |
| [D0028](diagnostics.md#d0028), [D0029](diagnostics.md#d0029) | `cli.bad-contract-level`, `cli.test-checks-contracts` | [Check levels](#check-levels) |
| [R0100](diagnostics.md#r0100), [R0101](diagnostics.md#r0101), [R0102](diagnostics.md#r0102) | `contract.requires-failed`, `contract.ensures-failed`, `contract.invariant-failed` | [Violations](#violations) |
| [E0900](diagnostics.md#e0900) | `stage0.unsupported` | [What waits for the verifier](#what-waits-for-the-verifier) |
