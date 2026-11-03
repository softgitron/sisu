# Costs you can see

Sisu does not promise to be the fastest language. It promises that the
cost of code is visible in the code: allocation shows in types, a copy
is written `.copy()`, sharing is a type, a call through a trait object
needs `dyn` behind a visible `Box` or `ref`, generic code is compiled
for each type it is used with, and the compiler itself has targets for
its own speed. This page explains why predictable performance outranks
expressiveness in Sisu's design, how each rule makes a cost visible,
what Sisu pays for the rules, and what would change them.

**See also:** the tutorial's [chapter 10](../tutorial/10-ownership.md)
(moves and copies), [chapter 13](../tutorial/13-sharing.md) (`Box` and
`Shared`) and [chapter
15](../tutorial/15-generics.md#what-generics-and-dyn-cost) (what
generics and `dyn` cost); the reference's [R2,
Types](../reference/types.md#monomorphisation) and [R7, Ownership and
borrowing](../reference/ownership.md#sharing-cells-and-graphs); the
proposal's
[00](../../proposal/00-vision-and-principles.md#priorities-in-order),
[03](../../proposal/03-types-and-generics.md#dispatch) and
[04](../../proposal/04-memory-and-ownership.md#resource-attributes), and
decisions D-001, D-007, D-017, D-023 and D-059 in its [decision
log](../../proposal/15-decision-log.md); the research in
[polymorphism-and-generics.md](../../research/topics/polymorphism-and-generics.md)
and
[tooling-and-compile-speed.md](../../research/topics/tooling-and-compile-speed.md).

## The problem: costs that hide

Most performance problems in ordinary programs are not slow algorithms.
They are costs that the code does not show: an allocation inside a call
that looked free, a copy made because a value was held in two places, a
reference count updated atomically on every assignment, an attribute
read that runs a method. The research names two such costs as the most
common complaints about otherwise well-liked languages (D-001). In
Swift, every copy of a class reference is a retain and a release, often
atomic, and copy-on-write hides the moment that an innocent-looking
change copies a whole array
([swift.md](../../research/languages/swift.md)). In Haskell, laziness
made it, in its designers' words, "very hard for even experienced
programmers to predict the space behaviour of lazy programs", and the
fixes, from `seq` to `foldl'` and strict data, are the admission
([haskell.md](../../research/languages/haskell.md)).

The opposite lesson comes from Go. It is not the fastest language, and
its success suggests that for most software "fast enough and
predictable" beats "fastest". So Sisu puts predictable performance third
in its priorities, after safety and readability and ahead of
expressiveness (D-001). The promise of
[00](../../proposal/00-vision-and-principles.md#priorities-in-order) is
exact about what that means: no hidden allocation, no hidden control
flow, and no hidden reference counting in code that does not ask for it.

## Each cost where it is paid

**Allocation shows in types.** `list`, `dict`, `String`, `Box` and
`Shared` allocate; a struct, an array, an enum and a tuple do not
([04](../../proposal/04-memory-and-ownership.md#the-decision), rule 7).
Nothing allocates behind a closure, a generic or a trait object: a
closure that borrows its captures is a value on the stack, and a trait
object needs a `Box` or a `ref` that the code writes. The standard
library follows the same rule, with no allocation unless an owned type
is returned, geometric growth for a `list`, and `with_capacity` and
`shrink_to_fit` as explicit calls
([11](../../proposal/11-stdlib.md#principles-for-std-api-design)).

**Copies are written.** Assignment moves a value, and only a type that
asks for `Copy` with `@derive(Copy)` is copied silently, which suits
small plain values such as numbers and points. Anything else is copied
by `.copy()`, at a cost that grows with its size (D-007). Swift's
implicit copying with copy-on-write was rejected because it hides
allocation, the cause of its "unexpected copy" performance problems.

**Sharing is a type.** A value with several owners is a `Shared[T]`, and
each `.copy()` of it updates a count. The count is atomic only when the
value can reach another task, which the compiler decides by escape
analysis (D-005); the developer never chooses, and can ask which was
chosen.

**Dispatch is visible.** A generic function is compiled once for each
type it is used with, so a call to it is a direct call that the
optimiser can see through (D-017). Dynamic dispatch exists, and it is
spelled: `dyn Shape` is a type of its own, used behind a `Box` or a
`ref`.

**Control flow is visible.** There are no exceptions, so every call that
can fail is marked with `?` or handled on the spot, and there are no
properties, so `x.y` reads a field and never runs code (D-023). Integer
overflow and out-of-bounds indexing are checked in every build mode,
which costs a few percent in Swift's and Rust's measurements; Sisu
accepts the cost because safety ranks first
([14](../../proposal/14-non-goals.md#not-fastest-at-all-costs)).

This program pays each of those costs once, and each is written where it
is paid:

<!-- file: examples/explanation-costs/visible.sisu -->
```sisu
# Each cost is written where it is paid: a copy, a count, a box on the
# heap, a call through a trait object.
trait Shape:
    def area(self) -> f64

struct Square:
    side: f64

impl Shape for Square:
    def area(self) -> f64:
        return self.side * self.side

def total(shapes: list[Box[dyn Shape]]) -> f64:
    var sum_so_far = 0.0
    for s in shapes:
        sum_so_far += s.area()
    return sum_so_far

def main():
    let sides = [2.0, 3.0]
    let kept = sides.copy()
    let square = Shared(Square(4.0))
    let other = square.copy()
    let boxes: list[Box[dyn Shape]] = [Box(Square(2.0)), Box(Square(3.0))]
    print("{len(kept)} {other.area()} {total(boxes)}")
```

Because the costs are written, tools can count them. This session runs
the program, then asks `sisu audit --ownership` for its copies and
shared values, and then asks the program for its `Shared` values while
it runs:

<!-- output: examples/explanation-costs/visible.console -->
```console
$ sisu run visible.sisu
2 16.0 13.0
$ sisu audit --ownership visible.sisu
visible.sisu lines=19 Shared=1 Weak=0 Cell=0 Arena=0 Key=0 copy=2 unsafe=0
$ sisu run --stats shared visible.sisu
2 16.0 13.0
stats shared: 1 of 1 `Shared` construction sites tagged task-local, 0 may-cross
stats shared: visible.sisu:22:18 in `main`, task-local: 1 cells made, 0 reached by another task
```

The audit finds the two `.copy()` calls and the one `Shared`, which is
what the experiment counts when it measures how much copying and sharing
real programs need. The statistics show the compiler's choice for the
`Shared` on line 22: no other task can reach it, so its count is updated
without atomic instructions.

The design goes one step further for code with hard limits (D-059).
`@no_alloc` on a function rejects any heap allocation in it or in what
it calls, `@no_block` rejects any call that could wait, and
`@stack(max=N)` bounds the depth of its frames, each checked over the
call graph once generics are compiled for their types. They are cheap to
check precisely because allocation is visible in types and every point
that can wait is a capability call or a lock, and they are meant for an
audio callback that must finish in 2.7 milliseconds or an interrupt
handler that must never allocate. Clang 20 checks the first two on
function types, and Ada's `pragma Restrictions` is the older precedent.
Stage 0 does not have them yet.

## Generics compiled per type, and their price

Generic code has no free answer, and the research says so (finding 6).
Rust compiles a generic function separately for each type it is used
with, which gives fast code and costs compile time and binary size;
Rust's first designer counts it among the choices he would not have
made. Swift's unspecialised generics pass a table of functions at run
time, which keeps one copy of the code and makes its speed depend on
what the optimiser managed to specialise. Sisu takes Rust's side because
a table of functions behind every generic call would make performance
unpredictable, which priority 3 forbids
([03](../../proposal/03-types-and-generics.md#dispatch)). It limits the
cost in two ways that the design names: instantiations whose types have
the same layout and call trait methods the same way share one copy of
the code, as Go's GC-shape stenciling does, and each package has an
instantiation budget that `sisu build --explain-size` reports. Both wait
for the native backend to mature, and stage 0 compiles one copy per
type.

## Compile time is a cost too

A slow compiler is a cost that every edit pays. Compile time is the top
productivity complaint wherever it is slow: about a quarter of Rust's
users call it a big problem and 56% want it improved, Swift's type
checker times out on expressions it cannot solve, and Julia's time to
its first plot defined its image for a decade
([tooling-and-compile-speed.md](../../research/topics/tooling-and-compile-speed.md),
finding 1). The research finds the causes in language decisions rather
than compiler bugs (finding 2), and it warns that what is not measured
grows (finding 9). So the proposal sets numbers, such as `sisu check`
within 300 milliseconds after one function's body changes in a
100,000-line package, and a clean debug build of the same within 10
seconds, and makes language decisions to reach them: explicit
signatures, no package cycles, no macros, generics checked once, and a
backend for debug builds that is not LLVM (D-038). [How the compiler
works](compiler.md) describes how the compiler is built to meet them,
and how close it is.

## What it costs

- **More to write.** `.copy()`, `Box`, `dyn`, `Shared(...)` and `mut` at
  a call site are words that Python never asks for. Each says something
  true about the program, and each is still a word to type and read.
- **Compile time and size for generics.** Sharing instantiations and a
  budget reduce the cost of compiling per type; they do not remove it.
- **Checks that cost time at run time.** Overflow and bounds checks run
  in every build mode, strict floating point gives up fused
  multiply-adds that the code does not ask for
  ([Determinism](determinism.md)), and code that needs the last few
  percent writes `unsafe` in a package the root trusts, where
  `sisu audit` can find it.
- **Some costs stay out of sight.** Whether a count is atomic is the
  compiler's choice, visible only when asked for. The yield points that
  the compiler puts in loops so that tasks share a thread cost a check
  on each loop's back edge, which the Phase 1 harness measured at 3.5%
  on one machine and at nothing measurable on another. And a drop runs
  at the end of its scope, which is predictable but not free: dropping a
  large structure spends its time at the dedent that ends the block.

## What would change it

Each rule names the measurement that would reopen it. More than one
`.copy()` per 30 lines of application code in the corpus reopens
implicit copying, Q-3, which would be tested by a variant of the
language that copies as Swift does and a count of the "why is this slow"
reports it draws
([16](../../proposal/16-validation-plan.md#then-in-parallel)). The
resource attributes stay if, on three ports shaped like real-time code
(an audio callback, an interrupt-driven serial driver and a game's frame
loop), fewer than one rejection in ten is a false positive that the
author cannot fix by restructuring the code (D-059). Strict floating
point reopens if its throughput falls more than 25% behind
`@float(fast)` on the kernels that matter (Q-44). How fast generated
code must be, and what LLVM buys on Sisu's own programs, is Q-60, which
decides how much of the release backend is worth its weight. And the
compile-speed targets are tracked in the compiler's CI from the first
commit, so that a regression is seen on the change that makes it.
