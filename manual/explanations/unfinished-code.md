# Working with unfinished code

A Sisu program may contain a typed hole, `todo("name")`, where code is
still to be written. The compiler checks everything around the hole and
reports what type belongs in it; the interpreter runs until it reaches
the hole; and a test that reaches it ends neither passed nor failed but
blocked. This page explains why holes are part of the language rather
than a placeholder function, how the manual's exercises are built on
them, what the report and the fill engine are designed to do, what the
design costs, and what would change it.

**See also:** the tutorial's [chapter
1](../tutorial/01-hello.md#how-exercises-work) (how exercises work) and
[chapter 3](../tutorial/03-functions.md) (typed holes in your own code);
the guide [Plan with typed holes](../guides/holes.md); the reference's
[R16, The sisu
command](../reference/sisu-command.md) and [R17,
Diagnostics](../reference/diagnostics.md#w0001); the proposal's [17,
Testing](../../proposal/17-testing.md#assertions) and N-27 of [19, Novel
features](../../proposal/19-novel-features.md), and decisions D-061 and
D-057 in its [decision log](../../proposal/15-decision-log.md); the
research in
[holes-synthesis-and-ai-era.md](../../research/unconventional/holes-synthesis-and-ai-era.md).

## The problem: unfinished code has nowhere to go

Programs are written from the outside in: a function's signature and its
tests before its body, half a module before the rest. Most languages
offer unfinished code two poor choices. Leave the code out, and the
program does not compile, so nothing else in it can be run or tested. Or
put in a placeholder, Rust's `todo!()`, Scala's `???`, Python's
`raise NotImplementedError`: a call that fits any type and says nothing
about what belongs there. A test that reaches one fails as if it had
found a bug, and nothing stops the placeholder from shipping.

Languages with dependent types made the hole the centre of how
programmers work. GHC has reported the type of a hole since 2014 and,
since 2018, the names in scope whose types fit it. Idris writes `?name`,
shows each hole's context and expected type in the editor, and teaches a
loop of "type, define, refine" with the type checker as an assistant;
Agda and Lean work the same way. Hazel goes further and lets evaluation
proceed around a hole, keeping the values in scope each time it is
reached
([holes-synthesis-and-ai-era.md](../../research/unconventional/holes-synthesis-and-ai-era.md),
ideas 1 and 2). The evidence for holes is designers' accounts and
imitation, not a controlled study. One mainstream transplant went
poorly: rust-analyzer's search for expressions that fit a `todo!()` was
little used, and the research reads that as saying that the report on a
hole matters more than enumerating what could fill it.

## What a typed hole is in Sisu

`todo[T](name: str = "") -> T` is a compiler intrinsic, as `assert` is
(D-061). Its type is whatever the place around it needs, so the code
around a hole type-checks, and the compiler can say what the hole must
be. This file has one function finished and one with a hole, and a test
for each:

<!-- file: examples/explanation-unfinished-code/median.sisu -->
```sisu
# A program with a hole: it type-checks, its tests that reach the hole
# are blocked rather than failed, and the rest of it runs.
def median(xs: list[int]) -> int:
    let sorted_xs = sorted(xs)
    return todo("the middle element of sorted_xs")

def mean(xs: list[int]) -> int:
    return sum(xs) // len(xs)

test "the mean of three":
    assert mean([3, 1, 2]) == 2

test "the median of three":
    assert median([3, 1, 2]) == 2
```

<!-- output: examples/explanation-unfinished-code/median.console -->
```console
$ sisu check median.sisu
warning[W0001]: typed hole `the middle element of sorted_xs` of type `int`
  --> median.sisu:5:12
  |
5 |     return todo("the middle element of sorted_xs")
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

$ sisu test median.sisu
warning[W0001]: typed hole `the middle element of sorted_xs` of type `int`
  --> median.sisu:5:12
  |
5 |     return todo("the middle element of sorted_xs")
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "the mean of three" ... ok
test "the median of three" ... blocked by todo("the middle element of sorted_xs")

test "the median of three" ... blocked by todo("the middle element of sorted_xs")
what the test printed:
  in scope at the typed hole `todo("the middle element of sorted_xs")`:
    xs = [3, 1, 2]
    sorted_xs = [1, 2, 3]

1 passed, 0 failed, 1 blocked
```

W0001 (`type.hole`) is a warning, not an error, so the program still
runs: the finished function's test passes, and the test that reaches the
hole is reported as blocked by it. After the list, the blocked test's
block shows the hole's closure, the bindings in scope where the test
reached it, here `xs` and `sorted_xs`: what the missing code has to
work with. Blocked is the third outcome of a test (D-057), and it is what separates "not written
yet" from "wrong". A suite written before its code says which hole each
test is waiting on, and its last line says how far the work has come. A
program that reaches a hole while it runs prints the same closure and
stops there, as a panic does. A placeholder that is meant to ship is
spelled `panic("...")` instead, so that the two never look alike, and
the design closes the loop: a release build rejects each hole (E0445),
and `sisu publish` is designed to, so blocked is never a state that
ships ([17](../../proposal/17-testing.md#assertions)).

The spelling was a choice. The research proposed `?name` in an operand's
place, as Idris, Agda and Lean write it, and retiring `todo`. The owner
kept `todo("name")` (Q-46): `?` already means propagation, and `T?`,
`?.` and `??` use it too, one form of hole is enough, and an intrinsic
adds no new lexical form and leaves nothing to migrate.

## Holes as exercises

Every exercise in this manual is a file with holes and the tests that
judge it. As handed to the learner it compiles and every test is
blocked; the learner fills the holes until every test passes, and never
edits the tests. Chapter 1 explains the loop, and `tools/check.py`
verifies each exercise's start state.

The design follows the learning research. Worked examples help novices
more than solving problems from scratch, and completion problems, in
which learners modify and extend programs that are already written, beat
writing new programs in van Merriënboer's study of program construction
([manuals.md](../../research/topics/manuals.md), finding 3). A typed
hole is a completion problem by construction: the code around it is the
worked part, and it type-checks before the learner writes anything, so
the first thing a learner sees is a program that works except where they
are asked to work.

## The report and the fill engine to come

Two tools build on holes, and neither is in stage 0 yet. The first is a
report. A run and a blocked test already print a hole's closure, the
values in scope each time it is reached, which is Hazel's idea; `sisu
check --holes` is designed to print, for every hole at once, the
expected type and the definitions of the types it mentions, the bindings
in scope with their conventions and capabilities, the enclosing
`requires` as facts and `ensures` as obligations, the tests that reach
it with the closures they reached it with, and the expressions in scope
that fit, to a depth of two, as GHC lists them. Stage 1, the compiler
written in Sisu, has its first part, each hole with its type, its
function, the bindings in scope and those of its type, and the driver
does not run it yet. It is meant for people,
as a language-server query, and for programs that write code, in JSON. The last audience is not an
afterthought: in a study of filling holes in Hazel, giving a
code-writing model what the checker knew about a hole helped it far more
in Hazel, the language with the least training data, than in TypeScript,
and the definitions of the types involved helped more than anything
else. Sisu is in Hazel's position (idea 7).

The second is `sisu fill`, tooling after version 0.1. It asks proposers
for candidates: a built-in enumerator, or any external process that
speaks its JSON protocol, with no model or vendor built in, so that the
one binary stays neutral (D-029). It formats and deduplicates the
candidates, checks their types and ownership, and runs them in the
interpreter against the tests that reach the hole, the function's
contracts turned into properties, and the property tests that call it,
all on fakes. A failing property's shrunk counterexample goes back to
the proposer; the candidates that survive are told apart by inputs on
which they differ; and a person chooses among them by their behaviour
and reads the code.

The hard part is the judge, and the research is specific about why. A
passing test suite is weak evidence: in a manual sample of PropR's
repairs of student programs, from 63% to 85% of the patches that passed
the tests were overfitted, and on a widely used benchmark of agents'
patches to real projects, stronger tests rejected about a fifth of the
patches that had passed (idea 6). Code generators also game their judge
when they can: on tasks whose tests contradicted their specification, so
that any pass was cheating, one leading model passed more than half.
Read-only tests stopped models editing the tests but not special-casing
the inputs; hiding the tests cut cheating to almost nothing but hurt
honest work; and an explicit way to answer "this cannot be done" cut it
from more than half to under a tenth. So the design keeps the judge out
of the filler's reach. A fill changes only the hole's span; tests,
contracts, generators and snapshots are read-only while it runs; every
name must resolve in the lockfile, since models invent packages that do
not exist; a candidate that compares an input with a literal taken from
a test is flagged as special-casing; mutants of the fill that the tests
do not kill are reported; and `unfillable` with a reason is a legal
answer.

## What it costs

- **A hole is a warning, so it runs until it is reached.** A hole in a
  branch that tests never take survives until that branch runs. The
  release gate is what catches it: `sisu build --native --release`
  refuses a program with a hole (E0445), which a debug build only warns
  about.
- **Blocked is not passed.** `sisu test` exits with status 3 when a test
  is blocked and none fails, 1 when one fails and 0 when all pass, so a
  pipeline that reads only the status can tell a suite full of holes
  from a finished one. A pipeline that treats every status but 0 as a
  failure has to learn the 3, or it reads an unfinished plan as broken.
- **The report is partial.** A closure leaves out a local that holds a
  borrow or a view, a value whose type writes its own `Debug`, and one
  that holds a capability, so it can miss the value that matters, and
  `sisu check --holes`, the report of every hole at once, waits for
  the driver to run stage 1's first part of it.
- **`todo` looks like a call.** It is less visible in a page of code
  than Idris's `?name`, which is the price of not adding a lexical form.
- **A fill engine can make overfitting cheap.** The safeguards in "The
  report and the fill engine to come" reduce it, and none removes the
  need for a person to read what was filled.

## What would change it

The typed hole, its report and the release gate are decided for version
1 (D-061, with Q-46 for the spelling), and the fill engine is tooling
whose shape is a verdict with medium confidence (N-27). It is settled by
a study of how often fills are accepted, once `sisu fill` exists after
0.1. Two of this manual's open questions touch holes as well: where the
exercise runner lives (Q-63), and how hidden the solutions and tests are
and when the runner reveals a solution (Q-65), since a filler that can
read the solutions turns every exercise into a lookup.
