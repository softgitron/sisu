# Determinism

A `pure` function in Sisu returns the same bits for the same arguments
on every run, in every process, with any number of threads and on every
target (D-074). Three rules make that true: `dict` and `set` iterate in
the order their keys were inserted, the operations that would observe
the scheduler are errors in `pure` code, and floating point is strict.
This page explains why Sisu promises that much, what compile-time code,
parallel loops, simulation and replay get from the promise, what it
costs, and what would change it.

**See also:** the tutorial's [chapter 26](../tutorial/26-purity.md)
(purity and data parallelism), [chapter
24](../tutorial/24-numbers.md#floating-point-that-gives-the-same-bits-everywhere)
(floating point) and [chapter 25](../tutorial/25-compile-time.md)
(compile-time code); the reference's [R2,
Types](../reference/types.md#pure-function-types); the proposal's
[06](../../proposal/06-concurrency-and-capabilities.md#effects),
[03](../../proposal/03-types-and-generics.md#numbers) and
[08](../../proposal/08-metaprogramming.md), and decisions D-049, D-069
and D-074 in its [decision log](../../proposal/15-decision-log.md); the
research in
[systems-and-determinism.md](../../research/ideation/systems-and-determinism.md),
candidates 1 and 4.

## The problem: three ways that chance leaks into code

A function that takes no capability cannot read the clock, the network
or a random number, and it is tempting to call such a function
deterministic. In most languages it is not, because chance leaks in
through three doors that the function never opened.

The first is the order of a hash table. Go randomises the iteration
order of its maps on purpose, Rust's `HashMap` takes a random key in
each process, and Sisu's own first design did the same: D-049 gave
`dict` a per-process hash key and iteration in the order of that key, so
that no program could come to depend on an order by accident. The
research then found what that does to a function with no capabilities at
all
([systems-and-determinism.md](../../research/ideation/systems-and-determinism.md),
finding F1). A function that lists a `dict`'s keys returns different
lists in different processes. That breaks memoisation across processes,
breaks the replay of a recorded run and the simulation of a system, and
breaks the promise of [08](../../proposal/08-metaprogramming.md) that
code run at compile time gives "exactly what would run at run time".

The second is the schedule. When two tasks race to fail, "the first
failure" depends on which thread ran first, so two runs of one program
can report different errors (F3). The third is floating point, which
Sisu's proposal had specified only as "IEEE 754" (F2). A compiler may
fuse `a * b + c` into one fused multiply-add that rounds once instead of
twice, may reorder a sum under fast-math flags, and may link a different
library for `sin` on each platform. Clang fuses within a statement by
default, which is enough to make x86 and ARM disagree.

Projects that need determinism live with all three and enforce it by
review: replicated state machines, deterministic databases, lockstep
multiplayer games since Age of Empires, reproducible builds and
scientific runs. Temporal's workflow engine polices deterministic code
with a linter whose own documentation says it "will not catch all cases
of non-determinism", and Azure's Durable Functions with a run-time
detector that "won't catch all violations" (candidate 1). The research
found no language that checks bit-for-bit determinism across thread
counts and targets.

## What `pure` promises

A `pure` function takes no capability, contains no `unsafe` and changes
nothing shared, and since D-074 it is deterministic in the strong sense:
the same bits on every run, process, thread count and target. The owner
decided the three rules that make it hold in October 2026 (Q-42 to
Q-44).

**`dict` and `set` iterate in insertion order.** The per-process hash
key stays, so the defence against hash-flooding attacks that D-049 was
chosen for is unchanged; only the order of iteration stops depending on
the key. Python made the same split in 3.7, keeping its keyed hash and
guaranteeing insertion order, and Ruby and JavaScript's `Map` guarantee
it too. The reason D-049 gave against a specified order, that code would
come to depend on it silently, becomes the point: an order that is
specified is safe to depend on. It also removes a silent false friend
for readers who know Python ([Loud divergence](loud-divergence.md)).

**Operations that observe the schedule are errors in `pure` code.**
`select`, `Mutex`, `Atomic`, channels with more than one sender,
`try_lock`, `try_recv` and `try_send`, `Weak.upgrade` across tasks and
values derived from addresses all depend on timing, so a `pure` function
cannot use them. `task.scope`, `spawn` and `join` stay legal, so a
`pure` function may still fork work and join it. A scope in `pure` code
runs every task spawned before a failing one to completion, cancels the
ones spawned after it, and reports the failure of the lowest index, so
its result does not depend on which task failed first in time. Rayon's
`join` already reports the first closure's panic by position rather than
by time.

**Floating point is strict.** Each operation rounds once, as IEEE 754
says, and nothing is fused, reordered or simplified behind the program's
back. A fused multiply-add happens only where it is written, as
`a.mul_add(b, c)`; `sum` adds in a pairwise tree whose shape depends on
the length of the input, never on the machine or the number of workers;
and `@float(fast)` on a function allows the optimisations there and
gives up the guarantee for that function alone. Java made its floating
point strict in Java 17, after twenty years in which x87 hardware made
it impractical, and Box2D's physics engine reached identical results
across platforms and thread counts by turning off fused multiply-add,
avoiding fast-math and fixing the order of its merges (candidate 4).

`pure` is part of a function's type, so higher-order `pure` code can
demand `pure` callbacks. The checker enforces the rest: this function is
`pure`, and Sisu runs it once while compiling and once while the program
runs:

<!-- file: examples/explanation-determinism/same_bits.sisu -->
```sisu
# A `pure` function gives the same bits wherever it runs: at compile
# time, at run time, in any process and on any machine.
pure def mean(readings: list[f64]) -> f64:
    var total = 0.0
    var count = 0.0
    for i in range(len(readings)):
        total += readings[i]
        count += 1.0
    return total / count

const AT_COMPILE_TIME: f64 = comptime mean([82.5, 79.0, 91.25, 0.1])

def main():
    let at_run_time = mean([82.5, 79.0, 91.25, 0.1])
    print("{AT_COMPILE_TIME} {at_run_time}")
    print(at_run_time == AT_COMPILE_TIME)
```

<!-- output: examples/explanation-determinism/same_bits.out -->
```text
63.2125 63.2125
True
```

The constant was computed by the compiler's evaluator and the second
value by the running program, and they agree to the last bit, which `==`
on two floats tests. The strict rules are what make that hold for the
evaluator, the interpreter and the native backend alike, on any target.

## What the promise buys

Compile-time code is the first beneficiary. The evaluator that runs
`comptime` is an interpreter over the same typed IR as the program, so
08's promise, "exactly what would run at run time", is true only if
nothing it runs depends on chance. With ordered dictionaries and strict
floats it does not, and a table computed while compiling equals the one
the program would compute on any target ([Compile-time code instead of
macros](compile-time.md) explains the rest of the evaluator).

Parallel loops are the second. `std.par` runs `pure` bodies on a pool of
workers, and because the bodies are `pure` and reductions have a fixed
shape, removing the parallelism gives the same result, and a loop that
fails reports the failure of the lowest index (D-069). A histogram
counted on one worker and on eight has the same bits by construction,
and chapter 26 builds one as an exercise; stage 0 still runs the chunks
on one task, so the claim is tested only when the runtime's pool of
workers exists.

Simulation and replay are the third. A replay token can reproduce a
failing test only if everything the test computes outside the fakes
gives the same bits again, and a simulated cluster can be explored
schedule by schedule only if the code between capability calls is
deterministic ([Testing without mocks](testing.md)). Beyond the
toolchain, the same property is what replicated state machines, lockstep
simulations, reproducible pipelines, sound memoisation across processes
and configuration rendered by `sisu eval` from `pure` modules (D-063)
each need, and each project otherwise has to enforce by review.

## What it costs

- **An ordered `dict` is bigger and a little slower.** Keeping insertion
  order costs an index array, an indirection on lookup and tombstones on
  deletion, as Python's compact dictionary and Rust's `indexmap` pay.
  Python's version was still 20% to 25% smaller than the dictionary it
  replaced, so the price is moderate, and the corpus measures it.
- **A failure in a `pure` scope surfaces later.** Earlier siblings run
  to completion before the lowest-index failure is reported, and one
  that never ends delays the report until a deadline stops it.
- **Strict floats leave speed on the table.** Without implicit fused
  multiply-add, dense numerical kernels lose peak throughput until they
  use `mul_add` or explicit SIMD; correctly rounded functions in
  `std.math`, which carry the guarantee to `sin` and `exp`, are slower
  than the fastest libraries on some inputs and wait for Q-23; and a
  fixed reduction shape constrains how parallel loops split their work.
  Relaxed SIMD and GPU code cannot meet the guarantee at all and must
  opt out.
- **`pure` code can do less.** No lock, no atomic, no `select`, and in
  stage 0 not yet the iterator methods, whose `count`, `map` and `sum`
  are not marked `pure` (a gap the plan's findings record).
- **The promise is new.** Each rule has shipped somewhere, but the
  combination is checked by no language, so the corpus is the first test
  of whether it holds in practice.

## What would change it

The three decisions name their own tests. Q-42 is settled by the
insertion-ordered `dict` staying within 10% of a plain SipHash table on
the corpus, and by counting the corpus programs that read a dictionary's
order. Q-43 is settled by running the corpus's `pure` functions under
`--explore 1000`, and later natively with 1 and 16 workers, and diffing
the outputs; more than one `pure` function rejected per 500 lines of
`pure` code reopens the list of forbidden operations. Q-44 is settled by
a numerical corpus, a physics step, an n-body integrator and a
dot-product kernel, built for x86-64 and AArch64 at both optimisation
levels: the outputs must agree bit for bit, and a gap in throughput of
more than 25% between strict and `@float(fast)` on the kernels that
matter reopens the default. Numerics as a whole, SIMD types and arrays
among them, is Q-23, which is still open, so the correctly rounded
`std.math` and the shape of vector code are decided there.
