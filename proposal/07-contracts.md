# 07 - Contracts and verification

## Decision

Preconditions, postconditions, type invariants and refinement types are
part of the language, not a library. They are:

- **Documented automatically**: `sisu doc` renders them with the signature.
- **Checked at runtime** in debug and test builds, and in release builds
  unless disabled per package (`[profile.release] contracts = "off" |
  "preconditions" | "all"`). The default for release is `preconditions`,
  because a violated precondition is a caller bug that is cheap to detect
  and expensive to miss. One check ignores the level: the `invariant` of a
  type protected by a `Mutex` is re-checked when unwinding drops the guard
  (D-077, [06](06-concurrency-and-capabilities.md)), in every build.
- **Used by the optimiser**: a proven contract removes bounds checks and
  overflow checks.
- **Available to a static checker**: the grammar of contract expressions is
  the ordinary expression grammar restricted to `pure` functions, so an
  SMT-backed verifier can consume them without a second specification
  language. The verifier itself is not part of version 1.

```sisu
def binary_search[T: Ord](items: Slice[T], target: T) -> int?:
    requires items.is_sorted()
    ensures result.map(items[it] == target) ?? not items.contains(target)
    ...

struct Account:
    balance: Money
    limit: Money
    invariant self.balance >= -self.limit

type Port = int where 1 <= self <= 65535
type Email = String where self.contains('@')
```

Contract clauses are the first lines of a body, before any statement, in
the position Python reserves for a docstring; `invariant` lines sit among a
struct's fields. A labelled result (`-> (q: int, r: int)`) is named by its
labels in `ensures`; an unlabelled one is `result`.

- `requires` is checked on entry; `ensures` on every exit, with `result`
  bound to the return value and `old(expr)` available for the entry value.
- `invariant` on a struct is checked after construction and after every
  `mut self` method returns. Fields of a type with an invariant cannot be
  assigned from outside the type's `impl` (the invariant would be
  unenforceable); the compiler enforces this.
- A refinement `type X = T where pred` is a subtype of `T`; constructing one
  checks `pred` (statically when the value is a constant); it can be used
  wherever `T` can; `T` can be converted to it with `X.try_from(t) -> X ! RefinementError`.

## Evidence

- **Ada/SPARK** ([`ada-spark.md`](../research/languages/ada-spark.md)):
  pre/postconditions and predicates were added as language aspects in Ada
  2012 after decades as pragmas; SPARK proves their absence of runtime
  errors in production avionics, rail and (since 2019) NVIDIA firmware. The
  lesson is that contracts in the language, in the same expression syntax,
  are what makes gradual verification possible. The lesson on why it did not
  spread is about ecosystem and verbosity, not about contracts.
- **Eiffel** introduced design by contract; **D** ships `in`/`out`/
  `invariant` blocks and its designers count them among the features they
  would keep ([`d.md`](../research/languages/d.md)).
- **Dafny, Liquid Haskell, F\***: show that refinement types over a decidable
  fragment handle most array-bounds and arithmetic properties with little
  annotation ([`idris-agda-lean.md`](../research/languages/idris-agda-lean.md)).
  Full dependent types are rejected for Sisu (Q-16, decided) because of
  proof burden and compile-time cost; refinement types are the proven
  subset.
- **Rust's `debug_assert!` and Zig's `std.debug.assert`** are contracts
  without documentation or static use; the research on API misuse (Stylos &
  Clarke, Parnin et al.) shows preconditions that live only in prose are not
  read.
- **C++ Contracts** were voted into C++26 after being removed from C++20;
  the decade of committee papers (P0542, P2900) is a catalogue of the
  design questions Sisu answers above: when to check, what happens on
  violation (panic, like any bug), and whether the optimiser may assume them
  (only when proven, never when merely declared, because assumed-but-false
  contracts turn bugs into undefined behaviour).

## Interaction with the rest of the language

- A contract violation is a panic ([05-errors.md](05-errors.md)): the
  affected task unwinds to its scope.
- Contract expressions must be `pure` (no capability parameters, no
  `unsafe`, no mutation of anything outside the expression). The compiler
  rejects anything else, which is what keeps them safe to disable.
- Contracts participate in API semver: strengthening a precondition or
  weakening a postcondition is a breaking change, and `sisu publish`'s API
  diff treats it as one ([09-modules-packages-and-build.md](09-modules-packages-and-build.md)).
- Trait methods may declare contracts; an implementation may weaken the
  precondition and strengthen the postcondition (Liskov), which the compiler
  checks syntactically (the implementation's `requires` must be implied by
  the trait's, checked by the static verifier when available and by
  runtime double-checking otherwise).

## What is deferred

- The static verifier (SMT-backed discharge of contracts) is a tooling
  project after version 1. The language is designed so that it needs no
  changes to add it.
- Loop invariants and termination measures (`decreases`) are contextual
  keywords without semantics in version 1.
- Ghost code (specification-only variables) waits for the verifier (Q-16, decided).
