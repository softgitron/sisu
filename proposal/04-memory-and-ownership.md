# 04 - Memory and ownership

This is the most consequential and the least settled part of the design.
The decision is made on the evidence available; the two largest open
questions in [`../open-questions.md`](../open-questions.md) are about it.

## The problem

The evidence base ([`../research/evidence/memory-safety.md`](../research/evidence/memory-safety.md))
is unambiguous on one point: in languages without memory safety, memory
errors are the majority of severe security bugs (Microsoft, Chromium,
Android all report around 70%), and when a project moves new code to a
memory-safe language that share falls faster than the code is replaced
(Android: from 76% in 2019 to under 25% in 2024). A new language that is
not memory-safe by default would be ignoring the strongest single result in
the research. Within that class, temporal errors dominate: half of
Chromium's memory bugs and 17 of Project Zero's 39 in-the-wild memory
corruption 0-days in 2021 were use-after-free. Bounds checking alone (the
Zig/Odin position) addresses the smaller half; a lifetime discipline is
what addresses the larger.

The approaches, with their documented costs:

| Approach | Used by | Cost |
|---|---|---|
| Tracing GC | Java, Go, C#, OCaml, Haskell, JS | Runtime, pauses or barrier overhead, memory headroom (Hertz & Berger: 2-5x for equal throughput), no deterministic destruction, unsuitable for the smallest targets |
| Reference counting | Swift, Python, Nim, Koka/Lean (Perceus) | Retain/release traffic (Swift's measured 10-30% on pointer-heavy code), cycles leak without a collector or `weak`, still no aliasing control |
| Ownership + borrowing with lifetimes | Rust | Learning curve (the top reason non-adopters and ex-users give in Rust's surveys; about one current user in ten calls the borrow checker a big problem), lifetime annotations, self-referential structures, async interaction, API design burden |
| Mutable value semantics, no first-class references | Hylo, (Swift partially) | Immature; some patterns (graphs, intrusive structures, callbacks) need escape hatches |
| Linear types | Austral, (Idris 2) | Every value used exactly once: very explicit, verbose for plain data |
| Generational references | Vale (archived 2025; its successor Valen moved to "group borrowing") | Runtime check per dereference (5-11% on the one published benchmark); no shipped language uses it as the primary model |
| Manual, with debug checks | Zig, Odin, C | No guarantee in release builds |

## The decision

Sisu uses **ownership with value semantics, second-class references, and
explicit shared ownership**. Concretely:

1. **Every variable owns its value.** Assignment and passing to an `own`
   parameter move. Types marked `Copy` copy instead. Anything else is copied
   only by an explicit `.copy()`.
2. **Parameters borrow by default.** A parameter without a convention
   keyword is a read-only borrow for the duration of the call. `mut x`
   is an exclusive, in-place borrow (the caller writes `f(mut v)`).
   `own x` takes ownership.
3. **References exist but are second-class.** `ref T` and `ref mut T` can be
   bound to locals, passed as arguments, and returned from functions tied to
   a borrowed parameter. They cannot be stored in ordinary structs, in
   `Shared`, in collections, or captured by closures that escape. The
   compiler tracks their validity with the same aliasing rule as Rust (one
   mutable or any number of immutable), with non-lexical scopes, but with
   **no named lifetimes**: every reference's lifetime is the stack frame or
   the borrowed parameter it came from.
4. **View types are the escape hatch for storing references.** A struct
   declared `view struct` may contain `ref` fields. A view has exactly one
   implicit lifetime, the shortest of its fields', and is itself
   second-class. `str`, `Slice[T]`, iterators over borrowed collections,
   parsed tokens pointing into a source buffer, and `dyn Trait` references
   are all views.
5. **Shared ownership is a type.** `Shared[T]` is an atomically
   reference-counted, immutable-by-default pointer. Mutation through it goes
   through `Cell[T]` (single task) or `Mutex[T]`/`Atomic` (any task), and
   the `Send`/`Sync` rules make it impossible to pick the wrong one. Cycles
   are broken with `Weak[T]`; see open question Q-2 on whether to also ship a
   cycle collector or a tracing region.
6. **Destruction is deterministic.** When an owner goes out of scope its
   value's `Drop` runs, then its fields' in reverse declaration order. Drop
   runs at the end of the scope, not at last use (Mojo's ASAP destruction
   was considered; scope-end is more predictable for locks and guards and is
   what every RAII language does). The compiler warns when a value with a
   `Drop` impl is unused after a point and holding it is expensive (a lock
   guard held across a long computation).
7. **Allocation is explicit at the type level, not the call site.**
   `List`, `Map`, `String`, `Box`, `Shared` allocate; a struct, an array, an
   enum, a tuple do not. There are no hidden heap allocations for closures,
   generics or trait objects: a closure that captures by reference is a
   stack value; `dyn` requires a visible `Box` or `ref`.

## What this buys

- **No lifetime annotations.** Rust's `'a` is needed because a reference is
  a first-class value that may be stored anywhere; the compiler needs to be
  told how long. With references confined to the stack and to single-lifetime
  views, the compiler always knows. Graydon Hoare's retrospective and the
  Hylo project both identify first-class references as the source of most
  of Rust's annotation burden; Swift's `~Escapable` and Mojo's inferred
  origins are moves in the same direction.
- **No self-referential struct problem, and therefore no `Pin`.** A struct
  cannot contain a reference to itself because ordinary structs cannot
  contain references.
- **Borrowing across tasks is simple.** A `task.scope` is a stack frame;
  tasks spawned in it may borrow from the frame because the scope cannot
  return until they finish. Rust needs `thread::scope` plus `'static`
  bounds for everything else; Sisu has only the scoped form (see
  [06-concurrency-and-capabilities.md](06-concurrency-and-capabilities.md)).
- **Value semantics by default.** Two variables never alias. A `List[T]`
  passed to a function cannot be changed behind the caller's back unless the
  parameter says `mut`. This is the property Dave Abrahams argues is the
  real source of Swift's and Hylo's safety, and the one that makes local
  reasoning possible.

## What this costs, honestly

- **Some data structures need `Shared` or indices.** A doubly linked list,
  a graph with parent pointers, an observer that stores a reference to its
  subject, an intrusive list: in Rust these need `Rc<RefCell>`, raw pointers
  or an arena; in Sisu they need `Shared[Cell[T]]`, `Weak`, integer handles
  into an arena, or `unsafe`. The research on Rust shows this is already
  idiomatic practice (ECS architectures, `petgraph`'s index-based graphs,
  slotmaps); the research on Hylo shows the same answer. Sisu ships an
  `Arena[T]` with generational handles in the standard library to make the
  idiomatic answer the easy one. This is Vale's generation-checked
  reference idea ([`vale.md`](../research/languages/vale.md)) confined to
  an arena: a stale handle is detected at use, the check is elided when the
  compiler can see the arena was not mutated, and the mechanism never
  appears in ordinary code's signatures.
- **Callbacks that outlive their caller must own what they use.** A closure
  stored in a struct cannot borrow; it must capture by `own` (copy or move)
  or hold a `Shared`. This is the same rule as Swift's escaping closures and
  Rust's `'static` closures, made uniform.
- **Views are infectious upwards.** A struct holding a `str` field must
  itself be a `view struct` or hold a `String`. Rust has exactly this rule
  with `&'a str`; Sisu merely removes the name of the lifetime.
- **Reference counting is not free.** `Shared` costs an atomic increment
  per copy. Perceus-style reuse (Koka, Lean, Roc) is applied: when the
  compiler can prove a `Shared` value is uniquely owned at a mutation site,
  it mutates in place. Non-atomic `Shared` for single-task use is open
  question Q-2.
- **No tracing GC means no "just allocate and forget".** Application code
  in Go or Java never thinks about this; Sisu application code thinks about
  it slightly more than Swift and much less than Rust. Whether this is the
  right trade for the application tier, or whether an optional GC'd region
  should exist, is open question Q-1 and the one most likely to be revisited
  after a prototype.

## Borrowing rules in detail

```sisu
var xs = List.of(1, 2, 3)
let first = ref xs[0]       // immutable borrow of part of xs
xs.push(4)                  // error: xs is borrowed by `first` (would need `mut xs`)
println(first)              // borrow ends after last use (non-lexical)
xs.push(4)                  // fine

fn longest(a: str, b: str) -> ref str from a, b { if a.len() > b.len() { a } else { b } }
fn head(xs: List[int]) -> ref int { xs[0] }          // single borrowed param: no `from` needed
fn get(self, i: int) -> ref mut Item { ... }         // `self` wins when present
```

Elision rules, in order: one borrowed parameter; otherwise `self`; otherwise
a `from` clause is required. The same three rules as Rust's, with `from`
replacing lifetime names.

### Views

```sisu
view struct Token {
    kind: TokenKind
    text: str           // borrows from the source buffer
    line: int
}

fn tokenize(src: str) -> List[Token] { ... }   // the List is also a view: it holds views
```

A view's lifetime is the minimum of its borrowed parts. A `List[Token]`
where `Token` is a view is itself view-like and cannot escape the frame that
owns `src`. There is no way to express "this token borrows from one buffer
and that field from another"; if that is needed, split the struct. Rust's
experience is that multi-lifetime structs are rare and usually a design
error.

### Closures

A closure borrows its captures. A closure stored or sent somewhere must be
`own { }`, which moves or copies the captures in. The compiler reports
which capture prevented a closure from escaping.

### Iterators and projections

`for x in xs` borrows `xs` and yields `ref T`; `for x in mut xs` yields
`ref mut T`; `for x in own xs` consumes. Methods that return a reference to
part of `self` (`get`, `first`, `entry`) are ordinary functions returning
`ref`; Hylo's subscript/projection concept is implemented by them without a
separate declaration form.

## Unsafe

`unsafe { }` blocks permit raw pointers (`ptr[T]`, `ptr mut T`), FFI calls,
and the handful of operations whose safety the compiler cannot check. The
rules follow Rust's: unsafe code must uphold the invariants safe code relies
on, the block is the audit unit, and the compiler's safety guarantees apply
unchanged to safe code. Additionally, `unsafe` is a **package permission**:
a dependency may contain `unsafe` only if the root manifest lists it under
`[trust]`, which is what makes capabilities enforceable
([09-modules-packages-and-build.md](09-modules-packages-and-build.md)).

## Allocators

The global allocator is replaceable at link time. Collections take an
optional allocator at construction (`List.with_allocator(arena)`), and
`Arena` is in the standard library. Zig's explicit-allocator-everywhere
design was studied closely: it is the right default for systems code and a
constant tax for application code. Sisu's default is Rust's (implicit
global), with Zig's available by choice, and with the `freestanding` runtime
profile requiring an explicit allocator because there is no global one
([12-interop-and-targets.md](12-interop-and-targets.md)).

Allocation failure panics by default; `List.try_push` and friends return
`! OutOfMemory` for code that needs to handle it (kernels, embedded). Open
question Q-4.

## Comparison to the neighbours

| | Rust | Swift | Hylo | Mojo | Sisu |
|---|---|---|---|---|---|
| Default parameter passing | owned | borrowed | borrowed (`let`) | borrowed (`imm`) | borrowed |
| Named lifetimes | yes | rarely (`@lifetime` in 6.2) | no | inferred origins | no |
| References in structs | with lifetime | `~Escapable` only | no | with origin | `view struct` only |
| Shared ownership | `Rc`/`Arc` | classes (ARC) | planned | Python objects / `Arc` | `Shared` |
| Copy of non-trivial values | explicit `.clone()` | implicit (COW) | implicit for `Copyable` | implicit for `Copyable`, `^` moves | explicit `.copy()` |
| Destruction | scope end | last use (ARC) | scope end | last use (ASAP) | scope end |

Two further data points from the research bound the design from either
side. Nim's ARC/ORC (moves and destructors without a borrow checker) is
fast but not safe: its 2.2.x releases were still fixing use-after-free bugs
in the compiler's own move inference ([`nim.md`](../research/languages/nim.md)),
which is why Sisu's checks are a sound static discipline rather than an
optimisation. OxCaml's modes (locality, uniqueness, portability, inferred,
with legacy defaults; [`ocaml.md`](../research/languages/ocaml.md)) show
ownership-like control being added to an ML as orthogonal inferred
annotations; Sisu's `Send`/`Sync`/`Copy`/`Escapable` auto-traits are the
same idea with fewer axes.

The honest summary: Sisu's model is Rust's with first-class references
removed and the lifetime machinery that supported them collapsed into two
rules. The bet is that this removes most of the learning curve while
keeping all of the guarantees. The risk is that the removed expressiveness
is needed more often than the Rust codebases studied suggest. Validating
that bet on real programs is the first thing a prototype must do.
