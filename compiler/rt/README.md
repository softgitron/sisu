# The runtime in Sisu

The runtime of natively compiled Sisu is being written in Sisu, piece by
piece, as D-086 orders ([15](../../proposal/15-decision-log.md)); this
package holds what is written so far, and `crates/sisu-rt`, the runtime
in Rust, the rest. `docs/architecture.md` ("Stage 2", "The runtime in
Sisu"; decisions 335, 336, 350 to 353, 358, 382 to 385, 422, 423, 464, 469, 506, 542, 562, 625, 645, 646, 680, 681, 724, 780, 784, 820 and 821) describes it, and
`docs/port-debt.md` ("The runtime in Sisu") lists what is left.

## What it is

Each `export_NAME` function of the root module (`src/lib.sisu`) is the
runtime's symbol `sisu_rt_NAME`, of Sisu's convention (the result's
address, then each argument's, and the status byte): the generated code
and the runtime in Rust call it as any function is called. What the
exports call is compiled with them under local symbols. The code is the
runtime's own: it counts no frame, passes no yield point and adds no line
to a stack trace, as drop glue does not; it reaches raw memory through the
operations of std.mem the backend compiles inline, and the runtime in Rust
and C's `memmove` and `memcmp` through foreign calls (`unsafe =
"extern"`).

| Module | What |
|---|---|
| `lib.sisu` | the exports, `==` of two texts' bytes, and the text entries interpolation calls most (their bodies here, a call fewer) |
| `dispatch.sisu` | the intrinsics of lists, dicts, sets and slices, a list's and a set's iterator and the lookups by a `str` (decision 784) among them, and the thirteen that make an iterator or take every item of one (`range`, `zip`, `sorted`, `any`, `iter.ref` and their kin, decision 820), decoded from their descriptors and done here (decisions 625 and 646): the runtime in Rust reads the intrinsic's number alone and hands the call over; a panic, a drop and a list's view are asked of it; and `in` as an operator, a table's literal, the generated code's inline lookups (decision 646) and `d{k}`'s slot (decision 681), which the runtime in Rust's entries hand over |
| `queue.sisu` | the scheduler's queues: the run queue, the waiters under their keys and the timers, records of one to three words in a list's layout, whose blocks the runtime in Rust gives and keeps (decision 645) |
| `channel.sisu` | the channels' buffers: the values sent and not received, in the order they were sent, in a ring of slots of the value's size that grows when it is full, whose blocks the runtime in Rust gives and keeps (decision 680) |
| `tasks.sisu` | the tasks' records of the scheduler, three words a record in a queue: a task's state, what it waits for and whether it is a generator; a task parked under its keys in the waiters' queue, and the tasks a key or a cancellation wakes put in the run queue in the interpreter's order, the coroutine and the rest of each task the runtime in Rust's under the same number (decision 780) |
| `choose.sisu` | the scheduler's other two strategies: PCT's change points, its priorities by task drawn when first asked, a yield point at which a task gives way or is preempted and the next task by priority; the exhaustive search's choices from its prefix, recorded in its trail, and the next task among the ready ones in creation order, each over queues the runtime in Rust keeps and the generator of `draw.sisu` (decision 821) |
| `draw.sisu` | the scheduler's seeded generator, the interpreter's SplitMix64 with its seeding, a number below a bound, and the choice of the next task under the default strategy, taken out of the run queue as `queue.sisu` takes it, the generator's word the runtime in Rust's (decision 724) |
| `types.sisu` | the program's type table, as the runtime in Rust lays it out in memory when the program starts (`crates/sisu-rt/src/desc.rs`) |
| `host.sisu` | the runtime in Rust's functions this code calls, and C's |
| `value.sisu` | equality, order, the structural hash (a `Shared` followed to its value, decision 464), `.copy()`, the drop and the count of a copy's handles (a function value's environment in a heap cell copied whole, decision 382), each a walk of a value by its type's parts |
| `text.sisu` | a `String`'s growth, a `String` made from a text, bytes, a byte and an integer's digits appended, an integer's digits in a base |
| `search.sisu` | text search by Two-Way, and a prefix or a suffix |
| `floats.sisu` | a float's text: the shortest that reads back as the same number, the fixed form and the form with an exponent, on plain integers or big numbers |
| `show.sisu` | `Display` and `Debug` of a value by its type, with the names the table holds (decision 383), and a text quoted as `Debug` writes it, which the runtime in Rust asks for (decision 469) |
| `time.sisu` | a time's text: RFC 3339, as a `DateTime`'s `Debug` writes it, and `fmt.strftime`'s directives (decision 469) |
| `list.sisu` | lists over raw memory: growth, the moves of `insert`, `remove`, `pop`, `extend`, `reverse` and `swap`, a slice's copies, the stable sort, `in` and `join` (decision 385); `dispatch.sisu` checks the indices of `insert`, `remove` and `swap`, whose panics the runtime in Rust raises (decisions 562 and 625) |
| `table.sisu` | the tables of dicts and sets: entries in insertion order, each with its key's hash, and an index of slots probed linearly; a lookup, an entry appended and one taken out, and a table copied, compared and dropped (decision 422); the header's layout, which the runtime in Rust reads only through the exports (decision 506) |
| `iter.sisu` | iterators: the object compiled code steps in part (decision 253), the next item of each kind, the iterator of an `Iterable[T]` argument, every item left, the drop (decision 423); a new iterator of each of the eleven kinds and what one may wait for, the layout the runtime in Rust no longer mirrors (decision 542), six of which (a range, a borrowed list's, table's or receiver's items, an enumeration and a zip) only `dispatch.sisu` makes (decision 820) |

The runtime's code has no text of its own to point to, so the words
`Debug` writes are constants of their bytes packed eight to a word
(`show.sisu`), and the names of the program's types come from the table.

The functions are safe `def`s that say what they require of their
arguments (`Requires:`), and their `unsafe` blocks name the obligations of
std.mem's operations and of the foreign functions they discharge: the
frozen Rust front end, whose formatter checks this package, reads no
`unsafe def`. The exports are private, as the runtime's own; the other
modules' functions are `pub(package)`. The walks read the type table's
words and the values' own in place rather than through helpers, since a
call of Sisu's convention costs a probe and its arguments' memory.

## How it is built

`compiler/target/rt/rt.o`, which every native program is linked with
before the runtime in Rust, is compiled from the package's seed,
`rt.sir`, the SIR stage 1 prints of it, by the Rust lowering in the
release profile and Cranelift (`sisu_codegen::compile_library`), when it
is missing or older than the seed: the runtime must exist before stage 1,
a native program itself, can compile it. A test holds the seed to what
`sisu sir` prints here. After a change to `src/`:

```text
cd compiler/rt
../target/release/sisu check          # the package's diagnostics, which should be none
../target/release/sisu sir > rt.sir   # the seed again
```

Stage 2 compiles the same seed with the backend written in Sisu (`sisu2
build rt.sir --library --output rt.o`), and its fixed point
(`compiler/stage2/bootstrap.sh`) links the builds it compares with the
objects so made, so that the runtime in Sisu is in the fixed point too.
`SISU_RT=PATH` makes a native build link with the object at `PATH` as it
is.
