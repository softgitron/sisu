# Port debt

Stage 0 is written in Rust and borrows its runtime from the host
([18](../../proposal/18-next-steps.md), "Implementation language and
architecture"). Every place where a result depends on Rust rather than
on a rule written down, and every shortcut that a later stage must not
copy, is listed here, so that the stage-1 front end in Sisu and the native
backend can be transcriptions of documented behaviour
([architecture.md](architecture.md)) instead of reverse engineering.

Each entry says where the debt is, why it is debt, and what paying it
means. The second part lists what Phases 0 and 2 ask for that stage 0
does not have yet.

## Behaviour that comes from the host

**Unicode tables.** `char.is_alphabetic`, `is_whitespace`,
`is_uppercase`, `is_lowercase`, `str.to_lowercase`, `to_uppercase`,
`split_whitespace` and `trim` use the tables of the Rust standard
library (`sisu-interp/src/intrinsics.rs`), which follow whatever Unicode
version that Rust release ships. *Pay*: name the Unicode version in the
std contracts and pin it in stage 1's tables; add conformance cases at the
edges (a character added in a recent version, Turkish dotted `İ`).

**Float to text.** `Display` of an `f64` is Rust's shortest round-trip
form and `{x:.N}` is Rust's correctly rounded fixed form
(`sisu-interp/src/format.rs`). Any correct shortest-round-trip algorithm
gives the same digits, but the exponent spelling (`1e100`, `1.5e-7`) and
the rounding of fixed forms are Rust's. *Pay*: specify both in std's
`fmt` contract and implement them in Sisu (Ryu-style shortest digits,
round half to even on the exact binary value).

**Text to float.** Float literals are converted by Rust's `str::parse`
in the lexer, which is correctly rounded. *Pay*: stage 1 needs a
correctly rounded parser; add literals at rounding boundaries to the
suite.

**Hashing.** `dict` and `set` keep an insertion-ordered vector with a Rust
`HashMap` as the index (`sisu-interp/src/value.rs`). Nothing observable
depends on the hash, since iteration is in insertion order (D-074). *Pay*:
keep it that way; when `Hash` becomes a trait (Phase 4), the per-process
SipHash key of D-049 is the std implementation's, not the host's.

**Integer and string parsing at run time.** `str.parse_int` checks the
syntax itself, then uses Rust's `parse` for the value; `cli.parse` uses
Rust's `parse` for `int` and `f64` options. *Pay*: write both in Sisu
with the contracts in `compiler/std/`.

**Civil time.** `DateTime` is seconds since the epoch in UTC and
`strftime` supports a fixed subset (`format.rs`). *Pay*: the time module
of `std` replaces it; time zones are its problem.

## Shortcuts in the interpreter

**Callbacks nest the host stack.** Sisu calls are trampolined, but an
intrinsic that calls a closure (`list.sort_by`, `Iter.map`, `filter`,
`find`, `fold`, `max_by`, `min_by`, `all`, `any`, `Option.map`) runs a
nested run loop on the host stack, and so does drop glue calling a type's
`Drop`. The run uses a 256 MiB host stack to make this safe. *Pay*: write
these functions in Sisu (Phase 4, "std in Sisu"), and emit drop glue as SIR
functions (below); then only the run loop remains in the host.

**Drop glue is computed from values.** `drop` walks the value it drops
and finds `Drop` implementations by type name at run time
(`sisu-interp/src/exec.rs`), and a partly moved value is handled by the
`Moved` marker left in its moved parts. A native backend has neither.
*Pay*: generate a drop function per type in SIR (fields in reverse order,
elements in order), and track partial moves with per-field drop flags or
move paths in lowering, as Rust's MIR does.

**`str` is a copy.** `strview` copies the text into a shared immutable
string, so a view costs O(length). Semantics are the same because the
viewed `String` cannot change while the view exists, which Phase 3
enforces. *Pay*: a view is a pointer and a length in the backend.

**References are paths.** A reference is a frame, a local and a path of
steps walked on every access, with a serial number to detect a reference
into a returned frame. *Pay*: none for the interpreter (it is also the
dynamic monitor's hook); a backend uses addresses.

**Ownership errors are run-time panics.** Reading a moved value and using
a dangling reference panic (`R0033`, `R0034`), because there is no
ownership checker yet. *Pay*: Phase 3's checker rejects these programs;
the interpreter's checks become the dynamic monitor of 18, which treats
them as internal soundness errors with the SIR dump.

**Error traces are always recorded.** Every `?` appends its location to
the error value, in every build; 05 says traces cost nothing in release
builds. *Pay*: a build profile in which lowering omits `error.trace_push`.

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

**The query layer is hand-written.** `sisu_driver::Db` memoises per file
on a hash of its text. *Pay*: per-item queries with stable node identities
as keys (the tree already provides them) before the language server.

**Bounds are a table.** Which types are `Display`, `Ord`, `Hash` and so
on is decided by `Program::satisfies` in `sisu-resolve/src/ty.rs`, and
`Display` of a user type is its `Debug` form. *Pay*: Phase 4's traits
and impls replace the table; the prelude's bounds become real traits.

**Generic intrinsics run polymorphically.** Std generics are checked with
inference but run without monomorphisation; an intrinsic receives type
arguments only when lowering's rule says it needs them (a type parameter
only in the result, or an `Iterable[T]` parameter). *Pay*: with generic
Sisu bodies (Phase 4), pass type arguments as run-time values to every
generic call, as 18 describes, and drop the rule.

**`cli.parse` is one intrinsic.** It stands in for a comptime derive over
reflection (11, `std.x.cli`). *Pay*: Phase 4 and 8.

**Capabilities are host objects with flags.** `read_only` and
`restrict_to` are flags on the Rust-side handle, and `Fs.in_memory` is a
Rust map shared through a reference-counted cell. *Pay*: 18 puts fakes in
Sisu over the `std.cap` traits; the shared map needs synchronisation once
tasks exist (Phase 6).

**Purity is checked on the typed tree.** 18 lists the purity checker among
the SIR checkers (pipeline step 6). *Pay*: move it to SIR with the other
dataflow checks in Phase 3.

**Drop flags come from lowering twice.** A function is lowered once to
find locals moved as a whole and again to give them flags. *Pay*: an
initialisation dataflow on SIR (Phase 3 needs it anyway) decides where a
flag is needed, including for partial moves.

**`defer` is copied to every exit.** A deferred block is emitted at each
exit of its scope, normal and cleanup, which multiplies code for a scope
with many exits. *Pay*: acceptable for the interpreter; a backend may
share one landing block.

**Formatting lives in the interpreter.** The format-spec mini-language,
`Display` and `Debug` are Rust code in `format.rs`. *Pay*: a `fmt`
module in Sisu (Phase 4), with the rules in architecture.md as its
contract.

## Not yet done from Phases 0 and 2

- **Modules and packages.** Phase 2 asks for file and directory modules,
  `pub`/`pub(package)`, `sisu.toml` with path dependencies and `sisu run`
  on a package. Stage 0 runs one file and imports only std modules
  (`import cli`, `import std.math`); `pub` parses and has no effect yet,
  since nothing imports user code.
- **`Array`** as a host-backed builtin.
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
- **Power assertions.** A failing `assert a == b` shows both sides; 17
  wants the value of every sub-expression.
- **The other examples.** `hello.sisu` and `wordcount.sisu` run
  unmodified; `errors.sisu` checks except for the two names it uses
  without defining (`default_port_for`, and `.context`, a std convention
  that does not exist yet) and has no `main`; the other examples use
  features of later phases.
