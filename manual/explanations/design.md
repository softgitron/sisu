# The design in one page

Sisu's design follows from three short lists: an order of priorities
that settles every conflict, ten principles that apply it, and the
things Sisu leaves out on purpose. This page explains each list, shows
the principles at work in one function, and says where the design could
still be wrong. The other explanations take one topic each and go
deeper.

**See also:** the proposal's [one-page
summary](../../proposal/README.md), [00, Vision and
principles](../../proposal/00-vision-and-principles.md) and [14,
Non-goals](../../proposal/14-non-goals.md); the [decision
log](../../proposal/15-decision-log.md), whose D-001 is the priority
order; the [open questions](../../open-questions.md); the [list of
explanations](../README.md#the-explanations); and the tutorial's
[chapter 1](../tutorial/01-hello.md), where the language starts.

## Why a language needs an order of priorities

Every language trades one good thing for another. A garbage collector is
convenient, and it makes destruction happen at a moment nobody chooses.
Inference across a whole program saves typing, and it reports an error
far from the mistake. Macros let a library extend the language, and they
make the language harder for tools to read. A `pure` function can be
tested with no setup, and cannot do I/O. The proposal's own question
about this, whether "only the upsides" is even coherent, is answered
with the order: the best available choice under a fixed ranking,
revisited as evidence arrives (Q-40).

The languages that tried to have everything show what happens without a
ranking. In 2018 Bjarne Stroustrup wrote of the C++ committee that
"individually, many (most?) proposals make sense. Together they are
insanity to the point of endangering the future of C++"; Scala and D
tell the same story of features accreting without a governing rule
([designer-retrospectives.md](../../research/evidence/designer-retrospectives.md)).
So Sisu ranks its goals once, in D-001, and when the evidence on a
choice is mixed, the higher goal wins:

1. **Safety.** Safe code cannot have memory errors, data races, null
   dereferences or unhandled errors.
2. **Readability.** Code is read far more often than written; a feature
   that makes code shorter to write and harder to read a year later
   loses.
3. **Predictable performance.** The cost of code is visible in the code:
   no hidden allocation, control flow or reference counting.
4. **Fast feedback.** Compile times, diagnostics and tooling are design
   constraints, not afterthoughts.
5. **Expressiveness.** Enough power to write libraries without
   boilerplate, and not so much that libraries become languages of their
   own.
6. **Interoperability.** The C ABI in both directions, WebAssembly, and
   a runtime that shrinks to nothing on embedded targets.

The order shows in decisions that would otherwise be close. Integer
overflow traps in every build mode, release builds included (D-009),
because safety outranks the few percent the check costs. There are no
user-defined operators (D-022) and no macros in version 1 (D-018),
because readability outranks expressiveness. There are no properties
(D-023), because `x.y` that runs code is hidden control flow, and
predictable performance outranks the convenience. Performance comes
third rather than first because Go's success suggests that "fast enough
and predictable" beats "fastest" for most software; it stays ahead of
expressiveness because hidden costs are the most common complaint about
Swift's reference counting and Haskell's laziness (D-001).

Three things are missing from the list on purpose. Brevity is a result
of good design, not a goal. Familiarity to users of any one language is
traded away whenever the evidence says the familiar thing is a mistake.
And novelty is avoided: each feature should have a predecessor that
showed it works.

## Ten principles

The principles of
[00](../../proposal/00-vision-and-principles.md#principles) are the
order applied to recurring kinds of choice. Most have an explanation of
their own; here is what each says and why.

**One obvious way** (P1). Where two features overlap, Sisu keeps one,
because every overlapping pair is a style debate, a learning cost and an
interaction to specify, as C++, Scala, D and Perl show. The owner added
a rule with the decision on parallel loops (D-069): a feature goes into
the standard library wherever that is reasonable, and into the language
only when a library cannot express it or check it. So parallel loops are
`std.par` and not a keyword, exact numbers are `BigInt`, `Ratio` and
`Decimal` in `std.num` (D-076), and the core stays small.

**Explicit at boundaries, inferred inside** (P2, D-003). Signatures,
public types and module interfaces are written in full, and types are
inferred inside a body. Go, Rust, Swift, Kotlin, TypeScript and Carbon
all landed here on their own, for the same three reasons: readable
interfaces, errors reported where the mistake is, and a body that can be
checked knowing only the signatures it calls, which is what makes
compilation parallel and incremental.

**Errors are values, bugs are panics** (P3, D-011). A function that can
fail says so in its type, and a broken invariant panics, as [Errors
without exceptions, absence without null](errors.md) explains.

**No ambient authority** (P4, D-015). Code reaches files, the network,
the clock and the environment only through capability values that its
caller hands it. Of the known defences against a malicious dependency,
it is the only one that the compiler enforces rather than a policy that
people follow ([Capabilities and the supply chain](capabilities.md)).

**Values by default, sharing by declaration** (P5, D-004). A variable
owns its value, a parameter borrows it, and sharing is a type you
choose, which is what makes the absence of a garbage collector tolerable
([Ownership without lifetimes](ownership.md)).

**The tooling is the language** (P6, D-029), **designed to compile
fast** (P7, D-038) and **error messages are a feature** (P8, D-065). One
binary builds, tests, formats, documents and serves the editor; compile
speed has numeric targets; every diagnostic has a stable code and, where
it can, a fix. Go, Rust, Zig and Elixir show that integrated tooling
decides a language's first day more than anything else, Elm shows that
messages can be designed like any interface, and Rust, Swift, Scala and
C++ show the cost of leaving compile speed for later ([How the compiler
works](compiler.md), [Diagnostics as a product](diagnostics.md)).

**Evolve without breaking** (P9, D-036). A specification, a
compatibility promise, editions and a migration tool, because Python 3,
Swift's first three versions and Scala 3 show what breaking costs ([How
Sisu evolves](evolution.md)).

**Steal, do not invent** (P10). Every feature has a predecessor in a
shipped language, and the decision log names it; where none exists, the
feature is an open question rather than a decision. A borrowed spelling
carries its borrowed meaning: where Sisu writes something as Python or
Rust does, it means the same or fails to compile (D-062), so a reader's
habits never mislead silently ([Loud divergence](loud-divergence.md)).

Four more explanations follow the principles into particular corners:
[Costs you can see](costs.md) takes the third priority in detail,
[Compile-time code instead of macros](compile-time.md) applies P1 and P7
to code that writes code, [Determinism](determinism.md) says what `pure`
promises and why, and [Working with unfinished code](unfinished-code.md)
shows how a program with a typed hole still compiles and runs.

## The principles in one function

This function and its tests use six of the principles at once:

<!-- file: examples/explanation-design/signature.sisu -->
```sisu
# A signature says what a function may touch, how it can fail and
# what it borrows; the body infers the rest.
def longest_line(fs: Fs, path: Path) -> int ! IoError:
    let text = fs.read_to_string(path)?
    var longest = 0
    for line in text.lines():
        longest = max(longest, len(line))
    return Ok(longest)

test "the longest line of a file":
    let fs = Fs.in_memory({"notes.txt": "sisu\nmeans grit\n"})
    assert longest_line(fs, "notes.txt") == Ok(10)

test "a missing file is an error, not a crash":
    assert longest_line(Fs.in_memory(), "notes.txt").is_err()
```

`sisu test` runs both tests:

<!-- output: examples/explanation-design/signature.test -->
```text
test "the longest line of a file" ... ok
test "a missing file is an error, not a crash" ... ok
2 passed, 0 failed, 0 blocked
```

Read the first line of `longest_line` as a caller does. `fs: Fs` says
that the function reads or writes files and touches nothing else: no
network, no clock, no environment (P4). `-> int ! IoError` says that it
can fail and how, and the `?` in the body passes a failure on, visibly
(P3). Both parameters borrow, so the caller keeps its file system and
its path (P5). Inside the body nothing is annotated: `text`, `longest`
and `line` get their types from what they are given (P2). The tests need
no mocking library: an in-memory file system is a value like any other,
passed where the real one would be ([Testing without
mocks](testing.md)). And everything here, from the test runner to the
formatter that laid out the file, is the one `sisu` command (P6).

## What Sisu leaves out on purpose

[14](../../proposal/14-non-goals.md) lists what Sisu does not try to be,
with a reason for each; a non-goal can be revisited, but only by an
explicit decision with evidence, never by drift.

Sisu is not a dynamic language, and it has no mode in which types are
optional. Dart reversed from optional types to sound ones, TypeScript's
`any` is a leading source of the run-time type errors its users meet,
and the typing retrofits of Python and Ruby are still partial (D-002),
which suggests that gradual typing is a way to migrate, not a design.
Exploration is served instead by the interpreter, scripts, worksheets
and fast checking.

Sisu is not a Python superset, though it borrows Python's surface
wherever the idiom is not a documented mistake (D-055). Mojo walked back
its claim to be one, because compatibility with a language's meaning
imports its mistakes. It is not a language for calling C++ templates and
classes either, which is Carbon's purpose.

Sisu is not a research vehicle. Algebraic effect types, dependent types,
linear types as the main discipline, code stored by content hash and
image-based live development were each found unproven at scale, and each
is deferred to an open question or a later release. The one place where
Sisu combines known ideas in a way no shipped language has, ownership
with second-class references together with capabilities and colourless
structured concurrency, is named as its largest risk.

Sisu is not a kitchen sink. It has no class inheritance, exceptions,
`null`, macros in version 1, operator definitions, implicit lossy
conversions, properties, overloading by parameter type, `goto`, union
types outside enums and error sets, or unsynchronised global mutable
variables, and each absence is a decision in the log. It is not built
for one domain: matrices, relations, actors, reactive streams, GPU
kernels and user interfaces are libraries, and where a library cannot
express one well, that is an open question for the language. Nor is it a
notebook: a worksheet reruns a whole file and shows its values, with no
cells and no hidden state, and a notebook stays out unless Q-48's
conditions are met.

And Sisu is not fastest at any cost. Safe code has no undefined
behaviour, no build mode drops bounds checks, and no flag turns off the
overflow check outside `unsafe`. The measured cost of those checks, a
few percent in Swift's and Rust's data, is accepted; code that needs the
last few percent writes `unsafe`, in a package the root trusts, where
`sisu audit` finds it.

## Where the design could be wrong

Most of Sisu combines things that a shipped language has validated.
Three combinations have not shipped together, and the proposal calls
them its bets ([README](../../proposal/README.md#the-three-bets)):

- **Ownership without named lifetimes is expressive enough.** Sisu bets
  that second-class references, views with one implicit span and
  `Shared` cover what real programs need. If people who did not design
  the language need `unsafe` for ordinary patterns, or fight the checker
  as often as in Rust, the answer is a reference type checked at run
  time or opt-in lifetime names on views ([Ownership without
  lifetimes](ownership.md#what-would-change-the-design)).
- **Capabilities as ordinary parameters are tolerable at scale.** Every
  function that does I/O takes one more parameter. If the median such
  function carries more than two, or authors bundle every capability
  into one struct, which is ambient authority under another name, the
  reserved `with Fs, Clock` clause in signatures is implemented instead
  ([16, Validation plan](../../proposal/16-validation-plan.md), bet 2;
  Q-5).
- **Colourless tasks without a garbage collector have acceptable
  stacks.** Go and Erlang move stacks because their collectors can; Sisu
  cannot. If a task needs more than four times Go's memory, or stacks
  overflow in ordinary code, task bodies are compiled to state machines
  behind the same `scope` and `spawn` (bet 3, Q-6, which is still open;
  [Concurrency without colours](concurrency.md#what-would-change-it) has
  the harness's first numbers).

The experiment of [20](../../proposal/20-pre-registration.md) fixes in
advance what counts as each result, so that the answer is not bent to
fit the design. Beyond the bets, a few questions are open by choice: the
runtime's stacks (Q-6), governance (Q-21), numerics and arrays (Q-23),
and the newer questions of the unsafe sublanguage and the native backend
(Q-54 to Q-61, Q-71 and Q-72).

The design also chooses an audience, because an audience chooses the
trade-offs
([designer-retrospectives.md](../../research/evidence/designer-retrospectives.md#synthesis-for-sisu),
point 9). Sisu is for teams writing services and tools that must keep
working, who want Rust's guarantees with Go's build times and learning
curve; for embedded and systems programmers who want safety without a
runtime; for application developers who want value semantics, optional
values and structured concurrency without a platform vendor; and for
people learning their first compiled language. A reader who ranks the
goals differently will disagree with some decisions, and the decision
log is written so that they can see exactly which.
