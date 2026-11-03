# Diagnostics as a product

Sisu treats its compiler's messages as a product with users, tests and a
specification of their own. Each diagnostic has a stable code, an
identifier with parameters taken from a catalog, a place in the source,
and where it can, a fix that says what it changes. This page explains
why messages get that much design, what the evidence does and does not
show, how the catalog and the corpus of wrong programs work, what the
approach costs, and what would change it.

**See also:** the tutorial's [chapter
1](../tutorial/01-hello.md#when-something-is-wrong), which explains the
parts of a diagnostic; the reference's [R17,
Diagnostics](../reference/diagnostics.md), with a checked wrong program
for every code; the proposal's [10,
Tooling](../../proposal/10-tooling.md#diagnostics) and decision D-065 in
its [decision log](../../proposal/15-decision-log.md); the compiler's
[architecture
notes](../../compiler/docs/architecture.md#diagnostics-and-the-message-catalog);
the research in
[end-user-and-live.md](../../research/ideation/end-user-and-live.md),
ideas 8 and 13, and [studies.md](../../research/evidence/studies.md),
section 5.

## The problem: the interface people meet most

A programmer meets a compiler's messages more often than any other part
of a language's tooling, and a learner meets little else in the first
weeks. Elm's "Compiler Errors for Humans" (2015) started the modern
effort to design them, and Rust's diagnostics work followed. After a
decade of it, 9.1% of Rust's users call unclear compiler errors a big
problem and 52.4% say they are not an issue, while Julia's users have
put confusing or verbose error messages among their least liked features
at 22% to 36% since the question was added
([surveys.md](../../research/evidence/surveys.md)).

The evidence is honest about its shape. A review of some 300 papers on
error messages found the controlled results mixed: one study of enhanced
messages for novices found no measurable effect on their error rates,
another found fewer repeated errors, a third found no effect
([studies.md](../../research/evidence/studies.md), section 5). Design
principles such as context, specificity and showing the fix are
supported qualitatively, and large effects on learning are unproven. Two
findings are sharper. Marceau, Fisler and Krishnamurthi found that
students often misread DrRacket's carefully designed messages, largely
over vocabulary: the words, not the layout, failed them
([end-user-and-live.md](../../research/ideation/end-user-and-live.md),
idea 13). And Brown and Altadmri found that educators agree only weakly
on the most common novice mistakes and match the data of over 100,000
students only moderately, however experienced they are
([manuals.md](../../research/topics/manuals.md), finding 6), so the
mistakes worth a good message should come from data, not from a writer's
guess.

Sisu draws three conclusions (D-065). Messages are a satisfaction
feature that is cheap beside a language feature, so they get designed
and tested like any interface. Their words must be changeable without
touching the compiler, because the words are what readers misread. And
the mistakes they cover are collected, not imagined.

## What a diagnostic is made of

A Sisu diagnostic is data before it is text. It has a code, which is
stable and never reused; an identifier with named parameters, which
picks a template from the message catalog; a primary place in the source
and labelled secondary places; notes; and fixes, each a message with the
edits that apply it. This session shows one diagnostic in its human form
and in its form as data, and then asks for its code:

<!-- file: examples/explanation-diagnostics/moved.sisu -->
```sisu
# A list used after it was moved: one mistake, one diagnostic.
def main():
    let guests = ["Aino", "Eero"]
    let sauna = guests
    print(guests)
    print(sauna)
```

<!-- output: examples/explanation-diagnostics/moved.console -->
```console
$ sisu check moved.sisu
error[E0400]: `guests` was moved on line 4 and cannot be used afterwards
  --> moved.sisu:5:11
  |
5 |     print(guests)
  |           ^^^^^^
  |
4 |     let sauna = guests
  |                 ------ `guests` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

$ sisu check --format=ids moved.sisu
5:11-5:17 E0400 borrow.use-after-move name="guests" line="4"
  label 4:17-4:23 borrow.moved-here name="guests"
$ sisu explain E0400
E0400 (error) borrow.use-after-move: `{name}` was moved on line {line} and cannot be used afterwards
```

The human form is what a reader sees. The second form is what the
conformance suite compares, and what an editor or an agent reads: the
code, the identifier `borrow.use-after-move`, the parameters `name` and
`line`, and the label's place. The third is the catalog's template. The
English sentence can be reworded, or translated, without changing the
code, the identifier or any test that pins them, which is why the manual
names a diagnostic by its code and identifier, E0400
(`borrow.use-after-move`), and never relies on its wording.

The implementation enforces the split. Every message that Sisu prints,
diagnostics, notes, fixes, run-time panics and the driver's own
messages, is a line of the catalog, and the compiler's code holds no
English: each identifier becomes a constant generated from the catalog,
so a message that is not in the catalog does not compile ([architecture
notes](../../compiler/docs/architecture.md#diagnostics-and-the-message-catalog)).
Templates avoid number agreement, so the catalog needs no plural rules
yet. The cost of a catalog is small at the first diagnostic and large to
retrofit, which is why it came first
([end-user-and-live.md](../../research/ideation/end-user-and-live.md),
idea 8).

Two more rules keep messages honest. A mistake is reported once: when
`x` has an unknown type, the uses of `x` are not reported again, and two
identical diagnostics from one piece of source are printed as one. And
the summary names the thing that is wrong, not the phase of the compiler
that noticed it.

## Fixes that say what they cost

Where Sisu can tell what to write instead, the diagnostic says so, as a
fix whose edits an editor can apply or as a note. The proposal commits
to two kinds in particular. A borrow error names the fix, because the
research on Rust's learning curve is specific that the explanation of
borrow errors improved over time, not the rule. A missing trait lists
the candidates, so that `Point` with every method of `Display` gets the
fix `impl Display for Point`.

When there is more than one way out, D-065 asks for a line per fix that
says what it costs: "`add_user(mut team, roni.copy())` keeps `roni`
usable and gives `team` its own copy". That answers Marceau's finding
directly, since a reader who misreads the message can still read what
each choice does. Two richer designs were rejected: a dialogue in which
the compiler asks which outcome the reader wants, which teaches choosing
an option without understanding it, and a generator that writes prose
about what code does, which would rot and duplicates what other tools
already do (idea 13).

The consequence lines are designed and not yet built as a part of each
fix. E0400 in the session says in its note what each of its three ways
out costs: a copy, a later move, and a parameter that borrows instead of
taking with `own`. The tutorial says the same in prose, and the plan's
findings record what other diagnostics still lack.

## Tested like any interface

A message that nothing tests drifts, so Sisu's are tested three ways.
The compiler's corpus of wrong programs holds programs that each carry
one intended mistake. For each one the checker must report exactly one
error, the error must name its fix, and applying a fix that has edits,
with the fixes that follow from it, must leave a program with no errors.
The programs whose mistake is a rule of the ownership model form a slice
of their own, sixty-two of them, and twelve shapes of rustc's own
borrow-checker tests are rewritten in Sisu beside them ([architecture
notes](../../compiler/docs/architecture.md), decision 263). The
conformance suite pins each case's diagnostics by identifier. And when
stage 1, the front end written in Sisu, was built, it had to reproduce
the Rust front end's diagnostics byte for byte on the whole corpus
before it replaced it (D-078, D-079), so a message is part of what a
front end is checked against.

This manual is the third test. Every diagnostic a page shows comes from
a session that `tools/check.py` runs, every code has an entry in R17
with a wrong program and its fix, and a writer who meets an unhelpful
message shows it as it is and adds it to the plan's list of findings.
That list has grown with every chapter: a fix that leads to the same
error again, a note that names a remedy for text where the value is a
capability, a habit from Python that gets a generic parse error. The
list is read: seven of stage 1's rules (R10 to R16 in its
[RULES.md](../../compiler/stage1/RULES.md)) answer findings from it,
from naming every pattern that a `match` misses to the note that sends
Python's `open` to `Fs`, which [Capabilities and the supply
chain](capabilities.md) shows. The corpus that decides which messages
matter most will come later from readers, through the playground's logs,
with their consent (Q-68).

## Explaining what the compiler worked out

Some errors are about something the reader never wrote: an inferred
error set, a type that is not `Send` because of a field three structs
down, a function that is not `pure` because of a call it makes. For
these the design adds explanations of what the compiler inferred.
`sisu explain-borrow FILE:LINE` already shows the borrows live at a
line, and `sisu run --defer-borrow` runs a rejected program under the
ownership monitor to show whether the conflict happens. `sisu why`,
designed for error sets, `Send` and `Sync`, `pure`, the reach of
capabilities, impl selection and compile-time fuel, prints the chain of
facts behind a conclusion. Its precedents measured large effects: with
Argus, participants found 2.2 times as many trait-inference faults, 3.3
times faster, and the Whyline cut debugging time by nearly a factor of
eight
([ai-era-and-evolution.md](../../research/ideation/ai-era-and-evolution.md),
A9). The design also lets a library that builds types at compile time
say how they print in a diagnostic, through `@diagnostic_name`, so that
a units error shows metres per second and not an exponent vector.

## What it costs

- **Every message is work.** A new diagnostic means a catalog entry, a
  wrong program, an entry in R17 and a review of its words, and later a
  translation for each language (Q-69).
- **A fix that misleads is worse than none.** The corpus checks that a
  fix with edits fixes, but a note cannot be checked that way, and the
  manual's findings record notes that send a reader into the same error
  again, such as E0407's `.copy()` for a borrowed result, or that offer
  `.to_lowercase()` for a `str` where a `String` is expected, which
  changes the text.
- **Stability constrains.** A code is never reused, so splitting one
  diagnostic into two takes a new code, and the tests and pages that pin
  a message's human form must all be blessed again when its words
  change, which a reviewer then reads as a diff (Q-74 is one such change
  waiting).
- **The evidence does not promise learning.** The investment is made
  because messages are cheap beside language features, because
  satisfaction surveys reward them, and because tools read them as data.
  It is not made on a measured cut in novices' errors, which no study
  has shown at scale.

## What would change it

D-065 is leaning, and most of its parts are tested by data that is still
being collected. Idea 13 names the test of consequence lines: a review
of the diagnostics corpus, and a comprehension test of messages with and
without them. Where `sisu explain` keeps its long text, in the manual or
in the catalog, is Q-66; today it prints the catalog's line only. And
diagnostics are evidence about the language as well as about their own
words. The validation plan reopens Q-9, the choice of explicit `return`
over a block's last expression, if "expected `()`, found `T`" becomes a
top-ten diagnostic in the corpus
([16](../../proposal/16-validation-plan.md#then-in-parallel)); the same
frequency counts, from the experiment and later from the playground,
decide which messages are rewritten first.
