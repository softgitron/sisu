# Hylo (formerly Val)

## Snapshot
| | |
|---|---|
| First appeared | Public as "Val" in 2022; renamed Hylo in 2023. Racordon says the first ideas were "scribbled on a notepad four and a half years" before July 2025, i.e. early 2021 [8]. |
| Designed by | Dimi (Dimitri) Racordon (EPFL LAMP; earlier Univ. of Geneva and Northeastern) and Dave Abrahams (Adobe at the time of the MVS paper; previously on Apple's Swift team, founder of Boost), with a small open-source team (Ambrus Tóth and others) [1][2][7]. |
| Current version (Oct 2026) | No numbered release. The original compiler repo (`hylo-lang/hylo`) is frozen at tag `v-old-0.0.42`; its README says "Most of our efforts now go towards the new compiler at hylo-new". The new compiler (`hylo-lang/hylo-new`, Swift 6.3.2, LLVM 23) had 678 commits, 52 stars and a last commit on 11 Oct 2026, with no release at all [1][3][4][5]. |
| Governance | Open source (Apache-2.0), academic-led; twice-weekly community meetings; EPFL research collaboration page [1][6]. |
| Typing | Static, nominal, generic programming with traits in the Swift/C++-concepts tradition; "novel approaches to compiling generics with coherence" (claimed, not independently evaluated) [6]. |
| Memory | Mutable value semantics (MVS): no first-class references; values owned inline; no GC; explicit copies ("avoids hidden costs such as implicit copies"); unsafe pointers only via "explicit, auditable opt-in" [7][9]. |
| Compilation | Ahead-of-time via LLVM; compiler written in Swift [1][3]. |
| Primary domains | "High-level systems programming"; target audience is C++/Swift programmers [7]. |
| Notable users | None. Research/prototype stage. |

## Stated design goals
The homepage pitch is "A Safe Systems Programming Language, all in on Value
Semantics and Generic Programming" [6]. The introduction gives three goals:
"fast by definition" (no hidden costs, in-place mutation supported by the
type system), "safe by default" (ordinary code is "memory safe, typesafe, and
data-race-free"; unsafe constructs are "explicit, auditable opt-in") and
"simple" (borrows heavily from Swift, "de-emphasizes reference semantics" and
so avoids "memory regions, lifetime annotations, etc.") [7]. Racordon's 2025
summary: the premise is that "(mutable) value semantics can be leveraged for
safety and efficiency" [8].

The intellectual lineage is explicit. Abrahams co-designed Swift's value
types and `inout`; the JOT 2022 paper studies "Swiftlet", a subset of Swift
"focused exclusively on MVS", and Hylo is the attempt to build a language that
is *only* that core, with Swift's reference-type half removed [2][7]. The
"Swift done right" framing is the community's, not an official slogan; what
the designers actually say is that Swift already "adopted MVS at the core of
its semantics" and Hylo pushes it to "the discipline's strictest form" [2].

## The central bet: mutable value semantics

### What MVS is
From the paper: "Mutable value semantics is a programming discipline that
upholds the independence of values to support local reasoning. In the
discipline's strictest form, references become second-class citizens: they
are only created implicitly, at function boundaries, and cannot be stored in
variables or object fields. Hence, variables can never share mutable state.
Unlike pure functional programming, however, mutable value semantics allows
part-wise in-place mutation" [2]. Consequently "all values form disjoint
topological trees rooted in the program's variables" [2]. Every type in Hylo
is a value type; "their instances behave like ints" [7].

### Parameter passing conventions replace references
Instead of reference types, Hylo has four ways to pass an argument [9][10]:

| Convention | Caller side | Callee may | Rust analogue |
|---|---|---|---|
| `let` (default) | `f(x)` | read; nothing else can mutate `x` during the call; cannot escape without copy | `&T` |
| `inout` | `f(&x)` (`&` marks mutation, not address-of) | mutate in place, even temporarily deinitialize, must leave a valid value | `&mut T` |
| `sink` | `f(x)` (last use) | take ownership; caller's binding dies | move |
| `set` | `f(&x)` on uninitialized `x` | must initialize | placement-new |

The spec's rules: "`sink` parameters are immutable and escapable", "`inout`
parameters are mutable and escapable, and they must be alive at the end of
every terminating execution path", "`set` parameters are dead and mutable"
[10]. Independence is enforced at the call site: "`inout` and `sink` arguments
must be unique. The same variable cannot be passed to two parameters of one
call" [9]. This is Swift's Law of Exclusivity applied statically: the paper
shows that because `inout` parameters "are second-class citizens: they have
lexically-bounded lifetimes", "aliases can be prevented simply by verifying
that the 'path' to a value (i.e., a list of member accesses and/or array
subscripts) never appears twice in inout arguments to a single function"
[2]. No lifetimes, no regions, no borrow graph: the check is syntactic
overlap of access paths, plus a dynamic check for subscripts with computed
indices (`swap(&x[0], &x[1])` is fine, `swap(&x[f(0)], &x[1])` is checked at
run time in Swiftlet) [2].

### Projections and subscripts: borrowing without reference values
Hylo's answer to "return a reference into a container" is the *subscript*, a
coroutine that *yields* access to part of a value for a bounded duration:

```hylo
subscript min<T: Comparable>(_ a: T, _ b: T): T {
  let   { yield if a > b { b } else { a } }      // read-only projection
  inout { yield &(if a > b { &b } else { &a }) } // mutable projection
}

fun main() {
  var x = 2
  var y = 3
  &min[x, y] += 1   // mutates x in place through the projection
}
```

The spec: "A projection exposes an object yielded by a subscript call or
property access" and "If a projection `p` projects an object `o` mutably, `o`
is inaccessible for the duration of `p`'s lifetime"; immutably, "`o` is
immutable for the duration of `p`'s lifetime" [10]. The introduction's
`longer_of` example is "semantically similar to a Rust function that mutably
borrows its arguments and returns a mutable reference bound by their
lifetimes", yet "Hylo has no lifetime annotations" because it "uses a simpler
model, devoid of references": the projection cannot outlive the yield, so
there is nothing to annotate [7]. Subscripts come in bundles (`let`, `inout`,
`set`, `sink`); the compiler synthesizes missing variants (e.g. `sink` from
`let`) and picks one from use context; a `yielded` parameter placeholder lets
one body serve several conventions [11]. Methods are likewise "bundles" of
`let`/`inout`/`sink` implementations selected by how the receiver is used
[9]. Field access and computed properties are the degenerate case of this
mechanism, so `a.b.c += 1` is a nested mutable projection, not a pointer chase.

This is the precise sense in which Hylo claims most Rust lifetime annotations
are unnecessary: the paper argues that approaches with first-class references
"invariably complicate type systems with mechanisms like named lifetimes,
which significantly raise the barrier to entry" and MVS "sits at [a] third
point in the design space where both goals are satisfied and mutation is
supported, without the complexity inherent to flow-sensitive type systems"
[2]. Borretti's independent analysis agrees that the simplification is
real—with second-class references, borrow checking reduces to "check that the
reference is a function argument, that the variable isn't already dropped, and
that mutable paths don't overlap"—and also names the cost: "Iterators are the
main problem, since they store references in structs" [12].

### Shared mutable state and escape hatches
In safe Hylo there is no shared mutable state at all: no reference types, no
`Rc<RefCell>`, no globals with reference semantics. Patterns that need
identity (graphs, observers, doubly linked lists) are to be expressed with
indices into owned containers; the homepage claims "double linked lists with
value semantics" are "safe and faster than using reference semantics" [6]
(no benchmark is published for that claim). The escape hatches are the
unsafe pointer types in the standard library built on `Builtin.Pointer`, "an
untyped pointer to a memory location" whose misuse is undefined behaviour,
intended for writing safe abstractions [10]. Concurrency is meant to inherit
safety from independence: spawned work receives `sink`/`let` values, so there
is nothing to race on [13].

## Strengths

### Memory and data-race safety with a small rule set
The whole safety story is: values are independent, `inout`/`sink` arguments
are unique, projections bound access. No lifetimes, no regions, no `Send`/
`Sync`, no aliasing XOR mutability on stored references because there are no
stored references. Evidence that the discipline is viable at scale is
indirect but real: the paper cites Swift's standard library, the Boost Graph
Library and Swift for TensorFlow as large MVS codebases [2].

### In-place mutation of parts without copying
MVS "allows part-wise in-place mutation, thereby eliminating the memory
traffic usually associated with functional updates of immutable data" [2].
The paper's compilation strategy: fixed-size values on the stack, enabling
"numerous off-the-shelf compiler optimizations"; dynamically sized containers
use copy-on-write in Swiftlet; Hylo instead makes copies explicit [2][7].

### Projections unify fields, properties, subscripts and borrows
One construct covers `x.f`, computed properties, indexing and "return a
reference into a collection", with read/write/consume variants synthesized by
the compiler [11]. This is more uniform than Swift (`_read`/`_modify`
accessors are unofficial) and than Rust (`Index`/`IndexMut`/`Deref` traits).

### Structured concurrency without function colouring (planned)
The design principles: concurrent code "is written like non-concurrent code",
"no function coloring such as `async`", `spawn`/`await` with thread
inversion, custom schedulers via `spawn(scheduler:)`, cancellation via
`StopToken` [13]. This is a design document, not an implementation.

## Weaknesses

### Maturity: two compilers, zero releases, tiny team
The old compiler's final tag is `v-old-0.0.42`; the new one has no release,
52 stars and most commits from two people [3][4][5]. The homepage admits the
compiler and standard library are "still in their early stages" [6]. Racordon's
2025 talk is partly about "challenges in keeping the project alive and its
community engaged" [8]. Nothing has been built in Hylo except its standard
library and samples. Every performance claim below the paper's Swiftlet
benchmarks is therefore unverified for Hylo itself.

### Expressiveness: no stored references means no first-class iterators or graphs
Borretti's analysis of second-class references (which he explicitly maps to
Hylo's MVS) lists what is lost: "Generality. Iterators are the main problem,
since they store references in structs. Options are coroutines, typed indices
(which lose the tie to their collection), or unsafe pointers"; typed indices
"can dangle and be used on the wrong collection"; operating on an element
"often requires passing the whole container", the "part-whole conflict" [12].
Hylo's answer—projections are coroutines—is elegant but pushes every
"cursor" abstraction into a yield-based style whose compilation (stack
splitting or CPS) is non-trivial and still being worked out in the new
compiler. The MVS paper restricts closures to explicit capture-by-copy for the
same reason [2].

### Concurrency is undesigned
"The concurrency approach of Hylo is still under design" [13]. The claimed
data-race freedom is a consequence of value semantics only if concurrent
tasks never share, which rules out shared caches, work-stealing queues and
other mutable structures without an escape hatch; what that escape hatch
looks like is unspecified.

### Learning curve of four conventions plus bundles
Programmers must choose `let`/`inout`/`sink`/`set` for every parameter and
understand when the compiler synthesizes method variants. The designers argue
"a developer can often write only the `let` variant and let the compiler
supply the rest, when performance is not a concern" [9]; whether this stays
simple in generic code is untested.

### No published evaluation against Rust
The only measurements are the paper's comparison of Swiftlet with Swift, Scala
and C++ on "handwritten and randomly generated programs with varying numbers
of mutating operations" [2]. There is no evidence yet on compile times,
generated-code quality for projections, or developer productivity.

## Designer retrospectives
- Racordon (2025): the design "took a 'surprisingly long road'" and "has
  solidified into a coherent system offering promising results", while "the
  language is still under development" [8].
- The decision to abandon the first compiler for `hylo-new` (2025) is itself a
  retrospective: the README of the old repo now says the new compiler is where
  effort goes, and the old release notes call the new one "still incomplete"
  [1][3]. No public document explains what was wrong with the first design.
- Abrahams and Racordon on Swift (paper): Swift's implicit-capture closures
  "introduce reference semantics" and its Law of Exclusivity violation "is
  detected only at runtime", both of which Swiftlet/Hylo fix statically [2].

## Influence
- **Took from**: Swift (value types, `inout`, protocols/generics, method
  bundles generalize Swift's `_read`/`_modify`), C++ generic programming
  (Stepanov; the README links fm2gp.com), Rust (uniqueness of `inout`), and
  Dave Abrahams's and Jeremy Siek's work on the Boost Graph Library [1][2].
- **Gave to**: Swift's own evolution towards non-copyable types, `borrowing`/
  `consuming` parameter modifiers and `~Copyable` (2023-24) moves Swift in the
  same direction (covered by the Swift researcher). Borretti considered
  replacing Austral's first-class references with Hylo-style second-class ones
  [12]. Ovadia's taxonomy lists MVS as a distinct approach and corrects his
  earlier conflation of it with "not-MVS" (one reference per object) [14].
  Mojo's argument conventions (`borrowed`/`inout`/`owned`, since renamed) are the same vocabulary
  (covered by the Mojo researcher).

## Lessons for Sisu
- **Adopt**: Argument-passing conventions as the primary borrowing mechanism.
  `let`/`inout`/`sink` on parameters, with uniqueness of `inout`/`sink`
  arguments checked by access-path overlap, gives most of Rust's safety for
  function calls with no lifetime syntax at all. Also adopt the principle that
  "`&` marks mutation at the call site", so every in-place mutation is visible
  in the caller's text.
- **Adopt**: Projections/accessors as coroutines for the "return a reference
  into a collection" problem, at least for fields, properties and indexing.
  This is what makes `v[i].x += 1` safe and allocation-free without a stored
  reference.
- **Adapt**: Hylo's "no first-class references anywhere" is too strict for a
  general-purpose language; iterators, cursors, observers and intrusive
  structures need *some* stored-reference story. Sisu should treat MVS as the
  default tier and add a bounded second tier (e.g. region-scoped or
  generation-checked references, see `vale.md` and `pony.md`) rather than
  pretend indices suffice. The open question is whether that second tier can
  be kept out of ordinary code's type signatures.
- **Adapt**: Explicit copies (Hylo) versus copy-on-write (Swift). Hylo's choice
  makes costs visible but is noisy; a middle ground is implicit copies only
  for types that are cheap (trivially copyable) and explicit `.copy()`
  otherwise.
- **Avoid**: Shipping the concurrency model last. Hylo's data-race-freedom
  claim rests on a design document; Sisu should prototype the shared-state
  escape hatch (if any) alongside the value model, not after.
- **Avoid**: Betting a language on a single research team's compiler. Two
  rewrites and no release in four-plus years is the main reason Hylo has no
  users; Sisu's plan must budget for a boring, shippable compiler early.
- **Open questions raised**: (1) Can yield-based projections be compiled to
  code as efficient as returning a pointer, in the presence of generics and
  separate compilation? (2) How much real-world code genuinely needs stored
  references once projections exist—there is no empirical study, and Rust's
  experience (most references are function arguments, per Borretti [12])
  suggests "less than feared" but not "none". (3) Does the four-convention
  vocabulary stay learnable when combined with generics and closures?

## Sources
1. Hylo repository README, https://github.com/hylo-lang/hylo (2026).
2. D. Racordon, D. Shabalin, D. Zheng, D. Abrahams, B. Saeta, "Implementation Strategies for Mutable Value Semantics", Journal of Object Technology 21(2), 2022, https://www.jot.fm/issues/issue_2022_02/article2.pdf.
3. Old compiler releases page (tag `v-old-0.0.42`), https://github.com/hylo-lang/hylo/releases (2026).
4. New compiler repository, https://github.com/hylo-lang/hylo-new (2026).
5. New compiler commit log (last commit 11 Oct 2026), https://github.com/hylo-lang/hylo-new/commits/main.
6. Hylo homepage, https://www.hylo-lang.org/ (2026).
7. Hylo introduction, https://www.hylo-lang.org/introduction/ (2026).
8. D. Racordon, "Designing Hylo, a programming language for safe systems programming", PLSS 2025 (ECOOP), https://2025.ecoop.org/details/plss-2025-papers/12/ (2025).
9. Hylo language tour, "Functions and methods", https://docs.hylo-lang.org/language-tour/functions-and-methods (2026).
10. Hylo specification, https://github.com/hylo-lang/specification/blob/main/spec.md (2026).
11. Hylo language tour, "Subscripts", https://docs.hylo-lang.org/language-tour/subscripts (2026).
12. F. Borretti, "Second-Class References", https://borretti.me/article/second-class-references (2023).
13. Hylo language tour, "Concurrency" (marked "still under design"), https://docs.hylo-lang.org/language-tour/concurrency (2026).
14. E. Ovadia, "Borrow checking, RC, GC, and the Eleven (!) Other Memory Safety Approaches", https://verdagon.dev/grimoire/grimoire (2023, updated).
15. D. Abrahams, "Values: Safety, Regularity, Independence, and the Future of Programming", CppCon 2022; D. Racordon, "Val Wants To Be Your Friend", CppCon 2022, https://github.com/CppCon/CppCon2022. (The C++Now 2022 talk "A Future of Value Semantics and Generic Programming" by Abrahams and Racordon is cited from memory and was not re-verified.)
