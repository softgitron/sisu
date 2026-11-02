# 22 - Unsafe

Sisu's toolchain is two languages today. The front end is Sisu (stage 1,
at the fixed point of D-078), but the backend (`sisu-codegen`) and the
runtime (`sisu-rt`) are Rust, and the owner's goal is a toolchain written
only in Sisu. The backend is ordinary code. The runtime is not: it
allocates raw memory, keeps cells with counts in front of values, carves
task stacks out of reserved address space, switches between them, and
calls the operating system. This chapter is the sublanguage that lets
Sisu do those things itself, kept as small as the runtime allows. The
survey behind it is
[`../research/topics/unsafe.md`](../research/topics/unsafe.md); the
decisions are D-080 to D-084, D-085 and D-086 (leaning, coordinator,
November 2026), and the questions it leaves are Q-54 to Q-57, Q-71 and
Q-72. It replaces the short "Unsafe" section of
[04](04-memory-and-ownership.md) and refines the enforcement rules of
[06](06-concurrency-and-capabilities.md),
[09](09-modules-packages-and-build.md) and
[12](12-interop-and-targets.md).

## The problem

Every memory-safe language has code its compiler cannot check, and the
evidence on where to put it is consistent
([`unsafe.md`](../research/topics/unsafe.md), "What the evidence says"):

- **A hard boundary works; a lint does not get adopted.** Rust's `unsafe`
  is a compile-time rule, and an ecosystem of tools (cargo-geiger,
  cargo-vet, Miri, clippy's safety-comment lints) exists because of it.
  Swift 6.2's strict memory safety is a set of warnings that its own
  vision does not expect to become a default; D set its safe default too
  late; Safe Haskell's ecosystem never materialised.
- **The bugs live there and have a few shapes.** Every memory-safety bug
  in the studied Rust CVEs and corpora needs unsafe code, and the shapes
  recur: a panic that exposes half-built state, unsafe code trusting a
  trait implementation it does not control, a manual `Send` or `Sync` impl
  missing a bound, a safe wrapper with an unchecked precondition, a drop
  running on uninitialised memory (Rudra, Xu et al., RustBelt's
  `MutexGuard` bug).
- **Unsafe is small per package and transitive, and mostly foreign
  interop.** About one Rust crate in five uses `unsafe` and one in three
  calls one that does; in Go one popular project in four uses `unsafe`,
  and nine in ten have it somewhere in their dependency tree.
- **An interpreter can check it.** Miri interprets Rust's MIR, tracks
  provenance and initialisation, runs 70% of the ecosystem's tests and has
  found bugs in the standard library. Sisu already interprets SIR, its
  MIR, under a monitor that checks borrows at run time.

What Sisu said so far was four sentences: `unsafe:` blocks permit raw
pointers and foreign calls, the block is the audit unit, and only
packages the root manifest lists under `[trust]` may contain any. That
leaves open which operations are unsafe, what an unsafe block promises,
how a package says that it uses one, what the audit shows, how unsafe
code meets `pure`, comptime, the monitor and the simulation profile, and
how a runtime is built on it.

## The decision

1. **One form permits.** An unsafe operation may appear only inside an
   `unsafe:` block or an `unsafe e` expression. Anywhere else it is a
   compile error, in every profile and every edition (D-080).
2. **Two markers oblige and permit nothing.** An `unsafe def` has a
   precondition its callers must meet, so calling it is an unsafe
   operation; its body is ordinary safe code, in which unsafe operations
   still need a block. An `unsafe trait` has a promise its implementors
   must keep, so `unsafe impl` is an unsafe operation; `Send`, `Sync` and
   `std.mem.Allocator` are the standard ones. Rust needed an edition to
   separate the obligation from the permission; Sisu starts separated.
3. **The operations are library functions.** There are exactly three
   unsafe operations: calling an `unsafe def`, calling a foreign function,
   and writing an `unsafe impl`. Reading through a raw pointer, offsetting
   it, allocating and releasing raw memory, transmuting and switching
   stacks are `unsafe def`s in the standard library with intrinsic bodies,
   so the language gains no dereference operator, no address-of syntax and
   no pointer arithmetic. Creating, copying and comparing a raw pointer is
   safe.
4. **Raw memory lives in `std.mem`.** `ptr[T]` and `ptr mut T` mirror
   `ref T` and `ref mut T`: never null (`ptr[T]?` is the nullable C
   pointer, with its layout), strict provenance (no integer-to-pointer
   cast), `Escapable` exactly when `T` is, never `Send` or `Sync`. A typed
   raw buffer, `RawBuf[T]`, a layout type, the `Allocator` trait and a
   plain-data marker with a safe bit cast are the layer above them that
   most unsafe code uses instead (D-081).
5. **The block keeps a contract**, written out below: valid values,
   valid references for their whole origin, every value dropped once, no
   leaked views, scopes or frames, `Send` and `Sync` respected, no
   authority without a capability, nothing observable that depends on an
   address, and no reliance on safe code the block does not control.
6. **A package declares its unsafe code and the program consents.** A
   package's manifest says `unsafe = "memory"` or `unsafe = "extern"`;
   the root manifest's existing `[trust] unsafe` list consents; the
   lockfile records the level, and an update that raises it is refused
   without an explicit flag (D-082).
7. **The audit lists every block with its reason.** Every block, `unsafe
   def`, `unsafe impl` and foreign declaration carries a safety comment;
   the comment names the obligations it discharges; `sisu audit --unsafe`
   lists them by package and module; six lints keep the declarations
   honest (D-083).
8. **Unsafe is not a capability; authority stays with capabilities.** A
   trusted package that reaches the world through foreign calls must make
   that authority available only to code that holds the matching
   capability, and the runtime is the one place that turns foreign calls
   into root capabilities (D-084).
9. **The interpreter checks unsafe code.** Under the monitor, raw memory
   is a checked model (allocations, bounds, alignment, initialisation, the
   validity of the bits read, the loans of the references made from raw
   pointers), so `sisu test` checks unsafe code on every run; comptime uses
   the same model; an unsafe block does not make a function impure, its
   impure operations do, unless a `pure` function asserts them (D-085).
10. **The runtime is written in Sisu on this layer**, with three machine
    intrinsics instead of inline assembly, no mutable globals, and the
    Rust runtime retired only when the Sisu runtime reproduces it on every
    native case (D-086).

## Syntax

```sisu
# safety: <why each obligation of each unsafe operation below holds>
unsafe:
    ...

# safety: <the same, for the one expression>
let item = unsafe self.buf.read(self.head)

## Reads the `T` that `p` points to.
##
## Safety:
## - `valid`: `p` points to a live, initialised `T`
## - `aligned`: `p` is aligned for `T`
unsafe def read_one[T: Copy](p: ptr[T]) -> T:
    # safety: valid, aligned: this function's own obligations, which its callers meet.
    return unsafe p.read()          # the body of an `unsafe def` still needs the block

## What a collection allocates from.
##
## Safety (for implementors):
## - `valid_block`: `acquire` returns memory valid for reads and writes of
##   `layout.size` bytes, aligned to `layout.align`, overlapping no live block
## - `stable`: a block stays valid until it is released or the allocator ends
## - `pure`: neither method has an effect that `pure` code can observe, other
##   than the memory it hands out and takes back
unsafe trait Allocator:
    pure def acquire(self, layout: Layout) -> ptr mut u8?
    ## Safety:
    ## - `from_here`: `block` came from `acquire` on this allocator with `layout`
    ## - `once`: it is released once and not used after
    pure unsafe def release(self, block: ptr mut u8, layout: Layout)

# safety: a RawBuf owns the values in its slots, so moving it to another task moves
#   them, which is sound exactly when T is Send.
unsafe impl[T: Send, A: Allocator + Send] Send for RawBuf[T, A]:
    pass

# safety: signatures as POSIX declares them, for 64-bit targets.
extern "C":
    def clock_gettime(clock: c_int, out: ptr mut Timespec) -> c_int
    @blocking
    def read(fd: c_int, buf: ptr mut u8, count: c_size) -> c_ssize
```

- The comment is a `# safety:` line comment, continued on indented
  comment lines, immediately above the `unsafe:` line, the statement that
  contains `unsafe e`, the `unsafe impl` or the `extern` block. Its parts
  are named after the obligations it discharges (`valid, aligned:` above),
  which the audit checks (D-083).
- An `unsafe def` or `unsafe trait` documents its obligations in a
  `Safety:` list in its `##` comment, one named item each, in the way the
  Linux kernel's Rust guidelines require a `# Safety` section and a
  `// SAFETY:` comment for every block.
- `unsafe` combines with `pure` (`pure unsafe def`) and with the
  resource attributes (`@no_block`, `@no_alloc`), which keep their
  meaning.
- No other form exists: no `unsafe` module, file, field, attribute or
  closure, and no unsafe context that a function body inherits.

## Semantics

### The unsafe operations

| Operation | Why the compiler cannot check it |
|---|---|
| Calling an `unsafe def` (including the raw pointer methods, `RawBuf`'s slot access, `Allocator.release`, `mem.transmute`, `Slice.from_raw` and the machine intrinsics of `std.rt`) | the precondition is about memory the type system does not track |
| Calling a function declared in an `extern` block or produced by a C header import | the foreign code is not Sisu, and its signature is a claim |
| Writing an `unsafe impl` (`Send`, `Sync`, `Allocator`, any `unsafe trait`) | the promise is about every instance, which no single check covers |

Everything else is safe, including all of these: `ptr.of(ref x)`,
`ptr.of_mut(mut x)`, copying and comparing raw pointers, `p.cast[U]()`,
`p.with_addr(a)`, `mem.alloc_raw(layout)`, `mem.size_of[T]()`,
`mem.bit_cast` between plain types, `Cell.as_ptr()`, passing a raw
pointer to a safe function, and `@export`ing a function. A pointer that
nobody dereferences cannot hurt, which is the rule Rust always had and C#
15 adopted after 24 years.

### Raw pointers

| | Safe | Unsafe, with its obligations |
|---|---|---|
| Making one | `ptr.of(ref x) -> ptr[T]`, `ptr.of_mut(mut x) -> ptr mut T`, `xs.as_ptr()`, `ptr.dangling[T]()` (aligned, for empty buffers) | `ptr.from_address[T](a)` (`exposed`: `a` names memory or a device that foreign code or the hardware gave out) |
| Moving it | `p.cast[U]()`, `p.with_addr(a)` (keeps the provenance of `p`), the implicit `ptr mut T` to `ptr[T]` | `p.offset(n)` (`in_bounds`: stays within the allocation or one past it), `p.cast_mut()` (`writable`: `p` came from a `ptr mut`, not from a shared borrow) |
| Reading and writing | | `p.read() -> T` (`valid`, `aligned`; for a non-`Copy` `T` the value is moved out), `p.write(own v)` (`writable`, `aligned`; the old contents are not dropped), `p.drop_in_place()`, `p.copy_from(q, n)`, `p.read_volatile()`, `p.write_volatile(v)` |
| References | | `p.as_ref() -> ref T`, `p.as_mut() -> ref mut T` (`valid`, `aligned`, `valid_for_origin`) |
| Observing | `p.addr() -> int` and `p == q`, both impure: they are address-derived values, which `pure` code may not observe (D-074) | |

A reference made from a raw pointer is second-class like every other:
the checker gives it the origin its use asks for, the enclosing frame
when it is bound to a local, or the `from` parameter (or the elision rule
of [04](04-memory-and-ownership.md)) when it is returned. Its worst case
is therefore the frame or the caller's borrow, never the unbounded
lifetime that "becomes as big as context demands", which the Rustonomicon
traces mostly to references made from raw pointers. The obligation `valid_for_origin` is
that the memory stays valid, and the aliasing rule holds, for that whole
origin:

```sisu
struct Buffer:
    data: ptr mut u8                 # owned: allocated in `new`, released in Drop
    len: int

    def as_slice(self) -> Slice[u8]:  # a view tied to `self` by elision
        # safety: valid, valid_for_origin: data points to len initialised bytes that self
        #   owns and releases only in Drop, which cannot run while the view borrows self.
        return unsafe Slice.from_raw(self.data, self.len)
```

### Uninitialised memory, moves and drops

Locals are always initialised (D-051), so uninitialised memory exists
only behind raw pointers, almost always in a `RawBuf`. `p.write(v)`
initialises a slot without dropping what was there, `p.read()` moves the
value out and leaves the slot uninitialised, and `p.drop_in_place()` runs
the value's `Drop`. Reading an uninitialised `T`, or bits that are not a
valid `T` (a `bool` other than 0 or 1, a `char` that is not a scalar
value, an enum tag that names no variant), is undefined behaviour. There
is no `MaybeUninit` type and no `forget`: writing a value into memory
that is never read again is the one way to skip its drop, and the
contract forbids it for views, scopes, suspended frames and must-consume
values.

### Plain data and reinterpretation

`@derive(Plain)` marks a type whose every bit pattern is a valid value:
the compiler accepts it when the type is `Copy` and `@repr(c)` (or a
single-field wrapper), every field is plain, there is no padding, and the
type has no `invariant` and no refinement. Integers, floats and arrays of
plain types are plain; `bool`, `char`, enums, references and raw pointers
are not (a pointer rebuilt from bytes has no provenance). For plain types
the conversions are safe: `mem.bit_cast[U](x)` between types of one size
(checked at compile time), `x.as_bytes()`, `data.read_plain[T](at)`
(copies, any alignment) and `data.view_plain[T]()`, which is `None` when
the bytes are misaligned or short. Byte order is the target's; `std.encoding`
reads and writes a fixed one. Everything else is `mem.transmute[U](own x)`,
unsafe, sizes checked at compile time, with the obligation `valid_bits`.
This takes the commonest reason application code reaches for `unsafe`
(bytes in, structs out) out of unsafe code, as Go's `unsafe.String` and
Rust's zerocopy and bytemuck crates did; whether it ships in version 1 is
Q-71.

### Foreign calls

An `extern "C":` block or a C header import declares foreign functions,
and calling one is an unsafe operation. Their parameter and result types
are C-representable: the `std.ffi` types, `@repr(c)` structs, raw
pointers and their optional forms, and function pointers to `@export`ed
functions. A header import maps a C `T *` to `ptr mut T?` and a
`const T *` to `ptr[T]?`, and annotated non-null pointers to the
non-optional forms, as Swift imports C. A declaration that can block for
an unbounded time is `@blocking`, so the scheduler moves the call off the
task threads ([06](06-concurrency-and-capabilities.md)); a declaration
without it is a promise, in the `extern` block's safety comment, that the
call returns promptly.

Unwinding never crosses a foreign frame: a panic that reaches the boundary
of an `@export`ed function aborts the process, and since Sisu's unwinding
is ordinary control flow the backend inserts the check. An `@export`ed
function is safe to write because its signature may hold no reference,
view, `Shared`, closure or capability, so a foreign caller cannot hand
Sisu a dangling reference. A callback from foreign code runs on the thread
that made the foreign call and is `@no_block`, unless it is reached through
a `@blocking` call, whose thread it may then block.

### Machine intrinsics

Three operations of the runtime have no expression in Sisu or in C. They
are `unsafe def`s in `std.rt`, a module internal to `std`, with intrinsic
bodies that the backend lowers:

- `rt.switch(save: ptr mut Context, load: ptr[Context])` saves the
  running stack's registers into `save` and continues from those in
  `load`; while the backend is Cranelift, its `stack_switch` instruction
  lowers it on x86-64 Linux, the backend written in Sisu (D-087) emits the
  instructions as it emits any function, and the release backend writes
  them into the LLVM IR it produces (D-088);
- `rt.stack_pointer() -> int`, for the decommit watermark of the stack
  pool;
- `rt.thread_slot()`, a pointer to one word of thread-local storage, which
  holds a pointer to the thread's runtime value.

There is no inline assembly. Whether three are enough, for the runtime on
x86-64 and AArch64 and for a `freestanding` driver, is Q-54; whether
`std.rt` becomes public for `freestanding` programs is Q-57.

### Auto traits

`ptr[T]` and `ptr mut T` are `Copy`, `Escapable` exactly when `T` is,
and neither `Send` nor `Sync`. The first makes a buffer of views built on
raw pointers a view with no marker; the second is Rust's rule for Rust's
reason, so that a type holding a raw pointer is not made thread-safe by
inference. `RawBuf[T, A]` asserts `Send` when `T` and `A` are `Send` and
`Sync` when they are `Sync`, so a container built on it gets the right
auto traits structurally and needs no `unsafe impl` of its own, which
removes most of the manual impls in which Rudra found its third bug
pattern. `Send` and `Sync` can be asserted with `unsafe impl`; `Escapable`,
`Sized` and `Copy` never can, since D-058's guarantee rests on the first.

## The soundness contract

Safe code cannot cause undefined behaviour, whatever it does and whatever
it is given. An `unsafe` block is its author's promise that the
operations inside keep that true for every input, including whatever
safe code its callers pass in. In plain words, at the end of the block
and at every point inside it where a panic can start unwinding:

1. **Every value reachable from a live owner is a valid value of its
   type**: initialised, with valid bits, aligned, and owning what it owns
   exactly once.
2. **Every live reference points to a live, valid value for its whole
   origin**, and the aliasing rule holds as long as it lives: nothing
   writes behind a `ref` except through a `Cell` (or a type built on one,
   such as `Mutex` and `Atomic`), and nothing touches what a `ref mut`
   points to except through it.
3. **Every value is dropped exactly once or moved out first.** Views,
   scopes, suspended task frames and `@must_consume` values are never left
   in memory that is not read again (D-058, D-054); a buffer lent to the
   kernel or a device is released by it before the borrowing call returns.
4. **Tasks see only what they may.** Nothing crosses `spawn` that is not
   `Send` and nothing is shared that is not `Sync`; an `unsafe impl` of
   either is a promise about every instance. A raw pointer into a task's
   stack stays valid while the frame lives, also while the task is parked,
   because stacks never move; code that must not be interleaved with
   another task on its thread is `@no_block`; a foreign call that can block
   is `@blocking`.
5. **No authority without a capability.** A foreign call that affects the
   world is reachable only from code that holds the capability for that
   authority. A trusted wrapper never turns `open(2)` into a safe function
   that takes no `Fs`.
6. **Nothing observable depends on an address**, except alignment
   arithmetic on an aligned base, so `pure`, comptime, replay and the
   simulation profile stay deterministic. In a `pure` function, unsafe code
   writes only memory that the function allocated or holds through `own`
   or `mut`, and an impure operation in one of its unsafe blocks is a
   promise that the function is observationally pure.
7. **Soundness never rests on safe code the block does not control**: a
   generic parameter's trait implementation, a caller's closure, another
   package's contract. A wrong `Hash` or `Ord` may make a collection
   misbehave, never unsound; and since contracts and invariants can be
   compiled out ([07](07-contracts.md)), no unsafe block relies on one
   being checked. Unsafe code may rely on the invariants its own module
   keeps through privacy.

The undefined behaviours are therefore: reading uninitialised memory or
invalid bits; an access out of bounds, misaligned, or after release;
releasing twice or with another layout; breaking the aliasing rule; a data
race; calling foreign code through a wrong signature or against its
documented preconditions; breaking an `unsafe trait`'s promise; and
switching to a context that is not a suspended stack. All of them are
reachable only from unsafe code. The specification will list them, and
the interpreter checks every one it can observe (below). Which aliasing
rules exactly raw pointers follow is Q-56.

## Interaction with the rest of the language

- **The ownership checker** does not track raw pointers. A borrow that
  makes one (`ptr.of(ref x)`) ends at once, and the pointer may outlive
  it; a reference made from a raw pointer is tracked from the moment it is
  made, with the origin above.
- **Views, scopes and frames.** A raw pointer to a view is itself a view
  (`Escapable` follows `T`), so a collection of views built on raw memory
  cannot leave the frame of what it borrows. Unsafe code that stores a
  view in raw memory owes the drop of clause 3.
- **`Shared`.** The compiler still chooses atomic or plain counting by
  escape analysis (Q-2): it sets a flag in the cell it constructs, and
  std's implementation, ordinary unsafe code over a typed header with an
  `Atomic` count, takes the matching path, as `sisu-rt` reads its
  `SHARED_ATOMIC` flag today.
- **`pure`.** Purity stays declared and checked as today, a `pure`
  function calling only `pure` functions, and an `unsafe` block neither
  makes a function impure nor escapes the check (D-085). Std declares its
  raw memory operations `pure` (reading, writing and offsetting a pointer,
  `RawBuf`'s slots, `alloc_raw`), since clause 6 confines their writes to
  memory the function allocated or holds through `own` or `mut`. The
  impure operations are those that observe an address (`addr`, comparing
  pointers), make a pointer out of nothing (`ptr.from_address`,
  `rt.thread_slot`), mutate behind a shared reference (`Cell.set` and
  `Cell.as_ptr`, as `Cell.set` already is), the other machine intrinsics,
  and every foreign function. Inside a `pure def` an impure operation is
  accepted only within an `unsafe` block, as its author's assertion that
  the function stays observationally pure: the block's comment says why
  under a `pure:` part, and `sisu audit` lists the function. So a `pure`
  function can push to a `list` that std builds on `RawBuf`, and the
  allocator underneath (`Global` over `malloc`, a `Region` whose counter is
  a `Cell`) asserts its own purity once, in its own block. That is the
  trusted "observationally pure" declaration that N-63 asked for
  ([19](19-novel-features.md)), with no new form, and the text of
  [02](02-syntax.md) and [06](06-concurrency-and-capabilities.md) that a
  `pure` function "contains no `unsafe`" is amended to this rule.
- **Comptime.** Raw memory operations evaluate at compile time in the
  interpreter's checked model, and the interpreter supplies `Global`
  itself (an allocation in that model, where native code calls `malloc`),
  which is how std's collections, written over raw memory, keep working in
  `derive` and `const`; a foreign call, a machine intrinsic, an address
  observation, or a raw pointer in a constant's value stops compile-time
  evaluation with an error. Code that a
  comptime function generates may contain no `unsafe` at all, so every
  block stays in a source file where the audit can show it. This amends
  "no `unsafe`" in [08](08-metaprogramming.md).
- **The dynamic monitor.** Under `sisu test` (and `sisu run --monitor`)
  raw memory is a checked model, Miri's design on top of the existing
  monitor: an allocation is a block with an identity, a raw pointer is a
  block, an offset and the loan it was made from, and every access checks
  liveness, bounds, alignment and initialisation, validates the bits read
  for the type, and applies the loan rules the monitor already applies to
  references, so a write through a pointer made from a shared borrow, or an
  access that conflicts with a live `mut` loan, stops the run with a report
  like an ownership violation. The cost falls only on raw operations. For a
  function that contains unsafe code, `sisu test --panic-each` forces a
  panic at each potential panic site in turn and checks the drops that
  follow, which tests clause 1 under unwinding mechanically. How foreign
  calls run in the interpreter is Q-55.
- **The `sim` profile.** Raw memory operations cannot escape a
  simulation; foreign calls and the machine intrinsics can, and the `sim`
  runtime supplies its own scheduler. So a `sim` build accepts packages
  declared `unsafe = "memory"` and requires a simulation model for each
  reachable foreign call of an `extern` package, which is what stage 0's
  check already does ([17](17-testing.md)).
- **Contracts.** A contract expression may not contain an `unsafe` block
  or expression itself, though it may call safe functions that are built on
  one, and unsafe code may not rely on contracts (clause 7).
- **Hot replacement** does not swap a module that contains `unsafe`: its
  change restarts the program, since its blocks rely on module invariants
  that old and new code could disagree about (D-068).
- **The teaching profile** has no `unsafe` at any level (D-073).

## Packages: the manifest field

A package that contains any `unsafe` block, `unsafe def`, `unsafe impl`,
`extern` block or C header import says so in its own manifest:

```toml
[package]
name = "zlib"
version = "1.3.0"
unsafe = "extern"        # raw memory and foreign calls; "memory" for raw memory only
```

The root package consents in the `[trust]` list it already has:

```toml
[trust]
unsafe = ["zlib", "fasthash"]        # each may contain the unsafe code its manifest declares
```

- A package with unsafe code and no declaration does not compile; one
  whose declaration exceeds its code gets a warning to lower it. `memory`
  allows every unsafe operation except calls to foreign functions and the
  header import; `extern` allows those too.
- A dependency that declares `unsafe` compiles only if the root's `[trust]
  unsafe` lists it; a transitive dependency cannot grant itself the
  permission, as before. The root package and every workspace member
  declare their own level like any other package, so an application's own
  unsafe code is visible in its manifest too.
- The lockfile records each package's declared level. `sisu update`
  refuses an update that raises it (none to `memory`, `memory` to `extern`)
  unless given `--accept-unsafe`, which unattended runs never pass; the
  change is part of the authority delta that admission prints
  ([09](09-modules-packages-and-build.md), "Admission").
- The registry shows the level beside the capability summary, so a
  dependent sees it before installing; a package with no capability types
  in its API and no `unsafe` declaration keeps the *pure* label (D-066).
- `std` declares `extern` and is trusted by every program; `sisu audit
  --std` shows its blocks.

The level is not a security boundary. Code that can write raw memory can
be made to do anything by a malicious author, so `[trust]` consent covers
both levels and the distinction serves review: a `memory` package needs a
soundness review, an `extern` package a soundness and an authority review,
and the `sim` profile reads the level. Pony's `--safe`, Java's
`--enable-native-access` and Safe Haskell's package trust are the same
grant by the builder of the program; the declaration on the package's side
is what makes it visible before the grant.

## The audit and the lints

`sisu audit --unsafe` lists, per package and its declared level, every
module that contains unsafe code, and in each module every block,
`unsafe def`, `unsafe impl` and foreign declaration with its location, its
enclosing function and its safety comment; then the package's foreign
functions, each marked with the capability its callers hold or the
`pure` it asserts:

```text
$ sisu audit --unsafe
zlib 1.3.0   unsafe = "extern"   trusted in sisu.toml:14
  zlib (src/lib.sisu)   header import zlib.h, 1 block, module hash 7f3e...
    src/lib.sisu:3    import c "zlib.h"   # safety: zlib 1.3.1 headers, linked as -lz
    src/lib.sisu:12   crc32               # safety: valid: ...; pure: ...
  foreign calls: crc32 (asserted pure)
fasthash 2.0.1   unsafe = "memory"   trusted in sisu.toml:14
  fasthash.table (src/table.sisu)   4 blocks, 1 unsafe impl, module hash 0b91...
  ...
std (toolchain)   unsafe = "extern"   see sisu audit --unsafe --std
```

The module, not the block, is the unit of review: unsafe code that relies
on a field's invariant depends on every function of the module that can
write the field, which is the Rustonomicon's "pollutes a whole
_module_" and Modula-3's safe interface over an unsafe module. The module's hash is the
key of the review ledger (N-43), so a review carries over to the next
version when the module is unchanged and only the changed modules are
listed otherwise, as cargo-vet records audits as deltas.

| Lint | What it reports | Level |
|---|---|---|
| unused unsafe | a block with no unsafe operation; `sisu fix` removes it | warning |
| undocumented | a block, `unsafe def`, `unsafe trait`, `unsafe impl` or `extern` block without its comment or `Safety:` list | warning; an error at `sisu publish` |
| unnamed obligation | a block whose comment does not name an obligation of one of the unsafe defs it calls (Q-72) | warning |
| level | a declared level above what the package uses, or a declaration with no unsafe code | warning |
| auto-trait bound | an `unsafe impl Send` or `Sync` on a generic type whose parameters lack the matching bound (Rudra's check) | warning |
| ambient authority | a public safe function that reaches a foreign call without taking a capability type, outside `std.rt` | warning |

A use outside a block is an error that names the obligation:

```text
error[E0981]: `RawBuf.read` is an `unsafe def`: call it inside an `unsafe:` block
  --> src/ring.sisu:24:20
   |
24 |         let item = self.buf.read(self.head)
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^ its obligation: `initialised`
   = help: write the reason it holds in a `# safety:` comment above the block
```

## The standard library's layer

| Module | Contents |
|---|---|
| `std.mem` | `ptr[T]`, `ptr mut T`; `Layout`; `size_of`, `align_of`; `alloc_raw` (safe) and the `Allocator` trait (unsafe to implement), `Global`, the bump `Region` (D-060); `RawBuf[T, A]`; `Plain`, `bit_cast`, `transmute`; volatile access for `freestanding` targets |
| `std.ffi` | the C types (`c_int`, `c_char`, `c_size`, `c_ssize`, ...), `CStr` (a view of a NUL-terminated string) and `CString`, `@repr(c)` support, the header import and `extern` blocks |
| `std.rt` (internal to `std`) | the machine intrinsics; the stack pool; the scheduler beneath `std.task`; the C library bindings behind `System` (`std.rt.os`); the process entry |

This moves the raw pointer types from `std.ffi`, where
[11](11-stdlib.md) and [12](12-interop-and-targets.md) put them, to
`std.mem`, since the runtime uses raw pointers for memory with no foreign
code in sight. `RawBuf[T, A]` is the building block for every collection
that stores values in place: `with_capacity(n)`, `capacity()`, `grow(n)`,
and `slot(i) -> ptr mut T`, all safe and bounds-checked against the
capacity, plus `read(i)`, `write(i, v)`, `get(i) -> ref T` and
`drop_range(lo, hi)`, unsafe, whose only obligations are which slots hold
values. Spatial safety becomes a run-time check and the owner keeps only
the temporal bookkeeping, which is what Swift's `Span` and Java's memory
segments do for theirs. Std's `list`, `String`, `Deque`, `Heap`, the
storage of `dict` and `set`, and `SoaList` are built on it, and so is
anything a third-party package writes.

## Examples

### A bounded buffer

```sisu
from std.mem import RawBuf

error RingError:
    Full

## A first-in, first-out queue of at most `capacity` values, stored in place.
## Invariant (kept by every method below): the slots head, head+1, ...,
## head+count-1, taken modulo the capacity, hold values; the others do not.
pub struct Ring[T]:
    buf: RawBuf[T]
    head: int
    count: int

    pub def new(capacity: int) -> Ring[T]:
        requires capacity > 0
        return Ring(buf=RawBuf.with_capacity(capacity), head=0, count=0)

    pub def push(mut self, own item: T) -> ! RingError:
        if self.count == self.buf.capacity():
            return Err(Full)
        let i = (self.head + self.count) % self.buf.capacity()
        # safety: uninitialised: slot i is the first one past the occupied range, which is
        #   shorter than the capacity, so it holds no value.
        unsafe self.buf.write(i, item)
        self.count += 1               # nothing between the write and this line can panic
        return Ok(())

    pub def pop(mut self) -> T?:
        if self.count == 0:
            return None
        # safety: initialised: count > 0, so slot head holds a value; it is moved out, and
        #   the two lines after it take the slot out of the range before anything can panic.
        let item = unsafe self.buf.read(self.head)
        self.head = (self.head + 1) % self.buf.capacity()
        self.count -= 1
        return Some(item)

impl[T] Drop for Ring[T]:
    def drop(mut self):
        while let Some(item) = self.pop():
            pass                      # each item is dropped at the end of its iteration
        # the RawBuf's own Drop then releases the memory without touching any slot
```

`Ring[T]` is `Send` and `Sync` exactly when `T` is, through `RawBuf`, and
needs no `unsafe impl`. Its fields are private, so the invariant the
blocks rely on is kept by this module alone, which is the module the
audit lists. Under `sisu test` the interpreter checks each `read` against
the slot's initialisation, and `--panic-each` checks that a panic in
`T`'s `Drop` during `pop` leaves nothing dropped twice.

### A foreign call

A binding to zlib that needs no capability, because CRC-32 is
computation:

```sisu
# sisu.toml: [package] name = "zlib", unsafe = "extern"
# safety: zlib 1.3.1 headers, linked as -lz; the declarations are generated from them.
import c "zlib.h" as z with link="z"

## The CRC-32 of `data`, as zlib computes it.
pub pure def crc32(data: Slice[u8]) -> u32:
    var crc: z.uLong = 0
    for chunk in data.chunks(1 << 30):          # zlib takes a 32-bit length: never more
        # safety: valid: zlib reads len(chunk) bytes from the chunk's buffer, which `data`
        #   keeps alive and initialised for the call, and keeps no pointer after it;
        #   pure: crc32 depends on its arguments alone.
        crc = unsafe z.crc32(crc, Some(chunk.as_ptr()), len(chunk).truncate_to[z.uInt]())
    return crc.truncate_to[u32]()
```

A binding with authority looks different, because of clause 5. Here is
how `std.rt.os` gives `sys.clock` its time:

```sisu
from std.ffi import c_int

const CLOCK_REALTIME: c_int = 0

@repr(c)
struct Timespec:
    sec: i64
    nsec: i64

# safety: signatures as POSIX declares them, for 64-bit Linux.
extern "C":
    def clock_gettime(clock: c_int, out: ptr mut Timespec) -> c_int

## The object behind `sys.clock`. Only this module makes one, so only `main`'s `System` has it.
struct HostClock:
    pass

impl Clock for HostClock:
    def now(self) -> DateTime:
        var ts = Timespec(sec=0, nsec=0)
        # safety: valid: `out` points to the live local `ts`, which the call fills;
        #   authority: reading the clock is what `Clock` grants, and only a holder of a
        #   `Clock` reaches this method.
        let rc = unsafe clock_gettime(CLOCK_REALTIME, ptr.of_mut(mut ts))
        assert rc == 0, "clock_gettime cannot fail with a valid clock and pointer"
        return DateTime.from_unix_nanos(ts.sec * 1_000_000_000 + ts.nsec)
```

A user package that binds SQLite is in `HostClock`'s position, not
`crc32`'s: its `open` takes an `Fs` and routes SQLite's file access
through it (SQLite's VFS layer exists for this), or the ambient-authority
lint reports the public function that reaches `sqlite3_open` without one.

### An allocator

The bump `Region` of [04](04-memory-and-ownership.md), which makes
per-request memory safe, is a page of unsafe code:

```sisu
from std.mem import Allocator, Global, Layout, align_up

const REGION_ALIGN: int = 64

## Memory handed out in order and released all at once (04, "Allocators").
pub struct Region:
    base: ptr mut u8              # aligned to REGION_ALIGN
    size: int
    used: Cell[int]               # a Cell: `acquire` takes `self` shared

    pub def bump(capacity: int) -> Region:
        let layout = Layout(size=capacity, align=REGION_ALIGN)
        let base = Global.acquire(layout) else: panic("out of memory")
        return Region(base=base, size=capacity, used=Cell(0))

    ## Empties the region. Needs `mut`, which the checker grants only when no
    ## collection still uses the region.
    pub def reset(mut self):
        self.used.set(0)

# safety: valid_block: blocks are disjoint, since `used` only grows until `reset`, which needs
#   `mut self`; they are aligned, since `base` is aligned to REGION_ALIGN and every offset is
#   rounded up to `layout.align`, at most REGION_ALIGN, so the padding depends on `used` and
#   never on the address, and the region behaves the same in every run (clause 6).
#   stable: blocks stay valid until the region ends or is reset, and neither can happen while
#   a collection allocated in it is alive, because that collection is a view of the region.
unsafe impl Allocator for Region:
    pure def acquire(self, layout: Layout) -> ptr mut u8?:
        if layout.align > REGION_ALIGN:
            return None
        let start = align_up(self.used.get(), layout.align)
        if start + layout.size > self.size:
            return None
        # safety: in_bounds: start + layout.size <= size, so the block lies inside the region;
        #   pure: the counter changes only what later calls return, and that depends on the
        #   sizes asked for, never on an address, so the same calls give the same results.
        unsafe:
            self.used.set(start + layout.size)
            return Some(self.base.offset(start))

    pure unsafe def release(self, block: ptr mut u8, layout: Layout):
        pass                      # a region releases everything at once

impl Drop for Region:
    def drop(mut self):
        # safety: from_here, once: base came from Global.acquire with this layout, and the
        #   region ends here, after every collection that borrowed it.
        unsafe Global.release(self.base, Layout(size=self.size, align=REGION_ALIGN))
```

`Region` holds a `Cell` and a raw pointer, so it is neither `Sync` nor
`Send`, which is the rule [04](04-memory-and-ownership.md) states for
regions: a list in one cannot cross `spawn`. `acquire` is `pure` because
the trait declares it so, which is what lets `pure` code grow a
collection; the `Cell` update is the impure operation its block asserts,
and `Global.acquire`, a `pure def` over the C allocator, asserts the same
about `malloc`.

### A stack switch

The scheduler beneath `std.task` is safe Sisu, as the interpreter's is;
the one unsafe line is the switch:

```sisu
## The registers a suspended stack resumes from. Only `switch` and `Context.start` write one.
@repr(c)
struct Context:
    sp: int
    fp: int
    ip: int

## Saves the running stack's registers in `save` and continues from those in `load`.
## Returns when a later `switch` loads `save` again.
##
## Safety:
## - `valid_context`: `load` was saved by an earlier `switch`, or made by `Context.start`
##   for a live stack slot
## - `stable`: neither context moves while its stack is suspended
## - `no_yield`: the caller is `@no_block`, so no other switch happens in between
## - `no_live_borrow`: no reference that the other stacks could invalidate is live across it
@intrinsic("rt.switch")
unsafe def switch(save: ptr mut Context, load: ptr[Context])

## Gives the thread to task `next`, from the scheduler's loop or from a task that parks.
@no_block
def give_to(next: TaskId):
    let save, load = tasks.contexts(current(), next)    # raw pointers into stable task records
    set_current(next)
    # safety: valid_context: next's context was saved when it last gave the thread away,
    #   or made by Context.start when it was spawned; stable: task records live in a table
    #   whose slots never move while their task lives; no_yield: give_to is @no_block;
    #   no_live_borrow: `tasks`, `current` and `set_current` reach the scheduler through the
    #   thread slot and return no reference, so none is live across the switch.
    unsafe switch(save, load)
```

Everything above it (the ready list, timers, channels, `select`, mutexes,
scopes, cancellation, generators, the seeded schedules of
[17](17-testing.md)) is the Rust runtime's logic in safe Sisu, and its
critical sections are `@no_block`, so no compiler-inserted yield point
can interrupt them, as code on Go's system stack is non-preemptible.

## What is deliberately not allowed

- **Inline assembly.** Three machine intrinsics cover the runtime's needs
  (Q-54); assembly would make every target a second language in the
  toolchain.
- **Mutable globals**, even in unsafe code. The runtime's per-thread state
  hangs off one thread slot; everything else is a value passed down.
- **Integer-to-pointer casts.** `ptr.from_address` (unsafe, for devices
  and foreign handles) is the only way from an address to a pointer, and
  `p.with_addr(a)` keeps an existing pointer's provenance, so code stays
  correct on CHERI-like targets and analysable by the monitor.
- **An unsafe context a body inherits**, an unsafe module, file, closure
  or attribute, and unsafe fields (Q-72): the block is the only
  permission.
- **Unsafe code from comptime**, and raw pointers in constants.
- **Asserting `Escapable`, `Sized` or `Copy`.**
- **A safe `forget` or `leak`**, and leaking views, scopes, frames or
  must-consume values through raw memory.
- **Unwinding through foreign frames**: the export boundary aborts.
- **Unions.** A C union is imported as bytes with typed accessors.
- **A switch that turns checks off.** No profile removes bounds or
  overflow checks ([09](09-modules-packages-and-build.md)); code that must
  skip one calls an unchecked `unsafe def` in a block.
- **An `Unsafe` capability and run-time permission flags** (D-084):
  permission is a property of code, decided when the program is built.

## How the runtime in Sisu uses it

`sisu-rt` is 12,340 lines of Rust with 253 `unsafe` blocks, and its real
trusted base is larger: it calls Rust's standard library for allocation,
sockets, files, processes and thread-locals, and corosensei's inline
assembly for the switch. Sorted by what each block does, about 200 of the
253 exist because Rust code is reading Sisu values through raw addresses
by a type table ([`unsafe.md`](../research/topics/unsafe.md), "What Sisu's
runtime and backend need"). Written in Sisu and compiled with the program,
the runtime takes typed values instead, and that class disappears:

| Today (`sisu-rt`) | In Sisu | Unsafe left (estimate) |
|---|---|---|
| `abi.rs` (the type table), `entry.rs` and the other exported entry points (31 in all), `value.rs` and `dict.rs` (structural drop, copy, equality, order, hash, `Display`, `Debug`), `intrinsics.rs`, `iter.rs`, `bytes.rs`, `sized.rs`, `wide.rs`, `views.rs` | gone: generic functions instantiated per type (drop glue is SIR already), comptime reflection for the structural impls, intrinsics as ordinary std functions, `Slice[T]` over `RawBuf` | none |
| `mem.rs` | `std.mem`: `Global` over the C allocator, later an allocator in Sisu over `mmap`; `RawBuf`; `Box` | about 8 |
| `cells.rs`, `caps.rs` | `Shared` and `Weak` as typed cells with `Atomic` counts and the compiler's crossing flag; capability handles as counted cells; `Arena` over `list`, safe | about 6 |
| `stack.rs` | `std.rt`'s pool over `mmap`, `madvise` and `sysconf` | about 4 |
| `sched.rs`, `tasks.rs` | `std.rt`'s scheduler and `std.task` in safe Sisu over `switch` and the system stack | about 4 |
| `host.rs` and Rust's standard library | `std.rt.os`: one wrapper per C library function (files, sockets, processes, clock, random bits, standard streams, exit), behind the capability objects | about 25 |
| `rt.rs` (panics in flight, the output buffer), four exported statics | fields of the thread's runtime value, reached through the thread slot | about 2 |

About 50 blocks instead of 253, half of them in one module of foreign
bindings, all of them in Sisu. The backend needs none: a code generator,
a register allocator, an object and DWARF writer and a linker driver are
programs over byte buffers and `Proc`, and so is the interpreter when it
is rewritten in Sisu, its checked memory model included. The one later
exception would be an in-process JIT for native hot replacement (D-053),
which runs in the interpreter today.

The port is step 3 of the backend's order of work (D-087) and follows the
rule D-079 set for the front end (D-086). The Sisu runtime replaces
`sisu-rt` piece by piece, in this order: `std.mem` and `RawBuf` with the
buffers of `list` and `String`; `Box`, `Shared` and `Weak`; `std.rt.os`
for the standard streams, files and the clock; the stack pool and
`switch`, on x86-64 Linux first; the scheduler and `std.task`; sockets and
processes; the process entry. Each step must keep every native conformance
case's output identical, zero live allocations at exit
(`SISU_RT_ALLOCS`), the same schedule under every seed tried, and the
Phase 1 harness's memory and switch-cost results on the same machines. `sisu-rt` is retired when the
last piece passes, and Rust leaves the toolchain at the fixed point of
the whole toolchain that D-087 names. Generating copy, drop, equality and
hashing per type instead of walking a type table is also what lets an
optimiser inline the runtime's fast paths, which the backend research
found to be half of stage 1's instructions
([`backends.md`](../research/topics/backends.md)).

## What this costs, honestly

- **Three forms, not one.** The block is the only permission, but
  `unsafe def` and `unsafe impl` are declarations too. Without the first, a
  helper with a precondition is unsound or copied into every caller; the
  alternative is Modula-3's unsafe modules, which make the whole module
  the unit that permits.
- **Std's raw-memory layer becomes the toolchain's trusted base.**
  RustBelt found a soundness bug in Rust's `MutexGuard` and Rudra two in
  Rust's standard library; Sisu's will have bugs too. The monitor, the
  forced panics, the small size and the review ledger are the mitigations,
  not a proof.
- **The checked memory model is a large piece of work.** Miri took years,
  runs 3,000 to 7,000 times slower than unoptimised native code, and cannot
  run a fifth of crates because they call C. Sisu's model pays only on raw
  operations, but tests of unsafe-heavy code will be slow (Q-55).
- **No inline assembly** means every new machine need is a change to the
  backend, and a `freestanding` kernel may need more than three
  intrinsics (Q-54, Q-57).
- **The authority clause is a lint, not a proof.** A trusted package can
  still call `open(2)` from a function that takes `Fs` and ignore the
  restriction it carries; review is the defence, which is why the audit
  lists foreign calls apart.
- **A hard error from 0.1 is friction.** An application with many native
  dependencies lists each in `[trust]`, and a patch release that adds
  foreign calls waits for consent. That is the point, and the bet-2 port
  study should count how often it bites.
- **The port takes months, and the Rust runtime stays until it is done.**

## Comparison to the neighbours

| | Rust | Swift 6.2 | C# 15 | Go | Zig | D | Modula-3 | Sisu |
|---|---|---|---|---|---|---|---|---|
| What permits | `unsafe {}` block | nothing (strict mode warns; `unsafe` silences) | `unsafe` block or member | importing `unsafe` | nothing | `@trusted`, `@system` | an `UNSAFE` module | `unsafe:` block |
| Caller obligation | `unsafe fn` | `@unsafe` | requires-unsafe member | none | none | `@system` | an unsafe interface | `unsafe def` |
| Its body permits unsafe operations | no (2024 edition) | n/a | no | n/a | n/a | yes | yes | no |
| Creating a raw pointer | safe | safe | safe (15) | safe | safe | safe | unsafe module | safe |
| Foreign declarations | `unsafe extern` | unsafe by their types | `extern`, `safe` to opt out | cgo | ordinary | `@system` | unsafe interface | trusted package, unsafe to call |
| Grant per package | `[lints]` can forbid | none | `AllowUnsafeBlocks` | none | none | none | none | declared level, root `[trust]` |
| Dynamic checker | Miri (separate) | sanitizers | none | `checkptr` | safe build modes | none | none | the interpreter's monitor |
| Inline assembly | `asm!` | no | no | assembly files | `asm` | `asm` | no | no (intrinsics) |

Sisu's design is Rust's, with the 2024 edition's corrections from the
start, C# 15's point that permission belongs to operations, Modula-3's
module as the unit of review, Pony's and Java's per-package grant, and
Miri inside the interpreter that already exists. What no neighbour has is
the contract's three Sisu clauses: no leaked views, no authority without a
capability, and no observable addresses.
