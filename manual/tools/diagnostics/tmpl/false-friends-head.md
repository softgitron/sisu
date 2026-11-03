# False friends

Where Sisu spells something as Python does, or as Rust does for syntax
the two share, it means the same or fails to compile (D-062). A false
friend is a spelling whose familiar meaning Sisu does not have; this page
lists them, each with the check that makes it loud, and the few where
Sisu's meaning differs without one. People and models read a familiar
spelling with its familiar meaning first, and Python 3's silent change
to `/` is the cautionary tale.

**See also:** the tutorial's "From Python" notes, in
[chapter 1](../tutorial/01-hello.md) onwards; the proposal's
[02, "Departures from Python, with reasons"](../../proposal/02-syntax.md),
[13](../../proposal/13-evolution-and-governance.md), which keeps the table
in the specification, and the decision D-062; [R17,
Diagnostics](diagnostics.md) for the codes.

## How to read this page

Each row shows a program written with the habit and what `sisu check`
says about it, or, for a spelling that compiles, what the program
prints. The fix is the Sisu spelling the row names. Every example ran
when the manual was last checked, with stage 0, the compiler in this
repository.
