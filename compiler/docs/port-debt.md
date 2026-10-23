# Port debt

Stage 0 is written in Rust and borrows its runtime from the host
([18](../../proposal/18-next-steps.md), "Implementation language and
architecture"). Every place where a result depends on Rust rather than
on a rule written down, and every shortcut that a later stage must not
copy, is listed here, so that the stage-1 front end in Sisu and the native
backend can be transcriptions of documented behaviour
([architecture.md](architecture.md)) instead of reverse engineering.

Each entry says where the debt is, why it is debt, and what paying it
means. The last part lists what Phases 0, 2, 3, 4, 6 and 8 ask for that
stage 0 does not have yet.

## Behaviour that comes from the host

**Unicode tables.** `char.is_alphabetic`, `is_whitespace`,
`is_uppercase`, `is_lowercase`, `str.to_lowercase` and `to_uppercase`
use the tables of the Rust standard library
(`sisu-interp/src/intrinsics.rs`), which follow whatever Unicode version
that Rust release ships; `split_whitespace` and the `trim` methods are
Sisu, over `char.is_whitespace`. The case mappings of a whole text stay
on the host because a final sigma lowers by its context (`SpecialCasing`),
which needs the `Cased` and `Case_Ignorable` properties that Rust does
not expose. *Pay*: name the Unicode version in the std contracts and pin
it in stage 1's tables (with `Cased` and `Case_Ignorable`, so the case
mappings can be Sisu over `char` mappings); add conformance cases at the
edges (a character added in a recent version, Turkish dotted `İ`, a
final `Σ`).

**Float to text.** `Display` of an `f64` or an `f32` is Rust's shortest
round-trip form for its type and `{x:.N}` is Rust's correctly rounded
fixed form (the intrinsics `fmt.float_digits` and `fmt.float32_digits`,
`sisu-interp/src/format.rs`; the rest of a format spec is Sisu). Any correct shortest-round-trip algorithm gives the same
digits, but the exponent spelling (`1e100`, `1.5e-7`) and the rounding of
fixed forms are Rust's. *Pay*: specify both in std's contract and
implement them in Sisu (Ryu-style shortest digits, round half to even on
the exact binary value).

**Text to float.** Float literals are converted by Rust's `str::parse`
in the AST builder (`f64`) and the checker (`f32`, rounded once from the
text), which is correctly rounded. *Pay*: stage 1 needs a correctly
rounded parser for both widths; add literals at rounding boundaries to
the suite.

**Float powers.** `x ** y` on floats is Rust's `powf` (an `f32` power is
the `f64` one, rounded), which is the platform's maths library and not
correctly rounded: the one float operation whose bits may differ between
targets, against D-074. `+`, `-`, `*`, `/`, the square root and
`mul_add` are exactly specified by IEEE 754, and `%` is exact up to its
one addition. *Pay*: a correctly rounded `pow`, with `std.math`'s
functions (Q-23).

**Hashing.** `dict` and `set` keep an insertion-ordered vector with a Rust
`HashMap` as the index (`sisu-interp/src/value.rs`), over the host's
structural key of a value, not the `Hash` and `Eq` impls: a key type with
impls of its own is rejected (`E0398`) instead. Nothing observable
depends on the hash, since iteration is in insertion order (D-074). The
native runtime's tables hash the same structural key a word at a time
with a multiplicative mix (decision 253), keyed by nothing, so neither
implementation resists hash flooding yet. *Pay*: write `dict` and `set`
in Sisu over `Hash` and `Eq`, with the per-process SipHash key of D-049
as std's, and lift `E0398`.

**Text to a float at run time.** `str.parse_float` (which `cli.parse`
uses for an `f64` option) and `str.parse_f32` are Rust's `parse`, after
a check that the text is decimal syntax (no `inf` or `nan`);
`str.parse_int` is Sisu. *Pay*: a Sisu float parser, correctly rounded,
as for literals.

**Civil time.** `DateTime` is nanoseconds since the epoch in UTC, in an
`i64` (the years 1678 to 2261); its RFC 3339 parser is Sisu, its
`Display` and `strftime` (a fixed subset) are the host's (`format.rs`).
*Pay*: the time module of `std` replaces them; time zones are its
problem.

## Shortcuts in the interpreter

**Structural operations recurse.** The host's structural printing,
comparison, hashing and `.copy()` recurse over a value on the host
stack, which is why the run uses a 256 MiB stack. *Pay*: none for the
interpreter; std written in Sisu over the traits replaces them where the
fast paths stop paying.

**Drops inside intrinsics are untyped.** A drop statement gives the drop
glue its type arguments from the place's type, but a value an intrinsic
drops (`list.clear`, `Cell.set`, `dict.insert` replacing a value) is
dropped by what it is, so the glue of a generic type runs there without
type arguments, which only a `Drop` calling through a trait on them
could notice. *Pay*: give those intrinsics their element types (or write
them in Sisu, where the drop is a statement).

**The host-backed types' drops are interpreter code.** A `list`, `dict`,
`set`, tuple, option, `Shared`, `Weak`, `Cell`, `Mutex`, guard and
`Arena` are taken apart by the interpreter (`sisu-interp/src/drops.rs`),
in the orders their contracts give. *Pay*: when they are written in Sisu
over raw storage (stage 1 and the backend), their glue is SIR like any
declared type's.

**`str` is a copy.** `strview` copies the text into a shared immutable
string, so a view costs O(length), and so does a piece of a view
(`byte_slice`, and the pieces `split` and the others make). Semantics are the same because the
viewed `String` cannot change while the view exists, which the ownership
checker enforces, and which the monitor checks through the loan each
view carries. *Pay*: a view is a pointer and a length in the backend,
as it is in the native backend's first round (architecture.md, "Stage
2").

**The commonest intrinsics have a second path.** Outside the monitor the
run loop calls about twenty intrinsics directly (architecture.md,
"Speed"), each a copy of part of its general implementation that must
give the same value and effects or decline before any effect. A change
to one of them is a change in two places; the conformance suite runs
every program case under the monitor too (the general path) and compares
the outputs, which is what holds the two together. *Pay*: none for a
backend; for the interpreter, intrinsics resolved once to a function
that both paths call.

**References are paths.** A reference is a frame, a local and a path of
steps walked on every access, with a serial number to detect a reference
into a returned frame. *Pay*: none for the interpreter (it is also the
dynamic monitor's hook); a backend uses addresses.

**The monitor is interpreter code.** Loans, tags and the loan tree of
the dynamic ownership monitor live in `sisu-interp/src/monitor.rs`, and
reading a moved value or a dangling reference without the monitor still
panics (`R0033`, `R0034`). *Pay*: none for the interpreter; the native
backend has no monitor and relies on the checker, so the conformance
suite's monitor rerun and the mutation fuzzing must keep running against
the interpreter for as long as the checker changes.

**The monitor's scan of held views is bounded.** Passing a reference to
an intrinsic or an operator checks the views and references held by the
value behind it, and passing a value to a function checks those the value
itself holds, but only the first 256 values (`VIEW_SCAN_BUDGET`).
`set.add` dropping a duplicate must-consume value, the key of
`dict.insert`, and `Path.as_str` (a view without a loan) are not
observed. *Pay*: tag every view-holding value with the loans it holds
(as the static dataflow does), so a use checks a set instead of scanning
a value.

**The counterexample mode counts positions, not statements.** Its
coverage is keyed by function, line and column, so statements that start
at the same position are counted together. *Pay*: key coverage by SIR
block and statement (findings already record theirs).

**Error traces are always recorded.** Every `?` of the program appends
its location to the error value, in every build (those of the standard
library and of its templates' instances do not); 05 says traces cost
nothing in release builds. *Pay*: a build profile in which lowering omits
`error.trace_push`.

**`old` copies on entry.** Every `old(e)` of a function is a copy of `e`
made on entry, whatever exit is taken and whatever the level, when the
level checks `ensures`. *Pay*: a backend copies only what a reached
`ensures` reads, and shares what is `Copy` or unchanged.

**Which error set a value belongs to is a name.** An error value carries
its declaration's name, and `error.is[D]` compares names. *Pay*: the
backend chooses a representation for unions of error sets (a global tag
per variant, or declaration plus index); SIR keeps `error.is` either way.
The native backend's first round chose declaration plus index: an error
value's first word is its own type's number, which `error.is[D]`
compares (architecture.md, "Stage 2").

**Panic messages are rendered when raised.** A panic carries English
text rendered from the catalog, not an identifier and parameters, so
there is no `--format=ids` for run-time failures. *Pay*: carry the
`MsgId` and parameters to the top and render there; the conformance
suite can then compare panics by identifier.

**Host details with no Sisu meaning.** The driver catches a host panic
with `catch_unwind` to report an internal compiler error; the
interpreter's step limit exists for the conformance runner's sake; the
frame limit is a constant (100 000). *Pay*: stage 1 has no host panics;
the limits become options of `sisu run`.

## Shortcuts in the front end

**The query layer is hand-written.** `sisu_driver::Db` memoises per
program on hashes of its files' and manifests' texts, and inside that per
item (architecture.md, "The query layer"): checking, lowering and the
ownership checker reuse what an item's text and the program's interface
(a hash of what resolution reads, computed from the tokens) leave
unchanged. Name resolution itself still runs again over the whole program
for every change, std's modules included (only their parsing and the
parsing of the text it writes are shared): 5 to 10 ms, most of the 14 ms a
one-body re-check takes. *Pay*: per-item signature queries keyed by stable
node identities (the tree provides them), so that resolution too redoes
only what a changed declaration reaches; with them, std's resolution is
done once per process.

**The memo is not kept between commands.** The per-item memo lives in the
process: the language server, `sisu run --watch` and the REPL keep it, but
each `sisu check` starts from nothing (55 ms at the median for a
conformance case, the process start and std's parse included). *Pay*: write the
memo to disk keyed by the interface and the items' texts (the typed bodies
and SIR functions it holds are plain data), or a resident server the
command line asks.

**Some steps of a build still run on one thread.** A program's files
are parsed, the bodies the memo lacks checked, its functions put through
the ownership checker and compiled by Cranelift on threads (decision
219), and the derived impls' templates run and the bodies lowered on
threads too (decision 228); what is left on one thread is the rest of
resolution (0.13 s of a clean native build of stage 1, 3.9 s on 4 cores
for 59 000 lines with std), lowering's worklist, and the code generator's
writing of IR (0.6 s), which numbers types, functions and data in the
order it meets them. A package's dependencies are not built apart and at
once across the graph, as 10 plans. *Pay*: IR written on threads into
functions whose numbers are given and rewritten in order when each is
taken, and 10's builds per package.

**Intrinsics get type arguments by a rule.** Generic Sisu functions get
every type argument at run time; an intrinsic gets them only when
lowering's rule says it needs them (a type parameter only in the result,
or an `Iterable[T]` parameter). *Pay*: pass every intrinsic its type
arguments, which the untyped drops above also want.

**Handles forward by dispatch.** A call on a capability handle or a
`Lent` is a `call trait` that the dispatcher resolves by the object in
the handle's cell (`cap.forward` in the prelude, a special case in
lowering's `callee_for` and in `dispatch.rs`), since the object's type is
known only at run time. *Pay*: none for the interpreter. The native
backend compiles a handle as a `dyn` of its trait where the program makes
objects for its type (decision 209), and a call through any other
handle to the host's object (decision 194).

**`Wrap` and `Fake` are written by the compiler.** 06 builds `Wrap[T]`
from the reflection behind `Fake[T]`. std.cap writes `Wrap`'s impl for
each standard capability's trait by hand, and for a program's own trait
the resolver writes the impl and the fake as source text from the
trait's declaration (`sisu-resolve/src/shims.rs`), only for a module
whose text names `Wrap` or `Fake`, and only for traits whose methods can
all be forwarded. *Pay*: a way for compile-time code to write a method
with a reflected signature (Q-27), now that `reflect.methods_of` gives
the trait's methods (decision 251); then one generic impl and `Fake[T]`
as std code, for every trait.

**The host's network is polled.** A host socket that would block makes
its task wait one millisecond on the host's clock and try again, in the
interpreter and natively (decision 209); there is no event loop, and a
host socket is closed only by `close` (dropping a `Conn` does not). *Pay*: an event loop (`epoll` or `kqueue`) behind the
host contract, and a `Drop` for the host's streams.

**`Slice` is a copy, and not `Copy`.** A `Slice[T]` is a list of copies
of the values it views (like `str`), so making one costs a copy, and it
is passed by reference because an intrinsic cannot point into a value
passed to it by value. *Pay*: a view is a pointer and a length in the
backend, and then `Copy`, as `str` is.

**A method's type is read from its name.** The must-consume rule (a
type's own methods are where its values end) recognises a method by its
SIR name, `Type.method`, in the checker and in the monitor. *Pay*: SIR
records a method's owner type.

**Borrow facts of lowering-only intrinsics are written in lowering.**
`iter.ref`, `iter.mut`, `iter.own` and `list.slice` get their `from`,
`inner` and `own` lists from a match in `sisu-lower/src/lib.rs`, because
they have no prelude signature. *Pay*: give them signatures in a std
module that only lowering may call.

**`inner` knows which builtins own text.** Whether a result can point
into an argument's storage is decided by a walk over types in
`sisu-resolve/src/borrows.rs` that knows `String` and `Path` own text and
treats `Iter`, `Entry`, `Iterable` and generic parameters as owning
anything. *Pay*: a property of the type declarations (or a marker
trait, now that traits exist) instead of a list of names.

**Temporaries are named by their source text.** A diagnostic about an
unnamed local names it by the source range of the statement that first
assigns it (`make()` does not live long enough), which depends on
lowering's locations being exact ranges. *Pay*: SIR could record a
display name for temporaries; until then, a front end must emit the same
ranges.

**Fixpoint bounds.** The liveness and must-consume dataflows stop after
a bound (10 000 and 1 000 rounds) and then fall back to the conservative
answer (everything live; no iterator exhausted). Union and intersection
over finite sets converge well before, so the bound is never reached in
practice. *Pay*: a worklist in reverse postorder, which needs no bound.

**`defer` is copied to every exit.** A deferred block is emitted at each
exit of its scope, normal and cleanup, which multiplies code for a scope
with many exits. *Pay*: acceptable for the interpreter; a backend may
share one landing block.

**A format spec is read at run time.** `Spec.parse` reads the spec text
each time a leaf applies it. *Pay*: lowering can pass the spec already
read, `comptime Spec.parse(...)` at the interpolation.

**Templates unfold as text.** A trait's `comptime def derive`, and a
generic function that reflects on its type parameters, are evaluated by
unfolding their bodies into source text (`template.rs`): the
compile-time parts are evaluated over a small domain of values (types,
their fields, variants and attributes, integers, text, booleans, lists
and tuples of them), and the impl or the instance is then parsed and
checked like written code, so an error in it is reported in the
generated text. `reflect.construct` and `reflect.default_of` make code
rather than values. *Pay*: the comptime evaluator over typed IR producing
the impl or the instance as a value (08: "`return impl ... for T:` is an
expression form"), with reflection as a module of values.

**A promotion resolves again.** The instances of templates a check asks
for are declared on top of the resolution it checked, and the bodies are
checked once per check, but a promotion (a generic function that calls a
template with its own type parameters becomes one) changes a declaration,
so the driver resolves the whole program again with it (`reflect/promotion`
re-checks in 117 ms, the slowest of the suite's programs). *Pay*: per-item
signature queries (above); with monomorphisation in the backend, the
instances are its own.

**Compile-time evaluation is incomplete.** There is no memory limit (08
asks for one) and no progress report for a long evaluation; the order of
evaluation follows direct calls and constants, not calls through a
trait; and a `comptime` expression that depends on its function's type
parameters makes the function a template, made per list of type
arguments (decision 244), except in a trait impl's method, which is made
once (decision 251). *Pay*: count heap cells and collection sizes
against a limit; report progress past a time; with monomorphisation (the
backend), evaluate per instantiation there too.

**The library types of 04 are host values.** `Shared`, `Weak`, `Cell`,
`Mutex` (and its guard), `RwLock` (and its read guard), `Atomic` and
`Arena` are values the interpreter represents (`sisu-interp/src/value.rs`;
an `RwLock` is a mutex that counts readers, an `Atomic` a cell), with
their contracts in the prelude and std.task; a `Shared`'s value lives in
a heap of cells that are never reused, and the monitor sees a mutex's
interior through the guard's path. The escape analysis tags each
`Shared` site atomic or not, but the interpreter's counts are never
contended, and an `Atomic`'s operations are atomic because one task runs
at a time, every memory order acting as `SeqCst` (decision 239). The
native runtime has them as objects of its own too, in Rust (decision
207), freeing a `Shared` cell with its last handle, and its counts are
never contended either. An `RwLock` prefers its writers (decision 249),
and a `Condvar` makes a channel for each wait. *Pay*: write them in Sisu
over raw memory and atomics (stage 1 and the backend), with the tags
choosing the counting and the memory orders meaning what 06 says on the
`full` runtime.

**The scheduler is the interpreter's.** Tasks are frame stacks swapped
into the interpreter, a run is single-threaded, and a schedule is the
seeded random, PCT or bounded exhaustive scheduler's (`sched.rs`): there
is no M:N runtime and no work stealing, and the exhaustive search has no
state hashing, so it runs again schedules that reach a state it has
seen. A generator dropped before its end is closed where its consumer
dropped it, outside the schedule (decision 236), so its cleanup cannot
wait (`R0129`); a generator that code which cannot wait takes items from
(an adapter, an intrinsic's `Iterable[T]`) runs to its `yield` there, and
a wait in it aborts (`R0130`, decision 247); a receiver's iterator inside
another iterator's adapter cannot wait (`R0091`); in a `pure` scope the
lowest-index failure is reported, but a body that joins a later task
first sees it in time order. *Pay*: the runtime of 12's profiles; state
hashing in the explorer (18, Phase 6); a closing generator that runs as
a task of its own, which may wait, and so a driven one.

**Faults are injected by the interpreter.** In the `sim` profile, the
interpreter decides a fault at a trait call of `FsOps` or `NetOps` whose
object is one of the simulation's (`cap.SimFs`, `cap.SimNetwork`,
`sim.SimNet`, known by name, `exec.rs`), and builds an error of the
operation's declared set (a path field from the call's path argument).
*Pay*: the fault model as Sisu in the simulation objects themselves,
drawing from the run's seeded generator.

**Tests share the host behind a lock.** The parallel test runner
(`parallel.rs`) gives each test thread a view of the one host whose calls
take a lock, and each thread a 256 MiB stack reserve, as every
interpreter thread has. Only the tests that do not take the real
`System` run in parallel (those that name capabilities get fakes,
decision 241), so only the host's clock is really shared. *Pay*: the M:N
runtime runs tests as tasks.

**The capability log is the host's wrapper.** Recording and replay wrap
the host contract (`record.rs`), so they see the interpreter's host
calls, not Sisu's capability methods; the data a program writes is logged
by its FNV-1a hash, `hash-key` is always 0 (hashing is not keyed per
process in stage 0), and hot replacement logs each save it saw (`edit`)
rather than the swap it made, so a replay compiles the saves again.
*Pay*: the interception point of D-075 (`Wrap` around each root
capability) records, so the log is of Sisu calls; the epochs a run
published are logged as `swap` records, as 10 says, so a replay needs no
compiler.

**Sized numbers and bytes are host values.** A `u64`, `u128`, `i128` and
`f32` are value kinds of their own and the other sized integers are
`int` values kept in range by typed intrinsics
(`sisu-interp/src/sized.rs`), which compute in `i128` up to 64 bits and
in the type's own host integer for the 128-bit ones; a 128-bit literal
beyond `int` travels in SIR as decimal text (`i128.from_text`), since
SIR's integer constant is 64 bits, and an `f32` literal as the `f64` that
holds it (`f32.from_literal`), since SIR has one float constant; `f32`
arithmetic is computed in `f64` and rounded once (`float32.rs`), which
gives `f32`'s results. `bytes` is a shared immutable host buffer. *Pay*:
none for the interpreter; the backend gives each its width, and SIR may
grow 128-bit and `f32` constants.

**`Array` is a list.** An `Array[T, N]` is a host list of `N` values
(`Array.filled` copies its value `N - 1` times), checked against its
length on every index, so it has no inline layout and costs an
allocation. *Pay*: the backend lays it out inline, `N` values wide, and
can drop the check where `N` and the index are known.

**A type's constant expression is folded, not evaluated.** A constant
argument written as an expression (`Array[T, W * H]`) is computed where
the type is written by `sisu-resolve/src/consts.rs`: integers, `const`
parameters, constants whose values are written that way and the integer
operators. A call or `comptime` there is `E0264`, though 08 says such an
argument requires compile-time evaluation, because the evaluator runs on
SIR, after every type is known. An operation on `const` parameters
stays symbolic until substitution folds it, and two such operations are
equal only when they are written alike (`W * H` is not `H * W`). *Pay*:
the comptime evaluator over typed IR (above) run per instantiation, and
normalised constant expressions if programs need the algebra.

**A generic `comptime assert` is folded, not evaluated.** A `comptime
assert` over `const` parameters is checked at each call that gives them
constants by folding its condition (`sisu-typeck/src/generic_asserts.rs`):
integer and boolean arithmetic, comparisons and interpolation. One that
calls a function, or a call whose constants come from the caller's own
`const` parameters, is checked nowhere, and the run never sees it either.
*Pay*: evaluate it per instantiation with the comptime evaluator (with
monomorphisation, the backend's).

**The ownership checker trusts std per program.** A program's run takes
std's function bodies as checked (`Options.trust_std`): their drops are
elaborated but their loans are not analysed again for every program;
`sisu test --std` and `sisu-driver/tests/std_ownership.rs` (every std
body lowered) check them. *Pay*: none while std is checked whenever it
changes; a later stage must keep that check in CI.

**Hot replacement compares whole programs and keeps every epoch.** A save
is compiled through the database the run was compiled with, so only the
items it changed are checked, lowered and borrow-checked again, but the
interpreter then compares every function, type and impl of the new SIR
with all the run has (`sisu-interp/src/live.rs`); the programs of every
epoch live as long as the process (old frames point into them), so a long
session's memory grows with its saves. A restart drops the old run's
values without their `Drop`s, and there is no migration of a value whose
layout changed: the program starts again. *Pay*: the rebuild tells the
interpreter which functions its per-item queries made again, so it
compares only those; a backend's indirection table frees an epoch's code
when no frame of it is left; a layout change could migrate values by a
`live` hook (D-068 leaves it open).

**The REPL replays its session.** Each input runs every earlier input
again against the log of the run before it (`record::Resume`), so a
session of n inputs does O(n²) work, and an input whose capability
results the world no longer gives diverges (`D0091`). `:why` follows the
session's own bindings and their inputs' logs, not the data flow inside
the functions they call (10's `sisu why`, a dynamic slice). *Pay*: the
comptime interpreter of 10 keeps the session's values, or snapshots of
them; the monitor's trace gives the dynamic slice.

**Property testing is small.** The shrinker deletes runs of choices and
lowers each by binary search, at most 1000 attempts; there is no
database of failing cases (17's `tests/corpus/`), no `--fuzz` and no
coverage guidance, and a case's budget is statements, not time. *Pay*:
Hypothesis's shrink passes and example database, then the fuzzer.

**Snapshots and transcripts update without a ceremony.** `sisu test
--update` shows each change and writes the files; it does not warn above
a size, refuse content the fakes have not pinned (17), or ask. *Pay*:
the review flow of 17, with the size threshold and the check for
unpinned content.

**Benchmarks have no statistics.** `sisu bench` reports the mean time of
an iteration on the host's clock, measured around the whole benchmark,
with no warm-up, confidence interval or comparison with a baseline (17).
*Pay*: the harness of 17: repeated samples, intervals, a stored baseline.

**`Contextual` is displayed by a host rule.** `Result.context` wraps an
error in std's `Contextual`, whose `Display` (`message: cause`) is a case
in the host's display text (`format.rs`) because error sets display
structurally. *Pay*: a `Display` impl in the prelude, when error sets can
have impls that the fast path respects.

## Stage 2: what the native backend leaves out

The native debug backend (architecture.md, "Stage 2") compiles every
program the conformance suite runs (`conformance/native.list`, 165
cases), with the debug profile and the release one (decision 232), and
every one of those runs frees what it allocated (decision 226). A
construct it does not compile is `D0103` under `--native`, and the
program runs as before without it (`sisu-conformance --native-survey
conformance/native.list` tries the cases outside the list). What it
leaves out:

- **Intrinsics the runtime does not have**, which no runnable case
  reaches; `sisu_rt::abi::Intrinsic` lists the ones it has. *Pay*: each
  one, as the construct guide's step 8 says.
- **A field of a union of error sets read through the union**, which
  stage 0's lowering no longer emits (a pattern reads a member's fields
  through a place of the member's type, decision 221), but SIR from
  elsewhere may: members that lay the field out differently (beyond a
  `String` in one and a `Path` in another, decision 211) are `D0103`, and
  so is a function's constant naming a generic function (stage 0 makes a
  closure of its value wrapper instead, decision 225). *Pay*: a read
  through a switch on the member, if another front end needs one.
- **The interpreter's modes**: the ownership monitor, the counterexample
  mode, the capability log and replay, hot replacement, `--stats
  stack-depth` and `--stats shared`, the schedulers other than the
  seeded one (`--scheduler pct`, `exhaustive`), and `sisu test`, whose
  tests the backend does not compile. *Pay*: `sisu test --native`, now
  that tasks are native (tests run in parallel tasks); PCT and the
  bounded search as the runtime's other ways to pick (decision 204 keeps
  the generator's draws in one place).

What round 2 (tasks, decisions 203 to 206; the rest of the suite,
decisions 207 to 211) leaves:

- **A task's stack is finite natively.** The interpreter limits only the
  number of frames (decision 31); a native task has `SISU_TASK_STACK`
  (1 MiB) of stack, and the debug backend's frames are large (every
  temporary has a slot), so a recursion of a few thousand calls that the
  interpreter runs in a task overflows it natively (the same
  `panic.stack-overflow` the interpreter raises at 100 000 frames,
  decision 234; the root task keeps 1 GiB). *Pay*: smaller frames (the
  entry on temporaries below), and the default reservation from the
  corpus's histogram of depths (`--stats stack-depth`), 8 MiB being the
  Phase 1 report's next step.
- **Yield checks everywhere the interpreter has them**: at every
  function's entry and every back edge, so that a seed means the same
  schedule. The Phase 1 report found a slice of a few hundred microseconds
  enough against starvation and the check on every back edge costly on one
  machine. *Pay*: a release backend checks less often (a counter on back
  edges, none in leaf loops that cannot run long), and a seed then names a
  schedule of its own.
- **A function entered without its count whose entry fails** its probe or
  its count of nested calls before its yield point leaves the budget one
  higher than the interpreter's (only when it would not have reached the
  scheduler; decision 204). Only a stack exhausted or 100 000 calls deep
  at that very call shows it. *Pay*: the yield point before the probe,
  which needs the probe's reserve to hold the scheduler's call.

What round 1 does, and a later round must not copy:

- **Every part of a value is a word** (decision 188): a `u8` takes eight
  bytes, an option a word for its tag, a `bool` a word. *Pay*: in a
  release backend, each type its own width and alignment, and options
  that use a niche; the layout stays in `sisu_rt::abi`, so both sides
  change together.
- **Every temporary has a stack slot of its own**, so a large function's
  frame is the sum of its temporaries, except a release build's scalars
  and references, which are Cranelift variables (decision 246). *Pay*:
  slots reused between statements, and the debug profile's scalars in
  variables too once its debug information can describe them.
- **Instances are not shared**: each list of type arguments compiles a
  function again (decision 191); 10 asks for sharing by shape and a
  budget. *Pay*: compile once per layout rather than per type where the
  code is the same.
- **The type table is read at start**, and the structural operations walk
  it on every call (decision 192), as the interpreter's walk its values.
  *Pay*: a release backend generates them per type where profiling says
  so.
- **Debug information is partial** (decision 227): the line table, the
  locals and parameters with their types, and the frames are there, but
  a local is described at its slot for the whole function, not from its
  `let` to the end of its scope, so a debugger shows one not yet
  assigned with what its slot holds, and two locals of one name in
  different scopes side by side; function values, dictionaries, sets,
  iterators, channels and tasks are opaque words or addresses; a lent
  parameter is a reference; lldb without a Rust plugin prints a variant
  part raw; there is no `sisu run --profile` (10, "Debugging and
  profiling"). *Pay*: lexical blocks from the code of each scope, and
  descriptions of the runtime's structures from the type table.
- **The release profile is Cranelift's optimisation level** (decision
  232), not 09's LLVM, with scalars in registers and the integer
  division inline (decision 246: arithmetic loops 1.3 to 2.9 times
  faster): nothing a contract proves removes a check, and the runtime's
  calls cost what they cost in a debug build, and there is no
  `release-size`. The commonest intrinsics are inline in both profiles
  (decision 253: stage 1 twice as fast); the others still pass a
  descriptor that the runtime decodes, and follow their arguments'
  references by the type table, on every call. *Pay*: the rest of the
  intrinsics stage 1 calls most (text building, a dict's reads and
  writes, `.copy()` and drops of structured values) without the
  descriptor, and LLVM for release builds (18, stage 2).
- **The runtime trusts the generated code**: an address or a type number
  that breaks the runtime's rules is not checked everywhere, and where it
  is, it is `R0122` and status 134, not a diagnostic of the program.

## Not yet done from Phases 0, 2, 3, 4, 6 and 8

- **The parser fuzz target** (Phase 0): "never panics, always returns a
  tree that reprints its input byte for byte, never emits a diagnostic
  storm", run for 24 hours and then as a CI job. Two deterministic tests
  hold the property on every `cargo test`: the parser's
  (`fuzz_never_panics_and_round_trips`, 3 000 random token strings and
  mutations of every proposal example, with a bound on diagnostics per
  line) and the driver's (`mutated_programs_never_break_the_pipeline`,
  300 mutated programs through every stage, a typed tree for each and
  valid SIR for those that check), and the suite checks the reprint for
  every case. There is no coverage-guided fuzzer and no long run. (The
  wrong-program corpus Phase 2 asks for is `conformance/cases/wrong`:
  forty programs with one mistake each, held by `tests/wrong.rs` to one
  error that names its fix, each fix with edits applied and checked.)
- **Benchmarks as a gate** (Phase 0). CI runs `cargo run --release -p
  sisu-driver --example speed` on every push and prints the front end's
  milliseconds per kLOC (text to the SIR that runs) and the
  interpreter's nanoseconds per step (a SIR statement or terminator) for
  std and the conformance corpus; it fails nothing, runs once and keeps
  no history. On the development machine (4 shared cores): std's 6 590
  lines in 62 ms, 9.4 ms per kLOC (28 ms once parsed); the corpus's 358
  cases, 8 571 lines, in 2.3 s beyond std's, 272 ms per kLOC or 6.5 ms per
  case, which is work per program (the std functions a program reaches
  are lowered and checked with it) more than per line; the interpreter
  at 194 ns per step over std's doc examples (16 558 steps, a fresh
  interpreter per example) and 128 ns over the 127 programs of the corpus
  that run alone (3.5 million steps). Phase 2's "parse to SIR under 50 ms
  per kLOC" holds for std, not for the corpus's small files.
- **The examples.** `hello.sisu` and `wordcount.sisu` run unmodified;
  `traits.sisu` and `errors.sisu` run with a `main` (and, for
  `errors.sisu`, the `default_port_for` it uses) appended, and the
  conformance cases are kept starting with the originals by a test.
  `ownership.sisu`: everything before `Shared` (`ownership/proposal_core`)
  and the graph section (`ownership/proposal_graph`) run, kept identical
  by tests, as does the `Shared` section (`shared_demo` and
  `counter_demo`, `ownership/proposal_shared`).
  `comptime.sisu`: the user-defined derive (`Describe` and `Point`) runs
  as `comptime/proposal_derive`, its JSON encoder over reflection, the
  derive and the conditional compilation with its test run as
  `reflect/proposal_comptime`, and the CRC table and the ring buffer
  (`Array`, `u32` arithmetic, const generics and a generic `comptime
  assert`) as `comptime/proposal_arrays`, each section kept identical by
  a test (`sisu run` leaves out its `@cfg(test)` helper, decision 243);
  the checked SQL needs the `sql` package. `capabilities.sisu` (its
  tests), `concurrency.sisu`, `contracts.sisu`
  (`contracts/proposal_contracts`) and `testing.sisu`
  (`testing/proposal_testing`, whose last test reads `HOME` from the
  real environment and fails in the suite's empty one) run with their
  missing declarations (and a `main`) appended, kept starting with the
  originals by a test. `ffi.sisu` needs foreign functions.

From Phase 3 (18, "Phase 3: the ownership checker and the dynamic
monitor"):

- **The generation-checked reference type** behind a flag, if Phase 0
  triggered it.
- **The corpora**: a translated slice of rustc's borrow-checker UI tests,
  the referee's 20 adversarial programs, and the 60-file wrong-ownership
  corpus with expected codes and named fixes. The suite has 74 ownership
  cases (32 accepted, 42 rejected) and the soundness tests below.
- **24 hours of mutation fuzzing.** `sisu-driver/tests/soundness.rs` runs
  a deterministic mutation fuzz (statement reorderings of accepted
  programs' SIR) on every `cargo test`, and more with
  `SISU_FUZZ_MUTANTS`; the longest run so far is 27 000 mutants (8 054
  accepted by the checker, none tripping the monitor), not 24 hours, and
  it is not a CI job.
- **Diagnostics that name the fix.** Most ownership diagnostics have a
  note or a fix with an edit (`E0411`, `E0413`, `E0415`); the conflict
  diagnostics' notes are prose, not edits.

From Phase 4 (18, "Phase 4: traits, generics, std in Sisu"):

- **Const generics beyond `int`.** Every number type of 03 (`f32`
  included), `bytes`, `Slice`, `Array` and const generics exist. A
  `const` parameter is an `int` (another type is `E0900`); its argument
  may be an integer expression of literals, `const` parameters and
  constants (03's `Array[T, W * H]`), folded where it is written (above),
  but not a call or `comptime` (`Array[u32, comptime size()]`).
- **`std` as modules.** The core traits and their impls, the iterator
  adapters, the collection methods that take closures, formatting and
  `parse_int` and the text methods are written in Sisu in the prelude,
  which 11 makes `std.core`; 11's modules are modules an import names
  (decisions 245 and 252), written in Sisu over the prelude's
  intrinsics. What 11 names and is not there: `std.mem` (the `Allocator`
  trait, `Region`, D-060's allocator parameter of the collections),
  `std.reflect` as a module (reflection is the compile-time `reflect`),
  `From` and a `Range` type in `std.core`, `SoaList` (Q-27's type
  construction) and the `persistent` collections, `std.string`'s
  graphemes and normalisation forms (Unicode's tables), `std.math`'s
  elementary functions (Q-23), pipes, signals and the working directory
  in `std.process` and `std.env` (the host contract runs a program
  without input and has no current directory), time zones in
  `std.time`, decoding an enum in `std.json` (reflection makes no
  variant), and a `BigInt` literal beyond `int` (03: checked at compile
  time; a literal reaches `FromLiteral[int]` only). `Decimal` reads a
  float literal from its shortest digits, so one with more significant
  digits than an `f64` holds is rounded; a `str` literal through
  `Decimal.parse` is exact. The modules run their work on the asking
  task, `std.par`'s chunks one after another, and `std.io` has no
  stream over a file, since the file capability reads and writes whole
  files.
- **Reflection beyond templates** (Phase 8 in 18): `reflect`, `comptime
  for` and `x.[f]` work in functions, methods, a trait's own methods,
  the methods of trait impls, `comptime def`s (with `T: type`
  parameters), constants, defaults, tests and closures (decisions 244 and
  251), but not in a refinement's `where` clause or an `invariant`
  (`E0900`, naming the place). A trait impl's method is made once for
  every type argument, so it cannot ask about its type parameters
  (`E0609`); per-type instances behind a trait's dispatch wait for
  monomorphisation in the backend. `reflect.methods_of` names a trait's
  methods, but no syntax makes a method from a descriptor (Q-27), which
  a recording fake derived from an interface needs (below). (`@cfg` is
  there: decision 243.)

From Phase 6 (18, "Phase 6: tasks, capabilities, an HTTP server in
Sisu"):

- **The `sim` profile**'s network has partitions and faults but no
  latency or loss model, and the bounded exhaustive search has no state
  hashing (above).
- **`task.scope`'s trace context** beyond the task path `log` prints.
- **The 10k-connection echo server** (the exit criterion is functional,
  not fast): the polled host network above holds connections but has not
  been measured at that count; the corpus echo and HTTP servers run over
  `Net.fake` under `--explore 1000` (`http/server`).
- **`--stats` latency of an I/O task under a compute-bound sibling** (the
  Q-34 number) is reported but not yet measured on a corpus program.

From Phase 8's tools (10, "Formatter", "Language server", "Debugging
and profiling", "REPL and scripts"):

- **The language server** has completion, rename and references, the
  catalog's fixes as code actions, inlay hints, "explain this borrow" and
  worksheets as inline values, but documents are synchronised whole;
  rename finds a type's uses in types by their spelling (a type of
  another module with the same name would be renamed too, since types in
  annotations are not in the typed tree); completion after `.` checks the
  document again without the word being typed. Of the four features 10
  puts after 0.1, worksheets in the editor are there; always-on
  evaluation, structural editing (N-31's commands beyond the fixes) and
  the stepper are not.
- **The formatter** writes 10's `T ! E` for `Result[T, E]` when `E`
  names an error set, which the driver finds by resolving the file
  (`sisu-driver/src/format.rs`). `sisu fmt` resolves each file as a
  program of its own, so in a package an error set the file imports from
  another of the package's modules is not known and its `Result[T, E]`
  stays as written; the language server formats a document with its
  whole program. Resolving the package (or caching its error sets) would
  make the two agree.
- **Hot replacement** swaps bodies only; resuming from a chosen frame
  after a fix (10's stated goal of the debug ABI) is a backend's, and
  DWARF-level debugging is the native backend's (decision 227).
- **The REPL** runs a session's inputs in the run interpreter, by
  replay, not in the comptime interpreter as 10 says; it has no
  completion and no `:type` or `:doc`.
