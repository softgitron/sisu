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

**Float to text.** `Display` of an `f64` is Rust's shortest round-trip
form and `{x:.N}` is Rust's correctly rounded fixed form (the intrinsic
`fmt.float_digits`, `sisu-interp/src/format.rs`; the rest of a format
spec is Sisu). Any correct shortest-round-trip algorithm gives the same
digits, but the exponent spelling (`1e100`, `1.5e-7`) and the rounding of
fixed forms are Rust's. *Pay*: specify both in std's contract and
implement them in Sisu (Ryu-style shortest digits, round half to even on
the exact binary value).

**Text to float.** Float literals are converted by Rust's `str::parse`
in the lexer, which is correctly rounded. *Pay*: stage 1 needs a
correctly rounded parser; add literals at rounding boundaries to the
suite.

**Hashing.** `dict` and `set` keep an insertion-ordered vector with a Rust
`HashMap` as the index (`sisu-interp/src/value.rs`), over the host's
structural key of a value, not the `Hash` and `Eq` impls: a key type with
impls of its own is rejected (`E0398`) instead. Nothing observable
depends on the hash, since iteration is in insertion order (D-074).
*Pay*: write `dict` and `set` in Sisu over `Hash` and `Eq`, with the
per-process SipHash key of D-049 as std's, and lift `E0398`.

**Text search is naive.** `str.byte_find`, which `find`, `contains`,
`split`, `split_once`, `lines` and `replace` are written over, tries each
place where the first byte matches: O(n·m) for a text of n bytes and a
pattern of m in the worst case (`"aaaa...b".find("aa...ab")`). *Pay*: a
search with a linear bound (Two-Way, as Rust's `str::find`), in Sisu.

**Text to `f64` at run time.** `str.parse_float` (which `cli.parse` uses
for an `f64` option) is Rust's `parse`, after a check that the text is
decimal syntax (no `inf` or `nan`); `str.parse_int` is Sisu. *Pay*: a
Sisu `f64` parser, correctly rounded, as for literals.

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
view carries. *Pay*: a view is a pointer and a length in the backend.

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
program on hashes of its files' and manifests' texts, so every change
re-resolves and re-checks the whole program, std's modules included
(only their parsing is shared): the language server answers a change to
`std/num.sisu` in about 65 ms and to `std/json.sisu`, whose reflection
takes three rounds (below), in about 115 ms, against 10's 100 ms. *Pay*:
per-item queries with stable node identities as keys (the tree already
provides them).

**Intrinsics get type arguments by a rule.** Generic Sisu functions get
every type argument at run time; an intrinsic gets them only when
lowering's rule says it needs them (a type parameter only in the result,
or an `Iterable[T]` parameter). *Pay*: pass every intrinsic its type
arguments, which the untyped drops above also want.

**Capability objects are never freed.** A capability handle's object and
a `Cap`'s value live in a heap cell that every copy reaches and that
nothing frees, because a `Copy` handle has no last copy; a `Drop` of the
object never runs. In the interpreter the cells go with the run. *Pay*:
the backend needs a lifetime for them: an arena per run (they are few),
or counting copies, which would make handles not `Copy`.

**Handles forward by dispatch.** A call on a capability handle or a
`Lent` is a `call trait` that the dispatcher resolves by the object in
the handle's cell (`cap.forward` in the prelude, a special case in
lowering's `callee_for` and in `dispatch.rs`), since the object's type is
known only at run time. *Pay*: none for the interpreter; a backend
compiles a handle as a `dyn` of its trait.

**`Wrap` and `Fake` are written by the compiler.** 06 builds `Wrap[T]`
from the reflection behind `Fake[T]`. std.cap writes `Wrap`'s impl for
each standard capability's trait by hand, and for a program's own trait
the resolver writes the impl and the fake as source text from the
trait's declaration (`sisu-resolve/src/shims.rs`), only for a module
whose text names `Wrap` or `Fake`, and only for traits whose methods can
all be forwarded. *Pay*: reflection over a trait's methods
(`reflect.methods_of`) and a way for compile-time code to write a method
with a reflected signature (Q-27); then one generic impl and `Fake[T]` as
std code, for every trait.

**The host's network is polled.** A host socket that would block makes
its task wait one millisecond on the host's clock and try again; there is
no event loop, and a host socket is closed only by `close` (dropping a
`Conn` does not). *Pay*: an event loop (`epoll` or `kqueue`) behind the
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

**Instances are found by resolving again.** Which instances of templates
a program needs is known once its calls are typed, so the driver
resolves and checks the whole program again with the checker's requests,
up to 32 rounds (`Db::checked`); each round repeats all the work of the
one before. *Pay*: per-item queries, so a round re-checks only the new
instances and their callers; with monomorphisation in the backend, the
instances are its own.

**Compile-time evaluation is incomplete.** There is no memory limit (08
asks for one) and no progress report for a long evaluation; the order of
evaluation follows direct calls and constants, not calls through a
trait; and a `comptime` expression cannot depend on its function's type
parameters, since nothing is monomorphised. *Pay*: count heap cells and
collection sizes against a limit; report progress past a time; with
monomorphisation (the backend), evaluate per instantiation.

**The library types of 04 are host values.** `Shared`, `Weak`, `Cell`,
`Mutex` (and its guard) and `Arena` are values the interpreter
represents (`sisu-interp/src/value.rs`), with their contracts in the
prelude; a `Shared`'s value lives in a heap of cells that are never
reused, and the monitor sees a mutex's interior through the guard's
path. The escape analysis tags each `Shared` site atomic or not, but the
interpreter's counts are never contended. *Pay*: write them in Sisu over
raw memory and atomics (stage 1 and the backend), with the tags choosing
the counting.

**The scheduler is the interpreter's.** Tasks are frame stacks swapped
into the interpreter, a run is single-threaded, and a schedule is the
seeded random, PCT or bounded exhaustive scheduler's (`sched.rs`): there
is no M:N runtime and no work stealing, and the exhaustive search has no
state hashing, so it runs again schedules that reach a state it has
seen. A generator dropped before its end is abandoned; a receiver's or
generator's iterator inside another iterator's adapter cannot wait
(`R0091`); in a `pure` scope the lowest-index failure is reported, but a
body that joins a later task first sees it in time order. *Pay*: the
runtime of 12's profiles; state hashing in the explorer (18, Phase 6);
the generator's frames dropped with it.

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
interpreter thread has. Only the tests that take no `System` run in
parallel, so only the host's clock is really shared. *Pay*: the M:N
runtime runs tests as tasks; per-test capability declarations (17) let
the tests that touch only fakes of the world run in parallel too.

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

**Sized integers and bytes are host values.** A `u64`, `u128` and
`i128` are value kinds of their own and the other sized integers are
`int` values kept in range by typed intrinsics
(`sisu-interp/src/sized.rs`), which compute in `i128` up to 64 bits and
in the type's own host integer for the 128-bit ones; a 128-bit literal
beyond `int` travels in SIR as decimal text (`i128.from_text`), since
SIR's integer constant is 64 bits. `bytes` is a shared immutable host
buffer. *Pay*: none for the interpreter; the backend gives each its
width, and SIR may grow a 128-bit constant.

**`Array` is a list.** An `Array[T, N]` is a host list of `N` values
(`Array.filled` copies its value `N - 1` times), checked against its
length on every index, so it has no inline layout and costs an
allocation. *Pay*: the backend lays it out inline, `N` values wide, and
can drop the check where `N` and the index are known.

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

**Hot replacement compares whole programs and keeps every epoch.** Each
save is compiled again from the top, and the interpreter compares every
function, type and impl of the new SIR with all the run has
(`sisu-interp/src/live.rs`); the programs of every epoch live as long as
the process (old frames point into them), so a long session's memory
grows with its saves. A restart drops the old run's values without their
`Drop`s, and there is no migration of a value whose layout changed: the
program starts again. *Pay*: per-item queries decide what changed; a
backend's indirection table frees an epoch's code when no frame of it is
left; a layout change could migrate values by a `live` hook (D-068 leaves
it open).

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

## Not yet done from Phases 0, 2, 3, 4, 6 and 8

- **The wrong-program corpus.** The suite has about thirty cases with
  expected codes under `syntax/`, `names/` and `types/`; Phase 2 asks for
  forty files, and for the checker to return a typed tree for every
  parser-fuzz input.
- **The parser fuzz target** (Phase 0): "never panics, always returns a
  tree that reprints its input byte for byte, never emits a diagnostic
  storm". The suite checks the reprint property for every case; there is
  no fuzzer.
- **Benchmarks in CI** (Phase 0): front-end milliseconds per kLOC and
  interpreter nanoseconds per SIR statement, and Phase 2's "parse to SIR
  under 50 ms per kLOC".
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
  a test; the checked SQL needs the `sql` package, and `@cfg` is not
  there. `capabilities.sisu` (its tests), `concurrency.sisu`,
  `contracts.sisu` (`contracts/proposal_contracts`) and `testing.sisu`
  (`testing/proposal_testing`, whose last test reads `HOME` from the
  real environment and fails in the suite's empty one) run with their
  missing declarations (and a `main`) appended, kept starting with the
  originals by a test. `ffi.sisu` needs foreign functions.

From Phase 3 (18, "Phase 3: the ownership checker and the dynamic
monitor"):

- **The generation-checked reference type** behind a flag, if Phase 0
  triggered it; `sisu check --diary`.
- **The corpora**: a translated slice of rustc's borrow-checker UI tests,
  the referee's 20 adversarial programs, and the 60-file wrong-ownership
  corpus with expected codes and named fixes. The suite has 73 ownership
  cases (31 accepted, 42 rejected) and the soundness tests below.
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

- **`f32`** is `E0900`; every sized integer type, `bytes`, `Slice`,
  `Array` and const generics exist. A `const` parameter is an `int`
  (another type is `E0900`), and its argument an integer literal or a
  `const` parameter: an expression, as 03's `Array[T, W * H]`, does not
  parse there yet.
- **`@derive(Default)`**; the auto traits `Send`, `Sync` and `Escapable`
  are inferred, and `Copy` is computed from fields.
- **`std` as modules.** The core traits and their impls, the iterator
  adapters, the collection methods that take closures, formatting and
  `parse_int` and the text methods are written in Sisu, but in the
  prelude, not as `std.core`, `std.collections`, `std.string` and
  `std.fmt` modules (std.collections and std.num re-export the
  prelude's); the host-backed collections are intrinsics with their
  contracts there.
- **The re-check target** of 300 ms for one edited body: the language
  server re-checks a whole std module in 50 to 115 ms (above), but the
  query layer is per program, not per body.
- **Reflection beyond templates** (Phase 8 in 18): `reflect`, `comptime
  for` and `x.[f]` in methods and non-generic functions, reflection over
  a trait's methods, and `comptime def` in a trait other than its derive
  are `E0900`; `@cfg` is not there.

From Phase 6 (18, "Phase 6: tasks, capabilities, an HTTP server in
Sisu"):

- **The `sim` profile**'s network has partitions and faults but no
  latency or loss model, and the bounded exhaustive search has no state
  hashing (above).
- **`RwLock`, `Atomic`, `Condvar`, `Once`**, and `task.scope`'s trace
  context beyond the task path `log` prints.
- **`sisu audit --caps`**: capability parameters per I/O function,
  threading depth to `main`, structs bundling three or more.
- **The 10k-connection echo server** (the exit criterion is functional,
  not fast): the polled host network above holds connections but has not
  been measured at that count; the corpus echo and HTTP servers run over
  `Net.fake` under `--explore 1000` (`http/server`).
- **`--stats` latency of an I/O task under a compute-bound sibling** (the
  Q-34 number) is reported but not yet measured on a corpus program.
- **Per-test capability declarations** beyond `sys: System` (which
  receives the real one, or the simulated one under `--sim`), and the
  test cache keyed by the capabilities a test touched.

From Phase 8's tools (10, "Formatter", "Language server", "Debugging
and profiling", "REPL and scripts"):

- **The language server** has no completion, rename, references, fix-its
  as code actions, inlay hints for inferred types and parameter
  conventions, or the "explain this borrow" command (the CLI's
  `explain-borrow` has it); documents are synchronised whole. None of the
  four features 10 puts after 0.1 (always-on evaluation, worksheets in
  the editor, structural editing, the stepper) is there: a worksheet runs
  from the command line (`sisu ws`), not as the editor's virtual text.
- **The formatter** keeps `Result[T, E]` rather than writing 10's
  `T ! E`, since whether `E` is an error set is not in the syntax; with
  name resolution in the formatter's reach (the language server has it)
  it could be.
- **Hot replacement** swaps bodies only; resuming from a chosen frame
  after a fix (10's stated goal of the debug ABI) and DWARF-level
  debugging are a backend's.
- **The REPL** runs a session's inputs in the run interpreter, by
  replay, not in the comptime interpreter as 10 says; it has no
  completion and no `:type` or `:doc`.
