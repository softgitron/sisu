# Ownership without lifetimes

Sisu frees memory without a garbage collector, stops a program from
using memory after it is freed, and does both without asking you to
write how long a reference lives. This page explains how those three
aims fit together: why every value has one owner, why a reference lives
only on the stack or inside a view, what that gives up beside Rust,
Swift and Python, and what evidence would change the design.

**See also:** the tutorial's [chapter 10](../tutorial/10-ownership.md)
(moves and drops), [chapter 11](../tutorial/11-borrowing.md)
(borrowing), [chapter 12](../tutorial/12-views.md) (views) and [chapter
13](../tutorial/13-sharing.md) (sharing); the reference's [R7, Ownership
and borrowing](../reference/ownership.md); the proposal's [04, Memory
and ownership](../../proposal/04-memory-and-ownership.md) and decisions
D-004 to D-008 in its [decision log](../../proposal/15-decision-log.md);
the research in
[memory-management.md](../../research/topics/memory-management.md).

## The problem: safety without a collector

Memory safety comes first in the priority order that settles every
conflict in Sisu's design (D-001), and the reason is the strongest
single result in the research. In large C and C++ code bases about 70%
of severe vulnerabilities are memory-safety bugs: Microsoft reported it
for its CVEs from 2006 to 2018, and Chromium for its serious bugs from
2015 to 2020. When Android wrote its new code in memory-safe languages,
the share fell from 76% in 2019 to 24% in 2024, faster than the old code
was replaced
([memory-safety.md](../../research/evidence/memory-safety.md)). The
largest group is temporal: half of Chromium's memory bugs, and 17 of the
39 memory-corruption exploits that Project Zero found in the wild in
2021, used memory after it was freed. Checking array bounds stops the
smaller half. The larger half needs a rule about how long memory stays
valid.

The research also says that the safe way has to be the default. D's
`@safe`, C++'s safety profiles and Zig's checked release mode all exist,
and in each case the safe mode is not the default and is not what people
ship
([memory-management.md](../../research/topics/memory-management.md),
finding 1). A language with an optional memory model ends up with two: D
split into a collected camp and a manual one, and Nim changed its
default memory manager three times (finding 2). Sisu has one model, safe
by default, with `unsafe` as the marked way out.

A tracing garbage collector, as in Java, Go and C#, would make safety
cheap to learn. It is ruled out as the default by the third priority,
predictable performance. A collector pays for safety with pauses or
barriers, with memory headroom (in Hertz and Berger's measurements a
collector needed five times the memory to match explicit management, and
ran nearly 70% slower with twice the memory), with destruction at a
moment nobody chooses, and with a runtime too large for the embedded and
WebAssembly targets that Sisu aims at. So Sisu faces the question Rust
faced: how to free memory at a known moment, and still prove that
nothing uses it afterwards.

## The answers other languages give

Python counts references to every object, and runs a cycle collector for
what counting misses. Every name refers to a shared object, so after
`b = a` both names reach one list, and a change made through `b` shows
through `a`. That is memory safe, but nothing controls who may change
what, every assignment updates a count that the program never shows, and
nothing stops two threads from changing one list at once.

Swift counts references too, but makes its structs, enums and standard
collections values: assigning one gives an independent value, copied
when one of the two is changed. Swift's own retrospectives agree that
value semantics is the part to keep
([swift.md](../../research/languages/swift.md)). What they question is
the rest. Every copy of a class reference is a retain and a release,
often atomic, which the optimiser cannot always remove; cycles leak
unless a `weak` reference breaks them; and copy-on-write hides the
moment an expensive copy happens. Swift has spent its second decade
adding ownership (`borrowing` and `consuming` parameters, types that
cannot be copied or cannot escape), and Dave Abrahams, who led its
standard library, left to build Hylo on the thesis that value semantics
should be the whole model.

Rust has single ownership, moves, and first-class references. A
reference is a value like any other: it can be stored in a struct, put
in a vector, returned, or kept in a closure. Rust gets memory safety and
freedom from data races at no cost at run time, and Android counts about
a thousand times fewer vulnerabilities per line in its Rust code than it
had in its C and C++. Rust pays in the learning curve. In the 2024 Rust
survey, 10.6% of users called "borrow checker rejecting valid code" a
big problem and 44% said it could be improved; among people who stopped
using Rust, about a quarter gave its difficulty as a reason
([surveys.md](../../research/evidence/surveys.md)), and Google's own
figure is two to four months before a programmer feels productive
([rust.md](../../research/languages/rust.md)). The research traces the
curve to one cause. Because a reference may be stored anywhere, the
compiler has to be told how long each one is valid, and those lifetime
names (`'a`) spread from a struct that holds a reference to every
function that touches it; `struct Parser<'a>` is the wall that learners
describe. Ownership and moves are the cheap part to learn
([memory-management.md](../../research/topics/memory-management.md),
finding 4), and by a second-hand account of his 2023 retrospective,
Rust's first designer, Graydon Hoare, opposed explicit lifetime syntax
and wanted lifetimes inferred.

Hylo goes to the other extreme: it has no first-class references at all.
A reference exists only while a call runs, as a parameter, so checking
borrows reduces to "check that the reference is a function argument,
that the variable isn't already dropped, and that mutable paths don't
overlap" ([hylo.md](../../research/languages/hylo.md)). The same
analysis names the cost: "Iterators are the main problem, since they
store references in structs".

Three more answers were weighed and set aside: Vale's generational
references, which check each access at run time (5% to 11% on its one
published benchmark) and whose project changed its model three times
before it stalled ([vale.md](../../research/languages/vale.md));
Austral's linear types, under which every value is used exactly once,
which its own author found hostile to loops and early returns
([austral.md](../../research/languages/austral.md)); and manual memory
with checks in debug builds, which guarantees nothing in a release.

## Sisu's answer: references that cannot be stored

Sisu keeps the half of Rust that is cheap to learn and changes the half
that is not (D-004):

- Every value has one owner: a binding, a field or a collection.
  Assigning it, returning it or passing it to an `own` parameter moves
  it. A `Copy` type copies instead, and anything else is copied only by
  `.copy()`, so a copy is always written down (D-007). Swift's implicit
  copy-on-write is the cause of its "unexpected copy" performance
  problems, which is why Sisu spells the copy out.
- A parameter borrows by default, and `mut` at the call site marks a
  change in place (D-006). Swift and Hylo show that a borrow is what
  most functions want, so the default answers Rust's recurring question,
  "should this take `&T` or `T`?", for most functions before it is
  asked.
- A reference is second-class. `ref T` and `ref mut T` can be bound to a
  local, passed as an argument, and returned from a function that
  borrowed what it points into. A reference cannot be stored in an
  ordinary struct or a `Shared` value, nor captured by a closure that
  escapes. Rust's aliasing rule, one mutable borrow or any number of
  shared ones, holds unchanged, and a borrow ends at its last use, as in
  Rust.

The third rule is the one that removes lifetimes. A reference that
cannot be stored lasts no longer than the frame it was made in, or the
parameter it was borrowed from. The compiler can always see which, so
there is nothing to write. The one case it cannot see alone is a
function with two borrowed parameters that returns a borrow of one of
them, and there you name the parameters, not a lifetime:
`def longer(a: str, b: str) -> str from a, b:`. When the clause may be
left out follows Rust's three elision rules, with `from` in the place of
lifetime names.

Stored references are still needed, as Hylo's iterators show: a token
that points into the source text, an iterator over a borrowed list, a
`str` that is part of a `String`. For these Sisu has one more tier, the
view. A `view struct` may hold references; it lasts only as long as the
shortest of the borrows it holds, and like a reference it cannot leave
the frame that owns what it borrows. A list of views is a view too, and
`str`, `Slice[T]` and the iterators over borrowed collections are the
library's views. This program has a struct that borrows from text and a
function that returns a list of them:

<!-- file: examples/explanation-ownership/words.sisu -->
```sisu
# A struct that borrows, and a function that returns a list of them:
# no lifetime is named anywhere.
view struct Word:
    text: str
    line: int

def words(source: str) -> list[Word]:
    var found: list[Word] = []
    for i, line in enumerate(source.lines()):
        for text in line.split_whitespace():
            found.push(Word(text, i + 1))
    return found

def main():
    let source = "sisu means grit\nand it lasts".to_string()
    for word in words(source):
        print("{word.line}: {word.text}")
```

<!-- output: examples/explanation-ownership/words.out -->
```text
1: sisu
1: means
1: grit
2: and
2: it
2: lasts
```

Each `Word` and the list that `words` returns borrow from `source`, the
`String` that `main` owns. Rust writes the same program with a lifetime
parameter on the struct and another on the function, `struct Word<'a>`
and `fn words<'a>(source: &'a str) -> Vec<Word<'a>>`, and every type
that holds a `Word` takes one too. Sisu needs no name because a view
borrows for one span only. A struct that would borrow from two buffers
with different ends is split into two structs, and the proposal reads
Rust's experience as saying that structs with several lifetimes are rare
and usually a design error.

The rest of the model follows from the same choices:

- Sharing is a type. `Shared[T]` counts references (D-005), atomically
  only when the value can reach another task, which the compiler
  decides; a change goes through a `Cell` or a `Mutex` inside it, and a
  cycle is broken with `Weak`, since there is no cycle collector. A
  graph lives in an `Arena[T]` and is addressed by `Key[T]` handles,
  which detect a stale key when it is used: Vale's generation check,
  confined to an arena. Rust programmers reach for the same shape in
  entity systems and index-based graphs.
- A value is dropped when its owner's scope ends, in the reverse of the
  order the values were made (D-008), not at its last use as in Mojo,
  because a lock or a file should end where a reader expects it to.
- A view, a task scope and a suspended task frame are never leaked
  (D-058). As withoutboats, a designer of Rust's async support, put it,
  "any object can be leaked in Rust, trivially and safely", so a Rust
  library for completion I/O has to take a buffer by value and hand it
  back. In Sisu the operating system can write into a buffer that the
  caller's frame owns, because the frame cannot vanish without running
  its drops
  ([systems-and-determinism.md](../../research/ideation/systems-and-determinism.md),
  candidate 7).

## What the rule gives you

No signature in Sisu holds a lifetime, and none of the problems that
come from stored references arise. An ordinary struct cannot hold a
reference, so it cannot hold one to itself, and Rust's `Pin`, which
exists for self-referential values, has no counterpart. A `task.scope`
is a frame that cannot return before its tasks end, so a task may borrow
from the function that started it, which Rust allows only inside
`thread::scope` and otherwise answers with `'static` bounds ([chapter
20](../tutorial/20-concurrency.md) shows it, and [Concurrency without
colours](concurrency.md#scopes-as-supervisors) explains the scope).

Most of all, two bindings never reach the same value unless its type is
`Shared`. A list passed to a function cannot change behind the caller's
back unless the call says `mut`, so you can read a function and know
what it may change by reading its signature. That local reasoning is
what Abrahams argues is the real source of Swift's and Hylo's safety,
and it is the part a Python programmer notices first: `let b = a` moves
the list, and `a` cannot be used afterwards.

## What it costs

Sisu gives up things that Rust can express. The proposal names them, and
so does the compiler's list of programs it rejects.

- Some data structures need `Shared`, `Weak` or an arena. A doubly
  linked list, a tree with parent pointers or an observer that keeps a
  reference to its subject cannot hold references; it holds keys into an
  arena or counted pointers. Rust needs `Rc<RefCell<T>>`, raw pointers
  or an arena for the same structures, so the difference is smaller than
  it looks, but Sisu has no plain reference to fall back on.
- A closure that outlives its caller must own what it uses: it is an
  `own` closure, which moves or copies its captures. That is the rule of
  Swift's escaping closures and of Rust's `'static` ones, made uniform.
- Views spread upwards. A struct with a `str` field is a view or holds a
  `String` instead, and a view cannot be returned from the frame that
  owns its text. A program that wants to keep tokens after the buffer
  they point into is gone has two ways out, which [chapter
  12](../tutorial/12-views.md#two-ways-out) shows: own the data, at the
  cost of a copy, or keep the work inside the frame. Rust has the same
  rule for `&'a str`; Sisu removes the name, not the rule.
- One span per view. A struct whose fields borrow from two places that
  end at different times is split, where Rust would give it two lifetime
  parameters.
- The checker rejects some programs that are safe. It is sound before it
  is complete: where it cannot tell, it says no. A borrow of `xs[0]`
  counts as a borrow of every element, a call borrows each argument
  whole, and `from` is read as "may borrow from all of these" even when
  the body returns one. Each such case is listed in the compiler's
  [architecture
  notes](../../compiler/docs/architecture.md#known-over-rejections), and
  `sisu run --defer-borrow` reruns a rejected program under the
  ownership monitor to show whether the conflict happens at all.
- Counting is not free. Each `.copy()` of a `Shared` value updates a
  count, atomically when the value can reach another task. The design
  adds the reuse of Koka, Lean and Roc (D-005): when the compiler can
  prove that a `Shared` value has one owner, it changes the value in
  place.
- There is no "allocate and forget". Application code thinks about
  memory a little more than in Swift and much less than in Rust, which
  is a claim to be measured, not a fact. Per-request regions, which
  D-060 designs, give allocation-heavy services fast allocation and
  freeing all at once, and they cost a type parameter: a function that
  builds a list must be generic over its allocator to work inside a
  region.

## What would change the design

The research states the bet plainly: that ownership without lifetimes,
with views and arenas, covers what Rust's lifetimes cover in practice,
at a fraction of the learning cost, and keeps all of Rust's measured
safety benefit. It is the first of the three bets that the experiment
tests ([16, Validation plan](../../proposal/16-validation-plan.md), bet
1). People who did not design Sisu write six programs at the places
where Rust programmers meet lifetimes, among them a parser whose tree
borrows from its source, a graph with parent pointers, a server with a
shared cache and a widget tree with callbacks, and each writes the same
program in Rust. The pre-registered verdict
([20](../../proposal/20-pre-registration.md#22-bet-1-ownership-without-named-lifetimes-d-004-q-1))
holds if the rate of "fighting the checker" is at most half the same
author's Rust rate and at most one program needs `unsafe` for an
ordinary pattern. It fails if more programs need `unsafe`, or if the
rate is more than 0.8 times Rust's. A failure has a named answer: a
reference type checked by generation outside arenas, or named lifetimes
as an opt-in annotation on `view struct`, the one thing this page
explains the absence of.

Smaller measurements guard the parts. More than one `.copy()` per 30
lines of application code reopens implicit copying (Q-3). An
escape-hatch density (`Shared`, `Weak`, `Cell` and arena keys per
hundred lines) more than 1.5 times the Rust version's in three or more
programs counts against the bet. Any violation that the ownership
monitor finds in a program the checker accepted blocks the first
release, and one involving views reopens the rules for views and
escapes. The owner has closed the question of an optional collector
(Q-1), with the corpus still to show that the model is expressive enough
without one.

One unknown stays open whatever the experiment shows. The Android
numbers measure Rust's safety, but they cannot say how much of it comes
from the borrow checker and how much from bounds checks, from the
absence of undefined behaviour in safe code, and from sum types
([memory-management.md](../../research/topics/memory-management.md),
finding 3). Sisu keeps all four, with Rust's aliasing rule unchanged, so
the answer matters less to it than to a language that drops the borrow
checker altogether, as Zig does, betting that the other three give most
of the benefit.
