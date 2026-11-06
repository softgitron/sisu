# 27. Systems programming

Every chapter so far has used code that the compiler checks. Some code
it cannot check: code that works with raw memory, or calls a function
written in C. In this chapter you write that code inside `unsafe`, say
why it is right, and see the interpreter check it as it runs. You build
a safe type on raw memory, lay data out as C does, call C, keep the
authority of a foreign call behind a capability, and declare the unsafe
code of a package. Last, you see what is designed for allocators,
runtime profiles and targets. You need about two hours.

## Code the compiler cannot check

A raw pointer, `ptr[T]` or `ptr mut T` from `std.mem`, points to a
value without the rules that keep a reference safe. Making one is safe;
reading or writing through it is an unsafe operation, which only an
`unsafe:` block or an `unsafe` expression may hold:

<!-- file: examples/27-systems/pointer.sisu -->
```sisu
# A raw pointer is safe to make; reading and writing through one is not.
from std.mem import ptr

def main():
    var celsius = 80
    let p = ptr.of_mut(mut celsius)
    # safety: valid, aligned, writable: p points to the live local
    #   celsius, which nothing else uses until the block ends.
    unsafe:
        p.write(p.read() + 5)
    print(celsius)
```

<!-- output: examples/27-systems/pointer.out -->
```text
85
```

- `ptr.of_mut(mut celsius)` makes a pointer through which `celsius`
  can be read and written. The borrow that makes it ends at once, and
  the ownership checker does not follow the pointer.
- `p.read()` and `p.write(...)` are `unsafe def`s of `std.mem`: each has
  obligations, things its caller must make true, such as `valid` (the
  pointer points to a live value) and `aligned`.
- The `# safety:` comment above the block says why each obligation
  holds, naming them. The block is where you, not the compiler, answer
  for the code.

There are three unsafe operations: calling an `unsafe def`, calling a
function written in C, and writing an `unsafe impl`. Everything else,
including making, copying and passing raw pointers, is safe.

**Try it.** Remove the `unsafe:` line, and dedent the line under it.
Then put it back, and read through `p` again after the `print`, in an
`unsafe` expression: `let now = unsafe p.read()`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/27-systems/pointer_bare.console -->
```console
$ sisu run pointer_bare.sisu
error[E0981]: `(ptr mut T).write` is an `unsafe def`: call it inside an `unsafe:` block or an `unsafe` expression
  --> pointer_bare.sisu:9:5
  |
9 |     p.write(p.read() + 5)
  |     ^^^^^^^^^^^^^^^^^^^^^
  |     --------------------- its obligations: `writable`, `aligned`
  = note: write why they hold in a `# safety:` comment above the block, naming each obligation it discharges

error[E0981]: `(ptr mut T).read` is an `unsafe def`: call it inside an `unsafe:` block or an `unsafe` expression
  --> pointer_bare.sisu:9:13
  |
9 |     p.write(p.read() + 5)
  |             ^^^^^^^^
  |             -------- its obligations: `valid`, `aligned`
  = note: write why they hold in a `# safety:` comment above the block, naming each obligation it discharges

error[D0006]: the program was not run: errors found (2)
```

E0981 (`unsafe.call`) names each `unsafe def` called outside a block,
and its label lists the obligations the call takes on.

<!-- output: examples/27-systems/pointer_after.console -->
```console
$ sisu run pointer_after.sisu
85
85
$ sisu run --monitor pointer_after.sisu
85
ownership violation: a reference was used after its loan had ended
  the loan: a `mut` borrow of `celsius` in `main`, created at pointer_after.sisu:6:13 by `_2 = call intrinsic mem.ptr.of_mut(copy _3)`
  ended by: a use of `celsius` at pointer_after.sisu:11:5 (`_7 = call intrinsic print(copy _1)`)
  used at: pointer_after.sisu:13:22 (`_8 = call intrinsic mem.ptr.read(copy _2)`)
  value: `celsius` = 85
  loans of `celsius` not ended yet: 0 shared, 0 with `mut`
  at main (pointer_after.sisu:13:22)
SIR of `main`, its blocks that the conflict passes through:
fn main(0) source 0 @4:5-4:9 {
  bb0:
    _1 = const 80 @5:19-5:21
    _3 = ref mut _1 @6:28-6:35
    _2 = call intrinsic mem.ptr.of_mut(copy _3) @6:13-6:36
    _6 = call intrinsic mem.ptr.read(copy _2) @10:17-10:25
    _5 = add copy _6, const 5 @10:17-10:29
    _4 = call intrinsic mem.ptr.write(copy _2, copy _5) @10:9-10:30
    _7 = call intrinsic print(copy _1) @11:5-11:19
    _8 = call intrinsic mem.ptr.read(copy _2) @13:22-13:30
    _9 = call intrinsic print(copy _8) @14:5-14:15
    _0 = const ()
    return
}
```

The plain run prints 85 twice, and nothing seems wrong. Under the
monitor, which checks raw memory as it checks borrows, the second read
is an ownership violation: `print(celsius)` used `celsius` directly,
which ended the borrow that the pointer was made from, so the pointer
may no longer be used. The report ends with the part of the SIR of
`main`, the compiler's form of the code, that the conflict passes
through. The comment's reason, that `celsius` is
alive, was not enough. Run code that holds `unsafe` under the monitor.

</details>

A pointer can also outlive the local it points to. `reading` returns a
pointer to its own `celsius`, which ends with the call:

<!-- file: examples/27-systems/pointer_gone.sisu -->
```sisu
# A pointer that outlives the local it points to.
from std.mem import ptr

def reading() -> ptr[int]:
    let celsius = 80
    return ptr.of(ref celsius)

def main():
    let p = reading()
    # safety: valid: wrong, `celsius` went with the frame of `reading`.
    print(unsafe p.read())
```

<!-- output: examples/27-systems/pointer_gone.console -->
```console
$ sisu run pointer_gone.sisu
panic: `ptr.read` through a pointer to a value that no longer exists (`valid`)
  at main (pointer_gone.sisu:11:18)
$ sisu run --native pointer_gone.sisu
panic: `ptr.read` through a pointer to a value that no longer exists (`valid`)
  at main (pointer_gone.sisu:11:18)
```

Both the interpreter and a native debug build, `sisu run --native`,
stop at the read: the obligation `valid` does not hold, since the
frame of `reading` has returned. A native build checks it by address,
so it says less than the interpreter: a pointer into a frame that has
returned, whose stack memory a later call of the same depth now uses,
looks alive to it.

## Saying why it holds

An `unsafe def` of your own lists its obligations in a `Safety:` list
of its `##` comment, and each caller's `# safety:` comment names the
ones it discharges:

<!-- file: examples/27-systems/obligations.sisu -->
```sisu
# An `unsafe def` names what its callers must make true.
from std.mem import as_ptr

## The reading at `i` of `temps`, without the check that there is one.
##
## Safety:
## - `in_range`: `i` is at least 0 and less than `len(temps)`
unsafe def reading_at(temps: Slice[int], i: int) -> int:
    let first = as_ptr(temps)
    # safety: in_bounds, valid, aligned: this function's `in_range`, and
    #   `temps` lives, unchanged, until the read.
    return unsafe first.offset(i).read()

def main():
    let temps = [78, 85, 92]
    # safety: in_range: 2 is less than the three readings.
    print(unsafe reading_at(temps, 2))
```

<!-- output: examples/27-systems/obligations.out -->
```text
92
```

- `reading_at` skips the check that `i` is in range, which its callers
  must make instead: its obligation is `in_range`.
- Its body is ordinary code, in which the read still needs a block of
  its own: an `unsafe def` obliges its callers and permits nothing.
- `as_ptr(temps)` gives a pointer to the first value of the slice, and
  `offset(i)` moves it `i` values on, which is unsafe since the result
  must stay inside the slice (`in_bounds`).

**Try it.** Write the comment of the call as `# safety: the list was
made on the line above.`

<details>
<summary>What Sisu says</summary>

<!-- output: examples/27-systems/obligations_unnamed.console -->
```console
$ sisu run obligations_unnamed.sisu
warning[W0010]: the `# safety:` comment names no obligation of `reading_at`, which are `in_range`
  --> obligations_unnamed.sisu:16:5
   |
16 |     # safety: the list was made on the line above.
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

92
```

W0010 (`unsafe.unnamed-obligation`) warns that the comment names none
of the obligations of `reading_at`, so a reader cannot tell which it
answers. The program still runs. Other warnings of the same kind keep
the comments honest: W0007 for an unsafe block with no comment, W0008
for an `unsafe def` without its `Safety:` list, and W0009 for an
`unsafe` around code that holds no unsafe operation.

</details>

## Raw memory

`Global().acquire(layout)` hands out a block of raw memory of the size
and alignment that a `Layout` gives, or `None` when there is none, and
`Global().release(block, layout)` gives it back. What the block holds
is the program's to manage:

<!-- file: examples/27-systems/block.sisu -->
```sisu
# Raw memory from `Global`, holding values of a type the program chooses.
from std.mem import Global, Layout

def main():
    let layout = Layout.array[int](3) ?? Layout(size=0, align=8)
    let block = Global().acquire(layout) else:
        print("out of memory")
        return
    let temps = block.cast[int]()
    # safety: writable, aligned, in_bounds: the block holds three ints and
    #   nothing yet; valid: each is written before it is read; from_here,
    #   once: the block came from Global with this layout, released once.
    unsafe:
        for i in range(3):
            temps.offset(i).write(70 + 10 * i)
        print(temps.offset(2).read())
        Global().release(block, layout)
```

<!-- output: examples/27-systems/block.out -->
```text
90
```

`Layout.array[int](3)` is the layout of three `int`s, and
`block.cast[int]()` makes the block's pointer a pointer to `int`s. The
block holds nothing until the loop writes three values, and after
`release` nothing may read it.

The interpreter keeps a model of raw memory: each block, its size, what
each byte holds, and whether it is still alive. Every read, write and
offset is checked against it, so a mistake in unsafe code stops the
program where it happens instead of damaging memory. A native build in
the debug profile keeps the same model for the blocks that `Global`
and C's `malloc` give out and for pointers to the program's own
variables, and only the release profile runs unchecked.

**Try it.** Write four values in place of three. Then put it back, and
move the `print` after the `release`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/27-systems/block_past.console -->
```console
$ sisu run block_past.sisu
panic: `ptr.write` of 8 bytes at offset 24 of a block of 24 bytes reaches outside it (`writable`)
  at main (block_past.sisu:15:13)
```

<!-- output: examples/27-systems/block_released.console -->
```console
$ sisu run block_released.sisu
panic: `ptr.read` of memory that was released (`valid`)
  at main (block_released.sisu:17:15)
$ sisu run --native block_released.sisu
panic: `ptr.read` of memory that was released (`valid`)
  at main (block_released.sisu:17:15)
$ sisu run --native --release block_released.sisu
90
```

Each panic names the operation, the obligation it broke and where.
A native debug build, `sisu run --native`, checks its raw memory against
the same model and stops with the same panic. The release build,
`--release`, leaves the checks out: the read after the release printed
what the memory still happened to hold, which nothing promises. That is
the undefined behaviour that unsafe code must never reach, and the
reason to run it in the interpreter or a debug build while you write
it, and to ship the release build only once its tests pass.

</details>

## A safe type over raw memory

Most unsafe code lives inside a type whose methods are safe to call.
`RawBuf[T]` of `std.mem` is a buffer of slots for values of `T`, with
the capacity checked on every access; which slots hold a value is the
owner's to know. A stack keeps that knowledge in its invariant:

<!-- file: examples/27-systems/stack.sisu -->
```sisu
# A safe stack on `RawBuf`: unsafe code inside, a safe type outside.
from std.mem import RawBuf

## A last-in, first-out stack. Invariant, kept by every method: slots 0
## to count-1 hold values, and the others none.
struct Stack[T]:
    buf: RawBuf[T]
    count: int

impl[T] Stack[T]:
    def new() -> Stack[T]:
        return Stack(buf=RawBuf.with_capacity(2), count=0)

    def push(mut self, own value: T):
        if self.count == self.buf.capacity():
            self.buf.grow(2 * self.buf.capacity())
        # safety: uninitialised: slot count holds no value, by the invariant.
        unsafe self.buf.write(self.count, value)
        self.count += 1

    def pop(mut self) -> T?:
        if self.count == 0:
            return None
        self.count -= 1
        # safety: initialised: slot count held a value, which moves out.
        return Some(unsafe self.buf.read(self.count))

impl[T] Drop for Stack[T]:
    def drop(mut self):
        # safety: initialised: slots 0 to count-1 hold values, dropped once.
        unsafe self.buf.drop_range(0, self.count)

def main():
    var words: Stack[String] = Stack.new()
    for w in ["kiuas", "löyly", "vihta"]:
        words.push(w.to_string())
    print(words.pop())
    print("{words.count} left")
```

<!-- output: examples/27-systems/stack.out -->
```text
Some("vihta")
2 left
```

- `write` and `read` of a slot are unsafe, with the obligations
  `uninitialised` and `initialised`: a slot must be empty to be written
  and full to be read. The invariant in the struct's comment is what
  each `# safety:` comment relies on.
- `grow` moves the values to a larger buffer, and is safe.
- Callers see `push` and `pop`, which are safe, and they cannot break
  the invariant: only the type's own methods touch `buf` and `count`.
- The `Drop` drops the values still in the stack. `RawBuf` gives its
  memory back when it is dropped, but it does not know which slots hold
  values.

**Try it.** Remove the `impl Drop` of `Stack`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/27-systems/stack_no_drop.console -->
```console
$ sisu run stack_no_drop.sisu
Some("vihta")
2 left
panic: `Global.release` of memory that still holds a value of type `String` at offset 0, which was never dropped: every value is dropped once (22, the soundness contract, clause 3)
  at main (stack_no_drop.sisu:33:32)
```

The two strings left in the stack would never be dropped, and the model
stops the release of memory that still holds a value. Every value is
dropped exactly once: that is one of the promises an unsafe block
keeps, which [22](../../proposal/22-unsafe.md#the-soundness-contract)
lists as its soundness contract.

</details>

## Data laid out as C lays it out

`@repr(c)` lays a struct out as C does: its fields in order, each at an
offset that is a multiple of its alignment, with padding where needed.
A struct whose every bit pattern is a valid value, `@derive(Plain)`,
can be read from bytes with no unsafe code at all:

<!-- file: examples/27-systems/plain.sisu -->
```sisu
# `@repr(c)` lays a struct out as C does; plain data comes from bytes.
from std.mem import Plain, align_of, read_plain, size_of

## A reading as a sensor writes it: the minute, the temperature in tenths
## of a degree, the sensor's number and its flags, with no padding.
@repr(c)
@derive(Plain, Copy, Debug)
struct Reading:
    minute: u32
    tenths: i16
    sensor: u8
    flags: u8

## A byte and a word, which C pads to sixteen bytes.
@repr(c)
struct Padded:
    flag: u8
    count: int

def main():
    print("{size_of[Reading]()} bytes, aligned to {align_of[Reading]()}")
    print("{size_of[Padded]()} bytes, aligned to {align_of[Padded]()}")
    let data = b"\x2d\x00\x00\x00\x1e\x03\x02\x00"
    print("{read_plain[Reading](data, 0):?}")
```

<!-- output: examples/27-systems/plain.out -->
```text
8 bytes, aligned to 4
16 bytes, aligned to 8
Reading(minute=45, tenths=798, sensor=2, flags=0)
```

- `Reading` has a `u32`, an `i16` and two `u8`s, eight bytes with no
  padding. `Padded` has a byte and then an `int`, which must start at a
  multiple of 8, so seven bytes of padding follow the byte.
- `read_plain[Reading](data, 0)` copies eight bytes into a `Reading`, in
  the byte order of the machine, the least significant byte first here:
  `2d 00 00 00` is the minute 45, and `1e 03` the temperature, 79.8
  degrees.
- `bit_cast` turns a plain value into another of the same size, and
  `as_bytes` gives a plain value's bytes. Both are safe. `transmute`, for
  types that are not plain, is unsafe.

A plain struct has no padding, since `as_bytes` would read bytes that
no field owns, so `@derive(Plain)` refuses a struct whose layout has a
gap: E0279 (`resolve.plain-padding`) between two fields, and E0280
(`resolve.plain-padding-end`) at the end:

<!-- file: examples/27-systems/plain_gap.sisu -->
```sisu
PLACEHOLDER
```

<!-- output: examples/27-systems/plain_gap.console -->
```console
PLACEHOLDER
```

Each names the bytes and their offsets, from the first byte of the gap
to the first after it. Order the fields from the widest, as `Reading`
does, or give the bytes fields of their own; the note's `Array` is not
`Copy`, which `Plain` needs, so write them as `u8` and `u16` fields
([R17](../reference/diagnostics.md#e0280)).

> **From Python.** This is `struct.unpack` or `ctypes.Structure`, with
> the layout checked by the compiler: a `@derive(Plain)` that does not
> hold, such as one on a struct with a `bool` field or with padding,
> does not compile, and neither does one on a struct of several fields
> without `@repr(c)`, whose layout would be the compiler's to choose
> (E0608). Where `ctypes` pads a structure without a word, Sisu asks you
> to say what the bytes are.

## Calling C

An `extern "C":` block declares functions written in C, with the types
of `std.ffi` for C's: `c_int`, `c_size`, `c_char` and the rest. Their
signatures are claims the compiler cannot check, so each call is an
unsafe operation:

<!-- file: examples/27-systems/foreign.sisu -->
```sisu
# Calling C: an `extern "C"` block declares its functions.
from std.ffi import c_char, c_int, c_size, c_ssize
from std.mem import Global, Layout, ptr

# safety: the signatures as <string.h> and <unistd.h> declare them for
#   64-bit Linux; each returns promptly.
extern "C":
    def strlen(s: ptr[c_char]) -> c_size
    def write(fd: c_int, buf: ptr[u8], n: c_size) -> c_ssize

def main():
    let layout = Layout(size=16, align=1)
    let block = Global().acquire(layout) else:
        print("out of memory")
        return
    # safety: writable, in_bounds, aligned: the block has room for the
    #   eight bytes and the NUL; valid: strlen and write read only bytes
    #   written first; from_here, once: the block is released once.
    unsafe:
        var at = 0
        for b in "kiitos!\n".bytes():
            block.offset(at).write(b)
            at += 1
        block.offset(at).write(0u8)
        let n = strlen(block.cast[c_char]().as_const())
        let written = write(1, block.as_const(), n)
        print("{n} bytes, {written} written")
        Global().release(block, layout)
```

<!-- output: examples/27-systems/foreign.out -->
```text
kiitos!
8 bytes, 8 written
```

The program writes `kiitos!`, Finnish for "thank you!", and a line
break into a block, with a NUL byte after them as C expects, measures it
with `strlen` and writes it to standard output with `write`. `sisu run
--native` calls the C library itself. The interpreter cannot, and runs
the six C functions it knows instead, `strlen`, `memcpy`, `malloc`,
`free`, `getenv` and `write`, on its model of memory, where the same
checks apply. A native debug build calls C's own functions and keeps
the same model of what they give out: `malloc`'s block is checked as a
block of `Global` is, `getenv`'s text may be read and not written, and
a `strlen` of text without its NUL stops the program.

- `@blocking` on a declaration says that the function can block its
  thread for a long time, such as a read from a socket, so that the
  scheduler moves the call off the threads that run tasks. On anything
  but a foreign function it is E0990 (`unsafe.blocking-not-foreign`).
- `@export("NAME")` makes a Sisu function callable from C: a native
  build gives it the global symbol `NAME`, whether or not the program
  calls it. Its signature may hold no
  reference, view, `Shared`, closure or capability, which C could not
  keep alive (E0989, `unsafe.export-signature`), and each of its types
  must be one C has a layout for, a number, `bool`, `char`, a raw
  pointer or a `@repr(c)` type (E0991, `unsafe.export-not-c`). C calls
  it with its own convention, `int32_t sauna_heat(int32_t minutes)`
  for a function of an `i32`, and passes a `@repr(c)` struct by a
  pointer to it, whose fields a native build lays out at C's offsets,
  as `@repr(c)` asks. A panic cannot unwind into C, so one that
  leaves the function prints its report and ends the program with status 134, as
  [R14](../reference/attributes.md#exported-functions) shows.
- Importing a C header, `import c "zlib.h"`, which would write the
  `extern` block from the header, is designed in
  [22](../../proposal/22-unsafe.md#foreign-calls) and not in the
  compiler yet (E0900).

**Try it.** Remove the `# safety:` comment above `extern "C":`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/27-systems/foreign_unexplained.console -->
```console
$ sisu run foreign_unexplained.sisu
warning[W0007]: this `extern` block has no `# safety:` comment above it saying why each obligation it takes on holds
  --> foreign_unexplained.sisu:5:1
  |
5 | extern "C":
  | ^^^^^^^^^^

kiitos!
8 bytes, 8 written
```

W0007 (`unsafe.no-safety-comment`) asks for the comment, which for an
`extern` block says where the signatures come from and whether each
call returns promptly.

</details>

## No authority without a capability

A C function can do anything the operating system allows: read the
environment, open files, connect to a network. Chapter 18's rule still
holds, so a safe function that reaches such a call takes the capability
for what the call does, and its callers can see the authority they
give:

<!-- file: examples/27-systems/authority.sisu -->
```sisu
# A safe function that reaches the world through C takes a capability.
from std.ffi import CString, c_char
from std.mem import ptr

# safety: the signature as <stdlib.h> declares it; it returns promptly.
extern "C":
    def getenv(name: ptr[c_char]) -> ptr[c_char]?

## Whether the environment variable `name` is set.
pub def is_set(name: str) -> bool:
    let c = CString.new(name) else:
        return False
    # safety: valid: `c` lives across the call and ends with its NUL, and
    #   getenv keeps no pointer to it.
    return unsafe getenv(c.as_ptr()).is_some()
```

<!-- output: examples/27-systems/authority.console -->
```console
$ sisu check authority.sisu
warning[W0014]: `is_set` is public and safe and reaches the foreign function `getenv` without taking a capability, so its callers need no authority for what the call does
  --> authority.sisu:10:9
   |
10 | pub def is_set(name: str) -> bool:
   |         ^^^^^^
  = note: take the capability whose authority the call exercises (22, clause 5), as `std.rt.os` does behind `System`

```

W0014 (`unsafe.ambient-authority`) says that `is_set` reads the
environment without an `Env`, so any code could call it and learn what
the environment holds. `CString.new(name)` makes the text with the NUL
that C needs, and `c.as_ptr()` its pointer, which points to the
`CString`'s own bytes and stays valid while it lives. C's `strlen`
counts them up to the NUL:

<!-- file: examples/27-systems/cstring.sisu -->
```sisu
# A `CString` hands C a pointer to its own bytes, ended by a NUL.
from std.ffi import CString, c_char, c_size
from std.mem import ptr

# safety: the signature as <string.h> declares it; it returns promptly.
extern "C":
    def strlen(s: ptr[c_char]) -> c_size

def main():
    let name = CString.new("löyly") else:
        return
    # safety: valid: `name` lives across the call and ends with its NUL,
    #   and strlen keeps no pointer to it.
    print(unsafe strlen(name.as_ptr()))
    print(name.byte_len())
```

<!-- output: examples/27-systems/cstring.console -->
```console
$ sisu run cstring.sisu
6
6
$ sisu run --native cstring.sisu
6
6
```

`löyly`, the steam of a sauna, is five letters and six bytes, since
`ö` takes two. The interpreter and a native build print the same: a
`u8` takes one byte in memory, a `u16` two and a `u32` four, as C's
types do, so the bytes of a `CString` lie one after another and
`as_ptr` points C at them in place.

**Try it.** Give `is_set` a first parameter `env: Env`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/27-systems/authority_env.console -->
```console
$ sisu check authority_env.sisu
```

`sisu check` prints nothing: the function's signature now says that it
uses the environment, and only code that holds an `Env` can call it.
The parameter is never read; it is the permission, as every capability
is. In the standard library, only `std.rt` turns foreign calls into the
capabilities of `System`.

</details>

## Packages that contain unsafe code

A package that holds unsafe code says so in its manifest:
`unsafe = "memory"` for raw memory, or `unsafe = "extern"` when it also
calls foreign code:

<!-- file: examples/27-systems/levels.pkg/sisu.toml -->
```toml
[package]
name = "sauna"
version = "0.1.0"
unsafe = "memory"
```

<!-- file: examples/27-systems/levels.pkg/src/main.sisu -->
```sisu
# A package with unsafe code says so in its manifest.
from std.mem import ptr

def main():
    var celsius = 80
    let p = ptr.of_mut(mut celsius)
    # safety: valid, aligned, writable: p points to the live local
    #   celsius, which nothing else uses until the block ends.
    unsafe:
        p.write(p.read() + 5)
    print(celsius)
```

<!-- output: examples/27-systems/levels.console -->
```console
$ sisu run
85
$ sisu audit --unsafe
sauna 0.1.0   unsafe = "memory"   the root package
  sauna (src/main.sisu)   1 block, module hash 7374184f049a918b
    src/main.sisu:9   unsafe block in main   # safety: valid, aligned, writable: p points to the live local celsius, which nothing else uses until the bloc...
std (toolchain)   unsafe = "extern"   see `sisu audit --unsafe --std`
```

`sisu audit --unsafe` lists the unsafe code a reviewer reads first: for
each package its level, and for each module its unsafe blocks and
functions, each with the start of its `# safety:` reason. The standard
library is listed by its level only, and `--std` audits it.

The lockfile records the level, so that an update of a dependency that
raises it, from none to `memory` or from `memory` to `extern`, is
refused unless you accept it with `--accept-unsafe`:

<!-- output: examples/27-systems/levels.lock -->
```text
# Written by `sisu` from the manifests: the packages this build resolved.
# It is data; `sisu` rewrites it when a manifest changes.
version = 1

[[package]]
name = "sauna"
version = "0.1.0"
source = "path+."
dependencies = []
unsafe = "memory"
```

A dependency that declares a level compiles only when the root
package's manifest consents, `unsafe = ["NAME"]` under `[trust]`,
E1021 (`package.unsafe-untrusted`) otherwise. A transitive dependency
cannot consent for itself: the program's author decides, for every
package in the build.

**Try it.** Remove the line `unsafe = "memory"` from `sisu.toml`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/27-systems/levels_undeclared.console -->
```console
$ sisu run
error[E1019]: the package `sauna` contains unsafe code, and its `sisu.toml` declares no `unsafe`: add `unsafe = "memory"` to its `[package]`
  --> src/main.sisu:9:5
  |
9 |     unsafe:
  |     ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

E1019 (`package.unsafe-undeclared`) points at the first unsafe code and
names the level to declare. The declaration is how a reader of the
manifest, or a reviewer of a dependency, learns that the package needs
a closer look.

</details>

## Testing unsafe code

Tests of unsafe code run as any tests do, on the same checked memory
model as `sisu run`, which stops a test at the first obligation it
breaks:

<!-- file: examples/27-systems/tested.sisu -->
```sisu
# A test of unsafe code runs on the checked memory model.
from std.mem import ptr

## `celsius` raised by `by` degrees, through a raw pointer.
def raised(celsius: int, by: int) -> int:
    var value = celsius
    let p = ptr.of_mut(mut value)
    # safety: valid, aligned, writable: p points to the live local
    #   value, which nothing else uses until the block ends.
    unsafe:
        p.write(p.read() + by)
    return value

test "five degrees more":
    assert raised(80, 5) == 85
```

<!-- output: examples/27-systems/tested.console -->
```console
$ sisu test tested.sisu
test "five degrees more" ... ok
1 passed, 0 failed, 0 blocked
```

A package's tests of unsafe code run in the same way, with `sisu test`
in its directory, as the wrapper package of
[G19](../guides/c-library.md#4-test-it-and-run-it-in-the-interpreter-and-natively)
shows. The interpreter emulates a few C functions, `strlen`, `memcpy`,
`malloc`, `free`, `getenv` and `write`, so a test reaches those through
`extern "C"` too; a test that calls another C function stops at the
call with a panic, since only a native build makes it.

## What is still to come

Much of systems programming in Sisu is designed and not in the
compiler yet. Allocators for the
collections, `list.with_allocator(Region.bump(n))` for memory released
all at once and `try_push` for allocation that can fail, are designed in
[04](../../proposal/04-memory-and-ownership.md#allocators) (E0351 and
E0304 today); `Region` exists, under `RawBuf`. The resource attributes
`@no_alloc`, `@no_block` and `@stack` compile with the warning W0030
(`resolve.unchecked-attribute`), since nothing checks them yet. The
manifest's runtime profile is read, but `single` gets the runtime that
`full` gets and `freestanding` is refused (E1023); the profiles'
own runtimes, targets and cross compilation (`--target`), and
`panic = "abort"` in a profile, which the compiler ignores with W0005,
wait for the runtime of [12](../../proposal/12-interop-and-targets.md)
([R22](../reference/targets.md#runtime-profiles)). A native executable
leaves out the code that nothing reaches, but links the C library
dynamically and keeps its debug information: a ten-line program is
6.25 MB, 1.74 MB stripped, nearly all of it the runtime, as `sisu build
--native --explain-size` shows part by part
([R22](../reference/targets.md#binary-size-and-startup)). A native debug build
checks raw memory as the interpreter does, the memory C gives out
included, except where the interpreter alone knows enough: which
borrow a pointer came from, which it checks only by address. The
exercises of this
chapter practise the safe side of systems work: layouts, bytes and
code that does not allocate.

## Exercise 27.1: predict the layouts

Open
[`manual/exercises/27-systems/predict_layout.sisu`](../exercises/27-systems/predict_layout.sisu):

<!-- file: exercises/27-systems/predict_layout.sisu -->
```sisu
##! Exercise 27.1, predict the layouts. Kind: predict. Level: 1.
##!
##! `c_layout` works out the size of a struct laid out as C lays it out,
##! from the size and the alignment of each of its fields, as `@repr(c)`
##! does. Each constant is your prediction of the size it gives. Write
##! each as a literal, then run `sisu test predict_layout.sisu`. The hints
##! are in chapter 27 of the tutorial, under "Exercise 27.1: predict the
##! layouts".

## The size and the alignment of a C struct whose fields have the sizes
## and alignments of `fields`, in order: each field starts at the next
## multiple of its alignment, and the size is rounded up to a multiple of
## the largest alignment.
def c_layout(fields: list[(int, int)]) -> (size: int, align: int):
    var offset = 0
    var align = 1
    for size, field_align in fields:
        offset = (offset + field_align - 1) // field_align * field_align
        offset += size
        align = max(align, field_align)
    return ((offset + align - 1) // align * align, align)

const BYTE_WORD: int = todo("a u8, then an int")
const WORD_BYTE: int = todo("an int, then a u8")
const SMALL_FIRST: int = todo("u8, u8, u16, u32")
const BYTE_AROUND: int = todo("u8, u32, u8")
const SHORT_BYTE: int = todo("i16, u8")

test "a byte before a word":
    assert c_layout([(1, 1), (8, 8)]).size == BYTE_WORD

test "a byte after a word":
    assert c_layout([(8, 8), (1, 1)]).size == WORD_BYTE

test "the small fields first":
    assert c_layout([(1, 1), (1, 1), (2, 2), (4, 4)]).size == SMALL_FIRST

test "a word between two bytes":
    assert c_layout([(1, 1), (4, 4), (1, 1)]).size == BYTE_AROUND

test "a short and a byte":
    assert c_layout([(2, 2), (1, 1)]).size == SHORT_BYTE
```

<!-- output: solutions/27-systems/predict_layout.console -->
```console
$ sisu test predict_layout.sisu
warning[W0001]: typed hole `a u8, then an int` of type `int`
  --> predict_layout.sisu:23:24
   |
23 | const BYTE_WORD: int = todo("a u8, then an int")
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `an int, then a u8` of type `int`
  --> predict_layout.sisu:24:24
   |
24 | const WORD_BYTE: int = todo("an int, then a u8")
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `u8, u8, u16, u32` of type `int`
  --> predict_layout.sisu:25:26
   |
25 | const SMALL_FIRST: int = todo("u8, u8, u16, u32")
   |                          ^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `u8, u32, u8` of type `int`
  --> predict_layout.sisu:26:26
   |
26 | const BYTE_AROUND: int = todo("u8, u32, u8")
   |                          ^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `i16, u8` of type `int`
  --> predict_layout.sisu:27:25
   |
27 | const SHORT_BYTE: int = todo("i16, u8")
   |                         ^^^^^^^^^^^^^^^

test "a byte before a word" ... blocked by todo("a u8, then an int")
test "a byte after a word" ... blocked by todo("an int, then a u8")
test "the small fields first" ... blocked by todo("u8, u8, u16, u32")
test "a word between two bytes" ... blocked by todo("u8, u32, u8")
test "a short and a byte" ... blocked by todo("i16, u8")
0 passed, 0 failed, 5 blocked
```

<details>
<summary>Hint 1</summary>

Place the fields one by one. A field starts at the first offset that is
a multiple of its alignment, at or after the end of the field before.

</details>

<details>
<summary>Hint 2</summary>

The size is the end of the last field, rounded up to a multiple of the
largest alignment, so that a list of the structs keeps every field
aligned. That is why a field at the end can add padding too.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/27-systems/predict_layout.sisu`](../solutions/27-systems/predict_layout.sisu). Each
layout with padding is one that `@derive(Plain)` refuses, with E0279 or
E0280.

## Exercise 27.2: the order of the bytes

Open
[`manual/exercises/27-systems/byte_order.sisu`](../exercises/27-systems/byte_order.sisu):

<!-- file: exercises/27-systems/byte_order.sisu -->
```sisu
##! Exercise 27.2, the order of the bytes. Kind: fix. Level: 1.
##!
##! A sensor writes each number of its records as four bytes, the least
##! significant byte first, and `u32_at` reads one back. Two tests fail.
##! Run `sisu test byte_order.sisu`, read the values in the reports, and
##! fix `u32_at`, not the tests, so that every test passes. The hints are
##! in chapter 27 of the tutorial, under "Exercise 27.2: the order of the
##! bytes".

## The number in the four bytes of `data` from `at`, least significant
## byte first.
def u32_at(data: bytes, at: int) -> u32:
    var n: u32 = 0
    for i in range(4):
        n = (n << 8) | data[at + i]
    return n

test "zero":
    assert u32_at(b"\x00\x00\x00\x00", 0) == 0

test "a minute count":
    assert u32_at(b"\x2d\x00\x00\x00", 0) == 45

test "four different bytes, not at the start":
    assert u32_at(b"\xff\x01\x02\x03\x04", 1) == 0x04030201
```

<!-- output: solutions/27-systems/byte_order.console -->
```console
$ sisu test byte_order.sisu
test "zero" ... ok
test "a minute count" ... FAILED
test "four different bytes, not at the start" ... FAILED

test "a minute count" ... FAILED
panic: assertion failed: u32_at(b"\x2d\x00\x00\x00", 0) == 45
  left:  754974720
  right: 45
  at test "a minute count" (byte_order.sisu:22:5)
replay: sisu test --replay 1-bdb5ec1159a21bd8-0.0.1-t1-s0-r0 byte_order.sisu

test "four different bytes, not at the start" ... FAILED
panic: assertion failed: u32_at(b"\xff\x01\x02\x03\x04", 1) == 0x04030201
  left:  16909060
  right: 67305985
  at test "four different bytes, not at the start" (byte_order.sisu:25:5)
replay: sisu test --replay 1-bdb5ec1159a21bd8-0.0.1-t2-s0-r0 byte_order.sisu

1 passed, 2 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

Write the numbers of the report in hexadecimal: `"{n:x}"` writes 754974720
as `2d000000`. The bytes are all there, in the wrong places.

</details>

<details>
<summary>Hint 2</summary>

The loop shifts what it has read up by a byte before it adds the next
byte, so the byte it reads first ends up the most significant. Read the
bytes in the other order.

</details>

When every test passes, compare your answer with
[`manual/solutions/27-systems/byte_order.sisu`](../solutions/27-systems/byte_order.sisu).

## Exercise 27.3: an audio callback without allocation

Open
[`manual/exercises/27-systems/mixer.sisu`](../exercises/27-systems/mixer.sisu):

<!-- file: exercises/27-systems/mixer.sisu -->
```sisu
##! Exercise 27.3, an audio callback without allocation. Kind: write. Level: 2.
##!
##! A sound card asks for the next samples of audio many times a second,
##! and a callback that allocates may miss its deadline. `mix` adds a
##! voice into the samples of `out`, in place, each scaled by `gain` and
##! kept between -1.0 and 1.0, and `fade` lowers the samples of `out`
##! steadily from their full level at the first to silence after the last.
##! Write both bodies without making any list, string or other value that
##! allocates, so that every test passes. Run the tests with
##! `sisu test mixer.sisu`. The hints are in chapter 27 of the tutorial,
##! under "Exercise 27.3: an audio callback without allocation".

## Adds `voice`, scaled by `gain`, into `out`, sample by sample, keeping
## each sample between -1.0 and 1.0.
@no_alloc
def mix[const N: int](mut out: Array[f32, N], voice: Array[f32, N], gain: f32):
    todo("mix")

## Scales the sample at `i` of `out` by `(N - i) / N`.
@no_alloc
def fade[const N: int](mut out: Array[f32, N]):
    todo("fade")

## Four samples, for the tests.
def frames(xs: list[f32]) -> Array[f32, 4]:
    var out: Array[f32, 4] = Array.filled(0.0f32)
    for i in range(4):
        out[i] = xs[i]
    return out

test "mixing a voice in":
    var out = frames([0.5, 0.25, -0.5, 0.0])
    mix(mut out, frames([0.5, 0.5, -0.5, 0.25]), 0.5)
    assert out[0] == 0.75 and out[1] == 0.5
    assert out[2] == -0.75 and out[3] == 0.125

test "mixing keeps the samples in range":
    var out = frames([0.75, -0.75, 0.5, -0.5])
    mix(mut out, frames([1.0, -1.0, 1.0, -1.0]), 1.0)
    assert out[0] == 1.0 and out[1] == -1.0
    assert out[2] == 1.0 and out[3] == -1.0

test "a fade to silence":
    var out = frames([1.0, 1.0, -1.0, 0.5])
    fade(mut out)
    assert out[0] == 1.0 and out[1] == 0.75
    assert out[2] == -0.5 and out[3] == 0.125
```

<!-- output: solutions/27-systems/mixer.console -->
```console
$ sisu test mixer.sisu
warning[W0030]: `@no_alloc` is accepted and checked by nothing yet, so nothing ensures that the function does not allocate
  --> mixer.sisu:15:2
   |
15 | @no_alloc
   |  ^^^^^^^^

warning[W0001]: typed hole `mix` of type `()`
  --> mixer.sisu:17:5
   |
17 |     todo("mix")
   |     ^^^^^^^^^^^

warning[W0030]: `@no_alloc` is accepted and checked by nothing yet, so nothing ensures that the function does not allocate
  --> mixer.sisu:20:2
   |
20 | @no_alloc
   |  ^^^^^^^^

warning[W0001]: typed hole `fade` of type `()`
  --> mixer.sisu:22:5
   |
22 |     todo("fade")
   |     ^^^^^^^^^^^^

test "mixing a voice in" ... blocked by todo("mix")
test "mixing keeps the samples in range" ... blocked by todo("mix")
test "a fade to silence" ... blocked by todo("fade")

test "mixing a voice in" ... blocked by todo("mix")
what the test printed:
  in scope at the typed hole `todo("mix")`:
    gain = 0.5

test "mixing keeps the samples in range" ... blocked by todo("mix")
what the test printed:
  in scope at the typed hole `todo("mix")`:
    gain = 1.0

0 passed, 0 failed, 3 blocked
```

`@no_alloc` states that the functions allocate nothing. W0030
(`resolve.unchecked-attribute`) at each says that nothing checks the
promise yet, so today it is yours to keep; the warning goes once the
compiler checks it.

<details>
<summary>Hint 1</summary>

An `Array` has its size in its type and changes in place, so a loop over
its indices, `range(N)`, reads and writes every sample without making
anything new.

</details>

<details>
<summary>Hint 2</summary>

An `f32` is not `Ord`, so `min` and `max` do not take one: keep a sample
in range with an `if` expression. For `fade`, make `N - i` and `N` into
`f32`s with `to_f32()`, a method that a constant parameter has as any
`int` does.

</details>

When every test passes, compare your answer with
[`manual/solutions/27-systems/mixer.sisu`](../solutions/27-systems/mixer.sisu).

## Exercise 27.4: records from a device's bytes

Open
[`manual/exercises/27-systems/records.sisu`](../exercises/27-systems/records.sisu):

<!-- file: exercises/27-systems/records.sisu -->
```sisu
##! Exercise 27.4, records from a device's bytes. Kind: write. Level: 3.
##!
##! A sauna's sensor writes its log as records of eight bytes: the minute
##! as four bytes and the temperature in tenths of a degree as two, both
##! least significant byte first, then the sensor's number and a byte of
##! flags. Write `parse_log`, which reads every record of a log, so that
##! every test passes. A log that ends inside a record gives `Truncated`
##! with the place where that record starts. Run the tests with
##! `sisu test records.sisu`. The hints are in chapter 27 of the tutorial,
##! under "Exercise 27.4: records from a device's bytes".

error LogError:
    Truncated(at: int)

## One record of the log.
@derive(Eq, Debug)
struct Reading:
    minute: u32
    tenths: i16
    sensor: u8
    flags: u8

## The records of `data`, in order.
def parse_log(data: bytes) -> list[Reading] ! LogError:
    return todo("parse_log")

test "two records":
    let start = b"\x2d\x00\x00\x00\x1e\x03\x02\x00"
    let log = start + b"\x2e\x00\x00\x00\x24\x03\x02\x01"
    let first = Reading(minute=45, tenths=798, sensor=2, flags=0)
    let second = Reading(minute=46, tenths=804, sensor=2, flags=1)
    assert parse_log(log) == Ok([first, second])

test "a temperature below zero":
    let log = b"\x00\x01\x00\x00\xe7\xff\x01\x00"
    let cold = Reading(minute=256, tenths=-25, sensor=1, flags=0)
    assert parse_log(log) == Ok([cold])

test "an empty log":
    assert parse_log(b"") == Ok([])

test "a log cut short":
    let log = b"\x2d\x00\x00\x00\x1e\x03\x02\x00\x2e\x00"
    assert parse_log(log) == Err(Truncated(at=8))
```

<!-- output: solutions/27-systems/records.console -->
```console
$ sisu test records.sisu
warning[W0001]: typed hole `parse_log` of type `list[Reading] ! LogError`
  --> records.sisu:25:12
   |
25 |     return todo("parse_log")
   |            ^^^^^^^^^^^^^^^^^

test "two records" ... blocked by todo("parse_log")
test "a temperature below zero" ... blocked by todo("parse_log")
test "an empty log" ... blocked by todo("parse_log")
test "a log cut short" ... blocked by todo("parse_log")

test "two records" ... blocked by todo("parse_log")
what the test printed:
  in scope at the typed hole `todo("parse_log")`:
    data = b"-\x00\x00\x00\x1e\x03\x02\x00.\x00\x00\x00$\x03\x02\x01"

test "a temperature below zero" ... blocked by todo("parse_log")
what the test printed:
  in scope at the typed hole `todo("parse_log")`:
    data = b"\x00\x01\x00\x00\xe7\xff\x01\x00"

test "an empty log" ... blocked by todo("parse_log")
what the test printed:
  in scope at the typed hole `todo("parse_log")`:
    data = b""

test "a log cut short" ... blocked by todo("parse_log")
what the test printed:
  in scope at the typed hole `todo("parse_log")`:
    data = b"-\x00\x00\x00\x1e\x03\x02\x00.\x00"

0 passed, 0 failed, 4 blocked
```

<details>
<summary>Hint 1</summary>

This is `read_plain` of the section "Data laid out as C lays it out",
written by hand. A function that reads a number of `width` bytes, least
significant first, serves both the minute and the temperature.

</details>

<details>
<summary>Hint 2</summary>

Read the two bytes of the temperature as a number from 0 to 65535 and
keep its lowest 16 bits as an `i16`, which makes 65511 into -25: chapter
24's `truncate_to` does that.

</details>

<details>
<summary>Hint 3</summary>

Walk the log eight bytes at a time, and check before each record that
all eight are there; the place of the record that is cut short is the
one `Truncated` names.

</details>

When every test passes, compare your answer with
[`manual/solutions/27-systems/records.sisu`](../solutions/27-systems/records.sisu).
`Reading`'s fields leave no padding, so the section's `@derive(Plain)`
takes them; with `sensor` first, C's layout would leave a gap, and the
derive would be E0279.

## What you learned

- The three unsafe operations, a call of an `unsafe def`, a call of a
  foreign function and an `unsafe impl`, need an `unsafe:` block or an
  `unsafe` expression (E0981, E0982), with a `# safety:` comment that
  names the obligations it discharges (W0007, W0010).
- Raw pointers and raw memory are `std.mem`'s: `ptr.of_mut`, `read`,
  `write`, `offset`, `Global`, `Layout` and `RawBuf`. The interpreter
  and a native debug build check every access against a model of
  memory, the release build does not, and the monitor checks the
  borrows behind the pointers.
- A safe type keeps its unsafe code behind methods that keep an
  invariant, and drops what it holds.
- `@repr(c)` lays a struct out as C does, and `@derive(Plain)` with
  `read_plain` reads one from bytes safely; a plain struct has no
  padding (E0279, E0280).
- `extern "C":` declares C functions, which the interpreter emulates for
  six of them; a safe function that reaches the world through one takes
  a capability (W0014).
- A package declares `unsafe = "memory"` or `"extern"` (E1019), the
  root consents with `[trust]` (E1021), and the lockfile records the
  level.

Chapter 28 is about code that lasts: editions and the compatibility
promise, deprecation and migrations that travel with a library, the
semantic versions of a published package, and planning work with typed
holes.
