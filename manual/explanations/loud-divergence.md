# Loud divergence

Where Sisu spells something the way Python does, or the way Rust does
for syntax the two share, it means the same thing, or the program does
not compile. A difference that compiles and runs with another meaning
counts as a bug in the design. This page explains where that rule comes
from, how it shaped the language, where it bends, what it costs, and how
it is measured.

**See also:** the reference's [R19, False
friends](../reference/false-friends.md), which lists every row with its
check; the tutorial's "From Python" notes, from [chapter
1](../tutorial/01-hello.md) onwards; the guide [Coming from
Python](../guides/from-python.md); the proposal's [00, Vision and
principles](../../proposal/00-vision-and-principles.md#p10-steal-do-not-invent),
the departures table of [02](../../proposal/02-syntax.md) and
[13](../../proposal/13-evolution-and-governance.md), which keeps the
table in the specification; decisions D-055 and D-062 in the [decision
log](../../proposal/15-decision-log.md); the research in
[python-idioms.md](../../research/topics/python-idioms.md) and
[ai-era-and-evolution.md](../../research/ideation/ai-era-and-evolution.md),
idea A11.

## The problem: readers bring meanings with them

Sisu borrows Python's surface on purpose. `def` with `name: Type`,
indentation and `:`, `elif`, `and`, `or` and `not`, `in`, half-open
slices, `range`, comprehensions, `match` and `case`, and Python's format
specifications all come from Python, because they are the forms that the
largest population of programmers already reads (D-055). That is a gift
to a reader, and a risk: a reader who sees a familiar spelling reads its
familiar meaning first, and keeps reading it until something says
otherwise.

Python itself is the cautionary tale. PEP 238 changed the meaning of `/`
on integers between Python 2 and 3 while keeping its spelling, so that
`7 / 2`, which had been 3, became 3.5. A program that ran under both
versions could give different answers under each, with nothing to say
so, and finding those places by reading was the work of Python 3's long
migration, whose overlap with Python 2 grew to twelve years
([python.md](../../research/languages/python.md)). A spelling that
changes meaning silently does its damage exactly where nobody looks.

A newer reader makes the problem sharper. Code generators read a
familiar spelling the same way people do. The research finds that models
default to Python's meaning for Python's spelling, drift between similar
languages, and, as agents in a language they do not know, prototype in
Python first
([ai-era-and-evolution.md](../../research/ideation/ai-era-and-evolution.md),
A11). Sisu looks like Python and has almost no code of its own for a
model to have learned from, so a silent false friend would be its most
common generated bug.

## The rule

D-062 states the rule in three parts. Where Sisu spells something as
Python or Rust does, it means the same, or it fails to compile. A
failure at run time is acceptable only where no check at compile time
can exist. And a silent difference is a bug in the design, not in the
program. The specification keeps the list of such spellings, the
false-friend table, with a check for each row
([13](../../proposal/13-evolution-and-governance.md)).

So a familiar spelling has one of three fates. Most mean what they mean
in Python: `and` and `or`, slicing, `//`, which rounds towards minus
infinity, `%`, which takes the sign of the divisor, and a `dict`, which
iterates in the order its keys were inserted. Some are refused at
compile time with a diagnostic that names the Sisu spelling. A few can
only fail at run time, because no check could see them earlier: an `int`
has 64 bits, and arithmetic that passes them panics where Python's
integer would grow.

This program is written with two Python habits, read with Python's
meanings, and Sisu refuses both:

<!-- file: examples/explanation-loud-divergence/habits.sisu -->
```sisu
# Two of Python's spellings, each read with Python's meaning: Sisu
# refuses both and says what to write.
def main():
    let scores = [7, 9, 4]
    let mean = sum(scores) / len(scores)
    print("the last score is {scores[-1]}, the mean {mean}")
```

<!-- output: examples/explanation-loud-divergence/habits.console -->
```console
$ sisu check habits.sisu
error[E0301]: `/` divides floats; for integers use `//`, or convert to `f64` first
  --> habits.sisu:5:16
  |
5 |     let mean = sum(scores) / len(scores)
  |                ^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: use `//` for integer division: `//`

error[E0344]: a constant negative index does not compile; count from the end with `^`: `scores[^1]`
  --> habits.sisu:6:38
  |
6 |     print("the last score is {scores[-1]}, the mean {mean}")
  |                                      ^^
  = fix: count from the end with `^`: `^1`

```

E0344 (`type.constant-negative-index`) refuses `scores[-1]` and writes
the Sisu spelling, `scores[^1]`, "the last element of scores". E0301
(`type.integer-division`) refuses `/` between integers, because in Sisu
`/` divides floats only, and its message names both ways on: `//` for
integer division, or a conversion to `f64` first. Its fix writes `//`,
which for a mean gives 6, not Python 3's 6.67. The rule makes the
difference visible, and it puts the decision in front of the reader,
where a silent change would have made it for them.

## How the rule shaped the language

The rule decides what happens to a Python spelling that Sisu does not
keep. Such a spelling is removed, not given a new meaning. There is no
`is`, no truthiness (D-025), no `a if c else b`, no `:=`, no `else` on a
loop, no `lambda` and no `try`, and each of them fails to parse or gets
a diagnostic. Where Sisu needs the idea, it gets a spelling of its own:
`^1`, taken from Nim, counts from the end; `a ?? b` gives a default
where Python writes `a or b`; `if c: a else: b` is the conditional
expression. A reader who has never seen `^1` has to look it up, and a
reader who has seen `-1` cannot misread it.

Negative indices show why a spelling is dropped rather than copied.
Python's `a[-1]` is convenient, and its documented objection is a
computed index that turns negative and quietly reads from the other end
([python-idioms.md](../../research/topics/python-idioms.md), finding
13). Sisu leaves the meaning out, and to stay loud it does two things: a
constant negative index is a compile error with the fix `^1`, and a
computed one panics.

The rule also reversed a decided point. D-049 made `dict` iteration
order random in each process, so that no program could come to depend on
an order by accident. But Python's dictionaries have kept insertion
order since 3.7, so a reader's Python meaning would have been wrong
silently, and differently on each run. D-074 changed it to insertion
order, answering Q-42, and kept the keyed hash that resists flooding
attacks, which is the split Python 3.7 made too.

Rust shares fewer spellings with Sisu, and they get the same treatment:
`let mut`, `&&`, `||` and `!`, `x as f64` and `.clone()` do not compile.
Where Python and Rust disagree about a spelling they share, Sisu takes
Python's meaning, so `-7 % 3` is 2, as in Python, where Rust gives -1.
That is the one row of the table with no check, and the corpus
translated from Rust is where it is watched.

## Where the rule bends

A few of Python's spellings compile in Sisu and mean something else on
purpose, and R19 lists them ([Python habits whose meaning Sisu
changes](../reference/false-friends.md#python-habits-whose-meaning-sisu-changes)).
A default argument is computed at each call, where Python computes it
once and shares a mutable default between calls; and each iteration of a
loop binds its own variable, a change Go made in 1.22 for the same
reason, where a Python closure sees the variable's last value. In each
case Python's meaning is a trap that its own documentation warns about,
and a closure that would see the late binding gets a compile error in
Sisu rather than a different value. The exception is narrow: a program
that relied on the trap, such as a cache kept in a mutable default,
compiles and silently loses its cache. The design drops Python's meaning
there because it is a documented mistake (D-055). By the rule's own
words these are silent differences, and neither is yet a row of the
table that the experiment measures.

Integer overflow is the other bend. Python's integers never overflow, so
code that counts past 2 to the 63rd compiles in Sisu and panics when it
runs. That is loud, but late, and it is the price of a fixed-width `int`
(D-009, D-035); `BigInt` is the type that grows.

## What it costs

- **Every row needs a check, and a good one.** A habit that fails to
  compile with a message that does not name the Sisu spelling is loud
  and still unhelpful. Of the two dozen Python habits that R19 lists as
  not compiling, about half get a diagnostic that names the Sisu way;
  `lambda`, `class`, `try`, `x is None`, `f"..."` and Python's method
  names, among others, get a generic error today. Those are in the
  manual's findings for the compiler's maintainers, and [Diagnostics as
  a product](diagnostics.md) explains why the gap matters.
- **A spelling once given away is gone.** Sisu cannot later give
  `xs[-1]` a meaning of its own, because readers would bring Python's to
  it. Rows added after version 1 go through an edition, so that the
  change is announced and migrated rather than silent (D-036, D-062).
- **Two languages' meanings bind the design.** Python and Rust keep
  changing, and a new meaning in either can make a Sisu spelling a false
  friend that was not one before.
- **The familiar idiom is refused at the moment it is reached for.**
  `xs[-1]`, `if xs:` and `a if c else b` are among the first things a
  Python programmer writes, and each is a compile error with a lesson.
  The design bets that one diagnostic early costs less than one silent
  difference late.

## What would change it

D-062 is leaning, and it is the rule of the experiment's model arm,
which measures it row by row
([20](../../proposal/20-pre-registration.md), section 6.9). Models write
the same tasks in Sisu and in Python, and for each row of the table the
arm counts the programs that compile but fail hidden tests because of
that row: a failure is attributed to a row when replacing the row's
construct with its Python meaning, or Rust's for `%`, makes the test
pass. The arm also reports how often each row's own diagnostic fired,
which shows whether the loud rows work. A row whose rate is measurable
needs a check, a rename, or an edition; the `dict` row is expected to be
empty and is measured to confirm it. Rows are added as people and models
find them, from the manual's "From Python" notes and the findings of its
writers among others, and none is removed; the two bends of "Where the
rule bends" are the first candidates.

The table is also where the rule would be judged too strict. If the
refused spellings cost more than they save, it would show as a compile
rate much lower than the same model's rate in Python, through the
table's own rows, and the count of each row's diagnostic separates a
rule that is too strict from a message that does not teach.
