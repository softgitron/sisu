# 04 - Memory and ownership

This is the most consequential and the least settled part of the design.
The decision is made on the evidence available. The owner closed its two
largest questions in October 2026 (Q-1, no tracing GC; Q-2, one
`Shared[T]`; see [`../open-questions.md`](../open-questions.md)), and the
prototype's corpus must still show that the model is expressive enough
(bet 1 in [16-validation-plan.md](16-validation-plan.md)).

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
5. **Shared ownership is a type.** `Shared[T]` is a reference-counted,
   immutable-by-default pointer. Copying one is written `.copy()` like any
   other copy; the cost is a count increment. Mutation through it goes
   through `Cell[T]` (single task) or `Mutex[T]`/`Atomic` (any task), and
   the `Send`/`Sync` rules make it impossible to pick the wrong one. Cycles
   are broken with `Weak[T]`; there is no cycle collector (Q-2) and no
   tracing region (Q-1).
6. **Destruction is deterministic.** When an owner goes out of scope its
   value's `Drop` runs, then its fields' in reverse declaration order. Drop
   runs at the end of the scope, not at last use (Mojo's ASAP destruction
   was considered; scope-end is more predictable for locks and guards and is
   what every RAII language does). The compiler warns when a value with a
   `Drop` impl is unused after a point and holding it is expensive (a lock
   guard held across a long computation). Three kinds of value are never
   leaked: a suspended task frame, a `task.scope` and any `view` value end
   only by running their `Drop` (leaning, D-058; see Guaranteed drop
   below).
7. **Allocation is explicit at the type level, not the call site.**
   `list`, `dict`, `String`, `Box`, `Shared` allocate; a struct, an array, an
   enum, a tuple do not. A collection's allocator is part of its type, as a
   type parameter with a default, so `list[T]` is `list[T, Global]`
   (leaning, D-060; see Allocators). There are no hidden heap allocations
   for closures, generics or trait objects: a closure that captures by
   reference is a stack value; `dyn` requires a visible `Box` or `ref`.

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
- **Completion I/O into borrowed buffers.** Because views and suspended
  frames are never leaked, io_uring and DMA can write straight into a
  buffer that the caller's frame owns, with no buffer pool and no
  owned-buffer round trip (see Guaranteed drop below). Rust cannot promise
  this in safe code, because leaking is safe there.
- **Value semantics by default.** Two variables never alias. A `list[T]`
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
  `Arena[T]` with generational keys (`Key[T]`) in the standard library to make the
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
- **Reference counting is not free.** `Shared` costs a count increment per
  copy, atomic when the value can reach another task; the compiler chooses
  atomic or non-atomic counting by escape analysis, and the developer never
  does (Q-2). Perceus-style reuse (Koka, Lean, Roc) is applied: when the
  compiler can prove a `Shared` value is uniquely owned at a mutation site,
  it mutates in place.
- **No tracing GC means no "just allocate and forget".** Application code
  in Go or Java never thinks about this; Sisu application code thinks about
  it slightly more than Swift and much less than Rust. The owner decided
  against an optional GC tier for application code (Q-1). Per-request
  regions (see Allocators) give allocation-heavy services bump-allocation
  speed and wholesale freeing without one, and the prototype's corpus
  measures the friction that remains.

## Borrowing rules in detail

```sisu
var xs = [1, 2, 3]
let first = ref xs[0]       # immutable borrow of part of xs
xs.push(4)                  # error: xs is borrowed by `first` (would need `mut xs`)
print(first)                # borrow ends after last use (non-lexical)
xs.push(4)                  # fine

def longest(a: str, b: str) -> ref str from a, b:
    return if len(a) > len(b): a else: b
def head(xs: list[int]) -> ref int:          # single borrowed param: no `from` needed
    return ref xs[0]
def get(self, i: int) -> ref mut Item: ...   # `self` wins when present
```

Elision rules, in order: one borrowed parameter; otherwise `self`; otherwise
a `from` clause is required. A parameter counts as borrowed when it is
passed without `own` and is not `Copy`, or when its type is a view (`str`,
`Slice[T]`), since a result may point into a view. The same three rules as
Rust's, with `from` replacing lifetime names.

### Views

```sisu
view struct Token:
    kind: TokenKind
    text: str           # borrows from the source buffer
    line: int

def tokenize(src: str) -> list[Token]: ...   # the list is also a view: it holds views
```

A view's lifetime is the minimum of its borrowed parts. A `list[Token]`
where `Token` is a view is itself view-like and cannot escape the frame that
owns `src`. There is no way to express "this token borrows from one buffer
and that field from another"; if that is needed, split the struct. Rust's
experience is that multi-lifetime structs are rare and usually a design
error. The same rule covers allocators: a collection allocated in a
borrowed region, `list[T, ref Region]`, holds a reference and is a view
(see Allocators).

### Closures

A closure borrows its captures. A closure stored or sent somewhere must be
an `own` closure (`own x -> ...`, `do own x:`), which moves or copies the
captures in. The compiler reports
which capture prevented a closure from escaping. Whether a function type
also records how a closure captures (borrowing, mutating or owning),
which `std.par` bodies and `pure` callbacks need, is open (Q-52).

### Iterators and projections

`for x in xs` borrows `xs` and yields `ref T`; `for x in mut xs` yields
`ref mut T`; `for x in own xs` consumes. Methods that return a reference to
part of `self` (`get`, `first`, `entry`) are ordinary functions returning
`ref`; Hylo's subscript/projection concept is implemented by them without a
separate declaration form.

## Destruction in detail

### Guaranteed drop

Rule 6's guarantee in full: a suspended task frame, a `task.scope` and any
`view` value end only by running their `Drop`, on scope exit or during
unwinding, and std offers no safe `forget` or `leak` for them. The design
nearly implied this already, since views are second-class and a scope
joins before it returns; D-058 states it as a rule because the runtime's
I/O depends on it. `Shared` cycles can still leak other values, which is
harmless because those cannot hold borrows.

A blocking operation that lent a buffer to the kernel or a DMA engine
returns, even when cancelled, only after the device has released the
buffer; an io_uring cancellation that comes back `-EALREADY` waits for the
original completion. Completion-based I/O and DMA therefore borrow
`mut buf` from the caller's frame instead of taking owned buffers:

```sisu
def fill(file: File, mut buf: Slice[u8], offset: int) -> int ! {IoError, TaskError}:
    return file.read_at(mut buf, offset)     # io_uring writes straight into the caller's buffer
# Cancelled mid-flight, read_at submits a kernel cancel, parks until the kernel has let go of
# `buf`, and only then returns Err(Cancelled).
```

Rust cannot offer this safely, because "any object can be leaked in Rust,
trivially and safely": tokio-uring takes buffers by value and hands them
back, and `embedded-dma` requires `'static` buffers. The enabling rule has
shipped twice, in Rust's closure-based `std::thread::scope` and in Swift's
noncopyable types, whose `deinit` always runs because SE-0390 rejected a
general `forget`
([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
candidate 7). Phase 1's harness checks the promise: it cancels a million
in-flight io_uring reads into stack buffers, and no buffer may be written
after its cancelled call has returned
([18-next-steps.md](18-next-steps.md), Q-49).

Process abort is the exception. Under `panic = abort`
([05-errors.md](05-errors.md)) and on a double panic no `Drop` runs, so a
DMA-lending API is unavailable with `panic = abort` unless the abort hook
resets the device. The rule costs cancellation latency, which becomes
"until the device lets go", and it constrains std permanently. It also
binds the fallback of bet 3: if the runtime experiment moves task bodies
to stackless state machines
([16-validation-plan.md](16-validation-plan.md)), those stay owned by
their scope and never become droppable future values, or Sisu inherits
Rust's problem.

### Must-consume values

A value of a type marked `@must_consume` must be consumed: letting it go
out of scope without passing it to an `own` parameter is a compile error,
and unwinding on panic is exempt (D-054). A file whose final flush can
fail, a transaction that must be committed or rolled back, and a receipt
that must be repaid are the cases it is for. It is opt-in per type because
Austral found strict linearity hostile to loops and early return
([`austral.md`](../research/languages/austral.md)).

A container of must-consume values is itself must-consume (leaning,
D-064), as Move's `Cup<T>` has `drop` only if `T` does. Consuming
iteration discharges it. A `?` out of such a loop is an error unless the
remainder is returned, because Sisu's `?` returns normally where a failed
Move transaction rolls back:

```sisu
@must_consume
struct Receipt:                 # no Copy, no implicit drop
    amount: int

def settle(mut pool: Pool, own receipts: list[Receipt]) -> ! RepayError:
    for r in own receipts:      # list[Receipt] is must-consume because Receipt is
        repay(mut pool, r)?     # error: `?` would drop the receipts not yet visited
    return Ok(())
```

This answers the half of Q-37 that the owner left to practice, whether
generic containers need special handling, and std gains consuming
container APIs to match
([`capabilities-distribution-and-resources.md`](../research/unconventional/capabilities-distribution-and-resources.md),
idea 1).

## Unsafe

`unsafe:` blocks (and `unsafe e` for one expression) permit raw pointers (`ptr[T]`, `ptr mut T`), FFI calls,
and the handful of operations whose safety the compiler cannot check. The
rules follow Rust's: unsafe code must uphold the invariants safe code relies
on (the guaranteed-drop rule among them), the block is the audit unit, and
the compiler's safety guarantees apply unchanged to safe code.
Additionally, `unsafe` is a **package permission**:
a dependency may contain `unsafe` only if the root manifest lists it under
`[trust]`, which is what makes capabilities enforceable
([09-modules-packages-and-build.md](09-modules-packages-and-build.md)).

## Allocators

The global allocator is replaceable at link time. Collections take their
allocator as a defaulted type parameter, `list[T, A: Allocator = Global]`
([02-syntax.md](02-syntax.md)): a use site that omits it gets `Global`, so
ordinary code never writes it, and `list.with_allocator(a)` builds a
collection on another allocator. Zig's explicit-allocator-everywhere
design was studied closely: it is the right default for systems code and a
constant tax for application code. Sisu's default is Rust's (implicit
global), with Zig's available by choice and visible in the type, and with
the `freestanding` runtime profile requiring an explicit allocator because
there is no global one
([12-interop-and-targets.md](12-interop-and-targets.md)).

A borrowed bump `Region` is a valid allocator, so a collection built on
one, `list[T, ref Region]`, holds a reference and is a view under the
existing rule. It cannot escape the frame that owns the region, be stored
in `Shared` or cross `spawn` (a bump region mutates through a shared
reference, so it is not `Sync`), and resetting the region needs
`mut region`, which the checker grants only once every collection using it
is dead. Per-request and per-query regions (nginx pools, PostgreSQL memory
contexts, protobuf arenas) become safe with no new mechanism and no
lifetime names. Element `Drop` still runs, so destruction stays
deterministic and only the release of memory is batched.

```sisu
from std.mem import Region

def handle(req: Request) -> Response ! HttpError:
    with Region.bump(capacity=65536) as region:       # one region per request, freed at the dedent
        var headers = list.with_allocator(ref region)  # list[Header, ref Region]: a view
        parse_headers(req.head, mut headers)?
        return Ok(render(headers))                     # the Response is built in the global heap
        # return Ok(headers)  # error: `headers` borrows `region` and cannot leave this block
```

Rust merged the stabilisation of `Vec<T, A>`, `Box<T, A>` and
`impl Allocator for &A` in September 2026, for Rust 1.100 and ten years
after RFC 1398; `Vec<T, &'a Bump>` is this design with a named lifetime.
Go's arena experiment is on hold because explicit arenas are infectious to
APIs, and the same cost applies here: a function that builds a `list` must
be generic over `A` to work inside a region. The corpus measures both
sides: a 20% end-to-end gain on the HTTP server with per-request regions
keeps the rule, and the count of functions that had to become
allocator-generic says what it costs
([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
candidate 8). A `Region` is not the generational `Arena[T]` above: a
region is an allocator whose memory is released all at once, an arena is a
collection of one type addressed by `Key[T]` handles, and std keeps the
two names apart.

Allocation failure panics by default; `list.try_push` and friends return
`! OutOfMemory` for code that needs to handle it (kernels, embedded), as
Q-4 decided.

## Resource attributes

Rule 7 makes every heap allocation visible in a type, and every blocking
point is a capability call, a synchronisation primitive or an `@blocking`
extern ([06-concurrency-and-capabilities.md](06-concurrency-and-capabilities.md)).
Together they make three checked attributes cheap (leaning, D-059):

- `@no_alloc`: no heap allocation, which means no construction or growth
  of `list`, `dict`, `String`, `Box` or `Shared` and no allocating callee.
- `@no_block`: no capability call, lock, channel operation, `join`,
  `select`, `@blocking` extern or compiler-inserted yield point, so the
  function runs to completion.
- `@stack(max=N)`: the deepest chain of frames fits in `N` bytes, with
  recursion allowed only through a `decreases` measure, which gives that
  contextual keyword its first semantics ([07-contracts.md](07-contracts.md)).

`@no_alloc` and `@no_block` are part of version 1. `@stack` arrives with
the native backend, which knows frame sizes; `sisu build --report
resources` then prints the bound per task entry point and per
`freestanding` image. The checks run over the call graph after
monomorphisation, a stated exception to the definition-site checking of
generics (D-017, [03-types-and-generics.md](03-types-and-generics.md)),
and an error is reported at the instantiation, as Swift and Clang report
theirs.

```sisu
@no_alloc
@no_block
def render(mut out: Slice[f32], mut synth: Synth):     # the audio callback, every 2.7 ms
    for i in range(len(out)):
        out[i] = synth.next_sample()
# error[E0951]: `render` is @no_alloc, but `Synth.next_sample` may allocate:
#   it calls `self.voices.push(v)` (synth.sisu:88), and `list.push` can grow the list
```

Panic paths are exempt, as they are for `@must_consume`. Drop glue for
recursive, auto-boxed types (D-043) must be iterative, or any function that
drops a tree has an unbounded stack. `@no_block` suppresses yield points,
so a long `@no_block` loop on a task thread starves its siblings (Q-34)
and gets a lint. Clang 20 checks `[[clang::nonblocking]]` and
`[[clang::nonallocating]]`, with RealtimeSanitizer as the run-time
complement; Swift has experimental `@_noAllocation` and `@_noLocks`; Ada
has `pragma Restrictions` and GNATstack
([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
candidate 5). Three realtime-shaped ports (an audio callback, an
interrupt-driven UART driver, a game frame loop) keep the attributes if
fewer than one rejection in ten is a false positive the author cannot fix
by restructuring.

`@no_panic` follows after 0.1. It asks the compiler to show that no panic
is reachable from a function: no trapping arithmetic that can overflow, no
out-of-bounds index, no `unwrap` or `expect`, no failing contract, no
allocation failure. Without the verifier the proof is syntactic plus
interval arithmetic over refinement bounds and `requires` clauses. That
interval analysis is written into the specification, because a program
must not pass or fail by compiler version (D-036). Until the attribute
exists, `sisu audit --panics f` lists every potential panic site reachable
from `f`. SPARK's Silver level and Wuffs, which discharges every check at
compile time and has decoded GIFs in Chrome since 2021, are the precedents
([`systems-and-determinism.md`](../research/ideation/systems-and-determinism.md),
candidate 6).

Whether these attributes enter function types, so that `dyn` methods and
closure parameters can require them, is open (Q-52): that would put three
tracked properties beside `pure`, the one Q-17 allows. The alternative
checks the attributes per function over the call graph and treats every
`dyn` call and closure parameter as allocating and blocking, so realtime
code gives up higher-order style, as Wuffs code does. The three ports
above settle it.

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
