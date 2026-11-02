# The rules stage 1 decides

Stage 1 is Sisu's front end now. The Rust front end (`sisu-lexer`,
`sisu-parser`, `sisu-resolve`, `sisu-typeck`, `sisu-lower`,
`sisu-borrowck` and the front-end commands of `sisu-driver`) is frozen and
will be retired, and a rule of the language changes in stage 1 only.
Stage 0's interpreter and native backend stay, as the oracles of what a
program does: they run SIR (`sisu run --from-sir`, `sisu build --native
--from-sir`, decision 212), whichever front end wrote it.

This file lists every rule where stage 1 intentionally differs from the
frozen Rust front end: the rule, why the documents call for it, what
stage 1 does, the cases that show it, and the decision text for
docs/architecture.md ("Decisions the documents did not make"), which the
coordinator copies there under the number given. DISCREPANCIES.md lists
the other kind of difference, stage 0's accidents where stage 1 follows
the documents (none now).

Where a rule differs, the classes of `check.sh` that compare stage 1 with
stage 0 (`types`, `sir`, `ownership`, `contracts-off`,
`contracts-preconditions`, `human`, `cases-check`, `cases-human`, the
`corpus-` classes, `fmt`, `fmt-check`, `doc`, `doc-output`) differ too,
for the programs the rule touches. `check.sh` accepts those differences
as it accepts DISCREPANCIES.md's: under each rule, a line ``- Differs:
`path`: outputs`` per case it changes, the path from the repository root
and the outputs among `tokens`, `tree`, `trivia-tree`, `names`, `types`,
`sir`, `ownership`, `contracts-off`, `contracts-preconditions`, `ids` (all
of what `sisu check --format=ids` prints), `human` (what `sisu check`
prints), `fmt`, `fmt-check`, `doc` and `doc-output`. Any other difference
fails the check. What a program does is checked against the conformance
suite's expectations instead, by the class `behaviour`, the primary
oracle: every runnable program of the corpus and of stage 1's cases, a
rule's own among them, lowered by stage 1 and run from its SIR by the
interpreter and natively, must print its `.out` and `.err` and exit with
its `.status`. Where no rule changed, stage 0's SIR stays an oracle too,
through the classes `sir` and `corpus-sir`.

## R1. What a channel carries is `Send` (decision 269)

**The rule.** A channel takes what one task sends to another, which may
keep it after the sender's frame is gone, so what it carries is `Send`,
as the bound asks it:

- `channel[T](...)` asks `T: Send`, and so does a send of a value of a
  type parameter (`Sender.send` and `try_send`, in a `select` arm too).
  A type parameter meets it by its bounds, so a generic function makes a
  channel of its type parameter:

  ```sisu
  def pipeline[T: Send + Ord](own xs: list[T], workers: int) -> list[T]:
      let tx, rx = channel[T](capacity=2)
      let results = Mutex(list())
      ...
  ```

  Without the bound it is `E0347` (`type.bound-not-satisfied`, "`T` does
  not implement `Send`"), with the fix `type.add-bound` ("bound `T` by
  `Send`"), at the channel or at the send, once a body for each
  parameter; for a type that holds parameters (`list[T]`) it names the
  first that is not `Send`, when bounding them makes the type `Send` (a
  `(T, str)` borrows whatever `T` is, and is told to own what it holds
  instead).
- `Send` as a bound asks the type to own what it holds, besides being
  `Send` by its parts (06, "Send, Sync, and shared state"): a view is not
  a `Send` type argument (`keep("a view")` for `def keep[T: Send]` is
  `E0347`, with the note `borrow.own-instead`). So a type parameter
  bounded by `Send` holds no loan, as one bounded by `Escapable` (decision
  257): its values may be kept in a `Mutex`, a `Shared` or a channel
  without the ownership checker's view rules. What a task captures is
  still checked by the parts of its type alone: a task of a scope may hold
  a view of what outlives the scope (docs/architecture.md, "Send, Sync and
  what a task captures").
- A concrete type that is a view or must-consume is refused at the channel
  as decision 230 refuses it (`E0421`, `E0422`); another that is not
  `Send` is `E0347` (`channel[Shared[Cell[int]]]`: tasks would change the
  cell at once).
- A channel may carry closures. A function type is `Send` when its
  closure is, which the type does not say (Q-52: what a closure captures
  never enters a function type), so each closure is checked where it is
  sent, as a task's closure is where it is spawned: a send on a channel of
  a function type takes a closure written at the call (`E0425`,
  `task.closure-literal`, as for `spawn`, with the note
  `task.write-closure`), which owns what it captures (a plain closure that
  captures anything borrows it: `E0421` with the note
  `borrow.use-own-closure`; one that captures nothing borrows nothing),
  and each capture owns what it holds (`E0421`, the note naming the
  capture) and is `Send` (`E0426`, `task.not-send`). The halves of a
  channel of a function type are then `Send` and `Sync` and own what they
  hold, so workers take jobs from one:

  ```sisu
  type Job = () -> int

  def submit(tx: Sender[Job], k: int):
      tx.send(own () -> k * k).expect("the workers outlive the sender")
  ```

  A function type is still no `Send` type argument: a generic body could
  send any closure of it, which nothing would check.

**Why.** The owner asked for a nice way to allow generic channels, which
decision 230 refused: it checks what a channel carries as what a `Shared`
owns (a channel's receiver may keep what was sent after what it borrows is
gone: the checker had accepted `tx.send("a{i}")` on a `channel[str]`, and
a native run read the freed text), and a type parameter and a function
value count as views there (known over-rejections 13 and 14), so no
generic function could make a `channel[T]` and no program a channel of
closures. Decision 257 since lets a type parameter bounded by `Escapable`
into an owner, so stage 0 accepts a generic channel written `[T: Send +
Escapable]`, `Send` for the tasks its halves cross to and `Escapable` for
the channel; the rule makes the one bound that says what a channel is for
enough. That `Send` as a bound must also ask ownership follows from the
bug decision 230 fixed: `str` is `Send` by its parts (text is a leaf that
is both, docs/architecture.md and the prelude say), so with `Send` alone a
generic `make_pair[T: Send]() -> (Sender[T], Receiver[T])` instantiated at
`str` would hand its caller a channel of views, and `tx.send("a{i}")`
would be the freed text again. A bound cannot see how long what a value
borrows lives; a scope can, which is why a task's captures keep the
structural check. Swift draws the line in the same place: a generic
parameter there is escapable unless it opts out (`~Escapable`), so a
`Sendable` one stands for values that may be kept. For closures, 03 and 04
decide (Q-52) that capture kinds stay out of function types and are
checked where the concrete closure is known; for a task that is its
`spawn`, which already takes a closure written at the call (`E0425`), and
for a channel it is the send.

**In stage 1.** `typeck.program`: `Program.can_borrow_in` (a parameter
bounded by `Send` holds no loan, nor does a half of a channel of closures,
`carries_closures`), `implements_depth` (`Send` as a bound asks
`can_borrow_in` to be false), `auto_depth` (a half of a channel of
closures is `Send` and `Sync`). `typeck.call`: a send records a
`SendCheck` (the element type, the value, the call). `typeck.tasks`:
`check_channel_type` (the channel's element, from `check_owners`) and
`check_sends` with `check_sent_closure` (each send, once captures are
decided), both before the bounds are checked. `typeck`:
`bound_not_satisfied` bounds the first parameter of a type that is not
`Send` because of its parameters, and notes `borrow.own-instead` for a
type that borrows whatever they are. Lowering
and the ownership checker are unchanged: the SIR of a generic channel is
what stage 0 writes for `[T: Send + Escapable]`, and a closure sent on a
channel is lowered as any closure passed by value; the interpreter and the
native backend run both (the class `behaviour`).

**What it changes.** No program of the corpus: none bounds a parameter by
`Send` and gives it a view, makes a generic channel or a channel of
closures, or makes a channel of a concrete type that is not `Send`, so
every class of `check.sh` over the corpus is unchanged. Stage 1's
`cases/channels/` show the rule:

- `generic_pipeline.sisu`: `pipeline[T: Send + Ord]` sends ints, `String`s
  and a struct of the program's own to worker tasks through a
  `channel[T]` and keeps them in a `Mutex[list[T]]`, and `relay[T: Send]`
  moves values from one channel to another; its `.out` is what both the
  interpreter and the native executable print from stage 1's SIR.
- `channel_of_closures.sisu`: worker tasks run the jobs a
  `channel[() -> int]` carries, `own` closures written at the send in a
  function that takes the `Sender`, and a `channel[(int) -> int]` carries
  an `own` closure and one that captures nothing; its `.out` likewise.
- `channel_refusals.sisu`: each refused form with its fix: a channel and
  a send of an unbounded parameter, of one bounded by `Escapable` alone,
  of a `list[T]`; a view given to a `Send` parameter; a `Shared[Cell[int]]`
  channel; a plain closure that captures, an `own` closure that copies a
  view, and a function value, sent on a channel of closures; and the two
  closures it accepts.

Stage 0 refuses the first two (`E0421`, and `E0427` for the receiver of
closures) and reports `E0421` where stage 1 reports the third's `E0347`s,
`E0425` and its other `E0421`s:

- Differs: `compiler/stage1/cases/channels/generic_pipeline.sisu`: sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/channels/channel_of_closures.sisu`: sir, ids, human, doc, doc-output
- Differs: `compiler/stage1/cases/channels/channel_refusals.sisu`: types, ids, human, doc, doc-output

**Documents to bring along** (outside stage 1, for the coordinator): the
prelude's documentation of `channel` ("What a channel carries can be
neither a view nor must-consume") and of `Send` (the bound asks ownership
too, and a channel half of a function type is both); 06, "Channels and
select" (a generic pipeline and a channel of jobs are now examples it can
show); docs/architecture.md, "Type checking" (the owners' bullet), "Send,
Sync and what a task captures", and known over-rejections 13 and 14,
which narrow; decision 230's last sentences.

**Decision 269**, for docs/architecture.md:

269. What a channel carries is `Send`, as the bound asks it, so a generic
     function makes a channel of its type parameter and a program a
     channel of closures, which decision 230 refused (a type parameter
     and a function value counted as views). A channel takes what one
     task sends to another, which may keep it after the sender's frame is
     gone: `channel[T]` and a send (`send`, `try_send`, a `select` arm's)
     of a value of a type parameter ask `T: Send`, and `Send` as a bound
     asks the type to own what it holds besides being `Send` by its parts
     (06, "Send, Sync, and shared state"; 04, "Views"), since a bound
     cannot see how long what a value borrows lives: with `Send` alone a
     generic function instantiated at `str` would hand its caller a
     channel of views, decision 230's bug. So a type parameter bounded by
     `Send` holds no loan, as one bounded by `Escapable` (decision 257),
     and `def pipeline[T: Send](own xs: list[T])` makes a `channel[T]` and
     keeps its values in a `Mutex`; a view is no `Send` type argument
     (`E0347`, with the note `borrow.own-instead`); what a task captures
     is still checked by its parts alone, since its scope sees how long a
     view lives. A missing bound is `E0347` with the fix `type.add-bound`
     naming `Send` (the first parameter in the way, for a type that holds
     some and would be `Send` with them bounded), once a body for each
     parameter; a concrete view or must-consume type is still `E0421` or
     `E0422` at the channel, and another type that is not `Send` is
     `E0347`. A function type is
     `Send` when its closure is, which the type does not say (Q-52), so
     each closure is checked where it is sent, as a task's where it is
     spawned: a send on a channel of a function type takes a closure
     written at the call (`E0425`, as `spawn`), which owns what it
     captures (a plain closure that captures anything borrows it:
     `E0421`, with the note `borrow.use-own-closure`), each capture owning
     what it holds (`E0421`, naming it) and `Send` (`E0426`); the halves
     of such a channel are `Send` and `Sync` and own what they hold, and
     a function type is still no `Send` type argument, since a generic
     body could send any closure of it. Lowering does not change: the SIR
     is what stage 0 writes for `[T: Send + Escapable]` and for a closure
     passed by value, and the interpreter and the native backend run it.
     Known over-rejections 13 and 14 narrow accordingly. Stage 1 decides
     it (its RULES.md, R1, with `cases/channels/`); the frozen Rust front
     end keeps decision 230's rule.
