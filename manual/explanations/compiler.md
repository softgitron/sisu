# How the compiler works

The `sisu` command is a compiler in stages. Stage 0 is an implementation
in Rust whose heart is an interpreter; stage 1 is a front end written in
Sisu, which is now the one the command checks, runs and builds with;
stage 2 compiles to machine code; and stage 3, a compiler that compiles
itself, is reached piece by piece. Between the front end and everything
that runs a program sits one intermediate form, SIR. This page explains
why the compiler is built in that order and around that form, what the
design buys and costs, and which questions about it are still open.

**See also:** the compiler's [architecture
notes](../../compiler/docs/architecture.md) and stage 1's
[README](../../compiler/stage1/README.md); the reference's [R16, The
sisu command](../reference/sisu-command.md); the proposal's [10,
Tooling](../../proposal/10-tooling.md#compile-speed-as-a-requirement)
and [18, Next
steps](../../proposal/18-next-steps.md#bootstrapping-the-road-to-a-self-hosted-compiler),
and decisions D-038, D-052, D-078, D-079 and D-087 to D-089 in its
[decision log](../../proposal/15-decision-log.md); the research in
[backends.md](../../research/topics/backends.md) and
[tooling-and-compile-speed.md](../../research/topics/tooling-and-compile-speed.md).

## The problem: a language has to run before it can be judged

The research behind Sisu is blunt about the order of work. Languages
that spend years designing without code that runs (Carbon, Hylo, Roc,
Pony, Vale, Austral) lose their audience before they ship, and languages
that ship a small runnable thing early (Zig, Gleam, Elixir, Go) keep it
([16, Validation plan](../../proposal/16-validation-plan.md)). Sisu's
three bets can only be tested by programs that run: people must write a
parser, a graph and a server in it before anyone knows whether ownership
without lifetimes is expressive enough. So the first implementation had
to run programs soon, check them soundly and report every mistake as a
diagnostic, and it had to be built so that it could later be replaced by
a compiler written in Sisu without changing what any program means.

## An interpreter first

The first implementation is an interpreter (D-052), for three reasons.
It gives a runnable language in the shortest time, which the research on
Carbon, Hylo, Roc and Pony names as the main factor in a young
language's survival. Compile-time evaluation needs an interpreter over
the typed program anyway, so building one serves `comptime` too. And an
interpreter is where the tools that make Sisu's claims checkable are
easiest to build: the REPL and worksheets, scripts, the ownership
monitor that re-checks every borrow while a program runs, the
counterexample mode that runs a program the checker rejected, the seeded
scheduler that explores schedules, the simulator, record and replay, and
hot replacement of functions while a program runs.

Stage 0 is written in Rust because Rust's semantics are the closest to
Sisu's: a Sisu move is a Rust move, a `Drop` is a `Drop`, a `Shared` is
a reference count, so the checker's data structures mirror a working
model and the interpreter stays honest. The interpreter borrows its
runtime from the host, the heap, strings, hash maps, sockets and timers,
which is the trick that let Elixir ship within months by sitting on
Erlang's virtual machine (18, "Implementation language and
architecture"). Roc's post-mortem of a compiler it abandoned, and Hylo's
two rewrites, supplied the rules that stage 0 follows from its first
commit: a parser that tolerates errors, no internal panic on any input,
every analysis incremental, the back end behind a stable intermediate
form, and a conformance suite run on every commit
([10](../../proposal/10-tooling.md#compile-speed-as-a-requirement)). The
workspace forbids `unwrap`, `expect` and `panic` outside tests, so a bug
in the compiler becomes a catalogued diagnostic, never a crash.

## SIR, the one boundary

Every stage of the pipeline is a total function: it takes any input,
broken or not, and returns its output and its diagnostics, so a program
with an error in one function is still parsed, resolved and type-checked
everywhere else. The front end ends in SIR, a typed graph of basic
blocks per function, in which every move, copy, borrow, drop and call is
explicit. No stage before SIR knows what will run it, and everything
that runs a program starts from it. This session runs a small program
with the interpreter and as a native executable, and then prints its
SIR:

<!-- file: examples/explanation-compiler/double.sisu -->
```sisu
# A small function, to see what the front end hands to the back ends.
def double(n: int) -> int:
    return n * 2

def main():
    print(double(21))
```

<!-- output: examples/explanation-compiler/double.console -->
```console
$ sisu run double.sisu
42
$ sisu run --native double.sisu
42
$ sisu sir double.sisu
sir 0
source 0 "double.sisu"
entry main

intrinsic print

fn double(1) source 0 @2:5-2:11 {
    let _0: int
    let _1: int "n" @2:12-2:18
  bb0:
    _0 = mul copy _1, const 2 @3:12-3:17
    return @3:5-3:17
}

fn main(0) source 0 @5:5-5:9 {
    let _0: ()
    let _1: ()
    let _2: int
  bb0:
    _2 = call fn double(const 21) @6:11-6:21
    _1 = call intrinsic print(copy _2) @6:5-6:22
    _0 = const ()
    return
}
```

Each function lists its locals, `_0` for the result and then the
parameters, and its blocks of statements, each statement with its place
in the source. The ownership checker works on this form, one function at
a time and without looking back at the source, so a front end in another
language could reuse it by emitting SIR; the interpreter runs it; and
the native backend compiles exactly the SIR that the interpreter would
run, after the checker and compile-time evaluation. SIR has a written
text form that any implementation can read and write, which is what made
the next stage possible.

## Stage 1: the front end in Sisu

The owner's goal is a compiler written in Sisu as early as possible,
with one rule: write in Sisu whatever Sisu can already run, and in Rust
only what must exist first (D-078). So the lexer, parser, name
resolution, type checker, lowering, ownership checker, formatter and
`sisu doc` were written again in Sisu, as stage 1, running first on
stage 0's interpreter and then compiled by stage 0's native backend.
Stage 1 is a transcription of stage 0's written rules, not a new design,
and it was checked byte for byte against stage 0 over the whole
conformance suite: tokens, trees, reprints, diagnostics and SIR.

That check found something worth knowing. In its first round it turned
up five places where stage 0's code contradicted its own comments or the
proposal, and in every one the document was right and the code was not;
none occurred in the suite, the standard library or the examples, so the
suite alone would not have caught them. The rule that followed is D-079:
where two stages differ, the written rule decides, and stage 0 changes
when its behaviour is an accident. A transcription that copied accidents
would have made them rules that nobody chose.

The front end has reached its fixed point: stage 1, compiled from its
own SIR, prints the same SIR again, byte for byte. That is what the
bootstrap compares, stage 1's SIR of itself with the SIR of the stage 1
built from it; the Rust front end's SIR of stage 1, which falls behind
as stage 1 gains rules, is compared too, and the difference is reported
rather than failing the build. Since November 2026
it is the front end of `sisu` (D-089), the Rust front end is frozen as
an oracle that `--front-end=rust` still runs, and a new rule of the
language is written in stage 1 alone, listed with its cases where it
departs from the oracle.

## Stage 2: machine code

The native backend's first round is Rust over Cranelift, a code
generator built for speed of compilation, with a runtime in Rust linked
into every executable. Its executables run 10 to 30 times as fast as the
interpreter. The second round replaces both (D-087). Stage 2 lowers SIR
to a low intermediate form, LIR, in Sisu; a debug backend written in
Sisu compiles LIR to x86-64 code in one pass per function, as the TPDE
backend does for LLVM's IR, and writes the object file and its debug
information itself; the runtime moves to Sisu under `unsafe`; and
release builds go to LLVM, reached by writing its bitcode and running it
as a separate program, so that no C++ and no link-time dependency on
LLVM enters the toolchain (D-088). The x86-64 backend in Sisu already
compiles programs, chosen with `--backend=sisu`. The runtime's move has
begun too: `compiler/rt`, Sisu code compiled once into every executable,
decodes the collections' operations, holds the run queues and, since
stage 0's thirteenth round, a task's record, its parking and its waking,
in the order the interpreter wakes tasks, so that a seed names the same
schedule natively; each task's coroutine, its outcome and its trace
stay in Rust beside the record, under the same number (decision 780 of
the [architecture notes](../../compiler/docs/architecture.md)). Since
the fourteenth, the thirteen operations that make an iterator or take
every item of one (`range`, `enumerate`, `zip`, `sorted`, a loop's
`iter.ref` and the rest) and the choices of the PCT and exhaustive
schedulers are Sisu's too, drawn in the interpreter's order, so a depth
and a seed, or a prefix, name the same schedule both ways (decisions
820 and 821). What stays in Rust is the decoding of most of the other
intrinsics, the tasks' switch and clocks, the checked memory model, the
layout of types and the bridge between the two halves.

The split follows the evidence. Zig's own x86-64 backend, its default
for debug builds since 2025, builds the Zig compiler in 20 seconds where
LLVM takes 75, and TPDE compiles 8 to 24 times faster than LLVM's lowest
optimisation level with code as good; the Go team turned LLVM down as
too large and slow and because it would have made changes to stack
management harder, which is what Sisu's task stacks, yield points and
hot replacement need. But an optimising backend for every target of
Sisu's tiers is beyond a team of three to five, so LLVM does the
optimising, for release builds and for targets the Sisu backend does not
cover yet ([backends.md](../../research/topics/backends.md)). The
research's warning is that the cost of an own backend lies in object
formats, linkers and debug information more than in choosing
instructions: five years in, Zig's AArch64 backend and Windows linker
were still unfinished.

## Queries, for speed of feedback

Fast feedback is fourth in Sisu's priorities, and the proposal sets
numbers for it (D-038): `sisu check` within 300 milliseconds after an
edit to one function of a 100,000-line package, a debug rebuild within a
second, a clean debug build of 100,000 lines within 10 seconds on a
laptop, and an answer from the language server within 100 milliseconds.
Several language decisions exist partly for these numbers: explicit
signatures, so that bodies are checked independently and in parallel; no
cycles between packages; no macros and a bounded `comptime`; generics
checked once against their bounds; and a backend for debug builds that
is not LLVM
([10](../../proposal/10-tooling.md#compile-speed-as-a-requirement)). The
research finds that slow compilers are made by language decisions more
than by compiler bugs: Rust's crate as the unit of compilation with
monomorphisation, derives by procedural macro and LLVM; Swift's
inference; C++'s textual inclusion and templates; and, in the other
direction, Go's acyclic imports, explicit signatures and own backend
([tooling-and-compile-speed.md](../../research/topics/tooling-and-compile-speed.md),
finding 2).

The driver memoises its work as queries: files are parsed once per text,
and a program's checking, lowering and ownership checking are reused per
item, keyed on a hash of each module's interface, so that an edit inside
a function body does not invalidate what does not depend on it. rustc
added such queries in its seventh year and Roslyn had them from the
start, and the proposal reads the difference as showing. A test edits a
body of every standard module and conformance case and checks that the
reused results equal fresh ones, so that the cache can never change an
answer. The current measurements are near the targets: stage 1, about
64,000 lines of Sisu, builds natively from a clean start in about 6
seconds on four shared cores, which puts a clean debug build of 100,000
lines close to the 10-second target; code generation is about a third of
that time, and LLVM's lowest level in its place would take such a build
to an estimated 11 to 16 seconds (D-087) ([architecture
notes](../../compiler/docs/architecture.md#compile-time-and-run-time)).

## What it costs

- **Two front ends, for a while.** Every rule existed twice until the
  Rust front end was frozen, and the oracle stays until nothing checks
  against it. Meanwhile the commands that stage 1 does not serve yet,
  such as `sisu test --std`, `sisu run --defer-borrow` and the language
  server's worksheets, run the Rust front end, which knows none of the
  rules added since it was frozen: a doc example of the standard
  library is written in the forms both front ends read. A program's
  transcripts left that list when stage 1's `sir --tests` came to keep
  the world they run in; until then a transcript of a program that used
  a newer rule failed with the Rust front end's errors (D0128).
- **The interpreter is slow.** It is fine for development, tests and an
  oracle, and not for users, which is why the native backend was built
  beside stage 1 rather than after it.
- **The interpreter's tools have no native counterpart yet.** The
  ownership monitor, the counterexample mode, record and replay, hot
  replacement and `sisu test` run on the interpreter only.
- **Self-hosting freezes early mistakes.** A compiler written in its own
  language must keep compiling through every change of the language,
  which makes early design errors expensive to undo; editions and the
  conformance suite are the mitigation. Zig treats the four years its
  self-hosted compiler ran beside its C++ one as its hardest period.
- **An own backend is a long project.** Object formats, linkers and
  debug information for every target are work that LLVM users never do,
  and the plan's order of work puts AArch64, macOS and Windows after
  x86-64 Linux.

## What would change it

Four questions from the backend research are open. Q-58 asks what seeds
the toolchain once no Rust remains: the previous release, as for Go and
Rust, a WebAssembly build of the compiler committed to the repository,
as Zig does, or the frozen stage 0 kept buildable; and whether Sisu
writes WebAssembly itself. Q-59 asks how LLVM ships, which versions the
release backend supports, and what `--release` does on a machine without
it. Q-60 asks how fast generated code must be: what LLVM buys on Sisu's
own programs, and whether the debug backend must keep values in
registers. Q-61 asks which linker the toolchain uses. Each has a
measurement named in [backends.md](../../research/topics/backends.md),
and each decides the shape of stage 2 rather than whether there is one.
The plan's stated conditions for retiring each Rust part do not move: a
part is replaced only when the native conformance runs pass with its
replacement and the fixed point holds with the replacement compiled by
itself (D-087).
